// SPDX-License-Identifier: AGPL-3.0-only

//! MemoryStore 契约（记忆外溢/共享）
//!
//! 增强能力 (借鉴 Hermes Agent):
//! - Trivial 输入门控: 跳过无信号输入的 prefetch
//! - 技能脚手架剥离: 用户调用技能时剥离技能正文后再写入记忆
//! - 生命周期钩子: prefetch/sync/on_session_end
//! - 记忆写审批门: 对重要记忆写入要求审批

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 基础 DTO ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAddRequest {
    pub content: String,
    pub namespace: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchItem {
    pub id: String,
    pub content: String,
    pub namespace: Option<String>,
    pub importance: u8,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchRequest {
    pub query: String,
    pub namespace: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTreeItem {
    pub id: String,
    pub content: String,
    pub children: Vec<MemoryTreeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryUpdateRequest {
    pub id: String,
    pub content: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFeedbackRequest {
    pub memory_id: String,
    pub helpful: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryActionResultDto {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGroupedDto {
    pub date: String,
    pub items: Vec<MemorySearchItem>,
}

// ── Trivial 输入门控 ───────────────────────────────────────────────

/// Trivial 输入门控 — 检测无信号输入以跳过 prefetch
///
/// 借鉴 Hermes 的 `TRIVIAL_PROMPT_RE`:
///   "yes/no/ok/thanks/继续/好的" 等无信号输入跳过 prefetch,
///   省网络往返,防污染上下文。
pub struct TrivialInputGate;

impl TrivialInputGate {
    /// 判断输入是否为 trivial (无信号)
    pub fn is_trivial(input: &str) -> bool {
        let trimmed = input.trim().to_lowercase();

        if trimmed.is_empty() {
            return true;
        }

        // 匹配 trivial 模式
        let trivial_patterns = [
            "yes",
            "no",
            "ok",
            "okay",
            "fine",
            "good",
            "thanks",
            "thank you",
            "continue",
            "go on",
            "好的",
            "继续",
            "是的",
            "不是",
            "谢谢",
            "对",
            "行",
            "可以",
            "没问题",
            "ok",
            "👍",
            "👎",
            "yes.",
            "no.",
            "sure",
            "alright",
            "got it",
            "understood",
            "cool",
        ];

        if trivial_patterns.contains(&trimmed.as_str()) {
            return true;
        }

        // 斜杠命令也跳过 (不是自然语言查询)
        if trimmed.starts_with('/') {
            return true;
        }

        // 极短输入 (≤ 2 字符) 大概率无信号
        if trimmed.chars().count() <= 2 {
            return true;
        }

        false
    }

    /// 判断是否应跳过记忆 prefetch
    pub fn should_skip_prefetch(input: &str) -> bool {
        Self::is_trivial(input)
    }
}

// ── 技能脚手架剥离 ─────────────────────────────────────────────────

/// 技能脚手架剥离器
///
/// 当用户调用 /skill 时, 模型消息会嵌入整篇技能正文。
/// 必须先剥离出真实用户指令再喂给记忆 provider,
/// 防止记忆库被 prompt 脚手架污染。
pub struct SkillScaffoldStripper;

impl SkillScaffoldStripper {
    /// 检测内容是否包含技能脚手架
    pub fn contains_skill_scaffold(content: &str) -> bool {
        let lower = content.to_lowercase();

        // 检测 SKILL.md / 技能 frontmatter 模式
        let scaffold_patterns = [
            "## when to use",
            "## procedure",
            "## pitfalls",
            "## verification",
            "skill.md",
            "```yaml",
            "name:",
            "description:",
            "version:",
            "platforms:",
        ];

        scaffold_patterns.iter().any(|p| lower.contains(p))
    }

    /// 剥离技能脚手架, 提取真实用户指令
    pub fn strip_scaffold(content: &str) -> StrippedContent {
        if !Self::contains_skill_scaffold(content) {
            return StrippedContent {
                original: content.to_string(),
                stripped: content.to_string(),
                was_stripped: false,
                skill_name: None,
            };
        }

        let mut skill_name = None;
        let mut stripped_lines = Vec::new();
        let mut in_scaffold = false;

        for line in content.lines() {
            let lower = line.to_lowercase();

            // 检测技能名
            if lower.starts_with("name:") || lower.starts_with("name: ") {
                skill_name =
                    Some(line.split(':').nth(1).map(|s| s.trim().to_string()).unwrap_or_default());
                in_scaffold = true;
                continue;
            }

            // 检测 section 标题
            if (lower.starts_with("# ") || lower.starts_with("## "))
                && (lower.contains("when to use")
                    || lower.contains("procedure")
                    || lower.contains("pitfalls")
                    || lower.contains("verification"))
            {
                in_scaffold = true;
                continue;
            }

            // 跳过 YAML frontmatter
            if line.trim_start().starts_with("---") {
                in_scaffold = true;
                continue;
            }

            // 不在脚手架内的内容保留
            if !in_scaffold && !line.trim().is_empty() {
                stripped_lines.push(line.to_string());
            }

            // 遇到空行后重置脚手架状态 (下一段可能是用户指令)
            if line.trim().is_empty() && in_scaffold {
                in_scaffold = false;
            }
        }

        let stripped = stripped_lines.join("\n");
        let was_stripped = stripped.len() < content.len();

        StrippedContent {
            original: content.to_string(),
            stripped: if was_stripped {
                stripped
            } else {
                content.to_string()
            },
            was_stripped,
            skill_name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrippedContent {
    pub original: String,
    pub stripped: String,
    pub was_stripped: bool,
    pub skill_name: Option<String>,
}

// ── 记忆生命周期钩子 ───────────────────────────────────────────────

/// 记忆生命周期事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "hook_type")]
pub enum MemoryLifecycleEvent {
    /// 会话开始
    SessionStarted { session_id: String },
    /// 会话结束
    SessionEnded { session_id: String, messages_count: usize },
    /// Prefetch 前 (可用于取消/修改 prefetch)
    PrefetchRequested { session_id: String, query: String },
    /// Sync 后 (新记忆已写入)
    MemorySynced { session_id: String, memory_count: usize },
    /// 压缩前 (可用于保存状态)
    BeforeCompress { session_id: String },
    /// 压缩后
    AfterCompress { session_id: String, compressed: bool },
}

/// 记忆生命周期钩子 trait
#[async_trait]
pub trait MemoryLifecycleHook: Send + Sync {
    async fn on_event(&self, event: &MemoryLifecycleEvent) -> Result<(), String>;
}

/// 默认钩子实现 (空操作)
pub struct NoopMemoryHook;

#[async_trait]
impl MemoryLifecycleHook for NoopMemoryHook {
    async fn on_event(&self, _event: &MemoryLifecycleEvent) -> Result<(), String> {
        Ok(())
    }
}

// ── 记忆写审批门 ───────────────────────────────────────────────────

/// 记忆写审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWriteApprovalRequest {
    pub id: String,
    pub content: String,
    pub namespace: Option<String>,
    pub importance: u8,
    pub reason: String,
    pub requires_approval: bool,
}

/// 记忆写审批状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWriteApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

/// 记忆写审批门配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWriteApprovalConfig {
    /// 启用审批门
    pub enabled: bool,
    ///  importance 超过此阈值需要审批 (0-255)
    pub min_importance_for_approval: u8,
    /// 自动审批的命名空间白名单
    pub auto_approve_namespaces: Vec<String>,
}

impl Default for MemoryWriteApprovalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_importance_for_approval: 200,
            auto_approve_namespaces: vec!["system".to_string(), "session".to_string()],
        }
    }
}

impl MemoryWriteApprovalRequest {
    /// 判断是否需要审批
    pub fn requires_approval(&self, config: &MemoryWriteApprovalConfig) -> bool {
        if !config.enabled {
            return false;
        }

        // 白名单命名空间自动通过
        if let Some(ref ns) = self.namespace
            && config.auto_approve_namespaces.contains(ns)
        {
            return false;
        }

        // 重要性超过阈值需要审批
        self.importance >= config.min_importance_for_approval
    }
}

// ── MemoryStore trait (增强版) ─────────────────────────────────────

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn add_memory(&self, req: MemoryAddRequest) -> Result<MemoryActionResultDto, String>;
    async fn search_memories(
        &self,
        req: MemorySearchRequest,
    ) -> Result<Vec<MemorySearchItem>, String>;
    async fn get_memory_tree(&self) -> Result<Vec<MemoryTreeItem>, String>;
    async fn get_working_memory(&self) -> Result<Option<String>, String>;
    async fn get_grouped_memories(&self) -> Result<Vec<MemoryGroupedDto>, String>;
    async fn submit_feedback(
        &self,
        req: MemoryFeedbackRequest,
    ) -> Result<MemoryActionResultDto, String>;
    async fn delete_memory(&self, id: &str) -> Result<MemoryActionResultDto, String>;
    async fn update_memory(
        &self,
        req: MemoryUpdateRequest,
    ) -> Result<MemoryActionResultDto, String>;
}
