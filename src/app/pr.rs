use super::*;

impl App {
    /// Runs slow `work` on a thread and applies its result on the UI thread in a later tick.
    pub(crate) fn spawn_job<T: Send + 'static>(
        &mut self,
        label: &str,
        work: impl FnOnce() -> T + Send + 'static,
        done: impl FnOnce(&mut App, T) + Send + 'static,
    ) {
        self.status_message = Some(format!("{label}..."));
        let tx = self.jobs_tx.clone();
        std::thread::spawn(move || {
            let out = work();
            let _ = tx.send(Box::new(move |app: &mut App| done(app, out)));
        });
    }

    pub fn open_pr_picker(&mut self) {
        self.pr.list.clear();
        self.pr.selected_idx = 0;
        self.active_view = ActiveView::PrPicker;
        let repo = self.git_engine.repo_path.clone();
        self.spawn_job(
            "Loading PRs",
            move || lgtm_core::forge::open(&repo).list_prs(),
            |app, res| match res {
                Ok(prs) => {
                    app.status_message = Some(format!("{} open PRs", prs.len()));
                    app.pr.list = prs;
                }
                Err(e) => app.status_message = Some(format!("PR list failed: {}", e)),
            },
        );
    }
}
