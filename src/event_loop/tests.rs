use super::*;
use crate::app::{
    test_util::{create_test_app as create_base_app, wait_for_view_data},
    ActiveView,
};
fn create_test_app() -> App {
    let mut app = create_base_app();
    wait_for_view_data(&mut app);
    app
}
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;

#[derive(Default)]
struct FakeInput {
    events: VecDeque<Event>,
    waits: Vec<Duration>,
    reads: usize,
}
impl Input for FakeInput {
    fn poll(&mut self, wait: Duration) -> std::io::Result<bool> {
        self.waits.push(wait);
        Ok(!self.events.is_empty())
    }
    fn read(&mut self) -> std::io::Result<Event> {
        self.reads += 1;
        Ok(self.events.pop_front().unwrap())
    }
}
fn key(kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char('x'),
        KeyModifiers::NONE,
        kind,
    ))
}
fn step(state: &mut UiLoop, app: &mut App, input: &mut FakeInput, draws: &mut usize) {
    state
        .step(app, input, &mut None, |_| {
            *draws += 1;
            Ok(())
        })
        .unwrap();
}

#[test]
fn startup_draws_once_and_idle_does_not_redraw() {
    let mut app = create_test_app();
    let mut state = UiLoop::default();
    let mut input = FakeInput::default();
    let mut draws = 0;
    for _ in 0..100 {
        step(&mut state, &mut app, &mut input, &mut draws);
    }
    assert_eq!(draws, 1);
    assert_eq!(input.waits[0], Duration::ZERO);
    assert!(input.waits[1..].iter().all(|&wait| wait == BACKGROUND_POLL));
}

#[test]
fn held_keys_are_bounded_preserved_and_draw_between_batches() {
    let mut app = create_test_app();
    app.active_view = ActiveView::CommandLine;
    let mut state = UiLoop::default();
    let mut input = FakeInput {
        events: (0..100).map(|_| key(KeyEventKind::Repeat)).collect(),
        ..Default::default()
    };
    let mut draws = 0;
    step(&mut state, &mut app, &mut input, &mut draws);
    assert!((1..=INPUT_BATCH).contains(&input.reads));
    assert_eq!(draws, 1);
    assert!(!input.events.is_empty());
    while !input.events.is_empty() {
        let previous = input.reads;
        let previous_draws = draws;
        step(&mut state, &mut app, &mut input, &mut draws);
        assert!((1..=INPUT_BATCH).contains(&(input.reads - previous)));
        assert_eq!(draws, previous_draws + 1);
    }
    assert_eq!(app.command.input, "x".repeat(100));
}

#[test]
fn resize_and_focus_redraw_while_key_release_does_not() {
    let mut app = create_test_app();
    let mut state = UiLoop::default();
    let mut input = FakeInput::default();
    let mut draws = 0;
    step(&mut state, &mut app, &mut input, &mut draws);
    input.events.push_back(key(KeyEventKind::Release));
    step(&mut state, &mut app, &mut input, &mut draws);
    assert_eq!(draws, 1);
    for event in [Event::Resize(90, 30), Event::FocusGained] {
        input.events.push_back(event);
        step(&mut state, &mut app, &mut input, &mut draws);
    }
    assert_eq!(draws, 3);
}

#[test]
fn quit_does_not_draw_or_consume_later_keys() {
    let mut app = create_test_app();
    let mut input = FakeInput {
        events: [
            Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            key(KeyEventKind::Press),
        ]
        .into(),
        ..Default::default()
    };
    let mut draws = 0;
    step(&mut UiLoop::default(), &mut app, &mut input, &mut draws);
    assert!(app.should_quit);
    assert_eq!(input.events.len(), 1);
    assert_eq!(draws, 0);
}

#[test]
fn failed_draw_remains_dirty_for_retry() {
    let mut app = create_test_app();
    let mut state = UiLoop::default();
    let mut input = FakeInput::default();
    assert!(state
        .step(&mut app, &mut input, &mut None, |_| anyhow::bail!(
            "draw failed"
        ))
        .is_err());
    let mut draws = 0;
    step(&mut state, &mut app, &mut input, &mut draws);
    assert_eq!(draws, 1);
}

#[test]
fn missing_watcher_refreshes_status_only_when_fallback_is_due() {
    let mut app = create_test_app();
    let mut state = UiLoop::default();
    state.enable_polling_fallback();
    let mut input = FakeInput::default();
    let mut draws = 0;
    step(&mut state, &mut app, &mut input, &mut draws);
    step(&mut state, &mut app, &mut input, &mut draws);
    assert_eq!(draws, 1);
    state.fallback_deadline = Some(Instant::now());
    step(&mut state, &mut app, &mut input, &mut draws);
    assert_eq!(draws, 2);
    wait_for_view_data(&mut app);
    step(&mut state, &mut app, &mut input, &mut draws);
    assert_eq!(draws, 2);
}

#[test]
fn enabled_diagnostics_do_not_change_input_or_draw_behavior() {
    let mut app = create_test_app();
    app.active_view = ActiveView::CommandLine;
    let mut state = UiLoop::default();
    let mut metrics = Some(diagnostics::UiLoop::default());
    let mut input = FakeInput {
        events: VecDeque::from([
            key(KeyEventKind::Press),
            Event::Resize(80, 24),
            key(KeyEventKind::Release),
        ]),
        ..Default::default()
    };
    let mut draws = 0;
    for _ in 0..2 {
        state
            .step(&mut app, &mut input, &mut metrics, |_| {
                draws += 1;
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(input.reads, 3);
    assert_eq!(draws, 1, "all three events fit one batch and one draw");
    assert_eq!(
        app.command.input, "x",
        "the press was typed once; the release ignored"
    );
}
