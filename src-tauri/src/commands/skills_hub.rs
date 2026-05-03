use crate::paths::axagent_home;
use crate::AppState;
use axagent_trajectory::{
    Skill, SkillsHubAdapter, SkillsHubClient, SkillsHubConfig, SkillsHubSearchResult,
};
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

#[derive(Debug, Serialize)]
pub struct SkillExportResult {
    pub hermes_json: String,
    pub skill_name: String,
    pub version: String,
    pub manifest: serde_json::Value,
}

/// 导出本地 skill 为可发布的格式（Hermes JSON + manifest 摘要）
#[tauri::command]
pub async fn skills_hub_export(skill_name: String) -> Result<SkillExportResult, String> {
    let home = dirs::home_dir().ok_or("无法确定 home 目录")?;
    let skill_dirs = vec![
        axagent_home().join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];

    let mut adapter = SkillsHubAdapter::new();
    let mut found_skill: Option<(String, serde_json::Value)> = None;

    // 搜索 skill
    for dir in &skill_dirs {
        let skill_path = dir.join(&skill_name);
        if !skill_path.exists() {
            continue;
        }

        // 读取 SKILL.md
        let skill_md_path = skill_path.join("SKILL.md");
        if skill_md_path.exists() {
            let content = std::fs::read_to_string(&skill_md_path)
                .map_err(|e| format!("读取 SKILL.md 失败: {}", e))?;
            adapter.parse_hermes_skill_md(&content)?;
        }

        // 读取 manifest.json
        let manifest_path = skill_path
            .join("manifest.json")
            .exists()
            .then(|| skill_path.join("manifest.json"))
            .or_else(|| {
                skill_path
                    .join("skill-manifest.json")
                    .exists()
                    .then(|| skill_path.join("skill-manifest.json"))
            });

        if let Some(mpath) = manifest_path {
            let manifest_content = std::fs::read_to_string(&mpath)
                .map_err(|e| format!("读取 manifest 失败: {}", e))?;
            let manifest_json: serde_json::Value = serde_json::from_str(&manifest_content)
                .map_err(|e| format!("manifest JSON 解析失败: {}", e))?;
            found_skill = Some((skill_name.clone(), manifest_json));
        }
        break;
    }

    let (name, manifest) = found_skill.ok_or_else(|| format!("Skill '{}' 未找到", skill_name))?;

    // 转换为 Hermes 格式
    let hermes = adapter.to_hermes_md();
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();

    Ok(SkillExportResult {
        hermes_json: serde_json::to_string_pretty(&hermes)
            .map_err(|e| format!("序列化失败: {}", e))?,
        skill_name: name,
        version,
        manifest,
    })
}

#[tauri::command]
pub async fn skills_hub_import(manifest_json: String) -> Result<Skill, String> {
    let mut adapter = SkillsHubAdapter::new();
    adapter.parse_hermes_manifest(&manifest_json)?;

    adapter.to_axagent_skill()
}
