use crate::ipc::base64_encode;
use std::io::{self, Write};

pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    // 1. Emit OSC 52 sequence to stdout (works in Windows Terminal, WezTerm, Alacritty)
    let b64 = base64_encode(text.as_bytes());
    let osc_sequence = format!("\x1b]52;c;{}\x07", b64);
    let mut stdout = io::stdout();
    let _ = stdout.write_all(osc_sequence.as_bytes());
    let _ = stdout.flush();

    // 2. On Windows, pipe to native clip.exe as fallback for legacy conhost / CMD
    #[cfg(target_os = "windows")]
    {
        if copy_via_clip_exe(text).is_err() {
            // 3. Secondary fallback to PowerShell Set-Clipboard
            let _ = copy_via_powershell(text);
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn copy_via_clip_exe(text: &str) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut child = std::process::Command::new("clip.exe")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
        stdin.flush()?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "clip.exe failed"))
    }
}

#[cfg(target_os = "windows")]
pub fn copy_via_powershell(text: &str) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut child = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Set-Clipboard -Value $input",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
        stdin.flush()?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "powershell Set-Clipboard failed",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_to_clipboard_does_not_panic() {
        // OSC 52 emission should write without error in test environment
        let result = copy_to_clipboard("test payload for clipboard");
        assert!(result.is_ok());
    }
}
