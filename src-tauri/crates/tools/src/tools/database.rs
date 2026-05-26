//! 数据库管理工具
//!
//! DatabaseQuery / DatabaseListTables / DatabaseMigrationStatus

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::Value;
use std::sync::Arc;

fn get_db() -> Result<Arc<sea_orm::DatabaseConnection>, ToolError> {
    crate::global_state::get_sea_db()
        .ok_or_else(|| ToolError::execution_failed("数据库未初始化".to_string()))
}

// ── DatabaseQueryTool ──

pub struct DatabaseQueryTool;

#[async_trait]
impl Tool for DatabaseQueryTool {
    fn name(&self) -> &str {
        "DatabaseQuery"
    }
    fn description(&self) -> &str {
        "执行只读 SQL 查询。仅支持 SELECT/EXPLAIN/DESCRIBE/SHOW/PRAGMA/WITH。结果以调试格式返回。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "sql": { "type": "string", "description": "只读 SQL 查询语句" }
            },
            "required": ["sql"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let sql = input["sql"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input("缺少 sql 参数"))?;
        let trimmed = sql.trim().to_uppercase();
        let allowed = ["SELECT", "EXPLAIN", "DESCRIBE", "SHOW", "PRAGMA", "WITH"];
        if !allowed.iter().any(|p| trimmed.starts_with(p)) {
            return Err(ToolError::invalid_input(
                "仅允许只读查询 (SELECT/EXPLAIN/DESCRIBE/SHOW/PRAGMA/WITH)",
            ));
        }
        if sql.contains(';') {
            return Err(ToolError::invalid_input("SQL 语句不允许包含分号（防止多语句注入）"));
        }
        if sql.contains("--") || sql.contains("/*") {
            return Err(ToolError::invalid_input("SQL 语句不允许包含注释（防止注入）"));
        }
        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let sql = input["sql"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("DatabaseQuery", "缺少 sql 参数"))?;
        let trimmed = sql.trim().to_uppercase();
        let allowed = ["SELECT", "EXPLAIN", "DESCRIBE", "SHOW", "PRAGMA", "WITH"];
        if !allowed.iter().any(|p| trimmed.starts_with(p)) {
            return Err(ToolError::invalid_input(
                "仅允许只读查询 (SELECT/EXPLAIN/DESCRIBE/SHOW/PRAGMA/WITH)",
            ));
        }
        if sql.contains(';') || sql.contains("--") || sql.contains("/*") {
            return Err(ToolError::invalid_input("SQL 语句未通过安全验证"));
        }
        let db = get_db()?;

        let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);
        match db.query_one_raw(stmt).await {
            Ok(Some(row)) => Ok(ToolResult::success(format!("## 查询结果\n\n```\n{:?}\n```", row))),
            Ok(None) => Ok(ToolResult::success("查询返回空结果集")),
            Err(e) => Err(ToolError::execution_failed(format!("查询执行失败: {}", e))),
        }
    }
}

// ── DatabaseListTablesTool ──

pub struct DatabaseListTablesTool;

#[async_trait]
impl Tool for DatabaseListTablesTool {
    fn name(&self) -> &str {
        "DatabaseListTables"
    }
    fn description(&self) -> &str {
        "列出数据库中所有表，含行数统计。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let db = get_db()?;

        let stmt = Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        );
        let row = db
            .query_one_raw(stmt)
            .await
            .map_err(|e| ToolError::execution_failed(format!("查询表列表失败: {}", e)))?;

        Ok(ToolResult::success(format!(
            "## 数据库表 (首条)\n\n```\n{:?}\n```\n\n> 提示: 使用 DatabaseQuery 执行 SELECT * FROM sqlite_master WHERE type='table' 获取完整列表",
            row
        )))
    }
}

// ── DatabaseMigrationStatusTool ──

pub struct DatabaseMigrationStatusTool;

#[async_trait]
impl Tool for DatabaseMigrationStatusTool {
    fn name(&self) -> &str {
        "DatabaseMigrationStatus"
    }
    fn description(&self) -> &str {
        "检查数据库迁移状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let db = get_db()?;

        let stmt = Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version, name FROM seaql_migrations ORDER BY version DESC LIMIT 10",
        );
        match db.query_one_raw(stmt).await {
            Ok(Some(row)) => Ok(ToolResult::success(format!(
                "## 数据库迁移状态 (最近记录)\n\n```\n{:?}\n```\n\n> 提示: 使用 DatabaseQuery 获取完整列表",
                row
            ))),
            Ok(None) => Ok(ToolResult::success("## 数据库迁移状态\n\n无迁移记录")),
            Err(e) => Ok(ToolResult::success(format!(
                "迁移状态查询失败: {}。可能 seaql_migrations 表不存在。",
                e
            ))),
        }
    }
}
