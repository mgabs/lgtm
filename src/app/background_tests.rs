use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn queue(app: &App, count: usize, applied: &Arc<AtomicUsize>) {
    for _ in 0..count {
        let applied = applied.clone();
        app.jobs_tx
            .send(Box::new(move |_| {
                applied.fetch_add(1, Ordering::SeqCst);
            }))
            .unwrap();
    }
}

#[test]
fn callback_and_watcher_floods_each_make_bounded_progress() {
    let mut app = test_util::create_test_app();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    for _ in 0..100 {
        tx.send(WatcherMessage::Error("fixture".into())).unwrap();
    }
    // Remote reviews do not need local refreshes for unrelated watcher traffic.
    app.pr.current_number = Some(1);
    let applied = Arc::new(AtomicUsize::new(0));
    queue(&app, 40, &applied);
    assert!(app.handle_background_events());
    assert!((1..=8).contains(&applied.load(Ordering::SeqCst)));
    assert!((68..100).contains(&app.watcher_rx.len()));
    while !app.jobs_rx.is_empty() || !app.watcher_rx.is_empty() {
        app.handle_background_events();
    }
    assert_eq!(applied.load(Ordering::SeqCst), 40);
    assert!(!app.handle_background_events());
}

#[test]
fn a_slow_callback_yields_before_the_next_callback() {
    let mut app = test_util::create_test_app();
    app.jobs_tx
        .send(Box::new(|_| {
            std::thread::sleep(std::time::Duration::from_millis(6))
        }))
        .unwrap();
    let applied = Arc::new(AtomicUsize::new(0));
    queue(&app, 1, &applied);
    app.handle_background_events();
    assert_eq!(applied.load(Ordering::SeqCst), 0);
    app.handle_background_events();
    assert_eq!(applied.load(Ordering::SeqCst), 1);
}

struct Keys;
impl crate::event_loop::Input for Keys {
    fn poll(&mut self, _: std::time::Duration) -> std::io::Result<bool> {
        Ok(true)
    }
    fn read(&mut self) -> std::io::Result<crossterm::event::Event> {
        Ok(crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        )))
    }
}

#[test]
fn continuous_input_and_callbacks_both_progress_before_each_draw() {
    let mut app = test_util::create_test_app();
    app.active_view = ActiveView::CommandLine;
    let applied = Arc::new(AtomicUsize::new(0));
    queue(&app, 40, &applied);
    let mut state = crate::event_loop::UiLoop::default();
    let mut previous = 0;
    for _ in 0..5 {
        state
            .step(&mut app, &mut Keys, &mut None, |app| {
                assert!(applied.load(Ordering::SeqCst) > previous);
                assert!(!app.command.input.is_empty());
                Ok(())
            })
            .unwrap();
        previous = applied.load(Ordering::SeqCst);
    }
}

#[test]
fn internal_logs_do_not_redraw_but_external_gate_and_turn_changes_do() {
    let mut app = test_util::create_test_app();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
    for (file, changed) in [
        ("debug.jsonl", false),
        ("gate.json", true),
        ("state.json", true),
        ("prompt.txt", true),
    ] {
        tx.send(WatcherMessage::FilesChanged(vec![app
            .git_engine
            .repo_path
            .join(".lgtm")
            .join(file)]))
            .unwrap();
        assert_eq!(app.handle_background_events(), changed, "{file}");
    }
}

#[test]
fn callbacks_see_durable_directives_even_when_notification_is_beyond_budget() {
    let mut app = test_util::create_test_app();
    app.pr.current_number = Some(1);
    let comment: CommentAnchor = serde_json::from_value(serde_json::json!({
        "id":"external", "file_path":"a.rs", "target_line_no":1,
        "target_code":"value", "context_before":[], "context_after":[],
        "comment_text":"newer external edit", "replies":[],
        "created_at_turn":1, "is_resolved":false,
    }))
    .unwrap();
    app.comment_store.save_comments(&[comment]).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    app.watcher_rx = rx;
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
        .join(".lgtm/directives.jsonl")]))
        .unwrap();
    app.jobs_tx
        .send(Box::new(|app| {
            assert_eq!(app.comments.len(), 1);
            assert_eq!(app.comments[0].comment_text, "newer external edit");
            app.comment_store.save_comments(&app.comments).unwrap();
        }))
        .unwrap();
    app.handle_background_events();
    assert!(!app.watcher_rx.is_empty());
    assert!(app.jobs_rx.is_empty());
    assert_eq!(
        app.comment_store.load_comments()[0].comment_text,
        "newer external edit"
    );
}
