use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    Transient,
    Recoverable,
    Unrecoverable,
    Unknown,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::Transient => "transient",
            ErrorType::Recoverable => "recoverable",
            ErrorType::Unrecoverable => "unrecoverable",
            ErrorType::Unknown => "unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ErrorType::Transient => "Temporary error - retry may resolve",
            ErrorType::Recoverable => "Recoverable error - can be fixed with adjustment",
            ErrorType::Unrecoverable => "Unrecoverable error - should fail",
            ErrorType::Unknown => "Unknown error type",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    pub error_type: ErrorType,
    pub original_error: String,
    pub error_code: Option<String>,
    pub context: Option<String>,
}

pub struct ErrorClassifier;

impl ErrorClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, error: &str) -> ErrorType {
        let error_lower = error.to_lowercase();

        if Self::is_transient(&error_lower) {
            ErrorType::Transient
        } else if Self::is_recoverable(&error_lower) {
            ErrorType::Recoverable
        } else if Self::is_unrecoverable(&error_lower) {
            ErrorType::Unrecoverable
        } else {
            ErrorType::Unknown
        }
    }

    pub fn classify_with_context(&self, error: &str, context: Option<String>) -> ClassifiedError {
        ClassifiedError {
            error_type: self.classify(error),
            original_error: error.to_string(),
            error_code: Self::extract_error_code(error),
            context,
        }
    }

    fn is_transient(error: &str) -> bool {
        let transient_patterns = [
            "timeout",
            "timed out",
            "network",
            "connection",
            "refused",
            "unreachable",
            "temporarily unavailable",
            "service unavailable",
            "503",
            "502",
            "504",
            "429",
            "rate limit",
            "too many requests",
            "reset by peer",
            "broken pipe",
            "econnreset",
            "econnrefused",
            "etimedout",
            "enotfound",
        ];

        transient_patterns.iter().any(|p| error.contains(p))
    }

    fn is_recoverable(error: &str) -> bool {
        let recoverable_patterns = [
            "permission denied",
            "access denied",
            "unauthorized",
            "forbidden",
            "resource exhausted",
            "out of memory",
            "disk full",
            "quota exceeded",
            "limit exceeded",
            "capacity",
            "insufficient",
            "not found",
            "invalid state",
            "conflict",
            "409",
            "413",
            "401",
            "403",
        ];

        recoverable_patterns.iter().any(|p| error.contains(p))
    }

    fn is_unrecoverable(error: &str) -> bool {
        let unrecoverable_patterns = [
            "syntax error",
            "parse error",
            "invalid syntax",
            "illegal",
            "malformed",
            "unsupported",
            "not implemented",
            "invalid format",
            "type mismatch",
            "cast error",
            "null pointer",
            "panic",
            "assertion",
            "invariant",
            "500",
            "internal error",
        ];

        unrecoverable_patterns.iter().any(|p| error.contains(p))
    }

    fn extract_error_code(error: &str) -> Option<String> {
        if let Some(caps) = regex_lite::Regex::new(r"(?i)error[_\s]?code[:\s]+(\d+)")
            .ok()
            .and_then(|r| r.captures(error))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }

        if let Some(caps) = regex_lite::Regex::new(r"\b(4\d{2}|5\d{2})\b")
            .ok()
            .and_then(|r| r.captures(error))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }

        None
    }
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(
            classifier.classify("connection timeout"),
            ErrorType::Transient
        );
        assert_eq!(
            classifier.classify("network error: 503"),
            ErrorType::Transient
        );
        assert_eq!(
            classifier.classify("rate limit exceeded"),
            ErrorType::Transient
        );
    }

    #[test]
    fn test_recoverable_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(
            classifier.classify("permission denied"),
            ErrorType::Recoverable
        );
        assert_eq!(
            classifier.classify("resource exhausted"),
            ErrorType::Recoverable
        );
        assert_eq!(
            classifier.classify("401 unauthorized"),
            ErrorType::Recoverable
        );
    }

    #[test]
    fn test_unrecoverable_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(
            classifier.classify("syntax error"),
            ErrorType::Unrecoverable
        );
        assert_eq!(
            classifier.classify("invalid format"),
            ErrorType::Unrecoverable
        );
        assert_eq!(
            classifier.classify("internal server error: 500"),
            ErrorType::Unrecoverable
        );
    }
}
