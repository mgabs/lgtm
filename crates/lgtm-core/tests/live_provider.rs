//! Opt-in live provider acceptance checks for GitHub and Azure DevOps.
//! Separate from offline CI; runs only when explicitly requested via:
//! `cargo test -p lgtm-core --test live_provider -- --ignored --nocapture`

use lgtm_core::forge::{self, ForgeKind};
use lgtm_core::pr::PullRequestState;
use std::path::PathBuf;

fn writes_allowed() -> bool {
    std::env::var("LGTM_LIVE_WRITES")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[test]
#[ignore = "opt-in live GitHub acceptance: set LGTM_LIVE_GITHUB_REPO and LGTM_LIVE_GITHUB_PR"]
fn test_live_github_provider_contracts() {
    let repo_str = match std::env::var("LGTM_LIVE_GITHUB_REPO") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("SKIPPED: LGTM_LIVE_GITHUB_REPO not set");
            return;
        }
    };
    let repo = PathBuf::from(repo_str);
    assert_eq!(
        forge::kind_of(&repo),
        ForgeKind::GitHub,
        "LGTM_LIVE_GITHUB_REPO must have a GitHub remote origin"
    );

    let pr = std::env::var("LGTM_LIVE_GITHUB_PR")
        .expect("LGTM_LIVE_GITHUB_PR must be set when running live GitHub tests")
        .parse::<u64>()
        .expect("PR must be a valid number");

    let provider = forge::open(&repo);
    assert!(
        provider.available(),
        "GitHub provider credentials or CLI must be available"
    );

    // 1. Verify PR listing across states
    for state in [
        PullRequestState::Open,
        PullRequestState::Closed,
        PullRequestState::All,
    ] {
        let list = provider
            .list_prs_state(state)
            .unwrap_or_else(|e| panic!("failed to list {state:?} PRs: {e}"));
        println!("GitHub live listing {state:?}: found {} PRs", list.len());
    }

    // 2. Verify PR diff retrieval
    let snapshot = provider
        .pr_diff(pr)
        .unwrap_or_else(|e| panic!("failed to load PR #{pr} diff: {e}"));
    println!(
        "GitHub live PR #{pr}: loaded snapshot with {} files",
        snapshot.files.len()
    );

    // 3. Verify checks retrieval
    let checks = provider
        .checks(pr)
        .unwrap_or_else(|e| panic!("failed to load checks for PR #{pr}: {e}"));
    println!("GitHub live PR #{pr}: found {} checks", checks.len());

    // 4. Verify threads retrieval
    let threads = provider
        .threads(pr)
        .unwrap_or_else(|e| panic!("failed to load threads for PR #{pr}: {e}"));
    println!(
        "GitHub live PR #{pr}: found {} review threads",
        threads.len()
    );

    // 5. Explicitly opted-in writes
    if writes_allowed() {
        if let Some(t) = threads.first() {
            println!(
                "GitHub live writes enabled: testing reply and resolve on thread {}",
                t.id
            );
            if let Some(comment) = t.comments.first() {
                let reply_id = provider
                    .reply(
                        pr,
                        &t.id,
                        comment.id,
                        "LGTM live acceptance probe (automated)",
                    )
                    .expect("live reply failed");
                println!("Live reply posted with ID {reply_id}");
            }
            // Toggle resolved then restore
            provider
                .set_resolved(pr, &t.id, !t.resolved)
                .expect("live resolve toggle failed");
            provider
                .set_resolved(pr, &t.id, t.resolved)
                .expect("live resolve cleanup failed");
            println!("Live resolve toggled and restored successfully");
        }
    } else {
        println!("Note: LGTM_LIVE_WRITES not set; remote write operations were skipped.");
    }
}

#[test]
#[ignore = "opt-in live Azure DevOps acceptance: set LGTM_LIVE_AZURE_REPO and LGTM_LIVE_AZURE_PR"]
fn test_live_azure_provider_contracts() {
    let repo_str = match std::env::var("LGTM_LIVE_AZURE_REPO") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("SKIPPED: LGTM_LIVE_AZURE_REPO not set");
            return;
        }
    };
    let repo = PathBuf::from(repo_str);
    assert_eq!(
        forge::kind_of(&repo),
        ForgeKind::Azure,
        "LGTM_LIVE_AZURE_REPO must have an Azure DevOps remote origin"
    );

    let pr = std::env::var("LGTM_LIVE_AZURE_PR")
        .expect("LGTM_LIVE_AZURE_PR must be set when running live Azure tests")
        .parse::<u64>()
        .expect("PR must be a valid number");

    let provider = forge::open(&repo);
    assert!(
        provider.available(),
        "Azure DevOps provider tools (az CLI or token) must be available"
    );

    // 1. Verify PR listing across states
    for state in [
        PullRequestState::Open,
        PullRequestState::Closed,
        PullRequestState::All,
    ] {
        let list = provider
            .list_prs_state(state)
            .unwrap_or_else(|e| panic!("failed to list Azure {state:?} PRs: {e}"));
        println!("Azure live listing {state:?}: found {} PRs", list.len());
    }

    // 2. Verify PR diff retrieval
    let snapshot = provider
        .pr_diff(pr)
        .unwrap_or_else(|e| panic!("failed to load Azure PR #{pr} diff: {e}"));
    println!(
        "Azure live PR #{pr}: loaded snapshot with {} files",
        snapshot.files.len()
    );

    // 3. Verify checks / branch policies
    let checks = provider
        .checks(pr)
        .unwrap_or_else(|e| panic!("failed to load checks for Azure PR #{pr}: {e}"));
    println!("Azure live PR #{pr}: found {} policy checks", checks.len());

    // 4. Verify threads retrieval
    let threads = provider
        .threads(pr)
        .unwrap_or_else(|e| panic!("failed to load threads for Azure PR #{pr}: {e}"));
    println!(
        "Azure live PR #{pr}: found {} discussion threads",
        threads.len()
    );

    // 5. Explicitly opted-in writes
    if writes_allowed() {
        if let Some(t) = threads.first() {
            println!(
                "Azure live writes enabled: testing reply and resolve on thread {}",
                t.id
            );
            if let Some(comment) = t.comments.first() {
                let reply_id = provider
                    .reply(
                        pr,
                        &t.id,
                        comment.id,
                        "LGTM live acceptance probe (automated)",
                    )
                    .expect("live reply failed");
                println!("Live reply posted with ID {reply_id}");
            }
            provider
                .set_resolved(pr, &t.id, !t.resolved)
                .expect("live resolve toggle failed");
            provider
                .set_resolved(pr, &t.id, t.resolved)
                .expect("live resolve cleanup failed");
            println!("Live resolve toggled and restored successfully");
        }
    } else {
        println!("Note: LGTM_LIVE_WRITES not set; remote write operations were skipped.");
    }
}
