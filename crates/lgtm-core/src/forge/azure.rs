//! Azure DevOps REST with cached CLI authentication and an `az` CLI fallback.
use super::{Forge, PrDiscussion};

use crate::comment::CommentSide;
use crate::github::{Bucket, Check, Thread, ThreadComment};
use crate::model::DiffSnapshot;
use crate::pr::{PrAuthor, PullRequestSummary};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(test)]
use std::process::Command;
use std::sync::{Arc, LazyLock, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

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
    pub org: String,
    pub project: String,
    pub repo: String,
}

fn unescape(s: &str) -> String {
    let mut decoded = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                decoded.push((high * 16 + low) as u8);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Parses the three Azure DevOps remote shapes (https, ssh `v3`, legacy `visualstudio.com`).
pub fn parse_remote(url: &str) -> Option<Remote> {
    let mk = |org: &str, project: &str, repo: &str| Remote {
        org: org.to_string(),
        project: unescape(project),
        repo: unescape(repo),
    };
    if let Some((_, rest)) = url.split_once("ssh.dev.azure.com:v3/") {
        let p: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
        return (p.len() == 3).then(|| mk(p[0], p[1], p[2]));
    }
    let rest = url.split_once("://")?.1;
    let (host, path) = rest.split_once('/')?;
    let host = host.rsplit('@').next()?;
    let p: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    if host == "dev.azure.com" && p.len() == 4 && p[2] == "_git" {
        return Some(mk(p[0], p[1], p[3]));
    }
    let org = host.strip_suffix(".visualstudio.com")?;
    let p: Vec<&str> = p
        .into_iter()
        .filter(|s| *s != "DefaultCollection")
        .collect();
    (p.len() == 3 && p[1] == "_git").then(|| mk(org, p[0], p[2]))
}

/// Bearer token shared by every forge instance in this process (a new forge is opened per command).
static TOKEN: std::sync::Mutex<Option<(String, std::time::Instant)>> = std::sync::Mutex::new(None);
const TOKEN_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);
type CacheEntry = (Instant, Arc<Value>);
type RestCache = RwLock<HashMap<String, CacheEntry>>;
/// REST read results by URL; cleared by any write so replies and resolves show up at once.
static CACHE: LazyLock<RestCache> = LazyLock::new(|| RwLock::new(HashMap::new()));
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

type MergeKey = (PathBuf, Option<Remote>, u64);
type MissingMergeRefs = Arc<Mutex<HashMap<MergeKey, Instant>>>;
static MISSING_MERGES: OnceLock<MissingMergeRefs> = OnceLock::new();
const MISSING_MERGE_TTL: Duration = Duration::from_secs(5 * 60);

pub struct AzureForge {
    repo: PathBuf,
    remote: Option<Remote>,
    runner: Box<dyn Runner>,
    /// Reuse the process-wide token (off in tests so each forge fetches its own).
    shared: bool,
    missing_merges: MissingMergeRefs,
}

impl AzureForge {
    pub fn new(repo: PathBuf) -> Self {
        let url = super::origin_url(&repo);
        Self::with_origin(repo, url)
    }

    pub fn with_runner(repo: PathBuf, runner: Box<dyn Runner>) -> Self {
        let remote = super::origin_url(&repo).as_deref().and_then(parse_remote);
        Self {
            repo,
            remote,
            runner,
            shared: false,
            missing_merges: Default::default(),
        }
    }

    pub(crate) fn with_origin(repo: PathBuf, origin_url: Option<String>) -> Self {
        let remote = origin_url.as_deref().and_then(parse_remote);
        let runner = Box::new(SystemRunner(repo.clone()));
        Self {
            repo,
            remote,
            runner,
            shared: true,
            missing_merges: MISSING_MERGES.get_or_init(Default::default).clone(),
        }
    }

    fn token(&self) -> Result<String> {
        if self.shared {
            if let Some((t, at)) = &*TOKEN.lock().unwrap() {
                if at.elapsed() < TOKEN_TTL {
                    return Ok(t.clone());
                }
            }
        }
        let t = self
            .runner
            .run(
                "az",
                &[
                    "account",
                    "get-access-token",
                    "--resource",
                    DEVOPS_RESOURCE,
                    "--query",
                    "accessToken",
                    "--output",
                    "tsv",
                ],
            )?
            .trim()
            .to_string();
        if t.is_empty() {
            bail!("az returned no access token");
        }
        if self.shared {
            *TOKEN.lock().unwrap() = Some((t.clone(), std::time::Instant::now()));
        }
        Ok(t)
    }

    /// GET against the Azure DevOps REST API with `curl` (about 100x faster than starting `az`).
    /// `scope` is `repo` (under the repository) or `project` (under the project). The bearer token
    /// goes through a private temp file so it never appears in the process list.
    fn rest(&self, scope: &str, path: &str) -> Result<Arc<Value>> {
        self.rest_read(scope, path, true)
    }

    fn rest_read(&self, scope: &str, path: &str, cached: bool) -> Result<Arc<Value>> {
        let Some(r) = &self.remote else {
            bail!("origin is not a recognized Azure DevOps URL");
        };
        let root = format!(
            "https://dev.azure.com/{}/{}/_apis",
            url_component(&r.org),
            url_component(&r.project)
        );
        let url = match scope {
            "repo" => format!("{root}/git/repositories/{}/{path}", url_component(&r.repo)),
            _ => format!("{root}/{path}"),
        };
        if self.shared && cached {
            let hit = {
                let cache = CACHE.read().unwrap();
                cache.get(&url).and_then(|(at, v)| {
                    if at.elapsed() < CACHE_TTL {
                        Some(Arc::clone(v))
                    } else {
                        None
                    }
                })
            };
            if let Some(v) = hit {
                return Ok(v);
            }
        }
        let token = self.token()?;
        let file = super::PrivateTempFile::write(
            "lgtm-hdr",
            format!("Authorization: Bearer {token}\n").as_bytes(),
        )?;
        let hdr = format!("@{}", file.path().display());
        let out = self
            .runner
            .run("curl", &["-sS", "--fail", "-H", &hdr, &url]);
        drop(file);
        let v: Value =
            serde_json::from_str(&out?).context("unexpected Azure DevOps REST response")?;
        let v = Arc::new(v);
        if self.shared {
            let mut cache = CACHE.write().unwrap();
            cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
            cache.insert(url, (Instant::now(), Arc::clone(&v)));
        }
        Ok(v)
    }

    fn pr_metadata(&self, pr: u64) -> Result<Arc<Value>> {
        crate::debug::measure("pr.fetch_metadata", || {
            self.rest_read("repo", &format!("pullRequests/{pr}?api-version=7.1"), false)
                .or_else(|_| {
                    let id = pr.to_string();
                    let org = self
                        .remote
                        .as_ref()
                        .map(|r| format!("https://dev.azure.com/{}", r.org));
                    let mut args = vec!["repos", "pr", "show", "--id", &id];
                    if let Some(org) = &org {
                        args.extend(["--organization", org]);
                    }
                    self.az_json(&args).map(Arc::new)
                })
        })
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
        cache.insert(self.merge_key(pr), Instant::now());
    }

    fn invalidate(&self) {
        if self.shared {
            CACHE.write().unwrap().clear();
        }
    }

    /// Policy evaluations need the project GUID, which comes from the PR itself.
    fn policies_rest(&self, pr: u64) -> Result<Value> {
        let p = self.rest("repo", &format!("pullRequests/{pr}?api-version=7.1"))?;
        let project = p
            .pointer("/repository/project/id")
            .and_then(Value::as_str)
            .context("PR has no project id")?;
        let artifact = format!("vstfs:///CodeReview/CodeReviewId/{project}/{pr}");
        let v = self.rest(
            "project",
            &format!("policy/evaluations?artifactId={artifact}&api-version=7.1-preview.1"),
        )?;
        Ok(v["value"].clone())
    }

    fn az_json(&self, args: &[&str]) -> Result<Value> {
        let mut full = args.to_vec();
        full.extend(["--detect", "true", "--output", "json"]);
        let text = self.runner.run("az", &full)?;
        serde_json::from_str(&text).context("unexpected `az` output")
    }

    /// REST call through `az devops invoke`, explicit about org, project and repository.
    fn invoke(&self, resource: &str, pr: u64) -> Result<Value> {
        self.call("get", resource, pr, None, None)
    }

    /// `az devops invoke` with an optional `threadId` route and JSON body (sent via a temp file).
    fn call(
        &self,
        method: &str,
        resource: &str,
        pr: u64,
        thread: Option<&str>,
        body: Option<&Value>,
    ) -> Result<Value> {
        let Some(r) = &self.remote else {
            bail!("origin is not a recognized Azure DevOps URL");
        };
        let org = format!("https://dev.azure.com/{}", r.org);
        let mut route = vec![
            format!("project={}", r.project),
            format!("repositoryId={}", r.repo),
            format!("pullRequestId={pr}"),
        ];
        if let Some(t) = thread {
            route.push(format!("threadId={t}"));
        }
        let file = match body {
            Some(b) => Some(super::PrivateTempFile::write(
                "lgtm-az",
                b.to_string().as_bytes(),
            )?),
            None => None,
        };
        if method != "get" {
            self.invalidate();
        }
        let mut args = vec![
            "devops",
            "invoke",
            "--area",
            "git",
            "--resource",
            resource,
            "--organization",
            &org,
            "--api-version",
            "7.1",
            "--http-method",
            method,
            "--output",
            "json",
        ];
        let path = file
            .as_ref()
            .map(|p| p.path().to_string_lossy().into_owned());
        if let Some(p) = &path {
            args.extend(["--in-file", p]);
        }
        args.push("--route-parameters");
        args.extend(route.iter().map(String::as_str));
        let out = self.runner.run("az", &args);
        drop(file);
        serde_json::from_str(&out?).context("unexpected `az` output")
    }

    fn new_thread(&self, pr: u64, body: &str, context: Option<Value>) -> Result<u64> {
        let mut t = serde_json::json!({
            "comments": [{"parentCommentId": 0, "content": body, "commentType": 1}],
            "status": 1,
        });
        if let Some(c) = context {
            t["threadContext"] = c;
        }
        let v = self.call("post", "pullRequestThreads", pr, None, Some(&t))?;
        v["id"]
            .as_u64()
            .context("Azure DevOps returned no thread id")
    }
}

fn branch(r: &str) -> &str {
    r.strip_prefix("refs/heads/").unwrap_or(r)
}

pub fn parse_prs(v: &Value) -> Vec<PullRequestSummary> {
    let s = |x: &Value, k: &str| x[k].as_str().unwrap_or_default().to_string();
    v.as_array()
        .into_iter()
        .flatten()
        .map(|p| PullRequestSummary {
            number: p["pullRequestId"].as_u64().unwrap_or(0),
            title: s(p, "title"),
            author: PrAuthor {
                login: s(&p["createdBy"], "displayName"),
            },
            head_ref_name: branch(p["sourceRefName"].as_str().unwrap_or_default()).to_string(),
            additions: 0,
            deletions: 0,
            state: s(p, "status"),
        })
        .collect()
}

/// Azure thread statuses that mean "done"; the rest (active, pending, unknown) stay open.
fn thread_closed(status: &str) -> bool {
    matches!(status, "fixed" | "closed" | "wontFix" | "byDesign")
}

pub fn parse_threads(v: &Value) -> Vec<Thread> {
    let s = |x: &Value, p: &str| {
        x.pointer(p)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    v["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| !t["isDeleted"].as_bool().unwrap_or(false))
        .filter_map(|t| {
            let comments: Vec<ThreadComment> = t["comments"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| {
                    c["commentType"].as_str() != Some("system")
                        && !c["isDeleted"].as_bool().unwrap_or(false)
                })
                .map(|c| ThreadComment {
                    id: c["id"].as_u64().unwrap_or(0),
                    author: s(c, "/author/displayName"),
                    body: s(c, "/content"),
                })
                .collect();
            if comments.is_empty() {
                return None;
            }
            Some(Thread {
                id: t["id"].as_u64().unwrap_or(0).to_string(),
                path: s(t, "/threadContext/filePath")
                    .trim_start_matches('/')
                    .to_string(),
                line: t
                    .pointer("/threadContext/rightFileStart/line")
                    .and_then(Value::as_u64)
                    .map(|l| l as u32),
                resolved: thread_closed(&s(t, "/status")),
                comments,
            })
        })
        .collect()
}

pub fn parse_policies(v: &Value) -> Vec<Check> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|p| Check {
            name: p["configuration"]["type"]["displayName"]
                .as_str()
                .unwrap_or("policy")
                .to_string(),
            bucket: match p["status"].as_str().unwrap_or_default() {
                "approved" => Bucket::Pass,
                "rejected" | "broken" => Bucket::Fail,
                "running" | "queued" => Bucket::Pending,
                _ => Bucket::Skipped,
            },
        })
        .collect()
}

impl Forge for AzureForge {
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        self.list_prs_state(crate::pr::PullRequestState::Open)
    }

    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        let statuses: &[&str] = match state {
            crate::pr::PullRequestState::Open => &["active"],
            crate::pr::PullRequestState::Closed => &["completed", "abandoned"],
            crate::pr::PullRequestState::All => &["all"],
        };
        let mut prs = Vec::new();
        for status in statuses {
            let page = match self.rest(
                "repo",
                &format!("pullrequests?searchCriteria.status={status}&api-version=7.1"),
            ) {
                Ok(value) => parse_prs(&value["value"]),
                Err(_) => parse_prs(&self.az_json(&["repos", "pr", "list", "--status", status])?),
            };
            prs.extend(page.into_iter().filter(|pr| state.includes(&pr.state)));
        }
        prs.sort_by_key(|pr| std::cmp::Reverse(pr.number));
        prs.dedup_by_key(|pr| pr.number);
        Ok(prs)
    }

    /// Read fresh metadata before Git; recorded commits avoid unavailable historical refs.
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        let v = self.pr_metadata(pr).context("Reading PR metadata")?;
        // Historical PRs may no longer have either branch. These commits describe the
        // source and target at the last PR merge attempt, including completed PRs.
        let source = v
            .pointer("/lastMergeSourceCommit/commitId")
            .and_then(Value::as_str);
        let target = v
            .pointer("/lastMergeTargetCommit/commitId")
            .and_then(Value::as_str);
        if let (Some(source), Some(target)) = (source, target) {
            git2::Oid::from_str(source).context("PR has an invalid recorded source commit")?;
            git2::Oid::from_str(target).context("PR has an invalid recorded target commit")?;
            if !super::have_commits(&self.repo, source, target) {
                let head = format!("refs/lgtm/pr/{pr}/head");
                let base = format!("refs/lgtm/pr/{pr}/base");
                self.runner.run("git", &["fetch", "--quiet", "origin", &format!("+{source}:{head}"), &format!("+{target}:{base}")])
                    .context("Fetching the PR's recorded commits; its branches may be deleted or its history unavailable")?;
            }
            return super::build_pr_diff(&self.repo, &format!("{target}...{source}"));
        }
        if matches!(v["status"].as_str(), Some("completed" | "abandoned")) {
            bail!("PR #{pr} is closed but its recorded source/target commits are unavailable; cannot safely reconstruct its historical diff");
        }
        if !self.merge_ref_missing(pr) {
            let merge = format!("refs/lgtm/pr/{pr}/merge");
            match self.runner.run(
                "git",
                &[
                    "fetch",
                    "--quiet",
                    "origin",
                    &format!("+refs/pull/{pr}/merge:{merge}"),
                ],
            ) {
                Ok(_) => {
                    self.missing_merges
                        .lock()
                        .unwrap()
                        .remove(&self.merge_key(pr));
                    return super::build_pr_diff(&self.repo, &format!("{merge}^1...{merge}^2"));
                }
                Err(error) if super::failure::missing_ref(&error) => {
                    self.remember_missing_merge(pr)
                }
                Err(error) => return Err(error).context("Fetching PR merge ref"),
            }
        } else {
            let _cached = crate::debug::Span::new("pr.merge_ref_cache_hit");
        }
        let (Some(src), Some(tgt)) = (v["sourceRefName"].as_str(), v["targetRefName"].as_str())
        else {
            bail!("PR #{pr} has no source or target branch");
        };
        let head = format!("refs/lgtm/pr/{pr}/head");
        let base = format!("refs/lgtm/pr/{pr}/base");
        self.runner.run(
            "git",
            &[
                "fetch",
                "--quiet",
                "origin",
                &format!("+{src}:{head}"),
                &format!("+{tgt}:{base}"),
            ],
        ).context("Fetching PR source/target branches; a branch may be deleted or belong to another repository")?;
        super::build_pr_diff(&self.repo, &format!("{base}...{head}"))
    }

    /// `APPROVE` and `REQUEST_CHANGES` set the vote (plus the body as a comment); `COMMENT` only comments.
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        let vote = match event {
            "APPROVE" | "approve" => Some("approve"),
            "REQUEST_CHANGES" | "request-changes" | "request_changes" => Some("wait-for-author"),
            "COMMENT" | "comment" => None,
            other => bail!("unknown review event '{other}'"),
        };
        if !body.is_empty() {
            self.post_general(pr, body)?;
        }
        if let Some(v) = vote {
            self.az_json(&[
                "repos",
                "pr",
                "set-vote",
                "--id",
                &pr.to_string(),
                "--vote",
                v,
            ])?;
            self.invalidate();
        }
        Ok(())
    }
}

impl PrDiscussion for AzureForge {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        if let Ok(v) = self.policies_rest(pr) {
            return Ok(parse_policies(&v));
        }
        Ok(parse_policies(&self.az_json(&[
            "repos",
            "pr",
            "policy",
            "list",
            "--id",
            &pr.to_string(),
        ])?))
    }
    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        if let Ok(v) = self.rest(
            "repo",
            &format!("pullRequests/{pr}/threads?api-version=7.1"),
        ) {
            return Ok(parse_threads(&v));
        }
        Ok(parse_threads(&self.invoke("pullRequestThreads", pr)?))
    }
    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        // Thread ids are numeric in Azure; reject unexpected routes before making a request.
        id.parse::<u64>().context("invalid Azure thread id")?;
        let value = match self.rest(
            "repo",
            &format!("pullRequests/{pr}/threads/{id}?api-version=7.1"),
        ) {
            Ok(value) => Arc::unwrap_or_clone(value),
            Err(_) => self.call("get", "pullRequestThreads", pr, Some(id), None)?,
        };
        Ok(parse_threads(&serde_json::json!({"value":[value]}))
            .into_iter()
            .next())
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        let v = self.pr_metadata(pr)?;
        v.pointer("/lastMergeSourceCommit/commitId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .context("PR has no source commit")
    }
    fn post_inline(
        &self,
        pr: u64,
        _sha: &str,
        path: &str,
        line: u32,
        side: CommentSide,
        body: &str,
    ) -> Result<u64> {
        let pos = serde_json::json!({"line": line, "offset": 1});
        let mut ctx = serde_json::json!({
            "filePath": format!("/{path}"),
        });
        match side {
            CommentSide::Left => {
                ctx["leftFileStart"] = pos.clone();
                ctx["leftFileEnd"] = pos;
            }
            CommentSide::Right => {
                ctx["rightFileStart"] = pos.clone();
                ctx["rightFileEnd"] = pos;
            }
        }
        self.new_thread(pr, body, Some(ctx))
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        self.new_thread(pr, body, None)
    }
    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
        thread_id
            .parse::<u64>()
            .context("invalid Azure thread id")?;
        let c =
            serde_json::json!({"parentCommentId": comment_id, "content": body, "commentType": 1});
        let v = self.call(
            "post",
            "pullRequestThreadComments",
            pr,
            Some(thread_id),
            Some(&c),
        )?;
        v["id"]
            .as_u64()
            .context("Azure DevOps returned no comment id")
    }
    fn set_resolved(&self, pr: u64, thread_id: &str, resolved: bool) -> Result<()> {
        thread_id
            .parse::<u64>()
            .context("invalid Azure thread id")?;
        let status = if resolved { "fixed" } else { "active" };
        let b = serde_json::json!({ "status": status });
        let v = self.call("patch", "pullRequestThreads", pr, Some(thread_id), Some(&b))?;
        v["id"]
            .as_u64()
            .context("Thread update has no confirmation; refresh before retrying")?;
        Ok(())
    }
}

#[cfg(test)]
static SHARED_CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod performance_tests;

#[cfg(test)]
mod tests {
    use super::*;
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
                .ok_or_else(|| {
                    if program == "git" && args.first() == Some(&"fetch") {
                        super::super::failure::classify(
                            "git",
                            Some(128),
                            "couldn't find remote ref",
                        )
                        .into()
                    } else {
                        anyhow::anyhow!("no fake for {line}")
                    }
                })
        }
    }

    fn forge(repo: &str, fakes: Vec<(&'static str, &'static str)>) -> AzureForge {
        AzureForge {
            repo: repo.into(),
            remote: parse_remote("https://dev.azure.com/org/My%20Proj/_git/repo"),
            runner: Box::new(Fake(RefCell::new(vec![]), fakes)),
            shared: false,
            missing_merges: Default::default(),
        }
    }

    #[test]
    fn parses_pr_list() {
        let f = forge(
            ".",
            vec![(
                "pr list",
                r#"[{"pullRequestId":7,"title":"T","status":"active","sourceRefName":"refs/heads/feat/x","createdBy":{"displayName":"Ann"}}]"#,
            )],
        );
        let prs = f.list_prs().unwrap();
        assert_eq!(
            (prs[0].number, prs[0].head_ref_name.as_str()),
            (7, "feat/x")
        );
        assert_eq!(
            (prs[0].author.login.as_str(), prs[0].state.as_str()),
            ("Ann", "active")
        );
    }

    #[test]
    fn maps_policies_to_checks() {
        let f = forge(
            ".",
            vec![(
                "policy list",
                r#"[{"status":"approved","configuration":{"type":{"displayName":"Build"}}},{"status":"rejected","configuration":{"type":{"displayName":"Min reviewers"}}},{"status":"queued","configuration":{"type":{"displayName":"B2"}}},{"status":"notApplicable"}]"#,
            )],
        );
        let c = f.checks(7).unwrap();
        let b: Vec<_> = c.iter().map(|x| x.bucket.clone()).collect();
        assert_eq!(
            b,
            [Bucket::Pass, Bucket::Fail, Bucket::Pending, Bucket::Skipped]
        );
        assert_eq!(c[1].name, "Min reviewers");
        assert_eq!(c[3].name, "policy");
    }

    #[test]
    fn diff_fetches_branches_then_diffs_locally() {
        let dir = std::env::temp_dir().join(format!("lgtm-az-{}", std::process::id()));
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
        git(&["update-ref", "refs/lgtm/pr/7/base", "HEAD"]);
        std::fs::write(dir.join("a.txt"), "1\n2\n").unwrap();
        git(&["commit", "-qam", "head"]);
        git(&["update-ref", "refs/lgtm/pr/7/head", "HEAD"]);
        // the fake "git fetch" succeeds; refs were prepared above
        let mut f = forge(
            dir.to_str().unwrap(),
            vec![
                (
                    "pr show",
                    r#"{"sourceRefName":"refs/heads/f","targetRefName":"refs/heads/main"}"#,
                ),
                ("git fetch", ""),
            ],
        );
        struct MissingMerge(Box<dyn Runner>);
        impl Runner for MissingMerge {
            fn run(&self, program: &str, args: &[&str]) -> Result<String> {
                if args.iter().any(|a| a.contains("/merge:")) {
                    return Err(super::super::failure::classify(
                        "git",
                        Some(128),
                        "couldn't find remote ref",
                    )
                    .into());
                }
                self.0.run(program, args)
            }
        }
        f.runner = Box::new(MissingMerge(f.runner));
        let snap = f.pr_diff(7).unwrap();
        assert_eq!(snap.files.len(), 1);
        assert_eq!(snap.total_additions, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn incomplete_active_metadata_can_still_use_the_merge_ref() {
        let dir = std::env::temp_dir().join(format!("lgtm-az-merge-{}", std::process::id()));
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
        let f = forge(
            dir.to_str().unwrap(),
            vec![("pr show", r#"{"status":"active"}"#), ("git fetch", "")],
        );
        let snap = f.pr_diff(8).unwrap();
        assert_eq!((snap.files.len(), snap.total_additions), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_branches_and_writes_are_errors() {
        let f = forge(".", vec![("pr show", "{}")]);
        assert!(f.pr_diff(1).unwrap_err().to_string().contains("no source"));
        assert!(f.post_review(1, "BOGUS", "y").is_err());
    }

    #[test]
    fn parses_remote_shapes() {
        let want = |o: &str, p: &str, r: &str| {
            Some(Remote {
                org: o.into(),
                project: p.into(),
                repo: r.into(),
            })
        };
        assert_eq!(
            parse_remote("https://dev.azure.com/o/P%20Q/_git/r"),
            want("o", "P Q", "r")
        );
        assert_eq!(
            parse_remote("https://u@dev.azure.com/o/p/_git/r/"),
            want("o", "p", "r")
        );
        assert_eq!(
            parse_remote("git@ssh.dev.azure.com:v3/o/p/r"),
            want("o", "p", "r")
        );
        assert_eq!(
            parse_remote("https://o.visualstudio.com/DefaultCollection/p/_git/r"),
            want("o", "p", "r")
        );
        assert_eq!(
            parse_remote("https://o.visualstudio.com/p/_git/r"),
            want("o", "p", "r")
        );
        assert_eq!(parse_remote("git@github.com:a/b.git"), None);
        assert_eq!(parse_remote("https://dev.azure.com/o/p"), None);
    }

    #[test]
    fn reads_threads_through_devops_invoke() {
        let json = r#"{"value":[
          {"id":3,"status":"active","threadContext":{"filePath":"/src/a.rs","rightFileStart":{"line":12}},
           "comments":[{"id":1,"author":{"displayName":"Ann"},"content":"fix","commentType":"text"}]},
          {"id":4,"status":"fixed","threadContext":null,
           "comments":[{"id":1,"author":{"displayName":"Bo"},"content":"general","commentType":"text"}]},
          {"id":5,"status":"active","comments":[{"id":1,"content":"x","commentType":"system"}]},
          {"id":6,"status":"active","isDeleted":true,"comments":[{"id":1,"content":"x","commentType":"text"}]}]}"#;
        let f = forge(".", vec![("pullRequestThreads", json)]);
        let t = f.threads(9).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(
            (
                t[0].id.as_str(),
                t[0].path.as_str(),
                t[0].line,
                t[0].resolved
            ),
            ("3", "src/a.rs", Some(12), false)
        );
        assert_eq!(t[0].comments[0].author, "Ann");
        assert!(t[1].resolved && t[1].line.is_none() && t[1].path.is_empty());
    }

    #[test]
    fn writes_go_through_devops_invoke_and_set_vote() {
        let f = forge(
            ".",
            vec![
                ("--http-method post --output json --in-file", r#"{"id":7}"#),
                ("--http-method patch", r#"{"id":5}"#),
                ("set-vote", "{}"),
                ("pr show", r#"{"lastMergeSourceCommit":{"commitId":"abc"}}"#),
            ],
        );
        assert_eq!(f.head_sha(2).unwrap(), "abc");
        assert_eq!(
            f.post_inline(2, "abc", "src/a.rs", 4, CommentSide::Right, "hi")
                .unwrap(),
            7
        );
        assert_eq!(f.post_general(2, "hi").unwrap(), 7);
        assert_eq!(f.reply(2, "5", 1, "ok").unwrap(), 7);
        f.set_resolved(2, "5", true).unwrap();
        f.post_review(2, "APPROVE", "lgtm").unwrap();
        f.post_review(2, "REQUEST_CHANGES", "").unwrap();
        assert!(f.post_review(2, "NOPE", "").is_err());
    }

    #[test]
    fn reads_go_through_curl_when_a_token_is_available() {
        let f = forge(
            ".",
            vec![
                ("get-access-token", "tok\n"),
                (
                    "pullrequests?searchCriteria",
                    r#"{"value":[{"pullRequestId":4,"title":"t","createdBy":{"displayName":"A"},"sourceRefName":"refs/heads/f","status":"active"}]}"#,
                ),
                (
                    "pullRequests/4/threads",
                    r#"{"value":[{"id":1,"status":"active","threadContext":{"filePath":"/a.rs","rightFileStart":{"line":2}},"comments":[{"id":1,"content":"x","commentType":"text"}]}]}"#,
                ),
                (
                    "pullRequests/4?api",
                    r#"{"repository":{"project":{"id":"guid"}}}"#,
                ),
                (
                    "policy/evaluations?artifactId=vstfs:///CodeReview/CodeReviewId/guid/4",
                    r#"{"value":[{"status":"approved","configuration":{"type":{"displayName":"Build"}}}]}"#,
                ),
            ],
        );
        assert_eq!(f.list_prs().unwrap()[0].number, 4);
        assert_eq!(f.threads(4).unwrap()[0].line, Some(2));
        let c = f.checks(4).unwrap();
        assert_eq!((c[0].name.as_str(), &c[0].bucket), ("Build", &Bucket::Pass));
    }

    #[test]
    fn reads_fall_back_to_az_when_rest_fails() {
        let f = forge(
            ".",
            vec![(
                "pr list",
                r#"[{"pullRequestId":9,"title":"t","status":"active"}]"#,
            )],
        );
        assert_eq!(f.list_prs().unwrap()[0].number, 9);
    }

    #[test]
    fn invoke_needs_a_recognized_remote() {
        let mut f = forge(".", vec![]);
        f.remote = None;
        assert!(f
            .threads(1)
            .unwrap_err()
            .to_string()
            .contains("not a recognized"));
    }
    #[test]
    fn fetch_authentication_and_network_errors_do_not_trigger_fallbacks() {
        use super::super::failure::{Failure, Kind};
        struct FailedFetch(Kind, std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Runner for FailedFetch {
            fn run(&self, program: &str, _args: &[&str]) -> Result<String> {
                if program == "az" {
                    if _args.contains(&"show") {
                        return Ok(r#"{"status":"active"}"#.into());
                    }
                    anyhow::bail!("REST token unavailable in test");
                }
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
            let mut provider = forge(".", vec![]);
            provider.runner = Box::new(FailedFetch(kind, calls.clone()));
            let error = provider.pr_diff(42).unwrap_err();
            assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
            assert_eq!(error.downcast_ref::<Failure>().unwrap().kind, kind);
            assert!(!provider.merge_ref_missing(42));
        }
    }
    #[test]
    fn reads_only_the_changed_thread() {
        let provider = forge(
            ".",
            vec![
                ("get-access-token", "test-token"),
                (
                    "pullRequests/12/threads/3",
                    r#"{"id":3,"status":"fixed","threadContext":{"filePath":"/a.rs","rightFileStart":{"line":1}},"comments":[{"id":2,"content":"reply","commentType":"text","author":{"displayName":"Ann"}}]}"#,
                ),
            ],
        );
        let thread = provider.thread(12, "3").unwrap().unwrap();
        assert_eq!(thread.id, "3");
        assert!(thread.resolved);
        assert_eq!(thread.comments[0].author, "Ann");
        assert!(provider.thread(12, "../other").is_err());
    }
    #[test]
    fn closed_list_includes_completed_and_abandoned_but_not_active() {
        let provider = forge(
            ".",
            vec![
                (
                    "--status completed",
                    r#"[{"pullRequestId":1,"status":"completed","title":"merged"}]"#,
                ),
                (
                    "--status abandoned",
                    r#"[{"pullRequestId":2,"status":"abandoned","title":"closed"}]"#,
                ),
                (
                    "--status all",
                    r#"[{"pullRequestId":3,"status":"active"},{"pullRequestId":2,"status":"abandoned"},{"pullRequestId":1,"status":"completed"}]"#,
                ),
            ],
        );
        let closed = provider
            .list_prs_state(crate::pr::PullRequestState::Closed)
            .unwrap();
        assert_eq!(
            closed.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            provider
                .list_prs_state(crate::pr::PullRequestState::All)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn completed_pr_uses_recorded_commits_without_the_deleted_source_branch() {
        fn git(dir: &std::path::Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .current_dir(dir)
                .args([
                    "-c",
                    "user.name=test",
                    "-c",
                    "user.email=test@example.invalid",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        let temp = tempfile::tempdir().unwrap();
        let upstream = temp.path().join("upstream");
        let client = temp.path().join("client");
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::create_dir_all(&client).unwrap();
        git(&upstream, &["init", "-q"]);
        std::fs::write(upstream.join("a.txt"), "base\n").unwrap();
        git(&upstream, &["add", "a.txt"]);
        git(&upstream, &["commit", "-qm", "base"]);
        let target = git(&upstream, &["rev-parse", "HEAD"]);
        std::fs::write(upstream.join("a.txt"), "base\nPR change\n").unwrap();
        git(&upstream, &["commit", "-qam", "PR source"]);
        let source = git(&upstream, &["rev-parse", "HEAD"]);
        git(&upstream, &["branch", "deleted-feature"]);
        // The only advertised branch has advanced beyond the PR, and no source branch exists.
        std::fs::write(
            upstream.join("a.txt"),
            "base\nPR change\nunrelated later change\n",
        )
        .unwrap();
        git(&upstream, &["commit", "-qam", "later target"]);
        git(&upstream, &["branch", "-m", "main"]);
        git(&upstream, &["branch", "-D", "deleted-feature"]);
        git(&client, &["init", "-q"]);
        git(
            &client,
            &["remote", "add", "origin", upstream.to_str().unwrap()],
        );
        struct Historical {
            repo: PathBuf,
            metadata: String,
            fetches: Arc<std::sync::atomic::AtomicUsize>,
        }
        impl Runner for Historical {
            fn run(&self, program: &str, args: &[&str]) -> Result<String> {
                if program == "az" {
                    assert!(
                        args.contains(&"get-access-token"),
                        "REST should avoid az pr show"
                    );
                    return Ok("test-token".into());
                }
                if program == "curl" {
                    return Ok(self.metadata.clone());
                }
                assert_eq!(program, "git");
                self.fetches
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                assert!(
                    !args.iter().any(|a| a.contains("/merge:")),
                    "historical PR must not probe a merge ref"
                );
                SystemRunner(self.repo.clone()).run(program, args)
            }
        }
        let metadata = serde_json::json!({"status":"completed","sourceRefName":"refs/heads/deleted-feature","targetRefName":"refs/heads/main",
            "lastMergeSourceCommit":{"commitId":source},"lastMergeTargetCommit":{"commitId":target}});
        let mut provider = forge(client.to_str().unwrap(), vec![]);
        let fetches = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        provider.runner = Box::new(Historical {
            repo: client,
            metadata: metadata.to_string(),
            fetches: fetches.clone(),
        });
        let diff = provider.pr_diff(19673).unwrap();
        assert_eq!(diff.total_additions, 1);
        assert_eq!(diff.total_deletions, 0);
        assert_eq!(diff.files.len(), 1);
        assert_eq!(provider.pr_diff(19673).unwrap().total_additions, 1);
        assert_eq!(fetches.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(diff.files[0]
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .all(|l| !l.content.contains("unrelated")));
    }

    #[test]
    fn missing_merge_cache_is_shared_scoped_and_expires() {
        let first = forge("repo-a", vec![]);
        first.remember_missing_merge(42);
        let mut second = forge("repo-a", vec![]);
        second.missing_merges = first.missing_merges.clone();
        assert!(second.merge_ref_missing(42));
        assert!(!second.merge_ref_missing(43));
        second.repo = "repo-b".into();
        assert!(!second.merge_ref_missing(42));
        second.repo = "repo-a".into();
        second.remote.as_mut().unwrap().repo = "different".into();
        assert!(!second.merge_ref_missing(42));
        first
            .missing_merges
            .lock()
            .unwrap()
            .insert(first.merge_key(42), Instant::now() - MISSING_MERGE_TTL);
        assert!(!first.merge_ref_missing(42));
    }

    #[test]
    fn repeated_load_skips_a_known_missing_merge_ref() {
        struct Missing(Arc<std::sync::atomic::AtomicUsize>);
        impl Runner for Missing {
            fn run(&self, program: &str, args: &[&str]) -> Result<String> {
                if program == "az" && args.contains(&"show") {
                    return Ok(r#"{"status":"active"}"#.into());
                }
                if program == "git" {
                    self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Err(super::super::failure::classify(
                        "git",
                        Some(128),
                        "couldn't find remote ref",
                    )
                    .into());
                }
                bail!("no token in test");
            }
        }
        let probes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut first = forge(".", vec![]);
        first.runner = Box::new(Missing(probes.clone()));
        assert!(first.pr_diff(42).is_err());
        let mut second = forge(".", vec![]);
        second.runner = Box::new(Missing(probes.clone()));
        second.missing_merges = first.missing_merges.clone();
        assert!(second.pr_diff(42).is_err());
        assert_eq!(probes.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn metadata_is_fresh_and_rest_components_are_encoded() {
        let _cache_guard = SHARED_CACHE_TEST_LOCK.lock().unwrap();
        struct Fresh;
        impl Runner for Fresh {
            fn run(&self, program: &str, args: &[&str]) -> Result<String> {
                if program == "az" {
                    assert!(args.contains(&"get-access-token"));
                    return Ok("test-token".into());
                }
                assert_eq!(program, "curl");
                assert_eq!(args.last(), Some(&"https://dev.azure.com/org/Fresh%20Proj/_apis/git/repositories/r%C3%A9po/pullRequests/987654?api-version=7.1"));
                Ok(r#"{"status":"completed"}"#.into())
            }
        }
        let mut provider = forge(".", vec![]);
        provider.remote = parse_remote("https://dev.azure.com/org/Fresh%20Proj/_git/r%C3%A9po");
        provider.runner = Box::new(Fresh);
        provider.shared = true;
        let url = "https://dev.azure.com/org/Fresh%20Proj/_apis/git/repositories/r%C3%A9po/pullRequests/987654?api-version=7.1";
        CACHE.write().unwrap().insert(
            url.into(),
            (
                Instant::now(),
                Arc::new(serde_json::json!({"status":"active"})),
            ),
        );
        assert_eq!(provider.pr_metadata(987654).unwrap()["status"], "completed");
        CACHE.write().unwrap().remove(url);
    }

    #[test]
    fn closed_pr_without_recorded_commits_is_not_compared_to_current_branches() {
        let provider = forge(
            ".",
            vec![(
                "pr show",
                r#"{"status":"completed","sourceRefName":"refs/heads/deleted","targetRefName":"refs/heads/main"}"#,
            )],
        );
        assert!(provider
            .pr_diff(1)
            .unwrap_err()
            .to_string()
            .contains("historical diff"));
    }
}
