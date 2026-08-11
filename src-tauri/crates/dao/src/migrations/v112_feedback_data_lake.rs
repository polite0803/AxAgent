// SPDX-License-Identifier: AGPL-3.0-only
//! v112_feedback_data_lake: 新建反馈数据湖相关表。
//!
//! ## 背景
//!
//! 为建立统一的反馈数据湖，整合以下四类反馈数据：
//! - retrieval_hits（已存在，由 v100 + v111 创建/扩展）
//! - tool_call_logs（新建）：工具调用记录，用于分析工具使用模式和成功率
//! - memory_access_logs（新建）：记忆访问记录，用于优化记忆检索策略
//! - wiki_edit_logs（新建）：Wiki 编辑记录，用于追踪 AI 对 Wiki 的修改
//!
//! 这些数据共同作为 RL 训练和自适应优化的数据基础。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// tool_call_logs 表建表 SQL
const CREATE_TOOL_CALL_LOGS: &str = r#"
CREATE TABLE IF NOT EXISTS tool_call_logs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT,
    trajectory_id TEXT,
    step_index INTEGER NOT NULL DEFAULT 0,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL DEFAULT '{}',
    result TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    related_source_id TEXT,
    created_at BIGINT NOT NULL DEFAULT 0
)"#;

/// memory_access_logs 表建表 SQL
const CREATE_MEMORY_ACCESS_LOGS: &str = r#"
CREATE TABLE IF NOT EXISTS memory_access_logs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT,
    namespace_id TEXT NOT NULL,
    memory_id TEXT NOT NULL,
    access_type TEXT NOT NULL,
    query TEXT,
    content_snippet TEXT,
    hit INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0
)"#;

/// wiki_edit_logs 表建表 SQL
const CREATE_WIKI_EDIT_LOGS: &str = r#"
CREATE TABLE IF NOT EXISTS wiki_edit_logs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT,
    wiki_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    before_snippet TEXT,
    after_snippet TEXT,
    reason TEXT,
    quality_score REAL,
    created_at BIGINT NOT NULL DEFAULT 0
)"#;

/// tool_call_logs 的索引
const CREATE_TOOL_CALL_LOGS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_tool_call_logs_tool_name ON tool_call_logs(tool_name)",
    "CREATE INDEX IF NOT EXISTS idx_tool_call_logs_conversation ON tool_call_logs(conversation_id)",
    "CREATE INDEX IF NOT EXISTS idx_tool_call_logs_created ON tool_call_logs(created_at)",
    "CREATE INDEX IF NOT EXISTS idx_tool_call_logs_success ON tool_call_logs(success) WHERE success = 0",
];

/// memory_access_logs 的索引
const CREATE_MEMORY_ACCESS_LOGS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_memory_access_logs_namespace ON memory_access_logs(namespace_id)",
    "CREATE INDEX IF NOT EXISTS idx_memory_access_logs_conversation ON memory_access_logs(conversation_id)",
    "CREATE INDEX IF NOT EXISTS idx_memory_access_logs_created ON memory_access_logs(created_at)",
];

/// wiki_edit_logs 的索引
const CREATE_WIKI_EDIT_LOGS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wiki_edit_logs_wiki ON wiki_edit_logs(wiki_id)",
    "CREATE INDEX IF NOT EXISTS idx_wiki_edit_logs_note ON wiki_edit_logs(note_id)",
    "CREATE INDEX IF NOT EXISTS idx_wiki_edit_logs_conversation ON wiki_edit_logs(conversation_id)",
    "CREATE INDEX IF NOT EXISTS idx_wiki_edit_logs_created ON wiki_edit_logs(created_at)",
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let is_pg = backend == DbBackend::Postgres;

    tracing::info!("[v112] 开始创建反馈数据湖表 (is_pg={})", is_pg);

    // 创建 tool_call_logs 表
    tracing::info!("[v112] 创建 tool_call_logs 表");
    db.execute_unprepared(CREATE_TOOL_CALL_LOGS).await?;
    for idx_sql in CREATE_TOOL_CALL_LOGS_INDEXES {
        let _ = db.execute_raw(Statement::from_string(backend, idx_sql.to_string())).await;
    }

    // 创建 memory_access_logs 表
    tracing::info!("[v112] 创建 memory_access_logs 表");
    db.execute_unprepared(CREATE_MEMORY_ACCESS_LOGS).await?;
    for idx_sql in CREATE_MEMORY_ACCESS_LOGS_INDEXES {
        let _ = db.execute_raw(Statement::from_string(backend, idx_sql.to_string())).await;
    }

    // 创建 wiki_edit_logs 表
    tracing::info!("[v112] 创建 wiki_edit_logs 表");
    db.execute_unprepared(CREATE_WIKI_EDIT_LOGS).await?;
    for idx_sql in CREATE_WIKI_EDIT_LOGS_INDEXES {
        let _ = db.execute_raw(Statement::from_string(backend, idx_sql.to_string())).await;
    }

    tracing::info!("[v112] 反馈数据湖表创建完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v112_creates_all_feedback_tables() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        // 验证 tool_call_logs 表存在
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='tool_call_logs'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let cnt: i64 = row.try_get_by("cnt").expect("测试应成功");
        assert_eq!(cnt, 1, "tool_call_logs table should exist");

        // 验证 memory_access_logs 表存在
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='memory_access_logs'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let cnt: i64 = row.try_get_by("cnt").expect("测试应成功");
        assert_eq!(cnt, 1, "memory_access_logs table should exist");

        // 验证 wiki_edit_logs 表存在
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='wiki_edit_logs'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let cnt: i64 = row.try_get_by("cnt").expect("测试应成功");
        assert_eq!(cnt, 1, "wiki_edit_logs table should exist");
    }

    #[tokio::test]
    async fn v112_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        // 第二次跑：CREATE TABLE IF NOT EXISTS 应跳过
        up(db.clone()).await.expect("v112 must be re-runnable without error");
    }

    #[tokio::test]
    async fn v112_can_insert_and_query_feedback_records() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        // 插入 tool_call_logs 记录
        db.execute_unprepared(
            "INSERT INTO tool_call_logs (id, conversation_id, trajectory_id, step_index, tool_name, \
             arguments, result, success, duration_ms, related_source_id, created_at) \
             VALUES ('tc1', 'conv1', 'traj1', 0, 'web_search', \
             '{\"query\": \"test\"}', '{\"results\": []}', 1, 150, 'kb1', 1700000000)"
        )
        .await
        .expect("测试应成功");

        // 验证 tool_call_logs 数据
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT tool_name, success, duration_ms FROM tool_call_logs WHERE id = 'tc1'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let tool_name: String = row.try_get_by("tool_name").expect("测试应成功");
        let success: i32 = row.try_get_by("success").expect("测试应成功");
        let duration_ms: i64 = row.try_get_by("duration_ms").expect("测试应成功");
        assert_eq!(tool_name, "web_search");
        assert_eq!(success, 1);
        assert_eq!(duration_ms, 150);

        // 插入 memory_access_logs 记录
        db.execute_unprepared(
            "INSERT INTO memory_access_logs (id, conversation_id, namespace_id, memory_id, \
             access_type, query, hit, created_at) \
             VALUES ('ma1', 'conv1', 'ns1', 'mem1', 'read', 'test query', 1, 1700000000)",
        )
        .await
        .expect("测试应成功");

        // 验证 memory_access_logs 数据
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT namespace_id, access_type, hit FROM memory_access_logs WHERE id = 'ma1'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let ns_id: String = row.try_get_by("namespace_id").expect("测试应成功");
        let access_type: String = row.try_get_by("access_type").expect("测试应成功");
        let hit: i32 = row.try_get_by("hit").expect("测试应成功");
        assert_eq!(ns_id, "ns1");
        assert_eq!(access_type, "read");
        assert_eq!(hit, 1);

        // 插入 wiki_edit_logs 记录
        db.execute_unprepared(
            "INSERT INTO wiki_edit_logs (id, conversation_id, wiki_id, note_id, operation, \
             reason, quality_score, created_at) \
             VALUES ('we1', 'conv1', 'wiki1', 'note1', 'update', 'ai_generated', 0.85, 1700000000)",
        )
        .await
        .expect("测试应成功");

        // 验证 wiki_edit_logs 数据
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT wiki_id, operation, quality_score FROM wiki_edit_logs WHERE id = 'we1'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let wiki_id: String = row.try_get_by("wiki_id").expect("测试应成功");
        let operation: String = row.try_get_by("operation").expect("测试应成功");
        let quality_score: f64 = row.try_get_by("quality_score").expect("测试应成功");
        assert_eq!(wiki_id, "wiki1");
        assert_eq!(operation, "update");
        assert!((quality_score - 0.85).abs() < 0.001);
    }
}
