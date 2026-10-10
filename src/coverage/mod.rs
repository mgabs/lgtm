use crate::model::{ChangeType, DiffSnapshot, LineType};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct FileCoverage {
    pub path: PathBuf,
    pub covered_lines: HashSet<usize>,
    pub uncovered_lines: HashSet<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct CoverageReport {
    pub files: HashMap<PathBuf, FileCoverage>,
}

impl CoverageReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse standard LCOV tracefile format
    pub fn parse_lcov(content: &str) -> Result<Self> {
        let mut report = CoverageReport::new();
        let mut current_file: Option<FileCoverage> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(source_file) = line.strip_prefix("SF:") {
                let path = PathBuf::from(source_file);
                current_file = Some(FileCoverage {
                    path,
                    covered_lines: HashSet::new(),
                    uncovered_lines: HashSet::new(),
                });
            } else if let Some(da_data) = line.strip_prefix("DA:") {
                if let Some(ref mut file_cov) = current_file {
                    let parts: Vec<&str> = da_data.split(',').collect();
                    if parts.len() >= 2 {
                        if let (Ok(line_no), Ok(hits)) =
                            (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                        {
                            if hits > 0 {
                                file_cov.covered_lines.insert(line_no);
                            } else {
                                file_cov.uncovered_lines.insert(line_no);
                            }
                        }
                    }
                }
            } else if line == "end_of_record" {
                if let Some(file_cov) = current_file.take() {
                    report.files.insert(file_cov.path.clone(), file_cov);
                }
            }
        }

        if let Some(file_cov) = current_file {
            report.files.insert(file_cov.path.clone(), file_cov);
        }

        Ok(report)
    }

    /// Check if a specific file and line was executed
    pub fn is_line_covered(&self, file_path: &Path, line_no: usize) -> Option<bool> {
        // Try exact path match, or normalized suffix match
        for (path, cov) in &self.files {
            if path == file_path || path.ends_with(file_path) || file_path.ends_with(path) {
                if cov.covered_lines.contains(&line_no) {
                    return Some(true);
                }
                if cov.uncovered_lines.contains(&line_no) {
                    return Some(false);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct UncoveredDiffLine {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct FileDeltaCoverage {
    pub file_path: PathBuf,
    pub added_lines_count: usize,
    pub covered_added_count: usize,
    #[allow(dead_code)]
    pub uncovered_lines: Vec<UncoveredDiffLine>,
}

impl FileDeltaCoverage {
    pub fn coverage_pct(&self) -> f64 {
        if self.added_lines_count == 0 {
            100.0
        } else {
            (self.covered_added_count as f64 / self.added_lines_count as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeltaCoverageResult {
    pub file_results: Vec<FileDeltaCoverage>,
    pub total_added_lines: usize,
    pub covered_added_lines: usize,
    pub overall_coverage_pct: f64,
    pub all_uncovered_lines: Vec<UncoveredDiffLine>,
}

pub struct DeltaCoverageEngine;

impl DeltaCoverageEngine {
    /// Compute delta coverage strictly for newly added lines across all changed files in diff
    pub fn compute(diff: &DiffSnapshot, report: &CoverageReport) -> DeltaCoverageResult {
        let mut file_results = Vec::new();
        let mut total_added_lines = 0;
        let mut covered_added_lines = 0;
        let mut all_uncovered_lines = Vec::new();

        for file_diff in &diff.files {
            // Only examine added / modified files (skip deleted)
            if file_diff.change_type == ChangeType::Deleted {
                continue;
            }

            let mut file_added_count = 0;
            let mut file_covered_count = 0;
            let mut file_uncovered = Vec::new();

            for hunk in &file_diff.hunks {
                for line in &hunk.lines {
                    if line.line_type == LineType::Addition {
                        let trimmed = line.content.trim();
                        // Ignore comments, empty lines, and simple structural braces
                        if trimmed.is_empty()
                            || trimmed.starts_with("//")
                            || trimmed.starts_with("/*")
                            || trimmed.starts_with('*')
                            || trimmed.starts_with('#')
                            || trimmed == "{"
                            || trimmed == "}"
                            || trimmed == "};"
                            || trimmed == ")"
                            || trimmed == ");"
                            || trimmed == "]"
                            || trimmed == "];"
                        {
                            continue;
                        }

                        if let Some(new_line_no) = line.new_line_num {
                            let line_idx = new_line_no as usize;
                            file_added_count += 1;
                            let is_covered = report
                                .is_line_covered(&file_diff.new_path, line_idx)
                                .unwrap_or(false);

                            if is_covered {
                                file_covered_count += 1;
                            } else {
                                let unc = UncoveredDiffLine {
                                    file_path: file_diff.new_path.clone(),
                                    line_number: line_idx,
                                    content: line.content.clone(),
                                };
                                file_uncovered.push(unc.clone());
                                all_uncovered_lines.push(unc);
                            }
                        }
                    }
                }
            }

            total_added_lines += file_added_count;
            covered_added_lines += file_covered_count;

            file_results.push(FileDeltaCoverage {
                file_path: file_diff.new_path.clone(),
                added_lines_count: file_added_count,
                covered_added_count: file_covered_count,
                uncovered_lines: file_uncovered,
            });
        }

        let overall_coverage_pct = if total_added_lines == 0 {
            100.0
        } else {
            (covered_added_lines as f64 / total_added_lines as f64) * 100.0
        };

        DeltaCoverageResult {
            file_results,
            total_added_lines,
            covered_added_lines,
            overall_coverage_pct,
            all_uncovered_lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiffHunk, DiffLine, FileDiff};

    #[test]
    fn test_parse_lcov_format() {
        let lcov_data = r#"
TN:
SF:src/lib.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,5
end_of_record
SF:src/main.rs
DA:10,0
DA:11,1
end_of_record
"#;
        let report = CoverageReport::parse_lcov(lcov_data).unwrap();
        assert_eq!(report.files.len(), 2);

        let lib_cov = report.files.get(Path::new("src/lib.rs")).unwrap();
        assert!(lib_cov.covered_lines.contains(&1));
        assert!(lib_cov.covered_lines.contains(&2));
        assert!(lib_cov.uncovered_lines.contains(&3));
        assert!(lib_cov.covered_lines.contains(&4));

        assert_eq!(
            report.is_line_covered(Path::new("src/lib.rs"), 1),
            Some(true)
        );
        assert_eq!(
            report.is_line_covered(Path::new("src/lib.rs"), 3),
            Some(false)
        );
        assert_eq!(report.is_line_covered(Path::new("src/lib.rs"), 99), None);
    }

    #[test]
    fn test_delta_coverage_computation() {
        let lcov_data = r#"
SF:src/auth.rs
DA:10,1
DA:11,1
DA:12,0
DA:13,1
end_of_record
"#;
        let report = CoverageReport::parse_lcov(lcov_data).unwrap();

        let mut diff = DiffSnapshot::default();
        let hunk = DiffHunk {
            header: "@@ -1,5 +1,5 @@".to_string(),
            old_start: 1,
            old_lines: 5,
            new_start: 1,
            new_lines: 5,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "let token = authenticate(user);".to_string(),
                    old_line_num: None,
                    new_line_num: Some(10),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "validate_token(&token)?;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(11),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "log_security_event(&user);".to_string(),
                    old_line_num: None,
                    new_line_num: Some(12),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "// this is a comment".to_string(),
                    old_line_num: None,
                    new_line_num: Some(15),
                },
            ],
            is_folded: false,
        };

        diff.files.push(FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/auth.rs"),
            change_type: ChangeType::Modified,
            additions: 4,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        });

        let delta_res = DeltaCoverageEngine::compute(&diff, &report);
        // Lines 10, 11 covered; line 12 uncovered; line 15 is comment (ignored)
        assert_eq!(delta_res.total_added_lines, 3);
        assert_eq!(delta_res.covered_added_lines, 2);
        assert_eq!(delta_res.all_uncovered_lines.len(), 1);
        assert_eq!(delta_res.all_uncovered_lines[0].line_number, 12);
        assert!((delta_res.overall_coverage_pct - 66.66).abs() < 0.1);
    }

    #[test]
    fn test_delta_coverage_deleted_and_empty() {
        let report = CoverageReport::default();
        let mut diff = DiffSnapshot::default();

        diff.files.push(FileDiff {
            old_path: Some(PathBuf::from("src/old.rs")),
            new_path: PathBuf::from("src/old.rs"),
            change_type: ChangeType::Deleted,
            additions: 0,
            deletions: 10,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        });

        let delta_res = DeltaCoverageEngine::compute(&diff, &report);
        assert_eq!(delta_res.total_added_lines, 0);
        assert_eq!(delta_res.covered_added_lines, 0);
        assert_eq!(delta_res.overall_coverage_pct, 100.0);
    }

    #[test]
    fn test_delta_coverage_fully_covered() {
        let lcov_data = "SF:src/lib.rs\nDA:1,1\nDA:2,2\nend_of_record\n";
        let report = CoverageReport::parse_lcov(lcov_data).unwrap();

        let mut diff = DiffSnapshot::default();
        let hunk = DiffHunk {
            header: "@@ -1,2 +1,2 @@".to_string(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "let x = 1;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "let y = 2;".to_string(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
        };

        diff.files.push(FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/lib.rs"),
            change_type: ChangeType::Added,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        });

        let delta_res = DeltaCoverageEngine::compute(&diff, &report);
        assert_eq!(delta_res.total_added_lines, 2);
        assert_eq!(delta_res.covered_added_lines, 2);
        assert_eq!(delta_res.overall_coverage_pct, 100.0);
        assert!(delta_res.all_uncovered_lines.is_empty());
    }

    #[test]
    fn test_file_delta_coverage_pct() {
        let fdc = FileDeltaCoverage {
            file_path: PathBuf::from("src/test.rs"),
            added_lines_count: 0,
            covered_added_count: 0,
            uncovered_lines: vec![],
        };
        assert_eq!(fdc.coverage_pct(), 100.0);

        let fdc2 = FileDeltaCoverage {
            file_path: PathBuf::from("src/test.rs"),
            added_lines_count: 4,
            covered_added_count: 3,
            uncovered_lines: vec![],
        };
        assert_eq!(fdc2.coverage_pct(), 75.0);
    }
}
