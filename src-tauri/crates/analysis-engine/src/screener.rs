#![allow(clippy::unnecessary_sort_by)]

use std::collections::HashSet;

use axagent_astock_data::AStockClient;

use crate::concept_index::ConceptIndex;
use crate::recommender::FALLBACK_STOCKS;

/// 筛选条件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCriteria {
    /// 最小涨跌幅（默认None=不限）
    pub min_change_pct: Option<f64>,
    /// 最大涨跌幅
    pub max_change_pct: Option<f64>,
    /// 龙虎榜净买入>N（万元）
    pub dragon_tiger_net_min: Option<f64>,
    /// 主力净流入>N（万元）
    pub main_inflow_min: Option<f64>,
    /// 北向持仓占比>N%
    pub northbound_ratio_min: Option<f64>,
    /// 换手率>N%
    pub turnover_rate_min: Option<f64>,
    /// RSI 超卖（<30）
    pub rsi_oversold: Option<bool>,
    /// RSI 超买（>70）
    pub rsi_overbought: Option<bool>,
    // ── #1 选股主题维度：概念 / 行业 / 产业链 筛选 ──
    /// 概念筛选（规范 id 或别名，运行时经本体对齐解析），如 `["AI","芯片"]`
    pub concepts: Option<Vec<String>>,
    /// 行业筛选，如 `["银行","半导体"]`
    pub industries: Option<Vec<String>>,
    /// 产业链筛选，如 `["新能源汽车产业链"]`
    pub industry_chains: Option<Vec<String>>,
    /// 多主题之间是否要求同时满足（AND），默认 OR
    pub require_all_themes: Option<bool>,
    /// 估值上限（PE）：用于「低估值主题股」组合筛选
    pub max_pe: Option<f64>,
    /// 估值上限（PB）：用于「低市净率主题股」组合筛选
    pub max_pb: Option<f64>,
}

/// 筛选结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenResult {
    pub stock_code: String,
    pub stock_name: String,
    pub price: f64,
    pub change_pct: f64,
    /// 匹配原因
    pub reasons: Vec<String>,
    /// 综合得分
    pub score: u32,
    /// 命中的主题（概念/行业显示名），主题筛选激活时填充
    pub matched_themes: Option<Vec<String>>,
}

/// 候选快照（量化字段，网络无关，便于单测与主题筛选纯逻辑复用）
#[derive(Debug, Clone)]
pub struct StockSnapshot {
    pub code: String,
    pub name: String,
    pub price: f64,
    pub change_pct: f64,
    pub turnover_rate: f64,
    pub pe: Option<f64>,
    pub pb: Option<f64>,
}

/// 股票筛选器
pub struct StockScreener;

impl StockScreener {
    /// 从自选股中筛选符合条件的标的;自选股为空时回退到 FALLBACK_STOCKS 池
    pub async fn screen_watchlist(
        client: &AStockClient,
        watchlist: &[(String, String)],
        criteria: &ScreenCriteria,
    ) -> Result<Vec<ScreenResult>, String> {
        let mut results = Vec::new();

        // 自选股为空时,使用 FALLBACK_STOCKS 兜底池
        let pool: Vec<(String, String)> = if watchlist.is_empty() {
            tracing::info!(
                "screen_watchlist: 自选股为空,使用 FALLBACK_STOCKS 池 ({} 只)",
                FALLBACK_STOCKS.len()
            );
            FALLBACK_STOCKS
                .iter()
                .map(|(code, name)| (code.to_string(), name.to_string()))
                .collect()
        } else {
            watchlist.to_vec()
        };

        for (code, name) in &pool {
            let mut reasons = Vec::new();
            let mut score = 0u32;

            // 获取行情
            let quote = match client.get_quote(code).await {
                Ok(q) => q,
                Err(_) => continue,
            };

            // 涨跌幅筛选
            if let Some(min) = criteria.min_change_pct {
                if quote.change_pct < min {
                    continue;
                }
                reasons.push(format!("涨幅{:.2}%", quote.change_pct));
                score += 2;
            }
            if let Some(max) = criteria.max_change_pct {
                if quote.change_pct > max {
                    continue;
                }
            }

            // 换手率筛选
            if let Some(tr) = criteria.turnover_rate_min {
                if quote.turnover_rate < tr {
                    continue;
                }
                reasons.push(format!("换手率{:.2}%", quote.turnover_rate));
                score += 2;
            }

            // 龙虎榜
            if criteria.dragon_tiger_net_min.is_some() {
                if let Ok(dt) = client.get_dragon_tiger(code).await {
                    let net: f64 = dt.iter().map(|d| d.net_amount).sum();
                    if let Some(min_net) = criteria.dragon_tiger_net_min {
                        if net >= min_net {
                            reasons.push(format!("龙虎榜净买入{:.0}万", net / 10000.0));
                            score += 3;
                        } else {
                            continue;
                        }
                    }
                }
            }

            // 主力资金
            if criteria.main_inflow_min.is_some() {
                if let Ok(Some(mf)) = client.get_money_flow(code).await {
                    if let Some(min_inflow) = criteria.main_inflow_min {
                        if mf.main_net_inflow >= min_inflow {
                            reasons
                                .push(format!("主力净流入{:.0}万", mf.main_net_inflow / 10000.0));
                            score += 3;
                        } else {
                            continue;
                        }
                    }
                }
            }

            // 北向持仓占比
            if let Some(min_ratio) = criteria.northbound_ratio_min {
                if let Ok(Some(nb)) = client.get_north_bound_holding(code).await {
                    if nb.holding_ratio >= min_ratio {
                        reasons.push(format!("北向持仓{:.2}%", nb.holding_ratio));
                        score += 3;
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if criteria.rsi_oversold.unwrap_or(false) || criteria.rsi_overbought.unwrap_or(false) {
                if let Ok(klines) = client.get_klines(code, "daily", 30).await {
                    if klines.len() >= 8 {
                        let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                        let period = 6usize;
                        let mut avg_gain = 0.0;
                        let mut avg_loss = 0.0;
                        for i in 1..=period {
                            let diff = closes[i] - closes[i - 1];
                            if diff > 0.0 {
                                avg_gain += diff;
                            } else {
                                avg_loss += -diff;
                            }
                        }
                        avg_gain /= period as f64;
                        avg_loss /= period as f64;
                        for i in (period + 1)..closes.len() {
                            let diff = closes[i] - closes[i - 1];
                            let gain = if diff > 0.0 { diff } else { 0.0 };
                            let loss = if diff < 0.0 { -diff } else { 0.0 };
                            avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
                            avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
                        }
                        let rs = if avg_loss > 1e-10 {
                            avg_gain / avg_loss
                        } else {
                            100.0
                        };
                        let rsi = 100.0 - 100.0 / (1.0 + rs);
                        let matches_criteria = (criteria.rsi_oversold.unwrap_or(false)
                            && rsi < 30.0)
                            || (criteria.rsi_overbought.unwrap_or(false) && rsi > 70.0);
                        if !matches_criteria {
                            continue;
                        }
                        if rsi < 30.0 {
                            reasons.push(format!("RSI超卖{:.1}", rsi));
                            score += 3;
                        } else {
                            reasons.push(format!("RSI超买{:.1}", rsi));
                            score += 1;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            results.push(ScreenResult {
                stock_code: code.clone(),
                stock_name: name.clone(),
                price: quote.price,
                change_pct: quote.change_pct,
                reasons,
                score,
                matched_themes: None,
            });
        }

        // 按得分降序
        results.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(results)
    }

    /// 从全市场发现热门候选标的（优先使用实时热门股/行业排名数据，回退到沪深300成分股）
    pub async fn discover_candidates(client: &AStockClient) -> Result<Vec<ScreenResult>, String> {
        // 优先：从实时市场数据获取股票列表
        let stock_list = Self::fetch_dynamic_candidates(client).await;

        let mut candidates = Vec::new();
        for (code, name) in &stock_list {
            let quote = match client.get_quote(code).await {
                Ok(q) => q,
                Err(_) => continue,
            };
            let mut reasons = Vec::new();
            let mut score = 5u32;
            if quote.change_pct.abs() > 2.0 {
                let dir = if quote.change_pct > 0.0 { "涨" } else { "跌" };
                reasons.push(format!("{}幅 {:.2}%", dir, quote.change_pct));
                score += (quote.change_pct.abs() * 2.0) as u32;
            }
            if quote.turnover_rate > 3.0 {
                reasons.push(format!("换手 {:.2}%", quote.turnover_rate));
                score += (quote.turnover_rate / 2.0) as u32;
            }

            // PE/PB 估值信号
            if let Some(pe) = quote.pe {
                if pe < 15.0 && pe > 0.0 {
                    reasons.push(format!("低PE {:.1}", pe));
                    score += 5;
                }
            }

            // ST 风险提示
            if quote.is_st {
                reasons.push("ST警示".to_string());
                score = score.saturating_sub(10);
            }

            candidates.push(ScreenResult {
                stock_code: code.to_string(),
                stock_name: name.to_string(),
                price: quote.price,
                change_pct: quote.change_pct,
                reasons,
                score,
                matched_themes: None,
            });
        }

        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(candidates.into_iter().take(20).collect())
    }

    /// 纯函数筛选：**先按主题收窄候选宇宙，再叠加量化指标打分**。
    ///
    /// - `index` 为 `None` 或无障碍主题筛选时，退化为纯量化筛选（与旧 `screen_watchlist` 行为一致）。
    /// - 主题宇宙由 `ConceptIndex::theme_universe` 解析（OR/AND 由 `require_all_themes` 控制），
    ///   不在宇宙内的股票直接跳过——这就是「选股从纯指标升级为主题 + 指标」的核心。
    pub fn screen_snapshots(
        snapshots: &[StockSnapshot],
        criteria: &ScreenCriteria,
        index: Option<&ConceptIndex>,
    ) -> Vec<ScreenResult> {
        // 1) 收集主题查询词
        let mut theme_queries: Vec<String> = Vec::new();
        if let Some(c) = &criteria.concepts {
            theme_queries.extend(c.iter().cloned());
        }
        if let Some(i) = &criteria.industries {
            theme_queries.extend(i.iter().cloned());
        }
        if let Some(ic) = &criteria.industry_chains {
            theme_queries.extend(ic.iter().cloned());
        }

        let require_all = criteria.require_all_themes.unwrap_or(false);
        let theme_universe: HashSet<String> = match (&index, !theme_queries.is_empty()) {
            (Some(idx), true) => idx.theme_universe(&theme_queries, require_all),
            _ => HashSet::new(),
        };
        let theme_active = !theme_universe.is_empty();

        let mut results = Vec::new();
        for s in snapshots {
            // 主题收窄：不在主题宇宙内的直接跳过
            if theme_active && !theme_universe.contains(&s.code) {
                continue;
            }

            let mut reasons = Vec::new();
            let mut score = 0u32;

            // 涨跌幅
            if let Some(min) = criteria.min_change_pct {
                if s.change_pct < min {
                    continue;
                }
                reasons.push(format!("涨幅{:.2}%", s.change_pct));
                score += 2;
            }
            if let Some(max) = criteria.max_change_pct {
                if s.change_pct > max {
                    continue;
                }
            }

            // 换手率
            if let Some(tr) = criteria.turnover_rate_min {
                if s.turnover_rate < tr {
                    continue;
                }
                reasons.push(format!("换手率{:.2}%", s.turnover_rate));
                score += 2;
            }

            // 估值上限（PE）：低估值主题股
            if let Some(max_pe) = criteria.max_pe {
                match s.pe {
                    Some(pe) if pe > 0.0 && pe <= max_pe => {
                        reasons.push(format!("低PE{:.1}", pe));
                        score += 5;
                    },
                    _ => continue,
                }
            }

            // 估值上限（PB）：低市净率主题股
            if let Some(max_pb) = criteria.max_pb {
                match s.pb {
                    Some(pb) if pb > 0.0 && pb <= max_pb => {
                        reasons.push(format!("低PB{:.1}", pb));
                        score += 3;
                    },
                    _ => continue,
                }
            }

            // 主题命中标注（命中的概念/行业显示名）
            let matched_themes: Vec<String> = if theme_active {
                let mut set = HashSet::new();
                if let Some(idx) = index {
                    for q in &theme_queries {
                        if let Some(cid) = idx.resolve(q) {
                            if idx.members(cid).contains(&s.code) {
                                if let Some(d) = idx.display(cid) {
                                    set.insert(d.to_string());
                                }
                            }
                        }
                    }
                }
                set.into_iter().collect()
            } else {
                Vec::new()
            };

            results.push(ScreenResult {
                stock_code: s.code.clone(),
                stock_name: s.name.clone(),
                price: s.price,
                change_pct: s.change_pct,
                reasons,
                score,
                matched_themes: if matched_themes.is_empty() {
                    None
                } else {
                    Some(matched_themes)
                },
            });
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// 主题选股（网络版）：基于知识库 / 本体索引，先按主题收窄候选宇宙，再叠加量化指标。
    ///
    /// - `universe`：初始候选池（如自选股）；为空且主题筛选激活时，直接以概念成员作为宇宙。
    /// - `index`：概念 / 行业主题索引（来自知识图谱或 vendor）；为 `None` 时退化为纯量化选股。
    ///
    /// 生产环境建议：`index` 的成员数据由 `astock-data`（vendor 概念板块）填充，
    /// 知识图谱仅作种子 / 补全，避免实时性不足。
    pub async fn screen_by_theme(
        client: &AStockClient,
        universe: &[(String, String)],
        criteria: &ScreenCriteria,
        index: Option<&ConceptIndex>,
    ) -> Result<Vec<ScreenResult>, String> {
        // 解析主题宇宙
        let mut theme_queries: Vec<String> = Vec::new();
        if let Some(c) = &criteria.concepts {
            theme_queries.extend(c.iter().cloned());
        }
        if let Some(i) = &criteria.industries {
            theme_queries.extend(i.iter().cloned());
        }
        if let Some(ic) = &criteria.industry_chains {
            theme_queries.extend(ic.iter().cloned());
        }
        let theme_universe: HashSet<String> = match (&index, !theme_queries.is_empty()) {
            (Some(idx), true) => {
                idx.theme_universe(&theme_queries, criteria.require_all_themes.unwrap_or(false))
            },
            _ => HashSet::new(),
        };

        // 候选代码集合：主题宇宙 ∩ 自选池（若两者都有），否则取非空者
        let base: HashSet<String> = universe.iter().map(|(c, _)| c.clone()).collect();
        let candidate_codes: HashSet<String> = if !theme_universe.is_empty() {
            if base.is_empty() {
                theme_universe
            } else {
                base.intersection(&theme_universe).cloned().collect()
            }
        } else {
            base
        };

        // 拉取行情构建快照
        let mut snapshots = Vec::new();
        for code in &candidate_codes {
            let quote = match client.get_quote(code).await {
                Ok(q) => q,
                Err(_) => continue,
            };
            let name = universe
                .iter()
                .find(|(c, _)| c == code)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            snapshots.push(StockSnapshot {
                code: code.clone(),
                name,
                price: quote.price,
                change_pct: quote.change_pct,
                turnover_rate: quote.turnover_rate,
                pe: quote.pe,
                pb: quote.pb,
            });
        }

        Ok(Self::screen_snapshots(&snapshots, criteria, index))
    }

    /// 从实时市场数据动态获取候选股票列表（热门股 + 行业排名），回退到沪深300
    async fn fetch_dynamic_candidates(client: &AStockClient) -> Vec<(String, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut stocks = Vec::new();

        // 1. 热门个股
        if let Ok(hot) = client.get_hot_stocks().await {
            for h in hot.iter().take(30) {
                if seen.insert(h.stock_code.clone()) {
                    stocks.push((h.stock_code.clone(), h.stock_name.clone()));
                }
            }
        }

        // 2. 行业排名靠前的龙头
        if let Ok(industries) = client.get_industry_ranking().await {
            for ind in industries.iter().take(10) {
                if let (Some(ref code), Some(ref name)) = (&ind.leader_code, &ind.leader_name) {
                    if seen.insert(code.clone()) {
                        stocks.push((code.clone(), name.clone()));
                    }
                }
            }
        }

        // 回退：沪深300 核心成分股
        if stocks.is_empty() {
            for (code, name) in FALLBACK_STOCKS {
                if seen.insert(code.to_string()) {
                    stocks.push((code.to_string(), name.to_string()));
                }
            }
        }

        stocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept_index::build_sample_index;

    /// 构造样例快照：覆盖 AI 概念三只 + 半导体行业 + 银行/保险
    fn sample_snapshots() -> Vec<StockSnapshot> {
        vec![
            // AI 概念成员：002415(PE18), 688981(PE25), 601318(PE12)
            StockSnapshot {
                code: "002415".into(),
                name: "汇川技术".into(),
                price: 60.0,
                change_pct: 1.5,
                turnover_rate: 2.0,
                pe: Some(18.0),
                pb: Some(5.0),
            },
            StockSnapshot {
                code: "688981".into(),
                name: "中芯国际".into(),
                price: 50.0,
                change_pct: 3.0,
                turnover_rate: 4.0,
                pe: Some(25.0),
                pb: Some(3.0),
            },
            StockSnapshot {
                code: "601318".into(),
                name: "中国平安".into(),
                price: 48.0,
                change_pct: -0.5,
                turnover_rate: 1.0,
                pe: Some(12.0),
                pb: Some(1.2),
            },
            // 半导体行业但非 AI 概念：603501
            StockSnapshot {
                code: "603501".into(),
                name: "韦尔股份".into(),
                price: 100.0,
                change_pct: 2.0,
                turnover_rate: 3.0,
                pe: Some(30.0),
                pb: Some(6.0),
            },
            // 银行：000001（非 AI / 半导体）
            StockSnapshot {
                code: "000001".into(),
                name: "平安银行".into(),
                price: 11.0,
                change_pct: 0.5,
                turnover_rate: 1.5,
                pe: Some(5.0),
                pb: Some(0.6),
            },
        ]
    }

    #[test]
    fn theme_plus_valuation_ai_low_pe() {
        let idx = build_sample_index();
        let snaps = sample_snapshots();
        let criteria = ScreenCriteria {
            min_change_pct: None,
            max_change_pct: None,
            dragon_tiger_net_min: None,
            main_inflow_min: None,
            northbound_ratio_min: None,
            turnover_rate_min: None,
            rsi_oversold: None,
            rsi_overbought: None,
            concepts: Some(vec!["AI".to_string()]),
            industries: None,
            industry_chains: None,
            require_all_themes: None,
            max_pe: Some(20.0),
            max_pb: None,
        };

        let results = StockScreener::screen_snapshots(&snaps, &criteria, Some(&idx));

        // AI 概念 ∩ PE<=20 → 002415(PE18) 与 601318(PE12)；688981(PE25) 被估值过滤掉
        let codes: Vec<&str> = results.iter().map(|r| r.stock_code.as_str()).collect();
        assert!(codes.contains(&"002415"), "应含汇川技术(低PE的AI概念股): {:?}", codes);
        assert!(codes.contains(&"601318"), "应含中国平安(低PE的AI概念股): {:?}", codes);
        assert!(!codes.contains(&"688981"), "中芯国际 PE=25 应被 max_pe=20 过滤掉: {:?}", codes);
        assert!(!codes.contains(&"000001"), "平安银行非 AI 概念，应被主题宇宙收窄掉: {:?}", codes);

        // 命中的主题应标注为「人工智能」
        for r in &results {
            assert_eq!(r.matched_themes.as_deref(), Some(&["人工智能".to_string()][..]));
            assert!(r.reasons.iter().any(|x| x.starts_with("低PE")));
        }
    }

    #[test]
    fn no_theme_falls_back_to_pure_quant() {
        let snaps = sample_snapshots();
        // 不传 index，且只设估值上限：应退化为纯量化（不受主题收窄）
        let criteria = ScreenCriteria {
            min_change_pct: None,
            max_change_pct: None,
            dragon_tiger_net_min: None,
            main_inflow_min: None,
            northbound_ratio_min: None,
            turnover_rate_min: None,
            rsi_oversold: None,
            rsi_overbought: None,
            concepts: None,
            industries: None,
            industry_chains: None,
            require_all_themes: None,
            max_pe: Some(20.0),
            max_pb: None,
        };
        let results = StockScreener::screen_snapshots(&snaps, &criteria, None);
        // 纯量化下，所有 PE<=20 的股票都入选（含非 AI 的 000001 / 601318）
        let codes: Vec<&str> = results.iter().map(|r| r.stock_code.as_str()).collect();
        assert!(codes.contains(&"000001"), "无主题时应退化纯量化，平安银行 PE=5 入选: {:?}", codes);
        assert!(codes.contains(&"601318"));
        assert!(!codes.contains(&"688981")); // PE=25 仍被过滤
        assert!(codes.contains(&"002415"), "汇川技术 PE=18 应入选: {:?}", codes);
        // 无主题时不应有 matched_themes
        for r in &results {
            assert!(r.matched_themes.is_none());
        }
    }

    #[test]
    fn industry_screen_uses_industry_membership() {
        let idx = build_sample_index();
        let snaps = sample_snapshots();
        let criteria = ScreenCriteria {
            min_change_pct: None,
            max_change_pct: None,
            dragon_tiger_net_min: None,
            main_inflow_min: None,
            northbound_ratio_min: None,
            turnover_rate_min: None,
            rsi_oversold: None,
            rsi_overbought: None,
            concepts: None,
            industries: Some(vec!["半导体".to_string()]),
            industry_chains: None,
            require_all_themes: None,
            max_pe: None,
            max_pb: None,
        };
        let results = StockScreener::screen_snapshots(&snaps, &criteria, Some(&idx));
        let codes: Vec<&str> = results.iter().map(|r| r.stock_code.as_str()).collect();
        // 半导体行业成员：002415, 688981, 603501（均入选，无估值过滤）
        assert!(codes.contains(&"002415"));
        assert!(codes.contains(&"688981"));
        assert!(codes.contains(&"603501"));
        assert!(!codes.contains(&"601318")); // 保险，非半导体
    }
}
