// SPDX-License-Identifier: AGPL-3.0-only
//! v203_price_alerts_align_monitor: price_alerts 表与 RealtimeMonitor 告警模型对齐
//!
//! ## 背景
//!
//! 修复前存在三套不一致的告警类型：
//! - DB entity 注释: `above` / `below` (2 种)
//! - 前端 Select.Option: `above` / `below` / `change_up` / `change_down` / `volume_spike` (5 种)
//! - RealtimeMonitor alert_type: `stop_loss` / `take_profit` / `resistance` / `support` / `change` / `volume` (6 种)
//!
//! `monitor_emitter.rs` 用 6→2 降级映射导致 `change` / `volume` 两类告警
//! 无法持久化到 price_alerts 表，历史复盘永久丢失。
//!
//! ## 本迁移做的事
//!
//! - 新增 `alert_type` 列（6 类语义，对齐 RealtimeMonitor）
//! - 新增 `condition_type` 列（阈值语义: `price` / `change_pct` / `turnover_rate`）
//! - 新增 `threshold` 列（通用阈值）
//! - 数据回填: 老 `condition` 值映射到新 `alert_type` + `condition_type=price` + `threshold=target_price`
//! - 新增索引 `idx_price_alerts_alert_type`
//!
//! 老列 `condition` / `target_price` 保留兼容，新代码读 `alert_type` / `threshold`。
//!
//! ## DDL 风格
//!
//! 与 v200/v201/v202 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。
//! ALTER TABLE ADD COLUMN 在 SQLite/PG 语法相同，无需适配。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // PHASE 1: 新增 alert_type 列（6 类语义，对齐 RealtimeMonitor）
    add_column_if_not_exists(&db, "ALTER TABLE price_alerts ADD COLUMN alert_type TEXT").await?;

    // PHASE 2: 新增 condition_type 列（阈值语义: price / change_pct / turnover_rate）
    add_column_if_not_exists(&db, "ALTER TABLE price_alerts ADD COLUMN condition_type TEXT")
        .await?;

    // PHASE 3: 新增 threshold 列（通用阈值，替代 target_price 的单一语义）
    add_column_if_not_exists(&db, "ALTER TABLE price_alerts ADD COLUMN threshold DOUBLE PRECISION")
        .await?;

    // PHASE 4: 数据回填 — 老数据 condition → alert_type + condition_type + threshold
    // above → take_profit / condition_type=price / threshold=target_price
    // below → stop_loss   / condition_type=price / threshold=target_price
    // 其他老值（change_up/change_down/volume_spike 等历史脏数据）保守映射为 change/volume
    db.execute_unprepared(
        "UPDATE price_alerts \
         SET alert_type = CASE \
             WHEN condition = 'above' THEN 'take_profit' \
             WHEN condition = 'below' THEN 'stop_loss' \
             WHEN condition IN ('change_up', 'change_down') THEN 'change' \
             WHEN condition = 'volume_spike' THEN 'volume' \
             ELSE condition \
         END, \
         condition_type = CASE \
             WHEN condition IN ('above', 'below') THEN 'price' \
             WHEN condition IN ('change_up', 'change_down') THEN 'change_pct' \
             WHEN condition = 'volume_spike' THEN 'turnover_rate' \
             ELSE 'price' \
         END, \
         threshold = target_price \
         WHERE alert_type IS NULL",
    )
    .await?;

    // PHASE 5: 索引（按 alert_type 查询，复盘报告常用）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_price_alerts_alert_type \
         ON price_alerts(alert_type)",
    )
    .await?;

    Ok(())
}

/// ALTER TABLE ADD COLUMN 在 SQLite/PG 都不支持 IF NOT EXISTS，
/// 列已存在时直接忽略错误。
async fn add_column_if_not_exists(
    db: &sea_orm::DatabaseConnection,
    sql: &str,
) -> Result<(), DbErr> {
    match db.execute_unprepared(sql).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            // 兼容中英文错误消息：PostgreSQL 中文本地化返回 "已经存在"
            if msg.contains("duplicate column")
                || msg.contains("already exists")
                || msg.contains("已经存在")
            {
                Ok(())
            } else {
                Err(e)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v203_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // price_alerts 表由 v200_axinvest_stock_tables 创建，非 v100
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        super::super::v200_axinvest_stock_tables::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：列已存在错误被忽略，UPDATE 命中 0 行，索引 IF NOT EXISTS 幂等
        up(db).await.expect("v203 must be re-runnable in isolation");
    }

    #[tokio::test]
    async fn v203_backfills_legacy_above_below_correctly() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        super::super::v200_axinvest_stock_tables::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        // 插入老格式数据（模拟升级前快照）
        db.execute_unprepared(
            "INSERT INTO price_alerts (id, stock_code, stock_name, condition, target_price, \
             is_triggered, triggered_at, created_at, updated_at) \
             VALUES \
             ('t1', '000001', '平安银行', 'above', 15.5, 0, NULL, 0, 0), \
             ('t2', '000002', '万科A', 'below', 9.2, 0, NULL, 0, 0), \
             ('t3', '600519', '贵州茅台', 'change_up', 3.0, 0, NULL, 0, 0)",
        )
        .await
        .unwrap();

        // 回填后再跑一次 up()（模拟重启）
        up(db.clone()).await.unwrap();

        // 验证 above → take_profit + price + 15.5
        let row = db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Sqlite,
                "SELECT alert_type, condition_type, threshold FROM price_alerts WHERE id='t1'",
                [],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get_by::<String, _>("alert_type").unwrap(), "take_profit");
        assert_eq!(row.try_get_by::<String, _>("condition_type").unwrap(), "price");
        assert!((row.try_get_by::<f64, _>("threshold").unwrap() - 15.5).abs() < 1e-6);

        // 验证 below → stop_loss
        let row = db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Sqlite,
                "SELECT alert_type, condition_type, threshold FROM price_alerts WHERE id='t2'",
                [],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get_by::<String, _>("alert_type").unwrap(), "stop_loss");
        assert_eq!(row.try_get_by::<String, _>("condition_type").unwrap(), "price");
        assert!((row.try_get_by::<f64, _>("threshold").unwrap() - 9.2).abs() < 1e-6);

        // 验证 change_up → change + change_pct
        let row = db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Sqlite,
                "SELECT alert_type, condition_type, threshold FROM price_alerts WHERE id='t3'",
                [],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get_by::<String, _>("alert_type").unwrap(), "change");
        assert_eq!(row.try_get_by::<String, _>("condition_type").unwrap(), "change_pct");
        assert!((row.try_get_by::<f64, _>("threshold").unwrap() - 3.0).abs() < 1e-6);
    }
}
