//! `.lgtm/drafts/pr-<N>.json`: drafts only, written atomically, scoped per PR and repository.
use super::DraftSet;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    repo_key: String,
    drafts: DraftSet,
}

fn path(root: &Path, pr: u64) -> PathBuf {
    root.join(".lgtm/drafts").join(format!("pr-{pr}.json"))
}

/// Missing, unreadable, other-version or other-repo files yield an empty set.
pub fn load(root: &Path, pr: u64, repo_key: &str) -> DraftSet {
    std::fs::read_to_string(path(root, pr))
        .ok()
        .and_then(|s| serde_json::from_str::<File>(&s).ok())
        .filter(|f| f.version == VERSION && f.repo_key == repo_key)
        .map(|f| f.drafts)
        .unwrap_or_default()
}

/// Atomic write; an empty set removes the file.
pub fn save(root: &Path, pr: u64, repo_key: &str, drafts: &DraftSet) -> Result<()> {
    let target = path(root, pr);
    if drafts.is_empty() {
        return match std::fs::remove_file(&target) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(target.parent().expect("drafts dir"))?;
    let tmp = target.with_extension("json.tmp");
    let file = File {
        version: VERSION,
        repo_key: repo_key.to_string(),
        drafts: drafts.clone(),
    };
    std::fs::write(&tmp, serde_json::to_vec_pretty(&file)?).context("write drafts")?;
    std::fs::rename(&tmp, &target).context("replace drafts")
}
