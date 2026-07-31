// SPDX-License-Identifier: AGPL-3.0-only

//! 反馈数据湖统一接口
//!
//! 整合 retrieval_hits / tool_call_logs / memory_access_logs / wiki_edit_logs 四类反馈数据，
//! 作为 RL 训练和自适应优化的统一数据基础。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;

// ── 反馈事件类型 ─────────────────────────────────────────────

/// 反馈事件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeedbackEventType {
    /// 检索命中（retrieval_hits）
    RetrievalHit,
    /// 工具调用（tool_call_logs）
    ToolCall,
    /// 记忆访问（memory_access_logs）
    MemoryAccess,
    /// Wiki 编辑（wiki_edit_logs）
    WikiEdit,
}

impl FeedbackEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RetrievalHit => "retrieval_hit",
            Self::ToolCall => "tool_call",
            Self::MemoryAccess => "memory_access",
            Self::WikiEdit => "wiki_edit",
        }
    }
}

// ── 统一反馈事件 ─────────────────────────────────────────────

/// 统一反馈事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: FeedbackEventType,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 会话/会话 ID
    pub session_id: Option<String>,
    /// 关联的知识源 ID（kb_id / wiki_id / namespace_id 等）
    pub source_id: Option<String>,
    /// 关联的源类型
    pub source_type: Option<String>,
    /// 事件数据（各类型的具体字段）
    pub payload: serde_json::Value,
    /// 创建时间戳（Unix 毫秒）
    pub created_at: i64,
}

// ── 检索命中反馈 ─────────────────────────────────────────────

/// 检索命中记录（与 retrieval_hits 表对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalHitRecord {
    /// 检索 ID（UUID）
    pub id: String,
    /// 会话 ID
    pub conversation_id: String,
    /// 消息 ID
    pub message_id: String,
    /// 知识库 ID
    pub knowledge_base_id: String,
    /// 文档 ID
    pub document_id: String,
    /// chunk 引用
    pub chunk_ref: String,
    /// 相关性分数
    pub score: f64,
    /// 摘要
    pub preview: String,
    /// 用户反馈：'positive' / 'negative' / 'irrelevant'
    pub feedback: Option<String>,
    /// 反馈时间戳
    pub feedback_at: Option<i64>,
    /// 是否在最终回复中被引用
    pub used_in_response: bool,
    /// 重排后分数
    pub score_after_rerank: Option<f64>,
    /// 创建时间戳
    pub created_at: i64,
}

// ── 工具调用反馈 ─────────────────────────────────────────────

/// 工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// 调用 ID（UUID）
    pub id: String,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 轨迹 ID
    pub trajectory_id: Option<String>,
    /// 步骤索引
    pub step_index: i32,
    /// 工具名称
    pub tool_name: String,
    /// 调用参数（JSON）
    pub arguments: serde_json::Value,
    /// 调用结果（JSON）
    pub result: Option<serde_json::Value>,
    /// 是否成功
    pub success: bool,
    /// 耗时（毫秒）
    pub duration_ms: u64,
    /// 相关上下文/知识源 ID
    pub related_source_id: Option<String>,
    /// 创建时间戳
    pub created_at: i64,
}

// ── 记忆访问反馈 ─────────────────────────────────────────────

/// 记忆访问记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccessRecord {
    /// 访问 ID（UUID）
    pub id: String,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 记忆命名空间 ID
    pub namespace_id: String,
    /// 记忆条目 ID
    pub memory_id: String,
    /// 访问类型：'read' / 'write' / 'search'
    pub access_type: String,
    /// 查询文本（搜索时）
    pub query: Option<String>,
    /// 记忆内容摘要
    pub content_snippet: Option<String>,
    /// 是否命中
    pub hit: bool,
    /// 创建时间戳
    pub created_at: i64,
}

// ── Wiki 编辑反馈 ─────────────────────────────────────────────

/// Wiki 编辑记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEditRecord {
    /// 编辑 ID（UUID）
    pub id: String,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// Wiki ID
    pub wiki_id: String,
    /// 笔记 ID
    pub note_id: String,
    /// 操作类型：'create' / 'update' / 'delete' / 'append'
    pub operation: String,
    /// 编辑前内容（摘要）
    pub before_snippet: Option<String>,
    /// 编辑后内容（摘要）
    pub after_snippet: Option<String>,
    /// 变更原因（AI 生成 / 用户手动）
    pub reason: Option<String>,
    /// 质量分数（0.0 ~ 1.0）
    pub quality_score: Option<f64>,
    /// 创建时间戳
    pub created_at: i64,
}

// ── 反馈查询条件 ─────────────────────────────────────────────

/// 反馈查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackQuery {
    /// 事件类型过滤
    pub event_types: Option<Vec<FeedbackEventType>>,
    /// 会话 ID 过滤
    pub conversation_id: Option<String>,
    /// 源 ID 过滤
    pub source_id: Option<String>,
    /// 源类型过滤
    pub source_type: Option<String>,
    /// 开始时间戳（Unix 毫秒）
    pub start_time: Option<i64>,
    /// 结束时间戳（Unix 毫秒）
    pub end_time: Option<i64>,
    /// 最大返回数
    pub limit: Option<u32>,
    /// 偏移量
    pub offset: Option<u32>,
}

// ── 反馈数据湖 trait ──────────────────────────────────────────

/// 反馈数据湖统一接口
///
/// 整合四类反馈数据的写入和查询，作为 RL 训练和自适应优化的数据基础。
/// 实现方负责将数据写入对应的数据库表。
#[async_trait]
pub trait FeedbackDataLake: Send + Sync {
    // ── 写入接口 ──────────────────────────────────────────

    /// 写入检索命中记录
    async fn insert_retrieval_hit(&self, record: RetrievalHitRecord) -> Result<()>;

    /// 批量写入检索命中记录
    async fn batch_insert_retrieval_hits(&self, records: Vec<RetrievalHitRecord>) -> Result<()>;

    /// 已存在的检索命中记录上附加反馈（不再重复插入）
    async fn update_retrieval_hit_feedback(
        &self,
        hit_id: &str,
        feedback: Option<&str>,
        used_in_response: Option<bool>,
    ) -> Result<()>;

    /// 写入工具调用记录
    async fn insert_tool_call(&self, record: ToolCallRecord) -> Result<()>;

    /// 批量写入工具调用记录
    async fn batch_insert_tool_calls(&self, records: Vec<ToolCallRecord>) -> Result<()>;

    /// 写入记忆访问记录
    async fn insert_memory_access(&self, record: MemoryAccessRecord) -> Result<()>;

    /// 写入 Wiki 编辑记录
    async fn insert_wiki_edit(&self, record: WikiEditRecord) -> Result<()>;

    // ── 查询接口 ──────────────────────────────────────────

    /// 按条件查询反馈事件
    async fn query_feedback(&self, filter: FeedbackQuery) -> Result<Vec<FeedbackEvent>>;

    /// 查询检索命中记录
    async fn query_retrieval_hits(&self, filter: FeedbackQuery) -> Result<Vec<RetrievalHitRecord>>;

    /// 查询工具调用记录
    async fn query_tool_calls(&self, filter: FeedbackQuery) -> Result<Vec<ToolCallRecord>>;

    /// 查询记忆访问记录
    async fn query_memory_access(&self, filter: FeedbackQuery) -> Result<Vec<MemoryAccessRecord>>;

    /// 查询 Wiki 编辑记录
    async fn query_wiki_edits(&self, filter: FeedbackQuery) -> Result<Vec<WikiEditRecord>>;

    // ── 统计接口 ──────────────────────────────────────────

    /// 统计反馈事件数量（按类型分组）
    async fn count_by_event_type(&self, filter: FeedbackQuery) -> Result<HashMap<String, u64>>;

    /// 获取正反馈率（positive / total）
    async fn positive_feedback_rate(&self, knowledge_base_id: &str, since: i64) -> Result<f64>;
}

// ── 反馈数据湖注册表 ──────────────────────────────────────────

use std::collections::HashMap;

/// 反馈数据湖注册表
///
/// 在 wiring 层启动时注册 `FeedbackDataLake` 实现，
/// 各模块通过该注册表获取统一的反馈数据访问接口。
pub struct FeedbackDataLakeRegistry {
    lake: Option<Arc<dyn FeedbackDataLake>>,
}

impl FeedbackDataLakeRegistry {
    pub fn new() -> Self {
        Self { lake: None }
    }

    pub fn register(&mut self, lake: Arc<dyn FeedbackDataLake>) {
        self.lake = Some(lake);
    }

    pub fn get(&self) -> Option<Arc<dyn FeedbackDataLake>> {
        self.lake.clone()
    }
}

impl Default for FeedbackDataLakeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::Arc;
use std::sync::OnceLock;

/// 全局反馈数据湖注册表（进程内单例）
static GLOBAL_LAKE: OnceLock<Arc<dyn FeedbackDataLake>> = OnceLock::new();

/// 在 wiring 层启动时注册全局 FeedbackDataLake 实现
pub fn register_feedback_lake(lake: Arc<dyn FeedbackDataLake>) {
    if GLOBAL_LAKE.set(lake).is_err() {
        tracing::warn!("尝试重复注册全局 FeedbackDataLake — 已忽略，只有第一次注册生效");
    }
}

/// 获取全局注册的 FeedbackDataLake 实现
pub fn global_feedback_lake() -> Option<Arc<dyn FeedbackDataLake>> {
    GLOBAL_LAKE.get().cloned()
}
