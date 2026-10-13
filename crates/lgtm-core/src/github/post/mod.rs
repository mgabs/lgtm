//! Resumable posting of local PR comments: inline where the line is in the diff, otherwise one general comment.
use super::GithubClient;
use crate::comment::CommentAnchor;
use crate::model::FileDiff;
use anyhow::Result;
use std::time::Duration;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Indexes of comments whose line is part of the diff.
    pub inline: Vec<usize>,
    /// Indexes of comments outside the diff, posted together as one comment.
    pub general: Vec<usize>,
}

impl Plan {
    pub fn total(&self) -> usize {
        self.inline.len() + self.general.len()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub posted: usize,
    pub total: usize,
    pub error: Option<String>,
}

impl Report {
    pub fn summary(&self) -> String {
        match &self.error {
            Some(e) => format!("{} of {} posted; stopped: {}", self.posted, self.total, e),
            None => format!("{} of {} posted", self.posted, self.total),
        }
    }
}

fn in_diff(files: &[FileDiff], c: &CommentAnchor) -> bool {
    files.iter().any(|f| {
        f.display_name() == c.file_path
            && f.hunks
                .iter()
                .flat_map(|h| &h.lines)
                .any(|l| l.new_line_num == Some(c.target_line_no))
    })
}

/// Unposted, unresolved comments of this PR, split by whether they can be inline.
pub fn plan(comments: &[CommentAnchor], files: &[FileDiff], pr: u64) -> Plan {
    let mut p = Plan::default();
    for (i, c) in comments.iter().enumerate() {
        if c.pr_number != Some(pr) || c.remote_id.is_some() || c.is_resolved {
            continue;
        }
        if in_diff(files, c) {
            p.inline.push(i);
        } else {
            p.general.push(i);
        }
    }
    p
}

fn general_body(comments: &[CommentAnchor], idx: &[usize]) -> String {
    idx.iter()
        .map(|&i| {
            let c = &comments[i];
            format!(
                "**{}:{}**\n{}",
                c.file_path, c.target_line_no, c.comment_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn run(
    client: &dyn GithubClient,
    pr: u64,
    comments: &mut [CommentAnchor],
    plan: &Plan,
    delay: Duration,
    save: &mut dyn FnMut(&[CommentAnchor]) -> Result<()>,
    posted: &mut usize,
) -> Result<()> {
    let sha = client.head_sha(pr)?;
    for &i in &plan.inline {
        let c = &comments[i];
        let id = client.post_inline(pr, &sha, &c.file_path, c.target_line_no, &c.comment_text)?;
        comments[i].remote_id = Some(id);
        *posted += 1;
        save(comments)?;
        std::thread::sleep(delay);
    }
    if !plan.general.is_empty() {
        let id = client.post_general(pr, &general_body(comments, &plan.general))?;
        for &i in &plan.general {
            comments[i].remote_id = Some(id);
        }
        *posted += plan.general.len();
        save(comments)?;
    }
    Ok(())
}

/// Post `plan`, calling `save` after every success so a failed run resumes where it stopped.
/// Stops at the first error and reports progress.
pub fn post(
    client: &dyn GithubClient,
    pr: u64,
    comments: &mut [CommentAnchor],
    plan: &Plan,
    delay: Duration,
    mut save: impl FnMut(&[CommentAnchor]) -> Result<()>,
) -> Report {
    let mut r = Report {
        total: plan.total(),
        ..Default::default()
    };
    if let Err(e) = run(client, pr, comments, plan, delay, &mut save, &mut r.posted) {
        r.error = Some(e.to_string());
    }
    r
}

#[cfg(test)]
mod tests;
