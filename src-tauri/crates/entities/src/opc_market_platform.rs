// SPDX-License-Identifier: AGPL-3.0-only

//! 市场平台连接器配置表实体
//!
//! 记录闲鱼 / 猪八戒等需求平台的连接器配置：启停状态、抓取参数、最近同步时间。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_market_platform")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub platform_type: String,
    pub enabled: i32,
    pub base_url: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub config_json: String,
    pub last_sync_at: Option<i64>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
