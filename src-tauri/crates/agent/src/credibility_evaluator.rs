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
}
