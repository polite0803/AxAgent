// SPDX-License-Identifier: AGPL-3.0-only

// ABANDONED(2026-07-05): 此模块经评估不建议继续维护。
// 原因：硬编码虚构 API 端点。
// 若未来需求变更可解除此标记，当前通过 #[cfg(feature = "abandoned")] 隔离。

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

/// Skills Hub 配置，由环境变量驱动。
/// 不设置则默认不可用，无硬编码端点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsHubConfig {
    pub api_url: String,
    pub api_key: Option<String>,
}

impl SkillsHubConfig {
    /// 从环境变量构造配置：
    /// - `AGENTSKILLS_API_URL` — 后端 API 地址
    /// - `AGENTSKILLS_API_KEY` — 可选的 API Key
    pub fn from_env() -> Self {
        Self {
            api_url: std::env::var("AGENTSKILLS_API_URL").unwrap_or_default(),
            api_key: std::env::var("AGENTSKILLS_API_KEY").ok(),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.api_url.is_empty()
    }
}
