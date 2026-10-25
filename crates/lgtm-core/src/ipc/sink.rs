use super::IpcBridge;
use anyhow::Result;
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSinkKind<'a> {
    Tmux(&'a str),
    Zellij(Option<&'a str>),
    WezTerm(Option<&'a str>),
    Kitty(Option<&'a str>),
    Osc52,
    Direct,
}

pub type AgentSink<'a> = AgentSinkKind<'a>;

impl<'a> AgentSinkKind<'a> {
    pub const CLIPBOARD: Self = Self::Osc52;
    pub const DUMMY: Self = Self::Direct;

    #[allow(non_upper_case_globals)]
    pub const Clipboard: Self = Self::Osc52;
    #[allow(non_upper_case_globals)]
    pub const Dummy: Self = Self::Direct;

    pub fn send(&self, text: &str) -> Result<()> {
        match *self {
            Self::Tmux(pane) => IpcBridge::send_to_tmux(pane, text),
            Self::Zellij(pane) => IpcBridge::send_to_zellij(pane, text),
            Self::WezTerm(pane) => IpcBridge::send_to_wezterm(pane, text),
            Self::Kitty(pane) => IpcBridge::send_to_kitty(pane, text),
            Self::Osc52 => {
                IpcBridge::send_osc52_clipboard(text);
                Ok(())
            }
            Self::Direct => Ok(()),
        }
    }

    pub fn label(&self) -> Cow<'a, str> {
        match *self {
            Self::Tmux(pane) => Cow::Owned(format!("pane {}", pane)),
            Self::Zellij(Some(p)) => Cow::Owned(format!("zellij pane {}", p)),
            Self::Zellij(None) => Cow::Borrowed("zellij active pane"),
            Self::WezTerm(Some(p)) => Cow::Owned(format!("wezterm pane {}", p)),
            Self::WezTerm(None) => Cow::Borrowed("wezterm active pane"),
            Self::Kitty(Some(w)) => Cow::Owned(format!("kitty window {}", w)),
            Self::Kitty(None) => Cow::Borrowed("kitty active window"),
            Self::Osc52 => Cow::Borrowed("clipboard (OSC 52)"),
            Self::Direct => Cow::Borrowed("dummy (/dev/null)"),
        }
    }
}

impl<'a> fmt::Display for AgentSinkKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Tmux(pane) => write!(f, "pane {}", pane),
            Self::Zellij(Some(p)) => write!(f, "zellij pane {}", p),
            Self::Zellij(None) => write!(f, "zellij active pane"),
            Self::WezTerm(Some(p)) => write!(f, "wezterm pane {}", p),
            Self::WezTerm(None) => write!(f, "wezterm active pane"),
            Self::Kitty(Some(w)) => write!(f, "kitty window {}", w),
            Self::Kitty(None) => write!(f, "kitty active window"),
            Self::Osc52 => write!(f, "clipboard (OSC 52)"),
            Self::Direct => write!(f, "dummy (/dev/null)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_target() {
        assert_eq!(AgentSinkKind::Tmux("%3").label(), "pane %3");
        assert_eq!(AgentSinkKind::Zellij(Some("1")).label(), "zellij pane 1");
        assert_eq!(AgentSinkKind::Zellij(None).label(), "zellij active pane");
        assert_eq!(AgentSinkKind::WezTerm(Some("2")).label(), "wezterm pane 2");
        assert_eq!(AgentSinkKind::WezTerm(None).label(), "wezterm active pane");
        assert_eq!(AgentSinkKind::Kitty(Some("3")).label(), "kitty window 3");
        assert_eq!(AgentSinkKind::Kitty(None).label(), "kitty active window");
        assert_eq!(AgentSinkKind::Osc52.label(), "clipboard (OSC 52)");
        assert_eq!(AgentSinkKind::Direct.label(), "dummy (/dev/null)");
        assert_eq!(AgentSinkKind::Clipboard.label(), "clipboard (OSC 52)");
        assert_eq!(AgentSinkKind::Dummy.label(), "dummy (/dev/null)");
    }

    #[test]
    fn display_matches_label() {
        let sinks = [
            AgentSinkKind::Tmux("%3"),
            AgentSinkKind::Zellij(Some("1")),
            AgentSinkKind::Zellij(None),
            AgentSinkKind::WezTerm(Some("2")),
            AgentSinkKind::WezTerm(None),
            AgentSinkKind::Kitty(Some("3")),
            AgentSinkKind::Kitty(None),
            AgentSinkKind::Osc52,
            AgentSinkKind::Direct,
        ];
        for s in sinks {
            assert_eq!(s.to_string(), s.label());
        }
    }

    #[test]
    fn dummy_send_succeeds() {
        assert!(AgentSinkKind::Direct.send("payload").is_ok());
    }
}
