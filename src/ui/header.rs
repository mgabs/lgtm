use super::*;
use std::hash::{Hash, Hasher};

fn header_signature(app: &App) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    app.active_view.hash(&mut hasher);
    if app.active_view == ActiveView::AuditReport {
        app.audit.alerts.len().hash(&mut hasher);
        for a in &app.audit.alerts {
            (a.severity as u8).hash(&mut hasher);
        }
        app.audit.filter_severity.map(|s| s as u8).hash(&mut hasher);
        app.audit.progress.as_deref().hash(&mut hasher);
        app.audit.files.len().hash(&mut hasher);
    } else if !app.view_data.is_git && app.pr.current_number.is_none() {
        app.git_engine.repo_path.hash(&mut hasher);
    } else {
        app.agent_sink().label().hash(&mut hasher);
        app.snapshot.total_additions.hash(&mut hasher);
        app.snapshot.total_deletions.hash(&mut hasher);
        app.pr.current_number.hash(&mut hasher);
        if let Some(ci) = &app.ci {
            (ci.passed, ci.failed, ci.pending).hash(&mut hasher);
        }
        app.view_data.turn.hash(&mut hasher);
        app.snapshot.files.len().hash(&mut hasher);
        {
            let h = app.hidden.borrow();
            (h.suppressed.len(), h.existing.len()).hash(&mut hasher);
        }
        app.alerts.len().hash(&mut hasher);
        app.comments.len().hash(&mut hasher);
        let orphans = app.comments.iter().filter(|c| c.orphaned).count();
        orphans.hash(&mut hasher);
        if let Some((status, _)) = &app.view_data.gate {
            status.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn build_header_line(app: &App) -> Line<'static> {
    if app.active_view == ActiveView::AuditReport {
        let crit = app
            .audit
            .alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::Critical)
            .count();
        let high = app
            .audit
            .alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::High)
            .count();
        let warn = app
            .audit
            .alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::Warn)
            .count();
        let filter_label = match app.audit.filter_severity {
            Some(crate::guard::Severity::Critical) => "🔴 Crit [1]",
            Some(crate::guard::Severity::High) => "🟠 High+ [2]",
            _ => "All [3]",
        };
        return Line::from(vec![
            Span::styled(
                " lgtm audit ",
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                app.audit
                    .progress
                    .as_ref()
                    .map_or_else(|| " | ".to_string(), |progress| format!(" | {progress} | ")),
            ),
            Span::styled(
                format!("Total: {} issue(s) ", app.audit.alerts.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[🔴 {} Crit] ", crit),
                Style::default().fg(Color::Red),
            ),
            Span::styled(
                format!("[🟠 {} High] ", high),
                Style::default().fg(Color::LightRed),
            ),
            Span::styled(
                format!("[🟡 {} Warn] ", warn),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(
                "| Filter: {} | Scanned: {} files | ",
                filter_label,
                app.audit.files.len()
            )),
            Span::styled(
                "[w] Export lgtm-report.md",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    if !app.view_data.is_git && app.pr.current_number.is_none() {
        return Line::from(vec![
            Span::styled(
                " lgtm ",
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled(
                "⚠️ Not a git repository",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " | Directory: {} | ",
                app.git_engine.repo_path.display()
            )),
            Span::styled("[q] Quit", Style::default().fg(Color::White)),
        ]);
    }

    let agent_str = format!(" Target: {} ", app.agent_sink());

    let total_add = app.snapshot.total_additions;
    let total_del = app.snapshot.total_deletions;

    let mut header_spans = vec![Span::styled(
        " lgtm ",
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(pr_num) = app.pr.current_number {
        header_spans.push(Span::styled(
            format!(" PR #{} | ", pr_num),
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        if let Some(ci) = &app.ci {
            header_spans.push(Span::styled(
                format!("CI ✓{} ✗{} ⏳{} | ", ci.passed, ci.failed, ci.pending),
                Style::default().fg(if ci.failed > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ));
        }
    } else {
        let turn = app.view_data.turn;
        header_spans.push(Span::raw(format!(" Turn #{} | ", turn)));
    }

    header_spans.push(Span::styled(
        format_worktree_badge(app),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));

    header_spans.extend(vec![
        Span::styled(
            format!("+{}", total_add),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("-{}", total_del),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " in {} changed file(s) |",
            app.snapshot.files.len()
        )),
    ]);

    let (n_supp, n_base) = {
        let h = app.hidden.borrow();
        (h.suppressed.len(), h.existing.len())
    };
    if n_supp + n_base > 0 {
        header_spans.push(Span::styled(
            format!(" (+{} suppressed, {} baselined :hidden) |", n_supp, n_base),
            Style::default().fg(Color::DarkGray),
        ));
    }

    if !app.alerts.is_empty() {
        header_spans.push(Span::styled(
            format!(" ⚠️ {} alerts (w) |", app.alerts.len()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if !app.comments.is_empty() {
        let orphans = app.comments.iter().filter(|c| c.orphaned).count();
        let orphan_tag = if orphans > 0 {
            format!(" ({orphans} ⚠ orphaned)")
        } else {
            String::new()
        };
        header_spans.push(Span::styled(
            format!(" 💬 {} comments{} |", app.comments.len(), orphan_tag),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some((status, _msg)) = &app.view_data.gate {
        if status == "waiting" {
            header_spans.push(Span::styled(
                " 🛑 GATE WAITING (A/:approve) |",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
    }

    header_spans.push(Span::styled(
        agent_str,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    Line::from(header_spans)
}

pub(crate) fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let sig = header_signature(app);
    let mut cache = app.render_cache.borrow_mut();
    if let Some((cached_sig, cached_line)) = &cache.header {
        if *cached_sig == sig {
            let line = Line::from(
                cached_line
                    .spans
                    .iter()
                    .map(|s| Span::styled(s.content.as_ref(), s.style))
                    .collect::<Vec<_>>(),
            );
            frame.render_widget(Paragraph::new(line), area);
            return;
        }
    }
    let header_line = build_header_line(app);
    cache.header = Some((sig, header_line));
    if let Some((_, cached_line)) = &cache.header {
        let line = Line::from(
            cached_line
                .spans
                .iter()
                .map(|s| Span::styled(s.content.as_ref(), s.style))
                .collect::<Vec<_>>(),
        );
        frame.render_widget(Paragraph::new(line), area);
    }
}

pub(crate) fn format_worktree_badge(app: &App) -> String {
    let repo_path = &app.git_engine.repo_path;
    let name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("root");

    if let Ok(repo) = app.git_engine.open_repo() {
        if let Ok(head) = repo.head() {
            if head.is_branch() {
                if let Some(branch) = head.shorthand() {
                    return format!(" [wt: {} ({})] |", name, branch);
                }
            } else if let Ok(commit) = head.peel_to_commit() {
                let sha = commit.id().to_string();
                let short_sha = if sha.len() >= 7 { &sha[..7] } else { &sha };
                return format!(" [wt: {} (detached@{})] |", name, short_sha);
            }
        }
    }
    format!(" [wt: {}] |", name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_format_worktree_badge() {
        let dir = tempdir().unwrap();
        let (_tx, rx) = crossbeam_channel::unbounded();
        let app = App::new(dir.path().to_path_buf(), rx, None, None, None);

        let badge = format_worktree_badge(&app);
        assert!(badge.contains("[wt: "));
    }

    fn header_text(app: &App) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(250, 1)).unwrap();
        terminal.draw(|f| render_header(f, f.area(), app)).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    fn alert(sev: crate::guard::Severity) -> crate::guard::GuardAlert {
        crate::guard::GuardAlert {
            rule_id: "r".into(),
            fingerprint: "f".into(),
            severity: sev,
            title: "t".into(),
            description: "d".into(),
            line_no: None,
            file_path: "a.rs".into(),
        }
    }

    #[test]
    fn non_git_header_shows_directory_and_quit() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = false;
        let text = header_text(&app);
        assert!(text.contains("Not a git repository"));
        assert!(text.contains(&app.git_engine.repo_path.display().to_string()));
        assert!(text.contains("[q] Quit"));
    }

    #[test]
    fn audit_header_counts_severities_and_progress() {
        use crate::guard::Severity::*;
        let mut app = crate::app::test_util::create_test_app();
        app.active_view = ActiveView::AuditReport;
        app.audit.alerts = vec![alert(Critical), alert(High), alert(High), alert(Warn)];
        app.audit.progress = Some("scanning 3/9".into());
        app.audit.filter_severity = Some(High);
        let text = header_text(&app);
        assert!(text.contains("lgtm audit"));
        assert!(text.contains("scanning 3/9"));
        assert!(text.contains("Total: 4 issue(s)"));
        assert!(text.contains("1 Crit"));
        assert!(text.contains("2 High"));
        assert!(text.contains("1 Warn"));
        assert!(text.contains("High+ [2]"));
        assert!(text.contains("Scanned: 0 files"));
        assert!(text.contains("[w] Export lgtm-report.md"));

        app.audit.filter_severity = Some(Critical);
        app.audit.progress = None;
        let text = header_text(&app);
        assert!(text.contains("Crit [1]"));
        assert!(!text.contains("scanning"));
    }

    #[test]
    fn git_header_shows_turn_counts_and_badges() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        app.view_data.turn = 4;
        app.snapshot.total_additions = 12;
        app.snapshot.total_deletions = 3;
        let text = header_text(&app);
        assert!(text.contains("Turn #4"));
        assert!(text.contains("+12 -3 in 0 changed file(s)"));
        assert!(text.contains("Target: "));
        assert!(!text.contains("alerts"));
        assert!(!text.contains("comments"));
        assert!(!text.contains("GATE"));

        app.alerts = vec![alert(crate::guard::Severity::Warn)];
        app.hidden.borrow_mut().suppressed = vec![alert(crate::guard::Severity::Warn)];
        app.view_data.gate = Some(("waiting".into(), "msg".into()));
        let text = header_text(&app);
        assert!(text.contains("1 alerts (w)"));
        assert!(text.contains("+1 suppressed, 0 baselined"));
        assert!(text.contains("GATE WAITING"));

        app.view_data.gate = Some(("approved".into(), "msg".into()));
        assert!(!header_text(&app).contains("GATE WAITING"));
    }

    #[test]
    fn header_shows_pr_ci_and_orphaned_comments_and_cache_refreshes() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        app.pr.current_number = Some(7);
        app.ci = Some(lgtm_core::github::CiSummary {
            passed: 5,
            failed: 1,
            pending: 2,
        });
        let text = header_text(&app);
        assert!(text.contains("PR #7"));
        assert!(text.contains("CI ✓5 ✗1"));
        assert!(text.contains("2 | "));
        assert!(!text.contains("Turn #"));
        // second draw hits the cached line
        assert_eq!(header_text(&app), text);

        let mut c = lgtm_core::comment::CommentAnchor {
            id: "1".into(),
            file_path: "a.rs".into(),
            target_line_no: 1,
            target_code: String::new(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "x".into(),
            replies: vec![],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            orphaned: true,
            track: Default::default(),
        };
        app.comments = vec![c.clone()];
        let text = header_text(&app);
        assert!(text.contains("1 comments (1 ⚠ orphaned)"));
        c.orphaned = false;
        app.comments = vec![c];
        assert!(header_text(&app).contains("1 comments |"));
    }

    #[test]
    fn worktree_badge_reports_branch_and_detached_head() {
        let dir = tempdir().unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(args)
                .current_dir(dir.path())
                .env_remove("GIT_DIR")
                .env_remove("GIT_INDEX_FILE")
                .env_remove("GIT_WORK_TREE")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?} failed");
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        };
        git(&["init", "-q"]);
        git(&["commit", "-q", "--allow-empty", "-m", "init"]);
        let oid = git(&["rev-parse", "HEAD"]);
        let (_tx, rx) = crossbeam_channel::unbounded();
        let app = App::new(dir.path().to_path_buf(), rx, None, None, None);
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let badge = format_worktree_badge(&app);
        assert!(badge.starts_with(&format!(" [wt: {name} (")) && !badge.contains("detached"));

        git(&["checkout", "-q", "--detach"]);
        let badge = format_worktree_badge(&app);
        assert_eq!(badge, format!(" [wt: {name} (detached@{})] |", &oid[..7]));
    }
}
