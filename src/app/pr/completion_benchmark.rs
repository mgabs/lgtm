use super::*;
use crate::app::test_util::create_test_app;
use std::time::Instant;

fn fixture(app: &mut App) -> DiffSnapshot {
    let mut patch =
        "diff --git a/a.rs b/a.rs\n--- /dev/null\n+++ b/a.rs\n@@ -0,0 +1,2000 @@\n".to_string();
    for line in 1..=2000 {
        patch.push_str(&format!("+let value_{line} = {line};\n"));
    }
    let snapshot = crate::pr::PrEngine::parse_unified_diff(&patch).unwrap();
    app.comments = (1..=1000).map(|line| serde_json::from_value(serde_json::json!({
        "id":format!("fixture-{line}"), "file_path":"a.rs", "target_line_no":1,
        "target_code":format!("let value_{line} = {line};"), "context_before":[], "context_after":[],
        "comment_text":"Check this value", "created_at_turn":1,"is_resolved":false,"pr_number":42,
        "track":{"hunk_hash":"earlier-hash"}
    })).unwrap()).collect();
    std::fs::write(
        app.git_engine.repo_path.join(".lgtm/state.json"),
        r#"{"current_turn":2,"last_reviewed_tree_oid":null,"turn_timestamps":[]}"#,
    )
    .unwrap();
    snapshot
}

#[test]
#[ignore = "offline PR completion benchmark; no network or credentials"]
fn pr_completion_workload_benchmark() {
    if let Some(path) = std::env::var_os("LGTM_PR_PERF_LOG") {
        lgtm_core::debug::init(std::path::Path::new(&path)).unwrap();
    }
    let mut app = create_test_app();
    let mut snapshot = fixture(&mut app);
    app.pr.current_number = Some(7);
    app.review_generation = 1;
    app.pr_load_latest
        .store(1, std::sync::atomic::Ordering::Relaxed);
    app.pr_loading = Some((42, 1));
    app.view_data.is_git = true;
    let started = Instant::now();
    let input = completion::Input::capture(&app, 42);
    let repo = app.git_engine.repo_path.clone();
    let (tx, rx) = crossbeam_channel::bounded(1);
    app.spawn_silent_job(
        move || {
            let started = Instant::now();
            let completion = completion::prepare(&repo, 42, &mut snapshot, input, || true).unwrap();
            let prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
            (
                LoadedPr {
                    pr: 42,
                    generation: 1,
                    repo,
                    snapshot,
                    findings: Default::default(),
                    completion,
                },
                prepare_ms,
            )
        },
        move |app, (ready, prepare_ms)| {
            app.ready_pr = Some(ready);
            let started = Instant::now();
            app.apply_ready_pr();
            tx.send((prepare_ms, started.elapsed().as_secs_f64() * 1000.0))
                .unwrap();
        },
    );
    let request_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    let mut frames = Vec::new();
    let deadline = Instant::now() + std::time::Duration::from_secs(60);
    let (prepare_ms, apply_ms) = loop {
        app.handle_background_events();
        if let Ok(result) = rx.try_recv() {
            break result;
        }
        assert!(Instant::now() < deadline, "completion benchmark timed out");
        let started = Instant::now();
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        terminal
            .draw(|frame| crate::ui::render(frame, &mut app))
            .unwrap();
        frames.push(started.elapsed().as_secs_f64() * 1000.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    frames.sort_by(f64::total_cmp);
    assert!(!frames.is_empty());
    println!("pr_completion directives=1000 lines=2000 request_ms={request_ms:.3} worker_prepare_ms={prepare_ms:.3} ui_completion_ms={apply_ms:.3} presented_frames={} input_draw_p95_ms={:.3}", frames.len(), frames[(frames.len()*95).div_ceil(100)-1]);
    assert_eq!(app.pr.current_number, Some(42));
    assert_eq!(app.comments[999].target_line_no, 1000);
    assert_eq!(app.comment_store.load_comments(), app.comments);
}
