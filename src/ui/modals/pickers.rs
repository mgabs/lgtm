use super::super::*;

pub(crate) fn render_file_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(popup_area);

    let input = Paragraph::new(app.file_picker.query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Jump to File (Type to filter, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .file_picker
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(list_idx, &file_idx)| {
            let file = &app.snapshot.files[file_idx];
            let is_selected = list_idx == app.file_picker.selected_idx;

            let file_alerts_count = app
                .alerts
                .iter()
                .filter(|a| a.file_path == file.display_name())
                .count();
            let alert_tag = if file_alerts_count > 0 {
                format!(" ⚠️{}", file_alerts_count)
            } else {
                "".to_string()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", file.change_type.badge()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(file.display_name()),
                Span::styled(
                    alert_tag,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" (+{}/-{})", file.additions, file.deletions),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, chunks[1]);
}

pub(crate) fn render_pane_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .pane_picker
        .list
        .iter()
        .enumerate()
        .map(|(idx, pane)| {
            let is_selected = idx == app.pane_picker.selected_idx;
            let active_str = if pane.active { " (current)" } else { "" };
            let line = Line::from(vec![
                Span::styled(
                    &pane.id,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(": {}{}", pane.title, active_str)),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let title = format!(
        " Select Target Agent Pane [{}] (Enter to select, Esc to close) ",
        app.resolved_sink_kind()
    );

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(list, popup_area);
}

pub(crate) fn render_pr_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(75, 65, area);
    frame.render_widget(Clear, popup_area);

    if app.pr.list.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading, or no matching pull requests. Check the status bar for errors.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("  Filters: 1 Open | 2 Closed | 3 All (closed includes merged/completed)."),
            Line::from("  Press `Esc` or `q` to close."),
        ];
        let p = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {} pull requests (1 Open | 2 Closed | 3 All) ",
                    app.pr.filter.as_str()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        );
        frame.render_widget(p, popup_area);
        return;
    }

    let items: Vec<ListItem> = app
        .pr
        .list
        .iter()
        .enumerate()
        .map(|(idx, pr)| {
            let is_selected = idx == app.pr.selected_idx;
            let line = Line::from(vec![
                Span::styled(
                    format!("#{} ", pr.number),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{} ", pr.title)),
                Span::styled(
                    format!("by @{} ", pr.author.login),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("(+{}/-{}) ", pr.additions, pr.deletions),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("[{}] ({})", pr.head_ref_name, pr.state),
                    Style::default().fg(Color::Cyan),
                ),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " {} PRs [{}] (1 Open | 2 Closed | 3 All | Enter: Load) ",
                app.pr.filter.as_str(),
                app.pr.list.len()
            ))
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(list, popup_area);
}

pub(crate) fn render_worktree_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(70, 50, area);
    frame.render_widget(Clear, popup_area);

    if app.worktree_picker.list.is_empty() {
        let p = Paragraph::new("No active worktrees discovered. Press Esc to close.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Git Worktrees ")
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(p, popup_area);
        return;
    }

    let items: Vec<ListItem> = app
        .worktree_picker
        .list
        .iter()
        .enumerate()
        .map(|(idx, wt)| {
            let is_selected = idx == app.worktree_picker.selected_idx;
            let current_badge = if wt.is_current { "* " } else { "  " };
            let branch_label = wt.branch.as_deref().unwrap_or("(detached HEAD)");
            let short_head = if wt.head.len() >= 7 {
                &wt.head[..7]
            } else {
                &wt.head
            };
            let name = wt.path.file_name().and_then(|f| f.to_str()).unwrap_or(".");

            let line = Line::from(vec![
                Span::styled(
                    current_badge,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}] ", name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<20} ", branch_label),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("@{} ", short_head),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(wt.path.display().to_string()),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Git Worktrees [{}] (j/k: navigate, Enter: switch, Esc: close) ",
                app.worktree_picker.list.len()
            ))
            .border_style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(list, popup_area);
}

pub(crate) fn render_worktree_confirm_guard(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 25, area);
    frame.render_widget(Clear, popup_area);

    let target_name = app
        .worktree_picker
        .pending_target
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or("target worktree");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Switching to: {}", target_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "You have pending unexported review directives in this worktree.",
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[s] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Save directives to disk and switch    "),
            Span::styled(
                "[d] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Discard directives and switch"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Esc / c] ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cancel switch and remain in current worktree"),
        ]),
    ];

    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ⚠ Pending Review Directives ")
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(p, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, FileDiff};
    use lgtm_core::git::WorktreeInfo;
    use lgtm_core::guard::{GuardAlert, Severity};
    use lgtm_core::ipc::TmuxPane;
    use lgtm_core::pr::{PrAuthor, PullRequestState, PullRequestSummary};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn draw(app: &App, f: fn(&mut Frame, Rect, &App)) -> String {
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|fr| f(fr, fr.area(), app)).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn file(name: &str, adds: usize, dels: usize) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: PathBuf::from(name),
            change_type: ChangeType::Modified,
            additions: adds,
            deletions: dels,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn file_picker_lists_filtered_files_with_alert_counts() {
        let mut app = crate::app::test_util::create_test_app();
        app.snapshot.files = vec![file("src/a.rs", 3, 1), file("src/b.rs", 0, 5)];
        app.alerts = vec![GuardAlert {
            rule_id: "r".into(),
            fingerprint: "f".into(),
            severity: Severity::Warn,
            title: "t".into(),
            description: "d".into(),
            line_no: None,
            file_path: "src/b.rs".into(),
        }];
        app.file_picker.query = "src".into();
        app.file_picker.filtered_indices = vec![0, 1];
        app.file_picker.selected_idx = 1;
        let out = draw(&app, render_file_picker);
        assert!(out.contains("Jump to File"));
        assert!(out.contains("src/a.rs"));
        assert!(out.contains("(+3/-1)"));
        let line_of = |name: &str| out.lines().find(|l| l.contains(name)).unwrap();
        assert!(line_of("src/b.rs").contains("⚠") && line_of("src/b.rs").contains("(+0/-5)"));
        assert!(!line_of("src/a.rs").contains("⚠"));
    }

    #[test]
    fn pane_picker_marks_current_pane() {
        let mut app = crate::app::test_util::create_test_app();
        app.pane_picker.list = vec![
            TmuxPane {
                id: "%1".into(),
                title: "claude".into(),
                active: true,
            },
            TmuxPane {
                id: "%2".into(),
                title: "shell".into(),
                active: false,
            },
        ];
        let out = draw(&app, render_pane_picker);
        assert!(out.contains("Select Target Agent Pane"));
        assert!(out.contains("%1: claude (current)"));
        assert!(out.contains("%2: shell"));
        assert!(!out.contains("shell (current)"));
    }

    #[test]
    fn pr_picker_empty_and_populated() {
        let mut app = crate::app::test_util::create_test_app();
        app.pr.filter = PullRequestState::Closed;
        let out = draw(&app, render_pr_picker);
        assert!(out.contains("closed pull requests"));
        assert!(out.contains("no matching pull requests"));

        app.pr.list = vec![PullRequestSummary {
            number: 42,
            title: "Fix bug".into(),
            author: PrAuthor {
                login: "alice".into(),
            },
            head_ref_name: "fix/bug".into(),
            additions: 10,
            deletions: 2,
            state: "OPEN".into(),
        }];
        let out = draw(&app, render_pr_picker);
        assert!(out.contains("closed PRs [1]"));
        assert!(out.contains("#42 Fix bug by @alice (+10/-2) [fix/bug] (OPEN)"));
    }

    #[test]
    fn worktree_picker_empty_and_populated() {
        let mut app = crate::app::test_util::create_test_app();
        assert!(draw(&app, render_worktree_picker).contains("No active worktrees discovered"));

        app.worktree_picker.list = vec![
            WorktreeInfo {
                path: PathBuf::from("/repos/main"),
                head: "abcdef1234567".into(),
                branch: Some("main".into()),
                is_bare: false,
                is_current: true,
            },
            WorktreeInfo {
                path: PathBuf::from("/repos/wip"),
                head: "abc".into(),
                branch: None,
                is_bare: false,
                is_current: false,
            },
        ];
        let out = draw(&app, render_worktree_picker);
        assert!(out.contains("Git Worktrees [2]"));
        assert!(out.contains("* [main] main"));
        assert!(out.contains("@abcdef1"));
        assert!(!out.contains("@abcdef12"));
        assert!(out.contains("[wip] (detached HEAD)"));
        assert!(out.contains("@abc /repos/wip"));
    }

    #[test]
    fn worktree_guard_names_target() {
        let mut app = crate::app::test_util::create_test_app();
        assert!(draw(&app, render_worktree_confirm_guard).contains("Switching to: target worktree"));

        app.worktree_picker.pending_target = Some(PathBuf::from("/repos/feature-x"));
        let out = draw(&app, render_worktree_confirm_guard);
        assert!(out.contains("Switching to: feature-x"));
        assert!(out.contains("pending unexported review directives"));
        assert!(out.contains("[Esc / c]"));
    }
}
