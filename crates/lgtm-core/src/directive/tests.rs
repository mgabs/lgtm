use super::*;
use crate::guard::{GuardAlert, Severity};

#[test]
fn test_directive_from_guard_alert() {
    let alert = GuardAlert {
        rule_id: "sec-001".to_string(),
        fingerprint: "fp-123".to_string(),
        severity: Severity::High,
        title: "Hardcoded Secret".to_string(),
        description: "Found AWS secret key".to_string(),
        line_no: Some(42),
        file_path: "src/main.rs".to_string(),
    };

    let directive = Directive::from_guard_alert(&alert);
    assert_eq!(directive.file_path, "src/main.rs");
    assert_eq!(directive.target_line_no, 42);
    assert!(directive.comment_text.contains("Hardcoded Secret"));
    assert!(directive.comment_text.contains("Found AWS secret key"));
    assert!(!directive.is_resolved);

    let no_line_alert = GuardAlert {
        rule_id: "sec-002".to_string(),
        fingerprint: "fp-456".to_string(),
        severity: Severity::Warn,
        title: "Missing Doc".to_string(),
        description: "File lacks header doc".to_string(),
        line_no: None,
        file_path: "src/doc.rs".to_string(),
    };
    let d2 = Directive::from_guard_alert(&no_line_alert);
    assert_eq!(d2.target_line_no, 1);
}

#[test]
fn test_new_id_is_unique_and_non_static() {
    let id1 = new_id();
    let id2 = new_id();
    assert_ne!(id1, id2);
    assert_ne!(id1, "xyzzy");
    assert!(id1.contains('-'));
}

#[test]
fn test_compile_batch_review_prompt_replies_formatting() {
    let mut d = Directive {
        id: "d1".to_string(),
        file_path: "src/a.rs".to_string(),
        target_line_no: 5,
        target_code: "foo()".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "Clean this".to_string(),
        replies: Vec::new(),
        created_at_turn: 1,
        is_resolved: false,
        pr_number: None,
        orphaned: false,
        track: Default::default(),
    };
    let (prompt_without, _) = DirectiveStore::compile_batch_review_prompt(&[d.clone()], 1);
    assert!(!prompt_without.contains("**Replies:**"));

    d.replies.push("I will".to_string());
    let (prompt_with, _) = DirectiveStore::compile_batch_review_prompt(&[d], 1);
    assert!(prompt_with.contains("**Replies:**"));
    assert!(prompt_with.contains("- I will"));
}

#[test]
fn test_compile_pr_review_prompt_scoping_and_code_block() {
    let target_pr = 100;
    let other_pr = 200;

    let d_matching = Directive {
        id: "d1".to_string(),
        file_path: "src/a.rs".to_string(),
        target_line_no: 10,
        target_code: "let x = 1;".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "matching comment".to_string(),
        replies: vec!["reply 1".to_string()],
        created_at_turn: 1,
        is_resolved: false,
        pr_number: Some(target_pr),
        orphaned: false,
        track: Default::default(),
    };

    let d_different_pr = Directive {
        id: "d2".to_string(),
        file_path: "src/b.rs".to_string(),
        target_line_no: 20,
        target_code: "let y = 2;".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "different pr comment".to_string(),
        replies: Vec::new(),
        created_at_turn: 1,
        is_resolved: false,
        pr_number: Some(other_pr),
        orphaned: false,
        track: Default::default(),
    };

    let d_resolved_matching = Directive {
        id: "d3".to_string(),
        file_path: "src/c.rs".to_string(),
        target_line_no: 30,
        target_code: "let z = 3;".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "resolved matching comment".to_string(),
        replies: Vec::new(),
        created_at_turn: 1,
        is_resolved: true,
        pr_number: Some(target_pr),
        orphaned: false,
        track: Default::default(),
    };

    let d_empty_code = Directive {
        id: "d4".to_string(),
        file_path: "src/d.rs".to_string(),
        target_line_no: 40,
        target_code: "".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "empty code comment".to_string(),
        replies: Vec::new(),
        created_at_turn: 1,
        is_resolved: false,
        pr_number: None,
        orphaned: false,
        track: Default::default(),
    };

    let (prompt, _) = DirectiveStore::compile_pr_review_prompt(
        &[
            d_matching,
            d_different_pr,
            d_resolved_matching,
            d_empty_code,
        ],
        target_pr,
    );

    assert!(prompt.contains("matching comment"));
    assert!(prompt.contains("```\nlet x = 1;\n```"));
    assert!(!prompt.contains("different pr comment"));
    assert!(!prompt.contains("resolved matching comment"));
    assert!(prompt.contains("empty code comment"));
    let occurrences = prompt.matches("```").count();
    assert_eq!(occurrences, 2);
}

#[test]
fn test_directive_store_persistence() {
    let tmp = tempfile::tempdir().unwrap();
    let store = DirectiveStore::new(tmp.path());
    let mut directive = Directive {
        id: "dir-1".to_string(),
        file_path: "src/lib.rs".to_string(),
        target_line_no: 10,
        target_code: "fn test() {}".to_string(),
        context_before: Vec::new(),
        context_after: Vec::new(),
        comment_text: "Refactor me".to_string(),
        replies: Vec::new(),
        created_at_turn: 1,
        is_resolved: false,
        pr_number: None,
        orphaned: false,
        track: Default::default(),
    };

    store.add_directive(&directive).unwrap();
    let loaded = store.load_directives();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "dir-1");

    directive.add_reply("Fixed in commit abc");
    assert_eq!(directive.replies.len(), 1);

    let (prompt_turn, path_turn) =
        DirectiveStore::compile_batch_review_prompt(std::slice::from_ref(&directive), 1);
    assert!(prompt_turn.contains("Refactor me"));
    assert_eq!(path_turn, PathBuf::from(".lgtm/review-1.md"));

    let (prompt_pr, path_pr) =
        DirectiveStore::compile_pr_review_prompt(std::slice::from_ref(&directive), 42);
    assert!(prompt_pr.contains("Refactor me"));
    assert_eq!(path_pr, PathBuf::from(".lgtm/pr-42-review.md"));

    directive.toggle_resolved();
    assert!(directive.is_resolved);

    let (resolved_turn, _) =
        DirectiveStore::compile_batch_review_prompt(std::slice::from_ref(&directive), 1);
    assert!(!resolved_turn.contains("Refactor me"));

    let (empty_prompt, _) = DirectiveStore::compile_pr_review_prompt(&[], 42);
    assert!(empty_prompt.contains("LGTM!"));

    // Test add_alert_directive
    let test_alert = GuardAlert {
        rule_id: "sec-001".to_string(),
        fingerprint: "fp-123".to_string(),
        severity: Severity::High,
        title: "Hardcoded Secret".to_string(),
        description: "Found AWS secret key".to_string(),
        line_no: Some(42),
        file_path: "src/main.rs".to_string(),
    };
    let created = store.add_alert_directive(&test_alert).unwrap();
    assert_eq!(created.file_path, "src/main.rs");
    let loaded = store.load_directives();
    assert_eq!(loaded.len(), 2);

    // Test compatibility shims
    assert_eq!(store.load_comments().len(), 2);
    store.add_comment(&created).unwrap();
    assert_eq!(store.load_directives().len(), 3);
    store.save_comments(&[directive]).unwrap();
    assert_eq!(store.load_directives().len(), 1);
}

#[test]
fn legacy_directives_get_persisted_stable_ids() {
    let dir = tempfile::tempdir().unwrap();
    let store = DirectiveStore::new(dir.path());
    let legacy = serde_json::json!({
        "file_path": "a.rs",
        "target_line_no": 1,
        "target_code": "code",
        "context_before": [],
        "context_after": [],
        "comment_text": "keep me",
        "created_at_turn": 1,
        "is_resolved": false
    });
    std::fs::write(
        dir.path().join(".lgtm/directives.jsonl"),
        format!("{legacy}\n"),
    )
    .unwrap();
    let first = store.load_directives();
    let second = store.load_directives();
    assert_eq!(first.len(), 1);
    assert!(!first[0].id.is_empty());
    assert_eq!(first, second);
    assert_eq!(first[0].comment_text, "keep me");
    assert!(
        std::fs::read_to_string(dir.path().join(".lgtm/directives.jsonl"))
            .unwrap()
            .contains(&first[0].id)
    );
}

#[test]
fn legacy_comments_file_migrates_once_to_directives() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join(".lgtm/comments.jsonl");
    let new_path = dir.path().join(".lgtm/directives.jsonl");
    std::fs::create_dir_all(dir.path().join(".lgtm")).unwrap();
    let legacy = serde_json::json!({
        "id": "abc",
        "file_path": "a.rs",
        "target_line_no": 1,
        "target_code": "c",
        "context_before": [],
        "context_after": [],
        "comment_text": "migrate me",
        "created_at_turn": 1,
        "is_resolved": false
    });
    std::fs::write(&legacy_path, format!("{legacy}\n")).unwrap();
    let loaded = DirectiveStore::new(dir.path()).load_directives();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].comment_text, "migrate me");
    assert!(new_path.is_file());
    assert!(!legacy_path.exists());
    assert!(std::fs::read_to_string(&new_path)
        .unwrap()
        .contains("migrate me"));
}

#[test]
fn legacy_comments_file_never_overwrites_existing_directives() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".lgtm")).unwrap();
    let line = |t: &str| {
        format!(
            "{}\n",
            serde_json::json!({
                "id": t,
                "file_path": "a.rs",
                "target_line_no": 1,
                "target_code": "c",
                "context_before": [],
                "context_after": [],
                "comment_text": t,
                "created_at_turn": 1,
                "is_resolved": false
            })
        )
    };
    std::fs::write(dir.path().join(".lgtm/comments.jsonl"), line("old")).unwrap();
    std::fs::write(dir.path().join(".lgtm/directives.jsonl"), line("new")).unwrap();
    let loaded = DirectiveStore::new(dir.path()).load_directives();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].comment_text, "new");
    assert!(dir.path().join(".lgtm/comments.jsonl").exists());
}
