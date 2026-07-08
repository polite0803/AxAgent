#![allow(clippy::unnecessary_sort_by)]

use axagent_astock_data::AStockClient;

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
}

/// 回退候选股列表（沪深300核心成分股，覆盖主要行业）
const FALLBACK_STOCKS: &[(&str, &str)] = &[
    ("600519", "贵州茅台"),
    ("000858", "五粮液"),
    ("300750", "宁德时代"),
    ("600036", "招商银行"),
    ("601318", "中国平安"),
    ("000333", "美的集团"),
    ("002475", "立讯精密"),
    ("600276", "恒瑞医药"),
    ("300059", "东方财富"),
    ("000651", "格力电器"),
    ("002415", "海康威视"),
    ("600900", "长江电力"),
    ("601888", "中国中免"),
    ("300014", "亿纬锂能"),
    ("002594", "比亚迪"),
    ("601012", "隆基绿能"),
    ("000001", "平安银行"),
    ("600030", "中信证券"),
    ("000002", "万科A"),
    ("601166", "兴业银行"),
    ("601899", "紫金矿业"),
    ("300124", "汇川技术"),
    ("600809", "山西汾酒"),
    ("002714", "牧原股份"),
    ("000568", "泸州老窖"),
    ("603259", "药明康德"),
    ("600887", "伊利股份"),
    ("002230", "科大讯飞"),
    ("300274", "阳光电源"),
    ("601088", "中国神华"),
    ("600585", "海螺水泥"),
    ("000725", "京东方A"),
    ("002304", "洋河股份"),
    ("300760", "迈瑞医疗"),
    ("600031", "三一重工"),
    ("601211", "国泰君安"),
    ("002241", "歌尔股份"),
    ("300408", "三环集团"),
    ("603986", "兆易创新"),
    ("600745", "闻泰科技"),
    ("002044", "美年健康"),
    ("300122", "智飞生物"),
    ("000063", "中兴通讯"),
    ("002049", "紫光国微"),
    ("603501", "韦尔股份"),
    ("601398", "工商银行"),
    ("600028", "中国石化"),
    ("601857", "中国石油"),
];

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
            });
        }

        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(candidates.into_iter().take(20).collect())
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
