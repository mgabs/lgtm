//! Acceptance scenarios that launch the compiled application.
use std::process::{Command, Output};

fn lgtm(repo: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lgtm"))
        .current_dir(repo)
        .env_remove("LGTM_DEBUG_LOG")
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn bdd_cli_001_user_can_install_inspect_and_remove_the_hook() {
    // Given a fresh Git repository.
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path()).unwrap();

    // When the user installs LGTM's hook and asks for its status.
    assert_success(&lgtm(repo.path(), &["hook", "install"]));
    let status = lgtm(repo.path(), &["hook", "status"]);
    assert_success(&status);
    assert!(String::from_utf8_lossy(&status.stdout).contains("installed and active"));
    assert!(
        std::fs::read_to_string(repo.path().join(".git/hooks/pre-commit"))
            .unwrap()
            .contains("lgtm check --strict")
    );

    // Then uninstall succeeds and status reports the hook is absent.
    assert_success(&lgtm(repo.path(), &["hook", "uninstall"]));
    let status = lgtm(repo.path(), &["hook", "status"]);
    assert_success(&status);
    assert!(String::from_utf8_lossy(&status.stdout).contains("not installed"));
    assert!(!repo.path().join(".git/hooks/pre-commit").exists());
}

#[test]
fn bdd_cli_002_a_log_path_requires_explicit_debug_opt_in() {
    // Given a user-specified diagnostic log path.
    let repo = tempfile::tempdir().unwrap();
    let path = repo.path().join("debug.jsonl");

    // When a command supplies the path without enabling debug.
    let output = lgtm(
        repo.path(),
        &["--debug-log", path.to_str().unwrap(), "hook", "status"],
    );

    // Then argument validation rejects the command without creating the log.
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--debug"));
    assert!(!path.exists());
}

#[test]
fn bdd_cli_003_debug_environment_alone_does_not_enable_logging() {
    // Given a valid repository and a diagnostic path supplied through the environment.
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path()).unwrap();
    let path = repo.path().join("debug.jsonl");
    let run = |debug: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lgtm"));
        command
            .current_dir(repo.path())
            .env("LGTM_DEBUG_LOG", &path);
        if debug {
            command.args(["--debug", "--debug-log"]).arg(&path);
        }
        command.args(["hook", "status"]).output().unwrap()
    };

    // When a normal command runs, then no diagnostic file is created.
    assert_success(&run(false));
    assert!(!path.exists());

    // When the user explicitly enables debug and selects the path with the CLI,
    // then the requested file is created. The environment is not a path option.
    assert_success(&run(true));
    assert!(path.is_file());
}

#[test]
fn bdd_cli_004_hook_install_outside_git_fails_without_creating_a_hook() {
    // Given a directory that does not belong to a Git repository.
    let repo = tempfile::tempdir().unwrap();

    // When the user tries to install the hook.
    let output = lgtm(repo.path(), &["hook", "install"]);

    // Then the application reports the failure and leaves the directory untouched.
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Not a git repository"));
    assert_eq!(std::fs::read_dir(repo.path()).unwrap().count(), 0);
}

#[test]
fn bdd_cli_005_invalid_pr_state_fails_before_loading_the_picker() {
    // Given an unsupported PR state filter.
    let repo = tempfile::tempdir().unwrap();

    // When the user requests that listing.
    let output = lgtm(repo.path(), &["prs", "--state", "merged-only"]);

    // Then argument validation reports supported choices without launching the TUI.
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("merged-only"));
    assert!(error.contains("open, closed, all"));
    assert_eq!(std::fs::read_dir(repo.path()).unwrap().count(), 0);
}
