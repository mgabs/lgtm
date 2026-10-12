use super::*;

pub(crate) fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    if app.active_view == ActiveView::CommandLine {
        let mut spans = vec![
            Span::styled(
                ":",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.command_input,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█ ", Style::default().fg(Color::Yellow)),
        ];

        if !app.command_completions.is_empty() {
            spans.push(Span::styled(" ❮ ", Style::default().fg(Color::DarkGray)));
            for (idx, completion) in app.command_completions.iter().enumerate() {
                let is_active = app.command_completion_idx == Some(idx);
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
        let focus_tag = match app.audit_focus {
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
    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", msg),
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled("[V]", Style::default().fg(Color::Cyan)),
        Span::raw(" Visual | "),
        Span::styled("[s/u/x]", Style::default().fg(Color::Cyan)),
        Span::raw(" Hunk | "),
        Span::styled("[d]", Style::default().fg(Color::Cyan)),
        Span::raw(" Split | "),
        Span::styled("[:]", Style::default().fg(Color::Cyan)),
        Span::raw(" Cmd | "),
        Span::styled("[/]", Style::default().fg(Color::Cyan)),
        Span::raw(" Search | "),
        Span::styled("[n/N]", Style::default().fg(Color::Cyan)),
        Span::raw(" Match | "),
        Span::styled("[gg/G]", Style::default().fg(Color::Cyan)),
        Span::raw(" Top/End | "),
        Span::styled("[Ctrl-d/u]", Style::default().fg(Color::Cyan)),
        Span::raw(" Page | "),
        Span::styled("[y]", Style::default().fg(Color::Cyan)),
        Span::raw(" Yank | "),
        Span::styled("[c]", Style::default().fg(Color::Cyan)),
        Span::raw(" Comment | "),
        Span::styled("[Ctrl-s]", Style::default().fg(Color::Cyan)),
        Span::raw(" Send | "),
        Span::styled("[w]", Style::default().fg(Color::Cyan)),
        Span::raw(" Alerts | "),
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Ref | "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)),
        Span::raw(" Help | "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit"),
    ]);

    frame.render_widget(Paragraph::new(status_line), area);
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
