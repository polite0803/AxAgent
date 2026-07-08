//! v007 — `reco_picks` 加 `pick_data` 列
//!
//! 用途:为支持"智能荐股页打开时显示上一次的荐股结果"特性,
//! 持久化时把完整的 `RecoPick` 序列化到这一列(price / entryLow / entryHigh /
//! stopLoss / targetPrice / positionPct / holdingDays / reasons / riskNotes /
//! secondaryStyles / sector 等),后续 `get_cached_recommendation` 命令读这一列
//! 反序列化为 `RecoPick`,缓存展示与实时完全等价。
//!
//! 与已有列的关系:
//!   - `stock_code` / `stock_name` / `style` / `period` / `confidence` /
//!     `synthetic` 保留:backtest 统计 / 候选池查询依然用这些列做索引过滤,
//!     简单 SELECT 即可命中。
//!   - `pick_data` 新增:承担"完整 pick 还原"的职责,一次 JSON 序列化
//!     避免 schema 再加 10+ 列。
//!
//! 与 v005 / v006 的设计保持一致:用 ALTER TABLE 单列追加 + TEXT,
//! 读写路径分别用 `serde_json::to_string` / `serde_json::from_str`。
//!
//! 旧行(本 migration 之前生成的)pick_data 为 NULL,缓存读时
//! `get_cached_recommendation` 会跳过这些行,只展示新行(语义合理:
//! 旧行的详细字段已经无法恢复,展示为"无缓存"即可,下次实时推荐后
//! 会被新行带 pick_data 覆盖)。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE reco_picks ADD COLUMN pick_data TEXT")
        .await?;
    Ok(())
}
