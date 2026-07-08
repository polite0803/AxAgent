use axagent_astock_data::FinancialReport;

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
    pub mos_level: String, // "充足" | "适中" | "不足" | "无"
    /// Piotroski F-Score (0-9)
    pub f_score: u32,
    pub f_score_level: String, // "优秀(7-9)" | "良好(5-6)" | "一般(3-4)" | "弱(0-2)"
    /// 护城河量化评分 (0-100)
    pub moat_score: u32,
    pub moat_level: String, // "宽阔" | "狭窄" | "无"
    /// 所有者收益（Buffett's Owner Earnings）
    pub owner_earnings: f64,
    pub owner_earnings_yield: f64, // 所有者收益率 = OE / 市值
    /// 综合判断
    pub value_signal: String, // "低估" | "合理偏低" | "合理" | "偏高" | "高估"
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
        let mos_level = if mos > 30.0 {
            "充足"
        } else if mos > 15.0 {
            "适中"
        } else if mos > 0.0 {
            "不足"
        } else {
            "无"
        };

        // 3. F-Score
        let f_score = Self::piotroski_f_score(financials);
        let f_score_level = match f_score {
            7..=9 => "优秀(7-9)",
            5..=6 => "良好(5-6)",
            3..=4 => "一般(3-4)",
            _ => "弱(0-2)",
        };

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

        let value_signal = if value_score >= 60 {
            "低估"
        } else if value_score >= 45 {
            "合理偏低"
        } else if value_score >= 30 {
            "合理"
        } else if value_score >= 15 {
            "偏高"
        } else {
            "高估"
        };

        let summary = format!(
            "内在价值≈{:.2} | 安全边际{:.0}%({}) | F-Score={}/9({}) | 护城河{}/100({}) | OE收益率{:.1}% | {}",
            dcf_mid, mos, mos_level, f_score, f_score_level, moat_score, moat_level, oe_yield, value_signal
        );

        ValueMetrics {
            dcf_low,
            dcf_mid,
            dcf_high,
            margin_of_safety_pct: mos,
            mos_level: mos_level.to_string(),
            f_score,
            f_score_level: f_score_level.to_string(),
            moat_score,
            moat_level: moat_level.to_string(),
            owner_earnings,
            owner_earnings_yield: oe_yield,
            value_signal: value_signal.to_string(),
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

        let base_g = growth_rate.unwrap_or(0.08).max(0.0);
        let base_p = perpetual_rate.unwrap_or(0.03);
        let base_d = discount_rate.unwrap_or(0.10);
        let low = Self::dcf_two_stage(
            fcf_per_share,
            (base_g * 0.6).max(0.01),
            (base_p * 0.7).max(0.01),
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

    /// Piotroski F-Score (0-9分)
    /// 盈利(0-4) + 财务健康(0-3) + 运营效率(0-2)
    fn piotroski_f_score(financials: &[FinancialReport]) -> u32 {
        if financials.len() < 2 {
            return 0;
        }
        let curr = &financials[0];
        let prev = &financials[1];
        let mut score = 0u32;

        // 盈利能力 (4分)
        if curr.net_profit.unwrap_or(0.0) > 0.0 {
            score += 1;
        } // ROA > 0
        if curr.net_profit.unwrap_or(0.0) > prev.net_profit.unwrap_or(0.0) {
            score += 1;
        } // 净利润增长
        if curr.roe.unwrap_or(0.0) > prev.roe.unwrap_or(0.0) {
            score += 1;
        } // ROE增长
        let net_margin = curr.net_margin.unwrap_or(0.0);
        let curr_roe = curr.roe.unwrap_or(0.0);
        if curr.revenue.unwrap_or(0.0) > 0.0 && net_margin > 0.0 && net_margin < curr_roe * 2.0 {
            score += 1;
        } // 盈利质量(应计项测试)

        // 财务健康 (3分)
        if curr.debt_ratio.unwrap_or(100.0) < prev.debt_ratio.unwrap_or(100.0) {
            score += 1;
        } // 负债率下降
        if curr.current_ratio.map(|r| r > 1.5).unwrap_or(false) {
            score += 1;
        }
        // 无增发(用EPS不稀释代理)
        if curr.eps.unwrap_or(0.0) >= prev.eps.unwrap_or(0.0) {
            score += 1;
        } // EPS不稀释

        // 运营效率 (2分)
        if curr.gross_margin.unwrap_or(0.0) > prev.gross_margin.unwrap_or(0.0) {
            score += 1;
        } // 毛利率改善
        let rev_growth = if prev.revenue.unwrap_or(0.0) > 0.0 {
            (curr.revenue.unwrap_or(0.0) - prev.revenue.unwrap_or(0.0))
                / prev.revenue.unwrap_or(0.0)
        } else {
            0.0
        };
        let profit_growth = if prev.net_profit.unwrap_or(0.0) > 0.0 {
            (curr.net_profit.unwrap_or(0.0) - prev.net_profit.unwrap_or(0.0))
                / prev.net_profit.unwrap_or(0.0)
        } else if curr.net_profit.unwrap_or(0.0) > 0.0 {
            1.0
        } else {
            0.0
        };
        if rev_growth > 0.0 && rev_growth >= profit_growth {
            score += 1;
        }

        score
    }

    /// 护城河量化清单 (0-100分)
    fn moat_checklist(
        financials: &[FinancialReport],
        pe: Option<f64>,
        _pb: Option<f64>,
    ) -> (u32, String) {
        if financials.is_empty() {
            return (0, "无".to_string());
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
        let gm_values: Vec<f64> = financials
            .iter()
            .take(5)
            .filter_map(|r| r.gross_margin)
            .collect();
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
        let all_profitable = financials
            .iter()
            .take(5)
            .all(|r| r.net_profit.unwrap_or(-1.0) > 0.0);
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

        let level = if score >= 70 {
            "宽阔"
        } else if score >= 40 {
            "狭窄"
        } else {
            "无"
        };
        (score, level.to_string())
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
        if ref_value < 100.0 {
            1_0000_0000.0
        } else if ref_value < 10_000.0 {
            1_0000.0
        } else if ref_value < 1_0000_0000.0 {
            1.0
        } else {
            0.001
        }
    }
}
