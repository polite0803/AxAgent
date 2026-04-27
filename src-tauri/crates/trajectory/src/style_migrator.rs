use crate::style_applier::{CodeStyleTemplate, DocumentStyleApplicator, StyleApplier};
use crate::style_extractor::{DocumentStyleProfile, ExtractedCodePatterns, StyleExtractor};
use crate::style_vectorizer::{CodeSample, MessageSample, StyleVector, StyleVectorizer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStyleProfile {
    pub id: String,
    pub user_id: String,
    pub code_style_vector: StyleVector,
    pub document_style_profile: DocumentStyleProfile,
    pub code_templates: Vec<CodeStyleTemplate>,
    pub learned_patterns: Vec<LearnedPattern>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_samples: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: String,
    pub pattern_type: LearnedPatternType,
    pub original: String,
    pub transformed: String,
    pub context: String,
    pub usage_count: u32,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearnedPatternType {
    Naming,
    Formatting,
    Comment,
    Structure,
    Document,
}

impl UserStyleProfile {
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            code_style_vector: StyleVector::default_style(),
            document_style_profile: DocumentStyleProfile {
                formality_level: 0.5,
                structure_level: 0.5,
                technical_vocabulary_ratio: 0.5,
                explanation_detail_level: 0.5,
                preferred_format: crate::style_extractor::DocumentFormat::Markdown,
            },
            code_templates: Vec::new(),
            learned_patterns: Vec::new(),
            created_at: now,
            updated_at: now,
            total_samples: 0,
            confidence: 0.0,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new("default".to_string())
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn merge(&mut self, other: &UserStyleProfile) {
        self.code_style_vector =
            self.interpolate_style_vectors(&self.code_style_vector, &other.code_style_vector, 0.5);
        self.total_samples += other.total_samples;
        self.confidence = (self.confidence + other.confidence) / 2.0;
        self.update_timestamp();
    }

    fn interpolate_style_vectors(
        &self,
        v1: &StyleVector,
        v2: &StyleVector,
        factor: f32,
    ) -> StyleVector {
        let inv_factor = 1.0 - factor;
        StyleVector::new(
            crate::style_vectorizer::StyleDimensions {
                naming_score: v1.dimensions.naming_score * inv_factor
                    + v2.dimensions.naming_score * factor,
                density_score: v1.dimensions.density_score * inv_factor
                    + v2.dimensions.density_score * factor,
                comment_ratio: v1.dimensions.comment_ratio * inv_factor
                    + v2.dimensions.comment_ratio * factor,
                abstraction_level: v1.dimensions.abstraction_level * inv_factor
                    + v2.dimensions.abstraction_level * factor,
                formality_score: v1.dimensions.formality_score * inv_factor
                    + v2.dimensions.formality_score * factor,
                structure_score: v1.dimensions.structure_score * inv_factor
                    + v2.dimensions.structure_score * factor,
                technical_depth: v1.dimensions.technical_depth * inv_factor
                    + v2.dimensions.technical_depth * factor,
                explanation_length: v1.dimensions.explanation_length * inv_factor
                    + v2.dimensions.explanation_length * factor,
            },
            v1.source_confidence * inv_factor + v2.source_confidence * factor,
            v1.sample_count + v2.sample_count,
        )
    }
}

impl Default for UserStyleProfile {
    fn default() -> Self {
        Self::with_defaults()
    }
}

pub struct StyleMigrator {
    vectorizer: StyleVectorizer,
    extractor: StyleExtractor,
    applier: StyleApplier,
    profiles: HashMap<String, UserStyleProfile>,
    default_profile: UserStyleProfile,
}

impl StyleMigrator {
    pub fn new() -> Self {
        Self {
            vectorizer: StyleVectorizer::new(),
            extractor: StyleExtractor::new(),
            applier: StyleApplier::new(),
            profiles: HashMap::new(),
            default_profile: UserStyleProfile::with_defaults(),
        }
    }

    pub fn learn_from_code_samples(
        &mut self,
        user_id: &str,
        samples: &[CodeSample],
    ) -> StyleVector {
        let vector = self.vectorizer.from_coding_samples(samples);
        let patterns = self.extractor.extract_from_code(samples);

        let profile = self
            .profiles
            .entry(user_id.to_string())
            .or_insert_with(|| UserStyleProfile::new(user_id.to_string()));

        profile.code_style_vector = vector.clone();
        profile.total_samples += samples.len() as u32;
        profile.confidence = self.vectorizer.calculate_confidence(samples);
        profile.update_timestamp();

        Self::merge_patterns(profile, &patterns);

        vector
    }

    pub fn learn_from_messages(
        &mut self,
        user_id: &str,
        messages: &[MessageSample],
    ) -> DocumentStyleProfile {
        let profile = self.extractor.extract_from_messages(messages);

        let user_profile = self
            .profiles
            .entry(user_id.to_string())
            .or_insert_with(|| UserStyleProfile::new(user_id.to_string()));
        user_profile.document_style_profile = profile.clone();
        user_profile.total_samples += messages.len() as u32;
        user_profile.update_timestamp();

        profile
    }

    fn merge_patterns(profile: &mut UserStyleProfile, patterns: &ExtractedCodePatterns) {
        for func_pattern in &patterns.function_patterns {
            let pattern = LearnedPattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: LearnedPatternType::Structure,
                original: func_pattern.name.clone(),
                transformed: func_pattern.name.clone(),
                context: format!("function with {} params", func_pattern.param_count),
                usage_count: 1,
                last_used: Utc::now(),
            };
            profile.learned_patterns.push(pattern);
        }
    }

    pub fn apply_code_style(&self, code: &str, user_id: &str) -> String {
        let profile = self.profiles.get(user_id).unwrap_or(&self.default_profile);
        self.applier
            .apply_code_style(code, &profile.code_style_vector)
    }

    pub fn apply_document_style(&self, content: &str, user_id: &str) -> String {
        let profile = self.profiles.get(user_id).unwrap_or(&self.default_profile);
        let doc_applicator = DocumentStyleApplicator::new(profile.document_style_profile.clone());
        doc_applicator.apply_to_message(content)
    }

    pub fn get_style_vector(&self, user_id: &str) -> StyleVector {
        self.profiles
            .get(user_id)
            .map(|p| p.code_style_vector.clone())
            .unwrap_or_else(StyleVector::default_style)
    }

    pub fn get_document_profile(&self, user_id: &str) -> DocumentStyleProfile {
        self.profiles
            .get(user_id)
            .map(|p| p.document_style_profile.clone())
            .unwrap_or_else(|| DocumentStyleProfile {
                formality_level: 0.5,
                structure_level: 0.5,
                technical_vocabulary_ratio: 0.5,
                explanation_detail_level: 0.5,
                preferred_format: crate::style_extractor::DocumentFormat::Markdown,
            })
    }

    pub fn get_user_profile(&self, user_id: &str) -> Option<&UserStyleProfile> {
        self.profiles.get(user_id)
    }

    pub fn get_all_profiles(&self) -> Vec<&UserStyleProfile> {
        self.profiles.values().collect()
    }

    pub fn delete_profile(&mut self, user_id: &str) -> bool {
        self.profiles.remove(user_id).is_some()
    }

    pub fn similarity(&self, user_id1: &str, user_id2: &str) -> f32 {
        let v1 = self.get_style_vector(user_id1);
        let v2 = self.get_style_vector(user_id2);
        v1.similarity(&v2)
    }

    pub fn blend_styles(&self, user_id1: &str, user_id2: &str, factor: f32) -> StyleVector {
        let v1 = self.get_style_vector(user_id1);
        let v2 = self.get_style_vector(user_id2);

        let inv_factor = 1.0 - factor;
        StyleVector::new(
            crate::style_vectorizer::StyleDimensions {
                naming_score: v1.dimensions.naming_score * inv_factor
                    + v2.dimensions.naming_score * factor,
                density_score: v1.dimensions.density_score * inv_factor
                    + v2.dimensions.density_score * factor,
                comment_ratio: v1.dimensions.comment_ratio * inv_factor
                    + v2.dimensions.comment_ratio * factor,
                abstraction_level: v1.dimensions.abstraction_level * inv_factor
                    + v2.dimensions.abstraction_level * factor,
                formality_score: v1.dimensions.formality_score * inv_factor
                    + v2.dimensions.formality_score * factor,
                structure_score: v1.dimensions.structure_score * inv_factor
                    + v2.dimensions.structure_score * factor,
                technical_depth: v1.dimensions.technical_depth * inv_factor
                    + v2.dimensions.technical_depth * factor,
                explanation_length: v1.dimensions.explanation_length * inv_factor
                    + v2.dimensions.explanation_length * factor,
            },
            v1.source_confidence * inv_factor + v2.source_confidence * factor,
            v1.sample_count + v2.sample_count,
        )
    }

    pub fn apply_blended_style(
        &self,
        code: &str,
        user_id1: &str,
        user_id2: &str,
        factor: f32,
    ) -> String {
        let blended_vector = self.blend_styles(user_id1, user_id2, factor);
        self.applier.apply_code_style(code, &blended_vector)
    }

    pub fn export_profile(&self, user_id: &str) -> Option<String> {
        self.profiles
            .get(user_id)
            .and_then(|p| serde_json::to_string_pretty(p).ok())
    }

    pub fn import_profile(&mut self, user_id: &str, json: &str) -> Result<(), String> {
        let profile: UserStyleProfile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.profiles.insert(user_id.to_string(), profile);
        Ok(())
    }

    pub fn clear_all_profiles(&mut self) {
        self.profiles.clear();
    }

    pub fn get_stats(&self) -> StyleMigratorStats {
        StyleMigratorStats {
            total_profiles: self.profiles.len(),
            total_samples: self.profiles.values().map(|p| p.total_samples).sum(),
            average_confidence: if self.profiles.is_empty() {
                0.0
            } else {
                self.profiles.values().map(|p| p.confidence).sum::<f32>()
                    / self.profiles.len() as f32
            },
        }
    }
}

impl Default for StyleMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleMigratorStats {
    pub total_profiles: usize,
    pub total_samples: u32,
    pub average_confidence: f32,
}

pub struct StyleMigrationResult {
    pub original: String,
    pub transformed: String,
    pub style_vector: StyleVector,
    pub applied_patterns: Vec<String>,
    pub confidence: f32,
}

impl StyleMigrationResult {
    pub fn new(original: String, transformed: String, style_vector: StyleVector) -> Self {
        let confidence = style_vector.source_confidence;
        Self {
            original,
            transformed,
            style_vector,
            applied_patterns: Vec::new(),
            confidence,
        }
    }

    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.applied_patterns = patterns;
        self
    }
}
