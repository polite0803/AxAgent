//! R2 组合监控主模块
//!
//! 5 个职责：
//! 1. `compute_dashboard` —— 组合聚合指标（5 个 metric card + 行业集中度 + 警告）
//! 2. `compute_correlation_matrix` —— 两两相关性（最多 20 只持仓一次算完）
//! 3. `run_stress_scenario` —— 3 场景压测（大盘 -10% / -20% / 黑天鹅）
//! 4. `refresh_metrics` —— 把当前快照写库
//! 5. `get_dashboard` / `get_timeline` —— 读快照（时间旅行按 as_of_date 走）
//!
//! 时间旅行：as_of_date 存在时，从 portfolio_metrics_daily 表查 <= as_of_date 的最新一行；
//! 缺数据返回默认空 dashboard（带 `isHistorical: true` 标记）。

use std::collections::HashMap;

use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::Set;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axagent_harness::market_data::MarketDataProvider;

use super::position_limits::PositionLimits;
use super::trading::PositionSummary;

// ── 数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioDashboard {
    /// 是否为历史快照（time travel 下为 true）
    pub is_historical: bool,
    pub as_of_date: Option<String>,
    pub total_market_value: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub cash_pct: f64,
    pub max_drawdown_pct: f64,
    pub beta: Option<f64>,
    pub sharpe_30d: Option<f64>,
    pub correlation_avg: Option<f64>,
    pub top_concentration_pct: f64,
    pub sector_exposure: HashMap<String, f64>,
    pub concentration_warning: Option<String>,
    pub risk_level: String,
    pub diversification_score: u32,
    pub stress_test: StressTestBundle,
    pub positions: Vec<PositionSummary>,
    pub snapshot_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StressTestBundle {
    pub m10: Option<StressTestResult>,
    pub m20: Option<StressTestResult>,
    pub black_swan: Option<StressTestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressTestResult {
    pub scenario: String,
    pub label: String,
    /// 组合整体 P&L 估值（元）
    pub portfolio_pnl: f64,
    /// 组合整体 P&L 百分比
    pub portfolio_pnl_pct: f64,
    /// 受影响最大的持仓（按 code / name / pnl_pct）
    pub top_hit: Option<PositionHit>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionHit {
    pub stock_code: String,
    pub stock_name: String,
    pub pnl_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationCell {
    pub code_a: String,
    pub code_b: String,
    pub correlation: f64,
}

// ── 纯函数：可独立测试 ──

/// 把"组合 P&L 序列"折算成最大回撤（百分比），复用 `crate::risk::peak_trough_drawdown`。
pub fn compute_max_drawdown_pct(equity_curve_pct: &[f64]) -> f64 {
    // peak_trough_drawdown 返回 0~1 比例，此处转为百分比
    crate::risk::peak_trough_drawdown(equity_curve_pct) * 100.0
}

/// Sharpe ratio（年化）—— 复用 `crate::risk::sharpe_components`（样本方差 n-1）。
pub fn compute_sharpe(returns_pct: &[f64], annualization: f64) -> Option<f64> {
    if returns_pct.len() < 5 {
        return None;
    }
    // sharpe_components 返回 (sharpe, annualized, mean_return, stddev)，取 annualized
    Some(crate::risk::sharpe_components(returns_pct, 0.0, annualization).1)
}

/// Pearson 相关系数（长度必须一致，>5 个点）
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 5 {
        return None;
    }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for (a, b) in x.iter().zip(y.iter()) {
        let da = a - mx;
        let db = b - my;
        num += da * db;
        dx2 += da * da;
        dy2 += db * db;
    }
    let denom = (dx2 * dy2).sqrt();
    if denom < 1e-9 {
        None
    } else {
        Some((num / denom).clamp(-1.0, 1.0))
    }
}

/// 组合 beta = Cov(组合, 市场) / Var(市场)
pub fn compute_beta(portfolio_returns: &[f64], market_returns: &[f64]) -> Option<f64> {
    if portfolio_returns.len() != market_returns.len() || portfolio_returns.len() < 10 {
        return None;
    }
    let n = portfolio_returns.len() as f64;
    let mp = portfolio_returns.iter().sum::<f64>() / n;
    let mm = market_returns.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut dm2 = 0.0;
    for (a, b) in portfolio_returns.iter().zip(market_returns.iter()) {
        let dp = a - mp;
        let dm = b - mm;
        num += dp * dm;
        dm2 += dm * dm;
    }
    if dm2 < 1e-9 {
        None
    } else {
        Some(num / dm2)
    }
}

/// 集中度（refactored from portfolio_risk::compute_from_positions）
pub fn compute_concentration(positions: &[PositionSummary]) -> (f64, HashMap<String, f64>, f64) {
    let total_mv: f64 = positions.iter().map(|p| p.market_value.unwrap_or(0.0)).sum();
    let max_mv = positions.iter().map(|p| p.market_value.unwrap_or(0.0)).fold(0.0_f64, f64::max);
    let top_pct = if total_mv > 0.0 {
        (max_mv / total_mv) * 100.0
    } else {
        0.0
    };

    let mut sector: HashMap<String, f64> = HashMap::new();
    for p in positions {
        if let (Some(mv), Some(s)) = (p.market_value, &p.sector_name) {
            if !s.is_empty() && total_mv > 0.0 {
                *sector.entry(s.clone()).or_default() += (mv / total_mv) * 100.0;
            }
        }
    }
    let max_sector_pct = sector.values().cloned().fold(0.0_f64, f64::max);
    (top_pct, sector, max_sector_pct)
}

/// 风险等级（与 portfolio_risk 对齐）
pub fn compute_risk_level(top_pct: f64, max_sector_pct: f64, n: usize) -> String {
    if n == 0 {
        return "无持仓".to_string();
    }
    if top_pct > 50.0 || max_sector_pct > 60.0 {
        "高风险".to_string()
    } else if top_pct > 30.0 || max_sector_pct > 40.0 {
        "中高风险".to_string()
    } else if top_pct > 20.0 || max_sector_pct > 30.0 {
        "中等风险".to_string()
    } else {
        "低风险".to_string()
    }
}

/// 分散度评分 0-100
pub fn compute_diversification_score(n: usize, top_pct: f64, max_sector_pct: f64) -> u32 {
    if n >= 8 && top_pct <= 15.0 && max_sector_pct < 30.0 {
        90
    } else if n >= 5 && top_pct <= 25.0 && max_sector_pct < 40.0 {
        70
    } else if n >= 3 && top_pct <= 35.0 {
        50
    } else if n >= 1 {
        30
    } else {
        0
    }
}

/// 集中度警告文本
pub fn compute_concentration_warning(
    top_pct: f64,
    max_sector_pct: f64,
    n: usize,
) -> Option<String> {
    let mut warns = Vec::new();
    if top_pct > 40.0 {
        warns.push(format!("单股集中度 {:.0}% 过高，建议 ≤30%", top_pct));
    } else if top_pct > 30.0 {
        warns.push(format!("单股集中度 {:.0}% 偏高，关注分散风险", top_pct));
    }
    if max_sector_pct > 50.0 {
        warns.push(format!("行业暴露 {:.0}% 过高，建议 ≤40%", max_sector_pct));
    }
    if n < 3 && n > 0 {
        warns.push(format!("仅 {} 只持仓，分散度不足，建议 ≥3 只", n));
    }
    if warns.is_empty() {
        None
    } else {
        Some(warns.join("；"))
    }
}

/// 压测：单股 i 在 scenario 下预计跌幅 = avg_beta_i * market_drop
///
/// 修复 P2-10: 原代码接收 `sector_exposure` 参数却完全未使用（`let _ = sector_exposure`），
/// 行业集中度风险被忽略。改为：当某行业暴露占比超过阈值（40%）时，该行业持仓的
/// 损失放大 1.2 倍——行业越集中，下跌时踩踏越严重（流动性折价 + 相关性坍缩）。
/// 返回 (组合总 P&L, P&L%, 受损最大持仓)
pub fn run_stress_scenario(
    positions: &[PositionSummary],
    sector_exposure: &HashMap<String, f64>,
    scenario: StressScenario,
) -> StressTestResult {
    let total_mv: f64 = positions.iter().map(|p| p.market_value.unwrap_or(0.0)).sum();
    if total_mv <= 0.0 || positions.is_empty() {
        return StressTestResult {
            scenario: scenario.code().to_string(),
            label: scenario.label().to_string(),
            portfolio_pnl: 0.0,
            portfolio_pnl_pct: 0.0,
            top_hit: None,
            note: "无持仓，跳过压测".to_string(),
        };
    }
    // 简化：单股用 sector 平均 beta 近似（科技 1.3 / 消费 0.7 / 银行 0.5 / 其他 1.0）
    let market_drop = scenario.market_drop();
    let mut total_pnl = 0.0;
    let mut worst_hit: Option<(f64, &PositionSummary)> = None;
    let mut concentration_penalty_applied = false;
    for p in positions {
        let mv = p.market_value.unwrap_or(0.0);
        let sector = p.sector_name.as_deref().unwrap_or("");
        let base_beta = sector_beta(sector);
        // 行业集中度惩罚：sector_exposure[sector] > 40% 时放大 beta
        let sector_pct = sector_exposure.get(sector).copied().unwrap_or(0.0);
        let adjusted_beta = if sector_pct > SECTOR_CONCENTRATION_THRESHOLD {
            concentration_penalty_applied = true;
            base_beta * SECTOR_CONCENTRATION_PENALTY
        } else {
            base_beta
        };
        let pct = adjusted_beta * market_drop * 100.0;
        let pnl = mv * adjusted_beta * market_drop;
        total_pnl += pnl;
        if worst_hit.as_ref().map(|(w, _)| pct < *w).unwrap_or(true) {
            worst_hit = Some((pct, p));
        }
    }
    let top = worst_hit.map(|(_, p)| PositionHit {
        stock_code: p.stock_code.clone(),
        stock_name: p.stock_name.clone(),
        pnl_pct: worst_hit.as_ref().map(|(w, _)| *w).unwrap_or(0.0),
    });
    let note = if concentration_penalty_applied {
        "线性近似 + 行业集中度惩罚：sector_pct>40% 时 beta × 1.2（流动性折价）".to_string()
    } else {
        "线性近似：单股跌幅 = sector_beta × 大盘跌幅".to_string()
    };
    StressTestResult {
        scenario: scenario.code().to_string(),
        label: scenario.label().to_string(),
        portfolio_pnl: total_pnl,
        portfolio_pnl_pct: (total_pnl / total_mv) * 100.0,
        top_hit: top,
        note,
    }
}

/// 行业集中度阈值：超过此值时压测中该行业持仓 beta 放大
const SECTOR_CONCENTRATION_THRESHOLD: f64 = 40.0;
/// 行业集中度惩罚系数：beta × 1.2（模拟流动性折价 + 相关性坍缩）
const SECTOR_CONCENTRATION_PENALTY: f64 = 1.2;

fn sector_beta(sector: &str) -> f64 {
    let s = sector.to_lowercase();
    if s.contains("科技") || s.contains("tech") || s.contains("it") || s.contains("互联网") {
        1.3
    } else if s.contains("消费") || s.contains("consumer") || s.contains("食品") {
        0.7
    } else if s.contains("银行")
        || s.contains("金融")
        || s.contains("bank")
        || s.contains("finance")
    {
        0.5
    } else if s.contains("医药") || s.contains("medical") || s.contains("health") {
        0.9
    } else if s.contains("能源") || s.contains("energy") {
        1.1
    } else if s.contains("地产") || s.contains("房地产") || s.contains("real_estate") {
        1.2
    } else if s.contains("公用") || s.contains("utility") {
        0.4
    } else {
        1.0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StressScenario {
    MarketDown10,
    MarketDown20,
    BlackSwan,
}

impl StressScenario {
    pub fn code(&self) -> &'static str {
        match self {
            StressScenario::MarketDown10 => "m10",
            StressScenario::MarketDown20 => "m20",
            StressScenario::BlackSwan => "blackSwan",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            StressScenario::MarketDown10 => "大盘 -10%",
            StressScenario::MarketDown20 => "大盘 -20%",
            StressScenario::BlackSwan => "黑天鹅 (-30%)",
        }
    }
    pub fn market_drop(&self) -> f64 {
        match self {
            StressScenario::MarketDown10 => -0.10,
            StressScenario::MarketDown20 => -0.20,
            StressScenario::BlackSwan => -0.30,
        }
    }
}

pub fn run_all_scenarios(
    positions: &[PositionSummary],
    sector_exposure: &HashMap<String, f64>,
) -> StressTestBundle {
    StressTestBundle {
        m10: Some(run_stress_scenario(positions, sector_exposure, StressScenario::MarketDown10)),
        m20: Some(run_stress_scenario(positions, sector_exposure, StressScenario::MarketDown20)),
        black_swan: Some(run_stress_scenario(
            positions,
            sector_exposure,
            StressScenario::BlackSwan,
        )),
    }
}

// ── 整合：组合 dashboard ──
// 多参数是组合监控的统一输出需求（alpha + 风险 + 压力测试），不打包为结构体以保持调用方扁平。
#[allow(clippy::too_many_arguments)]
pub fn compute_dashboard(
    positions: &[PositionSummary],
    _limits: &PositionLimits,
    beta: Option<f64>,
    sharpe_30d: Option<f64>,
    correlation_avg: Option<f64>,
    stress: StressTestBundle,
    is_historical: bool,
    as_of_date: Option<String>,
) -> PortfolioDashboard {
    // P0-2: 计算当前组合中各标的市值占比，用于暴露度分析
    // 注: portfolio-mgr 的"建议仓位"存在 stock_analyses 表，不在此处聚合。
    //     真正的组合仓位归一化由 normalize_position_weights 函数提供，
    //     待前端/命令层集成后启用（当前版本保持 API 可用）。
    let total_mv: f64 = positions.iter().map(|p| p.market_value.unwrap_or(0.0)).sum();
    let total_pnl: f64 = positions.iter().map(|p| p.unrealized_pnl.unwrap_or(0.0)).sum();
    let total_cost: f64 = positions.iter().map(|p| p.avg_cost * p.total_shares as f64).sum();
    let total_pnl_pct = if total_cost > 0.0 {
        (total_pnl / total_cost) * 100.0
    } else {
        0.0
    };
    let n = positions.len();
    let (top_pct, sector, max_sector_pct) = compute_concentration(positions);
    let risk_level = compute_risk_level(top_pct, max_sector_pct, n);
    let div_score = compute_diversification_score(n, top_pct, max_sector_pct);
    let warning = compute_concentration_warning(top_pct, max_sector_pct, n);

    PortfolioDashboard {
        is_historical,
        as_of_date,
        total_market_value: total_mv,
        total_pnl,
        total_pnl_pct,
        cash_pct: 0.0,         // 由 refresh_metrics 在落库前用"现金/总资产"补算
        max_drawdown_pct: 0.0, // 由 refresh_metrics 走历史
        beta,
        sharpe_30d,
        correlation_avg,
        top_concentration_pct: top_pct,
        sector_exposure: sector,
        concentration_warning: warning,
        risk_level,
        diversification_score: div_score,
        stress_test: stress,
        positions: positions.to_vec(),
        snapshot_at: chrono::Utc::now().timestamp_millis(),
    }
}

// ── P0-2: 组合仓位归一化 ──
// portfolio-mgr 对每只股票独立输出 position_pct，各标的仓位之和可能远超 100%。
// 例如 4 只标的同时建议 35% → 总仓位 140%，实际无法执行。
// 本函数将传入的仓位列表按比例压缩到总上限内，同时保持相对权重不变。
//
// 场景：
//   A. raw_sum ≤ cap → 不压缩（仓位已经自洽）
//   B. raw_sum > cap → 按 cap/raw_sum 比例压缩
//
// 例: [30%, 25%, 20%] sum=75% ≤ 100% → 不变
//     [50%, 40%, 30%] sum=120% > 100% → 各 ×100/120=[41.7%, 33.3%, 25.0%]
pub fn normalize_position_weights(positions: &[f64], max_total_pct: f64) -> Vec<f64> {
    let raw_sum: f64 = positions.iter().sum();
    if raw_sum <= max_total_pct || raw_sum <= 0.0 {
        return positions.to_vec();
    }
    let factor = max_total_pct / raw_sum;
    positions.iter().map(|&p| (p * factor * 10.0).round() / 10.0).collect()
}

// ── 持久化层 ──

pub async fn refresh_metrics(
    db: &DatabaseConnection,
    positions: &[PositionSummary],
    limits: &PositionLimits,
    beta: Option<f64>,
    sharpe_30d: Option<f64>,
    correlation_avg: Option<f64>,
    as_of_date: Option<&str>,
) -> Result<(String, u32), String> {
    let today = as_of_date
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let stress = run_all_scenarios(positions, &compute_concentration(positions).1);
    let dashboard = compute_dashboard(
        positions,
        limits,
        beta,
        sharpe_30d,
        correlation_avg,
        stress,
        false,
        Some(today.clone()),
    );

    let sector_json = serde_json::to_string(&dashboard.sector_exposure)
        .map_err(|e| format!("serialize sector_exposure: {e}"))?;
    let stress_json = serde_json::to_string(&dashboard.stress_test)
        .map_err(|e| format!("serialize stress_test: {e}"))?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let new_row = axagent_entities::portfolio_metrics_daily::ActiveModel {
        id: Set(id.clone()),
        snapshot_date: Set(today),
        total_market_value: Set(dashboard.total_market_value),
        cash_pct: Set(dashboard.cash_pct),
        total_pnl: Set(dashboard.total_pnl),
        total_pnl_pct: Set(dashboard.total_pnl_pct),
        max_drawdown_pct: Set(dashboard.max_drawdown_pct),
        beta: Set(dashboard.beta),
        sharpe_30d: Set(dashboard.sharpe_30d),
        correlation_avg: Set(dashboard.correlation_avg),
        top_concentration_pct: Set(dashboard.top_concentration_pct),
        sector_exposure_json: Set(sector_json),
        stress_test_json: Set(Some(stress_json)),
        created_at: Set(now),
    };
    new_row.insert(db).await.map_err(|e| format!("insert portfolio_metrics_daily: {e}"))?;
    Ok((id, 1))
}

pub async fn get_dashboard(
    db: &DatabaseConnection,
    as_of_date: Option<&str>,
) -> Result<PortfolioDashboard, String> {
    use axagent_entities::portfolio_metrics_daily;

    let row = if let Some(date) = as_of_date {
        // time travel：取 <= as_of_date 的最新一行
        portfolio_metrics_daily::Entity::find()
            .filter(portfolio_metrics_daily::Column::SnapshotDate.lte(date.to_string()))
            .order_by_desc(portfolio_metrics_daily::Column::SnapshotDate)
            .one(db)
            .await
            .map_err(|e| format!("query portfolio_metrics_daily: {e}"))?
    } else {
        portfolio_metrics_daily::Entity::find()
            .order_by_desc(portfolio_metrics_daily::Column::SnapshotDate)
            .one(db)
            .await
            .map_err(|e| format!("query portfolio_metrics_daily: {e}"))?
    };

    match row {
        Some(m) => {
            let sector: HashMap<String, f64> =
                serde_json::from_str(&m.sector_exposure_json).unwrap_or_default();
            let max_sector_pct = sector.values().cloned().fold(0.0_f64, f64::max);
            let stress: StressTestBundle = m
                .stress_test_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            Ok(PortfolioDashboard {
                is_historical: as_of_date.is_some(),
                as_of_date: as_of_date.map(|s| s.to_string()).or(Some(m.snapshot_date.clone())),
                total_market_value: m.total_market_value,
                total_pnl: m.total_pnl,
                total_pnl_pct: m.total_pnl_pct,
                cash_pct: m.cash_pct,
                max_drawdown_pct: m.max_drawdown_pct,
                beta: m.beta,
                sharpe_30d: m.sharpe_30d,
                correlation_avg: m.correlation_avg,
                top_concentration_pct: m.top_concentration_pct,
                sector_exposure: sector,
                concentration_warning: compute_concentration_warning(
                    m.top_concentration_pct,
                    max_sector_pct,
                    0,
                ),
                risk_level: "—".to_string(),
                diversification_score: 0,
                stress_test: stress,
                positions: vec![],
                snapshot_at: m.created_at,
            })
        },
        None => {
            // 空 dashboard
            Ok(PortfolioDashboard {
                is_historical: as_of_date.is_some(),
                as_of_date: as_of_date.map(|s| s.to_string()),
                total_market_value: 0.0,
                total_pnl: 0.0,
                total_pnl_pct: 0.0,
                cash_pct: 0.0,
                max_drawdown_pct: 0.0,
                beta: None,
                sharpe_30d: None,
                correlation_avg: None,
                top_concentration_pct: 0.0,
                sector_exposure: HashMap::new(),
                concentration_warning: Some("尚无快照数据，请点击「刷新」".to_string()),
                risk_level: "无持仓".to_string(),
                diversification_score: 0,
                stress_test: StressTestBundle::default(),
                positions: vec![],
                snapshot_at: 0,
            })
        },
    }
}

/// 计算并落库两两相关性（拉 K 线、pearson、写库）
pub async fn refresh_correlation(
    db: &DatabaseConnection,
    client: &dyn MarketDataProvider,
    positions: &[PositionSummary],
    lookback_days: u32,
    as_of_date: Option<&str>,
) -> Result<u32, String> {
    use axagent_entities::portfolio_correlation_snapshot;

    if positions.len() < 2 {
        return Ok(0);
    }
    // N≤20 全算；N>20 退化为只算与最大持仓的相关性
    let anchor = positions
        .iter()
        .max_by(|a, b| {
            a.market_value
                .unwrap_or(0.0)
                .partial_cmp(&b.market_value.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| p.stock_code.clone())
        .unwrap_or_default();

    let codes: Vec<String> = if positions.len() <= 20 {
        positions.iter().map(|p| p.stock_code.clone()).collect()
    } else {
        vec![anchor.clone()]
    };
    let pairs: Vec<(String, String)> = if positions.len() <= 20 {
        let mut out = Vec::new();
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                out.push((codes[i].clone(), codes[j].clone()));
            }
        }
        out
    } else {
        positions
            .iter()
            .filter(|p| p.stock_code != anchor)
            .map(|p| (anchor.clone(), p.stock_code.clone()))
            .collect()
    };
    if pairs.is_empty() {
        return Ok(0);
    }

    // 拉每只股票的 K 线
    let mut series: HashMap<String, Vec<f64>> = HashMap::new();
    for code in &codes {
        match client.get_klines(code, "daily", lookback_days, None).await {
            Ok(ks) => {
                let closes: Vec<f64> = ks.iter().map(|k| k.close).collect();
                if closes.len() >= 5 {
                    series.insert(code.clone(), closes);
                }
            },
            Err(e) => {
                eprintln!("[portfolio_monitor] kline fetch failed for {code}: {e}");
            },
        }
    }
    if series.len() < 2 {
        return Ok(0);
    }

    let today = as_of_date
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let mut written = 0u32;
    for (a, b) in pairs {
        let (Some(x), Some(y)) = (series.get(&a), series.get(&b)) else {
            continue;
        };
        // 长度对齐：取较短者尾部
        let n = x.len().min(y.len());
        let x_tail: Vec<f64> = x[x.len() - n..].to_vec();
        let y_tail: Vec<f64> = y[y.len() - n..].to_vec();
        let corr = match pearson_correlation(&x_tail, &y_tail) {
            Some(c) => c,
            None => continue,
        };
        let id = Uuid::new_v4().to_string();
        let row = portfolio_correlation_snapshot::ActiveModel {
            id: Set(id),
            snapshot_date: Set(today.clone()),
            lookback_days: Set(lookback_days as i32),
            code_a: Set(a),
            code_b: Set(b),
            correlation: Set(corr),
            created_at: Set(now),
        };
        if row.insert(db).await.is_ok() {
            written += 1;
        }
    }
    Ok(written)
}

/// 读最近一次相关性快照（按 snapshot_date desc）
pub async fn get_correlation_snapshot(
    db: &DatabaseConnection,
    as_of_date: Option<&str>,
) -> Result<Vec<CorrelationCell>, String> {
    use axagent_entities::portfolio_correlation_snapshot;

    // 找到最近一次 snapshot_date
    let latest_date: Option<String> = if let Some(date) = as_of_date {
        portfolio_correlation_snapshot::Entity::find()
            .filter(portfolio_correlation_snapshot::Column::SnapshotDate.lte(date.to_string()))
            .select_only()
            .column(portfolio_correlation_snapshot::Column::SnapshotDate)
            .order_by_desc(portfolio_correlation_snapshot::Column::SnapshotDate)
            .into_tuple()
            .one(db)
            .await
            .map_err(|e| format!("query latest corr date: {e}"))?
    } else {
        portfolio_correlation_snapshot::Entity::find()
            .select_only()
            .column(portfolio_correlation_snapshot::Column::SnapshotDate)
            .order_by_desc(portfolio_correlation_snapshot::Column::SnapshotDate)
            .into_tuple()
            .one(db)
            .await
            .map_err(|e| format!("query latest corr date: {e}"))?
    };
    let Some(date) = latest_date else { return Ok(vec![]) };
    let rows = portfolio_correlation_snapshot::Entity::find()
        .filter(portfolio_correlation_snapshot::Column::SnapshotDate.eq(date.clone()))
        .all(db)
        .await
        .map_err(|e| format!("query corr rows: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|r| CorrelationCell { code_a: r.code_a, code_b: r.code_b, correlation: r.correlation })
        .collect())
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn ps(code: &str, mv: f64, sector: Option<&str>, cost: f64) -> PositionSummary {
        PositionSummary {
            stock_code: code.into(),
            stock_name: code.into(),
            total_shares: 100,
            avg_cost: cost,
            current_price: Some(mv / 100.0),
            market_value: Some(mv),
            unrealized_pnl: Some(mv - cost * 100.0),
            unrealized_pnl_pct: Some(((mv - cost * 100.0) / (cost * 100.0)) * 100.0),
            total_realized_pnl: 0.0,
            sector_name: sector.map(|s| s.to_string()),
        }
    }

    #[test]
    fn empty_positions_returns_zero_concentration() {
        let (top, sector, max_sec) = compute_concentration(&[]);
        assert_eq!(top, 0.0);
        assert!(sector.is_empty());
        assert_eq!(max_sec, 0.0);
    }

    #[test]
    fn single_position_full_concentration() {
        let p = ps("000001", 10000.0, Some("银行"), 50.0);
        let (top, sector, _max) = compute_concentration(&[p]);
        assert!((top - 100.0).abs() < 1e-6);
        assert_eq!(sector.get("银行"), Some(&100.0));
    }

    #[test]
    fn multi_position_concentration_proportional() {
        let pos = vec![
            ps("a", 6000.0, Some("科技"), 50.0),
            ps("b", 3000.0, Some("科技"), 50.0),
            ps("c", 1000.0, Some("消费"), 50.0),
        ];
        let (top, sector, max_sec) = compute_concentration(&pos);
        assert!((top - 60.0).abs() < 1e-6);
        assert!((sector["科技"] - 90.0).abs() < 1e-6);
        assert!((sector["消费"] - 10.0).abs() < 1e-6);
        assert!((max_sec - 90.0).abs() < 1e-6);
    }

    #[test]
    fn risk_level_thresholds() {
        assert_eq!(compute_risk_level(10.0, 10.0, 5), "低风险");
        assert_eq!(compute_risk_level(25.0, 35.0, 3), "中等风险");
        assert_eq!(compute_risk_level(35.0, 35.0, 3), "中高风险");
        assert_eq!(compute_risk_level(60.0, 60.0, 3), "高风险");
        assert_eq!(compute_risk_level(0.0, 0.0, 0), "无持仓");
    }

    #[test]
    fn max_drawdown_handles_empty() {
        assert_eq!(compute_max_drawdown_pct(&[]), 0.0);
        assert_eq!(compute_max_drawdown_pct(&[0.0]), 0.0);
    }

    #[test]
    fn max_drawdown_basic_curve() {
        // 价格水平曲线：100 → 120（peak）→ 96 → 88（trough）
        // dd = (120 - 88) / 120 = 26.67%
        let curve = vec![100.0, 110.0, 120.0, 96.0, 88.0];
        let dd = compute_max_drawdown_pct(&curve);
        assert!(dd > 26.0 && dd < 28.0, "dd = {dd}");
    }

    #[test]
    fn sharpe_rejects_too_few_points() {
        // P3-C8: 年化因子切换为 A 股 244 天
        assert!(compute_sharpe(&[1.0, 2.0, 3.0], 244.0).is_none());
    }

    #[test]
    fn sharpe_basic_calculation() {
        // 6 个点 1% mean 0.5% std → sharpe = 1/0.5 * sqrt(244) ≈ 31.2
        let r = vec![0.5, 1.0, 1.5, 1.0, 0.5, 1.5];
        let s = compute_sharpe(&r, 244.0).unwrap();
        assert!(s > 25.0 && s < 40.0, "sharpe = {s}");
    }

    #[test]
    fn pearson_perfect_positive_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((pearson_correlation(&x, &y).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pearson_perfect_negative_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let c = pearson_correlation(&x, &y).unwrap();
        assert!((-1.0..-0.99).contains(&c));
    }

    #[test]
    fn pearson_too_short_returns_none() {
        assert!(pearson_correlation(&[1.0, 2.0], &[3.0, 4.0]).is_none());
    }

    #[test]
    fn stress_scenario_m10_basic() {
        let pos = vec![ps("a", 10000.0, Some("科技"), 50.0)];
        let sector: HashMap<String, f64> = [("科技".into(), 100.0)].into_iter().collect();
        let r = run_stress_scenario(&pos, &sector, StressScenario::MarketDown10);
        // 科技 beta=1.3, m10=10%, sector_pct=100%>40% → beta×1.2=1.56 → 单股 -15.6%
        assert!(
            r.portfolio_pnl_pct < -14.0 && r.portfolio_pnl_pct > -17.0,
            "pct = {}",
            r.portfolio_pnl_pct
        );
        assert_eq!(r.top_hit.as_ref().unwrap().stock_code, "a");
        assert!(r.note.contains("行业集中度惩罚"));
    }

    #[test]
    fn stress_scenario_no_penalty_when_sector_low() {
        let pos = vec![ps("a", 10000.0, Some("科技"), 50.0)];
        let sector: HashMap<String, f64> = [("科技".into(), 30.0)].into_iter().collect();
        let r = run_stress_scenario(&pos, &sector, StressScenario::MarketDown10);
        // sector_pct=30% < 40% 阈值 → 无惩罚，beta=1.3 → 单股 -13%
        assert!(
            r.portfolio_pnl_pct < -12.0 && r.portfolio_pnl_pct > -14.0,
            "pct = {}",
            r.portfolio_pnl_pct
        );
        assert!(!r.note.contains("行业集中度惩罚"));
    }

    #[test]
    fn stress_scenario_empty_positions() {
        let r = run_stress_scenario(&[], &HashMap::new(), StressScenario::BlackSwan);
        assert_eq!(r.portfolio_pnl, 0.0);
        assert!(r.top_hit.is_none());
        assert!(r.note.contains("无持仓"));
    }

    #[test]
    fn concentration_warning_under_threshold() {
        assert!(compute_concentration_warning(20.0, 30.0, 5).is_none());
    }

    #[test]
    fn concentration_warning_multi_issues() {
        let w = compute_concentration_warning(45.0, 55.0, 2).unwrap();
        assert!(w.contains("单股集中度"));
        assert!(w.contains("行业暴露"));
        assert!(w.contains("分散度不足"));
    }

    #[test]
    fn diversification_score_buckets() {
        assert_eq!(compute_diversification_score(0, 0.0, 0.0), 0);
        assert_eq!(compute_diversification_score(2, 40.0, 50.0), 30);
        assert_eq!(compute_diversification_score(5, 20.0, 35.0), 70);
        assert_eq!(compute_diversification_score(10, 10.0, 20.0), 90);
    }

    #[test]
    fn run_all_scenarios_returns_three() {
        let pos = vec![ps("a", 10000.0, Some("银行"), 50.0)];
        let s = run_all_scenarios(&pos, &HashMap::new());
        assert!(s.m10.is_some());
        assert!(s.m20.is_some());
        assert!(s.black_swan.is_some());
    }

    #[test]
    fn sector_beta_buckets() {
        assert!((sector_beta("科技") - 1.3).abs() < 1e-9);
        assert!((sector_beta("银行") - 0.5).abs() < 1e-9);
        assert!((sector_beta("消费") - 0.7).abs() < 1e-9);
        assert!((sector_beta("") - 1.0).abs() < 1e-9);
    }
}
