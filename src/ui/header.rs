use super::*;

pub(crate) fn render_header(frame: &mut Frame, area: Rect, app: &App) {
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
        let header_line = Line::from(vec![
            Span::styled(
                " lgtm audit ",
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
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
        frame.render_widget(Paragraph::new(header_line), area);
        return;
    }

    if !app.git_engine.is_git_repo() && app.pr.current_number.is_none() {
        let header_line = Line::from(vec![
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
        frame.render_widget(Paragraph::new(header_line), area);
        return;
    }

    let agent_str = format!(" Target: {} ", app.agent_sink().label());

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
        let turn = app.turn_manager.load_state().current_turn;
        header_spans.push(Span::raw(format!(" Turn #{} | ", turn)));
    }

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

    if let Some((status, _msg)) = app.check_gate_status() {
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

    frame.render_widget(Paragraph::new(Line::from(header_spans)), area);
}
