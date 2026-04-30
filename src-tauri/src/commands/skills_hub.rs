use axagent_trajectory::{
    Skill, SkillsHubAdapter, SkillsHubClient, SkillsHubConfig, SkillsHubSearchResult,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubSearchResponse {
    pub skills: Vec<SkillsHubSkillInfo>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubSkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub downloads: u32,
    pub rating: f32,
}

impl From<SkillsHubSearchResult> for SkillsHubSearchResponse {
    fn from(result: SkillsHubSearchResult) -> Self {
        Self {
            skills: result
                .skills
                .into_iter()
                .map(|s| SkillsHubSkillInfo {
                    id: s.id,
                    name: s.name,
                    description: s.description,
                    category: s.category,
                    author: s.author,
                    version: s.version,
                    tags: s.tags,
                    downloads: s.downloads as u32,
                    rating: s.rating as f32,
                })
                .collect(),
            total: result.total as u32,
            page: result.page as u32,
            page_size: result.page_size as u32,
        }
    }
}

#[tauri::command]
pub async fn skills_hub_search(
    query: String,
    category: Option<String>,
    page: u32,
    page_size: u32,
) -> Result<SkillsHubSearchResponse, String> {
    let client = SkillsHubClient::new(SkillsHubConfig::default());
    let result = client
        .search(
            &query,
            category.as_deref(),
            page as usize,
            page_size as usize,
        )
        .await?;
    Ok(result.into())
}

#[tauri::command]
pub async fn skills_hub_install(
    _state: State<'_, AppState>,
    skill_id: String,
) -> Result<(), String> {
    let client = SkillsHubClient::new(SkillsHubConfig::default());
    let mut adapter = SkillsHubAdapter::new();

    let skill = client.get_skill(&skill_id).await?;

    adapter.parse_hermes_skill_md(skill.readme_url.as_deref().unwrap_or_default())?;

    let axagent_skill = adapter.to_axagent_skill()?;

    tracing::info!("Installing skill '{}' from Skills Hub", axagent_skill.name);

    Ok(())
}

#[tauri::command]
pub async fn skills_hub_export(_skill_id: String) -> Result<String, String> {
    Err("Export not yet implemented - requires skill lookup".to_string())
}

#[tauri::command]
pub async fn skills_hub_import(manifest_json: String) -> Result<Skill, String> {
    let mut adapter = SkillsHubAdapter::new();
    adapter.parse_hermes_manifest(&manifest_json)?;

    adapter.to_axagent_skill()
}
