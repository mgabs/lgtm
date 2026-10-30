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

#[cfg(test)]
mod tests {
    use super::*;
    use lgtm_core::comment::CommentAnchor;
    use lgtm_core::guard::{GuardAlert, Severity};
    use lgtm_core::outcome::{Claim, Outcome};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

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

    fn comment(text: &str, line: u32) -> CommentAnchor {
        CommentAnchor {
            id: format!("c{line}"),
            file_path: "src/a.rs".into(),
            target_line_no: line,
            target_code: "x".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: text.into(),
            replies: vec![],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        }
    }

    #[test]
    fn input_without_target_shows_generic_title_and_text() {
        let mut app = crate::app::test_util::create_test_app();
        app.comment_input = "tighten this".into();
        let out = draw(&app, render_comment_input);
        assert!(out.contains("Add Review Directive"));
        assert!(out.contains("tighten this"));
        assert!(out.contains(".lgtm/directives.jsonl"));
    }

    #[test]
    fn input_on_finding_shows_location_and_severity() {
        let mut app = crate::app::test_util::create_test_app();
        let mut alert = GuardAlert {
            rule_id: "r".into(),
            fingerprint: "f".into(),
            severity: Severity::Critical,
            title: "Hardcoded secret".into(),
            description: "rotate it".into(),
            line_no: Some(7),
            file_path: "src/a.rs".into(),
        };
        app.comment_target_alert = Some(alert.clone());
        let out = draw(&app, render_comment_input);
        assert!(out.contains("src/a.rs:7"));
        assert!(out.contains("CRITICAL"));
        assert!(out.contains("Finding: Hardcoded secret - rotate it"));

        alert.line_no = None;
        app.comment_target_alert = Some(alert);
        let out = draw(&app, render_comment_input);
        assert!(out.contains("[src/a.rs"));
    }

    #[test]
    fn reply_shows_target_state_or_placeholder() {
        let mut app = crate::app::test_util::create_test_app();
        app.comment_reply_input = "will do".into();
        assert!(draw(&app, render_comment_reply).contains("No directive selected"));

        app.comments = vec![comment("use a map", 3), comment("done already", 4)];
        app.comments[1].is_resolved = true;
        app.comment_reply_target_idx = Some(9);
        assert!(draw(&app, render_comment_reply).contains("No directive selected"));

        app.comment_reply_target_idx = Some(0);
        let out = draw(&app, render_comment_reply);
        assert!(out.contains("src/a.rs:3"));
        assert!(out.contains("Open]"));
        assert!(out.contains("Directive: use a map"));
        assert!(out.contains("will do"));

        app.comment_reply_target_idx = Some(1);
        assert!(draw(&app, render_comment_reply).contains("[✓ Resolved]"));
    }

    #[test]
    fn list_is_empty_state_without_comments() {
        let app = crate::app::test_util::create_test_app();
        let out = draw(&app, render_comment_list);
        assert!(out.contains("No active review directives"));
        assert!(out.contains("Esc to close"));
    }

    #[test]
    fn list_shows_badges_replies_and_verdicts() {
        let mut app = crate::app::test_util::create_test_app();
        let mut orphan = comment("orphan one", 1);
        orphan.orphaned = true;
        let mut resolved = comment("resolved one", 2);
        resolved.is_resolved = true;
        let mut replied = comment("one reply", 3);
        replied.replies = vec!["r".into()];
        let mut many = comment("two replies", 4);
        many.replies = vec!["r".into(), "s".into()];
        let mut claimed = comment("claimed one", 5);
        claimed.track.outcome = Some(Outcome::Untouched);
        claimed.track.claim = Some(Claim {
            note: "fixed".into(),
            turn: 1,
        });
        let mut changed = comment("changed one", 6);
        changed.track.outcome = Some(Outcome::Changed);
        app.comments = vec![orphan, resolved, replied, many, claimed, changed];
        app.comment_list_selected_idx = 2;

        let out = draw(&app, render_comment_list);
        assert!(out.contains("[⚠ Orphaned] src/a.rs:1 orphan one"));
        assert!(out.contains("[✓ Resolved] src/a.rs:2 resolved one"));
        assert!(out.contains("one reply (1 reply)"));
        assert!(out.contains("two replies (2 replies)"));
        assert!(out.contains("[claimed but not changed]"));
        assert!(out.contains("[changed]"));
        assert!(out.contains("Review Directives [6]"));
        assert!(out.contains("1 untouched"));
    }
}
