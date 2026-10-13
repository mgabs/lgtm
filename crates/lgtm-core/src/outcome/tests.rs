use super::*;
use crate::model::{ChangeType, DiffHunk, DiffLine, LineType};
use crate::review::hunk_hash_at;

fn file(path: &str, adds: &[&str]) -> FileDiff {
    FileDiff {
        old_path: None,
        new_path: path.into(),
        change_type: ChangeType::Modified,
        additions: 0,
        deletions: 0,
        hunks: vec![DiffHunk {
            lines: adds
                .iter()
                .enumerate()
                .map(|(i, s)| DiffLine {
                    line_type: LineType::Addition,
                    content: (*s).into(),
                    old_line_num: None,
                    new_line_num: Some(1 + i as u32),
                })
                .collect(),
            ..Default::default()
        }],
        is_folded: false,
        is_reviewed: false,
    }
}

fn directive(files: &[FileDiff], turn: u64) -> CommentAnchor {
    CommentAnchor {
        file_path: "a.rs".into(),
        target_line_no: 1,
        target_code: "x".into(),
        context_before: vec![],
        context_after: vec![],
        comment_text: "fix".into(),
        replies: vec![],
        created_at_turn: turn,
        is_resolved: false,
        pr_number: None,
        remote_id: None,
        orphaned: false,
        track: Tracking {
            hunk_hash: hunk_hash_at(files, "a.rs", 1),
            ..Default::default()
        },
    }
}

#[test]
fn classifies_each_outcome() {
    let before = [file("a.rs", &["x", "y"])];
    let c = directive(&before, 1);
    assert!(c.track.hunk_hash.is_some());
    assert_eq!(
        classify(&c, &[file("a.rs", &["x", "y"])]),
        Outcome::Untouched
    );
    assert_eq!(
        classify(&c, &[file("a.rs", &["x2", "y"])]),
        Outcome::Changed
    );
    assert_eq!(classify(&c, &[file("b.rs", &["x", "y"])]), Outcome::Gone);
    assert_eq!(classify(&c, &[]), Outcome::Gone);
    let mut no_base = c.clone();
    no_base.track.hunk_hash = None;
    assert_eq!(classify(&no_base, &before), Outcome::Orphaned);
}

#[test]
fn recheck_skips_current_turn_and_resolved_and_reports_change() {
    let f = [file("a.rs", &["x"])];
    let mut cs = vec![directive(&f, 1), directive(&f, 2), directive(&f, 1)];
    cs[2].is_resolved = true;
    assert!(recheck(&mut cs, &[file("a.rs", &["z"])], 2));
    assert_eq!(cs[0].track.outcome, Some(Outcome::Changed));
    assert_eq!((cs[1].track.outcome, cs[2].track.outcome), (None, None));
    assert!(!recheck(&mut cs, &[file("a.rs", &["z"])], 2));
}

#[test]
fn claims_never_change_outcome_and_verdict_flags_mismatch() {
    let f = [file("a.rs", &["x"])];
    let mut c = directive(&f, 1);
    assert_eq!(verdict(&c), "sent");
    claim(&mut c, "done", 2);
    assert_eq!(verdict(&c), "claimed");
    assert!(!c.is_resolved);
    c.track.outcome = Some(Outcome::Untouched);
    assert_eq!(verdict(&c), "claimed but not changed");
    c.track.outcome = Some(Outcome::Gone);
    assert_eq!(verdict(&c), "claimed, code changed");
    c.track.claim = None;
    for (o, v) in [
        (Outcome::Untouched, "untouched"),
        (Outcome::Changed, "changed"),
        (Outcome::Gone, "gone"),
        (Outcome::Orphaned, "orphaned"),
    ] {
        c.track.outcome = Some(o);
        assert_eq!(verdict(&c), v);
    }
}

#[test]
fn summary_counts_open_directives_only() {
    let f = [file("a.rs", &["x"])];
    let mk = |o: Option<Outcome>, claimed: bool, resolved: bool| {
        let mut c = directive(&f, 1);
        c.track.outcome = o;
        c.is_resolved = resolved;
        if claimed {
            claim(&mut c, "n", 2);
        }
        c
    };
    let cs = vec![
        mk(Some(Outcome::Untouched), true, false),
        mk(Some(Outcome::Untouched), false, false),
        mk(Some(Outcome::Changed), false, false),
        mk(Some(Outcome::Gone), false, false),
        mk(Some(Outcome::Orphaned), false, false),
        mk(Some(Outcome::Changed), false, true),
        mk(None, false, false),
    ];
    let s = Summary::of(&cs);
    assert_eq!(
        s,
        Summary {
            untouched: 2,
            changed: 1,
            gone: 1,
            orphaned: 1,
            claimed_unchanged: 1
        }
    );
    assert_eq!(
        s.line(),
        "2 untouched, 1 changed, 1 gone, 1 orphaned (1 claimed but not changed)"
    );
    assert_eq!(
        Summary::default().line(),
        "0 untouched, 0 changed, 0 gone, 0 orphaned"
    );
}

#[test]
fn old_comments_without_tracking_still_load() {
    let json = r#"{"file_path":"a","target_line_no":1,"target_code":"","context_before":[],"context_after":[],"comment_text":"t","created_at_turn":1,"is_resolved":false}"#;
    let c: CommentAnchor = serde_json::from_str(json).unwrap();
    assert_eq!(c.track, Tracking::default());
}
