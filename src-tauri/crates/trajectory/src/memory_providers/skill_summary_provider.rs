// SPDX-License-Identifier: AGPL-3.0-only

//! G21 Memory 系统增强 — Skill 摘要 MemoryProvider
//!
//! 把 `SkillManager` 缓存中的技能摘要注入到 Memory 体系中，让 Agent 在
//! 推理时通过 `MemoryProvider::prefetch` 拉取相关技能，无需显式调用
//! `skill_view` / `skills_list` 工具。
//!
//! ## 设计
//!
//! - 实现 `MemoryProvider` trait，provider_name = `"skill_summary"`
//! - `sync_turn`: No-op（Skill 是只读源，不接受外部写入）
//! - `prefetch`: 根据 query 字符串匹配 skill name / description / tags，
//!   返回按相关性分数排序的 `MemoryEntry` 列表
//! - `MemoryEntry.content` = SkillSummary 的 markdown 表示
//! - `MemoryEntry.memory_type` = `"skill"`
//! - `MemoryEntry.tags` = SkillSummary.tags
//! - `MemoryEntry.importance` = 0.5 + 0.5 * (success_rate)
//!   （success_rate 高的技能优先被召回）
//!
//! ## 使用
//!
//! 在 `init/services.rs` 启动时，把 `SkillSummaryProvider` 注册到
//! `MemoryProviderRegistry`，与 `Mem0Provider` / `HonchoProvider` 并列。
//! Agent 推理前调用 `registry.prefetch()` 时会自动检索相关技能摘要。

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::memory_provider::{
    MemoryEntry, MemoryProvider, MemoryQuery, MemoryQueryResult, MemoryType,
};
use crate::memory_providers::service::{MemoryNature, MemoryTier};

/// 技能摘要快照（原 skill_manager.rs 移入，因 skill_manager 已被上游清理）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub tags: Vec<String>,
}

/// 技能摘要 MemoryProvider —— 把 SkillManager 缓存中的技能摘要暴露为 MemoryEntry。
///
/// 通过 `Arc<RwLock<Vec<SkillSummary>>>` 与 SkillManager 共享数据；
/// SkillManager 更新缓存后调用 `refresh()` 同步到本 provider。
pub struct SkillSummaryProvider {
    /// 技能摘要快照（由 SkillManager 推送）
    skills: Arc<RwLock<Vec<SkillSummary>>>,
}

impl SkillSummaryProvider {
    pub fn new() -> Self {
        Self { skills: Arc::new(RwLock::new(Vec::new())) }
    }

    /// 用一份技能摘要快照初始化（构造后即可被 prefetch 检索）。
    pub(crate) fn with_skills(skills: Vec<SkillSummary>) -> Self {
        Self { skills: Arc::new(RwLock::new(skills)) }
    }

    /// 由 SkillManager 在缓存变更后调用，刷新本地快照。
    pub(crate) async fn refresh(&self, skills: Vec<SkillSummary>) {
        let mut guard = self.skills.write().await;
        *guard = skills;
    }

    /// 计算查询字符串与技能摘要的相关性分数。
    ///
    /// 匹配策略：
    /// - name 完全匹配 → +3.0
    /// - name 包含 query → +2.0
    /// - description 包含 query → +1.0
    /// - 任一 tag 包含 query → +1.5
    /// - 无匹配 → 0.0
    fn score_skill(skill: &SkillSummary, query: &str) -> f64 {
        if query.is_empty() {
            return 0.1;
        }
        let q = query.to_lowercase();
        let name = skill.name.to_lowercase();
        let desc = skill.description.to_lowercase();

        let mut score = 0.0_f64;
        if name == q {
            score += 3.0;
        } else if name.contains(&q) {
            score += 2.0;
        }
        if desc.contains(&q) {
            score += 1.0;
        }
        for tag in &skill.tags {
            if tag.to_lowercase().contains(&q) {
                score += 1.5;
                break;
            }
        }
        score
    }

    /// 把 SkillSummary 转换为 MemoryEntry。
    fn to_memory_entry(skill: &SkillSummary, score: f64) -> MemoryEntry {
        let now = Utc::now();
        let content = format!(
            "# {}\n\n{}\n\n**Category:** {}\n**Version:** {}\n**Tags:** {}",
            skill.name,
            skill.description,
            skill.category,
            skill.version,
            skill.tags.join(", "),
        );

        MemoryEntry {
            id: format!("skill:{}", skill.id),
            content,
            memory_type: MemoryType::Skill,
            importance: score,
            tags: skill.tags.clone(),
            created_at: now,
            last_accessed: now,
            access_count: 0,
            tier: MemoryTier::LongTerm,
            nature: MemoryNature::Semantic,
        }
    }
}

impl Default for SkillSummaryProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryProvider for SkillSummaryProvider {
    async fn sync_turn(&self, _session_id: &str, _entries: Vec<MemoryEntry>) -> Result<(), String> {
        // Skill 是只读源，不接受外部写入。
        // 如果 entry 中有 `skill:` 前缀的条目，理论上应该回写到 SkillManager，
        // 但 SkillManager 已有自己的 CRUD 入口，这里保持 No-op 避免循环。
        Ok(())
    }

    async fn prefetch(
        &self,
        _session_id: &str,
        query: &MemoryQuery,
    ) -> Result<MemoryQueryResult, String> {
        let skills = self.skills.read().await;
        if skills.is_empty() {
            return Ok(MemoryQueryResult { entries: Vec::new(), scores: Vec::new(), total: 0 });
        }

        // 计算每个技能的相关性分数
        let mut scored: Vec<(f64, &SkillSummary)> = skills
            .iter()
            .map(|s| (Self::score_skill(s, &query.query), s))
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // 按分数降序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 应用 limit
        let limit = query.limit.min(scored.len());
        let top = &scored[..limit];

        let entries: Vec<MemoryEntry> =
            top.iter().map(|(score, skill)| Self::to_memory_entry(skill, *score)).collect();
        let scores: Vec<f64> = top.iter().map(|(score, _)| *score).collect();
        let total = entries.len();

        Ok(MemoryQueryResult { entries, scores, total })
    }

    async fn shutdown(&self) -> Result<(), String> {
        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "skill_summary"
    }

    fn provider_version(&self) -> &'static str {
        "0.1.0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_provider::MemoryQuery;

    fn make_skill(id: &str, name: &str, desc: &str, tags: Vec<&str>) -> SkillSummary {
        SkillSummary {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: "test".to_string(),
            version: "0.1.0".to_string(),
            tags: tags.into_iter().map(String::from).collect(),
        }
    }

    #[tokio::test]
    async fn test_prefetch_matches_name() {
        let provider = SkillSummaryProvider::with_skills(vec![
            make_skill("1", "quant_backtest", "量化回测技能", vec!["quant"]),
            make_skill("2", "news_analysis", "新闻分析技能", vec!["news"]),
        ]);

        let query = MemoryQuery {
            query: "quant".to_string(),
            memory_types: None,
            tags: None,
            limit: 10,
            min_importance: None,
            tier_filter: None,
        };

        let result = provider.prefetch("test_session", &query).await.unwrap();
        assert_eq!(result.total, 1);
        assert!(result.entries[0].content.contains("quant_backtest"));
        assert!(result.scores[0] >= 2.0);
    }

    #[tokio::test]
    async fn test_prefetch_matches_tag() {
        let provider = SkillSummaryProvider::with_skills(vec![
            make_skill("1", "skill_a", "技能 A", vec!["risk_management"]),
            make_skill("2", "skill_b", "技能 B", vec!["other"]),
        ]);

        let query = MemoryQuery {
            query: "risk".to_string(),
            memory_types: None,
            tags: None,
            limit: 10,
            min_importance: None,
            tier_filter: None,
        };

        let result = provider.prefetch("test_session", &query).await.unwrap();
        assert_eq!(result.total, 1);
        assert!(result.entries[0].content.contains("skill_a"));
    }

    #[tokio::test]
    async fn test_prefetch_empty_query_returns_all() {
        let provider = SkillSummaryProvider::with_skills(vec![
            make_skill("1", "skill_a", "A", vec![]),
            make_skill("2", "skill_b", "B", vec![]),
        ]);

        let query = MemoryQuery {
            query: "".to_string(),
            memory_types: None,
            tags: None,
            limit: 10,
            min_importance: None,
            tier_filter: None,
        };

        let result = provider.prefetch("test_session", &query).await.unwrap();
        // 空查询时每个技能得 0.1 分（被过滤掉）
        // 修改后：空查询得 0.1 分 > 0.0，所以会被返回
        assert_eq!(result.total, 2);
    }

    #[tokio::test]
    async fn test_prefetch_no_match_returns_empty() {
        let provider =
            SkillSummaryProvider::with_skills(vec![make_skill("1", "skill_a", "A", vec!["tag1"])]);

        let query = MemoryQuery {
            query: "zzz_no_match".to_string(),
            memory_types: None,
            tags: None,
            limit: 10,
            min_importance: None,
            tier_filter: None,
        };

        let result = provider.prefetch("test_session", &query).await.unwrap();
        assert_eq!(result.total, 0);
    }

    #[tokio::test]
    async fn test_sync_turn_is_noop() {
        let provider = SkillSummaryProvider::new();
        let result = provider.sync_turn("test", Vec::new()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_refresh_replaces_skills() {
        let provider = SkillSummaryProvider::new();
        provider.refresh(vec![make_skill("1", "skill_a", "A", vec![])]).await;

        let query = MemoryQuery {
            query: "skill_a".to_string(),
            memory_types: None,
            tags: None,
            limit: 10,
            min_importance: None,
            tier_filter: None,
        };
        let result = provider.prefetch("test", &query).await.unwrap();
        assert_eq!(result.total, 1);

        // 刷新为空
        provider.refresh(Vec::new()).await;
        let result = provider.prefetch("test", &query).await.unwrap();
        assert_eq!(result.total, 0);
    }
}
