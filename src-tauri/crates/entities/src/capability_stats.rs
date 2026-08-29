// SPDX-License-Identifier: AGPL-3.0-only

//! 能力护照执行统计 —— 记录能力（workflow/skill/agent/tool）每次执行的效果数据，
//! 供能力发现排序器（CapabilityRanker 的 β 历史成功率 / γ 耗时 / 探索提权）消费。
//!
//! # 背景
//! 护照的 `CapabilityStats` 此前注册后从未被写回（全部 `Default::default()`），
//! 导致 `total_calls` 恒 0 → `total_calls < 10` 恒真 → 探索提权对所有能力生效，
//! 排序器的 β/γ/δ/探索四维全部失真。本表是反馈闭环的持久化载体：
//! - 写路径：接线点（rt-workflow 引擎 / skill 执行 / agent 执行）调 `record_execution`
//! - 读路径：`CapabilityIndexerImpl` 在返回护照前合并本表数据到 `passport.stats`

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "capability_stats")]
pub struct Model {
    /// 能力护照 ID（`workflow:{id}` / `skill:{name}` / `agent:{id}` / `tool:{name}`）
    #[sea_orm(primary_key, auto_increment = false)]
    pub capability_id: String,
    /// 总调用次数
    #[sea_orm(column_name = "total_calls")]
    pub total_calls: i64,
    /// 成功次数
    #[sea_orm(column_name = "success_count")]
    pub success_count: i64,
    /// 最近 N 次执行结果（JSON 数组，[0/1, ...]，0=失败 1=成功），用于计算近 N 次成功率
    #[sea_orm(column_name = "recent_window")]
    pub recent_window: String,
    /// 平均执行耗时（毫秒）
    #[sea_orm(column_name = "avg_duration_ms")]
    pub avg_duration_ms: i64,
    /// 最近一次执行时间（Unix 毫秒）
    #[sea_orm(column_name = "last_executed_at")]
    pub last_executed_at: Option<i64>,
    /// 记录更新时间（Unix 毫秒）
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
