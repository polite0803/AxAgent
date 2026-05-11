//! 已精简：仅保留知识搜索回调和元数据类型
//!
//! 所有工具 handler 已迁移至 tools/*.rs 下的 Tool trait 实现。
//! dispatch()、init_builtin_handlers() 等均已移除。
//! 知识搜索回调暂时保留，供 knowledge.rs 和 state.rs 使用。

use axagent_core::error::AxAgentError;
use serde::{Deserialize, Serialize};

/// 技能元数据（供测试使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// 知识库搜索命中条目
pub struct KnowledgeSearchHit {
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f32,
}

/// 全局知识搜索回调（全 RAG pipeline）
#[allow(clippy::type_complexity)]
static KNOWLEDGE_SEARCH_CALLBACK: std::sync::OnceLock<
    std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>>
                        + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
> = std::sync::OnceLock::new();

/// 设置全局知识搜索回调。应用启动时调用一次。
#[allow(clippy::type_complexity)]
pub fn set_knowledge_search_callback(
    cb: std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>>
                        + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
) {
    let _ = KNOWLEDGE_SEARCH_CALLBACK.set(cb);
}

/// 获取全局知识搜索回调（供 knowledge.rs 等新 Tool trait 实现使用）
pub fn get_knowledge_search_callback() -> Option<
    std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            )
                -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>>
                        + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
> {
    KNOWLEDGE_SEARCH_CALLBACK.get().cloned()
}
