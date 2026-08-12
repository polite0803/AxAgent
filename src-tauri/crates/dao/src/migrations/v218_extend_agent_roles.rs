// SPDX-License-Identifier: AGPL-3.0-only

//! v218: agent_roles 扩展字段
//!
//! 给 agent_roles 补齐 8 个扩展字段，供股票分析/OPC 等业务种子化使用。
//!
//! 幂等：列存在性先检查再操作；重复执行安全。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    let new_cols: &[(&str, &str)] = &[
        ("responsibilities", "TEXT"),
        ("decision_authority", "TEXT"),
        ("reports_to", "TEXT"),
        ("managed_expert_ids", "TEXT"),
        ("required_certifications", "TEXT"),
        ("icon", "TEXT"),
        ("color", "TEXT"),
        ("is_enabled", "INTEGER NOT NULL DEFAULT 1"),
    ];
    for (col, ty) in new_cols {
        let exists = column_exists(&db, is_pg, "agent_roles", col).await?;
        if !exists {
            db.execute_unprepared(&format!("ALTER TABLE agent_roles ADD COLUMN {col} {ty}"))
                .await?;
        }
    }

    Ok(())
}

async fn column_exists(
    db: &sea_orm::DatabaseConnection,
    is_pg: bool,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT 1 AS f FROM information_schema.columns \
                     WHERE table_schema = current_schema() \
                       AND table_name = '{table}' AND column_name = '{column}'"
                ),
            ))
            .await?;
        Ok(row.is_some())
    } else {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                [table.into()],
            ))
            .await?;
        Ok(rows
            .iter()
            .any(|r| r.try_get_by::<String, _>("name").map(|n| n == column).unwrap_or(false)))
    }
}
