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
    assert!(t[0].resolved);
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
