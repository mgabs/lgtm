use super::*;
use crate::app::{
    test_util::{create_test_app, wait_for_audit, wait_for_local_refresh},
    ActiveView, App,
};
use crate::model::{ChangeType, DiffHunk, DiffLine, DiffSnapshot, FileDiff, LineType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SECRET: &str = "sentinel-private-source-and-comment";

fn init_repo(path: &Path) {
    let output = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn draw(
    terminal: &mut Terminal<TestBackend>,
    app: &mut App,
    metrics: &mut Option<UiLoop>,
    activity: bool,
) {
    lgtm_core::debug::measure("ui.draw", || terminal.draw(|f| crate::ui::render(f, app))).unwrap();
    if let Some(metrics) = metrics {
        metrics.drawn(activity);
    }
}

fn key(app: &mut App, metrics: &mut Option<UiLoop>, code: KeyCode) {
    if let Some(metrics) = metrics {
        metrics.event();
        metrics.input();
    }
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn snapshot(lines: usize) -> DiffSnapshot {
    DiffSnapshot::new(vec![FileDiff {
        old_path: None,
        new_path: "review.txt".into(),
        change_type: ChangeType::Added,
        additions: lines,
        deletions: 0,
        is_folded: false,
        is_reviewed: false,
        hunks: vec![DiffHunk {
            header: format!("@@ -0,0 +1,{lines} @@"),
            new_start: 1,
            new_lines: lines as u32,
            lines: (1..=lines)
                .map(|line| DiffLine {
                    line_type: LineType::Addition,
                    content: format!("{SECRET} {line}"),
                    old_line_num: None,
                    new_line_num: Some(line as u32),
                })
                .collect(),
            ..Default::default()
        }],
    }])
}

#[test]
fn diagnostic_child() {
    if std::env::var_os("LGTM_UI_TEST_CHILD").is_none() {
        return;
    }
    let mut app = create_test_app();
    let repo = app.git_engine.repo_path.clone();
    init_repo(&repo);
    std::fs::write(repo.join(".gitignore"), ".lgtm/\n").unwrap();
    std::fs::write(repo.join("review.txt"), SECRET).unwrap();
    if std::env::var_os("LGTM_UI_TEST_ENABLE").is_some() {
        lgtm_core::debug::init(Path::new(&std::env::var_os("LGTM_UI_TEST_LOG").unwrap())).unwrap();
    }
    crate::app::test_util::wait_for_view_data(&mut app);
    app.snapshot = snapshot(20);
    let mut metrics = lgtm_core::debug::enabled().then(UiLoop::default);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    draw(&mut terminal, &mut app, &mut metrics, false); // Startup, not idle.
    draw(&mut terminal, &mut app, &mut metrics, false); // Idle redraw.
    key(&mut app, &mut metrics, KeyCode::Down);
    app.active_view = ActiveView::CommentInput;
    app.comment_input = SECRET.into();
    key(&mut app, &mut metrics, KeyCode::Char('x'));
    draw(&mut terminal, &mut app, &mut metrics, false);
    app.spawn_job(
        SECRET,
        || (),
        |app, ()| app.status_message = Some("callback done".into()),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut activity = false;
    while app.status_message.as_deref() != Some("callback done") {
        activity |= app.handle_background_events();
        assert!(Instant::now() < deadline, "callback did not arrive");
        std::thread::yield_now();
    }
    draw(&mut terminal, &mut app, &mut metrics, activity);
    app.trigger_diff_scan();
    wait_for_audit(&mut app);
    app.trigger_repo_scan();
    wait_for_audit(&mut app);
    app.export_audit_report(Some(&repo.join(".lgtm/privacy-report.md")))
        .unwrap();
    wait_for_audit(&mut app);
    app.audit.alerts = vec![crate::guard::GuardAlert {
        file_path: "review.txt".into(),
        line_no: Some(1),
        rule_id: "fixture".into(),
        fingerprint: "fixture".into(),
        title: SECRET.into(),
        description: SECRET.into(),
        severity: crate::guard::Severity::High,
    }];
    app.active_view = ActiveView::AuditReport;
    crate::app::test_util::wait_for_view_data(&mut app);
    assert!(app
        .view_data
        .snippet_for(&app.audit.root_dir, &app.audit.alerts[0])
        .unwrap()[0]
        .1
        .contains(SECRET));
    std::fs::write(repo.join(".lgtm/prompt.txt"), SECRET).unwrap();
    app.open_agent_prompt();
    crate::app::test_util::wait_for_view_data(&mut app);
    app.active_view = ActiveView::Diff;
    app.reload_diff();
    wait_for_local_refresh(&mut app);
    // A key followed by quit/no draw must not look like a presented frame.
    key(&mut app, &mut metrics, KeyCode::Down);
    drop(metrics);
    drop(app);
    let _ = std::fs::remove_dir_all(repo);
}

fn child(path: &Path, enabled: bool) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "diagnostics::tests::diagnostic_child"])
        .env("LGTM_UI_TEST_CHILD", "1")
        .env("LGTM_UI_TEST_LOG", path)
        .env("LGTM_DEBUG_LOG", path)
        .env_remove("LGTM_UI_TEST_ENABLE");
    if enabled {
        command.env("LGTM_UI_TEST_ENABLE", "1");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn log_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lgtm-ui-{label}-{}.jsonl", std::process::id()))
}

#[test]
fn ui_diagnostics_remain_disabled_by_default() {
    let path = log_path("disabled");
    let _ = std::fs::remove_file(&path);
    child(&path, false);
    assert!(!path.exists());
}

#[test]
fn ui_diagnostics_measure_activity_without_recording_content() {
    let path = log_path("enabled");
    let _ = std::fs::remove_file(&path);
    child(&path, true);
    let contents = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!contents.contains(SECRET));
    assert!(!contents.contains("review.txt"));
    let records = contents
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let ends = |name| {
        records
            .iter()
            .filter(move |r| r["event"] == "end" && r["operation"] == name)
            .collect::<Vec<_>>()
    };
    for start in records.iter().filter(|r| r["event"] == "start") {
        let matched = records
            .iter()
            .filter(|r| r["event"] == "end" && r["id"] == start["id"] && r["pid"] == start["pid"])
            .collect::<Vec<_>>();
        assert_eq!(
            matched.len(),
            1,
            "unpaired operation: {}",
            start["operation"]
        );
        assert!(matched[0]["elapsed_ms"].as_f64().unwrap() >= 0.0);
    }
    let frames = ends("ui.frame");
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0]["counts"]["idle"], 0);
    assert_eq!(frames[1]["counts"]["idle"], 1);
    assert_eq!(frames[2]["counts"]["inputs"], 2);
    assert_eq!(frames[3]["counts"]["background_activity"], 1);
    assert_eq!(frames[3]["counts"]["idle_frames_total"], 1);
    let inputs = ends("ui.input_to_draw");
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["counts"]["presented"], 1);
    assert_eq!(inputs[0]["counts"]["inputs"], 2);
    assert_eq!(inputs[1]["counts"]["presented"], 0);
    assert_eq!(ends("ui.render").len(), 4);
    assert_eq!(ends("ui.render")[0]["counts"]["width"], 100);
    assert_eq!(ends("ui.render")[0]["counts"]["selected_lines"], 20);
    assert_eq!(ends("ui.render")[0]["counts"]["changed_lines"], 20);
    assert!(ends("ui.callback").len() >= 2);
    assert_eq!(ends("ui.callback_wait")[0]["counts"]["applied"], 1);
    for name in [
        "ui.input",
        "ui.draw",
        "review.local_snapshot",
        "review.scan",
        "review.reanchor",
        "review.outcomes",
        "review.load_marks",
        "ui.audit_request",
        "ui.audit_snapshot",
        "ui.audit_progress",
        "ui.audit_apply",
        "audit.changes",
        "audit.repository",
        "audit.export",
        "ui.view_request",
        "ui.view_prepare",
        "ui.view_apply",
    ] {
        assert!(!ends(name).is_empty(), "missing {name}");
    }
}

fn percentile(values: &mut [f64], percent: usize) -> f64 {
    values.sort_by(f64::total_cmp);
    values[(values.len() * percent).div_ceil(100) - 1]
}

#[test]
#[ignore = "offline UI benchmark; optional LGTM_UI_PERF_LOG enables debug JSONL"]
fn ui_workload_benchmark() {
    if let Some(path) = std::env::var_os("LGTM_UI_PERF_LOG") {
        lgtm_core::debug::init(Path::new(&path)).unwrap();
    }
    for (name, lines, comments, working_files) in [
        ("baseline", 100, 0, 0),
        ("large_diff", 10_000, 0, 0),
        ("many_comments", 2_000, 1_000, 0),
        ("large_worktree", 100, 0, 2_000),
    ] {
        let mut app = create_test_app();
        let repo = app.git_engine.repo_path.clone();
        init_repo(&repo);
        std::fs::write(repo.join(".gitignore"), ".lgtm/\n").unwrap();
        for file in 0..working_files {
            std::fs::write(
                repo.join(format!("work-{file}.txt")),
                "working tree content\n",
            )
            .unwrap();
        }
        let view_prepare_started = Instant::now();
        crate::app::test_util::wait_for_view_data(&mut app);
        let view_prepare_ms = view_prepare_started.elapsed().as_secs_f64() * 1000.0;
        app.snapshot = snapshot(lines);
        app.comments = (0..comments).map(|idx| serde_json::from_value(serde_json::json!({
            "id": format!("fixture-{idx}"), "file_path": "review.txt", "target_line_no": idx + 1,
            "target_code": SECRET, "context_before": [], "context_after": [], "comment_text": SECRET,
            "created_at_turn": 1, "is_resolved": false,
        })).unwrap()).collect();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        let mut metrics = lgtm_core::debug::enabled().then(UiLoop::default);
        let mut render = Vec::new();
        let mut first_draw = 0.0;
        let mut input = Vec::new();
        let mut input_draw = Vec::new();
        for iteration in 0..21 {
            let started = Instant::now();
            draw(&mut terminal, &mut app, &mut metrics, false);
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            if iteration > 0 {
                render.push(elapsed);
            } else {
                first_draw = elapsed;
            }
            let started = Instant::now();
            key(&mut app, &mut metrics, KeyCode::Down);
            input.push(started.elapsed().as_secs_f64() * 1000.0);
            draw(&mut terminal, &mut app, &mut metrics, false);
            input_draw.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        let mut refresh = Vec::new();
        let mut refresh_request = Vec::new();
        // Refresh against real local files, separately from the synthetic review snapshot.
        for _ in 0..3 {
            let started = Instant::now();
            app.reload_diff();
            refresh_request.push(started.elapsed().as_secs_f64() * 1000.0);
            wait_for_local_refresh(&mut app);
            refresh.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        println!("scenario={name} lines={lines} comments={comments} working_files={working_files} view_prepare_ms={view_prepare_ms:.3} first_draw_ms={first_draw:.3} draw_median_ms={:.3} draw_p95_ms={:.3} input_p95_ms={:.3} input_to_draw_p95_ms={:.3} refresh_request_median_ms={:.3} refresh_complete_median_ms={:.3}",
            percentile(&mut render, 50), percentile(&mut render, 95), percentile(&mut input, 95), percentile(&mut input_draw, 95), percentile(&mut refresh_request, 50), percentile(&mut refresh, 50));
        drop(app);
        std::fs::remove_dir_all(repo).unwrap();
    }
    println!("TestBackend, 120x40, 21 no-input draws (first is startup) and 21 input/draw batches per scenario; no terminal flush or event polling. Compare the same build profile; timings are observations, not pass/fail thresholds.");
}
