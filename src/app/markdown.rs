//! Markdown parser, reflower, and diff highlighter for in-terminal preview.

use crate::model::{DiffLine, FileDiff, LineType};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use std::path::Path;

/// Diff status for a rendered markdown row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownDiffKind {
    Unchanged,
    Addition,
    Deletion,
}

/// A single rendered line within the markdown preview viewport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedMarkdownRow {
    pub line_number: Option<u32>,
    pub diff_kind: MarkdownDiffKind,
    pub spans: Vec<Span<'static>>,
}

/// Persistent state for the markdown preview view.
#[derive(Debug, Default, Clone)]
pub struct MarkdownPreviewState {
    pub scroll_offset: usize,
    pub rendered_rows: Vec<RenderedMarkdownRow>,
    pub cached_file_idx: Option<usize>,
    pub cached_width: u16,
    /// Rows visible in the last render; scrolling clamps against it.
    pub viewport_height: usize,
    /// Screen lines per row (the row plus its directives), set by the renderer.
    pub row_heights: Vec<usize>,
}

impl MarkdownPreviewState {
    fn height(&self, row: usize) -> usize {
        self.row_heights.get(row).copied().unwrap_or(1)
    }

    /// Largest offset that still fills the viewport, counting directive rows under each line.
    pub fn max_scroll(&self, viewport_height: usize) -> usize {
        if self.row_heights.is_empty() {
            return self.rendered_rows.len().saturating_sub(viewport_height);
        }
        let mut used = 0;
        let mut first = self.rendered_rows.len();
        while first > 0 && used + self.height(first - 1) <= viewport_height {
            first -= 1;
            used += self.height(first);
        }
        first
    }

    pub fn scroll_down(&mut self, amount: usize, viewport_height: usize) {
        self.scroll_offset = (self.scroll_offset + amount).min(self.max_scroll(viewport_height));
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        self.scroll_offset = self.max_scroll(viewport_height);
    }
}

/// Returns true if the file path has a recognized markdown extension.
pub fn is_markdown_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "mdown" | "mkd"
    )
}

/// Builds the complete list of rendered markdown rows from a file diff.
pub fn build_markdown_preview(file: &FileDiff, viewport_width: u16) -> Vec<RenderedMarkdownRow> {
    let text_width = (viewport_width.saturating_sub(10)).max(20) as usize;

    let total_lines: usize = file.hunks.iter().map(|h| h.lines.len()).sum();
    let mut all_lines: Vec<&DiffLine> = Vec::with_capacity(total_lines);
    for hunk in &file.hunks {
        for line in &hunk.lines {
            if line.line_type != LineType::HunkHeader {
                all_lines.push(line);
            }
        }
    }

    if all_lines.is_empty() {
        return vec![RenderedMarkdownRow {
            line_number: None,
            diff_kind: MarkdownDiffKind::Unchanged,
            spans: vec![Span::styled(
                "(Empty markdown document)",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )],
        }];
    }

    let mut rows = Vec::with_capacity(all_lines.len());
    let mut in_code_fence = false;

    for line in all_lines {
        let trimmed = line.content.trim_end_matches(['\r', '\n']);
        let diff_kind = match line.line_type {
            LineType::Addition => MarkdownDiffKind::Addition,
            LineType::Deletion => MarkdownDiffKind::Deletion,
            _ => MarkdownDiffKind::Unchanged,
        };
        let line_num = line.new_line_num.or(line.old_line_num);

        // Check for code fence start/end: ``` or ~~~
        let trimmed_start = trimmed.trim_start();
        if trimmed_start.starts_with("```") || trimmed_start.starts_with("~~~") {
            let marker = if trimmed_start.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            if !in_code_fence {
                in_code_fence = true;
                let lang = trimmed_start.trim_start_matches(marker).trim();
                let code_fence_lang = if lang.is_empty() {
                    "code".to_string()
                } else {
                    lang.to_string()
                };
                let header_title = format!("─── [{}] ───", code_fence_lang);
                let header_style = apply_diff_style(
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
                    diff_kind,
                );
                rows.push(RenderedMarkdownRow {
                    line_number: line_num,
                    diff_kind,
                    spans: vec![Span::styled(header_title, header_style)],
                });
                continue;
            } else {
                in_code_fence = false;
                let footer_style = apply_diff_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                    diff_kind,
                );
                rows.push(RenderedMarkdownRow {
                    line_number: line_num,
                    diff_kind,
                    spans: vec![Span::styled("───────────────", footer_style)],
                });
                continue;
            }
        }

        if in_code_fence {
            let code_style = apply_diff_style(
                Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 35)),
                diff_kind,
            );
            rows.push(RenderedMarkdownRow {
                line_number: line_num,
                diff_kind,
                spans: vec![Span::styled(format!("  {}", trimmed), code_style)],
            });
            continue;
        }

        // Horizontal Rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let hr_style = apply_diff_style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
                diff_kind,
            );
            let bar = "─".repeat(text_width.min(40));
            rows.push(RenderedMarkdownRow {
                line_number: line_num,
                diff_kind,
                spans: vec![Span::styled(bar, hr_style)],
            });
            continue;
        }

        // Heading: # .. ######
        if let Some((level, heading_text)) = parse_heading(trimmed) {
            let heading_style = get_heading_style(level, diff_kind);
            let mut spans = parse_inline_spans(heading_text, heading_style);
            // Prefix level indicator
            let prefix_marker = format!("{} ", "#".repeat(level));
            let prefix_style = heading_style.add_modifier(Modifier::DIM);
            spans.insert(0, Span::styled(prefix_marker, prefix_style));

            rows.push(RenderedMarkdownRow {
                line_number: line_num,
                diff_kind,
                spans,
            });
            continue;
        }

        // Blockquote: >
        if trimmed_start.starts_with('>') {
            let quote_content = trimmed_start.trim_start_matches('>').trim();
            let base_style = apply_diff_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::ITALIC),
                diff_kind,
            );
            let wrapped_lines = wrap_text(quote_content, text_width.saturating_sub(4));
            for (idx, w_line) in wrapped_lines.into_iter().enumerate() {
                let gutter_style = apply_diff_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                    diff_kind,
                );
                let mut spans = vec![Span::styled("│ ", gutter_style)];
                spans.extend(parse_inline_spans(&w_line, base_style));
                rows.push(RenderedMarkdownRow {
                    line_number: if idx == 0 { line_num } else { None },
                    diff_kind,
                    spans,
                });
            }
            continue;
        }

        // List item: bullet or numbered
        if let Some((prefix, list_content, indent_depth)) = parse_list_item(trimmed) {
            let base_style = apply_diff_style(Style::default().fg(Color::White), diff_kind);
            let indent_spaces = "  ".repeat(indent_depth);
            let prefix_len = indent_spaces.chars().count() + prefix.chars().count();
            let sub_width = text_width.saturating_sub(prefix_len);
            let wrapped_lines = wrap_text(list_content, sub_width);

            for (idx, w_line) in wrapped_lines.into_iter().enumerate() {
                let mut spans = Vec::new();
                if idx == 0 {
                    let p_style = apply_diff_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                        diff_kind,
                    );
                    spans.push(Span::raw(indent_spaces.clone()));
                    spans.push(Span::styled(prefix.clone(), p_style));
                } else {
                    let pad = " ".repeat(prefix_len);
                    spans.push(Span::raw(pad));
                }
                spans.extend(parse_inline_spans(&w_line, base_style));
                rows.push(RenderedMarkdownRow {
                    line_number: if idx == 0 { line_num } else { None },
                    diff_kind,
                    spans,
                });
            }
            continue;
        }

        // Standard Paragraph or empty line
        if trimmed.trim().is_empty() {
            rows.push(RenderedMarkdownRow {
                line_number: line_num,
                diff_kind,
                spans: vec![Span::raw("")],
            });
            continue;
        }

        let base_style = apply_diff_style(Style::default().fg(Color::White), diff_kind);
        let wrapped_lines = wrap_text(trimmed, text_width);
        for (idx, w_line) in wrapped_lines.into_iter().enumerate() {
            let spans = parse_inline_spans(&w_line, base_style);
            rows.push(RenderedMarkdownRow {
                line_number: if idx == 0 { line_num } else { None },
                diff_kind,
                spans,
            });
        }
    }

    rows
}

/// Applies green/red styling tint for addition/deletion.
pub fn apply_diff_style(base: Style, diff_kind: MarkdownDiffKind) -> Style {
    match diff_kind {
        MarkdownDiffKind::Addition => base.fg(Color::Green),
        MarkdownDiffKind::Deletion => base.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT),
        MarkdownDiffKind::Unchanged => base,
    }
}

/// Detects heading level (1 to 6) and returns the clean heading string.
fn parse_heading(text: &str) -> Option<(usize, &str)> {
    let trimmed = text.trim_start();
    let mut level = 0;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }
    if (1..=6).contains(&level) {
        let rest = &trimmed[level..];
        if rest.starts_with(' ') {
            return Some((level, rest.trim()));
        }
    }
    None
}

/// Determines Ratatui style for a given heading level and diff status.
fn get_heading_style(level: usize, diff_kind: MarkdownDiffKind) -> Style {
    let mut style = match level {
        1 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
        2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        3 => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        4 => Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
        5 => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    };

    style = apply_diff_style(style, diff_kind);
    style
}

/// Parses a list item prefix (bullet or number) and returns `(prefix, content, indent_depth)`.
fn parse_list_item(text: &str) -> Option<(String, &str, usize)> {
    let indent_chars = text.chars().take_while(|c| c.is_whitespace()).count();
    let depth = indent_chars / 2;
    let trimmed = text.trim_start();

    // Bullet items: - , * , +
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let content = &trimmed[2..];
        return Some(("• ".to_string(), content.trim(), depth));
    }

    // Numbered items: 1. , 12.
    if let Some(dot_idx) = trimmed.find(". ") {
        let num_slice = &trimmed[..dot_idx];
        if !num_slice.is_empty() && num_slice.chars().all(|c| c.is_ascii_digit()) {
            let prefix = format!("{}. ", num_slice);
            let content = &trimmed[dot_idx + 2..];
            return Some((prefix, content.trim(), depth));
        }
    }

    None
}

/// Tokenizes inline markdown: `**bold**`, `*italic*`, `` `code` ``, `[label](url)`.
pub fn parse_inline_spans(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut current_text = String::new();

    let flush_current = |current: &mut String, spans: &mut Vec<Span<'static>>| {
        if !current.is_empty() {
            spans.push(Span::styled(std::mem::take(current), base_style));
        }
    };

    while let Some(&(i, ch)) = chars.peek() {
        // Bold: **
        if ch == '*' && text[i..].starts_with("**") {
            if let Some(end_rel) = text[i + 2..].find("**") {
                flush_current(&mut current_text, &mut spans);
                let bold_content = &text[i + 2..i + 2 + end_rel];
                let bold_style = base_style.add_modifier(Modifier::BOLD);
                spans.push(Span::styled(bold_content.to_string(), bold_style));

                // Advance chars past end marker
                let advance_to = i + 2 + end_rel + 2;
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i < advance_to {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }

        // Italic: *
        if ch == '*' && !text[i..].starts_with("**") {
            if let Some(end_rel) = text[i + 1..].find('*') {
                if !text[i + 1..i + 1 + end_rel].contains('*') {
                    flush_current(&mut current_text, &mut spans);
                    let italic_content = &text[i + 1..i + 1 + end_rel];
                    let italic_style = base_style.add_modifier(Modifier::ITALIC);
                    spans.push(Span::styled(italic_content.to_string(), italic_style));

                    let advance_to = i + 1 + end_rel + 1;
                    while let Some(&(next_i, _)) = chars.peek() {
                        if next_i < advance_to {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
        }

        // Inline code: `
        if ch == '`' {
            if let Some(end_rel) = text[i + 1..].find('`') {
                flush_current(&mut current_text, &mut spans);
                let code_content = &text[i + 1..i + 1 + end_rel];
                let code_style = Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Rgb(40, 40, 45));
                spans.push(Span::styled(format!(" {} ", code_content), code_style));

                let advance_to = i + 1 + end_rel + 1;
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i < advance_to {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }

        // Link: [label](url)
        if ch == '[' {
            if let Some(label_end) = text[i + 1..].find(']') {
                let after_label = i + 1 + label_end + 1;
                if text[after_label..].starts_with('(') {
                    if let Some(url_end) = text[after_label + 1..].find(')') {
                        flush_current(&mut current_text, &mut spans);
                        let label = &text[i + 1..after_label - 1];
                        let url = &text[after_label + 1..after_label + 1 + url_end];
                        let link_style = base_style
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED);
                        spans.push(Span::styled(label.to_string(), link_style));
                        let url_style = Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM);
                        spans.push(Span::styled(format!(" ({})", url), url_style));

                        let advance_to = after_label + 1 + url_end + 1;
                        while let Some(&(next_i, _)) = chars.peek() {
                            if next_i < advance_to {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                }
            }
        }

        current_text.push(ch);
        chars.next();
    }

    flush_current(&mut current_text, &mut spans);
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }
    spans
}

/// Wraps text cleanly at word boundaries respecting maximum character width.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut current_line = String::new();
    for word in words {
        // If single word exceeds max_width, split word directly
        if word.chars().count() > max_width {
            if !current_line.is_empty() {
                lines.push(std::mem::take(&mut current_line));
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.chars().count() == max_width {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(c);
            }
            if !chunk.is_empty() {
                current_line = chunk;
            }
            continue;
        }

        let needed_len = if current_line.is_empty() {
            word.chars().count()
        } else {
            current_line.chars().count() + 1 + word.chars().count()
        };

        if needed_len <= max_width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current_line));
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk};
    use std::path::PathBuf;

    fn make_test_file_diff(lines: Vec<DiffLine>) -> FileDiff {
        FileDiff {
            old_path: Some(PathBuf::from("README.md")),
            new_path: PathBuf::from("README.md"),
            change_type: ChangeType::Modified,
            additions: lines
                .iter()
                .filter(|l| l.line_type == LineType::Addition)
                .count(),
            deletions: lines
                .iter()
                .filter(|l| l.line_type == LineType::Deletion)
                .count(),
            hunks: vec![DiffHunk {
                header: "@@ -1,5 +1,5 @@".into(),
                old_start: 1,
                old_lines: 5,
                new_start: 1,
                new_lines: 5,
                lines,
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn test_is_markdown_file_recognizes_extensions() {
        assert!(is_markdown_file(Path::new("README.md")));
        assert!(is_markdown_file(Path::new("docs/spec.markdown")));
        assert!(is_markdown_file(Path::new("NOTES.MKD")));
        assert!(is_markdown_file(Path::new("proposal.mdown")));
        assert!(!is_markdown_file(Path::new("src/main.rs")));
        assert!(!is_markdown_file(Path::new("Cargo.toml")));
        assert!(!is_markdown_file(Path::new("Makefile")));
    }

    #[test]
    fn test_empty_document_renders_placeholder() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("empty.md"),
            change_type: ChangeType::Added,
            additions: 0,
            deletions: 0,
            hunks: Vec::new(),
            is_folded: false,
            is_reviewed: false,
        };

        let rows = build_markdown_preview(&file, 80);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spans[0].content, "(Empty markdown document)");
        assert_eq!(rows[0].diff_kind, MarkdownDiffKind::Unchanged);
    }

    #[test]
    fn test_parse_headings_with_diff_styles() {
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "# Main Title".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "## Added Section".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Deletion,
                content: "### Removed Subtitle".into(),
                old_line_num: Some(2),
                new_line_num: None,
            },
        ];

        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].diff_kind, MarkdownDiffKind::Unchanged);
        assert_eq!(rows[0].spans[0].content, "# ");
        assert_eq!(rows[0].spans[1].content, "Main Title");

        assert_eq!(rows[1].diff_kind, MarkdownDiffKind::Addition);
        assert_eq!(rows[1].spans[0].content, "## ");
        assert_eq!(rows[1].spans[1].content, "Added Section");
        assert_eq!(rows[1].spans[1].style.fg, Some(Color::Green));

        assert_eq!(rows[2].diff_kind, MarkdownDiffKind::Deletion);
        assert_eq!(rows[2].spans[0].content, "### ");
        assert_eq!(rows[2].spans[1].content, "Removed Subtitle");
        assert_eq!(rows[2].spans[1].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_parse_lists_and_indentation() {
        let lines = vec![
            DiffLine {
                line_type: LineType::Addition,
                content: "* First bullet".into(),
                old_line_num: None,
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "  - Nested bullet".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "1. Numbered item".into(),
                old_line_num: Some(2),
                new_line_num: Some(3),
            },
        ];

        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].spans[1].content, "• ");
        assert_eq!(rows[0].spans[2].content, "First bullet");

        assert_eq!(rows[1].spans[0].content, "  ");
        assert_eq!(rows[1].spans[1].content, "• ");
        assert_eq!(rows[1].spans[2].content, "Nested bullet");

        assert_eq!(rows[2].spans[1].content, "1. ");
        assert_eq!(rows[2].spans[2].content, "Numbered item");
    }

    #[test]
    fn test_parse_code_blocks() {
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "```rust".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "fn hello() -> &'static str { \"world\" }".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "```".into(),
                old_line_num: Some(2),
                new_line_num: Some(3),
            },
        ];

        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].spans[0].content, "─── [rust] ───");
        assert_eq!(rows[1].diff_kind, MarkdownDiffKind::Addition);
        assert!(rows[1].spans[0].content.contains("fn hello"));
        assert_eq!(rows[2].spans[0].content, "───────────────");
    }

    #[test]
    fn test_parse_blockquotes() {
        let lines = vec![DiffLine {
            line_type: LineType::Addition,
            content: "> Important architectural notice".into(),
            old_line_num: None,
            new_line_num: Some(1),
        }];

        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spans[0].content, "│ ");
        assert_eq!(rows[0].spans[1].content, "Important architectural notice");
        assert_eq!(rows[0].diff_kind, MarkdownDiffKind::Addition);
    }

    #[test]
    fn test_parse_inline_spans_tokens() {
        let spans = parse_inline_spans(
            "This has **bold**, *italic*, `code`, and [link](https://lgtm.rs) in it.",
            Style::default(),
        );

        let contents: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(contents.contains(&"bold"));
        assert!(contents.contains(&"italic"));
        assert!(contents.contains(&" code "));
        assert!(contents.contains(&"link"));
        assert!(contents.contains(&" (https://lgtm.rs)"));
    }

    #[test]
    fn test_word_wrapping_long_words() {
        let text = "Short words and an_unbroken_super_long_word_that_exceeds_viewport_limit end";
        let wrapped = wrap_text(text, 15);
        for line in &wrapped {
            assert!(line.chars().count() <= 15);
        }
        assert!(wrapped.len() > 3);
    }

    #[test]
    fn test_scroll_navigation_and_clamping() {
        let mut state = MarkdownPreviewState {
            rendered_rows: (0..50)
                .map(|i| RenderedMarkdownRow {
                    line_number: Some(i as u32),
                    diff_kind: MarkdownDiffKind::Unchanged,
                    spans: vec![Span::raw("line")],
                })
                .collect(),
            ..Default::default()
        };

        // Viewport height 20 -> max_scroll = 30
        state.scroll_down(10, 20);
        assert_eq!(state.scroll_offset, 10);

        state.scroll_down(30, 20);
        assert_eq!(state.scroll_offset, 30); // clamped at 30

        state.scroll_down(5, 20);
        assert_eq!(state.scroll_offset, 30); // stays clamped

        state.scroll_up(15);
        assert_eq!(state.scroll_offset, 15);

        state.scroll_to_bottom(20);
        assert_eq!(state.scroll_offset, 30);

        state.scroll_to_top();
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_performance_budgets_nfr() {
        use std::time::Instant;

        // Generate 5,000 lines of markdown
        let mut lines = Vec::with_capacity(5000);
        for i in 0..5000 {
            let line_type = if i % 10 == 0 {
                LineType::Addition
            } else if i % 10 == 1 {
                LineType::Deletion
            } else {
                LineType::Context
            };
            let content = match i % 5 {
                0 => format!("# Heading level 1 for section {i}"),
                1 => format!("* Bullet list item with some description text for index {i}"),
                2 => format!("```rust\nlet x_{i} = {i};\n```"),
                3 => format!("> Blockquote notice containing important RFC detail {i}"),
                _ => format!(
                    "A standard paragraph with **bold** and *italic* formatting for row {i}."
                ),
            };
            lines.push(DiffLine {
                line_type,
                content,
                old_line_num: Some(i as u32),
                new_line_num: Some(i as u32),
            });
        }

        let file = make_test_file_diff(lines);

        // Budget NFR-MDP-02: Parse and layout for 5,000 lines must complete in < 50ms
        let start_parse = Instant::now();
        let rows = build_markdown_preview(&file, 100);
        let parse_duration = start_parse.elapsed();
        let threshold_ms = if cfg!(debug_assertions) { 150 } else { 50 };
        assert!(
            parse_duration.as_millis() < threshold_ms,
            "Parsing 5,000 lines took {:?}, expected < {}ms",
            parse_duration,
            threshold_ms
        );
        assert!(!rows.is_empty());

        // Budget NFR-MDP-03: Zero-allocation scrolling < 1µs per keystroke
        let mut state = MarkdownPreviewState {
            rendered_rows: rows,
            ..Default::default()
        };
        let start_scroll = Instant::now();
        for _ in 0..1000 {
            state.scroll_down(1, 40);
        }
        let scroll_duration = start_scroll.elapsed();
        let per_keystroke_nanos = scroll_duration.as_nanos() / 1000;
        assert!(
            per_keystroke_nanos < 1000,
            "Per-keystroke navigation took {}ns, expected < 1µs (1000ns)",
            per_keystroke_nanos
        );
    }

    #[test]
    fn test_all_heading_levels_and_diff_styles() {
        for level in 1..=6 {
            let hashes = "#".repeat(level);
            let heading_str = format!("{} Heading {}", hashes, level);
            let lines = vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: heading_str.clone(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: heading_str.clone(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
                DiffLine {
                    line_type: LineType::Deletion,
                    content: heading_str,
                    old_line_num: Some(2),
                    new_line_num: None,
                },
            ];
            let file = make_test_file_diff(lines);
            let rows = build_markdown_preview(&file, 80);
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0].diff_kind, MarkdownDiffKind::Unchanged);
            assert_eq!(rows[1].diff_kind, MarkdownDiffKind::Addition);
            assert_eq!(rows[2].diff_kind, MarkdownDiffKind::Deletion);
        }
    }

    #[test]
    fn test_horizontal_rules() {
        for rule in ["---", "***", "___"] {
            let lines = vec![DiffLine {
                line_type: LineType::Context,
                content: rule.to_string(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            }];
            let file = make_test_file_diff(lines);
            let rows = build_markdown_preview(&file, 80);
            assert_eq!(rows.len(), 1);
            assert!(rows[0].spans[0].content.contains('─'));
        }
    }

    #[test]
    fn test_tilde_code_fence_and_unspecified_lang() {
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "~~~".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "verbatim content".into(),
                old_line_num: Some(2),
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "~~~".into(),
                old_line_num: Some(3),
                new_line_num: Some(3),
            },
        ];
        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].spans[0].content, "─── [code] ───");
        assert!(rows[1].spans[0].content.contains("verbatim content"));
        assert_eq!(rows[2].spans[0].content, "───────────────");
    }

    #[test]
    fn test_wrap_text_zero_width_and_whitespace_only() {
        let res = wrap_text("hello world", 0);
        assert_eq!(res, vec!["hello world"]);

        let res2 = wrap_text("   \n\t  ", 10);
        assert_eq!(res2, vec![""]);
    }

    #[test]
    fn test_pure_deletion_document() {
        let lines: Vec<DiffLine> = (1..=20)
            .map(|i| DiffLine {
                line_type: LineType::Deletion,
                content: format!("deleted line {i}"),
                old_line_num: Some(i),
                new_line_num: None,
            })
            .collect();
        let file = make_test_file_diff(lines);
        let rows = build_markdown_preview(&file, 80);
        assert_eq!(rows.len(), 20);
        for row in rows {
            assert_eq!(row.diff_kind, MarkdownDiffKind::Deletion);
        }
    }
}
