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

#[cfg(unix)]
mod pty_journey {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::FromRawFd;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    extern "C" {
        fn openpty(
            amaster: *mut std::ffi::c_int,
            aslave: *mut std::ffi::c_int,
            name: *mut std::ffi::c_char,
            termp: *const std::ffi::c_void,
            winp: *const Winsize,
        ) -> std::ffi::c_int;
    }

    struct PtyApp {
        master: File,
        child: Child,
        screen: Arc<Mutex<String>>,
    }

    impl PtyApp {
        fn spawn(repo: &Path, bin_dir: &Path, log_path: &Path) -> Self {
            let mut master_fd = -1;
            let mut slave_fd = -1;
            let size = Winsize {
                ws_row: 40,
                ws_col: 120,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let rc = unsafe {
                openpty(
                    &mut master_fd,
                    &mut slave_fd,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &size,
                )
            };
            assert_eq!(rc, 0, "openpty failed");

            let master = unsafe { File::from_raw_fd(master_fd) };
            let slave_in = unsafe { File::from_raw_fd(slave_fd) };
            let slave_out = slave_in.try_clone().unwrap();
            let slave_err = slave_in.try_clone().unwrap();

            let base_path = std::env::var("PATH").unwrap_or_default();
            let path_env = format!("{}:{base_path}", bin_dir.display());

            let child = Command::new(env!("CARGO_BIN_EXE_lgtm"))
                .current_dir(repo)
                .env("PATH", path_env)
                .env("TERM", "xterm-256color")
                .env_remove("GITHUB_TOKEN")
                .env_remove("GH_TOKEN")
                .env_remove("LGTM_DEBUG_LOG")
                .args([
                    "--debug",
                    "--debug-log",
                    log_path.to_str().unwrap(),
                    "--agent-sink",
                    "dummy",
                ])
                .stdin(Stdio::from(slave_in))
                .stdout(Stdio::from(slave_out))
                .stderr(Stdio::from(slave_err))
                .spawn()
                .unwrap();

            let mut reader = master.try_clone().unwrap();
            let screen = Arc::new(Mutex::new(String::new()));
            let screen_buf = screen.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    screen_buf
                        .lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(&buf[..n]));
                }
            });

            Self {
                master,
                child,
                screen,
            }
        }

        fn send(&mut self, bytes: &[u8]) {
            self.master.write_all(bytes).unwrap();
            self.master.flush().unwrap();
        }

        fn output(&self) -> String {
            self.screen.lock().unwrap().clone()
        }

        fn wait_for(&self, mut predicate: impl FnMut(&str) -> bool, label: &str) {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                let text = self.output();
                if predicate(&text) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            panic!(
                "timed out waiting for {label}; tail of terminal output:\n{}",
                self.output()
                    .chars()
                    .rev()
                    .take(600)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            );
        }
    }

    impl Drop for PtyApp {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn write_executable(path: &Path, script: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, script).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    fn wait_file(path: &Path, label: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if path.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        panic!("timed out waiting for file {label}: {}", path.display());
    }

    #[test]
    fn bdd_cli_006_terminal_journey_navigates_switches_prs_preserves_drafts_and_rejects_stale_loads(
    ) {
        // Given an isolated local Git repository with a multi-line working-tree diff
        // and local fake provider commands on PATH.
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("repo");
        let bin_dir = temp.path().join("bin");
        let gates_dir = temp.path().join("gates");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&gates_dir).unwrap();

        let repo = lgtm_core::git2::Repository::init(&repo_dir).unwrap();
        repo.remote("origin", "https://github.com/acme/repo.git")
            .unwrap();
        std::fs::write(
            repo_dir.join("local.rs"),
            "fn one() {}\nfn two() {}\nfn three() {}\n",
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("local.rs")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = lgtm_core::git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        std::fs::write(
            repo_dir.join("local.rs"),
            "fn one() {}\nfn two() {}\nfn three() {}\nfn four() {}\nfn five() {}\n",
        )
        .unwrap();

        write_executable(
            &bin_dir.join("git"),
            r#"#!/bin/sh
if [ "$1" = "fetch" ]; then
    echo "fatal: couldn't find remote ref" >&2
    exit 128
fi
exec /usr/bin/git "$@"
"#,
        );

        let gh_script = format!(
            r#"#!/bin/sh
GATES="{}"
echo "$@" >> "$GATES/gh_calls.log"
if [ "$1" = "pr" ] && [ "$2" = "diff" ] && [ "$3" = "10" ]; then
    touch "$GATES/pr10_started"
    while [ ! -f "$GATES/release_pr10" ]; do sleep 0.02; done
    cat << 'EOF'
diff --git a/pr10.rs b/pr10.rs
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/pr10.rs
@@ -0,0 +1,3 @@
+fn stale_pr_ten() {{}}
+fn ten_two() {{}}
+fn ten_three() {{}}
EOF
    exit 0
fi
if [ "$1" = "pr" ] && [ "$2" = "diff" ] && [ "$3" = "20" ]; then
    if [ ! -f "$GATES/pr20_first_done" ]; then
        touch "$GATES/pr20_started"
        while [ ! -f "$GATES/release_pr20" ]; do sleep 0.02; done
        touch "$GATES/pr20_first_done"
    else
        touch "$GATES/pr20_second_started"
        while [ ! -f "$GATES/release_pr20_second" ]; do sleep 0.02; done
        touch "$GATES/pr20_second_done"
    fi
    cat << 'EOF'
diff --git a/pr20.rs b/pr20.rs
new file mode 100644
index 0000000..2222222
--- /dev/null
+++ b/pr20.rs
@@ -0,0 +1,3 @@
+fn active_pr_twenty() {{}}
+fn twenty_two() {{}}
+fn twenty_three() {{}}
EOF
    exit 0
fi
if [ "$1" = "pr" ] && [ "$2" = "checks" ]; then
    echo "[]"
    exit 0
fi
if [ "$1" = "api" ]; then
    echo '{{"data":{{"repository":{{"pullRequest":{{"reviewThreads":{{"nodes":[]}}}}}}}}}}'
    exit 0
fi
echo "[]"
"#,
            gates_dir.display()
        );
        write_executable(&bin_dir.join("gh"), &gh_script);

        let log_path = temp.path().join("debug.jsonl");
        let mut app = PtyApp::spawn(&repo_dir, &bin_dir, &log_path);

        // Wait for initial local diff to render in the PTY.
        app.wait_for(|s| s.contains("local.rs"), "initial local.rs render");

        // When the reviewer starts loading PR #10 and navigates while it is still loading:
        app.send(b":pr 10\r");
        wait_file(&gates_dir.join("pr10_started"), "pr10_started");
        let draws_before_nav = std::fs::read_to_string(&log_path)
            .unwrap_or_default()
            .lines()
            .filter(|l| l.contains("\"span\":\"ui.draw\""))
            .count();
        app.send(b"jj");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let draws = std::fs::read_to_string(&log_path)
                .unwrap_or_default()
                .lines()
                .filter(|l| l.contains("\"span\":\"ui.draw\""))
                .count();
            if draws > draws_before_nav {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }

        // And switches to PR #20 before PR #10 completes, then PR #10 finishes:
        app.send(b":pr 20\r");
        std::fs::write(gates_dir.join("release_pr10"), "").unwrap();
        wait_file(&gates_dir.join("pr20_started"), "pr20_started");

        // And starts a visual selection while PR #20 finishes loading:
        app.send(b"V");
        app.wait_for(|s| s.contains("VISUAL"), "visual selection active");
        std::fs::write(gates_dir.join("release_pr20"), "").unwrap();
        wait_file(&gates_dir.join("pr20_first_done"), "pr20_first_done");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            if log.contains("\"span\":\"review.pr_prepare\"")
                && log.contains("\"callbacks_remaining\":0")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        assert!(
            !app.output().contains("stale_pr_ten"),
            "stale PR #10 must never be applied"
        );
        assert!(
            !app.output().contains("active_pr_twenty"),
            "PR #20 must remain deferred while visual selection is active"
        );

        // Clearing the visual selection applies PR #20 immediately.
        app.send(b"\x1b");
        app.wait_for(|s| s.contains("active_pr_twenty"), "PR #20 diff applied");
        assert!(!app.output().contains("stale_pr_ten"));

        // When PR #20 is reloaded while a comment draft is open:
        app.send(b":pr 20\r");
        wait_file(
            &gates_dir.join("pr20_second_started"),
            "pr20_second_started",
        );
        app.send(b"ccretained_draft_on_pr20");
        app.wait_for(
            |s| s.contains("retained_draft_on_pr20"),
            "draft visible in modal",
        );
        std::fs::write(gates_dir.join("release_pr20_second"), "").unwrap();
        wait_file(&gates_dir.join("pr20_second_done"), "pr20_second_done");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            let prepares = log
                .lines()
                .filter(|l| l.contains("\"span\":\"review.pr_prepare\""))
                .count();
            if prepares >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        // Give the UI loop one background poll cycle to receive the callback and defer while modal is open.
        std::thread::sleep(Duration::from_millis(40));

        // Submitting the draft saves the directive on PR #20, reprepares without an extra fetch, and exits cleanly.
        app.send(b"\r");
        let comments_path = repo_dir.join(".lgtm/comments.jsonl");
        wait_file(&comments_path, "saved comments.jsonl");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            let applies = log
                .lines()
                .filter(|l| l.contains("\"span\":\"ui.pr_apply\""))
                .count();
            if applies >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }

        app.send(b":q\r");
        let status = app.child.wait().unwrap();
        assert!(status.success(), "expected clean exit, got {status}");

        // Then the saved directive belongs to PR #20, stale PR #10 was rejected, and no extra PR #20 fetch occurred.
        let saved = lgtm_core::comment::CommentStore::new(&repo_dir).load_comments();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].pr_number, Some(20));
        assert_eq!(saved[0].file_path, "pr20.rs");
        assert_eq!(saved[0].comment_text, "retained_draft_on_pr20");

        let gh_calls = std::fs::read_to_string(gates_dir.join("gh_calls.log")).unwrap();
        let pr20_diff_calls = gh_calls
            .lines()
            .filter(|line| *line == "pr diff 20")
            .count();
        assert_eq!(
            pr20_diff_calls, 2,
            "repreparation after draft submit must not refetch PR #20"
        );
    }

    #[test]
    fn bdd_cli_007_terminal_responsiveness_under_load_measures_percentiles() {
        // Given a large review fixture with 1,000 changed lines and 50 saved directives.
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("repo");
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();

        let repo = lgtm_core::git2::Repository::init(&repo_dir).unwrap();
        let base_content = (0..500)
            .map(|i| format!("fn line_{i}() {{}}\n"))
            .collect::<String>();
        std::fs::write(repo_dir.join("main.rs"), &base_content).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("main.rs")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = lgtm_core::git2::Signature::now("Benchmark", "bench@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let modified_content = (0..1000)
            .map(|i| format!("fn modified_line_{i}() {{ println!(\"load\"); }}\n"))
            .collect::<String>();
        std::fs::write(repo_dir.join("main.rs"), &modified_content).unwrap();

        // 50 saved directives
        let comment_store = lgtm_core::comment::CommentStore::new(&repo_dir);
        let comments: Vec<lgtm_core::comment::CommentAnchor> = (0..50)
            .map(|i| {
                serde_json::from_value(serde_json::json!({
                    "id": format!("bench-directive-{i}"),
                    "file_path": "main.rs",
                    "target_line_no": (i * 10 + 1) as u32,
                    "target_code": "code",
                    "context_before": [],
                    "context_after": [],
                    "comment_text": format!("perf directive #{i}"),
                    "created_at_turn": 1,
                    "is_resolved": false,
                }))
                .unwrap()
            })
            .collect();
        comment_store.save_comments(&comments).unwrap();

        let log_path = temp.path().join("debug.jsonl");
        let mut app = PtyApp::spawn(&repo_dir, &bin_dir, &log_path);

        // Wait for initial diff render
        app.wait_for(|s| s.contains("main.rs"), "initial main.rs render");

        // When navigating through lines in the real terminal under load:
        let mut samples_ms = Vec::new();
        for _ in 0..15 {
            let draws_before = std::fs::read_to_string(&log_path)
                .unwrap_or_default()
                .lines()
                .filter(|l| l.contains("\"span\":\"ui.draw\""))
                .count();
            let start = Instant::now();
            app.send(b"j");
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                let draws = std::fs::read_to_string(&log_path)
                    .unwrap_or_default()
                    .lines()
                    .filter(|l| l.contains("\"span\":\"ui.draw\""))
                    .count();
                if draws > draws_before {
                    samples_ms.push(start.elapsed().as_secs_f64() * 1000.0);
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        app.send(b":q\r");
        let status = app.child.wait().unwrap();
        assert!(status.success(), "expected clean exit, got {status}");

        // Then responsiveness percentiles are recorded and reported without machine-speed assertions.
        if !samples_ms.is_empty() {
            samples_ms.sort_by(f64::total_cmp);
            let p50 = samples_ms[samples_ms.len() * 50 / 100];
            let p90 = samples_ms[samples_ms.len() * 90 / 100];
            let p95 = samples_ms[samples_ms.len() * 95 / 100];
            println!(
                "Q6 Terminal Responsiveness (1000 lines, 50 directives, 120x40 PTY): \
                 p50={:.2}ms, p90={:.2}ms, p95={:.2}ms (n={})",
                p50,
                p90,
                p95,
                samples_ms.len()
            );
        }
    }

    #[test]
    fn bdd_cli_008_command_line_vim_readline_editing_shortcuts() {
        // Given an active LGTM TUI session in a git repository.
        let temp = tempfile::tempdir().unwrap();
        let repo_dir = temp.path().join("repo");
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::create_dir_all(&bin_dir).unwrap();

        let repo = lgtm_core::git2::Repository::init(&repo_dir).unwrap();
        let base_content = "fn main() {}\n";
        std::fs::write(repo_dir.join("main.rs"), base_content).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("main.rs")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = lgtm_core::git2::Signature::now("Tester", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        std::fs::write(
            repo_dir.join("main.rs"),
            "fn main() { println!(\"edit\"); }\n",
        )
        .unwrap();

        let log_path = temp.path().join("debug.jsonl");
        let mut app = PtyApp::spawn(&repo_dir, &bin_dir, &log_path);
        app.wait_for(|s| s.contains("main.rs"), "initial main.rs render");

        // When entering command line mode, typing garbage, then clearing with Ctrl-u:
        app.send(b":garbage\x15");

        // And typing a command with extra argument, then deleting the argument with Ctrl-w:
        app.send(b"help 123\x17\r");

        // Then the Help modal successfully opens:
        app.wait_for(
            |s| s.contains("COMMAND / EX Mode") || s.contains("LGTM Documentation"),
            "Help modal opened via line-edited command",
        );

        // Close help modal with Esc:
        app.send(b"\x1b");
        app.wait_for(|s| s.contains("main.rs"), "returned to diff view");

        // Clean exit with :q
        app.send(b":q\r");
        let status = app.child.wait().unwrap();
        assert!(status.success(), "expected clean exit, got {status}");
    }
}
