//! Azure DevOps through the `az` CLI (`azure-devops` extension). lgtm never touches tokens.
use super::Forge;
use crate::git::GitEngine;
use crate::github::{Bucket, Check, GithubClient, Thread, ThreadComment};
use crate::model::DiffSnapshot;
use crate::pr::{PrAuthor, PullRequestSummary};
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
            .env("AZURE_CORE_COLLECT_TELEMETRY", "false")
            .current_dir(&self.0)
            .output()
            .with_context(|| format!("`{program}` is not installed or not on PATH"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            bail!(explain(&String::from_utf8_lossy(&out.stderr)))
        }
    }
}

pub fn explain(stderr: &str) -> String {
    let e = stderr.trim();
    if e.contains("az login") || e.contains("TF400813") || e.contains("401") {
        "az is not authenticated; run `az login` (or `az devops login`)".to_string()
    } else if e.contains("not in the 'az' command group") || e.contains("azure-devops") {
        "the Azure DevOps extension is missing; run `az extension add --name azure-devops`"
            .to_string()
    } else {
        format!("az failed: {e}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub org: String,
    pub project: String,
    pub repo: String,
}

fn unescape(s: &str) -> String {
    s.replace("%20", " ")
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

static BODY_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub struct AzureForge {
    repo: PathBuf,
    remote: Option<Remote>,
    runner: Box<dyn Runner>,
}

impl AzureForge {
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
        }
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
            Some(b) => {
                let p = std::env::temp_dir().join(format!(
                    "lgtm-az-{}-{}.json",
                    std::process::id(),
                    BODY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                ));
                std::fs::write(&p, b.to_string())?;
                Some(p)
            }
            None => None,
        };
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
        let path = file.as_ref().map(|p| p.to_string_lossy().into_owned());
        if let Some(p) = &path {
            args.extend(["--in-file", p]);
        }
        args.push("--route-parameters");
        args.extend(route.iter().map(String::as_str));
        let out = self.runner.run("az", &args);
        if let Some(p) = &file {
            let _ = std::fs::remove_file(p);
        }
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
        Ok(parse_prs(
            &self.az_json(&["repos", "pr", "list", "--status", "active"])?,
        ))
    }

    /// Fast path: one `git fetch` of `refs/pull/<n>/merge` (parents are target, source), no `az`
    /// call (each costs seconds). Falls back to fetching both branches when that ref is missing.
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
        let merge = format!("refs/lgtm/pr/{pr}/merge");
        let fetched = self.runner.run(
            "git",
            &[
                "fetch",
                "--quiet",
                "origin",
                &format!("+refs/pull/{pr}/merge:{merge}"),
            ],
        );
        if fetched.is_ok() {
            if let Ok(snap) =
                GitEngine::new(&self.repo).compute_revision_diff(&format!("{merge}^1...{merge}^2"))
            {
                return Ok(snap);
            }
        }
        let id = pr.to_string();
        let v = self.az_json(&["repos", "pr", "show", "--id", &id])?;
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
        )?;
        GitEngine::new(&self.repo).compute_revision_diff(&format!("{base}...{head}"))
    }

    /// `APPROVE` and `REQUEST_CHANGES` set the vote (plus the body as a comment); `COMMENT` only comments.
    fn post_review(&self, pr: u64, event: &str, body: &str) -> Result<()> {
        let vote = match event {
            "APPROVE" => Some("approve"),
            "REQUEST_CHANGES" => Some("wait-for-author"),
            "COMMENT" => None,
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
        }
        Ok(())
    }
}

impl GithubClient for AzureForge {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
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
        Ok(parse_threads(&self.invoke("pullRequestThreads", pr)?))
    }
    fn head_sha(&self, pr: u64) -> Result<String> {
        let v = self.az_json(&["repos", "pr", "show", "--id", &pr.to_string()])?;
        v.pointer("/lastMergeSourceCommit/commitId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .context("PR has no source commit")
    }
    fn post_inline(&self, pr: u64, _sha: &str, path: &str, line: u32, body: &str) -> Result<u64> {
        let pos = serde_json::json!({"line": line, "offset": 1});
        let ctx = serde_json::json!({
            "filePath": format!("/{path}"),
            "rightFileStart": pos,
            "rightFileEnd": pos,
        });
        self.new_thread(pr, body, Some(ctx))
    }
    fn post_general(&self, pr: u64, body: &str) -> Result<u64> {
        self.new_thread(pr, body, None)
    }
    fn reply(&self, pr: u64, thread_id: &str, comment_id: u64, body: &str) -> Result<u64> {
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
        let status = if resolved { "fixed" } else { "active" };
        let b = serde_json::json!({ "status": status });
        self.call("patch", "pullRequestThreads", pr, Some(thread_id), Some(&b))?;
        Ok(())
    }
}

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
                .ok_or_else(|| anyhow::anyhow!("no fake for {line}"))
        }
    }

    fn forge(repo: &str, fakes: Vec<(&'static str, &'static str)>) -> AzureForge {
        AzureForge {
            repo: repo.into(),
            remote: parse_remote("https://dev.azure.com/org/My%20Proj/_git/repo"),
            runner: Box::new(Fake(RefCell::new(vec![]), fakes)),
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
        let f = forge(
            dir.to_str().unwrap(),
            vec![
                (
                    "pr show",
                    r#"{"sourceRefName":"refs/heads/f","targetRefName":"refs/heads/main"}"#,
                ),
                ("git fetch", ""),
            ],
        );
        let snap = f.pr_diff(7).unwrap();
        assert_eq!(snap.files.len(), 1);
        assert_eq!(snap.total_additions, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_uses_the_merge_ref_without_calling_az() {
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
        // only "git fetch" is faked: any `az` call would error
        let f = forge(dir.to_str().unwrap(), vec![("git fetch", "")]);
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
    fn explains_az_errors() {
        assert!(explain("Please run 'az login' to setup account").contains("az login"));
        assert!(explain("'repos' is not in the 'az' command group").contains("extension add"));
        assert!(explain("boom").starts_with("az failed"));
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
                ("--http-method patch", "{}"),
                ("set-vote", "{}"),
                ("pr show", r#"{"lastMergeSourceCommit":{"commitId":"abc"}}"#),
            ],
        );
        assert_eq!(f.head_sha(2).unwrap(), "abc");
        assert_eq!(f.post_inline(2, "abc", "src/a.rs", 4, "hi").unwrap(), 7);
        assert_eq!(f.post_general(2, "hi").unwrap(), 7);
        assert_eq!(f.reply(2, "5", 1, "ok").unwrap(), 7);
        f.set_resolved(2, "5", true).unwrap();
        f.post_review(2, "APPROVE", "lgtm").unwrap();
        f.post_review(2, "REQUEST_CHANGES", "").unwrap();
        assert!(f.post_review(2, "NOPE", "").is_err());
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
}
