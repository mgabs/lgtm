use super::*;
use crate::app::test_util::{create_test_app, wait_for_local_refresh};
use std::time::Duration;

fn result(text: &str) -> LocalResult {
    let patch =
        format!("diff --git a/a.txt b/a.txt\n--- /dev/null\n+++ b/a.txt\n@@ -0,0 +1 @@\n+{text}\n");
    LocalResult {
        target: LocalTarget::WorkingTree,
        snapshot: crate::pr::PrEngine::parse_unified_diff(&patch).unwrap(),
        findings: Default::default(),
    }
}

fn queue(app: &mut App, text: &'static str) {
    app.queue_local_job(LocalTarget::WorkingTree, false, move |_, _| {
        Ok(result(text))
    });
}

fn apply_next(app: &mut App) {
    let job = app
        .jobs_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("worker did not complete");
    job(app);
}

fn text(app: &App) -> &str {
    &app.snapshot.files[0].hunks[0].lines[0].content
}

fn git(app: &App, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(&app.git_engine.repo_path)
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn watcher_backlog_invalidates_a_ready_worker_before_its_callback() {
    let mut app = create_test_app();
    app.snapshot = result("visible").snapshot;
    queue(&mut app, "obsolete");
    let job = app.jobs_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    app.jobs_tx.send(job).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    // Put the actual workspace change beyond the per-iteration watcher budget.
    for _ in 0..32 {
        tx.send(WatcherMessage::FilesChanged(vec![app
            .git_engine
            .repo_path
            .join(".lgtm/debug.jsonl")]))
            .unwrap();
    }
    tx.send(WatcherMessage::FilesChanged(vec![app
        .git_engine
        .repo_path
        .join("a.txt")]))
        .unwrap();
    app.handle_background_events();
    assert_eq!(text(&app), "visible");
    assert!(!app.watcher_rx.is_empty());
    wait_for_local_refresh(&mut app);
    assert!(!app
        .snapshot
        .files
        .iter()
        .flat_map(|f| &f.hunks)
        .flat_map(|h| &h.lines)
        .any(|line| line.content == "obsolete"));
}

#[test]
fn explicit_audit_supersedes_deferred_startup_diff_audit() {
    let mut app = create_test_app();
    queue(&mut app, "local");
    app.open_initial_diff_audit();
    assert!(app.local_refresh.audit_after);
    app.trigger_repo_scan();
    assert!(!app.local_refresh.audit_after);
    app.active_view = ActiveView::Diff;
    wait_for_local_refresh(&mut app);
    crate::app::test_util::wait_for_audit(&mut app);
    assert_eq!(app.active_view, ActiveView::Diff);
}

#[test]
fn requests_coalesce_and_input_works_while_a_worker_is_blocked() {
    let mut app = create_test_app();
    app.snapshot = result("visible").snapshot;
    let ui_thread = std::thread::current().id();
    let (entered_tx, entered_rx) = crossbeam_channel::bounded(1);
    let (release_tx, release_rx) = crossbeam_channel::bounded(1);
    let runs = Arc::new(AtomicU64::new(0));
    let first_runs = runs.clone();
    app.queue_local_job(LocalTarget::WorkingTree, false, move |_, _| {
        assert_ne!(std::thread::current().id(), ui_thread);
        first_runs.fetch_add(1, Ordering::SeqCst);
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(result("obsolete"))
    });
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    // No elapsed-time assertion: the barrier proves the work hasn't completed yet.
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    assert_eq!(app.active_view, ActiveView::CommandLine);
    for index in 0..20 {
        let runs = runs.clone();
        app.queue_local_job(LocalTarget::WorkingTree, false, move |_, _| {
            runs.fetch_add(1, Ordering::SeqCst);
            assert_eq!(index, 19, "intermediate request should be coalesced away");
            Ok(result("latest"))
        });
    }
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    release_tx.send(()).unwrap();
    apply_next(&mut app);
    assert_eq!(text(&app), "visible");
    app.active_view = ActiveView::Diff;
    apply_next(&mut app);
    assert_eq!(text(&app), "latest");
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert!(!app.local_refresh_pending());
}

#[test]
fn watcher_invalidates_a_queued_completion_before_it_can_apply() {
    let mut app = create_test_app();
    git(&app, &["init", "-q"]);
    std::fs::write(
        app.git_engine.repo_path.join(".git/info/exclude"),
        ".lgtm/\n",
    )
    .unwrap();
    let file = app.git_engine.repo_path.join("a.txt");
    std::fs::write(&file, "genuine edit\n").unwrap();
    app.snapshot = result("visible").snapshot;
    queue(&mut app, "obsolete");
    let callback = app.jobs_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    app.jobs_tx.send(callback).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    tx.send(WatcherMessage::FilesChanged(vec![file])).unwrap();
    app.handle_background_events();
    assert_ne!(text(&app), "obsolete");
    wait_for_local_refresh(&mut app);
    assert_eq!(text(&app), "genuine edit");
}

#[test]
fn applying_waits_for_drafts_and_preserves_newer_directive_edits() {
    let mut app = create_test_app();
    app.snapshot = result("visible").snapshot;
    app.active_view = ActiveView::CommentInput;
    app.comment_input = "draft tied to visible snapshot".into();
    queue(&mut app, "replacement");
    apply_next(&mut app);
    assert_eq!(text(&app), "visible");
    assert!(app.local_refresh.ready.is_some());
    let mut directive: CommentAnchor = serde_json::from_value(serde_json::json!({
        "id":"stable", "file_path":"a.txt", "target_line_no":1, "target_code":"visible",
        "context_before":[], "context_after":[], "comment_text":"newer edit", "replies":["newer reply"],
        "created_at_turn":1, "is_resolved":true, "remote_id":99,
    })).unwrap();
    app.comments.push(directive.clone());
    directive.id = "different-review".into();
    directive.pr_number = Some(42);
    app.comments.push(directive.clone());
    app.active_view = ActiveView::Diff;
    assert!(app.apply_ready_local());
    assert_eq!(text(&app), "replacement");
    assert_eq!(app.comment_input, "draft tied to visible snapshot");
    assert_eq!(app.comments[0].id, "stable");
    assert_eq!(app.comments[0].comment_text, "newer edit");
    assert_eq!(app.comments[0].replies, ["newer reply"]);
    assert_eq!(app.comments[0].remote_id, Some(99));
    assert!(app.comments[0].is_resolved);
    assert_eq!(app.comments[1], directive);
}

#[test]
fn newer_edits_replace_a_result_held_for_a_modal() {
    let mut app = create_test_app();
    app.snapshot = result("visible").snapshot;
    app.active_view = ActiveView::CommentReply;
    app.comment_reply_input = "reply draft".into();
    queue(&mut app, "first");
    apply_next(&mut app);
    queue(&mut app, "newer edit");
    assert!(app.local_refresh.ready.is_none());
    apply_next(&mut app);
    assert_eq!(text(&app), "visible");
    app.active_view = ActiveView::Diff;
    app.handle_background_events();
    assert_eq!(text(&app), "newer edit");
    assert_eq!(app.comment_reply_input, "reply draft");
}

#[test]
fn pr_selection_cancels_local_results_and_pending_work() {
    let mut app = create_test_app();
    app.snapshot = result("keep PR").snapshot;
    queue(&mut app, "obsolete workspace");
    let local_callback = app.jobs_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    queue(&mut app, "queued workspace");
    app.pr.current_number = Some(42);
    app.load_pr_async(43); // Test forge returns Unsupported, preserving the current PR.
    local_callback(&mut app);
    assert!(!app.local_refresh_pending());
    assert_eq!(app.pr.current_number, Some(42));
    assert_eq!(text(&app), "keep PR");
    assert!(app.local_refresh.pending.is_none());
}

#[test]
fn a_failed_refresh_keeps_the_current_review() {
    let mut app = create_test_app();
    app.snapshot = result("keep me").snapshot;
    app.queue_local_job(LocalTarget::WorkingTree, false, |_, _| {
        anyhow::bail!("fixture failure")
    });
    apply_next(&mut app);
    assert_eq!(text(&app), "keep me");
    assert!(app
        .status_message
        .as_deref()
        .unwrap()
        .contains("fixture failure"));
    assert!(!app.local_refresh_pending());
}

#[test]
fn visual_selection_blocks_replacement_and_navigation_tracks_the_file() {
    let mut app = create_test_app();
    let mut old = result("visible").snapshot;
    old.files[0].new_path = "keep.txt".into();
    app.snapshot = old;
    app.visual_mode = true;
    app.cursor_line_idx = 99;
    app.search_matches.push((42, 99));
    app.queue_local_job(LocalTarget::WorkingTree, false, |_, _| {
        let mut result = result("other file");
        let mut keep = result.snapshot.files[0].clone();
        keep.new_path = "keep.txt".into();
        result.snapshot.files.push(keep);
        Ok(result)
    });
    apply_next(&mut app);
    assert_eq!(app.snapshot.files.len(), 1);
    app.visual_mode = false;
    app.apply_ready_local();
    assert_eq!(app.selected_file_idx, 1);
    assert_eq!(app.cursor_line_idx, 0);
    assert!(app.search_matches.is_empty());
}

#[test]
fn revision_loading_and_reload_preserve_the_requested_source() {
    let mut app = create_test_app();
    git(&app, &["init", "-q"]);
    let file = app.git_engine.repo_path.join("a.txt");
    std::fs::write(&file, "base\n").unwrap();
    git(&app, &["add", "a.txt"]);
    git(&app, &["commit", "-qm", "base"]);
    std::fs::write(&file, "base\nfeature\n").unwrap();
    git(&app, &["commit", "-qam", "feature"]);
    std::fs::write(&file, "base\nfeature\nuncommitted\n").unwrap();
    app.snapshot = result("keep PR until ready").snapshot;
    app.pr.current_number = Some(42);
    app.execute_command(":diff HEAD");
    assert_eq!(app.pr.current_number, Some(42));
    assert_eq!(text(&app), "keep PR until ready");
    wait_for_local_refresh(&mut app);
    assert_eq!(app.pr.current_number, None);
    assert_eq!(app.active_revision.as_deref(), Some("HEAD"));
    assert_eq!(app.snapshot.total_additions, 1);
    app.execute_command(":reload");
    wait_for_local_refresh(&mut app);
    assert_eq!(app.snapshot.total_additions, 1);
    assert_eq!(app.active_revision.as_deref(), Some("HEAD"));
}

#[test]
fn startup_and_initial_diff_audit_wait_for_the_background_snapshot() {
    let first = create_test_app();
    git(&first, &["init", "-q"]);
    std::fs::write(
        first.git_engine.repo_path.join(".git/info/exclude"),
        ".lgtm/\n",
    )
    .unwrap();
    std::fs::write(first.git_engine.repo_path.join("a.txt"), "startup\n").unwrap();
    let (_, rx) = crossbeam_channel::unbounded();
    let mut app = App::new(first.git_engine.repo_path.clone(), rx, None, None, None);
    assert!(app.snapshot.files.is_empty());
    assert!(app.local_refresh_pending());
    app.open_initial_diff_audit();
    wait_for_local_refresh(&mut app);
    assert_eq!(text(&app), "startup");
    assert_eq!(app.active_view, ActiveView::AuditReport);
    assert_eq!(app.audit.files, app.snapshot.files);
}
