use crate::app::App;
use crate::ui::centered_rect;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub(crate) fn render_threads_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(84, 80, area);
    frame.render_widget(Clear, popup_area);

    if app.threads.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (No remote review threads found or still loading...)",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("  PR threads load in background. Run `:threads` to refresh manually."),
            Line::from("  Ensure `gh` (GitHub) or `az` (Azure DevOps) CLI is authenticated."),
            Line::from("  Press `Esc` or `q` to close."),
        ];
        let p = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Remote PR Review Threads (Esc to close) ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(p, popup_area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(popup_area);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[0]);

    let open_count = app.threads.iter().filter(|t| !t.resolved).count();

    // Left pane: Thread list
    let items: Vec<ListItem> = app
        .threads
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let is_selected = idx == app.thread_selected_idx;
            let (marker, marker_color) = if t.resolved {
                ("◇", Color::Green)
            } else {
                ("◆", Color::Cyan)
            };
            let line_str = match t.line {
                Some(l) => format!("{}:{}", t.path, l),
                None => t.path.clone(),
            };
            let count_str = format!(
                "({}💬) {}",
                t.comments.len(),
                app.thread_pending_label(&t.id).unwrap_or("")
            );
            let author = t
                .comments
                .first()
                .map(|c| c.author.as_str())
                .unwrap_or("unknown");

            let line = Line::from(vec![
                Span::styled(
                    format!("{marker} "),
                    Style::default()
                        .fg(marker_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("#{:<2} ", idx + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{line_str} "),
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(format!("@{author} "), Style::default().fg(Color::Yellow)),
                Span::styled(count_str, Style::default().fg(Color::DarkGray)),
            ]);

            let item_style = if is_selected {
                Style::default().bg(Color::Rgb(20, 50, 70))
            } else {
                Style::default()
            };

            ListItem::new(line).style(item_style)
        })
        .collect();

    let list_title = format!(
        " Threads ({} total, {} open) ",
        app.threads.len(),
        open_count
    );
    let list_widget = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(list_title)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list_widget, main_chunks[0]);

    // Right pane: Selected thread conversation with full comment bodies
    if let Some(t) = app.threads.get(app.thread_selected_idx) {
        let (status_text, status_color) = if t.resolved {
            ("✓ Resolved", Color::Green)
        } else {
            ("● Open", Color::Cyan)
        };
        let loc = match t.line {
            Some(l) => format!("{}:{}", t.path, l),
            None => t.path.clone(),
        };
        let detail_title = format!(
            " #{}: {} • {} ",
            app.thread_selected_idx + 1,
            loc,
            app.thread_pending_label(&t.id).unwrap_or(status_text)
        );

        let mut lines = Vec::new();
        if t.comments.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  (No comments in this thread)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (c_idx, comment) in t.comments.iter().enumerate() {
                if c_idx > 0 {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  ────────────────────────────────────────────────────────",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  @{}", comment.author),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  (id: {})", comment.id),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                lines.push(Line::from(""));
                for body_line in comment.body.lines() {
                    lines.push(Line::from(Span::raw(format!("    {}", body_line))));
                }
            }
        }

        let conversation_widget = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(detail_title)
                    .border_style(Style::default().fg(status_color)),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.thread_scroll_offset as u16, 0));
        frame.render_widget(conversation_widget, main_chunks[1]);
    }

    // Footer bar
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " [j/k]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Nav Threads  "),
        Span::styled(
            "[J/K]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Scroll  "),
        Span::styled(
            "[Enter]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Jump to line  "),
        Span::styled(
            "[v/R]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Toggle resolve  "),
        Span::styled(
            "[r]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Reply  "),
        Span::styled("[Esc/q]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Close"),
    ]))
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, chunks[1]);
}
