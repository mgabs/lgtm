//! In-terminal TUI renderer for the rendered markdown preview view.

use crate::app::{
    markdown::{MarkdownDiffKind, RenderedMarkdownRow},
    App,
};
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub(crate) fn render_markdown_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let file_opt = app.snapshot.files.get(app.selected_file_idx);
    let (file_name, additions, deletions) = match file_opt {
        Some(f) => (f.display_name(), f.additions, f.deletions),
        None => ("No file selected".to_string(), 0, 0),
    };

    if let Some(file) = file_opt {
        let width = area.width;
        let needs_rebuild = app.markdown_preview.rendered_rows.is_empty()
            || app.markdown_preview.cached_file_idx != Some(app.selected_file_idx)
            || app.markdown_preview.cached_width != width;

        if needs_rebuild {
            app.markdown_preview.rendered_rows =
                crate::app::markdown::build_markdown_preview(file, width);
            app.markdown_preview.cached_file_idx = Some(app.selected_file_idx);
            app.markdown_preview.cached_width = width;
            app.markdown_preview.scroll_offset = 0;
        }
    }

    let title = format!(
        " [MARKDOWN PREVIEW] {} (+{} -{}) ",
        file_name, additions, deletions
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.height == 0 || inner_area.width == 0 {
        return;
    }

    let viewport_height = inner_area.height as usize;
    app.markdown_preview.viewport_height = viewport_height;

    let side =
        |c: &lgtm_core::comment::CommentAnchor| c.side == lgtm_core::comment::CommentSide::Left;
    let file_comments: Vec<_> = app
        .comments
        .iter()
        .filter(|c| !c.orphaned && c.file_path == file_name)
        .collect();
    let directives_for = |row: &RenderedMarkdownRow| -> Vec<&lgtm_core::comment::CommentAnchor> {
        let deleted = row.diff_kind == MarkdownDiffKind::Deletion;
        match row.line_number {
            Some(n) => file_comments
                .iter()
                .copied()
                .filter(|c| c.target_line_no == n && side(c) == deleted)
                .collect(),
            None => Vec::new(),
        }
    };

    let heights: Vec<usize> = app
        .markdown_preview
        .rendered_rows
        .iter()
        .map(|r| {
            1 + directives_for(r)
                .iter()
                .map(|c| 1 + c.replies.len())
                .sum::<usize>()
        })
        .collect();
    app.markdown_preview.row_heights = heights;

    let total_rows = app.markdown_preview.rendered_rows.len();
    let scroll = app
        .markdown_preview
        .scroll_offset
        .min(app.markdown_preview.max_scroll(viewport_height));
    app.markdown_preview.scroll_offset = scroll;

    let mut lines = Vec::new();
    for row in app.markdown_preview.rendered_rows.iter().skip(scroll) {
        if lines.len() >= viewport_height {
            break;
        }
        let (marker_char, marker_color) = match row.diff_kind {
            MarkdownDiffKind::Addition => ('+', Color::Green),
            MarkdownDiffKind::Deletion => ('-', Color::Red),
            MarkdownDiffKind::Unchanged => (' ', Color::DarkGray),
        };
        let line_num_str = match row.line_number {
            Some(n) => format!("{:4}", n),
            None => "    ".to_string(),
        };
        let mut row_spans = vec![Span::styled(
            format!("{} {} │ ", marker_char, line_num_str),
            Style::default().fg(marker_color),
        )];
        row_spans.extend(row.spans.clone());
        lines.push(Line::from(row_spans));

        for c in directives_for(row) {
            let (icon, color) = if c.is_resolved {
                ("✅", Color::DarkGray)
            } else {
                ("💬", Color::Yellow)
            };
            lines.push(Line::from(Span::styled(
                format!("       │ {icon} {}", c.comment_text),
                Style::default().fg(color),
            )));
            for reply in &c.replies {
                lines.push(Line::from(Span::styled(
                    format!("       │   ↳ {reply}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    if total_rows > 0 {
        let last = (scroll + lines.len()).min(total_rows);
        let hint = format!(" {}-{}/{} ", scroll + 1, last.max(scroll + 1), total_rows);
        frame.render_widget(
            Paragraph::new(Line::from(hint).right_aligned()),
            Rect {
                y: area.y + area.height.saturating_sub(1),
                height: 1,
                ..area
            },
        );
        if total_rows > viewport_height {
            let mut bar = ScrollbarState::new(app.markdown_preview.max_scroll(viewport_height) + 1)
                .position(scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut bar,
            );
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::markdown::{MarkdownDiffKind, RenderedMarkdownRow};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_markdown_preview_empty_area() {
        let backend = TestBackend::new(0, 0);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 0, 0);

        let mut app = crate::app::test_util::create_test_app();
        terminal
            .draw(|f| {
                render_markdown_preview(f, area, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_markdown_preview_with_rows() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 80, 24);

        let mut app = crate::app::test_util::create_test_app();
        app.markdown_preview.rendered_rows = vec![
            RenderedMarkdownRow {
                line_number: Some(1),
                diff_kind: MarkdownDiffKind::Unchanged,
                spans: vec![Span::raw("Heading line")],
            },
            RenderedMarkdownRow {
                line_number: Some(2),
                diff_kind: MarkdownDiffKind::Addition,
                spans: vec![Span::raw("Added bullet point")],
            },
        ];

        terminal
            .draw(|f| {
                render_markdown_preview(f, area, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let output = format!("{:?}", buffer);
        assert!(output.contains("MARKDOWN PREVIEW"));
    }

    #[test]
    fn directives_render_under_their_line() {
        let mut app = crate::app::test_util::create_test_app();
        app.snapshot.files = vec![crate::model::FileDiff {
            old_path: None,
            new_path: "a.md".into(),
            change_type: crate::model::ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: Vec::new(),
            is_folded: false,
            is_reviewed: false,
        }];
        let row = |kind| RenderedMarkdownRow {
            line_number: Some(3),
            diff_kind: kind,
            spans: vec![Span::raw("body")],
        };
        app.markdown_preview.rendered_rows = vec![
            row(MarkdownDiffKind::Unchanged),
            row(MarkdownDiffKind::Deletion),
        ];
        app.markdown_preview.cached_file_idx = Some(0);
        app.markdown_preview.cached_width = 80;
        let open = lgtm_core::comment::CommentAnchor {
            id: String::new(),
            file_path: "a.md".into(),
            target_line_no: 3,
            target_code: String::new(),
            context_before: Vec::new(),
            context_after: Vec::new(),
            comment_text: "open one".into(),
            replies: vec!["a reply".into()],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            side: lgtm_core::comment::CommentSide::Right,
            track: Default::default(),
        };
        let mut done = open.clone();
        done.comment_text = "done one".into();
        done.is_resolved = true;
        let mut old_side = open.clone();
        old_side.comment_text = "old side".into();
        old_side.side = lgtm_core::comment::CommentSide::Left;
        app.comments = vec![open, done, old_side];

        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        terminal
            .draw(|f| render_markdown_preview(f, f.area(), &mut app))
            .unwrap();
        let out = format!("{:?}", terminal.backend().buffer());
        assert!(out.contains("💬 open one"));
        assert!(out.contains("↳ a reply"));
        assert!(out.contains("✅ done one"));
        assert_eq!(out.matches("old side").count(), 1);
        assert_eq!(out.matches("open one").count(), 1);
    }

    #[test]
    fn scrolling_to_the_end_uses_the_rendered_height() {
        let mut app = crate::app::test_util::create_test_app();
        app.markdown_preview.rendered_rows = (1..=100)
            .map(|n| RenderedMarkdownRow {
                line_number: Some(n),
                diff_kind: MarkdownDiffKind::Unchanged,
                spans: vec![Span::raw("x")],
            })
            .collect();
        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        terminal
            .draw(|f| render_markdown_preview(f, f.area(), &mut app))
            .unwrap();
        assert_eq!(app.markdown_preview.viewport_height, 10);
        app.markdown_preview.rendered_rows.truncate(100);
        let vh = app.markdown_preview.viewport_height;
        app.markdown_preview.scroll_to_bottom(vh);
        assert_eq!(app.markdown_preview.scroll_offset, 90);
    }

    #[test]
    fn end_of_document_stays_visible_with_directives_and_shows_position() {
        let mut app = crate::app::test_util::create_test_app();
        app.snapshot.files = vec![crate::model::FileDiff {
            old_path: None,
            new_path: "a.md".into(),
            change_type: crate::model::ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: Vec::new(),
            is_folded: false,
            is_reviewed: false,
        }];
        app.markdown_preview.rendered_rows = (1..=30)
            .map(|n| RenderedMarkdownRow {
                line_number: Some(n),
                diff_kind: MarkdownDiffKind::Unchanged,
                spans: vec![Span::raw(format!("row{n}"))],
            })
            .collect();
        app.markdown_preview.cached_file_idx = Some(0);
        app.markdown_preview.cached_width = 80;
        let mut last = lgtm_core::comment::CommentAnchor {
            id: String::new(),
            file_path: "a.md".into(),
            target_line_no: 30,
            target_code: String::new(),
            context_before: Vec::new(),
            context_after: Vec::new(),
            comment_text: "final note".into(),
            replies: vec!["r1".into(), "r2".into()],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            side: lgtm_core::comment::CommentSide::Right,
            track: Default::default(),
        };
        last.replies.push("r3".into());
        app.comments = vec![last];

        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        let mut draw = |app: &mut crate::app::App| {
            terminal
                .draw(|f| render_markdown_preview(f, f.area(), app))
                .unwrap();
            format!("{:?}", terminal.backend().buffer())
        };
        draw(&mut app);
        let vh = app.markdown_preview.viewport_height;
        app.markdown_preview.scroll_to_bottom(vh);
        let out = draw(&mut app);
        assert!(out.contains("row30"));
        assert!(out.contains("↳ r3"));
        assert!(out.contains("/30"));

        app.markdown_preview.scroll_offset = 10_000;
        draw(&mut app);
        assert_eq!(
            app.markdown_preview.scroll_offset,
            app.markdown_preview.max_scroll(vh)
        );
    }
}
