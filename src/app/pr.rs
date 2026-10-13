use super::*;

impl App {
    pub fn open_pr_picker(&mut self) {
        match lgtm_core::forge::open(&self.git_engine.repo_path).list_prs() {
            Ok(prs) => {
                self.pr.list = prs;
                self.pr.selected_idx = 0;
                self.active_view = ActiveView::PrPicker;
            }
            Err(e) => {
                self.pr.list.clear();
                self.status_message = Some(format!("PR list failed: {}", e));
                self.active_view = ActiveView::PrPicker;
            }
        }
    }
}
