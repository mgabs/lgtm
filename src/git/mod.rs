use crate::model::{ChangeType, DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
use anyhow::{Context, Result};
use git2::{Delta, Diff, DiffFormat, DiffOptions, Repository};
use std::path::{Path, PathBuf};

pub struct GitEngine {
    pub repo_path: PathBuf,
}

impl GitEngine {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        let path = repo_path.as_ref().to_path_buf();
        Self { repo_path: path }
    }

    pub fn is_git_repo(&self) -> bool {
        Repository::discover(&self.repo_path).is_ok()
    }

    pub fn open_repo(&self) -> Result<Repository> {
        Repository::discover(&self.repo_path)
            .with_context(|| format!("No git repository found at {}", self.repo_path.display()))
    }

    pub fn compute_diff_snapshot(&self) -> Result<DiffSnapshot> {
        let repo = match self.open_repo() {
            Ok(r) => r,
            Err(_) => return Ok(DiffSnapshot::default()),
        };

        let mut diff_opts = DiffOptions::new();
        diff_opts
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .show_untracked_content(true)
            .include_unmodified(false)
            .context_lines(3);

        // Diff HEAD against working tree (both staged and unstaged)
        let head_tree = match repo.head() {
            Ok(head_ref) => head_ref.peel_to_tree().ok(),
            Err(_) => None,
        };

        let diff =
            repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_opts))?;
        self.parse_diff(&diff)
    }

    fn parse_diff(&self, diff: &Diff) -> Result<DiffSnapshot> {
        let mut file_diffs: Vec<FileDiff> = Vec::new();
        let mut current_file: Option<FileDiff> = None;
        let mut current_hunk: Option<DiffHunk> = None;

        let _ = diff.print(DiffFormat::Patch, |delta_cb, hunk_cb, line_cb| {
            let old_file = delta_cb.old_file().path().map(|p| p.to_path_buf());
            let new_file = delta_cb
                .new_file()
                .path()
                .map(|p| p.to_path_buf())
                .or_else(|| old_file.clone())
                .unwrap_or_else(|| PathBuf::from("unknown"));

            let is_new_file = match &current_file {
                Some(curr) => curr.new_path != new_file,
                None => true,
            };

            if is_new_file {
                if let Some(h) = current_hunk.take() {
                    if let Some(ref mut f) = current_file {
                        f.hunks.push(h);
                    }
                }
                if let Some(f) = current_file.take() {
                    file_diffs.push(f);
                }

                let change_type = match delta_cb.status() {
                    Delta::Added => ChangeType::Added,
                    Delta::Deleted => ChangeType::Deleted,
                    Delta::Modified => ChangeType::Modified,
                    Delta::Renamed => ChangeType::Renamed,
                    Delta::Untracked => ChangeType::Untracked,
                    _ => ChangeType::Modified,
                };

                let file_str = new_file.to_string_lossy().to_lowercase();
                let is_folded = file_str.ends_with("lock")
                    || file_str.ends_with(".lock")
                    || file_str.ends_with("-lock.json");

                current_file = Some(FileDiff {
                    old_path: old_file,
                    new_path: new_file,
                    change_type,
                    additions: 0,
                    deletions: 0,
                    hunks: Vec::new(),
                    is_folded,
                    is_reviewed: false,
                });
            }

            if let Some(hunk) = hunk_cb {
                let hunk_header = String::from_utf8_lossy(hunk.header()).trim().to_string();
                let is_new_hunk = match &current_hunk {
                    Some(curr) => {
                        curr.old_start != hunk.old_start()
                            || curr.new_start != hunk.new_start()
                            || curr.header != hunk_header
                    }
                    None => true,
                };

                if is_new_hunk {
                    if let Some(h) = current_hunk.take() {
                        if let Some(ref mut f) = current_file {
                            f.hunks.push(h);
                        }
                    }
                    current_hunk = Some(DiffHunk {
                        header: hunk_header,
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        lines: Vec::new(),
                    });
                }
            }

            let line_origin = line_cb.origin();
            if line_origin == 'H' || line_origin == 'F' || line_origin == 'B' {
                return true;
            }

            let line_content = String::from_utf8_lossy(line_cb.content())
                .trim_end_matches(['\r', '\n'])
                .to_string();

            let line_type = match line_origin {
                '+' => {
                    if let Some(ref mut f) = current_file {
                        f.additions += 1;
                    }
                    LineType::Addition
                }
                '-' => {
                    if let Some(ref mut f) = current_file {
                        f.deletions += 1;
                    }
                    LineType::Deletion
                }
                _ => LineType::Context,
            };

            if let Some(ref mut hunk) = current_hunk {
                hunk.lines.push(DiffLine {
                    line_type,
                    content: line_content,
                    old_line_num: line_cb.old_lineno(),
                    new_line_num: line_cb.new_lineno(),
                });
            }

            true
        });

        if let Some(h) = current_hunk {
            if let Some(ref mut f) = current_file {
                f.hunks.push(h);
            }
        }
        if let Some(f) = current_file {
            file_diffs.push(f);
        }

        Ok(DiffSnapshot::new(file_diffs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lgtm-test-{}-{}", name, unique));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn test_git_engine_compute_diff() {
        let engine = GitEngine::new(".");
        let start = std::time::Instant::now();
        let snapshot = engine
            .compute_diff_snapshot()
            .expect("snapshot should compute");
        assert!(
            start.elapsed() < std::time::Duration::from_millis(500),
            "Diff parsing took too long: {:?}",
            start.elapsed()
        );
        if engine.is_git_repo() {
            println!("Discovered {} files in snapshot", snapshot.files.len());
        }
    }

    #[test]
    fn test_non_git_dir_fallback() {
        let tmp = test_dir("nongit");
        let engine = GitEngine::new(&tmp);
        assert!(!engine.is_git_repo());
        let snapshot = engine.compute_diff_snapshot().unwrap();
        assert_eq!(snapshot.files.len(), 0);
    }

    #[test]
    fn test_temp_git_repo_diff() {
        let tmp = test_dir("temprepo");
        let _repo = Repository::init(&tmp).unwrap();
        let file_path = tmp.join("hello.txt");
        std::fs::write(&file_path, "line 1\nline 2\n").unwrap();

        let engine = GitEngine::new(&tmp);
        assert!(engine.is_git_repo());
        let snapshot = engine.compute_diff_snapshot().unwrap();
        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.files[0].change_type, ChangeType::Untracked);
    }
}
