use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 量化策略元数据
///
/// - builtin: 内置 Rust 策略，script_source 为空
/// - rhai: Rhai 脚本策略，script_source 保存源码（用于热加载 / 编辑）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quant_strategies")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 策略名（unique，用户可读）
    pub name: String,
    pub version: String,
    /// "builtin" | "rhai"
    pub strategy_type: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Rhai 脚本源码（builtin 留空）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub script_source: Option<String>,
    /// 参数 JSON 字符串
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub params_json: Option<String>,
    /// 是否启用 Walk-Forward（D3 决策，默认 true）
    pub walk_forward_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
