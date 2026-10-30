use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

static COMMENT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn new_id() -> String {
    let sequence = COMMENT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}-{now:x}-{sequence:x}", std::process::id())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum CommentSide {
    #[default]
    Right,
    Left,
}

impl CommentSide {
    pub fn as_github_str(&self) -> &'static str {
        match self {
            Self::Right => "RIGHT",
            Self::Left => "LEFT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentAnchor {
    /// Stable local identity, retained when code moves or the directive is edited.
    #[serde(default)]
    pub id: String,
    pub file_path: String,
    pub target_line_no: u32,
    pub target_code: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub comment_text: String,
    #[serde(default)]
    pub replies: Vec<String>,
    pub created_at_turn: u64,
    pub is_resolved: bool,
    /// PR this directive was written against; scopes saved directives when switching PRs.
    #[serde(default)]
    pub pr_number: Option<u64>,
    /// Set by re-anchoring when the commented code can no longer be found in the diff.
    #[serde(default)]
    pub orphaned: bool,
    /// Baseline and observed outcome used to tell whether the agent acted on a directive.
    #[serde(default)]
    pub track: crate::outcome::Tracking,
}

impl CommentAnchor {
    pub fn add_reply(&mut self, reply: impl Into<String>) {
        self.replies.push(reply.into());
    }

    pub fn toggle_resolved(&mut self) -> bool {
        self.is_resolved = !self.is_resolved;
        self.is_resolved
    }
}

pub struct CommentStore {
    comments_file: PathBuf,
}

impl CommentStore {
    pub fn new<P: AsRef<Path>>(repo_root: P) -> Self {
        let dir = repo_root.as_ref().join(".lgtm");
        let _ = create_dir_all(&dir);
        Self {
            comments_file: dir.join("directives.jsonl"),
        }
    }

    /// One-time move of the legacy `comments.jsonl`; never clobbers an existing store.
    fn migrate_legacy_file(&self) {
        let legacy = self.comments_file.with_file_name("comments.jsonl");
        if legacy.is_file() && !self.comments_file.exists() {
            let _ = std::fs::rename(&legacy, &self.comments_file);
        }
    }

    pub fn load_comments(&self) -> Vec<CommentAnchor> {
        let _timing = crate::debug::Span::new("comments.load_comments");
        self.migrate_legacy_file();
        if !self.comments_file.exists() {
            return Vec::new();
        }

        let file = match std::fs::File::open(&self.comments_file) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut comments: Vec<CommentAnchor> = reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<CommentAnchor>(&line).ok())
            .collect();
        // Persist the one-time migration before any background post can capture these ids.
        let mut migrated = false;
        for comment in &mut comments {
            if comment.id.is_empty() {
                comment.id = new_id();
                migrated = true;
            }
        }
        if migrated {
            let _ = self.save_comments(&comments);
        }
        comments
    }

    pub fn add_comment(&self, comment: &CommentAnchor) -> Result<()> {
        let _timing = crate::debug::Span::new("comments.add_comment");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.comments_file)
            .context("Failed to open comments.jsonl for writing")?;

        let json = serde_json::to_string(comment)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn save_comments(&self, comments: &[CommentAnchor]) -> Result<()> {
        let _timing = crate::debug::Span::new("comments.save_comments");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.comments_file)
            .context("Failed to open comments.jsonl for writing")?;

        for comment in comments {
            let json = serde_json::to_string(comment)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }

    pub fn compile_batch_review_prompt(comments: &[CommentAnchor], turn: u64) -> (String, PathBuf) {
        let mut prompt = format!("# LGTM Code Review (Turn #{})\n\n", turn);
        prompt.push_str("Please address the following review directives on the codebase:\n\n");

        for (i, c) in comments.iter().filter(|c| !c.is_resolved).enumerate() {
            prompt.push_str(&format!(
                "### Review Item #{}: `{}:{}`\n",
                i + 1,
                c.file_path,
                c.target_line_no
            ));
            prompt.push_str(&format!(
                "**Target Code:**\n```\n{}\n```\n",
                c.target_code.trim()
            ));
            prompt.push_str(&format!("**Directive:**\n{}\n", c.comment_text.trim()));
            for (r_i, reply) in c.replies.iter().enumerate() {
                prompt.push_str(&format!("  * **Reply #{}:** {}\n", r_i + 1, reply.trim()));
            }
            prompt.push('\n');
        }

        prompt.push_str("---\n*Generated by lgtm diff review engine.*");
        (prompt, PathBuf::from(format!(".lgtm/review-{}.md", turn)))
    }

    pub fn compile_pr_review_prompt(comments: &[CommentAnchor], pr_num: u64) -> (String, PathBuf) {
        let mut prompt = format!("# LGTM Pull Request Review (PR #{})\n\n", pr_num);
        let pr_comments: Vec<&CommentAnchor> = comments
            .iter()
            .filter(|c| !c.is_resolved && (c.pr_number == Some(pr_num) || c.pr_number.is_none()))
            .collect();

        if pr_comments.is_empty() {
            prompt.push_str("LGTM! No outstanding review directives or blocker comments.\n");
        } else {
            prompt
                .push_str("Please address the following review comments on this Pull Request:\n\n");
            for (i, c) in pr_comments.iter().enumerate() {
                prompt.push_str(&format!(
                    "### #{}: `{}:{}`\n",
                    i + 1,
                    c.file_path,
                    c.target_line_no
                ));
                prompt.push_str(&format!("```\n{}\n```\n", c.target_code.trim()));
                prompt.push_str(&format!("**Review Comment:** {}\n", c.comment_text.trim()));
                for (r_i, reply) in c.replies.iter().enumerate() {
                    prompt.push_str(&format!("  * **Reply #{}:** {}\n", r_i + 1, reply.trim()));
                }
                prompt.push('\n');
            }
        }

        prompt.push_str("---\n*Reviewed with lgtm TUI forge.*");
        (
            prompt,
            PathBuf::from(format!(".lgtm/pr-{}-review.md", pr_num)),
        )
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
    fn test_comment_store_crud() {
        let tmp = test_dir("comments");
        let store = CommentStore::new(&tmp);

        let mut c1 = CommentAnchor {
            id: crate::comment::new_id(),
            file_path: "src/main.rs".into(),
            target_line_no: 42,
            target_code: "let x = 1;".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Refactor this".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        };

        store.add_comment(&c1).unwrap();
        let loaded = store.load_comments();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].comment_text, "Refactor this");
        assert_eq!(loaded[0].replies.len(), 0);

        c1.add_reply("Done refactoring");
        assert_eq!(c1.replies.len(), 1);
        assert_eq!(c1.replies[0], "Done refactoring");

        let (prompt_turn, path_turn) =
            CommentStore::compile_batch_review_prompt(std::slice::from_ref(&c1), 1);
        assert!(prompt_turn.contains("Refactor this"));
        assert!(prompt_turn.contains("Reply #1:** Done refactoring"));
        assert_eq!(path_turn, PathBuf::from(".lgtm/review-1.md"));

        let is_res = c1.toggle_resolved();
        assert!(is_res);
        assert!(c1.is_resolved);

        let (resolved_turn, _) =
            CommentStore::compile_batch_review_prompt(std::slice::from_ref(&c1), 1);
        assert!(!resolved_turn.contains("Refactor this"));

        let c2 = CommentAnchor {
            id: crate::comment::new_id(),
            file_path: "src/lib.rs".into(),
            target_line_no: 10,
            target_code: "foo();".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Add unit test".into(),
            replies: vec!["Unit test added in tests.rs".into()],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: Some(99),
            orphaned: false,
            track: Default::default(),
        };

        store.save_comments(&[c1.clone(), c2.clone()]).unwrap();
        let loaded2 = store.load_comments();
        assert_eq!(loaded2.len(), 2);
        assert_eq!(loaded2[0].replies.len(), 1);
        assert_eq!(loaded2[1].replies.len(), 1);

        let (prompt_pr, path_pr) =
            CommentStore::compile_pr_review_prompt(std::slice::from_ref(&c2), 99);
        assert!(prompt_pr.contains("Add unit test"));
        assert!(prompt_pr.contains("Reply #1:** Unit test added in tests.rs"));
        assert_eq!(path_pr, PathBuf::from(".lgtm/pr-99-review.md"));

        let (empty_pr_prompt, _) = CommentStore::compile_pr_review_prompt(&[], 100);
        assert!(empty_pr_prompt.contains("LGTM!"));
    }
    #[test]
    fn legacy_directives_get_persisted_stable_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = CommentStore::new(dir.path());
        let legacy = serde_json::json!({"file_path":"a.rs", "target_line_no":1, "target_code":"code",
            "context_before":[], "context_after":[], "comment_text":"keep me", "created_at_turn":1, "is_resolved":false});
        std::fs::write(
            dir.path().join(".lgtm/directives.jsonl"),
            format!("{legacy}\n"),
        )
        .unwrap();
        let first = store.load_comments();
        let second = store.load_comments();
        assert_eq!(first.len(), 1);
        assert!(!first[0].id.is_empty());
        assert_eq!(first, second);
        assert_eq!(first[0].comment_text, "keep me");
        assert!(
            std::fs::read_to_string(dir.path().join(".lgtm/directives.jsonl"))
                .unwrap()
                .contains(&first[0].id)
        );
    }

    #[test]
    fn legacy_comments_file_migrates_once_to_directives() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join(".lgtm/comments.jsonl");
        let new_path = dir.path().join(".lgtm/directives.jsonl");
        std::fs::create_dir_all(dir.path().join(".lgtm")).unwrap();
        let legacy = serde_json::json!({"id":"abc","file_path":"a.rs","target_line_no":1,"target_code":"c",
            "context_before":[],"context_after":[],"comment_text":"migrate me","created_at_turn":1,"is_resolved":false});
        std::fs::write(&legacy_path, format!("{legacy}\n")).unwrap();
        let loaded = CommentStore::new(dir.path()).load_comments();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].comment_text, "migrate me");
        assert!(new_path.is_file());
        assert!(!legacy_path.exists());
        assert!(std::fs::read_to_string(&new_path)
            .unwrap()
            .contains("migrate me"));
    }

    #[test]
    fn legacy_comments_file_never_overwrites_existing_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".lgtm")).unwrap();
        let line = |t: &str| {
            format!(
                "{}\n",
                serde_json::json!({"id":t,"file_path":"a.rs","target_line_no":1,"target_code":"c",
            "context_before":[],"context_after":[],"comment_text":t,"created_at_turn":1,"is_resolved":false})
            )
        };
        std::fs::write(dir.path().join(".lgtm/comments.jsonl"), line("old")).unwrap();
        std::fs::write(dir.path().join(".lgtm/directives.jsonl"), line("new")).unwrap();
        let loaded = CommentStore::new(dir.path()).load_comments();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].comment_text, "new");
        assert!(dir.path().join(".lgtm/comments.jsonl").exists());
    }

    #[test]
    fn legacy_forge_sync_fields_are_ignored_and_not_reserialized() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".lgtm")).unwrap();
        let legacy = serde_json::json!({"id":"x1","file_path":"a.rs","target_line_no":1,"target_code":"c",
            "context_before":[],"context_after":[],"comment_text":"keep","created_at_turn":1,"is_resolved":false,
            "pr_number":7,"remote_id":99,"side":"LEFT","track":{"remote_uncertain":true}});
        std::fs::write(
            dir.path().join(".lgtm/directives.jsonl"),
            format!("{legacy}\n"),
        )
        .unwrap();
        let store = CommentStore::new(dir.path());
        let loaded = store.load_comments();
        assert_eq!(loaded.len(), 1);
        store.save_comments(&loaded).unwrap();
        let out = std::fs::read_to_string(dir.path().join(".lgtm/directives.jsonl")).unwrap();
        for key in ["remote_id", "remote_uncertain", "\"side\""] {
            assert!(!out.contains(key), "{key} leaked: {out}");
        }
    }

    #[test]
    fn comment_side_serde() {
        let json = r#"{"side":"LEFT"}"#;
        #[derive(Deserialize)]
        struct S {
            side: CommentSide,
        }
        let s: S = serde_json::from_str(json).unwrap();
        assert_eq!(s.side, CommentSide::Left);
        assert_eq!(s.side.as_github_str(), "LEFT");
        assert_eq!(CommentSide::Right.as_github_str(), "RIGHT");
    }
}
