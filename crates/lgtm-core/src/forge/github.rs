//! GitHub forge: native Git diffs, authenticated API/CLI access, and anonymous public reads.
use super::failure::is_ambiguous_write;
use super::{post_each, Forge, InlineDraft, PrDiscussion};
use crate::comment::CommentSide;
use crate::github::{parse_threads, Bucket, Check, GhClient, Thread, THREADS_QUERY};
use crate::model::DiffSnapshot;
use crate::pr::{PrAuthor, PrEngine, PullRequestSummary};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::PathBuf;
#[cfg(test)]
use std::process::Command;

pub trait Runner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;
}

pub struct SystemRunner(pub PathBuf);

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        super::failure::run(&self.0, program, args)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Remote {
    pub owner: String,
    pub repo: String,
}

/// Parses GitHub owner and repository from SSH or HTTPS remote URLs.
pub fn parse_remote(url: &str) -> Option<Remote> {
    let u = url.trim().trim_end_matches(".git");
    let path = if let Some(idx) = u.find("://") {
        let after = &u[idx + 3..];
        let slash = after.find('/')?;
        &after[slash + 1..]
    } else {
        let colon = u.find(':')?;
        &u[colon + 1..]
    };
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.len() >= 2 {
        Some(Remote {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
        })
    } else {
        None
    }
}

pub fn parse_check_runs(v: &Value) -> Vec<Check> {
    v["check_runs"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|cr| {
            let name = cr["name"].as_str().unwrap_or("").to_string();
            let status = cr["status"].as_str().unwrap_or("");
            let conclusion = cr["conclusion"].as_str().unwrap_or("");
            let bucket = if status != "completed" {
                Bucket::Pending
            } else {
                match conclusion {
                    "success" => Bucket::Pass,
                    "failure" | "timed_out" => Bucket::Fail,
                    "cancelled" | "neutral" | "skipped" | "action_required" => Bucket::Skipped,
                    _ => Bucket::Pending,
                }
            };
            Check { name, bucket }
        })
        .collect()
}

type MergeKey = (PathBuf, Option<Remote>, u64);
type MissingMergeRefs =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<MergeKey, std::time::Instant>>>;
static MISSING_MERGES: std::sync::OnceLock<MissingMergeRefs> = std::sync::OnceLock::new();
const MISSING_MERGE_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

fn is_github_com_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
        || url.starts_with("http://github.com/")
        || url.starts_with("git@github.com:")
        || url.starts_with("ssh://git@github.com/")
        || url.starts_with("ssh://github.com/")
}

pub struct GitHubForge {
    repo: PathBuf,
    remote: Option<Remote>,
    is_github_com: bool,
    runner: Box<dyn Runner>,
    explicit_token: Option<String>,
    missing_merges: MissingMergeRefs,
}

impl GitHubForge {
    pub fn new(repo: PathBuf) -> Self {
        let url = super::origin_url(&repo);
        Self::with_origin(repo, url)
    }

    pub(crate) fn with_origin(repo: PathBuf, origin_url: Option<String>) -> Self {
        let remote = origin_url.as_deref().and_then(parse_remote);
        let is_github_com = origin_url.as_deref().is_some_and(is_github_com_url);
        let runner = Box::new(SystemRunner(repo.clone()));
        Self {
            repo,
            remote,
            is_github_com,
            runner,
            explicit_token: None,
            missing_merges: MISSING_MERGES.get_or_init(Default::default).clone(),
        }
    }

    pub fn with_runner(repo: PathBuf, runner: Box<dyn Runner>) -> Self {
        let origin_url = super::origin_url(&repo);
        let remote = origin_url.as_deref().and_then(parse_remote);
        let is_github_com = origin_url.as_deref().is_some_and(is_github_com_url);
        Self {
            repo,
            remote,
            is_github_com,
            runner,
            explicit_token: None,
            missing_merges: MISSING_MERGES.get_or_init(Default::default).clone(),
        }
    }

    /// Override the token used by direct API requests. `None` disables environment
    /// token lookup; authentication managed by `gh` remains independent.
    pub fn with_api_token(mut self, token: Option<String>) -> Self {
        self.explicit_token = Some(token.unwrap_or_default());
        self
    }

    fn merge_key(&self, pr: u64) -> MergeKey {
        (self.repo.clone(), self.remote.clone(), pr)
    }
    fn merge_ref_missing(&self, pr: u64) -> bool {
        self.missing_merges
            .lock()
            .unwrap()
            .get(&self.merge_key(pr))
            .is_some_and(|at| at.elapsed() < MISSING_MERGE_TTL)
    }
    fn remember_missing_merge(&self, pr: u64) {
        let mut cache = self.missing_merges.lock().unwrap();
        cache.retain(|_, at| at.elapsed() < MISSING_MERGE_TTL);
        cache.insert(self.merge_key(pr), std::time::Instant::now());
    }

    fn token(&self) -> Option<String> {
        if let Some(t) = &self.explicit_token {
            return (!t.trim().is_empty()).then(|| t.clone());
        }
        ["GITHUB_TOKEN", "GH_TOKEN"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|token| !token.trim().is_empty())
    }

    fn anonymous_remote(&self) -> bool {
        self.is_github_com
    }

    fn can_read_anonymously(&self, error: &anyhow::Error) -> bool {
        self.token().is_none()
            && self.anonymous_remote()
            && error
                .downcast_ref::<super::failure::Failure>()
                .is_some_and(|failure| {
                    matches!(
                        failure.kind,
                        super::failure::Kind::Authentication
                            | super::failure::Kind::ToolUnavailable
                    )
                })
    }

    fn anonymous_get(&self, url: &str, accept: &str) -> Result<String> {
        let _timing = crate::debug::Span::new("pr.anonymous_read");
        let header = format!("Accept: {accept}");
        // This path only performs GET requests and carries no authorization or body.
        self.runner
            .run("curl", &["-sS", "--fail", "-H", "User-Agent: lgtm", "-H", &header, url])
            .map_err(|error| {
                let detail = error.downcast_ref::<super::failure::Failure>()
                    .map(ToString::to_string).unwrap_or_else(|| "provider request failed".into());
                error.context(format!("Anonymous GitHub read failed: {detail}; private repositories require authentication, and public reads may be rate limited"))
            })
    }

    fn cli_list(&self, state: crate::pr::PullRequestState) -> Result<Vec<PullRequestSummary>> {
        let mut args = vec!["pr", "list", "--state"];
        if state == crate::pr::PullRequestState::Closed {
            args.extend(["all", "--search", "is:closed"]);
        } else {
            args.push(state.as_str());
        }
        args.extend([
            "--json",
            "number,title,author,headRefName,additions,deletions,state",
            "--limit",
            "30",
        ]);
        let _timing = crate::debug::Span::new("command.gh.pr.list");
        let raw = self.runner.run("gh", &args)?;
        serde_json::from_str(&raw).context("Failed to parse pull requests JSON from GitHub CLI")
    }

    fn curl(&self, url: &str, method: &str, body: Option<&Value>) -> Result<Value> {
        let raw = self.curl_raw(url, method, body, "application/vnd.github+json")?;
        if raw.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&raw).context("unexpected GitHub API response")
    }

    fn curl_raw(
        &self,
        url: &str,
        method: &str,
        body: Option<&Value>,
        accept: &str,
    ) -> Result<String> {
        let Some(tok) = self.token() else {
            bail!("GITHUB_TOKEN is not set");
        };
        let hdr_file = super::PrivateTempFile::write(
            "lgtm-gh-hdr",
            format!("Authorization: Bearer {tok}\n").as_bytes(),
        )?;

        let accept_header = format!("Accept: {accept}");
        let mut args = vec![
            "-sS",
            "--fail",
            "-H",
            "User-Agent: lgtm",
            "-H",
            &accept_header,
        ];
        let hdr_arg = format!("@{}", hdr_file.path().display());
        args.extend(["-H", &hdr_arg]);

        let body_file = match body {
            Some(b) => Some(super::PrivateTempFile::write(
                "lgtm-gh-body",
                &serde_json::to_vec(b)?,
            )?),
            None => None,
        };
        let body_arg = body_file
            .as_ref()
            .map(|f| format!("@{}", f.path().display()));
        if let Some(body_arg) = &body_arg {
            args.extend([
                "-H",
                "Content-Type: application/json",
                "-X",
                method,
                "-d",
                body_arg,
            ]);
        }

        args.push(url);

        self.runner.run("curl", &args)
    }
}

impl Forge for GitHubForge {
    fn available(&self) -> bool {
        self.token().is_some() || PrEngine::is_gh_installed()
    }

    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        self.list_prs_state(crate::pr::PullRequestState::Open)
    }

    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        let Some(r) = &self.remote else {
            return self.cli_list(state);
        };
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls?state={}&per_page=30",
            r.owner,
            r.repo,
            state.as_str()
        );
        let value = if self.token().is_some() {
            self.curl(&url, "GET", None)?
        } else {
            match self.cli_list(state) {
                Ok(prs) => return Ok(prs),
                Err(error) if self.can_read_anonymously(&error) => {
                    let raw = self.anonymous_get(&url, "application/vnd.github+json")?;
                    serde_json::from_str(&raw).context("Invalid public GitHub PR listing")?
                }
                Err(error) => return Err(error),
            }
        };
        let items = value
            .as_array()
            .context("GitHub PR listing was not an array")?;
        Ok(items
            .iter()
            .map(|item| PullRequestSummary {
                number: item["number"].as_u64().unwrap_or(0),
                title: item["title"].as_str().unwrap_or("").to_string(),
                author: PrAuthor {
                    login: item["user"]["login"].as_str().unwrap_or("").to_string(),
                },
                head_ref_name: item["head"]["ref"].as_str().unwrap_or("").to_string(),
                additions: 0,
                deletions: 0,
                state: if item["merged_at"].as_str().is_some() {
                    "merged".into()
                } else {
                    item["state"].as_str().unwrap_or("open").to_string()
                },
            })
            .collect())
    }

    /// Fast path: git fetch refs/pull/<n>/merge, then revision diff.
    /// Missing merge ref: retrieve the stored PR patch through the API or gh, including closed PRs.
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        if !self.merge_ref_missing(pr) {
            let merge = format!("refs/lgtm/pr/{pr}/merge");
            let fetched_merge = self.runner.run(
                "git",
                &[
                    "fetch",
                    "--quiet",
                    "origin",
                    &format!("+refs/pull/{pr}/merge:{merge}"),
                ],
            );
            match fetched_merge {
                Ok(_) => {
                    self.missing_merges
                        .lock()
                        .unwrap()
                        .remove(&self.merge_key(pr));
                    return super::build_pr_diff(&self.repo, &format!("{merge}^1...{merge}^2"));
                }
                Err(error) if super::failure::missing_ref(&error) => {
                    self.remember_missing_merge(pr);
                }
                Err(error) => return Err(error).context("Fetching PR merge ref"),
            }
        } else {
            let _cached = crate::debug::Span::new("pr.merge_ref_cache_hit");
        }
        // The stored PR patch remains meaningful for merged/closed PRs. Guessing a
        // current base branch can produce an empty or unrelated historical diff.
        if let Some(remote) = self.remote.as_ref().filter(|_| self.token().is_some()) {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}",
                remote.owner, remote.repo
            );
            let patch = self
                .curl_raw(&url, "GET", None, "application/vnd.github.diff")
                .context("Loading PR patch from GitHub")?;
            return PrEngine::parse_unified_diff(&patch);
        }
        let _timing = crate::debug::Span::new("command.gh.pr.diff");
        let patch = match self.runner.run("gh", &["pr", "diff", &pr.to_string()]) {
            Ok(patch) => patch,
            Err(error) if self.can_read_anonymously(&error) => {
                let remote = self
                    .remote
                    .as_ref()
                    .context("GitHub origin is unavailable")?;
                let url = format!(
                    "https://api.github.com/repos/{}/{}/pulls/{pr}",
                    remote.owner, remote.repo
                );
                self.anonymous_get(&url, "application/vnd.github.diff")?
            }
            Err(error) => return Err(error).context("Loading PR patch through GitHub CLI"),
        };
        PrEngine::parse_unified_diff(&patch)
    }

    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        if let Some(r) = self.remote.as_ref().filter(|_| self.token().is_some()) {
            let ev = match event {
                "approve" | "APPROVE" => "APPROVE",
                "request-changes" | "request_changes" | "REQUEST_CHANGES" => "REQUEST_CHANGES",
                "comment" | "COMMENT" => "COMMENT",
                other => bail!("unknown review event '{other}'"),
            };
            let body_val = serde_json::json!({
                "event": ev,
                "body": body,
            });
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/reviews",
                r.owner, r.repo
            );
            let receipt = self.curl(&url, "POST", Some(&body_val))?;
            receipt["id"]
                .as_u64()
                .context("Review response has no receipt; check the PR before retrying")?;
            return Ok(());
        }
        PrEngine::post_review(pr, event, body)
    }
}

impl PrDiscussion for GitHubForge {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        if let Some(r) = &self.remote {
            if let Ok(sha) = self.head_sha(pr) {
                let url = format!(
                    "https://api.github.com/repos/{}/{}/commits/{sha}/check-runs",
                    r.owner, r.repo
                );
                if let Ok(v) = self.curl(&url, "GET", None) {
                    return Ok(parse_check_runs(&v));
                }
            }
        }
        GhClient.checks(pr)
    }

    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        if let Some(r) = &self.remote {
            let payload = serde_json::json!({
                "query": THREADS_QUERY,
                "variables": {
                    "owner": r.owner,
                    "name": r.repo,
                    "n": pr
                }
            });
            if let Ok(v) = self.curl("https://api.github.com/graphql", "POST", Some(&payload)) {
                let s = serde_json::to_string(&v).unwrap_or_default();
                if let Ok(threads) = parse_threads(&s) {
                    return Ok(threads);
                }
            }
        }
        GhClient.threads(pr)
    }

    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        if self.token().is_some() {
            let payload =
                serde_json::json!({"query":crate::github::THREAD_QUERY,"variables":{"id":id}});
            if let Ok(value) = self.curl("https://api.github.com/graphql", "POST", Some(&payload)) {
                if let Ok(thread) = crate::github::parse_thread(&value.to_string()) {
                    return Ok(thread);
                }
            }
        }
        GhClient.thread(pr, id)
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        if let Some(r) = &self.remote {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}",
                r.owner, r.repo
            );
            if let Ok(v) = self.curl(&url, "GET", None) {
                if let Some(sha) = v.pointer("/head/sha").and_then(Value::as_str) {
                    return Ok(sha.to_string());
                }
            }
        }
        GhClient.head_sha(pr)
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
        if let Some(r) = self.remote.as_ref().filter(|_| self.token().is_some()) {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/comments",
                r.owner, r.repo
            );
            let payload = serde_json::json!({
                "body": body,
                "commit_id": sha,
                "path": path,
                "line": line,
                "side": side.as_github_str()
            });
            let receipt = self.curl(&url, "POST", Some(&payload))?;
            return receipt["id"]
                .as_u64()
                .context("Comment response has no receipt; check the thread before retrying");
        }
        GhClient.post_inline(pr, sha, path, line, side, body)
    }

    /// Two or more comments go out as one COMMENT review: one notification, all or nothing.
    /// A rejected review created nothing, so items then fall back to individual posts; an
    /// ambiguous outcome is reported for every item so the caller can check the forge first.
    fn post_inline_batch(&self, pr: u64, sha: &str, items: &[InlineDraft]) -> Vec<Result<()>> {
        let Some(r) = self
            .remote
            .as_ref()
            .filter(|_| items.len() > 1 && self.token().is_some())
        else {
            return post_each(self, pr, sha, items);
        };
        let comments: Vec<Value> = items
            .iter()
            .map(|i| {
                serde_json::json!({
                    "path": i.path,
                    "line": i.line,
                    "side": i.side.as_github_str(),
                    "body": i.body
                })
            })
            .collect();
        let payload = serde_json::json!({
            "commit_id": sha,
            "event": "COMMENT",
            "comments": comments
        });
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{pr}/reviews",
            r.owner, r.repo
        );
        let sent = self.curl(&url, "POST", Some(&payload)).and_then(|receipt| {
            receipt["id"]
                .as_u64()
                .map(|_| ())
                .context("Review response has no receipt; check the thread before retrying")
        });
        match sent {
            Ok(()) => items.iter().map(|_| Ok(())).collect(),
            Err(e) if is_ambiguous_write(&e) => items
                .iter()
                .map(|_| Err(anyhow::anyhow!("{e:#}")))
                .collect(),
            Err(_) => post_each(self, pr, sha, items),
        }
    }

    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        if let Some(r) = self.remote.as_ref().filter(|_| self.token().is_some()) {
            let url = format!(
                "https://api.github.com/repos/{}/{}/issues/{pr}/comments",
                r.owner, r.repo
            );
            let payload = serde_json::json!({ "body": body });
            let receipt = self.curl(&url, "POST", Some(&payload))?;
            return receipt["id"]
                .as_u64()
                .context("Comment response has no receipt; check the thread before retrying");
        }
        GhClient.post_general(pr, body)
    }

    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
        if let Some(r) = self.remote.as_ref().filter(|_| self.token().is_some()) {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/comments/{comment_id}/replies",
                r.owner, r.repo
            );
            let payload = serde_json::json!({ "body": body });
            let receipt = self.curl(&url, "POST", Some(&payload))?;
            return receipt["id"]
                .as_u64()
                .context("Comment response has no receipt; check the thread before retrying");
        }
        GhClient.reply(pr, thread_id, comment_id, body)
    }

    fn set_resolved(&self, pr: u64, thread_id: &str, resolved: bool) -> Result<()> {
        if self.token().is_some() {
            let op = if resolved {
                "resolveReviewThread"
            } else {
                "unresolveReviewThread"
            };
            let q = format!("mutation($id:ID!){{{op}(input:{{threadId:$id}}){{thread{{id}}}}}}");
            let payload = serde_json::json!({
                "query": q,
                "variables": { "id": thread_id }
            });
            let result = self.curl("https://api.github.com/graphql", "POST", Some(&payload))?;
            if result.get("errors").is_some() {
                bail!("GitHub did not confirm the thread update");
            }
            result
                .pointer(&format!("/data/{op}/thread/id"))
                .and_then(Value::as_str)
                .context("Thread update has no confirmation; refresh before retrying")?;
            return Ok(());
        }
        GhClient.set_resolved(pr, thread_id, resolved)
    }
}

#[cfg(test)]
mod performance_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_remote_urls() {
        assert_eq!(
            parse_remote("git@github.com:mgabs/lgtm.git"),
            Some(Remote {
                owner: "mgabs".into(),
                repo: "lgtm".into()
            })
        );
        assert_eq!(
            parse_remote("https://github.com/mgabs/lgtm"),
            Some(Remote {
                owner: "mgabs".into(),
                repo: "lgtm".into()
            })
        );
        assert_eq!(
            parse_remote("ssh://git@github.com/mgabs/lgtm.git"),
            Some(Remote {
                owner: "mgabs".into(),
                repo: "lgtm".into()
            })
        );
        assert_eq!(
            parse_remote("git@github-personal:owner/repo.git"),
            Some(Remote {
                owner: "owner".into(),
                repo: "repo".into()
            })
        );
    }

    #[test]
    fn parses_check_runs_json() {
        let json = serde_json::json!({
            "check_runs": [
                {
                    "name": "build",
                    "status": "completed",
                    "conclusion": "success"
                },
                {
                    "name": "test",
                    "status": "completed",
                    "conclusion": "failure"
                },
                {
                    "name": "lint",
                    "status": "in_progress",
                    "conclusion": null
                }
            ]
        });
        let checks = parse_check_runs(&json);
        assert_eq!(checks.len(), 3);
        assert_eq!(checks[0].bucket, Bucket::Pass);
        assert_eq!(checks[1].bucket, Bucket::Fail);
        assert_eq!(checks[2].bucket, Bucket::Pending);
    }

    use std::cell::RefCell;

    struct Fake(RefCell<Vec<String>>, Vec<(&'static str, &'static str)>);

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            let line = format!("{program} {}", args.join(" "));
            self.0.borrow_mut().push(line.clone());
            self.1
                .iter()
                .find(|(k, _)| line.contains(k))
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| anyhow::anyhow!("no fake for {line}"))
        }
    }

    fn test_forge(repo: &str, fakes: Vec<(&'static str, &'static str)>) -> GitHubForge {
        GitHubForge {
            repo: PathBuf::from(repo),
            remote: Some(Remote {
                owner: "mgabs".into(),
                repo: "lgtm".into(),
            }),
            is_github_com: true,
            runner: Box::new(Fake(RefCell::new(vec![]), fakes)),
            explicit_token: Some("mock-token-for-test".into()),
            missing_merges: Default::default(),
        }
    }

    #[test]
    fn diff_uses_the_merge_ref_without_calling_gh() {
        let dir = std::env::temp_dir().join(format!("lgtm-gh-merge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |a: &[&str]| {
            let ok = Command::new("git")
                .args(["-c", "user.email=a@b", "-c", "user.name=a"])
                .args(a)
                .current_dir(&dir)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {a:?}");
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join("a.txt"), "1\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "base"]);
        git(&["tag", "t-base"]);
        std::fs::write(dir.join("a.txt"), "1\n2\n").unwrap();
        git(&["commit", "-qam", "head"]);
        git(&["tag", "t-tip"]);
        git(&["checkout", "-q", "t-base"]);
        git(&["merge", "--no-ff", "-qm", "m", "t-tip"]);
        git(&["update-ref", "refs/lgtm/pr/8/merge", "HEAD"]);
        // only "git fetch" is faked: any `gh` call would error
        let f = test_forge(dir.to_str().unwrap(), vec![("git fetch", "")]);
        let snap = f.pr_diff(8).unwrap();
        assert_eq!((snap.files.len(), snap.total_additions), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_prs_through_rest() {
        let f = test_forge(
            ".",
            vec![(
                "pulls?state=open",
                r#"[{"number":12,"title":"New feature","user":{"login":"charlie"},"head":{"ref":"feat"},"state":"open"}]"#,
            )],
        );
        let prs = f.list_prs().unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 12);
        assert_eq!(prs[0].title, "New feature");
        assert_eq!(prs[0].author.login, "charlie");
        assert_eq!(prs[0].head_ref_name, "feat");
    }

    #[test]
    fn reads_threads_through_graphql() {
        let json = r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"id":"RT_1","isResolved":false,"path":"src/main.rs","line":15,"comments":{"nodes":[{"databaseId":42,"author":{"login":"dave"},"body":"Check this"}]}}]}}}}}"#;
        let f = test_forge(".", vec![("graphql", json)]);
        let threads = f.threads(12).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, "RT_1");
        assert_eq!(threads[0].path, "src/main.rs");
        assert_eq!(threads[0].line, Some(15));
        assert_eq!(threads[0].comments.len(), 1);
        assert_eq!(threads[0].comments[0].author, "dave");
        assert_eq!(threads[0].comments[0].body, "Check this");
    }

    #[test]
    fn reads_checks_through_rest() {
        let json_commit = r#"{"head":{"sha":"commit123"}}"#;
        let json_checks =
            r#"{"check_runs":[{"name":"ci/test","status":"completed","conclusion":"success"}]}"#;
        let f = test_forge(
            ".",
            vec![("pulls/12", json_commit), ("check-runs", json_checks)],
        );
        let checks = f.checks(12).unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "ci/test");
        assert_eq!(checks[0].bucket, Bucket::Pass);
    }
    #[test]
    fn fetch_authentication_and_network_errors_do_not_trigger_fallbacks() {
        use super::super::failure::{Failure, Kind};
        struct FailedFetch(Kind, std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Runner for FailedFetch {
            fn run(&self, program: &str, _args: &[&str]) -> Result<String> {
                assert_eq!(program, "git", "provider fallback must not run");
                self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Err(Failure {
                    kind: self.0,
                    exit_code: Some(128),
                    http_status: None,
                }
                .into())
            }
        }
        for kind in [
            Kind::Authentication,
            Kind::Network,
            Kind::Timeout,
            Kind::Other,
        ] {
            let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let mut provider = test_forge(".", vec![]);
            provider.runner = Box::new(FailedFetch(kind, calls.clone()));
            let error = provider.pr_diff(42).unwrap_err();
            assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
            assert_eq!(error.downcast_ref::<Failure>().unwrap().kind, kind);
        }
    }
    #[test]
    fn reads_one_thread_through_graphql() {
        let provider = test_forge(
            ".",
            vec![(
                "graphql",
                r#"{"data":{"node":{"id":"RT1","isResolved":false,"path":"a.rs","line":1,"comments":{"nodes":[{"databaseId":10,"author":{"login":"Ann"},"body":"reply"}]}}}}"#,
            )],
        );
        let thread = provider.thread(12, "RT1").unwrap().unwrap();
        assert_eq!(thread.id, "RT1");
        assert_eq!(thread.comments[0].author, "Ann");
    }

    /// Records `(url, json body)` of every curl call and answers per URL suffix.
    struct Recorder {
        log: std::rc::Rc<RefCell<Vec<(String, Value)>>>,
        reviews: fn() -> Result<String>,
    }

    impl Runner for Recorder {
        fn run(&self, _program: &str, args: &[&str]) -> Result<String> {
            let url = args.last().unwrap().to_string();
            let body = args
                .iter()
                .find_map(|a| a.strip_prefix('@').filter(|p| p.contains("lgtm-gh-body")))
                .map(|p| serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap())
                .unwrap_or(Value::Null);
            self.log.borrow_mut().push((url.clone(), body));
            if url.ends_with("/reviews") {
                (self.reviews)()
            } else {
                Ok(r#"{"id":7}"#.into())
            }
        }
    }

    type RequestLog = std::rc::Rc<RefCell<Vec<(String, Value)>>>;

    fn recording(reviews: fn() -> Result<String>) -> (GitHubForge, RequestLog) {
        let log = std::rc::Rc::new(RefCell::new(vec![]));
        let mut f = test_forge(".", vec![]);
        f.runner = Box::new(Recorder {
            log: log.clone(),
            reviews,
        });
        (f, log)
    }

    fn two() -> [InlineDraft<'static>; 2] {
        [
            InlineDraft {
                path: "a.rs",
                line: 1,
                side: CommentSide::Right,
                body: "x",
            },
            InlineDraft {
                path: "b.rs",
                line: 2,
                side: CommentSide::Left,
                body: "y",
            },
        ]
    }

    #[test]
    fn inline_batch_is_one_comment_review() {
        let (f, log) = recording(|| Ok(r#"{"id":5}"#.into()));
        let r = f.post_inline_batch(3, "sha1", &two());
        assert!(r.len() == 2 && r.iter().all(Result::is_ok));
        let log = log.borrow();
        assert_eq!(log.len(), 1);
        assert!(log[0].0.ends_with("/repos/mgabs/lgtm/pulls/3/reviews"));
        assert_eq!(
            log[0].1,
            serde_json::json!({"commit_id":"sha1","event":"COMMENT","comments":[
                {"path":"a.rs","line":1,"side":"RIGHT","body":"x"},
                {"path":"b.rs","line":2,"side":"LEFT","body":"y"}]})
        );
    }

    #[test]
    fn rejected_review_falls_back_to_individual_comments() {
        let (f, log) = recording(|| bail!("HTTP 422"));
        let r = f.post_inline_batch(3, "sha1", &two());
        assert!(r.iter().all(Result::is_ok));
        let urls: Vec<String> = log.borrow().iter().map(|(u, _)| u.clone()).collect();
        assert_eq!(urls.len(), 3);
        assert!(urls[0].ends_with("/reviews"));
        assert!(urls[1..].iter().all(|u| u.ends_with("/pulls/3/comments")));
    }

    #[test]
    fn review_without_receipt_is_ambiguous_and_never_resent() {
        let (f, log) = recording(|| Ok("{}".into()));
        let r = f.post_inline_batch(3, "sha1", &two());
        assert!(r
            .iter()
            .all(|x| is_ambiguous_write(x.as_ref().unwrap_err())));
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn timed_out_review_is_ambiguous_for_every_item() {
        let (f, log) = recording(|| bail!("request timed out"));
        let r = f.post_inline_batch(3, "sha1", &two());
        assert_eq!(r.len(), 2);
        assert!(r
            .iter()
            .all(|x| is_ambiguous_write(x.as_ref().unwrap_err())));
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn a_single_comment_is_posted_directly() {
        let (f, log) = recording(|| bail!("unused"));
        let r = f.post_inline_batch(3, "sha1", &two()[..1]);
        assert!(r.len() == 1 && r[0].is_ok());
        let log = log.borrow();
        assert_eq!(log.len(), 1);
        assert!(log[0].0.ends_with("/pulls/3/comments"));
    }

    #[test]
    fn ambiguous_writes_never_retry_through_the_cli() {
        struct TimedOut;
        impl Runner for TimedOut {
            fn run(&self, program: &str, _args: &[&str]) -> Result<String> {
                assert_eq!(program, "curl");
                Err(super::super::failure::Failure {
                    kind: super::super::failure::Kind::Timeout,
                    exit_code: Some(28),
                    http_status: None,
                }
                .into())
            }
        }
        let mut provider = test_forge(".", vec![]);
        provider.runner = Box::new(TimedOut);
        let failures = [
            provider
                .post_inline(1, "sha", "a.rs", 1, CommentSide::Right, "text")
                .map(|_| ()),
            provider.post_general(1, "text").map(|_| ()),
            provider.reply(1, "RT1", 1, "reply").map(|_| ()),
            provider.post_review(1, "COMMENT", "body"),
            provider.set_resolved(1, "RT1", true),
        ];
        for result in failures {
            assert_eq!(
                result
                    .unwrap_err()
                    .downcast_ref::<super::super::failure::Failure>()
                    .unwrap()
                    .kind,
                super::super::failure::Kind::Timeout
            );
        }
    }
    #[test]
    fn closed_and_all_listing_use_matching_api_filters() {
        let provider = test_forge(
            ".",
            vec![
                (
                    "state=closed",
                    r#"[{"number":1,"state":"closed","merged_at":"2026-10-22"},{"number":2,"state":"closed","merged_at":null}]"#,
                ),
                (
                    "state=all",
                    r#"[{"number":3,"state":"open"},{"number":2,"state":"closed"}]"#,
                ),
            ],
        );
        let closed = provider
            .list_prs_state(crate::pr::PullRequestState::Closed)
            .unwrap();
        assert_eq!(closed.len(), 2);
        assert_eq!(closed[0].state, "merged");
        assert_eq!(closed[1].state, "closed");
        assert_eq!(
            provider
                .list_prs_state(crate::pr::PullRequestState::All)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn missing_merge_ref_uses_stored_patch_instead_of_guessing_current_base() {
        struct StoredPatch;
        impl Runner for StoredPatch {
            fn run(&self, program: &str, args: &[&str]) -> Result<String> {
                if program == "git" {
                    assert!(
                        args.iter().any(|a| a.contains("/merge:")),
                        "no branch fetch should follow"
                    );
                    return Err(super::super::failure::classify(
                        "git",
                        Some(128),
                        "couldn't find remote ref",
                    )
                    .into());
                }
                assert_eq!(program, "curl");
                assert!(args.contains(&"Accept: application/vnd.github.diff"));
                Ok("diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1,2 @@\n base\n+historical change\n".into())
            }
        }
        let mut provider = test_forge(".", vec![]);
        provider.runner = Box::new(StoredPatch);
        let snapshot = provider.pr_diff(42).unwrap();
        assert_eq!(snapshot.total_additions, 1);
        assert_eq!(snapshot.files.len(), 1);
    }
}
