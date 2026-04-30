use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_HISTORY_LINES: usize = 5000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalError {
    pub line_number: usize,
    pub error_type: TerminalErrorType,
    pub message: String,
    pub context: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalErrorType {
    CompilationError,
    RuntimeError,
    TestFailure,
    LintWarning,
    PermissionDenied,
    CommandNotFound,
    NetworkError,
    Timeout,
    OutOfMemory,
    Unknown,
}

impl std::fmt::Display for TerminalErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalErrorType::CompilationError => write!(f, "compilation_error"),
            TerminalErrorType::RuntimeError => write!(f, "runtime_error"),
            TerminalErrorType::TestFailure => write!(f, "test_failure"),
            TerminalErrorType::LintWarning => write!(f, "lint_warning"),
            TerminalErrorType::PermissionDenied => write!(f, "permission_denied"),
            TerminalErrorType::CommandNotFound => write!(f, "command_not_found"),
            TerminalErrorType::NetworkError => write!(f, "network_error"),
            TerminalErrorType::Timeout => write!(f, "timeout"),
            TerminalErrorType::OutOfMemory => write!(f, "out_of_memory"),
            TerminalErrorType::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalAnalysis {
    pub has_errors: bool,
    pub errors: Vec<TerminalError>,
    pub last_exit_code: Option<i32>,
    pub last_command: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSuggestion {
    pub action: String,
    pub description: String,
    pub confidence: f64,
}

pub struct TerminalAnalyzer {
    history: VecDeque<String>,
    last_exit_code: Option<i32>,
    last_command: Option<String>,
    #[allow(dead_code)]
    prompt_pattern: Regex,
    error_patterns: Vec<(Regex, TerminalErrorType)>,
}

impl TerminalAnalyzer {
    pub fn new() -> Self {
        let error_patterns = Self::build_error_patterns();

        let prompt_pattern =
            Regex::new(r"(?m)^[^$]*[\$#>]\s*$").unwrap_or_else(|_| Regex::new(r"^$").unwrap());

        Self {
            history: VecDeque::with_capacity(MAX_HISTORY_LINES),
            last_exit_code: None,
            last_command: None,
            prompt_pattern,
            error_patterns,
        }
    }

    fn build_error_patterns() -> Vec<(Regex, TerminalErrorType)> {
        let patterns: Vec<(&str, TerminalErrorType)> = vec![
            (r"(?mi)^error\[E\d+\]", TerminalErrorType::CompilationError),
            (r"(?mi)^error:\s*", TerminalErrorType::CompilationError),
            (
                r"(?mi)error:\s*aborted\s+due\s+to\s+previous\s+error",
                TerminalErrorType::CompilationError,
            ),
            (
                r"(?mi)cannot\s+find\s+",
                TerminalErrorType::CompilationError,
            ),
            (
                r"(?mi)mismatched\s+types?",
                TerminalErrorType::CompilationError,
            ),
            (r"(?mi)^FAILED\s+\(", TerminalErrorType::TestFailure),
            (
                r"(?mi)test\s+result:\s+FAILED",
                TerminalErrorType::TestFailure,
            ),
            (r"(?mi)panic!\(", TerminalErrorType::RuntimeError),
            (
                r"(?mi)thread\s+'[^']*'\s+panicked",
                TerminalErrorType::RuntimeError,
            ),
            (r"(?mi)stack\s+overflow", TerminalErrorType::RuntimeError),
            (
                r"(?mi)index\s+out\s+of\s+bounds",
                TerminalErrorType::RuntimeError,
            ),
            (
                r"(?mi)Permission\s+denied",
                TerminalErrorType::PermissionDenied,
            ),
            (
                r"(?mi)command\s+not\s+found",
                TerminalErrorType::CommandNotFound,
            ),
            (
                r"(?mi)no\s+such\s+file\s+or\s+directory",
                TerminalErrorType::CommandNotFound,
            ),
            (
                r"(?mi)Connection\s+refused",
                TerminalErrorType::NetworkError,
            ),
            (
                r"(?mi)Connection\s+timed?\s+out",
                TerminalErrorType::NetworkError,
            ),
            (
                r"(?mi)network\s+is\s+unreachable",
                TerminalErrorType::NetworkError,
            ),
            (r"(?mi)ETIMEDOUT", TerminalErrorType::Timeout),
            (r"(?mi)Timed?\s+out", TerminalErrorType::Timeout),
            (
                r"(?mi)Cannot\s+allocate\s+memory",
                TerminalErrorType::OutOfMemory,
            ),
            (r"(?mi)out\s+of\s+memory", TerminalErrorType::OutOfMemory),
            (r"(?mi)OOM", TerminalErrorType::OutOfMemory),
            (r"(?mi)warning:\s*", TerminalErrorType::LintWarning),
            (r"(?mi)warn\s*:\s*", TerminalErrorType::LintWarning),
        ];

        patterns
            .into_iter()
            .filter_map(|(p, t)| Regex::new(p).ok().map(|r| (r, t)))
            .collect()
    }

    pub fn push_output(&mut self, output: &str) {
        for line in output.lines() {
            if self.history.len() >= MAX_HISTORY_LINES {
                self.history.pop_front();
            }
            self.history.push_back(line.to_string());
        }
    }

    pub fn set_exit_code(&mut self, code: i32) {
        self.last_exit_code = Some(code);
    }

    pub fn set_last_command(&mut self, command: &str) {
        self.last_command = Some(command.to_string());
    }

    pub fn analyze(&self) -> TerminalAnalysis {
        let errors = self.detect_errors();

        let summary = if errors.is_empty() {
            if self.last_exit_code == Some(0) {
                "Command completed successfully".to_string()
            } else if let Some(code) = self.last_exit_code {
                format!(
                    "Command exited with code {} but no recognized error patterns found",
                    code
                )
            } else {
                "No errors detected in terminal output".to_string()
            }
        } else {
            let error_types: Vec<String> =
                errors.iter().map(|e| e.error_type.to_string()).collect();
            format!(
                "Found {} error(s): {}",
                errors.len(),
                error_types.join(", ")
            )
        };

        TerminalAnalysis {
            has_errors: !errors.is_empty(),
            errors,
            last_exit_code: self.last_exit_code,
            last_command: self.last_command.clone(),
            summary,
        }
    }

    fn detect_errors(&self) -> Vec<TerminalError> {
        let mut errors = Vec::new();
        let lines: Vec<&str> = self.history.iter().map(|s| s.as_str()).collect();

        for (line_idx, line) in lines.iter().enumerate() {
            for (pattern, error_type) in &self.error_patterns {
                if pattern.is_match(line) {
                    let context_start = line_idx.saturating_sub(2);
                    let context_end = (line_idx + 3).min(lines.len());
                    let context: Vec<String> = lines[context_start..context_end]
                        .iter()
                        .map(|l| l.to_string())
                        .collect();

                    errors.push(TerminalError {
                        line_number: line_idx + 1,
                        error_type: *error_type,
                        message: line.trim().to_string(),
                        context,
                    });
                    break;
                }
            }
        }

        errors
    }

    pub fn suggest_fixes(&self, analysis: &TerminalAnalysis) -> Vec<TerminalSuggestion> {
        let mut suggestions = Vec::new();

        for error in &analysis.errors {
            match error.error_type {
                TerminalErrorType::CompilationError => {
                    if error.message.contains("cannot find") {
                        suggestions.push(TerminalSuggestion {
                            action: "check_imports".to_string(),
                            description: "Check if the required module, crate, or symbol is properly imported or declared".to_string(),
                            confidence: 0.8,
                        });
                    }
                    if error.message.contains("mismatched types") {
                        suggestions.push(TerminalSuggestion {
                            action: "fix_type_mismatch".to_string(),
                            description: "Add type conversion or adjust the type annotation to match the expected type".to_string(),
                            confidence: 0.85,
                        });
                    }
                    suggestions.push(TerminalSuggestion {
                        action: "read_error_details".to_string(),
                        description: "Read the full compilation error output to identify the exact location and cause".to_string(),
                        confidence: 0.7,
                    });
                }
                TerminalErrorType::TestFailure => {
                    suggestions.push(TerminalSuggestion {
                        action: "read_test_output".to_string(),
                        description: "Read the test failure output to understand which assertion failed and why".to_string(),
                        confidence: 0.9,
                    });
                    suggestions.push(TerminalSuggestion {
                        action: "run_single_test".to_string(),
                        description:
                            "Run the specific failing test in isolation for clearer output"
                                .to_string(),
                        confidence: 0.75,
                    });
                }
                TerminalErrorType::PermissionDenied => {
                    suggestions.push(TerminalSuggestion {
                        action: "check_permissions".to_string(),
                        description:
                            "Check file/directory permissions or run with appropriate privileges"
                                .to_string(),
                        confidence: 0.85,
                    });
                }
                TerminalErrorType::CommandNotFound => {
                    suggestions.push(TerminalSuggestion {
                        action: "install_dependency".to_string(),
                        description: "The command was not found. Install the required tool or check the command name".to_string(),
                        confidence: 0.9,
                    });
                }
                TerminalErrorType::NetworkError => {
                    suggestions.push(TerminalSuggestion {
                        action: "check_network".to_string(),
                        description: "Check network connectivity and retry the operation"
                            .to_string(),
                        confidence: 0.7,
                    });
                }
                TerminalErrorType::Timeout => {
                    suggestions.push(TerminalSuggestion {
                        action: "increase_timeout".to_string(),
                        description: "The operation timed out. Consider increasing the timeout or optimizing the operation".to_string(),
                        confidence: 0.75,
                    });
                }
                TerminalErrorType::OutOfMemory => {
                    suggestions.push(TerminalSuggestion {
                        action: "reduce_memory_usage".to_string(),
                        description: "The process ran out of memory. Try processing data in smaller chunks or increasing available memory".to_string(),
                        confidence: 0.8,
                    });
                }
                TerminalErrorType::RuntimeError => {
                    suggestions.push(TerminalSuggestion {
                        action: "read_stack_trace".to_string(),
                        description:
                            "Read the stack trace to identify the source of the runtime error"
                                .to_string(),
                        confidence: 0.85,
                    });
                }
                TerminalErrorType::LintWarning => {
                    suggestions.push(TerminalSuggestion {
                        action: "fix_lint_warning".to_string(),
                        description: "Address the lint warning to improve code quality".to_string(),
                        confidence: 0.6,
                    });
                }
                TerminalErrorType::Unknown => {
                    suggestions.push(TerminalSuggestion {
                        action: "investigate".to_string(),
                        description: "Investigate the unrecognized error pattern".to_string(),
                        confidence: 0.3,
                    });
                }
            }
        }

        suggestions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions.dedup_by(|a, b| a.action == b.action);
        suggestions
    }

    pub fn get_recent_output(&self, max_lines: usize) -> Vec<String> {
        self.history
            .iter()
            .rev()
            .take(max_lines)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.last_exit_code = None;
        self.last_command = None;
    }
}

impl Default for TerminalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_compilation_error() {
        let mut analyzer = TerminalAnalyzer::new();
        analyzer.push_output("Compiling myproject v0.1.0\nerror[E0425]: cannot find value `x` in this scope\n  --> src/main.rs:4:5");
        let analysis = analyzer.analyze();
        assert!(analysis.has_errors);
        assert_eq!(
            analysis.errors[0].error_type,
            TerminalErrorType::CompilationError
        );
    }

    #[test]
    fn test_detect_test_failure() {
        let mut analyzer = TerminalAnalyzer::new();
        analyzer.push_output("running 3 tests\nFAILED test_addition\nassertion failed");
        let analysis = analyzer.analyze();
        assert!(analysis.has_errors);
        assert!(analysis
            .errors
            .iter()
            .any(|e| e.error_type == TerminalErrorType::TestFailure));
    }

    #[test]
    fn test_no_errors() {
        let mut analyzer = TerminalAnalyzer::new();
        analyzer.push_output("Compiling myproject v0.1.0\nFinished dev [unoptimized + debuginfo]\nRunning target/debug/myproject");
        analyzer.set_exit_code(0);
        let analysis = analyzer.analyze();
        assert!(!analysis.has_errors);
    }

    #[test]
    fn test_suggest_fix() {
        let mut analyzer = TerminalAnalyzer::new();
        analyzer.push_output("error: cannot find value `x` in this scope");
        let analysis = analyzer.analyze();
        let suggestions = analyzer.suggest_fixes(&analysis);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].confidence > 0.5);
    }

    #[test]
    fn test_recent_output() {
        let mut analyzer = TerminalAnalyzer::new();
        for i in 0..10 {
            analyzer.push_output(&format!("line {}", i));
        }
        let recent = analyzer.get_recent_output(3);
        assert_eq!(recent.len(), 3);
        assert!(recent[0].contains("line 7"));
    }
}
