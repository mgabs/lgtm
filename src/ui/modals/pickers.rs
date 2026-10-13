use super::super::*;

pub(crate) fn render_file_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(popup_area);

    let input = Paragraph::new(app.file_picker.query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Jump to File (Type to filter, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .file_picker
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(list_idx, &file_idx)| {
            let file = &app.snapshot.files[file_idx];
            let is_selected = list_idx == app.file_picker.selected_idx;

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

pub(crate) fn render_pane_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .pane_picker
        .list
        .iter()
        .enumerate()
        .map(|(idx, pane)| {
            let is_selected = idx == app.pane_picker.selected_idx;
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

pub(crate) fn render_pr_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(75, 65, area);
    frame.render_widget(Clear, popup_area);

    if app.pr.list.is_empty() {
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
        .pr
        .list
        .iter()
        .enumerate()
        .map(|(idx, pr)| {
            let is_selected = idx == app.pr.selected_idx;
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
                app.pr.list.len()
            ))
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(list, popup_area);
}
