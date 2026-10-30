use std::borrow::Cow;

/// Resolves an executable program name for the current operating system.
pub fn resolve_program<'a>(program: &'a str) -> Cow<'a, str> {
    resolve_program_for_os(program, cfg!(target_os = "windows"))
}

/// Resolves program name with explicit OS flag (enables deterministic cross-platform unit testing).
pub fn resolve_program_for_os<'a>(program: &'a str, is_windows: bool) -> Cow<'a, str> {
    if !is_windows {
        return Cow::Borrowed(program);
    }

    // If program already has an extension or path separator, do not alter it
    if program.contains('.') || program.contains('\\') || program.contains('/') {
        return Cow::Borrowed(program);
    }

    // Fast-path lookup for known CLI utilities commonly distributed as batch wrappers
    match program.to_ascii_lowercase().as_str() {
        "az" => return Cow::Borrowed("az.cmd"),
        "code" => return Cow::Borrowed("code.cmd"),
        "cursor" => return Cow::Borrowed("cursor.cmd"),
        _ => {}
    }

    // Probe PATHEXT if running natively on Windows
    #[cfg(target_os = "windows")]
    {
        if let Ok(path_var) = std::env::var("PATH") {
            let pathext_var =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
            let extensions: Vec<&str> = pathext_var.split(';').filter(|e| !e.is_empty()).collect();

            for dir in std::env::split_paths(&path_var) {
                for ext in &extensions {
                    let candidate = dir.join(format!("{}{}", program, ext.to_ascii_lowercase()));
                    if candidate.is_file() {
                        return Cow::Owned(format!("{}{}", program, ext.to_ascii_lowercase()));
                    }
                }
            }
        }
    }

    Cow::Borrowed(program)
}

/// Helper to configure a Command instance with safe Windows process flags.
pub fn create_command(program: &str) -> std::process::Command {
    let resolved = resolve_program(program);
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(resolved.as_ref());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        // Suppress console window popups for non-interactive helper utilities
        if program == "clip"
            || program == "clip.exe"
            || program == "powershell"
            || program == "powershell.exe"
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_program_unix() {
        assert_eq!(resolve_program_for_os("az", false), "az");
        assert_eq!(resolve_program_for_os("code", false), "code");
        assert_eq!(resolve_program_for_os("git", false), "git");
    }

    #[test]
    fn test_resolve_program_windows_fast_path() {
        assert_eq!(resolve_program_for_os("az", true), "az.cmd");
        assert_eq!(resolve_program_for_os("code", true), "code.cmd");
        assert_eq!(resolve_program_for_os("cursor", true), "cursor.cmd");
        // Unknown or standard exes without extension fall through
        assert_eq!(
            resolve_program_for_os("lgtm-no-such-tool", true),
            "lgtm-no-such-tool"
        );
    }

    #[test]
    fn test_resolve_program_windows_explicit_extension_preserved() {
        assert_eq!(resolve_program_for_os("az.exe", true), "az.exe");
        assert_eq!(resolve_program_for_os("code.cmd", true), "code.cmd");
        assert_eq!(resolve_program_for_os(r"C:\bin\az", true), r"C:\bin\az");
    }
}
