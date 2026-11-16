use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

static DIRECTIVE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn new_id() -> String {
    let sequence = DIRECTIVE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}-{now:x}-{sequence:x}", std::process::id())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directive {
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

impl Directive {
    pub fn add_reply(&mut self, reply: impl Into<String>) {
        self.replies.push(reply.into());
    }

    pub fn toggle_resolved(&mut self) -> bool {
        self.is_resolved = !self.is_resolved;
        self.is_resolved
    }

    pub fn from_guard_alert(alert: &crate::guard::GuardAlert) -> Self {
        Self {
            id: new_id(),
            file_path: alert.file_path.clone(),
            target_line_no: alert.line_no.unwrap_or(1),
            target_code: String::new(),
            context_before: Vec::new(),
            context_after: Vec::new(),
            comment_text: format!(
                "[{}] {}: {}",
                alert.severity.badge(),
                alert.title,
                alert.description
            ),
            replies: Vec::new(),
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        }
    }
}

pub struct DirectiveStore {
    directives_file: PathBuf,
}

impl DirectiveStore {
    pub fn new<P: AsRef<Path>>(repo_root: P) -> Self {
        let dir = repo_root.as_ref().join(".lgtm");
        let _ = create_dir_all(&dir); // lgtm:ignore
        Self {
            directives_file: dir.join("directives.jsonl"),
        }
    }

    /// One-time move of legacy `comments.jsonl`; never clobbers existing store.
    fn migrate_legacy_file(&self) {
        let legacy = self.directives_file.with_file_name("comments.jsonl");
        if legacy.is_file() && !self.directives_file.exists() {
            let _ = std::fs::rename(&legacy, &self.directives_file); // lgtm:ignore
        }
    }

    pub fn load_directives(&self) -> Vec<Directive> {
        let _timing = crate::debug::Span::new("directives.load_directives");
        self.migrate_legacy_file();
        if !self.directives_file.exists() {
            return Vec::new();
        }

        let file = match std::fs::File::open(&self.directives_file) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut directives: Vec<Directive> = reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<Directive>(&line).ok())
            .collect();

        let mut migrated = false;
        for directive in &mut directives {
            if directive.id.is_empty() {
                directive.id = new_id();
                migrated = true;
            }
        }
        if migrated {
            let _ = self.save_directives(&directives); // lgtm:ignore
        }
        directives
    }

    pub fn add_directive(&self, directive: &Directive) -> Result<()> {
        let _timing = crate::debug::Span::new("directives.add_directive");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.directives_file)
            .context("Failed to open directives.jsonl for writing")?;

        let json = serde_json::to_string(directive)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn save_directives(&self, directives: &[Directive]) -> Result<()> {
        let _timing = crate::debug::Span::new("directives.save_directives");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.directives_file)
            .context("Failed to open directives.jsonl for writing")?;

        for directive in directives {
            let json = serde_json::to_string(directive)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }

    pub fn add_alert_directive(&self, alert: &crate::guard::GuardAlert) -> Result<Directive> {
        let directive = Directive::from_guard_alert(alert);
        self.add_directive(&directive)?;
        Ok(directive)
    }

    pub fn compile_batch_review_prompt(directives: &[Directive], turn: u64) -> (String, PathBuf) {
        let mut prompt = format!("# LGTM Code Review (Turn #{})\n\n", turn);
        prompt.push_str("Please address the following review directives on the codebase:\n\n");

        for (i, d) in directives.iter().filter(|d| !d.is_resolved).enumerate() {
            prompt.push_str(&format!(
                "### Review Item #{}: `{}:{}`\n",
                i + 1,
                d.file_path,
                d.target_line_no
            ));
            prompt.push_str(&format!("**Directive:** {}\n", d.comment_text));
            if !d.replies.is_empty() {
                prompt.push_str("**Replies:**\n");
                for r in &d.replies {
                    prompt.push_str(&format!("- {}\n", r));
                }
            }
            prompt.push('\n');
        }

        prompt.push_str("---\n*Generated by lgtm diff review engine.*");
        (prompt, PathBuf::from(format!(".lgtm/review-{}.md", turn)))
    }

    pub fn compile_pr_review_prompt(directives: &[Directive], pr_num: u64) -> (String, PathBuf) {
        let mut prompt = format!("# LGTM Pull Request Review (PR #{})\n\n", pr_num);
        let pr_directives: Vec<&Directive> = directives
            .iter()
            .filter(|d| !d.is_resolved && (d.pr_number == Some(pr_num) || d.pr_number.is_none()))
            .collect();

        if pr_directives.is_empty() {
            prompt.push_str("LGTM! No outstanding review directives or blocker comments.\n");
        } else {
            prompt.push_str(
                "Please address the following review directives on this Pull Request:\n\n",
            );
            for (i, d) in pr_directives.iter().enumerate() {
                prompt.push_str(&format!(
                    "### #{}: `{}:{}`\n",
                    i + 1,
                    d.file_path,
                    d.target_line_no
                ));
                if !d.target_code.is_empty() {
                    prompt.push_str(&format!("```\n{}\n```\n", d.target_code.trim()));
                }
                prompt.push_str(&format!("**Directive:** {}\n", d.comment_text.trim()));
                for (r_i, reply) in d.replies.iter().enumerate() {
                    prompt.push_str(&format!("  * **Reply #{}:** {}\n", r_i + 1, reply.trim()));
                }
                prompt.push('\n');
            }
        }

        prompt.push_str("---\n*Reviewed with lgtm TUI.*");
        (
            prompt,
            PathBuf::from(format!(".lgtm/pr-{}-review.md", pr_num)),
        )
    }

    // Compatibility shims for transition
    pub fn load_comments(&self) -> Vec<Directive> {
        self.load_directives()
    }

    pub fn add_comment(&self, comment: &Directive) -> Result<()> {
        self.add_directive(comment)
    }

    pub fn save_comments(&self, comments: &[Directive]) -> Result<()> {
        self.save_directives(comments)
    }
}

// Backwards-compatible aliases
pub type CommentAnchor = Directive;
pub type CommentStore = DirectiveStore;

#[cfg(test)]
mod tests;
