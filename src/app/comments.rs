use super::*;

impl App {
    pub fn current_line_number(&self) -> Option<u32> {
        let file = self.snapshot.files.get(self.selected_file_idx)?;
        let mut line_counter = 0;
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line_counter == self.cursor_line_idx {
                    return line.new_line_num.or(line.old_line_num);
                }
                line_counter += 1;
            }
        }
        None
    }

    pub fn open_threads_modal(&mut self) {
        if self.pr.current_number.is_none() {
            self.status_message = Some("Open a PR first (:pr <n>)".to_string());
            return;
        }
        if self.threads.is_empty() {
            self.active_view = ActiveView::ThreadView;
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            if let Some(lno) = self.current_line_number() {
                if let Some(pos) = self
                    .threads
                    .iter()
                    .position(|t| t.path == path && t.line == Some(lno))
                {
                    self.thread_selected_idx = pos;
                }
            }
        }
        self.thread_selected_idx = self
            .thread_selected_idx
            .min(self.threads.len().saturating_sub(1));
        self.thread_scroll_offset = 0;
        self.active_view = ActiveView::ThreadView;
    }

    pub fn open_comment_list(&mut self) {
        self.comments = self.comment_store.load_comments();
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            if let Some(lno) = self.current_line_number() {
                if let Some(pos) = self.comments.iter().position(|c| {
                    (c.file_path == path_str
                        || c.file_path.ends_with(&path_str)
                        || path_str.ends_with(&c.file_path))
                        && c.target_line_no == lno
                }) {
                    self.comment_list_selected_idx = pos;
                } else {
                    self.comment_list_selected_idx = 0;
                }
            } else {
                self.comment_list_selected_idx = 0;
            }
        } else {
            self.comment_list_selected_idx = 0;
        }
        self.active_view = ActiveView::CommentList;
    }

    pub fn has_comment_on_cursor(&self) -> bool {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        if let Some(lno) = line.new_line_num.or(line.old_line_num) {
                            return self.comments.iter().any(|c| {
                                !c.orphaned && c.file_path == path_str && c.target_line_no == lno
                            });
                        }
                    }
                    line_counter += 1;
                }
            }
        }
        false
    }

    pub fn delete_current_line_comment(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            let mut target_line_no = None;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line_no = line.new_line_num.or(line.old_line_num);
                        break;
                    }
                    line_counter += 1;
                }
                if target_line_no.is_some() {
                    break;
                }
            }

            if let Some(lno) = target_line_no {
                let initial_len = self.comments.len();
                self.comments
                    .retain(|c| !(c.file_path == path_str && c.target_line_no == lno));
                if self.comments.len() < initial_len {
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message = Some(format!("Deleted comment on {}:{}", path_str, lno));
                } else {
                    self.status_message = Some(format!("No comment on {}:{}", path_str, lno));
                }
            }
        }
    }

    pub fn comment_idx_at_cursor(&self) -> Option<usize> {
        let file = self.snapshot.files.get(self.selected_file_idx)?;
        let path_str = file.display_name();
        let mut line_counter = 0;
        let mut target_line_no = None;
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line_counter == self.cursor_line_idx {
                    target_line_no = line.new_line_num.or(line.old_line_num);
                    break;
                }
                line_counter += 1;
            }
            if target_line_no.is_some() {
                break;
            }
        }
        let lno = target_line_no?;
        self.comments
            .iter()
            .position(|c| c.file_path == path_str && c.target_line_no == lno)
    }

    pub fn toggle_resolve_current_comment(&mut self) {
        if let Some(idx) = self.comment_idx_at_cursor() {
            let is_res = self.comments[idx].toggle_resolved();
            let _ = self.comment_store.save_comments(&self.comments);
            self.status_message = Some(if is_res {
                "✅ Directive marked resolved".to_string()
            } else {
                "💬 Directive marked active".to_string()
            });
        } else {
            self.status_message = Some("No directive on current line".to_string());
        }
    }

    pub fn start_reply_to_comment(&mut self, idx: usize) {
        if idx < self.comments.len() {
            self.comment_reply_target_idx = Some(idx);
            self.comment_reply_input.clear();
            self.active_view = ActiveView::CommentReply;
        }
    }

    pub fn action_comment_to_agent(&mut self, idx: usize) {
        if let Some(comment) = self.comments.get(idx) {
            let mut payload = format!(
                "#ref {}:{} DIRECTIVE: {}",
                comment.file_path, comment.target_line_no, comment.comment_text
            );
            for reply in &comment.replies {
                payload.push_str(&format!("\n↳ Reply: {}", reply));
            }
            let sink = self.agent_sink();
            let result = sink.send(&payload);
            let label = sink.label();
            drop(sink);
            self.status_message = Some(match result {
                Ok(()) => format!(
                    "Sent to {}: {}:{}",
                    label, comment.file_path, comment.target_line_no
                ),
                Err(e) => {
                    let _ = crate::ipc::sink::ClipboardSink.send(&payload);
                    format!("Send failed: {}", e)
                }
            });
        }
    }

    /// Synchronous variant of `refresh_github_async`, used with fake clients in tests.
    #[cfg(test)]
    pub fn refresh_github(&mut self, client: &dyn lgtm_core::github::GithubClient, threads: bool) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Open a PR first (:pr <n>)".to_string());
            return;
        };
        if threads {
            self.apply_threads(client.threads(pr));
        } else {
            self.apply_checks(client.checks(pr));
        }
    }

    /// Same as `refresh_github` but off the UI thread, for providers that shell out.
    pub fn refresh_github_async(&mut self, threads: bool) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Open a PR first (:pr <n>)".to_string());
            return;
        };
        let repo = self.git_engine.repo_path.clone();
        if threads {
            self.spawn_job(
                "Loading threads",
                move || lgtm_core::forge::open(&repo).threads(pr),
                |app, r| app.apply_threads(r),
            );
        } else {
            self.spawn_job(
                "Loading checks",
                move || lgtm_core::forge::open(&repo).checks(pr),
                |app, r| app.apply_checks(r),
            );
        }
    }

    fn apply_threads(&mut self, res: anyhow::Result<Vec<lgtm_core::github::Thread>>) {
        self.status_message = Some(match res {
            Ok(t) => {
                let open: Vec<String> = t
                    .iter()
                    .filter(|t| !t.resolved)
                    .take(5)
                    .map(|t| format!("{}:{}", t.path, t.line.unwrap_or(0)))
                    .collect();
                let msg = format!(
                    "{} threads, {} open: {}",
                    t.len(),
                    open.len(),
                    open.join(", ")
                );
                self.threads = t;
                msg
            }
            Err(e) => e.to_string(),
        });
    }

    fn apply_checks(&mut self, res: anyhow::Result<Vec<lgtm_core::github::Check>>) {
        self.status_message = Some(match res {
            Ok(c) => {
                let s = lgtm_core::github::CiSummary::of(&c);
                let msg = format!(
                    "CI: {} passed, {} failed, {} pending",
                    s.passed, s.failed, s.pending
                );
                self.ci = Some(s);
                msg
            }
            Err(e) => e.to_string(),
        });
    }

    /// `:pr post`: preview unless `confirmed` (`--yes`); nothing is sent without it.
    pub fn post_pr_comments(
        &mut self,
        client: &dyn lgtm_core::github::GithubClient,
        confirmed: bool,
    ) {
        use lgtm_core::github::post;
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Open a PR first (:pr <n>)".to_string());
            return;
        };
        let plan = post::plan(&self.comments, &self.snapshot.files, pr);
        if plan.total() == 0 {
            self.status_message = Some("No unposted comments for this PR".to_string());
            return;
        }
        if !confirmed {
            self.status_message = Some(format!(
                "Would post {} inline + {} general comment(s) to PR #{}; run `:pr post --yes` to send",
                plan.inline.len(),
                plan.general.len(),
                pr
            ));
            return;
        }
        let store = &self.comment_store;
        let report = post::post(
            client,
            pr,
            &mut self.comments,
            &plan,
            std::time::Duration::from_millis(500),
            |c| store.save_comments(c),
        );
        self.status_message = Some(report.summary());
    }

    /// `:thread reply <n> <text> --yes` / `:thread resolve|unresolve <n> --yes`; `n` is the 1-based
    /// position in `:threads`. Without `--yes` it only previews.
    pub fn thread_action(&mut self, client: &dyn lgtm_core::github::GithubClient, args: &[&str]) {
        let Some(pr) = self.pr.current_number else {
            self.status_message = Some("Open a PR first (:pr <n>)".to_string());
            return;
        };
        let confirmed = args.last() == Some(&"--yes");
        let args = if confirmed {
            &args[..args.len() - 1]
        } else {
            args
        };
        let (Some(&verb), Some(n)) = (
            args.first(),
            args.get(1).and_then(|n| n.parse::<usize>().ok()),
        ) else {
            self.status_message = Some(
                "Usage: :thread reply <n> <text> | resolve <n> | unresolve <n> (add --yes to send)"
                    .to_string(),
            );
            return;
        };
        let Some(t) = n.checked_sub(1).and_then(|i| self.threads.get(i)) else {
            self.status_message = Some("No such thread; run :threads first".to_string());
            return;
        };
        let text = args[2..].join(" ");
        let (id, target) = (t.id.clone(), t.comments.first().map_or(0, |c| c.id));
        let preview = match verb {
            "reply" if text.is_empty() => {
                self.status_message = Some("Reply text is empty".to_string());
                return;
            }
            "reply" => format!("Would reply on {}:{}", t.path, t.line.unwrap_or(0)),
            "resolve" | "unresolve" => format!(
                "Would {} thread on {}:{}",
                verb,
                t.path,
                t.line.unwrap_or(0)
            ),
            _ => {
                self.status_message = Some(format!("Unknown thread action '{verb}'"));
                return;
            }
        };
        if !confirmed {
            self.status_message = Some(format!("{preview}; add --yes to send"));
            return;
        }
        let res = match verb {
            "reply" => client.reply(pr, &id, target, &text).map(|_| "Reply posted"),
            _ => client
                .set_resolved(pr, &id, verb == "resolve")
                .map(|_| "Thread updated"),
        };
        self.status_message = Some(match res {
            Ok(done) => {
                if verb != "reply" {
                    self.threads[n - 1].resolved = verb == "resolve";
                }
                done.to_string()
            }
            Err(e) => e.to_string(),
        });
    }

    /// `:directives orphans` lists them, `orphans repin <n>` moves orphan `n` to the cursor line,
    /// `orphans drop [--yes]` deletes them all (preview without `--yes`).
    pub fn orphans_command(&mut self, args: &[&str]) {
        let idx: Vec<usize> = (0..self.comments.len())
            .filter(|&i| self.comments[i].orphaned)
            .collect();
        let msg = match (args.first().copied(), args.get(1)) {
            _ if idx.is_empty() => "No orphaned comments".to_string(),
            (None, _) => {
                let list: Vec<String> = idx
                    .iter()
                    .take(5)
                    .enumerate()
                    .map(|(n, &i)| {
                        let c = &self.comments[i];
                        format!("{}) {}:{}", n + 1, c.file_path, c.target_line_no)
                    })
                    .collect();
                format!("{} orphaned: {}", idx.len(), list.join(", "))
            }
            (Some("drop"), Some(&"--yes")) => {
                self.comments.retain(|c| !c.orphaned);
                let _ = self.comment_store.save_comments(&self.comments);
                format!("Deleted {} orphaned comment(s)", idx.len())
            }
            (Some("drop"), _) => format!(
                "Would delete {} orphaned comment(s); add --yes to confirm",
                idx.len()
            ),
            (Some("repin"), n) => {
                let Some(&i) = n
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|n| idx.get(n))
                else {
                    self.status_message = Some("Usage: :directives orphans repin <n>".into());
                    return;
                };
                let Some(file) = self.snapshot.files.get(self.selected_file_idx) else {
                    return;
                };
                let path = file.display_name();
                let Some(line) = file
                    .hunks
                    .iter()
                    .flat_map(|h| &h.lines)
                    .nth(self.cursor_line_idx)
                else {
                    return;
                };
                let (no, code) = (
                    line.new_line_num.or(line.old_line_num).unwrap_or(1),
                    line.content.clone(),
                );
                let hash = lgtm_core::review::hunk_hash_at(&self.snapshot.files, &path, no);
                let c = &mut self.comments[i];
                (c.file_path, c.target_line_no, c.target_code, c.orphaned) =
                    (path.clone(), no, code, false);
                c.track.hunk_hash = hash;
                let _ = self.comment_store.save_comments(&self.comments);
                format!("Re-pinned to {path}:{no}")
            }
            _ => "Usage: :directives orphans [repin <n> | drop [--yes]]".to_string(),
        };
        self.status_message = Some(msg);
    }

    /// `:directives recheck`: reclassify open directives against the current diff and report counts.
    pub fn recheck_directives(&mut self) {
        let turn = self.turn_manager.load_state().current_turn;
        if lgtm_core::outcome::recheck(&mut self.comments, &self.snapshot.files, turn) {
            let _ = self.comment_store.save_comments(&self.comments);
        }
        self.status_message = Some(format!(
            "Directives: {}",
            lgtm_core::outcome::Summary::of(&self.comments).line()
        ));
    }

    pub fn review_target(&self) -> String {
        match (self.pr.current_number, self.active_revision.as_deref()) {
            (Some(n), _) => format!("pr-{n}"),
            (None, Some(rev)) => rev.to_string(),
            _ => "working".to_string(),
        }
    }

    fn marks_store(&self) -> Option<lgtm_core::review::ReviewStore> {
        lgtm_core::review::ReviewStore::open_marks_only(
            &self.git_engine.repo_path,
            &self.review_target(),
        )
        .ok()
    }

    /// Reload marks for the current target and refresh each file's reviewed flag.
    pub fn load_marks(&mut self) {
        self.marks = self
            .marks_store()
            .and_then(|s| s.load().ok())
            .map(|st| st.hunks)
            .unwrap_or_default();
        for f in &mut self.snapshot.files {
            f.is_reviewed = lgtm_core::review::file_reviewed(&self.marks, f);
        }
    }

    fn save_marks(&mut self) {
        let Some(store) = self.marks_store() else {
            return;
        };
        let mut st = store.load().unwrap_or_default();
        st.hunks = self.marks.clone();
        if store.save(&st).is_err() {
            self.status_message = Some("Failed to save review marks".to_string());
        }
    }

    pub fn toggle_review_current_file(&mut self) {
        use lgtm_core::review::{file_reviewed, set_file};
        let now = crate::turn::chrono_iso_now();
        let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) else {
            return;
        };
        if file.hunks.is_empty() {
            file.is_reviewed = !file.is_reviewed;
        } else {
            let on = !file_reviewed(&self.marks, file);
            set_file(&mut self.marks, file, on, &now);
            file.is_reviewed = on;
            self.save_marks();
        }
        if self.snapshot.files[self.selected_file_idx].is_reviewed {
            self.next_file();
        }
    }

    pub fn toggle_review_current_hunk(&mut self) {
        let now = crate::turn::chrono_iso_now();
        let Some((_, _, h_idx)) = self.current_hunk_info() else {
            return;
        };
        let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) else {
            return;
        };
        let on = lgtm_core::review::toggle_hunk(&mut self.marks, file, h_idx, &now);
        file.is_reviewed = lgtm_core::review::file_reviewed(&self.marks, file);
        self.status_message = Some(format!("Hunk {}", if on { "reviewed" } else { "unmarked" }));
        self.save_marks();
    }

    pub fn save_current_line_comment(&mut self) {
        if let Some(alert) = self.comment_target_alert.take() {
            let target_line_no = alert.line_no.unwrap_or(1);
            let path_str = alert.file_path;
            let target_code = format!("[{}] {}", alert.severity.badge(), alert.title);
            let turn_state = self.turn_manager.load_state();
            let anchor = CommentAnchor {
                file_path: path_str.clone(),
                target_line_no,
                target_code,
                context_before: Vec::new(),
                context_after: Vec::new(),
                comment_text: self.comment_input.clone(),
                replies: Vec::new(),
                created_at_turn: turn_state.current_turn,
                is_resolved: false,
                pr_number: self.pr.current_number,
                remote_id: None,
                orphaned: false,
                track: lgtm_core::outcome::Tracking {
                    hunk_hash: lgtm_core::review::hunk_hash_at(
                        &self.snapshot.files,
                        &path_str,
                        target_line_no,
                    ),
                    ..Default::default()
                },
            };

            if let Err(e) = self.comment_store.add_comment(&anchor) {
                self.status_message = Some(format!("Failed to save comment: {}", e));
                return;
            }
            self.comments.push(anchor);
            if let Some(pr_num) = self.pr.current_number {
                self.status_message =
                    Some(format!("Comment on finding added to PR #{} review", pr_num));
            } else {
                self.status_message =
                    Some("Comment on finding added to .lgtm/comments.jsonl".to_string());
            }
            return;
        }

        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut target_line_no = 1;
            let mut target_code = String::new();
            let mut line_counter = 0;

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line_no = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        target_code = line.content.clone();
                        break;
                    }
                    line_counter += 1;
                }
            }

            let turn_state = self.turn_manager.load_state();
            let anchor = CommentAnchor {
                file_path: path_str.clone(),
                target_line_no,
                target_code,
                context_before: Vec::new(),
                context_after: Vec::new(),
                comment_text: self.comment_input.clone(),
                replies: Vec::new(),
                created_at_turn: turn_state.current_turn,
                is_resolved: false,
                pr_number: self.pr.current_number,
                remote_id: None,
                orphaned: false,
                track: lgtm_core::outcome::Tracking {
                    hunk_hash: lgtm_core::review::hunk_hash_at(
                        &self.snapshot.files,
                        &path_str,
                        target_line_no,
                    ),
                    ..Default::default()
                },
            };

            if let Err(e) = self.comment_store.add_comment(&anchor) {
                self.status_message = Some(format!("Failed to save comment: {}", e));
            } else {
                self.comments.push(anchor);
                if let Some(pr_num) = self.pr.current_number {
                    self.status_message = Some(format!("Comment added to PR #{} review", pr_num));
                } else {
                    self.status_message = Some("Comment added to .lgtm/comments.jsonl".to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::app::test_util::create_test_app;
    use crate::model::*;
    use crossbeam_channel::unbounded;
    use std::path::PathBuf;

    #[test]
    fn test_comment_add_and_remove_and_pr_review() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![DiffLine {
            line_type: LineType::Addition,
            content: "    let secret = 123;".into(),
            old_line_num: None,
            new_line_num: Some(1),
        }];
        let hunk = DiffHunk {
            header: "@@ -0,0 +1,1 @@".into(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 1,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("auth.rs"),
            change_type: ChangeType::Added,
            additions: 1,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);
        app.pr.current_number = Some(99);

        app.cursor_line_idx = 0;
        app.comment_input = "Do not hardcode secret".into();
        app.save_current_line_comment();

        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].pr_number, Some(99));

        let (prompt, file_path) = CommentStore::compile_pr_review_prompt(&app.comments, 99);
        assert!(prompt.contains("PR #99"));
        assert!(prompt.contains("Do not hardcode secret"));
        assert_eq!(file_path, PathBuf::from(".lgtm/pr-99-review.md"));

        // Test delete comment
        app.delete_current_line_comment();
        assert_eq!(app.comments.len(), 0);
    }

    #[test]
    fn test_comment_actions_and_jump() {
        let mut app = create_test_app();

        let hunk = DiffHunk {
            header: "@@ -1,15 +1,15 @@".into(),
            old_start: 1,
            old_lines: 15,
            new_start: 1,
            new_lines: 15,
            lines: (1..=15)
                .map(|i| DiffLine {
                    line_type: LineType::Context,
                    content: format!("line {}", i),
                    old_line_num: Some(i),
                    new_line_num: Some(i),
                })
                .collect(),
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/lib.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: true,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);

        let anchor = CommentAnchor {
            file_path: "src/lib.rs".into(),
            target_line_no: 12,
            target_code: "line 12".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Need refactoring here".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: Default::default(),
        };
        app.comments.push(anchor);

        // Test jump unfolds file, sets cursor and diff_scroll_offset
        app.jump_to_file_and_line("src/lib.rs", 12);
        assert!(!app.snapshot.files[0].is_folded);
        assert_eq!(app.cursor_line_idx, 11);
        assert_eq!(app.diff_scroll_offset, 1);

        // Test cursor comment detection
        assert_eq!(app.comment_idx_at_cursor(), Some(0));

        // Test toggle resolve
        app.toggle_resolve_current_comment();
        assert!(app.comments[0].is_resolved);
        app.toggle_resolve_current_comment();
        assert!(!app.comments[0].is_resolved);

        // Test start reply and submit reply
        app.start_reply_to_comment(0);
        assert_eq!(app.active_view, ActiveView::CommentReply);
        assert_eq!(app.comment_reply_target_idx, Some(0));

        app.comment_reply_input = "Will fix in next commit".into();
        app.handle_comment_reply_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.comments[0].replies.len(), 1);
        assert_eq!(app.comments[0].replies[0], "Will fix in next commit");

        // Test CommentList actions (resolve toggle, delete)
        app.active_view = ActiveView::CommentList;
        app.comment_list_selected_idx = 0;

        // Toggle resolve via 'v'
        app.handle_comment_list_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(app.comments[0].is_resolved);

        // Test ex command :unresolve and :resolve
        app.execute_command(":unresolve");
        assert!(!app.comments[0].is_resolved);
        app.execute_command(":resolve all");
        assert!(app.comments[0].is_resolved);

        // Test :clear-resolved
        app.execute_command(":clear-resolved");
        assert!(app.comments.is_empty());

        // Test open_comment_list and Enter key navigation
        let anchor2 = CommentAnchor {
            file_path: "src/lib.rs".into(),
            target_line_no: 5,
            target_code: "line 5".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Line 5 comment".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: Default::default(),
        };
        let _ = app.comment_store.add_comment(&anchor2);
        app.cursor_line_idx = 4;
        app.open_comment_list();
        assert_eq!(app.active_view, ActiveView::CommentList);
        assert_eq!(app.comment_list_selected_idx, 0);

        // Press Enter to jump
        app.handle_comment_list_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.cursor_line_idx, 4);
    }

    #[test]
    fn test_comment_on_audit_finding() {
        let mut app = create_test_app();
        let alert = crate::guard::GuardAlert {
            rule_id: "sec.entropy".into(),
            fingerprint: "abc".into(),
            severity: crate::guard::Severity::Critical,
            title: "High-Entropy Secret".into(),
            description: "Found hardcoded token".into(),
            line_no: Some(42),
            file_path: "src/secret.rs".into(),
        };

        app.audit.alerts = vec![alert.clone()];
        app.audit.selected_idx = 0;
        app.active_view = ActiveView::AuditReport;

        // Press 'c' to comment on finding
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::CommentInput);
        assert_eq!(app.comment_source_view, ActiveView::AuditReport);
        assert_eq!(app.comment_target_alert.as_ref(), Some(&alert));

        // Type directive and press Enter
        app.comment_input = "Rotate and move to environment variable".into();
        app.handle_comment_input_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // Should return to AuditReport view
        assert_eq!(app.active_view, ActiveView::AuditReport);
        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].file_path, "src/secret.rs");
        assert_eq!(app.comments[0].target_line_no, 42);
        assert_eq!(
            app.comments[0].comment_text,
            "Rotate and move to environment variable"
        );
        assert!(app.comments[0].target_code.contains("High-Entropy Secret"));
    }
}
