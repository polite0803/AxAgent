use super::{BenchScore, BenchEvaluator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweBenchConfig {
    pub repo_path: String,
    pub base_commit: Option<String>,
    pub test_command: String,
    pub timeout_secs: u64,
}

impl Default for SweBenchConfig {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            base_commit: None,
            test_command: "pytest".into(),
            timeout_secs: 300,
        }
    }
}

pub struct SweBenchEvaluator;

impl BenchEvaluator for SweBenchEvaluator {
    fn evaluate(
        &self,
        output: &str,
        expected: Option<&str>,
        _context: Option<&serde_json::Value>,
    ) -> BenchScore {
        if let Some(expected_output) = expected {
            let normalized_output = output.trim().to_lowercase();
            let normalized_expected = expected_output.trim().to_lowercase();

            if normalized_output.contains(&normalized_expected) {
                return BenchScore {
                    score: 1.0,
                    passed: true,
                    details: Some("Output matches expected result".into()),
                };
            }

            let similarity = strsim::levenshtein(&normalized_output, &normalized_expected);
            let max_len = normalized_output.len().max(normalized_expected.len()).max(1);
            let score = 1.0 - (similarity as f64 / max_len as f64);

            return BenchScore {
                score,
                passed: false,
                details: Some(format!(
                    "Output differs from expected (levenshtein distance: {})",
                    similarity
                )),
            };
        }

        let has_error = output.to_lowercase().contains("error")
            || output.to_lowercase().contains("failed")
            || output.to_lowercase().contains("traceback");

        if has_error {
            BenchScore {
                score: 0.0,
                passed: false,
                details: Some("Output contains error indications".into()),
            }
        } else {
            BenchScore {
                score: 1.0,
                passed: true,
                details: Some("No errors detected in output".into()),
            }
        }
    }
}

mod strsim {
    pub fn levenshtein(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let m = a_chars.len();
        let n = b_chars.len();

        let mut dp = vec![vec![0usize; n + 1]; 2];

        for (j, item) in dp[0].iter_mut().enumerate().take(n + 1) {
            *item = j;
        }

        for i in 1..=m {
            dp[1][0] = i;
            for j in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                dp[1][j] = (dp[0][j] + 1)
                    .min(dp[1][j - 1] + 1)
                    .min(dp[0][j - 1] + cost);
            }
            dp.swap(0, 1);
        }

        dp[1][n]
    }
}
