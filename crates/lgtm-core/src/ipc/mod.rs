pub mod sink;

use anyhow::{bail, Context, Result};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SinkKind {
    #[default]
    Auto,
    Tmux,
    Zellij,
    WezTerm,
    Kitty,
    Clipboard,
    Dummy,
}

impl std::str::FromStr for SinkKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "tmux" => Ok(SinkKind::Tmux),
            "zellij" => Ok(SinkKind::Zellij),
            "wezterm" => Ok(SinkKind::WezTerm),
            "kitty" => Ok(SinkKind::Kitty),
            "clipboard" | "clip" => Ok(SinkKind::Clipboard),
            "dummy" | "null" | "/dev/null" | "none" => Ok(SinkKind::Dummy),
            "auto" => Ok(SinkKind::Auto),
            other => bail!(
                "unknown agent sink `{}` (expected tmux, zellij, wezterm, kitty, clipboard, or dummy)",
                other
            ),
        }
    }
}

impl std::fmt::Display for SinkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SinkKind::Auto => write!(f, "auto"),
            SinkKind::Tmux => write!(f, "tmux"),
            SinkKind::Zellij => write!(f, "zellij"),
            SinkKind::WezTerm => write!(f, "wezterm"),
            SinkKind::Kitty => write!(f, "kitty"),
            SinkKind::Clipboard => write!(f, "clipboard"),
            SinkKind::Dummy => write!(f, "dummy"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxPane {
    pub id: String,
    pub title: String,
    pub active: bool,
}

pub struct IpcBridge;

impl IpcBridge {
    pub fn is_inside_tmux() -> bool {
        std::env::var("TMUX").is_ok()
    }

    pub fn is_inside_zellij() -> bool {
        std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok()
    }

    pub fn is_inside_wezterm() -> bool {
        std::env::var("WEZTERM_PANE").is_ok()
    }

    pub fn is_inside_kitty() -> bool {
        std::env::var("KITTY_WINDOW_ID").is_ok() || std::env::var("KITTY_PID").is_ok()
    }

    pub fn detect_sink() -> SinkKind {
        if Self::is_inside_tmux() {
            SinkKind::Tmux
        } else if Self::is_inside_zellij() {
            SinkKind::Zellij
        } else if Self::is_inside_wezterm() {
            SinkKind::WezTerm
        } else if Self::is_inside_kitty() {
            SinkKind::Kitty
        } else {
            SinkKind::Clipboard
        }
    }

    pub fn list_tmux_panes() -> Result<Vec<TmuxPane>> {
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-F",
                "#{pane_id}:#{pane_title}:#{pane_active}",
            ])
            .output()
            .context("Failed to execute tmux list-panes")?;

        if !output.status.success() {
            bail!("tmux list-panes returned non-zero status");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut panes = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                panes.push(TmuxPane {
                    id: parts[0].to_string(),
                    title: parts[1].to_string(),
                    active: parts[2] == "1",
                });
            }
        }

        Ok(panes)
    }

    pub fn detect_agent_pane() -> Option<String> {
        Self::detect_agent_pane_for(Self::detect_sink())
    }

    pub fn detect_agent_pane_for(sink: SinkKind) -> Option<String> {
        let panes = Self::list_panes(sink).ok()?;
        // Prefer non-active pane in the current window/session
        panes.into_iter().find(|p| !p.active).map(|p| p.id)
    }

    pub fn list_panes(sink: SinkKind) -> Result<Vec<TmuxPane>> {
        match sink {
            SinkKind::Tmux => Self::list_tmux_panes(),
            SinkKind::Zellij => Self::list_zellij_panes(),
            SinkKind::WezTerm => Self::list_wezterm_panes(),
            SinkKind::Kitty => Self::list_kitty_windows(),
            SinkKind::Auto => {
                let detected = Self::detect_sink();
                if detected != SinkKind::Clipboard && detected != SinkKind::Auto {
                    Self::list_panes(detected)
                } else {
                    Ok(Vec::new())
                }
            }
            SinkKind::Clipboard | SinkKind::Dummy => Ok(Vec::new()),
        }
    }

    pub fn list_wezterm_panes() -> Result<Vec<TmuxPane>> {
        let output = Command::new("wezterm")
            .args(["cli", "list", "--format", "json"])
            .output()
            .context("Failed to execute wezterm cli list")?;

        if !output.status.success() {
            bail!("wezterm cli list returned non-zero status");
        }

        Self::parse_wezterm_panes_json(&String::from_utf8_lossy(&output.stdout))
    }

    pub fn parse_wezterm_panes_json(json_str: &str) -> Result<Vec<TmuxPane>> {
        let val: serde_json::Value =
            serde_json::from_str(json_str).context("Failed to parse wezterm cli list JSON")?;
        let mut panes = Vec::new();
        if let Some(arr) = val.as_array() {
            for item in arr {
                let id = item
                    .get("pane_id")
                    .and_then(|v| {
                        v.as_i64()
                            .map(|n| n.to_string())
                            .or_else(|| v.as_str().map(|s| s.to_string()))
                    })
                    .unwrap_or_default();
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pane")
                    .to_string();
                let active = item
                    .get("is_active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !id.is_empty() {
                    panes.push(TmuxPane { id, title, active });
                }
            }
        }
        Ok(panes)
    }

    pub fn list_kitty_windows() -> Result<Vec<TmuxPane>> {
        let output = Command::new("kitty")
            .args(["@", "ls"])
            .output()
            .context("Failed to execute kitty @ ls")?;

        if !output.status.success() {
            bail!("kitty @ ls returned non-zero status");
        }

        Self::parse_kitty_windows_json(&String::from_utf8_lossy(&output.stdout))
    }

    pub fn parse_kitty_windows_json(json_str: &str) -> Result<Vec<TmuxPane>> {
        let val: serde_json::Value =
            serde_json::from_str(json_str).context("Failed to parse kitty @ ls JSON")?;
        let mut windows = Vec::new();
        if let Some(os_windows) = val.as_array() {
            for os_win in os_windows {
                if let Some(tabs) = os_win.get("tabs").and_then(|t| t.as_array()) {
                    for tab in tabs {
                        if let Some(wins) = tab.get("windows").and_then(|w| w.as_array()) {
                            for win in wins {
                                let id = win
                                    .get("id")
                                    .and_then(|v| {
                                        v.as_i64()
                                            .map(|n| n.to_string())
                                            .or_else(|| v.as_str().map(|s| s.to_string()))
                                    })
                                    .unwrap_or_default();
                                let title = win
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("window")
                                    .to_string();
                                let active = win
                                    .get("is_focused")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if !id.is_empty() {
                                    windows.push(TmuxPane { id, title, active });
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(windows)
    }

    pub fn list_zellij_panes() -> Result<Vec<TmuxPane>> {
        let output = Command::new("zellij")
            .args(["action", "query-tab-names"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut panes = Vec::new();
                for (idx, line) in stdout.lines().enumerate() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        panes.push(TmuxPane {
                            id: (idx + 1).to_string(),
                            title: trimmed.to_string(),
                            active: idx == 0,
                        });
                    }
                }
                if !panes.is_empty() {
                    return Ok(panes);
                }
            }
        }

        Ok(vec![TmuxPane {
            id: "current".to_string(),
            title: "active pane".to_string(),
            active: true,
        }])
    }

    pub fn send_to_tmux(pane_id: &str, text: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["send-keys", "-t", pane_id, "-l", text])
            .status()
            .context("Failed to run tmux send-keys")?;

        if !status.success() {
            bail!("tmux send-keys failed for pane {}", pane_id);
        }

        Ok(())
    }

    pub fn send_to_zellij(pane_id: Option<&str>, text: &str) -> Result<()> {
        let mut cmd = Command::new("zellij");
        cmd.arg("action").arg("write-chars");
        if let Some(pid) = pane_id {
            cmd.args(["--pane-id", pid]);
        }
        cmd.arg(text);
        let status = cmd
            .status()
            .context("Failed to run zellij action write-chars")?;
        if !status.success() {
            bail!("zellij write-chars failed");
        }
        Ok(())
    }

    pub fn send_to_wezterm(pane_id: Option<&str>, text: &str) -> Result<()> {
        let mut cmd = Command::new("wezterm");
        cmd.args(["cli", "send-text"]);
        if let Some(pid) = pane_id {
            cmd.args(["--pane-id", pid]);
        }
        cmd.args(["--no-paste", text]);
        let status = cmd
            .status()
            .context("Failed to run wezterm cli send-text")?;
        if !status.success() {
            bail!("wezterm send-text failed");
        }
        Ok(())
    }

    pub fn send_to_kitty(window_id: Option<&str>, text: &str) -> Result<()> {
        let mut cmd = Command::new("kitty");
        cmd.args(["@", "send-text"]);
        if let Some(wid) = window_id {
            cmd.args(["--match", &format!("id:{}", wid)]);
        }
        cmd.arg(text);
        let status = cmd.status().context("Failed to run kitty @ send-text")?;
        if !status.success() {
            bail!("kitty send-text failed");
        }
        Ok(())
    }

    pub fn send_osc52_clipboard(text: &str) {
        use std::io::Write;
        let b64 = base64_encode(text.as_bytes());
        let osc_sequence = format!("\x1b]52;c;{}\x07", b64);
        let _ = std::io::stdout().write_all(osc_sequence.as_bytes());
        let _ = std::io::stdout().flush();
    }
}

pub fn base64_encode(data: &[u8]) -> String {
    // Standard Base64 RFC 4648 charset table // lgtm:ignore
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let len = data.len();

    for i in (0..len).step_by(3) {
        let b0 = data[i];
        let b1 = if i + 1 < len { data[i + 1] } else { 0 };
        let b2 = if i + 2 < len { data[i + 2] } else { 0 };

        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < len {
            out.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < len {
            out.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"Hello, world!"), "SGVsbG8sIHdvcmxkIQ==");
    }

    #[test]
    fn test_is_inside_tmux_fallback() {
        // Just verify it doesn't crash
        let _ = IpcBridge::is_inside_tmux();
        let _ = IpcBridge::detect_agent_pane();
    }

    #[test]
    fn test_send_osc52_clipboard() {
        // Verify send_osc52_clipboard executes cleanly
        IpcBridge::send_osc52_clipboard("test payload");
    }

    #[test]
    fn test_sink_kind_parsing_and_display() {
        assert_eq!("tmux".parse::<SinkKind>().unwrap(), SinkKind::Tmux);
        assert_eq!("zellij".parse::<SinkKind>().unwrap(), SinkKind::Zellij);
        assert_eq!("wezterm".parse::<SinkKind>().unwrap(), SinkKind::WezTerm);
        assert_eq!("kitty".parse::<SinkKind>().unwrap(), SinkKind::Kitty);
        assert_eq!(
            "clipboard".parse::<SinkKind>().unwrap(),
            SinkKind::Clipboard
        );
        assert_eq!("clip".parse::<SinkKind>().unwrap(), SinkKind::Clipboard);
        assert_eq!("dummy".parse::<SinkKind>().unwrap(), SinkKind::Dummy);
        assert_eq!("/dev/null".parse::<SinkKind>().unwrap(), SinkKind::Dummy);
        assert_eq!("none".parse::<SinkKind>().unwrap(), SinkKind::Dummy);
        assert_eq!("null".parse::<SinkKind>().unwrap(), SinkKind::Dummy);
        assert_eq!("auto".parse::<SinkKind>().unwrap(), SinkKind::Auto);
        assert!("unknown".parse::<SinkKind>().is_err());

        assert_eq!(SinkKind::Tmux.to_string(), "tmux");
        assert_eq!(SinkKind::Zellij.to_string(), "zellij");
        assert_eq!(SinkKind::WezTerm.to_string(), "wezterm");
        assert_eq!(SinkKind::Kitty.to_string(), "kitty");
        assert_eq!(SinkKind::Clipboard.to_string(), "clipboard");
        assert_eq!(SinkKind::Dummy.to_string(), "dummy");
    }

    #[test]
    fn test_parse_wezterm_panes_json() {
        let json = r#"[
            {"pane_id": 0, "title": "agent", "is_active": false},
            {"pane_id": 1, "title": "editor", "is_active": true}
        ]"#;
        let panes = IpcBridge::parse_wezterm_panes_json(json).unwrap();
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].id, "0");
        assert_eq!(panes[0].title, "agent");
        assert!(!panes[0].active);
        assert_eq!(panes[1].id, "1");
        assert!(panes[1].active);
    }

    #[test]
    fn test_parse_kitty_windows_json() {
        let json = r#"[
            {
                "tabs": [
                    {
                        "windows": [
                            {"id": 10, "title": "claude", "is_focused": false},
                            {"id": 11, "title": "lgtm", "is_focused": true}
                        ]
                    }
                ]
            }
        ]"#;
        let windows = IpcBridge::parse_kitty_windows_json(json).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].id, "10");
        assert_eq!(windows[0].title, "claude");
        assert!(!windows[0].active);
        assert_eq!(windows[1].id, "11");
        assert!(windows[1].active);
    }

    #[test]
    fn test_detect_sink_fallback() {
        let sink = IpcBridge::detect_sink();
        assert!(matches!(
            sink,
            SinkKind::Tmux
                | SinkKind::Zellij
                | SinkKind::WezTerm
                | SinkKind::Kitty
                | SinkKind::Clipboard
        ));
    }
}
