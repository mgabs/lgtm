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
        if c.pr_number != Some(pr)
            || c.remote_id.is_some()
            || c.is_remote_uncertain()
            || c.is_resolved
        {
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
    let sha = if plan.inline.is_empty() {
        String::new()
    } else {
        client.head_sha(pr)?
    };
    for (position, &i) in plan.inline.iter().enumerate() {
        let c = &comments[i];
        let id = match client.post_inline(pr, &sha, &c.file_path, c.target_line_no, &c.comment_text)
        {
            Ok(id) => id,
            Err(err) => {
                if crate::forge::failure::is_ambiguous_write(&err) {
                    comments[i].set_remote_uncertain(true);
                    let _ = save(comments);
                    return Err(err.context(
                        "remote write outcome is uncertain; reconcile threads before retrying",
                    ));
                }
                return Err(err);
            }
        };
        comments[i].set_remote_uncertain(false);
        comments[i].remote_id = Some(id);
        *posted += 1;
        save(comments)?;
        if position + 1 < plan.inline.len() || !plan.general.is_empty() {
            let _timing = crate::debug::Span::new("pr.post_throttle");
            std::thread::sleep(delay);
        }
    }
    if !plan.general.is_empty() {
        let id = match client.post_general(pr, &general_body(comments, &plan.general)) {
            Ok(id) => id,
            Err(err) => {
                if crate::forge::failure::is_ambiguous_write(&err) {
                    for &i in &plan.general {
                        comments[i].set_remote_uncertain(true);
                    }
                    let _ = save(comments);
                    return Err(err.context(
                        "remote write outcome is uncertain; reconcile threads before retrying",
                    ));
                }
                return Err(err);
            }
        };
        for &i in &plan.general {
            comments[i].set_remote_uncertain(false);
            comments[i].remote_id = Some(id);
        }
        *posted += plan.general.len();
        save(comments)?;
    }
    Ok(())
}

/// Reconcile uncertain directives for `pr` against remote `threads`.
/// Attaches `remote_id` when the directive text is present on the remote line;
/// returns the number of directives confirmed as delivered.
pub fn reconcile(comments: &mut [CommentAnchor], threads: &[super::Thread], pr: u64) -> usize {
    let mut used: std::collections::HashSet<u64> =
        comments.iter().filter_map(|c| c.remote_id).collect();
    let mut confirmed = 0;
    for c in comments.iter_mut() {
        if c.pr_number != Some(pr) || !c.is_remote_uncertain() || c.remote_id.is_some() {
            continue;
        }
        let matched = threads
            .iter()
            .filter(|t| t.path == c.file_path && t.line == Some(c.target_line_no))
            .flat_map(|t| &t.comments)
            .find(|tc| {
                tc.id != 0 && !used.contains(&tc.id) && tc.body.trim() == c.comment_text.trim()
            })
            .map(|tc| tc.id);
        if let Some(id) = matched {
            c.remote_id = Some(id);
            c.set_remote_uncertain(false);
            used.insert(id);
            confirmed += 1;
        }
    }
    confirmed
}

/// Clear remaining `remote_uncertain` flags for `pr` after an explicit reviewer decision
/// (or after verifying the comment is absent remotely) so unconfirmed directives can be retried.
pub fn clear_uncertain(comments: &mut [CommentAnchor], pr: u64) -> usize {
    let mut cleared = 0;
    for c in comments.iter_mut() {
        if c.pr_number == Some(pr) && c.remote_id.is_none() && c.is_remote_uncertain() {
            c.set_remote_uncertain(false);
            cleared += 1;
        }
    }
    cleared
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
    let mut timing = crate::debug::Span::new("pr.post_batch");
    let mut r = Report {
        total: plan.total(),
        ..Default::default()
    };
    let result = run(client, pr, comments, plan, delay, &mut save, &mut r.posted);
    timing.result(&result);
    if let Err(e) = result {
        r.error = Some(e.to_string());
    }
    r
}

#[cfg(test)]
mod tests;
