// SPDX-License-Identifier: AGPL-3.0-only
//! v108_memory_applicability: 为 memory_items 表添加 applicability_tags + confirmed 字段。
//!
//! ## 背景
//!
//! 自进化闭环「标准记忆管理」要求:
//! - 划分记忆的适用范围边界(applicability_tags)
//! - 人工确认关键记忆的准确性(confirmed)
//!
//! Reflector 自动沉淀的经验默认 `confirmed=0`(未确认),
//! 仅在晋升到 core 层(promote_memory_entry)时需要 `confirmed=1` 门槛。
//! `applicability_tags` 为 JSON 数组字符串(如 `["rust","frontend"]`),
//! RAG 检索时可按当前任务上下文标签过滤,降低无关记忆干扰。
//!
//! 本迁移曾用 ALTER TABLE 加两列，现在由 v100 PHASE 3.9 全表合规检查统一处理，
//! 本迁移保留为无操作（兼容历史数据库标记此版本已应用）。

use sea_orm::DbErr;

pub async fn up(_db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("[v108] 列定义已由 v100 PHASE 3.9 统一保障，无需 ALTER TABLE");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[tokio::test]
    async fn v108_adds_applicability_and_confirmed_columns() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100（含 PHASE 3.9 合规检查）建 memory_items 表并补全所有列
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // 再跑 v108（现在为 no-op，但应不报错）
        up(db.clone()).await.unwrap();

        // 验证列存在：SELECT 0 行不会报错说明列存在
        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT applicability_tags FROM memory_items LIMIT 0",
            ))
            .await;
        assert!(result.is_ok(), "applicability_tags column should exist in memory_items");

        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT confirmed FROM memory_items LIMIT 0",
            ))
            .await;
        assert!(result.is_ok(), "confirmed column should exist in memory_items");
    }

    #[tokio::test]
    async fn v108_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：v108 现在为 no-op，重复跑不报错
        up(db).await.expect("v108 must be re-runnable in isolation");
    }
}
