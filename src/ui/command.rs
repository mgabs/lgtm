use super::*;

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

pub(crate) fn render_command_completion_column(frame: &mut Frame, area: Rect, app: &App) {
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
    #![allow(unused_imports)]
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
}
