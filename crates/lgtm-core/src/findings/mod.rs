//! Pure post-guard pipeline: suppress (inline + path ignores) -> severity overrides.
use crate::config::LgtmConfig;
use crate::guard::{GuardAlert, SecurityAndTestGuard, Severity};
use crate::model::FileDiff;

#[derive(Debug, Default, Clone)]
pub struct Findings {
    pub active: Vec<GuardAlert>,
    pub suppressed: Vec<GuardAlert>,
    /// Matched a baseline entry; never blocks.
    pub existing: Vec<GuardAlert>,
    /// Baseline entries with no matching finding (full scans only).
    pub stale: Vec<crate::baseline::Entry>,
}

/// Run all guards, then apply config.
pub fn run(files: &[FileDiff], cfg: &LgtmConfig) -> Findings {
    apply(SecurityAndTestGuard::scan_snapshot(files), files, cfg)
}

/// `run` plus the repo baseline (diff scope, so no stale entries).
pub fn run_in(
    repo: &std::path::Path,
    files: &[FileDiff],
    cfg: &LgtmConfig,
) -> anyhow::Result<Findings> {
    crate::baseline::apply(repo, cfg, run(files, cfg), false)
}

pub fn apply(alerts: Vec<GuardAlert>, files: &[FileDiff], cfg: &LgtmConfig) -> Findings {
    let line_map = crate::guard::DiffLineMap::build(files);
    let mut out = Findings::default();
    for mut a in alerts {
        let ignored = path_ignored(&a, cfg)
            || inline_ignored_mapped(&a, &line_map)
            || below_threshold(&a, cfg);
        if ignored {
            out.suppressed.push(a);
            continue;
        }
        match override_for(&a.rule_id, cfg) {
            Some(None) => out.suppressed.push(a),
            Some(Some(sev)) => {
                a.severity = sev;
                out.active.push(a);
            }
            None => out.active.push(a),
        }
    }
    out
}

fn inline_ignored_mapped(a: &GuardAlert, line_map: &crate::guard::DiffLineMap<'_>) -> bool {
    let Some(ln) = a.line_no else {
        return false;
    };
    let Some(text) = line_map.get(&a.file_path, ln) else {
        return false;
    };
    let Some(rest) = text.split_once("lgtm:ignore(").map(|(_, r)| r) else {
        return false;
    };
    let list = rest.split(')').next().unwrap_or("");
    list.split(',')
        .map(str::trim)
        .any(|p| rule_matches(p, &a.rule_id))
}

/// Prefix glob: `*` matches everything, `dead/*` matches `dead/unused-function`.
fn rule_matches(pat: &str, id: &str) -> bool {
    pat == id || pat.strip_suffix('*').is_some_and(|p| id.starts_with(p))
}

/// `*` stays within a path segment, `**` crosses `/`, and `**/` also matches zero segments.
pub fn path_glob(pat: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') if p.starts_with(b"**") => {
                let rest = &p[p.iter().take_while(|&&c| c == b'*').count()..];
                let bare = rest.strip_prefix(b"/");
                (0..=t.len()).any(|i| go(rest, &t[i..]) || bare.is_some_and(|r| go(r, &t[i..])))
            }
            Some(b'*') => (0..=t.len()).any(|i| !t[..i].contains(&b'/') && go(&p[1..], &t[i..])),
            Some(&c) => t.first() == Some(&c) && go(&p[1..], &t[1..]),
        }
    }
    go(pat.as_bytes(), text.as_bytes())
}

fn path_ignored(a: &GuardAlert, cfg: &LgtmConfig) -> bool {
    cfg.ignore.paths.iter().any(|p| path_glob(p, &a.file_path))
        || cfg.ignore.rules.iter().any(|(r, paths)| {
            rule_matches(r, &a.rule_id) && paths.iter().any(|p| path_glob(p, &a.file_path))
        })
}

/// Guards report spikes from a floor of 8 points; a higher configured threshold hides smaller ones.
fn below_threshold(a: &GuardAlert, cfg: &LgtmConfig) -> bool {
    a.rule_id == "cx/complexity-spike"
        && a.description
            .split("(+")
            .nth(1)
            .and_then(|r| r.split(' ').next())
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n < cfg.thresholds.complexity_increase)
}

/// None = no override, Some(None) = off, Some(Some(s)) = new severity. Exact IDs beat globs.
fn override_for(id: &str, cfg: &LgtmConfig) -> Option<Option<Severity>> {
    let level = cfg.severity.get(id).or_else(|| {
        cfg.severity
            .iter()
            .filter(|(p, _)| rule_matches(p, id))
            .max_by_key(|(p, _)| p.len())
            .map(|(_, l)| l)
    })?;
    Some(match level.as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        // ponytail: no Info severity in the model yet, so `info` maps to Warn
        "warning" | "info" => Some(Severity::Warn),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, LineType};
    use std::path::PathBuf;

    fn file(path: &str, line: &str) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: PathBuf::from(path),
            change_type: ChangeType::Added,
            hunks: vec![DiffHunk {
                lines: vec![DiffLine {
                    line_type: LineType::Addition,
                    content: format!("requests.get(u, {}) {line}", ["verify=", "False"].concat()), // lgtm:ignore
                    old_line_num: None,
                    new_line_num: Some(1),
                }],
                ..Default::default()
            }],
            additions: 1,
            deletions: 0,
            is_folded: false,
            is_reviewed: false,
        }
    }

    fn cfg(toml: &str) -> LgtmConfig {
        let d =
            std::env::temp_dir().join(format!("lgtm-fnd-{}-{}", std::process::id(), toml.len()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(crate::config::REPO_FILE), toml).unwrap();
        LgtmConfig::load(&d, None).unwrap()
    }

    #[test]
    fn no_config_changes_nothing() {
        let f = run(&[file("src/a.py", "")], &LgtmConfig::default());
        assert_eq!(f.active.len(), 1);
        assert!(f.suppressed.is_empty());
        assert_eq!(f.active[0].severity, Severity::High);
    }

    #[test]
    fn severity_override_and_off() {
        let files = [file("src/a.py", "")];
        let f = run(
            &files,
            &cfg("[severity]\n\"sec/tls-bypass\" = \"warning\"\n"),
        );
        assert_eq!(f.active[0].severity, Severity::Warn);
        let f = run(&files, &cfg("[severity]\n\"sec/*\" = \"off\"\n"));
        assert!(f.active.is_empty());
        assert_eq!(f.suppressed.len(), 1);
        let f = run(
            &files,
            &cfg("[severity]\n\"sec/*\" = \"off\"\n\"sec/tls-bypass\" = \"critical\"\n"),
        );
        assert_eq!(f.active[0].severity, Severity::Critical);
    }

    #[test]
    fn path_ignores() {
        let g = "[ignore]\npaths = [\"vendor/**\", \"**/generated/**\"]\n";
        for p in ["vendor/x/a.py", "src/generated/a.py", "generated/a.py"] {
            assert_eq!(run(&[file(p, "")], &cfg(g)).suppressed.len(), 1, "{p}");
        }
        assert_eq!(run(&[file("src/a.py", "")], &cfg(g)).active.len(), 1);
        let r = "[ignore.rules]\n\"sec/*\" = [\"tests/**\"]\n";
        assert_eq!(run(&[file("tests/a.py", "")], &cfg(r)).suppressed.len(), 1);
        assert_eq!(run(&[file("src/a.py", "")], &cfg(r)).active.len(), 1);
    }

    #[test]
    fn scoped_inline_suppression() {
        let c = LgtmConfig::default();
        let hit = run(&[file("a.py", "# lgtm:ignore(sec/tls-bypass)")], &c);
        assert_eq!(hit.suppressed.len(), 1);
        let glob = run(&[file("a.py", "# lgtm:ignore(dead/*, sec/*)")], &c);
        assert_eq!(glob.suppressed.len(), 1);
        let miss = run(&[file("a.py", "# lgtm:ignore(dead/*)")], &c);
        assert_eq!(miss.active.len(), 1);
    }

    #[test]
    fn complexity_threshold_hides_small_spikes() {
        let a = GuardAlert {
            rule_id: "cx/complexity-spike".into(),
            fingerprint: String::new(),
            severity: Severity::High,
            title: "t".into(),
            description: "Hunk introduces (+9 points with nesting)".into(),
            line_no: None,
            file_path: "a.rs".into(),
        };
        assert_eq!(
            apply(
                vec![a.clone()],
                &[],
                &cfg("[thresholds]\ncomplexity_increase = 10\n")
            )
            .suppressed
            .len(),
            1
        );
        assert_eq!(apply(vec![a], &[], &LgtmConfig::default()).active.len(), 1);
    }

    #[test]
    fn globs() {
        assert!(path_glob("*.rs", "a.rs"));
        assert!(!path_glob("*.rs", "src/a.rs"));
        assert!(path_glob("src/**", "src/a/b.rs"));
        assert!(path_glob("**/b.rs", "b.rs"));
    }
}
