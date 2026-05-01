use crate::research_state::SourceType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceCategory {
    Official,
    Media,
    Academic,
    Community,
    Personal,
    Unknown,
}

impl SourceCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceCategory::Official => "official",
            SourceCategory::Media => "media",
            SourceCategory::Academic => "academic",
            SourceCategory::Community => "community",
            SourceCategory::Personal => "personal",
            SourceCategory::Unknown => "unknown",
        }
    }

    pub fn from_source_type(source_type: SourceType) -> Self {
        match source_type {
            SourceType::Academic => SourceCategory::Academic,
            SourceType::Documentation => SourceCategory::Official,
            SourceType::GitHub => SourceCategory::Community,
            SourceType::News => SourceCategory::Media,
            SourceType::Wikipedia => SourceCategory::Community,
            SourceType::Web => SourceCategory::Unknown,
            SourceType::Blog => SourceCategory::Personal,
            SourceType::Forum => SourceCategory::Community,
            SourceType::Unknown => SourceCategory::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceClassification {
    pub url: String,
    pub source_type: SourceType,
    pub category: SourceCategory,
    pub domain: String,
    pub subdomains: Vec<String>,
    pub path_depth: usize,
    pub is_verified: bool,
    pub classification_confidence: f32,
}

pub struct SourceClassifier {
    known_domains: HashMap<String, DomainClassification>,
    patterns: Vec<DomainPattern>,
}

#[derive(Debug, Clone)]
pub struct DomainClassification {
    source_type: SourceType,
    category: SourceCategory,
    confidence: f32,
}

#[derive(Debug, Clone)]
struct DomainPattern {
    pattern: String,
    source_type: SourceType,
    category: SourceCategory,
    confidence: f32,
}

impl SourceClassifier {
    pub fn new() -> Self {
        let mut known_domains = HashMap::new();

        known_domains.insert(
            "arxiv.org".to_string(),
            DomainClassification {
                source_type: SourceType::Academic,
                category: SourceCategory::Academic,
                confidence: 1.0,
            },
        );

        known_domains.insert(
            "github.com".to_string(),
            DomainClassification {
                source_type: SourceType::GitHub,
                category: SourceCategory::Community,
                confidence: 1.0,
            },
        );

        known_domains.insert(
            "gitlab.com".to_string(),
            DomainClassification {
                source_type: SourceType::GitHub,
                category: SourceCategory::Community,
                confidence: 1.0,
            },
        );

        known_domains.insert(
            "wikipedia.org".to_string(),
            DomainClassification {
                source_type: SourceType::Wikipedia,
                category: SourceCategory::Community,
                confidence: 1.0,
            },
        );

        known_domains.insert(
            "docs.rs".to_string(),
            DomainClassification {
                source_type: SourceType::Documentation,
                category: SourceCategory::Official,
                confidence: 1.0,
            },
        );

        known_domains.insert(
            "rust-lang.org".to_string(),
            DomainClassification {
                source_type: SourceType::Documentation,
                category: SourceCategory::Official,
                confidence: 0.95,
            },
        );

        known_domains.insert(
            "mozilla.org".to_string(),
            DomainClassification {
                source_type: SourceType::Documentation,
                category: SourceCategory::Official,
                confidence: 0.95,
            },
        );

        known_domains.insert(
            "stackoverflow.com".to_string(),
            DomainClassification {
                source_type: SourceType::Forum,
                category: SourceCategory::Community,
                confidence: 0.9,
            },
        );

        known_domains.insert(
            "reddit.com".to_string(),
            DomainClassification {
                source_type: SourceType::Forum,
                category: SourceCategory::Community,
                confidence: 0.8,
            },
        );

        known_domains.insert(
            "medium.com".to_string(),
            DomainClassification {
                source_type: SourceType::Blog,
                category: SourceCategory::Media,
                confidence: 0.7,
            },
        );

        known_domains.insert(
            "dev.to".to_string(),
            DomainClassification {
                source_type: SourceType::Blog,
                category: SourceCategory::Community,
                confidence: 0.75,
            },
        );

        let patterns = vec![
            DomainPattern {
                pattern: r"^blog\.".to_string(),
                source_type: SourceType::Blog,
                category: SourceCategory::Personal,
                confidence: 0.7,
            },
            DomainPattern {
                pattern: r"^wiki\.".to_string(),
                source_type: SourceType::Wikipedia,
                category: SourceCategory::Community,
                confidence: 0.8,
            },
            DomainPattern {
                pattern: r"^docs\.".to_string(),
                source_type: SourceType::Documentation,
                category: SourceCategory::Official,
                confidence: 0.85,
            },
            DomainPattern {
                pattern: r"^news\.".to_string(),
                source_type: SourceType::News,
                category: SourceCategory::Media,
                confidence: 0.75,
            },
            DomainPattern {
                pattern: r"\.edu$".to_string(),
                source_type: SourceType::Academic,
                category: SourceCategory::Academic,
                confidence: 0.9,
            },
            DomainPattern {
                pattern: r"\.gov$".to_string(),
                source_type: SourceType::Documentation,
                category: SourceCategory::Official,
                confidence: 0.95,
            },
        ];

        Self {
            known_domains,
            patterns,
        }
    }

    pub fn classify(&self, url: &str) -> SourceClassification {
        let parsed = url::Url::parse(url).ok();
        let domain = parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let parts: Vec<&str> = domain.split('.').collect();
        let subdomains: Vec<String> = if parts.len() > 2 {
            parts[..parts.len() - 2]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let base_domain = if parts.len() >= 2 {
            parts[parts.len() - 2..].join(".")
        } else {
            domain.clone()
        };

        let path_depth = parsed
            .as_ref()
            .and_then(|u| u.path().strip_prefix('/'))
            .map(|p| p.matches('/').count())
            .unwrap_or(0);

        if let Some(known) = self.known_domains.get(&domain) {
            return SourceClassification {
                url: url.to_string(),
                source_type: known.source_type,
                category: known.category,
                domain: domain.clone(),
                subdomains,
                path_depth,
                is_verified: true,
                classification_confidence: known.confidence,
            };
        }

        if let Some(known) = self.known_domains.get(&base_domain) {
            return SourceClassification {
                url: url.to_string(),
                source_type: known.source_type,
                category: known.category,
                domain: base_domain,
                subdomains,
                path_depth,
                is_verified: true,
                classification_confidence: known.confidence,
            };
        }

        for pattern in &self.patterns {
            if let Ok(regex) = regex_lite::Regex::new(&pattern.pattern) {
                if regex.is_match(&domain) {
                    return SourceClassification {
                        url: url.to_string(),
                        source_type: pattern.source_type,
                        category: pattern.category,
                        domain,
                        subdomains,
                        path_depth,
                        is_verified: false,
                        classification_confidence: pattern.confidence,
                    };
                }
            }
        }

        let inferred_type = self.infer_source_type(&domain, &subdomains);
        let inferred_category = SourceCategory::from_source_type(inferred_type);

        SourceClassification {
            url: url.to_string(),
            source_type: inferred_type,
            category: inferred_category,
            domain,
            subdomains,
            path_depth,
            is_verified: false,
            classification_confidence: 0.5,
        }
    }

    fn infer_source_type(&self, domain: &str, subdomains: &[String]) -> SourceType {
        let domain_lower = domain.to_lowercase();

        if domain_lower.contains("github") || domain_lower.contains("gitlab") {
            return SourceType::GitHub;
        }

        if domain_lower.contains("arxiv") || domain_lower.contains("scholar") {
            return SourceType::Academic;
        }

        if domain_lower.contains("stackoverflow") || domain_lower.contains("stackexchange") {
            return SourceType::Forum;
        }

        for subdomain in subdomains {
            let s = subdomain.to_lowercase();
            match s.as_str() {
                "docs" | "documentation" => return SourceType::Documentation,
                "blog" => return SourceType::Blog,
                "wiki" => return SourceType::Wikipedia,
                "news" => return SourceType::News,
                _ => {},
            }
        }

        SourceType::Web
    }

    pub fn classify_batch(&self, urls: &[String]) -> Vec<SourceClassification> {
        urls.iter().map(|url| self.classify(url)).collect()
    }

    pub fn add_known_domain(&mut self, domain: String, classification: DomainClassification) {
        self.known_domains.insert(domain, classification);
    }

    pub fn get_category_stats(&self, classifications: &[SourceClassification]) -> CategoryStats {
        let mut stats = CategoryStats::default();

        for c in classifications {
            match c.category {
                SourceCategory::Official => stats.official += 1,
                SourceCategory::Media => stats.media += 1,
                SourceCategory::Academic => stats.academic += 1,
                SourceCategory::Community => stats.community += 1,
                SourceCategory::Personal => stats.personal += 1,
                SourceCategory::Unknown => stats.unknown += 1,
            }

            if c.is_verified {
                stats.verified += 1;
            } else {
                stats.unverified += 1;
            }

            stats.total_confidence += c.classification_confidence;
        }

        if !classifications.is_empty() {
            stats.average_confidence = stats.total_confidence / classifications.len() as f32;
        }

        stats
    }
}

impl Default for SourceClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryStats {
    pub official: usize,
    pub media: usize,
    pub academic: usize,
    pub community: usize,
    pub personal: usize,
    pub unknown: usize,
    pub verified: usize,
    pub unverified: usize,
    pub total_confidence: f32,
    pub average_confidence: f32,
}

impl CategoryStats {
    pub fn total(&self) -> usize {
        self.official + self.media + self.academic + self.community + self.personal + self.unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_known_domain() {
        let classifier = SourceClassifier::new();

        let result = classifier.classify("https://arxiv.org/abs/2103.00001");
        assert_eq!(result.source_type, SourceType::Academic);
        assert_eq!(result.category, SourceCategory::Academic);
        assert!(result.is_verified);
    }

    #[test]
    fn test_classify_github() {
        let classifier = SourceClassifier::new();

        let result = classifier.classify("https://github.com/user/repo");
        assert_eq!(result.source_type, SourceType::GitHub);
        assert_eq!(result.category, SourceCategory::Community);
        assert!(result.is_verified);
    }

    #[test]
    fn test_classify_blog_subdomain() {
        let classifier = SourceClassifier::new();

        let result = classifier.classify("https://blog.example.com/post");
        assert_eq!(result.source_type, SourceType::Blog);
        assert_eq!(result.category, SourceCategory::Personal);
    }

    #[test]
    fn test_classify_docs_subdomain() {
        let classifier = SourceClassifier::new();

        let result = classifier.classify("https://docs.example.com/guide");
        assert_eq!(result.source_type, SourceType::Documentation);
        assert_eq!(result.category, SourceCategory::Official);
    }

    #[test]
    fn test_category_stats() {
        let classifier = SourceClassifier::new();

        let urls = vec![
            "https://arxiv.org/abs/2103.00001".to_string(),
            "https://github.com/user/repo".to_string(),
            "https://blog.example.com/post".to_string(),
        ];

        let classifications = classifier.classify_batch(&urls);
        let stats = classifier.get_category_stats(&classifications);

        assert_eq!(stats.academic, 1);
        assert_eq!(stats.community, 1);
        assert_eq!(stats.personal, 1);
        assert_eq!(stats.total(), 3);
    }
}
