use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resolution {
    Done,
    WontFix,
    Other(String),
}

impl Resolution {
    pub fn is_settable(&self) -> bool {
        !matches!(self, Resolution::Other(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreadState {
    Open,
    Resolved(Resolution),
}

impl ThreadState {
    pub fn display_name(&self) -> String {
        match self {
            ThreadState::Open => "Open".to_string(),
            ThreadState::Resolved(Resolution::Done) => "Done".to_string(),
            ThreadState::Resolved(Resolution::WontFix) => "WontFix".to_string(),
            ThreadState::Resolved(Resolution::Other(text)) => format!("Other({text})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadComment {
    pub id: u64,
    pub author: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    pub id: String,
    pub path: String,
    pub line: Option<u32>,
    pub state: ThreadState,
    pub outdated: bool,
    pub can_resolve: Option<bool>,
    pub can_unresolve: Option<bool>,
    pub comments: Vec<ThreadComment>,
}

impl Thread {
    pub fn resolved(&self) -> bool {
        matches!(self.state, ThreadState::Resolved(_))
    }

    pub fn check_permission(&self, target: &ThreadState) -> Result<(), &'static str> {
        match target {
            ThreadState::Resolved(res) => {
                if !res.is_settable() {
                    return Err("Resolution type is read-only");
                }
                if self.can_resolve == Some(false) {
                    return Err("User does not have permission to resolve this thread");
                }
                Ok(())
            }
            ThreadState::Open => {
                if self.can_unresolve == Some(false) {
                    return Err("User does not have permission to unresolve this thread");
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub requested: ThreadState,
    pub applied: ThreadState,
}

impl Applied {
    pub fn exact(state: ThreadState) -> Self {
        Self {
            requested: state.clone(),
            applied: state,
        }
    }

    pub fn fallback(requested: ThreadState, applied: ThreadState) -> Self {
        Self { requested, applied }
    }

    pub fn is_fallback(&self) -> bool {
        self.requested != self.applied
    }

    pub fn summary_note(&self) -> Option<String> {
        if self.is_fallback() {
            Some(format!(
                "requested {}, applied {}",
                self.requested.display_name(),
                self.applied.display_name()
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSetResolved {
    pub thread_id: String,
    pub resolved: bool,
    #[serde(default)]
    pub resolution: Option<Resolution>,
}

impl DraftSetResolved {
    pub fn target_state(&self) -> ThreadState {
        if self.resolved {
            ThreadState::Resolved(self.resolution.clone().unwrap_or(Resolution::Done))
        } else {
            ThreadState::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_state_and_resolved() {
        let t_resolved = Thread {
            id: "t1".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Resolved(Resolution::Done),
            outdated: false,
            can_resolve: Some(true),
            can_unresolve: Some(true),
            comments: vec![],
        };
        assert!(t_resolved.resolved());

        let t_open = Thread {
            id: "t2".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Open,
            outdated: false,
            can_resolve: Some(true),
            can_unresolve: Some(true),
            comments: vec![],
        };
        assert!(!t_open.resolved());
    }

    #[test]
    fn test_applied_fallback() {
        let exact = Applied::exact(ThreadState::Resolved(Resolution::Done));
        assert!(!exact.is_fallback());
        assert_eq!(exact.summary_note(), None);

        let fallback = Applied::fallback(
            ThreadState::Resolved(Resolution::WontFix),
            ThreadState::Resolved(Resolution::Done),
        );
        assert!(fallback.is_fallback());
        assert_eq!(
            fallback.summary_note(),
            Some("requested WontFix, applied Done".to_string())
        );
    }

    #[test]
    fn test_other_not_settable() {
        let other = Resolution::Other("closed".into());
        assert!(!other.is_settable());

        let t = Thread {
            id: "t1".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Open,
            outdated: false,
            can_resolve: Some(true),
            can_unresolve: Some(true),
            comments: vec![],
        };
        assert!(t.check_permission(&ThreadState::Resolved(other)).is_err());
    }

    #[test]
    fn test_permission_rejection() {
        let t_no_resolve = Thread {
            id: "t1".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Open,
            outdated: false,
            can_resolve: Some(false),
            can_unresolve: Some(true),
            comments: vec![],
        };
        assert!(t_no_resolve
            .check_permission(&ThreadState::Resolved(Resolution::Done))
            .is_err());

        let t_no_unresolve = Thread {
            id: "t2".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Resolved(Resolution::Done),
            outdated: false,
            can_resolve: Some(true),
            can_unresolve: Some(false),
            comments: vec![],
        };
        assert!(t_no_unresolve.check_permission(&ThreadState::Open).is_err());

        let t_unknown = Thread {
            id: "t3".into(),
            path: "a.rs".into(),
            line: Some(10),
            state: ThreadState::Open,
            outdated: false,
            can_resolve: None,
            can_unresolve: None,
            comments: vec![],
        };
        assert!(t_unknown
            .check_permission(&ThreadState::Resolved(Resolution::Done))
            .is_ok());
    }

    #[test]
    fn test_draft_serde_with_and_without_resolution() {
        // With explicit resolution
        let json_with_res = r#"{"thread_id":"t1","resolved":true,"resolution":"WontFix"}"#;
        let draft: DraftSetResolved = serde_json::from_str(json_with_res).unwrap();
        assert_eq!(
            draft.target_state(),
            ThreadState::Resolved(Resolution::WontFix)
        );

        // Legacy draft without resolution field
        let json_legacy = r#"{"thread_id":"t2","resolved":true}"#;
        let draft_legacy: DraftSetResolved = serde_json::from_str(json_legacy).unwrap();
        assert_eq!(
            draft_legacy.target_state(),
            ThreadState::Resolved(Resolution::Done)
        );

        // Legacy unresolve draft
        let json_unresolve = r#"{"thread_id":"t3","resolved":false}"#;
        let draft_unres: DraftSetResolved = serde_json::from_str(json_unresolve).unwrap();
        assert_eq!(draft_unres.target_state(), ThreadState::Open);
    }
}
