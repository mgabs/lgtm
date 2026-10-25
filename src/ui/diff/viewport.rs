use super::*;
use cache::{Anchor, Layout, LayoutKey, RenderCache};

pub(super) fn padded_gutter(icon: &str) -> String {
    format!(
        "{icon}{}",
        " ".repeat(3usize.saturating_sub(Span::raw(icon).width()))
    )
}

fn background(cursor: bool, visual: bool) -> Option<Color> {
    match (cursor, visual) {
        (true, true) => Some(Color::Indexed(241)),
        (false, true) => Some(Color::Indexed(238)),
        (true, false) => Some(Color::Indexed(237)),
        _ => None,
    }
}

fn shade(spans: &mut [Span<'_>], bg: Option<Color>, bold: bool) {
    for span in spans {
        if span.style.bg.is_none() {
            if let Some(bg) = bg {
                span.style = span.style.bg(bg);
            }
        }
        if bold {
            span.style = span.style.add_modifier(Modifier::BOLD);
        }
    }
}

fn number(value: Option<u32>, cursor: bool) -> Span<'static> {
    let text = match (value, cursor) {
        (Some(n), true) => format!("▶{n:3} "),
        (Some(n), false) => format!(" {n:3} "),
        (None, true) => "▶    ".into(),
        (None, false) => "     ".into(),
    };
    Span::styled(
        text,
        if cursor {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        },
    )
}

fn split_row<'a>(
    app: &'a App,
    cache: &mut RenderCache,
    anchor: Anchor,
    width: u16,
    canonical: usize,
) -> Line<'a> {
    let Anchor::Split {
        hunk,
        old,
        new,
        base,
        context,
    } = anchor
    else {
        unreachable!()
    };
    let lines = &app.snapshot.files[app.selected_file_idx].hunks[hunk].lines;
    let cursor = old.into_iter().chain(new).any(|i| base + i == canonical);
    let visual = old
        .into_iter()
        .chain(new)
        .any(|i| app.is_in_visual_selection(base + i));
    let bg = background(cursor, visual);
    let mut left = Vec::new();
    let mut right = Vec::new();
    if let Some(i) = old {
        let line = &lines[i];
        let old_no = line
            .old_line_num
            .map(|n| {
                if context {
                    format!(" {n:3} ")
                } else {
                    format!("{n:4} ")
                }
            })
            .unwrap_or_else(|| "     ".into());
        left.push(Span::styled(old_no, Style::default().fg(Color::DarkGray)));
        left.push(Span::styled(
            if context { "  " } else { "- " },
            Style::default().fg(if context { Color::Reset } else { Color::Red }),
        ));
        let mut body = match (context, new) {
            (false, Some(new)) => cache.word_spans(app, hunk, i, new, true),
            _ => vec![Span::styled(
                &line.content,
                Style::default().fg(if context { Color::White } else { Color::Red }),
            )],
        };
        shade(&mut body, bg, context && cursor);
        left.extend(body);
    }
    if let Some(i) = new {
        let line = &lines[i];
        right.push(number(line.new_line_num, cursor));
        right.push(Span::styled(
            if context { "  " } else { "+ " },
            Style::default().fg(if context { Color::Reset } else { Color::Green }),
        ));
        let mut body = match (context, old) {
            (false, Some(old)) => cache.word_spans(app, hunk, old, i, false),
            _ => vec![Span::styled(
                &line.content,
                Style::default().fg(if context { Color::White } else { Color::Green }),
            )],
        };
        shade(&mut body, bg, context && cursor);
        right.extend(body);
    }
    let columns = (width.saturating_sub(6) / 2) as usize;
    let mut spans = pad_or_truncate_spans(left, columns);
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.extend(pad_or_truncate_spans(right, columns));
    Line::from(spans)
}

pub(super) fn styled_row<'a>(
    app: &'a App,
    cache: &mut RenderCache,
    row: &'a Line<'static>,
    anchor: Option<Anchor>,
    width: u16,
    canonical: usize,
) -> Line<'a> {
    if let Some(anchor @ Anchor::Split { .. }) = anchor {
        return split_row(app, cache, anchor, width, canonical);
    }
    match anchor {
        Some(Anchor::Fold { raw }) if raw == canonical => {
            let spans: Vec<Span<'a>> = row
                .spans
                .iter()
                .map(|s| Span::styled(s.content.as_ref(), s.style.bg(Color::Indexed(237))))
                .collect();
            let mut line = Line::from(spans);
            line.alignment = row.alignment;
            line.style = row.style;
            line
        }
        Some(Anchor::Unified {
            raw,
            hunk,
            line,
            pair,
        }) => {
            let cursor = canonical == raw;
            let visual = app.is_in_visual_selection(raw);
            let mut spans: Vec<Span<'a>> = if let Some((other, deletion)) = pair {
                let mut s: Vec<Span<'a>> = row.spans[..4]
                    .iter()
                    .map(|span| Span::styled(span.content.as_ref(), span.style))
                    .collect();
                s.extend(if deletion {
                    cache.word_spans(app, hunk, line, other, true)
                } else {
                    cache.word_spans(app, hunk, other, line, false)
                });
                s
            } else {
                row.spans
                    .iter()
                    .map(|span| Span::styled(span.content.as_ref(), span.style))
                    .collect()
            };
            if cursor {
                let mut s = spans[0].content.to_string();
                s.replace_range(0..1, "▶");
                spans[0] = Span::styled(
                    s,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            }
            if visual {
                spans[1] = Span::styled(
                    padded_gutter("▌ "),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            }
            shade(
                &mut spans[4..],
                background(cursor, visual),
                cursor && pair.is_none(),
            );
            let mut line = Line::from(spans);
            line.alignment = row.alignment;
            line.style = row.style;
            line
        }
        _ => {
            let spans: Vec<Span<'a>> = row
                .spans
                .iter()
                .map(|s| Span::styled(s.content.as_ref(), s.style))
                .collect();
            let mut line = Line::from(spans);
            line.alignment = row.alignment;
            line.style = row.style;
            line
        }
    }
}

pub(super) fn render_diff_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let mapping = app.cached_rows();
    let canonical = mapping
        .representatives
        .get(app.cursor_line_idx)
        .copied()
        .unwrap_or(app.cursor_line_idx);
    let key = LayoutKey {
        source: crate::app::rows::Key::of(app),
        width: area.width,
        split: app.is_split_view && area.width >= 60,
        annotations: cache::annotation_signature(app),
        git: app.view_data.is_git,
    };
    let mut cache = app.render_cache.borrow_mut();
    cache.prepare_content(key.source.content());
    if cache.layout.as_ref().is_none_or(|old| old.key != key) {
        let mut timing = lgtm_core::debug::Span::new("ui.layout_prepare");
        let index = cache.index(app, key.annotations);
        let (title, mut rows) = prepare_diff(app, area, &index, &mapping, key.git);
        rows.wrap(inner.width);
        timing.count("rows", rows.len() as u64);
        cache.layouts_prepared += 1;
        cache.layout = Some(Layout { key, title, rows });
    }
    let layout = cache.layout.take().unwrap();
    let rows = &layout.rows;
    let mut timing = lgtm_core::debug::Span::new("ui.diff_viewport");
    let active = rows
        .source_rows
        .get(&canonical)
        .map(|&row| rows.offsets[row]);
    let height = inner.height as usize;
    app.diff_scroll_offset = app
        .diff_scroll_offset
        .min(rows.total_height().saturating_sub(height));
    update_diff_scroll_offset(
        &mut app.diff_scroll_offset,
        rows.total_height(),
        height,
        active,
    );
    let title = app
        .current_hunk_info()
        .and_then(|(_, h, _)| h.header.split("@@").nth(2))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(
            || layout.title.clone(),
            |scope| format!("{} │ 📍 {scope} ", layout.title.trim_end()),
        );
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan)),
        area,
    );
    frame.render_widget(Clear, inner);
    let mut logical = rows
        .offsets
        .partition_point(|&offset| offset <= app.diff_scroll_offset)
        .saturating_sub(1);
    let mut skip = app
        .diff_scroll_offset
        .saturating_sub(rows.offsets.get(logical).copied().unwrap_or(0));
    let mut y = 0usize;
    let mut drawn = 0usize;
    while logical < rows.lines.len() && y < height {
        let row_height = rows.offsets[logical + 1] - rows.offsets[logical];
        let count = (row_height - skip).min(height - y);
        let line = styled_row(
            app,
            &mut cache,
            &rows.lines[logical],
            rows.anchors[logical],
            area.width,
            canonical,
        );
        frame.render_widget(
            Paragraph::new(line)
                .wrap(Wrap { trim: false })
                .scroll((skip.min(u16::MAX as usize) as u16, 0)),
            Rect::new(inner.x, inner.y + y as u16, inner.width, count as u16),
        );
        drawn += 1;
        y += count;
        logical += 1;
        skip = 0;
    }
    cache.viewport_rows = drawn;
    cache.layout = Some(layout);
    timing.count("rows", drawn as u64);
}
