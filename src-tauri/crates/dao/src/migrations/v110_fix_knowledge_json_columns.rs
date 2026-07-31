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
//! 本迁移将存量数据库（已在 v100 用 TEXT 建表的库）的 JSON 列改为 JSONB，
//! 并把已有 TEXT 数据通过 `USING col::jsonb` 显式转换为 JSONB。
//!
//! ## 幂等性
//!
//! - PostgreSQL：`ALTER TABLE ... ALTER COLUMN ... TYPE JSONB USING col::jsonb`
//!   在列已是 JSONB 时是 no-op（源类型 = 目标类型，USING 表达式为 identity）。
//! - SQLite：动态类型亲和性下 TEXT 存储 JSON 字符串即可被 SeaORM 正常
//!   序列化/反序列化，无需任何 ALTER（no-op）。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

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

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    tracing::info!("[v110] 开始修复知识图谱 JSON 列类型 (is_pg={})", is_pg);

    if is_pg {
        // PostgreSQL：将 TEXT 列改为 JSONB，已有数据用 `USING col::jsonb` 显式转换。
        for (table, column) in JSON_COLUMNS {
            let sql = format!(
                "ALTER TABLE {table} ALTER COLUMN {column} TYPE JSONB USING {column}::jsonb"
            );
            tracing::info!("[v110] 执行: {}", sql);
            db.execute_unprepared(&sql).await?;
        }
    } else {
        // SQLite：TEXT 存储 JSON 字符串即可兼容 SeaORM Json 类型，无需修改。
        tracing::info!("[v110] SQLite 下无需修改，TEXT 存储 JSON 字符串已兼容");
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
}
