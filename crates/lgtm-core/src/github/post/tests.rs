use super::*;
use crate::github::FakeClient;
use crate::model::{ChangeType, DiffHunk, DiffLine, LineType};

fn files() -> Vec<FileDiff> {
    vec![FileDiff {
        old_path: None,
        new_path: "a.rs".into(),
        change_type: ChangeType::Modified,
        additions: 1,
        deletions: 0,
        hunks: vec![DiffHunk {
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "x".into(),
                old_line_num: None,
                new_line_num: Some(5),
            }],
            ..Default::default()
        }],
        is_folded: false,
        is_reviewed: false,
    }]
}

fn c(path: &str, line: u32, pr: Option<u64>) -> CommentAnchor {
    CommentAnchor {
        file_path: path.into(),
        target_line_no: line,
        target_code: String::new(),
        context_before: vec![],
        context_after: vec![],
        comment_text: format!("t{line}"),
        replies: vec![],
        created_at_turn: 1,
        is_resolved: false,
        pr_number: pr,
        remote_id: None,
        track: Default::default(),
    }
}

fn set() -> Vec<CommentAnchor> {
    let mut posted = c("a.rs", 5, Some(7));
    posted.remote_id = Some(1);
    let mut resolved = c("a.rs", 5, Some(7));
    resolved.is_resolved = true;
    vec![
        c("a.rs", 5, Some(7)),  // inline
        c("a.rs", 90, Some(7)), // outside the diff
        c("b.rs", 1, Some(7)),  // file not in diff
        c("a.rs", 5, Some(8)),  // other PR
        c("a.rs", 5, None),     // local only
        posted,
        resolved,
    ]
}

#[test]
fn plan_splits_inline_general_and_skips_the_rest() {
    let p = plan(&set(), &files(), 7);
    assert_eq!(
        p,
        Plan {
            inline: vec![0],
            general: vec![1, 2]
        }
    );
    assert_eq!(p.total(), 3);
}

#[test]
fn posts_inline_then_one_general_and_saves_each_step() {
    let mut cs = set();
    let p = plan(&cs, &files(), 7);
    let fake = FakeClient::default();
    let mut saves = 0;
    let r = post(&fake, 7, &mut cs, &p, Duration::ZERO, |_| {
        saves += 1;
        Ok(())
    });
    assert_eq!(
        r,
        Report {
            posted: 3,
            total: 3,
            error: None
        }
    );
    assert_eq!(r.summary(), "3 of 3 posted");
    assert_eq!(saves, 2);
    let posted = fake.posted.borrow();
    assert_eq!(posted.len(), 2);
    assert_eq!(posted[0], "a.rs:5:t5");
    assert!(posted[1].starts_with("general:**a.rs:90**\nt90\n\n**b.rs:1**\nt1"));
    assert_eq!(cs[1].remote_id, cs[2].remote_id);
    assert!(plan(&cs, &files(), 7).total() == 0);
}

#[test]
fn failure_stops_keeps_progress_and_resumes() {
    let mut cs = vec![
        c("a.rs", 5, Some(7)),
        c("a.rs", 5, Some(7)),
        c("a.rs", 5, Some(7)),
    ];
    let p = plan(&cs, &files(), 7);
    let flaky = FakeClient {
        fail_post_after: Some(1),
        ..Default::default()
    };
    let r = post(&flaky, 7, &mut cs, &p, Duration::ZERO, |_| Ok(()));
    assert_eq!((r.posted, r.total), (1, 3));
    assert_eq!(r.summary(), "1 of 3 posted; stopped: rate limited");
    assert!(cs[0].remote_id.is_some() && cs[1].remote_id.is_none());
    let again = plan(&cs, &files(), 7);
    assert_eq!(again.total(), 2);
    let r = post(
        &FakeClient::default(),
        7,
        &mut cs,
        &again,
        Duration::ZERO,
        |_| Ok(()),
    );
    assert_eq!(r.posted, 2);
    assert!(cs.iter().all(|c| c.remote_id.is_some()));
}

#[test]
fn head_sha_and_save_errors_are_reported() {
    let mut cs = vec![c("a.rs", 5, Some(7))];
    let p = plan(&cs, &files(), 7);
    let bad = FakeClient {
        error: Some("no gh".into()),
        ..Default::default()
    };
    let r = post(&bad, 7, &mut cs, &p, Duration::ZERO, |_| Ok(()));
    assert_eq!((r.posted, r.error.as_deref()), (0, Some("no gh")));
    let r = post(
        &FakeClient::default(),
        7,
        &mut cs,
        &p,
        Duration::ZERO,
        |_| anyhow::bail!("disk full"),
    );
    assert_eq!((r.posted, r.error.as_deref()), (1, Some("disk full")));
}
