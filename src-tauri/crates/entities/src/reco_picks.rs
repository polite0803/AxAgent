//! 荐股推荐持久化
//!
//! 每次 `recommend_stocks()` 被调用时，输出的 reco picks 写入此表。
//! 用于后续回测的正向样本（被推荐的股票）和负向样本（候选池中未被推荐的股票）来源。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reco_picks")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 荐股生成时间戳 (ISO 8601)
    pub generated_at: String,
    /// 周期: "short" | "mid" | "long"
    pub period: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 命中风格: "trend" | "value" | "capital" | "reversion" | "watchlist"
    pub style: String,
    /// 置信度 0-100
    pub confidence: i32,
    /// 是否为兜底合成 (1=true, 0=false)
    pub synthetic: i32,
    /// 候选池快照 (JSON array of [code, name])
    pub seed_pool_json: Option<String>,
    /// 荐股时的策略权重配置快照 (JSON object, e.g. {"trend_short": 0.85})
    pub strategy_weights_json: Option<String>,
    /// 完整 RecoPick 序列化 (JSON, v007 新增) —— 用于 get_cached_recommendation
    /// 命令把上一次的荐股结果完整还原到前端,与实时拉取结果 schema 完全等价。
    /// 旧行(本 migration 之前生成的)此字段为 NULL,缓存读时被跳过。
    pub pick_data: Option<String>,
    /// 记录创建时间
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
