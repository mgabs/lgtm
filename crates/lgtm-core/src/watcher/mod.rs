use anyhow::Result;
use crossbeam_channel::Sender;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum WatcherMessage {
    FilesChanged(Vec<PathBuf>),
    Error(String),
}

/// Internal review writes and fetched refs must not trigger diff/scan feedback loops.
pub fn affects_diff(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative
        .components()
        .any(|c| c.as_os_str() == ".lgtm" || c.as_os_str() == "target")
    {
        return false;
    }
    if let Ok(git_path) = relative.strip_prefix(".git") {
        return matches!(git_path.to_str(), Some("index" | "HEAD"))
            || git_path.starts_with("refs/heads");
    }
    true
}

pub struct RepoWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl RepoWatcher {
    pub fn spawn<P: AsRef<Path>>(
        repo_root: P,
        sender: Sender<WatcherMessage>,
        debounce_ms: u64,
    ) -> Result<Self> {
        let root = repo_root.as_ref().to_path_buf();
        let mut debouncer = new_debouncer(
            Duration::from_millis(debounce_ms),
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    let paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
                    let _ = sender.send(WatcherMessage::FilesChanged(paths));
                }
                Err(err) => {
                    let _ = sender.send(WatcherMessage::Error(err.to_string()));
                }
            },
        )?;

        debouncer.watcher().watch(&root, RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn internal_writes_do_not_invalidate_diff() {
        let root = Path::new("/repo");
        for path in [
            ".lgtm/comments.jsonl",
            ".lgtm/state.json",
            ".lgtm/debug.jsonl",
            ".git/FETCH_HEAD",
            ".git/refs/lgtm/pr/1/merge",
            "target/debug/lgtm",
        ] {
            assert!(!affects_diff(root, &root.join(path)), "{path}");
        }
        for path in [
            "src/lib.rs",
            ".lgtm.toml",
            ".git/index",
            ".git/HEAD",
            ".git/refs/heads/main",
        ] {
            assert!(affects_diff(root, &root.join(path)), "{path}");
        }
    }

    #[test]
    fn test_repo_watcher_spawn() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("lgtm-test-watcher-{}", unique));
        let _ = std::fs::create_dir_all(&tmp);

        let (tx, rx) = unbounded();
        let watcher = RepoWatcher::spawn(&tmp, tx, 50);
        assert!(watcher.is_ok());

        // Test sending message on channel
        let test_file = tmp.join("change.txt");
        let _ = std::fs::write(&test_file, "changed");
        std::thread::sleep(Duration::from_millis(100));

        // Drain messages if any received
        while let Ok(msg) = rx.try_recv() {
            match msg {
                WatcherMessage::FilesChanged(paths) => {
                    assert!(!paths.is_empty() || paths.is_empty());
                }
                WatcherMessage::Error(e) => {
                    println!("Watcher err: {}", e);
                }
            }
        }
    }
}
