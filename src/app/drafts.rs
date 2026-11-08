//! Live PR thread actions and in-memory retry data.
use super::*;
use lgtm_core::comment::CommentSide;
use lgtm_core::forge::{Resolution, ThreadState};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentRetryBuffer {
    pub path: String,
    pub line: u32,
    pub side: CommentSide,
    pub body: String,
    pub target_thread_id: Option<String>,
}

const USAGE: &str = "Usage: :thread comment <text> | reply <n> <text> | resolve <n> | unresolve <n> | retry | refresh | outdated";

impl App {
    pub fn toggle_resolve_thread_at_cursor(&mut self) -> bool {
        let Some(idx) = self.thread_at_cursor() else {
            return false;
        };
        self.set_thread_resolved(idx, None);
        true
    }

    pub(crate) fn start_thread_reply(&mut self, index: usize) {
        if let Some(position) = self
            .visible_thread_indices()
            .iter()
            .position(|&visible| visible == index)
        {
            self.thread_selected_idx = position;
        }
        self.command
            .set_text(format!("thread reply {} ", index + 1));
        self.command.history_idx = None;
        self.active_view = ActiveView::CommandLine;
    }

    /// File, line and side of the diff line under the cursor; deletions are on the LEFT.
    fn cursor_anchor(&self) -> Option<(String, u32, CommentSide)> {
        let file = self.snapshot.files.get(self.selected_file_idx)?;
        let line = file
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .nth(self.cursor_line_idx)?;
        let (n, side) = match (line.new_line_num, line.old_line_num) {
            (Some(n), _) => (n, CommentSide::Right),
            (None, Some(o)) => (o, CommentSide::Left),
            _ => return None,
        };
        Some((file.display_name().to_string(), n, side))
    }

    pub(crate) fn post_inline_comment(&mut self, body: &str) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        };
        let Some((path, line, side)) = self.cursor_anchor() else {
            self.status_message = Some("Cursor is not on a diff line of a hunk".into());
            return;
        };
        self.post_inline_to_target(&path, line, side, body, pr);
    }

    pub(crate) fn post_inline_to_target(
        &mut self,
        path: &str,
        line: u32,
        side: CommentSide,
        body: &str,
        pr: u64,
    ) {
        let body_str = body.trim().to_string();
        if body_str.is_empty() {
            self.status_message = Some("Comment cannot be empty".into());
            return;
        }

        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let path_str = path.to_string();
        let target_path = path_str.clone();
        let retry_body = body_str.clone();

        self.spawn_job(
            "Posting comment",
            move || {
                let forge = factory(&repo);
                let sha = forge.head_sha(pr)?;
                forge
                    .post_inline(pr, &sha, &path_str, line, side, &body_str)
                    .map(|id| (id, body_str))
            },
            move |app, result| match result {
                Ok((comment_id, _)) => {
                    app.status_message = Some(format!("Comment #{comment_id} posted"));
                    app.comment_retry = None;
                    app.refresh_threads_quietly(pr);
                }
                Err(e) => {
                    app.status_message = Some(format!("Failed to post comment: {e:#}"));
                    app.comment_retry = Some(CommentRetryBuffer {
                        path: target_path,
                        line,
                        side,
                        body: retry_body,
                        target_thread_id: None,
                    });
                }
            },
        );
    }

    pub(crate) fn post_reply(&mut self, thread_idx: usize, body: &str) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        };
        let Some(t) = self.threads.get(thread_idx) else {
            self.status_message = Some("No such thread; run :threads first".into());
            return;
        };
        let comment_id = t.comments.first().map_or(0, |c| c.id);
        if comment_id == 0 {
            self.status_message = Some("Thread has no reply target; run :thread refresh".into());
            return;
        }
        let body_str = body.trim().to_string();
        if body_str.is_empty() {
            self.status_message = Some("Reply cannot be empty".into());
            return;
        }

        let thread_id = t.id.clone();
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let thread_id_for_retry = thread_id.clone();
        let retry_body = body_str.clone();

        self.spawn_job(
            "Posting reply",
            move || {
                let forge = factory(&repo);
                forge
                    .reply(pr, &thread_id, comment_id, &body_str)
                    .map(|id| (id, body_str))
            },
            move |app, result| match result {
                Ok((new_id, _)) => {
                    app.status_message = Some(format!("Reply #{new_id} posted"));
                    app.comment_retry = None;
                    app.refresh_threads_quietly(pr);
                }
                Err(e) => {
                    app.status_message = Some(format!("Failed to post reply: {e:#}"));
                    app.comment_retry = Some(CommentRetryBuffer {
                        path: String::new(),
                        line: 0,
                        side: CommentSide::Right,
                        body: retry_body,
                        target_thread_id: Some(thread_id_for_retry),
                    });
                }
            },
        );
    }

    pub(crate) fn set_thread_resolved(&mut self, thread_idx: usize, resolved: Option<bool>) {
        let Some(t) = self.threads.get(thread_idx) else {
            self.status_message = Some("No such thread; run :threads first".into());
            return;
        };
        let target = if let Some(r) = resolved {
            if r {
                ThreadState::Resolved(Resolution::Done)
            } else {
                ThreadState::Open
            }
        } else if t.resolved() {
            ThreadState::Open
        } else {
            ThreadState::Resolved(Resolution::Done)
        };
        self.set_thread_state(thread_idx, target);
    }

    pub(crate) fn set_thread_state(&mut self, thread_idx: usize, target: ThreadState) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        };
        let Some(t) = self.threads.get(thread_idx) else {
            self.status_message = Some("No such thread; run :threads first".into());
            return;
        };
        if let Err(reason) = t.check_permission(&target) {
            self.status_message = Some(reason.to_string());
            return;
        }
        let thread_id = t.id.clone();
        let thread_id_for_rollback = thread_id.clone();
        let old_state = t.state.clone();
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let is_resolve = matches!(target, ThreadState::Resolved(_));

        if let Some(t_mut) = self.threads.get_mut(thread_idx) {
            t_mut.state = target.clone();
        }

        self.spawn_job(
            if is_resolve {
                "Resolving thread"
            } else {
                "Unresolving thread"
            },
            move || {
                let forge = factory(&repo);
                forge.set_state(pr, &thread_id, target)
            },
            move |app, result| match result {
                Ok(applied) => {
                    if let Some(t_mut) = app
                        .threads
                        .iter_mut()
                        .find(|t| t.id == thread_id_for_rollback)
                    {
                        t_mut.state = applied.applied.clone();
                    }
                    if let Some(note) = applied.summary_note() {
                        app.status_message = Some(format!("Thread resolved ({note})"));
                    } else {
                        let status = if applied.applied == ThreadState::Open {
                            "unresolved"
                        } else {
                            "resolved"
                        };
                        app.status_message = Some(format!("Thread {status}"));
                    }
                    app.refresh_threads_quietly(pr);
                }
                Err(e) => {
                    if let Some(t_mut) = app
                        .threads
                        .iter_mut()
                        .find(|t| t.id == thread_id_for_rollback)
                    {
                        t_mut.state = old_state;
                    }
                    app.status_message = Some(format!("Failed to update thread resolution: {e:#}"));
                }
            },
        );
    }

    pub(crate) fn retry_failed_comment(&mut self) {
        let Some(retry) = self.comment_retry.clone() else {
            self.status_message = Some("No failed comment to retry".into());
            return;
        };
        if let Some(thread_id) = retry.target_thread_id {
            if let Some(idx) = self.threads.iter().position(|t| t.id == thread_id) {
                self.post_reply(idx, &retry.body);
            } else {
                self.status_message = Some("Target thread no longer exists".into());
            }
        } else if let Some(pr) = self.pr.current_number {
            self.post_inline_to_target(&retry.path, retry.line, retry.side, &retry.body, pr);
        } else {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
        }
    }

    pub(crate) fn thread_command(&mut self, args: &[&str]) {
        let verb = args.first().copied().unwrap_or("");
        let rest = &args[1.min(args.len())..];
        let text = rest.get(1..).unwrap_or_default().join(" ");
        let thread_at = |app: &App| {
            rest.first()
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| n.checked_sub(1))
                .filter(|i| *i < app.threads.len())
        };
        match verb {
            "comment" => {
                let body = rest.join(" ");
                self.post_inline_comment(&body);
            }
            "reply" => {
                let Some(i) = thread_at(self) else {
                    self.status_message = Some(if rest.is_empty() {
                        USAGE.into()
                    } else {
                        "No such thread; run :threads first".into()
                    });
                    return;
                };
                self.post_reply(i, &text);
            }
            "resolve" | "unresolve" => {
                let Some(i) = thread_at(self) else {
                    self.status_message = Some(if rest.is_empty() {
                        USAGE.into()
                    } else {
                        "No such thread; run :threads first".into()
                    });
                    return;
                };
                self.set_thread_resolved(i, Some(verb == "resolve"));
            }
            "retry" => {
                self.retry_failed_comment();
            }
            "outdated" => {
                self.set_thread_filter(ThreadFilter::Outdated);
                self.open_threads_modal();
            }
            "refresh" => self.refresh_github_async(true),
            _ => self.status_message = Some(USAGE.into()),
        }
    }

    /// Reloads live threads after a thread action without replacing its status line.
    fn refresh_threads_quietly(&mut self, pr: u64) {
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let generation = self.review_generation;
        self.spawn_silent_job(
            move || factory(&repo).threads(pr),
            move |app, result| {
                if app.pr.current_number == Some(pr) && app.review_generation == generation {
                    if let Ok(threads) = result {
                        app.threads = app.normalize_threads(threads);
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_util::create_test_app;
    use lgtm_core::forge::{Applied, PrDiscussion};
    use lgtm_core::github::{Thread, ThreadComment};
    use lgtm_core::model::*;
    use std::sync::Mutex;

    /// Shared state for the `fn`-pointer forge factory; tests using it hold `LOCK`.
    static LOCK: Mutex<()> = Mutex::new(());
    static SENT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct Recording;
    impl lgtm_core::forge::PrDiscussion for Recording {
        fn threads(&self, _: u64) -> anyhow::Result<Vec<Thread>> {
            Ok(vec![thread()])
        }
        fn head_sha(&self, _: u64) -> anyhow::Result<String> {
            Ok("sha".into())
        }
        fn post_inline(
            &self,
            _: u64,
            _: &str,
            path: &str,
            line: u32,
            side: CommentSide,
            body: &str,
        ) -> anyhow::Result<u64> {
            SENT.lock()
                .unwrap()
                .push(format!("{path}:{line}:{side:?}:{body}"));
            Ok(1)
        }
        fn reply(&self, _: u64, t: &str, c: u64, body: &str) -> anyhow::Result<u64> {
            SENT.lock().unwrap().push(format!("reply:{t}:{c}:{body}"));
            Ok(2)
        }
        fn set_state(&self, _: u64, t: &str, state: ThreadState) -> anyhow::Result<Applied> {
            let is_resolved = matches!(state, ThreadState::Resolved(_));
            SENT.lock()
                .unwrap()
                .push(format!("resolve:{t}:{is_resolved}"));
            Ok(Applied::exact(state))
        }
    }
    impl lgtm_core::forge::Forge for Recording {
        fn list_prs(&self) -> anyhow::Result<Vec<lgtm_core::pr::PullRequestSummary>> {
            Ok(vec![])
        }
        fn pr_diff(&self, _: u64) -> anyhow::Result<DiffSnapshot> {
            Ok(DiffSnapshot::new(vec![]))
        }
        fn post_review(&self, _: u64, _: &str, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn thread() -> Thread {
        Thread {
            id: "T1".into(),
            path: "a.rs".into(),
            line: Some(2),
            state: ThreadState::Open,
            outdated: false,
            can_resolve: None,
            can_unresolve: None,
            comments: vec![ThreadComment {
                id: 9,
                author: "bob".into(),
                body: "hmm".into(),
            }],
        }
    }

    fn line(kind: LineType, old: Option<u32>, new: Option<u32>) -> DiffLine {
        DiffLine {
            line_type: kind,
            content: "x".into(),
            old_line_num: old,
            new_line_num: new,
        }
    }

    fn pr_app() -> App {
        let mut app = create_test_app();
        let hunk = DiffHunk {
            header: "@@ -4,2 +4,2 @@".into(),
            old_start: 4,
            old_lines: 2,
            new_start: 4,
            new_lines: 2,
            lines: vec![
                line(LineType::Deletion, Some(4), None),
                line(LineType::Addition, None, Some(4)),
                line(LineType::Context, Some(5), Some(5)),
            ],
            is_folded: false,
            folded_blocks: Vec::new(),
        };
        app.snapshot = DiffSnapshot::new(vec![FileDiff {
            old_path: None,
            new_path: "a.rs".into(),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 1,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        }]);
        app.pr.current_number = Some(7);
        app.threads = vec![thread()];
        app
    }

    fn status(app: &App) -> String {
        app.status_message.clone().unwrap_or_default()
    }

    fn wait_jobs(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            app.handle_background_events();
            if app.jobs_rx.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(50));
                app.handle_background_events();
                if app.jobs_rx.is_empty() {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn comment_needs_a_pr_and_valid_cursor() {
        let mut app = create_test_app();
        app.thread_command(&["comment", "hi"]);
        assert!(status(&app).starts_with("Comments require a PR"));
        app.thread_command(&["submit"]);
        assert!(status(&app).contains("Usage:"));

        let mut app = pr_app();
        app.cursor_line_idx = 9; // off hunk
        app.thread_command(&["comment", "off"]);
        assert!(status(&app).contains("not on a diff line"));

        for deprecated in ["submit", "drafts", "discard", "edit"] {
            app.thread_command(&[deprecated]);
            assert!(status(&app).contains("Usage:"), "{deprecated}");
        }
    }

    #[test]
    fn toggle_at_cursor_resolves_online() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        assert!(!app.toggle_resolve_thread_at_cursor());
        app.threads[0].line = Some(4);
        app.cursor_line_idx = 1;
        assert!(app.toggle_resolve_thread_at_cursor());
        wait_jobs(&mut app);
        assert_eq!(*SENT.lock().unwrap(), vec!["resolve:T1:true".to_string()]);
        assert!(app.threads[0].resolved());

        app.start_thread_reply(0);
        assert_eq!(app.command.input, "thread reply 1 ");
    }

    #[test]
    fn test_online_post_inline_dispatches_directly() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        app.cursor_line_idx = 1; // addition line 4
        app.post_inline_comment("online comment");
        wait_jobs(&mut app);
        assert_eq!(
            *SENT.lock().unwrap(),
            vec!["a.rs:4:Right:online comment".to_string()]
        );
        assert!(status(&app).contains("Comment #1 posted"));
        assert!(app.comment_retry.is_none());
    }

    #[test]
    fn test_online_post_reply_dispatches_directly() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        app.post_reply(0, "online reply");
        wait_jobs(&mut app);
        assert_eq!(
            *SENT.lock().unwrap(),
            vec!["reply:T1:9:online reply".to_string()]
        );
        assert!(status(&app).contains("Reply #2 posted"));
        assert!(app.comment_retry.is_none());
    }

    #[test]
    fn test_online_set_resolved_dispatches_directly() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        app.set_thread_resolved(0, Some(true));
        wait_jobs(&mut app);
        assert_eq!(*SENT.lock().unwrap(), vec!["resolve:T1:true".to_string()]);
        assert!(status(&app).contains("Thread resolved"));
    }

    #[test]
    fn test_online_failed_post_retains_retry_buffer() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        struct FailingForge;
        impl PrDiscussion for FailingForge {
            fn head_sha(&self, _: u64) -> anyhow::Result<String> {
                anyhow::bail!("Network down")
            }
        }
        impl lgtm_core::forge::Forge for FailingForge {
            fn list_prs(&self) -> anyhow::Result<Vec<lgtm_core::pr::PullRequestSummary>> {
                Ok(vec![])
            }
            fn pr_diff(&self, _: u64) -> anyhow::Result<DiffSnapshot> {
                Ok(DiffSnapshot::new(vec![]))
            }
            fn post_review(&self, _: u64, _: &str, _: &str) -> anyhow::Result<()> {
                Ok(())
            }
        }
        app.forge_factory = |_| Box::new(FailingForge);
        app.cursor_line_idx = 1;
        app.post_inline_comment("failed comment text");
        wait_jobs(&mut app);
        assert!(status(&app).contains("Failed to post comment"));
        let retry = app
            .comment_retry
            .as_ref()
            .expect("retry buffer must be set");
        assert_eq!(retry.body, "failed comment text");
        assert_eq!(retry.path, "a.rs");
        assert_eq!(retry.line, 4);
    }

    #[test]
    fn test_online_retry_command() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        app.comment_retry = Some(CommentRetryBuffer {
            path: "a.rs".into(),
            line: 4,
            side: CommentSide::Right,
            body: "retried comment".into(),
            target_thread_id: None,
        });
        app.thread_command(&["retry"]);
        wait_jobs(&mut app);
        assert_eq!(
            *SENT.lock().unwrap(),
            vec!["a.rs:4:Right:retried comment".to_string()]
        );
        assert!(app.comment_retry.is_none());
    }
}
