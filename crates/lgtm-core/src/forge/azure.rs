//! Azure DevOps through the `az` CLI (`azure-devops` extension). lgtm never touches tokens.
use super::{Forge, Unsupported};
use crate::git::GitEngine;
use crate::github::{Bucket, Check, GithubClient, Thread};
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

pub struct AzureForge {
    repo: PathBuf,
    runner: Box<dyn Runner>,
}

impl AzureForge {
    pub fn new(repo: PathBuf) -> Self {
        let runner = Box::new(SystemRunner(repo.clone()));
        Self { repo, runner }
    }

    fn az_json(&self, args: &[&str]) -> Result<Value> {
        let mut full = args.to_vec();
        full.extend(["--detect", "true", "--output", "json"]);
        let text = self.runner.run("az", &full)?;
        serde_json::from_str(&text).context("unexpected `az` output")
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

    /// Fetches both branches into private refs, then diffs `target...source` locally.
    fn pr_diff(&self, pr: u64) -> Result<DiffSnapshot> {
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

    fn post_review(&self, _: u64, _: &str, _: &str) -> Result<()> {
        Unsupported("Azure DevOps review posting").post_review(0, "", "")
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
    fn threads(&self, _: u64) -> Result<Vec<Thread>> {
        Unsupported("Azure DevOps threads").threads(0)
    }
    fn head_sha(&self, _: u64) -> Result<String> {
        Unsupported("Azure DevOps inline comments").head_sha(0)
    }
    fn post_inline(&self, _: u64, _: &str, _: &str, _: u32, _: &str) -> Result<u64> {
        Unsupported("Azure DevOps inline comments").post_inline(0, "", "", 0, "")
    }
    fn post_general(&self, _: u64, _: &str) -> Result<u64> {
        Unsupported("Azure DevOps comments").post_general(0, "")
    }
    fn reply(&self, _: u64, _: u64, _: &str) -> Result<u64> {
        Unsupported("Azure DevOps replies").reply(0, 0, "")
    }
    fn set_resolved(&self, _: &str, _: bool) -> Result<()> {
        Unsupported("Azure DevOps thread resolution").set_resolved("", false)
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
    fn missing_branches_and_writes_are_errors() {
        let f = forge(".", vec![("pr show", "{}")]);
        assert!(f.pr_diff(1).unwrap_err().to_string().contains("no source"));
        assert!(f.threads(1).is_err() && f.post_general(1, "x").is_err());
        assert!(f.post_review(1, "x", "y").is_err());
    }

    #[test]
    fn explains_az_errors() {
        assert!(explain("Please run 'az login' to setup account").contains("az login"));
        assert!(explain("'repos' is not in the 'az' command group").contains("extension add"));
        assert!(explain("boom").starts_with("az failed"));
    }
}
