use crate::app::{ActiveView, App};
use crate::model::{ChangeType, DiffLine, LineType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};
use similar::{ChangeTag, TextDiff};

mod audit;
mod command;
mod diff;
pub(crate) use diff::cache::RenderCache;
mod header;
mod help;
pub(crate) mod markdown;
mod modals;
mod status;
use self::audit::render_audit_report_view;
use self::command::render_command_completion_column;
use self::diff::render_body;
use self::header::render_header;
use self::help::render_help_modal;
use self::modals::comments::{render_comment_input, render_comment_list, render_comment_reply};
use self::modals::commit::render_commit_modal;
use self::modals::pickers::{
    render_file_picker, render_pane_picker, render_pr_picker, render_worktree_confirm_guard,
    render_worktree_picker,
};
use self::modals::threads::render_threads_modal;
use self::status::{render_search_input, render_status_bar};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let mut timing = lgtm_core::debug::Span::new("ui.render");
    timing.count("files", app.snapshot.files.len() as u64);
    timing.count("comments", app.comments.len() as u64);
    timing.count("threads", app.threads.len() as u64);
    timing.count("alerts", app.alerts.len() as u64);
    timing.count("width", size.width.into());
    timing.count("height", size.height.into());
    timing.count(
        "changed_lines",
        (app.snapshot.total_additions + app.snapshot.total_deletions) as u64,
    );
    if lgtm_core::debug::enabled() {
        let selected_lines = app
            .snapshot
            .files
            .get(app.selected_file_idx)
            .map_or(0, |file| {
                file.hunks
                    .iter()
                    .map(|hunk| hunk.lines.len())
                    .sum::<usize>()
            });
        timing.count("selected_lines", selected_lines as u64);
    }

    // Main layout: Header (1 line), Body (rest), Status bar (1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top bar
            Constraint::Min(5),    // Main workspace
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);

    // Render active overlay modal if needed
    match app.active_view {
        ActiveView::FilePicker => render_file_picker(frame, size, app),
        ActiveView::PanePicker => render_pane_picker(frame, size, app),
        ActiveView::CommentInput => render_comment_input(frame, size, app),
        ActiveView::CommentReply => render_comment_reply(frame, size, app),
        ActiveView::CommentList => render_comment_list(frame, size, app),
        ActiveView::ThreadView => render_threads_modal(frame, size, app),
        ActiveView::PrPicker => render_pr_picker(frame, size, app),
        ActiveView::WorktreePicker => render_worktree_picker(frame, size, app),
        ActiveView::WorktreeConfirmGuard => render_worktree_confirm_guard(frame, size, app),
        ActiveView::Help => render_help_modal(frame, size, app),
        ActiveView::Search => render_search_input(frame, size, app),
        ActiveView::CommitModal => render_commit_modal(frame, size, app),
        ActiveView::CommandLine => {
            if !app.command.completions.is_empty() {
                render_command_completion_column(frame, size, app);
            }
        }
        ActiveView::AgentPrompt => render_agent_prompt_modal(frame, size, app),
        ActiveView::Diff | ActiveView::AuditReport | ActiveView::MarkdownPreview => {}
    }
}

fn render_agent_prompt_modal(frame: &mut Frame, area: Rect, app: &mut App) {
    let modal_area = centered_rect(75, 60, area);
    frame.render_widget(Clear, modal_area);

    let (source, content) = app.view_data.prompt_for(&app.git_engine.repo_path);

    let block = Block::default()
        .title(format!(
            " 🤖 Agent Prompt Context ({}) [Esc/q/P to close] ",
            source
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, modal_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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
    fn test_centered_rect_math() {
        let parent = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 40, parent);
        assert_eq!(centered.width, 60);
        assert_eq!(centered.height, 20);
        assert!(centered.x > 0);
        assert!(centered.y > 0);
    }

    #[test]
    fn test_full_ui_rendering_on_test_backend() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let (_tx, rx) = unbounded();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("lgtm-test-ui-{}", unique));
        let _ = std::fs::create_dir_all(&tmp);

        let mut app = App::new(tmp, rx, None, None, None);

        // Test rendering Diff View
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Help View
        app.active_view = ActiveView::Help;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommandLine with completions
        app.active_view = ActiveView::CommandLine;
        app.command.completions = vec!["w".into(), "write".into(), "wq".into()];
        app.command.completion_idx = Some(1);
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering FilePicker
        app.active_view = ActiveView::FilePicker;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering PanePicker
        app.active_view = ActiveView::PanePicker;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommentInput
        app.active_view = ActiveView::CommentInput;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommentList
        app.active_view = ActiveView::CommentList;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering ThreadView (empty and populated)
        app.active_view = ActiveView::ThreadView;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        app.threads = vec![lgtm_core::github::Thread {
            id: "1".into(),
            path: "test.rs".into(),
            line: Some(5),
            resolved: false,
            comments: vec![
                lgtm_core::github::ThreadComment {
                    id: 1,
                    author: "alice".into(),
                    body: "Comment 1".into(),
                },
                lgtm_core::github::ThreadComment {
                    id: 2,
                    author: "bob".into(),
                    body: "Reply 1".into(),
                },
            ],
        }];
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Search
        app.active_view = ActiveView::Search;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering CommitModal
        app.active_view = ActiveView::CommitModal;
        app.commit_message_input = "feat: add cognitive complexity guard".to_string();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Visual Mode
        app.active_view = ActiveView::Diff;
        app.visual_mode = true;
        app.visual_anchor_idx = Some(0);
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Test rendering Split View
        app.is_split_view = true;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
}
