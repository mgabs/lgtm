use super::*;

pub(crate) fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    if app.active_view == ActiveView::CommandLine {
        let mut spans = vec![Span::styled(
            ":",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )];

        let chars: Vec<char> = app.command.input.chars().collect();
        let cursor = app.command.cursor.min(chars.len());

        let before: String = chars[..cursor].iter().collect();
        if !before.is_empty() {
            spans.push(Span::styled(
                before,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if cursor < chars.len() {
            let cursor_char = chars[cursor].to_string();
            spans.push(Span::styled(
                cursor_char,
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            let after: String = chars[(cursor + 1)..].iter().collect();
            if !after.is_empty() {
                spans.push(Span::styled(
                    after,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        } else {
            spans.push(Span::styled("█ ", Style::default().fg(Color::Yellow)));
        }

        if !app.command.completions.is_empty() {
            spans.push(Span::styled(" ❮ ", Style::default().fg(Color::DarkGray)));
            for (idx, completion) in app.command.completions.iter().enumerate() {
                let is_active = app.command.completion_idx == Some(idx);
                if is_active {
                    spans.push(Span::styled(
                        format!(" {} ", completion),
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(
                        format!(" {} ", completion),
                        Style::default().fg(Color::Cyan),
                    ));
                }
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                "❯ (Tab/S-Tab cycle)",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let p = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(p, area);
        return;
    }

    if app.visual_mode {
        let status_line = Line::from(vec![
            Span::styled(
                " -- VISUAL LINE SELECTION -- ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled(
                "[s]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Stage Selection | "),
            Span::styled(
                "[u]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Unstage | "),
            Span::styled(
                "[x]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Discard | "),
            Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
            Span::raw(" Expand | "),
            Span::styled("[Esc/V]", Style::default().fg(Color::Cyan)),
            Span::raw(" Cancel"),
        ]);
        frame.render_widget(Paragraph::new(status_line), area);
        return;
    }

    if app.active_view == ActiveView::AuditReport {
        let msg = app.status_message.as_deref().unwrap_or("Audit Mode");
        let focus_tag = match app.audit.focus {
            crate::app::AuditPaneFocus::Files => "Files",
            crate::app::AuditPaneFocus::Issues => "Issues",
            crate::app::AuditPaneFocus::Inspector => "Inspector",
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", msg),
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Pane: "),
            Span::styled(
                format!("[{}]", focus_tag),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("[Tab/h/l]", Style::default().fg(Color::Cyan)),
            Span::raw(" Focus | "),
            Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
            Span::raw(" Nav | "),
            Span::styled("[Space/Enter]", Style::default().fg(Color::Cyan)),
            Span::raw(" Filter/Action | "),
            Span::styled("[Z/m]", Style::default().fg(Color::Cyan)),
            Span::raw(" Maximize | "),
            Span::styled("[b]", Style::default().fg(Color::Cyan)),
            Span::raw(" Sidebar | "),
            Span::styled("[w/W]", Style::default().fg(Color::Cyan)),
            Span::raw(" Next/Prev Issue | "),
            Span::styled("[[]/[]]", Style::default().fg(Color::Cyan)),
            Span::raw(" File | "),
            Span::styled("[1-3/a]", Style::default().fg(Color::Cyan)),
            Span::raw(" Sev | "),
            Span::styled("[e]", Style::default().fg(Color::Cyan)),
            Span::raw(" Edit | "),
            Span::styled("[y]", Style::default().fg(Color::Cyan)),
            Span::raw(" Yank | "),
            Span::styled("[E]", Style::default().fg(Color::Green)),
            Span::raw(" Export | "),
            Span::styled("[?]", Style::default().fg(Color::Cyan)),
            Span::raw(" Help | "),
            Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
            Span::raw(" Diff | "),
            Span::styled("[q]", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ]);
        frame.render_widget(Paragraph::new(status_line), area);
        return;
    }

    let msg = app.status_message.as_deref().unwrap_or("Watching");

    // Working directory (bottom-left)
    let cwd_display = app
        .git_engine
        .repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| app.git_engine.repo_path.display().to_string());

    // Context percentage (bottom-right): position within current file diff lines
    let total_lines = app.current_file_line_count();
    let pct_str = if total_lines == 0 {
        "Bot".to_string()
    } else if app.cursor_line_idx == 0 {
        "Top".to_string()
    } else if app.cursor_line_idx + 1 >= total_lines {
        "Bot".to_string()
    } else {
        let pct = ((app.cursor_line_idx + 1) * 100) / total_lines;
        format!("{:>3}%", pct)
    };

    let left_spans = vec![
        Span::styled(
            format!(" 📁 {} ", cwd_display),
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", msg),
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled("[s/u/x]", Style::default().fg(Color::Cyan)),
        Span::raw(" Hunk | "),
        Span::styled("[e]", Style::default().fg(Color::Cyan)),
        Span::raw(" Edit | "),
        Span::styled("[d]", Style::default().fg(Color::Cyan)),
        Span::raw(" Split | "),
        Span::styled("[:]", Style::default().fg(Color::Cyan)),
        Span::raw(" Cmd | "),
        Span::styled("[/]", Style::default().fg(Color::Cyan)),
        Span::raw(" Search | "),
        Span::styled("[c]", Style::default().fg(Color::Cyan)),
        Span::raw(" Comment | "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" Review | "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)),
        Span::raw(" Help | "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit"),
    ];

    let right_spans = vec![
        Span::raw(" "),
        Span::styled(
            format!(" 📍 {} ", pct_str),
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let right_len: usize = right_spans.iter().map(|s| s.content.chars().count()).sum();
    let left_width = (area.width as usize).saturating_sub(right_len);
    let mut row_spans = crate::ui::diff::pad_or_truncate_spans(left_spans, left_width);
    row_spans.extend(right_spans);

    frame.render_widget(Paragraph::new(Line::from(row_spans)), area);
}

pub(crate) fn render_search_input(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup_area);

    let input = Paragraph::new(format!("/{}", app.search_query.as_str())).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search in Diff (Enter to search, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_status_bar_command_line_cursor_at_end() {
        let mut app = crate::app::test_util::create_test_app();
        app.active_view = ActiveView::CommandLine;
        app.command.set_text("audit".into());
        assert_eq!(app.command.cursor, 5);

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_status_bar(f, f.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(0, 0).symbol(), ":");
        assert_eq!(buffer.get(1, 0).symbol(), "a");
        assert_eq!(buffer.get(5, 0).symbol(), "t");
        assert_eq!(buffer.get(6, 0).symbol(), "█");
    }

    #[test]
    fn test_render_status_bar_command_line_cursor_mid_string() {
        let mut app = crate::app::test_util::create_test_app();
        app.active_view = ActiveView::CommandLine;
        app.command.set_text("audit".into());
        app.command.cursor = 2; // at 'd'

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_status_bar(f, f.area(), &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(0, 0).symbol(), ":");
        assert_eq!(buffer.get(1, 0).symbol(), "a");
        assert_eq!(buffer.get(2, 0).symbol(), "u");
        let cell = buffer.get(3, 0);
        assert_eq!(cell.symbol(), "d");
        assert_eq!(cell.bg, Color::Yellow);
        assert_eq!(cell.fg, Color::Black);
        assert_eq!(buffer.get(4, 0).symbol(), "i");
        assert_eq!(buffer.get(5, 0).symbol(), "t");
    }
}
