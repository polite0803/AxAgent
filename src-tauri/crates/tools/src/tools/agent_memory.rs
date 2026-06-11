// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 记忆与会话管理工具
//!
//! SessionSearch (FTS5 全文搜索), MemoryFlush (记忆持久化),
//! AgentCheckpoint, AgentStatus, AgentRemember

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Mutex;

fn truncate_text(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

static CHECKPOINTS: std::sync::LazyLock<Mutex<Vec<(String, String, String)>>> =
    std::sync::LazyLock::new(|| Mutex::new(Vec::new()));
static AGENT_MEMORY: std::sync::LazyLock<Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

// ── SessionSearch ──

pub struct SessionSearchTool;

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str {
        "SessionSearch"
    }
    fn description(&self) -> &str {
        "全文搜索历史会话记录（FTS5）。按关键词匹配，返回相关片段和会话 ID。用于查找之前的讨论、决策或错误。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","default":10},"db_path":{"type":"string"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let limit = input.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
        if query.is_empty() {
            return Ok(ToolResult::error("Error: query 是必需的"));
        }
        let db_path_str = match crate::global_state::get_db_path() {
            Some(p) => p,
            None => return Ok(ToolResult::error("会话搜索不可用：未配置数据库路径")),
        };
        let db_file = db_path_str.strip_prefix("sqlite:").unwrap_or(&db_path_str);
        let conn = rusqlite::Connection::open(db_file)
            .map_err(|e| ToolError::execution_failed(format!("打开数据库失败: {}", e)))?;
        let fts_sql = "SELECT m.conversation_id, snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, bm25(messages_fts) as rank FROM messages_fts JOIN messages m ON m.rowid = messages_fts.rowid WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?";
        let rows: Vec<String> = match conn.prepare(fts_sql) {
            Ok(mut stmt) => stmt
                .query_map(rusqlite::params![query, limit], |row| {
                    let conv_id: String = row.get(0)?;
                    let snippet: String = row.get(1)?;
                    Ok(format!("[{}] {}", conv_id, snippet))
                })
                .map_err(|e| ToolError::execution_failed(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect(),
            Err(e) => return Ok(ToolResult::error(format!("会话搜索错误 (FTS5 不可用): {}", e))),
        };
        if rows.is_empty() {
            Ok(ToolResult::success(format!("未找到 '{}' 的结果", query)))
        } else {
            Ok(ToolResult::success(format!(
                "搜索 '{}' ({} 条):\n{}",
                query,
                rows.len(),
                rows.join("\n")
            )))
        }
    }
}

// ── MemoryFlush ──

pub struct MemoryFlushTool;

#[async_trait]
impl Tool for MemoryFlushTool {
    fn name(&self) -> &str {
        "MemoryFlush"
    }
    fn description(&self) -> &str {
        "将 Agent 的记忆或洞察持久化到长期存储。target: memory(项目记忆)/user(用户偏好)。category: insight/decision/error_solution/preference/pattern/workflow。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"content":{"type":"string"},"target":{"type":"string","enum":["memory","user"],"default":"memory"},"category":{"type":"string","enum":["insight","decision","error_solution","preference","pattern","workflow"],"default":"insight"}},"required":["content"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let target = input
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("memory");
        let category = input
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("insight");
        if content.is_empty() {
            return Ok(ToolResult::error("Error: content 是必需的"));
        }

        let db = match crate::global_state::get_sea_db() {
            Some(db) => db,
            None => {
                tracing::warn!("MemoryFlush: database not available, content not persisted");
                return Ok(ToolResult::success(format!(
                    "记忆未持久化 (数据库不可用): target={}, category={}",
                    target, category
                )));
            },
        };

        let namespaces = axagent_core::repo::memory::list_namespaces(&db).await;
        let ns_id = match &namespaces {
            Ok(list) => list
                .iter()
                .find(|ns| ns.name == target || ns.id == target)
                .map(|ns| ns.id.clone())
                .or_else(|| list.first().map(|ns| ns.id.clone())),
            Err(e) => {
                tracing::warn!("MemoryFlush: failed to list namespaces: {}", e);
                return Ok(ToolResult::error(format!("查询命名空间失败: {}", e)));
            },
        };

        let Some(namespace_id) = ns_id else {
            return Ok(ToolResult::error("没有可用的记忆命名空间，请先创建命名空间"));
        };

        let title = format!("[{}] {}", category, &content[..content.len().min(50)]);

        let input = axagent_harness::types::CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content: content.to_string(),
            source: Some(format!("agent_flush:{}", category)),
        };

        match axagent_core::repo::memory::add_item(&db, input).await {
            Ok(item) => {
                tracing::info!(
                    "MemoryFlush: persisted item {} to namespace {}",
                    item.id,
                    namespace_id
                );
                Ok(ToolResult::success(format!(
                    "记忆已持久化 (target: {}, category: {}, id: {})",
                    target,
                    category,
                    &item.id[..8.min(item.id.len())]
                )))
            },
            Err(e) => {
                tracing::error!("MemoryFlush: failed to persist: {}", e);
                Ok(ToolResult::error(format!("记忆持久化失败: {}", e)))
            },
        }
    }
}

// ── AgentCheckpoint ──

pub struct AgentCheckpointTool;

#[async_trait]
impl Tool for AgentCheckpointTool {
    fn name(&self) -> &str {
        "AgentCheckpoint"
    }
    fn description(&self) -> &str {
        "创建 Agent 会话检查点。保存当前进度，支持后续恢复或回滚。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"label":{"type":"string"},"data":{"type":"string"}},"required":["label"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let label = input
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("checkpoint");
        let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        CHECKPOINTS
            .lock()
            .map_err(|e| ToolError::execution_failed(e.to_string()))?
            .push((label.to_string(), data.to_string(), ts.clone()));
        Ok(ToolResult::success(format!("检查点已保存: {} ({})", label, ts)))
    }
}

// ── AgentStatus ──

pub struct AgentStatusTool;

#[async_trait]
impl Tool for AgentStatusTool {
    fn name(&self) -> &str {
        "AgentStatus"
    }
    fn description(&self) -> &str {
        "查看当前 Agent 会话状态：检查点数量、记忆条目、最近操作。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let checkpoints = CHECKPOINTS
            .lock()
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let memory = AGENT_MEMORY
            .lock()
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let mut lines = vec!["## Agent 会话状态\n".to_string()];
        lines.push(format!("检查点: {}", checkpoints.len()));
        lines.push(format!("记忆条目: {}", memory.len()));
        if let Some(last) = checkpoints.last() {
            lines.push(format!("最近检查点: {} ({})", last.0, last.2));
        }
        if !memory.is_empty() {
            lines.push("存储的键:".to_string());
            for key in memory.keys() {
                lines.push(format!("  - {}", key));
            }
        }
        Ok(ToolResult::success(lines.join("\n")))
    }
}

// ── AgentRemember ──

pub struct AgentRememberTool;

#[async_trait]
impl Tool for AgentRememberTool {
    fn name(&self) -> &str {
        "AgentRemember"
    }
    fn description(&self) -> &str {
        "让 Agent 记住一条键值对信息，跨工具调用持久化在当前会话中。key 用于后续检索，value 为记忆内容。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"}},"required":["key","value"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = input
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let value = input
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if key.is_empty() {
            return Ok(ToolResult::error("Error: key 是必需的"));
        }
        AGENT_MEMORY
            .lock()
            .map_err(|e| ToolError::execution_failed(e.to_string()))?
            .insert(key.to_string(), value.to_string());
        Ok(ToolResult::success(format!("已记住: {} = {}", key, truncate_text(value, 200))))
    }
}
