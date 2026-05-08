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

    #[test]
    fn test_issue_code_display() {
        assert_eq!(IssueCode::MalformedUrl.to_string(), "malformed_url");
        assert_eq!(IssueCode::Unreachable.to_string(), "unreachable");
        assert_eq!(IssueCode::DeadLink.to_string(), "dead_link");
        assert_eq!(IssueCode::SuspiciousContent.to_string(), "suspicious_content");
        assert_eq!(IssueCode::Paywall.to_string(), "paywall");
        assert_eq!(IssueCode::ExpiredContent.to_string(), "expired_content");
        assert_eq!(IssueCode::InvalidSsl.to_string(), "invalid_ssl");
        assert_eq!(IssueCode::RateLimited.to_string(), "rate_limited");
        assert_eq!(IssueCode::ParseError.to_string(), "parse_error");
    }

    #[test]
    fn test_issue_severity_equality() {
        assert_eq!(IssueSeverity::Error, IssueSeverity::Error);
        assert_eq!(IssueSeverity::Warning, IssueSeverity::Warning);
        assert_eq!(IssueSeverity::Info, IssueSeverity::Info);
        assert_ne!(IssueSeverity::Error, IssueSeverity::Warning);
    }

    #[test]
    fn test_issue_code_equality() {
        assert_eq!(IssueCode::MalformedUrl, IssueCode::MalformedUrl);
        assert_ne!(IssueCode::MalformedUrl, IssueCode::Unreachable);
    }

    #[test]
    fn test_validation_issue_construction() {
        let issue = ValidationIssue {
            severity: IssueSeverity::Error,
            code: IssueCode::MalformedUrl,
            message: "URL is malformed".to_string(),
        };
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(issue.code, IssueCode::MalformedUrl);
        assert_eq!(issue.message, "URL is malformed");
    }

    #[test]
    fn test_source_validation_result_construction() {
        let result = SourceValidationResult {
            url: "https://example.com".to_string(),
            is_valid: true,
            issues: vec![],
            score: 0.9,
            warnings: vec!["Known source: example.com".to_string()],
        };
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
        assert_eq!(result.score, 0.9);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        assert!(config.check_ssl);
        assert!(!config.check_accessibility);
        assert_eq!(config.max_age_days, Some(365));
        assert_eq!(config.allowed_content_types.len(), 3);
        assert!(config.allowed_content_types.contains(&"text/html".to_string()));
        assert!(config.allowed_content_types.contains(&"application/pdf".to_string()));
        assert!(config.allowed_content_types.contains(&"text/plain".to_string()));
    }

    #[test]
    fn test_source_validator_new() {
        let validator = SourceValidator::new();
        assert!(validator.known_domains.contains_key("arxiv.org"));
        assert!(validator.known_domains.contains_key("github.com"));
        assert!(validator.known_domains.contains_key("wikipedia.org"));
        assert!(validator.known_domains.contains_key("docs.rs"));
    }

    #[test]
    fn test_source_validator_default() {
        let validator = SourceValidator::default();
        assert!(validator.known_domains.contains_key("arxiv.org"));
    }

    #[test]
    fn test_source_validator_with_config() {
        let config = ValidatorConfig {
            check_ssl: false,
            check_accessibility: true,
            max_age_days: None,
            allowed_content_types: vec!["text/html".to_string()],
        };
        let validator = SourceValidator::new().with_config(config);
        assert!(!validator.config.check_ssl);
        assert!(validator.config.check_accessibility);
        assert!(validator.config.max_age_days.is_none());
    }

    #[test]
    fn test_add_known_domain() {
        let mut validator = SourceValidator::new();
        let domain_info = DomainInfo {
            domain: "custom.org".to_string(),
            source_type: SourceType::Academic,
            credibility_weight: 0.88,
            is_paywalled: true,
            notes: "Custom academic source".to_string(),
        };
        validator.add_known_domain(domain_info);
        assert!(validator.known_domains.contains_key("custom.org"));
        let info = validator.known_domains.get("custom.org").unwrap();
        assert_eq!(info.source_type, SourceType::Academic);
        assert!(info.is_paywalled);
        assert_eq!(info.credibility_weight, 0.88);
    }

    #[test]
    fn test_add_known_domain_overwrites_existing() {
        let mut validator = SourceValidator::new();
        let original = validator.known_domains.get("arxiv.org").unwrap().clone();
        assert!(!original.is_paywalled);

        let updated = DomainInfo {
            domain: "arxiv.org".to_string(),
            source_type: SourceType::Academic,
            credibility_weight: 0.99,
            is_paywalled: true,
            notes: "Updated".to_string(),
        };
        validator.add_known_domain(updated);
        let info = validator.known_domains.get("arxiv.org").unwrap();
        assert!(info.is_paywalled);
        assert_eq!(info.credibility_weight, 0.99);
    }

    #[tokio::test]
    async fn test_validate_url_known_domain() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("https://arxiv.org/abs/2103.00001").await;
        assert!(result.is_valid);
        assert!(result.score > 0.0);
    }

    #[tokio::test]
    async fn test_validate_url_malformed() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("not-a-valid-url").await;
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|i| i.code == IssueCode::MalformedUrl));
        assert!(result.issues.iter().any(|i| i.severity == IssueSeverity::Error));
    }

    #[tokio::test]
    async fn test_validate_url_empty() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("").await;
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_validate_url_no_scheme() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("example.com/page").await;
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_validate_url_paywalled_domain() {
        let mut validator = SourceValidator::new();
        validator.add_known_domain(DomainInfo {
            domain: "paywalled.com".to_string(),
            source_type: SourceType::Web,
            credibility_weight: 0.5,
            is_paywalled: true,
            notes: "Paywalled content".to_string(),
        });
        let result = validator.validate_url("https://paywalled.com/article").await;
        assert!(result.warnings.iter().any(|w| w.contains("paywall")));
        assert!(result.score < 1.0);
    }

    #[tokio::test]
    async fn test_validate_url_known_domain_warning() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("https://github.com/user/repo").await;
        assert!(result.warnings.iter().any(|w| w.contains("Known source")));
    }

    #[tokio::test]
    async fn test_validate_url_unknown_valid() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("https://some-unknown-site.com/page").await;
        assert!(result.is_valid);
        assert_eq!(result.score, 1.0);
    }

    #[tokio::test]
    async fn test_validate_url_score_never_below_zero() {
        let validator = SourceValidator::new();
        let result = validator.validate_url("bad-url").await;
        assert!(result.score >= 0.0);
    }

    #[tokio::test]
    async fn test_validate_content_empty_text() {
        let validator = SourceValidator::new();
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "".to_string(),
        );
        let result = validator.validate_content(&content).await;
        assert!(result.issues.iter().any(|i| i.code == IssueCode::ParseError));
        assert!(result.score < 1.0);
    }

    #[tokio::test]
    async fn test_validate_content_short_text() {
        let validator = SourceValidator::new();
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "Short".to_string(),
        );
        let result = validator.validate_content(&content).await;
        assert!(result.warnings.iter().any(|w| w.contains("very short")));
        assert!(result.score < 1.0);
    }

    #[tokio::test]
    async fn test_validate_content_suspicious_patterns() {
        let validator = SourceValidator::new();
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "Click here to buy now. Subscribe for limited time offer!".to_string(),
        );
        let result = validator.validate_content(&content).await;
        assert!(result.warnings.iter().any(|w| w.contains("click here")));
        assert!(result.warnings.iter().any(|w| w.contains("buy now")));
        assert!(result.warnings.iter().any(|w| w.contains("subscribe")));
        assert!(result.warnings.iter().any(|w| w.contains("limited time")));
    }

    #[tokio::test]
    async fn test_validate_content_known_domain_credibility() {
        let validator = SourceValidator::new();
        let content = ExtractedContent::new(
            "https://arxiv.org/abs/2103.00001".to_string(),
            "Paper Title".to_string(),
            "This is a substantial academic paper about Rust programming language and its type system safety guarantees.".to_string(),
        );
        let result = validator.validate_content(&content).await;
        assert!(result.score < 1.0);
        assert!(result.score > 0.0);
    }

    #[tokio::test]
    async fn test_validate_content_good_content() {
        let validator = SourceValidator::new();
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "This is a well-written article with substantial content that provides useful information about the topic being discussed.".to_string(),
        );
        let result = validator.validate_content(&content).await;
        assert!(result.is_valid);
        assert!(result.issues.is_empty() || result.issues.iter().all(|i| i.severity != IssueSeverity::Error));
    }

    #[test]
    fn test_is_valid_url_valid_https() {
        let validator = SourceValidator::new();
        assert!(validator.is_valid_url("https://example.com"));
    }

    #[test]
    fn test_is_valid_url_valid_http() {
        let validator = SourceValidator::new();
        assert!(validator.is_valid_url("http://example.com"));
    }

    #[test]
    fn test_is_valid_url_no_scheme() {
        let validator = SourceValidator::new();
        assert!(!validator.is_valid_url("example.com"));
    }

    #[test]
    fn test_is_valid_url_empty() {
        let validator = SourceValidator::new();
        assert!(!validator.is_valid_url(""));
    }

    #[test]
    fn test_is_valid_url_with_path() {
        let validator = SourceValidator::new();
        assert!(validator.is_valid_url("https://example.com/path/to/page?query=1"));
    }

    #[test]
    fn test_extract_domain_valid_url() {
        let validator = SourceValidator::new();
        assert_eq!(validator.extract_domain("https://arxiv.org/abs/2103.00001"), "arxiv.org");
        assert_eq!(validator.extract_domain("https://github.com/user/repo"), "github.com");
    }

    #[test]
    fn test_extract_domain_invalid_url() {
        let validator = SourceValidator::new();
        assert_eq!(validator.extract_domain("invalid"), "");
    }

    #[test]
    fn test_extract_domain_with_subdomain() {
        let validator = SourceValidator::new();
        assert_eq!(validator.extract_domain("https://en.wikipedia.org/wiki/Rust"), "en.wikipedia.org");
    }

    #[test]
    fn test_get_domain_info_known() {
        let validator = SourceValidator::new();
        let info = validator.get_domain_info("https://arxiv.org/abs/2103.00001");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.source_type, SourceType::Academic);
        assert!(!info.is_paywalled);
    }

    #[test]
    fn test_get_domain_info_unknown() {
        let validator = SourceValidator::new();
        let info = validator.get_domain_info("https://unknown-site.com/page");
        assert!(info.is_none());
    }

    #[test]
    fn test_get_source_type_from_domain_known() {
        let validator = SourceValidator::new();
        assert_eq!(validator.get_source_type_from_domain("https://arxiv.org/abs/2103"), Some(SourceType::Academic));
        assert_eq!(validator.get_source_type_from_domain("https://github.com/user/repo"), Some(SourceType::GitHub));
        assert_eq!(validator.get_source_type_from_domain("https://en.wikipedia.org/wiki/Rust"), Some(SourceType::Wikipedia));
        assert_eq!(validator.get_source_type_from_domain("https://docs.rs/crate/test"), Some(SourceType::Documentation));
    }

    #[test]
    fn test_get_source_type_from_domain_unknown() {
        let validator = SourceValidator::new();
        assert_eq!(validator.get_source_type_from_domain("https://unknown.com/page"), None);
    }

    #[tokio::test]
    async fn test_validate_batch() {
        let validator = SourceValidator::new();
        let urls = vec![
            "https://arxiv.org/abs/2103.00001".to_string(),
            "not-valid".to_string(),
            "https://github.com/user/repo".to_string(),
        ];
        let results = validator.validate_batch(&urls);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_valid);
        assert!(!results[1].is_valid);
        assert!(results[2].is_valid);
    }

    #[test]
    fn test_source_filter_new() {
        let filter = SourceFilter::new();
        assert_eq!(filter.min_score, 0.3);
        assert!(filter.allowed_types.is_empty());
        assert!(filter.blocked_domains.is_empty());
    }

    #[test]
    fn test_source_filter_default() {
        let filter = SourceFilter::default();
        assert_eq!(filter.min_score, 0.3);
    }

    #[test]
    fn test_source_filter_min_score() {
        let filter = SourceFilter::new().min_score(0.5);
        assert_eq!(filter.min_score, 0.5);
    }

    #[test]
    fn test_source_filter_allowed_types_builder() {
        let filter = SourceFilter::new().allowed_types(vec![SourceType::Academic, SourceType::Wikipedia]);
        assert_eq!(filter.allowed_types.len(), 2);
    }

    #[test]
    fn test_source_filter_block_domain() {
        let filter = SourceFilter::new().block_domain("spam.com").block_domain("ads.com");
        assert_eq!(filter.blocked_domains.len(), 2);
        assert!(filter.blocked_domains.contains(&"spam.com".to_string()));
        assert!(filter.blocked_domains.contains(&"ads.com".to_string()));
    }

    #[tokio::test]
    async fn test_source_filter_by_score() {
        let filter = SourceFilter::new().min_score(0.8);
        let validator = SourceValidator::new();
        let high_score_result = validator.validate_url("https://arxiv.org/abs/2103").await;
        let low_score_result = validator.validate_url("bad-url").await;

        let input = vec![
            ("https://arxiv.org/abs/2103".to_string(), high_score_result),
            ("bad-url".to_string(), low_score_result),
        ];
        let filtered = filter.filter(input);
        assert!(filtered.contains(&"https://arxiv.org/abs/2103".to_string()));
        assert!(!filtered.contains(&"bad-url".to_string()));
    }

    #[tokio::test]
    async fn test_source_filter_blocked_domain() {
        let filter = SourceFilter::new().block_domain("arxiv.org");
        let validator = SourceValidator::new();
        let result = validator.validate_url("https://arxiv.org/abs/2103").await;

        let input = vec![("https://arxiv.org/abs/2103".to_string(), result)];
        let filtered = filter.filter(input);
        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn test_source_filter_allowed_types() {
        let filter = SourceFilter::new().allowed_types(vec![SourceType::Academic]);
        let validator = SourceValidator::new();
        let academic_result = validator.validate_url("https://arxiv.org/abs/2103").await;
        let github_result = validator.validate_url("https://github.com/user/repo").await;

        let input = vec![
            ("https://arxiv.org/abs/2103".to_string(), academic_result),
            ("https://github.com/user/repo".to_string(), github_result),
        ];
        let filtered = filter.filter(input);
        assert!(filtered.contains(&"https://arxiv.org/abs/2103".to_string()));
        assert!(!filtered.contains(&"https://github.com/user/repo".to_string()));
    }

    #[tokio::test]
    async fn test_source_filter_no_restrictions() {
        let filter = SourceFilter::new();
        let validator = SourceValidator::new();
        let result = validator.validate_url("https://arxiv.org/abs/2103").await;

        let input = vec![("https://arxiv.org/abs/2103".to_string(), result)];
        let filtered = filter.filter(input);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_domain_info_serialization() {
        let info = DomainInfo {
            domain: "test.org".to_string(),
            source_type: SourceType::Academic,
            credibility_weight: 0.85,
            is_paywalled: false,
            notes: "Test domain".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DomainInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain, "test.org");
        assert_eq!(deserialized.source_type, SourceType::Academic);
        assert_eq!(deserialized.credibility_weight, 0.85);
    }

    #[test]
    fn test_validator_config_serialization() {
        let config = ValidatorConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ValidatorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.check_ssl, config.check_ssl);
        assert_eq!(deserialized.check_accessibility, config.check_accessibility);
        assert_eq!(deserialized.max_age_days, config.max_age_days);
    }

    #[test]
    fn test_source_validation_result_serialization() {
        let result = SourceValidationResult {
            url: "https://example.com".to_string(),
            is_valid: true,
            issues: vec![ValidationIssue {
                severity: IssueSeverity::Warning,
                code: IssueCode::Paywall,
                message: "Paywall detected".to_string(),
            }],
            score: 0.8,
            warnings: vec!["Warning".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SourceValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, "https://example.com");
        assert!(deserialized.is_valid);
        assert_eq!(deserialized.issues.len(), 1);
    }
}
