use super::*;
use crate::app::test_util::{create_test_app, wait_for_audit};
use std::time::Duration;

fn output(name: &str) -> Output {
    let patch = format!("diff --git a/{name}.rs b/{name}.rs\n--- /dev/null\n+++ b/{name}.rs\n@@ -0,0 +1 @@\n+let value = 1;\n");
    Output {
        files: crate::pr::PrEngine::parse_unified_diff(&patch)
            .unwrap()
            .files,
        findings: Default::default(),
    }
}
fn seed(app: &mut App) {
    app.audit.files = output("previous").files;
    app.audit.has_result = true;
}
fn blocked(app: &mut App, kind: Kind) -> crossbeam_channel::Sender<()> {
    let (entered_tx, entered_rx) = crossbeam_channel::bounded(1);
    let (release_tx, release_rx) = crossbeam_channel::bounded(1);
    let ui = std::thread::current().id();
    app.queue_audit(kind, move |progress| {
        assert_ne!(std::thread::current().id(), ui);
        assert!(progress(0, 10));
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(output("finished"))
    });
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    release_tx
}

#[test]
fn blocked_audit_preserves_previous_results_and_allows_input() {
    let mut app = create_test_app();
    seed(&mut app);
    let release = blocked(&mut app, Kind::Repo);
    assert_eq!(app.audit.files[0].display_name(), "previous.rs");
    app.handle_key(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE));
    assert!(app.audit.maximized_inspector);
    app.handle_background_events();
    assert!(app.audit.progress.as_ref().unwrap().contains("0/10"));
    release.send(()).unwrap();
    wait_for_audit(&mut app);
    assert_eq!(app.audit.files[0].display_name(), "finished.rs");
    assert!(app.audit.progress.is_none());
}

#[test]
fn requests_keep_only_one_running_and_the_latest_queued_scan() {
    let mut app = create_test_app();
    let release = blocked(&mut app, Kind::Repo);
    app.queue_audit(Kind::Diff, |_| panic!("superseded pending work ran"));
    app.queue_audit(Kind::Repo, |_| Ok(output("latest")));
    release.send(()).unwrap();
    wait_for_audit(&mut app);
    assert_eq!(app.audit.files[0].display_name(), "latest.rs");
    assert!(!app.audit_pending());
}

#[test]
fn target_changes_and_source_edits_discard_old_audits() {
    for source_edit in [false, true] {
        let mut app = create_test_app();
        seed(&mut app);
        let release = blocked(&mut app, Kind::Repo);
        if source_edit {
            app.pr.current_number = Some(1);
            let (tx, rx) = crossbeam_channel::unbounded();
            app.watcher_rx = rx;
            tx.send(WatcherMessage::FilesChanged(vec![app
                .git_engine
                .repo_path
                .join("source.rs")]))
                .unwrap();
            app.handle_background_events();
        } else {
            app.load_local_revision("HEAD".into());
        }
        release.send(()).unwrap();
        wait_for_audit(&mut app);
        assert_eq!(app.audit.files[0].display_name(), "previous.rs");
        assert!(app.audit.progress.is_none());
    }
}

#[test]
fn snapshot_change_cancels_diff_audit_but_not_repository_audit() {
    for kind in [Kind::Diff, Kind::Repo] {
        let mut app = create_test_app();
        seed(&mut app);
        let release = blocked(&mut app, kind);
        app.invalidate_diff_content();
        release.send(()).unwrap();
        wait_for_audit(&mut app);
        assert_eq!(
            app.audit.files[0].display_name(),
            if kind == Kind::Repo {
                "finished.rs"
            } else {
                "previous.rs"
            }
        );
    }
}

#[test]
fn completion_preserves_newer_modal_draft_filters_and_diff_selection() {
    let mut app = create_test_app();
    let release = blocked(&mut app, Kind::Repo);
    app.active_view = ActiveView::CommentInput;
    app.comment_input = "newer draft".into();
    app.selected_file_idx = 7;
    app.audit.file_filter = Some("chosen.rs".into());
    app.audit.filter_severity = Some(crate::guard::Severity::High);
    release.send(()).unwrap();
    wait_for_audit(&mut app);
    assert_eq!(app.active_view, ActiveView::CommentInput);
    assert_eq!(app.comment_input, "newer draft");
    assert_eq!(app.selected_file_idx, 7);
    assert_eq!(app.audit.file_filter.as_deref(), Some("chosen.rs"));
    assert_eq!(
        app.audit.filter_severity,
        Some(crate::guard::Severity::High)
    );
}

#[test]
fn failure_keeps_previous_results() {
    let mut app = create_test_app();
    seed(&mut app);
    app.queue_audit(Kind::Repo, |_| anyhow::bail!("fixture failure"));
    wait_for_audit(&mut app);
    assert_eq!(app.audit.files[0].display_name(), "previous.rs");
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("fixture failure"));
    assert!(app.audit.progress.is_none());
}

#[test]
fn progress_updates_are_bounded_and_monotonic() {
    let mut app = create_test_app();
    let (entered_tx, entered_rx) = crossbeam_channel::bounded(1);
    let (release_tx, release_rx) = crossbeam_channel::bounded(1);
    app.queue_audit(Kind::Repo, move |progress| {
        for count in 0..=1000 {
            assert!(progress(count, 1000));
        }
        assert!(progress(10, 1000));
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(output("done"))
    });
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(app.jobs_rx.len() <= 21);
    let mut previous = 0;
    while !app.jobs_rx.is_empty() {
        app.handle_background_events();
        assert!(app.audit_jobs.completed >= previous);
        previous = app.audit_jobs.completed;
    }
    assert_eq!(previous, 1000);
    release_tx.send(()).unwrap();
    wait_for_audit(&mut app);
}

#[test]
fn export_without_results_scans_off_thread_and_normal_quit_waits() {
    let mut app = create_test_app();
    std::fs::write(app.audit.root_dir.join("sample.rs"), "pub fn sample() {}\n").unwrap();
    let destination = app.audit.root_dir.join("report.md");
    assert_eq!(
        app.export_audit_report(Some(&destination)).unwrap(),
        destination
    );
    app.request_quit();
    assert!(!app.should_quit);
    wait_for_audit(&mut app);
    assert!(app.should_quit);
    let content = std::fs::read_to_string(destination).unwrap();
    assert!(content.contains("1 source files"));
}

#[test]
fn failed_export_reports_error_and_cancels_deferred_quit() {
    let mut app = create_test_app();
    app.audit.has_result = true;
    let directory = app.audit.root_dir.clone();
    app.export_audit_report(Some(&directory)).unwrap();
    app.request_quit();
    wait_for_audit(&mut app);
    assert!(!app.should_quit);
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("export failed"));
}

#[test]
fn risk_scan_updates_diff_findings_without_replacing_audit_results() {
    let mut app = create_test_app();
    seed(&mut app);
    app.snapshot.files = output("risk").files;
    app.execute_command("risk");
    assert_eq!(app.active_view, ActiveView::Diff);
    wait_for_audit(&mut app);
    assert_eq!(app.audit.files[0].display_name(), "previous.rs");
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .starts_with("Checking risks complete"));
}
