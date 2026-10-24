//! Which code host a repository lives on, and the PR operations lgtm needs from it.
use crate::github::{Check, GithubClient, Thread};
use crate::model::DiffSnapshot;
use crate::pr::PullRequestSummary;
use anyhow::{bail, Result};
use std::path::Path;

pub mod azure;
pub mod failure;
pub mod github;

pub use azure::AzureForge;
pub use github::GitHubForge;

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
    origin_url(repo).map_or(ForgeKind::Unknown, |u| detect(&u))
}

pub(crate) fn origin_url(repo: &Path) -> Option<String> {
    git2::Repository::open(repo)
        .ok()
        .and_then(|r| r.find_remote("origin").ok()?.url().map(str::to_string))
}

pub(crate) struct PrivateTempFile(std::path::PathBuf);

impl PrivateTempFile {
    pub fn write(prefix: &str, contents: &[u8]) -> std::io::Result<Self> {
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        use std::io::Write;
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{seq}", std::process::id()));
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
        let mut file = o.open(&path)?;
        let guard = Self(path);
        file.write_all(contents)?;
        Ok(guard)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for PrivateTempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub trait Forge: GithubClient {
    fn available(&self) -> bool {
        true
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>>;
    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        if state == crate::pr::PullRequestState::Open {
            self.list_prs()
        } else {
            bail!("This provider does not support historical PR listing")
        }
    }
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot>;
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()>;
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
    let url = origin_url(repo);
    let kind = url.as_deref().map_or(ForgeKind::Unknown, detect);
    Box::new(TimedForge(match kind {
        ForgeKind::Azure => Box::new(azure::AzureForge::with_origin(repo.to_path_buf(), url)),
        _ => Box::new(GitHubForge::with_origin(repo.to_path_buf(), url)),
    }))
}

/// Observe provider selection and fallbacks as one end-to-end operation.
struct TimedForge(Box<dyn Forge>);
impl GithubClient for TimedForge {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        crate::debug::measure("pr.checks", || self.0.checks(pr))
    }
    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        crate::debug::measure("pr.threads", || self.0.threads(pr))
    }
    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        crate::debug::measure("pr.thread", || self.0.thread(pr, id))
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        crate::debug::measure("pr.head_sha", || self.0.head_sha(pr))
    }
    fn post_inline(&self, pr: u64, sha: &str, path: &str, line: u32, body: &str) -> Result<u64> {
        crate::debug::measure("pr.post_inline", || {
            self.0.post_inline(pr, sha, path, line, body)
        })
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        crate::debug::measure("pr.post_general", || self.0.post_general(pr, body))
    }
    fn reply(&self, pr: u64, thread: &str, comment: u64, body: &str) -> Result<u64> {
        crate::debug::measure("pr.reply", || self.0.reply(pr, thread, comment, body))
    }
    fn set_resolved(&self, pr: u64, thread: &str, resolved: bool) -> Result<()> {
        crate::debug::measure(
            if resolved {
                "pr.resolve"
            } else {
                "pr.unresolve"
            },
            || self.0.set_resolved(pr, thread, resolved),
        )
    }
}
impl Forge for TimedForge {
    fn available(&self) -> bool {
        self.0.available()
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        crate::debug::measure("pr.list", || self.0.list_prs())
    }
    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        crate::debug::measure("pr.list", || self.0.list_prs_state(state))
    }
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        crate::debug::measure("pr.diff", || self.0.pr_diff(pr))
    }
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        crate::debug::measure("pr.post_review", || self.0.post_review(pr, event, body))
    }
}

fn have_commits(repo: &Path, source: &str, target: &str) -> bool {
    let Ok(repo) = git2::Repository::open(repo) else {
        return false;
    };
    [source, target].iter().all(|sha| {
        git2::Oid::from_str(sha)
            .ok()
            .is_some_and(|oid| repo.find_commit(oid).is_ok())
    })
}

fn build_pr_diff(repo: &Path, spec: &str) -> Result<DiffSnapshot> {
    let result = crate::debug::measure("pr.build_diff", || {
        crate::git::GitEngine::new(repo).compute_revision_diff(spec)
    });
    if result.is_err() {
        crate::debug::command_failure("pr.build_diff", "diff_build", None, None);
    }
    result.map_err(|error| error.context("Building PR diff"))
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
