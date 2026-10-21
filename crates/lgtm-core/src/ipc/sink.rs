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

pub struct ZellijSink<'a>(pub Option<&'a str>);

impl AgentSink for ZellijSink<'_> {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_to_zellij(self.0, text)
    }
    fn label(&self) -> String {
        match self.0 {
            Some(p) => format!("zellij pane {}", p),
            None => "zellij active pane".to_string(),
        }
    }
}

pub struct WezTermSink<'a>(pub Option<&'a str>);

impl AgentSink for WezTermSink<'_> {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_to_wezterm(self.0, text)
    }
    fn label(&self) -> String {
        match self.0 {
            Some(p) => format!("wezterm pane {}", p),
            None => "wezterm active pane".to_string(),
        }
    }
}

pub struct KittySink<'a>(pub Option<&'a str>);

impl AgentSink for KittySink<'_> {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_to_kitty(self.0, text)
    }
    fn label(&self) -> String {
        match self.0 {
            Some(w) => format!("kitty window {}", w),
            None => "kitty active window".to_string(),
        }
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

pub struct DummySink;

impl AgentSink for DummySink {
    fn send(&self, _text: &str) -> Result<()> {
        Ok(())
    }
    fn label(&self) -> String {
        "dummy (/dev/null)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_target() {
        assert_eq!(TmuxSink("%3").label(), "pane %3");
        assert_eq!(ZellijSink(Some("1")).label(), "zellij pane 1");
        assert_eq!(ZellijSink(None).label(), "zellij active pane");
        assert_eq!(WezTermSink(Some("2")).label(), "wezterm pane 2");
        assert_eq!(WezTermSink(None).label(), "wezterm active pane");
        assert_eq!(KittySink(Some("3")).label(), "kitty window 3");
        assert_eq!(KittySink(None).label(), "kitty active window");
        assert_eq!(ClipboardSink.label(), "clipboard (OSC 52)");
        assert_eq!(DummySink.label(), "dummy (/dev/null)");
    }
}
