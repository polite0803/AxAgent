// SPDX-License-Identifier: AGPL-3.0-only

//! Skill lifecycle management module
//!
//! Replaces TypeScript `SkillManager.ts` with Rust implementation.
//! Provides skill CRUD operations, filtering, and lifecycle management.
//! Uses `TrajectoryStorage` as the single source of truth for persistence.

use crate::skill::{HermesMetadata, Skill, SkillMetadata};
use crate::storage::TrajectoryStorage;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillCreationParams {
    pub name: String,
    pub description: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillFilter {
    pub category: Option<String>,
    pub tag: Option<String>,
    pub platform: Option<String>,
}

pub(crate) fn create_skill_from_params(params: SkillCreationParams) -> Skill {
    let category = params.category.unwrap_or_else(|| "general".to_string());
    let tags = params.tags.unwrap_or_default();
    let platforms = params.platforms.unwrap_or_else(|| vec![detect_os()]);

    let mut skill = Skill::new(params.name, params.description, params.content, category.clone());

    skill.tags = tags.clone();
    skill.platforms = platforms.clone();
    skill.metadata = SkillMetadata {
        hermes: HermesMetadata {
            tags,
            category,
            fallback_for_toolsets: Vec::new(),
            requires_toolsets: Vec::new(),
            config: Vec::new(),
            ..Default::default()
        },
        references: Vec::new(),
    };

    skill
}

pub(crate) fn update_skill_from_params(skill: &mut Skill, params: SkillUpdateParams) {
    let now = Utc::now();

    if let Some(name) = params.name {
        skill.name = name;
    }
    if let Some(description) = params.description {
        skill.description = description;
    }
    if let Some(content) = params.content {
        skill.content = content;
    }
    if let Some(category) = params.category {
        skill.category = category.clone();
        skill.metadata.hermes.category = category;
    }
    if let Some(tags) = params.tags {
        skill.tags = tags.clone();
        skill.metadata.hermes.tags = tags;
    }
    if let Some(platforms) = params.platforms {
        skill.platforms = platforms;
    }

    skill.updated_at = now;
}

pub(crate) fn patch_skill_content(
    skill: &mut Skill,
    old_string: &str,
    new_string: &str,
) -> Result<(), &'static str> {
    if !skill.content.contains(old_string) {
        return Err("未找到匹配的文字");
    }
    skill.content = skill.content.replace(old_string, new_string);
    skill.updated_at = Utc::now();
    Ok(())
}

pub(crate) fn skill_to_summary(skill: &Skill) -> SkillSummary {
    SkillSummary {
        id: skill.id.clone(),
        name: skill.name.clone(),
        description: skill.description.clone(),
        category: skill.metadata.hermes.category.clone(),
        version: skill.version.clone(),
        tags: skill.metadata.hermes.tags.clone(),
    }
}

pub(crate) fn increment_skill_usage(skill: &mut Skill, success: bool) {
    skill.total_usages += 1;
    if success {
        skill.successful_usages += 1;
    }
    skill.success_rate = skill.successful_usages as f64 / skill.total_usages as f64;
    skill.last_used_at = Some(Utc::now());
}

fn detect_os() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "unknown".to_string();
}

pub(crate) struct SkillManager {
    storage: Arc<TrajectoryStorage>,
    cache: HashMap<String, Skill>,
    name_index: HashMap<String, String>,
}

impl SkillManager {
    pub(crate) fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self { storage, cache: HashMap::new(), name_index: HashMap::new() }
    }

    pub(crate) async fn load_from_db(&mut self) -> Result<(), String> {
        let skills = self.storage.get_skills().await.map_err(|e| e.to_string())?;
        for skill in skills {
            let id = skill.id.clone();
            self.name_index.insert(skill.name.clone(), id.clone());
            self.cache.insert(id, skill);
        }
        Ok(())
    }

    pub(crate) async fn create_skill(
        &mut self,
        params: SkillCreationParams,
    ) -> Result<Skill, String> {
        let skill = create_skill_from_params(params);
        let id = skill.id.clone();
        self.storage.save_skill(&skill).await.map_err(|e| e.to_string())?;
        self.name_index.insert(skill.name.clone(), id.clone());
        self.cache.insert(id, skill.clone());
        Ok(skill)
    }

    pub(crate) async fn update_skill(
        &mut self,
        id: &str,
        params: SkillUpdateParams,
    ) -> Option<Skill> {
        if let Some(skill) = self.cache.get_mut(id) {
            update_skill_from_params(skill, params);
            let updated = skill.clone();
            // 同步到数据库
            if let Err(e) = self.storage.save_skill(&updated).await {
                tracing::error!("Failed to sync skill update to DB: {}", e);
            }
            Some(updated)
        } else {
            None
        }
    }

    pub(crate) async fn patch_skill_content(
        &mut self,
        id: &str,
        old_string: &str,
        new_string: &str,
    ) -> Option<Result<Skill, &'static str>> {
        if let Some(skill) = self.cache.get_mut(id) {
            match patch_skill_content(skill, old_string, new_string) {
                Ok(()) => {
                    let updated = skill.clone();
                    if let Err(e) = self.storage.save_skill(&updated).await {
                        tracing::error!("Failed to sync skill patch to DB: {}", e);
                    }
                    Some(Ok(updated))
                },
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }

    pub(crate) async fn delete_skill(&mut self, id: &str) -> bool {
        if let Some(skill) = self.cache.remove(id) {
            self.name_index.remove(&skill.name);
            if let Err(e) = self.storage.delete_skill(id).await {
                tracing::error!("Failed to delete skill from DB: {}", e);
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn get_skill(&self, id: &str) -> Option<&Skill> {
        self.cache.get(id)
    }

    pub(crate) fn get_skill_by_name(&self, name: &str) -> Option<&Skill> {
        self.name_index.get(name).and_then(|id| self.cache.get(id))
    }

    pub(crate) fn get_all_skills(&self) -> Vec<&Skill> {
        self.cache.values().collect()
    }

    pub(crate) fn get_all_skills_owned(&self) -> Vec<Skill> {
        self.cache.values().cloned().collect()
    }

    pub(crate) fn get_skills_by_category(&self, category: &str) -> Vec<&Skill> {
        self.cache.values().filter(|s| s.category == category).collect()
    }

    pub(crate) fn list_skills(&self, filter: Option<SkillFilter>) -> Vec<SkillSummary> {
        let mut skills: Vec<Skill> = self.get_all_skills_owned();

        if let Some(ref f) = filter {
            if let Some(ref category) = f.category {
                skills.retain(|s| s.metadata.hermes.category == *category);
            }
            if let Some(ref tag) = f.tag {
                skills.retain(|s| s.metadata.hermes.tags.contains(tag));
            }
            if let Some(ref platform) = f.platform {
                skills.retain(|s| s.platforms.is_empty() || s.platforms.contains(platform));
            }
        }

        skills.into_iter().map(|s| skill_to_summary(&s)).collect()
    }

    pub(crate) fn get_skill_content(&self, id: &str, level: u8) -> Option<SkillContentResult> {
        match level {
            0 => Some(SkillContentResult::List(self.list_skills(None))),
            1 => {
                let skill = self.get_skill(id)?;
                Some(SkillContentResult::Formatted(self.format_skill_content(skill)))
            },
            2 => {
                let skill = self.get_skill(id)?;
                Some(SkillContentResult::Full(skill.clone()))
            },
            _ => Some(SkillContentResult::List(self.list_skills(None))),
        }
    }

    fn format_skill_content(&self, skill: &Skill) -> String {
        format!(
            "# {}\n\n{}\n\n---\n\n**Category:** {}\n**Tags:** {}\n**Version:** {}\n**Usage:** {} ({}% success)\n\n---\n\n```\n{}\n```",
            skill.name,
            skill.description,
            skill.category,
            skill.tags.join(", "),
            skill.version,
            skill.total_usages,
            (skill.success_rate * 100.0) as u32,
            skill.content
        )
    }

    pub(crate) fn record_usage(&mut self, id: &str, success: bool) -> bool {
        if let Some(skill) = self.cache.get_mut(id) {
            increment_skill_usage(skill, success);
            true
        } else {
            false
        }
    }

    pub(crate) fn get_stats(&self) -> SkillStats {
        let total = self.cache.len();
        let total_usages: u32 = self.cache.values().map(|s| s.total_usages).sum();
        let total_successful: u32 = self.cache.values().map(|s| s.successful_usages).sum();
        let avg_success = if total_usages > 0 {
            total_successful as f64 / total_usages as f64
        } else {
            0.0
        };

        let mut category_counts: HashMap<String, usize> = HashMap::new();
        for skill in self.cache.values() {
            *category_counts.entry(skill.category.clone()).or_insert(0) += 1;
        }

        SkillStats {
            total_skills: total,
            total_usages,
            average_success_rate: avg_success,
            category_distribution: category_counts,
        }
    }

    pub(crate) fn import_skills(&mut self, skills: Vec<Skill>) {
        for skill in skills {
            let id = skill.id.clone();
            self.name_index.insert(skill.name.clone(), id.clone());
            self.cache.insert(id, skill);
        }
    }

    pub(crate) fn export_skills(&self) -> Vec<Skill> {
        self.get_all_skills_owned()
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.name_index.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.cache.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum SkillContentResult {
    List(Vec<SkillSummary>),
    Formatted(String),
    Full(Skill),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillStats {
    #[serde(rename = "totalSkills")]
    pub total_skills: usize,
    #[serde(rename = "totalUsages")]
    pub total_usages: u32,
    #[serde(rename = "averageSuccessRate")]
    pub average_success_rate: f64,
    #[serde(rename = "categoryDistribution")]
    pub category_distribution: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = create_skill_from_params(SkillCreationParams {
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            content: "Hello World".to_string(),
            category: Some("testing".to_string()),
            tags: Some(vec!["test".to_string()]),
            platforms: None,
        });

        // ID 现在是 UUID 格式
        assert!(!skill.id.is_empty());
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.category, "testing");
    }

    #[test]
    fn test_skill_update() {
        let mut skill = create_skill_from_params(SkillCreationParams {
            name: "Original".to_string(),
            description: "Original desc".to_string(),
            content: "Original content".to_string(),
            category: None,
            tags: None,
            platforms: None,
        });

        update_skill_from_params(
            &mut skill,
            SkillUpdateParams {
                name: Some("Updated".to_string()),
                description: Some("Updated desc".to_string()),
                content: None,
                category: None,
                tags: None,
                platforms: None,
            },
        );

        assert_eq!(skill.name, "Updated");
        assert_eq!(skill.description, "Updated desc");
    }

    #[test]
    fn test_skill_patch() {
        let mut skill = create_skill_from_params(SkillCreationParams {
            name: "Patch Test".to_string(),
            description: "Testing patch".to_string(),
            content: "Hello World".to_string(),
            category: None,
            tags: None,
            platforms: None,
        });

        let result = patch_skill_content(&mut skill, "World", "Rust");
        assert!(result.is_ok());
        assert_eq!(skill.content, "Hello Rust");
    }

    #[test]
    fn test_skill_filter() {
        let skill1 = create_skill_from_params(SkillCreationParams {
            name: "Skill 1".to_string(),
            description: "".to_string(),
            content: "".to_string(),
            category: Some("backend".to_string()),
            tags: Some(vec!["rust".to_string()]),
            platforms: None,
        });
        let skill2 = create_skill_from_params(SkillCreationParams {
            name: "Skill 2".to_string(),
            description: "".to_string(),
            content: "".to_string(),
            category: Some("frontend".to_string()),
            tags: Some(vec!["react".to_string()]),
            platforms: None,
        });

        let skills = [skill1, skill2];
        let filtered: Vec<&Skill> =
            skills.iter().filter(|s| s.metadata.hermes.category == "backend").collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].category, "backend");
    }

    #[test]
    fn test_usage_tracking() {
        let mut skill = create_skill_from_params(SkillCreationParams {
            name: "Usage Test".to_string(),
            description: "".to_string(),
            content: "".to_string(),
            category: None,
            tags: None,
            platforms: None,
        });

        increment_skill_usage(&mut skill, true);
        increment_skill_usage(&mut skill, true);
        increment_skill_usage(&mut skill, false);

        assert_eq!(skill.total_usages, 3);
        assert_eq!(skill.successful_usages, 2);
        assert!((skill.success_rate - 0.666).abs() < 0.01);
    }
}
