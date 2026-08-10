// SPDX-License-Identifier: AGPL-3.0-only

//! v220 — 叙事结构持久化表
//!
//! ## 背景
//!
//! 为支持文学创作工作流中叙事结构（角色弧线、交汇点、伏笔网络）的
//! 跨会话保存与恢复，新增 `narrative_structures` 表。
//!
//! ## 功能
//!
//! - 保存完整的叙事结构设计（arcs/confluences/foreshadows 以 JSON 存储）
//! - 支持模板标记（is_template），便于用户复用预设结构
//! - 支持版本号，便于后续扩展
//!
//! ## 幂等性
//!
//! 使用 CREATE TABLE IF NOT EXISTS，重跑安全。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_table = r#"
CREATE TABLE IF NOT EXISTS narrative_structures (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    genre TEXT NOT NULL DEFAULT 'novel',
    arcs TEXT NOT NULL DEFAULT '[]',
    confluences TEXT NOT NULL DEFAULT '[]',
    foreshadows TEXT NOT NULL DEFAULT '[]',
    is_template INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_narrative_structures_is_template ON narrative_structures(is_template)",
        "CREATE INDEX IF NOT EXISTS idx_narrative_structures_genre ON narrative_structures(genre)",
    ];

    db.execute_unprepared(create_table).await?;
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
