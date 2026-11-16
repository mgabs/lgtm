//! GitLab REST API integration with `glab` / `GITLAB_TOKEN` authentication.
use super::model::{Applied, Resolution, Thread, ThreadComment, ThreadState};
use super::{Forge, PrDiscussion};

use crate::comment::CommentSide;
use crate::github::Check;
use crate::model::DiffSnapshot;
use crate::pr::{PrAuthor, PullRequestSummary};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

pub use super::azure::Runner;
pub use super::azure::SystemRunner;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Remote {
    pub host: String,
    pub path: String, // project path e.g. "group/subgroup/project"
}

impl Remote {
    pub fn encoded_path(&self) -> String {
        url_encode_path(&self.path)
    }
}

pub fn url_encode_path(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Parses GitLab remote URLs into host and project path.
pub fn parse_remote(url: &str) -> Option<Remote> {
    let u = url.trim().trim_end_matches(".git");
    if let Some((proto, rest)) = u.split_once("://") {
        if !proto.starts_with("http") {
            return None;
        }
        let (host_part, path_part) = rest.split_once('/')?;
        let host = host_part.rsplit('@').next()?;
        let path = path_part.trim_matches('/').to_string();
        if path.is_empty() {
            return None;
        }
        Some(Remote {
            host: host.to_string(),
            path,
        })
    } else if let Some((host_part, path_part)) = u.split_once(':') {
        let host = host_part.rsplit('@').next()?;
        let path = path_part.trim_matches('/').to_string();
        if path.is_empty() {
            return None;
        }
        Some(Remote {
            host: host.to_string(),
            path,
        })
    } else {
        None
    }
}

type CacheEntry = (Instant, Arc<Value>);
type RestCache = RwLock<HashMap<String, CacheEntry>>;
static CACHE: LazyLock<RestCache> = LazyLock::new(|| RwLock::new(HashMap::new()));
const CACHE_TTL: Duration = Duration::from_secs(300);

static TOKEN_CACHE: LazyLock<Mutex<HashMap<String, (String, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const TOKEN_TTL: Duration = Duration::from_secs(1800);

fn cache_fresh(at: Instant, now: Instant, ttl: Duration) -> bool {
    now.duration_since(at) < ttl
}

pub struct GitLabForge {
    repo: PathBuf,
    remote: Option<Remote>,
    runner: Box<dyn Runner>,
    shared: bool,
    api_token: Option<String>,
}

impl GitLabForge {
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
            api_token: None,
        }
    }

    pub fn with_api_token(mut self, token: Option<String>) -> Self {
        self.api_token = token;
        self
    }

    pub(crate) fn with_origin(repo: PathBuf, origin_url: Option<String>) -> Self {
        let remote = origin_url.as_deref().and_then(parse_remote);
        let runner = Box::new(SystemRunner(repo.clone()));
        Self {
            repo,
            remote,
            runner,
            shared: true,
            api_token: None,
        }
    }

    fn token(&self) -> Result<String> {
        if let Some(tok) = &self.api_token {
            return Ok(tok.clone());
        }
        if let Ok(tok) = std::env::var("GITLAB_TOKEN") {
            let trimmed = tok.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        let host = self
            .remote
            .as_ref()
            .map(|r| r.host.as_str())
            .unwrap_or("gitlab.com");
        if self.shared {
            let cache = TOKEN_CACHE.lock().unwrap();
            if let Some((tok, at)) = cache.get(host) {
                if cache_fresh(*at, Instant::now(), TOKEN_TTL) {
                    return Ok(tok.clone());
                }
            }
        }
        let out = self
            .runner
            .run("glab", &["auth", "token", "--hostname", host])
            .or_else(|_| self.runner.run("glab", &["auth", "token"]))
            .context("Authentication failed; run `glab auth login` or set `GITLAB_TOKEN`")?;
        let tok = out.trim().to_string();
        if tok.is_empty() {
            bail!("Authentication failed; run `glab auth login` or set `GITLAB_TOKEN`");
        }
        if self.shared {
            TOKEN_CACHE
                .lock()
                .unwrap()
                .insert(host.to_string(), (tok.clone(), Instant::now()));
        }
        Ok(tok)
    }

    fn rest_read(&self, path: &str, cached: bool) -> Result<Arc<Value>> {
        let Some(r) = &self.remote else {
            bail!("origin is not a recognized GitLab URL");
        };
        let url = format!(
            "https://{}/api/v4/projects/{}/{}",
            r.host,
            r.encoded_path(),
            path.trim_start_matches('/')
        );
        if self.shared && cached {
            let hit = {
                let cache = CACHE.read().unwrap();
                cache.get(&url).and_then(|(at, v)| {
                    if cache_fresh(*at, Instant::now(), CACHE_TTL) {
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
            "lgtm-gl-hdr",
            format!("PRIVATE-TOKEN: {token}\n").as_bytes(),
        )?;
        let hdr = format!("@{}", file.path().display());
        let out = self
            .runner
            .run("curl", &["-sS", "--fail", "-H", &hdr, &url]);
        drop(file);
        let out = out.map_err(|e| {
            if let Some(failure) = e.downcast_ref::<super::failure::Failure>() {
                if failure.http_status == Some(401) {
                    return anyhow::anyhow!("GitLab authentication failed (401); run `glab auth login` or check GITLAB_TOKEN");
                }
            }
            e
        })?;
        let v: Value = serde_json::from_str(&out).context("unexpected GitLab REST response")?;
        let v = Arc::new(v);
        if self.shared {
            let mut cache = CACHE.write().unwrap();
            let now = Instant::now();
            cache.retain(|_, (at, _)| cache_fresh(*at, now, CACHE_TTL));
            cache.insert(url, (Instant::now(), Arc::clone(&v)));
        }
        Ok(v)
    }

    fn rest_write(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let Some(r) = &self.remote else {
            bail!("origin is not a recognized GitLab URL");
        };
        let url = format!(
            "https://{}/api/v4/projects/{}/{}",
            r.host,
            r.encoded_path(),
            path.trim_start_matches('/')
        );
        let token = self.token()?;
        let file = super::PrivateTempFile::write(
            "lgtm-gl-hdr",
            format!("PRIVATE-TOKEN: {token}\nContent-Type: application/json\n").as_bytes(),
        )?;
        let hdr = format!("@{}", file.path().display());
        let mut args = vec!["-sS", "--fail", "-X", method, "-H", &hdr];
        let body_str;
        let mut _body_file = None;
        if let Some(b) = body {
            body_str = serde_json::to_string(b)?;
            let bf = super::PrivateTempFile::write("lgtm-gl-body", body_str.as_bytes())?;
            args.push("--data");
            args.push(Box::leak(
                format!("@{}", bf.path().display()).into_boxed_str(),
            ));
            _body_file = Some(bf);
        }
        args.push(&url);
        let out = self.runner.run("curl", &args);
        drop(file);
        drop(_body_file);
        if self.shared {
            let mut cache = CACHE.write().unwrap();
            cache.clear();
        }
        let out = out.map_err(|e| {
            if let Some(failure) = e.downcast_ref::<super::failure::Failure>() {
                if failure.http_status == Some(401) {
                    return anyhow::anyhow!("GitLab authentication failed (401); run `glab auth login` or check GITLAB_TOKEN");
                }
            }
            e
        })?;
        if out.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&out).context("unexpected GitLab REST response")
    }

    fn mr_metadata(&self, mr: u64) -> Result<Arc<Value>> {
        crate::debug::measure("pr.fetch_metadata", || {
            self.rest_read(&format!("merge_requests/{mr}"), false)
        })
    }
}

pub fn parse_mrs(v: &Value) -> Vec<PullRequestSummary> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|mr| {
            let number = mr["iid"]
                .as_u64()
                .or_else(|| mr["id"].as_u64())
                .unwrap_or(0);
            let title = mr["title"].as_str().unwrap_or_default().to_string();
            let author_login = mr["author"]["username"]
                .as_str()
                .or_else(|| mr["author"]["name"].as_str())
                .unwrap_or_default()
                .to_string();
            let head_ref_name = mr["source_branch"].as_str().unwrap_or_default().to_string();
            let state = mr["state"].as_str().unwrap_or("opened").to_string();
            PullRequestSummary {
                number,
                title,
                author: PrAuthor {
                    login: author_login,
                },
                head_ref_name,
                additions: 0,
                deletions: 0,
                state,
            }
        })
        .collect()
}

pub fn parse_discussions(v: &Value) -> Vec<Thread> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| {
            let id = d["id"].as_str().unwrap_or_default().to_string();
            let notes = d["notes"].as_array()?;
            if notes.is_empty() {
                return None;
            }
            // Check if individual notes are system notes
            let non_system_notes: Vec<&Value> = notes
                .iter()
                .filter(|n| !n["system"].as_bool().unwrap_or(false))
                .collect();
            if non_system_notes.is_empty() {
                return None;
            }

            let first_note = non_system_notes[0];
            let pos = &first_note["position"];
            let path = pos["new_path"]
                .as_str()
                .or_else(|| pos["old_path"].as_str())
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string();
            let line = pos["new_line"]
                .as_u64()
                .or_else(|| pos["old_line"].as_u64())
                .map(|l| l as u32);

            let all_resolved = non_system_notes.iter().all(|n| {
                if n["resolvable"].as_bool().unwrap_or(false) {
                    n["resolved"].as_bool().unwrap_or(false)
                } else {
                    true
                }
            }) && non_system_notes
                .iter()
                .any(|n| n["resolvable"].as_bool().unwrap_or(false));

            let state = if all_resolved {
                ThreadState::Resolved(Resolution::Done)
            } else {
                ThreadState::Open
            };

            let comments: Vec<ThreadComment> = non_system_notes
                .into_iter()
                .map(|n| ThreadComment {
                    id: n["id"].as_u64().unwrap_or(0),
                    author: n["author"]["username"]
                        .as_str()
                        .or_else(|| n["author"]["name"].as_str())
                        .unwrap_or_default()
                        .to_string(),
                    body: n["body"].as_str().unwrap_or_default().to_string(),
                })
                .collect();

            Some(Thread {
                id,
                path,
                line,
                state,
                outdated: false,
                can_resolve: None,
                can_unresolve: None,
                comments,
            })
        })
        .collect()
}

impl PrDiscussion for GitLabForge {
    fn checks(&self, _pr: u64) -> Result<Vec<Check>> {
        bail!("CI checks are not supported by this provider")
    }

    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        let v = self.rest_read(&format!("merge_requests/{pr}/discussions"), true)?;
        Ok(parse_discussions(&v))
    }

    fn thread(&self, pr: u64, id: &str) -> Result<Option<Thread>> {
        let v = self.rest_read(&format!("merge_requests/{pr}/discussions/{id}"), false)?;
        let threads = parse_discussions(&serde_json::json!([*v]));
        Ok(threads.into_iter().next())
    }

    fn head_sha(&self, pr: u64) -> Result<String> {
        let v = self.mr_metadata(pr)?;
        v["sha"]
            .as_str()
            .or_else(|| v.pointer("/diff_refs/head_sha").and_then(Value::as_str))
            .map(str::to_string)
            .context("MR has no head sha")
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
        let meta = self.mr_metadata(pr)?;
        let diff_refs = &meta["diff_refs"];
        let base_sha = diff_refs["base_sha"].as_str().unwrap_or_default();
        let start_sha = diff_refs["start_sha"].as_str().unwrap_or_default();
        let head_sha = diff_refs["head_sha"].as_str().unwrap_or_default();

        let mut pos = serde_json::json!({
            "base_sha": base_sha,
            "start_sha": start_sha,
            "head_sha": head_sha,
            "position_type": "text",
            "new_path": path,
            "old_path": path,
        });

        match side {
            CommentSide::Left => {
                pos["old_line"] = serde_json::json!(line);
            }
            CommentSide::Right => {
                pos["new_line"] = serde_json::json!(line);
            }
        }

        let payload = serde_json::json!({
            "body": body,
            "position": pos,
        });

        let res = self.rest_write(
            "POST",
            &format!("merge_requests/{pr}/discussions"),
            Some(&payload),
        )?;
        res.pointer("/notes/0/id")
            .and_then(Value::as_u64)
            .or_else(|| res["id"].as_u64())
            .context("GitLab returned no note id for discussion")
    }

    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        let payload = serde_json::json!({ "body": body });
        let res = self.rest_write(
            "POST",
            &format!("merge_requests/{pr}/notes"),
            Some(&payload),
        )?;
        res["id"].as_u64().context("GitLab returned no note id")
    }

    fn reply(&self, pr: u64, thread_id: &str, _comment_id: u64, body: &str) -> Result<u64> {
        let payload = serde_json::json!({ "body": body });
        let res = self.rest_write(
            "POST",
            &format!("merge_requests/{pr}/discussions/{thread_id}/notes"),
            Some(&payload),
        )?;
        res["id"]
            .as_u64()
            .context("GitLab returned no note id for reply")
    }

    fn set_state(&self, pr: u64, thread_id: &str, state: ThreadState) -> Result<Applied> {
        let (resolved, applied_res) = match state {
            ThreadState::Open => (false, None),
            ThreadState::Resolved(Resolution::Done) => (true, Some(Resolution::Done)),
            ThreadState::Resolved(Resolution::WontFix) => (true, Some(Resolution::Done)),
            ThreadState::Resolved(Resolution::Other(kind)) => {
                bail!("Resolution Other({kind}) is read-only and cannot be set")
            }
        };

        let path = format!("merge_requests/{pr}/discussions/{thread_id}?resolved={resolved}");
        let res = self.rest_write("PUT", &path, None)?;
        if res.is_null() || res["id"].as_str().is_none() {
            bail!("Thread update has no confirmation; refresh before retrying");
        }
        let applied_state = match applied_res {
            Some(res) => ThreadState::Resolved(res),
            None => ThreadState::Open,
        };
        Ok(Applied {
            requested: state,
            applied: applied_state,
        })
    }
}

impl Forge for GitLabForge {
    fn list_prs(&self) -> Result<Vec<PullRequestSummary>> {
        self.list_prs_state(crate::pr::PullRequestState::Open)
    }

    fn list_prs_state(
        &self,
        state: crate::pr::PullRequestState,
    ) -> Result<Vec<PullRequestSummary>> {
        let status = match state {
            crate::pr::PullRequestState::Open => "opened",
            crate::pr::PullRequestState::Closed => "closed",
            crate::pr::PullRequestState::All => "all",
        };
        let query = if status == "all" {
            "merge_requests?per_page=100".to_string()
        } else {
            format!("merge_requests?state={status}&per_page=100")
        };
        let v = self.rest_read(&query, true)?;
        Ok(parse_mrs(&v))
    }

    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        let v = self.mr_metadata(pr)?;
        let source = v
            .pointer("/diff_refs/head_sha")
            .and_then(Value::as_str)
            .or_else(|| v["sha"].as_str());
        let target = v.pointer("/diff_refs/base_sha").and_then(Value::as_str);

        if let (Some(source), Some(target)) = (source, target) {
            git2::Oid::from_str(source).context("MR has an invalid recorded source commit")?;
            git2::Oid::from_str(target).context("MR has an invalid recorded target commit")?;
            if !super::have_commits(&self.repo, source, target) {
                let head = format!("refs/lgtm/pr/{pr}/head");
                let base = format!("refs/lgtm/pr/{pr}/base");
                self.runner
                    .run(
                        "git",
                        &[
                            "fetch",
                            "--quiet",
                            "origin",
                            &format!("+{source}:{head}"),
                            &format!("+{target}:{base}"),
                        ],
                    )
                    .context("Fetching MR recorded commits")?;
            }
            return super::build_pr_diff(&self.repo, &format!("{target}...{source}"));
        }

        let src_branch = v["source_branch"]
            .as_str()
            .context("MR has no source branch")?;
        let tgt_branch = v["target_branch"]
            .as_str()
            .context("MR has no target branch")?;
        let head = format!("refs/lgtm/pr/{pr}/head");
        let base = format!("refs/lgtm/pr/{pr}/base");
        self.runner
            .run(
                "git",
                &[
                    "fetch",
                    "--quiet",
                    "origin",
                    &format!("+{src_branch}:{head}"),
                    &format!("+{tgt_branch}:{base}"),
                ],
            )
            .context("Fetching MR source/target branches")?;
        super::build_pr_diff(&self.repo, &format!("{base}...{head}"))
    }

    fn post_review(&self, pr: u64, _event: &str, body: &str) -> Result<()> {
        if !body.is_empty() {
            self.post_general(pr, body)?;
        }
        Ok(())
    }
}

#[cfg(test)]
static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Arc<Mutex<Vec<String>>>, Vec<(&'static str, &'static str)>);

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            let line = format!("{program} {}", args.join(" "));
            self.0.lock().unwrap().push(line.clone());
            self.1
                .iter()
                .find(|(k, _)| line.contains(k))
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| anyhow::anyhow!("unexpected command in test: {line}"))
        }
    }

    fn fake_forge(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (GitLabForge, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = Fake(Arc::clone(&calls), responses);
        let repo = PathBuf::from(".");
        let forge = GitLabForge {
            repo,
            remote: Some(Remote {
                host: "gitlab.com".to_string(),
                path: "my-group/my-proj".to_string(),
            }),
            runner: Box::new(runner),
            shared: false,
            api_token: Some("glpat-test-token".to_string()),
        };
        (forge, calls)
    }

    #[test]
    fn url_encode_path_encodes_special_chars() {
        assert_eq!(url_encode_path("group/project"), "group%2Fproject");
        assert_eq!(
            url_encode_path("group/subgroup/project"),
            "group%2Fsubgroup%2Fproject"
        );
        assert_eq!(url_encode_path("plain-name_1.0~"), "plain-name_1.0~");
        assert_eq!(
            url_encode_path("project with spaces"),
            "project%20with%20spaces"
        );
    }

    #[test]
    fn parse_remote_urls() {
        assert_eq!(
            parse_remote("https://gitlab.com/group/project.git"),
            Some(Remote {
                host: "gitlab.com".to_string(),
                path: "group/project".to_string()
            })
        );
        assert_eq!(
            parse_remote("git@gitlab.internal.corp:group/subgroup/project.git"),
            Some(Remote {
                host: "gitlab.internal.corp".to_string(),
                path: "group/subgroup/project".to_string()
            })
        );
        assert_eq!(parse_remote("https://gitlab.com/"), None);
        assert_eq!(parse_remote("git@gitlab.com:"), None);
        assert_eq!(parse_remote("ftp://gitlab.com/repo.git"), None);
        assert_eq!(parse_remote("not-a-url"), None);
    }

    #[test]
    fn discussion_mapping_and_side_detection() {
        let json = serde_json::json!([
            {
                "id": "disc-empty",
                "notes": []
            },
            {
                "id": "disc-1",
                "notes": [
                    {
                        "id": 101,
                        "system": false,
                        "resolvable": true,
                        "resolved": true,
                        "author": { "username": "alice" },
                        "body": "left note",
                        "position": {
                            "old_path": "src/lib.rs",
                            "old_line": 42
                        }
                    }
                ]
            },
            {
                "id": "disc-2",
                "notes": [
                    {
                        "id": 102,
                        "system": false,
                        "resolvable": true,
                        "resolved": false,
                        "author": { "username": "bob" },
                        "body": "right note",
                        "position": {
                            "new_path": "src/main.rs",
                            "new_line": 15
                        }
                    }
                ]
            },
            {
                "id": "disc-system",
                "notes": [
                    {
                        "id": 999,
                        "system": true,
                        "body": "system event"
                    }
                ]
            }
        ]);

        let threads = parse_discussions(&json);
        assert_eq!(threads.len(), 2);

        assert_eq!(threads[0].id, "disc-1");
        assert_eq!(threads[0].path, "src/lib.rs");
        assert_eq!(threads[0].line, Some(42));
        assert_eq!(threads[0].state, ThreadState::Resolved(Resolution::Done));
        assert_eq!(threads[0].comments[0].author, "alice");
        assert_eq!(threads[0].comments[0].body, "left note");

        assert_eq!(threads[1].id, "disc-2");
        assert_eq!(threads[1].path, "src/main.rs");
        assert_eq!(threads[1].line, Some(15));
        assert_eq!(threads[1].state, ThreadState::Open);
        assert_eq!(threads[1].comments[0].author, "bob");
    }

    #[test]
    fn parse_mrs_mapping() {
        let json = serde_json::json!([
            {
                "iid": 12,
                "title": "Add feature",
                "author": { "username": "developer" },
                "source_branch": "feat/xyz",
                "state": "opened"
            }
        ]);
        let mrs = parse_mrs(&json);
        assert_eq!(mrs.len(), 1);
        assert_eq!(mrs[0].number, 12);
        assert_eq!(mrs[0].title, "Add feature");
        assert_eq!(mrs[0].author.login, "developer");
        assert_eq!(mrs[0].head_ref_name, "feat/xyz");
        assert_eq!(mrs[0].state, "opened");
    }

    #[test]
    fn set_state_wontfix_fallback_and_token_never_in_args() {
        let (forge, calls) = fake_forge(vec![("PUT", r#"{"id":"disc-1","notes":[]}"#)]);

        let applied = forge
            .set_state(10, "disc-1", ThreadState::Resolved(Resolution::WontFix))
            .unwrap();
        assert_eq!(
            applied.requested,
            ThreadState::Resolved(Resolution::WontFix)
        );
        assert_eq!(applied.applied, ThreadState::Resolved(Resolution::Done));

        let executed_calls = calls.lock().unwrap();
        assert_eq!(executed_calls.len(), 1);
        let call = &executed_calls[0];
        assert!(call.contains("curl -sS --fail -X PUT"));
        assert!(call.contains("merge_requests/10/discussions/disc-1?resolved=true"));
        // Security check: token must NEVER be passed as CLI argument
        assert!(!call.contains("glpat-test-token"));
        assert!(call.contains("@")); // Passed via temp file header
    }

    #[test]
    fn set_state_open_unresolves_discussion() {
        let (forge, calls) = fake_forge(vec![
            (
                "merge_requests/10/discussions/disc-done",
                r#"{"id":"disc-done"}"#,
            ),
            ("PUT", r#"{"id":"disc-1"}"#),
        ]);

        let applied = forge.set_state(10, "disc-1", ThreadState::Open).unwrap();
        assert_eq!(applied.requested, ThreadState::Open);
        assert_eq!(applied.applied, ThreadState::Open);

        let done = forge
            .set_state(10, "disc-done", ThreadState::Resolved(Resolution::Done))
            .unwrap();
        assert_eq!(done.applied, ThreadState::Resolved(Resolution::Done));

        let call = &calls.lock().unwrap()[0];
        assert!(call.contains("merge_requests/10/discussions/disc-1?resolved=false"));
    }

    #[test]
    fn set_state_requires_id_confirmation() {
        let (forge, _) = fake_forge(vec![("PUT", r#"{"notes":[]}"#)]);
        let err = forge
            .set_state(10, "disc-1", ThreadState::Open)
            .unwrap_err();
        assert!(err.to_string().contains("no confirmation"));
    }

    #[test]
    fn set_state_rejects_other_resolutions() {
        let (forge, _) = fake_forge(vec![]);
        let err = forge
            .set_state(
                10,
                "disc-1",
                ThreadState::Resolved(Resolution::Other("custom".into())),
            )
            .unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }

    #[test]
    fn checks_returns_unsupported_error() {
        let (forge, _) = fake_forge(vec![]);
        let err = forge.checks(10).unwrap_err();
        assert!(err.to_string().contains("CI checks are not supported"));
    }

    #[test]
    fn env_token_override_takes_precedence() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let prev = std::env::var("GITLAB_TOKEN").ok();
        std::env::set_var("GITLAB_TOKEN", "env-override-token");
        let forge = GitLabForge {
            repo: PathBuf::from("."),
            remote: Some(Remote {
                host: "gitlab.com".into(),
                path: "p".into(),
            }),
            runner: Box::new(Fake(Arc::new(Mutex::new(vec![])), vec![])),
            shared: false,
            api_token: None,
        };
        let token = forge.token().unwrap();
        if let Some(val) = prev {
            std::env::set_var("GITLAB_TOKEN", val);
        } else {
            std::env::remove_var("GITLAB_TOKEN");
        }
        assert_eq!(token, "env-override-token");
    }

    #[test]
    fn glab_token_fallback_when_env_not_set() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let prev = std::env::var("GITLAB_TOKEN").ok();
        std::env::remove_var("GITLAB_TOKEN");
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = Fake(
            Arc::clone(&calls),
            vec![("glab auth token", "glab-cli-token\n")],
        );
        let forge = GitLabForge {
            repo: PathBuf::from("."),
            remote: Some(Remote {
                host: "gitlab.com".into(),
                path: "p".into(),
            }),
            runner: Box::new(runner),
            shared: false,
            api_token: None,
        };
        let token = forge.token().unwrap();
        if let Some(val) = prev {
            std::env::set_var("GITLAB_TOKEN", val);
        }
        assert_eq!(token, "glab-cli-token");
    }

    #[test]
    fn post_inline_and_reply_payloads() {
        let (forge, calls) = fake_forge(vec![
            ("discussions/d1/notes", r#"{"id":555}"#),
            (
                "merge_requests/5/discussions",
                r#"{"id":"d1","notes":[{"id":555}]}"#,
            ),
            (
                "merge_requests/5",
                r#"{"diff_refs":{"base_sha":"b1","start_sha":"s1","head_sha":"h1"}}"#,
            ),
        ]);

        let note_id = forge
            .post_inline(5, "h1", "src/file.rs", 25, CommentSide::Right, "looks good")
            .unwrap();
        assert_eq!(note_id, 555);

        let note_left = forge
            .post_inline(5, "h1", "src/file.rs", 30, CommentSide::Left, "left change")
            .unwrap();
        assert_eq!(note_left, 555);

        let reply_id = forge.reply(5, "d1", 555, "thanks!").unwrap();
        assert_eq!(reply_id, 555);

        let executed = calls.lock().unwrap();
        assert!(executed
            .iter()
            .any(|c| c.contains("merge_requests/5/discussions")));
        assert!(executed
            .iter()
            .any(|c| c.contains("merge_requests/5/discussions/d1/notes")));
    }

    #[test]
    fn fetch_threads_and_thread_by_id() {
        let (forge, _) = fake_forge(vec![
            (
                "merge_requests/7/discussions/d1",
                r#"{"id":"d1","notes":[{"id":10,"author":{"username":"u"},"body":"msg","resolvable":true,"resolved":true}]}"#,
            ),
            (
                "merge_requests/7/discussions",
                r#"[{"id":"d1","notes":[{"id":10,"author":{"username":"u"},"body":"msg","resolvable":true,"resolved":true}]}]"#,
            ),
        ]);
        let threads = forge.threads(7).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, "d1");

        let single = forge.thread(7, "d1").unwrap().unwrap();
        assert_eq!(single.id, "d1");
    }

    #[test]
    fn list_prs_and_state_filtering() {
        let (forge, _) = fake_forge(vec![
            (
                "merge_requests?state=opened",
                r#"[{"iid":1,"title":"Open MR","state":"opened"}]"#,
            ),
            (
                "merge_requests?state=closed",
                r#"[{"iid":2,"title":"Closed MR","state":"closed"}]"#,
            ),
            (
                "merge_requests?",
                r#"[{"iid":1,"title":"Open MR","state":"opened"},{"iid":2,"title":"Closed MR","state":"closed"}]"#,
            ),
        ]);
        let open_mrs = forge.list_prs().unwrap();
        assert_eq!(open_mrs.len(), 1);
        assert_eq!(open_mrs[0].number, 1);

        let closed_mrs = forge
            .list_prs_state(crate::pr::PullRequestState::Closed)
            .unwrap();
        assert_eq!(closed_mrs.len(), 1);
        assert_eq!(closed_mrs[0].number, 2);

        let all_mrs = forge
            .list_prs_state(crate::pr::PullRequestState::All)
            .unwrap();
        assert_eq!(all_mrs.len(), 2);
    }

    #[test]
    fn post_review_and_post_general() {
        let (forge, calls) = fake_forge(vec![("merge_requests/8/notes", r#"{"id":777}"#)]);
        let note_id = forge.post_general(8, "general comment").unwrap();
        assert_eq!(note_id, 777);

        forge.post_review(8, "COMMENT", "review body").unwrap();
        assert!(calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.contains("merge_requests/8/notes")));

        // Empty review body does nothing
        forge.post_review(8, "COMMENT", "").unwrap();
    }

    #[test]
    fn head_sha_resolution() {
        let (forge, _) = fake_forge(vec![(
            "merge_requests/9",
            r#"{"sha":"commit-sha-123","diff_refs":{"head_sha":"commit-sha-123"}}"#,
        )]);
        assert_eq!(forge.head_sha(9).unwrap(), "commit-sha-123");
    }

    #[test]
    fn pr_diff_reconstructs_diff_from_commits() {
        let dir = std::env::temp_dir().join(format!("lgtm-gl-diff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |a: &[&str]| {
            let ok = std::process::Command::new("git")
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
        std::fs::write(dir.join("f.txt"), "1\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "base"]);
        let base_sha = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let base_sha = String::from_utf8(base_sha.stdout)
            .unwrap()
            .trim()
            .to_string();

        std::fs::write(dir.join("f.txt"), "1\n2\n").unwrap();
        git(&["commit", "-qam", "head"]);
        let head_sha = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let head_sha = String::from_utf8(head_sha.stdout)
            .unwrap()
            .trim()
            .to_string();

        let json =
            format!(r#"{{"diff_refs":{{"base_sha":"{base_sha}","head_sha":"{head_sha}"}}}}"#);
        let runner = Fake(
            Arc::new(Mutex::new(vec![])),
            vec![("merge_requests/42", Box::leak(json.into_boxed_str()))],
        );

        let forge = GitLabForge {
            repo: dir.clone(),
            remote: Some(Remote {
                host: "gitlab.com".into(),
                path: "p".into(),
            }),
            runner: Box::new(runner),
            shared: false,
            api_token: Some("tok".into()),
        };

        let diff = forge.pr_diff(42).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.total_additions, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shared_cache_and_token_cache_paths() {
        let mut forge = GitLabForge::with_origin(
            PathBuf::from("."),
            Some("https://gitlab.com/grp/prj".into()),
        );
        forge.runner = Box::new(Fake(
            Arc::new(Mutex::new(vec![])),
            vec![
                ("glab auth token", "cli-cached-token"),
                (
                    "merge_requests/1/discussions",
                    r#"[{"id":"d1","notes":[{"id":1,"resolvable":false}]}]"#,
                ),
            ],
        ));
        // First token fetch populates cache
        assert_eq!(forge.token().unwrap(), "cli-cached-token");
        // Second fetch hits token cache
        assert_eq!(forge.token().unwrap(), "cli-cached-token");

        // First rest_read populates CACHE
        let t1 = forge.threads(1).unwrap();
        assert_eq!(t1.len(), 1);
        // Second rest_read hits CACHE
        let t2 = forge.threads(1).unwrap();
        assert_eq!(t2.len(), 1);
    }

    #[test]
    fn error_paths_and_401_mapping() {
        let no_remote = GitLabForge {
            repo: PathBuf::from("."),
            remote: None,
            runner: Box::new(Fake(Arc::new(Mutex::new(vec![])), vec![])),
            shared: false,
            api_token: Some("tok".into()),
        };
        assert!(no_remote.threads(1).is_err());
        assert!(no_remote.set_state(1, "d", ThreadState::Open).is_err());

        // 401 failure mapping
        struct Fail401Runner;
        impl Runner for Fail401Runner {
            fn run(&self, _prog: &str, _args: &[&str]) -> Result<String> {
                Err(
                    super::super::failure::classify("curl", Some(22), "HTTP 401 Unauthorized")
                        .into(),
                )
            }
        }
        let forge_401 = GitLabForge {
            repo: PathBuf::from("."),
            remote: Some(Remote {
                host: "gitlab.com".into(),
                path: "p".into(),
            }),
            runner: Box::new(Fail401Runner),
            shared: false,
            api_token: Some("tok".into()),
        };
        let err = forge_401.threads(1).unwrap_err();
        assert!(err.to_string().contains("glab auth login"));

        let write_err = forge_401.set_state(1, "d", ThreadState::Open).unwrap_err();
        assert!(write_err.to_string().contains("glab auth login"));
    }
}
