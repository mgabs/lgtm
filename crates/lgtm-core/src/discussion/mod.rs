//! Draft-then-submit PR discussion: drafts are local, the forge is the source of truth.
use crate::comment::CommentSide;
use crate::forge::failure::{is_ambiguous_write, Failure, Kind};
use crate::forge::PrDiscussion;
use crate::github::Thread;
use serde::{Deserialize, Serialize};

pub mod store;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DraftKind {
    Thread {
        path: String,
        line: u32,
        side: CommentSide,
    },
    General,
    Reply {
        thread_id: String,
        comment_id: u64,
    },
    SetResolved {
        thread_id: String,
        resolved: bool,
    },
}

impl DraftKind {
    fn rank(&self) -> u8 {
        match self {
            Self::Thread { .. } => 0,
            Self::General => 1,
            Self::Reply { .. } => 2,
            Self::SetResolved { .. } => 3,
        }
    }

    fn thread_id(&self) -> Option<&str> {
        match self {
            Self::Reply { thread_id, .. } | Self::SetResolved { thread_id, .. } => Some(thread_id),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Thread { .. } => "thread",
            Self::General => "general",
            Self::Reply { .. } => "reply",
            Self::SetResolved { resolved: true, .. } => "resolve",
            Self::SetResolved { .. } => "unresolve",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub id: u64,
    #[serde(flatten)]
    pub kind: DraftKind,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub stale: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl Draft {
    /// `path:line` for inline drafts, otherwise the thread it targets.
    pub fn target(&self) -> String {
        match &self.kind {
            DraftKind::Thread { path, line, .. } => format!("{path}:{line}"),
            DraftKind::General => "PR conversation".into(),
            DraftKind::Reply { thread_id, .. } | DraftKind::SetResolved { thread_id, .. } => {
                format!("thread {thread_id}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftError {
    EmptyBody,
    InvalidLine,
    NoBody,
}

impl std::fmt::Display for DraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyBody => "Comment body is empty",
            Self::InvalidLine => "Line must be inside a diff hunk",
            Self::NoBody => "Resolve drafts have no text to edit",
        })
    }
}

impl std::error::Error for DraftError {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSet {
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    items: Vec<Draft>,
}

impl DraftSet {
    pub fn items(&self) -> &[Draft] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, id: u64) -> Option<&Draft> {
        self.items.iter().find(|d| d.id == id)
    }

    /// Adds a draft; for resolve/unresolve the last draft per thread wins.
    pub fn add(&mut self, kind: DraftKind, body: &str) -> Result<u64, DraftError> {
        let body = body.trim();
        match &kind {
            DraftKind::SetResolved { thread_id, .. } => {
                self.items.retain(|d| {
                    !matches!(&d.kind, DraftKind::SetResolved { thread_id: t, .. } if t == thread_id)
                });
            }
            DraftKind::Thread { line: 0, .. } => return Err(DraftError::InvalidLine),
            _ if body.is_empty() => return Err(DraftError::EmptyBody),
            _ => {}
        }
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(Draft {
            id,
            kind,
            body: body.to_string(),
            stale: None,
            error: None,
        });
        Ok(id)
    }

    /// Replaces the body of an existing draft and clears its stale/error state.
    pub fn edit(&mut self, id: u64, body: &str) -> Result<(), DraftError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(DraftError::EmptyBody);
        }
        if let Some(d) = self.items.iter_mut().find(|d| d.id == id) {
            if matches!(d.kind, DraftKind::SetResolved { .. }) {
                return Err(DraftError::NoBody);
            }
            d.body = body.to_string();
            d.stale = None;
            d.error = None;
        }
        Ok(())
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|d| d.id != id);
        self.items.len() != before
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Marks drafts targeting a thread that no longer exists; clears the mark when it returns.
    pub fn mark_stale(&mut self, threads: &[Thread]) {
        for d in &mut self.items {
            let Some(id) = d.kind.thread_id() else {
                continue;
            };
            d.stale = (!threads.iter().any(|t| t.id == id)).then(|| "thread not found".into());
        }
    }

    /// Draft ids in send order: threads, general, replies, resolves; stale drafts are skipped.
    pub fn plan(&self) -> Vec<u64> {
        let mut sendable: Vec<&Draft> = self.items.iter().filter(|d| d.stale.is_none()).collect();
        sendable.sort_by_key(|d| (d.kind.rank(), d.id));
        sendable.into_iter().map(|d| d.id).collect()
    }

    /// `path:line` of every pending inline draft, for gutter markers.
    pub fn inline_lines(&self) -> impl Iterator<Item = (&str, u32)> {
        self.items.iter().filter_map(|d| match &d.kind {
            DraftKind::Thread { path, line, .. } => Some((path.as_str(), *line)),
            _ => None,
        })
    }

    /// `n thread, n reply …` batch summary.
    pub fn summary(&self) -> String {
        let parts: Vec<String> = ["thread", "general", "reply", "resolve", "unresolve"]
            .iter()
            .filter_map(|l| {
                let n = self.items.iter().filter(|d| d.kind.label() == *l).count();
                (n > 0).then(|| format!("{n} {l}"))
            })
            .collect();
        if parts.is_empty() {
            "no drafts".into()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SubmitReport {
    /// Items accepted by the forge.
    pub sent: usize,
    /// Items found already present on the forge (never re-sent).
    pub deduped: usize,
    /// Items left in the draft set because they are stale.
    pub stale: usize,
    /// Items that failed, as `(draft id, message)`.
    pub failed: Vec<(u64, String)>,
}

impl SubmitReport {
    pub fn message(&self) -> String {
        let mut m = format!("Submitted {}", self.sent + self.deduped);
        if self.stale > 0 {
            m += &format!(", {} stale", self.stale);
        }
        if let Some((_, first)) = self.failed.first() {
            m += &format!(", {} failed: {first}", self.failed.len());
        }
        m
    }
}

/// What to do about a failure, for every class the forge can report.
pub fn remediation(error: &anyhow::Error) -> &'static str {
    const DIFF: &str = "the line is no longer in the PR diff; edit or discard the draft";
    if let Some(f) = error.downcast_ref::<Failure>() {
        match f.kind {
            Kind::Authentication => return "log in to the provider CLI and retry",
            Kind::AccessDenied => return "check your permission on this repository",
            Kind::Timeout | Kind::Network => return "check connectivity and submit again",
            _ => {}
        }
        if f.http_status == Some(422) {
            return DIFF;
        }
    }
    let text = format!("{error:#}").to_ascii_lowercase();
    if text.contains("422") || text.contains("unprocessable") {
        DIFF
    } else if text.contains("timed out") || text.contains("timeout") {
        "check connectivity and submit again"
    } else if text.contains("rate limit") {
        "wait for the rate limit to reset, then submit again"
    } else if text.contains("not found") || text.contains("404") {
        "the thread is gone; refresh and discard the draft"
    } else if text.contains("not supported") {
        "this provider does not support the operation"
    } else {
        "fix the cause and submit again"
    }
}

fn describe(draft: &Draft, error: &anyhow::Error) -> String {
    format!(
        "{} {}: {error:#} ({})",
        draft.kind.label(),
        draft.target(),
        remediation(error)
    )
}

/// True when the forge already shows this draft, so it must not be sent again.
fn already_present(draft: &Draft, threads: &[Thread]) -> bool {
    let has = |t: &Thread| t.comments.iter().any(|c| c.body.trim() == draft.body);
    match &draft.kind {
        DraftKind::Thread { path, line, .. } => threads
            .iter()
            .any(|t| &t.path == path && t.line == Some(*line) && has(t)),
        DraftKind::Reply { thread_id, .. } => threads.iter().any(|t| &t.id == thread_id && has(t)),
        DraftKind::SetResolved {
            thread_id,
            resolved,
        } => threads
            .iter()
            .any(|t| &t.id == thread_id && t.resolved == *resolved),
        DraftKind::General => false,
    }
}

fn send(
    forge: &dyn PrDiscussion,
    pr: u64,
    draft: &Draft,
    sha: &anyhow::Result<String>,
) -> anyhow::Result<()> {
    match &draft.kind {
        DraftKind::Thread { path, line, side } => match sha {
            Ok(sha) => forge
                .post_inline(pr, sha, path, *line, *side, &draft.body)
                .map(|_| ()),
            Err(e) => Err(anyhow::anyhow!("{e:#}")),
        },
        DraftKind::General => forge.post_general(pr, &draft.body).map(|_| ()),
        DraftKind::Reply {
            thread_id,
            comment_id,
        } => forge
            .reply(pr, thread_id, *comment_id, &draft.body)
            .map(|_| ()),
        DraftKind::SetResolved {
            thread_id,
            resolved,
        } => forge.set_resolved(pr, thread_id, *resolved),
    }
}

/// Sends every sendable draft; each item succeeds or fails on its own and only successes leave the set.
pub fn submit(
    forge: &dyn PrDiscussion,
    pr: u64,
    drafts: &mut DraftSet,
) -> anyhow::Result<SubmitReport> {
    let mut threads = forge.threads(pr)?;
    drafts.mark_stale(&threads);
    let mut report = SubmitReport {
        stale: drafts.items.iter().filter(|d| d.stale.is_some()).count(),
        ..Default::default()
    };
    let sha = if drafts
        .items
        .iter()
        .any(|d| d.stale.is_none() && matches!(d.kind, DraftKind::Thread { .. }))
    {
        forge.head_sha(pr)
    } else {
        Ok(String::new())
    };
    for id in drafts.plan() {
        let draft = drafts.get(id).cloned().expect("planned id exists");
        if already_present(&draft, &threads) {
            drafts.remove(id);
            report.deduped += 1;
            continue;
        }
        match send(forge, pr, &draft, &sha) {
            Ok(()) => {
                drafts.remove(id);
                report.sent += 1;
            }
            Err(e) => {
                // An ambiguous outcome may have landed; check fresh threads before keeping it.
                if is_ambiguous_write(&e) {
                    if let Ok(fresh) = forge.threads(pr) {
                        threads = fresh;
                        if already_present(&draft, &threads) {
                            drafts.remove(id);
                            report.deduped += 1;
                            continue;
                        }
                    }
                }
                let message = describe(&draft, &e);
                if let Some(d) = drafts.items.iter_mut().find(|d| d.id == id) {
                    d.error = Some(message.clone());
                }
                report.failed.push((id, message));
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
