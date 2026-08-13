//! 全局概念索引种子化 — 启动时构建 ConceptIndex 供主题解析使用
//!
//! 流程：
//! 1. 创建空的 ConceptIndex
//! 2. 注册 A 股行业/概念本体（49 行业 + 163 概念）
//! 3. 从 DB 加载 lemonhu 知识库成员关系（若仍为空则静默降级）

use std::path::Path;

use sea_orm::DatabaseConnection;

use axagent_analysis_engine::concept_index::{ConceptIndex, seed_ashare_ontology};
use axagent_analysis_engine::knowledge_loader::load_concept_index_from_db;

/// 构建全局 ConceptIndex
///
/// 注册基线本体（行业 + 概念），从数据库加载 lemonhu 的成员关系。
/// 若知识库为空或加载失败，返回仅含本体的空索引（成员数据由动态搜索兜底）。
pub(crate) async fn ensure_concept_index(db: &DatabaseConnection, _app_dir: &Path) -> ConceptIndex {
    let mut idx = ConceptIndex::new();

    // 1. 基线本体：49 行业 + 163 概念（同花顺分类）
    seed_ashare_ontology(&mut idx);

    // 2. 从 DB 加载 lemonhu 成员关系
    match load_concept_index_from_db(&mut idx, db).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("[concept_index] 从 lemonhu 知识库加载 {count} 条成员关系成功");
            } else {
                tracing::warn!("[concept_index] lemonhu 知识库无成员关系，返回基线索引");
            }
        },
        Err(e) => {
            tracing::warn!("[concept_index] 从 DB 加载概念索引失败（非致命）: {e}，返回基线索引");
        },
    }

    idx
}
