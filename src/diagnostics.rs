//! UI loop accounting is constructed only when diagnostics are enabled.
use lgtm_core::debug::Span;

#[derive(Default)]
pub(crate) struct UiLoop {
    frames: u64,
    idle_frames: u64,
    events: u64,
    inputs: u64,
    input_to_draw: Option<Span>,
}

impl UiLoop {
    pub(crate) fn event(&mut self) {
        self.events += 1;
    }

    pub(crate) fn input(&mut self) {
        self.inputs += 1;
        if self.input_to_draw.is_none() {
            let mut timing = Span::new("ui.input_to_draw");
            // Quitting before another draw leaves this at zero.
            timing.count("presented", 0);
            self.input_to_draw = Some(timing);
        }
    }

    pub(crate) fn drawn(&mut self, background_activity: bool) {
        let idle = self.frames > 0 && self.events == 0 && !background_activity;
        self.frames += 1;
        self.idle_frames += u64::from(idle);
        if let Some(mut input) = self.input_to_draw.take() {
            input.count("presented", 1);
            input.count("inputs", self.inputs);
        }
        let mut timing = Span::new("ui.frame");
        timing.count("frames_total", self.frames);
        timing.count("idle_frames_total", self.idle_frames);
        timing.count("idle", u64::from(idle));
        timing.count("events", self.events);
        timing.count("inputs", self.inputs);
        timing.count("background_activity", u64::from(background_activity));
        self.events = 0;
        self.inputs = 0;
    }
}

#[cfg(test)]
mod tests;
