use super::*;

pub fn command_description(cmd: &str) -> &'static str {
    match cmd {
        "w" | "write" => "Save directives to .lgtm/directives.jsonl",
        "q" | "quit" => "Quit lgtm",
        "wq" | "x" | "exit" => "Save directives and quit",
        "h" | "help" | "?" => "Open help modal",
        "pr" => "Review GitHub PR diff (:pr <num>)",
        "prs" => "Open interactive GitHub PR picker",
        "pane" | "panes" => "Select target agent pane (tmux, zellij, wezterm, kitty)",
        "sink" => "Set or view agent steering sink (:sink [tmux|zellij|wezterm|kitty|clipboard])",
        "f" | "files" | "find" => "Open fuzzy file picker",
        "tree" | "drawer" | "sidebar" => "Toggle file tree sidebar",
        "sp" | "split" | "vsp" | "vsplit" => "Toggle split / unified view",
        "diff" => "Return to diff view, or `:diff <branch>` to review and audit this branch against <branch>",
        "commit" | "ci" => "Open Commit Forge to commit staged changes",
        "c" | "comments" | "directives" | "todo" => {
            "Open review directives list (`:directives recheck` reclassifies outcomes; `:directives orphans` lists, re-pins or drops orphaned ones)"
        }
        "r" | "review" | "send" | "push" | "submit" => "Export & blast batch review to agent",
        "thread" => "Draft comments, replies and resolves, then send them (:thread comment|reply|resolve|unresolve|edit|discard|drafts|submit)",
        "checks" => "Refresh CI check status for the open PR",
        "threads" => "List remote review threads for the open PR",
        "hidden" => "List suppressed and baselined findings",
        "hook" => "Manage Git pre-commit hook (:hook [install|uninstall|status])",
        "wt" | "worktrees" => "Open interactive Git worktree picker modal (:wt [name|path])",
        "md" | "preview" => "Toggle rendered Markdown preview for .md files",
        "audit" | "issues" => {
            "Audit the whole codebase (not just the diff) for security & architecture issues"
        }
        "scan" => "Scan changed files, or the whole codebase with `:scan repo`",
        "risk" => "Re-scan diff for security & test risks",
        "editor" => "Set or view external editor (:editor [nvim|vim|vi|vscode|code|cursor])",
        "e" | "edit" | "reload" => "Reload diff from disk / PR",
        "clear" => "Clear all review directives",
        "clear-resolved" => "Remove all resolved review directives",
        "resolve" | "res" => "Resolve directive on current line (or `:resolve all`)",
        "unresolve" => "Mark directive on current line active (or `:unresolve all`)",
        "next" | "n" => "Jump to next file (or next issue in Audit mode)",
        "prev" | "p" => "Jump to previous file (or previous issue in Audit mode)",
        "prompt" | "context" => "Open agent prompt / gate context modal",
        "approve" | "lgtm" => "Approve agent breakpoint gate",
        "reject" => "Reject hunk under cursor and re-prompt agent with reason",
        "reject-all" | "reject-gate" => "Reject waiting agent breakpoint gate",
        "filter" | "sev" => "Filter audit issues by severity (:filter 1|2|3|crit|high|all)",
        "inject" => "Inject violation remediation directive to agent (:inject all)",
        "export" => "Export audit report to Markdown (:export [path])",
        "y" | "yank" => "Yank current code line",
        "yr" | "ref" => "Yank #ref file:line to clipboard",
        "d" | "del" | "delete" => "Delete directive on current line",
        _ => "",
    }
}

pub(crate) fn render_command_completion_column(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.command.completions.len();
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
    let active_idx = app.command.completion_idx.unwrap_or(0);
    let start_idx = if active_idx >= visible_rows {
        active_idx - visible_rows + 1
    } else {
        0
    };
    let end_idx = (start_idx + visible_rows).min(total);

    let items: Vec<ListItem> = app.command.completions[start_idx..end_idx]
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
    use super::*;

    #[test]
    fn test_command_descriptions() {
        assert_eq!(
            command_description("w"),
            "Save directives to .lgtm/directives.jsonl"
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
        assert!(command_description("audit").contains("whole codebase"));
        assert!(command_description("wt").contains("worktree picker"));
        assert!(command_description("worktrees").contains("worktree picker"));
        assert!(command_description("md").contains("Markdown"));
        assert_eq!(command_description("unknown_command"), "");
    }
}
