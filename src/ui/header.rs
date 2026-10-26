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
}
