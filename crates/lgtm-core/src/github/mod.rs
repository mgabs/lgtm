//! Read-only GitHub access behind a trait. The real client shells out to `gh`; lgtm never touches tokens.
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bucket {
    Pass,
    Fail,
    Pending,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub bucket: Bucket,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CiSummary {
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
}

impl CiSummary {
    pub fn of(checks: &[Check]) -> Self {
        let n = |b: Bucket| checks.iter().filter(|c| c.bucket == b).count();
        Self {
            passed: n(Bucket::Pass),
            failed: n(Bucket::Fail),
            pending: n(Bucket::Pending),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadComment {
    pub author: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    pub id: String,
    pub path: String,
    pub line: Option<u32>,
    pub resolved: bool,
    pub comments: Vec<ThreadComment>,
}

pub trait GithubClient {
    fn checks(&self, pr: u64) -> Result<Vec<Check>>;
    fn threads(&self, pr: u64) -> Result<Vec<Thread>>;
}

pub struct GhClient;

#[derive(Deserialize)]
struct RawCheck {
    name: String,
    bucket: String,
}

pub fn parse_checks(json: &str) -> Result<Vec<Check>> {
    let raw: Vec<RawCheck> =
        serde_json::from_str(json).context("unexpected `gh pr checks` output")?;
    Ok(raw
        .into_iter()
        .map(|r| Check {
            bucket: match r.bucket.as_str() {
                "pass" => Bucket::Pass,
                "fail" | "cancel" => Bucket::Fail,
                "pending" => Bucket::Pending,
                _ => Bucket::Skipped,
            },
            name: r.name,
        })
        .collect())
}

pub fn parse_threads(json: &str) -> Result<Vec<Thread>> {
    let v: serde_json::Value = serde_json::from_str(json).context("unexpected GraphQL output")?;
    let nodes = v
        .pointer("/data/repository/pullRequest/reviewThreads/nodes")
        .and_then(|n| n.as_array())
        .context("pull request not found in GraphQL response")?;
    let s = |v: &serde_json::Value, p: &str| {
        v.pointer(p)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    Ok(nodes
        .iter()
        .map(|n| Thread {
            id: s(n, "/id"),
            path: s(n, "/path"),
            line: n
                .pointer("/line")
                .and_then(|l| l.as_u64())
                .map(|l| l as u32),
            resolved: n
                .pointer("/isResolved")
                .and_then(|b| b.as_bool())
                .unwrap_or(false),
            comments: n
                .pointer("/comments/nodes")
                .and_then(|c| c.as_array())
                .map(|cs| {
                    cs.iter()
                        .map(|c| ThreadComment {
                            author: s(c, "/author/login"),
                            body: s(c, "/body"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect())
}

/// Turn a failed `gh` run into a message the user can act on.
fn explain(stderr: &str) -> String {
    let e = stderr.trim();
    if e.contains("auth login") || e.contains("not logged") || e.contains("HTTP 401") {
        "gh is not authenticated; run `gh auth login`".to_string()
    } else if e.contains("HTTP 403") {
        format!("GitHub denied access (403): {e}")
    } else {
        format!("gh failed: {e}")
    }
}

fn gh(args: &[&str]) -> Result<std::process::Output> {
    Command::new("gh")
        .args(args)
        .output()
        .context("`gh` is not installed or not on PATH; GitHub features are unavailable")
}

const THREADS_QUERY: &str = "query($owner:String!,$name:String!,$n:Int!){repository(owner:$owner,name:$name){pullRequest(number:$n){reviewThreads(first:100){nodes{id isResolved path line comments(first:20){nodes{author{login} body}}}}}}}";

impl GithubClient for GhClient {
    fn checks(&self, pr: u64) -> Result<Vec<Check>> {
        let out = gh(&["pr", "checks", &pr.to_string(), "--json", "name,bucket"])?;
        // `gh pr checks` exits non-zero for failing or pending checks but still prints the JSON.
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.trim().is_empty() {
            if out.status.success() {
                return Ok(Vec::new());
            }
            bail!(explain(&String::from_utf8_lossy(&out.stderr)));
        }
        parse_checks(&stdout)
    }

    fn threads(&self, pr: u64) -> Result<Vec<Thread>> {
        let q = format!("query={THREADS_QUERY}");
        let n = format!("n={pr}");
        let out = gh(&[
            "api",
            "graphql",
            "-F",
            "owner={owner}",
            "-F",
            "name={repo}",
            "-F",
            &n,
            "-f",
            &q,
        ])?;
        if !out.status.success() {
            bail!(explain(&String::from_utf8_lossy(&out.stderr)));
        }
        parse_threads(&String::from_utf8_lossy(&out.stdout))
    }
}

/// In-memory client for tests; set `error` to simulate a failure.
#[derive(Default)]
pub struct FakeClient {
    pub checks: Vec<Check>,
    pub threads: Vec<Thread>,
    pub error: Option<String>,
}

impl GithubClient for FakeClient {
    fn checks(&self, _pr: u64) -> Result<Vec<Check>> {
        match &self.error {
            Some(e) => bail!("{e}"),
            None => Ok(self.checks.clone()),
        }
    }
    fn threads(&self, _pr: u64) -> Result<Vec<Thread>> {
        match &self.error {
            Some(e) => bail!("{e}"),
            None => Ok(self.threads.clone()),
        }
    }
}

#[cfg(test)]
mod tests;
