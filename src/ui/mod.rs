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
mod header;
mod help;
mod modals;
mod status;
use self::audit::render_audit_report_view;
use self::command::render_command_completion_column;
use self::diff::render_body;
use self::header::render_header;
use self::help::render_help_modal;
use self::modals::comments::{render_comment_input, render_comment_list, render_comment_reply};
use self::modals::commit::render_commit_modal;
use self::modals::pickers::{render_file_picker, render_pane_picker, render_pr_picker};
use self::status::{render_search_input, render_status_bar};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

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
        ActiveView::PrPicker => render_pr_picker(frame, size, app),
        ActiveView::Help => render_help_modal(frame, size, app),
        ActiveView::Search => render_search_input(frame, size, app),
        ActiveView::CommitModal => render_commit_modal(frame, size, app),
        ActiveView::CommandLine => {
            if !app.command_completions.is_empty() {
                render_command_completion_column(frame, size, app);
            }
        }
        ActiveView::Diff | ActiveView::AuditReport => {}
    }
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
        app.command_completions = vec!["w".into(), "write".into(), "wq".into()];
        app.command_completion_idx = Some(1);
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
