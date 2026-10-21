use super::*;

pub(crate) fn render_body(frame: &mut Frame, area: Rect, app: &mut App) {
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

pub(crate) fn render_file_drawer(frame: &mut Frame, area: Rect, app: &App) {
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
            let done = lgtm_core::review::reviewed_flags(&app.marks, file)
                .iter()
                .filter(|&&b| b)
                .count();
            if done > 0 && !file.is_reviewed {
                spans.push(Span::styled(
                    format!(" ✓{}/{}", done, file.hunks.len()),
                    Style::default().fg(Color::Green),
                ));
            }

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

    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.selected_file_idx));
    frame.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn find_intra_line_pairs(
    lines: &[DiffLine],
) -> std::collections::HashMap<usize, (usize, bool)> {
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

pub(crate) fn render_inline_diff_spans(
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

pub(crate) fn pad_or_truncate_spans(
    spans: Vec<Span<'static>>,
    target_width: usize,
) -> Vec<Span<'static>> {
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

pub(crate) fn render_diff_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let current_file = app.snapshot.files.get(app.selected_file_idx);

    let (title, content_lines, active_cursor_row) = if !app.git_engine.is_git_repo() {
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
            None,
        )
    } else {
        match current_file {
            Some(file) => {
                let mut active_cursor_row: Option<usize> = None;
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
                            if is_cursor_in_hunk && active_cursor_row.is_none() {
                                active_cursor_row = Some(lines.len() - 1);
                            }
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
                            if let Some(range) = App::is_line_in_folded_block(hunk, h_idx) {
                                if App::is_first_line_of_folded_block(hunk, h_idx) == Some(range) {
                                    let is_cursor = line_counter == app.cursor_line_idx;
                                    let style = if is_cursor {
                                        Style::default()
                                            .bg(Color::Indexed(237))
                                            .fg(Color::Yellow)
                                            .add_modifier(Modifier::BOLD)
                                    } else {
                                        Style::default()
                                            .fg(Color::Yellow)
                                            .add_modifier(Modifier::BOLD)
                                    };
                                    let count = range.end_line.saturating_sub(range.start_line) + 1;
                                    let snippet = hunk.lines[h_idx].content.trim();
                                    lines.push(Line::from(vec![Span::styled(
                                        format!(
                                            "  ▶ [Folded AST Block (L{}-L{}, {} lines): {} | zo/za to expand]",
                                            range.start_line, range.end_line, count, snippet
                                        ),
                                        style,
                                    )]));
                                    if is_cursor && active_cursor_row.is_none() {
                                        active_cursor_row = Some(lines.len() - 1);
                                    }
                                }
                                line_counter += 1;
                                h_idx += 1;
                                continue;
                            }

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
                                        .map(|n| format!(" {:3} ", n))
                                        .unwrap_or_else(|| "     ".to_string());
                                    let new_no = line
                                        .new_line_num
                                        .map(|n| {
                                            if is_cursor {
                                                format!("▶{:3} ", n)
                                            } else {
                                                format!(" {:3} ", n)
                                            }
                                        })
                                        .unwrap_or_else(|| {
                                            if is_cursor {
                                                "▶    ".to_string()
                                            } else {
                                                "     ".to_string()
                                            }
                                        });

                                    let mut style = Style::default().fg(Color::White);
                                    if let Some(bg) = base_bg {
                                         style = style.bg(bg);
                                    }
                                    if is_cursor {
                                        style = style.add_modifier(Modifier::BOLD);
                                    }

                                    let right_no_style = if is_cursor {
                                        Style::default()
                                            .fg(Color::Cyan)
                                            .add_modifier(Modifier::BOLD)
                                    } else {
                                        Style::default().fg(Color::DarkGray)
                                    };

                                    let left_spans = vec![
                                        Span::styled(old_no, Style::default().fg(Color::DarkGray)),
                                        Span::styled("  ", Style::default()),
                                        Span::styled(line.content.clone(), style),
                                    ];
                                    let right_spans = vec![
                                        Span::styled(new_no, right_no_style),
                                        Span::styled("  ", Style::default()),
                                        Span::styled(line.content.clone(), style),
                                    ];

                                    let mut row_spans = pad_or_truncate_spans(left_spans, col_width);
                                    row_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                                    row_spans.extend(pad_or_truncate_spans(right_spans, col_width));

                                    lines.push(Line::from(row_spans));
                                    if is_cursor && active_cursor_row.is_none() {
                                        active_cursor_row = Some(lines.len() - 1);
                                    }
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
                                                .map(|n| {
                                                    if is_cursor {
                                                        format!("▶{:3} ", n)
                                                    } else {
                                                        format!(" {:3} ", n)
                                                    }
                                                })
                                                .unwrap_or_else(|| {
                                                    if is_cursor {
                                                        "▶    ".to_string()
                                                    } else {
                                                        "     ".to_string()
                                                    }
                                                });
                                            let right_no_style = if is_cursor {
                                                Style::default()
                                                    .fg(Color::Cyan)
                                                    .add_modifier(Modifier::BOLD)
                                            } else {
                                                Style::default().fg(Color::DarkGray)
                                            };
                                            right_spans.push(Span::styled(
                                                new_no,
                                                right_no_style,
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
                                        if is_cursor && active_cursor_row.is_none() {
                                            active_cursor_row = Some(lines.len() - 1);
                                        }
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
                            if is_cursor_in_hunk && active_cursor_row.is_none() {
                                active_cursor_row = Some(lines.len() - 1);
                            }
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
                            if let Some(range) = App::is_line_in_folded_block(hunk, h_line_idx) {
                                if App::is_first_line_of_folded_block(hunk, h_line_idx) == Some(range) {
                                    let is_cursor = line_counter == app.cursor_line_idx;
                                    let style = if is_cursor {
                                        Style::default()
                                            .bg(Color::Indexed(237))
                                            .fg(Color::Yellow)
                                            .add_modifier(Modifier::BOLD)
                                    } else {
                                        Style::default()
                                            .fg(Color::Yellow)
                                            .add_modifier(Modifier::BOLD)
                                    };
                                    let count = range.end_line.saturating_sub(range.start_line) + 1;
                                    let snippet = dline.content.trim();
                                    lines.push(Line::from(vec![Span::styled(
                                        format!(
                                            "  ▶ [Folded AST Block (L{}-L{}, {} lines): {} | zo/za to expand]",
                                            range.start_line, range.end_line, count, snippet
                                        ),
                                        style,
                                    )]));
                                    if is_cursor && active_cursor_row.is_none() {
                                        active_cursor_row = Some(lines.len() - 1);
                                    }
                                }
                                line_counter += 1;
                                continue;
                            }

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
                                Some(num) if is_cursor => format!("▶{:3} ", num),
                                Some(num) => format!(" {:3} ", num),
                                None if is_cursor => "▶    ".to_string(),
                                None => "     ".to_string(),
                            };

                            // Check matching line-anchored alerts
                            let matching_alerts: Vec<&&crate::guard::GuardAlert> = file_alerts
                                .iter()
                                .filter(|a| a.line_no.is_some() && a.line_no == line_no)
                                .collect();

                            // Check matching line-anchored comments
                            let dname = file.display_name();
                            let clean_dname = dname.trim_start_matches("./");
                            let matching_comments: Vec<&crate::comment::CommentAnchor> = app
                                .comments
                                .iter()
                                .filter(|c| {
                                    if c.orphaned || Some(c.target_line_no) != line_no {
                                        return false;
                                    }
                                    let clean_cpath = c.file_path.trim_start_matches("./");
                                    clean_cpath == clean_dname
                                        || clean_dname.ends_with(clean_cpath)
                                        || clean_cpath.ends_with(clean_dname)
                                })
                                .collect();

                            let thread_state = line_no.and_then(|n| {
                                lgtm_core::github::thread_marker(
                                    &app.threads,
                                    &file.display_name(),
                                    n,
                                )
                            });

                            let gutter_icon = if in_visual {
                                "▌ "
                            } else if !matching_alerts.is_empty() {
                                "⚠️ "
                            } else if let Some(open) = thread_state {
                                if open {
                                    "◆ "
                                } else {
                                    "◇ "
                                }
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
                            } else if let Some(open) = thread_state {
                                Style::default()
                                    .fg(if open { Color::Cyan } else { Color::Green })
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

                            let line_no_style = if is_cursor {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };

                            let mut line_spans = vec![
                                Span::styled(line_no_str, line_no_style),
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
                            if is_cursor && active_cursor_row.is_none() {
                                active_cursor_row = Some(lines.len() - 1);
                            }

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
                (title, lines, active_cursor_row)
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
                    Line::from("  • za / zc / zo Toggle / close / open AST block or hunk fold"),
                    Line::from("  • zM / zR      Fold all / unfold all hunks"),
                    Line::from("  • v            Mark file reviewed and fold"),
                    Line::from("  • p            Select target tmux pane"),
                    Line::from("  • q            Quit"),
                ],
                None,
            ),
        }
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    if inner_height > 0 {
        if content_lines.len() <= inner_height {
            app.diff_scroll_offset = 0;
        } else if let Some(cursor_row) = active_cursor_row {
            if cursor_row < app.diff_scroll_offset {
                app.diff_scroll_offset = cursor_row;
            } else if cursor_row >= app.diff_scroll_offset + inner_height {
                app.diff_scroll_offset = cursor_row.saturating_sub(inner_height - 1);
            }
        }
    }

    let paragraph = Paragraph::new(content_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .scroll((app.diff_scroll_offset as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::app::App;
    use crate::model::*;
    use crossbeam_channel::unbounded;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

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

    #[test]
    fn test_render_diff_view_auto_scrolls_to_active_line() {
        let mut app = crate::app::test_util::create_test_app();
        app.git_engine = lgtm_core::git::GitEngine::new(".");
        let lines: Vec<DiffLine> = (0..50)
            .map(|i| DiffLine {
                line_type: LineType::Context,
                content: format!("println!(\"line {}\");", i),
                old_line_num: Some(i + 1),
                new_line_num: Some(i + 1),
            })
            .collect();

        let hunk = DiffHunk {
            header: "@@ -1,50 +1,50 @@".into(),
            old_start: 1,
            old_lines: 50,
            new_start: 1,
            new_lines: 50,
            lines,
            is_folded: false,
            folded_blocks: Vec::new(),
        };

        let file = FileDiff {
            old_path: None,
            new_path: std::path::PathBuf::from("main.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);
        app.selected_file_idx = 0;
        app.cursor_line_idx = 0;
        app.diff_scroll_offset = 0;

        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Initial render at line 0: diff_scroll_offset should be 0
        terminal
            .draw(|f| {
                render_diff_view(f, f.area(), &mut app);
            })
            .unwrap();
        assert_eq!(app.diff_scroll_offset, 0);

        // 2. Move cursor down past the screen: cursor_line_idx = 25
        app.cursor_line_idx = 25;
        terminal
            .draw(|f| {
                render_diff_view(f, f.area(), &mut app);
            })
            .unwrap();
        assert!(app.diff_scroll_offset > 0);
        let inner_height = 10;
        let active_row = 26; // 1 hunk header + 25 lines
        assert!(active_row >= app.diff_scroll_offset);
        assert!(active_row < app.diff_scroll_offset + inner_height);

        // 3. Move cursor up above the screen: cursor_line_idx = 2
        app.cursor_line_idx = 2;
        terminal
            .draw(|f| {
                render_diff_view(f, f.area(), &mut app);
            })
            .unwrap();
        let active_row_up = 3;
        assert!(active_row_up >= app.diff_scroll_offset);
        assert!(active_row_up < app.diff_scroll_offset + inner_height);
    }

    #[test]
    fn test_render_diff_view_auto_scrolls_in_split_view() {
        let mut app = crate::app::test_util::create_test_app();
        app.git_engine = lgtm_core::git::GitEngine::new(".");
        app.is_split_view = true;
        let lines: Vec<DiffLine> = (0..50)
            .map(|i| DiffLine {
                line_type: LineType::Context,
                content: format!("println!(\"line {}\");", i),
                old_line_num: Some(i + 1),
                new_line_num: Some(i + 1),
            })
            .collect();

        let hunk = DiffHunk {
            header: "@@ -1,50 +1,50 @@".into(),
            old_start: 1,
            old_lines: 50,
            new_start: 1,
            new_lines: 50,
            lines,
            is_folded: false,
            folded_blocks: Vec::new(),
        };

        let file = FileDiff {
            old_path: None,
            new_path: std::path::PathBuf::from("main.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);
        app.selected_file_idx = 0;
        app.cursor_line_idx = 30;
        app.diff_scroll_offset = 0;

        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_diff_view(f, f.area(), &mut app);
            })
            .unwrap();

        assert!(app.diff_scroll_offset > 0);
        let inner_height = 13;
        let active_row = 31; // 1 hunk header + 30 lines
        assert!(active_row >= app.diff_scroll_offset);
        assert!(active_row < app.diff_scroll_offset + inner_height);
    }
}
