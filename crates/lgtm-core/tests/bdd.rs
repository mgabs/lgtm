//! Acceptance scenarios exercised through lgtm-core's public API.
use lgtm_core::{
    git::GitEngine,
    pr::PrEngine,
    review::{file_reviewed, set_file, ReviewStore},
};

#[test]
fn bdd_core_001_removing_lgtm_preserves_an_existing_hook() {
    // Given a repository with a user's existing pre-commit hook.
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path()).unwrap();
    let engine = GitEngine::new(repo.path());
    let path = engine.hook_path("pre-commit").unwrap();
    let original = "#!/bin/sh\necho custom-check\n";
    std::fs::write(&path, original).unwrap();

    // When LGTM is installed twice and then removed.
    engine.install_pre_commit_hook().unwrap();
    let installed = std::fs::read_to_string(&path).unwrap();
    engine.install_pre_commit_hook().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), installed);
    assert!(engine.pre_commit_hook_status().unwrap());
    assert!(engine.uninstall_pre_commit_hook().unwrap());

    // Then the original hook remains, and LGTM is no longer installed.
    assert_eq!(
        std::fs::read_to_string(path).unwrap().trim_end(),
        original.trim_end()
    );
    assert!(!engine.pre_commit_hook_status().unwrap());
    assert!(!engine.uninstall_pre_commit_hook().unwrap());
}

fn patch(line: u32, content: &str) -> lgtm_core::model::DiffSnapshot {
    PrEngine::parse_unified_diff(&format!(
        "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -0,0 +{line},1 @@\n+{content}\n"
    ))
    .unwrap()
}

#[test]
fn bdd_core_002_saved_review_marks_follow_content_and_stay_with_their_pr() {
    // Given a marked PR hunk saved in a prior review session.
    let repo = tempfile::tempdir().unwrap();
    let store = ReviewStore::open_marks_only(repo.path(), "pr-42").unwrap();
    let mut state = store.load().unwrap();
    set_file(
        &mut state.hunks,
        &patch(1, "let x = 1;").files[0],
        true,
        "now",
    );
    store.save(&state).unwrap();
    drop(store);

    // When the session is reopened and that same code has moved down the file.
    let saved = ReviewStore::open_marks_only(repo.path(), "pr-42")
        .unwrap()
        .load()
        .unwrap();

    // Then moved content stays reviewed, changed content needs review,
    // and another PR does not inherit the marks.
    assert!(file_reviewed(
        &saved.hunks,
        &patch(40, "let x = 1;").files[0]
    ));
    assert!(!file_reviewed(
        &saved.hunks,
        &patch(40, "let x = 2;").files[0]
    ));
    let other = ReviewStore::open_marks_only(repo.path(), "pr-43")
        .unwrap()
        .load()
        .unwrap();
    assert!(other.hunks.is_empty());
}

#[derive(Default)]
struct HistoricalGithub {
    calls: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    denied: bool,
}

impl lgtm_core::forge::github::Runner for HistoricalGithub {
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<String> {
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        if program == "git" {
            assert_eq!(args[0], "fetch");
            assert!(args.iter().any(|arg| arg.contains("refs/pull/42/merge:")));
            let message = if self.denied {
                "Authentication failed"
            } else {
                "couldn't find remote ref"
            };
            return Err(lgtm_core::forge::failure::classify("git", Some(128), message).into());
        }
        assert!(!self.denied, "authentication failure must stop the load");
        assert_eq!((program, args), ("gh", &["pr", "diff", "42"][..]));
        Ok("diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1,2 @@\n base\n+historical change\n".into())
    }
}

#[test]
fn bdd_core_003_missing_merge_ref_loads_the_historical_patch() {
    use lgtm_core::forge::Forge;
    // Given a historical PR whose merge ref no longer exists. No origin remote
    // is configured, so this fixture uses gh regardless of token environment.
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path()).unwrap();
    let runner = HistoricalGithub::default();
    let calls = runner.calls.clone();
    let provider = lgtm_core::forge::GitHubForge::with_runner(repo.path().into(), Box::new(runner));

    // When the reviewer opens that PR through the public forge API.
    let snapshot = provider.pr_diff(42).unwrap();

    // Then the stored patch is shown without guessing current branches.
    assert_eq!(
        (
            snapshot.files.len(),
            snapshot.total_additions,
            snapshot.total_deletions
        ),
        (1, 1, 0)
    );
    assert!(snapshot.files[0].hunks[0]
        .lines
        .iter()
        .any(|line| line.content == "historical change"));
    assert_eq!(calls.borrow().len(), 2);
    assert_eq!(calls.borrow()[1], "gh pr diff 42");
}

#[test]
fn bdd_core_004_authentication_failure_does_not_probe_another_transport() {
    use lgtm_core::forge::Forge;
    // Given a repository whose fetch is rejected for authentication.
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path()).unwrap();
    let runner = HistoricalGithub {
        denied: true,
        ..Default::default()
    };
    let calls = runner.calls.clone();
    let provider = lgtm_core::forge::GitHubForge::with_runner(repo.path().into(), Box::new(runner));

    // When the reviewer opens the PR, then loading fails after the first request.
    assert!(provider.pr_diff(42).is_err());
    assert_eq!(calls.borrow().len(), 1);
}

fn directive(text: &str) -> lgtm_core::comment::CommentAnchor {
    serde_json::from_value(serde_json::json!({
        "id": lgtm_core::comment::new_id(), "file_path": "a.rs", "target_line_no": 1,
        "target_code": "code", "context_before": [], "context_after": [],
        "comment_text": text, "created_at_turn": 1, "is_resolved": false, "pr_number": 42
    }))
    .unwrap()
}

#[test]
fn bdd_core_005_retry_after_reopening_skips_durably_posted_comments() {
    use lgtm_core::{
        comment::CommentStore,
        github::{post, FakeClient},
    };
    // Given two directives and a provider that fails after the first successful post.
    let repo = tempfile::tempdir().unwrap();
    let store = CommentStore::new(repo.path());
    let mut comments = vec![directive("first"), directive("second")];
    store.save_comments(&comments).unwrap();
    let files = patch(1, "code").files;
    let client = FakeClient {
        fail_post_after: Some(1),
        ..Default::default()
    };
    let plan = post::plan(&comments, &files, 42);

    // When posting stops and the reviewer reopens saved directives before retrying.
    let report = post::post(
        &client,
        42,
        &mut comments,
        &plan,
        std::time::Duration::ZERO,
        |current| store.save_comments(current),
    );
    assert_eq!(report.posted, 1);
    assert!(report.error.is_some());
    let mut reopened = CommentStore::new(repo.path()).load_comments();
    assert!(reopened[0].remote_id.is_some());
    assert!(reopened[1].remote_id.is_none());
    let retry = post::plan(&reopened, &files, 42);
    assert_eq!(retry.inline, vec![1]);
    let recovered = FakeClient::default();
    let report = post::post(
        &recovered,
        42,
        &mut reopened,
        &retry,
        std::time::Duration::ZERO,
        |current| store.save_comments(current),
    );

    // Then only the remaining directive is sent, and both receipts are saved.
    assert!(report.error.is_none());
    assert_eq!(report.posted, 1);
    assert_eq!(client.posted.borrow().as_slice(), ["a.rs:1:first"]);
    assert_eq!(recovered.posted.borrow().as_slice(), ["a.rs:1:second"]);
    assert_eq!(store.load_comments(), reopened);
    assert_eq!(post::plan(&reopened, &files, 42).total(), 0);
}

#[test]
fn bdd_core_006_receipt_save_failure_stops_the_next_remote_write() {
    use lgtm_core::{
        comment::CommentStore,
        github::{post, FakeClient},
    };
    // Given two directives and a destination that cannot save the first receipt.
    let repo = tempfile::tempdir().unwrap();
    let store = CommentStore::new(repo.path());
    std::fs::create_dir(repo.path().join(".lgtm/comments.jsonl")).unwrap();
    let mut comments = vec![directive("first"), directive("second")];
    let files = patch(1, "code").files;
    let client = FakeClient::default();
    let plan = post::plan(&comments, &files, 42);

    // When the provider accepts the first comment but its local save fails.
    let report = post::post(
        &client,
        42,
        &mut comments,
        &plan,
        std::time::Duration::ZERO,
        |current| store.save_comments(current),
    );

    // Then the failure is reported and the second comment is never sent.
    assert_eq!(report.posted, 1);
    assert!(report
        .error
        .as_deref()
        .unwrap()
        .contains("Failed to open comments.jsonl"));
    assert_eq!(client.posted.borrow().as_slice(), ["a.rs:1:first"]);
    assert!(comments[0].remote_id.is_some());
    assert!(comments[1].remote_id.is_none());
}

#[test]
fn bdd_core_007_posting_uses_only_unresolved_unposted_directives_for_this_pr() {
    use lgtm_core::{
        comment::CommentStore,
        github::{post, FakeClient},
    };
    // Given a saved review containing posted, resolved, other-PR and local-only directives.
    let repo = tempfile::tempdir().unwrap();
    let store = CommentStore::new(repo.path());
    let eligible = directive("send this");
    let mut posted = directive("already sent");
    posted.remote_id = Some(99);
    let mut resolved = directive("resolved");
    resolved.is_resolved = true;
    let mut other_pr = directive("other review");
    other_pr.pr_number = Some(43);
    let mut local = directive("local only");
    local.pr_number = None;
    let original = vec![eligible, posted, resolved, other_pr, local];
    store.save_comments(&original).unwrap();
    let mut comments = store.load_comments();
    let files = patch(1, "code").files;
    let client = FakeClient::default();

    // When the reviewer posts the current PR's outstanding directives.
    let plan = post::plan(&comments, &files, 42);
    let report = post::post(
        &client,
        42,
        &mut comments,
        &plan,
        std::time::Duration::ZERO,
        |current| store.save_comments(current),
    );

    // Then only the eligible directive is sent; all excluded directives stay intact.
    assert!(report.error.is_none());
    assert_eq!(report.posted, 1);
    assert_eq!(client.posted.borrow().as_slice(), ["a.rs:1:send this"]);
    assert_eq!(&store.load_comments()[1..], &original[1..]);
}

#[test]
fn bdd_core_008_provider_threads_show_equivalent_open_and_resolved_markers() {
    use lgtm_core::github::{parse_threads, thread_marker};
    // Given equivalent GitHub and Azure discussions on the same reviewed line.
    for (github_closed, azure_status) in [
        (false, "active"),
        (true, "fixed"),
        (true, "closed"),
        (true, "wontFix"),
        (true, "byDesign"),
    ] {
        let github = serde_json::json!({"data":{"repository":{"pullRequest":{
            "reviewThreads":{"nodes":[{
                "id":"thread-7", "path":"a.rs", "line":1, "isResolved":github_closed,
                "comments":{"nodes":[{"databaseId":10,"author":{"login":"reviewer"},"body":"please check"}]}
            }]}
        }}}});
        let azure = serde_json::json!({"value":[{
            "id":7, "status":azure_status,
            "threadContext":{"filePath":"/a.rs","rightFileStart":{"line":1}},
            "comments":[{"id":10,"author":{"displayName":"reviewer"},"content":"please check","commentType":"text"}]
        }]});

        // When provider responses are normalized for the review UI.
        let github_threads = parse_threads(&github.to_string()).unwrap();
        let azure_threads = lgtm_core::forge::azure::parse_threads(&azure);

        // Then users see the same location, text and resolution marker.
        for threads in [&github_threads, &azure_threads] {
            assert_eq!(threads.len(), 1);
            assert_eq!(threads[0].comments[0].body, "please check");
            assert_eq!(thread_marker(threads, "a.rs", 1), Some(!github_closed));
            assert_eq!(thread_marker(threads, "other.rs", 1), None);
        }
    }
}

#[test]
fn bdd_core_009_unsupported_review_version_is_rejected_without_overwriting_it() {
    // Given a saved review created by a newer, incompatible LGTM version.
    let repo = tempfile::tempdir().unwrap();
    let store = ReviewStore::open_marks_only(repo.path(), "pr-42").unwrap();
    let path = repo.path().join(".lgtm/reviews/pr-42.json");
    let newer = serde_json::json!({"version":u32::MAX,"target":"pr-42","hunks":[]});
    let original = serde_json::to_string_pretty(&newer).unwrap();
    std::fs::write(&path, &original).unwrap();

    // When this version tries to open the saved review.
    let error = store.load().unwrap_err();

    // Then it asks for an upgrade and leaves the original data untouched.
    assert!(error.to_string().contains("newer than supported"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
}

#[test]
fn bdd_core_010_state_filters_select_equivalent_github_and_azure_reviews() {
    use lgtm_core::pr::{PullRequestState, PullRequestSummary};
    // Given normalized listing responses containing open, merged and closed reviews.
    let github: Vec<PullRequestSummary> = serde_json::from_value(serde_json::json!([
        {"number":1,"title":"open","author":{"login":"reviewer"},"headRefName":"a","additions":0,"deletions":0,"state":"OPEN"},
        {"number":2,"title":"merged","author":{"login":"reviewer"},"headRefName":"b","additions":0,"deletions":0,"state":"MERGED"},
        {"number":3,"title":"closed","author":{"login":"reviewer"},"headRefName":"c","additions":0,"deletions":0,"state":"CLOSED"}
    ])).unwrap();
    let azure = lgtm_core::forge::azure::parse_prs(&serde_json::json!([
        {"pullRequestId":1,"title":"open","status":"active","sourceRefName":"refs/heads/a","createdBy":{"displayName":"reviewer"}},
        {"pullRequestId":2,"title":"merged","status":"completed","sourceRefName":"refs/heads/b","createdBy":{"displayName":"reviewer"}},
        {"pullRequestId":3,"title":"closed","status":"abandoned","sourceRefName":"refs/heads/c","createdBy":{"displayName":"reviewer"}}
    ]));

    // When the reviewer selects open, closed or all.
    for (state, expected) in [
        (PullRequestState::Open, vec![1]),
        (PullRequestState::Closed, vec![2, 3]),
        (PullRequestState::All, vec![1, 2, 3]),
    ] {
        // Then either provider selects the equivalent reviews; closed includes merged.
        for reviews in [&github, &azure] {
            let selected: Vec<u64> = reviews
                .iter()
                .filter(|pr| state.includes(&pr.state))
                .map(|pr| pr.number)
                .collect();
            assert_eq!(selected, expected);
        }
    }
}

#[derive(Clone)]
struct PublicReadRunner {
    calls: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    gh_failure: lgtm_core::forge::failure::Kind,
    http_status: Option<u16>,
    response: String,
    authenticated: bool,
    gh_response: Option<String>,
}

impl lgtm_core::forge::github::Runner for PublicReadRunner {
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<String> {
        use lgtm_core::forge::failure::{classify, Failure, Kind};
        // Store only fixture command targets; never retain credentials or body files.
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.last().unwrap()));
        match program {
            "git" => Err(classify("git", Some(128), "couldn't find remote ref").into()),
            "gh" if self.gh_response.is_some() => Ok(self.gh_response.clone().unwrap()),
            "gh" if self.gh_failure == Kind::Authentication => {
                Err(classify("gh", Some(4), "To get started, run: gh auth login").into())
            }
            "gh" => Err(Failure {
                kind: self.gh_failure,
                exit_code: None,
                http_status: None,
            }
            .into()),
            "curl" => {
                assert!(args.last().unwrap().starts_with("https://api.github.com/"));
                let header_file = args
                    .windows(2)
                    .find(|pair| pair[0] == "-H" && pair[1].starts_with('@'));
                if self.authenticated {
                    let header = header_file.expect("authenticated API must retain its credential");
                    assert!(std::fs::read_to_string(&header[1][1..])
                        .unwrap()
                        .contains("Bearer fixture-token"));
                } else {
                    assert!(
                        header_file.is_none(),
                        "anonymous requests carry no credential file"
                    );
                    assert!(!args.iter().any(|arg| arg.contains("Authorization")));
                    let expected_accept = if args.last().unwrap().contains('?') {
                        "Accept: application/vnd.github+json"
                    } else {
                        "Accept: application/vnd.github.diff"
                    };
                    assert!(args.contains(&expected_accept));
                    assert!(
                        !args.contains(&"-X") && !args.contains(&"-d"),
                        "anonymous reads cannot write"
                    );
                }
                if let Some(status) = self.http_status {
                    return Err(classify(
                        "curl",
                        Some(22),
                        &format!("The requested URL returned error: {status}"),
                    )
                    .into());
                }
                Ok(self.response.clone())
            }
            _ => panic!("unexpected provider command"),
        }
    }
}

fn public_provider(
    runner: PublicReadRunner,
    origin: &str,
) -> (tempfile::TempDir, lgtm_core::forge::GitHubForge) {
    let repo = tempfile::tempdir().unwrap();
    lgtm_core::git2::Repository::init(repo.path())
        .unwrap()
        .remote("origin", origin)
        .unwrap();
    let provider = lgtm_core::forge::GitHubForge::with_runner(repo.path().into(), Box::new(runner))
        .with_api_token(None); // Never depend on the developer's token environment.
    (repo, provider)
}

fn public_runner(response: &str) -> PublicReadRunner {
    PublicReadRunner {
        calls: Default::default(),
        gh_failure: lgtm_core::forge::failure::Kind::Authentication,
        http_status: None,
        response: response.into(),
        authenticated: false,
        gh_response: None,
    }
}

#[test]
fn bdd_core_011_public_pr_listing_works_without_gh_login() {
    use lgtm_core::{forge::Forge, pr::PullRequestState};
    // Given a public GitHub repo and gh that requires login, or is not installed.
    for missing in [false, true] {
        for (state, api_state, status) in [
            (PullRequestState::Open, "open", "open"),
            (PullRequestState::Closed, "closed", "closed"),
            (PullRequestState::All, "all", "closed"),
        ] {
            let merged = if status == "closed" {
                Some("2026-10-22")
            } else {
                None
            };
            let response = serde_json::json!([{"number":42,"title":"public","state":status,
                "merged_at":merged,"head":{"ref":"feature"},"user":{"login":"reviewer"}}]);
            let mut runner = public_runner(&response.to_string());
            if missing {
                runner.gh_failure = lgtm_core::forge::failure::Kind::ToolUnavailable;
            }
            let calls = runner.calls.clone();
            let (_repo, provider) =
                public_provider(runner, "https://github.com/fixture/public.git");

            // When the user lists PRs, then one anonymous GET returns the entries.
            let prs = provider.list_prs_state(state).unwrap();
            assert_eq!(prs.len(), 1);
            assert_eq!(prs[0].number, 42);
            assert_eq!(
                prs[0].state,
                if merged.is_some() { "merged" } else { "open" }
            );
            assert_eq!(calls.borrow().len(), 2);
            assert_eq!(calls.borrow()[1], format!("curl https://api.github.com/repos/fixture/public/pulls?state={api_state}&per_page=30"));
        }
    }
}

#[test]
fn bdd_core_012_historical_public_patch_works_without_gh_login() {
    use lgtm_core::forge::Forge;
    // Given a missing merge ref and gh that needs login.
    let runner = public_runner(
        "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -0,0 +1,1 @@\n+public change\n",
    );
    let calls = runner.calls.clone();
    let (_repo, provider) = public_provider(runner, "git@github.com:fixture/public.git");

    // When the historical PR loads, then its stored patch comes from anonymous HTTPS.
    let diff = provider.pr_diff(42).unwrap();
    assert_eq!((diff.files.len(), diff.total_additions), (1, 1));
    assert_eq!(diff.files[0].hunks[0].lines[0].content, "public change");
    assert_eq!(calls.borrow().len(), 3);
    assert_eq!(
        calls.borrow()[2],
        "curl https://api.github.com/repos/fixture/public/pulls/42"
    );
}

#[test]
fn bdd_core_013_private_or_rate_limited_anonymous_reads_stop_without_retry() {
    use lgtm_core::forge::Forge;
    // Given a private/missing repository, denied request, or exhausted public rate limit.
    for status in [404, 403, 429] {
        let mut runner = public_runner("");
        runner.http_status = Some(status);
        let calls = runner.calls.clone();
        let (_repo, provider) = public_provider(runner, "https://github.com/fixture/public.git");

        // When the anonymous fallback fails, then its status is retained and no retry runs.
        let error = provider.list_prs().unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<lgtm_core::forge::failure::Failure>()
                .unwrap()
                .http_status,
            Some(status)
        );
        assert!(error.to_string().contains("authentication"));
        assert!(error.to_string().contains(&format!("HTTP {status}")));
        assert_eq!(calls.borrow().len(), 2);
    }
}

#[test]
fn bdd_core_014_gh_network_failures_do_not_trigger_anonymous_reads() {
    use lgtm_core::{forge::failure::Kind, forge::Forge};
    // Given gh failed for connectivity rather than missing credentials or installation.
    for kind in [
        Kind::Network,
        Kind::Timeout,
        Kind::AccessDenied,
        Kind::Other,
    ] {
        let mut runner = public_runner("[]");
        runner.gh_failure = kind;
        let calls = runner.calls.clone();
        let (_repo, provider) = public_provider(runner, "https://github.com/fixture/public.git");
        // When loading the listing, then the original failure is returned after one request.
        let error = provider.list_prs().unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<lgtm_core::forge::failure::Failure>()
                .unwrap()
                .kind,
            kind
        );
        assert_eq!(calls.borrow().len(), 1);
    }
}

#[test]
fn bdd_core_015_other_hosts_never_use_github_com_anonymous_fallback() {
    use lgtm_core::forge::Forge;
    // Given an enterprise host, SSH alias, or a hostname resembling github.com.
    for origin in [
        "https://github.enterprise.invalid/org/repo.git",
        "https://github.com.attacker.invalid/org/repo.git",
        "git@github-personal:org/repo.git",
    ] {
        let runner = public_runner("[]");
        let calls = runner.calls.clone();
        let (_repo, provider) = public_provider(runner, origin);
        // When gh needs login, then no request is sent to GitHub.com's API.
        assert!(provider.list_prs().is_err());
        assert_eq!(calls.borrow().len(), 1);
    }
}

#[test]
fn bdd_core_016_invalid_tokens_do_not_downgrade_reads_or_writes() {
    use lgtm_core::{forge::Forge, github::GithubClient};
    // Given an explicit API token rejected by GitHub.
    let mut runner = public_runner("");
    runner.authenticated = true;
    runner.http_status = Some(401);
    let calls = runner.calls.clone();
    let (_repo, provider) = public_provider(runner, "https://github.com/fixture/public.git");
    let provider = provider.with_api_token(Some("fixture-token".into()));

    // When reading or writing, then each request fails without CLI or anonymous retry.
    assert!(provider.list_prs().is_err());
    assert!(provider.post_general(42, "text").is_err());
    assert!(provider.post_inline(42, "sha", "a.rs", 1, "text").is_err());
    assert!(provider.reply(42, "thread", 1, "text").is_err());
    assert!(provider.set_resolved(42, "thread", true).is_err());
    assert!(provider.post_review(42, "COMMENT", "text").is_err());
    assert_eq!(calls.borrow().len(), 6);
    assert!(calls.borrow().iter().all(|call| call.starts_with("curl ")));
}

#[test]
fn bdd_core_017_valid_gh_reads_do_not_make_anonymous_requests() {
    use lgtm_core::forge::Forge;
    // Given gh already returns the public PR listing successfully.
    let mut runner = public_runner("unneeded anonymous response");
    runner.gh_response = Some(r#"[{"number":42,"title":"PR","author":{"login":"reviewer"},"headRefName":"feature","additions":1,"deletions":0,"state":"OPEN"}]"#.into());
    let calls = runner.calls.clone();
    let (_repo, provider) = public_provider(runner, "https://github.com/fixture/public.git");
    // When listing, then the existing CLI result is used with no additional HTTP request.
    assert_eq!(provider.list_prs().unwrap()[0].number, 42);
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn bdd_core_018_invalid_anonymous_listing_is_reported_without_retry() {
    use lgtm_core::forge::Forge;
    // Given a successful HTTP response that is not a PR array.
    for response in ["invalid JSON", r#"{"message":"unexpected API response"}"#] {
        let runner = public_runner(response);
        let calls = runner.calls.clone();
        let (_repo, provider) = public_provider(runner, "https://github.com/fixture/public.git");
        // When reading the listing, then it fails instead of opening an empty picker or retrying.
        assert!(provider.list_prs().is_err());
        assert_eq!(calls.borrow().len(), 2);
    }
}
