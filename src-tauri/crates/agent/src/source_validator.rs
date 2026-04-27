use crate::research_state::SourceType;
use crate::search_provider::ExtractedContent;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceValidationResult {
    pub url: String,
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub score: f32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub code: IssueCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueCode {
    MalformedUrl,
    Unreachable,
    DeadLink,
    SuspiciousContent,
    Paywall,
    ExpiredContent,
    InvalidSsl,
    RateLimited,
    ParseError,
}

impl std::fmt::Display for IssueCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueCode::MalformedUrl => write!(f, "malformed_url"),
            IssueCode::Unreachable => write!(f, "unreachable"),
            IssueCode::DeadLink => write!(f, "dead_link"),
            IssueCode::SuspiciousContent => write!(f, "suspicious_content"),
            IssueCode::Paywall => write!(f, "paywall"),
            IssueCode::ExpiredContent => write!(f, "expired_content"),
            IssueCode::InvalidSsl => write!(f, "invalid_ssl"),
            IssueCode::RateLimited => write!(f, "rate_limited"),
            IssueCode::ParseError => write!(f, "parse_error"),
        }
    }
}

pub struct SourceValidator {
    config: ValidatorConfig,
    known_domains: HashMap<String, DomainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub check_ssl: bool,
    pub check_accessibility: bool,
    pub max_age_days: Option<i64>,
    pub allowed_content_types: Vec<String>,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            check_ssl: true,
            check_accessibility: false,
            max_age_days: Some(365),
            allowed_content_types: vec![
                "text/html".to_string(),
                "application/pdf".to_string(),
                "text/plain".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    pub domain: String,
    pub source_type: SourceType,
    pub credibility_weight: f32,
    pub is_paywalled: bool,
    pub notes: String,
}

impl SourceValidator {
    pub fn new() -> Self {
        let mut known_domains = HashMap::new();

        known_domains.insert(
            "arxiv.org".to_string(),
            DomainInfo {
                domain: "arxiv.org".to_string(),
                source_type: SourceType::Academic,
                credibility_weight: 0.95,
                is_paywalled: false,
                notes: "Open access preprint server for academic papers".to_string(),
            },
        );

        known_domains.insert(
            "github.com".to_string(),
            DomainInfo {
                domain: "github.com".to_string(),
                source_type: SourceType::GitHub,
                credibility_weight: 0.85,
                is_paywalled: false,
                notes: "Code hosting and collaboration platform".to_string(),
            },
        );

        known_domains.insert(
            "wikipedia.org".to_string(),
            DomainInfo {
                domain: "wikipedia.org".to_string(),
                source_type: SourceType::Wikipedia,
                credibility_weight: 0.75,
                is_paywalled: false,
                notes: "Free online encyclopedia".to_string(),
            },
        );

        known_domains.insert(
            "docs.rs".to_string(),
            DomainInfo {
                domain: "docs.rs".to_string(),
                source_type: SourceType::Documentation,
                credibility_weight: 0.9,
                is_paywalled: false,
                notes: "Rust documentation and crate registry".to_string(),
            },
        );

        Self {
            config: ValidatorConfig::default(),
            known_domains,
        }
    }

    pub fn with_config(mut self, config: ValidatorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_known_domain(&mut self, info: DomainInfo) {
        self.known_domains.insert(info.domain.clone(), info);
    }

    pub async fn validate_url(&self, url: &str) -> SourceValidationResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut score: f32 = 1.0;

        if !self.is_valid_url(url) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                code: IssueCode::MalformedUrl,
                message: format!("URL '{}' is malformed", url),
            });
            score -= 0.5;
        }

        if let Some(domain_info) = self.get_domain_info(url) {
            if domain_info.is_paywalled {
                warnings.push("This source may be behind a paywall".to_string());
                score -= 0.1;
            }
        }

        let domain = self.extract_domain(url);
        if self.known_domains.contains_key(&domain) {
            warnings.push(format!("Known source: {}", domain));
        }

        let is_valid = issues.iter().all(|i| i.severity != IssueSeverity::Error);

        SourceValidationResult {
            url: url.to_string(),
            is_valid,
            issues,
            score: score.max(0.0_f32),
            warnings,
        }
    }

    pub async fn validate_content(&self, content: &ExtractedContent) -> SourceValidationResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut score: f32 = 1.0;

        if content.text.is_empty() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                code: IssueCode::ParseError,
                message: "Content appears to be empty".to_string(),
            });
            score -= 0.2;
        }

        if content.text.len() < 100 {
            warnings.push("Content is very short".to_string());
            score -= 0.1;
        }

        let suspicious_patterns = vec![
            ("click here", "suspicious_link"),
            ("buy now", "advertisement"),
            ("subscribe", "paywall_hint"),
            ("limited time", "advertisement"),
        ];

        let text_lower = content.text.to_lowercase();
        for (pattern, _) in &suspicious_patterns {
            if text_lower.contains(pattern) {
                warnings.push(format!("Contains suspicious pattern: {}", pattern));
                score -= 0.05;
            }
        }

        let domain = self.extract_domain(&content.url);
        if let Some(domain_info) = self.known_domains.get(&domain) {
            score *= domain_info.credibility_weight;
        }

        if content.extracted_at < Utc::now() - Duration::days(30) {
            warnings.push("Content was extracted over 30 days ago".to_string());
        }

        let is_valid = issues.iter().all(|i| i.severity != IssueSeverity::Error);

        SourceValidationResult {
            url: content.url.clone(),
            is_valid,
            issues,
            score: score.max(0.0),
            warnings,
        }
    }

    pub fn is_valid_url(&self, url: &str) -> bool {
        if url.is_empty() {
            return false;
        }

        let has_scheme = url.starts_with("http://") || url.starts_with("https://");
        if !has_scheme {
            return false;
        }

        url::Url::parse(url).is_ok()
    }

    pub fn extract_domain(&self, url: &str) -> String {
        url::Url::parse(url)
            .ok()
            .and_then(|u: url::Url| u.host_str().map(|s: &str| s.to_string()))
            .unwrap_or_default()
    }

    pub fn get_domain_info(&self, url: &str) -> Option<&DomainInfo> {
        let domain = self.extract_domain(url);
        self.known_domains.get(&domain)
    }

    pub fn validate_batch(&self, urls: &[String]) -> Vec<SourceValidationResult> {
        urls.iter()
            .map(|url| tokio::runtime::Handle::current().block_on(self.validate_url(url)))
            .collect()
    }

    pub fn get_source_type_from_domain(&self, url: &str) -> Option<SourceType> {
        let domain = self.extract_domain(url);
        self.known_domains.get(&domain).map(|info| info.source_type)
    }
}

impl Default for SourceValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SourceFilter {
    min_score: f32,
    allowed_types: Vec<SourceType>,
    blocked_domains: Vec<String>,
}

impl SourceFilter {
    pub fn new() -> Self {
        Self {
            min_score: 0.3,
            allowed_types: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }

    pub fn min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }

    pub fn allowed_types(mut self, types: Vec<SourceType>) -> Self {
        self.allowed_types = types;
        self
    }

    pub fn block_domain(mut self, domain: impl Into<String>) -> Self {
        self.blocked_domains.push(domain.into());
        self
    }

    pub fn filter(&self, results: Vec<(String, SourceValidationResult)>) -> Vec<String> {
        results
            .into_iter()
            .filter(|(url, validation)| {
                if validation.score < self.min_score {
                    return false;
                }

                let domain = url::Url::parse(url)
                    .ok()
                    .and_then(|u: url::Url| u.host_str().map(|s: &str| s.to_string()))
                    .unwrap_or_default();

                if self.blocked_domains.contains(&domain) {
                    return false;
                }

                if !self.allowed_types.is_empty() {
                    if let Some(source_type) =
                        SourceValidator::new().get_source_type_from_domain(url)
                    {
                        return self.allowed_types.contains(&source_type);
                    }
                }

                true
            })
            .map(|(url, _)| url)
            .collect()
    }
}

impl Default for SourceFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_known_domain() {
        let validator = SourceValidator::new();
        let result = validator
            .validate_url("https://arxiv.org/abs/2103.00001")
            .await;

        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_validate_malformed_url() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("not-a-valid-url").await;

        assert!(!result.is_valid);
    }

    #[test]
    fn test_extract_domain() {
        let validator = SourceValidator::new();

        assert_eq!(
            validator.extract_domain("https://arxiv.org/abs/2103.00001"),
            "arxiv.org"
        );
        assert_eq!(
            validator.extract_domain("https://github.com/user/repo"),
            "github.com"
        );
        assert_eq!(validator.extract_domain("invalid"), "");
    }

    #[test]
    fn test_is_valid_url() {
        let validator = SourceValidator::new();

        assert!(validator.is_valid_url("https://example.com"));
        assert!(validator.is_valid_url("http://example.com"));
        assert!(!validator.is_valid_url("example.com"));
        assert!(!validator.is_valid_url(""));
    }
}
