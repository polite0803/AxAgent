//! 复盘 → 进化：漂移数据访问与权重持久化层
//!
//! 主要职责：
//! 1. 读取 strategy_performance 表现行（按窗口过滤）
//! 2. 读取最近一次 strategy_weight_history 作为 current_weights
//! 3. 调用 `weight_decay::compute_adjusted_weights` 计算新权重
//! 4. 写回 strategy_weight_history（每次调整全量留痕）
//! 5. 提供 list/load 供前端 EvolutionDriftPanel 渲染
//!
//! 时间旅行注意：
//! - `as_of_date: Option<String>` 决定是否走 Replay 模式
//! - Live 模式（as_of_date = None）：基于当前时间窗口
//! - Replay 模式（as_of_date = "2024-09-30"）：基于 as_of 之前的窗口，避免未来泄漏

use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use axagent_core::entity::{strategy_performance, strategy_weight_history};

use crate::weight_decay::{
    compute_adjusted_weights, format_rationale, StrategyPerformanceRow, WeightDecayConfig,
};

/// 前端 `EvolutionDriftPanel` 使用的统一响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionDriftDashboard {
    /// 当前生效的 (strategy, period) -> 权重
    pub current_weights: HashMap<(String, String), f64>,
    /// 最近一次重算时间（ms），0 表示尚未重算
    pub last_recalc_at: i64,
    /// 仪表盘当前所有 (strategy, period) 的统计
    pub stats: Vec<StrategyStatRow>,
    /// 最近 N 次调整原因（Top 5）
    pub recent_changes: Vec<RecentChangeRow>,
    /// 各策略汇总视图（按 strategy_id 聚合）
    pub strategy_summary: Vec<StrategySummaryRow>,
}

/// 单条 (strategy, period) 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyStatRow {
    pub strategy_id: String,
    pub period: String,
    pub new_weight: f64,
    pub old_weight: f64,
    pub delta_pct: f64,
    pub win_rate: f64,
    pub sample_size: u32,
    pub confidence: f64,
    pub rationale: String,
}

/// Top 5 调整原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentChangeRow {
    pub id: String,
    pub strategy_id: String,
    pub period: String,
    pub old_weight: f64,
    pub new_weight: f64,
    pub delta_pct: f64,
    pub trigger: String,
    pub rationale: Option<String>,
    pub applied_at: i64,
}

/// 按 strategy_id 聚合的摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategySummaryRow {
    pub strategy_id: String,
    pub avg_weight: f64,
    pub total_samples: u32,
    pub avg_win_rate: f64,
    pub trend: String, // "up" | "down" | "stable"
}

/// 读取当前生效的 weights（每个 (strategy, period) 取最新一条 weight_history.new_weight）
pub async fn load_current_weights(
    db: &DatabaseConnection,
) -> Result<HashMap<(String, String), f64>, String> {
    let all = strategy_weight_history::Entity::find()
        .order_by_desc(strategy_weight_history::Column::AppliedAt)
        .all(db)
        .await
        .map_err(|e| format!("读取 weight_history 失败: {e}"))?;

    // 同一 (strategy, period) 取最新一行
    let mut map: HashMap<(String, String), f64> = HashMap::new();
    for row in all {
        let key = (row.strategy_id.clone(), row.period.clone());
        map.entry(key).or_insert(row.new_weight);
    }
    Ok(map)
}

/// 读取 strategy_performance 在窗口内的所有行
pub async fn load_performance_window(
    db: &DatabaseConnection,
    lookback_days: u32,
    as_of_date: Option<&str>,
) -> Result<Vec<StrategyPerformanceRow>, String> {
    let cutoff = if let Some(d) = as_of_date {
        // Replay 模式：以 as_of_date 当作"今天"
        let date = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .map_err(|e| format!("as_of_date 格式错误: {e}"))?;
        let dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| "无效日期".to_string())?;
        dt.and_utc().timestamp_millis() - (lookback_days as i64) * 86_400_000
    } else {
        Utc::now().timestamp_millis() - (lookback_days as i64) * 86_400_000
    };

    let rows = strategy_performance::Entity::find()
        .filter(strategy_performance::Column::ExitAt.gte(cutoff))
        .all(db)
        .await
        .map_err(|e| format!("读取 strategy_performance 失败: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| StrategyPerformanceRow {
            strategy_id: r.strategy_id,
            period: r.period,
            was_correct: r.was_correct,
            exit_at: r.exit_at,
        })
        .collect())
}

/// 重算并写回所有 (strategy, period) 权重调整
///
/// 返回值：(写入了多少行 weight_history, 新权重表)
pub async fn recalc_and_persist(
    db: &DatabaseConnection,
    trigger: &str, // "cron" | "manual" | "rule"
    source_reflection_id: Option<&str>,
    as_of_date: Option<&str>,
) -> Result<(usize, HashMap<(String, String), f64>), String> {
    let cfg = WeightDecayConfig::default();
    let current = load_current_weights(db).await?;
    let history = load_performance_window(db, cfg.lookback_days, as_of_date).await?;
    let new_map = compute_adjusted_weights(&history, &cfg, &current);

    if new_map.is_empty() {
        warn!("[evolution_drift] 窗口内无表现数据,跳过调整");
        return Ok((0, current));
    }

    let now = Utc::now().timestamp_millis();
    let mut written = 0usize;
    for (key, aw) in &new_map {
        let old = current.get(key).copied().unwrap_or(1.0);
        let delta_pct = if old.abs() > f64::EPSILON {
            (aw.new_weight - old) / old * 100.0
        } else {
            0.0
        };
        // 容忍极小抖动（< 1%）：不写库，避免污染
        if delta_pct.abs() < 1.0 {
            continue;
        }
        let rationale = format_rationale(aw, aw.sample_size);
        let am = strategy_weight_history::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            strategy_id: Set(aw.strategy_id.clone()),
            period: Set(aw.period.clone()),
            old_weight: Set(old),
            new_weight: Set(aw.new_weight),
            delta_pct: Set(delta_pct),
            trigger: Set(trigger.to_string()),
            source_reflection_id: Set(source_reflection_id.map(|s| s.to_string())),
            sample_size: Set(aw.sample_size as i32),
            win_rate: Set(aw.win_rate),
            rationale: Set(Some(rationale)),
            applied_at: Set(now),
        };
        strategy_weight_history::Entity::insert(am)
            .exec(db)
            .await
            .map_err(|e| format!("写入 weight_history 失败: {e}"))?;
        written += 1;
    }

    info!("[evolution_drift] 触发={trigger} 写入 {written} 条权重调整");
    let weight_only: HashMap<(String, String), f64> = new_map
        .iter()
        .map(|(k, v)| (k.clone(), v.new_weight))
        .collect();
    Ok((written, weight_only))
}

/// 仪表盘（前端 EvolutionDriftPanel 主页用）
pub async fn get_dashboard(
    db: &DatabaseConnection,
    as_of_date: Option<&str>,
) -> Result<EvolutionDriftDashboard, String> {
    let cfg = WeightDecayConfig::default();
    let current = load_current_weights(db).await?;
    let history = load_performance_window(db, cfg.lookback_days, as_of_date).await?;
    let new_map = compute_adjusted_weights(&history, &cfg, &current);

    // 拉取最近 5 次调整原因
    let recent = strategy_weight_history::Entity::find()
        .order_by_desc(strategy_weight_history::Column::AppliedAt)
        .limit(5)
        .all(db)
        .await
        .map_err(|e| format!("读取 recent_changes 失败: {e}"))?;
    let recent_changes: Vec<RecentChangeRow> = recent
        .into_iter()
        .map(|r| RecentChangeRow {
            id: r.id,
            strategy_id: r.strategy_id,
            period: r.period,
            old_weight: r.old_weight,
            new_weight: r.new_weight,
            delta_pct: r.delta_pct,
            trigger: r.trigger,
            rationale: r.rationale,
            applied_at: r.applied_at,
        })
        .collect();

    // 拉取最近一次 applied_at 作为"最近重算时间"
    let last_recalc_at = strategy_weight_history::Entity::find()
        .order_by_desc(strategy_weight_history::Column::AppliedAt)
        .one(db)
        .await
        .map_err(|e| format!("查询 last_recalc 失败: {e}"))?
        .map(|r| r.applied_at)
        .unwrap_or(0);

    // 构造 stats：合并 current 与 new,old 来自最近一次 weight_history
    let mut stats: Vec<StrategyStatRow> = Vec::new();
    for (key, aw) in &new_map {
        let old = current.get(key).copied().unwrap_or(1.0);
        let delta_pct = if old.abs() > f64::EPSILON {
            (aw.new_weight - old) / old * 100.0
        } else {
            0.0
        };
        let rationale = format_rationale(aw, aw.sample_size);
        stats.push(StrategyStatRow {
            strategy_id: aw.strategy_id.clone(),
            period: aw.period.clone(),
            new_weight: aw.new_weight,
            old_weight: old,
            delta_pct,
            win_rate: aw.win_rate,
            sample_size: aw.sample_size,
            confidence: aw.confidence,
            rationale,
        });
    }
    // 补齐 new_map 中没有但 current 中有的（旧策略无近期表现）
    for (key, w) in &current {
        if !new_map.contains_key(key) {
            stats.push(StrategyStatRow {
                strategy_id: key.0.clone(),
                period: key.1.clone(),
                new_weight: *w,
                old_weight: *w,
                delta_pct: 0.0,
                win_rate: 0.0,
                sample_size: 0,
                confidence: 0.0,
                rationale: "窗口内无表现数据,保持上次权重".to_string(),
            });
        }
    }

    // strategy_summary: 按 strategy_id 聚合
    let mut summary_map: HashMap<String, (f64, u32, f64, u32)> = HashMap::new(); // (sum_weight, sum_samples, sum_win, count)
    for s in &stats {
        let entry = summary_map
            .entry(s.strategy_id.clone())
            .or_insert((0.0, 0, 0.0, 0));
        entry.0 += s.new_weight;
        entry.1 += s.sample_size;
        entry.2 += s.win_rate;
        entry.3 += 1;
    }
    let strategy_summary: Vec<StrategySummaryRow> = summary_map
        .into_iter()
        .map(|(sid, (sum_w, sum_s, sum_wr, cnt))| {
            let avg_w = if cnt > 0 { sum_w / cnt as f64 } else { 1.0 };
            let avg_wr = if cnt > 0 { sum_wr / cnt as f64 } else { 0.0 };
            // 简单趋势：根据 delta 符号聚合
            let stats_for: Vec<&StrategyStatRow> =
                stats.iter().filter(|s| s.strategy_id == sid).collect();
            let net_delta: f64 = stats_for.iter().map(|s| s.delta_pct).sum();
            let trend = if net_delta > 5.0 {
                "up"
            } else if net_delta < -5.0 {
                "down"
            } else {
                "stable"
            };
            StrategySummaryRow {
                strategy_id: sid,
                avg_weight: avg_w,
                total_samples: sum_s,
                avg_win_rate: avg_wr,
                trend: trend.to_string(),
            }
        })
        .collect();

    let weight_only: HashMap<(String, String), f64> = new_map
        .iter()
        .map(|(k, v)| (k.clone(), v.new_weight))
        .collect();

    Ok(EvolutionDriftDashboard {
        current_weights: weight_only,
        last_recalc_at,
        stats,
        recent_changes,
        strategy_summary,
    })
}

/// 拉取时间线（前端 sparkline 用）
pub async fn get_timeline(
    db: &DatabaseConnection,
    strategy_id: &str,
    period: &str,
    limit: u32,
) -> Result<Vec<TimelinePoint>, String> {
    let rows = strategy_weight_history::Entity::find()
        .filter(strategy_weight_history::Column::StrategyId.eq(strategy_id))
        .filter(strategy_weight_history::Column::Period.eq(period))
        .order_by_desc(strategy_weight_history::Column::AppliedAt)
        .limit(limit as u64)
        .all(db)
        .await
        .map_err(|e| format!("读取 timeline 失败: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|r| TimelinePoint {
            applied_at: r.applied_at,
            new_weight: r.new_weight,
            old_weight: r.old_weight,
            delta_pct: r.delta_pct,
            trigger: r.trigger,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePoint {
    pub applied_at: i64,
    pub new_weight: f64,
    pub old_weight: f64,
    pub delta_pct: f64,
    pub trigger: String,
}

/// 写入一行 strategy_performance（复盘 cron 触发时使用）
// 参数较多(策略调用方: cron / replay runner / 离线评测 共用)，保持显式参数以避免引入过多构造包装。
#[allow(clippy::too_many_arguments)]
pub async fn record_performance(
    db: &DatabaseConnection,
    strategy_id: &str,
    period: &str,
    stock_code: &str,
    stock_name: &str,
    decision_at: i64,
    exit_at: i64,
    holding_days: i32,
    return_pct: f64,
    was_correct: i32,
    decision_confidence: i32,
    horizon_pnl_json: Option<&str>,
    agreement_score: Option<i32>,
) -> Result<String, String> {
    let now = Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let am = strategy_performance::ActiveModel {
        id: Set(id.clone()),
        strategy_id: Set(strategy_id.to_string()),
        period: Set(period.to_string()),
        stock_code: Set(stock_code.to_string()),
        stock_name: Set(stock_name.to_string()),
        decision_at: Set(decision_at),
        exit_at: Set(exit_at),
        holding_days: Set(holding_days),
        return_pct: Set(return_pct),
        was_correct: Set(was_correct),
        decision_confidence: Set(decision_confidence),
        horizon_pnl_json: Set(horizon_pnl_json.map(|s| s.to_string())),
        agreement_score: Set(agreement_score),
        created_at: Set(now),
    };
    strategy_performance::Entity::insert(am)
        .exec(db)
        .await
        .map_err(|e| format!("写入 strategy_performance 失败: {e}"))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_struct_serde_camel_case() {
        let d = EvolutionDriftDashboard {
            current_weights: HashMap::new(),
            last_recalc_at: 123,
            stats: vec![],
            recent_changes: vec![],
            strategy_summary: vec![],
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"currentWeights\""), "camelCase 序列化");
        assert!(json.contains("\"lastRecalcAt\""), "camelCase 序列化");
        assert!(!json.contains("\"last_recalc_at\""), "不应有 snake_case 字段");
    }

    #[test]
    fn strategy_summary_trend_aggregation() {
        // 验证 trend 字段在 net_delta > 5 时为 "up",<-5 为 "down",其他 stable
        let stats = vec![
            StrategyStatRow {
                strategy_id: "trend".to_string(),
                period: "short".to_string(),
                new_weight: 1.2,
                old_weight: 1.0,
                delta_pct: 20.0, // +20%
                win_rate: 0.6,
                sample_size: 30,
                confidence: 0.95,
                rationale: "ok".to_string(),
            },
            StrategyStatRow {
                strategy_id: "trend".to_string(),
                period: "mid".to_string(),
                new_weight: 0.5,
                old_weight: 1.0,
                delta_pct: -50.0,
                win_rate: 0.3,
                sample_size: 30,
                confidence: 0.9,
                rationale: "ok".to_string(),
            },
        ];
        // trend/short: +20, trend/mid: -50 → net_delta = -30 → trend = "down"
        let net: f64 = stats.iter().map(|s| s.delta_pct).sum();
        assert!(net < -5.0, "净 delta 应明显为负");
    }
}
