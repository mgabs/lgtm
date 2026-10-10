use crate::app::{ActiveView, App};
use crate::model::{ChangeType, LineType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Main layout: Header (1 line), Body (rest), Status bar (1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top bar
            Constraint::Min(5),    // Main workspace
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);

    // Render active overlay modal if needed
    match app.active_view {
        ActiveView::FilePicker => render_file_picker(frame, size, app),
        ActiveView::PanePicker => render_pane_picker(frame, size, app),
        ActiveView::CommentInput => render_comment_input(frame, size, app),
        ActiveView::CommentList => render_comment_list(frame, size, app),
        ActiveView::PrPicker => render_pr_picker(frame, size, app),
        ActiveView::Help => render_help_modal(frame, size, app),
        ActiveView::Search => render_search_input(frame, size, app),
        ActiveView::CommandLine => {
            if !app.command_completions.is_empty() {
                render_command_completion_column(frame, size, app);
            }
        }
        ActiveView::Diff => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    if !app.git_engine.is_git_repo() && app.current_pr_number.is_none() {
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

    let agent_str = match &app.agent_pane {
        Some(pane) => format!(" Target Pane: {} ", pane),
        None => " Target: Clipboard (OSC 52) ".to_string(),
    };

    let total_add = app.snapshot.total_additions;
    let total_del = app.snapshot.total_deletions;

    let mut header_spans = vec![Span::styled(
        " lgtm ",
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(pr_num) = app.current_pr_number {
        header_spans.push(Span::styled(
            format!(" PR #{} | ", pr_num),
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
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

    if !app.alerts.is_empty() {
        header_spans.push(Span::styled(
            format!(" ⚠️ {} alerts (w) |", app.alerts.len()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if !app.comments.is_empty() {
        header_spans.push(Span::styled(
            format!(" 💬 {} comments |", app.comments.len()),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    header_spans.push(Span::styled(
        agent_str,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    frame.render_widget(Paragraph::new(Line::from(header_spans)), area);
}

fn render_body(frame: &mut Frame, area: Rect, app: &App) {
    if app.show_file_drawer && area.width >= 50 {
        let drawer_width = if area.width >= 100 { 35 } else { 28 };
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(drawer_width), Constraint::Min(20)])
            .split(area);

        render_file_drawer(frame, body_chunks[0], app);
        render_diff_view(frame, body_chunks[1], app);
    } else {
        render_diff_view(frame, area, app);
    }
}

fn render_file_drawer(frame: &mut Frame, area: Rect, app: &App) {
    if !app.git_engine.is_git_repo() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            " (No git repository) ",
            Style::default().fg(Color::Yellow),
        )))];
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Changed Files (Tab) ")
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(list, area);
        return;
    }

    if app.snapshot.files.is_empty() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            " (Working tree clean) ",
            Style::default().fg(Color::Green),
        )))];
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Changed Files (Tab) ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .snapshot
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = idx == app.selected_file_idx;
            let checkmark = if file.is_reviewed { "✓ " } else { "  " };

            let badge_color = match file.change_type {
                ChangeType::Added => Color::Green,
                ChangeType::Deleted => Color::Red,
                ChangeType::Modified => Color::Yellow,
                ChangeType::Renamed => Color::Blue,
                ChangeType::Untracked => Color::Magenta,
            };

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

            let mut spans = vec![
                Span::styled(
                    checkmark,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}] ", file.change_type.badge()),
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(file.display_name()),
            ];

            if !alert_tag.is_empty() {
                spans.push(Span::styled(
                    alert_tag,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            if file.additions > 0 || file.deletions > 0 {
                spans.push(Span::styled(
                    format!(" +{}/-{}", file.additions, file.deletions),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let style = if is_selected {
                Style::default()
                    .bg(Color::Cyan)
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
                " Changed Files [{}] (Tab) ",
                app.snapshot.files.len()
            ))
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_diff_view(frame: &mut Frame, area: Rect, app: &App) {
    let current_file = app.snapshot.files.get(app.selected_file_idx);

    let (title, content_lines) = if !app.git_engine.is_git_repo() {
        (
            " Not a Git Repository ".to_string(),
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ⚠️  Not a git repository",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Directory: "),
                    Span::styled(
                        app.git_engine.repo_path.display().to_string(),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(""),
                Line::from(
                    "  Run `git init` in this directory to begin diff review and agent steering,",
                ),
                Line::from("  or pass a repository path via `lgtm -r /path/to/repo`."),
                Line::from(""),
                Line::from(Span::styled(
                    "  lgtm will automatically detect changes once git is initialized.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  [q] Quit lgtm",
                    Style::default().fg(Color::White),
                )),
            ],
        )
    } else {
        match current_file {
            Some(file) => {
                let file_alerts: Vec<&crate::guard::GuardAlert> = app
                    .alerts
                    .iter()
                    .filter(|a| a.file_path == file.display_name())
                    .collect();

                let alert_badge_title = if !file_alerts.is_empty() {
                    format!(" | ⚠️ {} Alert(s)", file_alerts.len())
                } else {
                    "".to_string()
                };

                let title = format!(
                    " {} (+{}/-{}){} ",
                    file.display_name(),
                    file.additions,
                    file.deletions,
                    alert_badge_title
                );
                let mut lines = Vec::new();

                // Surface file-level alerts (alerts without specific line numbers) at top
                let file_level_alerts: Vec<&&crate::guard::GuardAlert> =
                    file_alerts.iter().filter(|a| a.line_no.is_none()).collect();

                for alert in file_level_alerts {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {} ", alert.severity.badge()),
                            Style::default()
                                .bg(Color::Red)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            &alert.title,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(": "),
                        Span::styled(&alert.description, Style::default().fg(Color::White)),
                    ]));
                }
                if !file_alerts.iter().any(|a| a.line_no.is_none()) && !file_alerts.is_empty() {
                    // All alerts are line-anchored inline
                } else if file_alerts.iter().any(|a| a.line_no.is_none()) {
                    lines.push(Line::from(""));
                }

                if file.hunks.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (Binary file or empty diff) ",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    let mut line_counter = 0;
                    for hunk in &file.hunks {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", hunk.header),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::ITALIC),
                        )));

                        for dline in &hunk.lines {
                            let is_cursor = line_counter == app.cursor_line_idx;
                            let prefix = match dline.line_type {
                                LineType::Addition => "+",
                                LineType::Deletion => "-",
                                _ => " ",
                            };

                            let style = match dline.line_type {
                                LineType::Addition => Style::default().fg(Color::Green),
                                LineType::Deletion => Style::default().fg(Color::Red),
                                _ => Style::default().fg(Color::White),
                            };

                            let line_style = if is_cursor {
                                style.bg(Color::Indexed(237)).add_modifier(Modifier::BOLD)
                            } else {
                                style
                            };

                            let line_no = dline.new_line_num.or(dline.old_line_num);
                            let line_no_str = match line_no {
                                Some(num) => format!("{:4} ", num),
                                None => "     ".to_string(),
                            };

                            // Check matching line-anchored alerts
                            let matching_alerts: Vec<&&crate::guard::GuardAlert> = file_alerts
                                .iter()
                                .filter(|a| a.line_no.is_some() && a.line_no == line_no)
                                .collect();

                            // Check matching line-anchored comments
                            let matching_comments: Vec<&crate::comment::CommentAnchor> = app
                                .comments
                                .iter()
                                .filter(|c| {
                                    c.file_path == file.display_name()
                                        && Some(c.target_line_no) == line_no
                                })
                                .collect();

                            let gutter_icon = if !matching_alerts.is_empty() {
                                "⚠️ "
                            } else if !matching_comments.is_empty() {
                                "💬 "
                            } else {
                                "   "
                            };

                            let icon_style = if !matching_alerts.is_empty() {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD)
                            };

                            lines.push(Line::from(vec![
                                Span::styled(line_no_str, Style::default().fg(Color::DarkGray)),
                                Span::styled(gutter_icon, icon_style),
                                Span::styled(prefix, line_style),
                                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                                Span::styled(&dline.content, line_style),
                            ]));

                            // Render inline alerts directly beneath this code line
                            for alert in matching_alerts {
                                let sev_style = match alert.severity {
                                    crate::guard::Severity::Critical => {
                                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                                    }
                                    crate::guard::Severity::High => Style::default()
                                        .fg(Color::LightRed)
                                        .add_modifier(Modifier::BOLD),
                                    crate::guard::Severity::Warn => Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                };

                                lines.push(Line::from(vec![
                                    Span::raw("        │ "),
                                    Span::styled(
                                        format!("▲ {} {}: ", alert.severity.badge(), alert.title),
                                        sev_style,
                                    ),
                                    Span::styled(
                                        &alert.description,
                                        Style::default().fg(Color::White),
                                    ),
                                ]));
                            }

                            // Render inline comments directly beneath this code line
                            for comment in matching_comments {
                                lines.push(Line::from(vec![
                                    Span::raw("        │ "),
                                    Span::styled(
                                        "💬 DIRECTIVE: ",
                                        Style::default()
                                            .fg(Color::Magenta)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(
                                        format!("\"{}\"", comment.comment_text),
                                        Style::default().fg(Color::LightMagenta),
                                    ),
                                ]));
                            }

                            line_counter += 1;
                        }
                    }
                }
                (title, lines)
            }
            None => (
                " Working Tree Clean ".to_string(),
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ✨ Working tree is clean (no modified or untracked files)",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("  Repository: "),
                        Span::styled(
                            app.git_engine.repo_path.display().to_string(),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]),
                    Line::from(""),
                    Line::from("  lgtm is actively monitoring the workspace for real-time diffs."),
                    Line::from("  Edit files or run a coding agent turn to stream changes here."),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Key Navigation:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from("  • Tab          Toggle changed-files sidebar"),
                    Line::from("  • f / Ctrl-p   Open fuzzy file switcher"),
                    Line::from("  • ] / [        Jump to next / previous changed file"),
                    Line::from("  • c            Add review comment on current line"),
                    Line::from("  • Ctrl-s       Compile & send batch review prompt to agent"),
                    Line::from("  • w / W        Jump to next / previous safety alert"),
                    Line::from("  • Enter        Inject file:line reference into agent pane"),
                    Line::from("  • v            Mark file reviewed and fold"),
                    Line::from("  • p            Select target tmux pane"),
                    Line::from("  • q            Quit"),
                ],
            ),
        }
    };

    let paragraph = Paragraph::new(content_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_comment_input(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(65, 30, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(popup_area);

    let input = Paragraph::new(app.comment_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Add Review Directive (Enter to save, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(input, chunks[0]);

    let help = Paragraph::new("Comment will be context-anchored in .lgtm/comments.jsonl. Press Ctrl-s in diff view to compile into agent prompt.").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[1]);
}

fn render_file_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(popup_area);

    let input = Paragraph::new(app.picker_query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Jump to File (Type to filter, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .picker_filtered_indices
        .iter()
        .enumerate()
        .map(|(list_idx, &file_idx)| {
            let file = &app.snapshot.files[file_idx];
            let is_selected = list_idx == app.picker_selected_idx;

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

fn render_pane_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .pane_list
        .iter()
        .enumerate()
        .map(|(idx, pane)| {
            let is_selected = idx == app.pane_selected_idx;
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Target tmux Pane (Enter to select, Esc to close) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(list, popup_area);
}

fn render_search_input(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_comment_list(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(72, 60, area);
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
            let line = Line::from(vec![
                Span::styled(
                    format!("{}:{} ", comment.file_path, comment.target_line_no),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("💬 ", Style::default().fg(Color::Magenta)),
                Span::raw(&comment.comment_text),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::Magenta)
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
            .title(format!(" Review Directives [{}] (Enter: Jump | d: Delete | Ctrl-s: Batch Export | Esc: Close) ", app.comments.len()))
            .border_style(Style::default().fg(Color::Magenta)),
    );

    frame.render_widget(list, popup_area);
}

fn render_pr_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(75, 65, area);
    frame.render_widget(Clear, popup_area);

    if app.pr_list.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (No open pull requests found or `gh` CLI not authenticated)",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("  Ensure `gh auth login` is configured in this repository."),
            Line::from("  Press `Esc` or `q` to close."),
        ];
        let p = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" GitHub Pull Requests (Esc to close) ")
                .border_style(Style::default().fg(Color::Blue)),
        );
        frame.render_widget(p, popup_area);
        return;
    }

    let items: Vec<ListItem> = app
        .pr_list
        .iter()
        .enumerate()
        .map(|(idx, pr)| {
            let is_selected = idx == app.pr_selected_idx;
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
                    format!("[{}]", pr.head_ref_name),
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
                " Select Pull Request [{}] (Enter: Load Diff | Esc: Close) ",
                app.pr_list.len()
            ))
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(list, popup_area);
}

fn render_help_modal(frame: &mut Frame, area: Rect, _app: &App) {
    let popup_area = centered_rect(72, 85, area);
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "LGTM — Fast Diff Reviewer & Agent Steering Engine",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "VIM MOTIONS & NAVIGATION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  j / k, ↓ / ↑     ", Style::default().fg(Color::Cyan)),
            Span::raw("Move cursor line-by-line"),
        ]),
        Line::from(vec![
            Span::styled("  gg / G           ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to top / bottom of file diff"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-d / Ctrl-u  ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll half-page down / up"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-f / Ctrl-b  ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll full page down / up (or PgDn / PgUp)"),
        ]),
        Line::from(vec![
            Span::styled("  } / {            ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to next / previous paragraph (blank line)"),
        ]),
        Line::from(vec![
            Span::styled("  0 / $            ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to start / end of current line"),
        ]),
        Line::from(vec![
            Span::styled("  ]f / [f (or ]/[) ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to Next / Previous changed file"),
        ]),
        Line::from(vec![
            Span::styled("  ]h / [h          ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to Next / Previous diff hunk"),
        ]),
        Line::from(vec![
            Span::styled("  ]c / [c          ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to Next / Previous review directive / comment"),
        ]),
        Line::from(vec![
            Span::styled("  ]w / [w (or w/W) ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to Next / Previous security alert / test fraud warning"),
        ]),
        Line::from(vec![
            Span::styled("  ]u / [u          ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to Next / Previous unreviewed file"),
        ]),
        Line::from(vec![
            Span::styled("  zz / zt / zb     ", Style::default().fg(Color::Cyan)),
            Span::raw("Center view / Top of view / Bottom of view"),
        ]),
        Line::from(vec![
            Span::styled("  o / e            ", Style::default().fg(Color::Cyan)),
            Span::raw("Open current file:line in $EDITOR (nvim/vim/code)"),
        ]),
        Line::from(vec![
            Span::styled("  s                ", Style::default().fg(Color::Cyan)),
            Span::raw("Stage / Unstage current file in Git"),
        ]),
        Line::from(vec![
            Span::styled("  X                ", Style::default().fg(Color::Cyan)),
            Span::raw("Discard unstaged changes in current file"),
        ]),
        Line::from(vec![
            Span::styled("  f / Ctrl-p       ", Style::default().fg(Color::Cyan)),
            Span::raw("Fuzzy switcher to search and jump to any changed file"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "SEARCH & YANK",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  /                ", Style::default().fg(Color::Cyan)),
            Span::raw("Interactive search across all file diffs"),
        ]),
        Line::from(vec![
            Span::styled("  n / N            ", Style::default().fg(Color::Cyan)),
            Span::raw("Step to Next / Previous search match"),
        ]),
        Line::from(vec![
            Span::styled("  * / #            ", Style::default().fg(Color::Cyan)),
            Span::raw("Search identifier under cursor forward / backward"),
        ]),
        Line::from(vec![
            Span::styled("  y                ", Style::default().fg(Color::Cyan)),
            Span::raw("Yank current code line to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  <Space>y         ", Style::default().fg(Color::Cyan)),
            Span::raw("Yank #ref file:line anchor to clipboard (OSC 52)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "AGENT STEERING & IPC",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Enter            ", Style::default().fg(Color::Cyan)),
            Span::raw("Inject #ref file:line anchor into target agent tmux pane"),
        ]),
        Line::from(vec![
            Span::styled("  p (or <Space>p)  ", Style::default().fg(Color::Cyan)),
            Span::raw("Open tmux pane selector (choose target agent)"),
        ]),
        Line::from(vec![
            Span::styled("  Tab              ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle file tree drawer (left panel)"),
        ]),
        Line::from(vec![
            Span::styled("  d                ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle Unified / Side-by-Side Split Diff view"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "REVIEW DIRECTIVES & TESTING",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  c (or <Space>c)  ", Style::default().fg(Color::Cyan)),
            Span::raw("Add inline comment anchored to current file & line"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-s           ", Style::default().fg(Color::Cyan)),
            Span::raw("Export all comments to batch prompt & inject to agent"),
        ]),
        Line::from(vec![
            Span::styled("  v                ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle file reviewed status"),
        ]),
        Line::from(vec![
            Span::styled("  T                ", Style::default().fg(Color::Cyan)),
            Span::raw("Run test mutation / fraud verification probe"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "NVIM EX-COMMANDS (:cmds)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  :w, :write       ", Style::default().fg(Color::Cyan)),
            Span::raw("Save comments to .lgtm/comments.jsonl"),
        ]),
        Line::from(vec![
            Span::styled("  :q, :quit        ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit lgtm (:q! force quit)"),
        ]),
        Line::from(vec![
            Span::styled("  :wq, :x          ", Style::default().fg(Color::Cyan)),
            Span::raw("Save comments and quit"),
        ]),
        Line::from(vec![
            Span::styled("  :pr <number>     ", Style::default().fg(Color::Cyan)),
            Span::raw("Fetch and switch to GitHub PR diff"),
        ]),
        Line::from(vec![
            Span::styled("  :prs             ", Style::default().fg(Color::Cyan)),
            Span::raw("Open GitHub PR picker modal"),
        ]),
        Line::from(vec![
            Span::styled("  :pane [id]       ", Style::default().fg(Color::Cyan)),
            Span::raw("Select or set target tmux agent pane"),
        ]),
        Line::from(vec![
            Span::styled("  :f, :files       ", Style::default().fg(Color::Cyan)),
            Span::raw("Open fuzzy file switcher"),
        ]),
        Line::from(vec![
            Span::styled("  :sp, :split      ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle side-by-side split view"),
        ]),
        Line::from(vec![
            Span::styled("  :c, :comments    ", Style::default().fg(Color::Cyan)),
            Span::raw("Open review directives & comments list"),
        ]),
        Line::from(vec![
            Span::styled("  :r, :review      ", Style::default().fg(Color::Cyan)),
            Span::raw("Compile and send review turn to agent"),
        ]),
        Line::from(vec![
            Span::styled("  :risk, :audit    ", Style::default().fg(Color::Cyan)),
            Span::raw("Re-scan for security bombs & test fraud"),
        ]),
        Line::from(vec![
            Span::styled("  :e, :reload      ", Style::default().fg(Color::Cyan)),
            Span::raw("Reload diff from disk / git / PR"),
        ]),
        Line::from(vec![
            Span::styled("  :<line_num>      ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump cursor to line number in current diff (e.g. :42)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "GENERAL & CLI",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ?                ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this Help modal"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc          ", Style::default().fg(Color::Cyan)),
            Span::raw("Close modal / Quit lgtm"),
        ]),
        Line::from(vec![
            Span::styled("  CLI `lgtm risk`  ", Style::default().fg(Color::Yellow)),
            Span::raw("Headless security & test fraud audit in CI/CD"),
        ]),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Keyboard Shortcuts & Guide (Press Esc, ?, or Enter to close) ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn command_description(cmd: &str) -> &'static str {
    match cmd {
        "w" | "write" => "Save directives to .lgtm/comments.jsonl",
        "q" | "quit" => "Quit lgtm",
        "wq" | "x" | "exit" => "Save directives and quit",
        "h" | "help" | "?" => "Open help modal",
        "pr" => "Review GitHub PR diff (:pr <num>)",
        "prs" => "Open interactive GitHub PR picker",
        "pane" | "panes" => "Select target tmux agent pane",
        "f" | "files" | "find" => "Open fuzzy file picker",
        "tree" | "drawer" | "sidebar" => "Toggle file tree sidebar",
        "sp" | "split" | "vsp" | "vsplit" => "Toggle split / unified view",
        "diff" => "Return to diff view",
        "c" | "comments" | "directives" | "todo" => "Open review directives list",
        "r" | "review" | "send" | "push" | "submit" => "Export & blast batch review to agent",
        "risk" | "audit" | "scan" => "Re-scan diff for security & test risks",
        "e" | "edit" | "reload" => "Reload diff from disk / PR",
        "clear" => "Clear all review directives",
        "y" | "yank" => "Yank current code line",
        "yr" | "ref" => "Yank #ref file:line to clipboard",
        "d" | "del" | "delete" => "Delete directive on current line",
        _ => "",
    }
}

fn render_command_completion_column(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.command_completions.len();
    if total == 0 {
        return;
    }

    let max_height = 14.min(area.height.saturating_sub(3));
    let popup_height = (total as u16 + 2).min(max_height).max(3);
    let popup_width = 58.min(area.width.saturating_sub(2));
    let popup_x = 1;
    let popup_y = area.height.saturating_sub(1).saturating_sub(popup_height);

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
    frame.render_widget(Clear, popup_area);

    let visible_rows = popup_height.saturating_sub(2) as usize;
    let active_idx = app.command_completion_idx.unwrap_or(0);
    let start_idx = if active_idx >= visible_rows {
        active_idx - visible_rows + 1
    } else {
        0
    };
    let end_idx = (start_idx + visible_rows).min(total);

    let items: Vec<ListItem> = app.command_completions[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(rel_idx, cmd)| {
            let abs_idx = start_idx + rel_idx;
            let is_selected = abs_idx == active_idx;
            let desc = command_description(cmd);

            let line = if is_selected {
                Line::from(vec![
                    Span::styled(
                        format!(" ▶ :{: <12}", cmd),
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {: <38}", desc),
                        Style::default().bg(Color::Yellow).fg(Color::Black),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("   :{: <12}", cmd),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" {}", desc), Style::default().fg(Color::DarkGray)),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let title = format!(
        " Commands ({}/{}) - Tab/S-Tab to cycle ",
        active_idx + 1,
        total
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(list, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crossbeam_channel::unbounded;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_command_descriptions() {
        assert_eq!(
            command_description("w"),
            "Save directives to .lgtm/comments.jsonl"
        );
        assert_eq!(command_description("q"), "Quit lgtm");
        assert_eq!(
            command_description("prs"),
            "Open interactive GitHub PR picker"
        );
        assert_eq!(
            command_description("risk"),
            "Re-scan diff for security & test risks"
        );
        assert_eq!(command_description("unknown_command"), "");
    }

    #[test]
    fn test_centered_rect_math() {
        let parent = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 40, parent);
        assert_eq!(centered.width, 60);
        assert_eq!(centered.height, 20);
        assert!(centered.x > 0);
        assert!(centered.y > 0);
    }

    #[test]
    fn test_full_ui_rendering_on_test_backend() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let (_tx, rx) = unbounded();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("lgtm-test-ui-{}", unique));
        let _ = std::fs::create_dir_all(&tmp);

        let mut app = App::new(tmp, rx, None, None);

        // Test rendering Diff View
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Help View
        app.active_view = ActiveView::Help;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommandLine with completions
        app.active_view = ActiveView::CommandLine;
        app.command_completions = vec!["w".into(), "write".into(), "wq".into()];
        app.command_completion_idx = Some(1);
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering FilePicker
        app.active_view = ActiveView::FilePicker;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering PanePicker
        app.active_view = ActiveView::PanePicker;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommentInput
        app.active_view = ActiveView::CommentInput;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommentList
        app.active_view = ActiveView::CommentList;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Search
        app.active_view = ActiveView::Search;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
}
