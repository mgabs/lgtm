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
        id: crate::comment::new_id(),
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
        side: Default::default(),
        remote_id: None,
        orphaned: false,
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
    assert_eq!(p.inline, vec![0]);
    assert_eq!(p.general, vec![1, 2]);
    assert_eq!(p.skipped.len(), 4);
    assert_eq!(p.total(), 3);
}

#[test]
fn associate_directives_binds_unattached_comments_to_pr() {
    let mut comments = vec![
        c("a.rs", 5, None),
        c("other.rs", 10, None),
        c("a.rs", 5, Some(99)),
    ];
    let count = associate_directives(&mut comments, &files(), 7);
    assert_eq!(count, 1);
    assert_eq!(comments[0].pr_number, Some(7));
    assert_eq!(comments[1].pr_number, None);
    assert_eq!(comments[2].pr_number, Some(99));
}

#[test]
fn in_diff_checks_side_and_deletion() {
    use crate::model::LineType;
    use std::path::PathBuf;
    let files = vec![FileDiff {
        old_path: Some(PathBuf::from("del.rs")),
        new_path: PathBuf::from("del.rs"),
        change_type: ChangeType::Modified,
        additions: 0,
        deletions: 1,
        hunks: vec![DiffHunk {
            header: "@@ -10,1 +10,0 @@".into(),
            old_start: 10,
            old_lines: 1,
            new_start: 10,
            new_lines: 0,
            lines: vec![DiffLine {
                line_type: LineType::Deletion,
                content: "deleted line".into(),
                old_line_num: Some(10),
                new_line_num: None,
            }],
            is_folded: false,
            folded_blocks: Vec::new(),
        }],
        is_folded: false,
        is_reviewed: false,
    }];

    let mut left_comment = c("del.rs", 10, Some(7));
    left_comment.side = CommentSide::Left;
    assert!(in_diff(&files, &left_comment));

    let mut right_comment = c("del.rs", 10, Some(7));
    right_comment.side = CommentSide::Right;
    assert!(!in_diff(&files, &right_comment));
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

#[test]
fn general_only_batch_does_not_need_head_sha() {
    let mut comments = vec![c("outside.rs", 10, Some(7))];
    let plan = plan(&comments, &files(), 7);
    // Read operations fail; general posting is still available.
    let client = FakeClient {
        error: Some("head lookup must not run".into()),
        ..Default::default()
    };
    let report = post(&client, 7, &mut comments, &plan, Duration::ZERO, |_| Ok(()));
    assert_eq!(report.posted, 1);
    assert_eq!(report.error, None);
    assert!(comments[0].remote_id.is_some());
}
