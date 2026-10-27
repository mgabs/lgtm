//! Which code host a repository lives on, and the PR operations lgtm needs from it.
use crate::comment::CommentSide;
use crate::github::{Check, Thread};
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

/// Provider-agnostic PR discussion and review capabilities (comments, threads, checks).
pub trait PrDiscussion {
    fn checks(&self, _pr: u64) -> Result<Vec<Check>> {
        bail!("CI checks are not supported by this provider")
    }
    fn threads(&self, _pr: u64) -> Result<Vec<Thread>> {
        bail!("Review threads are not supported by this provider")
    }
    /// Reconcile just one changed discussion when the provider supports it.
    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        Ok(self.threads(pr)?.into_iter().find(|t| t.id == id))
    }
    fn head_sha(&self, _pr: u64) -> Result<String> {
        bail!("Source head commit is not supported by this provider")
    }
    /// Inline review comment on the specified side of `path:line`; returns the comment id.
    fn post_inline(
        &self,
        _pr: u64,
        _sha: &str,
        _path: &str,
        _line: u32,
        _side: CommentSide,
        _body: &str,
    ) -> Result<u64> {
        bail!("Inline comments are not supported by this provider")
    }
    /// General PR conversation comment; returns the comment id.
    fn post_general(&self, _pr: u64, _body: &str) -> Result<u64> {
        bail!("General comments are not supported by this provider")
    }
    /// Reply to a comment (its numeric id) in a thread; returns the new comment id.
    fn reply(&self, _pr: u64, _thread_id: &str, _comment_id: u64, _body: &str) -> Result<u64> {
        bail!("Reply comments are not supported by this provider")
    }
    fn set_resolved(&self, _pr: u64, _thread_id: &str, _resolved: bool) -> Result<()> {
        bail!("Thread resolution is not supported by this provider")
    }
}

pub use PrDiscussion as DiscussionProvider;

impl<T: ?Sized + PrDiscussion> PrDiscussion for Box<T> {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        (**self).checks(pr)
    }
    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        (**self).threads(pr)
    }
    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        (**self).thread(pr, id)
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        (**self).head_sha(pr)
    }
    fn post_inline(
        &self,
        pr: u64,
        sha: &str,
        path: &str,
        line: u32,
        side: CommentSide,
        body: &str,
    ) -> Result<u64> {
        (**self).post_inline(pr, sha, path, line, side, body)
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        (**self).post_general(pr, body)
    }
    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
        (**self).reply(pr, thread_id, comment_id, body)
    }
    fn set_resolved(&self, pr: u64, thread_id: &str, resolved: bool) -> Result<()> {
        (**self).set_resolved(pr, thread_id, resolved)
    }
}

pub trait Forge: PrDiscussion {
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

impl<T: ?Sized + Forge> Forge for Box<T> {
    fn available(&self) -> bool {
        (**self).available()
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        (**self).list_prs()
    }
    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        (**self).list_prs_state(state)
    }
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        (**self).pr_diff(pr)
    }
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        (**self).post_review(pr, event, body)
    }
}

/// Concrete forge backend implementation with zero heap boxing.
pub enum ForgeImpl {
    GitHub(GitHubForge),
    Azure(AzureForge),
    Unsupported(Unsupported),
}

impl PrDiscussion for ForgeImpl {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        match self {
            Self::GitHub(f) => f.checks(pr),
            Self::Azure(f) => f.checks(pr),
            Self::Unsupported(f) => f.checks(pr),
        }
    }
    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        match self {
            Self::GitHub(f) => f.threads(pr),
            Self::Azure(f) => f.threads(pr),
            Self::Unsupported(f) => f.threads(pr),
        }
    }
    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        match self {
            Self::GitHub(f) => f.thread(pr, id),
            Self::Azure(f) => f.thread(pr, id),
            Self::Unsupported(f) => f.thread(pr, id),
        }
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        match self {
            Self::GitHub(f) => f.head_sha(pr),
            Self::Azure(f) => f.head_sha(pr),
            Self::Unsupported(f) => f.head_sha(pr),
        }
    }
    fn post_inline(
        &self,
        pr: u64,
        sha: &str,
        path: &str,
        line: u32,
        side: CommentSide,
        body: &str,
    ) -> Result<u64> {
        match self {
            Self::GitHub(f) => f.post_inline(pr, sha, path, line, side, body),
            Self::Azure(f) => f.post_inline(pr, sha, path, line, side, body),
            Self::Unsupported(f) => f.post_inline(pr, sha, path, line, side, body),
        }
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        match self {
            Self::GitHub(f) => f.post_general(pr, body),
            Self::Azure(f) => f.post_general(pr, body),
            Self::Unsupported(f) => f.post_general(pr, body),
        }
    }
    fn reply(&self, pr: u64, thread: &str, comment: u64, body: &str) -> Result<u64> {
        match self {
            Self::GitHub(f) => f.reply(pr, thread, comment, body),
            Self::Azure(f) => f.reply(pr, thread, comment, body),
            Self::Unsupported(f) => f.reply(pr, thread, comment, body),
        }
    }
    fn set_resolved(&self, pr: u64, thread: &str, resolved: bool) -> Result<()> {
        match self {
            Self::GitHub(f) => f.set_resolved(pr, thread, resolved),
            Self::Azure(f) => f.set_resolved(pr, thread, resolved),
            Self::Unsupported(f) => f.set_resolved(pr, thread, resolved),
        }
    }
}

impl Forge for ForgeImpl {
    fn available(&self) -> bool {
        match self {
            Self::GitHub(f) => f.available(),
            Self::Azure(f) => f.available(),
            Self::Unsupported(f) => f.available(),
        }
    }
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        match self {
            Self::GitHub(f) => f.list_prs(),
            Self::Azure(f) => f.list_prs(),
            Self::Unsupported(f) => f.list_prs(),
        }
    }
    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        match self {
            Self::GitHub(f) => f.list_prs_state(state),
            Self::Azure(f) => f.list_prs_state(state),
            Self::Unsupported(f) => f.list_prs_state(state),
        }
    }
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        match self {
            Self::GitHub(f) => f.pr_diff(pr),
            Self::Azure(f) => f.pr_diff(pr),
            Self::Unsupported(f) => f.pr_diff(pr),
        }
    }
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        match self {
            Self::GitHub(f) => f.post_review(pr, event, body),
            Self::Azure(f) => f.post_review(pr, event, body),
            Self::Unsupported(f) => f.post_review(pr, event, body),
        }
    }
}

/// Placeholder for operations a provider has not implemented yet; every call explains why.
pub struct Unsupported(pub &'static str);

impl Unsupported {
    fn no<T>(&self) -> Result<T> {
        bail!("{} pull requests are not supported yet", self.0)
    }
}

impl PrDiscussion for Unsupported {
    fn checks(&self, _: u64) -> Result<Vec<Check>> {
        self.no()
    }
    fn threads(&self, _: u64) -> Result<Vec<Thread>> {
        self.no()
    }
    fn head_sha(&self, _: u64) -> Result<String> {
        self.no()
    }
    fn post_inline(
        &self,
        _: u64,
        _: &str,
        _: &str,
        _: u32,
        _: CommentSide,
        _: &str,
    ) -> Result<u64> {
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
    let backend = match kind {
        ForgeKind::Azure => {
            ForgeImpl::Azure(azure::AzureForge::with_origin(repo.to_path_buf(), url))
        }
        _ => ForgeImpl::GitHub(GitHubForge::with_origin(repo.to_path_buf(), url)),
    };
    Box::new(TimedForge(backend))
}

/// Observe provider selection and fallbacks as one end-to-end operation.
pub struct TimedForge<F>(pub F);

impl<F: PrDiscussion> PrDiscussion for TimedForge<F> {
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
    fn post_inline(
        &self,
        pr: u64,
        sha: &str,
        path: &str,
        line: u32,
        side: CommentSide,
        body: &str,
    ) -> Result<u64> {
        crate::debug::measure("pr.post_inline", || {
            self.0.post_inline(pr, sha, path, line, side, body)
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

impl<F: Forge> Forge for TimedForge<F> {
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

    struct DummyProvider;
    impl PrDiscussion for DummyProvider {}
    impl Forge for DummyProvider {
        fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
            Ok(vec![])
        }
        fn pr_diff(&self, _pr: u64) -> Result<DiffSnapshot> {
            bail!("not implemented")
        }
        fn post_review(&self, _pr: u64, _event: &str, _body: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn pr_discussion_defaults_provide_clean_fallback() {
        let dummy = DummyProvider;
        assert!(dummy.checks(1).is_err());
        assert!(dummy.threads(1).is_err());
        assert!(dummy.head_sha(1).is_err());
        assert!(dummy
            .post_inline(1, "sha", "p", 1, CommentSide::Right, "body")
            .is_err());
        assert!(dummy.post_general(1, "body").is_err());
        assert!(dummy.reply(1, "t", 1, "body").is_err());
        assert!(dummy.set_resolved(1, "t", true).is_err());
    }

    #[test]
    fn forge_impl_and_timed_forge_wrap_without_double_boxing() {
        let unsupp = Unsupported("TestHost");
        let impl_unsupp = ForgeImpl::Unsupported(unsupp);
        assert!(!impl_unsupp.available());
        assert!(impl_unsupp.list_prs().is_err());

        let timed = TimedForge(impl_unsupp);
        assert!(!timed.available());
        assert!(timed.list_prs().is_err());
        assert!(timed.checks(1).is_err());
    }
}
