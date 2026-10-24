#[cfg(test)]
mod completion_benchmark;
use super::*;
mod completion;

pub(crate) struct LoadedPr {
    pub pr: u64,
    pub generation: u64,
    pub snapshot: DiffSnapshot,
    pub findings: lgtm_core::findings::Findings,
    pub repo: PathBuf,
    completion: completion::Prepared,
}

impl App {
    /// Runs slow `work` on a thread and applies its result on the UI thread in a later tick.
    pub(crate) fn spawn_job<T: Send + 'static>(
        &mut self,
        label: &str,
        work: impl FnOnce() -> T + Send + 'static,
        done: impl FnOnce(&mut App, T) + Send + 'static,
    ) {
        self.status_message = Some(format!("{label}..."));
        self.spawn_silent_job(work, done);
    }

    pub(crate) fn spawn_silent_job<T: Send + 'static>(
        &mut self,
        work: impl FnOnce() -> T + Send + 'static,
        done: impl FnOnce(&mut App, T) + Send + 'static,
    ) {
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let timing = lgtm_core::debug::Span::new("ui.background_job");
            let out = work();
            drop(timing);
            let mut waiting = lgtm_core::debug::Span::new("ui.callback_wait");
            waiting.count("applied", 0);
            let _ = tx.send(Box::new(move |app: &mut App| {
                waiting.count("applied", 1);
                drop(waiting);
                done(app, out);
            }));
        });
    }

    pub fn load_pr_async(&mut self, pr: u64) {
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        self.load_pr_job(pr, move || factory(&repo).pr_diff(pr));
    }

    fn load_pr_job(
        &mut self,
        pr: u64,
        work: impl FnOnce() -> anyhow::Result<DiffSnapshot> + Send + 'static,
    ) {
        self.invalidate_local_refresh();
        self.invalidate_audit(false);
        self.review_generation += 1;
        let generation = self.review_generation;
        self.pr_load_latest
            .store(generation, std::sync::atomic::Ordering::Relaxed);
        let latest = self.pr_load_latest.clone();
        self.pr_loading = Some((pr, generation));
        self.ready_pr = None;
        let config = self.config.clone();
        let repo = self.git_engine.repo_path.clone();
        let completion_input = completion::Input::capture(self, pr);
        let lock = self.pr_load_lock.clone();
        let progress = self.jobs_tx.clone();
        self.spawn_job(
            &format!("Fetching and building PR #{pr} diff"),
            move || {
                // Git fetches share refs; keep overlapping loads from contending for ref locks.
                let mut snapshot = {
                    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                    if latest.load(std::sync::atomic::Ordering::Relaxed) != generation {
                        anyhow::bail!("superseded PR load");
                    }
                    work()?
                };
                if latest.load(std::sync::atomic::Ordering::Relaxed) != generation {
                    anyhow::bail!("superseded PR load");
                }
                let _ = progress.send(Box::new(move |app| {
                    if app.pr_loading == Some((pr, generation)) {
                        app.status_message = Some(format!("Scanning PR #{pr}..."));
                    }
                }));
                let findings = scan_findings(&repo, &snapshot.files, &config);
                let completion =
                    completion::prepare(&repo, pr, &mut snapshot, completion_input, || {
                        latest.load(std::sync::atomic::Ordering::Relaxed) == generation
                    })?;
                Ok::<_, anyhow::Error>(LoadedPr {
                    pr,
                    generation,
                    snapshot,
                    findings,
                    repo,
                    completion,
                })
            },
            move |app, result| {
                if app.pr_loading != Some((pr, generation)) || app.review_generation != generation {
                    return;
                }
                match result {
                    Ok(ready) => {
                        app.ready_pr = Some(ready);
                        app.apply_ready_pr();
                    }
                    Err(error) => {
                        app.pr_loading = None;
                        app.status_message =
                            Some(format!("PR #{pr} failed: {error:#}; retry with :pr {pr}"))
                    }
                }
            },
        );
    }

    pub(crate) fn apply_ready_pr(&mut self) {
        let Some(ready) = self.ready_pr.as_ref() else {
            return;
        };
        if ready.generation != self.review_generation {
            self.ready_pr = None;
            return;
        }
        if ready.repo != self.git_engine.repo_path {
            self.ready_pr = None;
            self.pr_loading = None;
            self.status_message = Some("PR repository changed; current review kept".into());
            return;
        }
        // Do not change a draft's review target or cursor anchor underneath the editor.
        if self.active_view != ActiveView::Diff || self.visual_mode {
            self.status_message = Some(format!(
                "PR #{} ready; close this view or selection to switch",
                ready.pr
            ));
            return;
        }
        let ready = self.ready_pr.take().unwrap();
        if !ready.completion.matches(self, ready.pr) {
            self.reprepare_pr(ready);
            return;
        }
        self.pr_loading = None;
        if self.apply_prepared_pr(ready) {
            self.refresh_github_async(true);
        }
    }

    fn apply_prepared_pr(&mut self, ready: LoadedPr) -> bool {
        let LoadedPr {
            pr,
            snapshot,
            findings,
            completion,
            ..
        } = ready;
        let _timing = lgtm_core::debug::Span::new("ui.pr_apply");
        self.pr.current_number = Some(pr);
        self.active_revision = None;
        self.snapshot = snapshot;
        self.invalidate_diff_content();
        self.alerts = findings.active;
        *self.hidden.borrow_mut() = Hidden {
            suppressed: findings.suppressed,
            existing: findings.existing,
        };
        self.selected_file_idx = 0;
        self.cursor_line_idx = 0;
        self.diff_scroll_offset = 0;
        self.threads.clear();
        self.thread_revision += 1;
        self.ci = None;
        self.thread_selected_idx = 0;
        self.thread_scroll_offset = 0;
        // Only target directives were prepared. All unrelated directives and posting receipts
        // remain owned by the UI; matching was checked immediately before this merge.
        for (current, updated) in self
            .comments
            .iter_mut()
            .filter(|c| c.pr_number == Some(pr))
            .zip(completion.comments)
        {
            *current = updated;
        }
        self.marks = completion.marks;
        self.invalidate_diff_folds();
        let mut persisted = true;
        if completion.changed {
            // Retain the UI-owned durable write boundary used by remote posting receipts.
            let _timing = lgtm_core::debug::Span::new("review.persist_pr_directives");
            if let Err(error) = self.comment_store.save_comments(&self.comments) {
                persisted = false;
                self.status_message = Some(format!(
                    "PR loaded; directive updates could not be saved: {error}"
                ));
            }
        }
        self.update_file_picker_filter();
        persisted
    }

    #[cfg(test)]
    pub(crate) fn apply_pr_snapshot(
        &mut self,
        pr: u64,
        mut snapshot: DiffSnapshot,
        findings: lgtm_core::findings::Findings,
    ) {
        let input = completion::Input::capture(self, pr);
        let completion =
            completion::prepare(&self.git_engine.repo_path, pr, &mut snapshot, input, || {
                true
            })
            .unwrap();
        self.apply_prepared_pr(LoadedPr {
            pr,
            generation: self.review_generation,
            repo: self.git_engine.repo_path.clone(),
            snapshot,
            findings,
            completion,
        });
    }

    pub fn open_pr_picker(&mut self) {
        self.open_pr_picker_state(self.pr.filter);
    }

    pub fn open_pr_picker_state(&mut self, state: crate::pr::PullRequestState) {
        self.pr.filter = state;
        self.pr.list_generation += 1;
        let generation = self.pr.list_generation;
        self.pr.list.clear();
        self.pr.selected_idx = 0;
        self.active_view = ActiveView::PrPicker;
        let repo = self.git_engine.repo_path.clone();
        let factory = self.forge_factory;
        self.spawn_job(
            &format!("Loading {} PRs", state.as_str()),
            move || factory(&repo).list_prs_state(state),
            move |app, res| {
                if app.pr.list_generation != generation {
                    return;
                }
                match res {
                    Ok(prs) => {
                        app.status_message = Some(format!("{} {} PRs", prs.len(), state.as_str()));
                        app.pr.list = prs;
                    }
                    Err(e) => app.status_message = Some(format!("PR list failed: {e}")),
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::create_test_app;
    use super::*;

    #[test]
    fn watcher_does_not_reload_remote_pr() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.status_message = Some("unchanged".into());
        let (tx, rx) = crossbeam_channel::unbounded();
        app.watcher_rx = rx;
        tx.send(WatcherMessage::FilesChanged(vec![app
            .git_engine
            .repo_path
            .join("src/lib.rs")]))
            .unwrap();
        app.handle_background_events();
        assert_eq!(app.status_message.as_deref(), Some("unchanged"));
    }

    #[test]
    fn comment_write_does_not_reload_local_diff() {
        let mut app = create_test_app();
        app.status_message = Some("unchanged".into());
        let (tx, rx) = crossbeam_channel::unbounded();
        app.watcher_rx = rx;
        tx.send(WatcherMessage::FilesChanged(vec![app
            .git_engine
            .repo_path
            .join(".lgtm/comments.jsonl")]))
            .unwrap();
        app.handle_background_events();
        assert_eq!(app.status_message.as_deref(), Some("unchanged"));
    }
    fn thread(id: &str) -> lgtm_core::github::Thread {
        lgtm_core::github::Thread {
            id: id.into(),
            path: "a.rs".into(),
            line: Some(1),
            resolved: false,
            comments: vec![],
        }
    }

    #[test]
    fn resolution_runs_in_background_and_applies_by_id() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first"), thread("second")];
        let (tx, rx) = crossbeam_channel::bounded(1);
        app.resolve_thread_job(42, "first".into(), true, move || {
            rx.recv().unwrap();
            Ok(())
        });
        // The worker cannot finish until this UI-side send. Selection/order may change meanwhile.
        assert!(!app.threads[0].resolved);
        app.threads.swap(0, 1);
        tx.send(()).unwrap();
        let job = app
            .jobs_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        job(&mut app);
        assert!(!app.threads[0].resolved);
        assert!(app.threads[1].resolved);
    }

    #[test]
    fn old_pr_resolution_is_ignored_and_failed_resolution_stays_open() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        app.resolve_thread_job(42, "first".into(), true, || Ok(()));
        let job = app
            .jobs_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        app.pr.current_number = Some(43);
        app.status_message = Some("new PR".into());
        job(&mut app);
        assert!(!app.threads[0].resolved);
        assert_eq!(app.status_message.as_deref(), Some("new PR"));
        app.resolve_thread_job(43, "first".into(), true, || anyhow::bail!("denied"));
        let job = app
            .jobs_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        job(&mut app);
        assert!(!app.threads[0].resolved);
        assert!(app.status_message.as_deref().unwrap().contains("denied"));
    }
    fn apply_next(app: &mut App) {
        let job = app
            .jobs_rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("background job did not complete");
        job(app);
    }

    fn directive(text: &str, line: u32) -> CommentAnchor {
        serde_json::from_value(serde_json::json!({"id":lgtm_core::comment::new_id(), "file_path":"a.rs", "target_line_no":line,
            "target_code":"code", "context_before":[], "context_after":[], "comment_text":text, "created_at_turn":1,
            "is_resolved":false, "pr_number":42})).unwrap()
    }

    #[test]
    fn newest_load_wins_and_failure_preserves_current_review() {
        let mut app = create_test_app();
        app.pr.current_number = Some(7);
        app.threads = vec![thread("keep")];
        app.comments = vec![directive("keep draft", 1)];
        let (started_tx, started_rx) = crossbeam_channel::bounded(1);
        let (release_tx, release_rx) = crossbeam_channel::bounded(1);
        app.load_pr_job(8, move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(DiffSnapshot::default())
        });
        started_rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        assert_eq!(app.pr.current_number, Some(7));
        app.load_pr_job(9, || anyhow::bail!("network unavailable"));
        release_tx.send(()).unwrap();
        apply_next(&mut app);
        apply_next(&mut app);
        assert_eq!(app.pr.current_number, Some(7));
        assert_eq!(app.threads[0].id, "keep");
        assert_eq!(app.comments[0].comment_text, "keep draft");
        assert!(app.pr_loading.is_none());
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("retry with :pr 9"));
    }

    #[test]
    fn applying_pr_snapshot_clears_old_navigation_and_threads() {
        let mut app = create_test_app();
        app.pr.current_number = Some(7);
        app.active_revision = Some("old-revision".into());
        app.threads = vec![thread("old")];
        app.selected_file_idx = 5;
        app.cursor_line_idx = 20;
        app.apply_pr_snapshot(42, DiffSnapshot::default(), Default::default());
        assert_eq!(app.pr.current_number, Some(42));
        assert!(app.active_revision.is_none());
        assert!(app.threads.is_empty());
        assert_eq!(app.selected_file_idx, 0);
        assert_eq!(app.cursor_line_idx, 0);
    }

    #[test]
    fn thread_queue_orders_actions_and_deduplicates_pending_requests() {
        use super::super::comments::{ThreadAction, ThreadOutcome};
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        let (tx, rx) = crossbeam_channel::bounded(1);
        let action = ThreadAction::Reply {
            target: 1,
            text: "reply".into(),
        };
        app.enqueue_thread_job(42, "first".into(), action.clone(), move || {
            rx.recv().unwrap();
            Ok(ThreadOutcome::Reply(20))
        });
        app.enqueue_thread_job(42, "first".into(), action, || {
            panic!("duplicate reply executed")
        });
        app.resolve_thread_job(42, "first".into(), true, || Ok(()));
        assert_eq!(app.thread_jobs.get(&(42, "first".into())).unwrap().len(), 2);
        assert!(!app.threads[0].resolved);
        tx.send(()).unwrap();
        apply_next(&mut app);
        assert_eq!(app.threads[0].comments[0].body, "reply");
        assert_eq!(app.threads[0].comments[0].id, 20);
        assert!(!app.threads[0].resolved);
        apply_next(&mut app);
        assert!(app.threads[0].resolved);
        assert!(app.thread_jobs.is_empty());
    }

    #[test]
    fn bdd_ui_006_successful_reply_survives_failed_refresh_without_resubmission() {
        use super::super::comments::{ThreadAction, ThreadOutcome};
        // Given an open thread and a reply draft. The test app's provider rejects
        // reads, so the automatic reconciliation after success will fail.
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        let key = (42, "first".to_string());
        app.reply_drafts
            .insert(key.clone(), "accepted reply".into());
        let writes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let worker_writes = writes.clone();

        // When posting succeeds and the subsequent thread refresh fails.
        app.enqueue_thread_job(
            42,
            "first".into(),
            ThreadAction::Reply {
                target: 1,
                text: "accepted reply".into(),
            },
            move || {
                worker_writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ThreadOutcome::Reply(20))
            },
        );
        apply_next(&mut app); // acknowledged write
        apply_next(&mut app); // failed read through Unsupported test provider

        // Then the accepted reply stays visible, with no retry draft or new write.
        assert_eq!(app.threads[0].comments.len(), 1);
        assert_eq!(app.threads[0].comments[0].id, 20);
        assert_eq!(app.threads[0].comments[0].body, "accepted reply");
        assert!(!app.reply_drafts.contains_key(&key));
        assert!(app.thread_jobs.is_empty());
        assert_eq!(app.status_message.as_deref(), Some("Reply posted"));
        assert_eq!(writes.load(std::sync::atomic::Ordering::SeqCst), 1);
        app.start_thread_reply(0);
        assert_eq!(app.command.input, "thread reply 1 ");
    }

    #[test]
    fn bdd_ui_007_acknowledged_reply_preserves_a_newer_draft() {
        use super::super::comments::{ThreadAction, ThreadOutcome};
        // Given a submitted reply whose completion has not yet been applied.
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        let key = (42, "first".to_string());
        app.reply_drafts.insert(key.clone(), "submitted".into());
        app.enqueue_thread_job(
            42,
            "first".into(),
            ThreadAction::Reply {
                target: 1,
                text: "submitted".into(),
            },
            || Ok(ThreadOutcome::Reply(20)),
        );

        // When the reviewer writes a newer draft before the acknowledgement arrives.
        app.reply_drafts
            .insert(key.clone(), "newer unsent draft".into());
        apply_next(&mut app);
        apply_next(&mut app);

        // Then only the submitted text appears remotely; the newer draft stays editable.
        assert_eq!(app.threads[0].comments[0].body, "submitted");
        assert_eq!(app.reply_drafts.get(&key).unwrap(), "newer unsent draft");
        app.start_thread_reply(0);
        assert_eq!(app.command.input, "thread reply 1 newer unsent draft");
    }

    #[test]
    fn bdd_ui_008_failed_resolution_can_be_retried_and_reopened() {
        // Given an open discussion whose first resolution request is rejected.
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        app.resolve_thread_job(42, "first".into(), true, || anyhow::bail!("denied"));
        apply_next(&mut app);
        assert!(!app.threads[0].resolved);
        assert!(app.thread_jobs.is_empty());
        assert!(app.status_message.as_deref().unwrap().contains("denied"));

        // When the reviewer retries successfully and later reopens the discussion.
        app.resolve_thread_job(42, "first".into(), true, || Ok(()));
        apply_next(&mut app);
        assert!(app.threads[0].resolved);
        app.resolve_thread_job(42, "first".into(), false, || Ok(()));
        apply_next(&mut app);

        // Then the discussion is active again and no pending action remains.
        assert!(!app.threads[0].resolved);
        assert!(app.thread_jobs.is_empty());
    }

    #[test]
    fn failed_remote_reply_retains_its_draft() {
        use super::super::comments::ThreadAction;
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        app.enqueue_thread_job(
            42,
            "first".into(),
            ThreadAction::Reply {
                target: 1,
                text: "keep me".into(),
            },
            || anyhow::bail!("denied"),
        );
        apply_next(&mut app);
        assert_eq!(
            app.reply_drafts.get(&(42, "first".into())).unwrap(),
            "keep me"
        );
        assert!(app.threads[0].comments.is_empty());
    }

    #[test]
    fn background_post_preserves_edits_and_success_before_later_failure() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.snapshot = lgtm_core::pr::PrEngine::parse_unified_diff(
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -0,0 +1,2 @@\n+first\n+second\n",
        )
        .unwrap();
        app.comments = vec![directive("first", 1), directive("second", 2)];
        app.post_pr_comments_job(true, || {
            Box::new(lgtm_core::github::FakeClient {
                fail_post_after: Some(1),
                ..Default::default()
            })
        });
        app.post_pr_comments_job(true, || panic!("duplicate batch executed"));
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("already being posted"));
        app.comments[0].add_reply("edited while posting");
        app.comments.push(directive("new draft", 3));
        app.comments.swap(0, 1);
        apply_next(&mut app); // receipt is merged by identity, not the old list index
        assert!(app.comments[1].remote_id.is_some());
        assert_eq!(app.comments[1].replies, ["edited while posting"]);
        apply_next(&mut app); // later post fails; first receipt is already durable
        assert!(app.comments[0].remote_id.is_none());
        assert!(app.comments[2].remote_id.is_none());
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("1 of 2 posted; stopped"));
        assert!(app.posting_prs.is_empty());
        assert!(app.posting_comments.is_empty());
        let stored = app.comment_store.load_comments();
        assert_eq!(stored, app.comments);
        let retry = lgtm_core::github::post::plan(&app.comments, &app.snapshot.files, 42);
        assert_eq!(retry.total(), 2);
    }

    #[test]
    fn normal_quit_waits_for_successful_post_receipt_to_be_saved() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.comments = vec![directive("general comment", 1)];
        app.post_pr_comments_job(true, || Box::new(lgtm_core::github::FakeClient::default()));
        app.request_quit();
        assert!(!app.should_quit);
        apply_next(&mut app);
        assert!(app.comment_store.load_comments()[0].remote_id.is_some());
        apply_next(&mut app);
        app.handle_background_events();
        assert!(app.should_quit);
    }

    #[test]
    fn local_reply_save_failure_keeps_the_input_open() {
        let mut app = create_test_app();
        app.comments = vec![directive("comment", 1)];
        app.start_reply_to_comment(0);
        app.comment_reply_input = "draft reply".into();
        std::fs::create_dir(app.git_engine.repo_path.join(".lgtm/comments.jsonl")).unwrap();
        app.handle_comment_reply_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::CommentReply);
        assert_eq!(app.comment_reply_input, "draft reply");
        assert!(app.comments[0].replies.is_empty());
    }
    #[test]
    fn single_thread_refresh_ignores_outdated_results() {
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        let mut updated = thread("first");
        updated.resolved = true;
        app.refresh_thread_job(42, "first".into(), move || Ok(Some(updated)));
        app.thread_revision += 1;
        apply_next(&mut app);
        assert!(!app.threads[0].resolved);
        let mut fresh = thread("first");
        fresh.resolved = true;
        app.refresh_thread_job(42, "first".into(), move || Ok(Some(fresh)));
        apply_next(&mut app);
        assert!(app.threads[0].resolved);
    }

    #[test]
    fn pending_pr_load_is_not_restarted_by_local_edits() {
        let mut app = create_test_app();
        let (tx, rx) = crossbeam_channel::unbounded();
        app.watcher_rx = rx;
        app.pr_loading = Some((42, 1));
        tx.send(WatcherMessage::FilesChanged(vec![app
            .git_engine
            .repo_path
            .join("a.rs")]))
            .unwrap();
        app.handle_background_events();
        assert_eq!(app.pr_loading, Some((42, 1)));
        assert_eq!(app.review_generation, 0);
    }
    #[test]
    fn ready_pr_waits_until_the_current_draft_is_closed() {
        let mut app = create_test_app();
        app.pr.current_number = Some(7);
        app.active_view = ActiveView::CommentInput;
        app.comment_input = "draft for PR seven".into();
        app.review_generation = 1;
        app.pr_loading = Some((42, 1));
        let mut snapshot = DiffSnapshot::default();
        let input = completion::Input::capture(&app, 42);
        let completion =
            completion::prepare(&app.git_engine.repo_path, 42, &mut snapshot, input, || true)
                .unwrap();
        app.ready_pr = Some(LoadedPr {
            repo: app.git_engine.repo_path.clone(),
            completion,
            pr: 42,
            generation: 1,
            snapshot: DiffSnapshot::default(),
            findings: Default::default(),
        });
        app.apply_ready_pr();
        assert_eq!(app.pr.current_number, Some(7));
        assert_eq!(app.comment_input, "draft for PR seven");
        assert!(app.ready_pr.is_some());
        app.active_view = ActiveView::Diff;
        app.apply_ready_pr();
        assert_eq!(app.pr.current_number, Some(42));
        assert!(app.ready_pr.is_none());
    }
    #[test]
    fn picker_filters_and_selection_use_background_jobs() {
        let mut app = create_test_app();
        app.execute_command(":prs closed");
        assert_eq!(app.pr.filter, crate::pr::PullRequestState::Closed);
        assert_eq!(app.active_view, ActiveView::PrPicker);
        app.handle_pr_picker_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(app.pr.filter, crate::pr::PullRequestState::All);
        app.pr.list.push(crate::pr::PullRequestSummary {
            number: 19673,
            title: "old PR".into(),
            author: crate::pr::PrAuthor {
                login: "test".into(),
            },
            head_ref_name: "deleted".into(),
            additions: 0,
            deletions: 0,
            state: "completed".into(),
        });
        app.handle_pr_picker_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.pr.current_number, None);
        assert_eq!(app.pr_loading.map(|(number, _)| number), Some(19673));
    }

    #[test]
    fn bdd_ui_009_ambiguous_reply_timeout_blocks_blind_resubmission_until_reconciled() {
        use super::super::comments::ThreadAction;
        // Given an open PR thread and a reply that times out after the provider may have accepted it.
        let mut app = create_test_app();
        app.pr.current_number = Some(42);
        app.threads = vec![thread("first")];
        let action = ThreadAction::Reply {
            target: 1,
            text: "maybe accepted".into(),
        };
        app.enqueue_thread_job(42, "first".into(), action.clone(), || {
            Err(lgtm_core::forge::failure::Failure {
                kind: lgtm_core::forge::failure::Kind::Timeout,
                exit_code: Some(28),
                http_status: None,
            }
            .into())
        });
        apply_next(&mut app); // timeout completion
        apply_next(&mut app); // automatic refresh_one_thread fails on test provider

        // Then the outcome is surfaced as uncertain and immediate retry is blocked from blind resubmission.
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("Reply outcome is uncertain"));
        assert_eq!(
            app.reply_drafts
                .get(&(42, "first".into()))
                .map(String::as_str),
            Some("maybe accepted")
        );
        app.enqueue_thread_job(42, "first".into(), action, || {
            panic!("uncertain reply must not be blindly resubmitted")
        });
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("reconcile before retrying"));

        // When thread reconciliation observes the reply on the remote thread, the draft is cleared
        // and the comment is shown without a second write.
        let mut reconciled = thread("first");
        reconciled.comments.push(lgtm_core::github::ThreadComment {
            id: 77,
            author: "reviewer".into(),
            body: "maybe accepted".into(),
        });
        app.refresh_thread_job(42, "first".into(), move || Ok(Some(reconciled)));
        apply_next(&mut app);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Reply confirmed on remote thread")
        );
        assert!(!app.reply_drafts.contains_key(&(42, "first".into())));
        assert_eq!(app.threads[0].comments.len(), 1);
        assert_eq!(app.threads[0].comments[0].body, "maybe accepted");
    }
}
