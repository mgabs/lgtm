use super::*;
use crate::app::test_util::create_test_app;
use ratatui::{backend::TestBackend, Terminal};

fn fixture() -> App {
    let mut app = create_test_app();
    let output = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&app.git_engine.repo_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    crate::app::test_util::wait_for_view_data(&mut app);
    app.snapshot = crate::pr::PrEngine::parse_unified_diff(
        "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,4 +1,4 @@ fn example()\n fn example() {\n-    let answer = old_value;\n+    let answer = new_value;\n     println!(\"hello 界 é 🦀 and a longer sentence for wrapping\");\n }\n"
    ).unwrap();
    app.comments.push(
        serde_json::from_value(serde_json::json!({
            "id":"fixture", "file_path":"a.rs", "target_line_no":2,
            "target_code":"let answer", "context_before":[], "context_after":[],
            "comment_text":"check this value", "replies":["a reply"],
            "created_at_turn":1,"is_resolved":false,
        }))
        .unwrap(),
    );
    app.threads.push(lgtm_core::github::Thread {
        id: "thread".into(),
        path: "a.rs".into(),
        line: Some(1),
        resolved: false,
        comments: vec![],
    });
    for (line, title) in [(None, "file finding"), (Some(2), "line finding")] {
        app.alerts.push(crate::guard::GuardAlert {
            rule_id: "fixture".into(),
            fingerprint: "fixture".into(),
            severity: crate::guard::Severity::High,
            title: title.into(),
            description: "check the change".into(),
            line_no: line,
            file_path: "a.rs".into(),
        });
    }
    app
}

fn screen(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| render_diff_view(f, f.area(), app))
        .unwrap();
    let buffer = terminal.backend().buffer();
    if width == 0 {
        return String::new();
    }
    let mut output = String::new();
    for y in 0..height {
        let mut run = String::new();
        let mut style = buffer[(0, y)].style();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            if cell.style() != style {
                output.push_str(&format!("{style:?}|{run:?}\n"));
                run.clear();
                style = cell.style();
            }
            run.push_str(cell.symbol());
        }
        output.push_str(&format!("{style:?}|{run:?}\n--\n"));
    }
    output
}

#[test]
fn display_fixtures_preserve_text_and_styles() {
    for (name, expected) in [
        ("unified", include_str!("fixtures/unified.txt")),
        ("split", include_str!("fixtures/split.txt")),
        ("folded", include_str!("fixtures/folded.txt")),
        ("block", include_str!("fixtures/block.txt")),
        ("narrow", include_str!("fixtures/narrow.txt")),
    ] {
        let mut app = fixture();
        match name {
            "split" => app.is_split_view = true,
            "folded" => app.snapshot.files[0].hunks[0].is_folded = true,
            "block" => {
                app.snapshot.files[0].hunks[0]
                    .folded_blocks
                    .push(lgtm_core::model::BlockRange {
                        start_line: 2,
                        end_line: 3,
                    })
            }
            _ => {}
        }
        let actual = screen(&mut app, if name == "narrow" { 36 } else { 96 }, 20);
        if std::env::var_os("LGTM_UPDATE_GOLDENS").is_some() {
            std::fs::write(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("src/ui/diff/fixtures/{name}.txt")),
                actual,
            )
            .unwrap();
        } else {
            assert_eq!(actual, expected, "display fixture {name}");
        }
    }
}

#[test]
fn navigation_and_dynamic_styling_reuse_content_preparation() {
    let mut app = fixture();
    screen(&mut app, 96, 20);
    let rows = app.cached_rows();
    let layouts = app.render_cache.borrow().layouts_prepared;
    let words = app.render_cache.borrow().words_prepared;
    assert_eq!(words, 1);
    for _ in 0..8 {
        app.cursor_down();
        app.search_query = "example".into();
        app.visual_mode = true;
        app.visual_anchor_idx = Some(0);
        screen(&mut app, 96, 20);
        app.cursor_up();
    }
    assert!(std::rc::Rc::ptr_eq(&rows, &app.cached_rows()));
    assert_eq!(app.render_cache.borrow().layouts_prepared, layouts);
    assert_eq!(app.render_cache.borrow().words_prepared, words);
    app.visual_mode = false;
    app.comments[0].comment_text = "a newer directive".into();
    let updated = screen(&mut app, 96, 20);
    assert!(updated.contains("a newer directive"));
    assert_eq!(app.render_cache.borrow().layouts_prepared, layouts + 1);
    assert_eq!(app.render_cache.borrow().words_prepared, words);
    assert!(std::rc::Rc::ptr_eq(&rows, &app.cached_rows()));
    app.is_split_view = true;
    screen(&mut app, 96, 20);
    assert_eq!(app.render_cache.borrow().words_prepared, words);
    screen(&mut app, 80, 16);
    assert_eq!(app.render_cache.borrow().words_prepared, words);
}

#[test]
fn word_differences_are_prepared_only_for_visible_pairs() {
    let mut app = fixture();
    app.comments.clear();
    app.threads.clear();
    app.alerts.clear();
    let hunk = &mut app.snapshot.files[0].hunks[0];
    hunk.lines = (0..1000)
        .flat_map(|i| {
            [
                DiffLine {
                    line_type: LineType::Deletion,
                    content: format!("old value {i}"),
                    old_line_num: Some(i + 1),
                    new_line_num: None,
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: format!("new value {i}"),
                    old_line_num: None,
                    new_line_num: Some(i + 1),
                },
            ]
        })
        .collect();
    app.invalidate_diff_content();
    screen(&mut app, 96, 12);
    let words = app.render_cache.borrow().words_prepared;
    assert!((1..=6).contains(&words));
    assert!(app.render_cache.borrow().viewport_rows <= 10);
    app.cursor_to_bottom();
    screen(&mut app, 96, 12);
    assert!(app.render_cache.borrow().words_prepared <= words + 6);
    assert_eq!(app.render_cache.borrow().layouts_prepared, 1);
}

#[test]
fn split_cursor_tracks_both_members_of_a_changed_pair() {
    let mut app = fixture();
    app.is_split_view = true;
    app.alerts.clear();
    app.comments.clear();
    app.threads.clear();
    app.cursor_line_idx = 1; // Deletion.
    let left = screen(&mut app, 96, 12);
    app.cursor_line_idx = 2; // Paired addition, same rendered row.
    let right = screen(&mut app, 96, 12);
    assert_eq!(left, right);
    assert!(right.contains("▶  2"));
    app.cursor_line_idx = 3;
    assert_ne!(screen(&mut app, 96, 12), right);
}

#[test]
fn folds_and_same_allocation_content_updates_invalidate_the_right_cache() {
    let mut app = fixture();
    screen(&mut app, 96, 20);
    let mapping = app.cached_rows();
    app.fold_all_hunks();
    assert!(!std::rc::Rc::ptr_eq(&mapping, &app.cached_rows()));
    assert_eq!(*app.visible_line_indices(), vec![0]);
    assert!(screen(&mut app, 96, 20).contains("Folded"));
    app.unfold_all_hunks();
    app.snapshot.files[0].hunks[0].lines[0].content = "replacement source".into();
    app.invalidate_diff_content();
    assert!(screen(&mut app, 96, 20).contains("replacement source"));
}

#[test]
fn scrolling_past_u16_rows_keeps_the_actual_cursor_visible() {
    let mut app = fixture();
    app.alerts.clear();
    app.comments.clear();
    app.threads.clear();
    app.snapshot.files[0].hunks[0].lines = (1..=70_000)
        .map(|n| DiffLine {
            line_type: LineType::Addition,
            content: format!("source {n}"),
            old_line_num: None,
            new_line_num: Some(n),
        })
        .collect();
    app.invalidate_diff_content();
    app.cursor_to_bottom();
    let output = screen(&mut app, 80, 10);
    assert!(app.diff_scroll_offset > u16::MAX as usize);
    assert!(output.contains("source 70000"));
    assert!(app.render_cache.borrow().viewport_rows <= 8);
}

#[test]
fn virtual_wrapping_matches_a_complete_paragraph_on_unicode_and_resize() {
    use ratatui::{buffer::Buffer, widgets::Widget};
    let mut app = fixture();
    app.comments[0].comment_text =
        "界 🦀 é long directive with several words to wrap across narrow screens".repeat(3);
    for width in [20, 36, 80, 96] {
        for cursor in [0, 1, 2, 3, 4] {
            app.cursor_line_idx = cursor;
            app.visual_mode = true;
            app.visual_anchor_idx = Some(1);
            let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
            terminal
                .draw(|f| render_diff_view(f, f.area(), &mut app))
                .unwrap();
            let mapping = app.cached_rows();
            let canonical = mapping.representatives[cursor];
            let mut cache = app.render_cache.borrow_mut();
            let layout = cache.layout.take().unwrap();
            let lines = layout
                .rows
                .lines
                .iter()
                .zip(&layout.rows.anchors)
                .map(|(line, anchor)| {
                    viewport::styled_row(&app, &mut cache, line, *anchor, width, canonical)
                })
                .collect::<Vec<_>>();
            let mut expected = Buffer::empty(Rect::new(0, 0, width, 12));
            let inner = Rect::new(1, 1, width - 2, 10);
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((app.diff_scroll_offset as u16, 0))
                .render(inner, &mut expected);
            for y in 1..11 {
                for x in 1..width - 1 {
                    assert_eq!(
                        terminal.backend().buffer()[(x, y)],
                        expected[(x, y)],
                        "width {width}, cursor {cursor}, cell {x},{y}"
                    );
                }
            }
            cache.layout = Some(layout);
        }
    }
}

#[test]
fn tiny_viewports_do_not_panic() {
    let mut app = fixture();
    for width in 0..5 {
        for height in 0..5 {
            screen(&mut app, width, height);
        }
    }
}

#[test]
fn annotation_changes_refresh_rows_without_recomputing_word_differences() {
    let mut app = fixture();
    let before = screen(&mut app, 96, 30);
    assert!(before.contains("◆"));
    let words = app.render_cache.borrow().words_prepared;
    app.threads[0].resolved = true;
    let after = screen(&mut app, 96, 30);
    assert!(after.contains("◇"));
    assert!(!after.contains("◆"));
    app.comments[0].is_resolved = true;
    app.comments[0].remote_id = Some(42);
    app.comments[0].replies.push("a newer reply".into());
    let after = screen(&mut app, 96, 30);
    assert!(after.contains("RESOLVED DIRECTIVE"));
    assert!(after.contains("[↑]"));
    assert!(after.contains("a newer reply"));
    app.comments[0].orphaned = true;
    app.alerts[0].title = "a newer finding".into();
    let after = screen(&mut app, 96, 30);
    assert!(!after.contains("RESOLVED DIRECTIVE"));
    assert!(after.contains("a newer finding"));
    assert_eq!(app.render_cache.borrow().words_prepared, words);
}

#[test]
fn split_change_groups_stop_at_folded_block_boundaries() {
    let mut app = fixture();
    app.alerts.clear();
    app.comments.clear();
    app.threads.clear();
    app.is_split_view = true;
    let hunk = &mut app.snapshot.files[0].hunks[0];
    hunk.lines = vec![
        DiffLine {
            line_type: LineType::Deletion,
            content: "visible old".into(),
            old_line_num: Some(1),
            new_line_num: None,
        },
        DiffLine {
            line_type: LineType::Deletion,
            content: "hidden old".into(),
            old_line_num: Some(2),
            new_line_num: None,
        },
        DiffLine {
            line_type: LineType::Addition,
            content: "visible new".into(),
            old_line_num: None,
            new_line_num: Some(1),
        },
        DiffLine {
            line_type: LineType::Addition,
            content: "hidden new".into(),
            old_line_num: None,
            new_line_num: Some(2),
        },
    ];
    hunk.folded_blocks.push(lgtm_core::model::BlockRange {
        start_line: 2,
        end_line: 2,
    });
    app.invalidate_diff_content();
    let output = screen(&mut app, 96, 20);
    assert!(output.contains("visible old"));
    assert!(output.contains("visible new"));
    // The placeholder intentionally contains its first source snippet.
    assert_eq!(output.matches("hidden old").count(), 1);
    assert!(!output.contains("hidden new"));
    assert!(output.contains("Folded AST Block"));
}

#[test]
fn file_drawer_renders_only_its_viewport_and_refreshes_review_counts() {
    let mut app = fixture();
    let base = app.snapshot.files[0].clone();
    app.snapshot.files = (0..50)
        .map(|i| {
            let mut file = base.clone();
            file.new_path = format!("file-{i:02}.rs").into();
            file
        })
        .collect();
    app.selected_file_idx = 40;
    let mut extra = app.snapshot.files[40].hunks[0].clone();
    extra.new_start = 20;
    app.snapshot.files[40].hunks.push(extra);
    lgtm_core::review::toggle_hunk(&mut app.marks, &app.snapshot.files[40], 0, "fixture");
    app.invalidate_diff_content();
    let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
    terminal
        .draw(|f| render_file_drawer(f, f.area(), &app))
        .unwrap();
    let plain = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(plain.contains("file-40.rs"));
    assert!(!plain.contains("file-00.rs"));
    assert!(plain.contains("✓1/2"));
    assert_eq!(app.render_cache.borrow().drawer_rows, 4);
    app.marks.clear();
    terminal
        .draw(|f| render_file_drawer(f, f.area(), &app))
        .unwrap();
    let plain = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(!plain.contains("✓1/2"));
}
