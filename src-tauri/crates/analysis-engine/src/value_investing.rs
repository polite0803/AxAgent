use axagent_astock_data::FinancialReport;

use crate::types::{FScoreLevel, MoatLevel, MosLevel, ValueSignal};
use crate::value::ValueEngine;

/// 价值投资综合指标
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueMetrics {
    /// DCF 估值
    pub dcf_low: f64, // 保守估值
    pub dcf_mid: f64,  // 中性估值
    pub dcf_high: f64, // 乐观估值
    /// 安全边际
    pub margin_of_safety_pct: f64, // (内在价值-现价)/内在价值 × 100
    pub mos_level: MosLevel,
    /// Piotroski F-Score (0-9)
    pub f_score: u32,
    pub f_score_level: FScoreLevel,
    /// 护城河量化评分 (0-100)
    pub moat_score: u32,
    pub moat_level: MoatLevel,
    /// 所有者收益（Buffett's Owner Earnings）
    pub owner_earnings: f64,
    pub owner_earnings_yield: f64, // 所有者收益率 = OE / 市值
    /// 综合判断
    pub value_signal: ValueSignal,
    pub summary: String,
}

/// 价值投资计算引擎
pub struct ValueInvestingEngine;

impl ValueInvestingEngine {
    /// 主入口：计算所有价值投资指标
    pub fn compute(
        _stock_code: &str,
        current_price: f64,
        total_shares: Option<f64>,
        financials: &[FinancialReport],
        pe: Option<f64>,
        pb: Option<f64>,
        value_config: Option<&crate::decision::ValueConfig>,
    ) -> ValueMetrics {
        let latest = financials.first();
        let fcf_raw = latest
            .and_then(|f| {
                f.free_cash_flow.or_else(|| {
                    f.operating_cash_flow
                        .and_then(|ocf| f.capital_expenditure.map(|capex| ocf - capex))
                })
            })
            .unwrap_or_else(|| latest.and_then(|f| f.net_profit).unwrap_or(0.0) * 0.90);
        let fcf_scale = Self::detect_financial_unit(financials);
        let fcf = fcf_raw * fcf_scale;

        let g = value_config.map(|c| c.dcf_growth_rate / 100.0);
        let p = value_config.map(|c| c.dcf_perpetual_rate / 100.0);
        let d = value_config.map(|c| c.dcf_discount_rate / 100.0);

        let (dcf_low, dcf_mid, dcf_high) = Self::dcf_valuation(fcf, total_shares, g, p, d);

        // 2. 安全边际
        let intrinsic = dcf_mid;
        let mos = if intrinsic > 0.0 && current_price > 0.0 {
            ((intrinsic - current_price) / intrinsic) * 100.0
        } else {
            0.0
        };
        let mos_level = MosLevel::from_mos_pct(mos);

        // 3. F-Score（委托统一的 ValueEngine::f_score，保证与详情版口径一致）
        let f_score = if financials.len() >= 2 {
            ValueEngine::f_score(&financials[0], &financials[1]).total
        } else {
            0
        };
        let f_score_level = FScoreLevel::from_score(f_score);

        // 4. 护城河评分
        let (moat_score, moat_level) = Self::moat_checklist(financials, pe, pb);

        // 5. 所有者收益
        let owner_earnings = Self::buffett_owner_earnings(financials);
        let market_cap = if let Some(ts) = total_shares {
            current_price * ts * 1_0000_0000.0
        } else {
            0.0
        };
        let oe_yield = if market_cap > 0.0 {
            (owner_earnings / market_cap) * 100.0
        } else {
            0.0
        };

        // 6. 综合判断
        let value_score = if mos > 20.0 {
            30
        } else if mos > 10.0 {
            20
        } else if mos > 0.0 {
            10
        } else {
            0
        } + f_score.min(9) * 5
            + moat_score.min(100) / 5
            + if oe_yield > 5.0 {
                20
            } else if oe_yield > 3.0 {
                10
            } else {
                0
            };

        let value_signal = ValueSignal::from_score(value_score as i32);

        let summary = format!(
            "内在价值≈{:.2} | 安全边际{:.0}%({}) | F-Score={}/9({}) | 护城河{}/100({}) | OE收益率{:.1}% | {}",
            dcf_mid,
            mos,
            mos_level.label(),
            f_score,
            f_score_level.label(),
            moat_score,
            moat_level.label(),
            oe_yield,
            value_signal.label()
        );

        ValueMetrics {
            dcf_low,
            dcf_mid,
            dcf_high,
            margin_of_safety_pct: mos,
            mos_level,
            f_score,
            f_score_level,
            moat_score,
            moat_level,
            owner_earnings,
            owner_earnings_yield: oe_yield,
            value_signal,
            summary,
        }
    }

    /// 简化 DCF 估值（二阶段模型）
    /// Phase 1: 5年增长期, Phase 2: 永续增长
    fn dcf_valuation(
        fcf: f64,
        total_shares: Option<f64>,
        growth_rate: Option<f64>,
        perpetual_rate: Option<f64>,
        discount_rate: Option<f64>,
    ) -> (f64, f64, f64) {
        if fcf <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let shares = match total_shares {
            Some(s) if s > 0.0 => s,
            _ => return (0.0, 0.0, 0.0),
        };
        let fcf_per_share = fcf / shares / 1_0000_0000.0;

        let base_g = growth_rate.unwrap_or(0.12).max(0.0);
        let base_p = perpetual_rate.unwrap_or(0.04);
        let base_d = discount_rate.unwrap_or(0.085);
        // low 端从 growth×0.6 调整为 growth×0.7，避免过度悲观
        // （原 ×0.6 会让 low 估值只有 mid 的 60% 左右，加剧系统性低估）
        let low = Self::dcf_two_stage(
            fcf_per_share,
            (base_g * 0.7).max(0.01),
            (base_p * 0.8).max(0.01),
            base_d,
        );
        let mid = Self::dcf_two_stage(fcf_per_share, base_g.max(0.01), base_p, base_d);
        let high = Self::dcf_two_stage(
            fcf_per_share,
            (base_g * 1.5).clamp(0.02, 0.30),
            (base_p * 1.3).min(0.05),
            base_d,
        );

        (low, mid, high)
    }

    fn dcf_two_stage(fcf: f64, growth: f64, perpetual: f64, discount: f64) -> f64 {
        let mut pv = 0.0;
        let mut current_fcf = fcf;
        for year in 1..=5 {
            current_fcf *= 1.0 + growth;
            pv += current_fcf / (1.0 + discount).powi(year);
        }
        // Terminal value
        let terminal_fcf = current_fcf * (1.0 + perpetual);
        let terminal_spread = (discount - perpetual).max(0.001);
        let terminal_value = terminal_fcf / terminal_spread;
        let terminal_pv = terminal_value / (1.0 + discount).powi(5);
        pv + terminal_pv
    }

    /// 护城河量化清单 (0-100分)
    fn moat_checklist(
        financials: &[FinancialReport],
        pe: Option<f64>,
        _pb: Option<f64>,
    ) -> (u32, MoatLevel) {
        if financials.is_empty() {
            return (0, MoatLevel::None);
        }
        let f = &financials[0];
        let mut score = 0u32;

        // 1. ROE 持续性 (30分)
        let roe_values: Vec<f64> = financials.iter().take(5).filter_map(|r| r.roe).collect();
        let roe_count = roe_values.len() as f64;
        let avg_roe = if roe_count > 0.0 {
            roe_values.iter().sum::<f64>() / roe_count
        } else {
            0.0
        };
        if avg_roe > 20.0 {
            score += 30;
        } else if avg_roe > 15.0 {
            score += 20;
        } else if avg_roe > 10.0 {
            score += 10;
        }

        // 2. 毛利率稳定性 (20分)
        let gm_values: Vec<f64> =
            financials.iter().take(5).filter_map(|r| r.gross_margin).collect();
        let gm_count = gm_values.len() as f64;
        let avg_gm = if gm_count > 0.0 {
            gm_values.iter().sum::<f64>() / gm_count
        } else {
            0.0
        };
        if avg_gm > 60.0 {
            score += 20;
        } else if avg_gm > 40.0 {
            score += 15;
        } else if avg_gm > 20.0 {
            score += 8;
        }

        // 3. 低负债 (20分)
        let debt = f.debt_ratio.unwrap_or(100.0);
        if debt < 20.0 {
            score += 20;
        } else if debt < 40.0 {
            score += 15;
        } else if debt < 60.0 {
            score += 8;
        }

        // 4. 盈利稳定性 (15分)
        let all_profitable = financials.iter().take(5).all(|r| r.net_profit.unwrap_or(-1.0) > 0.0);
        if all_profitable {
            score += 15;
        }

        // 5. 估值合理性 (15分)
        if let Some(pe_val) = pe {
            if pe_val < 15.0 && pe_val > 0.0 {
                score += 15;
            } else if pe_val < 25.0 {
                score += 10;
            } else if pe_val < 50.0 {
                score += 5;
            }
        }

        let level = MoatLevel::from_score(score);
        (score, level)
    }

    /// 巴菲特所有者收益 = 净利润 + 折旧摊销 - 资本支出
    /// 简化：用净利润 × 0.95 近似（假设折旧摊销略大于资本支出）
    fn buffett_owner_earnings(financials: &[FinancialReport]) -> f64 {
        if financials.is_empty() {
            return 0.0;
        }
        let f = &financials[0];
        let scale = Self::detect_financial_unit(financials);
        let net = f.net_profit.unwrap_or(0.0) * scale;
        let oe = if let (Some(ocf), Some(capex)) = (f.operating_cash_flow, f.capital_expenditure) {
            let ocf_scaled = ocf * scale;
            let capex_scaled = capex * scale;
            ocf_scaled - capex_scaled
        } else if let Some(fcf) = f.free_cash_flow {
            fcf * scale
        } else {
            let debt_ratio = f.debt_ratio.unwrap_or(50.0);
            let capex_ratio = if debt_ratio > 60.0 {
                0.85
            } else if debt_ratio > 40.0 {
                0.90
            } else {
                0.95
            };
            net * capex_ratio
        };
        oe.max(0.0)
    }

    fn detect_financial_unit(financials: &[FinancialReport]) -> f64 {
        if financials.is_empty() {
            return 1_0000_0000.0;
        }
        let f = &financials[0];
        let revenue = f.revenue.unwrap_or(0.0).abs();
        let net_profit = f.net_profit.unwrap_or(0.0).abs();
        let ref_value = if revenue > 0.0 { revenue } else { net_profit };
        if ref_value <= 0.0 {
            return 1_0000_0000.0;
        }
        // 修复(2026-07-29): 原最后分支返回 0.001 是致命 bug。
        //   当数据源返回元单位且营收>1亿元（A股大盘股常见，如贵州茅台营收上千亿=1e11），
        //   原 0.001 会把所有财务数据错误缩小 1000 倍，导致 DCF 估值、所有者收益、
        //   oe_yield 全部失真。≥1亿元的数据应视为元单位，返回 1.0。
        if ref_value < 100.0 {
            1_0000_0000.0 // 原始值<100，假设单位是亿元，×1e8 转元
        } else if ref_value < 10_000.0 {
            1_0000.0 // 原始值<1万，假设单位是万元，×1e4 转元
        } else {
            1.0 // 原始值≥1万，假设单位已经是元，不缩放
        }
    }
}
