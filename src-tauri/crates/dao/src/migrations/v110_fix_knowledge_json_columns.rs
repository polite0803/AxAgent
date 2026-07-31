// SPDX-License-Identifier: AGPL-3.0-only
//! v110_fix_knowledge_json_columns: 修复知识图谱表 JSON 列类型（TEXT → JSONB）
//!
//! ## 背景
//!
//! v100 建表时将知识图谱表（knowledge_entities / knowledge_relations /
//! knowledge_flows / knowledge_interfaces / knowledge_attributes）的 JSON 列
//! 定义为 TEXT，但 SeaORM entity 声明为 `Json` 类型（PostgreSQL 下映射为
//! JSONB），导致 PG 运行时类型不兼容错误：
//!
//! ```text
//! error occurred while decoding column "properties": mismatched types;
//! Rust type `Option<Json>` (as SQL type `JSONB`) is not compatible with SQL type `TEXT`
//! ```
//!
//! 本迁移处理两种场景：
//! 1. 表存在但列为 TEXT → 改为 JSONB，并把已有 TEXT 数据通过 `USING col::jsonb` 显式转换
//! 2. 表不存在 → 用正确的 JSONB/TEXT 列类型补建表（含 v101+ 追加的列）
//!
//! ## 幂等性
//!
//! - PostgreSQL：`ALTER TABLE ... ALTER COLUMN ... TYPE JSONB USING col::jsonb`
//!   在列已是 JSONB 时是 no-op（源类型 = 目标类型，USING 表达式为 identity）。
//! - SQLite：动态类型亲和性下 TEXT 存储 JSON 字符串即可被 SeaORM 正常
//!   序列化/反序列化，无需任何 ALTER（no-op）。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// 待修复的表与 JSON 列清单（表名, 列名）。
const JSON_COLUMNS: &[(&str, &str)] = &[
    // knowledge_entities
    ("knowledge_entities", "properties"),
    ("knowledge_entities", "lifecycle"),
    ("knowledge_entities", "behaviors"),
    ("knowledge_entities", "metadata"),
    // knowledge_relations
    ("knowledge_relations", "properties"),
    ("knowledge_relations", "metadata"),
    // knowledge_flows
    ("knowledge_flows", "steps"),
    ("knowledge_flows", "decision_points"),
    ("knowledge_flows", "error_handling"),
    ("knowledge_flows", "preconditions"),
    ("knowledge_flows", "postconditions"),
    ("knowledge_flows", "metadata"),
    // knowledge_interfaces
    ("knowledge_interfaces", "input_schema"),
    ("knowledge_interfaces", "output_schema"),
    ("knowledge_interfaces", "error_codes"),
    ("knowledge_interfaces", "metadata"),
    // knowledge_attributes
    ("knowledge_attributes", "constraints"),
    ("knowledge_attributes", "validation_rules"),
    ("knowledge_attributes", "metadata"),
];

/// 因 v100/v101 历史迁移中用了 REAL(f32) 而非 DOUBLE PRECISION(f64)，
/// 导致 SeaORM entity 的 f64 解码失败。此处统一修复为 DOUBLE PRECISION。
const FLOAT4_TO_FLOAT8_COLUMNS: &[(&str, &str)] = &[
    ("knowledge_entities", "confidence"),
    ("knowledge_relations", "weight"),
    ("memory_items", "importance"),
    ("memory_items", "decay_rate"),
];

/// 知识图谱表建表 DDL（PostgreSQL，JSON 列用 JSONB）。
/// 包含 v100 原始列 + v101 追加的轨迹合并列/权重列。
const CREATE_TABLES_PG: &[(&str, &str)] = &[
    (
        "knowledge_entities",
        "CREATE TABLE IF NOT EXISTS knowledge_entities (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            entity_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            source_language TEXT, properties JSONB NOT NULL, lifecycle JSONB, behaviors JSONB, \
            metadata JSONB, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            aliases TEXT NOT NULL DEFAULT '[]', mention_count INTEGER NOT NULL DEFAULT 1, \
            confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5, first_seen_at TEXT, last_seen_at TEXT)",
    ),
    (
        "knowledge_attributes",
        "CREATE TABLE IF NOT EXISTS knowledge_attributes (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            entity_id TEXT NOT NULL, name TEXT NOT NULL, attribute_type TEXT NOT NULL, \
            data_type TEXT NOT NULL, description TEXT, \
            is_required BOOLEAN NOT NULL DEFAULT FALSE, default_value TEXT, constraints JSONB, \
            validation_rules JSONB, metadata JSONB, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
    ),
    (
        "knowledge_relations",
        "CREATE TABLE IF NOT EXISTS knowledge_relations (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            source_entity_id TEXT NOT NULL, target_entity_id TEXT NOT NULL, \
            relation_type TEXT NOT NULL, description TEXT, properties JSONB, metadata JSONB, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            weight DOUBLE PRECISION NOT NULL DEFAULT 1.0)",
    ),
    (
        "knowledge_flows",
        "CREATE TABLE IF NOT EXISTS knowledge_flows (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            flow_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            steps JSONB NOT NULL, decision_points JSONB, error_handling JSONB, \
            preconditions JSONB, postconditions JSONB, metadata JSONB, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
    ),
    (
        "knowledge_interfaces",
        "CREATE TABLE IF NOT EXISTS knowledge_interfaces (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            interface_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            input_schema JSONB NOT NULL, output_schema JSONB NOT NULL, error_codes JSONB, \
            communication_pattern TEXT, version TEXT, metadata JSONB, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
    ),
];

/// 知识图谱表建表 DDL（SQLite，JSON 列用 TEXT）。
/// 结构与 PG 版一致，仅 JSONB → TEXT。
const CREATE_TABLES_SQLITE: &[(&str, &str)] = &[
    (
        "knowledge_entities",
        "CREATE TABLE IF NOT EXISTS knowledge_entities (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            entity_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            source_language TEXT, properties TEXT NOT NULL, lifecycle TEXT, behaviors TEXT, \
            metadata TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            aliases TEXT NOT NULL DEFAULT '[]', mention_count INTEGER NOT NULL DEFAULT 1, \
            confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5, first_seen_at TEXT, last_seen_at TEXT)",
    ),
    (
        "knowledge_attributes",
        "CREATE TABLE IF NOT EXISTS knowledge_attributes (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            entity_id TEXT NOT NULL, name TEXT NOT NULL, attribute_type TEXT NOT NULL, \
            data_type TEXT NOT NULL, description TEXT, \
            is_required INTEGER NOT NULL DEFAULT 0, default_value TEXT, constraints TEXT, \
            validation_rules TEXT, metadata TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
    ),
    (
        "knowledge_relations",
        "CREATE TABLE IF NOT EXISTS knowledge_relations (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            source_entity_id TEXT NOT NULL, target_entity_id TEXT NOT NULL, \
            relation_type TEXT NOT NULL, description TEXT, properties TEXT, metadata TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            weight DOUBLE PRECISION NOT NULL DEFAULT 1.0)",
    ),
    (
        "knowledge_flows",
        "CREATE TABLE IF NOT EXISTS knowledge_flows (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            flow_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            steps TEXT NOT NULL, decision_points TEXT, error_handling TEXT, \
            preconditions TEXT, postconditions TEXT, metadata TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
    ),
    (
        "knowledge_interfaces",
        "CREATE TABLE IF NOT EXISTS knowledge_interfaces (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            interface_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            input_schema TEXT NOT NULL, output_schema TEXT NOT NULL, error_codes TEXT, \
            communication_pattern TEXT, version TEXT, metadata TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
    ),
];

/// 检查表是否存在（PostgreSQL 走 information_schema，SQLite 走 sqlite_master）。
async fn table_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT COUNT(*) AS cnt FROM information_schema.tables WHERE table_name = '{}'",
            table
        )
    } else {
        format!("SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='{}'", table)
    };
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    match row {
        Some(r) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        None => Ok(false),
    }
}

/// 根据表名获取对应的建表 DDL。
fn get_create_table_sql(table: &str, is_pg: bool) -> Option<&'static str> {
    let tables = if is_pg {
        CREATE_TABLES_PG
    } else {
        CREATE_TABLES_SQLITE
    };
    tables.iter().find(|(t, _)| *t == table).map(|(_, sql)| *sql)
}

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    tracing::info!("[v110] 开始修复知识图谱 JSON 列类型 (is_pg={})", is_pg);

    // 记录已补建的表，避免重复建表
    let mut created_tables: Vec<&str> = Vec::new();

    // 第一步：修复 JSON 列类型（TEXT → JSONB）
    for (table, column) in JSON_COLUMNS {
        // 已补建的表跳过（刚建好，列类型已正确）
        if created_tables.contains(&table) {
            continue;
        }

        // 检查表是否存在
        if !table_exists(&db, table, is_pg).await? {
            // 表不存在 → 补建表
            if let Some(ddl) = get_create_table_sql(table, is_pg) {
                tracing::warn!("[v110] 表 {} 不存在，补建表", table);
                db.execute_unprepared(ddl).await?;
                created_tables.push(table);
            } else {
                tracing::error!("[v110] 表 {} 无对应建表 DDL，跳过", table);
            }
            continue;
        }

        // 表存在 → 按数据库类型处理列类型
        if is_pg {
            // PostgreSQL：将 TEXT 列改为 JSONB
            let sql = format!(
                "ALTER TABLE {table} ALTER COLUMN {column} TYPE JSONB USING {column}::jsonb"
            );
            tracing::info!("[v110] 执行: {}", sql);
            db.execute_unprepared(&sql).await?;
        } else {
            // SQLite：TEXT 存储 JSON 字符串即可兼容，无需修改
            tracing::debug!("[v110] SQLite 下表 {} 列 {} 无需修改", table, column);
        }
    }

    // 第二步：修复 f32 → f64 列类型（REAL → DOUBLE PRECISION）
    // 历史迁移 v100/v101 用了 REAL(f32)，但 SeaORM entity 声明为 f64，
    // 导致解码错误："mismatched types; FLOAT4 is not compatible with FLOAT8"
    if is_pg {
        for (table, column) in FLOAT4_TO_FLOAT8_COLUMNS {
            // 这些列可能属于 memory_items，不在知识图谱表列表中
            if !table_exists(&db, table, is_pg).await? {
                tracing::warn!("[v110] 表 {} 不存在，跳过 REAL→DOUBLE PRECISION 修复", table);
                continue;
            }
            let sql = format!(
                "ALTER TABLE {table} ALTER COLUMN {column} TYPE DOUBLE PRECISION USING {column}::double precision"
            );
            tracing::info!("[v110] 修复 REAL→DOUBLE PRECISION: {}", sql);
            match db.execute_unprepared(&sql).await {
                Ok(_) => {},
                Err(e) => {
                    // 列可能已是 DOUBLE PRECISION，ALTER TYPE 相同类型时 PG 会报错
                    // 用 USING col::double precision 可以处理 REAL→DOUBLE PRECISION，
                    // 但如果已经是 DOUBLE PRECISION，USING 转换是 identity 不会报错的
                    // 这里仅记录警告
                    tracing::warn!("[v110] REAL→DOUBLE PRECISION 跳过 {} {}: {}", table, column, e);
                },
            }
        }
    }

    tracing::info!("[v110] 知识图谱 JSON 列类型修复完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, Statement};

    #[tokio::test]
    async fn v110_can_run_on_fresh_sqlite_db() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // v110 在 SQLite 下应该无操作成功
        up(db.clone()).await.unwrap();

        // 验证列存在且可读取
        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT properties FROM knowledge_entities LIMIT 0".to_string(),
            ))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn v110_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 再跑一次也应该成功
        up(db.clone()).await.unwrap();
    }

    #[tokio::test]
    async fn v110_can_create_tables_when_missing() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 不跑 v100，直接跑 v110，应该能补建表
        up(db.clone()).await.unwrap();

        // 验证表和列存在
        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT properties FROM knowledge_entities LIMIT 0".to_string(),
            ))
            .await;
        assert!(result.is_ok());

        // 验证 v101 追加的列也存在
        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT aliases, confidence FROM knowledge_entities LIMIT 0".to_string(),
            ))
            .await;
        assert!(result.is_ok());

        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT weight FROM knowledge_relations LIMIT 0".to_string(),
            ))
            .await;
        assert!(result.is_ok());
    }
}
