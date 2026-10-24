//! Did the code respond to a directive? Diff-based and advisory: never resolves anything.
use crate::comment::CommentAnchor;
use crate::model::FileDiff;
use crate::review::hunk_hash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The hunk is byte-identical to when the directive was written.
    Untouched,
    /// The file is still in the diff but the hunk no longer matches.
    Changed,
    /// The file dropped out of the diff.
    Gone,
    /// No baseline was recorded, so nothing can be said.
    Orphaned,
}

/// An agent's claim that it handled the directive (recorded only; a human resolves).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub note: String,
    pub turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Tracking {
    #[serde(default)]
    pub hunk_hash: Option<String>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub claim: Option<Claim>,
    #[serde(default)]
    pub remote_uncertain: bool,
}

pub fn classify(c: &CommentAnchor, files: &[FileDiff]) -> Outcome {
    let Some(base) = &c.track.hunk_hash else {
        return Outcome::Orphaned;
    };
    let mut in_file = false;
    for f in files.iter().filter(|f| f.display_name() == c.file_path) {
        in_file = true;
        if f.hunks.iter().any(|h| &hunk_hash(h) == base) {
            return Outcome::Untouched;
        }
    }
    if in_file {
        Outcome::Changed
    } else {
        Outcome::Gone
    }
}

/// Re-classify open directives from earlier turns; returns whether anything changed.
pub fn recheck(comments: &mut [CommentAnchor], files: &[FileDiff], turn: u64) -> bool {
    let mut dirty = false;
    for c in comments
        .iter_mut()
        .filter(|c| !c.is_resolved && c.created_at_turn < turn)
    {
        let o = Some(classify(c, files));
        if c.track.outcome != o {
            c.track.outcome = o;
            dirty = true;
        }
    }
    dirty
}

/// Record a claim on a directive; the outcome is untouched by it.
pub fn claim(c: &mut CommentAnchor, note: &str, turn: u64) {
    c.track.claim = Some(Claim {
        note: note.to_string(),
        turn,
    });
}

/// One-line status combining the observed outcome and any claim.
pub fn verdict(c: &CommentAnchor) -> &'static str {
    use Outcome::*;
    match (c.track.claim.is_some(), c.track.outcome) {
        (true, Some(Untouched)) => "claimed but not changed",
        (true, Some(Changed | Gone)) => "claimed, code changed",
        (true, _) => "claimed",
        (false, Some(Untouched)) => "untouched",
        (false, Some(Changed)) => "changed",
        (false, Some(Gone)) => "gone",
        (false, Some(Orphaned)) => "orphaned",
        (false, None) => "sent",
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub untouched: usize,
    pub changed: usize,
    pub gone: usize,
    pub orphaned: usize,
    pub claimed_unchanged: usize,
}

impl Summary {
    pub fn of(comments: &[CommentAnchor]) -> Self {
        let mut s = Self::default();
        for c in comments.iter().filter(|c| !c.is_resolved) {
            match c.track.outcome {
                Some(Outcome::Untouched) => s.untouched += 1,
                Some(Outcome::Changed) => s.changed += 1,
                Some(Outcome::Gone) => s.gone += 1,
                Some(Outcome::Orphaned) => s.orphaned += 1,
                None => {}
            }
            if c.track.claim.is_some() && c.track.outcome == Some(Outcome::Untouched) {
                s.claimed_unchanged += 1;
            }
        }
        s
    }

    pub fn line(&self) -> String {
        let extra = if self.claimed_unchanged > 0 {
            format!(" ({} claimed but not changed)", self.claimed_unchanged)
        } else {
            String::new()
        };
        format!(
            "{} untouched, {} changed, {} gone, {} orphaned{}",
            self.untouched, self.changed, self.gone, self.orphaned, extra
        )
    }
}

#[cfg(test)]
mod tests;
