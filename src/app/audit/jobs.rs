use super::*;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Repo,
    Diff,
    Risk,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Repo => "Auditing repository",
            Self::Diff => "Scanning changes",
            Self::Risk => "Checking risks",
        }
    }
}
type Progress = Arc<dyn Fn(usize, usize) -> bool + Send + Sync>;
type Work = Box<dyn FnOnce(Progress) -> anyhow::Result<Output> + Send>;
struct Output {
    files: Vec<FileDiff>,
    findings: lgtm_core::findings::Findings,
}
struct Request {
    id: u64,
    generation: u64,
    revision: u64,
    root: PathBuf,
    kind: Kind,
    work: Work,
}

#[derive(Default)]
pub(in crate::app) struct AuditJobs {
    serial: u64,
    latest: Arc<AtomicU64>,
    in_flight: Option<u64>,
    pending: Option<Request>,
    desired: Option<Kind>,
    completed: usize,
    exports: usize,
}

impl App {
    #[cfg(test)]
    pub(crate) fn audit_pending(&self) -> bool {
        self.audit_jobs.in_flight.is_some()
            || self.audit_jobs.pending.is_some()
            || self.audit_jobs.exports > 0
    }

    pub(in crate::app) fn invalidate_audit(&mut self, diff_only: bool) {
        if self.audit_jobs.desired.is_none()
            || (diff_only && self.audit_jobs.desired == Some(Kind::Repo))
        {
            return;
        }
        self.audit_jobs.serial += 1;
        self.audit_jobs
            .latest
            .store(self.audit_jobs.serial, Ordering::Relaxed);
        self.audit_jobs.pending = None;
        self.audit_jobs.desired = None;
        self.audit.progress = None;
        self.status_message =
            Some("Audit superseded; previous results kept. Run :audit or :scan again.".into());
    }

    pub(in crate::app) fn audit_exports_pending(&self) -> bool {
        self.audit_jobs.exports > 0
    }

    pub fn trigger_repo_scan(&mut self) {
        let root = self.audit.root_dir.clone();
        let config = self.config.clone();
        self.queue_audit(Kind::Repo, move |progress| {
            let _timing = lgtm_core::debug::Span::new("audit.repository");
            let (files, findings) =
                crate::scan::FullRepoScanner::scan_with_progress(&root, &config, &*progress)?;
            Ok(Output { files, findings })
        });
    }

    pub fn trigger_diff_scan(&mut self) {
        self.queue_diff_audit(Kind::Diff);
    }
    pub fn trigger_risk_scan(&mut self) {
        self.queue_diff_audit(Kind::Risk);
    }

    fn queue_diff_audit(&mut self, kind: Kind) {
        let files = {
            let mut timing = lgtm_core::debug::Span::new("ui.audit_snapshot");
            timing.count("files", self.snapshot.files.len() as u64);
            self.snapshot.files.clone()
        };
        let repo = self.git_engine.repo_path.clone();
        let config = self.config.clone();
        self.queue_audit(kind, move |progress| {
            let _timing = lgtm_core::debug::Span::new("audit.changes");
            anyhow::ensure!(progress(0, files.len()), "audit cancelled");
            let findings = scan_findings(&repo, &files, &config);
            anyhow::ensure!(progress(files.len(), files.len()), "audit cancelled");
            Ok(Output { files, findings })
        });
    }

    fn queue_audit(
        &mut self,
        kind: Kind,
        work: impl FnOnce(Progress) -> anyhow::Result<Output> + Send + 'static,
    ) {
        self.cancel_initial_diff_audit();
        let mut timing = lgtm_core::debug::Span::new("ui.audit_request");
        timing.count("coalesced", u64::from(self.audit_jobs.in_flight.is_some()));
        self.audit_jobs.serial += 1;
        let id = self.audit_jobs.serial;
        self.audit_jobs.latest.store(id, Ordering::Relaxed);
        self.audit_jobs.desired = Some(kind);
        self.audit_jobs.completed = 0;
        self.audit_jobs.pending = Some(Request {
            id,
            generation: self.review_generation,
            revision: self.diff_revision,
            root: self.audit.root_dir.clone(),
            kind,
            work: Box::new(work),
        });
        self.audit.progress = Some(if self.audit_jobs.in_flight.is_some() {
            "Audit queued; finishing previous scan".into()
        } else {
            kind.label().into()
        });
        self.status_message = self.audit.progress.clone();
        self.active_view = if kind == Kind::Risk {
            ActiveView::Diff
        } else {
            ActiveView::AuditReport
        };
        self.start_pending_audit();
    }

    fn start_pending_audit(&mut self) {
        if self.audit_jobs.in_flight.is_some() {
            return;
        }
        let Some(request) = self.audit_jobs.pending.take() else {
            return;
        };
        let Request {
            id,
            generation,
            revision,
            root,
            kind,
            work,
        } = request;
        self.audit_jobs.in_flight = Some(id);
        self.audit.progress = Some(kind.label().into());
        let latest = self.audit_jobs.latest.clone();
        let reported = AtomicUsize::new(usize::MAX);
        let tx = self.jobs_tx.clone();
        let progress: Progress = Arc::new(move |done, total| {
            if latest.load(Ordering::Relaxed) != id {
                return false;
            }
            // At most about twenty updates per scan, monotonic even with parallel workers.
            let mut old = reported.load(Ordering::Relaxed);
            loop {
                if old != usize::MAX
                    && (done <= old || (done < total && done - old < total.div_ceil(20).max(1)))
                {
                    return true;
                }
                match reported.compare_exchange_weak(
                    old,
                    done,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(value) => old = value,
                }
            }
            let _ = tx.send(Box::new(move |app| {
                if app.audit_jobs.serial == id && app.audit_jobs.desired.is_some() {
                    app.audit_jobs.completed = app.audit_jobs.completed.max(done);
                    let mut timing = lgtm_core::debug::Span::new("ui.audit_progress");
                    timing.count("completed", app.audit_jobs.completed as u64);
                    timing.count("files", total as u64);
                    app.audit.progress = Some(format!(
                        "{}: {}/{} files",
                        kind.label(),
                        app.audit_jobs.completed,
                        total
                    ));
                }
            }));
            true
        });
        let run_latest = self.audit_jobs.latest.clone();
        self.spawn_job(
            kind.label(),
            move || {
                anyhow::ensure!(run_latest.load(Ordering::Relaxed) == id, "audit superseded");
                work(progress)
            },
            move |app, result| {
                if app.audit_jobs.in_flight != Some(id) {
                    return;
                }
                app.audit_jobs.in_flight = None;
                let current = app.audit_jobs.serial == id
                    && app.review_generation == generation
                    && app.audit.root_dir == root
                    && (kind == Kind::Repo || app.diff_revision == revision);
                if current {
                    app.audit_jobs.desired = None;
                    app.audit.progress = None;
                    match result {
                        Ok(output) => app.apply_audit(kind, output),
                        Err(error) => {
                            app.status_message =
                                Some(format!("Audit scan failed: {error}; previous results kept"))
                        }
                    }
                } else {
                    let _discarded = lgtm_core::debug::Span::new("ui.audit_discard");
                    if app.audit_jobs.serial == id {
                        app.audit_jobs.desired = None;
                        app.audit.progress = None;
                        app.status_message =
                            Some("Audit target changed; previous results kept".into());
                    }
                }
                app.start_pending_audit();
            },
        );
    }

    fn apply_audit(&mut self, kind: Kind, output: Output) {
        self.invalidate_view_data();
        let mut timing = lgtm_core::debug::Span::new("ui.audit_apply");
        timing.count("files", output.files.len() as u64);
        timing.count("alerts", output.findings.active.len() as u64);
        if kind != Kind::Repo {
            self.alerts = output.findings.active.clone();
            *self.hidden.borrow_mut() = Hidden {
                suppressed: output.findings.suppressed,
                existing: output.findings.existing,
            };
        }
        if kind != Kind::Risk {
            let selected = self
                .filtered_audit_alerts()
                .get(self.audit.selected_idx)
                .map(|a| (a.fingerprint.clone(), a.file_path.clone(), a.line_no));
            self.audit.files = output.files;
            self.audit.alerts = output.findings.active;
            self.audit.has_result = true;
            let filtered = self.filtered_audit_alerts();
            self.audit.selected_idx = selected
                .and_then(|selected| {
                    filtered.iter().position(|a| {
                        (a.fingerprint.clone(), a.file_path.clone(), a.line_no) == selected
                    })
                })
                .unwrap_or_else(|| {
                    self.audit
                        .selected_idx
                        .min(filtered.len().saturating_sub(1))
                });
            self.audit.inspector_scroll = 0;
            if self.active_view == ActiveView::AuditReport {
                self.sync_audit_selected_file();
            }
        }
        self.status_message = Some(format!(
            "{} complete: {} issues found",
            kind.label(),
            if kind == Kind::Risk {
                self.alerts.len()
            } else {
                self.audit.alerts.len()
            }
        ));
    }

    /// Enqueues export and returns its destination; completion is reported in the status bar.
    pub fn export_audit_report(
        &mut self,
        target_path: Option<&std::path::Path>,
    ) -> anyhow::Result<PathBuf> {
        let path = target_path.map_or_else(
            || self.audit.root_dir.join("lgtm-report.md"),
            |p| p.to_path_buf(),
        );
        let destination = path.clone();
        let returned = path.clone();
        let root = self.audit.root_dir.clone();
        let config = self.config.clone();
        let visible =
            self.audit.has_result || !self.audit.files.is_empty() || !self.audit.alerts.is_empty();
        let files = self.audit.files.clone();
        let alerts = self.audit.alerts.clone();
        self.audit_jobs.exports += 1;
        self.spawn_job(
            "Exporting audit report",
            move || -> anyhow::Result<()> {
                let _timing = lgtm_core::debug::Span::new("audit.export");
                let (files, alerts) = if visible {
                    (files, alerts)
                } else {
                    crate::scan::FullRepoScanner::scan_repository(&root, &config)?
                };
                let content =
                    crate::scan::FullRepoScanner::generate_markdown_report(&root, &files, &alerts);
                std::fs::write(&destination, content)?;
                Ok(())
            },
            move |app, result| {
                app.audit_jobs.exports -= 1;
                app.status_message = Some(match result {
                    Ok(()) => format!("Audit report exported to: {}", path.display()),
                    Err(error) => {
                        app.quit_after_jobs = false;
                        format!("Audit report export failed: {error}")
                    }
                });
            },
        );
        Ok(returned)
    }
}

#[cfg(test)]
mod tests;
