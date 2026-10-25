//! In-terminal TUI renderer for the rendered markdown preview view.

use crate::app::{markdown::MarkdownDiffKind, App};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
    let total_rows = app.markdown_preview.rendered_rows.len();
    let max_scroll = total_rows.saturating_sub(viewport_height);
    let scroll = app.markdown_preview.scroll_offset.min(max_scroll);

    let visible_rows = app
        .markdown_preview
        .rendered_rows
        .iter()
        .skip(scroll)
        .take(viewport_height);

    let mut lines = Vec::new();
    for row in visible_rows {
        let mut row_spans = Vec::new();

        let (marker_char, marker_color) = match row.diff_kind {
            MarkdownDiffKind::Addition => ('+', Color::Green),
            MarkdownDiffKind::Deletion => ('-', Color::Red),
            MarkdownDiffKind::Unchanged => (' ', Color::DarkGray),
        };

        let line_num_str = match row.line_number {
            Some(n) => format!("{:4}", n),
            None => "    ".to_string(),
        };

        let gutter_text = format!("{} {} │ ", marker_char, line_num_str);
        let gutter_style = Style::default().fg(marker_color);
        row_spans.push(Span::styled(gutter_text, gutter_style));
        row_spans.extend(row.spans.clone());
        lines.push(Line::from(row_spans));
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
}
