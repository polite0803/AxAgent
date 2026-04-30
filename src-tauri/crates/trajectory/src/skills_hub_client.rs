use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f64,
    pub readme_url: Option<String>,
    pub manifest_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubSearchResult {
    pub skills: Vec<SkillsHubSkill>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubConfig {
    pub api_url: String,
    pub api_key: Option<String>,
}

impl Default for SkillsHubConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.agentskills.io".to_string(),
            api_key: None,
        }
    }
}

pub struct SkillsHubClient {
    config: SkillsHubConfig,
    client: Client,
}

impl SkillsHubClient {
    pub fn new(config: SkillsHubConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        _category: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<SkillsHubSearchResult, String> {
        let url = format!("{}/v1/skills/search", self.config.api_url);
        let params = [
            ("q", query),
            ("page", &page.to_string()),
            ("page_size", &page_size.to_string()),
        ];
        let mut req = self.client.get(&url).query(&params);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Skills Hub API error: {}", resp.status()));
        }
        resp.json::<SkillsHubSearchResult>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_skill(&self, skill_id: &str) -> Result<SkillsHubSkill, String> {
        let url = format!("{}/v1/skills/{}", self.config.api_url, skill_id);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Skills Hub API error: {}", resp.status()));
        }
        resp.json::<SkillsHubSkill>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn download_skill(&self, skill_id: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}/v1/skills/{}/download", self.config.api_url, skill_id);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Download failed: {}", resp.status()));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| e.to_string())
    }

    pub async fn list_categories(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/v1/skills/categories", self.config.api_url);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Skills Hub API error: {}", resp.status()));
        }
        resp.json::<Vec<String>>().await.map_err(|e| e.to_string())
    }

    pub async fn featured(&self) -> Result<Vec<SkillsHubSkill>, String> {
        let url = format!("{}/v1/skills/featured", self.config.api_url);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Skills Hub API error: {}", resp.status()));
        }
        resp.json::<Vec<SkillsHubSkill>>()
            .await
            .map_err(|e| e.to_string())
    }
}
