use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub current_turn: u64,
    pub last_reviewed_tree_oid: Option<String>,
    pub turn_timestamps: Vec<String>,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            current_turn: 1,
            last_reviewed_tree_oid: None,
            turn_timestamps: Vec::new(),
        }
    }
}

pub struct TurnManager {
    state_path: PathBuf,
}

impl TurnManager {
    pub fn new<P: AsRef<Path>>(repo_root: P) -> Self {
        let dir = repo_root.as_ref().join(".lgtm");
        let _ = create_dir_all(&dir);
        Self {
            state_path: dir.join("state.json"),
        }
    }

    pub fn load_state(&self) -> TurnState {
        if let Ok(mut file) = File::open(&self.state_path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(state) = serde_json::from_str::<TurnState>(&content) {
                    return state;
                }
            }
        }
        TurnState::default()
    }

    pub fn advance_turn(&self, tree_oid: Option<String>) -> Result<u64> {
        let mut state = self.load_state();
        state.current_turn += 1;
        state.last_reviewed_tree_oid = tree_oid;
        state.turn_timestamps.push(chrono_iso_now());

        if let Some(parent) = self.state_path.parent() {
            let _ = create_dir_all(parent);
        }

        let mut file = File::create(&self.state_path)?;
        let json = serde_json::to_string_pretty(&state)?;
        file.write_all(json.as_bytes())?;
        Ok(state.current_turn)
    }
}

pub fn chrono_iso_now() -> String {
    // Basic timestamp helper without extra dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("timestamp_{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lgtm-test-{}-{}", name, unique));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn test_turn_manager_flow() {
        let tmp = test_dir("turn");
        let manager = TurnManager::new(&tmp);

        let initial = manager.load_state();
        assert_eq!(initial.current_turn, 1);
        assert!(initial.last_reviewed_tree_oid.is_none());

        let turn2 = manager
            .advance_turn(Some("abcdef123456".to_string()))
            .unwrap();
        assert_eq!(turn2, 2);

        let loaded = manager.load_state();
        assert_eq!(loaded.current_turn, 2);
        assert_eq!(
            loaded.last_reviewed_tree_oid.as_deref(),
            Some("abcdef123456")
        );
        assert_eq!(loaded.turn_timestamps.len(), 1);

        let turn3 = manager
            .advance_turn(Some("789012345678".to_string()))
            .unwrap();
        assert_eq!(turn3, 3);
        let loaded3 = manager.load_state();
        assert_eq!(loaded3.current_turn, 3);
        assert_eq!(loaded3.turn_timestamps.len(), 2);
    }
}
