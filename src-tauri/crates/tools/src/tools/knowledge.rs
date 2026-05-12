//! 知识库管理工具
//!
//! 将 builtin_handlers 中的 list_knowledge_bases、search_knowledge、
//! create_knowledge_entity/flow/interface、add_knowledge_document 迁移为 Tool trait。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::entity::{
    knowledge_documents, knowledge_entities, knowledge_flows, knowledge_interfaces,
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

// ── 辅助函数 ──

fn sea_db() -> Result<std::sync::Arc<sea_orm::DatabaseConnection>, ToolError> {
    crate::global_state::get_sea_db().ok_or_else(|| ToolError::execution_failed("数据库未初始化"))
}

fn db_path() -> Result<String, ToolError> {
    crate::global_state::get_db_path()
        .ok_or_else(|| ToolError::execution_failed("数据库路径未初始化"))
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn current_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

// ── ListKnowledgeBasesTool ─────────────────────────────────────────────────

pub struct ListKnowledgeBasesTool;

#[async_trait]
impl Tool for ListKnowledgeBasesTool {
    fn name(&self) -> &str {
        "ListKnowledgeBases"
    }

    fn description(&self) -> &str {
        "列出所有可用的知识库，包括名称、ID 和启用状态。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let raw_path = db_path()?;
        let db_file = raw_path.strip_prefix("sqlite:").unwrap_or(&raw_path);

        let conn = rusqlite::Connection::open(db_file)
            .map_err(|e| ToolError::execution_failed(format!("打开数据库失败: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT id, name, description, enabled FROM knowledge_bases ORDER BY sort_order, name")
            .map_err(|e| ToolError::execution_failed(format!("查询知识库失败: {}", e)))?;

        let rows: Vec<String> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let desc: Option<String> = row.get(2)?;
                let enabled: i32 = row.get(3)?;
                let status = if enabled != 0 { "enabled" } else { "disabled" };
                let desc_str = desc.map(|d| format!(" - {}", d)).unwrap_or_default();
                Ok(format!("- {} [{}] ({}){}", name, id, status, desc_str))
            })
            .map_err(|e| ToolError::execution_failed(format!("读取知识库列表失败: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            Ok(ToolResult::success("未找到知识库。请在 设置 > 知识库 中创建。"))
        } else {
            Ok(ToolResult::success(format!(
                "可用知识库 ({}):\n{}",
                rows.len(),
                rows.join("\n")
            )))
        }
    }
}

// ── SearchKnowledgeTool ────────────────────────────────────────────────────

pub struct SearchKnowledgeTool;

#[async_trait]
impl Tool for SearchKnowledgeTool {
    fn name(&self) -> &str {
        "SearchKnowledge"
    }

    fn description(&self) -> &str {
        "在指定知识库中搜索相关内容。支持语义搜索（通过向量嵌入）和文本匹配回退。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "base_id": {
                    "type": "string",
                    "description": "知识库 ID"
                },
                "query": {
                    "type": "string",
                    "description": "搜索查询"
                },
                "top_k": {
                    "type": "integer",
                    "description": "返回的结果数",
                    "default": 5
                }
            },
            "required": ["base_id", "query"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let base_id = input
            .get("base_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let top_k = input
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);

        if query.is_empty() {
            return Ok(ToolResult::error("Error: query 参数是必需的"));
        }

        // 尝试回调优先（全 RAG pipeline）
        if let Some(cb) = crate::knowledge_callback::get_knowledge_search_callback() {
            match cb(&base_id, &query, top_k).await {
                Ok(hits) => {
                    if hits.is_empty() {
                        return Ok(ToolResult::success(format!(
                            "在知识库 '{}' 中未找到 '{}' 的结果",
                            base_id, query
                        )));
                    }
                    let lines: Vec<String> = hits
                        .iter()
                        .map(|h| format!("[score={:.3}] {}", h.score, h.content))
                        .collect();
                    return Ok(ToolResult::success(format!(
                        "在 '{}' 中搜索 '{}' ({} 条结果):\n{}",
                        base_id,
                        query,
                        hits.len(),
                        lines.join("\n\n")
                    )));
                },
                Err(e) => {
                    return Ok(ToolResult::error(format!("知识库搜索错误: {}", e)));
                },
            }
        }

        // 回退：文本匹配
        let raw_path = db_path()?;
        let db_file = raw_path.strip_prefix("sqlite:").unwrap_or(&raw_path);
        let conn = rusqlite::Connection::open(db_file)
            .map_err(|e| ToolError::execution_failed(format!("打开数据库失败: {}", e)))?;

        let meta_table = format!("vec_kb_{}_meta", base_id);
        let sql =
            format!("SELECT content FROM {} WHERE content LIKE ? LIMIT {}", meta_table, top_k);
        let like_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            ToolError::execution_failed(format!("知识库 '{}' 可能不存在或未索引: {}", base_id, e))
        })?;

        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![like_pattern], |row| {
                let content: String = row.get(0)?;
                Ok(content)
            })
            .map_err(|e| ToolError::execution_failed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            Ok(ToolResult::success(format!(
                "在知识库 '{}' 中未找到 '{}' 的文本匹配",
                base_id, query
            )))
        } else {
            Ok(ToolResult::success(format!(
                "在 '{}' 中文本搜索 '{}' ({} 条结果，无语义排序):\n{}",
                base_id,
                query,
                rows.len(),
                rows.join("\n\n")
            )))
        }
    }
}

// ── CreateKnowledgeEntityTool ──────────────────────────────────────────────

pub struct CreateKnowledgeEntityTool;

#[async_trait]
impl Tool for CreateKnowledgeEntityTool {
    fn name(&self) -> &str {
        "CreateKnowledgeEntity"
    }

    fn description(&self) -> &str {
        "在知识库中创建实体条目，记录代码实体（类、函数、模块等）的结构化信息。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "knowledge_base_id": {"type": "string", "description": "知识库 ID"},
                "name": {"type": "string", "description": "实体名称"},
                "entity_type": {"type": "string", "description": "实体类型 (class, function, module 等)", "default": "entity"},
                "description": {"type": "string", "description": "描述"},
                "source_path": {"type": "string", "description": "源文件路径"},
                "source_language": {"type": "string", "description": "编程语言"},
                "properties": {"type": "object", "description": "属性"},
                "lifecycle": {"type": "object", "description": "生命周期方法"},
                "behaviors": {"type": "object", "description": "行为方法"}
            },
            "required": ["knowledge_base_id", "name"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kb_id = input
            .get("knowledge_base_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if kb_id.is_empty() {
            return Ok(ToolResult::error("Error: knowledge_base_id 是必需的"));
        }
        if name.is_empty() {
            return Ok(ToolResult::error("Error: name 是必需的"));
        }

        let db = sea_db()?;
        let id = generate_uuid();
        let now = current_timestamp();

        let am = knowledge_entities::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(kb_id.to_string()),
            name: Set(name.to_string()),
            entity_type: Set(input
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("entity")
                .to_string()),
            description: Set(input
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())),
            source_path: Set(input
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()),
            source_language: Set(input
                .get("source_language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())),
            properties: Set(input.get("properties").cloned().unwrap_or(Value::Null)),
            lifecycle: Set(input.get("lifecycle").cloned()),
            behaviors: Set(input.get("behaviors").cloned()),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match am.insert(db.as_ref()).await {
            Ok(_) => Ok(ToolResult::success(format!(
                "已创建知识实体 '{}' (id: {}) 在知识库 '{}' 中",
                name, id, kb_id
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建知识实体失败: {}", e))),
        }
    }
}

// ── CreateKnowledgeFlowTool ────────────────────────────────────────────────

pub struct CreateKnowledgeFlowTool;

#[async_trait]
impl Tool for CreateKnowledgeFlowTool {
    fn name(&self) -> &str {
        "CreateKnowledgeFlow"
    }

    fn description(&self) -> &str {
        "在知识库中创建工作流条目，记录业务流程、数据处理流水线等。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "knowledge_base_id": {"type": "string"},
                "name": {"type": "string"},
                "flow_type": {"type": "string", "default": "process"},
                "description": {"type": "string"},
                "source_path": {"type": "string"},
                "steps": {"type": "object"},
                "decision_points": {"type": "object"},
                "error_handling": {"type": "object"},
                "preconditions": {"type": "object"},
                "postconditions": {"type": "object"}
            },
            "required": ["knowledge_base_id", "name"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kb_id = input
            .get("knowledge_base_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if kb_id.is_empty() {
            return Ok(ToolResult::error("Error: knowledge_base_id 是必需的"));
        }
        if name.is_empty() {
            return Ok(ToolResult::error("Error: name 是必需的"));
        }

        let db = sea_db()?;
        let id = generate_uuid();
        let now = current_timestamp();

        let am = knowledge_flows::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(kb_id.to_string()),
            name: Set(name.to_string()),
            flow_type: Set(input
                .get("flow_type")
                .and_then(|v| v.as_str())
                .unwrap_or("process")
                .to_string()),
            description: Set(input
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())),
            source_path: Set(input
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()),
            steps: Set(input.get("steps").cloned().unwrap_or(Value::Null)),
            decision_points: Set(input.get("decision_points").cloned()),
            error_handling: Set(input.get("error_handling").cloned()),
            preconditions: Set(input.get("preconditions").cloned()),
            postconditions: Set(input.get("postconditions").cloned()),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match am.insert(db.as_ref()).await {
            Ok(_) => Ok(ToolResult::success(format!(
                "已创建知识流程 '{}' (id: {}) 在知识库 '{}' 中",
                name, id, kb_id
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建知识流程失败: {}", e))),
        }
    }
}

// ── CreateKnowledgeInterfaceTool ───────────────────────────────────────────

pub struct CreateKnowledgeInterfaceTool;

#[async_trait]
impl Tool for CreateKnowledgeInterfaceTool {
    fn name(&self) -> &str {
        "CreateKnowledgeInterface"
    }

    fn description(&self) -> &str {
        "在知识库中创建接口条目，记录 API 接口、函数签名等。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "knowledge_base_id": {"type": "string"},
                "name": {"type": "string"},
                "interface_type": {"type": "string", "default": "api"},
                "description": {"type": "string"},
                "source_path": {"type": "string"},
                "input_schema": {"type": "object"},
                "output_schema": {"type": "object"},
                "error_codes": {"type": "object"},
                "communication_pattern": {"type": "string"}
            },
            "required": ["knowledge_base_id", "name"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kb_id = input
            .get("knowledge_base_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if kb_id.is_empty() {
            return Ok(ToolResult::error("Error: knowledge_base_id 是必需的"));
        }
        if name.is_empty() {
            return Ok(ToolResult::error("Error: name 是必需的"));
        }

        let db = sea_db()?;
        let id = generate_uuid();
        let now = current_timestamp();

        let am = knowledge_interfaces::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(kb_id.to_string()),
            name: Set(name.to_string()),
            interface_type: Set(input
                .get("interface_type")
                .and_then(|v| v.as_str())
                .unwrap_or("api")
                .to_string()),
            description: Set(input
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())),
            source_path: Set(input
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()),
            input_schema: Set(input.get("input_schema").cloned().unwrap_or(Value::Null)),
            output_schema: Set(input.get("output_schema").cloned().unwrap_or(Value::Null)),
            error_codes: Set(input.get("error_codes").cloned()),
            communication_pattern: Set(input
                .get("communication_pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())),
            version: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match am.insert(db.as_ref()).await {
            Ok(_) => Ok(ToolResult::success(format!(
                "已创建知识接口 '{}' (id: {}) 在知识库 '{}' 中",
                name, id, kb_id
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建知识接口失败: {}", e))),
        }
    }
}

// ── AddKnowledgeDocumentTool ───────────────────────────────────────────────

pub struct AddKnowledgeDocumentTool;

#[async_trait]
impl Tool for AddKnowledgeDocumentTool {
    fn name(&self) -> &str {
        "AddKnowledgeDocument"
    }

    fn description(&self) -> &str {
        "向知识库添加文档，内容会被索引以供后续检索。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "knowledge_base_id": {"type": "string"},
                "title": {"type": "string"},
                "content": {"type": "string"}
            },
            "required": ["knowledge_base_id", "title", "content"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kb_id = input
            .get("knowledge_base_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if kb_id.is_empty() {
            return Ok(ToolResult::error("Error: knowledge_base_id 是必需的"));
        }
        if title.is_empty() {
            return Ok(ToolResult::error("Error: title 是必需的"));
        }
        if content.is_empty() {
            return Ok(ToolResult::error("Error: content 是必需的"));
        }

        let db = sea_db()?;

        let temp_dir = std::env::temp_dir();
        let doc_id = generate_uuid();
        let file_path = temp_dir.join(format!("kb_doc_{}.md", doc_id));

        std::fs::write(&file_path, content)
            .map_err(|e| ToolError::execution_failed(format!("写入临时文件失败: {}", e)))?;

        let id = generate_uuid();
        let now = current_timestamp();
        let file_path_str = file_path.to_string_lossy().to_string();

        let am = knowledge_documents::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(kb_id.to_string()),
            title: Set(title.to_string()),
            source_path: Set(file_path_str),
            mime_type: Set("text/markdown".to_string()),
            size_bytes: Set(content.len() as i64),
            indexing_status: Set("pending".to_string()),
            doc_type: Set("markdown".to_string()),
            index_error: Set(None),
            source_conversation_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match am.insert(db.as_ref()).await {
            Ok(_) => Ok(ToolResult::success(format!(
                "已添加文档 '{}' (id: {}) 到知识库 '{}'",
                title, id, kb_id
            ))),
            Err(e) => Ok(ToolResult::error(format!("添加知识文档失败: {}", e))),
        }
    }
}
