use crate::model::{ChangeType, DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
use anyhow::{Context, Result};
use git2::{Delta, Diff, DiffFormat, DiffOptions, Repository};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_current: bool,
}

pub struct GitEngine {
    pub repo_path: PathBuf,
}

impl GitEngine {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        let path = repo_path.as_ref().to_path_buf();
        Self { repo_path: path }
    }

    pub fn is_git_repo(&self) -> bool {
        Repository::open(&self.repo_path).is_ok() || Repository::discover(&self.repo_path).is_ok()
    }

    pub fn open_repo(&self) -> Result<Repository> {
        Repository::open(&self.repo_path)
            .or_else(|_| Repository::discover(&self.repo_path))
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

    /// A bare local or remote-tracking branch name becomes `<branch>...HEAD` (what the current
    /// branch changed since it diverged); commits and explicit ranges are returned as given.
    pub fn expand_branch_spec(&self, spec: &str) -> String {
        let is_branch = !spec.contains("..")
            && self.open_repo().is_ok_and(|r| {
                r.find_branch(spec, git2::BranchType::Local).is_ok()
                    || r.find_branch(spec, git2::BranchType::Remote).is_ok()
            });
        if is_branch {
            format!("{spec}...HEAD")
        } else {
            spec.to_string()
        }
    }

    pub fn compute_revision_diff(&self, rev_spec: &str) -> Result<DiffSnapshot> {
        let repo = self.open_repo()?;
        let mut diff_opts = DiffOptions::new();
        diff_opts.context_lines(3);

        let diff = if rev_spec.contains("...") {
            let parts: Vec<&str> = rev_spec.split("...").collect();
            let from_rev = parts.first().unwrap_or(&"HEAD");
            let to_rev = parts.get(1).unwrap_or(&"HEAD");
            let from_obj = repo.revparse_single(from_rev)?;
            let to_obj = repo.revparse_single(to_rev)?;
            let from_commit = from_obj.peel_to_commit()?;
            let to_commit = to_obj.peel_to_commit()?;
            let base_oid = repo.merge_base(from_commit.id(), to_commit.id())?;
            let base_commit = repo.find_commit(base_oid)?;
            let base_tree = base_commit.tree()?;
            let to_tree = to_commit.tree()?;
            repo.diff_tree_to_tree(Some(&base_tree), Some(&to_tree), Some(&mut diff_opts))?
        } else if rev_spec.contains("..") {
            let parts: Vec<&str> = rev_spec.split("..").collect();
            let from_rev = parts.first().unwrap_or(&"HEAD");
            let to_rev = parts.get(1).unwrap_or(&"HEAD");
            let from_obj = repo.revparse_single(from_rev)?;
            let to_obj = repo.revparse_single(to_rev)?;
            let from_tree = from_obj.peel_to_tree()?;
            let to_tree = to_obj.peel_to_tree()?;
            repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_opts))?
        } else {
            let obj = repo.revparse_single(rev_spec)?;
            let commit = obj.peel_to_commit()?;
            let to_tree = commit.tree()?;
            if commit.parent_count() > 0 {
                let parent = commit.parent(0)?;
                let from_tree = parent.tree()?;
                repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_opts))?
            } else {
                repo.diff_tree_to_tree(None, Some(&to_tree), Some(&mut diff_opts))?
            }
        };

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
                        is_folded: false,
                        folded_blocks: Vec::new(),
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

    pub fn stage_file(&self, path: &Path) -> Result<()> {
        let repo = self.open_repo()?;
        let mut index = repo.index()?;
        index.add_path(path)?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &Path) -> Result<()> {
        let repo = self.open_repo()?;
        let mut index = repo.index()?;
        index.remove_path(path)?;
        index.write()?;
        Ok(())
    }

    pub fn stage_hunk(&self, file: &FileDiff, hunk: &DiffHunk) -> Result<()> {
        let patch = generate_hunk_patch(file, hunk);
        apply_patch(&self.repo_path, &patch, true, false)
    }

    pub fn unstage_hunk(&self, file: &FileDiff, hunk: &DiffHunk) -> Result<()> {
        let patch = generate_hunk_patch(file, hunk);
        apply_patch(&self.repo_path, &patch, true, true)
    }

    pub fn discard_hunk(&self, file: &FileDiff, hunk: &DiffHunk) -> Result<()> {
        let patch = generate_hunk_patch(file, hunk);
        apply_patch(&self.repo_path, &patch, false, true)
    }

    pub fn stage_lines(
        &self,
        file: &FileDiff,
        hunk: &DiffHunk,
        selected_indices: &[usize],
    ) -> Result<()> {
        let patch = generate_line_selection_patch(file, hunk, selected_indices);
        apply_patch(&self.repo_path, &patch, true, false)
    }

    pub fn unstage_lines(
        &self,
        file: &FileDiff,
        hunk: &DiffHunk,
        selected_indices: &[usize],
    ) -> Result<()> {
        let patch = generate_line_selection_patch(file, hunk, selected_indices);
        apply_patch(&self.repo_path, &patch, true, true)
    }

    pub fn discard_lines(
        &self,
        file: &FileDiff,
        hunk: &DiffHunk,
        selected_indices: &[usize],
    ) -> Result<()> {
        let patch = generate_line_selection_patch(file, hunk, selected_indices);
        apply_patch(&self.repo_path, &patch, false, true)
    }

    pub fn commit(&self, message: &str) -> Result<git2::Oid> {
        let repo = self.open_repo()?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let sig = repo
            .signature()
            .unwrap_or_else(|_| git2::Signature::now("lgtm user", "lgtm@local").unwrap());

        let head = repo.head();
        let parents = match head {
            Ok(head_ref) => {
                if let Some(target) = head_ref.target() {
                    let parent_commit = repo.find_commit(target)?;
                    vec![parent_commit]
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };

        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;
        Ok(oid)
    }

    pub fn last_commit_message(&self) -> Option<String> {
        let repo = git2::Repository::open(&self.repo_path).ok()?;
        let head = repo.head().ok()?;
        let commit = head.peel_to_commit().ok()?;
        commit.message().map(|m| m.trim().to_string())
    }

    pub fn hooks_dir(&self) -> Result<PathBuf> {
        let repo = self.open_repo()?;
        Ok(repo.path().join("hooks"))
    }

    pub fn hook_path(&self, hook_name: &str) -> Result<PathBuf> {
        Ok(self.hooks_dir()?.join(hook_name))
    }

    pub fn install_pre_commit_hook(&self) -> Result<PathBuf> {
        let hook_file = self.hook_path("pre-commit")?;
        if let Some(parent) = hook_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let hook_content = "#!/bin/sh\n# Generated by LGTM (Looks Good To Me)\n# Pre-commit quality & security gate\nexec lgtm check --strict\n";
        if hook_file.exists() {
            let existing = std::fs::read_to_string(&hook_file)?;
            if !existing.contains("lgtm check") {
                let appended = format!(
                    "{}\n\n# LGTM pre-commit gate\nlgtm check --strict\n",
                    existing.trim_end()
                );
                std::fs::write(&hook_file, appended)?;
            }
        } else {
            std::fs::write(&hook_file, hook_content)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&hook_file) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&hook_file, perms)?;
            }
        }

        Ok(hook_file)
    }

    pub fn uninstall_pre_commit_hook(&self) -> Result<bool> {
        let hook_file = self.hook_path("pre-commit")?;
        if !hook_file.exists() {
            return Ok(false);
        }
        let existing = std::fs::read_to_string(&hook_file)?;
        if existing.contains("Generated by LGTM") {
            std::fs::remove_file(&hook_file)?;
            return Ok(true);
        } else if existing.contains("lgtm check") {
            let filtered: Vec<&str> = existing
                .lines()
                .filter(|l| !l.contains("lgtm check") && !l.contains("LGTM pre-commit gate"))
                .collect();
            std::fs::write(&hook_file, filtered.join("\n") + "\n")?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn pre_commit_hook_status(&self) -> Result<bool> {
        let hook_file = self.hook_path("pre-commit")?;
        if !hook_file.exists() {
            return Ok(false);
        }
        let existing = std::fs::read_to_string(&hook_file)?;
        Ok(existing.contains("lgtm check"))
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(&self.repo_path);
        cmd.args(["worktree", "list", "--porcelain"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = cmd.output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git worktree list failed: {}", err.trim());
        }

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_worktrees_porcelain(&text, &self.repo_path))
    }
}

pub fn generate_line_selection_patch(
    file: &FileDiff,
    hunk: &DiffHunk,
    selected_indices: &[usize],
) -> String {
    let old_p = file
        .old_path
        .as_ref()
        .unwrap_or(&file.new_path)
        .to_string_lossy();
    let new_p = file.new_path.to_string_lossy();

    let mut patch = format!("--- a/{}\n+++ b/{}\n", old_p, new_p);
    patch.push_str(&hunk.header);
    patch.push('\n');

    for (idx, line) in hunk.lines.iter().enumerate() {
        let is_selected = selected_indices.contains(&idx);
        match line.line_type {
            LineType::Context => {
                patch.push(' ');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            LineType::Addition => {
                if is_selected {
                    patch.push('+');
                    patch.push_str(&line.content);
                    patch.push('\n');
                }
            }
            LineType::Deletion => {
                if is_selected {
                    patch.push('-');
                    patch.push_str(&line.content);
                    patch.push('\n');
                } else {
                    patch.push(' ');
                    patch.push_str(&line.content);
                    patch.push('\n');
                }
            }
            LineType::HunkHeader => {}
        }
    }

    patch
}

pub fn generate_hunk_patch(file: &FileDiff, hunk: &DiffHunk) -> String {
    let old_p = file
        .old_path
        .as_ref()
        .unwrap_or(&file.new_path)
        .to_string_lossy();
    let new_p = file.new_path.to_string_lossy();

    let mut patch = format!("--- a/{}\n+++ b/{}\n", old_p, new_p);
    patch.push_str(&hunk.header);
    patch.push('\n');

    for line in &hunk.lines {
        match line.line_type {
            LineType::Context => {
                patch.push(' ');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            LineType::Addition => {
                patch.push('+');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            LineType::Deletion => {
                patch.push('-');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            LineType::HunkHeader => {}
        }
    }

    patch
}

pub fn apply_patch(repo_path: &Path, patch: &str, cached: bool, reverse: bool) -> Result<()> {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("apply");
    cmd.arg("--unidiff-zero");

    if cached {
        cmd.arg("--cached");
    }
    if reverse {
        cmd.arg("--reverse");
    }
    cmd.arg("-");

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(patch.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git apply failed: {}", err_msg.trim());
    }

    Ok(())
}

pub fn parse_worktrees_porcelain(output: &str, current_repo_path: &Path) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let norm_current = crate::sys::canonicalize_clean(current_repo_path)
        .unwrap_or_else(|_| crate::sys::strip_verbatim_prefix(current_repo_path.to_path_buf()));

    let mut cur_path: Option<PathBuf> = None;
    let mut cur_head = String::new();
    let mut cur_branch: Option<String> = None;
    let mut cur_bare = false;

    let flush = |path: Option<PathBuf>,
                 head: String,
                 branch: Option<String>,
                 bare: bool,
                 worktrees: &mut Vec<WorktreeInfo>| {
        if let Some(p) = path {
            let p_norm = crate::sys::canonicalize_clean(&p)
                .unwrap_or_else(|_| crate::sys::strip_verbatim_prefix(p.clone()));
            let is_current = p_norm == norm_current || p == current_repo_path;
            let p_clean = crate::sys::strip_verbatim_prefix(p);
            worktrees.push(WorktreeInfo {
                path: p_clean,
                head,
                branch,
                is_bare: bare,
                is_current,
            });
        }
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(
                cur_path.take(),
                std::mem::take(&mut cur_head),
                cur_branch.take(),
                cur_bare,
                &mut worktrees,
            );
            cur_bare = false;
        } else if let Some(p) = trimmed.strip_prefix("worktree ") {
            if cur_path.is_some() {
                flush(
                    cur_path.take(),
                    std::mem::take(&mut cur_head),
                    cur_branch.take(),
                    cur_bare,
                    &mut worktrees,
                );
                cur_bare = false;
            }
            cur_path = Some(PathBuf::from(p.trim()));
        } else if let Some(h) = trimmed.strip_prefix("HEAD ") {
            cur_head = h.trim().to_string();
        } else if let Some(b) = trimmed.strip_prefix("branch ") {
            let b_trim = b.trim();
            let branch_name = b_trim.strip_prefix("refs/heads/").unwrap_or(b_trim);
            cur_branch = Some(branch_name.to_string());
        } else if trimmed == "bare" {
            cur_bare = true;
        } else if trimmed == "detached" {
            cur_branch = None;
        }
    }

    if cur_path.is_some() {
        flush(cur_path, cur_head, cur_branch, cur_bare, &mut worktrees);
    }

    worktrees
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
    fn bare_branch_names_expand_to_a_merge_base_range() {
        let tmp = test_dir("branchspec");
        let repo = Repository::init(&tmp).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        let commit = |msg: &str, parent: Option<&git2::Commit>| {
            let mut idx = repo.index().unwrap();
            let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
            let parents: Vec<&git2::Commit> = parent.into_iter().collect();
            let oid = repo
                .commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
                .unwrap();
            repo.find_commit(oid).unwrap()
        };
        let first = commit("one", None);
        let base = repo.head().unwrap().shorthand().unwrap().to_string();
        commit("two", Some(&first));
        let engine = GitEngine::new(&tmp);
        assert_eq!(engine.expand_branch_spec(&base), format!("{base}...HEAD"));
        assert_eq!(engine.expand_branch_spec("HEAD~1"), "HEAD~1");
        assert_eq!(engine.expand_branch_spec("a..b"), "a..b");
        assert_eq!(engine.expand_branch_spec("nope"), "nope");
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

    #[test]
    fn test_git_hook_lifecycle() {
        let tmp = test_dir("hook_lifecycle");
        let _repo = Repository::init(&tmp).unwrap();
        let engine = GitEngine::new(&tmp);

        // Initially not installed
        assert!(!engine.pre_commit_hook_status().unwrap());

        // Install
        let path = engine.install_pre_commit_hook().unwrap();
        assert!(path.exists());
        assert!(engine.pre_commit_hook_status().unwrap());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("lgtm check --strict"));

        // Re-installing is idempotent
        let path2 = engine.install_pre_commit_hook().unwrap();
        assert_eq!(path, path2);
        assert!(engine.pre_commit_hook_status().unwrap());

        // Uninstall
        let removed = engine.uninstall_pre_commit_hook().unwrap();
        assert!(removed);
        assert!(!engine.pre_commit_hook_status().unwrap());
        assert!(!path.exists());

        // Uninstalling again returns false
        assert!(!engine.uninstall_pre_commit_hook().unwrap());
    }

    #[test]
    fn test_generate_hunk_patch() {
        let hunk = DiffHunk {
            header: "@@ -1,2 +1,3 @@".to_string(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 3,
            lines: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn test() {".to_string(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    let a = 42;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "}".to_string(),
                    old_line_num: Some(2),
                    new_line_num: Some(3),
                },
            ],
            is_folded: false,
            folded_blocks: Vec::new(),
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![hunk.clone()],
            is_folded: false,
            is_reviewed: false,
        };

        let patch = generate_hunk_patch(&file, &hunk);
        assert!(patch.contains("--- a/src/main.rs"));
        assert!(patch.contains("+++ b/src/main.rs"));
        assert!(patch.contains("@@ -1,2 +1,3 @@"));
        assert!(patch.contains("+    let a = 42;"));
    }

    #[test]
    fn test_generate_line_selection_patch_selective_addition() {
        let hunk = DiffHunk {
            header: "@@ -1,2 +1,4 @@".to_string(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 4,
            lines: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn test() {".to_string(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    let a = 1;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    let b = 2;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(3),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "}".to_string(),
                    old_line_num: Some(2),
                    new_line_num: Some(4),
                },
            ],
            is_folded: false,
            folded_blocks: Vec::new(),
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk.clone()],
            is_folded: false,
            is_reviewed: false,
        };

        // Select only line index 1 ("let a = 1;")
        let patch = generate_line_selection_patch(&file, &hunk, &[1]);
        assert!(patch.contains("+    let a = 1;"));
        assert!(!patch.contains("+    let b = 2;"));
        assert!(patch.contains(" fn test() {"));
        assert!(patch.contains(" }"));
    }

    #[test]
    fn test_generate_line_selection_patch_selective_deletion() {
        let hunk = DiffHunk {
            header: "@@ -1,4 +1,2 @@".to_string(),
            old_start: 1,
            old_lines: 4,
            new_start: 1,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn test() {".to_string(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Deletion,
                    content: "    let a = 1;".to_string(),
                    old_line_num: Some(2),
                    new_line_num: None,
                },
                DiffLine {
                    line_type: LineType::Deletion,
                    content: "    let b = 2;".to_string(),
                    old_line_num: Some(3),
                    new_line_num: None,
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "}".to_string(),
                    old_line_num: Some(4),
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
            folded_blocks: Vec::new(),
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 2,
            hunks: vec![hunk.clone()],
            is_folded: false,
            is_reviewed: false,
        };

        // Select only line index 2 ("let b = 2;") to delete; index 1 should remain as context
        let patch = generate_line_selection_patch(&file, &hunk, &[2]);
        assert!(patch.contains("-    let b = 2;"));
        assert!(patch.contains("     let a = 1;"));
        assert!(!patch.contains("-    let a = 1;"));
    }

    #[test]
    fn test_parse_worktrees_porcelain() {
        let sample = "\
worktree /Users/dev/repo
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree /Users/dev/repo/.worktrees/feat-x
HEAD 2222222222222222222222222222222222222222
branch refs/heads/feat-x

worktree /Users/dev/repo/.worktrees/detached
HEAD 3333333333333333333333333333333333333333
detached

worktree /Users/dev/repo/.worktrees/bare
bare
";
        let list = parse_worktrees_porcelain(sample, Path::new("/Users/dev/repo"));
        assert_eq!(list.len(), 4);

        assert_eq!(list[0].path, PathBuf::from("/Users/dev/repo"));
        assert_eq!(list[0].head, "1111111111111111111111111111111111111111");
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert!(!list[0].is_bare);
        assert!(list[0].is_current);

        assert_eq!(
            list[1].path,
            PathBuf::from("/Users/dev/repo/.worktrees/feat-x")
        );
        assert_eq!(list[1].branch.as_deref(), Some("feat-x"));
        assert!(!list[1].is_current);

        assert_eq!(
            list[2].path,
            PathBuf::from("/Users/dev/repo/.worktrees/detached")
        );
        assert_eq!(list[2].branch, None);

        assert_eq!(
            list[3].path,
            PathBuf::from("/Users/dev/repo/.worktrees/bare")
        );
        assert!(list[3].is_bare);
    }

    #[test]
    fn test_git_engine_list_worktrees_real_repo() {
        let root = test_dir("wt-real-root");
        let _ = Repository::init(&root).unwrap();

        // Need at least one commit to create a worktree
        let test_file = root.join("file.txt");
        std::fs::write(&test_file, "hello").unwrap();

        let engine = GitEngine::new(&root);
        // Stage and commit via git cli
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(&root);
        cmd.args(["add", "file.txt"]);
        let _ = cmd.output().unwrap();

        let mut cmd2 = std::process::Command::new("git");
        cmd2.current_dir(&root);
        cmd2.args(["commit", "-m", "initial commit", "--no-gpg-sign"]);
        let _ = cmd2.output().unwrap();

        let wt_path = test_dir("wt-real-child");
        let mut cmd3 = std::process::Command::new("git");
        cmd3.current_dir(&root);
        cmd3.args([
            "worktree",
            "add",
            "-b",
            "branch-wt",
            wt_path.to_str().unwrap(),
        ]);
        let out3 = cmd3.output().unwrap();
        if out3.status.success() {
            let worktrees = engine.list_worktrees().unwrap();
            assert!(worktrees.len() >= 2);
            assert!(worktrees.iter().any(|w| w.is_current));
            assert!(worktrees
                .iter()
                .any(|w| w.branch.as_deref() == Some("branch-wt")));
        }
    }
}
