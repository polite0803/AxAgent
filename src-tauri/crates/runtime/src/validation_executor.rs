use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub assertion_type: AssertionType,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    Equals,
    Contains,
    Matches,
    Exists,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub failed_assertions: Vec<FailedAssertion>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAssertion {
    pub assertion: Assertion,
    pub error: String,
}

pub struct ValidationExecutor;

impl ValidationExecutor {
    pub fn validate(assertions: &[Assertion], context: &serde_json::Value) -> ValidationResult {
        let start = std::time::Instant::now();
        let mut failed_assertions = Vec::new();

        for assertion in assertions {
            if let Err(e) = Self::check_assertion(assertion, context) {
                failed_assertions.push(FailedAssertion {
                    assertion: assertion.clone(),
                    error: e,
                });
            }
        }

        ValidationResult {
            passed: failed_assertions.is_empty(),
            failed_assertions,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn check_assertion(assertion: &Assertion, context: &serde_json::Value) -> Result<(), String> {
        match assertion.assertion_type {
            AssertionType::Equals => {
                let expected = assertion
                    .expected
                    .as_ref()
                    .ok_or("Missing expected value")?;
                let actual_path = assertion.actual.as_ref().ok_or("Missing actual path")?;
                let actual_str = context
                    .get(actual_path)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Cannot extract actual value".to_string())?;

                if actual_str == expected {
                    Ok(())
                } else {
                    Err(format!("Expected '{}' but got '{}'", expected, actual_str))
                }
            },
            AssertionType::Contains => {
                let expected = assertion
                    .expected
                    .as_ref()
                    .ok_or("Missing expected value")?;
                let actual_path = assertion.actual.as_ref().ok_or("Missing actual path")?;
                let actual_str = context
                    .get(actual_path)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Cannot extract actual value".to_string())?;

                if actual_str.contains(expected) {
                    Ok(())
                } else {
                    Err(format!("'{}' does not contain '{}'", actual_str, expected))
                }
            },
            AssertionType::Matches => {
                let pattern = assertion.expected.as_ref().ok_or("Missing regex pattern")?;
                let actual_path = assertion.actual.as_ref().ok_or("Missing actual path")?;
                let actual_str = context
                    .get(actual_path)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Cannot extract actual value".to_string())?;

                match Regex::new(pattern) {
                    Ok(re) => {
                        if re.is_match(actual_str) {
                            Ok(())
                        } else {
                            Err(format!(
                                "'{}' does not match pattern '{}'",
                                actual_str, pattern
                            ))
                        }
                    },
                    Err(_) => Err(format!("Invalid regex pattern: {}", pattern)),
                }
            },
            AssertionType::Exists => {
                let actual_path = assertion.actual.as_ref().ok_or("Missing path")?;
                if context.get(actual_path).is_some() {
                    Ok(())
                } else {
                    Err(format!("Path '{}' does not exist", actual_path))
                }
            },
            AssertionType::Custom => Err("Custom assertions require sandbox execution".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equals_assertion() {
        let assertions = vec![Assertion {
            assertion_type: AssertionType::Equals,
            expected: Some("hello".to_string()),
            actual: Some("result".to_string()),
            expression: None,
        }];
        let context = serde_json::json!({ "result": "hello" });

        let result = ValidationExecutor::validate(&assertions, &context);
        assert!(result.passed);
        assert!(result.failed_assertions.is_empty());
    }

    #[test]
    fn test_equals_assertion_fail() {
        let assertions = vec![Assertion {
            assertion_type: AssertionType::Equals,
            expected: Some("hello".to_string()),
            actual: Some("result".to_string()),
            expression: None,
        }];
        let context = serde_json::json!({ "result": "world" });

        let result = ValidationExecutor::validate(&assertions, &context);
        assert!(!result.passed);
        assert_eq!(result.failed_assertions.len(), 1);
    }

    #[test]
    fn test_contains_assertion() {
        let assertions = vec![Assertion {
            assertion_type: AssertionType::Contains,
            expected: Some("world".to_string()),
            actual: Some("message".to_string()),
            expression: None,
        }];
        let context = serde_json::json!({ "message": "hello world" });

        let result = ValidationExecutor::validate(&assertions, &context);
        assert!(result.passed);
    }

    #[test]
    fn test_matches_assertion() {
        let assertions = vec![Assertion {
            assertion_type: AssertionType::Matches,
            expected: Some(r"\d+-\d+-\d+".to_string()),
            actual: Some("date".to_string()),
            expression: None,
        }];
        let context = serde_json::json!({ "date": "2024-01-15" });

        let result = ValidationExecutor::validate(&assertions, &context);
        assert!(result.passed);
    }

    #[test]
    fn test_exists_assertion() {
        let assertions = vec![Assertion {
            assertion_type: AssertionType::Exists,
            expected: None,
            actual: Some("data".to_string()),
            expression: None,
        }];
        let context = serde_json::json!({ "data": "present" });

        let result = ValidationExecutor::validate(&assertions, &context);
        assert!(result.passed);
    }
}
