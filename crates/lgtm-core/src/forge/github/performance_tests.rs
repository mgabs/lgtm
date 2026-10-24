//! Offline request budgets and an opt-in live GitHub latency benchmark.
use super::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const PATCH: &str = "diff --git a/review.txt b/review.txt\n--- a/review.txt\n+++ b/review.txt\n@@ -1 +1,2 @@\n base\n+PR change\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Requests {
    merge: usize,
    other_fetches: usize,
    api_patch: usize,
    cli_patch: usize,
}

#[derive(Clone, Copy)]
enum Transport {
    Live,
    LocalMerge,
    StoredPatch,
    FailedFetch(super::super::failure::Kind),
    FailedPatch,
}

struct MeteredRunner {
    repo: PathBuf,
    requests: Arc<Mutex<Requests>>,
    transport: Transport,
}

impl Runner for MeteredRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        {
            let mut counts = self.requests.lock().unwrap();
            match program {
                "git" if args.first() == Some(&"fetch") => {
                    if args.iter().any(|arg| arg.contains("/merge:")) {
                        counts.merge += 1;
                    } else {
                        counts.other_fetches += 1;
                    }
                }
                "curl" => counts.api_patch += 1,
                "gh" if args.starts_with(&["pr", "diff"]) => counts.cli_patch += 1,
                _ => bail!("unexpected benchmark command"),
            }
        }
        match self.transport {
            Transport::Live => SystemRunner(self.repo.clone()).run(program, args),
            Transport::LocalMerge => {
                assert_eq!(program, "git", "local fixture must not contact a provider");
                SystemRunner(self.repo.clone()).run(program, args)
            }
            Transport::FailedFetch(kind) => Err(super::super::failure::Failure {
                kind,
                exit_code: Some(128),
                http_status: None,
            }
            .into()),
            Transport::StoredPatch | Transport::FailedPatch if program == "git" => Err(
                super::super::failure::classify("git", Some(128), "couldn't find remote ref")
                    .into(),
            ),
            Transport::FailedPatch => Err(super::super::failure::Failure {
                kind: super::super::failure::Kind::Network,
                exit_code: Some(7),
                http_status: None,
            }
            .into()),
            Transport::StoredPatch => {
                if program == "curl" {
                    assert!(args.contains(&"Accept: application/vnd.github.diff"));
                }
                Ok(PATCH.into())
            }
        }
    }
}

fn provider(
    repo: PathBuf,
    requests: Arc<Mutex<Requests>>,
    transport: Transport,
    api: bool,
    missing_merges: MissingMergeRefs,
) -> GitHubForge {
    GitHubForge {
        repo: repo.clone(),
        // A missing parsed remote forces the CLI path independently of CI token env vars.
        remote: api.then(|| Remote {
            owner: "fixture".into(),
            repo: "repo".into(),
        }),
        is_github_com: api,
        runner: Box::new(MeteredRunner {
            repo,
            requests,
            transport,
        }),
        explicit_token: Some("fixture-token".into()),
        missing_merges,
    }
}

#[test]
fn merge_ref_cold_and_reload_request_budgets() {
    let temp = tempfile::tempdir().unwrap();
    let upstream = git2::Repository::init(temp.path().join("upstream")).unwrap();
    let signature = git2::Signature::now("test", "test@example.invalid").unwrap();
    let commit = |text: &[u8], parents: &[git2::Oid]| {
        let blob = upstream.blob(text).unwrap();
        let mut builder = upstream.treebuilder(None).unwrap();
        builder.insert("review.txt", blob, 0o100644).unwrap();
        let tree = upstream.find_tree(builder.write().unwrap()).unwrap();
        let parents = parents
            .iter()
            .map(|id| upstream.find_commit(*id).unwrap())
            .collect::<Vec<_>>();
        let refs = parents.iter().collect::<Vec<_>>();
        upstream
            .commit(None, &signature, &signature, "fixture", &tree, &refs)
            .unwrap()
    };
    let base = commit(b"base\n", &[]);
    let head = commit(b"base\nPR change\n", &[base]);
    let merge = commit(b"base\nPR change\n", &[base, head]);
    upstream
        .reference("refs/pull/42/merge", merge, true, "fixture")
        .unwrap();
    let client = temp.path().join("client");
    git2::Repository::init(&client)
        .unwrap()
        .remote("origin", upstream.path().to_str().unwrap())
        .unwrap();
    let requests = Arc::new(Mutex::new(Requests::default()));
    let missing: MissingMergeRefs = Default::default();
    assert!(git2::Repository::open(&client)
        .unwrap()
        .find_commit(head)
        .is_err());
    let started = Instant::now();
    for load in 1..=11 {
        let diff = provider(
            client.clone(),
            requests.clone(),
            Transport::LocalMerge,
            true,
            missing.clone(),
        )
        .pr_diff(42)
        .unwrap();
        assert_eq!(
            (diff.files.len(), diff.total_additions, diff.total_deletions),
            (1, 1, 0)
        );
        assert_eq!(
            *requests.lock().unwrap(),
            Requests {
                merge: load,
                ..Requests::default()
            }
        );
    }
    println!(
        "GitHub merge ref: first load + 10 reloads={:?}, requests={:?}; Git uses a local remote",
        started.elapsed(),
        requests.lock().unwrap()
    );
}

#[test]
fn historical_api_patch_request_budget() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let missing: MissingMergeRefs = Default::default();
    for load in 1..=11 {
        let diff = provider(
            PathBuf::from("."),
            requests.clone(),
            Transport::StoredPatch,
            true,
            missing.clone(),
        )
        .pr_diff(42)
        .unwrap();
        assert_eq!((diff.total_additions, diff.total_deletions), (1, 0));
        assert_eq!(
            *requests.lock().unwrap(),
            Requests {
                merge: 1,
                api_patch: load,
                ..Requests::default()
            }
        );
    }
}

#[test]
fn historical_cli_patch_request_budget() {
    let requests = Arc::new(Mutex::new(Requests::default()));
    let missing: MissingMergeRefs = Default::default();
    for load in 1..=4 {
        let diff = provider(
            PathBuf::from("."),
            requests.clone(),
            Transport::StoredPatch,
            false,
            missing.clone(),
        )
        .pr_diff(42)
        .unwrap();
        assert_eq!((diff.total_additions, diff.total_deletions), (1, 0));
        assert_eq!(
            *requests.lock().unwrap(),
            Requests {
                merge: 1,
                cli_patch: load,
                ..Requests::default()
            }
        );
    }
}

#[test]
fn fetch_errors_stop_with_one_request() {
    use super::super::failure::Kind;
    for kind in [
        Kind::Authentication,
        Kind::AccessDenied,
        Kind::Network,
        Kind::Timeout,
    ] {
        let requests = Arc::new(Mutex::new(Requests::default()));
        let error = provider(
            PathBuf::from("."),
            requests.clone(),
            Transport::FailedFetch(kind),
            true,
            Default::default(),
        )
        .pr_diff(42)
        .unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<super::super::failure::Failure>()
                .unwrap()
                .kind,
            kind
        );
        assert_eq!(
            *requests.lock().unwrap(),
            Requests {
                merge: 1,
                ..Requests::default()
            }
        );
    }
}

#[test]
fn failed_patch_does_not_try_another_transport() {
    for api in [true, false] {
        let requests = Arc::new(Mutex::new(Requests::default()));
        assert!(provider(
            PathBuf::from("."),
            requests.clone(),
            Transport::FailedPatch,
            api,
            Default::default(),
        )
        .pr_diff(42)
        .is_err());
        assert_eq!(
            *requests.lock().unwrap(),
            Requests {
                merge: 1,
                api_patch: usize::from(api),
                cli_patch: usize::from(!api),
                other_fetches: 0,
            }
        );
    }
}

#[test]
#[ignore = "live GitHub benchmark: set LGTM_PERF_REPO and LGTM_PERF_PR; uses existing Git/API credentials and fetches PR objects"]
fn live_github_pr_load_benchmark() {
    let repo = PathBuf::from(
        std::env::var("LGTM_PERF_REPO").expect("set LGTM_PERF_REPO to a GitHub checkout"),
    );
    let pr = std::env::var("LGTM_PERF_PR")
        .expect("set LGTM_PERF_PR")
        .parse::<u64>()
        .expect("PR must be a number");
    let runs = std::env::var("LGTM_PERF_RUNS")
        .unwrap_or_else(|_| "10".into())
        .parse::<usize>()
        .expect("runs must be a number");
    assert!((2..=100).contains(&runs), "LGTM_PERF_RUNS must be 2..=100");
    assert!(
        matches!(
            super::super::kind_of(&repo),
            super::super::ForgeKind::GitHub
        ),
        "LGTM_PERF_REPO must have a GitHub origin"
    );
    let requests = Arc::new(Mutex::new(Requests::default()));
    let mut reloads = Vec::new();
    for index in 0..runs {
        let provider = GitHubForge::with_runner(
            repo.clone(),
            Box::new(MeteredRunner {
                repo: repo.clone(),
                requests: requests.clone(),
                transport: Transport::Live,
            }),
        );
        let before = *requests.lock().unwrap();
        let started = Instant::now();
        let result = provider.pr_diff(pr);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let after = *requests.lock().unwrap();
        let diff = result.unwrap_or_else(|error| {
            eprintln!(
                "sample={} elapsed_ms={elapsed:.2} outcome=error total_requests={after:?}",
                index + 1
            );
            panic!("sample {} failed: {error:#}", index + 1);
        });
        println!("sample={} elapsed_ms={elapsed:.2} files={} merge={} other_fetches={} api_patch={} cli_patch={}", index + 1, diff.files.len(), after.merge - before.merge, after.other_fetches - before.other_fetches, after.api_patch - before.api_patch, after.cli_patch - before.cli_patch);
        if index > 0 {
            reloads.push(elapsed);
        }
    }
    reloads.sort_by(f64::total_cmp);
    let median = if reloads.len() % 2 == 0 {
        (reloads[reloads.len() / 2 - 1] + reloads[reloads.len() / 2]) / 2.0
    } else {
        reloads[reloads.len() / 2]
    };
    let p95 = reloads[(reloads.len() * 95).div_ceil(100) - 1];
    println!(
        "reloads={} median_ms={median:.2} p95_ms={p95:.2} total_requests={:?}",
        reloads.len(),
        requests.lock().unwrap()
    );
    println!("Existing Git objects are retained. No timing threshold or before/after speedup is inferred.");
}
