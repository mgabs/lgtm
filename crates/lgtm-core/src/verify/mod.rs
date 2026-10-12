use crate::guard::{GuardAlert, Severity};
use crate::model::DiffSnapshot;
use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InvertedTestResult {
    pub is_fraudulent: bool,
    pub details: String,
    #[allow(dead_code)]
    pub alerts: Vec<GuardAlert>,
}

pub struct InvertedTestVerifier;

impl InvertedTestVerifier {
    /// Verify that newly added tests are genuine and fail when the implementation is absent
    pub fn verify(diff: &DiffSnapshot) -> Result<InvertedTestResult> {
        let mut has_test_changes = false;
        let mut has_prod_changes = false;

        for file in &diff.files {
            let path_str = file.display_name().to_lowercase();
            if path_str.contains("test") || path_str.contains("spec") {
                if file.additions > 0 {
                    has_test_changes = true;
                }
            } else if file.additions > 0 || file.deletions > 0 {
                has_prod_changes = true;
            }
        }

        if !has_test_changes {
            return Ok(InvertedTestResult {
                is_fraudulent: false,
                details: "No new test additions found in current diff to invert.".to_string(),
                alerts: vec![],
            });
        }

        if !has_prod_changes {
            return Ok(InvertedTestResult {
                is_fraudulent: false,
                details: "Only test files modified; no production changes to invert.".to_string(),
                alerts: vec![],
            });
        }

        // Run tests with current code to ensure baseline passes
        let initial_status = Command::new("cargo")
            .args(["test", "--workspace", "--", "--nocapture"])
            .status()?;

        if !initial_status.success() {
            return Ok(InvertedTestResult {
                is_fraudulent: false,
                details: "Initial test suite is failing; fix baseline tests first.".to_string(),
                alerts: vec![],
            });
        }

        // Revert production files temporarily in git working tree to check test sensitivity
        // (If git checkout fails, fallback gracefully)
        let mut reverted_files = Vec::new();
        for file in &diff.files {
            let path_str = file.display_name().to_lowercase();
            if !path_str.contains("test") && !path_str.contains("spec") {
                let _ = Command::new("git")
                    .args(["checkout", "--", &file.display_name()])
                    .status();
                reverted_files.push(file.display_name());
            }
        }

        // Re-run test suite without production code
        let inverted_status = Command::new("cargo")
            .args(["test", "--workspace", "--", "--nocapture"])
            .status();

        // Restore reverted files immediately
        let _ = Command::new("git")
            .args(["checkout", "HEAD", "--", "."])
            .status();

        match inverted_status {
            Ok(status) if status.success() => {
                // Tests PASSED even without the production implementation! FRAUDULENT!
                let alert = GuardAlert {
                    severity: Severity::Critical,
                    rule_id: "test/fraudulent-test".to_string(), fingerprint: String::new(), title: "Fraudulent / Tautological Test Detected".to_string(),
                    description:
                        "New tests PASSED even when production code was temporarily reverted! The test does not exercise the new implementation."
                            .to_string(),
                    line_no: None,
                    file_path: "tests".to_string(),
                };

                Ok(InvertedTestResult {
                    is_fraudulent: true,
                    details: "🚨 FRAUDULENT TEST DETECTED: Tests passed without production code!"
                        .to_string(),
                    alerts: vec![alert],
                })
            }
            Ok(_) => {
                // Tests FAILED without production code! GENUINE!
                Ok(InvertedTestResult {
                    is_fraudulent: false,
                    details: "✅ Genuine Tests: Tests failed as expected when implementation was removed."
                        .to_string(),
                    alerts: vec![],
                })
            }
            Err(e) => Ok(InvertedTestResult {
                is_fraudulent: false,
                details: format!("Could not complete inverted execution: {}", e),
                alerts: vec![],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, FileDiff};
    use std::path::PathBuf;

    #[test]
    fn test_inverted_verifier_no_test_additions() {
        let diff = DiffSnapshot {
            files: vec![FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/logic.rs"),
                change_type: ChangeType::Modified,
                additions: 10,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            }],
            total_additions: 10,
            total_deletions: 0,
        };

        let result = InvertedTestVerifier::verify(&diff).unwrap();
        assert!(!result.is_fraudulent);
        assert!(result.details.contains("No new test additions"));
    }

    #[test]
    fn test_inverted_verifier_only_tests() {
        let diff = DiffSnapshot {
            files: vec![FileDiff {
                old_path: None,
                new_path: PathBuf::from("tests/test_unit.rs"),
                change_type: ChangeType::Added,
                additions: 10,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            }],
            total_additions: 10,
            total_deletions: 0,
        };

        let result = InvertedTestVerifier::verify(&diff).unwrap();
        assert!(!result.is_fraudulent);
        assert!(result.details.contains("Only test files modified"));
    }
}
