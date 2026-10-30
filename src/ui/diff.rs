pub(crate) mod cache;
mod viewport;
use super::*;
use viewport::{padded_gutter, render_diff_view};

pub(crate) fn render_body(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.active_view == ActiveView::AuditReport {
        render_audit_report_view(frame, area, app);
        return;
    }

    if app.active_view == ActiveView::MarkdownPreview {
        if app.show_file_drawer && area.width >= 50 {
            let drawer_width = if area.width >= 100 { 35 } else { 28 };
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(drawer_width), Constraint::Min(20)])
                .split(area);

            render_file_drawer(frame, body_chunks[0], app);
            super::markdown::render_markdown_preview(frame, body_chunks[1], app);
        } else {
            super::markdown::render_markdown_preview(frame, area, app);
        }
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
    let mut timing = lgtm_core::debug::Span::new("ui.drawer_viewport");
    timing.count("rows", 0);
    app.render_cache.borrow_mut().drawer_rows = 0;
    if !app.view_data.is_git {
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

    let visible_height = area.height.saturating_sub(2) as usize;
    let first = app
        .selected_file_idx
        .saturating_sub(visible_height.saturating_sub(1));
    let mut cache = app.render_cache.borrow_mut();
    let annotations = cache.index(app, cache::annotation_signature(app));
    cache.prepare_reviewed(app);
    let items: Vec<ListItem> = app
        .snapshot
        .files
        .iter()
        .enumerate()
        .skip(first)
        .take(visible_height)
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

            let file_alerts_count = annotations
                .alert_counts
                .get(&file.display_name())
                .copied()
                .unwrap_or(0);
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
            let done = cache.reviewed_count(app, idx);
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

    cache.drawer_rows = items.len();
    timing.count("rows", items.len() as u64);
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Changed Files [{}] (Tab) ",
                app.snapshot.files.len()
            ))
            .border_style(Style::default().fg(Color::Cyan)),
    );

    let mut state = ratatui::widgets::ListState::default()
        .with_selected(Some(app.selected_file_idx.saturating_sub(first)));
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

pub(super) type WordRanges = (
    Vec<(std::ops::Range<usize>, Style)>,
    Vec<(std::ops::Range<usize>, Style)>,
);

pub(super) fn prepare_inline_diff_ranges(
    old_text: &str,
    new_text: &str,
    base_bg: Option<Color>,
) -> WordRanges {
    let diff = TextDiff::from_words(old_text, new_text);
    let mut old = Vec::new();
    let mut new = Vec::new();
    let mut old_offset = 0;
    let mut new_offset = 0;
    for change in diff.iter_all_changes() {
        let mut style = Style::default();
        if let Some(bg) = base_bg {
            style = style.bg(bg);
        }
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Equal => {
                let old_range = old_offset..old_offset + len;
                let new_range = new_offset..new_offset + len;
                old.push((old_range, style.fg(Color::LightRed)));
                new.push((new_range, style.fg(Color::LightGreen)));
                old_offset += len;
                new_offset += len;
            }
            ChangeTag::Delete => {
                let range = old_offset..old_offset + len;
                old.push((
                    range,
                    style
                        .bg(Color::Red)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                old_offset += len;
            }
            ChangeTag::Insert => {
                let range = new_offset..new_offset + len;
                new.push((
                    range,
                    style
                        .bg(Color::Green)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                new_offset += len;
            }
        }
    }
    (old, new)
}

const SPACES: &str = "                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";

pub(crate) fn pad_or_truncate_spans<'a>(
    spans: Vec<Span<'a>>,
    target_width: usize,
) -> Vec<Span<'a>> {
    let mut current_len = 0;
    let mut out = Vec::with_capacity(spans.len() + 1);
    for span in spans {
        let span_len = span.content.chars().count();
        if current_len + span_len <= target_width {
            current_len += span_len;
            out.push(span);
        } else {
            let take = target_width.saturating_sub(current_len);
            if take > 0 {
                let byte_end = span
                    .content
                    .char_indices()
                    .nth(take)
                    .map(|(i, _)| i)
                    .unwrap_or(span.content.len());
                let sliced = match span.content {
                    std::borrow::Cow::Borrowed(s) => std::borrow::Cow::Borrowed(&s[..byte_end]),
                    std::borrow::Cow::Owned(mut s) => {
                        s.truncate(byte_end);
                        std::borrow::Cow::Owned(s)
                    }
                };
                out.push(Span {
                    content: sliced,
                    style: span.style,
                });
                current_len += take;
            }
            break;
        }
    }
    if current_len < target_width {
        let pad = target_width - current_len;
        if pad <= SPACES.len() {
            out.push(Span::raw(&SPACES[..pad]));
        } else {
            out.push(Span::raw(" ".repeat(pad)));
        }
    }
    out
}

// Cached rows deliberately contain no cursor/selection styling. Word differences are
// filled lazily by the viewport; only cheap base text participates in wrap preparation.
fn prepare_diff(
    app: &App,
    area: Rect,
    index: &cache::Index,
    mapping: &crate::app::rows::Mapping,
    git: bool,
) -> (String, cache::Rows) {
    let current_file = app.snapshot.files.get(app.selected_file_idx);

    let (title, content_lines) = if !git {
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
            ]
            .into(),
        )
    } else {
        match current_file {
            Some(file) => {
                let file_alerts: Vec<&crate::guard::GuardAlert> =
                    index.file_alerts.iter().map(|&i| &app.alerts[i]).collect();

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

                let title = format!(
                    " {}{}(+{}/-{}){} ",
                    file.display_name(),
                    view_mode_tag,
                    file.additions,
                    file.deletions,
                    alert_badge_title
                );
                let mut lines = cache::Rows::default();

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

                    for (hunk_index, hunk) in file.hunks.iter().enumerate() {
                        let hunk_base = mapping.starts[hunk_index];
                        if hunk.is_folded {
                            let hunk_len = hunk.lines.len();
                            let is_cursor_in_hunk = false;
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
                            lines.tag(cache::Anchor::Fold { raw: hunk_base });
                            continue;
                        }

                        lines.push(Line::from(Span::styled(
                            format!("  {} (Split View: Left=Old, Right=New)", hunk.header),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::ITALIC),
                        )));

                        let mut h_idx = 0;
                        let mut seen_blocks = Vec::new();
                        let fold_map = &mapping.folds[hunk_index];
                        while h_idx < hunk.lines.len() {
                            if let Some(range) = fold_map[h_idx] {
                                if !seen_blocks.contains(&range) {
                                    seen_blocks.push(range);
                                    let is_cursor = false;
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
                                    lines.push(Line::from(vec![Span::styled( // lgtm:ignore
                                        format!(
                                            "  ▶ [Folded AST Block (L{}-L{}, {} lines): {} | zo/za to expand]",
                                            range.start_line, range.end_line, count, snippet
                                        ),
                                        style,
                                    )]));
                                    lines.tag(cache::Anchor::Fold {
                                        raw: hunk_base + h_idx,
                                    });
                                }
                                h_idx += 1;
                                continue;
                            }

                            let line = &hunk.lines[h_idx];
                            match line.line_type {
                                LineType::Context => {
                                    let is_cursor = false;
                                    let in_visual = false;
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
                                        Span::styled(&line.content, style),
                                    ];
                                    let right_spans = vec![
                                        Span::styled(new_no, right_no_style),
                                        Span::styled("  ", Style::default()),
                                        Span::styled(&line.content, style),
                                    ];

                                    let mut row_spans =
                                        pad_or_truncate_spans(left_spans, col_width);
                                    row_spans.push(Span::styled(
                                        " │ ",
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                    row_spans.extend(pad_or_truncate_spans(right_spans, col_width));

                                    lines.push(Line::from(row_spans));
                                    lines.tag(cache::Anchor::Split {
                                        hunk: hunk_index,
                                        old: Some(h_idx),
                                        new: Some(h_idx),
                                        base: hunk_base,
                                        context: true,
                                    });
                                    h_idx += 1;
                                }
                                LineType::Deletion | LineType::Addition => {
                                    let mut dels = Vec::new();
                                    while h_idx < hunk.lines.len()
                                        && hunk.lines[h_idx].line_type == LineType::Deletion
                                        && fold_map[h_idx].is_none()
                                    {
                                        dels.push((h_idx, &hunk.lines[h_idx]));
                                        h_idx += 1;
                                    }
                                    let mut adds = Vec::new();
                                    while h_idx < hunk.lines.len()
                                        && hunk.lines[h_idx].line_type == LineType::Addition
                                        && fold_map[h_idx].is_none()
                                    {
                                        adds.push((h_idx, &hunk.lines[h_idx]));
                                        h_idx += 1;
                                    }
                                    let max_len = dels.len().max(adds.len());
                                    for k in 0..max_len {
                                        let del_opt = dels.get(k);
                                        let add_opt = adds.get(k);
                                        let is_cursor = false;
                                        let in_visual = false;
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
                                            if let Some((_, _add_line)) = add_opt {
                                                left_spans.push(Span::styled(
                                                    &del_line.content,
                                                    Style::default().fg(Color::LightRed),
                                                ));
                                            } else {
                                                let mut style = Style::default().fg(Color::Red);
                                                if let Some(bg) = base_bg {
                                                    style = style.bg(bg);
                                                }
                                                left_spans
                                                    .push(Span::styled(&del_line.content, style));
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
                                            right_spans.push(Span::styled(new_no, right_no_style));
                                            right_spans.push(Span::styled(
                                                "+ ",
                                                Style::default().fg(Color::Green),
                                            ));
                                            if let Some((_, _del_line)) = del_opt {
                                                right_spans.push(Span::styled(
                                                    &add_line.content,
                                                    Style::default().fg(Color::LightGreen),
                                                ));
                                            } else {
                                                let mut style = Style::default().fg(Color::Green);
                                                if let Some(bg) = base_bg {
                                                    style = style.bg(bg);
                                                }
                                                right_spans
                                                    .push(Span::styled(&add_line.content, style));
                                            }
                                        }

                                        let mut row_spans =
                                            pad_or_truncate_spans(left_spans, col_width);
                                        row_spans.push(Span::styled(
                                            " │ ",
                                            Style::default().fg(Color::DarkGray),
                                        ));
                                        row_spans
                                            .extend(pad_or_truncate_spans(right_spans, col_width));

                                        lines.push(Line::from(row_spans));
                                        lines.tag(cache::Anchor::Split {
                                            hunk: hunk_index,
                                            old: del_opt.map(|(i, _)| *i),
                                            new: add_opt.map(|(i, _)| *i),
                                            base: hunk_base,
                                            context: false,
                                        });
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
                    for (hunk_index, hunk) in file.hunks.iter().enumerate() {
                        let hunk_base = mapping.starts[hunk_index];
                        if hunk.is_folded {
                            let hunk_len = hunk.lines.len();
                            let is_cursor_in_hunk = false;
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
                            lines.tag(cache::Anchor::Fold { raw: hunk_base });
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
                        let mut seen_blocks = Vec::new();
                        let fold_map = &mapping.folds[hunk_index];

                        for (h_line_idx, dline) in hunk.lines.iter().enumerate() {
                            if let Some(range) = fold_map[h_line_idx] {
                                if !seen_blocks.contains(&range) {
                                    seen_blocks.push(range);
                                    let is_cursor = false;
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
                                    lines.push(Line::from(vec![Span::styled( // lgtm:ignore
                                        format!(
                                            "  ▶ [Folded AST Block (L{}-L{}, {} lines): {} | zo/za to expand]",
                                            range.start_line, range.end_line, count, snippet
                                        ),
                                        style,
                                    )]));
                                    lines.tag(cache::Anchor::Fold { raw: line_counter });
                                }
                                line_counter += 1;
                                continue;
                            }

                            let is_cursor = false;
                            let in_visual = false;

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
                            let matching_alerts: Vec<&crate::guard::GuardAlert> = line_no
                                .and_then(|n| index.alerts.get(&n))
                                .into_iter()
                                .flatten()
                                .map(|&i| &app.alerts[i])
                                .collect();
                            let matching_comments: Vec<&crate::comment::CommentAnchor> = line_no
                                .and_then(|n| index.comments.get(&n))
                                .into_iter()
                                .flatten()
                                .map(|&i| &app.comments[i])
                                .collect();
                            let thread_state = line_no.and_then(|n| index.threads.get(&n).copied());

                            let matching_drafts: Vec<&lgtm_core::discussion::Draft> = line_no
                                .and_then(|n| index.drafts.get(&n))
                                .into_iter()
                                .flatten()
                                .map(|&i| &app.drafts.items()[i])
                                .collect();

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
                            } else if !matching_drafts.is_empty() {
                                "✎ "
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
                            } else if !matching_drafts.is_empty() {
                                Style::default()
                                    .fg(Color::LightBlue)
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
                                Span::styled(padded_gutter(gutter_icon), icon_style),
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

                            if let Some(&(_, is_del)) = pairs.get(&h_line_idx) {
                                line_spans.push(Span::styled(
                                    &dline.content,
                                    Style::default().fg(if is_del {
                                        Color::LightRed
                                    } else {
                                        Color::LightGreen
                                    }),
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
                            lines.tag(cache::Anchor::Unified {
                                raw: line_counter,
                                hunk: hunk_index,
                                line: h_line_idx,
                                pair: pairs.get(&h_line_idx).copied(),
                            });

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

                            for draft in matching_drafts {
                                let (state, color) = if draft.error.is_some() {
                                    ("failed", Color::Red)
                                } else if draft.stale.is_some() {
                                    ("stale", Color::Yellow)
                                } else {
                                    ("pending", Color::LightBlue)
                                };
                                lines.push(Line::from(vec![
                                    Span::raw("        │ "),
                                    Span::styled(
                                        format!("✎ DRAFT #{} ({state}): ", draft.id),
                                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(
                                        format!("\"{}\"", draft.body),
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

                                let comment_spans = vec![
                                    Span::raw("        │ "),
                                    Span::styled(badge, b_style),
                                    Span::styled(format!("\"{}\"", comment.comment_text), t_style),
                                ];
                                lines.push(Line::from(comment_spans));

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
            None => (" Working Tree Clean ".to_string(), landing_lines().into()),
        }
    };

    (title, content_lines)
}

const LOGO: [&str; 6] = [
    "██╗      ██████╗ ████████╗███╗   ███╗",
    "██║     ██╔════╝ ╚══██╔══╝████╗ ████║",
    "██║     ██║  ███╗   ██║   ██╔████╔██║",
    "██║     ██║   ██║   ██║   ██║╚██╔╝██║",
    "███████╗╚██████╔╝   ██║   ██║ ╚═╝ ██║",
    "╚══════╝ ╚═════╝    ╚═╝   ╚═╝     ╚═╝",
];

/// Landing screen shown when the working tree is clean.
pub(super) fn landing_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![Line::from(""), Line::from("")];
    lines.extend(LOGO.iter().map(|row| {
        Line::from(Span::styled(
            *row,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ))
        .centered()
    }));
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            "looks good to me",
            Style::default().add_modifier(Modifier::ITALIC),
        ))
        .centered(),
    );
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            "Fast, real-time terminal diff reviewer, AI agent steering TUI & CI security scanner.",
            Style::default().fg(Color::DarkGray),
        ))
        .centered(),
    );
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            "? help  ·  : commands",
            Style::default().fg(Color::DarkGray),
        ))
        .centered(),
    );
    lines
}

pub(super) fn update_diff_scroll_offset(
    scroll_offset: &mut usize,
    content_len: usize,
    inner_height: usize,
    active_rows: Option<(usize, usize)>,
) {
    if inner_height == 0 {
        return;
    }
    if content_len <= inner_height {
        *scroll_offset = 0;
        return;
    }
    // Show the whole active row when it fits; its first line always wins when it does not.
    if let Some((start, end)) = active_rows {
        *scroll_offset = (*scroll_offset)
            .max(end.saturating_sub(inner_height))
            .min(start);
    }
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
    fn test_prepare_inline_diff_ranges() {
        let old = "let x = 1;";
        let new = "let x = 2;";
        let (old_ranges, new_ranges) = prepare_inline_diff_ranges(old, new, None);
        let old_reconstructed: String = old_ranges.iter().map(|(r, _)| &old[r.clone()]).collect();
        let new_reconstructed: String = new_ranges.iter().map(|(r, _)| &new[r.clone()]).collect();
        assert_eq!(old_reconstructed, old);
        assert_eq!(new_reconstructed, new);
    }

    #[test]
    fn test_render_diff_view_auto_scrolls_to_active_line() {
        let mut app = crate::app::test_util::create_test_app();
        app.git_engine = lgtm_core::git::GitEngine::new(".");
        crate::app::test_util::wait_for_view_data(&mut app);
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
        let screen = format!("{:?}", terminal.backend().buffer());
        assert!(
            screen.contains('▲') && screen.contains('▼'),
            "scrollbar visible"
        );

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
        crate::app::test_util::wait_for_view_data(&mut app);
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

    #[test]
    fn scroll_keeps_the_whole_wrapped_cursor_row_visible() {
        let mut off = 0;
        update_diff_scroll_offset(&mut off, 100, 10, Some((12, 15)));
        assert_eq!(off, 5);
        let mut off = 50;
        update_diff_scroll_offset(&mut off, 100, 10, Some((12, 15)));
        assert_eq!(off, 12);
        let mut off = 0;
        update_diff_scroll_offset(&mut off, 100, 10, Some((20, 40)));
        assert_eq!(off, 20, "a row taller than the view shows its start");
    }

    fn screen_text(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render_body(f, f.area(), app)).unwrap();
        let buf = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn diff_file(name: &str, hunks: Vec<DiffHunk>) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: std::path::PathBuf::from(name),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 1,
            hunks,
            is_folded: false,
            is_reviewed: false,
        }
    }

    fn line(t: LineType, text: &str, old: Option<u32>, new: Option<u32>) -> DiffLine {
        DiffLine {
            line_type: t,
            content: text.into(),
            old_line_num: old,
            new_line_num: new,
        }
    }

    fn hunk(header: &str, folded: bool, lines: Vec<DiffLine>) -> DiffHunk {
        DiffHunk {
            header: header.into(),
            old_start: 1,
            old_lines: lines.len() as u32,
            new_start: 1,
            new_lines: lines.len() as u32,
            lines,
            is_folded: folded,
            folded_blocks: Vec::new(),
        }
    }

    #[test]
    fn drawer_reports_non_git_and_clean_states() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = false;
        assert!(screen_text(&mut app, 120, 10).contains("(No git repository)"));

        app.view_data.is_git = true;
        let out = screen_text(&mut app, 120, 10);
        assert!(out.contains("(Working tree clean)"));
    }

    #[test]
    fn drawer_lists_files_with_review_alert_and_stat_markers() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        let h = hunk(
            "@@ -1,1 +1,1 @@",
            false,
            vec![line(LineType::Context, "x", Some(1), Some(1))],
        );
        let mut reviewed = diff_file("done.rs", vec![h.clone()]);
        reviewed.is_reviewed = true;
        let mut added = diff_file("new.rs", vec![h.clone()]);
        added.change_type = ChangeType::Added;
        added.additions = 0;
        added.deletions = 0;
        app.snapshot = DiffSnapshot::new(vec![reviewed, diff_file("main.rs", vec![h]), added]);
        app.alerts = vec![crate::guard::GuardAlert {
            rule_id: "r".into(),
            fingerprint: "f".into(),
            severity: crate::guard::Severity::Warn,
            title: "t".into(),
            description: "d".into(),
            line_no: None,
            file_path: "main.rs".into(),
        }];
        app.selected_file_idx = 1;
        let out = screen_text(&mut app, 120, 12);
        let row = |name: &str| {
            out.lines()
                .map(|l| l.chars().take(35).collect::<String>())
                .find(|l| l.contains(name))
                .unwrap_or_else(|| panic!("{name} not in drawer:\n{out}"))
        };
        assert!(row("done.rs").contains("✓"));
        assert!(row("main.rs").contains("⚠") && !row("main.rs").contains("✓"));
        assert!(row("new.rs").contains("[A]"));
        assert!(out.contains("Changed Files [3]"));
        assert!(out.contains("+2/-1"));
    }

    #[test]
    fn narrow_body_skips_drawer_and_markdown_body_uses_preview() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        app.show_file_drawer = true;
        app.snapshot = DiffSnapshot::new(vec![diff_file("a.rs", vec![])]);
        assert!(!screen_text(&mut app, 40, 10).contains("Changed Files"));
        assert!(screen_text(&mut app, 120, 10).contains("(Binary file or empty diff)"));

        app.snapshot = DiffSnapshot::new(vec![diff_file("README.md", vec![])]);
        app.active_view = ActiveView::MarkdownPreview;
        assert!(screen_text(&mut app, 120, 10).contains("Changed Files"));
        assert!(screen_text(&mut app, 60, 10).contains("Changed Files"));
        assert!(!screen_text(&mut app, 40, 10).contains("Changed Files"));
    }

    #[test]
    fn audit_view_routes_to_audit_report() {
        let mut app = crate::app::test_util::create_test_app();
        app.active_view = ActiveView::AuditReport;
        let out = screen_text(&mut app, 120, 12);
        assert!(out.contains("Repository Issues (0/0)"));
        assert!(out.contains("No issue selected"));
    }

    #[test]
    fn split_and_unified_render_folded_hunks_and_alert_banners() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        app.show_file_drawer = false;
        let live = hunk(
            "@@ -1,3 +1,3 @@",
            false,
            vec![
                line(LineType::Context, "keep_me", Some(1), Some(1)),
                line(LineType::Deletion, "old_value", Some(2), None),
                line(LineType::Addition, "new_value", None, Some(2)),
            ],
        );
        let folded = hunk(
            "@@ -10,2 +10,2 @@",
            true,
            vec![line(LineType::Context, "hidden_text", Some(10), Some(10))],
        );
        app.snapshot = DiffSnapshot::new(vec![diff_file("a.rs", vec![live, folded])]);
        app.alerts = vec![
            crate::guard::GuardAlert {
                rule_id: "r".into(),
                fingerprint: "f1".into(),
                severity: crate::guard::Severity::High,
                title: "File wide".into(),
                description: "whole file".into(),
                line_no: None,
                file_path: "a.rs".into(),
            },
            crate::guard::GuardAlert {
                rule_id: "r".into(),
                fingerprint: "f2".into(),
                severity: crate::guard::Severity::Critical,
                title: "Line bad".into(),
                description: "on the line".into(),
                line_no: Some(2),
                file_path: "a.rs".into(),
            },
        ];

        let unified = screen_text(&mut app, 120, 20);
        assert!(unified.contains("[Unified]"));
        assert!(unified.contains("2 Alert(s)"));
        assert!(unified.contains("File wide: whole file"));
        assert!(unified.contains("Line bad: on the line"));
        assert!(unified.contains("new_value"));
        assert!(unified.contains("Folded - 1 lines"));
        assert!(!unified.contains("hidden_text"));

        app.is_split_view = true;
        let split = screen_text(&mut app, 120, 20);
        assert!(split.contains("[Split]"));
        assert!(split.contains("Split View: Left=Old, Right=New"));
        assert!(split.contains("keep_me"));
        assert!(split.contains("old_value") && split.contains("new_value"));
        assert!(split.contains("Folded - 1 lines"));
        assert!(!split.contains("hidden_text"));

        let narrow = screen_text(&mut app, 50, 20);
        assert!(narrow.contains("[Unified]"));
    }

    #[test]
    fn folded_ast_blocks_collapse_in_both_layouts() {
        let mut app = crate::app::test_util::create_test_app();
        app.view_data.is_git = true;
        app.show_file_drawer = false;
        let mut h = hunk(
            "@@ -1,4 +1,4 @@",
            false,
            vec![
                line(LineType::Context, "fn big() {", Some(1), Some(1)),
                line(LineType::Context, "secret_body();", Some(2), Some(2)),
                line(LineType::Context, "}", Some(3), Some(3)),
                line(LineType::Context, "after();", Some(4), Some(4)),
            ],
        );
        h.folded_blocks = vec![BlockRange {
            start_line: 1,
            end_line: 3,
        }];
        app.snapshot = DiffSnapshot::new(vec![diff_file("a.rs", vec![h])]);

        for split in [false, true] {
            app.is_split_view = split;
            let out = screen_text(&mut app, 120, 14);
            assert!(
                out.contains("Folded AST Block (L1-L3, 3 lines): fn big() {"),
                "{out}"
            );
            assert!(!out.contains("secret_body"));
            assert!(out.contains("after();"));
        }
    }

    #[test]
    fn landing_shows_logo_tagline_and_purpose() {
        let text: String = landing_lines()
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("███████╗╚██████╔╝"));
        assert!(text.contains("looks good to me"));
        assert!(text.contains("terminal diff reviewer"));
    }
}

#[cfg(test)]
mod regression;
