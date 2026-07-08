//! Serenity 瓶颈分析子策略
//!
//! 不同于其他 5 个策略基于价格/成交量信号扫描，
//! SerenityStrategy 的 seed pool 来自 serenity-screening workflow 的输出，
//! scan_one 从全局缓存读取全量诊断数据（serenity_score / catalysts / exit_signals），
//! 做上下文感知的确定性财务验证。
//!
//! 工作流：workflow 生成候选池 → 持久化 + 全量缓存 → SerenityStrategy 读取 → scan_one 验证

use super::super::strategy::{read_f64, RecoContext, RecommendStrategy};
use crate::recommender::scoring::{calc_confidence, calc_position};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;

pub struct SerenityStrategy {
    pub period: Period,
}

impl SerenityStrategy {
    /// Serenity 只做中长期（Mid / Long），不做短/超短
    pub const fn mid() -> Self {
        Self {
            period: Period::Mid,
        }
    }
    pub const fn long() -> Self {
        Self {
            period: Period::Long,
        }
    }

    async fn scan_one(
        &self,
        client: &AStockClient,
        code: &str,
        name: &str,
        sector: Option<String>,
        vars: &HashMap<String, Value>,
    ) -> Option<RecoPick> {
        // 0. 读取 Serenity 全量诊断数据（由 workflow 填入缓存）
        let detail = super::super::get_serenity_candidate_detail(code);
        // V53: 如果没有 serenity 缓存数据（非 workflow 候选股），跳过整个 scan_one
        // 避免对全 seed pool 190+ 只无候选数据的股票发送 financials/quote/klines 请求
        let detail = match detail {
            Some(d) => d,
            None => {
                tracing::trace!("{code}: 无 serenity 候选数据，跳过");
                return None;
            },
        };
        let serenity_score = detail["serenity_score"].as_f64().unwrap_or(0.0);
        let catalysts = detail["catalysts"].as_array().cloned().unwrap_or_default();
        let exit_signals = detail["exit_signals"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let _attention_metrics = detail["attention_metrics"]
            .as_object()
            .cloned()
            .unwrap_or_default();

        // 1. 获取财务数据验证护城河
        let financials = client.get_financials(code).await.ok()?;
        let latest = financials.first()?;

        // 2. 毛利率校验（上下文感知：瓶颈股早期可能微利，门槛不宜过高）
        // 默认 30%（可调），评分 >= 80 时豁免毛利率检查
        let min_gross_margin = read_f64(vars, "serenity_min_gross_margin", 30.0);
        let gm = latest.gross_margin.unwrap_or(0.0);
        if gm < min_gross_margin && serenity_score < 80.0 {
            tracing::info!("{code}: 毛利率 {gm:.1}% < {min_gross_margin}%, 因毛利率过低排除 (serenity_score={serenity_score:.0})");
            return None;
        }

        // 3. 负债率校验（扩张期瓶颈企业可能高负债，默认 70%（可调））
        let max_debt_ratio = read_f64(vars, "serenity_max_debt_ratio", 70.0);
        let dr = latest.debt_ratio?;
        if dr > max_debt_ratio && serenity_score < 85.0 {
            tracing::info!("{code}: 负债率 {dr:.1}% > {max_debt_ratio}%, 因负债率过高排除");
            return None;
        }

        // 4. 营收增速校验（成熟瓶颈可能增速不高，默认 5%（可调），评分 >= 85 时豁免）
        let min_rev_growth = read_f64(vars, "serenity_min_revenue_growth", 5.0);
        let rev_growth = latest.revenue_yoy.unwrap_or(0.0);
        if rev_growth < min_rev_growth && serenity_score < 85.0 {
            tracing::info!("{code}: 营收增速 {rev_growth:.1}% < {min_rev_growth}%, 因增速过低排除 (serenity_score={serenity_score:.0})");
            return None;
        }

        // ── V6 新增: 估值过滤器（PE/PB/涨幅上限，可在前端 Serenity 设置Tab中调整）──

        // 5. 获取行情（提前获取用于估值过滤）
        let quote = client.get_quote(code).await.ok()?;
        let price = quote.price;

        // 5a. PE 上限过滤（高增长豁免：营收增速≥阈值时跳过PE检查）
        let max_pe = read_f64(vars, "serenity_max_pe", 100.0);
        if let Some(pe) = quote.pe {
            if pe > max_pe && serenity_score < 85.0 {
                let growth_exempt_pct = read_f64(vars, "serenity_growth_exempt_pct", 50.0);
                // 高增长标的PE常偏高，营收增速超过阈值时豁免PE检查
                if rev_growth < growth_exempt_pct {
                    tracing::info!(
                        "{code}: PE={pe:.1} > {max_pe} 且增长率={rev_growth:.1}%<{growth_exempt_pct}%, 因估值过高排除"
                    );
                    return None;
                }
                // PE高但增速也高，放行（用户可调 growth_exempt_pct 控制松紧）
            }
        }

        // 5b. PB 上限过滤
        let max_pb = read_f64(vars, "serenity_max_pb", 10.0);
        if let Some(pb) = quote.pb {
            if pb > max_pb && serenity_score < 85.0 {
                tracing::info!("{code}: PB={pb:.1} > {max_pb}, 因估值过高排除");
                return None;
            }
        }

        // 5c. 近3月涨幅过滤（基于K线数据）
        let max_3m_gain = read_f64(vars, "serenity_max_3m_gain_pct", 80.0);
        let max_12m_gain = read_f64(vars, "serenity_max_12m_gain_pct", 300.0);

        if let Ok(klines) = client.get_klines_with_adj(code, "daily", 252, None).await {
            let latest_close = klines.last().map(|k| k.close).unwrap_or(price);
            // 近3月（约63个交易日）
            let k3m_idx = klines.len().saturating_sub(63);
            let k3m_close = klines.get(k3m_idx).map(|k| k.close);
            if let (Some(close_3m_back), true) =
                (k3m_close, latest_close > 0.0 && serenity_score < 85.0)
            {
                let gain_3m = (latest_close - close_3m_back) / close_3m_back * 100.0;
                if gain_3m > max_3m_gain {
                    tracing::info!(
                        "{code}: 近3月涨幅 {gain_3m:.0}% > {max_3m_gain}%, 因短期涨幅过大排除"
                    );
                    return None;
                }
            }
            // 近12月（约252个交易日）
            if let Some(first) = klines.first() {
                if first.close > 0.0 && latest_close > 0.0 && serenity_score < 85.0 {
                    let gain_12m = (latest_close - first.close) / first.close * 100.0;
                    if gain_12m > max_12m_gain {
                        tracing::info!("{code}: 近12月涨幅 {gain_12m:.0}% > {max_12m_gain}%, 因长期涨幅过大排除");
                        return None;
                    }
                }
            }
        }

        // ── 估值过滤结束 ──

        // 5a. ROIC 近似（ROE > 15% 为强信号）
        let roe = latest.roe.unwrap_or(0.0);
        let roe_ok = roe > 15.0;

        // 5b. 经营现金流/净利润比 > 0.7
        let ocf_ratio_ok = if let (Some(op_cf), Some(net_profit)) =
            (latest.operating_cash_flow, latest.net_profit)
        {
            if net_profit.abs() > 0.0 {
                (op_cf / net_profit).abs() > 0.7
            } else {
                true
            }
        } else {
            true
        };

        // 5c. 毛利率趋势
        let margin_trend_ok = if financials.len() >= 2 {
            let prev_gm = financials[1].gross_margin.unwrap_or(0.0);
            prev_gm > 0.0 && gm >= prev_gm * 0.9
        } else {
            true
        };

        // 6. 目标价与止损计算

        let target_mult = read_f64(vars, "serenity_target_mult", 1.30);
        let stop_mult = read_f64(vars, "serenity_stop_mult", 0.80);
        let entry_range = read_f64(vars, "serenity_entry_range", 0.05);

        let target_price = if let Some(eps) = latest.eps {
            let target_pe = read_f64(vars, "serenity_target_pe", 25.0);
            (eps * target_pe).max(price * target_mult)
        } else {
            price * target_mult
        };
        let stop_loss = price * stop_mult;
        let entry_low = price * (1.0 - entry_range);
        let entry_high = price * (1.0 + entry_range);
        let base_position = read_f64(vars, "serenity_base_position", 10.0);

        // 置信度计算（含 workflow 诊断因子）
        let conf_quality = (gm / 100.0).min(1.0);
        let conf_growth = (rev_growth / 50.0).min(1.0);
        let conf_roe = if roe_ok { 1.0 } else { 0.6 };
        let conf_ocf = if ocf_ratio_ok { 1.0 } else { 0.7 };
        let conf_margin_trend = if margin_trend_ok { 1.0 } else { 0.7 };
        // workflow 评分因子：0-100 归一化到 0.5-1.0
        let conf_workflow = 0.5 + (serenity_score / 200.0).min(0.5);
        // 催化剂加分：每个催化剂 confidence >= 70 加 0.05
        let conf_catalyst = (0.05
            * catalysts
                .iter()
                .filter(|c| c["confidence"].as_f64().unwrap_or(0.0) >= 70.0)
                .count() as f64)
            .min(0.15);

        let consistency = conf_quality * 0.25
            + conf_growth * 0.20
            + conf_roe * 0.15
            + conf_ocf * 0.10
            + conf_margin_trend * 0.10
            + conf_workflow * 0.15
            + conf_catalyst * 0.05;
        let signal_strength = conf_quality.min(conf_growth).min(conf_roe);

        let liquidity = (quote.turnover_rate / 5.0).clamp(0.1, 1.0);
        let price_momentum = (quote.change_pct.abs() / 10.0).clamp(0.1, 1.0);
        let turnover_anomaly = 1.0;
        let conf = calc_confidence(
            consistency,
            signal_strength,
            liquidity,
            price_momentum,
            turnover_anomaly,
        );
        let position = calc_position(base_position, conf, self.period);

        // 构建 reasons（含 workflow 诊断信息）
        let mut reasons = vec![
            format!("确定性财务验证通过"),
            format!("毛利率 {:.1}% > {}% 门槛", gm, min_gross_margin),
            format!("营收同比增速 {:.1}% > {}% 门槛", rev_growth, min_rev_growth),
            format!("负债率 {:.1}% < {}% 上限", dr, max_debt_ratio),
        ];
        if serenity_score > 0.0 {
            reasons.push(format!("瓶颈分析评分: {:.0}/100", serenity_score));
        }
        if roe_ok {
            reasons.push(format!("ROE {:.1}% > 15% 显示资本回报效率高", roe));
        }
        if margin_trend_ok && financials.len() >= 2 {
            let prev_gm = financials[1].gross_margin.unwrap_or(0.0);
            reasons.push(format!("毛利率趋势稳定：从 {:.1}% → {:.1}%", prev_gm, gm));
        }
        // 催化剂摘要
        for cat in catalysts.iter().take(2) {
            let cat_desc = cat["description"].as_str().unwrap_or("");
            let cat_tf = cat["expected_timeframe"].as_str().unwrap_or("");
            if !cat_desc.is_empty() {
                reasons.push(format!("催化剂: {} ({})", cat_desc, cat_tf));
            }
        }

        // 风险提示（含退出信号感知）
        let mut risk_notes = vec![
            "瓶颈环节可能因技术变革或竞争格局变化而失效".to_string(),
            "建议作为投资组合的弹性增强部分，而非全部".to_string(),
        ];
        // workflow 诊断的主风险
        if let Some(primary_risk) = detail["primary_risk"].as_str() {
            if !primary_risk.is_empty() {
                risk_notes.push(format!("工作流诊断风险: {}", primary_risk));
            }
        }
        // 退出信号
        if let Some(urgency) = exit_signals
            .get("overall_exit_urgency")
            .and_then(|v| v.as_str())
        {
            if urgency == "caution" {
                risk_notes.push("⚠ 退出信号：6-12月内关注退出条件".to_string());
            } else if urgency == "watch" {
                risk_notes.push("退出信号：12月以上关注技术替代风险".to_string());
            }
        }
        // 技术替代风险
        if let Some(tech_risk) = exit_signals
            .get("technology_disruption_risk")
            .and_then(|v| v.as_str())
        {
            if !tech_risk.is_empty() && !tech_risk.eq_ignore_ascii_case("null") {
                risk_notes.push(format!("技术替代风险: {}", tech_risk));
            }
        }
        // 毛利率下降
        if !margin_trend_ok && financials.len() >= 2 {
            let prev_gm = financials[1].gross_margin.unwrap_or(0.0);
            risk_notes.push(format!(
                "⚠ 毛利率从 {:.1}% 下降至 {:.1}%，关注技术替代或竞争加剧风险",
                prev_gm, gm
            ));
        }
        // 高负债率
        if dr > 50.0 {
            risk_notes.push("负债率偏高，关注再融资或稀释风险".to_string());
        }

        Some(RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector,
            style: Style::Serenity,
            period: self.period,
            price,
            entry_low,
            entry_high,
            stop_loss,
            target_price,
            position_pct: position,
            holding_days: self.period.default_holding_days(),
            confidence: conf,
            reasons,
            risk_notes,
            secondary_styles: vec![],
            synthetic: false,
        })
    }
}

#[async_trait]
impl RecommendStrategy for SerenityStrategy {
    fn id(&self) -> &'static str {
        match self.period {
            Period::Mid => "serenity_mid",
            Period::Long => "serenity_long",
            _ => unreachable!("Serenity 只支持 Mid/Long，不应被调用其他周期"),
        }
    }
    fn style(&self) -> Style {
        Style::Serenity
    }
    fn period(&self) -> Period {
        self.period
    }
    fn required_vendors(&self) -> &'static [&'static str] {
        &["eastmoney", "ths", "akshare"]
    }

    async fn scan(&self, ctx: &RecoContext<'_>) -> Result<Vec<RecoPick>, String> {
        let mut picks = Vec::new();
        for (code, name, sector) in ctx.seed {
            let _g = ctx.per_code_locks.lock_for(code).await;
            if let Some(p) = self
                .scan_one(ctx.client, code, name, sector.clone(), ctx.vars)
                .await
            {
                picks.push(p);
            }
        }
        Ok(picks)
    }
}
