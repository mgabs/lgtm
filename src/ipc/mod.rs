use anyhow::{bail, Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
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
        if !Self::is_inside_tmux() {
            return None;
        }

        let panes = Self::list_tmux_panes().ok()?;
        // Prefer non-active pane in the current window
        panes.into_iter().find(|p| !p.active).map(|p| p.id)
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
}
