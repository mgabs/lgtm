use crate::git::GitEngine;
use crate::model::DiffSnapshot;
use crate::pr::PrEngine;
use anyhow::Result;

pub trait DiffSource {
    fn snapshot(&self) -> Result<DiffSnapshot>;
    fn describe(&self) -> String;
}

pub struct WorkingTreeSource<'a>(pub &'a GitEngine);

impl DiffSource for WorkingTreeSource<'_> {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        self.0.compute_diff_snapshot()
    }
    fn describe(&self) -> String {
        "Diff".to_string()
    }
}

pub struct RevisionSource<'a> {
    pub engine: &'a GitEngine,
    pub rev: &'a str,
}

impl DiffSource for RevisionSource<'_> {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        self.engine.compute_revision_diff(self.rev)
    }
    fn describe(&self) -> String {
        format!("Revision '{}'", self.rev)
    }
}

pub struct PullRequestSource(pub u64);

impl DiffSource for PullRequestSource {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        PrEngine::fetch_pr_diff(self.0)
    }
    fn describe(&self) -> String {
        format!("PR #{} diff", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_matches_status_wording() {
        let g = GitEngine::new(".");
        assert_eq!(WorkingTreeSource(&g).describe(), "Diff");
        assert_eq!(
            RevisionSource {
                engine: &g,
                rev: "HEAD~1"
            }
            .describe(),
            "Revision 'HEAD~1'"
        );
        assert_eq!(PullRequestSource(7).describe(), "PR #7 diff");
    }
}
