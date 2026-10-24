use super::*;
pub(super) fn read_gate(repo: &std::path::Path) -> Option<(String, String)> {
    let gate_file = repo.join(".lgtm/gate.json");
    if gate_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&gate_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let status = val
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("waiting")
                    .to_string();
                let msg = val
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Agent awaiting approval")
                    .to_string();
                return Some((status, msg));
            }
        }
    }
    None
}

pub(super) fn read_prompt(repo: &std::path::Path) -> (String, String) {
    // 1. Check .lgtm/gate.json
    if let Some((_, msg)) = read_gate(repo) {
        if !msg.is_empty() && msg != "Agent awaiting approval" {
            return ("Gate Context".to_string(), msg);
        }
    }

    // 2. Check .lgtm/prompt.txt or .lgtm/last-prompt.md
    for name in &["prompt.txt", "last-prompt.md", "prompt.md"] {
        let p = repo.join(".lgtm").join(name);
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return (format!(".lgtm/{}", name), trimmed.to_string());
                }
            }
        }
    }

    // 3. Check git HEAD commit message
    if let Some(msg) = GitEngine::new(repo).last_commit_message() {
        if !msg.is_empty() {
            return ("Git HEAD Commit".to_string(), msg);
        }
    }

    (
            "No Prompt Found".to_string(),
            "No prompt context found.\n\nTo provide agent prompt context:\n- Pass message with `lgtm gate -m '<prompt>'`\n- Write prompt to `.lgtm/prompt.txt`\n- Or commit with a descriptive message.".to_string(),
        )
}

#[derive(Clone, PartialEq, Eq)]
struct Base {
    repo: PathBuf,
    epoch: u64,
}
#[derive(Clone, PartialEq, Eq)]
struct SnippetKey {
    root: PathBuf,
    file: String,
    line: u32,
    epoch: u64,
}
type Snippet = Option<Vec<(usize, String)>>;

pub(crate) struct Prepared {
    pub(crate) is_git: bool,
    pub(crate) turn: u64,
    pub(crate) gate: Option<(String, String)>,
    pub(crate) staged_count: usize,
    epoch: u64,
    metadata: Option<Base>,
    prompt: Option<(Base, (String, String))>,
    staged: Option<Base>,
    snippets: std::collections::VecDeque<(SnippetKey, Snippet)>,
    in_flight: bool,
    pub(crate) requests_started: u64,
}
impl Prepared {
    pub(super) fn new(is_git: bool) -> Self {
        Self {
            is_git,
            turn: 1,
            gate: None,
            staged_count: 0,
            epoch: 0,
            metadata: None,
            prompt: None,
            staged: None,
            snippets: Default::default(),
            in_flight: false,
            requests_started: 0,
        }
    }
    pub(crate) fn prompt_for(&self, repo: &std::path::Path) -> (&str, &str) {
        if let Some((base, (source, content))) = &self.prompt {
            if base.repo == repo && base.epoch == self.epoch {
                return (source, content);
            }
        }
        ("Loading", "Loading prompt context…")
    }
    pub(crate) fn snippet_for(
        &self,
        root: &std::path::Path,
        alert: &GuardAlert,
    ) -> Option<&[(usize, String)]> {
        self.snippets
            .iter()
            .find(|(key, _)| {
                key.root == root
                    && key.file == alert.file_path
                    && Some(key.line) == alert.line_no
                    && key.epoch == self.epoch
            })
            .and_then(|(_, value)| value.as_deref())
    }
}
struct Request {
    base: Base,
    metadata: bool,
    prompt: bool,
    staged: bool,
    snippet: Option<SnippetKey>,
}
struct Metadata {
    is_git: bool,
    turn: u64,
    gate: Option<(String, String)>,
}
struct ResultData {
    metadata: Option<Metadata>,
    prompt: Option<(String, String)>,
    staged: Option<usize>,
    snippet: Option<(SnippetKey, Snippet)>,
}
impl Request {
    fn read(&self) -> ResultData {
        let _timing = lgtm_core::debug::Span::new("ui.view_prepare");
        let repo = &self.base.repo;
        ResultData {
            metadata: self.metadata.then(|| {
                let turn = std::fs::read_to_string(repo.join(".lgtm/state.json"))
                    .ok()
                    .and_then(|s| serde_json::from_str::<lgtm_core::turn::TurnState>(&s).ok())
                    .unwrap_or_default()
                    .current_turn;
                Metadata {
                    is_git: GitEngine::new(repo).is_git_repo(),
                    turn,
                    gate: read_gate(repo),
                }
            }),
            prompt: self.prompt.then(|| read_prompt(repo)),
            staged: self.staged.then(|| read_staged_count(repo)),
            snippet: self
                .snippet
                .as_ref()
                .map(|key| (key.clone(), read_snippet(key))),
        }
    }
}
fn read_snippet(key: &SnippetKey) -> Snippet {
    let content = std::fs::read_to_string(key.root.join(&key.file)).ok()?;
    let start = (key.line as usize).saturating_sub(4);
    let end = key.line as usize + 3;
    Some(
        content
            .lines()
            .enumerate()
            .take(end)
            .skip(start)
            .map(|(i, s)| (i + 1, s.to_owned()))
            .collect(),
    )
}
pub(super) fn read_staged_count(root: &std::path::Path) -> usize {
    let mut timing = lgtm_core::debug::Span::new("ui.staged_prepare");
    timing.count("files", 0);
    let engine = GitEngine::new(root);
    let Ok(repo) = engine.open_repo() else {
        return 0;
    };
    let Ok(statuses) = repo.statuses(None) else {
        return 0;
    };
    let count = statuses
        .iter()
        .filter(|s| {
            let st = s.status();
            st.is_index_new()
                || st.is_index_modified()
                || st.is_index_deleted()
                || st.is_index_renamed()
                || st.is_index_typechange()
        })
        .count();
    timing.count("files", count as u64);
    count
}
impl App {
    pub(crate) fn open_agent_prompt(&mut self) {
        self.invalidate_view_data();
        self.view_data.prompt = None;
        self.active_view = ActiveView::AgentPrompt;
    }
    #[cfg(test)]
    pub(crate) fn view_data_pending(&self) -> bool {
        self.view_data.in_flight
    }
    pub(crate) fn invalidate_view_data(&mut self) {
        self.view_data.epoch = self.view_data.epoch.wrapping_add(1);
        self.view_data.snippets.clear();
    }
    pub(super) fn record_staged_count(&mut self, count: usize) {
        self.view_data.staged_count = count;
        self.view_data.staged = Some(Base {
            repo: self.git_engine.repo_path.clone(),
            epoch: self.view_data.epoch,
        });
    }
    /// Called at activity boundaries, never by a render function.
    pub(crate) fn prepare_view_data(&mut self) {
        if self.view_data.in_flight {
            return;
        }
        let base = Base {
            repo: self.git_engine.repo_path.clone(),
            epoch: self.view_data.epoch,
        };
        let metadata = self.view_data.metadata.as_ref() != Some(&base);
        let prompt = self.active_view == ActiveView::AgentPrompt
            && self.view_data.prompt.as_ref().map(|p| &p.0) != Some(&base);
        let staged = self.active_view == ActiveView::CommitModal
            && self.view_data.staged.as_ref() != Some(&base);
        let snippet = if self.active_view == ActiveView::AuditReport {
            self.filtered_audit_alerts()
                .get(self.audit.selected_idx)
                .and_then(|alert| {
                    alert.line_no.map(|line| SnippetKey {
                        root: self.audit.root_dir.clone(),
                        file: alert.file_path.clone(),
                        line,
                        epoch: base.epoch,
                    })
                })
                .filter(|key| {
                    !self
                        .view_data
                        .snippets
                        .iter()
                        .any(|(cached, _)| cached == key)
                })
        } else {
            None
        };
        if !metadata && !prompt && !staged && snippet.is_none() {
            return;
        }
        let request = Request {
            base: base.clone(),
            metadata,
            prompt,
            staged,
            snippet,
        };
        let mut timing = lgtm_core::debug::Span::new("ui.view_request");
        timing.count("metadata", u64::from(metadata));
        timing.count("prompt", u64::from(prompt));
        timing.count("staged", u64::from(staged));
        timing.count("snippet", u64::from(request.snippet.is_some()));
        self.view_data.in_flight = true;
        self.view_data.requests_started += 1;
        self.spawn_silent_job(
            move || request.read(),
            move |app, data| {
                app.view_data.in_flight = false;
                let mut timing = lgtm_core::debug::Span::new("ui.view_apply");
                let current =
                    app.git_engine.repo_path == base.repo && app.view_data.epoch == base.epoch;
                timing.count("applied", u64::from(current));
                if current {
                    if let Some(Metadata {
                        is_git: git,
                        turn,
                        gate,
                    }) = data.metadata
                    {
                        app.view_data.is_git = git;
                        app.view_data.turn = turn;
                        app.view_data.gate = gate;
                        app.view_data.metadata = Some(base.clone());
                    }
                    if let Some(prompt) = data.prompt {
                        app.view_data.prompt = Some((base.clone(), prompt));
                    }
                    if let Some(count) = data.staged {
                        app.view_data.staged_count = count;
                        app.view_data.staged = Some(base);
                    }
                    if let Some(snippet) = data.snippet {
                        if app.view_data.snippets.len() == 32 {
                            app.view_data.snippets.pop_front();
                        }
                        app.view_data.snippets.push_back(snippet);
                    }
                }
                // UI selection/view changes are coalesced into the latest demand.
                app.prepare_view_data();
            },
        );
    }
}

#[cfg(test)]
mod tests;
