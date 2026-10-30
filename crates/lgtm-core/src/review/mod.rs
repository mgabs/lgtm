//! Durable review state: hunk marks and comments keyed by content hash, not line numbers.
use crate::comment::CommentAnchor;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldMark {
    pub file: String,
    pub hunk_hash: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReviewState {
    pub version: u32,
    pub target: String,
    #[serde(default)]
    pub hunks: Vec<HunkMark>,
    #[serde(default)]
    pub comments: Vec<ReviewComment>,
    #[serde(default)]
    pub folds: Vec<FoldMark>,
}

/// Hash of the whitespace-normalized changed lines as a 64-bit integer.
pub fn hunk_hash_u64(h: &DiffHunk) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed_byte = |b: u8| {
        hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
    };

    let mut first_line = true;
    for l in &h.lines {
        let sign = match l.line_type {
            LineType::Addition => b'+',
            LineType::Deletion => b'-',
            _ => continue,
        };

        if !first_line {
            feed_byte(b'\n');
        }
        first_line = false;

        feed_byte(sign);

        let mut first_word = true;
        for word in l.content.split_whitespace() {
            if !first_word {
                feed_byte(b' ');
            }
            first_word = false;
            for &b in word.as_bytes() {
                feed_byte(b);
            }
        }
    }

    hash
}

/// Formats a 64-bit hunk hash as standard 16-hex characters.
pub fn format_hunk_hash(hash: u64) -> String {
    format!("{:016x}", hash)
}

/// Hash of the whitespace-normalized changed lines; ignores context, headers and line numbers.
pub fn hunk_hash(h: &DiffHunk) -> String {
    format_hunk_hash(hunk_hash_u64(h))
}

/// Hash of the hunk holding new-side line `line` of `path`, if that line is in the diff.
pub fn hunk_hash_at(files: &[FileDiff], path: &str, line: u32) -> Option<String> {
    files
        .iter()
        .filter(|f| f.display_name() == path)
        .flat_map(|f| &f.hunks)
        .find(|h| h.lines.iter().any(|l| l.new_line_num == Some(line)))
        .map(hunk_hash)
}

/// Per hunk of `file`: is there a mark with the same hash? Duplicate hashes pair with the
/// nearest unused mark by line, so identical hunks stay distinct.
pub fn reviewed_flags(marks: &[HunkMark], file: &FileDiff) -> Vec<bool> {
    let name = file.display_name();
    let file_marks: Vec<(usize, &HunkMark)> = marks
        .iter()
        .enumerate()
        .filter(|(_, m)| m.file == name)
        .collect();
    if file_marks.is_empty() {
        return vec![false; file.hunks.len()];
    }

    let hashes: Vec<String> = file.hunks.iter().map(hunk_hash).collect();
    let mut pairs = Vec::new();
    for (hi, h) in file.hunks.iter().enumerate() {
        for &(mi, m) in &file_marks {
            if m.content_hash == hashes[hi] {
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

/// Toggle a fold mark for an AST block inside a hunk.
/// Returns true if folded, false if unfolded.
pub fn toggle_fold_mark(
    folds: &mut Vec<FoldMark>,
    file: &str,
    hunk_hash: &str,
    start_line: u32,
    end_line: u32,
) -> bool {
    if let Some(pos) = folds.iter().position(|f| {
        f.file == file
            && f.hunk_hash == hunk_hash
            && f.start_line == start_line
            && f.end_line == end_line
    }) {
        folds.remove(pos);
        false
    } else {
        folds.push(FoldMark {
            file: file.to_string(),
            hunk_hash: hunk_hash.to_string(),
            start_line,
            end_line,
        });
        true
    }
}

/// Retrieve the folded block ranges for a specific hunk in a file.
pub fn load_hunk_folds(
    folds: &[FoldMark],
    file_name: &str,
    hunk_hash: &str,
) -> Vec<crate::model::BlockRange> {
    folds
        .iter()
        .filter(|f| f.file == file_name && f.hunk_hash == hunk_hash)
        .map(|f| crate::model::BlockRange {
            start_line: f.start_line,
            end_line: f.end_line,
        })
        .collect()
}

/// Merge `other` into `base`: hunk marks by (file, hash), comments by (file, code, text), folds by (file, hash, start, end).
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
    for f in other.folds {
        if !base.folds.iter().any(|b| {
            b.file == f.file
                && b.hunk_hash == f.hunk_hash
                && b.start_line == f.start_line
                && b.end_line == f.end_line
        }) {
            base.folds.push(f);
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

pub(crate) struct HunkIndex<'a> {
    by_hash: std::collections::HashMap<String, Vec<(&'a FileDiff, &'a DiffHunk)>>,
}

impl<'a> HunkIndex<'a> {
    pub fn build(files: &'a [FileDiff]) -> Self {
        let mut by_hash: std::collections::HashMap<String, Vec<(&'a FileDiff, &'a DiffHunk)>> =
            std::collections::HashMap::new();
        for f in files {
            for h in &f.hunks {
                let hash = hunk_hash(h);
                by_hash.entry(hash).or_default().push((f, h));
            }
        }
        Self { by_hash }
    }

    pub fn find_exact(&self, c: &ReviewComment) -> Option<(&'a FileDiff, &'a DiffHunk)> {
        let candidates = self.by_hash.get(&c.content_hash)?;
        candidates
            .iter()
            .min_by_key(|(f, h)| {
                let other = f.display_name() != c.anchor.file_path;
                (other, h.new_start.abs_diff(c.hunk_start))
            })
            .copied()
    }
}

fn reanchor_exact_indexed<'a>(c: &mut ReviewComment, index: &HunkIndex<'a>) -> bool {
    let Some((f, h)) = index.find_exact(c) else {
        return false;
    };
    let Some(line) = line_at(h, c.line_idx) else {
        return false;
    };
    c.hunk_start = h.new_start;
    c.settle(&f.display_name(), line);
    true
}

/// Re-anchor after a diff reload: exact hunk hash (same file first, then nearest hunk),
/// then fuzzy match on the commented code plus context; otherwise orphaned. Never deletes.
pub fn reanchor(comments: &mut [ReviewComment], files: &[FileDiff]) {
    let index = HunkIndex::build(files);
    for c in comments.iter_mut() {
        let hit = (!c.content_hash.is_empty() && reanchor_exact_indexed(c, &index))
            || reanchor_fuzzy(c, files);
        if !hit {
            c.status = Status::Orphaned;
        }
    }
}

/// Re-pins plain comments after a diff reload and flags the ones whose code is gone.
/// Comments on files absent from the diff are left alone (another diff mode may own them).
/// Returns true when any anchor changed.
pub fn reanchor_anchors(anchors: &mut [CommentAnchor], files: &[FileDiff]) -> bool {
    let mut changed = false;
    let index = HunkIndex::build(files);
    for a in anchors.iter_mut() {
        let in_file = || files.iter().filter(|f| f.display_name() == a.file_path);
        // Comments on deleted lines have no new-side anchor; they stay put.
        let on_deletion = in_file()
            .flat_map(|f| &f.hunks)
            .flat_map(|h| &h.lines)
            .any(|l| {
                l.new_line_num.is_none()
                    && l.old_line_num == Some(a.target_line_no)
                    && l.content.trim() == a.target_code.trim()
            });
        if on_deletion || in_file().next().is_none() {
            continue;
        }
        let mut rc = ReviewComment::new(a.clone(), &[]);
        let hit = (!rc.content_hash.is_empty() && reanchor_exact_indexed(&mut rc, &index))
            || reanchor_fuzzy(&mut rc, files);
        if !hit {
            rc.status = Status::Orphaned;
        }
        let orphaned = rc.status == Status::Orphaned;
        if rc.anchor.target_line_no != a.target_line_no || orphaned != a.orphaned {
            a.target_line_no = rc.anchor.target_line_no;
            a.orphaned = orphaned;
            changed = true;
        }
    }
    changed
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
    /// Opens the review store for `target`.
    pub fn open(repo: &Path, target: &str) -> Result<Self> {
        Self::open_marks_only(repo, target)
    }

    /// Opens the review store for `target` without touching `.lgtm/directives.jsonl`.
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

pub fn target_pr(target: &str) -> Option<u64> {
    target.strip_prefix("pr-")?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests;
