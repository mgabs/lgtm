use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Canonicalizes a path and strips Windows verbatim extended-length prefixes (`\\?\` and `\\?\UNC\`).
pub fn canonicalize_clean<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    Ok(strip_verbatim_prefix(canonical))
}

/// Strips `\\?\` and `\\?\UNC\` extended-length prefixes from a PathBuf.
/// Works across platforms so path normalization can be verified in cross-platform unit tests.
pub fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", stripped))
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

/// Normalizes path to use forward slashes (`/`) for deterministic Git indexing and comment references.
pub fn normalize_git_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.contains('\\') {
        s.replace('\\', "/")
    } else {
        s.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_verbatim_disk_prefix() {
        let p = PathBuf::from(r"\\?\C:\Users\runner\repo");
        assert_eq!(
            strip_verbatim_prefix(p),
            PathBuf::from(r"C:\Users\runner\repo")
        );
    }

    #[test]
    fn test_strip_verbatim_unc_prefix() {
        let p = PathBuf::from(r"\\?\UNC\server\share\repo");
        assert_eq!(
            strip_verbatim_prefix(p),
            PathBuf::from(r"\\server\share\repo")
        );
    }

    #[test]
    fn test_strip_verbatim_prefix_noop() {
        let p = PathBuf::from(r"C:\Users\runner\repo");
        assert_eq!(strip_verbatim_prefix(p.clone()), p);

        let unix_p = PathBuf::from("/home/runner/repo");
        assert_eq!(strip_verbatim_prefix(unix_p.clone()), unix_p);
    }

    #[test]
    fn test_normalize_git_path() {
        assert_eq!(
            normalize_git_path(Path::new(r"src\app\mod.rs")),
            "src/app/mod.rs"
        );
        assert_eq!(
            normalize_git_path(Path::new("src/app/mod.rs")),
            "src/app/mod.rs"
        );
        assert_eq!(
            normalize_git_path(Path::new(r"crates\lgtm-core\src\lib.rs")),
            "crates/lgtm-core/src/lib.rs"
        );
    }
}
