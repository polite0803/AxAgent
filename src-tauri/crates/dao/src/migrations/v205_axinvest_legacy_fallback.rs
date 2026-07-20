// SPDX-License-Identifier: AGPL-3.0-only

//! v205: AxInvest 旧库升级兜底（字段缺失 + 类型不匹配一次性修复）
//!
//! ## 背景
//!
//! AxInvest 在多次迭代中累积了若干 entity / schema 类型不一致或字段缺失问题，
//! 全部因「v100 已应用、补丁未跑」导致旧库永远没被修复：
//!
//! 1. **字段缺失**（commit `a7dbaaf3e` 在 v100 PHASE 3.7 才补，但旧库已跳过 v100）：
//!    - `agent_roles.active_domains`
//!    - `agency_experts.recommended_workflows`
//!    - `agency_experts.recommended_tools`
//!    - `agency_experts.active_domains`
//!
//! 2. **类型不匹配**（commit `5dee35a5c` 把 entity 从 `Option<bool>` 改为
//!    `Option<i32>`，但 v100 PHASE 2 仍建 BOOLEAN 列，repo 按 0/1 写入）：
//!    - `agent_profiles.search_enabled`（PG: BOOLEAN ≠ INTEGER）
//!
//! 本 v205 一次性兜底修复以上所有问题，幂等安全。
//!
//! ## 为何是 v205 而非 v204？
//!
//! v204 的第一版（只有 PHASE 1 字段缺失修复）可能已被旧库应用过，schema_version
//! 表里有 v204 记录。后续修改 v204 内容追加 PHASE 2 类型修正，但 run_migrations
//! 看到 applied_max >= 204 就跳过 v204，PHASE 2 永远不会跑。
//!
//! 改用全新版本号 v205，确保无论 v204 是否应用过，v205 都会跑一次。
//!
//! ## 幂等性
//!
//! - 字段缺失：PG `ADD COLUMN IF NOT EXISTS`；SQLite 吞重复列错误。
//! - 类型修正：PG 先查 `information_schema.columns.data_type`，仅在类型不符时
//!   ALTER；SQLite 动态类型，no-op。
//! - PG 不支持 `boolean::INTEGER` 直接 cast，USING 表达式用 `CASE WHEN` 转换。
//! - ALTER TYPE 前必须先 DROP DEFAULT：旧列可能有 boolean 默认值（如 true/false），
//!   PG 无法自动将 boolean DEFAULT 转换为 integer，会报
//!   「字段 xxx 的默认值不能转换成类型 integer」。
//!   entity 为 `Option<i32>`，不需要 DEFAULT，改完后不重新 SET DEFAULT。
//!
//! ## 为何不改 v100？
//!
//! v100 是合并迁移快照，已应用的库不会重跑 v100。改 v100 PHASE 2 / 3.6 / 3.7
//! 只能修新装库，已有库仍需 v205 兜底。为保持迁移不可变性，仅以 v205 修复所有库。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// 字段缺失目标列表（与 v100 PHASE 3.7 MISSING_COLUMN_TARGETS 保持一致）。
///
/// 若 v100 新增条目，本数组必须同步追加。
const MISSING_COLUMN_TARGETS: &[(&str, &str, &str)] = &[
    ("agent_roles", "active_domains", "TEXT"),
    ("agency_experts", "recommended_workflows", "TEXT"),
    ("agency_experts", "recommended_tools", "TEXT"),
    ("agency_experts", "active_domains", "TEXT"),
];

/// 类型不匹配目标列表：(table, column, 期望的 PG 类型, 转换 USING 表达式)。
///
/// 当 PG 检测到当前类型与 `expected_pg_type` 不一致时，执行
/// `ALTER COLUMN TYPE <expected_pg_type> USING <using_expr>`。
///
/// 注意：PG 不支持 `boolean::INTEGER` 直接 cast，必须用 `CASE WHEN` 转换。
const TYPE_MISMATCH_TARGETS: &[(&str, &str, &str, &str)] = &[(
    "agent_profiles",
    "search_enabled",
    "INTEGER",
    "(CASE WHEN search_enabled THEN 1 ELSE 0 END)",
)];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let is_pg = backend == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 补缺失字段
    // ========================================================================

    for (table, column, col_type) in MISSING_COLUMN_TARGETS {
        if is_pg {
            let sql = format!("ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {column} {col_type}");
            db.execute_unprepared(&sql).await?;
        } else {
            // SQLite: ADD COLUMN 不支持 IF NOT EXISTS，吞掉重复列错误实现幂等
            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}");
            let _ = db.execute_raw(Statement::from_string(backend, sql)).await;
        }
    }

    // ========================================================================
    // PHASE 2: 修正类型不匹配（仅 PG）
    // ========================================================================

    if is_pg {
        for (table, column, expected_type, using_expr) in TYPE_MISMATCH_TARGETS {
            let row = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    format!(
                        "SELECT data_type FROM information_schema.columns \
                         WHERE table_schema = current_schema() \
                           AND table_name = '{table}' AND column_name = '{column}'"
                    ),
                ))
                .await?;

            let Some(row) = row else {
                // 列不存在：由 PHASE 1 或 v100 负责建，本处跳过
                continue;
            };

            let current_type: String = row.try_get_by("data_type").unwrap_or_default();

            // 期望 PG 内部类型名（如 "integer"），与 expected_type 大小写无关
            let expected_lower = expected_type.to_lowercase();
            if current_type.to_lowercase() == expected_lower {
                continue;
            }

            // 先 DROP DEFAULT：旧列可能有 boolean 默认值（true/false），
            // PG 无法自动将 boolean DEFAULT 转为 integer，ALTER TYPE 会失败。
            // entity 是 Option<i32>，不需要 DEFAULT，改完后不重新 SET。
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} ALTER COLUMN {column} DROP DEFAULT"
            ))
            .await?;

            let sql = format!(
                "ALTER TABLE {table} ALTER COLUMN {column} TYPE {expected_type} USING {using_expr}"
            );
            db.execute_unprepared(&sql).await?;

            tracing::info!("[v205] {}.{}: {} → {}", table, column, current_type, expected_type);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v205_is_idempotent_on_fresh_sqlite() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        // 先创建 agent_roles / agency_experts 表（v205 不负责建表）
        db.execute_unprepared(
            "CREATE TABLE agent_roles (\
             id TEXT PRIMARY KEY, name TEXT, description TEXT, \
             system_prompt TEXT, default_tools TEXT, \
             max_concurrent INTEGER, timeout_seconds INTEGER, \
             source TEXT, sort_order INTEGER, \
             created_at INTEGER, updated_at INTEGER)",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "CREATE TABLE agency_experts (\
             id TEXT PRIMARY KEY, name TEXT, description TEXT, \
             system_prompt TEXT, expert_type TEXT, \
             created_at INTEGER, updated_at INTEGER)",
        )
        .await
        .unwrap();

        // 第一次跑：补 4 个字段
        up(db.clone()).await.unwrap();
        // 第二次跑：所有字段已存在，吞错误，不报错
        up(db.clone()).await.unwrap();

        // 验证字段存在（PRAGMA table_info 返回所有列名）
        let rows = db
            .query_all_raw(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA table_info(agent_roles)",
            ))
            .await
            .unwrap();
        let has_active_domains = rows
            .iter()
            .any(|r| r.try_get_by::<String, &str>("name").unwrap_or_default() == "active_domains");
        assert!(has_active_domains, "agent_roles.active_domains should exist");
    }
}
