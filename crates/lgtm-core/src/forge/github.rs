//! GitHub forge: native git fetch for PR diffs, direct REST/GraphQL API with GITHUB_TOKEN, and gh fallback.
use super::Forge;
use crate::git::GitEngine;
use crate::github::{parse_threads, Bucket, Check, GhClient, GithubClient, Thread, THREADS_QUERY};
use crate::model::DiffSnapshot;
use crate::pr::{PrAuthor, PrEngine, PullRequestSummary};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

pub trait Runner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;
}

pub struct SystemRunner(pub PathBuf);

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let out = Command::new(program)
            .args(args)
            .current_dir(&self.0)
            .output()
            .with_context(|| format!("`{program}` is not installed or not on PATH"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            bail!(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

static BODY_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub struct GitHubForge {
    repo: PathBuf,
    remote: Option<Remote>,
    runner: Box<dyn Runner>,
    explicit_token: Option<String>,
}

impl GitHubForge {
    pub fn new(repo: PathBuf) -> Self {
        let remote = git2::Repository::open(&repo)
            .ok()
            .and_then(|r| r.find_remote("origin").ok()?.url().map(str::to_string))
            .and_then(|u| parse_remote(&u));
        let runner = Box::new(SystemRunner(repo.clone()));
        Self {
            repo,
            remote,
            runner,
            explicit_token: None,
        }
    }

    pub fn with_runner(repo: PathBuf, runner: Box<dyn Runner>) -> Self {
        let remote = git2::Repository::open(&repo)
            .ok()
            .and_then(|r| r.find_remote("origin").ok()?.url().map(str::to_string))
            .and_then(|u| parse_remote(&u));
        Self {
            repo,
            remote,
            runner,
            explicit_token: None,
        }
    }

    fn token(&self) -> Option<String> {
        if let Some(t) = &self.explicit_token {
            return Some(t.clone());
        }
        std::env::var("GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .ok()
            .filter(|t| !t.trim().is_empty())
    }

    fn curl(&self, url: &str, method: &str, body: Option<&Value>) -> Result<Value> {
        let Some(tok) = self.token() else {
            bail!("GITHUB_TOKEN is not set");
        };
        let seq = BODY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let hdr_file = std::env::temp_dir().join(format!("lgtm-gh-hdr-{pid}-{seq}"));
        let body_file = std::env::temp_dir().join(format!("lgtm-gh-body-{pid}-{seq}"));

        {
            use std::io::Write;
            let mut o = std::fs::OpenOptions::new();
            o.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
            writeln!(o.open(&hdr_file)?, "Authorization: Bearer {tok}")?;
        }

        let mut args = vec![
            "-sS",
            "--fail",
            "-H",
            "User-Agent: lgtm",
            "-H",
            "Accept: application/vnd.github+json",
        ];
        let hdr_arg = format!("@{}", hdr_file.display());
        args.extend(["-H", &hdr_arg]);

        let body_arg;
        if let Some(b) = body {
            {
                let mut o = std::fs::OpenOptions::new();
                o.write(true).create_new(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
                serde_json::to_writer(o.open(&body_file)?, b)?;
            }
            body_arg = format!("@{}", body_file.display());
            args.extend([
                "-H",
                "Content-Type: application/json",
                "-X",
                method,
                "-d",
                &body_arg,
            ]);
        }

        args.push(url);

        let out = self.runner.run("curl", &args);
        let _ = std::fs::remove_file(&hdr_file);
        if body.is_some() {
            let _ = std::fs::remove_file(&body_file);
        }

        let raw = out?;
        if raw.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&raw).context("unexpected GitHub API response")
    }
}

impl Forge for GitHubForge {
    fn available(&self) -> bool {
        self.token().is_some() || PrEngine::is_gh_installed()
    }

    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        if let Some(r) = &self.remote {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls?state=open&per_page=30",
                r.owner, r.repo
            );
            if let Ok(v) = self.curl(&url, "GET", None) {
                if let Some(arr) = v.as_array() {
                    let prs: Vec<PullRequestSummary> = arr
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
                            state: item["state"].as_str().unwrap_or("open").to_string(),
                        })
                        .collect();
                    return Ok(prs);
                }
            }
        }
        PrEngine::list_pull_requests()
    }

    /// Fast path: git fetch refs/pull/<n>/merge, then revision diff.
    /// Next path: git fetch refs/pull/<n>/head, then diff against base.
    /// Fallback: gh pr diff <n>.
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
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
        if fetched_merge.is_ok() {
            if let Ok(snap) =
                GitEngine::new(&self.repo).compute_revision_diff(&format!("{merge}^1...{merge}^2"))
            {
                return Ok(snap);
            }
        }

        let head = format!("refs/lgtm/pr/{pr}/head");
        let fetched_head = self.runner.run(
            "git",
            &[
                "fetch",
                "--quiet",
                "origin",
                &format!("+refs/pull/{pr}/head:{head}"),
            ],
        );
        if fetched_head.is_ok() {
            for base in &["origin/main", "origin/master", "main", "master", "HEAD"] {
                if let Ok(snap) =
                    GitEngine::new(&self.repo).compute_revision_diff(&format!("{base}...{head}"))
                {
                    return Ok(snap);
                }
            }
        }

        PrEngine::fetch_pr_diff(pr)
    }

    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        if let Some(r) = &self.remote {
            let ev = match event {
                "approve" => "APPROVE",
                "request-changes" => "REQUEST_CHANGES",
                _ => "COMMENT",
            };
            let body_val = serde_json::json!({
                "event": ev,
                "body": body,
            });
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/reviews",
                r.owner, r.repo
            );
            if self.curl(&url, "POST", Some(&body_val)).is_ok() {
                return Ok(());
            }
        }
        PrEngine::post_review(pr, event, body)
    }
}

impl GithubClient for GitHubForge {
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

    fn post_inline(&self, pr: u64, sha: &str, path: &str, line: u32, body: &str) -> Result<u64> {
        if let Some(r) = &self.remote {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/comments",
                r.owner, r.repo
            );
            let payload = serde_json::json!({
                "body": body,
                "commit_id": sha,
                "path": path,
                "line": line,
                "side": "RIGHT"
            });
            if let Ok(v) = self.curl(&url, "POST", Some(&payload)) {
                if let Some(id) = v["id"].as_u64() {
                    return Ok(id);
                }
            }
        }
        GhClient.post_inline(pr, sha, path, line, body)
    }

    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        if let Some(r) = &self.remote {
            let url = format!(
                "https://api.github.com/repos/{}/{}/issues/{pr}/comments",
                r.owner, r.repo
            );
            let payload = serde_json::json!({ "body": body });
            if let Ok(v) = self.curl(&url, "POST", Some(&payload)) {
                if let Some(id) = v["id"].as_u64() {
                    return Ok(id);
                }
            }
        }
        GhClient.post_general(pr, body)
    }

    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
        if let Some(r) = &self.remote {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls/{pr}/comments/{comment_id}/replies",
                r.owner, r.repo
            );
            let payload = serde_json::json!({ "body": body });
            if let Ok(v) = self.curl(&url, "POST", Some(&payload)) {
                if let Some(id) = v["id"].as_u64() {
                    return Ok(id);
                }
            }
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
            if self
                .curl("https://api.github.com/graphql", "POST", Some(&payload))
                .is_ok()
            {
                return Ok(());
            }
        }
        GhClient.set_resolved(pr, thread_id, resolved)
    }
}

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
            runner: Box::new(Fake(RefCell::new(vec![]), fakes)),
            explicit_token: Some("mock-token-for-test".into()),
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
}
