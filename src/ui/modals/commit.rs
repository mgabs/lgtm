use super::super::*;

pub(crate) fn render_commit_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(65, 45, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Staged files summary
            Constraint::Length(3), // Commit message input box
            Constraint::Min(2),    // Help & shortcuts
        ])
        .split(popup_area);

    let staged_count = app.view_data.staged_count;
    let staged_summary = vec![
        Line::from(vec![
            Span::styled("Staged Files: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} file(s) ready to commit", staged_count),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Author: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "lgtm Interactive Reviewer",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let summary_widget = Paragraph::new(staged_summary).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Commit Forge ")
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(summary_widget, chunks[0]);

    let input = Paragraph::new(app.commit_message_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Commit Message (Enter: Commit | Ctrl-s: Commit | Esc: Cancel) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input, chunks[1]);

    let help = Paragraph::new(
        "Tip: Use conventional commit format (feat: ..., fix: ..., chore: ...)\nPress Enter or Ctrl-s to commit staged changes directly to git.",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
