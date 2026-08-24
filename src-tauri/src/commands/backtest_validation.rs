// SPDX-License-Identifier: AGPL-3.0-only

//! 决策事后验证（V55）—— 跑历史回放回测，测算现状 hit_rate 与 9 因子 IC
//!
//! ## 背景
//! 股票分析系统的最大问题是"决策可采信度低"。本模块用历史数据反推当前决策系统的
//! 真实命中率与因子有效性，把"事后验证"做成系统的一等公民。
//!
//! ## 工作流
//! ```text
//! reco_picks 表（历史荐股）
//!     ↓ run_decision_backtest
//! 拉取 T+5/T+20/T+60 实际 K 线 (行情 API)
//!     ↓
//! build_pick_validation() 推断 action + 计算 hit_outcome
//!     ↓
//! 写 decision_validations 表
//!     ↓
//! compute_hit_rate_report() 聚合 hit_rate + 9 因子 IC
//!     ↓
//! 返回 HitRateReport 给前端 / 反馈到 portfolio-mgr.rhai
//! ```
//!
//! ## 关键设计
//! - **dry_run=true**：不写表，只返回 report（用于预览）
//! - **synthetic_filter**：默认排除 synthetic=1 的兜底 pick（这些不是真实决策）
//! - **T+N 默认值**：[5, 20, 60]（短/中/长三个窗口）
//! - **K 线拉取**：复用 astock_client.get_klines，自动 vendor failover + 缓存

use axagent_analysis_engine::hit_rate_backtest::{
    HitRateReport, PickValidation, build_pick_validation, compute_hit_rate_report,
};
use axagent_analysis_engine::recommender::types::RecoPick;
use axagent_entities::decision_validations;
use axagent_entities::reco_picks;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;

/// 跑决策回测请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDecisionBacktestRequest {
    /// T+N 验证窗口（默认 [5, 20, 60]）
    pub t_plus_n_list: Option<Vec<i32>>,
    /// 是否排除 synthetic=1 的兜底 pick（默认 true）
    pub exclude_synthetic: Option<bool>,
    /// 最大回测 pick 数（默认 200，避免一次跑太多超时）
    pub max_picks: Option<u32>,
    /// 仅生成报告不写库（用于前端预览）
    pub dry_run: Option<bool>,
    /// 仅回测指定周期（"short" | "mid" | "long" | None=全部）
    pub period_filter: Option<String>,
}

/// 跑决策回测响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDecisionBacktestResponse {
    pub report: HitRateReport,
    /// 写库的条数（dry_run 时为 0）
    pub written_count: usize,
    /// 跳过的条数（合成 pick / 缺数据等）
    pub skipped_count: usize,
    /// 数据源（"eastmoney" | "sina" | "xueqiu" | "fallback_seed"）
    pub data_source: String,
}

/// 跑决策回测 —— 历史回放回测
///
/// 流程：
/// 1. 读取 reco_picks（默认排除 synthetic 兜底）
/// 2. 对每条 pick 拉取 T+N 窗口的日 K 线
/// 3. 用 `build_pick_validation` 构建 PickValidation
/// 4. 若 dry_run=false，写 decision_validations 表
/// 5. 聚合所有 validations → HitRateReport
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "跑决策回测")]
#[tauri::command]
pub async fn run_decision_backtest(
    state: State<'_, AppState>,
    request: RunDecisionBacktestRequest,
) -> Result<RunDecisionBacktestResponse, String> {
    // ── 参数标准化 ──
    let t_plus_n_list = request.t_plus_n_list.unwrap_or_else(|| vec![5, 20, 60]);
    let exclude_synthetic = request.exclude_synthetic.unwrap_or(true);
    let max_picks = request.max_picks.unwrap_or(200).min(2000);
    let dry_run = request.dry_run.unwrap_or(false);

    // ── 1. 读取 reco_picks ──
    let db = state.harness.db();
    let mut query = reco_picks::Entity::find();
    // P1 修复(2026-08-01): 排除 serenity-screening 候选行（style='serenity'，
    // pick_data 的 price 等可能为 0，无决策验证意义；且 seed_pool_json 格式不同）。
    query = query.filter(reco_picks::Column::Style.ne("serenity"));
    if exclude_synthetic {
        query = query.filter(reco_picks::Column::Synthetic.eq(0));
    }
    if let Some(ref period) = request.period_filter {
        query = query.filter(reco_picks::Column::Period.eq(period.as_str()));
    }
    // 按 generated_at 倒序，优先回测最近的数据
    let picks = query.all(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("读取 reco_picks 失败: {e}"))
    })?;

    if picks.is_empty() {
        // 空集合：返回空报告，避免前端拿到 None 报错
        return Ok(RunDecisionBacktestResponse {
            report: empty_report(),
            written_count: 0,
            skipped_count: 0,
            data_source: "none".to_string(),
        });
    }

    // 限制条数：取最近 max_picks 条
    let picks: Vec<_> = picks.into_iter().take(max_picks as usize).collect();

    // ── 2-3. 拉 K 线 + 构建 PickValidation ──
    let mut validations: Vec<PickValidation> = Vec::new();
    let mut skipped = 0usize;
    let mut data_source = "unknown".to_string();

    for pick_model in &picks {
        // 解析 pick_data → RecoPick（pick_data 是 None 或 JSON 解析失败时跳过）
        let Some(ref pick_data_str) = pick_model.pick_data else {
            skipped += 1;
            continue;
        };
        let reco_pick: RecoPick = match serde_json::from_str(pick_data_str) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    pick_id = %pick_model.id,
                    error = %e,
                    "解析 reco_picks.pick_data 失败，跳过"
                );
                skipped += 1;
                continue;
            },
        };

        // 对每个 T+N 跑一次验证
        for &t_plus_n in &t_plus_n_list {
            match fetch_and_validate(
                &state,
                &reco_pick,
                &pick_model.id,
                t_plus_n,
                &pick_model.generated_at,
            )
            .await
            {
                Ok((validation, src)) => {
                    if data_source == "unknown" {
                        data_source = src;
                    }
                    validations.push(validation);
                },
                Err(e) => {
                    tracing::warn!(
                        pick_id = %pick_model.id,
                        stock = %reco_pick.stock_code,
                        t_plus_n = t_plus_n,
                        error = %e,
                        "拉取/验证失败，跳过"
                    );
                    skipped += 1;
                },
            }
        }
    }

    // ── 4. 写库（dry_run=false 时）──
    let written_count = if dry_run || validations.is_empty() {
        0
    } else {
        write_decision_validations(db, &validations).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("写 decision_validations 失败: {e}"))
        })?
    };

    // ── 4.5 P2-F15: outcome 回写 ──
    // T+N 验证完成后，根据 hit_outcome 反推 stock_analyses.outcome（win/loss），
    // 然后回写 lesson_applications.outcome_at_validation。
    // 这样 run_lesson_validation 就能精确统计 success_count。
    //
    // 匹配策略：通过 stock_code + generated_at 日期匹配 stock_analyses 行。
    // 注意：reco_picks 和 stock_analyses 是两条独立路径，这里用日期近似匹配，
    // 可能存在一对多情况（同一天同一只股票多个 analysis），取最近的一条。
    if !dry_run && !validations.is_empty() {
        let synced = sync_outcomes_to_stock_analyses(db, &validations).await;
        if synced > 0 {
            tracing::info!(
                "[backtest] P2-F15: 从 decision_validations 回写 {synced} 条 stock_analyses.outcome + lesson_applications"
            );
        }
    }

    // ── 5. 聚合报告 ──
    let report = if validations.is_empty() {
        empty_report()
    } else {
        compute_hit_rate_report(&validations)
    };

    Ok(RunDecisionBacktestResponse { report, written_count, skipped_count: skipped, data_source })
}

/// 拉取 T+N 窗口日 K 线并构建 PickValidation
async fn fetch_and_validate(
    state: &State<'_, AppState>,
    reco_pick: &RecoPick,
    pick_id: &str,
    t_plus_n: i32,
    generated_at: &str,
) -> Result<(PickValidation, String), String> {
    // 从 generated_at 提取决策日期（格式 "YYYY-MM-DDTHH:MM:SS.fff" → "YYYY-MM-DD"）
    let decision_date = generated_at.get(..10).unwrap_or(generated_at);

    // 拉取窗口：取 T+N 后 10 个交易日（防止节假日窗口不足）。
    // 修复 P0: 原 fetch_limit 太小（t_plus_n + 10），当 pick 距今较远时
    // 不包含决策日之后的足够数据。改为 500（约 2 个交易年）确保覆盖。
    let fetch_limit = (t_plus_n as u32 + 10).max(500);

    let klines =
        state.astock_client.get_klines(&reco_pick.stock_code, "daily", fetch_limit).await.map_err(
            |e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("K 线拉取失败: {e}")),
        )?;

    if klines.is_empty() {
        return Err("K 线为空".to_string());
    }

    // 修复 P0: 原代码 klines[..n] 取的是最早的 n 根 K 线（决策日之前的数据），
    // 导致用决策前的价格"验证"决策，命中率完全失真。
    // 正确逻辑：找到决策日之后的第一根 K 线，取该位置之后的 n 根。
    // K 线按日期升序排列（vendors/sina.rs 等都 sort_by date）
    let start_idx = klines
        .iter()
        .position(|k| k.date.as_str() > decision_date)
        .ok_or_else(|| format!("决策日 {decision_date} 之后无 K 线数据"))?;

    let valid_klines = &klines[start_idx..];
    let n = (t_plus_n as usize).min(valid_klines.len());
    let closes: Vec<f64> = valid_klines[..n].iter().map(|k| k.close).collect();
    let highs: Vec<f64> = valid_klines[..n].iter().map(|k| k.high).collect();
    let lows: Vec<f64> = valid_klines[..n].iter().map(|k| k.low).collect();

    // 数据源标识：优先用 K 线数据的 vendor 名（这里简化为 "astock_client"，
    // 因为 AStockClient 内部已做 vendor failover，统一标记）
    let data_source = "astock_client".to_string();

    let validation =
        build_pick_validation(reco_pick, pick_id, t_plus_n, &closes, &highs, &lows, &data_source);

    Ok((validation, data_source))
}

/// 批量写 decision_validations（按 (pick_id, t_plus_n) 幂等）
async fn write_decision_validations(
    db: &sea_orm::DatabaseConnection,
    validations: &[PickValidation],
) -> Result<usize, String> {
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for v in validations {
        // 幂等：先查 (pick_id, t_plus_n) 是否已存在
        let existing = decision_validations::Entity::find()
            .filter(decision_validations::Column::PickId.eq(&v.pick_id))
            .filter(decision_validations::Column::TPlusN.eq(v.t_plus_n))
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("查询已存在验证记录失败: {e}"))
            })?;

        if existing.is_some() {
            // 已存在则跳过（避免覆盖前次结果）
            continue;
        }

        let factor_snapshot_json =
            v.factor_snapshot.as_ref().and_then(|m| serde_json::to_string(m).ok());

        let active = decision_validations::ActiveModel {
            id: Set(uuid_v4()),
            pick_id: Set(v.pick_id.clone()),
            stock_code: Set(v.stock_code.clone()),
            stock_name: Set(v.stock_name.clone()),
            style: Set(v.style.clone()),
            period: Set(v.period.clone()),
            t_plus_n: Set(v.t_plus_n),
            generated_at: Set(v.generated_at.clone()),
            validated_at: Set(now.clone()),
            entry_price: Set(v.entry_price),
            target_price: Set(v.target_price),
            stop_loss: Set(v.stop_loss),
            position_pct: Set(v.position_pct),
            confidence: Set(v.confidence),
            inferred_action: Set(v.inferred_action.clone()),
            t_plus_n_price: Set(v.t_plus_n_price),
            max_price: Set(v.max_price),
            min_price: Set(v.min_price),
            max_return_pct: Set(v.max_return_pct),
            max_drawdown_pct: Set(v.max_drawdown_pct),
            final_return_pct: Set(v.final_return_pct),
            hit_stop_loss: Set(v.hit_stop_loss),
            hit_target: Set(v.hit_target),
            hit_outcome: Set(v.hit_outcome.clone()),
            factor_snapshot: Set(factor_snapshot_json),
            data_source: Set(v.data_source.clone()),
            created_at: Set(now.clone()),
        };

        decision_validations::Entity::insert(active).exec(db).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("插入 decision_validations 失败: {e}"))
        })?;
        count += 1;
    }

    Ok(count)
}

/// 列表：已写入的决策验证记录
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionValidationItem {
    pub id: String,
    pub pick_id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub style: String,
    pub period: String,
    pub t_plus_n: i32,
    pub generated_at: String,
    pub validated_at: String,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub position_pct: f64,
    pub confidence: i32,
    pub inferred_action: String,
    pub t_plus_n_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_price: Option<f64>,
    pub max_return_pct: Option<f64>,
    pub max_drawdown_pct: Option<f64>,
    pub final_return_pct: Option<f64>,
    pub hit_stop_loss: Option<i32>,
    pub hit_target: Option<i32>,
    pub hit_outcome: Option<String>,
    pub data_source: String,
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出决策验证记录")]
#[tauri::command]
pub async fn list_decision_validations(
    state: State<'_, AppState>,
    stock_code: Option<String>,
    hit_outcome: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<DecisionValidationItem>, String> {
    use sea_orm::{PaginatorTrait, QueryOrder};
    let db = state.harness.db();

    let mut query = decision_validations::Entity::find();
    if let Some(ref code) = stock_code {
        query = query.filter(decision_validations::Column::StockCode.eq(code.as_str()));
    }
    if let Some(ref outcome) = hit_outcome {
        query = query.filter(decision_validations::Column::HitOutcome.eq(outcome.as_str()));
    }

    let paginator = query
        .order_by_desc(decision_validations::Column::ValidatedAt)
        .paginate(db, limit.unwrap_or(100) as u64);
    let items = paginator.fetch_page(offset.unwrap_or(0) as u64).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("查询 decision_validations 失败: {e}"))
    })?;

    Ok(items
        .into_iter()
        .map(|m| DecisionValidationItem {
            id: m.id,
            pick_id: m.pick_id,
            stock_code: m.stock_code,
            stock_name: m.stock_name,
            style: m.style,
            period: m.period,
            t_plus_n: m.t_plus_n,
            generated_at: m.generated_at,
            validated_at: m.validated_at,
            entry_price: m.entry_price,
            target_price: m.target_price,
            stop_loss: m.stop_loss,
            position_pct: m.position_pct,
            confidence: m.confidence,
            inferred_action: m.inferred_action,
            t_plus_n_price: m.t_plus_n_price,
            max_price: m.max_price,
            min_price: m.min_price,
            max_return_pct: m.max_return_pct,
            max_drawdown_pct: m.max_drawdown_pct,
            final_return_pct: m.final_return_pct,
            hit_stop_loss: m.hit_stop_loss,
            hit_target: m.hit_target,
            hit_outcome: m.hit_outcome,
            data_source: m.data_source,
        })
        .collect())
}

/// 聚合报告 —— 基于已写入的 decision_validations 重新计算
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "计算验证报告")]
#[tauri::command]
pub async fn compute_validation_report(
    state: State<'_, AppState>,
) -> Result<HitRateReport, String> {
    let db = state.harness.db();
    let all = decision_validations::Entity::find().all(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("读取 decision_validations 失败: {e}"))
    })?;

    if all.is_empty() {
        return Ok(empty_report());
    }

    // DB 行 → PickValidation
    let validations: Vec<PickValidation> = all
        .into_iter()
        .map(|m| {
            let factor_snapshot: Option<HashMap<String, f64>> =
                m.factor_snapshot.as_ref().and_then(|s| serde_json::from_str(s).ok());
            PickValidation {
                pick_id: m.pick_id,
                stock_code: m.stock_code,
                stock_name: m.stock_name,
                style: m.style,
                period: m.period,
                generated_at: m.generated_at,
                t_plus_n: m.t_plus_n,
                entry_price: m.entry_price,
                target_price: m.target_price,
                stop_loss: m.stop_loss,
                position_pct: m.position_pct,
                confidence: m.confidence,
                inferred_action: m.inferred_action,
                t_plus_n_price: m.t_plus_n_price,
                max_price: m.max_price,
                min_price: m.min_price,
                max_return_pct: m.max_return_pct,
                max_drawdown_pct: m.max_drawdown_pct,
                final_return_pct: m.final_return_pct,
                hit_stop_loss: m.hit_stop_loss,
                hit_target: m.hit_target,
                hit_outcome: m.hit_outcome,
                factor_snapshot,
                data_source: m.data_source,
            }
        })
        .collect();

    Ok(compute_hit_rate_report(&validations))
}

/// 空报告（无 pick 时返回，避免前端拿 None 报错）
fn empty_report() -> HitRateReport {
    HitRateReport {
        total: 0,
        generated_at: Utc::now().to_rfc3339(),
        by_action: HashMap::new(),
        by_style: HashMap::new(),
        by_t_plus_n: HashMap::new(),
        factor_ic: HashMap::new(),
        factor_ic_ranked: Vec::new(),
        best_picks: Vec::new(),
        worst_picks: Vec::new(),
    }
}

/// UUID v4 简易实现（避免引入额外依赖）
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    // 时间戳 + 纳秒数 + 进程 ID 拼一个伪 UUID
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (nanos >> 32) as u32,
        ((nanos >> 16) as u16),
        (nanos & 0x0FFF) as u16,
        std::process::id(),
        nanos as u64 & 0xFFFFFFFFFFFF
    )
}

/// P2-F15 切入点 3：从 decision_validations 回写 stock_analyses.outcome + lesson_applications.outcome_at_validation
///
/// 遍历本次 T+N 验证结果（`PickValidation`），根据 `hit_outcome` 推断 win/loss，
/// 然后通过 `stock_code + generated_at` 日期匹配 `stock_analyses` 行，更新其
/// `outcome` 字段，并同步回写 `lesson_applications.outcome_at_validation`。
///
/// ## hit_outcome → outcome 映射
/// - `hit` / `partial` → `win`
/// - `miss` / `false_hit` → `loss`
/// - `insufficient` / `None` → 跳过（数据不足，不做判定）
///
/// ## 匹配策略
/// `reco_picks.generated_at`（ISO 8601）取日期部分，匹配 `stock_analyses.analysis_date`
/// （YYYY-MM-DD）。同一只股票同一天可能有多个 analysis，取 `created_at` 最大（最新）的一条。
///
/// ## 幂等性
/// `update_lesson_application_outcome` 内部有 `outcome_at_validation IS NULL` 守卫，
/// 不会覆盖已验证结果。`stock_analyses.outcome` 用 `update_many` 直接覆盖，
/// 但同一 analysis 的 T+N 验证结果应该是一致的（T+5/T+20/T+60 可能不同，
/// 取最严重的 loss 优先）。
async fn sync_outcomes_to_stock_analyses(
    db: &sea_orm::DatabaseConnection,
    validations: &[PickValidation],
) -> u64 {
    use axagent_entities::stock_analyses;
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};

    let mut synced = 0u64;

    for v in validations {
        // 1. hit_outcome → outcome 映射
        let Some(ref hit_outcome) = v.hit_outcome else {
            continue;
        };
        let outcome = match hit_outcome.as_str() {
            "hit" | "partial" => "win",
            "miss" | "false_hit" => "loss",
            "insufficient" => continue,
            _ => continue,
        };

        // 2. 从 generated_at 提取日期（YYYY-MM-DD）
        let decision_date = v.generated_at.get(..10).unwrap_or(&v.generated_at);

        // 3. 查 stock_analyses 中匹配的行（stock_code + analysis_date）
        //    取最新的一条，避免一对多时更新多条
        //    只更新 outcome 为 NULL 或 pending 的行，避免覆盖已验证结果
        let matching = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::StockCode.eq(&v.stock_code))
            .filter(stock_analyses::Column::AnalysisDate.eq(decision_date))
            .filter(
                Condition::any()
                    .add(stock_analyses::Column::Outcome.is_null())
                    .add(stock_analyses::Column::Outcome.eq("pending")),
            )
            .order_by_desc(stock_analyses::Column::CreatedAt)
            .one(db)
            .await;

        let Ok(Some(analysis)) = matching else {
            // 无匹配行或查询失败，跳过
            continue;
        };

        // 4. 更新 stock_analyses.outcome
        let validation_source = match v.t_plus_n {
            5 => "t_plus_5",
            20 => "t_plus_20",
            60 => "t_plus_60",
            _ => "t_plus_n",
        };

        let update_result = stock_analyses::Entity::update_many()
            .col_expr(stock_analyses::Column::Outcome, Expr::value(outcome))
            .col_expr(
                stock_analyses::Column::UpdatedAt,
                Expr::value(chrono::Utc::now().timestamp_millis()),
            )
            .filter(stock_analyses::Column::Id.eq(&analysis.id))
            .exec(db)
            .await;

        if update_result.is_err() {
            continue;
        }

        // 5. 回写 lesson_applications.outcome_at_validation
        //    调用 core::update_lesson_application_outcome
        let affected = crate::commands::stock_workflow::core::update_lesson_application_outcome(
            db,
            &analysis.id,
            outcome,
            validation_source,
        )
        .await;

        if affected > 0 {
            synced += affected;
        }
    }

    synced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4_format() {
        let id = uuid_v4();
        // 格式: 8-4-4-4-12 hex
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
        assert!(id.split('-').next().unwrap().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_empty_report() {
        let r = empty_report();
        assert_eq!(r.total, 0);
        assert!(r.by_action.is_empty());
        assert!(r.by_style.is_empty());
        assert!(r.by_t_plus_n.is_empty());
        assert!(r.factor_ic.is_empty());
    }
}
