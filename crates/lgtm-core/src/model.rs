use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl ChangeType {
    pub fn badge(&self) -> &'static str {
        match self {
            ChangeType::Modified => "M",
            ChangeType::Added => "A",
            ChangeType::Deleted => "D",
            ChangeType::Renamed => "R",
            ChangeType::Untracked => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    Context,
    Addition,
    Deletion,
    HunkHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
    pub old_line_num: Option<u32>,
    pub new_line_num: Option<u32>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct BlockRange {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
    #[serde(default)]
    pub is_folded: bool,
    #[serde(default)]
    pub folded_blocks: Vec<BlockRange>,
}

impl DiffHunk {
    pub fn hash_u64(&self) -> u64 {
        crate::review::hunk_hash_u64(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: Option<PathBuf>,
    pub new_path: PathBuf,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
    pub is_folded: bool,
    pub is_reviewed: bool,
}

impl FileDiff {
    pub fn display_name(&self) -> String {
        self.new_path.to_string_lossy().to_string()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSnapshot {
    pub files: Vec<FileDiff>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

impl DiffSnapshot {
    pub fn new(files: Vec<FileDiff>) -> Self {
        let total_additions = files.iter().map(|f| f.additions).sum();
        let total_deletions = files.iter().map(|f| f.deletions).sum();
        Self {
            files,
            total_additions,
            total_deletions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_snapshot_totals() {
        let file1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Added,
            additions: 15,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        let file2 = FileDiff {
            old_path: Some(PathBuf::from("src/old.rs")),
            new_path: PathBuf::from("src/old.rs"),
            change_type: ChangeType::Modified,
            additions: 5,
            deletions: 8,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        let snapshot = DiffSnapshot::new(vec![file1, file2]);
        assert_eq!(snapshot.total_additions, 20);
        assert_eq!(snapshot.total_deletions, 8);
        assert_eq!(snapshot.files.len(), 2);
    }

    #[test]
    fn test_change_type_badges() {
        assert_eq!(ChangeType::Added.badge(), "A");
        assert_eq!(ChangeType::Modified.badge(), "M");
        assert_eq!(ChangeType::Deleted.badge(), "D");
        assert_eq!(ChangeType::Renamed.badge(), "R");
        assert_eq!(ChangeType::Untracked.badge(), "?");
    }
}
