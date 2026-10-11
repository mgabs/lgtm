use crate::app::{ActiveView, App};
use crate::model::{ChangeType, DiffLine, LineType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};
use similar::{ChangeTag, TextDiff};

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
        ActiveView::CommentReply => render_comment_reply(frame, size, app),
        ActiveView::CommentList => render_comment_list(frame, size, app),
        ActiveView::PrPicker => render_pr_picker(frame, size, app),
        ActiveView::Help => render_help_modal(frame, size, app),
        ActiveView::Search => render_search_input(frame, size, app),
        ActiveView::CommitModal => render_commit_modal(frame, size, app),
        ActiveView::CommandLine => {
            if !app.command_completions.is_empty() {
                render_command_completion_column(frame, size, app);
            }
        }
        ActiveView::Diff | ActiveView::AuditReport => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    if app.active_view == ActiveView::AuditReport {
        let crit = app
            .audit_alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::Critical)
            .count();
        let high = app
            .audit_alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::High)
            .count();
        let warn = app
            .audit_alerts
            .iter()
            .filter(|a| a.severity == crate::guard::Severity::Warn)
            .count();
        let filter_label = match app.audit_filter_severity {
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
                format!("Total: {} issue(s) ", app.audit_alerts.len()),
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
                app.audit_files.len()
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
    if app.active_view == ActiveView::AuditReport {
        render_audit_report_view(frame, area, app);
        return;
    }

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

fn find_intra_line_pairs(lines: &[DiffLine]) -> std::collections::HashMap<usize, (usize, bool)> {
    let mut pairs = std::collections::HashMap::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].line_type == LineType::Deletion {
            let mut del_indices = Vec::new();
            while i < lines.len() && lines[i].line_type == LineType::Deletion {
                del_indices.push(i);
                i += 1;
            }
            let mut add_indices = Vec::new();
            while i < lines.len() && lines[i].line_type == LineType::Addition {
                add_indices.push(i);
                i += 1;
            }
            let min_len = del_indices.len().min(add_indices.len());
            for k in 0..min_len {
                pairs.insert(del_indices[k], (add_indices[k], true));
                pairs.insert(add_indices[k], (del_indices[k], false));
            }
        } else {
            i += 1;
        }
    }
    pairs
}

fn render_inline_diff_spans(
    old_text: &str,
    new_text: &str,
    is_deletion: bool,
    base_bg: Option<Color>,
) -> Vec<Span<'static>> {
    let diff = TextDiff::from_words(old_text, new_text);
    let mut spans = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        let mut style = Style::default();
        if let Some(bg) = base_bg {
            style = style.bg(bg);
        }

        match change.tag() {
            ChangeTag::Equal => {
                if is_deletion {
                    spans.push(Span::styled(text, style.fg(Color::LightRed)));
                } else {
                    spans.push(Span::styled(text, style.fg(Color::LightGreen)));
                }
            }
            ChangeTag::Delete => {
                if is_deletion {
                    spans.push(Span::styled(
                        text,
                        style
                            .bg(Color::Red)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            ChangeTag::Insert => {
                if !is_deletion {
                    spans.push(Span::styled(
                        text,
                        style
                            .bg(Color::Green)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
        }
    }
    spans
}

fn pad_or_truncate_spans(spans: Vec<Span<'static>>, target_width: usize) -> Vec<Span<'static>> {
    let mut current_len = 0;
    let mut out = Vec::new();
    for span in spans {
        let span_len = span.content.chars().count();
        if current_len + span_len <= target_width {
            current_len += span_len;
            out.push(span);
        } else {
            let take = target_width.saturating_sub(current_len);
            if take > 0 {
                let s: String = span.content.chars().take(take).collect();
                out.push(Span::styled(s, span.style));
                current_len += take;
            }
            break;
        }
    }
    if current_len < target_width {
        out.push(Span::raw(" ".repeat(target_width - current_len)));
    }
    out
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

                let view_mode_tag = if app.is_split_view && area.width >= 60 {
                    " [Split]"
                } else {
                    " [Unified]"
                };

                let active_scope = app.current_hunk_info().and_then(|(_, hunk, _)| {
                    let parts: Vec<&str> = hunk.header.split("@@").collect();
                    if parts.len() >= 3 {
                        let candidate = parts[2].trim();
                        if !candidate.is_empty() {
                            return Some(candidate.to_string());
                        }
                    }
                    None
                });

                let scope_badge = if let Some(ref scope) = active_scope {
                    format!(" │ 📍 {}", scope)
                } else {
                    String::new()
                };

                let title = format!(
                    " {}{}(+{}/-{}){}{} ",
                    file.display_name(),
                    view_mode_tag,
                    file.additions,
                    file.deletions,
                    alert_badge_title,
                    scope_badge
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
                } else if app.is_split_view && area.width >= 60 {
                    let col_width = ((area.width.saturating_sub(6)) / 2) as usize;
                    let mut line_counter = 0;

                    for hunk in &file.hunks {
                        if hunk.is_folded {
                            let hunk_len = hunk.lines.len();
                            let is_cursor_in_hunk = app.cursor_line_idx >= line_counter
                                && app.cursor_line_idx < line_counter + hunk_len;
                            let style = if is_cursor_in_hunk {
                                Style::default()
                                    .bg(Color::Indexed(237))
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            };
                            lines.push(Line::from(vec![Span::styled(
                                format!(
                                    "  ▶ {} [Folded - {} lines | zo to expand]",
                                    hunk.header, hunk_len
                                ),
                                style,
                            )]));
                            line_counter += hunk_len;
                            continue;
                        }

                        lines.push(Line::from(Span::styled(
                            format!("  {} (Split View: Left=Old, Right=New)", hunk.header),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::ITALIC),
                        )));

                        let mut h_idx = 0;
                        while h_idx < hunk.lines.len() {
                            let line = &hunk.lines[h_idx];
                            match line.line_type {
                                LineType::Context => {
                                    let is_cursor = line_counter == app.cursor_line_idx;
                                    let in_visual = app.is_in_visual_selection(line_counter);
                                    let base_bg = if in_visual && is_cursor {
                                        Some(Color::Indexed(241))
                                    } else if in_visual {
                                        Some(Color::Indexed(238))
                                    } else if is_cursor {
                                        Some(Color::Indexed(237))
                                    } else {
                                        None
                                    };

                                    let old_no = line
                                        .old_line_num
                                        .map(|n| format!("{:4} ", n))
                                        .unwrap_or_else(|| "     ".to_string());
                                    let new_no = line
                                        .new_line_num
                                        .map(|n| format!("{:4} ", n))
                                        .unwrap_or_else(|| "     ".to_string());

                                    let mut style = Style::default().fg(Color::White);
                                    if let Some(bg) = base_bg {
                                        style = style.bg(bg);
                                    }
                                    if is_cursor {
                                        style = style.add_modifier(Modifier::BOLD);
                                    }

                                    let left_spans = vec![
                                        Span::styled(old_no, Style::default().fg(Color::DarkGray)),
                                        Span::styled("  ", Style::default()),
                                        Span::styled(line.content.clone(), style),
                                    ];
                                    let right_spans = vec![
                                        Span::styled(new_no, Style::default().fg(Color::DarkGray)),
                                        Span::styled("  ", Style::default()),
                                        Span::styled(line.content.clone(), style),
                                    ];

                                    let mut row_spans = pad_or_truncate_spans(left_spans, col_width);
                                    row_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                                    row_spans.extend(pad_or_truncate_spans(right_spans, col_width));

                                    lines.push(Line::from(row_spans));
                                    line_counter += 1;
                                    h_idx += 1;
                                }
                                LineType::Deletion | LineType::Addition => {
                                    let mut dels = Vec::new();
                                    while h_idx < hunk.lines.len()
                                        && hunk.lines[h_idx].line_type == LineType::Deletion
                                    {
                                        dels.push((h_idx, &hunk.lines[h_idx]));
                                        h_idx += 1;
                                    }
                                    let mut adds = Vec::new();
                                    while h_idx < hunk.lines.len()
                                        && hunk.lines[h_idx].line_type == LineType::Addition
                                    {
                                        adds.push((h_idx, &hunk.lines[h_idx]));
                                        h_idx += 1;
                                    }
                                    let max_len = dels.len().max(adds.len());
                                    for k in 0..max_len {
                                        let del_opt = dels.get(k);
                                        let add_opt = adds.get(k);
                                        let is_cursor = line_counter == app.cursor_line_idx;
                                        let in_visual = app.is_in_visual_selection(line_counter);
                                        let base_bg = if in_visual && is_cursor {
                                            Some(Color::Indexed(241))
                                        } else if in_visual {
                                            Some(Color::Indexed(238))
                                        } else if is_cursor {
                                            Some(Color::Indexed(237))
                                        } else {
                                            None
                                        };

                                        let mut left_spans = Vec::new();
                                        if let Some((_, del_line)) = del_opt {
                                            let old_no = del_line
                                                .old_line_num
                                                .map(|n| format!("{:4} ", n))
                                                .unwrap_or_else(|| "     ".to_string());
                                            left_spans.push(Span::styled(
                                                old_no,
                                                Style::default().fg(Color::DarkGray),
                                            ));
                                            left_spans.push(Span::styled(
                                                "- ",
                                                Style::default().fg(Color::Red),
                                            ));
                                            if let Some((_, add_line)) = add_opt {
                                                left_spans.extend(render_inline_diff_spans(
                                                    &del_line.content,
                                                    &add_line.content,
                                                    true,
                                                    base_bg,
                                                ));
                                            } else {
                                                let mut style = Style::default().fg(Color::Red);
                                                if let Some(bg) = base_bg {
                                                    style = style.bg(bg);
                                                }
                                                left_spans.push(Span::styled(
                                                    del_line.content.clone(),
                                                    style,
                                                ));
                                            }
                                        }

                                        let mut right_spans = Vec::new();
                                        if let Some((_, add_line)) = add_opt {
                                            let new_no = add_line
                                                .new_line_num
                                                .map(|n| format!("{:4} ", n))
                                                .unwrap_or_else(|| "     ".to_string());
                                            right_spans.push(Span::styled(
                                                new_no,
                                                Style::default().fg(Color::DarkGray),
                                            ));
                                            right_spans.push(Span::styled(
                                                "+ ",
                                                Style::default().fg(Color::Green),
                                            ));
                                            if let Some((_, del_line)) = del_opt {
                                                right_spans.extend(render_inline_diff_spans(
                                                    &del_line.content,
                                                    &add_line.content,
                                                    false,
                                                    base_bg,
                                                ));
                                            } else {
                                                let mut style = Style::default().fg(Color::Green);
                                                if let Some(bg) = base_bg {
                                                    style = style.bg(bg);
                                                }
                                                right_spans.push(Span::styled(
                                                    add_line.content.clone(),
                                                    style,
                                                ));
                                            }
                                        }

                                        let mut row_spans =
                                            pad_or_truncate_spans(left_spans, col_width);
                                        row_spans.push(Span::styled(
                                            " │ ",
                                            Style::default().fg(Color::DarkGray),
                                        ));
                                        row_spans.extend(pad_or_truncate_spans(
                                            right_spans,
                                            col_width,
                                        ));

                                        lines.push(Line::from(row_spans));
                                        line_counter += 1;
                                    }
                                }
                                LineType::HunkHeader => {
                                    h_idx += 1;
                                }
                            }
                        }
                    }
                } else {
                    let mut line_counter = 0;
                    for hunk in &file.hunks {
                        if hunk.is_folded {
                            let hunk_len = hunk.lines.len();
                            let is_cursor_in_hunk = app.cursor_line_idx >= line_counter
                                && app.cursor_line_idx < line_counter + hunk_len;
                            let style = if is_cursor_in_hunk {
                                Style::default()
                                    .bg(Color::Indexed(237))
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            };
                            lines.push(Line::from(vec![Span::styled(
                                format!(
                                    "  ▶ {} [Folded - {} lines | zo to expand]",
                                    hunk.header, hunk_len
                                ),
                                style,
                            )]));
                            line_counter += hunk_len;
                            continue;
                        }

                        lines.push(Line::from(Span::styled(
                            format!("  {}", hunk.header),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::ITALIC),
                        )));

                        let pairs = find_intra_line_pairs(&hunk.lines);

                        for (h_line_idx, dline) in hunk.lines.iter().enumerate() {
                            let is_cursor = line_counter == app.cursor_line_idx;
                            let in_visual = app.is_in_visual_selection(line_counter);

                            let base_bg = if in_visual && is_cursor {
                                Some(Color::Indexed(241))
                            } else if in_visual {
                                Some(Color::Indexed(238))
                            } else if is_cursor {
                                Some(Color::Indexed(237))
                            } else {
                                None
                            };

                            let prefix = match dline.line_type {
                                LineType::Addition => "+",
                                LineType::Deletion => "-",
                                _ => " ",
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

                            let gutter_icon = if in_visual {
                                "▌ "
                            } else if !matching_alerts.is_empty() {
                                "⚠️ "
                            } else if !matching_comments.is_empty() {
                                if matching_comments.iter().all(|c| c.is_resolved) {
                                    "✓ "
                                } else {
                                    "💬 "
                                }
                            } else {
                                "   "
                            };

                            let icon_style = if in_visual {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else if !matching_alerts.is_empty() {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else if !matching_comments.is_empty()
                                && matching_comments.iter().all(|c| c.is_resolved)
                            {
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD)
                            };

                            let mut line_spans = vec![
                                Span::styled(line_no_str, Style::default().fg(Color::DarkGray)),
                                Span::styled(gutter_icon, icon_style),
                                Span::styled(
                                    prefix,
                                    match dline.line_type {
                                        LineType::Addition => Style::default().fg(Color::Green),
                                        LineType::Deletion => Style::default().fg(Color::Red),
                                        _ => Style::default().fg(Color::White),
                                    },
                                ),
                                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                            ];

                            if let Some(&(paired_idx, is_del)) = pairs.get(&h_line_idx) {
                                let paired_line = &hunk.lines[paired_idx];
                                let (old_txt, new_txt) = if is_del {
                                    (dline.content.as_str(), paired_line.content.as_str())
                                } else {
                                    (paired_line.content.as_str(), dline.content.as_str())
                                };
                                line_spans.extend(render_inline_diff_spans(
                                    old_txt, new_txt, is_del, base_bg,
                                ));
                            } else {
                                let mut style = match dline.line_type {
                                    LineType::Addition => Style::default().fg(Color::Green),
                                    LineType::Deletion => Style::default().fg(Color::Red),
                                    _ => Style::default().fg(Color::White),
                                };
                                if let Some(bg) = base_bg {
                                    style = style.bg(bg);
                                }
                                if is_cursor {
                                    style = style.add_modifier(Modifier::BOLD);
                                }
                                line_spans.push(Span::styled(&dline.content, style));
                            }

                            lines.push(Line::from(line_spans));

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
                                let (badge, b_style, t_style) = if comment.is_resolved {
                                    (
                                        "✓ RESOLVED DIRECTIVE: ",
                                        Style::default()
                                            .fg(Color::Green)
                                            .add_modifier(Modifier::BOLD),
                                        Style::default().fg(Color::DarkGray),
                                    )
                                } else {
                                    (
                                        "💬 DIRECTIVE: ",
                                        Style::default()
                                            .fg(Color::Magenta)
                                            .add_modifier(Modifier::BOLD),
                                        Style::default().fg(Color::LightMagenta),
                                    )
                                };

                                lines.push(Line::from(vec![
                                    Span::raw("        │ "),
                                    Span::styled(badge, b_style),
                                    Span::styled(format!("\"{}\"", comment.comment_text), t_style),
                                ]));

                                for (r_idx, reply) in comment.replies.iter().enumerate() {
                                    lines.push(Line::from(vec![
                                        Span::raw("        │   "),
                                        Span::styled(
                                            format!("↳ Reply #{}: ", r_idx + 1),
                                            Style::default()
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled(
                                            format!("\"{}\"", reply),
                                            Style::default().fg(Color::LightCyan),
                                        ),
                                    ]));
                                }
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
                    Line::from("  • d            Toggle Side-by-Side Split / Unified view"),
                    Line::from("  • V            Visual line selection mode (s: stage, u: unstage, x: discard)"),
                    Line::from("  • s / u / x    Stage / Unstage / Discard current hunk"),
                    Line::from("  • S / U / X    Stage / Unstage / Discard entire file"),
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
        let drawer_tag = if app.show_file_drawer {
            "Hide Sidebar"
        } else {
            "Show Sidebar"
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", msg),
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("[Tab/b]", Style::default().fg(Color::Cyan)),
            Span::raw(format!(" {} | ", drawer_tag)),
            Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
            Span::raw(" Nav | "),
            Span::styled("[]/[]", Style::default().fg(Color::Cyan)),
            Span::raw(" Files | "),
            Span::styled("[1-3]", Style::default().fg(Color::Cyan)),
            Span::raw(" Filter | "),
            Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
            Span::raw(" Inject | "),
            Span::styled("[e/o]", Style::default().fg(Color::Cyan)),
            Span::raw(" Editor | "),
            Span::styled("[w/E]", Style::default().fg(Color::Cyan)),
            Span::raw(" Export | "),
            Span::styled("[:]", Style::default().fg(Color::Cyan)),
            Span::raw(" Cmd | "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" Diff | "),
            Span::styled("[q]", Style::default().fg(Color::Cyan)),
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

fn render_comment_reply(frame: &mut Frame, area: Rect, app: &App) {
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

fn render_comment_list(frame: &mut Frame, area: Rect, app: &App) {
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
            let status_badge = if comment.is_resolved {
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
                " Review Directives [{}] (Enter: Jump | r: Reply | v/Space: Resolve | a: Action | d: Delete | Esc: Close) ",
                app.comments.len()
            ))
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

fn render_commit_modal(frame: &mut Frame, area: Rect, app: &App) {
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

    let staged_count = app.staged_files_count();
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

fn render_help_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(88, 92, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs bar
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Footer help navigation
        ])
        .split(popup_area);

    let tab_titles = [
        "1:Overview",
        "2:Motions",
        "3:Review",
        "4:Staging",
        "5:Folding",
        "6:Guards",
        "7:CI/Ex",
    ];

    let tab_spans: Vec<Line> = tab_titles
        .iter()
        .enumerate()
        .map(|(idx, title)| {
            if idx == app.help_selected_tab {
                Line::from(vec![Span::styled(
                    format!(" ▶ {} ", title),
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )])
            } else {
                Line::from(vec![Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(Color::Gray),
                )])
            }
        })
        .collect();

    let guide_title = format!(
        " 📖 LGTM Documentation [Tab {}/7: {}] ",
        app.help_selected_tab + 1,
        tab_titles.get(app.help_selected_tab).unwrap_or(&"")
    );

    let tabs = Tabs::new(tab_spans)
        .select(app.help_selected_tab)
        .divider("│")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(guide_title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    let (content_title, help_lines) = match app.help_selected_tab {
        0 => (
            " Tab 1: Overview, Workflow & System Modes ",
            vec![
                Line::from(vec![Span::styled(
                    "HOW TO USE LGTM (CORE WORKFLOW)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  1. Navigate Diffs: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Browse changed files and hunks with Vim motions (j/k, ]f/[f, ]h/[h)."),
                ]),
                Line::from(vec![
                    Span::styled("  2. Guardrail Scan: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Jump between security alerts and complexity spikes with ]w / [w."),
                ]),
                Line::from(vec![
                    Span::styled("  3. Steer Agents:   ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Add inline directives (c), thread replies (r), blast to tmux (Enter / Ctrl-s)."),
                ]),
                Line::from(vec![
                    Span::styled("  4. Stage & Clean:  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Stage good hunks (s), fold clean hunks (zc), discard bad edits (x / X)."),
                ]),
                Line::from(vec![
                    Span::styled("  5. Commit Forge:   ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Press C to auto-synthesize conventional commits from staged diffs."),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "SYSTEM MODES & KEYBOARD STATES",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  NORMAL Mode        ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Diff navigation, staging (s/u), hunk folding (zc/zo), file review marks (v)"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMAND / EX Mode  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press ':' for wildmenu auto-complete (:w, :q, :commit, :prs, :pane, :risk, :<line>)"),
                ]),
                Line::from(vec![
                    Span::styled("  SEARCH Mode        ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press '/' for regex diff search; cycle matches with 'n' (next) and 'N' (previous)"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMENT Mode       ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press 'c' to enter inline directive text; press Enter to save or Esc to cancel"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMIT FORGE Mode  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press 'C' to edit synthesized conventional commit message and commit to repo"),
                ]),
                Line::from(vec![
                    Span::styled("  PR REVIEW Mode     ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Run 'lgtm pr <id>' or ':pr <id>' to stream and review GitHub PR diffs live"),
                ]),
                Line::from(vec![
                    Span::styled("  CODEBASE AUDIT Mode", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Run 'lgtm scan repo', 'lgtm audit', or ':scan' to inspect all repository security/arch issues"),
                ]),
                Line::from(vec![
                    Span::styled("  HELP Mode          ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press '?' or 'h' to open this interactive guide; switch tabs with Tab or 1-7"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "INSTALLATION & DOCUMENTATION",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  Homebrew Install:  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("brew install mgabs/tap/lgtm"),
                ]),
                Line::from(vec![
                    Span::styled("  GitHub Repo:       ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm"),
                ]),
                Line::from(vec![
                    Span::styled("  Documentation:     ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm#readme / docs/WIKI.md"),
                ]),
                Line::from(vec![
                    Span::styled("  Issues & Requests: ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm/issues"),
                ]),
                Line::from(vec![
                    Span::styled("  Discussions:       ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm/discussions"),
                ]),
            ],
        ),
        1 => (
            " Tab 2: Vim Motions & Code Navigation ",
            vec![
                Line::from(vec![Span::styled(
                    "CURSOR & SCROLLING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  j / k, ↓ / ↑     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Move cursor line-by-line through unified/split diff"),
                ]),
                Line::from(vec![
                    Span::styled("  gg / G           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump cursor to top / bottom of current file diff"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-d / Ctrl-u  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Scroll half-page down / up (15 lines)"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-f / Ctrl-b  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Scroll full page down / up (25 lines) or PgDn / PgUp"),
                ]),
                Line::from(vec![
                    Span::styled("  } / {            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to next / previous paragraph / blank line boundary"),
                ]),
                Line::from(vec![
                    Span::styled("  0 / $            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to first line / last line in current file diff"),
                ]),
                Line::from(vec![
                    Span::styled("  :<number>        ", Style::default().fg(Color::Cyan)),
                    Span::raw("Direct jump to target line number (e.g. :42 or :150)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "FILE & HUNK BRACKET JUMPS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  ]f / [f (or ]/[) ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous changed file (press ] or [ to chain)"),
                ]),
                Line::from(vec![
                    Span::styled("  ]h / [h          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous diff hunk"),
                ]),
                Line::from(vec![
                    Span::styled("  ]c / [c          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous review directive / inline comment"),
                ]),
                Line::from(vec![
                    Span::styled("  ]w / [w (or w/W) ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous security alert or guard warning"),
                ]),
                Line::from(vec![
                    Span::styled("  ]u / [u          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous unreviewed file"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "VIEWPORT & SEARCH CONTROLS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  /                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Interactive regex search across entire diff"),
                ]),
                Line::from(vec![
                    Span::styled("  n / N            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to next / previous search match"),
                ]),
                Line::from(vec![
                    Span::styled("  * / #            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Search word under cursor forward / backward"),
                ]),
                Line::from(vec![
                    Span::styled("  zz / zt / zb     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Center view on cursor / Cursor to top of view / Cursor to bottom"),
                ]),
                Line::from(vec![
                    Span::styled("  f / Ctrl-p       ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open fuzzy interactive switcher to find and jump to any changed file"),
                ]),
                Line::from(vec![
                    Span::styled("  Tab              ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle file tree drawer sidebar (left panel)"),
                ]),
                Line::from(vec![
                    Span::styled("  d                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Unified vs Side-by-Side Split Diff view"),
                ]),
                Line::from(vec![
                    Span::styled("  o / e            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open current file:line in $EDITOR (nvim, vim, vscode)"),
                ]),
            ],
        ),
        2 => (
            " Tab 3: Review Directives & Agent Steering ",
            vec![
                Line::from(vec![Span::styled(
                    "INLINE REVIEW DIRECTIVES & AGENT STEERING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  c (or <Space>c)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Anchor an inline directive comment directly to the current file:line"),
                ]),
                Line::from(vec![
                    Span::styled("  r                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Thread a reply under the directive at cursor (or open new directive)"),
                ]),
                Line::from(vec![
                    Span::styled("  R / v (or Space-r)", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle directive resolved state (✓ RESOLVED vs 💬 OPEN)"),
                ]),
                Line::from(vec![
                    Span::styled("  x / <Space>d     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Delete the directive anchored on the current line"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>l / :c    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open interactive Review Directives list drawer"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-s / :review ", Style::default().fg(Color::Cyan)),
                    Span::raw("Compile directives to batch prompt and blast to tmux agent / PR / clipboard"),
                ]),
                Line::from(vec![
                    Span::styled("  Enter            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inject single #ref file:line anchor into target agent tmux pane"),
                ]),
                Line::from(vec![
                    Span::styled("  p (or <Space>p)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open tmux pane selector to target specific agent session"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>y / yr    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank #ref file:line anchor to system clipboard via OSC 52"),
                ]),
                Line::from(vec![
                    Span::styled("  y                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank raw current line content to clipboard"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "REVIEW STATUS MARKS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  v                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle reviewed status checkmark (✓) for current file (persisted in .lgtm/state.json)"),
                ]),
            ],
        ),
        3 => (
            " Tab 4: Git Staging & Interactive Commit Forge ",
            vec![
                Line::from(vec![Span::styled(
                    "GIT HUNK & FILE STAGING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  s                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Stage current hunk (or selected visual lines) into Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  S                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Stage entire current file changes into Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  u                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unstage current hunk (or selected visual lines) from Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  U                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unstage entire current file from Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  V                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Visual Line Selection mode to stage/discard arbitrary line ranges"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>x         ", Style::default().fg(Color::Cyan)),
                    Span::raw("Discard changes in current hunk"),
                ]),
                Line::from(vec![
                    Span::styled("  X                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Discard all working tree changes in current file (git restore)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "COMMIT FORGE (:commit / C)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  C / :commit      ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open interactive Commit Forge modal for staged hunks"),
                ]),
                Line::from("  • Automatically synthesizes Conventional Commit messages (feat:, fix:, chore:, refactor:)."),
                Line::from("  • Live edit commit title & body before committing."),
                Line::from("  • Press Enter in Commit Forge to execute git commit immediately with instant feedback."),
            ],
        ),
        4 => (
            " Tab 5: Syntax-Aware AST & Hunk Folding ",
            vec![
                Line::from(vec![Span::styled(
                    "HUNK FOLDING CONTROLS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  zc               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Fold current hunk into compact header (▶ @@ ... @@ [Folded])"),
                ]),
                Line::from(vec![
                    Span::styled("  zo               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unfold current hunk to expand and inspect all diff lines"),
                ]),
                Line::from(vec![
                    Span::styled("  za               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle fold state of current hunk under cursor"),
                ]),
                Line::from(vec![
                    Span::styled("  zM / zm          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Fold ALL hunks across the current file / diff"),
                ]),
                Line::from(vec![
                    Span::styled("  zR / zr          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unfold ALL hunks across the current file / diff"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "WHY FOLDING MATTERS FOR AGENT REVIEW",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from("  • AI Coding Agents often touch massive boilerplate files alongside critical logic."),
                Line::from("  • Folding unchanged or verified hunks lets you isolate and focus on suspicious diffs."),
                Line::from("  • Line numbers and cursor indexing remain 100% accurate while folded."),
                Line::from("  • Press `zo` on any folded hunk to inspect or stage individual lines."),
            ],
        ),
        5 => (
            " Tab 6: Security Guardrails & Static Analysis ",
            vec![
                Line::from(vec![Span::styled(
                    "REAL-TIME STATIC ANALYSIS & CODE QUALITY GUARDS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  🔴 Shannon Entropy Secrets: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Sniffs high-entropy strings (H ≥ 4.3), API keys (sk-proj, ghp), RSA private keys"),
                ]),
                Line::from(vec![
                    Span::styled("  🔴 Injections & Traversal:   ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags os.system, exec, raw SQL string templates, SSRF sinks, and path traversal (..)"),
                ]),
                Line::from(vec![
                    Span::styled("  🔴 Insecure Deserialization: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags unsafe pickle.loads, yaml.load without SafeLoader, and hardcoded auth fallbacks"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Cognitive Complexity:    ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Warns when agent code spikes complexity (+5 points or nested loops/conditions)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Identifier Typo Sniffer: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Levenshtein distance sniffer catching 1-2 char typos against symbol index (e.g. usertoken)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Breaking Public API Drift: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Detects modified parameter lists or return types on pub fn / export functions"),
                ]),
                Line::from(vec![
                    Span::styled("  🟡 Dead Code Additions (YAGNI): ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags unreferenced helper functions added by agents with 0 call sites across repo"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Test Neutering & Skips:   ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Detects deleted assertions, @pytest.mark.skip, t.Skip, and tautological tests (assert True)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Architecture Sprawl & Leaks: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Catches UI->DB layer violations, blast radius sprawl (≥4 domains), and .env file leaks"),
                ]),
            ],
        ),
        6 => (
            " Tab 7: Ex-Commands & Headless CI Integration ",
            vec![
                Line::from(vec![Span::styled(
                    "EX-COMMAND REFERENCE (Type ':' in diff view, press Tab to cycle)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  :w, :write           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Save active directives to .lgtm/comments.jsonl"),
                ]),
                Line::from(vec![
                    Span::styled("  :q, :quit, :exit     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Quit lgtm (:q! force quit)"),
                ]),
                Line::from(vec![
                    Span::styled("  :wq, :x              ", Style::default().fg(Color::Cyan)),
                    Span::raw("Save directives to disk and exit"),
                ]),
                Line::from(vec![
                    Span::styled("  :commit [msg], :ci   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Commit staged changes with message (or open Commit Forge modal)"),
                ]),
                Line::from(vec![
                    Span::styled("  :prs                 ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open interactive GitHub PR picker modal"),
                ]),
                Line::from(vec![
                    Span::styled("  :pr <number>         ", Style::default().fg(Color::Cyan)),
                    Span::raw("Fetch and switch to GitHub pull request diff directly (e.g. :pr 42)"),
                ]),
                Line::from(vec![
                    Span::styled("  :pane [id]           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Select or set target tmux agent pane ID (e.g. :pane %2)"),
                ]),
                Line::from(vec![
                    Span::styled("  :f, :files, :find    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open fuzzy file switcher modal"),
                ]),
                Line::from(vec![
                    Span::styled("  :sp, :split, :vsp    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Unified vs Side-by-Side Split Diff mode"),
                ]),
                Line::from(vec![
                    Span::styled("  :scan, :audit        ", Style::default().fg(Color::Cyan)),
                    Span::raw("Launch full Codebase Audit mode (issues only, no diff view)"),
                ]),
                Line::from(vec![
                    Span::styled("  :export [path]       ", Style::default().fg(Color::Cyan)),
                    Span::raw("Export audit scorecard report to Markdown (default: lgtm-report.md)"),
                ]),
                Line::from(vec![
                    Span::styled("  :w [path]            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Export audit report (in Audit mode) or save review directives (in Diff mode)"),
                ]),
                Line::from(vec![
                    Span::styled("  :diff                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Return to Diff review mode from Audit mode (or specify revision range)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "AUDIT MODE CONTROLS (Active in Codebase Audit view)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  1 / 2 / 3            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Filter severity: [1] Critical only | [2] High+ | [3] All issues"),
                ]),
                Line::from(vec![
                    Span::styled("  Enter                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inject violation remediation directive directly to agent tmux pane / clipboard"),
                ]),
                Line::from(vec![
                    Span::styled("  e / o                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open current issue file and line in external editor ($EDITOR)"),
                ]),
                Line::from(vec![
                    Span::styled("  w / E                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Quick export repository audit report to lgtm-report.md"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "HEADLESS CI / CD & PRE-COMMIT GATES",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  lgtm check           ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Headless gate for GitHub Actions & CI. Exits code 1 on Critical/High risks."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan --tui      ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Launch interactive TUI Codebase Audit mode (`lgtm scan repo` / `lgtm audit`)."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan -o rep.md  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Generates full Markdown audit scorecard (Grade A/B/C/F) & remediation steps."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan -f sarif   ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Exports standard SARIF JSON for GitHub Security Code Scanning tab upload."),
                ]),
            ],
        ),
        _ => (" Help ", vec![Line::from("No help content")]),
    };

    let content_block = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(content_title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(content_block, chunks[1]);

    let footer_text = Line::from(vec![
        Span::styled(" Tabs: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "l",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "→",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Next | "),
        Span::styled(
            "BackTab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "h",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "←",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Prev | "),
        Span::styled(
            "1-7",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Jump Tab | "),
        Span::styled(
            "Esc / q / ?",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Close"),
    ]);
    let footer_widget = Paragraph::new(footer_text);
    frame.render_widget(footer_widget, chunks[2]);
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
        "commit" | "ci" => "Open Commit Forge to commit staged changes",
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

fn render_audit_file_drawer(frame: &mut Frame, area: Rect, app: &App) {
    if app.audit_files.is_empty() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            " (No files audited) ",
            Style::default().fg(Color::DarkGray),
        )))];
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Files (Tab) ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .audit_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = idx == app.selected_file_idx;
            let display_name = file.display_name();

            let file_alerts_count = app
                .audit_alerts
                .iter()
                .filter(|a| a.file_path == display_name)
                .count();

            let (badge, badge_color) = if file_alerts_count > 0 {
                (format!("⚠️{:2} ", file_alerts_count), Color::Yellow)
            } else {
                (" ✓   ".to_string(), Color::Green)
            };

            let spans = vec![
                Span::styled(
                    badge,
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(display_name),
            ];

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
            .title(format!(" Files [{}] (Tab) ", app.audit_files.len()))
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_audit_report_view(frame: &mut Frame, area: Rect, app: &App) {
    let filtered_alerts = app.filtered_audit_alerts();
    let total_alerts = app.audit_alerts.len();
    let filtered_count = filtered_alerts.len();

    // Check if sidebar should be rendered
    let (left_area, right_area) = if app.show_file_drawer && area.width >= 70 {
        let drawer_width = if area.width >= 110 { 32 } else { 25 };
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(drawer_width), Constraint::Min(40)])
            .split(area);

        render_audit_file_drawer(frame, top_chunks[0], app);

        let sub_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(45), // Issues list
                Constraint::Percentage(55), // Inspector
            ])
            .split(top_chunks[1]);

        (sub_chunks[0], sub_chunks[1])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(45), // Issues list
                Constraint::Percentage(55), // Inspector
            ])
            .split(area);

        (chunks[0], chunks[1])
    };

    let visible_height = (left_area.height.saturating_sub(2)) as usize;
    let total_filtered = filtered_alerts.len();
    let scroll_offset = if total_filtered <= visible_height || visible_height == 0 {
        0
    } else {
        let half = visible_height / 2;
        if app.audit_selected_idx > half {
            (app.audit_selected_idx - half).min(total_filtered - visible_height)
        } else {
            0
        }
    };

    let items: Vec<ListItem> = if filtered_alerts.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            " (No issues found matching filter) ",
            Style::default().fg(Color::Green),
        )]))]
    } else {
        filtered_alerts
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(if visible_height > 0 {
                visible_height
            } else {
                total_filtered
            })
            .map(|(idx, alert)| {
                let is_selected = idx == app.audit_selected_idx;
                let badge_style = match alert.severity {
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

                let loc = match alert.line_no {
                    Some(l) => format!("{}:{}", alert.file_path, l),
                    None => alert.file_path.clone(),
                };

                let spans = vec![
                    Span::styled(format!("[{}] ", alert.severity.badge()), badge_style),
                    Span::styled(format!("{} ", loc), Style::default().fg(Color::Cyan)),
                    Span::raw(&alert.title),
                ];

                let line_style = if is_selected {
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(line_style)
            })
            .collect()
    };

    let filter_text = match app.audit_filter_severity {
        Some(crate::guard::Severity::Critical) => "🔴 Crit [1]",
        Some(crate::guard::Severity::High) => "🟠 High+ [2]",
        _ => "All [3]",
    };

    let issues_title = format!(
        " Repository Issues ({}/{}) Filter: {} ",
        filtered_count, total_alerts, filter_text
    );

    let issues_block = Block::default()
        .borders(Borders::ALL)
        .title(issues_title)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(List::new(items).block(issues_block), left_area);

    let selected_alert = filtered_alerts.get(app.audit_selected_idx).copied();
    render_audit_inspector(frame, right_area, app, selected_alert);
}

fn render_audit_inspector(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    selected_alert: Option<&crate::guard::GuardAlert>,
) {
    let alert = match selected_alert {
        Some(a) => a,
        None => {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No issue selected",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from("  Run :scan or press 1-3 to filter issues."),
            ];
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Issue Inspector ")
                .border_style(Style::default().fg(Color::DarkGray));
            frame.render_widget(Paragraph::new(empty_text).block(block), area);
            return;
        }
    };

    let loc = match alert.line_no {
        Some(l) => format!("{}:{}", alert.file_path, l),
        None => alert.file_path.clone(),
    };

    let sev_color = match alert.severity {
        crate::guard::Severity::Critical => Color::Red,
        crate::guard::Severity::High => Color::LightRed,
        crate::guard::Severity::Warn => Color::Yellow,
    };

    let mut lines = Vec::new();

    // 1. Alert header
    lines.push(Line::from(vec![
        Span::styled(
            format!(" [{}] ", alert.severity.badge()),
            Style::default()
                .bg(sev_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            &alert.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Location: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &loc,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // 2. Description
    lines.push(Line::from(Span::styled(
        "Description / Finding:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    for desc_line in alert.description.lines() {
        lines.push(Line::from(Span::raw(format!("  {}", desc_line))));
    }
    lines.push(Line::from(""));

    // 3. Code context snippet (read lines around line_no if available)
    if let Some(target_line) = alert.line_no {
        let file_path = app.audit_root_dir.join(&alert.file_path);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            lines.push(Line::from(Span::styled(
                "Code Snippet Context:",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )));
            let all_file_lines: Vec<&str> = content.lines().collect();
            let start = (target_line as usize).saturating_sub(4);
            let end = ((target_line as usize) + 3).min(all_file_lines.len());

            for (idx, line_item) in all_file_lines.iter().enumerate().take(end).skip(start) {
                let current_no = idx + 1;
                let is_hit = current_no == (target_line as usize);
                let line_text = line_item.to_string();

                if is_hit {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  ▶ {:4} │ ", current_no),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            line_text,
                            Style::default()
                                .bg(Color::DarkGray)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("    {:4} │ ", current_no),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(line_text, Style::default().fg(Color::Gray)),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
    }

    // 4. Agent Remediation Directive
    lines.push(Line::from(Span::styled(
        "Remediation Directive (Press [Enter] to inject to agent):",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )));
    let directive = format!(
        "Fix security/architectural violation in {}: [{}] {} - {}",
        loc,
        alert.severity.badge(),
        alert.title,
        alert.description
    );
    lines.push(Line::from(Span::styled(
        format!("  {}", directive),
        Style::default().fg(Color::LightMagenta),
    )));
    lines.push(Line::from(""));

    // 5. Actions footer
    lines.push(Line::from(vec![
        Span::styled(
            "[Enter]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Inject Directive  "),
        Span::styled(
            "[e/o]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Open in Editor  "),
        Span::styled(
            "[w]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Export Report  "),
        Span::styled(
            "[Esc]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Diff View"),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Issue Inspector & Remediation ")
        .border_style(Style::default().fg(Color::LightBlue));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
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

        let mut app = App::new(tmp, rx, None, None, None);

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

        // Test rendering CommitModal
        app.active_view = ActiveView::CommitModal;
        app.commit_message_input = "feat: add cognitive complexity guard".to_string();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Visual Mode
        app.active_view = ActiveView::Diff;
        app.visual_mode = true;
        app.visual_anchor_idx = Some(0);
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Split View
        app.is_split_view = true;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }

    #[test]
    fn test_find_intra_line_pairs() {
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "header".to_string(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Deletion,
                content: "let x = 1;".to_string(),
                old_line_num: Some(2),
                new_line_num: None,
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "let x = 2;".to_string(),
                old_line_num: None,
                new_line_num: Some(2),
            },
        ];

        let pairs = find_intra_line_pairs(&lines);
        assert_eq!(pairs.get(&1), Some(&(2, true)));
        assert_eq!(pairs.get(&2), Some(&(1, false)));
    }

    #[test]
    fn test_pad_or_truncate_spans() {
        let spans = vec![Span::raw("hello")];
        let padded = pad_or_truncate_spans(spans, 10);
        let total_chars: usize = padded.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(total_chars, 10);

        let long_spans = vec![Span::raw("very long line that exceeds limit")];
        let truncated = pad_or_truncate_spans(long_spans, 8);
        let trunc_chars: usize = truncated.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(trunc_chars, 8);
    }
}
