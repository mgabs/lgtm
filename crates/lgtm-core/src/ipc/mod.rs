pub mod sink;

pub use sink::{AgentSink, AgentSinkKind};

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

const SINK_TABLE: &[(&str, SinkKind)] = &[
    ("tmux", SinkKind::Tmux),
    ("zellij", SinkKind::Zellij),
    ("wezterm", SinkKind::WezTerm),
    ("kitty", SinkKind::Kitty),
    ("clipboard", SinkKind::Clipboard),
    ("clip", SinkKind::Clipboard),
    ("dummy", SinkKind::Dummy),
    ("null", SinkKind::Dummy),
    ("/dev/null", SinkKind::Dummy),
    ("none", SinkKind::Dummy),
    ("auto", SinkKind::Auto),
];

impl std::str::FromStr for SinkKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.trim().to_lowercase();
        SINK_TABLE
            .iter()
            .find_map(|&(name, kind)| (name == lower).then_some(kind))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown agent sink `{}` (expected tmux, zellij, wezterm, kitty, clipboard, or dummy)",
                    lower
                )
            })
    }
}

impl std::fmt::Display for SinkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = SINK_TABLE
            .iter()
            .find_map(|&(name, kind)| (kind == *self).then_some(name))
            .unwrap_or("auto");
        write!(f, "{}", s)
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
        ["ZELLIJ", "ZELLIJ_SESSION_NAME"]
            .into_iter()
            .any(|k| std::env::var(k).is_ok())
    }

    pub fn is_inside_wezterm() -> bool {
        std::env::var("WEZTERM_PANE").is_ok()
    }

    pub fn is_inside_kitty() -> bool {
        ["KITTY_WINDOW_ID", "KITTY_PID"]
            .into_iter()
            .any(|k| std::env::var(k).is_ok())
    }

    pub fn detect_sink() -> SinkKind {
        [
            (Self::is_inside_tmux(), SinkKind::Tmux),
            (Self::is_inside_zellij(), SinkKind::Zellij),
            (Self::is_inside_wezterm(), SinkKind::WezTerm),
            (Self::is_inside_kitty(), SinkKind::Kitty),
        ]
        .into_iter()
        .find_map(|(active, kind)| active.then_some(kind))
        .unwrap_or(SinkKind::Clipboard)
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
        let Some(arr) = val.as_array() else {
            return Ok(Vec::new());
        };
        Ok(arr
            .iter()
            .filter_map(|item| Self::pane_from_json(item, "pane_id", "pane", "is_active"))
            .collect())
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
        let Some(os_windows) = val.as_array() else {
            return Ok(Vec::new());
        };
        let wins = os_windows
            .iter()
            .filter_map(|w| w.get("tabs")?.as_array())
            .flatten()
            .filter_map(|t| t.get("windows")?.as_array())
            .flatten()
            .filter_map(|win| Self::pane_from_json(win, "id", "window", "is_focused"))
            .collect();
        Ok(wins)
    }

    fn pane_from_json(
        item: &serde_json::Value,
        id_key: &str,
        default_title: &str,
        active_key: &str,
    ) -> Option<TmuxPane> {
        let v = item.get(id_key)?;
        let id = v
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| v.as_str().map(|s| s.to_string()))?;
        if id.is_empty() {
            return None;
        }
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(default_title)
            .to_string();
        let active = item
            .get(active_key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Some(TmuxPane { id, title, active })
    }

    pub fn list_zellij_panes() -> Result<Vec<TmuxPane>> {
        if let Ok(out) = Command::new("zellij")
            .args(["action", "query-tab-names"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .ok_or(())
        {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let panes: Vec<TmuxPane> = stdout
                .lines()
                .enumerate()
                .filter_map(|(idx, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty()).then(|| TmuxPane {
                        id: (idx + 1).to_string(),
                        title: trimmed.to_string(),
                        active: idx == 0,
                    })
                })
                .collect();
            if !panes.is_empty() {
                return Ok(panes);
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
        let _ = crate::sys::copy_to_clipboard(text);
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

    #[test]
    fn pane_json_edge_cases() {
        assert!(IpcBridge::parse_wezterm_panes_json("{}")
            .unwrap()
            .is_empty());
        assert!(IpcBridge::parse_kitty_windows_json("{}")
            .unwrap()
            .is_empty());
        assert!(IpcBridge::parse_wezterm_panes_json("nope").is_err());
        assert!(IpcBridge::parse_kitty_windows_json("nope").is_err());

        let panes = IpcBridge::parse_wezterm_panes_json(
            r#"[{"pane_id": "7", "title": "t", "is_active": true},
                {"pane_id": 8},
                {"pane_id": ""},
                {"title": "no id"}]"#,
        )
        .unwrap();
        assert_eq!(panes.len(), 2);
        assert_eq!((panes[0].id.as_str(), panes[0].title.as_str()), ("7", "t"));
        assert!(panes[0].active);
        assert_eq!(
            (panes[1].id.as_str(), panes[1].title.as_str()),
            ("8", "pane")
        );
        assert!(!panes[1].active);
    }

    #[test]
    fn listing_panes_for_non_multiplexer_sinks_is_empty() {
        assert!(IpcBridge::list_panes(SinkKind::Clipboard)
            .unwrap()
            .is_empty());
        assert!(IpcBridge::list_panes(SinkKind::Dummy).unwrap().is_empty());
        assert_eq!(IpcBridge::detect_agent_pane_for(SinkKind::Dummy), None);
    }
}
