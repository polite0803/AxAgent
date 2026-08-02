// SPDX-License-Identifier: AGPL-3.0-only
//! v115_fleet_member_agent_profile: 为 fleet_members 表添加 agent_profile_id 字段。
//!
//! ## 背景
//!
//! Fleet 成员的智能体身份此前仅由自由文本 `role` 承载。现按项目「用
//! AgentProfile 定义智能体（角色 + 专家组合）」的设计收敛：成员通过
//! `agent_profile_id` 关联 `agent_profiles` 表，执行器据此组合出系统提示词。
//!
//! 本迁移加一列：
//! - `agent_profile_id TEXT`：关联的 AgentProfile ID；NULL 表示旧成员
//!   （仍回退自由文本 role），不建外键（profile 删除后成员仍可保留）。
//!
//! ## 幂等
//!
//! SQLite: ALTER TABLE 不支持 IF NOT EXISTS，需先查 PRAGMA；
//! PostgreSQL: 用 `ADD COLUMN IF NOT EXISTS`。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE fleet_members ADD COLUMN IF NOT EXISTS agent_profile_id TEXT",
        )
        .await?;
    } else {
        let existing_cols = existing_columns(&db, "fleet_members").await?;
        if !existing_cols.iter().any(|c| c == "agent_profile_id") {
            db.execute_unprepared("ALTER TABLE fleet_members ADD COLUMN agent_profile_id TEXT")
                .await?;
        }
    }

    Ok(())
}

/// 查询指定表的所有列名（SQLite 走 PRAGMA，PG 走 information_schema）
async fn existing_columns(
    db: &sea_orm::DatabaseConnection,
    table: &str,
) -> Result<Vec<String>, DbErr> {
    let backend = db.get_database_backend();
    let rows = match backend {
        DbBackend::Sqlite => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        DbBackend::Postgres => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT column_name AS name FROM information_schema.columns \
                 WHERE table_name = $1",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        _ => return Ok(vec![]),
    };

    let mut cols = Vec::with_capacity(rows.len());
    for row in rows {
        if let Ok(name) = row.try_get_by::<String, _>("name") {
            cols.push(name);
        }
    }
    Ok(cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v115_adds_agent_profile_id_column() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v102 建 fleet_members 表
        super::super::v102_create_fleets::up(db.clone()).await.unwrap();
        // 再跑 v115
        up(db.clone()).await.unwrap();

        let cols = existing_columns(&db, "fleet_members").await.unwrap();
        assert!(
            cols.iter().any(|c| c == "agent_profile_id"),
            "agent_profile_id column should exist"
        );
    }

    #[tokio::test]
    async fn v115_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v102_create_fleets::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：列已存在，应跳过 ALTER，不报错
        up(db).await.expect("v115 must be re-runnable in isolation");
    }
}
