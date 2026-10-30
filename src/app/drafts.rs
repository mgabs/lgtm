//! Draft-then-submit PR comments: nothing leaves the machine until `:thread submit` is confirmed.
use super::*;
use lgtm_core::comment::CommentSide;
use lgtm_core::discussion::{self, store, DraftKind};

const USAGE: &str = "Usage: :thread comment <text> | reply <n> <text> | resolve <n> | unresolve <n> | edit <id> <text> | discard <id|all> | drafts | submit | refresh";

impl App {
    fn repo_key(&self) -> String {
        lgtm_core::forge::origin_url(&self.git_engine.repo_path)
            .unwrap_or_else(|| self.git_engine.repo_path.display().to_string())
            .to_ascii_lowercase()
    }

    pub(crate) fn load_drafts(&mut self, pr: u64) {
        self.drafts = store::load(&self.git_engine.repo_path, pr, &self.repo_key());
        self.submit_confirm = false;
    }

    fn persist_drafts(&mut self) {
        let Some(pr) = self.pr.current_number else {
            return;
        };
        if let Err(e) = store::save(
            &self.git_engine.repo_path,
            pr,
            &self.repo_key(),
            &self.drafts,
        ) {
            self.status_message = Some(format!("Could not save drafts: {e:#}"));
        }
    }

    /// Resolved state after pending drafts are applied.
    fn effective_resolved(&self, thread: &lgtm_core::github::Thread) -> bool {
        self.drafts
            .items()
            .iter()
            .rev()
            .find_map(|d| match &d.kind {
                DraftKind::SetResolved {
                    thread_id,
                    resolved,
                } if *thread_id == thread.id => Some(*resolved),
                _ => None,
            })
            .unwrap_or(thread.resolved)
    }

    pub fn thread_pending_label(&self, id: &str) -> Option<&'static str> {
        self.drafts
            .items()
            .iter()
            .rev()
            .find_map(|d| match &d.kind {
                DraftKind::SetResolved {
                    thread_id,
                    resolved,
                } if thread_id == id => Some(if *resolved {
                    "DRAFT resolve"
                } else {
                    "DRAFT unresolve"
                }),
                DraftKind::Reply { thread_id, .. } if thread_id == id => Some("DRAFT reply"),
                _ => None,
            })
    }

    pub fn toggle_resolve_thread_at_cursor(&mut self) -> bool {
        let Some(idx) = self.thread_at_cursor() else {
            return false;
        };
        self.draft_resolve(idx, None);
        true
    }

    /// Drafts resolve (`Some(true)`), unresolve (`Some(false)`) or the opposite of the current state.
    pub(crate) fn draft_resolve(&mut self, idx: usize, resolved: Option<bool>) {
        let Some(t) = self.threads.get(idx) else {
            self.status_message = Some("No such thread; run :threads first".into());
            return;
        };
        let wanted = resolved.unwrap_or(!self.effective_resolved(t));
        let kind = DraftKind::SetResolved {
            thread_id: t.id.clone(),
            resolved: wanted,
        };
        self.add_draft(kind, "");
    }

    pub(crate) fn start_thread_reply(&mut self, index: usize) {
        self.thread_selected_idx = index;
        self.command
            .set_text(format!("thread reply {} ", index + 1));
        self.command.history_idx = None;
        self.active_view = ActiveView::CommandLine;
    }

    fn add_draft(&mut self, kind: DraftKind, body: &str) {
        if self.pr.current_number.is_none() {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        }
        if self.submitting {
            self.status_message = Some("Submit in progress; drafts are locked".into());
            return;
        }
        self.submit_confirm = false;
        match self.drafts.add(kind, body) {
            Ok(id) => {
                let label = self.drafts.get(id).map_or("", |d| d.kind.label());
                self.status_message = Some(format!(
                    "Drafted {label} #{id} ({}); :thread submit to send",
                    self.drafts.summary()
                ));
                self.invalidate_diff_content();
                self.persist_drafts();
            }
            Err(e) => self.status_message = Some(e.to_string()),
        }
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

    pub(crate) fn draft_inline_here(&mut self, body: &str) {
        if self.pr.current_number.is_none() {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        }
        let Some((path, line, side)) = self.cursor_anchor() else {
            self.status_message = Some("Cursor is not on a diff line of a hunk".into());
            return;
        };
        self.add_draft(DraftKind::Thread { path, line, side }, body);
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
                self.draft_inline_here(&body);
            }
            "general" => self.add_draft(DraftKind::General, &rest.join(" ")),
            "reply" | "resolve" | "unresolve" => {
                let Some(i) = thread_at(self) else {
                    self.status_message = Some(if rest.is_empty() {
                        USAGE.into()
                    } else {
                        "No such thread; run :threads first".into()
                    });
                    return;
                };
                match verb {
                    "reply" => {
                        let t = &self.threads[i];
                        let comment_id = t.comments.first().map_or(0, |c| c.id);
                        if comment_id == 0 {
                            self.status_message =
                                Some("Thread has no reply target; run :thread refresh".into());
                            return;
                        }
                        let kind = DraftKind::Reply {
                            thread_id: t.id.clone(),
                            comment_id,
                        };
                        self.add_draft(kind, &text);
                    }
                    _ => self.draft_resolve(i, Some(verb == "resolve")),
                }
            }
            "edit" => {
                let id = rest.first().and_then(|n| n.parse::<u64>().ok());
                match id.filter(|id| self.drafts.get(*id).is_some()) {
                    Some(id) => match self.drafts.edit(id, &text) {
                        Ok(()) => {
                            self.status_message = Some(format!("Edited draft #{id}"));
                            self.persist_drafts();
                        }
                        Err(e) => self.status_message = Some(e.to_string()),
                    },
                    None => self.status_message = Some("No such draft; see :thread drafts".into()),
                }
            }
            "discard" => self.discard_drafts(rest.first().copied()),
            "drafts" => {
                self.status_message = Some(if self.drafts.is_empty() {
                    "No drafts".into()
                } else {
                    let lines: Vec<String> = self
                        .drafts
                        .items()
                        .iter()
                        .map(|d| {
                            let note = d
                                .stale
                                .as_deref()
                                .map(|s| format!(" STALE: {s}"))
                                .or_else(|| d.error.as_deref().map(|e| format!(" FAILED: {e}")))
                                .unwrap_or_default();
                            format!("#{} {} {}{note}", d.id, d.kind.label(), d.target())
                        })
                        .collect();
                    lines.join(" | ")
                });
            }
            "submit" => self.submit_drafts(),
            "refresh" => self.refresh_github_async(true),
            _ => self.status_message = Some(USAGE.into()),
        }
        if verb != "submit" {
            self.submit_confirm = false;
        }
    }

    fn discard_drafts(&mut self, which: Option<&str>) {
        if self.submitting {
            self.status_message = Some("Submit in progress; drafts are locked".into());
            return;
        }
        let message = match which {
            Some("all") => {
                let n = self.drafts.items().len();
                self.drafts.clear();
                format!("Discarded {n} drafts")
            }
            Some(id) if id.parse().is_ok_and(|id| self.drafts.remove(id)) => {
                format!("Discarded draft #{id}")
            }
            _ => "No such draft; see :thread drafts".into(),
        };
        self.status_message = Some(message);
        self.invalidate_diff_content();
        self.persist_drafts();
    }

    /// First call shows the batch; the second confirms and sends it off the UI thread.
    fn submit_drafts(&mut self) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Comments require a PR (:pr <n>)".into());
            return;
        };
        if self.submitting {
            self.status_message = Some("Submit already in progress".into());
            return;
        }
        if self.drafts.is_empty() {
            self.status_message = Some("No drafts to submit".into());
            return;
        }
        if !self.submit_confirm {
            self.submit_confirm = true;
            self.status_message = Some(format!(
                "Submit {} to PR #{pr}? Run :thread submit again to confirm (:thread drafts to review)",
                self.drafts.summary()
            ));
            return;
        }
        self.submit_confirm = false;
        self.submitting = true;
        let mut drafts = self.drafts.clone();
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let generation = self.review_generation;
        self.spawn_job(
            "Submitting drafts",
            move || {
                let result = discussion::submit(&factory(&repo), pr, &mut drafts);
                (drafts, result)
            },
            move |app, (drafts, result)| {
                app.submitting = false;
                let current = app.pr.current_number == Some(pr);
                let message = match &result {
                    Ok(report) => report.message(),
                    Err(e) => format!("Submit failed before sending anything: {e:#}"),
                };
                if result.is_ok() {
                    let key = app.repo_key();
                    if let Err(e) = store::save(&app.git_engine.repo_path, pr, &key, &drafts) {
                        app.status_message = Some(format!("Could not save drafts: {e:#}"));
                        return;
                    }
                    if current {
                        app.drafts = drafts;
                        app.invalidate_diff_content();
                    }
                }
                app.quit_after_jobs = app.quit_after_jobs && result.is_ok();
                if current && app.review_generation == generation {
                    app.refresh_threads_quietly(pr);
                }
                app.status_message = Some(message);
            },
        );
    }

    /// Reloads live threads after a submit without replacing the submit report in the status line.
    fn refresh_threads_quietly(&mut self, pr: u64) {
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        let generation = self.review_generation;
        self.spawn_silent_job(
            move || factory(&repo).threads(pr),
            move |app, result| {
                if app.pr.current_number == Some(pr) && app.review_generation == generation {
                    if let Ok(threads) = result {
                        app.drafts.mark_stale(&threads);
                        app.threads = threads;
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
        fn set_resolved(&self, _: u64, t: &str, r: bool) -> anyhow::Result<()> {
            SENT.lock().unwrap().push(format!("resolve:{t}:{r}"));
            Ok(())
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
            resolved: false,
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

    fn wait_submit(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.submitting {
            app.handle_background_events();
            assert!(std::time::Instant::now() < deadline, "submit hung");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        app.handle_background_events();
    }

    #[test]
    fn drafting_needs_a_pr_and_makes_no_forge_call() {
        let mut app = create_test_app();
        app.thread_command(&["comment", "hi"]);
        assert!(status(&app).starts_with("Comments require a PR"));
        app.thread_command(&["submit"]);
        assert!(status(&app).starts_with("Comments require a PR"));
        let mut app = pr_app();
        app.thread_command(&["comment", "hi"]);
        assert!(status(&app).starts_with("Drafted thread #1 (1 thread)"));
        assert_eq!(app.drafts.items().len(), 1);
    }

    #[test]
    fn inline_side_follows_the_line_type_and_drafts_persist() {
        let mut app = pr_app();
        app.cursor_line_idx = 0;
        app.thread_command(&["comment", "on", "deleted"]);
        app.cursor_line_idx = 1;
        app.thread_command(&["comment", "on", "added"]);
        app.cursor_line_idx = 2;
        app.thread_command(&["comment", "ctx"]);
        let sides: Vec<_> = app
            .drafts
            .items()
            .iter()
            .map(|d| match &d.kind {
                DraftKind::Thread { line, side, .. } => (*line, *side),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            sides,
            vec![
                (4, CommentSide::Left),
                (4, CommentSide::Right),
                (5, CommentSide::Right)
            ]
        );
        app.cursor_line_idx = 9;
        app.thread_command(&["comment", "off"]);
        assert!(status(&app).contains("not on a diff line"));
        app.thread_command(&["comment", " "]);
        assert_eq!(app.drafts.items().len(), 3);
        let saved =
            lgtm_core::discussion::store::load(&app.git_engine.repo_path, 7, &app.repo_key());
        assert_eq!(saved, app.drafts);
        app.load_drafts(7);
        assert_eq!(app.drafts.items().len(), 3);
    }

    #[test]
    fn reply_resolve_edit_discard_and_listing() {
        let mut app = pr_app();
        app.thread_command(&["reply"]);
        assert!(status(&app).starts_with("Usage"));
        app.thread_command(&["reply", "9", "x"]);
        assert!(status(&app).starts_with("No such thread"));
        app.thread_command(&["reply", "1", "thanks", "all"]);
        app.thread_command(&["resolve", "1"]);
        app.thread_command(&["unresolve", "1"]);
        assert_eq!(app.drafts.summary(), "1 reply, 1 unresolve");
        assert_eq!(app.thread_pending_label("T1"), Some("DRAFT unresolve"));
        app.thread_command(&["resolve", "1"]);
        assert_eq!(app.thread_pending_label("T1"), Some("DRAFT resolve"));
        app.drafts.remove(4);
        assert_eq!(app.thread_pending_label("T1"), Some("DRAFT reply"));
        assert_eq!(app.thread_pending_label("none"), None);
        app.thread_command(&["edit", "1", "new", "text"]);
        assert_eq!(app.drafts.get(1).unwrap().body, "new text");
        app.thread_command(&["edit", "1", " "]);
        assert!(status(&app).contains("empty"));
        app.thread_command(&["edit", "42", "x"]);
        assert!(status(&app).starts_with("No such draft"));
        app.thread_command(&["drafts"]);
        assert!(status(&app).contains("#1 reply thread T1"));
        app.drafts.mark_stale(&[]);
        app.thread_command(&["drafts"]);
        assert!(status(&app).contains("STALE: thread not found"));
        app.thread_command(&["discard", "1"]);
        app.thread_command(&["discard", "nope"]);
        assert!(status(&app).starts_with("No such draft"));
        app.thread_command(&["discard", "all"]);
        assert!(app.drafts.is_empty());
        app.thread_command(&["drafts"]);
        assert_eq!(status(&app), "No drafts");
        app.thread_command(&["general", "overall"]);
        assert_eq!(app.drafts.summary(), "1 general");
        app.thread_command(&["nuke"]);
        assert!(status(&app).starts_with("Usage"));
        let mut lone = pr_app();
        lone.threads[0].comments.clear();
        lone.thread_command(&["reply", "1", "x"]);
        assert!(status(&lone).contains("no reply target"));
    }

    #[test]
    fn toggle_at_cursor_flips_the_effective_state() {
        let mut app = pr_app();
        assert!(!app.toggle_resolve_thread_at_cursor());
        app.threads[0].line = Some(4);
        app.cursor_line_idx = 1;
        assert!(app.toggle_resolve_thread_at_cursor());
        assert!(app.toggle_resolve_thread_at_cursor());
        assert_eq!(app.drafts.items().len(), 1);
        assert_eq!(app.thread_pending_label("T1"), Some("DRAFT unresolve"));
        app.draft_resolve(5, None);
        assert!(status(&app).starts_with("No such thread"));
        app.start_thread_reply(0);
        assert_eq!(app.command.input, "thread reply 1 ");
    }

    #[test]
    fn submit_needs_confirmation_then_sends_and_clears() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        SENT.lock().unwrap().clear();
        let mut app = pr_app();
        app.forge_factory = |_| Box::new(Recording);
        app.thread_command(&["submit"]);
        assert_eq!(status(&app), "No drafts to submit");
        app.cursor_line_idx = 0;
        app.thread_command(&["comment", "n"]);
        app.thread_command(&["reply", "1", "r"]);
        app.thread_command(&["resolve", "1"]);
        app.thread_command(&["submit"]);
        assert!(status(&app).contains("1 thread, 1 reply, 1 resolve"));
        assert!(status(&app).contains("again to confirm"));
        assert!(SENT.lock().unwrap().is_empty());
        app.thread_command(&["drafts"]);
        app.thread_command(&["submit"]);
        assert!(status(&app).contains("again to confirm"), "confirm resets");
        app.thread_command(&["submit"]);
        assert!(app.submitting);
        app.thread_command(&["comment", "late"]);
        assert!(status(&app).contains("locked"));
        app.thread_command(&["discard", "all"]);
        assert!(status(&app).contains("locked"));
        app.thread_command(&["submit"]);
        assert!(status(&app).contains("already in progress"));
        assert!(app.has_pending_writes());
        wait_submit(&mut app);
        assert!(app.drafts.is_empty());
        assert_eq!(status(&app), "Submitted 3");
        assert_eq!(
            *SENT.lock().unwrap(),
            vec![
                "a.rs:4:Left:n".to_string(),
                "reply:T1:9:r".into(),
                "resolve:T1:true".into()
            ]
        );
        assert!(
            !lgtm_core::discussion::store::load(&app.git_engine.repo_path, 7, &app.repo_key())
                .items()
                .iter()
                .any(|_| true)
        );
    }

    #[test]
    fn submit_failure_before_sending_keeps_every_draft() {
        let mut app = pr_app();
        app.thread_command(&["reply", "1", "keep me"]);
        app.thread_command(&["submit"]);
        app.thread_command(&["submit"]);
        wait_submit(&mut app);
        assert!(status(&app).starts_with("Submit failed before sending anything"));
        assert_eq!(app.drafts.items()[0].body, "keep me");
        assert!(!app.submitting);
    }
}
