// SPDX-License-Identifier: AGPL-3.0-only
//! v009_tool_adaptation: 给 providers 表加 tool_adaptation 与 tool_adaptation_marker_prefix 列
//!
//! 用于持久化用户在 UI 中设置的「工具调用模式」选择（native/managed）及
//! 自定义 marker prefix。
//! 两列均为可空 TEXT，默认 NULL = 按 provider_type 推断 / 使用 "CHAT2API"。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE providers ADD COLUMN tool_adaptation TEXT").await?;
    db.execute_unprepared("ALTER TABLE providers ADD COLUMN tool_adaptation_marker_prefix TEXT")
        .await?;
    Ok(())
}
