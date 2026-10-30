use super::*;
use crate::comment::CommentStore;
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
        id: crate::comment::new_id(),
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
        orphaned: false,
        track: Default::default(),
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
fn hash_invariant_to_crlf_endings() {
    let lf = hunk(1, &["let x = 10;", "let y = 20;"]);
    let crlf = hunk(1, &["let x = 10;\r", "let y = 20;\r"]);
    assert_eq!(hunk_hash(&lf), hunk_hash(&crlf));
    assert_eq!(hunk_hash_u64(&lf), hunk_hash_u64(&crlf));
}

#[test]
fn hash_distinguishes_sign() {
    let mut d = hunk(1, &["a"]);
    d.lines[0].line_type = LineType::Deletion;
    assert_ne!(hunk_hash(&d), hunk_hash(&hunk(1, &["a"])));
}

#[test]
fn hash_streaming_matches_reference() {
    fn reference_hunk_hash(h: &DiffHunk) -> String {
        let body: Vec<String> = h
            .lines
            .iter()
            .filter_map(|l| {
                let sign = match l.line_type {
                    LineType::Addition => '+',
                    LineType::Deletion => '-',
                    _ => return None,
                };
                let text = l.content.split_whitespace().collect::<Vec<_>>().join(" ");
                Some(format!("{sign}{text}"))
            })
            .collect();
        format!("{:016x}", crate::guard::fnv1a(&body.join("\n")))
    }

    let cases = vec![
        DiffHunk::default(),
        DiffHunk {
            lines: vec![
                line(LineType::Context, "ctx line 1", None),
                line(LineType::HunkHeader, "@@ -1,3 +1,4 @@", None),
            ],
            ..Default::default()
        },
        DiffHunk {
            lines: vec![
                line(LineType::Context, "before", None),
                line(LineType::Addition, "  let   x = 1;  ", Some(1)),
                line(LineType::Deletion, "let x = 0;", Some(1)),
                line(LineType::Addition, "   ", Some(2)),
                line(LineType::Addition, "", Some(3)),
                line(LineType::Context, "after", None),
            ],
            ..Default::default()
        },
        DiffHunk {
            lines: vec![
                line(LineType::Addition, "multi line\nwith\ttabs", Some(1)),
                line(LineType::Deletion, "old\r\nvalue", Some(2)),
            ],
            ..Default::default()
        },
    ];

    for c in cases {
        assert_eq!(hunk_hash(&c), reference_hunk_hash(&c));
        assert_eq!(format_hunk_hash(hunk_hash_u64(&c)), hunk_hash(&c));
        assert_eq!(format_hunk_hash(c.hash_u64()), hunk_hash(&c));
    }
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
fn open_preserves_comments_jsonl() {
    let d = tempfile::tempdir().unwrap();
    let store = CommentStore::new(d.path());
    store.add_comment(&anchor("a.rs", 1, "p")).unwrap();
    let s = ReviewStore::open(d.path(), "main").unwrap();
    assert!(s.load().unwrap().comments.is_empty());
    assert_eq!(store.load_comments().len(), 1);
    assert!(d.path().join(".lgtm/directives.jsonl").is_file());
    assert!(!d.path().join(".lgtm/directives.jsonl.bak").exists());
}

#[test]
fn open_without_legacy_is_empty() {
    let d = tempfile::tempdir().unwrap();
    let s = ReviewStore::open(d.path(), "main").unwrap();
    assert!(s.load().unwrap().hunks.is_empty());
}

#[test]
fn set_file_marks_and_unmarks_all_hunks() {
    let f = file("a.rs", vec![hunk(1, &["x"]), hunk(9, &["y"])]);
    let mut marks = vec![];
    toggle_hunk(&mut marks, &f, 0, "t");
    set_file(&mut marks, &f, true, "t");
    assert!(file_reviewed(&marks, &f));
    assert_eq!(marks.len(), 2);
    set_file(&mut marks, &f, false, "t");
    assert!(marks.is_empty());
}

#[test]
fn merge_dedups_and_reports_conflicts() {
    let mk = |res: bool| {
        let mut a = anchor("a.rs", 1, "p");
        a.is_resolved = res;
        ReviewComment::new(a, &[])
    };
    let mark = HunkMark {
        file: "a".into(),
        content_hash: "h".into(),
        last_line: 1,
        reviewed_at: "t".into(),
    };
    let mut base = ReviewState {
        hunks: vec![mark.clone()],
        comments: vec![mk(false)],
        ..Default::default()
    };
    let mut other_c = ReviewComment::new(anchor("b.rs", 2, "q"), &[]);
    other_c.anchor.comment_text = "other".into();
    let other = ReviewState {
        hunks: vec![
            mark.clone(),
            HunkMark {
                content_hash: "h2".into(),
                ..mark
            },
        ],
        comments: vec![mk(true), other_c],
        ..Default::default()
    };
    let conflicts = merge(&mut base, other);
    assert_eq!(
        (base.hunks.len(), base.comments.len(), conflicts.len()),
        (2, 2, 1)
    );
    assert!(conflicts[0].contains("kept local"));
    let again = base.clone();
    assert!(merge(&mut base, again).is_empty());
}

#[test]
fn marks_only_leaves_legacy_comments() {
    let d = tempfile::tempdir().unwrap();
    CommentStore::new(d.path())
        .add_comment(&anchor("a.rs", 1, "p"))
        .unwrap();
    ReviewStore::open_marks_only(d.path(), "main").unwrap();
    assert!(d.path().join(".lgtm/directives.jsonl").is_file());
}

#[test]
fn plain_comments_repin_orphan_and_recover() {
    let mut a = [anchor("a.rs", 10, "x")];
    // unchanged: nothing to do
    assert!(!reanchor_anchors(
        &mut a,
        &[file("a.rs", vec![hunk(10, &["x"])])]
    ));
    // code shifted down
    assert!(reanchor_anchors(
        &mut a,
        &[file("a.rs", vec![hunk(14, &["x"])])]
    ));
    assert_eq!((a[0].target_line_no, a[0].orphaned), (14, false));
    // code gone from a file still in the diff
    assert!(reanchor_anchors(
        &mut a,
        &[file("a.rs", vec![hunk(14, &["y"])])]
    ));
    assert_eq!((a[0].target_line_no, a[0].orphaned), (14, true));
    // and back
    assert!(reanchor_anchors(
        &mut a,
        &[file("a.rs", vec![hunk(20, &["x"])])]
    ));
    assert_eq!((a[0].target_line_no, a[0].orphaned), (20, false));
}

#[test]
fn files_outside_the_diff_and_deleted_lines_are_left_alone() {
    let mut a = [anchor("b.rs", 3, "x")];
    assert!(!reanchor_anchors(
        &mut a,
        &[file("a.rs", vec![hunk(1, &["z"])])]
    ));
    assert!(!a[0].orphaned);

    let mut del = hunk(1, &[]);
    del.lines.push(line(LineType::Deletion, "gone", None));
    del.lines[0].old_line_num = Some(5);
    let mut a = [anchor("a.rs", 5, "gone")];
    assert!(!reanchor_anchors(&mut a, &[file("a.rs", vec![del])]));
    assert!(!a[0].orphaned);
}

#[test]
fn folds_round_trip_toggle_and_merge() {
    let f = file(
        "src/lib.rs",
        vec![hunk(1, &["fn foo() {", "    bar();", "}"])],
    );
    let h_hash = hunk_hash(&f.hunks[0]);
    let mut folds = Vec::new();

    // Toggle on
    let on = toggle_fold_mark(&mut folds, "src/lib.rs", &h_hash, 1, 3);
    assert!(on);
    assert_eq!(folds.len(), 1);

    // Load hunk folds
    let loaded = load_hunk_folds(&folds, "src/lib.rs", &h_hash);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].start_line, 1);
    assert_eq!(loaded[0].end_line, 3);

    // Save and load via ReviewStore
    let d = tempfile::tempdir().unwrap();
    let store = ReviewStore::open(d.path(), "test-branch").unwrap();
    let state = ReviewState {
        version: 1,
        target: "test-branch".into(),
        folds: folds.clone(),
        ..Default::default()
    };
    store.save(&state).unwrap();

    let reloaded = store.load().unwrap();
    assert_eq!(reloaded.folds.len(), 1);
    assert_eq!(reloaded.folds[0].start_line, 1);
    assert_eq!(reloaded.folds[0].end_line, 3);

    // Merge folds from another state
    let mut base = ReviewState::default();
    let other = ReviewState {
        folds: vec![FoldMark {
            file: "other.rs".into(),
            hunk_hash: "hash123".into(),
            start_line: 10,
            end_line: 20,
        }],
        ..Default::default()
    };
    merge(&mut base, other);
    assert_eq!(base.folds.len(), 1);

    // Toggle off
    let off = toggle_fold_mark(&mut folds, "src/lib.rs", &h_hash, 1, 3);
    assert!(!off);
    assert!(folds.is_empty());
}
