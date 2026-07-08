//! 基本面分析报告 (Phase 1 + Phase 2)
//!
//! Phase 2: 迁移到 astock-data 层,被工作流 t-fundamentals-data 节点
//! (tools/stock_data.rs::StockFundamentalsReportTool) 调用。
//!
//! Phase 1 时位于 axagent-stock-analysis,但 tools 依赖 stock-analysis
//! 会形成 cycle(stock-analysis 自身依赖 tools),故迁移到底层 astock-data。

use crate::{FinancialReport, StockQuote};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialRatios {
    pub pe: Option<f64>,
    pub pb: Option<f64>,
    pub ps: Option<f64>,
    pub peg: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub gross_margin: Option<f64>,
    pub net_margin: Option<f64>,
    pub debt_ratio: Option<f64>,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub fcf: Option<f64>,
    pub fcf_yield: Option<f64>,
    pub revenue_yoy: Option<f64>,
    pub profit_yoy: Option<f64>,
    pub eps: Option<f64>,
    pub bps: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    #[default]
    Weak,
    Average,
    Healthy,
    Excellent,
}

impl HealthLevel {
    pub fn from_score(score: u32) -> Self {
        match score {
            0..=40 => HealthLevel::Weak,
            41..=60 => HealthLevel::Average,
            61..=80 => HealthLevel::Healthy,
            _ => HealthLevel::Excellent,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HealthLevel::Weak => "弱",
            HealthLevel::Average => "一般",
            HealthLevel::Healthy => "良好",
            HealthLevel::Excellent => "优秀",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsReport {
    pub stock_code: String,
    pub stock_name: String,
    pub report_date: String,
    pub ratios: FinancialRatios,
    pub health_score: u32,
    pub health_level: HealthLevel,
    pub key_takeaways: Vec<String>,
    pub data_completeness: f64,
}

pub struct FundamentalsAnalyzer;

impl FundamentalsAnalyzer {
    pub fn generate(
        stock_code: &str,
        quote: &StockQuote,
        financials: &[FinancialReport],
    ) -> FundamentalsReport {
        let latest = financials.first();
        let ratios = Self::compute_ratios(quote, latest);
        let health_score = Self::health_score(&ratios);
        let health_level = HealthLevel::from_score(health_score);
        let key_takeaways = Self::takeaways(quote, &ratios);
        let data_completeness = Self::completeness(&ratios);

        FundamentalsReport {
            stock_code: stock_code.to_string(),
            stock_name: quote.name.clone(),
            report_date: latest
                .map(|f| f.report_date.clone())
                .unwrap_or_else(|| quote.timestamp.clone()),
            ratios,
            health_score,
            health_level,
            key_takeaways,
            data_completeness,
        }
    }

    pub fn compute_ratios(quote: &StockQuote, latest: Option<&FinancialReport>) -> FinancialRatios {
        let mut r = FinancialRatios::default();

        if let Some(f) = latest {
            r.roe = f.roe;
            r.gross_margin = f.gross_margin;
            r.net_margin = f.net_margin;
            r.debt_ratio = f.debt_ratio;
            r.current_ratio = f.current_ratio;
            r.quick_ratio = f.quick_ratio;
            r.fcf = f.free_cash_flow;
            r.revenue_yoy = f.revenue_yoy;
            r.profit_yoy = f.profit_yoy;
            r.eps = f.eps;
            r.bps = f.bps;
            r.roa = if let (Some(np), Some(ta)) = (f.net_profit, f.total_assets) {
                if ta > 0.0 {
                    Some(np / ta * 100.0)
                } else {
                    None
                }
            } else {
                None
            };
        }

        if let Some(pe) = quote.pe {
            r.pe = Some(pe);
        } else if let (Some(eps), true) = (r.eps, eps_is_valid(r.eps)) {
            r.pe = Some(quote.price / eps);
        }
        if let Some(pb) = quote.pb {
            r.pb = Some(pb);
        } else if let (Some(bps), true) = (r.bps, bps_is_valid(r.bps)) {
            r.pb = Some(quote.price / bps);
        }
        if let (Some(rev), Some(mv)) = (latest.and_then(|f| f.revenue), quote.total_mv) {
            if rev > 0.0 {
                r.ps = Some(mv / rev);
            }
        }
        if let (Some(pe), Some(yoy)) = (r.pe, r.revenue_yoy) {
            if yoy.abs() > 0.0 && pe > 0.0 {
                r.peg = Some(pe / yoy);
            }
        }
        if let (Some(mv), Some(fcf)) = (quote.total_mv, r.fcf) {
            if mv > 0.0 {
                r.fcf_yield = Some(fcf / mv * 100.0);
            }
        }

        r
    }

    pub fn health_score(r: &FinancialRatios) -> u32 {
        let profit_score = r
            .roe
            .map(|roe| {
                if roe >= 20.0 {
                    25
                } else if roe >= 15.0 {
                    20
                } else if roe >= 10.0 {
                    15
                } else if roe >= 5.0 {
                    10
                } else {
                    5
                }
            })
            .unwrap_or(0);

        let valuation_score =
            r.pe.map(|pe| {
                if pe <= 0.0 {
                    0
                } else if pe < 15.0 {
                    25
                } else if pe < 25.0 {
                    20
                } else if pe < 40.0 {
                    12
                } else {
                    5
                }
            })
            .unwrap_or(0);

        let growth_score = r
            .revenue_yoy
            .map(|yoy| {
                if yoy >= 30.0 {
                    25
                } else if yoy >= 15.0 {
                    20
                } else if yoy >= 5.0 {
                    12
                } else if yoy >= 0.0 {
                    8
                } else {
                    3
                }
            })
            .unwrap_or(0);

        let debt_score = r
            .debt_ratio
            .map(|d| {
                if d < 30.0 {
                    15
                } else if d < 50.0 {
                    12
                } else if d < 70.0 {
                    6
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let cash_score = r
            .fcf_yield
            .map(|y| {
                if y >= 8.0 {
                    10
                } else if y >= 4.0 {
                    7
                } else if y >= 0.0 {
                    4
                } else {
                    0
                }
            })
            .unwrap_or(0);

        (profit_score + valuation_score + growth_score + debt_score + cash_score).min(100)
    }

    pub fn takeaways(quote: &StockQuote, r: &FinancialRatios) -> Vec<String> {
        let mut t = Vec::new();
        if let Some(pe) = r.pe {
            t.push(format!("静态 PE = {:.1}", pe));
        }
        if let Some(pb) = r.pb {
            t.push(format!("PB = {:.2}", pb));
        }
        if let Some(roe) = r.roe {
            t.push(format!("ROE = {:.1}%", roe));
        }
        if let Some(yoy) = r.revenue_yoy {
            let dir = if yoy > 0.0 { "增长" } else { "下滑" };
            t.push(format!("营收同比 {:.1}% ({})", yoy, dir));
        }
        if let Some(yoy) = r.profit_yoy {
            let dir = if yoy > 0.0 { "增长" } else { "下滑" };
            t.push(format!("净利同比 {:.1}% ({})", yoy, dir));
        }
        if let Some(d) = r.debt_ratio {
            if d > 70.0 {
                t.push(format!("⚠️ 资产负债率偏高 {:.1}%", d));
            }
        }
        let market = crate::detect_market_type(&quote.code);
        let pct = crate::get_price_limit_pct(market);
        if let Some(limit_up) = quote.limit_up {
            t.push(format!("涨停价 = {:.2} (+{:.0}%)", limit_up, pct));
        }
        if let Some(limit_down) = quote.limit_down {
            t.push(format!("跌停价 = {:.2} (-{:.0}%)", limit_down, pct));
        }
        t
    }

    pub fn completeness(r: &FinancialRatios) -> f64 {
        let total = 14.0;
        let filled = [
            r.pe.is_some(),
            r.pb.is_some(),
            r.ps.is_some(),
            r.peg.is_some(),
            r.roe.is_some(),
            r.roa.is_some(),
            r.gross_margin.is_some(),
            r.net_margin.is_some(),
            r.debt_ratio.is_some(),
            r.current_ratio.is_some(),
            r.quick_ratio.is_some(),
            r.fcf.is_some(),
            r.fcf_yield.is_some(),
            r.revenue_yoy.is_some(),
        ]
        .iter()
        .filter(|x| **x)
        .count() as f64;
        (filled / total).min(1.0)
    }
}

fn eps_is_valid(eps: Option<f64>) -> bool {
    matches!(eps, Some(v) if v > 0.0)
}
fn bps_is_valid(bps: Option<f64>) -> bool {
    matches!(bps, Some(v) if v > 0.0)
}

impl FundamentalsReport {
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("## {} ({}) 基本面报告\n\n", self.stock_name, self.stock_code));
        s.push_str(&format!(
            "- 报告日期：{}\n- 数据完整度：{:.0}%\n- 综合评分：**{} / 100** ({})\n\n",
            self.report_date,
            self.data_completeness * 100.0,
            self.health_score,
            self.health_level.label()
        ));

        s.push_str("### 关键指标\n\n");
        s.push_str("| 维度 | 指标 | 数值 |\n|---|---|---|\n");
        let r = &self.ratios;
        if let Some(v) = r.pe {
            s.push_str(&format!("| 估值 | PE | {:.2} |\n", v));
        }
        if let Some(v) = r.pb {
            s.push_str(&format!("| 估值 | PB | {:.2} |\n", v));
        }
        if let Some(v) = r.ps {
            s.push_str(&format!("| 估值 | PS | {:.2} |\n", v));
        }
        if let Some(v) = r.peg {
            s.push_str(&format!("| 估值 | PEG | {:.2} |\n", v));
        }
        if let Some(v) = r.roe {
            s.push_str(&format!("| 盈利 | ROE | {:.1}% |\n", v));
        }
        if let Some(v) = r.roa {
            s.push_str(&format!("| 盈利 | ROA | {:.1}% |\n", v));
        }
        if let Some(v) = r.gross_margin {
            s.push_str(&format!("| 盈利 | 毛利率 | {:.1}% |\n", v));
        }
        if let Some(v) = r.net_margin {
            s.push_str(&format!("| 盈利 | 净利率 | {:.1}% |\n", v));
        }
        if let Some(v) = r.debt_ratio {
            s.push_str(&format!("| 偿债 | 资产负债率 | {:.1}% |\n", v));
        }
        if let Some(v) = r.current_ratio {
            s.push_str(&format!("| 偿债 | 流动比率 | {:.2} |\n", v));
        }
        if let Some(v) = r.quick_ratio {
            s.push_str(&format!("| 偿债 | 速动比率 | {:.2} |\n", v));
        }
        if let Some(v) = r.fcf_yield {
            s.push_str(&format!("| 现金流 | FCF 收益率 | {:.1}% |\n", v));
        }
        if let Some(v) = r.revenue_yoy {
            s.push_str(&format!("| 成长 | 营收同比 | {:.1}% |\n", v));
        }
        if let Some(v) = r.profit_yoy {
            s.push_str(&format!("| 成长 | 净利同比 | {:.1}% |\n", v));
        }

        if !self.key_takeaways.is_empty() {
            s.push_str("\n### 关键结论\n\n");
            for t in &self.key_takeaways {
                s.push_str(&format!("- {}\n", t));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote(price: f64) -> StockQuote {
        StockQuote {
            code: "600519".into(),
            name: "贵州茅台".into(),
            price,
            pre_close: price * 0.99,
            open: price * 0.995,
            high: price * 1.01,
            low: price * 0.98,
            volume: 1e6,
            amount: 1e9,
            change_pct: 1.0,
            turnover_rate: 0.3,
            pe: Some(35.0),
            pb: Some(12.0),
            total_mv: Some(2.0e12),
            circulating_mv: None,
            limit_up: Some(price * 1.1),
            limit_down: Some(price * 0.9),
            is_st: false,
            timestamp: "2026-01-15 14:00:00".into(),
        }
    }

    fn sample_financials() -> Vec<FinancialReport> {
        vec![FinancialReport {
            stock_code: "600519".into(),
            report_date: "2025-09-30".into(),
            revenue: Some(1.0e11),
            net_profit: Some(5.0e10),
            eps: Some(50.0),
            bps: Some(180.0),
            roe: Some(28.0),
            debt_ratio: Some(20.0),
            gross_margin: Some(90.0),
            net_margin: Some(50.0),
            revenue_yoy: Some(15.0),
            profit_yoy: Some(18.0),
            total_assets: Some(2.0e11),
            operating_cash_flow: Some(6.0e10),
            capital_expenditure: Some(1.0e9),
            free_cash_flow: Some(5.9e10),
            current_ratio: Some(2.5),
            quick_ratio: Some(2.0),
        }]
    }

    #[test]
    fn ratios_uses_quote_pe_when_present() {
        let q = sample_quote(1800.0);
        let f = sample_financials();
        let ratios = FundamentalsAnalyzer::compute_ratios(&q, f.first());
        assert_eq!(ratios.pe, Some(35.0));
        assert_eq!(ratios.pb, Some(12.0));
        assert_eq!(ratios.roe, Some(28.0));
    }

    #[test]
    fn ratios_computes_pe_from_eps_when_quote_missing() {
        let mut q = sample_quote(100.0);
        q.pe = None;
        let f = sample_financials();
        let ratios = FundamentalsAnalyzer::compute_ratios(&q, f.first());
        assert_eq!(ratios.pe, Some(2.0));
    }

    #[test]
    fn health_score_excellent_for_blue_chip() {
        let q = sample_quote(1800.0);
        let f = sample_financials();
        let r = FundamentalsAnalyzer::compute_ratios(&q, f.first());
        let score = FundamentalsAnalyzer::health_score(&r);
        assert!(score >= 70, "茅台数据应评优秀,实际 {}", score);
    }

    #[test]
    fn health_level_buckets() {
        assert_eq!(HealthLevel::from_score(0), HealthLevel::Weak);
        assert_eq!(HealthLevel::from_score(40), HealthLevel::Weak);
        assert_eq!(HealthLevel::from_score(41), HealthLevel::Average);
        assert_eq!(HealthLevel::from_score(60), HealthLevel::Average);
        assert_eq!(HealthLevel::from_score(61), HealthLevel::Healthy);
        assert_eq!(HealthLevel::from_score(80), HealthLevel::Healthy);
        assert_eq!(HealthLevel::from_score(81), HealthLevel::Excellent);
        assert_eq!(HealthLevel::from_score(100), HealthLevel::Excellent);
    }

    #[test]
    fn health_score_handles_missing_data() {
        let r = FinancialRatios::default();
        let s = FundamentalsAnalyzer::health_score(&r);
        assert_eq!(s, 0);
    }

    #[test]
    fn completeness_counts_filled_fields() {
        let mut r = FinancialRatios::default();
        r.pe = Some(10.0);
        r.pb = Some(2.0);
        let c = FundamentalsAnalyzer::completeness(&r);
        assert!((c - 2.0 / 14.0).abs() < 1e-6);
    }

    #[test]
    fn generate_full_report() {
        let q = sample_quote(1800.0);
        let f = sample_financials();
        let report = FundamentalsAnalyzer::generate("600519", &q, &f);
        assert_eq!(report.stock_code, "600519");
        assert!(report.health_score > 70);
        assert!(!report.key_takeaways.is_empty());
        let md = report.to_markdown();
        assert!(md.contains("基本面报告"));
        assert!(md.contains("茅台"));
        assert!(md.contains("ROE"));
    }

    #[test]
    fn markdown_includes_warning_for_high_debt() {
        let q = sample_quote(10.0);
        let mut f = sample_financials();
        f[0].debt_ratio = Some(85.0);
        let report = FundamentalsAnalyzer::generate("600519", &q, &f);
        let md = report.to_markdown();
        assert!(md.contains("资产负债率"));
        assert!(report
            .key_takeaways
            .iter()
            .any(|t| t.contains("资产负债率偏高")));
    }
}
