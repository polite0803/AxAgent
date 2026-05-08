use crate::research_state::{SearchResult, SourceType};
use crate::source_validator::SourceValidationResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CredibilityScore {
    pub overall: f32,
    pub authority: f32,
    pub consistency: f32,
    pub recency: f32,
    pub objectivity: f32,
}

impl CredibilityScore {
    pub fn new(authority: f32, consistency: f32, recency: f32, objectivity: f32) -> Self {
        let overall = Self::weighted_score(authority, consistency, recency, objectivity);
        Self {
            overall,
            authority,
            consistency,
            recency,
            objectivity,
        }
    }

    fn weighted_score(authority: f32, consistency: f32, recency: f32, objectivity: f32) -> f32 {
        authority * 0.30 + consistency * 0.25 + recency * 0.20 + objectivity * 0.25
    }

    pub fn min() -> Self {
        Self {
            overall: 0.0,
            authority: 0.0,
            consistency: 0.0,
            recency: 0.0,
            objectivity: 0.0,
        }
    }

    pub fn max() -> Self {
        Self {
            overall: 1.0,
            authority: 1.0,
            consistency: 1.0,
            recency: 1.0,
            objectivity: 1.0,
        }
    }

    pub fn is_high(&self) -> bool {
        self.overall >= 0.7
    }

    pub fn is_medium(&self) -> bool {
        self.overall >= 0.4 && self.overall < 0.7
    }

    pub fn is_low(&self) -> bool {
        self.overall < 0.4
    }
}

impl Default for CredibilityScore {
    fn default() -> Self {
        Self::min()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredibilityAssessment {
    pub source_url: String,
    pub source_title: String,
    pub source_type: SourceType,
    pub credibility: CredibilityScore,
    pub validation_result: Option<SourceValidationResult>,
    pub assessed_at: DateTime<Utc>,
    pub factors: Vec<CredibilityFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredibilityFactor {
    pub dimension: FactorDimension,
    pub score: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactorDimension {
    Authority,
    Consistency,
    Recency,
    Objectivity,
}

pub struct CredibilityEvaluator {
    source_weights: HashMap<SourceType, AuthorityWeight>,
    recency_threshold_days: i64,
}

#[derive(Debug, Clone, Copy)]
struct AuthorityWeight {
    base: f32,
    official: f32,
    media: f32,
    personal: f32,
}

impl Default for AuthorityWeight {
    fn default() -> Self {
        Self {
            base: 0.5,
            official: 0.95,
            media: 0.75,
            personal: 0.40,
        }
    }
}

impl CredibilityEvaluator {
    pub fn new() -> Self {
        let mut source_weights = HashMap::new();

        source_weights.insert(
            SourceType::Academic,
            AuthorityWeight {
                base: 0.9,
                official: 0.95,
                media: 0.85,
                personal: 0.7,
            },
        );

        source_weights.insert(
            SourceType::Documentation,
            AuthorityWeight {
                base: 0.85,
                official: 0.9,
                media: 0.8,
                personal: 0.75,
            },
        );

        source_weights.insert(
            SourceType::GitHub,
            AuthorityWeight {
                base: 0.8,
                official: 0.85,
                media: 0.75,
                personal: 0.7,
            },
        );

        source_weights.insert(
            SourceType::News,
            AuthorityWeight {
                base: 0.7,
                official: 0.85,
                media: 0.75,
                personal: 0.5,
            },
        );

        source_weights.insert(
            SourceType::Wikipedia,
            AuthorityWeight {
                base: 0.65,
                official: 0.75,
                media: 0.65,
                personal: 0.55,
            },
        );

        source_weights.insert(
            SourceType::Web,
            AuthorityWeight {
                base: 0.5,
                official: 0.8,
                media: 0.6,
                personal: 0.35,
            },
        );

        source_weights.insert(
            SourceType::Blog,
            AuthorityWeight {
                base: 0.4,
                official: 0.7,
                media: 0.5,
                personal: 0.3,
            },
        );

        source_weights.insert(
            SourceType::Forum,
            AuthorityWeight {
                base: 0.35,
                official: 0.6,
                media: 0.45,
                personal: 0.25,
            },
        );

        Self {
            source_weights,
            recency_threshold_days: 365,
        }
    }

    pub fn with_recency_threshold(mut self, days: i64) -> Self {
        self.recency_threshold_days = days;
        self
    }

    pub async fn evaluate(&self, result: &SearchResult) -> CredibilityAssessment {
        let authority = self.evaluate_authority(result).await;
        let consistency = self.evaluate_consistency(result).await;
        let recency = self.evaluate_recency(result);
        let objectivity = self.evaluate_objectivity(result).await;

        let credibility = CredibilityScore::new(
            authority.score,
            consistency.score,
            recency.score,
            objectivity.score,
        );

        let factors = vec![
            CredibilityFactor {
                dimension: authority.dimension,
                score: authority.score,
                reasoning: authority.reasoning,
            },
            CredibilityFactor {
                dimension: consistency.dimension,
                score: consistency.score,
                reasoning: consistency.reasoning,
            },
            CredibilityFactor {
                dimension: recency.dimension,
                score: recency.score,
                reasoning: recency.reasoning,
            },
            CredibilityFactor {
                dimension: objectivity.dimension,
                score: objectivity.score,
                reasoning: objectivity.reasoning,
            },
        ];

        CredibilityAssessment {
            source_url: result.url.clone(),
            source_title: result.title.clone(),
            source_type: result.source_type,
            credibility,
            validation_result: None,
            assessed_at: Utc::now(),
            factors,
        }
    }

    pub async fn evaluate_with_validation(
        &self,
        result: &SearchResult,
        validation: SourceValidationResult,
    ) -> CredibilityAssessment {
        let mut assessment = self.evaluate(result).await;
        assessment.validation_result = Some(validation);

        if let Some(score) = assessment.validation_result.as_ref().map(|v| v.score) {
            let mut adjusted = assessment.credibility;
            adjusted.overall = adjusted.overall * 0.7 + score * 0.3;
            adjusted.authority = adjusted.authority * 0.8 + score * 0.2;
            assessment.credibility = adjusted;
        }

        assessment
    }

    async fn evaluate_authority(&self, result: &SearchResult) -> FactorResult {
        let weight = self
            .source_weights
            .get(&result.source_type)
            .copied()
            .unwrap_or(AuthorityWeight::default());

        let base_score = weight.base;
        let is_media_source = matches!(result.source_type, SourceType::News | SourceType::Blog);

        let title_lower = result.title.to_lowercase();
        let domain_indicators: &[(&str, f32)] = if is_media_source {
            &[
                ("official", weight.official),
                ("media", weight.media),
                ("news", weight.media),
                ("docs", 0.85),
                ("guide", 0.7),
                ("wiki", 0.6),
                ("blog", weight.personal),
                ("personal", weight.personal),
            ]
        } else {
            &[
                ("official", weight.official),
                ("docs", 0.85),
                ("guide", 0.7),
                ("wiki", 0.6),
                ("blog", weight.personal),
                ("personal", weight.personal),
            ]
        };

        let mut title_boost: f32 = 0.0;
        for (indicator, boost) in domain_indicators {
            if title_lower.contains(indicator) {
                title_boost = title_boost.max(boost - base_score);
            }
        }

        let authority_score = (base_score + title_boost).min(1.0);

        FactorResult {
            dimension: FactorDimension::Authority,
            score: authority_score,
            reasoning: format!(
                "Source type '{}' with base authority {:.2}",
                format!("{:?}", result.source_type).to_lowercase(),
                base_score
            ),
        }
    }

    async fn evaluate_consistency(&self, _result: &SearchResult) -> FactorResult {
        FactorResult {
            dimension: FactorDimension::Consistency,
            score: 0.7,
            reasoning: "Cross-source verification not yet implemented".to_string(),
        }
    }

    fn evaluate_recency(&self, result: &SearchResult) -> FactorResult {
        let recency_score = match &result.published_date {
            Some(date_str) => {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    let published = date
                        .and_hms_opt(0, 0, 0)
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                    let now = Utc::now();

                    let age_days = if let Some(published) = published {
                        (now - published).num_days()
                    } else {
                        0
                    };

                    if age_days < 0 {
                        0.5
                    } else if age_days < 30 {
                        1.0
                    } else if age_days < 90 {
                        0.9
                    } else if age_days < 180 {
                        0.8
                    } else if age_days < 365 {
                        0.7
                    } else if age_days < 730 {
                        0.5
                    } else {
                        0.3
                    }
                } else {
                    0.5
                }
            },
            None => 0.5,
        };

        FactorResult {
            dimension: FactorDimension::Recency,
            score: recency_score,
            reasoning: match &result.published_date {
                Some(date) => format!("Published on {}", date),
                None => "No publication date available".to_string(),
            },
        }
    }

    async fn evaluate_objectivity(&self, result: &SearchResult) -> FactorResult {
        let snippet_lower = result.snippet.to_lowercase();

        let subjective_indicators = [
            ("i think", -0.2),
            ("i believe", -0.2),
            ("in my opinion", -0.25),
            ("amazing", -0.1),
            ("terrible", -0.1),
            ("best", -0.1),
            ("worst", -0.1),
            ("must have", 0.1),
            ("should avoid", -0.1),
        ];

        let objective_indicators = [
            ("according to", 0.1),
            ("research shows", 0.15),
            ("data indicates", 0.15),
            ("studies show", 0.15),
            ("official", 0.1),
            ("reported", 0.1),
        ];

        let mut objectivity_score: f32 = 0.7;

        for (indicator, delta) in subjective_indicators {
            if snippet_lower.contains(indicator) {
                objectivity_score += delta;
            }
        }

        for (indicator, delta) in objective_indicators {
            if snippet_lower.contains(indicator) {
                objectivity_score += delta;
            }
        }

        let objectivity = objectivity_score.clamp(0.0, 1.0);

        FactorResult {
            dimension: FactorDimension::Objectivity,
            score: objectivity,
            reasoning: if objectivity > 0.6 {
                "Content appears to be objective".to_string()
            } else {
                "Content may contain subjective language".to_string()
            },
        }
    }

    pub fn evaluate_batch(&self, results: &[SearchResult]) -> Vec<CredibilityAssessment> {
        let futures: Vec<_> = results.iter().map(|r| self.evaluate(r)).collect();
        tokio::runtime::Handle::current()
            .block_on(async { futures::future::join_all(futures).await })
    }
}

impl Default for CredibilityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct FactorResult {
    dimension: FactorDimension,
    score: f32,
    reasoning: String,
}

pub struct CredibilityRanking {
    min_score: f32,
}

impl CredibilityRanking {
    pub fn new() -> Self {
        Self { min_score: 0.0 }
    }

    pub fn min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }

    pub fn rank(&self, assessments: Vec<CredibilityAssessment>) -> Vec<CredibilityAssessment> {
        let mut filtered: Vec<_> = assessments
            .into_iter()
            .filter(|a| a.credibility.overall >= self.min_score)
            .collect();

        filtered.sort_by(|a, b| {
            b.credibility
                .overall
                .partial_cmp(&a.credibility.overall)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        filtered
    }
}

impl Default for CredibilityRanking {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluate_academic_source() {
        let evaluator = CredibilityEvaluator::new();

        let result = SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/2103.00001".to_string(),
            "A Study on Machine Learning".to_string(),
            "This paper presents a novel approach to machine learning.".to_string(),
        );

        let assessment = evaluator.evaluate(&result).await;

        assert!(assessment.credibility.overall > 0.7);
        assert_eq!(assessment.source_type, SourceType::Academic);
    }

    #[tokio::test]
    async fn test_evaluate_blog_source() {
        let evaluator = CredibilityEvaluator::new();

        let result = SearchResult::new(
            SourceType::Blog,
            "https://personal-blog.com/post".to_string(),
            "I think this is the best framework".to_string(),
            "In my opinion, this framework is amazing.".to_string(),
        );

        let assessment = evaluator.evaluate(&result).await;

        assert!(assessment.credibility.objectivity < 0.6);
    }

    #[test]
    fn test_credibility_score_classification() {
        let high = CredibilityScore::new(0.8, 0.8, 0.8, 0.8);
        assert!(high.is_high());

        let medium = CredibilityScore::new(0.5, 0.5, 0.5, 0.5);
        assert!(medium.is_medium());

        let low = CredibilityScore::new(0.3, 0.3, 0.3, 0.3);
        assert!(low.is_low());
    }

    #[test]
    fn test_credibility_score_weighted_calculation() {
        let score = CredibilityScore::new(1.0, 1.0, 1.0, 1.0);
        let expected = 1.0 * 0.30 + 1.0 * 0.25 + 1.0 * 0.20 + 1.0 * 0.25;
        assert!((score.overall - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_credibility_score_min() {
        let min = CredibilityScore::min();
        assert_eq!(min.overall, 0.0);
        assert_eq!(min.authority, 0.0);
        assert_eq!(min.consistency, 0.0);
        assert_eq!(min.recency, 0.0);
        assert_eq!(min.objectivity, 0.0);
        assert!(min.is_low());
    }

    #[test]
    fn test_credibility_score_max() {
        let max = CredibilityScore::max();
        assert_eq!(max.overall, 1.0);
        assert_eq!(max.authority, 1.0);
        assert!(max.is_high());
    }

    #[test]
    fn test_credibility_score_default() {
        let default = CredibilityScore::default();
        assert_eq!(default.overall, 0.0);
    }

    #[test]
    fn test_credibility_score_boundary_high() {
        let score = CredibilityScore::new(0.7, 0.7, 0.7, 0.7);
        assert!(score.is_high());
        assert!(!score.is_medium());
    }

    #[test]
    fn test_credibility_score_boundary_medium() {
        let score = CredibilityScore::new(0.4, 0.4, 0.4, 0.4);
        assert!(!score.is_high());
        assert!(score.is_medium());
        assert!(!score.is_low());
    }

    #[test]
    fn test_credibility_score_boundary_low() {
        let score = CredibilityScore::new(0.39, 0.39, 0.39, 0.39);
        assert!(score.is_low());
    }

    #[test]
    fn test_credibility_score_equality() {
        let a = CredibilityScore::new(0.5, 0.5, 0.5, 0.5);
        let b = CredibilityScore::new(0.5, 0.5, 0.5, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn test_credibility_score_serialization() {
        let score = CredibilityScore::new(0.8, 0.7, 0.6, 0.5);
        let json = serde_json::to_string(&score).unwrap();
        let deserialized: CredibilityScore = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, score);
    }

    #[tokio::test]
    async fn test_evaluate_github_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::GitHub,
            "https://github.com/rust-lang/rust".to_string(),
            "Rust Programming Language".to_string(),
            "The Rust programming language repository.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.authority > 0.5);
        assert_eq!(assessment.source_type, SourceType::GitHub);
    }

    #[tokio::test]
    async fn test_evaluate_news_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::News,
            "https://news.example.com/article".to_string(),
            "Official report on climate change".to_string(),
            "According to research shows data indicates the temperature is rising.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.objectivity > 0.6);
    }

    #[tokio::test]
    async fn test_evaluate_forum_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Forum,
            "https://forum.example.com/thread".to_string(),
            "Personal experience".to_string(),
            "I think this is the worst product ever.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.authority < 0.7);
    }

    #[tokio::test]
    async fn test_evaluate_with_recent_date() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/test".to_string(),
            "Recent Paper".to_string(),
            "A recent study.".to_string(),
        )
        .with_published_date("2026-01-01".to_string());
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.recency >= 0.8);
    }

    #[tokio::test]
    async fn test_evaluate_with_old_date() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/old".to_string(),
            "Old Paper".to_string(),
            "An old study.".to_string(),
        )
        .with_published_date("2020-01-01".to_string());
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.recency < 0.7);
    }

    #[tokio::test]
    async fn test_evaluate_no_date() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "No Date Article".to_string(),
            "Some content.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!((assessment.credibility.recency - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_evaluate_invalid_date() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Bad Date".to_string(),
            "Content.".to_string(),
        )
        .with_published_date("not-a-date".to_string());
        let assessment = evaluator.evaluate(&result).await;
        assert!((assessment.credibility.recency - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_evaluate_subjective_content() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Blog,
            "https://blog.example.com".to_string(),
            "My Opinion".to_string(),
            "I believe this is amazing and the best thing ever. In my opinion you should avoid everything else.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.objectivity < 0.7);
    }

    #[tokio::test]
    async fn test_evaluate_objective_content() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/test".to_string(),
            "Research Study".to_string(),
            "According to the official data, research shows that studies show the results are consistent. Reported findings confirm this.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.objectivity > 0.7);
    }

    #[tokio::test]
    async fn test_evaluate_with_validation_adjusts_score() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/test".to_string(),
            "Validated Paper".to_string(),
            "A validated study.".to_string(),
        );
        let assessment_no_val = evaluator.evaluate(&result).await;
        let validation = SourceValidationResult {
            url: result.url.clone(),
            is_valid: true,
            issues: vec![],
            score: 0.9,
            warnings: vec![],
        };
        let assessment_with_val = evaluator
            .evaluate_with_validation(&result, validation)
            .await;
        assert!(assessment_with_val.validation_result.is_some());
        assert!(assessment_with_val.credibility.overall != assessment_no_val.credibility.overall);
    }

    #[tokio::test]
    async fn test_evaluate_assessment_has_factors() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Test".to_string(),
            "Content.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert_eq!(assessment.factors.len(), 4);
    }

    #[tokio::test]
    async fn test_evaluate_assessment_factor_dimensions() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Test".to_string(),
            "Content.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        let dimensions: Vec<_> = assessment.factors.iter().map(|f| f.dimension).collect();
        assert!(dimensions.contains(&FactorDimension::Authority));
        assert!(dimensions.contains(&FactorDimension::Consistency));
        assert!(dimensions.contains(&FactorDimension::Recency));
        assert!(dimensions.contains(&FactorDimension::Objectivity));
    }

    #[test]
    fn test_evaluator_with_recency_threshold() {
        let evaluator = CredibilityEvaluator::new().with_recency_threshold(180);
        assert_eq!(evaluator.recency_threshold_days, 180);
    }

    #[test]
    fn test_evaluator_default() {
        let evaluator = CredibilityEvaluator::default();
        assert_eq!(evaluator.recency_threshold_days, 365);
    }

    #[test]
    fn test_credibility_ranking_new() {
        let ranking = CredibilityRanking::new();
        let assessments = vec![];
        let result = ranking.rank(assessments);
        assert!(result.is_empty());
    }

    #[test]
    fn test_credibility_ranking_sorts_by_score() {
        let ranking = CredibilityRanking::new();
        let assessments = vec![
            CredibilityAssessment {
                source_url: "low".to_string(),
                source_title: "Low".to_string(),
                source_type: SourceType::Blog,
                credibility: CredibilityScore::new(0.3, 0.3, 0.3, 0.3),
                validation_result: None,
                assessed_at: Utc::now(),
                factors: vec![],
            },
            CredibilityAssessment {
                source_url: "high".to_string(),
                source_title: "High".to_string(),
                source_type: SourceType::Academic,
                credibility: CredibilityScore::new(0.9, 0.9, 0.9, 0.9),
                validation_result: None,
                assessed_at: Utc::now(),
                factors: vec![],
            },
        ];
        let ranked = ranking.rank(assessments);
        assert_eq!(ranked[0].source_url, "high");
        assert_eq!(ranked[1].source_url, "low");
    }

    #[test]
    fn test_credibility_ranking_min_score_filter() {
        let ranking = CredibilityRanking::new().min_score(0.5);
        let assessments = vec![
            CredibilityAssessment {
                source_url: "low".to_string(),
                source_title: "Low".to_string(),
                source_type: SourceType::Blog,
                credibility: CredibilityScore::new(0.3, 0.3, 0.3, 0.3),
                validation_result: None,
                assessed_at: Utc::now(),
                factors: vec![],
            },
            CredibilityAssessment {
                source_url: "high".to_string(),
                source_title: "High".to_string(),
                source_type: SourceType::Academic,
                credibility: CredibilityScore::new(0.9, 0.9, 0.9, 0.9),
                validation_result: None,
                assessed_at: Utc::now(),
                factors: vec![],
            },
        ];
        let ranked = ranking.rank(assessments);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].source_url, "high");
    }

    #[test]
    fn test_credibility_ranking_default() {
        let ranking = CredibilityRanking::default();
        let result = ranking.rank(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_factor_dimension_variants() {
        let dims = [
            FactorDimension::Authority,
            FactorDimension::Consistency,
            FactorDimension::Recency,
            FactorDimension::Objectivity,
        ];
        assert_eq!(dims.len(), 4);
    }

    #[tokio::test]
    async fn test_evaluate_wikipedia_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Wikipedia,
            "https://en.wikipedia.org/wiki/Rust".to_string(),
            "Rust (programming language)".to_string(),
            "Rust is a systems programming language.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.authority > 0.5);
    }

    #[tokio::test]
    async fn test_evaluate_documentation_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Documentation,
            "https://docs.rs/tokio".to_string(),
            "Tokio Documentation".to_string(),
            "Official docs for the Tokio runtime.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.authority > 0.7);
    }

    #[tokio::test]
    async fn test_evaluate_web_source() {
        let evaluator = CredibilityEvaluator::new();
        let result = SearchResult::new(
            SourceType::Web,
            "https://random-site.com".to_string(),
            "Random Article".to_string(),
            "Some random content.".to_string(),
        );
        let assessment = evaluator.evaluate(&result).await;
        assert!(assessment.credibility.authority < 0.9);
    }

    #[test]
    fn test_credibility_factor_serialization() {
        let factor = CredibilityFactor {
            dimension: FactorDimension::Authority,
            score: 0.85,
            reasoning: "High authority source".to_string(),
        };
        let json = serde_json::to_string(&factor).unwrap();
        let deserialized: CredibilityFactor = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.dimension, FactorDimension::Authority);
        assert!((deserialized.score - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_credibility_assessment_serialization() {
        let assessment = CredibilityAssessment {
            source_url: "https://example.com".to_string(),
            source_title: "Test".to_string(),
            source_type: SourceType::Academic,
            credibility: CredibilityScore::new(0.8, 0.7, 0.6, 0.5),
            validation_result: None,
            assessed_at: Utc::now(),
            factors: vec![],
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: CredibilityAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_url, "https://example.com");
    }
}
