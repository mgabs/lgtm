use super::*;
use crate::model::{ChangeType, DiffLine};

fn line(t: LineType, s: &str, n: Option<u32>) -> DiffLine {
    DiffLine {
        line_type: t,
        content: s.into(),
        old_line_num: None,
        new_line_num: n,
    }
}

fn hunk(start: u32, adds: &[&str]) -> DiffHunk {
    DiffHunk {
        new_start: start,
        lines: adds
            .iter()
            .enumerate()
            .map(|(i, s)| line(LineType::Addition, s, Some(start + i as u32)))
            .collect(),
        ..Default::default()
    }
}

fn file(path: &str, hunks: Vec<DiffHunk>) -> FileDiff {
    FileDiff {
        old_path: None,
        new_path: path.into(),
        change_type: ChangeType::Modified,
        additions: 0,
        deletions: 0,
        hunks,
        is_folded: false,
        is_reviewed: false,
    }
}

fn anchor(path: &str, ln: u32, code: &str) -> CommentAnchor {
    CommentAnchor {
        file_path: path.into(),
        target_line_no: ln,
        target_code: code.into(),
        context_before: vec![],
        context_after: vec![],
        comment_text: "x".into(),
        replies: vec![],
        created_at_turn: 1,
        is_resolved: false,
        pr_number: None,
    }
}

#[test]
fn hash_ignores_whitespace_and_line_numbers() {
    assert_eq!(
        hunk_hash(&hunk(1, &["a  b", "c"])),
        hunk_hash(&hunk(50, &["a b", " c "]))
    );
    assert_ne!(hunk_hash(&hunk(1, &["a"])), hunk_hash(&hunk(1, &["b"])));
}

#[test]
fn hash_distinguishes_sign() {
    let mut d = hunk(1, &["a"]);
    d.lines[0].line_type = LineType::Deletion;
    assert_ne!(hunk_hash(&d), hunk_hash(&hunk(1, &["a"])));
}

#[test]
fn mark_survives_insert_above() {
    let f = file("a.rs", vec![hunk(10, &["x"])]);
    let mut marks = vec![];
    assert!(toggle_hunk(&mut marks, &f, 0, "t"));
    let moved = file("a.rs", vec![hunk(40, &["x"])]);
    assert!(file_reviewed(&marks, &moved));
}

#[test]
fn changed_hunk_is_not_reviewed() {
    let mut marks = vec![];
    toggle_hunk(&mut marks, &file("a.rs", vec![hunk(1, &["x"])]), 0, "t");
    assert!(!file_reviewed(&marks, &file("a.rs", vec![hunk(1, &["y"])])));
    assert!(!file_reviewed(&marks, &file("a.rs", vec![])));
}

#[test]
fn duplicate_hunks_pair_by_nearest_line() {
    let f = file("a.rs", vec![hunk(1, &["x"]), hunk(100, &["x"])]);
    let mut marks = vec![];
    toggle_hunk(&mut marks, &f, 1, "t");
    assert_eq!(reviewed_flags(&marks, &f), vec![false, true]);
    assert!(!toggle_hunk(&mut marks, &f, 1, "t"));
    assert!(marks.is_empty());
}

#[test]
fn comment_follows_hunk_when_moved() {
    let f = file("a.rs", vec![hunk(10, &["p", "q"])]);
    let mut c = vec![ReviewComment::new(anchor("a.rs", 11, "q"), &[f])];
    assert!(!c[0].content_hash.is_empty());
    reanchor(&mut c, &[file("a.rs", vec![hunk(60, &["p", "q"])])]);
    assert_eq!(c[0].anchor.target_line_no, 61);
    assert_eq!(c[0].status, Status::Active);
}

#[test]
fn comment_follows_file_rename() {
    let f = file("a.rs", vec![hunk(1, &["p"])]);
    let mut c = vec![ReviewComment::new(anchor("a.rs", 1, "p"), &[f])];
    reanchor(&mut c, &[file("b.rs", vec![hunk(1, &["p"])])]);
    assert_eq!(c[0].anchor.file_path, "b.rs");
}

#[test]
fn edited_hunk_falls_back_to_fuzzy() {
    let f = file("a.rs", vec![hunk(1, &["p", "q"])]);
    let mut c = vec![ReviewComment::new(anchor("a.rs", 2, "q"), &[f])];
    reanchor(&mut c, &[file("a.rs", vec![hunk(5, &["new", "q"])])]);
    assert_eq!(c[0].anchor.target_line_no, 6);
    assert_eq!(c[0].status, Status::Active);
}

#[test]
fn fuzzy_respects_context_and_picks_nearest() {
    let mut a = anchor("a.rs", 3, "q");
    a.context_before = vec!["p".into()];
    a.context_after = vec!["r".into()];
    let mut c = vec![ReviewComment::new(a, &[])];
    // "q" without the right neighbours must not match
    reanchor(&mut c, &[file("a.rs", vec![hunk(1, &["z", "q", "z"])])]);
    assert_eq!(c[0].status, Status::Orphaned);
    // two matches: nearest to line 3 wins
    let files = [file(
        "a.rs",
        vec![hunk(1, &["p", "q", "r"]), hunk(90, &["p", "q", "r"])],
    )];
    reanchor(&mut c, &files);
    assert_eq!(c[0].anchor.target_line_no, 2);
    assert_eq!(c[0].status, Status::Active);
}

#[test]
fn gone_code_is_orphaned_not_deleted_and_recovers() {
    let mut c = vec![ReviewComment::new(anchor("a.rs", 1, "p"), &[])];
    reanchor(&mut c, &[file("a.rs", vec![hunk(1, &["zzz"])])]);
    assert_eq!((c.len(), c[0].status), (1, Status::Orphaned));
    reanchor(&mut c, &[file("a.rs", vec![hunk(7, &["p"])])]);
    assert_eq!(
        (c[0].anchor.target_line_no, c[0].status),
        (7, Status::Active)
    );
}

#[test]
fn empty_code_is_orphaned_and_resolved_is_kept() {
    let mut c = vec![ReviewComment::new(anchor("a.rs", 1, "  "), &[])];
    reanchor(&mut c, &[]);
    assert_eq!(c[0].status, Status::Orphaned);
    let mut a = anchor("a.rs", 1, "p");
    a.is_resolved = true;
    let mut c = vec![ReviewComment::new(a, &[])];
    assert_eq!(c[0].status, Status::Resolved);
    reanchor(&mut c, &[file("a.rs", vec![hunk(3, &["p"])])]);
    assert_eq!(c[0].status, Status::Resolved);
}

#[test]
fn store_round_trip_and_newer_version_refused() {
    let d = tempfile::tempdir().unwrap();
    let s = ReviewStore::open(d.path(), "feat/x").unwrap();
    assert!(s.load().unwrap().comments.is_empty());
    let mut st = s.load().unwrap();
    st.hunks.push(HunkMark {
        file: "a".into(),
        content_hash: "h".into(),
        last_line: 1,
        reviewed_at: "t".into(),
    });
    s.save(&st).unwrap();
    assert_eq!(s.load().unwrap().hunks, st.hunks);
    let p = d.path().join(".lgtm/reviews/feat_x.json");
    std::fs::write(&p, r#"{"version":99,"target":"feat/x"}"#).unwrap();
    assert!(s.load().unwrap_err().to_string().contains("newer"));
    std::fs::write(&p, "nope").unwrap();
    assert!(s.load().is_err());
}

#[test]
fn migrates_legacy_comments_once() {
    let d = tempfile::tempdir().unwrap();
    let legacy = CommentStore::new(d.path());
    let mut pr = anchor("b.rs", 2, "q");
    pr.pr_number = Some(7);
    legacy.add_comment(&anchor("a.rs", 1, "p")).unwrap();
    legacy.add_comment(&pr).unwrap();
    let s = ReviewStore::open(d.path(), "main").unwrap();
    assert_eq!(s.load().unwrap().comments.len(), 1);
    assert!(d.path().join(".lgtm/comments.jsonl.bak").is_file());
    assert!(!d.path().join(".lgtm/comments.jsonl").exists());
    assert_eq!(
        ReviewStore::open(d.path(), "pr-7")
            .unwrap()
            .load()
            .unwrap()
            .comments
            .len(),
        1
    );
}

#[test]
fn open_without_legacy_is_empty() {
    let d = tempfile::tempdir().unwrap();
    let s = ReviewStore::open(d.path(), "main").unwrap();
    assert!(s.load().unwrap().hunks.is_empty());
}
