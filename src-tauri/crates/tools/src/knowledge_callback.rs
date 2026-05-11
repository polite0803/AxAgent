//! 知识搜索回调桥接
//!
//! 提供全局知识搜索回调的注册和获取，供 knowledge.rs 和 state.rs 使用。
//! 从 builtin_handlers.rs 迁移而来。

use axagent_core::error::AxAgentError;

/// 知识库搜索命中条目
pub struct KnowledgeSearchHit {
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f32,
}

/// 全局知识搜索回调（全 RAG pipeline，含 embedding + vector store）
#[allow(clippy::type_complexity)]
static CALLBACK: std::sync::OnceLock<
    std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>,
                        > + Send
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
                    dyn std::future::Future<
                            Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>,
                        > + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
) {
    let _ = CALLBACK.set(cb);
}

/// 获取全局知识搜索回调
pub fn get_knowledge_search_callback() -> Option<
    std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = std::result::Result<Vec<KnowledgeSearchHit>, AxAgentError>,
                        > + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
> {
    CALLBACK.get().cloned()
}
