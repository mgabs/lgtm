use crate::model::{ChangeType, DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Closed includes merged/completed as well as closed/abandoned PRs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PullRequestState {
    #[default]
    Open,
    Closed,
    All,
}
impl PullRequestState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "all" => Ok(Self::All),
            _ => bail!("Expected open, closed, or all"),
        }
    }
    pub fn includes(self, state: &str) -> bool {
        let open = matches!(state.to_ascii_lowercase().as_str(), "open" | "active");
        match self {
            Self::Open => open,
            Self::Closed => matches!(
                state.to_ascii_lowercase().as_str(),
                "closed" | "merged" | "completed" | "abandoned"
            ),
            Self::All => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestSummary {
    pub number: u64,
    pub title: String,
    pub author: PrAuthor,
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    pub additions: usize,
    pub deletions: usize,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrAuthor {
    pub login: String,
}

pub struct PrEngine;

impl PrEngine {
    pub fn is_gh_installed() -> bool {
        static INSTALLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *INSTALLED.get_or_init(|| Command::new("gh").arg("--version").output().is_ok())
    }

    pub fn post_review(pr_number: u64, event: &str, body: &str) -> Result<()> {
        let _timing = crate::debug::Span::new("command.gh.pr.review");
        let flag = match event {
            "approve" | "APPROVE" => "--approve",
            "request-changes" | "request_changes" | "REQUEST_CHANGES" => "--request-changes",
            "comment" | "COMMENT" => "--comment",
            other => bail!("unknown review event '{other}'"),
        };

        let mut cmd = Command::new("gh");
        cmd.args(["pr", "review", &pr_number.to_string(), flag]);
        if !body.is_empty() {
            cmd.args(["-b", body]);
        }

        let output = cmd
            .output()
            .context("Failed to submit PR review via `gh`")?;
        if !output.status.success() {
            bail!("`gh pr review` returned non-zero status code");
        }
        Ok(())
    }

    pub fn parse_unified_diff(diff_text: &str) -> Result<DiffSnapshot> {
        let mut files: Vec<FileDiff> = Vec::with_capacity(32);
        let mut current_file: Option<FileDiff> = None;
        let mut current_hunk: Option<DiffHunk> = None;

        let mut current_old_line = 0u32;
        let mut current_new_line = 0u32;

        for raw_line in diff_text.lines() {
            if raw_line.starts_with("diff --git ") {
                if let Some(hunk) = current_hunk.take() {
                    if let Some(ref mut f) = current_file {
                        f.hunks.push(hunk);
                    }
                }
                if let Some(f) = current_file.take() {
                    files.push(f);
                }

                // Extract path: diff --git a/foo/bar.rs b/foo/bar.rs
                let mut words = raw_line.split_ascii_whitespace();
                let _diff = words.next();
                let _git = words.next();
                let a_raw = words.next();
                let b_raw = words.next();
                let b_path = b_raw
                    .map(|s| s.strip_prefix("b/").unwrap_or(s))
                    .unwrap_or("unknown");
                let a_path = a_raw.map(|s| s.strip_prefix("a/").unwrap_or(s));

                current_file = Some(FileDiff {
                    old_path: a_path.map(PathBuf::from),
                    new_path: PathBuf::from(b_path),
                    change_type: ChangeType::Modified,
                    additions: 0,
                    deletions: 0,
                    hunks: Vec::with_capacity(8),
                    is_folded: false,
                    is_reviewed: false,
                });
            } else if raw_line.starts_with("new file mode") {
                if let Some(ref mut f) = current_file {
                    f.change_type = ChangeType::Added;
                }
            } else if raw_line.starts_with("deleted file mode") {
                if let Some(ref mut f) = current_file {
                    f.change_type = ChangeType::Deleted;
                }
            } else if raw_line.starts_with("@@ ") {
                if let Some(hunk) = current_hunk.take() {
                    if let Some(ref mut f) = current_file {
                        f.hunks.push(hunk);
                    }
                }

                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@ [section]
                let (old_start, old_lines, new_start, new_lines) = parse_hunk_header(raw_line);
                current_old_line = old_start;
                current_new_line = new_start;

                let capacity = (new_lines.max(old_lines) as usize).min(1024);
                current_hunk = Some(DiffHunk {
                    header: raw_line.to_string(),
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    lines: Vec::with_capacity(capacity),
                    is_folded: false,
                    folded_blocks: Vec::new(),
                });
            } else if let Some(ref mut hunk) = current_hunk {
                if raw_line.starts_with('+') && !raw_line.starts_with("+++") {
                    if let Some(ref mut f) = current_file {
                        f.additions += 1;
                    }
                    hunk.lines.push(DiffLine {
                        line_type: LineType::Addition,
                        content: raw_line[1..].to_string(),
                        old_line_num: None,
                        new_line_num: Some(current_new_line),
                    });
                    current_new_line += 1;
                } else if raw_line.starts_with('-') && !raw_line.starts_with("---") {
                    if let Some(ref mut f) = current_file {
                        f.deletions += 1;
                    }
                    hunk.lines.push(DiffLine {
                        line_type: LineType::Deletion,
                        content: raw_line[1..].to_string(),
                        old_line_num: Some(current_old_line),
                        new_line_num: None,
                    });
                    current_old_line += 1;
                } else if raw_line.starts_with(' ') || raw_line.is_empty() {
                    let content = raw_line.strip_prefix(' ').unwrap_or(raw_line);
                    hunk.lines.push(DiffLine {
                        line_type: LineType::Context,
                        content: content.to_string(),
                        old_line_num: Some(current_old_line),
                        new_line_num: Some(current_new_line),
                    });
                    current_old_line += 1;
                    current_new_line += 1;
                }
            }
        }

        if let Some(hunk) = current_hunk.take() {
            if let Some(ref mut f) = current_file {
                f.hunks.push(hunk);
            }
        }
        if let Some(f) = current_file.take() {
            files.push(f);
        }

        Ok(DiffSnapshot::new(files))
    }
}

fn parse_range(s: &str) -> (u32, u32) {
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}

pub fn parse_hunk_header(header: &str) -> (u32, u32, u32, u32) {
    let mut old_start = 1;
    let mut old_lines = 1;
    let mut new_start = 1;
    let mut new_lines = 1;

    if let Some(first) = header.find("@@") {
        let after_first = &header[first + 2..];
        if let Some(second) = after_first.find("@@") {
            let inner = &after_first[..second];
            for word in inner.split_ascii_whitespace() {
                if let Some(rest) = word.strip_prefix('-') {
                    let (s, l) = parse_range(rest);
                    old_start = s;
                    old_lines = l;
                } else if let Some(rest) = word.strip_prefix('+') {
                    let (s, l) = parse_range(rest);
                    new_start = s;
                    new_lines = l;
                }
            }
        }
    }

    (old_start, old_lines, new_start, new_lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unified_diff() {
        let sample_diff = r#"diff --git a/src/lib.rs b/src/lib.rs
index e69de29..495d4f2 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn hello() {
-    println!("old");
+    println!("new");
+    println!("extra");
 }
"#;

        let snapshot = PrEngine::parse_unified_diff(sample_diff).unwrap();
        assert_eq!(snapshot.files.len(), 1);
        let file = &snapshot.files[0];
        assert_eq!(file.new_path, PathBuf::from("src/lib.rs"));
        assert_eq!(file.additions, 2);
        assert_eq!(file.deletions, 1);
        assert_eq!(file.hunks.len(), 1);
        assert_eq!(file.hunks[0].lines.len(), 5);
    }

    #[test]
    fn test_parse_multi_file_unified_diff() {
        let diff = r#"diff --git a/new_file.txt b/new_file.txt
new file mode 100644
--- /dev/null
+++ b/new_file.txt
@@ -0,0 +1,2 @@
+Line 1
+Line 2
diff --git a/deleted.txt b/deleted.txt
deleted file mode 100644
--- a/deleted.txt
+++ /dev/null
@@ -1,1 +0,0 @@
-Goodbye
"#;
        let snapshot = PrEngine::parse_unified_diff(diff).unwrap();
        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(snapshot.files[0].change_type, ChangeType::Added);
        assert_eq!(snapshot.files[0].additions, 2);
        assert_eq!(snapshot.files[1].change_type, ChangeType::Deleted);
        assert_eq!(snapshot.files[1].deletions, 1);
    }

    #[test]
    fn test_parse_hunk_header() {
        let (old_start, old_lines, new_start, new_lines) =
            parse_hunk_header("@@ -10,5 +20,8 @@ fn test()");
        assert_eq!(old_start, 10);
        assert_eq!(old_lines, 5);
        assert_eq!(new_start, 20);
        assert_eq!(new_lines, 8);

        // Edge case: single line without comma count
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), (1, 1, 1, 1));
        assert_eq!(parse_hunk_header("@@ -42 +99 @@"), (42, 1, 99, 1));
        assert_eq!(parse_hunk_header("invalid"), (1, 1, 1, 1));
    }

    #[test]
    fn state_parse_and_filtering() {
        for (text, state) in [
            ("open", PullRequestState::Open),
            ("closed", PullRequestState::Closed),
            ("all", PullRequestState::All),
        ] {
            assert_eq!(PullRequestState::parse(text).unwrap(), state);
            assert_eq!(state.as_str(), text);
        }
        assert!(PullRequestState::parse("merged").is_err());
        assert!(PullRequestState::Open.includes("ACTIVE"));
        assert!(!PullRequestState::Open.includes("merged"));
        assert!(PullRequestState::Closed.includes("Abandoned"));
        assert!(!PullRequestState::Closed.includes("open"));
        assert!(PullRequestState::All.includes("anything"));
    }

    #[test]
    fn post_review_rejects_unknown_event_before_running_gh() {
        let err = PrEngine::post_review(1, "bogus", "").unwrap_err();
        assert!(err.to_string().contains("unknown review event 'bogus'"));
    }

    #[test]
    fn parse_keeps_multiple_hunks_and_line_numbers() {
        let diff = "diff --git a/f.rs b/f.rs\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n@@ -10 +10 @@\n-x\n+y\n";
        let file = &PrEngine::parse_unified_diff(diff).unwrap().files[0];
        assert_eq!(file.hunks.len(), 2);
        assert_eq!((file.additions, file.deletions), (2, 2));
        let second = &file.hunks[1];
        assert_eq!(second.old_start, 10);
        assert_eq!(second.lines[0].old_line_num, Some(10));
        assert_eq!(second.lines[1].new_line_num, Some(10));
        assert_eq!(file.hunks[0].lines[2].new_line_num, Some(2));
    }
}
