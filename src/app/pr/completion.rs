//! Background preparation of directive outcomes and target-specific marks.
use super::*;

pub(super) struct Input {
    comments: Vec<CommentAnchor>,
    revision: u64,
}
impl Input {
    pub(super) fn capture(app: &App, pr: u64) -> Self {
        let mut timing = lgtm_core::debug::Span::new("ui.pr_prepare_request");
        let comments: Vec<_> = app
            .comments
            .iter()
            .filter(|c| c.pr_number == Some(pr))
            .cloned()
            .collect();
        timing.count("directives", comments.len() as u64);
        Self {
            comments,
            revision: app.pr_completion_revision,
        }
    }
}
pub(super) struct Prepared {
    input: Input,
    pub(super) comments: Vec<CommentAnchor>,
    pub(super) marks: Vec<lgtm_core::review::HunkMark>,
    pub(super) changed: bool,
}
impl Prepared {
    pub(super) fn matches(&self, app: &App, pr: u64) -> bool {
        self.input.revision == app.pr_completion_revision
            && self
                .input
                .comments
                .iter()
                .eq(app.comments.iter().filter(|c| c.pr_number == Some(pr)))
    }
}

pub(super) fn prepare(
    repo: &std::path::Path,
    pr: u64,
    snapshot: &mut DiffSnapshot,
    input: Input,
    current: impl Fn() -> bool,
) -> anyhow::Result<Prepared> {
    let mut timing = lgtm_core::debug::Span::new("review.pr_prepare");
    timing.count("directives", input.comments.len() as u64);
    timing.count("files", snapshot.files.len() as u64);
    let mut comments = input.comments.clone();
    let turn = {
        let _timing = lgtm_core::debug::Span::new("review.pr_turn");
        std::fs::read_to_string(repo.join(".lgtm/state.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<lgtm_core::turn::TurnState>(&s).ok())
            .unwrap_or_default()
            .current_turn
    };
    let mut changed = false;
    {
        let _timing = lgtm_core::debug::Span::new("review.pr_directives");
        {
            let _timing = lgtm_core::debug::Span::new("review.pr_reanchor");
            for comment in &mut comments {
                anyhow::ensure!(current(), "superseded PR preparation");
                changed |= lgtm_core::review::reanchor_anchors(
                    std::slice::from_mut(comment),
                    &snapshot.files,
                );
            }
        }
        {
            let _timing = lgtm_core::debug::Span::new("review.pr_outcomes");
            anyhow::ensure!(current(), "superseded PR preparation");
            changed |= lgtm_core::outcome::recheck(&mut comments, &snapshot.files, turn);
        }
    }
    let state = {
        let _timing = lgtm_core::debug::Span::new("review.load_marks");
        anyhow::ensure!(current(), "superseded PR preparation");
        lgtm_core::review::ReviewStore::open_marks_only(repo, &format!("pr-{pr}"))
            .ok()
            .and_then(|s| s.load().ok())
            .unwrap_or_default()
    };
    {
        let _timing = lgtm_core::debug::Span::new("review.pr_mark_matching");
        for file in &mut snapshot.files {
            anyhow::ensure!(current(), "superseded PR preparation");
            file.is_reviewed = lgtm_core::review::file_reviewed(&state.hunks, file);
            let name = file.display_name();
            for hunk in &mut file.hunks {
                if state.folds.is_empty() {
                    hunk.folded_blocks.clear();
                } else {
                    let hash = lgtm_core::review::hunk_hash(hunk);
                    // A retry must also clear folds removed since the earlier preparation.
                    hunk.folded_blocks =
                        lgtm_core::review::load_hunk_folds(&state.folds, &name, &hash);
                }
            }
        }
    }
    Ok(Prepared {
        input,
        comments,
        marks: state.hunks,
        changed,
    })
}

impl App {
    pub(in crate::app) fn invalidate_pr_completion(&mut self) {
        self.pr_completion_revision = self.pr_completion_revision.wrapping_add(1);
    }

    pub(super) fn reprepare_pr(&mut self, mut ready: LoadedPr) {
        let input = Input::capture(self, ready.pr);
        let latest = self.pr_load_latest.clone();
        let pr = ready.pr;
        let generation = ready.generation;
        let _timing = lgtm_core::debug::Span::new("ui.pr_reprepare");
        self.status_message = Some(format!("Preparing PR #{pr} with current directives…"));
        self.spawn_silent_job(
            move || {
                ready.completion = prepare(&ready.repo, pr, &mut ready.snapshot, input, || {
                    latest.load(std::sync::atomic::Ordering::Relaxed) == generation
                })?;
                Ok::<_, anyhow::Error>(ready)
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
                        app.status_message = Some(format!(
                            "PR #{pr} preparation failed: {error}; current review kept"
                        ));
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests;
