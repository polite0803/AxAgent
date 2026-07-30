// SPDX-License-Identifier: AGPL-3.0-only
//! 知识图谱契约
use crate::types::rag_voice_etc::{CreateKnowledgeEntityInput, KnowledgeEntity, KnowledgeRelation};
use async_trait::async_trait;

#[async_trait]
pub trait EntityGraphProvider: Send + Sync {
    async fn get_entities(&self, kb_id: &str) -> Result<Vec<KnowledgeEntity>, String>;
    async fn search_entities(
        &self,
        kb_id: &str,
        query: &str,
    ) -> Result<Vec<KnowledgeEntity>, String>;
    async fn create_entity(
        &self,
        kb_id: &str,
        input: CreateKnowledgeEntityInput,
    ) -> Result<KnowledgeEntity, String>;
    async fn delete_entity(&self, entity_id: &str) -> Result<(), String>;
    async fn get_relations(&self, entity_id: &str) -> Result<Vec<KnowledgeRelation>, String>;
    async fn create_relation(
        &self,
        source_id: &str,
        target_id: &str,
        rel_type: &str,
    ) -> Result<KnowledgeRelation, String>;
    async fn delete_relation(&self, relation_id: &str) -> Result<(), String>;

    /// 核心方法：图增强检索
    /// 根据用户 Query 检索实体，并扩展其邻居关系，最终返回可直接注入 RAG 的上下文
    async fn graph_enhanced_search(
        &self,
        input: GraphEnhancedSearchInput,
    ) -> Result<GraphEnhancedSearchResult, String>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub description: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedRelation {
    pub source: String,
    pub target: String,
    pub relation_type: String,
}
#[async_trait]
pub trait EntityExtractor: Send + Sync {
    async fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>, String>;
    async fn extract_relations(
        &self,
        text: &str,
        entities: &[ExtractedEntity],
    ) -> Result<Vec<ExtractedRelation>, String>;
}

// ── LightRAG 跨文档实体抽取与图查询增强 DTO ──────────────────────────────

/// 跨文档实体抽取请求（调用方组装后传入）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractEntitiesFromDocumentsInput {
    pub knowledge_base_id: String,
    /// 待抽取的文档 ID 列表（最多 20 个，超出由调用方分批）
    pub document_ids: Vec<String>,
    /// 已抽取的 chunk 内容映射 document_id → Vec<chunk_content>
    /// 由调用方从 vector_store 加载后传入
    pub chunks_by_document: std::collections::HashMap<String, Vec<String>>,
    /// 已存在的实体列表（用于去重/合并判断），由调用方从 DAO 加载
    pub existing_entities: Vec<crate::types::KnowledgeEntity>,
}

/// 跨文档实体抽取结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractEntitiesResult {
    /// 新增的实体（已写入 DB，含最终 id）
    pub new_entities: Vec<crate::types::KnowledgeEntity>,
    /// 更新的实体（mention_count 累加 / properties 合并）
    pub updated_entities: Vec<crate::types::KnowledgeEntity>,
    /// 新增的关系
    pub new_relations: Vec<crate::types::KnowledgeRelation>,
    /// 跳过的 chunk 数（LLM 判定无实体）
    pub skipped_chunks: u32,
    /// 总耗时（毫秒）
    pub elapsed_ms: u64,
}

/// 图查询增强上下文片段
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEnhancedContextChunk {
    /// 实体名称
    pub entity_name: String,
    /// 实体类型
    pub entity_type: String,
    /// 实体描述
    pub description: Option<String>,
    /// 命中的关系列表
    pub relations: Vec<GraphRelationEdge>,
    /// 来源（哪个 KB 抽取的）
    pub knowledge_base_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRelationEdge {
    pub target_entity_name: String,
    pub relation_type: String,
    pub description: Option<String>,
    pub weight: f64,
}

/// 图查询增强请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEnhancedSearchInput {
    pub knowledge_base_id: String,
    pub query: String,

    /// 限制检索的实体类型 (e.g., ["company", "person"])
    /// 如果为空，则检索所有类型
    #[serde(default)]
    pub entity_type_filters: Vec<String>,

    /// 限制扩展的关系类型 (e.g., ["in_industry", "has_chairman"])
    /// 如果为空，则扩展所有关系
    #[serde(default)]
    pub relation_type_filters: Vec<String>,

    /// 最多返回的实体数（默认 10）
    pub top_k: Option<usize>,
    /// 是否包含 1-hop 邻居关系（默认 true）
    pub include_neighbors: Option<bool>,
}

/// 图查询增强结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEnhancedSearchResult {
    /// 命中的实体及其邻居关系
    pub entities: Vec<GraphEnhancedContextChunk>,
    /// 拼接好的上下文文本（可直接注入到 RAG context）
    pub context_text: String,
    /// 命中实体总数
    pub total_hits: usize,
}

/// 图上下文格式化器接口
/// 允许调用方自定义如何将实体关系网络转换为 LLM 可理解的文本格式
#[async_trait]
pub trait GraphContextFormatter: Send + Sync {
    /// 将图检索结果格式化为字符串
    async fn format_context(&self, result: &GraphEnhancedSearchResult) -> Result<String, String>;
}

/// 提供一个默认的简单格式化器
pub struct DefaultGraphFormatter;

#[async_trait]
impl GraphContextFormatter for DefaultGraphFormatter {
    async fn format_context(&self, result: &GraphEnhancedSearchResult) -> Result<String, String> {
        let mut context = String::new();
        for entity in &result.entities {
            context.push_str(&format!("【{} - {}】\n", entity.entity_type, entity.entity_name));
            if let Some(desc) = &entity.description {
                context.push_str(&format!("描述: {}\n", desc));
            }
            for rel in &entity.relations {
                context.push_str(&format!(
                    "- {} (关系: {})\n",
                    rel.target_entity_name, rel.relation_type
                ));
            }
            context.push('\n');
        }
        Ok(context)
    }
}
