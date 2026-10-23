use super::*;
use crate::app::test_util::{create_test_app, wait_for_view_data};
use ratatui::{backend::TestBackend, Terminal};
use std::time::Duration;

fn git(app: &App, args: &[&str]) {
    let result = std::process::Command::new("git")
        .current_dir(&app.git_engine.repo_path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
fn repo() -> App {
    let app = create_test_app();
    git(&app, &["init", "-q"]);
    app
}
fn screen(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(150, 45)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(frame, app))
        .unwrap();
    format!("{:?}", terminal.backend().buffer())
}
fn alert(path: &str, line: u32) -> GuardAlert {
    GuardAlert {
        file_path: path.into(),
        line_no: Some(line),
        rule_id: "fixture".into(),
        fingerprint: path.into(),
        title: "Finding".into(),
        description: "Check this line".into(),
        severity: crate::guard::Severity::High,
    }
}

#[test]
fn renderer_uses_prepared_git_turn_gate_and_staged_data() {
    let mut app = repo();
    let root = app.git_engine.repo_path.clone();
    std::fs::write(root.join("sample.rs"), "let value = 1;\n").unwrap();
    git(&app, &["add", "sample.rs"]);
    std::fs::write(
        root.join(".lgtm/state.json"),
        r#"{"current_turn":9,"last_reviewed_tree_oid":null,"turn_timestamps":[]}"#,
    )
    .unwrap();
    std::fs::write(
        root.join(".lgtm/gate.json"),
        r#"{"status":"waiting","message":"Review"}"#,
    )
    .unwrap();
    app.active_view = ActiveView::CommitModal;
    wait_for_view_data(&mut app);
    assert!(app.view_data.is_git);
    assert_eq!(app.view_data.turn, 9);
    assert_eq!(app.view_data.staged_count, 1);
    let before = screen(&mut app);
    let requests = app.view_data.requests_started;
    std::fs::rename(root.join(".git"), root.join(".git-away")).unwrap();
    std::fs::write(
        root.join(".lgtm/state.json"),
        r#"{"current_turn":77,"last_reviewed_tree_oid":null,"turn_timestamps":[]}"#,
    )
    .unwrap();
    std::fs::write(root.join(".lgtm/gate.json"), r#"{"status":"approved"}"#).unwrap();
    for _ in 0..3 {
        assert_eq!(screen(&mut app), before);
    }
    assert_eq!(app.view_data.requests_started, requests);
    app.invalidate_view_data();
    wait_for_view_data(&mut app);
    assert!(!app.view_data.is_git);
    assert_eq!(app.view_data.turn, 77);
    assert_eq!(app.view_data.staged_count, 0);
    assert_eq!(app.view_data.gate.as_ref().unwrap().0, "approved");
}

#[test]
fn snippet_is_cached_and_removed_only_after_invalidation() {
    let mut app = create_test_app();
    let path = app.audit.root_dir.join("sample.rs");
    std::fs::write(
        &path,
        (1..=10).map(|n| format!("line {n}\n")).collect::<String>(),
    )
    .unwrap();
    app.audit.alerts = vec![alert("sample.rs", 5)];
    app.active_view = ActiveView::AuditReport;
    wait_for_view_data(&mut app);
    let lines = app
        .view_data
        .snippet_for(&app.audit.root_dir, &app.audit.alerts[0])
        .unwrap();
    assert_eq!(lines.len(), 7);
    assert_eq!(lines[0], (2, "line 2".into()));
    assert_eq!(lines[6], (8, "line 8".into()));
    let before = screen(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(screen(&mut app), before);
    app.invalidate_view_data();
    wait_for_view_data(&mut app);
    assert!(app
        .view_data
        .snippet_for(&app.audit.root_dir, &app.audit.alerts[0])
        .is_none());
    let requests = app.view_data.requests_started;
    for _ in 0..3 {
        app.prepare_view_data();
        screen(&mut app);
    }
    assert_eq!(
        app.view_data.requests_started, requests,
        "missing files must also be cached"
    );
}

#[test]
fn rapid_selection_changes_coalesce_and_never_display_the_wrong_snippet() {
    let mut app = create_test_app();
    for path in ["first.rs", "middle.rs", "last.rs"] {
        std::fs::write(app.audit.root_dir.join(path), path).unwrap();
        app.audit.alerts.push(alert(path, 1));
    }
    app.active_view = ActiveView::AuditReport;
    app.prepare_view_data();
    let old = app.jobs_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    for selection in [1, 0, 1, 2] {
        app.audit.selected_idx = selection;
        app.prepare_view_data();
    }
    assert_eq!(app.view_data.requests_started, 1);
    old(&mut app);
    assert!(app
        .view_data
        .snippet_for(&app.audit.root_dir, &app.audit.alerts[2])
        .is_none());
    wait_for_view_data(&mut app);
    assert_eq!(app.view_data.requests_started, 2);
    assert_eq!(
        app.view_data
            .snippet_for(&app.audit.root_dir, &app.audit.alerts[2])
            .unwrap()[0]
            .1,
        "last.rs"
    );
}

#[test]
fn stale_results_are_discarded_after_file_changes() {
    let mut app = create_test_app();
    std::fs::write(
        app.git_engine.repo_path.join(".lgtm/prompt.txt"),
        "old prompt",
    )
    .unwrap();
    app.open_agent_prompt();
    app.prepare_view_data();
    let old = app.jobs_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    std::fs::write(
        app.git_engine.repo_path.join(".lgtm/prompt.txt"),
        "new prompt",
    )
    .unwrap();
    app.invalidate_view_data();
    old(&mut app);
    assert_eq!(
        app.view_data.prompt_for(&app.git_engine.repo_path).0,
        "Loading"
    );
    wait_for_view_data(&mut app);
    assert_eq!(
        app.view_data.prompt_for(&app.git_engine.repo_path).1,
        "new prompt"
    );
}

#[test]
fn reopening_prompt_refreshes_context_without_a_watcher_notification() {
    let mut app = create_test_app();
    let root = app.git_engine.repo_path.clone();
    std::fs::write(root.join(".lgtm/gate.json"), r#"{"message":"Gate prompt"}"#).unwrap();
    std::fs::write(root.join(".lgtm/prompt.txt"), "File prompt").unwrap();
    app.open_agent_prompt();
    wait_for_view_data(&mut app);
    assert_eq!(
        app.view_data.prompt_for(&root),
        ("Gate Context", "Gate prompt")
    );
    let before = screen(&mut app);
    std::fs::remove_file(root.join(".lgtm/gate.json")).unwrap();
    assert_eq!(screen(&mut app), before);
    app.open_agent_prompt();
    wait_for_view_data(&mut app);
    assert_eq!(
        app.view_data.prompt_for(&root),
        (".lgtm/prompt.txt", "File prompt")
    );
}

#[test]
fn background_completion_preserves_modal_drafts() {
    let mut app = create_test_app();
    app.open_agent_prompt();
    app.prepare_view_data();
    app.active_view = ActiveView::CommitModal;
    app.commit_message_input = "newer message".into();
    wait_for_view_data(&mut app);
    assert_eq!(app.active_view, ActiveView::CommitModal);
    assert_eq!(app.commit_message_input, "newer message");
}

#[test]
fn unreadable_snippets_and_malformed_status_files_do_not_retry_on_draw() {
    let mut app = create_test_app();
    let root = app.git_engine.repo_path.clone();
    std::fs::write(root.join("bad.rs"), [255, 254, 253]).unwrap();
    std::fs::write(root.join(".lgtm/state.json"), "invalid").unwrap();
    std::fs::write(root.join(".lgtm/gate.json"), "invalid").unwrap();
    app.audit.alerts = vec![alert("bad.rs", 1)];
    app.active_view = ActiveView::AuditReport;
    wait_for_view_data(&mut app);
    assert_eq!(app.view_data.turn, 1);
    assert!(app.view_data.gate.is_none());
    assert!(app
        .view_data
        .snippet_for(&root, &app.audit.alerts[0])
        .is_none());
    let requests = app.view_data.requests_started;
    app.prepare_view_data();
    screen(&mut app);
    assert_eq!(app.view_data.requests_started, requests);
}

#[test]
fn returning_to_a_recent_finding_reuses_its_snippet() {
    let mut app = create_test_app();
    for path in ["first.rs", "second.rs"] {
        std::fs::write(app.audit.root_dir.join(path), path).unwrap();
        app.audit.alerts.push(alert(path, 1));
    }
    app.active_view = ActiveView::AuditReport;
    wait_for_view_data(&mut app);
    app.audit.selected_idx = 1;
    wait_for_view_data(&mut app);
    let requests = app.view_data.requests_started;
    app.audit.selected_idx = 0;
    wait_for_view_data(&mut app);
    assert_eq!(app.view_data.requests_started, requests);
    assert_eq!(
        app.view_data
            .snippet_for(&app.audit.root_dir, &app.audit.alerts[0])
            .unwrap()[0]
            .1,
        "first.rs"
    );
}

#[test]
fn watcher_and_local_gate_updates_refresh_prepared_status() {
    let mut app = create_test_app();
    app.pr.current_number = Some(1);
    let root = app.git_engine.repo_path.clone();
    let gate = root.join(".lgtm/gate.json");
    std::fs::write(&gate, r#"{"status":"waiting"}"#).unwrap();
    wait_for_view_data(&mut app);
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    std::fs::write(&gate, r#"{"status":"approved"}"#).unwrap();
    tx.send(WatcherMessage::FilesChanged(vec![gate.clone()]))
        .unwrap();
    app.handle_background_events();
    wait_for_view_data(&mut app);
    assert_eq!(app.view_data.gate.as_ref().unwrap().0, "approved");
    std::fs::write(&gate, r#"{"status":"waiting"}"#).unwrap();
    app.approve_gate();
    wait_for_view_data(&mut app);
    assert_eq!(app.view_data.gate.as_ref().unwrap().0, "approved");
}
