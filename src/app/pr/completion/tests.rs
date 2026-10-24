use super::*;
use crate::app::test_util::create_test_app;
use std::time::{Duration, Instant};

fn snapshot() -> DiffSnapshot {
    crate::pr::PrEngine::parse_unified_diff("diff --git a/a.rs b/a.rs\n--- /dev/null\n+++ b/a.rs\n@@ -0,0 +1,4 @@\n+first\n+second\n+target\n+last\n").unwrap()
}
fn directive(pr: u64) -> CommentAnchor {
    serde_json::from_value(serde_json::json!({
        "id":lgtm_core::comment::new_id(),"file_path":"a.rs","target_line_no":1,
        "target_code":"target","context_before":[],"context_after":[],
        "comment_text":"original","created_at_turn":1,"is_resolved":false,"pr_number":pr,
        "track":{"hunk_hash":"earlier"}
    }))
    .unwrap()
}
fn setup() -> App {
    let mut app = create_test_app();
    app.pr.current_number = Some(7);
    app.review_generation = 1;
    app.pr_load_latest
        .store(1, std::sync::atomic::Ordering::Relaxed);
    app.pr_loading = Some((42, 1));
    app.comments = vec![directive(42), directive(7)];
    app
}
fn ready(app: &App) -> LoadedPr {
    let mut snapshot = snapshot();
    let input = Input::capture(app, 42);
    let completion = prepare(&app.git_engine.repo_path, 42, &mut snapshot, input, || true).unwrap();
    LoadedPr {
        pr: 42,
        generation: app.review_generation,
        repo: app.git_engine.repo_path.clone(),
        snapshot,
        findings: Default::default(),
        completion,
    }
}
fn wait(app: &mut App, predicate: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate(app) {
        app.handle_background_events();
        assert!(Instant::now() < deadline, "PR completion did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn edits_and_post_receipts_during_loading_survive_without_refetch() {
    let mut app = setup();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let (entered_tx, entered_rx) = crossbeam_channel::bounded(1);
    let (release_tx, release_rx) = crossbeam_channel::bounded(1);
    app.load_pr_job(42, move || {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(snapshot())
    });
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    app.comments[0].comment_text = "newer edit".into();
    app.comments[0].add_reply("newer reply");
    let id = app.comments[0].id.clone();
    app.save_post_receipts(&[(id, 99)], &[]).unwrap();
    app.comments[1].comment_text = "unrelated newer edit".into();
    app.comments.push(directive(42));
    app.comments.swap(0, 2);
    release_tx.send(()).unwrap();
    wait(&mut app, |app| app.pr_loading.is_none());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(app.comments[2].comment_text, "newer edit");
    assert_eq!(app.comments[2].remote_id, Some(99));
    assert_eq!(app.comments[2].replies, ["newer reply"]);
    assert_eq!(app.comments[1].comment_text, "unrelated newer edit");
    assert_eq!(app.comments[0].target_line_no, 3);
    assert_eq!(app.comments[2].target_line_no, 3);
    assert_eq!(app.comment_store.load_comments(), app.comments);
}

#[test]
fn preparation_runs_off_thread_and_does_not_replace_an_open_draft() {
    let mut app = setup();
    let input = Input::capture(&app, 42);
    let repo = app.git_engine.repo_path.clone();
    let worker_repo = repo.clone();
    let ui = std::thread::current().id();
    let (entered_tx, entered_rx) = crossbeam_channel::bounded(1);
    let (release_tx, release_rx) = crossbeam_channel::bounded(1);
    app.spawn_silent_job(
        move || {
            assert_ne!(std::thread::current().id(), ui);
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            let mut snapshot = snapshot();
            let completion = prepare(&worker_repo, 42, &mut snapshot, input, || true).unwrap();
            LoadedPr {
                pr: 42,
                generation: 1,
                snapshot,
                findings: Default::default(),
                repo: worker_repo,
                completion,
            }
        },
        |app, ready| {
            app.ready_pr = Some(ready);
            app.apply_ready_pr();
        },
    );
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    app.active_view = ActiveView::CommentInput;
    app.comment_input = "draft".into();
    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    release_tx.send(()).unwrap();
    wait(&mut app, |app| app.ready_pr.is_some());
    assert_eq!(app.pr.current_number, Some(7));
    assert_eq!(app.comment_input, "draftx");
    app.active_view = ActiveView::Diff;
    app.apply_ready_pr();
    assert_eq!(app.pr.current_number, Some(42));
    assert_eq!(app.comment_input, "draftx");
}

#[test]
fn removed_directives_are_not_resurrected_by_prepared_results() {
    let mut app = setup();
    app.ready_pr = Some(ready(&app));
    app.comments.remove(0);
    app.apply_ready_pr();
    assert_eq!(app.pr.current_number, Some(7));
    wait(&mut app, |app| app.pr_loading.is_none());
    assert_eq!(app.comments.len(), 1);
    assert_eq!(app.comments[0].pr_number, Some(7));
}

#[test]
fn edits_for_other_reviews_do_not_require_new_preparation() {
    let mut app = setup();
    app.ready_pr = Some(ready(&app));
    app.comments[1].comment_text = "new unrelated draft".into();
    app.apply_ready_pr();
    assert!(app.pr_loading.is_none());
    assert_eq!(app.pr.current_number, Some(42));
    assert_eq!(app.comments[1].comment_text, "new unrelated draft");
}

#[test]
fn visual_selection_defers_application_without_replacing_the_review() {
    let mut app = setup();
    app.ready_pr = Some(ready(&app));
    app.visual_mode = true;
    app.visual_anchor_idx = Some(2);
    app.apply_ready_pr();
    assert_eq!(app.pr.current_number, Some(7));
    assert!(app.ready_pr.is_some());
    assert_eq!(app.visual_anchor_idx, Some(2));
    app.visual_mode = false;
    app.apply_ready_pr();
    assert_eq!(app.pr.current_number, Some(42));
}

#[test]
fn mark_changes_after_preparation_are_reloaded_and_removed_folds_clear() {
    let mut app = setup();
    let initial = snapshot();
    let hash = lgtm_core::review::hunk_hash(&initial.files[0].hunks[0]);
    let store = lgtm_core::review::ReviewStore::open_marks_only(&app.git_engine.repo_path, "pr-42")
        .unwrap();
    store
        .save(&lgtm_core::review::ReviewState {
            hunks: vec![lgtm_core::review::HunkMark {
                file: "a.rs".into(),
                content_hash: hash.clone(),
                last_line: 1,
                reviewed_at: "now".into(),
            }],
            folds: vec![lgtm_core::review::FoldMark {
                file: "a.rs".into(),
                hunk_hash: hash,
                start_line: 2,
                end_line: 3,
            }],
            ..Default::default()
        })
        .unwrap();
    let prepared = ready(&app);
    assert!(prepared.snapshot.files[0].is_reviewed);
    assert_eq!(prepared.snapshot.files[0].hunks[0].folded_blocks.len(), 1);
    app.ready_pr = Some(prepared);
    store.save(&Default::default()).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    app.active_view = ActiveView::CommentInput;
    tx.send(WatcherMessage::FilesChanged(vec![app
        .git_engine
        .repo_path
        .join(".lgtm/reviews/pr-42.json")]))
        .unwrap();
    app.handle_background_events();
    app.active_view = ActiveView::Diff;
    app.apply_ready_pr();
    wait(&mut app, |app| app.pr_loading.is_none());
    assert!(app.marks.is_empty());
    assert!(!app.snapshot.files[0].is_reviewed);
    assert!(app.snapshot.files[0].hunks[0].folded_blocks.is_empty());
}

#[test]
fn changed_turn_rechecks_outcomes_before_application() {
    let mut app = setup();
    app.comments[0].created_at_turn = 1;
    app.ready_pr = Some(ready(&app));
    let path = app.git_engine.repo_path.join(".lgtm/state.json");
    std::fs::write(
        &path,
        r#"{"current_turn":2,"last_reviewed_tree_oid":null,"turn_timestamps":[]}"#,
    )
    .unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    app.active_view = ActiveView::CommentInput;
    tx.send(WatcherMessage::FilesChanged(vec![path])).unwrap();
    app.handle_background_events();
    app.active_view = ActiveView::Diff;
    app.apply_ready_pr();
    wait(&mut app, |app| app.pr_loading.is_none());
    assert_eq!(
        app.comments[0].track.outcome,
        Some(lgtm_core::outcome::Outcome::Changed)
    );
}

#[test]
fn repository_or_generation_changes_reject_prepared_results() {
    for change_repo in [false, true] {
        let mut app = setup();
        app.ready_pr = Some(ready(&app));
        if change_repo {
            app.git_engine.repo_path = app.git_engine.repo_path.join("other");
        } else {
            app.review_generation += 1;
        }
        app.apply_ready_pr();
        assert_eq!(app.pr.current_number, Some(7));
        assert!(app.ready_pr.is_none());
    }
}

#[test]
fn cancelled_preparation_does_not_persist_derived_directives() {
    let app = setup();
    app.comment_store.save_comments(&app.comments).unwrap();
    let result = prepare(
        &app.git_engine.repo_path,
        42,
        &mut snapshot(),
        Input::capture(&app, 42),
        || false,
    );
    assert!(result.is_err());
    assert_eq!(app.comment_store.load_comments(), app.comments);
}

#[test]
fn failed_completion_save_is_visible_and_posting_does_not_start() {
    let mut app = setup();
    app.ready_pr = Some(ready(&app));
    std::fs::create_dir(app.git_engine.repo_path.join(".lgtm/comments.jsonl")).unwrap();
    app.apply_ready_pr();
    assert_eq!(app.pr.current_number, Some(42));
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("could not be saved"));
    app.post_pr_comments_job(true, || panic!("posting started before durable save"));
    assert!(app.posting_prs.is_empty());
    assert!(app.status_message.as_ref().unwrap().contains("not posted"));
}
