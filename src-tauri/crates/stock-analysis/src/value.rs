use crate::decision::ValueConfig;
use axagent_astock_data::FinancialReport;

/// 内在价值估算结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntrinsicValue {
    /// DCF 估值
    pub dcf_value: Option<f64>,
    /// 格雷厄姆估值 (√(22.5 × EPS × BVPS))
    pub graham_value: Option<f64>,
    /// 平均内在价值
    pub avg_intrinsic_value: Option<f64>,
    /// 安全边际 (%)
    pub margin_of_safety: Option<f64>,
    /// 安全边际判断
    pub mos_judgment: String,
}

/// Piotroski F-Score (0-9)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FScore {
    pub profitability: u32, // 0-4 (ROA, CFO, ΔROA, Accrual)
    pub leverage: u32,      // 0-3 (ΔLeverage, ΔCurrent, Shares)
    pub efficiency: u32,    // 0-2 (ΔMargin, ΔTurnover)
    pub total: u32,         // 0-9
    pub grade: String,      // "优秀(7-9)" | "良好(5-6)" | "一般(3-4)" | "差(0-2)"
    pub details: Vec<String>,
}

/// 护城河量化评估
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoatAssessment {
    /// 连续 ROE > 15% 的年数
    pub roe_consistency_years: Option<usize>,
    /// 平均毛利率 (%)
    pub avg_gross_margin: Option<f64>,
    /// 毛利率稳定性 (标准差)
    pub gross_margin_stability: Option<f64>,
    /// 自由现金流/净利润 比率
    pub fcf_to_earnings: Option<f64>,
    /// 护城河评分 (0-100)
    pub moat_score: u32,
    /// 护城河类型猜测
    pub moat_type: String,
    /// 评估详情
    pub details: Vec<String>,
}

/// 价值投资综合评估
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueAssessment {
    pub intrinsic_value: IntrinsicValue,
    pub f_score: FScore,
    pub moat: MoatAssessment,
    pub value_judgment: String,
    pub buffett_verdict: String,
}

impl ValueAssessment {
    pub fn from_metrics(
        metrics: &crate::value_investing::ValueMetrics,
        _current_price: f64,
    ) -> Self {
        let dcf_value = if metrics.dcf_mid > 0.0 {
            Some(metrics.dcf_mid)
        } else {
            None
        };
        let mos = if dcf_value.is_some() {
            Some(metrics.margin_of_safety_pct)
        } else {
            None
        };
        let mos_judgment = match metrics.mos_level.as_str() {
            "充足" => format!("充足的安全边际 {:.0}%", metrics.margin_of_safety_pct),
            "适中" => format!("有一定安全边际 {:.0}%", metrics.margin_of_safety_pct),
            "不足" => format!("安全边际不足 {:.0}%", metrics.margin_of_safety_pct),
            _ => format!("无安全边际 {:.0}%", metrics.margin_of_safety_pct),
        };

        let f_score = FScore {
            profitability: (metrics.f_score).min(4),
            leverage: (metrics.f_score).saturating_sub(4).min(3),
            efficiency: (metrics.f_score).saturating_sub(7).min(2),
            total: metrics.f_score,
            grade: metrics.f_score_level.clone(),
            details: vec![format!("F-Score={}/9", metrics.f_score)],
        };

        let moat = MoatAssessment {
            roe_consistency_years: None,
            avg_gross_margin: None,
            gross_margin_stability: None,
            fcf_to_earnings: if metrics.owner_earnings_yield > 0.0 {
                Some(metrics.owner_earnings_yield / 100.0)
            } else {
                None
            },
            moat_score: metrics.moat_score,
            moat_type: metrics.moat_level.clone(),
            details: vec![format!(
                "护城河{}/100({})",
                metrics.moat_score, metrics.moat_level
            )],
        };

        let buffett_verdict = if metrics.moat_score >= 70
            && metrics.f_score >= 7
            && metrics.margin_of_safety_pct >= 20.0
        {
            "🎯 巴菲特可能会喜欢：宽护城河+财务健康+充足安全边际。以合理价格买入优秀公司。"
                .to_string()
        } else if metrics.moat_score >= 50
            && metrics.f_score >= 5
            && metrics.margin_of_safety_pct >= 10.0
        {
            "👍 有一定吸引力：护城河和财务状况尚可，安全边际处于临界点。可小仓位观察。".to_string()
        } else if metrics.moat_score >= 30 {
            "🤔 需要更多安全边际：公司质地一般，等待更好的价格。巴菲特会说：'等待那个又胖又慢的球'。"
                .to_string()
        } else {
            "❌ 不符合巴菲特标准：护城河不足或财务质量差。'以合理价格买入优秀公司比以便宜价格买入平庸公司好得多'。"
                .to_string()
        };

        let value_judgment = format!(
            "护城河评分{}/100 | F-Score {}/9 | {}",
            metrics.moat_score, metrics.f_score, mos_judgment
        );

        ValueAssessment {
            intrinsic_value: IntrinsicValue {
                dcf_value,
                graham_value: None,
                avg_intrinsic_value: dcf_value,
                margin_of_safety: mos,
                mos_judgment,
            },
            f_score,
            moat,
            value_judgment,
            buffett_verdict,
        }
    }
}

/// 价值投资引擎
pub struct ValueEngine;

impl ValueEngine {
    /// ── DCF 估值 (简化两阶段模型) ──
    pub fn dcf_valuation(
        free_cash_flow: f64,     // 最近年度自由现金流
        growth_rate: f64,        // 未来5年增长率 (如 0.10 = 10%)
        terminal_rate: f64,      // 永续增长率 (如 0.03 = 3%)
        discount_rate: f64,      // 折现率 (如 0.10 = 10%)
        shares_outstanding: f64, // 总股本
        years: u32,              // 预测年数
    ) -> f64 {
        let mut total_pv = 0.0;
        let mut fcf = free_cash_flow;

        // 第一阶段：高速增长期
        for i in 0..years {
            fcf *= 1.0 + growth_rate;
            let pv = fcf / (1.0 + discount_rate).powi(i as i32 + 1);
            total_pv += pv;
        }

        // 第二阶段：永续增长期
        let terminal_value =
            fcf * (1.0 + terminal_rate) / (discount_rate - terminal_rate.max(0.001));
        let terminal_pv = terminal_value / (1.0 + discount_rate).powi(years as i32 + 1);
        total_pv += terminal_pv;

        total_pv / shares_outstanding.max(1.0)
    }

    /// ── 格雷厄姆公式 ──
    /// 内在价值 = √(22.5 × EPS × BVPS)
    pub fn graham_formula(eps: f64, bvps: f64) -> f64 {
        let product = 22.5_f64 * eps * bvps;
        if product > 0.0 {
            product.sqrt()
        } else {
            0.0
        }
    }

    /// ── 安全边际 ──
    /// MOS = (内在价值 - 当前价格) / 内在价值 × 100%
    pub fn margin_of_safety(intrinsic_value: f64, current_price: f64) -> f64 {
        if intrinsic_value <= 0.0 {
            return 0.0;
        }
        ((intrinsic_value - current_price) / intrinsic_value) * 100.0
    }

    /// ── Piotroski F-Score (9分制) ──
    /// 基于最近两份年报计算
    pub fn f_score(current: &FinancialReport, previous: &FinancialReport) -> FScore {
        let mut details = Vec::new();
        let mut profitability = 0u32;
        let mut leverage = 0u32;
        let mut efficiency = 0u32;

        // 盈利能力 (4分)
        if current.net_profit.unwrap_or(0.0) > 0.0 {
            profitability += 1;
            details.push("净利润>0 ✓".into());
        } else {
            details.push("净利润≤0 ✗".into());
        }

        if current.net_profit.unwrap_or(0.0) > previous.net_profit.unwrap_or(0.0) {
            profitability += 1;
            details.push("净利润增长 ✓".into());
        } else {
            details.push("净利润未增长 ✗".into());
        }

        if current.roe.unwrap_or(0.0) > previous.roe.unwrap_or(0.0) {
            profitability += 1;
            details.push("ΔROE>0 ✓".into());
        } else {
            details.push("ΔROA≤0 ✗".into());
        }

        if current.revenue.unwrap_or(0.0) > 0.0
            && current.net_margin.unwrap_or(0.0) > 0.0
            && current.net_margin.unwrap_or(0.0) < current.roe.unwrap_or(0.0) * 2.0
        {
            profitability += 1;
            details.push("盈利质量好 ✓".into());
        } else {
            details.push("盈利质量差 ✗".into());
        }

        // 杠杆/流动性 (3分)
        let debt_current = current.debt_ratio.unwrap_or(100.0);
        let debt_prev = previous.debt_ratio.unwrap_or(100.0);
        if debt_current < debt_prev {
            leverage += 1;
            details.push("Δ负债率↓ ✓".into());
        } else {
            details.push("Δ负债率↑ ✗".into());
        }

        if current.current_ratio.map(|r| r > 1.5).unwrap_or(false) {
            leverage += 1;
            details.push("流动比率>1.5 ✓".into());
        } else if current.current_ratio.map(|r| r > 1.0).unwrap_or(false) {
            details.push("流动比率一般 ✗".into());
        } else {
            details.push("流动比率<1.0 ✗".into());
        }

        // 无增发：EPS 不稀释（用 EPS 不低于上年判断）
        if current.eps.unwrap_or(0.0) >= previous.eps.unwrap_or(0.0) {
            leverage += 1;
            details.push("无增发稀释 ✓".into());
        } else {
            details.push("EPS稀释 ✗".into());
        }

        // 运营效率 (2分)
        let margin_curr = current.gross_margin.unwrap_or(0.0);
        let margin_prev = previous.gross_margin.unwrap_or(0.0);
        if margin_curr > margin_prev {
            efficiency += 1;
            details.push("Δ毛利率↑ ✓".into());
        } else {
            details.push("Δ毛利率↓ ✗".into());
        }

        let rev_growth = if previous.revenue.unwrap_or(0.0) > 0.0 {
            (current.revenue.unwrap_or(0.0) - previous.revenue.unwrap_or(0.0))
                / previous.revenue.unwrap_or(0.0)
        } else {
            0.0
        };
        let profit_growth = if previous.net_profit.unwrap_or(0.0) > 0.0 {
            (current.net_profit.unwrap_or(0.0) - previous.net_profit.unwrap_or(0.0))
                / previous.net_profit.unwrap_or(0.0)
        } else if current.net_profit.unwrap_or(0.0) > 0.0 {
            1.0
        } else {
            0.0
        };
        if rev_growth > 0.0 && rev_growth >= profit_growth {
            efficiency += 1;
            details.push("Δ周转率↑ ✓".into());
        } else {
            details.push("Δ周转率↓ ✗".into());
        }

        let total = profitability + leverage + efficiency;
        let grade = match total {
            7..=9 => "优秀".to_string(),
            5..=6 => "良好".to_string(),
            3..=4 => "一般".to_string(),
            _ => "差".to_string(),
        };

        FScore {
            profitability,
            leverage,
            efficiency,
            total,
            grade,
            details,
        }
    }

    /// ── 护城河量化评估 ──
    pub fn moat_assessment(financials: &[FinancialReport]) -> MoatAssessment {
        let mut details = Vec::new();

        // ROE 一致性
        let roe_years_above_15 = financials
            .iter()
            .filter(|f| f.roe.unwrap_or(0.0) >= 15.0)
            .count();
        if roe_years_above_15 >= 3 {
            details.push(format!("ROE连续{}年>15%", roe_years_above_15));
        } else {
            details.push(format!("ROE>15%仅{}年", roe_years_above_15));
        }

        // 毛利率
        let margins: Vec<f64> = financials.iter().filter_map(|f| f.gross_margin).collect();
        let avg_margin = if !margins.is_empty() {
            margins.iter().sum::<f64>() / margins.len() as f64
        } else {
            0.0
        };

        let margin_std = if margins.len() > 1 {
            let mean = avg_margin;
            (margins.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / margins.len() as f64).sqrt()
        } else {
            0.0
        };

        if avg_margin > 40.0 {
            details.push(format!("高毛利率 {:.1}%", avg_margin));
        } else if avg_margin > 20.0 {
            details.push(format!("中等毛利率 {:.1}%", avg_margin));
        } else {
            details.push(format!("低毛利率 {:.1}%", avg_margin));
        }

        if margin_std < 5.0 && !margins.is_empty() {
            details.push("毛利率稳定(σ<5%)".to_string());
        }

        // FCF/净利润
        let net_values: Vec<f64> = financials
            .iter()
            .filter_map(|f| f.net_profit)
            .take(3)
            .collect();
        let avg_net = if net_values.is_empty() {
            0.0
        } else {
            net_values.iter().sum::<f64>() / net_values.len() as f64
        };
        let latest_debt_ratio = financials
            .first()
            .and_then(|f| f.debt_ratio)
            .unwrap_or(50.0);
        let est_fcf = financials
            .first()
            .and_then(|f| f.free_cash_flow)
            .or_else(|| {
                let capex_ratio = if latest_debt_ratio > 60.0 {
                    0.85
                } else if latest_debt_ratio > 40.0 {
                    0.90
                } else {
                    0.95
                };
                financials
                    .first()
                    .and_then(|f| f.net_profit)
                    .map(|np| np * capex_ratio)
            })
            .unwrap_or(0.0);
        let fcf_ratio = if avg_net > 0.0 {
            est_fcf / avg_net
        } else {
            0.0
        };

        if fcf_ratio > 0.8 {
            details.push("强自由现金流".to_string());
        } else if fcf_ratio > 0.5 {
            details.push("合理自由现金流".to_string());
        } else {
            details.push("自由现金流转弱".to_string());
        }

        // 护城河评分
        let mut score = 0u32;
        if roe_years_above_15 >= 5 {
            score += 30;
        } else if roe_years_above_15 >= 3 {
            score += 20;
        }
        if avg_margin > 40.0 {
            score += 25;
        } else if avg_margin > 20.0 {
            score += 15;
        }
        if margin_std < 5.0 && !margins.is_empty() {
            score += 10;
        }
        if fcf_ratio > 0.8 {
            score += 20;
        } else if fcf_ratio > 0.5 {
            score += 10;
        }

        let moat_type = if score >= 60 {
            "宽护城河".to_string()
        } else if score >= 35 {
            "窄护城河".to_string()
        } else {
            "无护城河".to_string()
        };

        MoatAssessment {
            roe_consistency_years: if roe_years_above_15 > 0 {
                Some(roe_years_above_15)
            } else {
                None
            },
            avg_gross_margin: if margins.is_empty() {
                None
            } else {
                Some(avg_margin)
            },
            gross_margin_stability: if margins.len() > 1 {
                Some(margin_std)
            } else {
                None
            },
            fcf_to_earnings: if avg_net > 0.0 { Some(fcf_ratio) } else { None },
            moat_score: score,
            moat_type,
            details,
        }
    }

    /// ── 综合价值评估（无股本数据时使用）──
    pub fn assess_no_shares(
        current_price: f64,
        financials: &[FinancialReport],
        _value_config: Option<&ValueConfig>,
    ) -> ValueAssessment {
        let latest = match financials.first() {
            Some(f) => f,
            None => {
                return ValueAssessment {
                    intrinsic_value: IntrinsicValue {
                        dcf_value: None,
                        graham_value: None,
                        avg_intrinsic_value: None,
                        margin_of_safety: None,
                        mos_judgment: "无财务数据，无法估值".to_string(),
                    },
                    f_score: FScore {
                        profitability: 0,
                        leverage: 0,
                        efficiency: 0,
                        total: 0,
                        grade: "无数据".to_string(),
                        details: vec![],
                    },
                    moat: MoatAssessment {
                        roe_consistency_years: None,
                        avg_gross_margin: None,
                        gross_margin_stability: None,
                        fcf_to_earnings: None,
                        moat_score: 0,
                        moat_type: "无数据".to_string(),
                        details: vec![],
                    },
                    value_judgment: "无足够财务数据".to_string(),
                    buffett_verdict: "数据不足，无法判断。巴菲特原则：不懂不做。".to_string(),
                };
            },
        };

        let eps = latest.eps.unwrap_or(0.0);
        let bvps = latest.bps.unwrap_or(0.0);

        let graham = if eps > 0.0 && bvps > 0.0 {
            Some(Self::graham_formula(eps, bvps))
        } else {
            None
        };

        let avg_iv = graham;
        let mos = avg_iv.map(|iv| Self::margin_of_safety(iv, current_price));
        let mos_judgment = match mos {
            Some(m) if m >= 30.0 => {
                format!("充足的安全边际 {:.0}%（内在价值远高于现价）", m)
            },
            Some(m) if m >= 15.0 => {
                format!("有一定安全边际 {:.0}%", m)
            },
            Some(m) if m >= 0.0 => {
                format!("安全边际不足 {:.0}%（现价接近内在价值）", m)
            },
            Some(m) => {
                format!("无安全边际 {:.0}%（现价高于内在价值）", m)
            },
            None => "无法计算安全边际（缺少股本数据，DCF不可用）".to_string(),
        };

        let previous = financials.get(1);
        let f_score = if let Some(prev) = previous {
            Self::f_score(latest, prev)
        } else {
            FScore {
                profitability: 0,
                leverage: 0,
                efficiency: 0,
                total: 0,
                grade: "无对比数据".to_string(),
                details: vec![],
            }
        };

        let moat = Self::moat_assessment(financials);

        let buffett_verdict = if moat.moat_score >= 70
            && f_score.total >= 7
            && mos.unwrap_or(0.0) >= 20.0
        {
            "🎯 巴菲特可能会喜欢：宽护城河+财务健康+充足安全边际。以合理价格买入优秀公司。"
                .to_string()
        } else if moat.moat_score >= 50 && f_score.total >= 5 && mos.unwrap_or(0.0) >= 10.0 {
            "👍 有一定吸引力：护城河和财务状况尚可，安全边际处于临界点。可小仓位观察。".to_string()
        } else if moat.moat_score >= 30 {
            "🤔 需要更多安全边际：公司质地一般，等待更好的价格。巴菲特会说：'等待那个又胖又慢的球'。"
                .to_string()
        } else {
            "❌ 不符合巴菲特标准：护城河不足或财务质量差。'以合理价格买入优秀公司比以便宜价格买入平庸公司好得多'。"
                .to_string()
        };

        let value_judgment = format!(
            "护城河评分{}/100 | F-Score {}/9 | {}（无股本数据，DCF不可用）",
            moat.moat_score, f_score.total, mos_judgment
        );

        ValueAssessment {
            intrinsic_value: IntrinsicValue {
                dcf_value: None,
                graham_value: graham,
                avg_intrinsic_value: avg_iv,
                margin_of_safety: mos,
                mos_judgment,
            },
            f_score,
            moat,
            value_judgment,
            buffett_verdict,
        }
    }

    /// ── 综合价值评估 ──
    pub fn assess(
        current_price: f64,
        financials: &[FinancialReport],
        shares_outstanding: f64,
        value_config: Option<&ValueConfig>,
    ) -> ValueAssessment {
        let latest = match financials.first() {
            Some(f) => f,
            None => {
                return ValueAssessment {
                    intrinsic_value: IntrinsicValue {
                        dcf_value: None,
                        graham_value: None,
                        avg_intrinsic_value: None,
                        margin_of_safety: None,
                        mos_judgment: "无财务数据，无法估值".to_string(),
                    },
                    f_score: FScore {
                        profitability: 0,
                        leverage: 0,
                        efficiency: 0,
                        total: 0,
                        grade: "无数据".to_string(),
                        details: vec![],
                    },
                    moat: MoatAssessment {
                        roe_consistency_years: None,
                        avg_gross_margin: None,
                        gross_margin_stability: None,
                        fcf_to_earnings: None,
                        moat_score: 0,
                        moat_type: "无数据".to_string(),
                        details: vec![],
                    },
                    value_judgment: "无足够财务数据".to_string(),
                    buffett_verdict: "数据不足，无法判断。巴菲特原则：不懂不做。".to_string(),
                };
            },
        };

        let eps = latest.eps.unwrap_or(0.0);
        let bvps = latest.bps.unwrap_or(0.0);
        let latest_debt_ratio = latest.debt_ratio.unwrap_or(50.0);
        let fcf = latest
            .free_cash_flow
            .or_else(|| {
                let capex_ratio = if latest_debt_ratio > 60.0 {
                    0.85
                } else if latest_debt_ratio > 40.0 {
                    0.90
                } else {
                    0.95
                };
                latest
                    .operating_cash_flow
                    .and_then(|ocf| latest.capital_expenditure.map(|capex| ocf - capex))
                    .or_else(|| latest.net_profit.map(|np| np * capex_ratio))
            })
            .unwrap_or(0.0);

        // DCF
        let dcf = if fcf > 0.0 && shares_outstanding > 0.0 {
            let growth_rate = value_config
                .map(|c| c.dcf_growth_rate / 100.0)
                .unwrap_or(0.08)
                .max(0.0);
            let terminal_rate = value_config
                .map(|c| c.dcf_perpetual_rate / 100.0)
                .unwrap_or(0.03);
            let discount_rate = value_config
                .map(|c| c.dcf_discount_rate / 100.0)
                .unwrap_or(0.10);
            Some(Self::dcf_valuation(
                fcf,
                growth_rate,
                terminal_rate,
                discount_rate,
                shares_outstanding.max(1.0),
                5,
            ))
        } else {
            None
        };

        // Graham
        let graham = if eps > 0.0 && bvps > 0.0 {
            Some(Self::graham_formula(eps, bvps))
        } else {
            None
        };

        // 平均内在价值
        let avg_iv = match (dcf, graham) {
            (Some(d), Some(g)) => Some((d + g) / 2.0),
            (Some(d), None) => Some(d),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };

        // 安全边际
        let mos = avg_iv.map(|iv| Self::margin_of_safety(iv, current_price));
        let mos_judgment = match mos {
            Some(m) if m >= 30.0 => {
                format!("充足的安全边际 {:.0}%（内在价值远高于现价）", m)
            },
            Some(m) if m >= 15.0 => {
                format!("有一定安全边际 {:.0}%", m)
            },
            Some(m) if m >= 0.0 => {
                format!("安全边际不足 {:.0}%（现价接近内在价值）", m)
            },
            Some(m) => {
                format!("无安全边际 {:.0}%（现价高于内在价值）", m)
            },
            None => "无法计算安全边际（缺少估值数据）".to_string(),
        };

        // F-Score
        let previous = financials.get(1);
        let f_score = if let Some(prev) = previous {
            Self::f_score(latest, prev)
        } else {
            FScore {
                profitability: 0,
                leverage: 0,
                efficiency: 0,
                total: 0,
                grade: "无对比数据".to_string(),
                details: vec![],
            }
        };

        // Moat
        let moat = Self::moat_assessment(financials);

        // 巴菲特式裁决
        let buffett_verdict = if moat.moat_score >= 70
            && f_score.total >= 7
            && mos.unwrap_or(0.0) >= 20.0
        {
            "🎯 巴菲特可能会喜欢：宽护城河+财务健康+充足安全边际。以合理价格买入优秀公司。"
                .to_string()
        } else if moat.moat_score >= 50 && f_score.total >= 5 && mos.unwrap_or(0.0) >= 10.0 {
            "👍 有一定吸引力：护城河和财务状况尚可，安全边际处于临界点。可小仓位观察。".to_string()
        } else if moat.moat_score >= 30 {
            "🤔 需要更多安全边际：公司质地一般，等待更好的价格。巴菲特会说：'等待那个又胖又慢的球'。"
                .to_string()
        } else {
            "❌ 不符合巴菲特标准：护城河不足或财务质量差。'以合理价格买入优秀公司比以便宜价格买入平庸公司好得多'。"
                .to_string()
        };

        // 综合判断 (在移动 f_score/moat/mos_judgment 前计算)
        let value_judgment = format!(
            "护城河评分{}/100 | F-Score {}/9 | {}",
            moat.moat_score, f_score.total, mos_judgment
        );

        ValueAssessment {
            intrinsic_value: IntrinsicValue {
                dcf_value: dcf,
                graham_value: graham,
                avg_intrinsic_value: avg_iv,
                margin_of_safety: mos,
                mos_judgment,
            },
            f_score,
            moat,
            value_judgment,
            buffett_verdict,
        }
    }
}
