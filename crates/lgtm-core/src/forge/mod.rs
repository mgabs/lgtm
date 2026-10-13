//! Which code host a repository lives on, and the PR operations lgtm needs from it.
use crate::github::{Check, GhClient, GithubClient, Thread};
use crate::model::DiffSnapshot;
use crate::pr::{PrEngine, PullRequestSummary};
use anyhow::{bail, Result};
use std::path::Path;

pub mod azure;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeKind {
    GitHub,
    Azure,
    Unknown,
}

/// Classifies a git remote URL.
pub fn detect(url: &str) -> ForgeKind {
    let u = url.to_ascii_lowercase();
    if u.contains("dev.azure.com") || u.contains(".visualstudio.com") || u.contains("/_git/") {
        ForgeKind::Azure
    } else if u.contains("github.com") || u.contains("github.") {
        ForgeKind::GitHub
    } else {
        ForgeKind::Unknown
    }
}

pub fn kind_of(repo: &Path) -> ForgeKind {
    let url = git2::Repository::open(repo)
        .ok()
        .and_then(|r| r.find_remote("origin").ok()?.url().map(str::to_string));
    url.map_or(ForgeKind::Unknown, |u| detect(&u))
}

pub trait Forge: GithubClient {
    fn available(&self) -> bool {
        true
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>>;
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot>;
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()>;
}

pub struct GitHubForge;

impl GithubClient for GitHubForge {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        GhClient.checks(pr)
    }
    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        GhClient.threads(pr)
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        GhClient.head_sha(pr)
    }
    fn post_inline(&self, pr: u64, sha: &str, path: &str, line: u32, body: &str) -> Result<u64> {
        GhClient.post_inline(pr, sha, path, line, body)
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        GhClient.post_general(pr, body)
    }
    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
        GhClient.reply(pr, thread_id, comment_id, body)
    }
    fn set_resolved(&self, pr: u64, thread_id: &str, resolved: bool) -> Result<()> {
        GhClient.set_resolved(pr, thread_id, resolved)
    }
}

impl Forge for GitHubForge {
    fn available(&self) -> bool {
        PrEngine::is_gh_installed()
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        PrEngine::list_pull_requests()
    }
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        PrEngine::fetch_pr_diff(pr)
    }
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        PrEngine::post_review(pr, event, body)
    }
}

/// Placeholder for operations a provider has not implemented yet; every call explains why.
pub struct Unsupported(pub &'static str);

impl Unsupported {
    fn no<T>(&self) -> Result<T> {
        bail!("{} pull requests are not supported yet", self.0)
    }
}

impl GithubClient for Unsupported {
    fn checks(&self, _: u64) -> Result<Vec<Check>> {
        self.no()
    }
    fn threads(&self, _: u64) -> Result<Vec<Thread>> {
        self.no()
    }
    fn head_sha(&self, _: u64) -> Result<String> {
        self.no()
    }
    fn post_inline(&self, _: u64, _: &str, _: &str, _: u32, _: &str) -> Result<u64> {
        self.no()
    }
    fn post_general(&self, _: u64, _: &str) -> Result<u64> {
        self.no()
    }
    fn reply(&self, _: u64, _: &str, _: u64, _: &str) -> Result<u64> {
        self.no()
    }
    fn set_resolved(&self, _: u64, _: &str, _: bool) -> Result<()> {
        self.no()
    }
}

impl Forge for Unsupported {
    fn available(&self) -> bool {
        false
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        self.no()
    }
    fn pr_diff(&self, _: u64) -> Result<DiffSnapshot> {
        self.no()
    }
    fn post_review(&self, _: u64, _: &str, _: &str) -> Result<()> {
        self.no()
    }
}

/// Provider for `repo`'s `origin`. Unknown hosts keep the GitHub (`gh`) behavior.
pub fn open(repo: &Path) -> Box<dyn Forge> {
    match kind_of(repo) {
        ForgeKind::Azure => Box::new(azure::AzureForge::new(repo.to_path_buf())),
        _ => Box::new(GitHubForge),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hosts() {
        for (url, want) in [
            ("https://dev.azure.com/o/p/_git/r", ForgeKind::Azure),
            (
                "https://tfs.corp.example/tfs/Col/p/_git/r",
                ForgeKind::Azure,
            ),
            ("git@ssh.dev.azure.com:v3/o/p/r", ForgeKind::Azure),
            ("https://o.visualstudio.com/p/_git/r", ForgeKind::Azure),
            ("git@github.com:a/b.git", ForgeKind::GitHub),
            ("git@github.personal:a/b.git", ForgeKind::GitHub),
            ("https://gitlab.com/a/b", ForgeKind::Unknown),
        ] {
            assert_eq!(detect(url), want, "{url}");
        }
    }

    #[test]
    fn unsupported_explains_and_is_unavailable() {
        let f = Unsupported("Azure DevOps");
        assert!(!f.available());
        assert!(f
            .list_prs()
            .unwrap_err()
            .to_string()
            .contains("Azure DevOps"));
        assert!(f.pr_diff(1).is_err() && f.checks(1).is_err());
        assert!(f.post_review(1, "x", "y").is_err());
    }

    #[test]
    fn repo_without_remote_is_unknown() {
        assert_eq!(kind_of(Path::new("/nonexistent")), ForgeKind::Unknown);
    }
}
