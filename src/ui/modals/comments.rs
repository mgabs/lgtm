use super::super::*;

pub(crate) fn render_comment_input(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(65, 30, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(popup_area);

    let title = if let Some(ref alert) = app.comment_target_alert {
        let loc = match alert.line_no {
            Some(l) => format!("{}:{}", alert.file_path, l),
            None => alert.file_path.clone(),
        };
        format!(
            " Add Directive on Finding [{} {}] (Enter to save, Esc to cancel) ",
            loc,
            alert.severity.badge()
        )
    } else {
        " Add Review Directive (Enter to save, Esc to cancel) ".to_string()
    };

    let input = Paragraph::new(app.comment_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(input, chunks[0]);

    let help_text = if let Some(ref alert) = app.comment_target_alert {
        format!(
            "Finding: {} - {}. Directive will be saved to .lgtm/directives.jsonl.",
            alert.title, alert.description
        )
    } else {
        "Comment will be context-anchored in .lgtm/directives.jsonl. Press Ctrl-s in diff view to compile into agent prompt.".to_string()
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[1]);
}

pub(crate) fn render_comment_reply(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(65, 40, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Target comment context
            Constraint::Length(3), // Reply input box
            Constraint::Min(2),    // Help text
        ])
        .split(popup_area);

    let target_info = if let Some(idx) = app.comment_reply_target_idx {
        if let Some(comment) = app.comments.get(idx) {
            vec![
                Line::from(vec![
                    Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}:{}", comment.file_path, comment.target_line_no),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    if comment.is_resolved {
                        Span::styled(
                            "[✓ Resolved]",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled(
                            "[💬 Open]",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        )
                    },
                ]),
                Line::from(vec![
                    Span::styled("Directive: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&comment.comment_text, Style::default().fg(Color::White)),
                ]),
            ]
        } else {
            vec![Line::from("No directive selected")]
        }
    } else {
        vec![Line::from("No directive selected")]
    };

    let context_widget = Paragraph::new(target_info).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Replying to Directive ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(context_widget, chunks[0]);

    let input = Paragraph::new(app.comment_reply_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Reply Message (Enter to save, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(input, chunks[1]);

    let help = Paragraph::new(
        "Reply will be threaded under this directive and exported to agent prompt on Ctrl-s.",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

pub(crate) fn render_comment_list(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(76, 65, area);
    frame.render_widget(Clear, popup_area);

    if app.comments.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (No active review directives or comments saved)",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("  Press `c` or `<Space>c` on any diff line to anchor an inline directive."),
            Line::from("  Press `Esc` or `q` to close."),
        ];
        let p = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Review Directives (Esc to close) ")
                .border_style(Style::default().fg(Color::Magenta)),
        );
        frame.render_widget(p, popup_area);
        return;
    }

    let items: Vec<ListItem> = app
        .comments
        .iter()
        .enumerate()
        .map(|(idx, comment)| {
            let is_selected = idx == app.comment_list_selected_idx;
            let status_badge = if comment.orphaned {
                Span::styled(
                    "[⚠ Orphaned] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            } else if comment.is_resolved {
                Span::styled(
                    "[✓ Resolved] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "[💬 Open] ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let replies_tag = if !comment.replies.is_empty() {
                format!(
                    " ({} {})",
                    comment.replies.len(),
                    if comment.replies.len() == 1 {
                        "reply"
                    } else {
                        "replies"
                    }
                )
            } else {
                "".to_string()
            };

            let mut spans = vec![
                status_badge,
                Span::styled(
                    format!("{}:{} ", comment.file_path, comment.target_line_no),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&comment.comment_text),
            ];

            if !replies_tag.is_empty() {
                spans.push(Span::styled(
                    replies_tag,
                    Style::default().fg(Color::LightCyan),
                ));
            }

            let verdict = lgtm_core::outcome::verdict(comment);
            if !comment.is_resolved && verdict != "sent" {
                let warn = verdict == "claimed but not changed";
                spans.push(Span::styled(
                    format!(" [{}]", verdict),
                    Style::default().fg(if warn { Color::Red } else { Color::Yellow }),
                ));
            }

            let style = if is_selected {
                Style::default()
                    .bg(Color::Magenta)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Review Directives [{}] {} (Enter: Jump | r: Reply | v/Space: Resolve | a: Action | d: Delete | Esc: Close) ",
                app.comments.len(),
                lgtm_core::outcome::Summary::of(&app.comments).line()
            ))
            .border_style(Style::default().fg(Color::Magenta)),
    );

    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.comment_list_selected_idx));
    frame.render_stateful_widget(list, popup_area, &mut state);
}
