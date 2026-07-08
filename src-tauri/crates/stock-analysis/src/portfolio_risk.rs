use std::collections::HashMap;

use crate::trading::PositionSummary;

/// 组合风险指标（保持向后兼容的旧 API）
///
/// 实际计算已迁到 `portfolio_monitor` 纯函数模块，本文件只做数据
/// 结构定义 + 薄封装调用。R2 起所有组合层算法都走 `portfolio_monitor`。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioRiskMetrics {
    pub total_positions: usize,
    pub total_market_value: f64,
    pub top_concentration_pct: f64,
    pub sector_exposure: HashMap<String, f64>,
    pub diversification_score: u32,
    pub risk_level: String,
    pub warning: Option<String>,
    pub correlation_risk: String,
}

pub struct PortfolioRiskManager;

impl PortfolioRiskManager {
    /// 组合风险指标（薄封装 → 实际算法在 portfolio_monitor）
    pub fn compute_from_positions(positions: &[PositionSummary]) -> PortfolioRiskMetrics {
        let (top_concentration_pct, sector_exposure, max_sector_pct) =
            crate::portfolio_monitor::compute_concentration(positions);
        let total_market_value: f64 = positions
            .iter()
            .map(|p| p.market_value.unwrap_or(0.0))
            .sum();
        let n = positions.len();

        let risk_level =
            crate::portfolio_monitor::compute_risk_level(top_concentration_pct, max_sector_pct, n);
        let diversification_score = crate::portfolio_monitor::compute_diversification_score(
            n,
            top_concentration_pct,
            max_sector_pct,
        );
        let warning = crate::portfolio_monitor::compute_concentration_warning(
            top_concentration_pct,
            max_sector_pct,
            n,
        );

        let correlation_risk = if n >= 5 && max_sector_pct < 30.0 {
            "低"
        } else if n >= 3 && max_sector_pct < 50.0 {
            "中"
        } else {
            "高"
        }
        .to_string();

        PortfolioRiskMetrics {
            total_positions: n,
            total_market_value,
            top_concentration_pct,
            sector_exposure,
            diversification_score,
            risk_level,
            warning,
            correlation_risk,
        }
    }
}
