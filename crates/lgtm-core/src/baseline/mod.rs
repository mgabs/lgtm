use crate::config::LgtmConfig;
use crate::findings::Findings;
use crate::guard::{GuardAlert, Severity};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entry {
    pub rule_id: String,
    pub file: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Entry {
    fn of(a: &GuardAlert) -> Self {
        Self {
            rule_id: a.rule_id.clone(),
            file: a.file_path.clone(),
            fingerprint: a.fingerprint.clone(),
        }
    }
}

impl Baseline {
    /// Missing file means an empty baseline.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text).with_context(|| format!("invalid baseline {}", path.display()))
    }

    /// Sorted and deduplicated so the file diffs cleanly.
    pub fn save(&self, path: &Path) -> Result<()> {
        let entries: BTreeSet<_> = self.entries.iter().cloned().collect();
        let out = Baseline {
            version: 1,
            entries: entries.into_iter().collect(),
        };
        std::fs::write(path, serde_json::to_string_pretty(&out)? + "\n")?;
        Ok(())
    }

    /// Add current findings (Critical only with `allow_critical`); `prune` also drops stale entries.
    pub fn update(&mut self, current: &[GuardAlert], allow_critical: bool, prune: bool) {
        let cur: BTreeSet<Entry> = current
            .iter()
            .filter(|a| baselinable(a, allow_critical))
            .map(Entry::of)
            .collect();
        let mut all: BTreeSet<Entry> = if prune {
            BTreeSet::new()
        } else {
            self.entries.iter().cloned().collect()
        };
        all.extend(cur);
        self.entries = all.into_iter().collect();
    }
}

fn baselinable(a: &GuardAlert, allow_critical: bool) -> bool {
    allow_critical || a.severity != Severity::Critical
}

/// Split into new (`active`), `existing` and, for full scans only, `stale` entries.
/// A diff only sees part of the repo, so stale is meaningless there.
pub fn partition(
    mut f: Findings,
    bl: &Baseline,
    allow_critical: bool,
    full_scan: bool,
) -> Findings {
    let known: BTreeSet<&Entry> = bl.entries.iter().collect();
    let mut seen = BTreeSet::new();
    for a in std::mem::take(&mut f.active) {
        let e = Entry::of(&a);
        if baselinable(&a, allow_critical) && known.contains(&e) {
            seen.insert(e);
            f.existing.push(a);
        } else {
            f.active.push(a);
        }
    }
    if full_scan {
        f.stale = bl
            .entries
            .iter()
            .filter(|e| !seen.contains(*e))
            .cloned()
            .collect();
    }
    f
}

pub fn path_for(repo: &Path, cfg: &LgtmConfig) -> std::path::PathBuf {
    repo.join(&cfg.baseline.file)
}

/// Load the repo baseline and partition `f` against it.
pub fn apply(repo: &Path, cfg: &LgtmConfig, f: Findings, full_scan: bool) -> Result<Findings> {
    let bl = Baseline::load(&path_for(repo, cfg))?;
    Ok(partition(f, &bl, cfg.baseline.allow_critical, full_scan))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(rule: &str, fp: &str, sev: Severity) -> GuardAlert {
        GuardAlert {
            rule_id: rule.into(),
            fingerprint: fp.into(),
            severity: sev,
            title: "t".into(),
            description: "d".into(),
            line_no: Some(1),
            file_path: "a.rs".into(),
        }
    }

    fn found(alerts: Vec<GuardAlert>) -> Findings {
        Findings {
            active: alerts,
            ..Default::default()
        }
    }

    #[test]
    fn update_partition_and_prune() {
        let a = alert("x/a", "1", Severity::High);
        let b = alert("x/b", "2", Severity::Warn);
        let mut bl = Baseline::default();
        bl.update(&[a.clone(), b.clone()], false, false);
        assert_eq!(bl.entries.len(), 2);

        let c = alert("x/c", "3", Severity::High);
        let p = partition(found(vec![a.clone(), c.clone()]), &bl, false, true);
        assert_eq!(p.active, vec![c.clone()]);
        assert_eq!(p.existing, vec![a.clone()]);
        assert_eq!(p.stale.len(), 1);
        assert_eq!(p.stale[0].rule_id, "x/b");

        let diff_only = partition(found(vec![a.clone()]), &bl, false, false);
        assert!(diff_only.stale.is_empty());

        bl.update(&[a.clone(), c], false, false);
        assert_eq!(bl.entries.len(), 3);
        bl.update(&[a], false, true);
        assert_eq!(bl.entries.len(), 1);
    }

    #[test]
    fn critical_never_baselined_by_default() {
        let k = alert("sec/k", "9", Severity::Critical);
        let mut bl = Baseline::default();
        bl.update(std::slice::from_ref(&k), false, false);
        assert!(bl.entries.is_empty());
        bl.update(std::slice::from_ref(&k), true, false);
        assert_eq!(bl.entries.len(), 1);
        assert_eq!(
            partition(found(vec![k.clone()]), &bl, false, false)
                .active
                .len(),
            1
        );
        assert_eq!(
            partition(found(vec![k]), &bl, true, false).existing.len(),
            1
        );
    }

    #[test]
    fn save_is_sorted_deterministic_and_roundtrips() {
        let d = std::env::temp_dir().join(format!("lgtm-bl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("b.json");
        assert!(Baseline::load(&p).unwrap().entries.is_empty());
        let mut bl = Baseline::default();
        bl.update(
            &[
                alert("z/z", "2", Severity::High),
                alert("a/a", "1", Severity::High),
            ],
            false,
            false,
        );
        bl.save(&p).unwrap();
        let first = std::fs::read_to_string(&p).unwrap();
        let loaded = Baseline::load(&p).unwrap();
        assert_eq!(loaded.entries[0].rule_id, "a/a");
        loaded.save(&p).unwrap();
        assert_eq!(first, std::fs::read_to_string(&p).unwrap());
        std::fs::write(&p, "nope").unwrap();
        assert!(Baseline::load(&p).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
