use super::IpcBridge;
use anyhow::Result;

pub trait AgentSink {
    fn send(&self, text: &str) -> Result<()>;
    fn label(&self) -> String;
}

pub struct TmuxSink<'a>(pub &'a str);

impl AgentSink for TmuxSink<'_> {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_to_tmux(self.0, text)
    }
    fn label(&self) -> String {
        format!("pane {}", self.0)
    }
}

pub struct ClipboardSink;

impl AgentSink for ClipboardSink {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_osc52_clipboard(text);
        Ok(())
    }
    fn label(&self) -> String {
        "clipboard (OSC 52)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_target() {
        assert_eq!(TmuxSink("%3").label(), "pane %3");
        assert_eq!(ClipboardSink.label(), "clipboard (OSC 52)");
    }
}
