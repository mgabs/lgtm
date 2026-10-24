//! Request budgets run in CI; the ignored benchmark measures an explicitly selected live PR.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Requests {
    token: usize,
    metadata: usize,
    cli_metadata: usize,
    merge: usize,
    commits: usize,
    branches: usize,
}

struct MeteredRunner {
    repo: PathBuf,
    requests: Arc<Mutex<Requests>>,
    // None delegates every command to the real transport for the opt-in live benchmark.
    metadata: Option<String>,
}

impl Runner for MeteredRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        {
            let mut counts = self.requests.lock().unwrap();
            if program == "az" && args.contains(&"get-access-token") {
                counts.token += 1;
            } else if program == "az" && args.contains(&"show") {
                counts.cli_metadata += 1;
            } else if program == "curl" {
                counts.metadata += 1;
            } else if program == "git" && args.first() == Some(&"fetch") {
                if args.iter().any(|arg| arg.contains("/merge:")) {
                    counts.merge += 1;
                } else if args.iter().any(|arg| arg.starts_with("+refs/heads/")) {
                    counts.branches += 1;
                } else {
                    counts.commits += 1;
                }
            }
        }
        if let Some(metadata) = &self.metadata {
            if program == "az" && args.contains(&"get-access-token") {
                return Ok("fixture-token".into());
            }
            if program == "curl" {
                return Ok(metadata.clone());
            }
            if program == "az" {
                bail!("unexpected CLI fallback in performance fixture");
            }
            if args.iter().any(|arg| arg.contains("/merge:")) {
                return Err(super::super::failure::classify(
                    "git",
                    Some(128),
                    "couldn't find remote ref",
                )
                .into());
            }
        }
        SystemRunner(self.repo.clone()).run(program, args)
    }
}

/// Isolates the process caches used by production forges from other cache tests.
struct CacheGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl CacheGuard {
    fn acquire() -> Self {
        let guard = Self {
            _lock: SHARED_CACHE_TEST_LOCK.lock().unwrap(),
        };
        *TOKEN.lock().unwrap() = None;
        CACHE.lock().unwrap().clear();
        guard
    }
}

impl Drop for CacheGuard {
    fn drop(&mut self) {
        *TOKEN.lock().unwrap() = None;
        CACHE.lock().unwrap().clear();
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    client: PathBuf,
    source: git2::Oid,
    target: git2::Oid,
    requests: Arc<Mutex<Requests>>,
    missing: MissingMergeRefs,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let upstream = git2::Repository::init(temp.path().join("upstream")).unwrap();
        let signature = git2::Signature::now("test", "test@example.invalid").unwrap();
        let commit = |contents: &[u8], parent: Option<git2::Oid>, branch| {
            let blob = upstream.blob(contents).unwrap();
            let mut tree = upstream.treebuilder(None).unwrap();
            tree.insert("review.txt", blob, 0o100644).unwrap();
            let tree = upstream.find_tree(tree.write().unwrap()).unwrap();
            let parent = parent.map(|id| upstream.find_commit(id).unwrap());
            let parents = parent.iter().collect::<Vec<_>>();
            upstream
                .commit(
                    Some(branch),
                    &signature,
                    &signature,
                    "fixture",
                    &tree,
                    &parents,
                )
                .unwrap()
        };
        let target = commit(b"base\n", None, "refs/heads/main");
        let source = commit(b"base\nPR change\n", Some(target), "refs/heads/feature");
        let client = temp.path().join("client");
        git2::Repository::init(&client)
            .unwrap()
            .remote("origin", upstream.path().to_str().unwrap())
            .unwrap();
        Self {
            _temp: temp,
            client,
            source,
            target,
            requests: Default::default(),
            missing: Default::default(),
        }
    }

    fn provider(&self, recorded: bool) -> AzureForge {
        let metadata = if recorded {
            serde_json::json!({
                "status": "completed",
                "lastMergeSourceCommit": {"commitId": self.source.to_string()},
                "lastMergeTargetCommit": {"commitId": self.target.to_string()}
            })
        } else {
            serde_json::json!({
                "status": "active", "sourceRefName": "refs/heads/feature",
                "targetRefName": "refs/heads/main"
            })
        };
        AzureForge {
            repo: self.client.clone(),
            remote: parse_remote("https://dev.azure.com/perf/fixture/_git/repo"),
            runner: Box::new(MeteredRunner {
                repo: self.client.clone(),
                requests: self.requests.clone(),
                metadata: Some(metadata.to_string()),
            }),
            shared: true,
            missing_merges: self.missing.clone(),
        }
    }

    fn counts(&self) -> Requests {
        *self.requests.lock().unwrap()
    }
}

#[test]
fn historical_pr_cold_and_reload_request_budgets() {
    let _cache = CacheGuard::acquire();
    let fixture = Fixture::new();
    assert!(!super::super::have_commits(
        &fixture.client,
        &fixture.source.to_string(),
        &fixture.target.to_string()
    ));
    let start = Instant::now();
    let first = fixture.provider(true).pr_diff(42).unwrap();
    let cold = start.elapsed();
    assert_eq!((first.total_additions, first.total_deletions), (1, 0));
    assert_eq!(
        fixture.counts(),
        Requests {
            token: 1,
            metadata: 1,
            commits: 1,
            ..Requests::default()
        }
    );

    // Commands create separate forges in production; caches must survive that boundary.
    let start = Instant::now();
    for _ in 0..10 {
        let diff = fixture.provider(true).pr_diff(42).unwrap();
        assert_eq!((diff.total_additions, diff.total_deletions), (1, 0));
    }
    let reloads = start.elapsed();
    assert_eq!(
        fixture.counts(),
        Requests {
            token: 1,
            metadata: 11,
            commits: 1,
            ..Requests::default()
        }
    );
    println!("historical PR: cold={cold:?}, 10 reloads={reloads:?}, requests={:?}; metadata/auth mocked, Git uses a local remote", fixture.counts());
}

#[test]
fn incomplete_metadata_reload_budget_skips_missing_ref_until_expiry() {
    let _cache = CacheGuard::acquire();
    let fixture = Fixture::new();
    assert_eq!(
        fixture.provider(false).pr_diff(42).unwrap().total_additions,
        1
    );
    assert_eq!(
        fixture.counts(),
        Requests {
            token: 1,
            metadata: 1,
            merge: 1,
            branches: 1,
            ..Requests::default()
        }
    );
    for _ in 0..3 {
        assert_eq!(
            fixture.provider(false).pr_diff(42).unwrap().total_additions,
            1
        );
    }
    assert_eq!(
        fixture.counts(),
        Requests {
            token: 1,
            metadata: 4,
            merge: 1,
            branches: 4,
            ..Requests::default()
        }
    );
    let provider = fixture.provider(false);
    fixture
        .missing
        .lock()
        .unwrap()
        .insert(provider.merge_key(42), Instant::now() - MISSING_MERGE_TTL);
    assert_eq!(provider.pr_diff(42).unwrap().total_additions, 1);
    assert_eq!(
        fixture.counts(),
        Requests {
            token: 1,
            metadata: 5,
            merge: 2,
            branches: 5,
            ..Requests::default()
        }
    );
}

#[test]
#[ignore = "live Azure benchmark: set LGTM_PERF_REPO and LGTM_PERF_PR; uses credentials and fetches PR objects"]
fn live_azure_pr_load_benchmark() {
    let _cache = CacheGuard::acquire();
    let repo = PathBuf::from(
        std::env::var("LGTM_PERF_REPO").expect("set LGTM_PERF_REPO to an Azure checkout"),
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
        matches!(super::super::kind_of(&repo), super::super::ForgeKind::Azure),
        "LGTM_PERF_REPO must have an Azure origin"
    );
    let requests = Arc::new(Mutex::new(Requests::default()));
    let missing: MissingMergeRefs = Default::default();
    let mut reloads = Vec::new();
    for index in 0..runs {
        let mut provider = AzureForge::new(repo.clone());
        provider.missing_merges = missing.clone();
        provider.runner = Box::new(MeteredRunner {
            repo: repo.clone(),
            requests: requests.clone(),
            metadata: None,
        });
        let before = *requests.lock().unwrap();
        let start = Instant::now();
        let diff = provider
            .pr_diff(pr)
            .unwrap_or_else(|error| panic!("sample {} failed: {error:#}", index + 1));
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let after = *requests.lock().unwrap();
        println!("sample={} elapsed_ms={elapsed:.2} files={} token={} metadata={} cli_metadata={} merge={} commits={} branches={}", index + 1, diff.files.len(), after.token - before.token, after.metadata - before.metadata, after.cli_metadata - before.cli_metadata, after.merge - before.merge, after.commits - before.commits, after.branches - before.branches);
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
    println!("First sample starts with empty process caches; existing Git objects are retained. No timing threshold or before/after speedup is inferred.");
}
