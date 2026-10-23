use super::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum LocalTarget {
    WorkingTree,
    Revision(String),
}

pub(super) struct LocalResult {
    target: LocalTarget,
    snapshot: DiffSnapshot,
    findings: lgtm_core::findings::Findings,
}

type Work = Box<dyn FnOnce(u64, Arc<AtomicU64>) -> anyhow::Result<LocalResult> + Send>;

struct Request {
    id: u64,
    generation: u64,
    reset_selection: bool,
    work: Work,
}

struct Ready {
    id: u64,
    generation: u64,
    reset_selection: bool,
    result: LocalResult,
}

#[derive(Default)]
pub(super) struct LocalRefresh {
    serial: u64,
    latest: Arc<AtomicU64>,
    in_flight: Option<u64>,
    pending: Option<Request>,
    ready: Option<Ready>,
    desired: Option<(LocalTarget, bool)>,
    audit_after: bool,
}

impl App {
    pub(super) fn cancel_initial_diff_audit(&mut self) {
        self.local_refresh.audit_after = false;
    }

    pub(super) fn local_ready_to_apply(&self) -> bool {
        self.local_refresh.ready.is_some()
            && self.active_view == ActiveView::Diff
            && !self.visual_mode
    }

    #[cfg(test)]
    pub(crate) fn local_refresh_pending(&self) -> bool {
        self.local_refresh.desired.is_some() || self.local_refresh.in_flight.is_some()
    }

    pub(super) fn local_refresh_requested(&self) -> bool {
        self.local_refresh.desired.is_some()
    }
    /// Keep the in-flight slot occupied until its callback arrives, even after cancellation.
    pub(super) fn invalidate_local_refresh(&mut self) {
        self.local_refresh.serial += 1;
        self.local_refresh
            .latest
            .store(self.local_refresh.serial, Ordering::Relaxed);
        self.local_refresh.pending = None;
        self.local_refresh.ready = None;
        self.local_refresh.desired = None;
        self.local_refresh.audit_after = false;
    }

    pub fn reload_diff(&mut self) {
        self.invalidate_view_data();
        if let Some((target, reset)) = self.local_refresh.desired.clone() {
            self.queue_local_refresh(target, reset);
        } else if let Some(pr) = self.pr_loading.map(|(pr, _)| pr).or(self.pr.current_number) {
            self.load_pr_async(pr);
        } else {
            let target = self
                .active_revision
                .clone()
                .map_or(LocalTarget::WorkingTree, LocalTarget::Revision);
            self.queue_local_refresh(target, false);
        }
    }

    pub(super) fn load_local_revision(&mut self, revision: String) {
        self.invalidate_local_refresh();
        self.invalidate_audit(false);
        self.review_generation += 1;
        self.pr_load_latest
            .store(self.review_generation, Ordering::Relaxed);
        self.pr_loading = None;
        self.ready_pr = None;
        self.queue_local_refresh(LocalTarget::Revision(revision), true);
    }

    fn queue_local_refresh(&mut self, target: LocalTarget, reset_selection: bool) {
        let repo = self.git_engine.repo_path.clone();
        let config = self.config.clone();
        let source = target.clone();
        self.queue_local_job(target, reset_selection, move |id, latest| {
            let _timing = lgtm_core::debug::Span::new("review.reload");
            let engine = GitEngine::new(&repo);
            let (target, snapshot) =
                lgtm_core::debug::measure("review.local_snapshot", || match source {
                    LocalTarget::WorkingTree => engine
                        .compute_diff_snapshot()
                        .map(|s| (LocalTarget::WorkingTree, s)),
                    LocalTarget::Revision(spec) => {
                        let expanded = engine.expand_branch_spec(&spec);
                        engine
                            .compute_revision_diff(&expanded)
                            .map(|s| (LocalTarget::Revision(expanded), s))
                    }
                })?;
            // Avoid scanning an obsolete snapshot when more workspace changes arrived.
            anyhow::ensure!(
                latest.load(Ordering::Relaxed) == id,
                "superseded workspace refresh"
            );
            let findings = scan_findings(&repo, &snapshot.files, &config);
            Ok(LocalResult {
                target,
                snapshot,
                findings,
            })
        });
    }

    fn queue_local_job(
        &mut self,
        target: LocalTarget,
        reset_selection: bool,
        work: impl FnOnce(u64, Arc<AtomicU64>) -> anyhow::Result<LocalResult> + Send + 'static,
    ) {
        let mut timing = lgtm_core::debug::Span::new("ui.local_request");
        timing.count(
            "coalesced",
            u64::from(self.local_refresh.in_flight.is_some()),
        );
        let reset_selection = reset_selection
            || self
                .local_refresh
                .desired
                .as_ref()
                .is_some_and(|(_, reset)| *reset);
        self.local_refresh.serial += 1;
        let id = self.local_refresh.serial;
        self.local_refresh.latest.store(id, Ordering::Relaxed);
        self.local_refresh.ready = None;
        self.local_refresh.desired = Some((target, reset_selection));
        self.local_refresh.pending = Some(Request {
            id,
            generation: self.review_generation,
            reset_selection,
            work: Box::new(work),
        });
        self.start_pending_local();
    }

    fn start_pending_local(&mut self) {
        if self.local_refresh.in_flight.is_some() {
            return;
        }
        let Some(request) = self.local_refresh.pending.take() else {
            return;
        };
        let Request {
            id,
            generation,
            reset_selection,
            work,
        } = request;
        self.local_refresh.in_flight = Some(id);
        let latest = self.local_refresh.latest.clone();
        self.spawn_job(
            "Refreshing diff",
            move || {
                anyhow::ensure!(
                    latest.load(Ordering::Relaxed) == id,
                    "superseded workspace refresh"
                );
                work(id, latest)
            },
            move |app, result| {
                if app.local_refresh.in_flight != Some(id) {
                    return;
                }
                app.local_refresh.in_flight = None;
                if app.local_refresh.serial == id && app.review_generation == generation {
                    match result {
                        Ok(result) => {
                            app.local_refresh.ready = Some(Ready {
                                id,
                                generation,
                                reset_selection,
                                result,
                            });
                            app.apply_ready_local();
                        }
                        Err(error) => {
                            app.local_refresh.desired = None;
                            app.local_refresh.audit_after = false;
                            app.status_message = Some(format!(
                                "Diff refresh failed: {error:#}; current review kept"
                            ));
                        }
                    }
                } else {
                    let _discarded = lgtm_core::debug::Span::new("ui.local_discard");
                }
                app.start_pending_local();
            },
        );
    }

    pub(super) fn apply_ready_local(&mut self) -> bool {
        let Some(ready) = self.local_refresh.ready.as_ref() else {
            return false;
        };
        if ready.id != self.local_refresh.serial || ready.generation != self.review_generation {
            self.local_refresh.ready = None;
            return false;
        }
        if self.active_view != ActiveView::Diff || self.visual_mode {
            self.status_message =
                Some("Diff update ready; close this view or selection to apply".into());
            return false;
        }
        let Ready {
            result,
            reset_selection,
            ..
        } = self.local_refresh.ready.take().unwrap();
        let _timing = lgtm_core::debug::Span::new("ui.local_apply");
        let selected = self
            .snapshot
            .files
            .get(self.selected_file_idx)
            .map(|f| f.new_path.clone());
        self.snapshot = result.snapshot;
        self.invalidate_diff_content();
        self.alerts = result.findings.active;
        *self.hidden.borrow_mut() = Hidden {
            suppressed: result.findings.suppressed,
            existing: result.findings.existing,
        };
        self.active_revision = match result.target {
            LocalTarget::WorkingTree => None,
            LocalTarget::Revision(spec) => Some(spec),
        };
        if self.pr.current_number.take().is_some() || reset_selection {
            self.threads.clear();
            self.thread_revision += 1;
            self.ci = None;
        }
        self.local_refresh.desired = None;
        if reset_selection {
            self.selected_file_idx = 0;
            self.cursor_line_idx = 0;
            self.diff_scroll_offset = 0;
        } else {
            self.selected_file_idx = selected
                .and_then(|path| self.snapshot.files.iter().position(|f| f.new_path == path))
                .unwrap_or(self.selected_file_idx)
                .min(self.snapshot.files.len().saturating_sub(1));
        }
        // Reconcile the current directives, never a worker's older copy of UI edits/receipts.
        let turn = self.turn_manager.load_state().current_turn;
        let mut changed = false;
        {
            let _timing = lgtm_core::debug::Span::new("review.reanchor");
            for comment in self.comments.iter_mut().filter(|c| c.pr_number.is_none()) {
                changed |= lgtm_core::review::reanchor_anchors(
                    std::slice::from_mut(comment),
                    &self.snapshot.files,
                );
            }
        }
        {
            let _timing = lgtm_core::debug::Span::new("review.outcomes");
            for comment in self.comments.iter_mut().filter(|c| c.pr_number.is_none()) {
                changed |= lgtm_core::outcome::recheck(
                    std::slice::from_mut(comment),
                    &self.snapshot.files,
                    turn,
                );
            }
        }
        let save_error = if changed {
            self.comment_store.save_comments(&self.comments).err()
        } else {
            None
        };
        {
            let _timing = lgtm_core::debug::Span::new("review.load_marks");
            self.load_marks();
        }
        let visible = self.visible_line_indices();
        self.cursor_line_idx = visible
            .iter()
            .copied()
            .find(|line| *line >= self.cursor_line_idx)
            .or_else(|| visible.last().copied())
            .unwrap_or(0);
        self.diff_scroll_offset = self
            .diff_scroll_offset
            .min(visible.last().copied().unwrap_or(0));
        self.search_matches.clear();
        self.search_match_idx = 0;
        self.update_file_picker_filter();
        self.status_message = Some(match save_error {
            Some(error) => format!("Diff updated; directive updates could not be saved: {error}"),
            None => format!(
                "Diff updated ({} files, {} alerts)",
                self.snapshot.files.len(),
                self.alerts.len()
            ),
        });
        if std::mem::take(&mut self.local_refresh.audit_after) {
            self.show_diff_audit();
        }
        true
    }

    pub(crate) fn open_initial_diff_audit(&mut self) {
        if self.local_refresh.desired.is_some() {
            self.local_refresh.audit_after = true;
        } else {
            self.trigger_diff_scan();
        }
    }
}

#[cfg(test)]
mod tests;
