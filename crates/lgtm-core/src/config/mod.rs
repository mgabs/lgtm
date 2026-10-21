use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const REPO_FILE: &str = ".lgtm.toml";
const LEVELS: [&str; 5] = ["critical", "high", "warning", "info", "off"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    pub complexity_increase: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Ignore {
    pub paths: Vec<String>,
    pub rules: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct BaselineConfig {
    pub file: String,
    pub allow_critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct LgtmConfig {
    pub thresholds: Thresholds,
    pub severity: BTreeMap<String, String>,
    pub ignore: Ignore,
    pub baseline: BaselineConfig,
    pub editor: Option<String>,
    pub agent_sink: Option<String>,
    #[serde(skip)]
    pub sources: BTreeMap<String, String>,
    #[serde(skip)]
    pub warnings: Vec<String>,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            complexity_increase: 5,
        }
    }
}

impl Default for BaselineConfig {
    fn default() -> Self {
        Self {
            file: ".lgtm-baseline.json".into(),
            allow_critical: false,
        }
    }
}

/// `~/.config/lgtm/config.toml`, or None when disabled by flag/env or HOME is unset.
pub fn user_config_path(disabled: bool) -> Option<PathBuf> {
    if disabled || std::env::var_os("LGTM_NO_USER_CONFIG").is_some() {
        return None;
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/lgtm/config.toml"))
}

fn read_table(path: &Path) -> Result<Option<toml::Table>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)?;
    let t = text
        .parse::<toml::Table>()
        .with_context(|| format!("invalid TOML in {}", path.display()))?;
    Ok(Some(t))
}

/// Tables merge key-by-key; any other value (including lists) replaces.
fn merge(
    base: &mut toml::Table,
    top: toml::Table,
    label: &str,
    prefix: &str,
    src: &mut BTreeMap<String, String>,
) {
    for (k, v) in top {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(t)) => merge(b, t, label, &path, src),
            (_, toml::Value::Table(t)) => {
                let mut fresh = toml::Table::new();
                merge(&mut fresh, t, label, &path, src);
                base.insert(k, toml::Value::Table(fresh));
            }
            (_, v) => {
                src.retain(|p, _| !p.starts_with(&format!("{path}.")));
                src.insert(path, label.to_string());
                base.insert(k, v);
            }
        }
    }
}

fn unknown_keys(t: &toml::Table, prefix: &str, out: &mut Vec<String>) {
    const FREE: [&str; 2] = ["severity", "ignore.rules"];
    for (k, v) in t {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        let known = [
            "thresholds",
            "severity",
            "ignore",
            "baseline",
            "editor",
            "agent_sink",
        ]
        .contains(&path.as_str())
            || [
                "thresholds.complexity_increase",
                "ignore.paths",
                "ignore.rules",
                "baseline.file",
                "baseline.allow_critical",
                "editor",
                "agent_sink",
            ]
            .contains(&path.as_str())
            || FREE.iter().any(|f| path.starts_with(&format!("{f}.")));
        if !known {
            out.push(format!("unknown config key `{path}`"));
        } else if let toml::Value::Table(sub) = v {
            unknown_keys(sub, &path, out);
        }
    }
}

impl LgtmConfig {
    /// Precedence: repo file > user file > defaults. CLI overrides are applied by callers.
    pub fn load(repo: &Path, user: Option<&Path>) -> Result<Self> {
        let mut merged = toml::Table::new();
        let mut sources = BTreeMap::new();
        let layers = [
            (user.map(Path::to_path_buf), "user"),
            (Some(repo.join(REPO_FILE)), "repo"),
        ];
        for (path, label) in layers {
            if let Some(t) = path.as_deref().map(read_table).transpose()?.flatten() {
                merge(&mut merged, t, label, "", &mut sources);
            }
        }
        let mut warnings = Vec::new();
        unknown_keys(&merged, "", &mut warnings);
        let mut cfg: LgtmConfig = merged.try_into().context("invalid config values")?;
        for (rule, level) in &cfg.severity {
            if !LEVELS.contains(&level.as_str()) {
                bail!(
                    "severity for `{rule}` must be one of {}, got `{level}`",
                    LEVELS.join("|")
                );
            }
        }
        cfg.sources = sources;
        cfg.warnings = warnings;
        Ok(cfg)
    }

    fn source(&self, key: &str) -> &str {
        self.sources.get(key).map_or("default", String::as_str)
    }

    pub fn show(&self) -> String {
        let mut o = String::new();
        let line = |o: &mut String, k: &str, v: String| {
            o.push_str(&format!("{k} = {v}  ({})\n", self.source(k)))
        };
        line(
            &mut o,
            "thresholds.complexity_increase",
            self.thresholds.complexity_increase.to_string(),
        );
        line(&mut o, "baseline.file", format!("{:?}", self.baseline.file));
        line(
            &mut o,
            "baseline.allow_critical",
            self.baseline.allow_critical.to_string(),
        );
        line(&mut o, "ignore.paths", format!("{:?}", self.ignore.paths));
        if let Some(ref ed) = self.editor {
            line(&mut o, "editor", format!("{ed:?}"));
        }
        if let Some(ref sink) = self.agent_sink {
            line(&mut o, "agent_sink", format!("{sink:?}"));
        }
        for (r, l) in &self.severity {
            line(&mut o, &format!("severity.{r}"), format!("{l:?}"));
        }
        for (r, p) in &self.ignore.rules {
            line(&mut o, &format!("ignore.rules.{r}"), format!("{p:?}"));
        }
        for w in &self.warnings {
            o.push_str(&format!("warning: {w}\n"));
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lgtm-cfg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn defaults_without_files() {
        let d = dir("none");
        let c = LgtmConfig::load(&d, None).unwrap();
        assert_eq!(c.thresholds.complexity_increase, 5);
        assert_eq!(c.baseline.file, ".lgtm-baseline.json");
        assert!(c
            .show()
            .contains("thresholds.complexity_increase = 5  (default)"));
    }

    #[test]
    fn repo_overrides_user_tables_merge_lists_replace() {
        let d = dir("merge");
        let user = d.join("user.toml");
        std::fs::write(
            &user,
            "[thresholds]\ncomplexity_increase = 9\n[severity]\n\"dead/*\" = \"off\"\n[ignore]\npaths = [\"a/**\", \"b/**\"]\n",
        )
        .unwrap();
        std::fs::write(
            d.join(REPO_FILE),
            "[thresholds]\ncomplexity_increase = 7\n[severity]\n\"sec/tls-bypass\" = \"high\"\n[ignore]\npaths = [\"c/**\"]\n",
        )
        .unwrap();
        let c = LgtmConfig::load(&d, Some(&user)).unwrap();
        assert_eq!(c.thresholds.complexity_increase, 7);
        assert_eq!(c.severity.len(), 2);
        assert_eq!(c.ignore.paths, vec!["c/**"]);
        let s = c.show();
        assert!(s.contains("complexity_increase = 7  (repo)"));
        assert!(s.contains("severity.dead/* = \"off\"  (user)"));
        assert!(s.contains("ignore.paths = [\"c/**\"]  (repo)"));
    }

    #[test]
    fn user_only_source_and_unknown_keys_warn() {
        let d = dir("unk");
        let user = d.join("u.toml");
        std::fs::write(
            &user,
            "[thresholds]\ncomplexity_increase = 3\nbogus = 1\n[nope]\nx = 1\n",
        )
        .unwrap();
        let c = LgtmConfig::load(&d, Some(&user)).unwrap();
        assert_eq!(c.thresholds.complexity_increase, 3);
        assert!(c.warnings.iter().any(|w| w.contains("thresholds.bogus")));
        assert!(c.warnings.iter().any(|w| w.contains("`nope`")));
        assert!(c.show().contains("(user)"));
    }

    #[test]
    fn rejects_bad_severity_and_bad_toml() {
        let d = dir("bad");
        std::fs::write(d.join(REPO_FILE), "[severity]\n\"a/b\" = \"loud\"\n").unwrap();
        assert!(LgtmConfig::load(&d, None)
            .unwrap_err()
            .to_string()
            .contains("loud"));
        std::fs::write(d.join(REPO_FILE), "= nope").unwrap();
        assert!(LgtmConfig::load(&d, None).is_err());
    }

    #[test]
    fn user_path_respects_disable() {
        assert!(user_config_path(true).is_none());
    }

    #[test]
    fn editor_config_option() {
        let d = dir("editor");
        std::fs::write(d.join(REPO_FILE), "editor = \"vscode\"\n").unwrap();
        let c = LgtmConfig::load(&d, None).unwrap();
        assert_eq!(c.editor.as_deref(), Some("vscode"));
        assert!(c.warnings.is_empty());
        assert!(c.show().contains("editor = \"vscode\"  (repo)"));
    }
}
