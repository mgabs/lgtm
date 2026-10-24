use super::*;

pub(crate) fn render_audit_file_drawer(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.audit.focus == crate::app::AuditPaneFocus::Files;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    if app.audit.files.is_empty() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            " (No files audited) ",
            Style::default().fg(Color::DarkGray),
        )))];
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Files (Tab) ")
                .border_style(Style::default().fg(border_color)),
        );
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .audit
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = idx == app.selected_file_idx;
            let display_name = file.display_name();
            let is_filtered = app.audit.file_filter.as_deref() == Some(&display_name);

            let file_alerts_count = app
                .audit
                .alerts
                .iter()
                .filter(|a| a.file_path == display_name)
                .count();

            let (badge, badge_color) = if file_alerts_count > 0 {
                (format!("⚠️{:2} ", file_alerts_count), Color::Yellow)
            } else {
                (" ✓   ".to_string(), Color::Green)
            };

            let prefix = if is_filtered { "● " } else { "  " };

            let spans = vec![
                Span::styled(
                    prefix,
                    if is_filtered {
                        Style::default()
                            .fg(Color::LightMagenta)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    badge,
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(display_name),
            ];

            let style = if is_selected {
                if is_focused {
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                }
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let title = if let Some(ref filt) = app.audit.file_filter {
        format!(" Files [{}] (Filtered: {}) ", app.audit.files.len(), filt)
    } else {
        format!(" Files [{}] (Tab/Space) ", app.audit.files.len())
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );

    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.selected_file_idx));
    frame.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn render_audit_report_view(frame: &mut Frame, area: Rect, app: &App) {
    let filtered_alerts = app.filtered_audit_alerts();
    let total_alerts = app.audit.alerts.len();
    let filtered_count = filtered_alerts.len();

    // If inspector is maximized, allocate full screen to inspector
    if app.audit.maximized_inspector {
        let selected_alert = filtered_alerts.get(app.audit.selected_idx).copied();
        render_audit_inspector(frame, area, app, selected_alert);
        return;
    }

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
        if app.audit.selected_idx > half {
            (app.audit.selected_idx - half).min(total_filtered - visible_height)
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
                let is_selected = idx == app.audit.selected_idx;
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
                    Some(l) => {
                        if app.audit.file_filter.is_some() {
                            format!("L{:4} ", l)
                        } else {
                            format!("{}:{} ", alert.file_path, l)
                        }
                    }
                    None => {
                        if app.audit.file_filter.is_some() {
                            "      ".to_string()
                        } else {
                            format!("{} ", alert.file_path)
                        }
                    }
                };

                let spans = vec![
                    Span::styled(format!("[{}] ", alert.severity.badge()), badge_style),
                    Span::styled(loc, Style::default().fg(Color::Cyan)),
                    Span::raw(&alert.title),
                ];

                let line_style = if is_selected {
                    if app.audit.focus == crate::app::AuditPaneFocus::Issues {
                        Style::default()
                            .bg(Color::Cyan)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(line_style)
            })
            .collect()
    };

    let filter_text = match app.audit.filter_severity {
        Some(crate::guard::Severity::Critical) => "🔴 Crit [1]",
        Some(crate::guard::Severity::High) => "🟠 High+ [2]",
        _ => "All [3]",
    };

    let issues_title = if let Some(ref file) = app.audit.file_filter {
        format!(
            " Issues for {} ({}/{}) Filter: {} [a: all] ",
            file, filtered_count, total_alerts, filter_text
        )
    } else {
        format!(
            " Repository Issues ({}/{}) Filter: {} ",
            filtered_count, total_alerts, filter_text
        )
    };

    let issues_border_color = if app.audit.focus == crate::app::AuditPaneFocus::Issues {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let issues_block = Block::default()
        .borders(Borders::ALL)
        .title(issues_title)
        .border_style(Style::default().fg(issues_border_color));

    frame.render_widget(List::new(items).block(issues_block), left_area);

    let selected_alert = filtered_alerts.get(app.audit.selected_idx).copied();
    render_audit_inspector(frame, right_area, app, selected_alert);
}

pub(crate) fn render_audit_inspector(
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

    let is_focused = app.audit.focus == crate::app::AuditPaneFocus::Inspector;
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
        if let Some(snippet) = app.view_data.snippet_for(&app.audit.root_dir, alert) {
            lines.push(Line::from(Span::styled(
                "Code Snippet Context:",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )));
            for (current_no, line_item) in snippet {
                let current_no = *current_no;
                let is_hit = current_no == target_line as usize;
                let line_text = line_item.as_str();

                if is_hit {
                    let ptr_color = if is_focused { Color::Cyan } else { Color::Red };
                    let bg_color = if is_focused {
                        Color::Indexed(237)
                    } else {
                        Color::DarkGray
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  ▶ {:4} │ ", current_no),
                            Style::default().fg(ptr_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            line_text,
                            Style::default()
                                .bg(bg_color)
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
        Span::raw(" Inject  "),
        Span::styled(
            "[c]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Comment  "),
        Span::styled(
            "[y]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Yank  "),
        Span::styled(
            "[e/o]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Editor  "),
        Span::styled(
            "[E]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Export  "),
        Span::styled(
            "[Esc]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Diff View"),
    ]));

    let is_focused = app.audit.focus == crate::app::AuditPaneFocus::Inspector;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = if app.audit.maximized_inspector {
        " Issue Inspector [MAXIMIZED - Z/m to restore] "
    } else if is_focused {
        " Issue Inspector & Remediation (Scroll: j/k, Ctrl-d/u) "
    } else {
        " Issue Inspector & Remediation "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.audit.inspector_scroll as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
