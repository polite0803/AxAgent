// SPDX-License-Identifier: AGPL-3.0-only

use crate::credibility_evaluator::{CredibilityAssessment, CredibilityEvaluator};
use crate::research_state::SearchResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactCheckStatus {
    Verified,
    LikelyTrue,
    Uncertain,
    LikelyFalse,
    Disproven,
    Unsupported,
}

impl FactCheckStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FactCheckStatus::Verified => "verified",
            FactCheckStatus::LikelyTrue => "likely_true",
            FactCheckStatus::Uncertain => "uncertain",
            FactCheckStatus::LikelyFalse => "likely_false",
            FactCheckStatus::Disproven => "disproven",
            FactCheckStatus::Unsupported => "unsupported",
        }
    }

    pub fn confidence_level(&self) -> f32 {
        match self {
            FactCheckStatus::Verified => 1.0,
            FactCheckStatus::LikelyTrue => 0.75,
            FactCheckStatus::Uncertain => 0.5,
            FactCheckStatus::LikelyFalse => 0.25,
            FactCheckStatus::Disproven => 0.0,
            FactCheckStatus::Unsupported => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub text: String,
    pub extracted_from: Option<String>,
    pub extracted_at: DateTime<Utc>,
}

impl Claim {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.into(),
            extracted_from: None,
            extracted_at: Utc::now(),
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.extracted_from = Some(source.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCheckResult {
    pub claim: Claim,
    pub status: FactCheckStatus,
    pub confidence: f32,
    pub supporting_sources: Vec<SourceEvidence>,
    pub contradicting_sources: Vec<SourceEvidence>,
    pub evidence_summary: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub source_url: String,
    pub source_title: String,
    pub credibility: f32,
    pub relevant_snippet: String,
    pub evidence_type: EvidenceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    Supports,
    Contradicts,
    Neutral,
}

pub trait RelevanceCalculator: Send + Sync {
    fn calculate(&self, claim: &Claim, source: &SearchResult) -> f32;
}

pub struct JaccardRelevanceCalculator;

impl JaccardRelevanceCalculator {
    pub fn new() -> Self {
        Self
    }
}

impl RelevanceCalculator for JaccardRelevanceCalculator {
    fn calculate(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let claim_words: HashSet<String> = claim
            .text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3)
            .collect();

        let source_words: HashSet<String> =
            format!("{} {}", source.title.to_lowercase(), source.snippet.to_lowercase())
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();

        if claim_words.is_empty() || source_words.is_empty() {
            return 0.0;
        }

        let intersection = claim_words.intersection(&source_words).count();
        let union = claim_words.union(&source_words).count();

        intersection as f32 / union.max(1) as f32
    }
}

impl Default for JaccardRelevanceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EnhancedRelevanceCalculator {
    jaccard_weight: f32,
    ngram_weight: f32,
    key_term_weight: f32,
    length_weight: f32,
}

impl EnhancedRelevanceCalculator {
    pub fn new() -> Self {
        Self {
            jaccard_weight: 0.3,
            ngram_weight: 0.3,
            key_term_weight: 0.3,
            length_weight: 0.1,
        }
    }

    fn extract_words(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| {
                w.chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '\'')
            })
            .collect()
    }

    fn jaccard_similarity(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let claim_words: HashSet<String> = claim
            .text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3)
            .collect();

        let source_words: HashSet<String> =
            format!("{} {}", source.title.to_lowercase(), source.snippet.to_lowercase())
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();

        if claim_words.is_empty() || source_words.is_empty() {
            return 0.0;
        }

        let intersection = claim_words.intersection(&source_words).count();
        let union = claim_words.union(&source_words).count();

        intersection as f32 / union.max(1) as f32
    }

    fn ngram_overlap(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let claim_words = self.extract_words(&claim.text);
        let source_text = format!("{} {}", source.title, source.snippet);
        let source_words = self.extract_words(&source_text);

        let claim_bigrams: HashSet<String> = claim_words.windows(2).map(|w| w.join(" ")).collect();

        let source_bigrams: HashSet<String> =
            source_words.windows(2).map(|w| w.join(" ")).collect();

        let claim_trigrams: HashSet<String> = claim_words.windows(3).map(|w| w.join(" ")).collect();

        let source_trigrams: HashSet<String> =
            source_words.windows(3).map(|w| w.join(" ")).collect();

        let bigram_score = if claim_bigrams.is_empty() || source_bigrams.is_empty() {
            0.0
        } else {
            let intersection = claim_bigrams.intersection(&source_bigrams).count();
            let union = claim_bigrams.union(&source_bigrams).count();
            intersection as f32 / union.max(1) as f32
        };

        let trigram_score = if claim_trigrams.is_empty() || source_trigrams.is_empty() {
            0.0
        } else {
            let intersection = claim_trigrams.intersection(&source_trigrams).count();
            let union = claim_trigrams.union(&source_trigrams).count();
            intersection as f32 / union.max(1) as f32
        };

        bigram_score * 0.6 + trigram_score * 0.4
    }

    fn key_term_overlap(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let claim_terms = self.extract_key_terms(&claim.text);
        let source_text = format!("{} {}", source.title, source.snippet);
        let source_terms = self.extract_key_terms(&source_text);

        if claim_terms.is_empty() || source_terms.is_empty() {
            return 0.0;
        }

        let intersection = claim_terms.intersection(&source_terms).count();
        let union = claim_terms.union(&source_terms).count();

        intersection as f32 / union.max(1) as f32
    }

    fn extract_key_terms(&self, text: &str) -> HashSet<String> {
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with",
            "at", "by", "from", "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "out", "off", "over", "under", "again", "further", "then", "once",
            "and", "but", "or", "nor", "not", "so", "yet", "both", "either", "neither", "each",
            "every", "all", "any", "few", "more", "most", "other", "some", "such", "no", "only",
            "own", "same", "than", "too", "very", "just", "because", "if", "when", "where", "how",
            "what", "which", "who", "whom", "this", "that", "these", "those", "it", "its", "he",
            "she", "they", "them", "his", "her", "their", "we", "us", "our", "you", "your", "i",
            "me", "my",
        ]
        .into_iter()
        .collect();

        text.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3 && !stop_words.contains(w.as_str()))
            .collect()
    }

    fn length_normalized_score(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let claim_len = claim.text.split_whitespace().count();
        let source_text = format!("{} {}", source.title, source.snippet);
        let source_len = source_text.split_whitespace().count();

        if claim_len == 0 || source_len == 0 {
            return 0.0;
        }

        (claim_len.min(source_len) as f32) / (claim_len.max(source_len) as f32)
    }
}

impl RelevanceCalculator for EnhancedRelevanceCalculator {
    fn calculate(&self, claim: &Claim, source: &SearchResult) -> f32 {
        let jaccard = self.jaccard_similarity(claim, source);
        let ngram = self.ngram_overlap(claim, source);
        let key_term = self.key_term_overlap(claim, source);
        let length = self.length_normalized_score(claim, source);

        (jaccard * self.jaccard_weight
            + ngram * self.ngram_weight
            + key_term * self.key_term_weight
            + length * self.length_weight)
            .clamp(0.0, 1.0)
    }
}

impl Default for EnhancedRelevanceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

type EmbeddingFn = Arc<dyn Fn(&str) -> Option<Vec<f32>> + Send + Sync>;

pub struct EmbeddingRelevanceCalculator {
    embedding_fn: Option<EmbeddingFn>,
    fallback: EnhancedRelevanceCalculator,
    embedding_weight: f32,
    text_weight: f32,
}

impl EmbeddingRelevanceCalculator {
    pub fn new() -> Self {
        Self {
            embedding_fn: None,
            fallback: EnhancedRelevanceCalculator::new(),
            embedding_weight: 0.6,
            text_weight: 0.4,
        }
    }

    pub fn with_embedding_fn(mut self, fn_impl: EmbeddingFn) -> Self {
        self.embedding_fn = Some(fn_impl);
        self
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
    }
}

impl RelevanceCalculator for EmbeddingRelevanceCalculator {
    fn calculate(&self, claim: &Claim, source: &SearchResult) -> f32 {
        if let Some(ref embed_fn) = self.embedding_fn {
            let claim_embedding = embed_fn(&claim.text);
            let source_text = format!("{} {}", source.title, source.snippet);
            let source_embedding = embed_fn(&source_text);

            match (claim_embedding, source_embedding) {
                (Some(ce), Some(se)) => {
                    let cosine = Self::cosine_similarity(&ce, &se);
                    let text_score = self.fallback.calculate(claim, source);
                    (cosine * self.embedding_weight + text_score * self.text_weight).clamp(0.0, 1.0)
                },
                _ => self.fallback.calculate(claim, source),
            }
        } else {
            self.fallback.calculate(claim, source)
        }
    }
}

impl Default for EmbeddingRelevanceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FactChecker {
    evaluator: CredibilityEvaluator,
    min_credibility_threshold: f32,
    min_evidence_count: usize,
    relevance_calculator: Box<dyn RelevanceCalculator>,
}

impl FactChecker {
    pub fn new() -> Self {
        Self {
            evaluator: CredibilityEvaluator::new(),
            min_credibility_threshold: 0.5,
            min_evidence_count: 1,
            relevance_calculator: Box::new(EnhancedRelevanceCalculator::new()),
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.min_credibility_threshold = threshold;
        self
    }

    pub fn with_min_evidence(mut self, count: usize) -> Self {
        self.min_evidence_count = count;
        self
    }

    pub fn with_relevance_calculator(mut self, calculator: Box<dyn RelevanceCalculator>) -> Self {
        self.relevance_calculator = calculator;
        self
    }

    pub async fn check_claim(&self, claim: &Claim, sources: &[SearchResult]) -> FactCheckResult {
        let futures: Vec<_> = sources.iter().map(|s| self.evaluator.evaluate(s)).collect();
        let assessments: Vec<CredibilityAssessment> = futures::future::join_all(futures).await;

        let mut supporting: Vec<SourceEvidence> = Vec::new();
        let contradicting: Vec<SourceEvidence> = Vec::new();

        for (result, assessment) in sources.iter().zip(assessments.iter()) {
            if assessment.credibility.overall < self.min_credibility_threshold {
                continue;
            }

            let relevance = self.calculate_relevance(claim, result);

            if relevance > 0.3 {
                let evidence = SourceEvidence {
                    source_url: result.url.clone(),
                    source_title: result.title.clone(),
                    credibility: assessment.credibility.overall,
                    relevant_snippet: result.snippet.clone(),
                    evidence_type: if relevance > 0.6 {
                        EvidenceType::Supports
                    } else {
                        EvidenceType::Neutral
                    },
                };

                if relevance > 0.6 {
                    supporting.push(evidence);
                }
            }
        }

        let status = self.determine_status(&supporting, &contradicting);
        let confidence = self.calculate_confidence(&supporting, &contradicting);

        let evidence_summary = self.generate_evidence_summary(&supporting, &contradicting, &status);

        FactCheckResult {
            claim: claim.clone(),
            status,
            confidence,
            supporting_sources: supporting,
            contradicting_sources: contradicting,
            evidence_summary,
            checked_at: Utc::now(),
        }
    }

    fn calculate_relevance(&self, claim: &Claim, source: &SearchResult) -> f32 {
        self.relevance_calculator.calculate(claim, source)
    }

    fn determine_status(
        &self,
        supporting: &[SourceEvidence],
        contradicting: &[SourceEvidence],
    ) -> FactCheckStatus {
        let supporting_cred: f32 =
            supporting.iter().map(|e| e.credibility).sum::<f32>() / supporting.len().max(1) as f32;

        let contradicting_cred: f32 = contradicting.iter().map(|e| e.credibility).sum::<f32>()
            / contradicting.len().max(1) as f32;

        if supporting.len() >= 2 && supporting_cred > 0.7 {
            FactCheckStatus::Verified
        } else if !supporting.is_empty() && supporting_cred > 0.6 {
            FactCheckStatus::LikelyTrue
        } else if !contradicting.is_empty() && contradicting_cred > 0.6 {
            FactCheckStatus::LikelyFalse
        } else if supporting.is_empty() && contradicting.is_empty() {
            FactCheckStatus::Unsupported
        } else {
            FactCheckStatus::Uncertain
        }
    }

    fn calculate_confidence(
        &self,
        supporting: &[SourceEvidence],
        contradicting: &[SourceEvidence],
    ) -> f32 {
        let evidence_count = supporting.len() + contradicting.len();

        if evidence_count == 0 {
            return 0.0;
        }

        let supporting_weight: f32 = supporting
            .iter()
            .map(|e| {
                e.credibility
                    * if e.evidence_type == EvidenceType::Supports {
                        1.0
                    } else {
                        0.5
                    }
            })
            .sum();

        let contradicting_weight: f32 = contradicting
            .iter()
            .map(|e| {
                e.credibility
                    * if e.evidence_type == EvidenceType::Contradicts {
                        1.0
                    } else {
                        0.5
                    }
            })
            .sum();

        let total_weight = supporting_weight + contradicting_weight;

        if total_weight == 0.0 {
            return 0.5;
        }

        let net_confidence = (supporting_weight - contradicting_weight) / total_weight;
        (net_confidence + 1.0) / 2.0
    }

    fn generate_evidence_summary(
        &self,
        supporting: &[SourceEvidence],
        contradicting: &[SourceEvidence],
        status: &FactCheckStatus,
    ) -> String {
        match status {
            FactCheckStatus::Verified => {
                format!("Verified by {} high-credibility source(s)", supporting.len())
            },
            FactCheckStatus::LikelyTrue => format!(
                "Supported by {} source(s) with average credibility {:.2}",
                supporting.len(),
                supporting.iter().map(|e| e.credibility).sum::<f32>()
                    / supporting.len().max(1) as f32
            ),
            FactCheckStatus::LikelyFalse => format!(
                "Contradicted by {} source(s) with average credibility {:.2}",
                contradicting.len(),
                contradicting.iter().map(|e| e.credibility).sum::<f32>()
                    / contradicting.len().max(1) as f32
            ),
            FactCheckStatus::Uncertain => format!(
                "Mixed evidence from {} supporting and {} contradicting source(s)",
                supporting.len(),
                contradicting.len()
            ),
            FactCheckStatus::Unsupported => {
                "No relevant sources found to verify this claim".to_string()
            },
            FactCheckStatus::Disproven => format!(
                "Disproven by {} high-credibility contradicting source(s)",
                contradicting.len()
            ),
        }
    }

    pub async fn check_batch(
        &self,
        claims: &[Claim],
        sources: &[SearchResult],
    ) -> Vec<FactCheckResult> {
        let mut results = Vec::new();

        for claim in claims {
            let result = self.check_claim(claim, sources).await;
            results.push(result);
        }

        results
    }
}

impl Default for FactChecker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ClaimExtractor {
    min_claim_length: usize,
    max_claims_per_source: usize,
}

impl ClaimExtractor {
    pub fn new() -> Self {
        Self {
            min_claim_length: 20,
            max_claims_per_source: 10,
        }
    }

    pub fn with_config(mut self, min_length: usize, max_per_source: usize) -> Self {
        self.min_claim_length = min_length;
        self.max_claims_per_source = max_per_source;
        self
    }

    pub fn extract_from_text(&self, text: &str, source_url: Option<&str>) -> Vec<Claim> {
        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| s.len() >= self.min_claim_length)
            .take(self.max_claims_per_source)
            .collect();

        sentences
            .into_iter()
            .map(|s| {
                let mut claim = Claim::new(s);
                if let Some(url) = source_url {
                    claim = claim.with_source(url);
                }
                claim
            })
            .collect()
    }

    pub fn extract_from_results(&self, results: &[SearchResult]) -> Vec<Claim> {
        let mut claims = Vec::new();

        for result in results {
            let extracted = self.extract_from_text(&result.snippet, Some(&result.url));
            claims.extend(extracted);
        }

        claims
    }
}

impl Default for ClaimExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_claim_with_supporting_source() {
        let checker = FactChecker::new();

        let claim = Claim::new("Rust is a systems programming language");

        let sources = vec![SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        )];

        let result = checker.check_claim(&claim, &sources).await;

        assert!(
            result.status == FactCheckStatus::LikelyTrue
                || result.status == FactCheckStatus::Verified
                || result.status == FactCheckStatus::Uncertain
                || result.status == FactCheckStatus::Unsupported,
            "expected LikelyTrue/Verified/Uncertain/Unsupported, got {:?}",
            result.status
        );
    }

    #[tokio::test]
    async fn test_check_claim_no_sources() {
        let checker = FactChecker::new();

        let claim = Claim::new("This is a test claim that should be unsupported");

        let sources: Vec<SearchResult> = vec![];

        let result = checker.check_claim(&claim, &sources).await;

        assert_eq!(result.status, FactCheckStatus::Unsupported);
    }

    #[test]
    fn test_claim_extraction() {
        let extractor = ClaimExtractor::new();

        let text = "Rust is a systems programming language. It focuses on safety and concurrency. This is a very short claim.";

        let claims = extractor.extract_from_text(text, None);

        assert!(claims.len() >= 2);
        assert!(claims.iter().all(|c| c.text.len() >= 20));
    }

    #[test]
    fn test_claim_with_source() {
        let claim = Claim::new("Test claim").with_source("https://example.com");

        assert_eq!(claim.extracted_from, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_jaccard_relevance_calculator() {
        let calc = JaccardRelevanceCalculator::new();
        let claim = Claim::new("Rust is a systems programming language");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let score = calc.calculate(&claim, &source);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_enhanced_relevance_calculator() {
        let calc = EnhancedRelevanceCalculator::new();
        let claim = Claim::new("Rust is a systems programming language");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let score = calc.calculate(&claim, &source);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_enhanced_relevance_calculator_unrelated() {
        let calc = EnhancedRelevanceCalculator::new();
        let claim = Claim::new("The Eiffel Tower is located in Paris France");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let score = calc.calculate(&claim, &source);
        assert!(score < 0.5);
    }

    #[test]
    fn test_embedding_relevance_calculator_fallback() {
        let calc = EmbeddingRelevanceCalculator::new();
        let claim = Claim::new("Rust is a systems programming language");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let score = calc.calculate(&claim, &source);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_embedding_relevance_calculator_with_fn() {
        let embedding_fn = Arc::new(|text: &str| -> Option<Vec<f32>> {
            if text.contains("Rust") {
                Some(vec![0.1, 0.9, 0.3])
            } else {
                None
            }
        });

        let calc = EmbeddingRelevanceCalculator::new().with_embedding_fn(embedding_fn);
        let claim = Claim::new("Rust is a systems programming language");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let score = calc.calculate(&claim, &source);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_fact_checker_with_custom_calculator() {
        let checker = FactChecker::new()
            .with_relevance_calculator(Box::new(JaccardRelevanceCalculator::new()));

        let claim = Claim::new("Rust is a systems programming language");
        let source = SearchResult::new(
            crate::research_state::SourceType::Documentation,
            "https://doc.rust-lang.org/".to_string(),
            "The Rust Programming Language".to_string(),
            "Rust is a systems programming language focused on safety, speed, and concurrency."
                .to_string(),
        );

        let calc = JaccardRelevanceCalculator::new();
        let expected = calc.calculate(&claim, &source);

        let actual = checker.calculate_relevance(&claim, &source);
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = EmbeddingRelevanceCalculator::cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![1.0, 0.0, 0.0];
        let sim2 = EmbeddingRelevanceCalculator::cosine_similarity(&c, &d);
        assert!((sim2 - 1.0).abs() < 1e-6);
    }
}
