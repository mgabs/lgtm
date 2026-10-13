use super::*;

impl App {
    pub fn open_pr_picker(&mut self) {
        match crate::pr::PrEngine::list_pull_requests() {
            Ok(prs) => {
                self.pr.list = prs;
                self.pr.selected_idx = 0;
                self.active_view = ActiveView::PrPicker;
            }
            Err(e) => {
                self.pr.list.clear();
                self.status_message = Some(format!("`gh` PR list failed: {}", e));
                self.active_view = ActiveView::PrPicker;
            }
        }
    }
}
