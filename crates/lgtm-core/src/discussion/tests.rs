use super::*;
use crate::github::ThreadComment;
use anyhow::{bail, Result};
use std::cell::RefCell;

fn thread(id: &str, path: &str, line: u32, resolved: bool, bodies: &[&str]) -> Thread {
    Thread {
        id: id.into(),
        path: path.into(),
        line: Some(line),
        resolved,
        comments: bodies
            .iter()
            .enumerate()
            .map(|(i, b)| ThreadComment {
                id: i as u64 + 1,
                author: "a".into(),
                body: (*b).into(),
            })
            .collect(),
    }
}

fn inline(path: &str, line: u32) -> DraftKind {
    DraftKind::Thread {
        path: path.into(),
        line,
        side: CommentSide::Right,
    }
}

#[derive(Default)]
struct Fake {
    threads: RefCell<Vec<Thread>>,
    calls: RefCell<Vec<String>>,
    fail: RefCell<Vec<(String, String)>>,
    /// Calls whose write lands on the forge but whose reply is lost.
    land_then_timeout: RefCell<Vec<String>>,
    sha: Option<String>,
    head_calls: std::cell::Cell<u32>,
}

impl Fake {
    fn call(&self, key: String, apply: impl FnOnce(&mut Vec<Thread>)) -> Result<u64> {
        self.calls.borrow_mut().push(key.clone());
        if let Some((_, msg)) = self.fail.borrow().iter().find(|(k, _)| *k == key) {
            bail!("{msg}");
        }
        apply(&mut self.threads.borrow_mut());
        if self.land_then_timeout.borrow().contains(&key) {
            bail!("request timed out");
        }
        Ok(1)
    }
}

impl PrDiscussion for Fake {
    fn threads(&self, _pr: u64) -> Result<Vec<Thread>> {
        Ok(self.threads.borrow().clone())
    }
    fn head_sha(&self, _pr: u64) -> Result<String> {
        self.head_calls.set(self.head_calls.get() + 1);
        self.sha.clone().ok_or_else(|| anyhow::anyhow!("no head"))
    }
    fn post_inline(
        &self,
        _pr: u64,
        _sha: &str,
        path: &str,
        line: u32,
        _side: CommentSide,
        body: &str,
    ) -> Result<u64> {
        self.call(format!("inline:{path}:{line}"), |t| {
            t.push(thread("new", path, line, false, &[body]))
        })
    }
    fn post_general(&self, _pr: u64, _body: &str) -> Result<u64> {
        self.call("general".into(), |_| {})
    }
    fn reply(&self, _pr: u64, thread_id: &str, _c: u64, body: &str) -> Result<u64> {
        self.call(format!("reply:{thread_id}"), |t| {
            if let Some(x) = t.iter_mut().find(|x| x.id == thread_id) {
                x.comments.push(ThreadComment {
                    id: 9,
                    author: "me".into(),
                    body: body.into(),
                });
            }
        })
    }
    fn set_resolved(&self, _pr: u64, thread_id: &str, resolved: bool) -> Result<()> {
        self.call(format!("resolve:{thread_id}"), |t| {
            if let Some(x) = t.iter_mut().find(|x| x.id == thread_id) {
                x.resolved = resolved;
            }
        })
        .map(|_| ())
    }
}

fn reply_kind(id: &str) -> DraftKind {
    DraftKind::Reply {
        thread_id: id.into(),
        comment_id: 1,
    }
}

fn resolve_kind(id: &str, resolved: bool) -> DraftKind {
    DraftKind::SetResolved {
        thread_id: id.into(),
        resolved,
    }
}

#[test]
fn add_validates_and_trims() {
    let mut s = DraftSet::default();
    assert_eq!(s.add(inline("a", 1), "  \n "), Err(DraftError::EmptyBody));
    assert_eq!(s.add(DraftKind::General, ""), Err(DraftError::EmptyBody));
    assert_eq!(s.add(reply_kind("t"), " "), Err(DraftError::EmptyBody));
    assert_eq!(s.add(inline("a", 0), "x"), Err(DraftError::InvalidLine));
    assert!(s.is_empty());
    let id = s.add(inline("a", 1), "  hi  ").unwrap();
    assert_eq!(s.get(id).unwrap().body, "hi");
    assert_eq!(s.add(inline("a", 1), "again").unwrap(), id + 1);
}

#[test]
fn last_resolve_draft_per_thread_wins() {
    let mut s = DraftSet::default();
    s.add(resolve_kind("t1", true), "").unwrap();
    s.add(resolve_kind("t2", true), "").unwrap();
    s.add(resolve_kind("t1", false), "").unwrap();
    assert_eq!(s.items().len(), 2);
    assert_eq!(s.items()[1].kind, resolve_kind("t1", false));
    assert_eq!(s.summary(), "1 resolve, 1 unresolve");
}

#[test]
fn edit_clears_state_and_rejects_bad_input() {
    let mut s = DraftSet::default();
    let a = s.add(reply_kind("t"), "x").unwrap();
    let r = s.add(resolve_kind("t", true), "").unwrap();
    s.mark_stale(&[]);
    assert!(s.get(a).unwrap().stale.is_some());
    assert_eq!(s.edit(a, " "), Err(DraftError::EmptyBody));
    assert_eq!(s.edit(r, "x"), Err(DraftError::NoBody));
    s.edit(a, "y").unwrap();
    let d = s.get(a).unwrap();
    assert_eq!((d.body.as_str(), d.stale.clone()), ("y", None));
    assert!(s.edit(999, "z").is_ok());
}

#[test]
fn remove_clear_and_summary() {
    let mut s = DraftSet::default();
    assert_eq!(s.summary(), "no drafts");
    let a = s.add(inline("a", 1), "x").unwrap();
    s.add(DraftKind::General, "g").unwrap();
    assert_eq!(s.summary(), "1 thread, 1 general");
    assert!(s.remove(a));
    assert!(!s.remove(a));
    s.clear();
    assert!(s.is_empty());
}

#[test]
fn plan_orders_kinds_and_skips_stale() {
    let mut s = DraftSet::default();
    let res = s.add(resolve_kind("t", true), "").unwrap();
    let rep = s.add(reply_kind("t"), "r").unwrap();
    let gen = s.add(DraftKind::General, "g").unwrap();
    let thr = s.add(inline("a", 2), "n").unwrap();
    let gone = s.add(reply_kind("gone"), "r").unwrap();
    s.mark_stale(&[thread("t", "a", 1, false, &["c"])]);
    assert_eq!(s.plan(), vec![thr, gen, rep, res]);
    assert_eq!(
        s.get(gone).unwrap().stale.as_deref(),
        Some("thread not found")
    );
    s.mark_stale(&[
        thread("t", "a", 1, false, &[]),
        thread("gone", "a", 1, false, &[]),
    ]);
    assert!(s.get(gone).unwrap().stale.is_none());
    assert_eq!(s.inline_lines().collect::<Vec<_>>(), vec![("a", 2)]);
}

#[test]
fn submit_sends_in_order_and_empties_the_set() {
    let fake = Fake {
        sha: Some("s".into()),
        threads: RefCell::new(vec![thread("t", "a", 1, false, &["c"])]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(resolve_kind("t", true), "").unwrap();
    s.add(reply_kind("t"), "r").unwrap();
    s.add(DraftKind::General, "g").unwrap();
    s.add(inline("b", 2), "n").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!(report.sent, 4);
    assert!(s.is_empty());
    assert_eq!(
        *fake.calls.borrow(),
        vec!["inline:b:2", "general", "reply:t", "resolve:t"]
    );
    assert_eq!(report.message(), "Submitted 4");
}

#[test]
fn partial_failure_keeps_only_failed_items_and_retry_sends_the_rest() {
    let fake = Fake {
        sha: Some("s".into()),
        threads: RefCell::new(vec![thread("t", "a", 1, false, &["c"])]),
        fail: RefCell::new(vec![("inline:b:2".into(), "HTTP 422 unprocessable".into())]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(inline("b", 2), "bad").unwrap();
    s.add(reply_kind("t"), "ok").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((report.sent, report.failed.len()), (1, 1));
    assert_eq!(s.items().len(), 1);
    let err = s.items()[0].error.clone().unwrap();
    assert!(err.starts_with("thread b:2:") && err.contains("no longer in the PR diff"));
    assert!(report.message().contains("1 failed: thread b:2"));
    fake.fail.borrow_mut().clear();
    let again = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((again.sent, again.failed.len()), (1, 0));
    assert_eq!(
        fake.calls
            .borrow()
            .iter()
            .filter(|c| *c == "reply:t")
            .count(),
        1
    );
}

#[test]
fn ambiguous_write_that_landed_is_never_duplicated() {
    let fake = Fake {
        sha: Some("s".into()),
        land_then_timeout: RefCell::new(vec!["inline:a:1".into()]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(inline("a", 1), "once").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((report.sent, report.deduped), (0, 1));
    assert!(s.is_empty());
    let again = submit(&fake, 1, &mut s).unwrap();
    assert_eq!(again, SubmitReport::default());
    assert_eq!(fake.calls.borrow().len(), 1);
}

#[test]
fn ambiguous_write_that_did_not_land_stays_for_retry() {
    let fake = Fake {
        sha: Some("s".into()),
        threads: RefCell::new(vec![thread("t", "a", 1, false, &["c"])]),
        fail: RefCell::new(vec![("reply:t".into(), "request timed out".into())]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(reply_kind("t"), "hello").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert!(s.items()[0]
        .error
        .as_deref()
        .unwrap()
        .contains("connectivity"));
}

#[test]
fn already_present_items_are_deduped_before_sending() {
    let fake = Fake {
        sha: Some("s".into()),
        threads: RefCell::new(vec![
            thread("t", "a", 1, true, &["c", "reply me"]),
            thread("u", "a", 5, false, &["note"]),
        ]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(reply_kind("t"), "reply me").unwrap();
    s.add(resolve_kind("t", true), "").unwrap();
    s.add(inline("a", 5), "note").unwrap();
    s.add(inline("a", 6), "note").unwrap();
    s.add(DraftKind::General, "g").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((report.sent, report.deduped), (2, 3));
    assert_eq!(*fake.calls.borrow(), vec!["inline:a:6", "general"]);
}

#[test]
fn unresolve_of_resolved_thread_sends_and_stale_is_reported() {
    let fake = Fake {
        sha: Some("s".into()),
        threads: RefCell::new(vec![thread("t", "a", 1, true, &["c"])]),
        ..Default::default()
    };
    let mut s = DraftSet::default();
    s.add(resolve_kind("t", false), "").unwrap();
    s.add(reply_kind("gone"), "r").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((report.sent, report.stale), (1, 1));
    assert!(!fake.threads.borrow()[0].resolved);
    assert_eq!(s.items().len(), 1);
    assert_eq!(report.message(), "Submitted 1, 1 stale");
}

#[test]
fn missing_head_fails_inline_items_without_losing_them() {
    let fake = Fake::default();
    let mut s = DraftSet::default();
    s.add(inline("a", 1), "x").unwrap();
    s.add(DraftKind::General, "g").unwrap();
    let report = submit(&fake, 1, &mut s).unwrap();
    assert_eq!((report.sent, report.failed.len()), (1, 1));
    assert_eq!(s.items()[0].body, "x");
}

#[test]
fn listing_failure_aborts_without_touching_drafts() {
    struct Down;
    impl PrDiscussion for Down {}
    let mut s = DraftSet::default();
    s.add(DraftKind::General, "g").unwrap();
    assert!(submit(&Down, 1, &mut s).is_err());
    assert_eq!(s.items().len(), 1);
}

#[test]
fn remediation_covers_every_failure_class() {
    let f = |kind, http_status| {
        anyhow::Error::new(Failure {
            kind,
            exit_code: None,
            http_status,
        })
    };
    let cases = [
        (f(Kind::Authentication, None), "log in"),
        (f(Kind::AccessDenied, None), "permission"),
        (f(Kind::Timeout, None), "connectivity"),
        (f(Kind::Network, None), "connectivity"),
        (f(Kind::Http, Some(422)), "PR diff"),
        (f(Kind::Http, Some(500)), "fix the cause"),
        (anyhow::anyhow!("API rate limit exceeded"), "rate limit"),
        (anyhow::anyhow!("HTTP 404 Not Found"), "thread is gone"),
        (anyhow::anyhow!("HTTP 404"), "thread is gone"),
        (anyhow::anyhow!("thread not found"), "thread is gone"),
        (
            anyhow::anyhow!("not supported by this provider"),
            "does not support",
        ),
        (anyhow::anyhow!("Unprocessable Entity"), "PR diff"),
        (anyhow::anyhow!("HTTP 422"), "PR diff"),
        (anyhow::anyhow!("request timed out"), "connectivity"),
        (anyhow::anyhow!("gateway timeout"), "connectivity"),
        (anyhow::anyhow!("boom"), "fix the cause"),
    ];
    for (err, want) in cases {
        assert!(remediation(&err).contains(want), "{err:#} -> {want}");
    }
}

#[test]
fn store_round_trips_and_scopes_by_pr_and_repo() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = DraftSet::default();
    s.add(inline("a", 1), "x").unwrap();
    s.add(resolve_kind("t", true), "").unwrap();
    store::save(dir.path(), 7, "o/r", &s).unwrap();
    assert_eq!(store::load(dir.path(), 7, "o/r"), s);
    assert!(store::load(dir.path(), 8, "o/r").is_empty());
    assert!(store::load(dir.path(), 7, "o/other").is_empty());
    assert!(!dir.path().join(".lgtm/drafts/pr-7.json.tmp").exists());
    store::save(dir.path(), 7, "o/r", &DraftSet::default()).unwrap();
    assert!(!dir.path().join(".lgtm/drafts/pr-7.json").exists());
    store::save(dir.path(), 7, "o/r", &DraftSet::default()).unwrap();
}

#[test]
fn store_ignores_corrupt_and_foreign_versions() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join(".lgtm/drafts");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("pr-1.json"), "{nope").unwrap();
    assert!(store::load(dir.path(), 1, "r").is_empty());
    std::fs::write(
        p.join("pr-2.json"),
        r#"{"version":9,"repo_key":"r","drafts":{"next_id":1,"items":[]}}"#,
    )
    .unwrap();
    assert!(store::load(dir.path(), 2, "r").is_empty());
}

#[test]
fn store_save_reports_unwritable_location() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".lgtm"), "file").unwrap();
    let mut s = DraftSet::default();
    s.add(DraftKind::General, "g").unwrap();
    assert!(store::save(dir.path(), 1, "r", &s).is_err());
    let dirpath = dir.path().join("x");
    std::fs::create_dir_all(dirpath.join(".lgtm/drafts/pr-1.json/inner")).unwrap();
    assert!(store::save(&dirpath, 1, "r", &DraftSet::default()).is_err());
}

#[test]
fn head_is_only_fetched_for_sendable_inline_drafts() {
    let fake = Fake::default();
    let mut s = DraftSet::default();
    s.add(DraftKind::General, "g").unwrap();
    s.add(reply_kind("gone"), "r").unwrap();
    submit(&fake, 1, &mut s).unwrap();
    assert_eq!(*fake.calls.borrow(), vec!["general"]);
    assert_eq!(fake.head_calls.get(), 0);
}

#[test]
fn draft_errors_have_readable_messages() {
    assert_eq!(DraftError::EmptyBody.to_string(), "Comment body is empty");
    assert!(DraftError::InvalidLine.to_string().contains("diff hunk"));
    assert!(DraftError::NoBody.to_string().contains("no text"));
}
