// SPDX-License-Identifier: AGPL-3.0-only
//! 知识图谱契约
use crate::types::rag_voice_etc::{CreateKnowledgeEntityInput, KnowledgeEntity, KnowledgeRelation};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 多源节点类型 ─────────────────────────────────────────────

/// 知识图谱节点来源类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphSourceType {
    /// RAG 知识库实体
    KnowledgeBase,
    /// Wiki 笔记
    Wiki,
    /// 记忆条目
    Memory,
    /// Obsidian Vault 笔记
    ObsidianVault,
}

impl GraphSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KnowledgeBase => "knowledge_base",
            Self::Wiki => "wiki",
            Self::Memory => "memory",
            Self::ObsidianVault => "obsidian_vault",
        }
    }
}

impl std::str::FromStr for GraphSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wiki" => Ok(Self::Wiki),
            "memory" => Ok(Self::Memory),
            "obsidian_vault" => Ok(Self::ObsidianVault),
            "knowledge_base" => Ok(Self::KnowledgeBase),
            _ => Err(format!("unknown GraphSourceType: {}", s)),
        }
    }
}

/// 知识图谱节点类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphNodeType {
    /// 知识库实体（原有）
    Entity,
    /// Wiki 笔记
    Note,
    /// 记忆条目
    MemoryItem,
    /// Obsidian Vault 笔记
    ObsidianNote,
}

impl GraphNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Note => "note",
            Self::MemoryItem => "memory_item",
            Self::ObsidianNote => "obsidian_note",
        }
    }
}

// ── 扩展的创建输入 ──────────────────────────────────────────

/// 创建多源实体输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMultiSourceEntityInput {
    /// 实体 ID（可选，不提供则自动生成）
    pub id: Option<String>,
    /// 知识库 ID（兼容原有字段）
    pub knowledge_base_id: String,
    /// 节点来源类型
    pub source_type: GraphSourceType,
    /// 源 ID（kb_id / wiki_id / namespace_id / vault_id）
    pub source_id: String,
    /// 节点类型
    pub node_type: GraphNodeType,
    /// 外部系统 ID（如 wiki note_id / memory_id / obsidian note path）
    pub external_id: Option<String>,
    /// 实体名称
    pub name: String,
    /// 实体类型（如 person / organization / concept）
    pub entity_type: String,
    /// 描述
    pub description: Option<String>,
    /// 来源路径
    pub source_path: String,
    /// 属性 JSON
    pub properties: Option<serde_json::Value>,
    /// 别名
    pub aliases: Option<Vec<String>>,
}

// ── EntityGraphProvider trait ──────────────────────────────

/// 因果边在 `knowledge_relations.relation_type` 上的保留取值。
///
/// 因果边是行为统计（工具序列 / 意图转移），不是文档知识：
/// RAG 图检索必须排除它，避免污染 `graph_enhanced_search` 结果。
/// 权威定义在契约层，trajectory（写入方）与 dao（检索方）都引用此处。
pub const CAUSAL_RELATION_TYPE: &str = "causes";

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

    // ── 多源扩展方法（默认实现，便于渐进式采用）──────────────

    /// 按来源类型获取实体
    async fn get_entities_by_source(
        &self,
        source_type: GraphSourceType,
        source_id: &str,
    ) -> Result<Vec<KnowledgeEntity>, String> {
        // 默认实现：调用方需覆盖
        let _ = (source_type, source_id);
        Ok(vec![])
    }

    /// 创建多源实体
    async fn create_multi_source_entity(
        &self,
        input: CreateMultiSourceEntityInput,
    ) -> Result<KnowledgeEntity, String> {
        // 默认实现：转换为原有 CreateKnowledgeEntityInput 并调用
        let kb_id = input.knowledge_base_id.clone();
        let name = input.name.clone();
        let entity_type = input.entity_type.clone();
        let description = input.description.clone();
        let source_path = input.source_path.clone();
        let source_language = Some("zh-CN".to_string());
        let properties =
            input.properties.unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let inner_input = CreateKnowledgeEntityInput {
            knowledge_base_id: kb_id.clone(),
            name,
            entity_type,
            description,
            source_path,
            source_language,
            properties,
            lifecycle: None,
            behaviors: None,
            metadata: None,
        };
        self.create_entity(&kb_id, inner_input).await
    }

    /// 按节点类型搜索实体
    async fn search_entities_by_node_type(
        &self,
        query: &str,
        node_type: GraphNodeType,
        source_id: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>, String> {
        let _ = (query, node_type, source_id, top_k);
        Ok(vec![])
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub description: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
