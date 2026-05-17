//! 知识搜索回调桥接
//!
//! 提供全局知识搜索回调的注册和获取，供 knowledge.rs 和 state.rs 使用。
//! 从 builtin_handlers.rs 迁移而来。

use axagent_core::error::AxAgentError;

pub struct KnowledgeSearchHit {
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f32,
}

pub type KnowledgeSearchFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>>
            + Send
            + 'static,
    >,
>;

pub type KnowledgeSearchCallback =
    std::sync::Arc<dyn Fn(&str, &str, usize) -> KnowledgeSearchFuture + Send + Sync>;

static CALLBACK: std::sync::OnceLock<KnowledgeSearchCallback> = std::sync::OnceLock::new();

pub fn set_knowledge_search_callback(cb: KnowledgeSearchCallback) {
    let _ = CALLBACK.set(cb);
}

pub fn get_knowledge_search_callback() -> Option<KnowledgeSearchCallback> {
    CALLBACK.get().cloned()
}
