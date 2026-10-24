//! One terminal reader, bounded batches, and draws only after activity.
use crate::{app::App, diagnostics};
use crossterm::event::{self, Event, KeyEventKind};
use std::time::{Duration, Instant};

// Keep the terminal reader on this thread: an external editor temporarily owns stdin.
// Polling bounds background-result latency without a reader stealing editor keystrokes.
const BACKGROUND_POLL: Duration = Duration::from_millis(16);
const BATCH_TIME: Duration = Duration::from_millis(4);
const INPUT_BATCH: usize = 32;

pub(crate) struct Budget {
    remaining: usize,
    started: Instant,
    first: bool,
}

impl Budget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            remaining: limit,
            started: Instant::now(),
            first: true,
        }
    }
    pub(crate) fn take(&mut self) -> bool {
        if self.remaining == 0 || (!self.first && self.started.elapsed() >= BATCH_TIME) {
            return false;
        }
        self.first = false;
        self.remaining -= 1;
        true
    }
}

pub(crate) trait Input {
    fn poll(&mut self, wait: Duration) -> std::io::Result<bool>;
    fn read(&mut self) -> std::io::Result<Event>;
}

pub(crate) struct TerminalInput;
impl Input for TerminalInput {
    fn poll(&mut self, wait: Duration) -> std::io::Result<bool> {
        event::poll(wait)
    }
    fn read(&mut self) -> std::io::Result<Event> {
        event::read()
    }
}

pub(crate) struct UiLoop {
    dirty: bool,
    fallback_deadline: Option<Instant>,
}
impl Default for UiLoop {
    fn default() -> Self {
        Self {
            dirty: true,
            fallback_deadline: None,
        }
    }
}

impl UiLoop {
    pub(crate) fn enable_polling_fallback(&mut self) {
        self.fallback_deadline = Some(Instant::now() + Duration::from_secs(1));
    }

    pub(crate) fn step(
        &mut self,
        app: &mut App,
        input: &mut impl Input,
        diagnostics: &mut Option<diagnostics::UiLoop>,
        mut draw: impl FnMut(&mut App) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let background = app.handle_background_events();
        self.dirty |= background;
        // Preserve external gate/turn status updates when the native watcher is unavailable.
        if self
            .fallback_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.dirty = true;
            app.invalidate_view_data();
            self.enable_polling_fallback();
        }
        let mut wait = if self.dirty {
            Duration::ZERO
        } else {
            BACKGROUND_POLL
        };
        let mut budget = Budget::new(INPUT_BATCH);
        while budget.take() && input.poll(wait)? {
            let event = input.read()?;
            if let Some(metrics) = diagnostics {
                metrics.event();
            }
            match event {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    if let Some(metrics) = diagnostics {
                        metrics.input();
                    }
                    app.handle_key(key);
                    self.dirty = true;
                }
                Event::Resize(..) => self.dirty = true,
                Event::FocusGained => {
                    app.invalidate_view_data();
                    self.dirty = true;
                }
                _ => {}
            }
            if app.should_quit {
                break;
            }
            wait = Duration::ZERO;
        }
        if self.dirty && !app.should_quit {
            app.prepare_view_data();
            draw(app)?;
            if let Some(metrics) = diagnostics {
                metrics.drawn(background);
            }
            self.dirty = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
