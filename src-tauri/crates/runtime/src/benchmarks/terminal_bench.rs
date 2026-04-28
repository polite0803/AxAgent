use super::{BenchScore, BenchEvaluator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalBenchConfig {
    pub shell: String,
    pub work_dir: String,
    pub timeout_secs: u64,
    pub env_vars: Vec<(String, String)>,
}

impl Default for TerminalBenchConfig {
    fn default() -> Self {
        Self {
            shell: "/bin/bash".into(),
            work_dir: ".".into(),
            timeout_secs: 120,
            env_vars: vec![],
        }
    }
}

pub struct TerminalBenchEvaluator;

impl BenchEvaluator for TerminalBenchEvaluator {
    fn evaluate(
        &self,
        output: &str,
        expected: Option<&str>,
        _context: Option<&serde_json::Value>,
    ) -> BenchScore {
        if let Some(expected_output) = expected {
            let trimmed = output.trim();
            let expected_trimmed = expected_output.trim();

            if trimmed == expected_trimmed {
                return BenchScore {
                    score: 1.0,
                    passed: true,
                    details: Some("Output exactly matches expected result".into()),
                };
            }

            if trimmed.contains(expected_trimmed) {
                return BenchScore {
                    score: 0.8,
                    passed: true,
                    details: Some("Expected output found within result".into()),
                };
            }

            let words: Vec<&str> = expected_trimmed.split_whitespace().collect();
            let found = words.iter().filter(|w| trimmed.contains(*w)).count();
            let score = if words.is_empty() {
                0.0
            } else {
                found as f64 / words.len() as f64
            };

            return BenchScore {
                score,
                passed: score > 0.7,
                details: Some(format!(
                    "Partial match: {}/{} expected words found",
                    found,
                    words.len()
                )),
            };
        }

        if output.is_empty() {
            BenchScore {
                score: 0.0,
                passed: false,
                details: Some("No output produced".into()),
            }
        } else if output.to_lowercase().contains("command not found") {
            BenchScore {
                score: 0.0,
                passed: false,
                details: Some("Command not found".into()),
            }
        } else {
            BenchScore {
                score: 0.5,
                passed: true,
                details: Some("Output produced but no expected value to compare".into()),
            }
        }
    }
}
