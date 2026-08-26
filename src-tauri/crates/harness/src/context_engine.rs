// SPDX-License-Identifier: AGPL-3.0-only

//! 上下文压缩引擎 trait (P1-6)
//!
//! 借鉴 Hermes Agent 的 context_engine.py：
//! - 可插拔的压缩策略（ContextEngine trait）
//! - 会话谱系追踪（CompactionRecord）
//! - 技能重注入支持

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::conversation_model::ConversationMessage;
use crate::runtime_types::compact::{CompactionConfig, CompactionResult};
use crate::runtime_types::session::Session;

/// 上下文压缩引擎 trait
///
/// 允许不同的压缩策略实现此 trait：
/// - 传统 LLM 摘要压缩
/// - 会话记忆压缩（基于结构化记忆）
/// - 混合压缩（先尝试记忆压缩，失败回退）
pub trait ContextEngine: Send + Sync {
    /// 压缩会话
    fn compact(
        &self,
        session: &Session,
        config: &CompactionConfig,
        context: &CompactionContext,
    ) -> CompactionResult;

    /// 引擎名称
    fn name(&self) -> &str;

    /// 是否支持当前压缩
    fn supports(&self, session: &Session) -> bool;
}

/// 压缩上下文 - 提供压缩时需要的额外信息
#[derive(Debug, Clone, Default)]
pub struct CompactionContext {
    /// 结构化记忆（用于会话记忆压缩）
    pub memories: Vec<StructuredMemory>,
    /// 当前激活的技能列表（用于重注入）
    pub active_skills: Vec<SkillInfo>,
    /// 上下文提供者注入的额外信息
    pub extra_context: HashMap<String, String>,
}

/// 结构化记忆 - 从轨迹系统提取
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredMemory {
    pub id: String,
    #[serde(alias = "memory_type")]
    pub memory_type: MemoryType,
    pub content: String,
    pub confidence: f64,
    #[serde(alias = "created_at")]
    pub created_at: String,
}

/// 记忆类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// 用户偏好
    UserPreference,
    /// 项目事实
    ProjectFact,
    /// 工作模式
    WorkingPattern,
    /// 知识片段
    KnowledgeSnippet,
    /// 历史决策
    HistoricalDecision,
    /// 技能经验
    SkillExperience,
}

/// 技能信息 - 用于重注入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(alias = "trigger_conditions")]
    pub trigger_conditions: Vec<String>,
}

/// 压缩记录 - 会话谱系追踪
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionRecord {
    /// 记录 ID
    pub id: String,
    /// 会话 ID
    #[serde(alias = "session_id")]
    pub session_id: String,
    /// 压缩时间
    pub timestamp: String,
    /// 使用的引擎名称
    #[serde(alias = "engine_name")]
    pub engine_name: String,
    /// 压缩配置快照
    pub config: CompactionConfigSnapshot,
    /// 压缩结果
    pub result: CompactionResultSummary,
    /// 触发原因
    pub trigger: CompactionTrigger,
    /// 版本号（用于追踪压缩次数）
    pub version: u32,
    /// 父记录 ID（压缩前的记录）
    #[serde(alias = "parent_id")]
    pub parent_id: Option<String>,
}

/// 压缩配置快照（用于记录历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionConfigSnapshot {
    #[serde(alias = "preserve_recent_messages")]
    pub preserve_recent_messages: usize,
    #[serde(alias = "max_estimated_tokens")]
    pub max_estimated_tokens: usize,
    #[serde(alias = "enable_turn_summaries")]
    pub enable_turn_summaries: bool,
    #[serde(alias = "enable_distance_decay")]
    pub enable_distance_decay: bool,
}

/// 压缩结果摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionResultSummary {
    #[serde(alias = "original_message_count")]
    pub original_message_count: usize,
    #[serde(alias = "compacted_message_count")]
    pub compacted_message_count: usize,
    #[serde(alias = "removed_message_count")]
    pub removed_message_count: usize,
    #[serde(alias = "summary_length")]
    pub summary_length: usize,
}

/// 压缩触发原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    /// 轮次触发（每 N 轮压缩）
    TurnCount,
    /// Token 阈值触发
    TokenThreshold,
    /// 响应式压缩（API 返回 context too long 错误）
    Reactive,
    /// 紧急模式（熔断器触发）
    Emergency,
    /// 手动触发
    Manual,
    /// 系统初始化
    SystemInit,
}

/// 会话谱系 - 追踪压缩历史
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLineage {
    /// 会话 ID
    #[serde(alias = "session_id")]
    pub session_id: String,
    /// 压缩记录列表（按时间顺序）
    #[serde(alias = "compaction_history")]
    pub compaction_history: Vec<CompactionRecord>,
    /// 当前版本号
    #[serde(alias = "current_version")]
    pub current_version: u32,
}

impl SessionLineage {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            compaction_history: Vec::new(),
            current_version: 0,
        }
    }

    /// 添加压缩记录
    pub fn add_record(&mut self, record: CompactionRecord) {
        self.current_version += 1;
        self.compaction_history.push(record);
    }

    /// 获取最新的压缩记录
    pub fn latest(&self) -> Option<&CompactionRecord> {
        self.compaction_history.last()
    }

    /// 获取压缩次数
    pub fn compaction_count(&self) -> usize {
        self.compaction_history.len()
    }

    /// 生成新的压缩记录 ID
    pub fn next_record_id(&self) -> String {
        format!("{}-v{}", self.session_id, self.current_version + 1)
    }
}

/// 技能重注入器 - 压缩后重新注入技能到 system prompt
pub struct SkillReinjector;

impl SkillReinjector {
    /// 生成技能注入的系统提示片段
    pub fn generate_skill_injection(skills: &[SkillInfo]) -> Option<String> {
        if skills.is_empty() {
            return None;
        }

        let mut injection = String::from("\n\n## 当前可用技能\n\n");
        injection.push_str("以下技能在压缩后仍然激活，请在后续对话中按需使用：\n\n");

        for skill in skills {
            injection.push_str(&format!(
                "- **{}**: {} (触发条件: {})\n",
                skill.name,
                skill.description,
                skill.trigger_conditions.join(", ")
            ));
        }

        Some(injection)
    }

    /// 从会话中提取已注入的技能信息
    pub fn extract_injected_skills(messages: &[ConversationMessage]) -> Vec<String> {
        let mut skills = Vec::new();

        for msg in messages {
            if msg.role == crate::conversation_model::MessageRole::System {
                for block in &msg.blocks {
                    if let crate::conversation_model::ContentBlock::Text { text } = block
                        && text.contains("## 当前可用技能")
                    {
                        skills.push(text.clone());
                    }
                }
            }
        }

        skills
    }

    /// 检查是否需要重新注入技能
    pub fn should_reinject(compacted_session: &Session, active_skills: &[SkillInfo]) -> bool {
        if active_skills.is_empty() {
            return false;
        }

        // 检查压缩后的会话是否还包含技能信息
        let has_skill_injection = Self::extract_injected_skills(&compacted_session.messages);
        has_skill_injection.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lineage() {
        let mut lineage = SessionLineage::new("test-session");
        assert_eq!(lineage.current_version, 0);
        assert_eq!(lineage.compaction_count(), 0);

        let record = CompactionRecord {
            id: lineage.next_record_id(),
            session_id: "test-session".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            engine_name: "test-engine".to_string(),
            config: CompactionConfigSnapshot {
                preserve_recent_messages: 12,
                max_estimated_tokens: 80000,
                enable_turn_summaries: true,
                enable_distance_decay: true,
            },
            result: CompactionResultSummary {
                original_message_count: 20,
                compacted_message_count: 13,
                removed_message_count: 7,
                summary_length: 500,
            },
            trigger: CompactionTrigger::TokenThreshold,
            version: 1,
            parent_id: None,
        };

        lineage.add_record(record);
        assert_eq!(lineage.current_version, 1);
        assert_eq!(lineage.compaction_count(), 1);
        assert!(lineage.latest().is_some());
    }

    #[test]
    fn test_skill_reinjector() {
        let skills = vec![SkillInfo {
            id: "1".to_string(),
            name: "代码审查".to_string(),
            description: "审查代码质量".to_string(),
            trigger_conditions: vec!["当用户请求审查代码".to_string()],
        }];

        let injection = SkillReinjector::generate_skill_injection(&skills);
        assert!(injection.is_some());
        let text = injection.expect("测试应成功");
        assert!(text.contains("代码审查"));
        assert!(text.contains("当用户请求审查代码"));
    }

    #[test]
    fn test_skill_reinjector_empty() {
        let skills: Vec<SkillInfo> = Vec::new();
        let injection = SkillReinjector::generate_skill_injection(&skills);
        assert!(injection.is_none());
    }
}
