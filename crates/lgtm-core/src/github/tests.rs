use super::*;

#[test]
fn parses_check_buckets_and_summarizes() {
    let c = parse_checks(
        r#"[{"name":"a","bucket":"pass"},{"name":"b","bucket":"fail"},{"name":"c","bucket":"pending"},{"name":"d","bucket":"skipping"},{"name":"e","bucket":"cancel"}]"#,
    )
    .unwrap();
    assert_eq!(c[3].bucket, Bucket::Skipped);
    assert_eq!(
        CiSummary::of(&c),
        CiSummary {
            passed: 1,
            failed: 2,
            pending: 1
        }
    );
    assert!(parse_checks("nope").is_err());
}

#[test]
fn parses_threads() {
    let t = parse_threads(
        r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
        {"id":"T1","isResolved":true,"path":"a.rs","line":7,"comments":{"nodes":[{"databaseId":42,"author":{"login":"bob"},"body":"hi"}]}},
        {"id":"T2","isResolved":false,"path":"b.rs","line":null,"comments":{"nodes":[]}}]}}}}}"#,
    )
    .unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].resolved());
    assert_eq!(
        (t[0].line, t[0].comments[0].author.as_str()),
        (Some(7), "bob")
    );
    assert_eq!(t[0].comments[0].id, 42);
    assert_eq!((t[1].line, t[1].comments.len()), (None, 0));
    assert!(parse_threads(r#"{"data":{"repository":null}}"#).is_err());
    assert!(parse_threads("x").is_err());
}

#[test]
fn explains_gh_errors() {
    assert!(
        explain("To get started with GitHub CLI, please run: gh auth login")
            .contains("gh auth login")
    );
    assert!(explain("HTTP 403: Forbidden").contains("403"));
    assert!(explain("not logged in").contains("gh auth login"));
    assert!(explain("boom").contains("boom"));
}

#[test]
fn fake_client_reports_errors() {
    let ok = FakeClient::default();
    assert!(ok.checks(1).unwrap().is_empty() && ok.threads(1).unwrap().is_empty());
    let bad = FakeClient {
        error: Some("no gh".into()),
        ..Default::default()
    };
    assert!(bad.checks(1).is_err() && bad.threads(1).is_err());
}

#[test]
fn thread_marker_reports_open_resolved_outdated_and_none() {
    use crate::model::{DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
    let t = |path: &str, line: u32, resolved: bool, outdated: bool| Thread {
        id: "1".into(),
        path: path.into(),
        line: Some(line),
        state: if resolved {
            ThreadState::Resolved(Resolution::Done)
        } else {
            ThreadState::Open
        },
        outdated,
        can_resolve: None,
        can_unresolve: None,
        comments: vec![],
    };
    let ts = [
        t("a.rs", 3, true, false),
        t("a.rs", 3, false, false),
        t("a.rs", 9, true, false),
        t("a.rs", 5, false, true),
        t("a.rs", 6, false, true),
        t("a.rs", 6, false, false),
    ];
    assert_eq!(thread_marker(&ts, "a.rs", 3), Some(ThreadMarkerKind::Open));
    assert_eq!(
        thread_marker(&ts, "a.rs", 9),
        Some(ThreadMarkerKind::Resolved)
    );
    assert_eq!(
        thread_marker(&ts, "a.rs", 5),
        Some(ThreadMarkerKind::Outdated)
    );
    assert_eq!(
        thread_marker(&ts, "a.rs", 6),
        Some(ThreadMarkerKind::Outdated)
    );
    assert_eq!(thread_marker(&ts, "a.rs", 4), None);
    assert_eq!(thread_marker(&ts, "b.rs", 3), None);

    let mut diff_file = FileDiff {
        old_path: None,
        new_path: "a.rs".into(),
        change_type: crate::model::ChangeType::Modified,
        additions: 0,
        deletions: 0,
        hunks: vec![],
        is_folded: false,
        is_reviewed: false,
    };
    diff_file.hunks.push(DiffHunk {
        header: String::new(),
        old_start: 1,
        old_lines: 1,
        new_start: 3,
        new_lines: 1,
        lines: vec![DiffLine {
            line_type: LineType::Context,
            content: String::new(),
            old_line_num: Some(1),
            new_line_num: Some(3),
        }],
        is_folded: false,
        folded_blocks: vec![],
    });
    let mut ts = vec![
        t("a.rs", 3, false, false),
        t("a.rs", 8, false, false),
        t("b.rs", 3, false, false),
    ];
    ts.push(Thread {
        line: None,
        ..t("a.rs", 0, false, false)
    });
    mark_outdated(&mut ts, &DiffSnapshot::new(vec![diff_file]));
    let flags: Vec<bool> = ts.iter().map(|t| t.outdated).collect();
    assert_eq!(flags, [false, true, true, false]);
}

#[test]
fn parse_threads_reads_is_outdated() {
    let j = r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
        {"id":"a","isOutdated":true,"path":"x","line":1,"comments":{"nodes":[]}},
        {"id":"b","path":"x","line":1,"comments":{"nodes":[]}}]}}}}}"#;
    let t = parse_threads(j).unwrap();
    assert!(t[0].outdated && !t[1].outdated);
}

#[test]
fn single_thread_preserves_outdated_state_and_reports_invalid_responses() {
    let thread = parse_thread(
        r#"{"data":{"node":{"id":"T1","path":"a.rs","line":null,"isResolved":false,"isOutdated":true,"comments":{"nodes":[{"databaseId":7,"body":"old code","author":null}]}}}}"#,
    )
    .unwrap()
    .unwrap();
    assert!(thread.outdated);
    assert_eq!(thread.comments[0].body, "old code");
    assert_eq!(thread.comments[0].author, "");
    assert_eq!(parse_thread(r#"{"data":{"node":null}}"#).unwrap(), None);
    assert!(parse_thread(r#"{"errors":[{"message":"denied"}]}"#).is_err());
    assert!(parse_thread(r#"{"data":{}}"#).is_err());
    assert!(parse_thread("invalid JSON").is_err());
    let defaults = parse_threads(
        r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{}]}}}}}"#,
    )
    .unwrap();
    assert_eq!(defaults[0].id, "");
    assert_eq!(defaults[0].line, None);
    assert!(defaults[0].comments.is_empty());
    assert!(!defaults[0].outdated);
}

#[test]
fn gh_client_process_contracts() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("fixture.rs");
    std::fs::write(
        &source,
        r#"
use std::{env, fs, io::{self, Read, Write}, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("LGTM_GH_FIXTURE").unwrap());
    fs::write(root.join("args"), env::args().skip(1).collect::<Vec<_>>().join("\n")).unwrap();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    fs::write(root.join("input"), input).unwrap();
    io::stdout().write_all(&fs::read(root.join("response")).unwrap()).unwrap();
    io::stderr().write_all(&fs::read(root.join("error")).unwrap()).unwrap();
    std::process::exit(fs::read_to_string(root.join("status")).unwrap().parse().unwrap());
}
"#,
    )
    .unwrap();
    let executable = dir.path().join(if cfg!(windows) { "gh.exe" } else { "gh" });
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let empty = tempfile::tempdir().unwrap();
    for (mode, path) in [("fixture", dir.path()), ("missing", empty.path())] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "github::tests::gh_client_fixture_child",
                "--nocapture",
            ])
            .env("LGTM_GH_FIXTURE", dir.path())
            .env("LGTM_GH_MODE", mode)
            .env("PATH", path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn gh_client_fixture_child() {
    let Some(root) = std::env::var_os("LGTM_GH_FIXTURE").map(std::path::PathBuf::from) else {
        return;
    };
    let client = GhClient;
    if std::env::var("LGTM_GH_MODE").unwrap() == "missing" {
        assert!(client
            .checks(42)
            .unwrap_err()
            .to_string()
            .contains("not installed"));
        assert!(client
            .post_general(42, "text")
            .unwrap_err()
            .to_string()
            .contains("not installed"));
        return;
    }
    let respond = |body: &str, error: &str, status: i32| {
        for (file, text) in [
            ("response", body.to_owned()),
            ("error", error.to_owned()),
            ("status", status.to_string()),
        ] {
            std::fs::write(root.join(file), text).unwrap();
        }
    };
    let args = || std::fs::read_to_string(root.join("args")).unwrap();
    let payload = || {
        serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(root.join("input")).unwrap(),
        )
        .unwrap()
    };
    respond(r#"[{"name":"CI","bucket":"fail"}]"#, "", 1);
    assert_eq!(client.checks(42).unwrap()[0].bucket, Bucket::Fail);
    assert!(args().contains("42\n--json\nname,bucket"));
    respond("", "", 0);
    assert!(client.checks(42).unwrap().is_empty());

    respond(
        r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"id":"T1","isOutdated":true}]}}}}}"#,
        "",
        0,
    );
    assert!(client.threads(42).unwrap()[0].outdated);
    assert!(args().contains("isOutdated"));
    assert!(args().contains("n=42"));
    respond(r#"{"data":{"node":{"id":"T1","isOutdated":true}}}"#, "", 0);
    assert!(client.thread(42, "T1").unwrap().unwrap().outdated);
    assert!(args().contains("id=T1"));
    assert!(
        args().contains("isOutdated"),
        "single-thread refresh must request outdated state"
    );
    respond(" sha42 \n", "", 0);
    assert_eq!(client.head_sha(42).unwrap(), "sha42");
    assert!(args().contains("headRefOid"));

    respond(r#"{"id":71}"#, "", 0);
    assert_eq!(
        client
            .post_inline(42, "sha", "a.rs", 3, CommentSide::Left, "inline")
            .unwrap(),
        71
    );
    assert!(args().contains("pulls/42/comments"));
    assert_eq!(
        payload(),
        serde_json::json!({
            "body":"inline", "commit_id":"sha", "path":"a.rs", "line":3, "side":"LEFT"
        })
    );
    assert_eq!(client.post_general(42, "general").unwrap(), 71);
    assert!(args().contains("issues/42/comments"));
    assert_eq!(payload()["body"], "general");
    assert_eq!(client.reply(42, "T1", 71, "reply").unwrap(), 71);
    assert!(args().contains("comments/71/replies"));
    assert_eq!(payload()["body"], "reply");
    for (state, operation) in [
        (
            ThreadState::Resolved(Resolution::Done),
            "resolveReviewThread",
        ),
        (ThreadState::Open, "unresolveReviewThread"),
    ] {
        client.set_state(42, "T1", state).unwrap();
        assert!(args().contains(operation));
        assert!(args().contains("id=T1"));
    }

    respond("", "HTTP 401", 1);
    for result in [
        client.checks(42).map(|_| ()),
        client.threads(42).map(|_| ()),
        client.thread(42, "T1").map(|_| ()),
        client.head_sha(42).map(|_| ()),
        client.post_general(42, "body").map(|_| ()),
        client
            .set_state(42, "T1", ThreadState::Resolved(Resolution::Done))
            .map(|_| ()),
    ] {
        assert!(result.unwrap_err().to_string().contains("gh auth login"));
    }
    for (error, expected) in [
        ("HTTP 403", "denied access"),
        ("HTTP 422", "line not part of diff"),
        ("could not be resolved", "invalid side"),
        ("network error", "gh failed"),
    ] {
        respond("", error, 1);
        assert!(client
            .post_general(42, "body")
            .unwrap_err()
            .to_string()
            .contains(expected));
    }
    respond("invalid", "", 0);
    assert!(client
        .post_general(42, "body")
        .unwrap_err()
        .to_string()
        .contains("unexpected comment response"));
    respond("{}", "", 0);
    assert!(client
        .post_general(42, "body")
        .unwrap_err()
        .to_string()
        .contains("no id"));
}

#[test]
fn fake_client_retry_state_and_failures() {
    let fake = FakeClient::default();
    assert_eq!(fake.head_sha(1).unwrap(), "sha1");
    assert_eq!(
        fake.post_inline(1, "sha1", "a.rs", 3, CommentSide::Right, "inline")
            .unwrap(),
        101
    );
    assert_eq!(fake.post_general(1, "general").unwrap(), 102);
    assert_eq!(fake.reply(1, "T1", 7, "reply").unwrap(), 103);
    fake.set_state(1, "T1", ThreadState::Open).unwrap();
    assert_eq!(
        *fake.posted.borrow(),
        [
            "a.rs:3:inline",
            "general:general",
            "reply:7:reply",
            "state:T1:Open"
        ]
    );
    let failing = FakeClient {
        error: Some("offline".into()),
        fail_post_after: Some(0),
        ..Default::default()
    };
    assert!(failing.head_sha(1).is_err());
    assert!(failing.post_general(1, "body").is_err());
    assert!(failing.posted.borrow().is_empty());
    let timeout = FakeClient {
        timeout_after_accept: true,
        ..Default::default()
    };
    let error = timeout.post_general(1, "body").unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<crate::forge::failure::Failure>()
            .unwrap()
            .kind,
        crate::forge::failure::Kind::Timeout
    );
    assert_eq!(*timeout.posted.borrow(), ["general:body"]);
}

#[test]
fn reconciliation_keeps_provider_outdated_state_when_the_line_is_present() {
    use crate::model::{ChangeType, DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
    let diff = DiffSnapshot::new(vec![FileDiff {
        old_path: None,
        new_path: "a.rs".into(),
        change_type: ChangeType::Modified,
        additions: 1,
        deletions: 0,
        hunks: vec![DiffHunk {
            header: "@@ -0,0 +1 @@".into(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 1,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "new code".into(),
                old_line_num: None,
                new_line_num: Some(1),
            }],
            is_folded: false,
            folded_blocks: vec![],
        }],
        is_folded: false,
        is_reviewed: false,
    }]);
    let mut threads = vec![Thread {
        id: "T1".into(),
        path: "a.rs".into(),
        line: Some(1),
        state: ThreadState::Open,
        outdated: true,
        can_resolve: None,
        can_unresolve: None,
        comments: vec![],
    }];
    mark_outdated(&mut threads, &diff);
    assert!(threads[0].outdated);
}
