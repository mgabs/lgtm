//! Durable review state: hunk marks and comments keyed by content hash, not line numbers.
use crate::comment::{CommentAnchor, CommentStore};
use crate::model::{DiffHunk, FileDiff, LineType};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Active,
    Resolved,
    Orphaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunkMark {
    pub file: String,
    pub content_hash: String,
    pub last_line: u32,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewComment {
    #[serde(flatten)]
    pub anchor: CommentAnchor,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub hunk_start: u32,
    #[serde(default)]
    pub line_idx: usize,
    #[serde(default)]
    pub last_line: u32,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub remote_thread: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReviewState {
    pub version: u32,
    pub target: String,
    #[serde(default)]
    pub hunks: Vec<HunkMark>,
    #[serde(default)]
    pub comments: Vec<ReviewComment>,
}

/// Hash of the whitespace-normalized changed lines; ignores context, headers and line numbers.
pub fn hunk_hash(h: &DiffHunk) -> String {
    let body: Vec<String> = h
        .lines
        .iter()
        .filter_map(|l| {
            let sign = match l.line_type {
                LineType::Addition => '+',
                LineType::Deletion => '-',
                _ => return None,
            };
            let text = l.content.split_whitespace().collect::<Vec<_>>().join(" ");
            Some(format!("{sign}{text}"))
        })
        .collect();
    format!("{:016x}", crate::guard::fnv1a(&body.join("\n")))
}

/// Per hunk of `file`: is there a mark with the same hash? Duplicate hashes pair with the
/// nearest unused mark by line, so identical hunks stay distinct.
pub fn reviewed_flags(marks: &[HunkMark], file: &FileDiff) -> Vec<bool> {
    let name = file.display_name();
    let hashes: Vec<String> = file.hunks.iter().map(hunk_hash).collect();
    let mut pairs = Vec::new();
    for (hi, h) in file.hunks.iter().enumerate() {
        for (mi, m) in marks.iter().enumerate() {
            if m.file == name && m.content_hash == hashes[hi] {
                pairs.push((m.last_line.abs_diff(h.new_start), hi, mi));
            }
        }
    }
    pairs.sort_unstable();
    let mut flags = vec![false; hashes.len()];
    let mut used = vec![false; marks.len()];
    for (_, hi, mi) in pairs {
        if !flags[hi] && !used[mi] {
            flags[hi] = true;
            used[mi] = true;
        }
    }
    flags
}

/// A file is reviewed when it has hunks and all of them are marked.
pub fn file_reviewed(marks: &[HunkMark], file: &FileDiff) -> bool {
    let flags = reviewed_flags(marks, file);
    !flags.is_empty() && flags.iter().all(|&b| b)
}

/// Mark or unmark one hunk; returns the new state (true = reviewed).
pub fn toggle_hunk(marks: &mut Vec<HunkMark>, file: &FileDiff, idx: usize, now: &str) -> bool {
    let name = file.display_name();
    let h = &file.hunks[idx];
    let hash = hunk_hash(h);
    let pos = marks
        .iter()
        .enumerate()
        .filter(|(_, m)| m.file == name && m.content_hash == hash)
        .min_by_key(|(_, m)| m.last_line.abs_diff(h.new_start))
        .map(|(i, _)| i);
    match pos {
        Some(i) => {
            marks.remove(i);
            false
        }
        None => {
            marks.push(HunkMark {
                file: name,
                content_hash: hash,
                last_line: h.new_start,
                reviewed_at: now.into(),
            });
            true
        }
    }
}

/// Mark or unmark every hunk of `file`.
pub fn set_file(marks: &mut Vec<HunkMark>, file: &FileDiff, reviewed: bool, now: &str) {
    let flags = reviewed_flags(marks, file);
    for (i, on) in flags.into_iter().enumerate() {
        if on != reviewed {
            toggle_hunk(marks, file, i, now);
        }
    }
}

/// Merge `other` into `base`: hunk marks by (file, hash), comments by (file, code, text).
/// Returns conflicts (same comment, different resolution); local values win.
pub fn merge(base: &mut ReviewState, other: ReviewState) -> Vec<String> {
    for m in other.hunks {
        if !base
            .hunks
            .iter()
            .any(|b| b.file == m.file && b.content_hash == m.content_hash)
        {
            base.hunks.push(m);
        }
    }
    let mut conflicts = Vec::new();
    for c in other.comments {
        let same = |b: &ReviewComment| {
            b.anchor.file_path == c.anchor.file_path
                && b.anchor.target_code == c.anchor.target_code
                && b.anchor.comment_text == c.anchor.comment_text
        };
        match base.comments.iter().find(|b| same(b)) {
            None => base.comments.push(c),
            Some(b) if b.status != c.status => conflicts.push(format!(
                "{}:{} status differs (local {:?}, imported {:?}); kept local",
                b.anchor.file_path, b.anchor.target_line_no, b.status, c.status
            )),
            Some(_) => {}
        }
    }
    conflicts
}

impl ReviewComment {
    /// Anchors to the hunk holding the comment's line; hash stays empty if the line isn't in the diff.
    pub fn new(anchor: CommentAnchor, files: &[FileDiff]) -> Self {
        let mut c = Self {
            last_line: anchor.target_line_no,
            status: if anchor.is_resolved {
                Status::Resolved
            } else {
                Status::Active
            },
            anchor,
            content_hash: String::new(),
            hunk_start: 0,
            line_idx: 0,
            remote_thread: None,
        };
        let line = c.anchor.target_line_no;
        let path = c.anchor.file_path.clone();
        'find: for f in files.iter().filter(|f| f.display_name() == path) {
            for h in &f.hunks {
                if let Some(i) = h.lines.iter().position(|l| l.new_line_num == Some(line)) {
                    c.bind(h, i);
                    break 'find;
                }
            }
        }
        c
    }

    fn bind(&mut self, h: &DiffHunk, idx: usize) {
        self.content_hash = hunk_hash(h);
        self.hunk_start = h.new_start;
        self.line_idx = idx;
    }

    fn settle(&mut self, path: &str, line: u32) {
        self.anchor.file_path = path.to_string();
        self.anchor.target_line_no = line;
        self.last_line = line;
        self.status = if self.anchor.is_resolved {
            Status::Resolved
        } else {
            Status::Active
        };
    }
}

fn line_at(h: &DiffHunk, idx: usize) -> Option<u32> {
    h.lines.iter().skip(idx).find_map(|l| l.new_line_num)
}

/// Re-anchor after a diff reload: exact hunk hash (same file first, then nearest hunk),
/// then fuzzy match on the commented code plus context; otherwise orphaned. Never deletes.
pub fn reanchor(comments: &mut [ReviewComment], files: &[FileDiff]) {
    for c in comments.iter_mut() {
        let hit =
            (!c.content_hash.is_empty() && reanchor_exact(c, files)) || reanchor_fuzzy(c, files);
        if !hit {
            c.status = Status::Orphaned;
        }
    }
}

fn reanchor_exact(c: &mut ReviewComment, files: &[FileDiff]) -> bool {
    let mut best: Option<((bool, u32), &FileDiff, &DiffHunk)> = None;
    for f in files {
        let other = f.display_name() != c.anchor.file_path;
        for h in f.hunks.iter().filter(|h| hunk_hash(h) == c.content_hash) {
            let key = (other, h.new_start.abs_diff(c.hunk_start));
            if best.is_none_or(|(k, ..)| key < k) {
                best = Some((key, f, h));
            }
        }
    }
    let Some((_, f, h)) = best else { return false };
    let Some(line) = line_at(h, c.line_idx) else {
        return false;
    };
    c.hunk_start = h.new_start;
    c.settle(&f.display_name(), line);
    true
}

fn reanchor_fuzzy(c: &mut ReviewComment, files: &[FileDiff]) -> bool {
    let code = c.anchor.target_code.trim().to_string();
    if code.is_empty() {
        return false;
    }
    let path = c.anchor.file_path.clone();
    let near = c.anchor.target_line_no;
    let mut best: Option<(u32, &DiffHunk, usize, u32)> = None;
    for f in files.iter().filter(|f| f.display_name() == path) {
        for h in &f.hunks {
            for (i, l) in h.lines.iter().enumerate() {
                let Some(n) = l.new_line_num else { continue };
                if l.content.trim() != code || !context_matches(c, h, i) {
                    continue;
                }
                let d = n.abs_diff(near);
                if best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, h, i, n));
                }
            }
        }
    }
    let Some((_, h, i, n)) = best else {
        return false;
    };
    c.bind(h, i);
    c.settle(&path, n);
    true
}

/// Stored context must match the neighbours when present (legacy comments store none).
fn context_matches(c: &ReviewComment, h: &DiffHunk, i: usize) -> bool {
    let text = |j: usize| h.lines.get(j).map(|l| l.content.trim());
    let before = c
        .anchor
        .context_before
        .last()
        .is_none_or(|b| i.checked_sub(1).and_then(text) == Some(b.trim()));
    let after = c
        .anchor
        .context_after
        .first()
        .is_none_or(|a| text(i + 1) == Some(a.trim()));
    before && after
}

pub struct ReviewStore {
    dir: PathBuf,
    target: String,
}

fn file_stem(target: &str) -> String {
    target
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl ReviewStore {
    /// Opens the store for `target`, migrating legacy `comments.jsonl` on first use.
    pub fn open(repo: &Path, target: &str) -> Result<Self> {
        let s = Self {
            dir: repo.join(".lgtm").join("reviews"),
            target: target.to_string(),
        };
        std::fs::create_dir_all(&s.dir)?;
        migrate(repo, &s)?;
        Ok(s)
    }

    /// Like `open` but leaves legacy `comments.jsonl` alone (until comments move to this store).
    pub fn open_marks_only(repo: &Path, target: &str) -> Result<Self> {
        let s = Self {
            dir: repo.join(".lgtm").join("reviews"),
            target: target.to_string(),
        };
        std::fs::create_dir_all(&s.dir)?;
        Ok(s)
    }

    fn path(&self, target: &str) -> PathBuf {
        self.dir.join(format!("{}.json", file_stem(target)))
    }

    pub fn load(&self) -> Result<ReviewState> {
        let p = self.path(&self.target);
        if !p.is_file() {
            return Ok(ReviewState {
                version: VERSION,
                target: self.target.clone(),
                ..Default::default()
            });
        }
        let st: ReviewState = serde_json::from_str(&std::fs::read_to_string(&p)?)
            .with_context(|| format!("invalid review state {}", p.display()))?;
        if st.version > VERSION {
            bail!(
                "review state {} has version {}, newer than supported {}; upgrade lgtm",
                p.display(),
                st.version,
                VERSION
            );
        }
        Ok(st)
    }

    pub fn save(&self, st: &ReviewState) -> Result<()> {
        self.write(&self.target, st)
    }

    fn write(&self, target: &str, st: &ReviewState) -> Result<()> {
        let st = ReviewState {
            version: VERSION,
            target: target.to_string(),
            ..st.clone()
        };
        let p = self.path(target);
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&st)? + "\n")?;
        std::fs::rename(tmp, p)?;
        Ok(())
    }
}

/// comments.jsonl -> reviews/<target>.json (PR comments go to `pr-<n>`); original kept as `.bak`.
/// state.json only holds the turn counter, so it stays where it is.
fn migrate(repo: &Path, store: &ReviewStore) -> Result<()> {
    let legacy = repo.join(".lgtm").join("comments.jsonl");
    if !legacy.is_file() {
        return Ok(());
    }
    let mut groups: std::collections::BTreeMap<String, Vec<ReviewComment>> = Default::default();
    for a in CommentStore::new(repo).load_comments() {
        let t = a
            .pr_number
            .map_or_else(|| store.target.clone(), |n| format!("pr-{n}"));
        groups
            .entry(t)
            .or_default()
            .push(ReviewComment::new(a, &[]));
    }
    for (t, comments) in groups {
        store.write(
            &t,
            &ReviewState {
                comments,
                ..Default::default()
            },
        )?;
    }
    std::fs::rename(&legacy, legacy.with_extension("jsonl.bak"))?;
    Ok(())
}

#[cfg(test)]
mod tests;
