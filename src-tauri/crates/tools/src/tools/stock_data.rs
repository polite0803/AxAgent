use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_astock_data::regime::MarketRegime;
use axagent_astock_data::{AStockClient, KLine};
use chrono::Local;
use serde_json::{Value, json};
use std::sync::Arc;

fn te(msg: String) -> ToolError {
    ToolError::execution_failed(msg)
}

/// 从 tool input 中提取 _template_vars 中指定 key 的值，取不到则返回默认值。
/// 模板变量在 tool_executor.rs 构建 `resolved_args` 时自动注入。
fn tv_f64(input: &Value, key: &str, default: f64) -> f64 {
    input
        .get("_template_vars")
        .and_then(|tv| tv.get(key))
        .and_then(|v| v.as_f64())
        .unwrap_or(default)
}


// ── 1. StockQuoteTool ──
pub struct StockQuoteTool {
    pub client: Arc<AStockClient>,
}
impl StockQuoteTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockQuoteTool {
    fn name(&self) -> &str {
        "get_stock_quote"
    }
    fn description(&self) -> &str {
        "获取A股实时行情：现价、涨跌幅、PE、PB、市值"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_quote(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 2. StockKlineTool ──
pub struct StockKlineTool {
    pub client: Arc<AStockClient>,
}
impl StockKlineTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockKlineTool {
    fn name(&self) -> &str {
        "get_stock_kline"
    }
    fn description(&self) -> &str {
        "获取A股历史K线"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"},"period":{"type":"string","description":"daily/weekly/monthly"},"limit":{"type":"integer","description":"数量","default":120}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let period = input["period"].as_str().unwrap_or("daily");
        let limit = input["limit"].as_u64().unwrap_or(120) as u32;
        let r = self
            .client
            .get_klines(code, period, limit)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 3. StockFinancialsTool ──
pub struct StockFinancialsTool {
    pub client: Arc<AStockClient>,
}
impl StockFinancialsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockFinancialsTool {
    fn name(&self) -> &str {
        "get_stock_financials"
    }
    fn description(&self) -> &str {
        "获取A股财务报表"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_financials(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 4. StockNewsTool ──
pub struct StockNewsTool {
    pub client: Arc<AStockClient>,
}
impl StockNewsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockNewsTool {
    fn name(&self) -> &str {
        "get_stock_news"
    }
    fn description(&self) -> &str {
        "获取A股新闻公告"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"},"limit":{"type":"integer","default":30}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let limit = input["limit"].as_u64().unwrap_or(30) as u32;
        let r = self
            .client
            .get_news(code, limit)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 5. StockMoneyFlowTool ──
pub struct StockMoneyFlowTool {
    pub client: Arc<AStockClient>,
}
impl StockMoneyFlowTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockMoneyFlowTool {
    fn name(&self) -> &str {
        "get_stock_money_flow"
    }
    fn description(&self) -> &str {
        "获取A股资金流向"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_money_flow(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 6. StockHotStocksTool ──
pub struct StockHotStocksTool {
    pub client: Arc<AStockClient>,
}
impl StockHotStocksTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockHotStocksTool {
    fn name(&self) -> &str {
        "get_hot_stocks"
    }
    fn description(&self) -> &str {
        "获取当前热门A股榜单"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let r = self
            .client
            .get_hot_stocks()
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 7. StockIndustryRankTool ──
pub struct StockIndustryRankTool {
    pub client: Arc<AStockClient>,
}
impl StockIndustryRankTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockIndustryRankTool {
    fn name(&self) -> &str {
        "get_industry_ranking"
    }
    fn description(&self) -> &str {
        "获取行业板块涨跌排名，传入 stock_code 时自动筛选目标股票所属行业"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "stock_code": {
                    "type": "string",
                    "description": "6位股票代码（可选，传入时返回目标股票的行业归属+该行业排名数据）"
                }
            }
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let stock_code = input["stock_code"].as_str().filter(|s| !s.is_empty());
        let all_rankings = self
            .client
            .get_industry_ranking()
            .await
            .map_err(|e| te(e.to_string()))?;

        // 如果提供了 stock_code，查询该股票的行业归属并在返回数据中高亮
        if let Some(code) = stock_code {
            let maybe_blocks = self.client.get_concept_blocks(code).await.ok().flatten();

            let stock_industry: &str = maybe_blocks
                .as_ref()
                .map(|cb| cb.industry.as_str())
                .unwrap_or("");

            // 在返回结果中标记目标股票的行业位置
            let enriched: Vec<serde_json::Value> = all_rankings
                .iter()
                .map(|r| {
                    let is_target = !stock_industry.is_empty() && r.industry_name == stock_industry;
                    serde_json::json!({
                        "industryName": r.industry_name,
                        "changePct": r.change_pct,
                        "mainInflow": r.main_inflow,
                        "leaderCode": r.leader_code,
                        "leaderName": r.leader_name,
                        "leaderChangePct": r.leader_change_pct,
                        "isTargetStockIndustry": is_target,
                    })
                })
                .collect();

            let has_match = !stock_industry.is_empty()
                && all_rankings
                    .iter()
                    .any(|r| r.industry_name == stock_industry);

            let mut result = serde_json::json!({
                "rankings": enriched,
                "targetStockIndustry": stock_industry,
                "hasMatchInRankings": has_match,
            });

            // 附加概念/地域标签（用 as_ref 避免 move 冲突）
            if let Some(ref cb) = maybe_blocks {
                result["conceptTags"] =
                    serde_json::to_value(cb.concepts.iter().map(|c| &c.name).collect::<Vec<_>>())
                        .unwrap_or_default();
                result["regionTags"] =
                    serde_json::to_value(cb.regions.iter().map(|r| &r.name).collect::<Vec<_>>())
                        .unwrap_or_default();
            }

            Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap_or_default()))
        } else {
            // 无 stock_code 时返回原始排名（向后兼容）
            Ok(ToolResult::success(serde_json::to_string(&all_rankings).unwrap_or_default()))
        }
    }
}

// ── 8. StockAnnouncementsTool ──
pub struct StockAnnouncementsTool {
    pub client: Arc<AStockClient>,
}
impl StockAnnouncementsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockAnnouncementsTool {
    fn name(&self) -> &str {
        "get_stock_announcements"
    }
    fn description(&self) -> &str {
        "获取A股公司公告/披露文件"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_announcements(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 9. StockConsensusEPSTool ──
pub struct StockConsensusEPSTool {
    pub client: Arc<AStockClient>,
}
impl StockConsensusEPSTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockConsensusEPSTool {
    fn name(&self) -> &str {
        "get_consensus_eps"
    }
    fn description(&self) -> &str {
        "获取分析师一致预期EPS预测"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_consensus_eps(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 10. SearchStockTool ──
pub struct SearchStockTool {
    pub client: Arc<AStockClient>,
}
impl SearchStockTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for SearchStockTool {
    fn name(&self) -> &str {
        "search_stock"
    }
    fn description(&self) -> &str {
        "按代码或名称模糊搜索A股"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"keyword":{"type":"string","description":"股票代码或名称"}},"required":["keyword"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kw = input["keyword"]
            .as_str()
            .ok_or_else(|| te("keyword不能为空".into()))?;
        let r = self
            .client
            .search_stock(kw)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 10b. SearchNewsTool ──
pub struct SearchNewsTool {
    pub client: Arc<AStockClient>,
}
impl SearchNewsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for SearchNewsTool {
    fn name(&self) -> &str {
        "search_news"
    }
    fn description(&self) -> &str {
        "按关键词搜索财经新闻，用于验证催化剂/CapEx/行业趋势"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"keyword":{"type":"string","description":"搜索关键词"},"limit":{"type":"integer","description":"返回条数"}},"required":["keyword"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let kw = input["keyword"]
            .as_str()
            .ok_or_else(|| te("keyword不能为空".into()))?;
        let limit = input["limit"].as_u64().unwrap_or(10) as u32;
        let r = self
            .client
            .search_news(kw, limit)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

pub struct ComputeScoringTool {
    pub client: Arc<AStockClient>,
}
impl ComputeScoringTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeScoringTool {
    fn name(&self) -> &str {
        "compute_scoring"
    }
    fn description(&self) -> &str {
        "六维度技术评分：趋势/乖离/MACD/量能/RSI/支撑+价值修正"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let (klines_r, quote_r) =
            tokio::join!(self.client.get_klines(code, "daily", 120), self.client.get_quote(code));
        let klines = klines_r.map_err(|e| te(e.to_string()))?;
        let quote = quote_r.map_err(|e| te(e.to_string()))?;
        let indicators = axagent_astock_data::indicators::compute_indicators(code, &klines);

        // ── 从 _template_vars 读取评分权重（用户在设置面板配置） ──
        let w_trend = tv_f64(&input, "scoring_trend", 30.0);
        let w_deviation = tv_f64(&input, "scoring_deviation", 20.0);
        let w_macd = tv_f64(&input, "scoring_macd", 15.0);
        let w_volume = tv_f64(&input, "scoring_volume", 15.0);
        let w_rsi = tv_f64(&input, "scoring_rsi", 10.0);
        let w_support = tv_f64(&input, "scoring_support", 10.0);

        // ── 从 _template_vars 读取催化剂维度（由 a-catalyst 节点注入） ──
        let catalyst_score = tv_f64(&input, "catalyst_analyst_score", 50.0);
        let catalyst_level = input
            .get("_template_vars")
            .and_then(|tv| tv.get("catalyst_level"))
            .and_then(|v| v.as_str())
            .unwrap_or("无")
            .to_string();
        let institutional_trace = input
            .get("_template_vars")
            .and_then(|tv| tv.get("institutional_trace"))
            .and_then(|v| v.as_str())
            .unwrap_or("无")
            .to_string();

        // ── 使用 ScoringEngine（已下沉到 astock-data crate）做客观评分 ──
        use axagent_astock_data::scoring::{ScoringEngine, ScoringWeights};
        let user_weights = ScoringWeights {
            trend: w_trend,
            deviation: w_deviation,
            macd: w_macd,
            volume: w_volume,
            rsi: w_rsi,
            support: w_support,
            ..Default::default()
        };
        let obj_score = ScoringEngine::score(&indicators, quote.price, Some(&user_weights));

        // 映射 ObjectiveScore（0-100 分档）到前端期望的 0-100 浮点数格式
        let trend_score = obj_score.trend_score as f64 / 30.0 * 100.0;
        let deviation_score = obj_score.deviation_score as f64 / 20.0 * 100.0;
        let macd_score = obj_score.macd_score as f64 / 15.0 * 100.0;
        let volume_score = obj_score.volume_score as f64 / 15.0 * 100.0;
        let rsi_score = obj_score.rsi_score as f64 / 10.0 * 100.0;
        let support_score = obj_score.support_score as f64 / 5.0 * 100.0;

        // 基本面修正（从 _template_vars 读取估值阈值）
        let pe_low = tv_f64(&input, "val_pe_low", 15.0);
        let pe_high = tv_f64(&input, "val_pe_high", 50.0);
        let pb_low = tv_f64(&input, "val_pb_low", 1.0);
        let pb_high = tv_f64(&input, "val_pb_high", 6.0);
        let mut value_adj = 0.0;
        if let Some(pe) = quote.pe {
            if pe > 0.0 && pe < pe_low {
                value_adj = 10.0;
            } else if (pe_low..pe_high).contains(&pe) {
                value_adj = 5.0;
            } else if pe >= pe_high {
                value_adj = -10.0;
            }
        }
        if let Some(pb) = quote.pb {
            if pb > 0.0 && pb < pb_low {
                value_adj += 10.0;
            } else if (pb_low..pb_high).contains(&pb) {
                value_adj += 5.0;
            } else if pb >= pb_high {
                value_adj -= 5.0;
            }
        }

        // ── 使用用户配置的权重计算最终评分 ──
        let weights = [
            w_trend / 100.0,
            w_deviation / 100.0,
            w_macd / 100.0,
            w_volume / 100.0,
            w_rsi / 100.0,
            w_support / 100.0,
        ];
        let scores = [
            trend_score,
            deviation_score,
            macd_score,
            volume_score,
            rsi_score,
            support_score,
        ];
        let weighted: f64 = scores.iter().zip(weights.iter()).map(|(s, w)| s * w).sum();

        // 催化剂加成：技术 6 维归 0.85，催化剂 1 维占 0.15
        // L3/L2 + 机构建仓时额外 +3~5 分（突破保守派的"安全边际"卡口）
        let catalyst_weight = 0.15;
        let technical_weight = 1.0 - catalyst_weight;
        let mut catalyst_bonus = 0.0;
        if catalyst_level == "L3估值体系级" {
            catalyst_bonus += 3.0;
        } else if catalyst_level == "L2业绩拐点级" {
            catalyst_bonus += 2.0;
        }
        if institutional_trace == "有建仓痕迹" || institutional_trace == "疑似建仓" {
            catalyst_bonus += 2.0;
        }
        let total_score = ((weighted * technical_weight)
            + (catalyst_score * catalyst_weight * 100.0)
            + value_adj
            + catalyst_bonus)
            .clamp(0.0, 100.0);

        let mut warnings: Vec<Value> = Vec::new();
        if indicators.bias_ma5.is_nan() || indicators.bias_ma20.is_nan() {
            warnings.push(Value::String("BIAS指标异常：MA数据不足".into()));
        }
        if quote.price <= 0.0 {
            warnings.push(Value::String("当前价格无效（可能停牌或未开盘）".into()));
        }
        let freshness = if indicators
            .latest_date
            .contains(&Local::now().format("%Y-%m-%d").to_string())
        {
            "today"
        } else if quote.price > 0.0 {
            "delayed"
        } else {
            "stale"
        };

        let rating = if total_score >= 80.0 {
            "强烈推荐"
        } else if total_score >= 65.0 {
            "推荐"
        } else if total_score >= 50.0 {
            "中性"
        } else if total_score >= 35.0 {
            "谨慎"
        } else {
            "回避"
        };

        let result = json!({
            "stockCode": code,
            "stockName": quote.name,
            "currentPrice": quote.price,
            "latestDate": indicators.latest_date,
            "totalScore": (total_score * 10.0).round() / 10.0,
            "rating": rating,
            "dimensions": {
                "trend": {"score": trend_score, "detail": indicators.ma_alignment},
                "deviation": {"score": deviation_score, "detail": format!("BIAS5={:.1}% BIAS20={:.1}%", indicators.bias_ma5, indicators.bias_ma20)},
                "macd": {"score": macd_score, "detail": indicators.macd_signal},
                "volume": {"score": volume_score, "detail": format!("{} 量比={:.2}", indicators.volume_signal, indicators.volume_ratio)},
                "rsi": {"score": rsi_score, "detail": format!("RSI6={:.1} RSI12={:.1} RSI24={:.1} {}", indicators.rsi6, indicators.rsi12, indicators.rsi24, indicators.rsi_signal)},
                "support": {"score": support_score, "detail": format!("支撑={:?} 压力={:?} 布林={}", indicators.support_levels, indicators.resistance_levels, indicators.boll_position)},
            },
            "valueAdjustment": value_adj,
            "catalystScore": (catalyst_score * 10.0).round() / 10.0,
            "catalystLevel": catalyst_level,
            "institutionalTrace": institutional_trace,
            "indicators": indicators,
            "credibility": {
                "dataCompleteness": 100.0,
                "dataFreshness": freshness,
                "source": "tencent|eastmoney",
                "warnings": Value::Array(warnings)
            },
            // P1/P2: 因子回测 + 市场状态分类
            "factor_backtest": compute_factor_backtest_stats(&klines, 5, 60),
            "market_regime": compute_market_regime(&klines),
        });
        Ok(ToolResult::success(serde_json::to_string(&result).unwrap_or_default()))
    }
}

// ── P1: 因子回测 ──

/// 计算 Pearson 相关系数作为信息系数（IC）。
/// V38 修复：旧实现用 (wr - 0.5) * 2.0，这只是胜率的线性变换，不是真正的信息系数。
/// 正确的 IC 应度量因子信号与未来收益的线性相关性。
fn pearson_ic(sig_vals: &[f64], ret_vals: &[f64]) -> f64 {
    let n = sig_vals.len();
    if n < 5 {
        return 0.0;
    }
    let mean_sig = sig_vals.iter().sum::<f64>() / n as f64;
    let mean_ret = ret_vals.iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_sig = 0.0;
    let mut var_ret = 0.0;
    for i in 0..n {
        let ds = sig_vals[i] - mean_sig;
        let dr = ret_vals[i] - mean_ret;
        cov += ds * dr;
        var_sig += ds * ds;
        var_ret += dr * dr;
    }
    if var_sig == 0.0 || var_ret == 0.0 {
        return 0.0;
    }
    (cov / (var_sig * var_ret).sqrt()).clamp(-1.0, 1.0)
}

/// 在历史 K 线数据上回测各技术因子的预测能力。
/// 对每个有效时间窗口计算因子信号与后续 N 日涨跌幅，用 Pearson 相关系数作为 IC。
/// V38 修复：扣除预估交易成本（手续费+印花税+滑点≈0.3%），避免短期回测虚高。
fn compute_factor_backtest_stats(klines: &[KLine], forecast: usize, lookback: usize) -> Value {
    let n = klines.len();
    if n < lookback + forecast + 5 {
        return json!({"error":"数据不足","windows":0});
    }
    let cost_pct = 0.003; // 交易成本：手续费0.025% + 印花税0.1% + 滑点0.175%
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let mut all_sigs: Vec<Vec<f64>> = Vec::new();
    let mut all_rets: Vec<f64> = Vec::new();
    for t in lookback..n - forecast {
        let window = &klines[t - lookback..t];
        let ind = axagent_astock_data::indicators::compute_indicators("", window);
        let raw_ret = (closes[t + forecast] - closes[t]) / closes[t];
        // 扣除交易成本：买入和卖出各一次，共 2*cost_pct
        let forward_ret = raw_ret - 2.0 * cost_pct;
        all_sigs.push(vec![
            match ind.ma_alignment.as_str() {
                "多头排列" => 1.0,
                "弱多头" => 0.5,
                "空头排列" => -1.0,
                _ => 0.0,
            },
            match ind.macd_signal.as_str() {
                "金叉" => 1.0,
                "多头运行" => 0.5,
                "死叉" => -1.0,
                "空头运行" => -0.5,
                _ => 0.0,
            },
            match ind.volume_signal.as_str() {
                "放量突破" | "放量上涨" => 1.0,
                "缩量回调" => 0.3,
                "放量下跌" => -1.0,
                _ => 0.0,
            },
            if ind.rsi6 > 70.0 {
                -0.3
            } else if ind.rsi6 > 60.0 {
                0.3
            } else if ind.rsi6 > 40.0 {
                0.5
            } else if ind.rsi6 > 30.0 {
                0.2
            } else {
                -0.5
            },
            if ind.bias_ma5 > 5.0 {
                -0.5
            } else if ind.bias_ma5 > 2.0 {
                0.3
            } else if ind.bias_ma5 > -2.0 {
                0.5
            } else if ind.bias_ma5 > -5.0 {
                0.2
            } else {
                0.5
            },
        ]);
        all_rets.push(forward_ret);
    }
    let names = ["trend", "macd", "volume", "rsi", "bias"];
    let mut factors = serde_json::Map::new();
    let mut total_abs_ic = 0.0_f64;
    for (i, name) in names.iter().enumerate() {
        let mut sig_vals: Vec<f64> = Vec::new();
        let mut ret_vals: Vec<f64> = Vec::new();
        for (sig_vec, ret) in all_sigs.iter().zip(all_rets.iter()) {
            let sig = sig_vec[i];
            if sig.abs() < 0.01 {
                continue;
            }
            sig_vals.push(sig);
            ret_vals.push(*ret);
        }
        let wr = if !ret_vals.is_empty() {
            let correct = sig_vals
                .iter()
                .zip(ret_vals.iter())
                .filter(|&(s, r)| (*s > 0.0 && *r > 0.0) || (*s < 0.0 && *r < 0.0))
                .count();
            correct as f64 / ret_vals.len() as f64
        } else {
            0.5
        };
        let ic = pearson_ic(&sig_vals, &ret_vals);
        total_abs_ic += ic.abs();
        factors.insert(
            (*name).to_string(),
            json!({"win_rate": (wr*1000.0).round()/1000.0,
                   "ic": (ic*100.0).round()/100.0,
                   "samples": ret_vals.len()}),
        );
    }
    // 归一化权重
    for name in names.iter() {
        if let Some(entry) = factors.get_mut(*name) {
            let ic_abs = entry["ic"].as_f64().unwrap_or(0.0).abs();
            let w = if total_abs_ic > 0.01 {
                ic_abs / total_abs_ic
            } else {
                0.2
            };
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("weight".into(), json!((w * 100.0).round() / 100.0));
            }
        }
    }

    // ── V56: 代理因子权重 — 为非技术面因子提供数据驱动的权重键 ──
    // f2(共识)/f5(估值)/f9(资金流)/f10(筹码) 的信号来自 LLM/外部数据，
    // 无法用 K 线直接回测 IC。但可以用语义相关的技术面 IC 作为代理：
    //   consensus ~ trend+macd（共识通常反映技术面趋势，多空辩论结论与趋势/金叉正相关）
    //   valuation ~ bias（乖离率反映价格偏离均值程度，与估值过高/过低语义类似）
    //   money_flow ~ volume（资金流直接与成交量正相关）
    //   chip ~ rsi（筹码集中度影响超买超卖，RSI 反映买卖力量失衡）
    // 注: 这些是代理权重，未来应替换为基于 decision_validations 表的真实历史 IC。
    //     代理权重的归一化基准 = 5 个技术因子的平均权重(0.20)，
    //     根据 proxy_ic 与技术因子平均 IC 的比值缩放。
    let tech_avg_ic = total_abs_ic / 5.0; // 5 个技术因子的平均绝对 IC
    let proxy_pairs: [(&str, &str); 4] = [
        ("consensus", "trend"),   // f2 共识 ~ trend
        ("valuation", "bias"),    // f5 估值 ~ bias
        ("money_flow", "volume"), // f9 资金流 ~ volume
        ("chip", "rsi"),          // f10 筹码 ~ rsi
    ];
    for (proxy_name, tech_name) in &proxy_pairs {
        let tech_ic_abs = factors
            .get(*tech_name)
            .and_then(|f| f["ic"].as_f64())
            .unwrap_or(0.0)
            .abs();
        // 代理权重 = 技术因子权重 × (proxy_ic / tech_avg_ic)，clamp 到 [0.05, 0.40]
        let proxy_weight = if tech_avg_ic > 0.001 {
            let raw = (tech_ic_abs / tech_avg_ic) * 0.20; // 基准 0.20
            raw.clamp(0.05, 0.40)
        } else {
            0.15 // 数据不足时用中等权重
        };
        let proxy_ic = factors
            .get(*tech_name)
            .and_then(|f| f["ic"].as_f64())
            .unwrap_or(0.0);
        factors.insert(
            proxy_name.to_string(),
            json!({
                "ic": (proxy_ic * 100.0).round() / 100.0,
                "weight": (proxy_weight * 100.0).round() / 100.0,
                "samples": factors.get(*tech_name).and_then(|f| f["samples"].as_u64()).unwrap_or(0),
                "proxy_source": *tech_name,
            }),
        );
    }

    json!({"factors": factors, "windows": all_sigs.len(), "forecast_days": forecast})
}

// ── P2: 市场状态分类 ──

/// 基于 K 线判断市场状态，输出先验概率。
/// P1-3 统一: 委托给 axagent_astock_data::regime::RegimeDetector::detect（计分制高级实现），
///           替代原内联的简单均线排列匹配，获得连涨连跌/布林带/年化波动率等额外维度。
/// V38 修复（已迁移）: 增加波动率维度；调整 prior 更接近 A 股现实。
/// 保持输出格式向后兼容（state/prior/reason/ma_alignment/volatility/vol_value）。
fn compute_market_regime(klines: &[KLine]) -> Value {
    use axagent_astock_data::regime::RegimeDetector;
    let n = klines.len();
    if n < 20 {
        return json!({"state":"neutral","prior":0.50,"reason":"数据不足"});
    }
    let report = RegimeDetector::detect(klines);
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();

    // 均线排列
    let ma_alignment = if let (Some(ma20), Some(ma60)) = (report.ma20, report.ma60) {
        if ma20 > ma60 * 1.02 { "多头排列" }
        else if ma60 > ma20 * 1.02 { "空头排列" }
        else { "缠绕/交叉" }
    } else { "数据不足" };

    // state: RegimeDetector 的枚举映射 + 波动率修正
    let vol_pct = report.volatility_20d.unwrap_or(30.0);
    let vol_str = if vol_pct > 50.0 { "high" } else if vol_pct < 15.0 { "low" } else { "normal" };

    let (state, prior, reason) = match report.regime {
        MarketRegime::Bull => {
            if vol_pct > 50.0 {
                ("bull_high_vol", 0.60, "上升趋势（高波动预警）")
            } else {
                ("bull", 0.65, "中长期上升趋势")
            }
        },
        MarketRegime::Bear => {
            if vol_pct > 50.0 {
                ("bear_high_vol", 0.40, "下降趋势（高波动加速）")
            } else {
                ("bear", 0.40, "中长期下降趋势")
            }
        },
        MarketRegime::Sideways => ("neutral", 0.50, "震荡趋势"),
        MarketRegime::Volatile => ("neutral", 0.50, "高波动环境，方向不明确"),
        MarketRegime::Unknown => ("neutral", 0.50, "数据不足"),
    };

    json!({"state": state, "prior": prior, "reason": reason,
           "ma_alignment": ma_alignment, "volatility": vol_str,
           "vol_value": (vol_pct*100.0).round()/100.0,
           "triggered_rules": report.triggered_rules,
           "confidence": report.confidence})
}

/// 因子回测工具——用历史 K 线回测各技术因子的预测能力。
///
/// 工作原理：
/// 1. 对 120 日 K 线进行滑动窗口分析
/// 2. 每个时间窗口计算因子信号 + 后续 N 日涨跌幅
/// 3. 统计每个因子的胜率、信息系数(IC)
/// 4. 输出数据驱动的因子权重 + 市场状态分类
///
/// 输出结构：
/// ```json
/// {
///   "factors": {
///     "trend":      { "win_rate": 0.55, "ic": 0.12, "weight": 0.25 },
///     "macd":       { "win_rate": 0.52, "ic": 0.08, "weight": 0.20 },
///     "volume":     { ... },
///     "rsi":        { ... },
///     "bias":       { ... },
///   },
///   "market_regime": { "state": "bull", "prior": 0.55, "reason": "上升趋势" },
///   "backtest": { "windows": 85, "forecast_days": 5, "avg_return": 0.002 }
/// }
/// ```
pub struct FactorBacktestTool {
    pub client: Arc<AStockClient>,
}
impl FactorBacktestTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for FactorBacktestTool {
    fn name(&self) -> &str {
        "compute_factor_backtest"
    }
    fn description(&self) -> &str {
        "因子回测：用历史K线回测各因子预测能力，输出胜率/IC/权重 + 市场状态"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
            "stock_code":{"type":"string","description":"6位股票代码"},
            "forecast_days":{"type":"integer","description":"预测天数(默认5)","default":5},
            "lookback_days":{"type":"integer","description":"因子计算回溯天数(默认60)","default":60}
        },"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let forecast = input
            .get("forecast_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .clamp(1, 60) as usize;
        let lookback = input
            .get("lookback_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(60)
            .clamp(20, 120) as usize;

        let klines = self
            .client
            .get_klines(code, "daily", 120)
            .await
            .map_err(|e| te(e.to_string()))?;
        if klines.len() < lookback + forecast + 5 {
            return Ok(ToolResult::success(format!(
                r#"{{"error":"K线数据不足，需要至少{}条"}}"#,
                lookback + forecast + 5
            )));
        }

        // 提取收盘价序列
        let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
        let n = closes.len();
        let mut signals: Vec<(Vec<f64>, f64)> = Vec::new(); // (因子信号向量, forward_return)

        // 对每个有效时间窗口计算信号
        for t in lookback..n - forecast {
            let window = &klines[t - lookback..t];
            let ind = axagent_astock_data::indicators::compute_indicators(code, window);
            let forward_ret = (closes[t + forecast] - closes[t]) / closes[t];

            // 5 个因子信号
            let trend_sig = match ind.ma_alignment.as_str() {
                "多头排列" => 1.0,
                "弱多头" => 0.5,
                "空头排列" => -1.0,
                "弱空头-ish" => -0.5,
                _ => 0.0,
            };
            let macd_sig = match ind.macd_signal.as_str() {
                "金叉" => 1.0,
                "多头运行" => 0.5,
                "死叉" => -1.0,
                "空头运行" => -0.5,
                _ => 0.0,
            };
            let vol_sig = match ind.volume_signal.as_str() {
                "放量突破" | "放量上涨" => 1.0,
                "缩量回调" => 0.3,
                "放量下跌" => -1.0,
                _ => 0.0,
            };
            let rsi_sig = if ind.rsi6 > 70.0 {
                -0.3
            } else if ind.rsi6 > 60.0 {
                0.3
            } else if ind.rsi6 > 40.0 {
                0.5
            } else if ind.rsi6 > 30.0 {
                0.2
            } else {
                -0.5
            };
            let bias_sig = if ind.bias_ma5 > 5.0 {
                -0.5
            }
            // 乖离过大→回调
            else if ind.bias_ma5 > 2.0 {
                0.3
            } else if ind.bias_ma5 > -2.0 {
                0.5
            }
            // 合理区间
            else if ind.bias_ma5 > -5.0 {
                0.2
            } else {
                0.5
            }; // 超跌反弹

            signals.push((vec![trend_sig, macd_sig, vol_sig, rsi_sig, bias_sig], forward_ret));
        }

        // 统计每个因子的预测能力
        fn compute_stats(signals: &[(Vec<f64>, f64)], factor_idx: usize) -> Value {
            let mut correct = 0usize;
            let mut total = 0usize;
            let mut sig_vals: Vec<f64> = Vec::new();
            let mut ret_vals: Vec<f64> = Vec::new();
            for (sigs, ret) in signals {
                let sig = sigs[factor_idx];
                if sig.abs() < 0.01 {
                    continue;
                } // 忽略中性信号
                total += 1;
                if (sig > 0.0 && *ret > 0.0) || (sig < 0.0 && *ret < 0.0) {
                    correct += 1;
                }
                sig_vals.push(sig);
                ret_vals.push(*ret);
            }
            let win_rate = if total > 0 {
                correct as f64 / total as f64
            } else {
                0.5
            };
            // V38 修复: 改用 Pearson 相关系数作为 IC，旧实现 (wr-0.5)*2.0 只是胜率线性变换
            let ic = pearson_ic(&sig_vals, &ret_vals);
            json!({"win_rate": (win_rate * 1000.0).round() / 1000.0,
                   "ic": (ic * 100.0).round() / 100.0,
                   "samples": total})
        }

        let factor_names = ["trend", "macd", "volume", "rsi", "bias"];
        let mut factor_results: Vec<Value> = Vec::new();
        let mut total_abs_ic = 0.0_f64;
        for (i, name) in factor_names.iter().enumerate() {
            let stats = compute_stats(&signals, i);
            let ic = stats["ic"].as_f64().unwrap_or(0.0).abs();
            total_abs_ic += ic;
            factor_results.push(json!({ "name": name, "stats": stats }));
        }
        // 计算归一化权重
        let mut factors_map = serde_json::Map::new();
        for (i, name) in factor_names.iter().enumerate() {
            let stats = &factor_results[i]["stats"];
            let ic_abs = stats["ic"].as_f64().unwrap_or(0.0).abs();
            let weight = if total_abs_ic > 0.01 {
                ic_abs / total_abs_ic
            } else {
                0.2
            };
            let mut entry = stats.clone();
            entry
                .as_object_mut()
                .unwrap()
                .insert("weight".into(), json!((weight * 100.0).round() / 100.0));
            factors_map.insert((*name).to_string(), entry);
        }

        // 市场状态分类：用最近 20 日趋势信号 + 波动率
        let trend_sig_vals: Vec<f64> = signals.iter().map(|(s, _)| s[0]).collect();
        let recent_trend = trend_sig_vals
            .iter()
            .rev()
            .take(20)
            .filter(|&&s| s > 0.0)
            .count();
        let recent_trend_neg = trend_sig_vals
            .iter()
            .rev()
            .take(20)
            .filter(|&&s| s < 0.0)
            .count();
        // 波动率：最近 20 个 forward_return 的标准差
        let recent_rets: Vec<f64> = signals.iter().rev().take(20).map(|(_, r)| *r).collect();
        let mean_r = recent_rets.iter().sum::<f64>() / recent_rets.len().max(1) as f64;
        let variance = recent_rets
            .iter()
            .map(|r| (r - mean_r).powi(2))
            .sum::<f64>()
            / recent_rets.len().max(1) as f64;
        let vol_std = variance.sqrt();
        let (regime, prior, reason) = if recent_trend >= 14 {
            if vol_std > 0.04 {
                ("bull", 0.60, "上升趋势（高波动预警）")
            } else {
                ("bull", 0.65, "上升趋势明显")
            }
        } else if recent_trend_neg >= 14 {
            if vol_std > 0.04 {
                ("bear", 0.38, "下降趋势加速")
            } else {
                ("bear", 0.40, "下降趋势明显")
            }
        } else {
            ("neutral", 0.50, "震荡趋势")
        };

        let result = json!({
            "stock_code": code,
            "factors": factors_map,
            "market_regime": { "state": regime, "prior": prior, "reason": reason,
                "recent_bull_windows": format!("{}/20", recent_trend) },
            "backtest": { "windows": signals.len(), "forecast_days": forecast,
                "lookback_days": lookback },
        });
        Ok(ToolResult::success(serde_json::to_string(&result).unwrap_or_default()))
    }
}

pub struct ComputeValuationTool {
    pub client: Arc<AStockClient>,
}
impl ComputeValuationTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeValuationTool {
    fn name(&self) -> &str {
        "compute_valuation"
    }
    fn description(&self) -> &str {
        "DCF两阶段+格雷厄姆+F-Score+护城河量化估值"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let (quote_r, financials_r) =
            tokio::join!(self.client.get_quote(code), self.client.get_financials(code));
        let quote = quote_r.map_err(|e| te(e.to_string()))?;
        let financials = financials_r.map_err(|e| te(e.to_string()))?;

        let latest_fin = financials.first();
        let eps = latest_fin.and_then(|f| f.eps).unwrap_or(0.0);
        let bps = latest_fin.and_then(|f| f.bps).unwrap_or(0.0);
        let roe = latest_fin.and_then(|f| f.roe).unwrap_or(0.0);
        let debt_ratio = latest_fin.and_then(|f| f.debt_ratio).unwrap_or(0.0);
        let gross_margin = latest_fin.and_then(|f| f.gross_margin).unwrap_or(0.0);
        let net_margin = latest_fin.and_then(|f| f.net_margin).unwrap_or(0.0);
        let revenue_yoy = latest_fin.and_then(|f| f.revenue_yoy).unwrap_or(0.0);
        let profit_yoy = latest_fin.and_then(|f| f.profit_yoy).unwrap_or(0.0);

        // ── 从 _template_vars 读取估值配置参数 ──
        let discount_rate = tv_f64(&input, "value_dcf_discount_rate", 10.0) / 100.0;
        let growth_stable = tv_f64(&input, "value_dcf_perpetual_rate", 3.0) / 100.0;
        let dcf_growth_rate = tv_f64(&input, "value_dcf_growth_rate", 8.0) / 100.0;
        let high_years = 5.0;
        let terminal_multiple = 15.0;

        let mut dcf_value: f64 = 0.0;
        if eps > 0.0 {
            let mut pv: f64 = 0.0;
            for year in 1..=(high_years as i32) {
                let projected_eps = eps * (1.0_f64 + dcf_growth_rate).powi(year);
                pv += projected_eps / (1.0_f64 + discount_rate).powi(year);
            }
            let terminal_eps = eps
                * (1.0_f64 + dcf_growth_rate).powi(high_years as i32)
                * (1.0_f64 + growth_stable);
            let terminal_value = terminal_eps * terminal_multiple
                / (1.0_f64 + discount_rate).powi(high_years as i32);
            dcf_value = pv + terminal_value;
        }

        let graham_value = if eps > 0.0 && bps > 0.0 {
            (22.5 * eps * bps).sqrt()
        } else {
            0.0
        };

        // F-Score: 从 _template_vars 读取阈值
        let fscore_roe_min = tv_f64(&input, "fscore_roe_min", 0.10);
        let fscore_gross_margin_min = tv_f64(&input, "fscore_gross_margin_min", 0.30);
        let fscore_net_margin_min = tv_f64(&input, "fscore_net_margin_min", 0.10);
        let fscore_debt_max = tv_f64(&input, "fscore_debt_max", 0.60);
        let fscore_pe_max = tv_f64(&input, "fscore_pe_max", 20.0);
        let mut f_score = 0i32;
        if profit_yoy > 0.0 {
            f_score += 1;
        }
        if revenue_yoy > 0.0 {
            f_score += 1;
        }
        if roe > fscore_roe_min {
            f_score += 1;
        }
        if gross_margin > fscore_gross_margin_min {
            f_score += 1;
        }
        if net_margin > fscore_net_margin_min {
            f_score += 1;
        }
        if debt_ratio < fscore_debt_max {
            f_score += 1;
        }
        if debt_ratio > 0.0
            && let Some(prev) = financials.get(1)
            && let Some(prev_dr) = prev.debt_ratio
            && debt_ratio < prev_dr
        {
            f_score += 1;
        }
        if financials.len() >= 2
            && let Some(prev) = financials.get(1)
            && let Some(prev_roe) = prev.roe
            && roe > prev_roe
        {
            f_score += 1;
        }
        if let Some(pe_val) = quote.pe
            && pe_val > 0.0
            && pe_val < fscore_pe_max
        {
            f_score += 1;
        }

        // 护城河量化：从 _template_vars 读取阈值
        let moat_gross_margin_high = tv_f64(&input, "moat_gross_margin_high", 0.40);
        let moat_roe_high = tv_f64(&input, "moat_roe_high", 0.15);
        let moat_roe_med = tv_f64(&input, "moat_roe_med", 0.10);
        let moat_debt_low = tv_f64(&input, "moat_debt_low", 0.40);
        let moat_debt_med = tv_f64(&input, "moat_debt_med", 0.60);
        let mut moat_score = 0i32;
        if gross_margin > moat_gross_margin_high {
            moat_score += 3;
        } else if gross_margin > 0.25 {
            moat_score += 2;
        } else if gross_margin > 0.15 {
            moat_score += 1;
        }
        if roe > moat_roe_high {
            moat_score += 3;
        } else if roe > moat_roe_med {
            moat_score += 2;
        } else if roe > 0.05 {
            moat_score += 1;
        }
        if debt_ratio < moat_debt_low {
            moat_score += 2;
        } else if debt_ratio < moat_debt_med {
            moat_score += 1;
        }
        if profit_yoy > 0.10 {
            moat_score += 2;
        } else if profit_yoy > 0.0 {
            moat_score += 1;
        }

        let moat_label = if moat_score >= 8 {
            "强护城河"
        } else if moat_score >= 5 {
            "中等护城河"
        } else if moat_score >= 3 {
            "弱护城河"
        } else {
            "无护城河"
        };

        let current_price = quote.price;
        let dcf_upside = if dcf_value > 0.0 {
            (dcf_value - current_price) / current_price * 100.0
        } else {
            0.0
        };
        let mut v_warnings: Vec<Value> = Vec::new();
        if eps <= 0.0 {
            v_warnings.push(Value::String("EPS≤0，DCF估值不可靠".into()));
        }
        let fin_count = financials.len();
        if fin_count < 2 {
            v_warnings
                .push(Value::String(format!("财务数据仅{}期，估值模型依赖多期数据", fin_count)));
        }
        let v_freshness = if let Some(lf) = latest_fin {
            if lf
                .report_date
                .contains(&Local::now().format("%Y-%m").to_string())
            {
                "current_quarter"
            } else if let Ok(now) = chrono::NaiveDate::parse_from_str(
                &Local::now().format("%Y-%m-%d").to_string(),
                "%Y-%m-%d",
            ) {
                if let Some(rd) = lf.report_date.strip_suffix("00:00:00") {
                    if let Ok(report_date) =
                        chrono::NaiveDate::parse_from_str(rd.trim(), "%Y-%m-%d")
                    {
                        let days_old = (now - report_date).num_days();
                        if days_old <= 90 {
                            "recent_quarter"
                        } else {
                            "outdated"
                        }
                    } else {
                        "unknown"
                    }
                } else {
                    "unknown"
                }
            } else {
                "unknown"
            }
        } else {
            "no_data"
        };

        let graham_upside = if graham_value > 0.0 {
            (graham_value - current_price) / current_price * 100.0
        } else {
            0.0
        };

        let result = json!({
            "stockCode": code,
            "stockName": quote.name,
            "currentPrice": current_price,
            "dcf": {
                "intrinsicValue": (dcf_value * 100.0).round() / 100.0,
                "upsidePct": (dcf_upside * 100.0).round() / 100.0,  // dcf_upside 已是百分比(25.5=25.5%), 只保留2位小数
                "assumptions": {
                    "eps": eps,
                    "highGrowthRate": dcf_growth_rate,
                    "stableGrowthRate": growth_stable,
                    "highGrowthYears": high_years,
                    "discountRate": discount_rate,
                    "terminalPE": terminal_multiple,
                },
            },
            "graham": {
                "intrinsicValue": (graham_value * 100.0).round() / 100.0,
                "upsidePct": (graham_upside * 100.0).round() / 100.0,  // graham_upside 已是百分比, 只保留2位小数
                "formula": "sqrt(22.5 * EPS * BPS)",
                "eps": eps,
                "bps": bps,
            },
            "fScore": {
                "score": f_score,
                "maxScore": 9,
                "interpretation": if f_score >= 7 { "财务强劲" } else if f_score >= 5 { "财务稳健" } else if f_score >= 3 { "财务一般" } else { "财务薄弱" },
            },
            "moat": {
                "score": moat_score,
                "maxScore": 10,
                "label": moat_label,
                "grossMargin": gross_margin,
                "roe": roe,
                "debtRatio": debt_ratio,
                "profitYoy": profit_yoy,
            },
            "financialsUsed": latest_fin.map(|f| f.report_date.clone()).unwrap_or_default(),
            "credibility": {
                "dataCompleteness": if fin_count >= 2 { 100.0 } else { fin_count as f64 * 50.0 },
                "dataFreshness": v_freshness,
                "source": "eastmoney",
                "warnings": Value::Array(v_warnings)
            },
        });
        Ok(ToolResult::success(serde_json::to_string(&result).unwrap_or_default()))
    }
}

pub struct ComputeRiskTool {
    pub client: Arc<AStockClient>,
}
impl ComputeRiskTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeRiskTool {
    fn name(&self) -> &str {
        "compute_portfolio_risk"
    }
    fn description(&self) -> &str {
        "组合风险指标：集中度/分散度/行业暴露；单股时计算波动率/VaR/最大回撤/夏普比率"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_codes":{"type":"string","description":"逗号分隔的股票代码列表（单只时计算单股风险画像:波动率/VaR/最大回撤/夏普）"},"weights":{"type":"string","description":"逗号分隔的持仓权重(0-1)，不填则等权"}},"required":["stock_codes"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let codes_str = input["stock_codes"].as_str().unwrap_or("");
        let codes: Vec<&str> = codes_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if codes.is_empty() {
            return Err(te("stock_codes不能为空".into()));
        }

        let weights: Vec<f64> = if let Some(w_str) = input["weights"].as_str() {
            let parsed: Vec<f64> = w_str
                .split(',')
                .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
                .collect();
            if parsed.len() == codes.len() {
                parsed
            } else {
                vec![1.0 / codes.len() as f64; codes.len()]
            }
        } else {
            vec![1.0 / codes.len() as f64; codes.len()]
        };

        let mut fetch_tasks = Vec::new();
        for &c in &codes {
            fetch_tasks.push(self.client.get_quote(c));
        }
        let results = futures::future::join_all(fetch_tasks).await;

        let mut positions = Vec::new();
        for (i, res) in results.into_iter().enumerate() {
            match res {
                Ok(q) => positions.push((q, weights.get(i).copied().unwrap_or(0.0))),
                Err(e) => {
                    let code = codes.get(i).unwrap_or(&"");
                    tracing::warn!("获取{}行情失败: {}", code, e);
                },
            }
        }

        if positions.is_empty() {
            return Err(te("所有股票行情获取失败".into()));
        }

        let total_weight: f64 = positions.iter().map(|(_, w)| w).sum();
        let norm_weights: Vec<f64> = if total_weight > 0.0 {
            positions.iter().map(|(_, w)| w / total_weight).collect()
        } else {
            vec![1.0 / positions.len() as f64; positions.len()]
        };

        let hhi: f64 = norm_weights.iter().map(|w| w * w).sum();
        let effective_n = if hhi > 0.0 { 1.0 / hhi } else { 1.0 };

        let mut sector_map: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let sector_tasks: Vec<_> = positions
            .iter()
            .map(|(q, _)| self.client.get_sector_info(&q.code))
            .collect();
        let sector_results = futures::future::join_all(sector_tasks).await;

        for (i, sr) in sector_results.into_iter().enumerate() {
            let sector_name = if let Ok(Some(si)) = sr {
                si.sector_name.clone()
            } else {
                "未知".to_string()
            };
            let w = norm_weights.get(i).copied().unwrap_or(0.0);
            *sector_map.entry(sector_name).or_insert(0.0) += w;
        }

        let max_sector_concentration = sector_map.values().copied().fold(0.0_f64, f64::max);
        let max_sector_name = sector_map
            .iter()
            .max_by_key(|(_, v)| (*v * 10000.0) as i64)
            .map(|(k, _)| k.clone())
            .unwrap_or_default();

        // 组合风控阈值：从 _template_vars 读取
        let hhi_concentrated = tv_f64(&input, "risk_hhi_concentrated", 0.25);
        let hhi_medium = tv_f64(&input, "risk_hhi_medium", 0.15);
        let divers_high = tv_f64(&input, "risk_divers_high", 8.0);
        let divers_medium = tv_f64(&input, "risk_divers_medium", 4.0);
        let concentration_label = if hhi > hhi_concentrated {
            "高度集中"
        } else if hhi > hhi_medium {
            "中度集中"
        } else {
            "分散"
        };
        let requested_count = codes.len();
        let loaded_count = positions.len();
        let r_warnings: Vec<Value> = if requested_count > loaded_count {
            vec![Value::String(format!(
                "{}/{} 只股票行情加载失败",
                requested_count - loaded_count,
                requested_count
            ))]
        } else {
            vec![]
        };

        let diversification_label = if effective_n >= divers_high {
            "充分分散"
        } else if effective_n >= divers_medium {
            "适度分散"
        } else {
            "集中风险"
        };

        let result = if codes.len() == 1 {
            // ── V50: 单股模式 — 计算真实风险指标 ──
            // 当只有一只股票时，组合级指标（HHI/分散度/行业暴露）无意义，
            // 改为计算波动率、VaR、最大回撤、夏普比率等单股风险指标。
            let stock_code = codes[0];
            let klines = self
                .client
                .get_klines(stock_code, "daily", 252)
                .await
                .unwrap_or_default();
            let prices: Vec<f64> = klines
                .iter()
                .map(|k| k.close)
                .filter(|&p| p > 0.0)
                .collect();

            // 获取财务数据（用于基本面风险维度）
            let financials = self
                .client
                .get_financials(stock_code)
                .await
                .unwrap_or_default();
            let latest_financial = financials.first(); // 取最新一期财报
            let roe_ttm = latest_financial.and_then(|f| f.roe).unwrap_or(0.0);
            let gross_margin_pct = latest_financial.and_then(|f| f.gross_margin).unwrap_or(0.0);
            let debt_ratio_pct = latest_financial.and_then(|f| f.debt_ratio).unwrap_or(0.0);
            let revenue_growth_yoy = latest_financial.and_then(|f| f.revenue_yoy).unwrap_or(0.0);

            // PE(TTM) 从实时行情获取（StockQuote.pe 字段）
            let pe_ttm = positions
                .first()
                .map(|(q, _)| q.pe.unwrap_or(0.0))
                .unwrap_or(0.0);

            let (annual_vol, var_95, max_dd, sharpe) = if prices.len() >= 20 {
                // 日收益率
                let returns: Vec<f64> = prices
                    .windows(2)
                    .map(|w| (w[1] - w[0]) / w[0])
                    .filter(|r| r.is_finite())
                    .collect();

                // 年化波动率
                let n = returns.len() as f64;
                let mean = returns.iter().sum::<f64>() / n;
                let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
                let daily_vol = variance.sqrt();
                let annual_vol = daily_vol * 252.0_f64.sqrt();

                // VaR 95%（历史模拟法）
                let mut sorted = returns.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let idx = ((1.0 - 0.95) * n).ceil() as usize;
                let var_95 = sorted
                    .get(idx.min(sorted.len() - 1))
                    .copied()
                    .unwrap_or(0.0);

                // 最大回撤
                let mut peak = prices[0];
                let mut max_dd: f64 = 0.0;
                for &p in &prices {
                    if p > peak {
                        peak = p;
                    }
                    if peak > 0.0 {
                        max_dd = max_dd.max((peak - p) / peak);
                    }
                }

                // 夏普比率（年化，无风险利率假设 2%）
                let risk_free_annual = 0.02;
                let risk_free_daily = risk_free_annual / 252.0;
                let excess_mean = returns.iter().map(|r| r - risk_free_daily).sum::<f64>() / n;
                let sharpe = if daily_vol > 0.0 {
                    excess_mean / daily_vol * 252.0_f64.sqrt()
                } else {
                    0.0
                };

                (
                    (annual_vol * 100.0 * 100.0).round() / 100.0, // 百分比
                    (var_95 * 100.0 * 100.0).round() / 100.0,     // 百分比
                    (max_dd * 100.0 * 100.0).round() / 100.0,     // 百分比
                    (sharpe * 100.0).round() / 100.0,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            let stock_name: &str = positions
                .first()
                .map(|(q, _)| q.name.as_str())
                .unwrap_or("");

            json!({
                "stockCount": 1,
                "singleStockRisk": true,
                "stock": { "code": stock_code, "name": stock_name, "price": positions.first().map(|(q, _)| q.price).unwrap_or(0.0) },
                "stockRiskProfile": {
                    "annualizedVolatilityPct": annual_vol,
                    "valueAtRisk95Pct": var_95,
                    "maxDrawdownPct": max_dd,
                    "sharpeRatio": sharpe,
                    "dataDays": prices.len(),
                    "klineDays": klines.len(),
                    "roeTTMPct": (roe_ttm * 100.0).round() / 100.0,
                    "grossMarginPct": (gross_margin_pct * 100.0).round() / 100.0,
                    "debtRatioPct": (debt_ratio_pct * 100.0).round() / 100.0,
                    "revenueGrowthYoYPct": (revenue_growth_yoy * 100.0).round() / 100.0,
                    "peTTM": pe_ttm,
                },
                "concentration": { "hhi": 1.0, "effectiveN": 1.0, "label": "单股(组合指标不适用)" },
                "diversification": { "effectiveStocks": 1.0, "label": "单股" },
                "sectorExposure": sector_map.into_iter().map(|(k, v)| json!({"sector": k, "weight": (v * 100.0).round() / 100.0})).collect::<Vec<_>>(),
                "maxSectorConcentration": { "sector": max_sector_name, "weightPct": (max_sector_concentration * 100.0).round() / 100.0 },
                "positions": positions.iter().zip(norm_weights.iter()).map(|((q, _), w)| json!({
                    "code": q.code, "name": q.name, "price": q.price,
                    "changePct": q.change_pct, "weightPct": (*w * 100.0).round() / 100.0,
                })).collect::<Vec<_>>(),
                "credibility": { "dataCompleteness": 100.0, "dataFreshness": "realtime", "source": "tencent|eastmoney", "warnings": Value::Array(r_warnings) }
            })
        } else {
            // ── 多股模式 — 原有组合风险指标 ──
            json!({
                "stockCount": positions.len(),
                "singleStockRisk": false,
                "concentration": {
                    "hhi": (hhi * 10000.0).round() / 10000.0,
                    "effectiveN": (effective_n * 100.0).round() / 100.0,
                    "label": concentration_label,
                },
                "diversification": {
                    "effectiveStocks": (effective_n * 100.0).round() / 100.0,
                    "label": diversification_label,
                },
                "sectorExposure": sector_map.into_iter().map(|(k, v)| json!({"sector": k, "weight": (v * 100.0).round() / 100.0})).collect::<Vec<_>>(),
                "maxSectorConcentration": {
                    "sector": max_sector_name,
                    "weightPct": (max_sector_concentration * 100.0).round() / 100.0,
                },
                "positions": positions.iter().zip(norm_weights.iter()).map(|((q, _), w)| json!({
                    "code": q.code, "name": q.name, "price": q.price,
                    "changePct": q.change_pct, "weightPct": (*w * 100.0).round() / 100.0,
                })).collect::<Vec<_>>(),
                "credibility": {
                    "dataCompleteness": if requested_count > 0 { (loaded_count as f64 / requested_count as f64) * 100.0 } else { 0.0 },
                    "dataFreshness": "realtime",
                    "source": "tencent|eastmoney",
                    "warnings": Value::Array(r_warnings)
                },
            })
        };
        Ok(ToolResult::success(serde_json::to_string(&result).unwrap_or_default()))
    }
}

// ── 14. StockBlockTradesTool ──
pub struct StockBlockTradesTool {
    pub client: Arc<AStockClient>,
}
impl StockBlockTradesTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockBlockTradesTool {
    fn name(&self) -> &str {
        "get_block_trades"
    }
    fn description(&self) -> &str {
        "获取A股大宗交易记录：成交价、成交量、买卖双方营业部、折价率"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_block_trades(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 15. StockInstitutionalVisitsTool ──
pub struct StockInstitutionalVisitsTool {
    pub client: Arc<AStockClient>,
}
impl StockInstitutionalVisitsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockInstitutionalVisitsTool {
    fn name(&self) -> &str {
        "get_institutional_visits"
    }
    fn description(&self) -> &str {
        "获取A股机构调研记录：调研日期、机构数量、调研内容、调研方式"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_institutional_visits(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 16. StockPeersTool ──
pub struct StockPeersTool {
    pub client: Arc<AStockClient>,
}
impl StockPeersTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockPeersTool {
    fn name(&self) -> &str {
        "get_stock_peers"
    }
    fn description(&self) -> &str {
        "获取同行业可比公司估值（PE/PB/ROE/涨跌幅/市值）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_peers(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 17. StockResearchReportsTool ──
pub struct StockResearchReportsTool {
    pub client: Arc<AStockClient>,
}
impl StockResearchReportsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockResearchReportsTool {
    fn name(&self) -> &str {
        "get_research_reports"
    }
    // [BUGFIX] 工作流 t-research-data 节点 (stock_analysis_setup.rs:1341) 引用
    // 工具名 "get_stock_research_reports",而本工具主名是 "get_research_reports" ——
    // 不加 alias 会导致 tool_registry.find() 报 "工具未找到"。加 alias 后两个名字
    // 都能找到工具,MCP 层 (astock-data/src/mcp_tools.rs) 也用 "get_stock_research_reports"
    // 保持一致。
    fn aliases(&self) -> &[&str] {
        &["get_stock_research_reports"]
    }
    fn description(&self) -> &str {
        "获取券商研报列表（机构、评级、目标价、EPS预测）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_research_reports(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 18. StockConceptBlocksTool ──
pub struct StockConceptBlocksTool {
    pub client: Arc<AStockClient>,
}
impl StockConceptBlocksTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockConceptBlocksTool {
    fn name(&self) -> &str {
        "get_stock_concept_blocks"
    }
    fn description(&self) -> &str {
        "获取概念板块三维归属（行业/概念/地域）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_concept_blocks(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 19. StockOptionPCRTool ──
pub struct StockOptionPCRTool {
    pub client: Arc<AStockClient>,
}
impl StockOptionPCRTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockOptionPCRTool {
    fn name(&self) -> &str {
        "get_stock_option_pcr"
    }
    fn description(&self) -> &str {
        "获取期权PCR（看跌/看涨成交量和持仓量比率，市场情绪前瞻指标）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_option_pcr(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 20. StockCLSFlashTool ──
pub struct StockCLSFlashTool {
    pub client: Arc<AStockClient>,
}
impl StockCLSFlashTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockCLSFlashTool {
    fn name(&self) -> &str {
        "get_cls_flash"
    }
    fn description(&self) -> &str {
        "获取财联社实时快讯（分钟级电报）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let r = self
            .client
            .get_cls_flash()
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 21. StockNorthBoundFlowTool ──
pub struct StockNorthBoundFlowTool {
    pub client: Arc<AStockClient>,
}
impl StockNorthBoundFlowTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockNorthBoundFlowTool {
    fn name(&self) -> &str {
        "get_north_bound_flow"
    }
    fn description(&self) -> &str {
        "获取北向资金分钟级流向（沪深股通）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let r = self
            .client
            .get_north_bound_flow()
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 22. StockMarketDragonTigerTool ──
pub struct StockMarketDragonTigerTool {
    pub client: Arc<AStockClient>,
}
impl StockMarketDragonTigerTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockMarketDragonTigerTool {
    fn name(&self) -> &str {
        "get_market_dragon_tiger"
    }
    fn description(&self) -> &str {
        "获取全市场龙虎榜（每日上榜股票+净买额排名）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let r = self
            .client
            .get_market_dragon_tiger()
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 23. StockIndexQuotesTool ──
pub struct StockIndexQuotesTool {
    pub client: Arc<AStockClient>,
}
impl StockIndexQuotesTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockIndexQuotesTool {
    fn name(&self) -> &str {
        "get_index_quotes"
    }
    fn description(&self) -> &str {
        "获取大盘指数行情（上证指数、深证成指、创业板指）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let r = self
            .client
            .get_index_quotes()
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 24. StockLockupTool ──
pub struct StockLockupTool {
    pub client: Arc<AStockClient>,
}
impl StockLockupTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockLockupTool {
    fn name(&self) -> &str {
        "get_stock_lockup"
    }
    fn description(&self) -> &str {
        "获取限售解禁日程（解禁日期、股数、比例、股东名称）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_lockup_schedule(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

/// 筹码面分析数据聚合工具：一次调用返回解禁 + 增减持 + 大宗交易
pub struct StockLockupBundleTool {
    pub client: Arc<AStockClient>,
}
impl StockLockupBundleTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockLockupBundleTool {
    fn name(&self) -> &str {
        "get_stock_lockup_bundle"
    }
    fn description(&self) -> &str {
        "获取筹码面分析数据（解禁 + 增减持 + 大宗交易三方聚合）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_lockup_bundle(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(r.to_string()))
    }
}

// ── 25. StockShareholderTradesTool ──
pub struct StockShareholderTradesTool {
    pub client: Arc<AStockClient>,
}
impl StockShareholderTradesTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockShareholderTradesTool {
    fn name(&self) -> &str {
        "get_stock_shareholder_trades"
    }
    fn description(&self) -> &str {
        "获取大股东增减持记录（变动类型、数量、均价、原因）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_shareholder_trades(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 26. StockDividendRecordsTool ──
pub struct StockDividendRecordsTool {
    pub client: Arc<AStockClient>,
}
impl StockDividendRecordsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockDividendRecordsTool {
    fn name(&self) -> &str {
        "get_stock_dividend_records"
    }
    fn description(&self) -> &str {
        "获取除权除息/分红送配记录"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_dividend_records(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 27. StockNorthBoundHoldingTool ──
pub struct StockNorthBoundHoldingTool {
    pub client: Arc<AStockClient>,
}
impl StockNorthBoundHoldingTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockNorthBoundHoldingTool {
    fn name(&self) -> &str {
        "get_stock_north_bound"
    }
    fn description(&self) -> &str {
        "获取北向资金个股持仓（持股数量、占比）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_north_bound_holding(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 28. StockDragonTigerTool ──
pub struct StockDragonTigerTool {
    pub client: Arc<AStockClient>,
}
impl StockDragonTigerTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockDragonTigerTool {
    fn name(&self) -> &str {
        "get_stock_dragon_tiger"
    }
    fn description(&self) -> &str {
        "获取个股龙虎榜数据（营业部买卖、上榜原因）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_dragon_tiger(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 29. StockMarginDataTool ──
pub struct StockMarginDataTool {
    pub client: Arc<AStockClient>,
}
impl StockMarginDataTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockMarginDataTool {
    fn name(&self) -> &str {
        "get_stock_margin_data"
    }
    fn description(&self) -> &str {
        "获取融资融券数据（融资买入额、余额、融券卖出量、余量）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_margin_data(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 31. AnnouncementContentTool — cninfo PDF 公告全文提取 ──
// P2 新增: 突破标题级别信息局限,从 cninfo 下载公告 PDF 并提取正文文本。
// 在 a-catalyst prompt 中指导 LLM 对可疑标题调用此工具获取全文。
pub struct AnnouncementContentTool {
    pub client: Arc<AStockClient>,
}
impl AnnouncementContentTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for AnnouncementContentTool {
    fn name(&self) -> &str {
        "get_announcement_content"
    }
    fn description(&self) -> &str {
        "获取A股公司公告PDF全文（已提取文本正文，非标题）"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "stock_code": {
                    "type": "string",
                    "description": "6位股票代码"
                },
                "max_items": {
                    "type": "integer",
                    "description": "最多读取最近几条公告的PDF(默认3)",
                    "default": 3
                }
            },
            "required": ["stock_code"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let max_items = input["max_items"].as_i64().unwrap_or(3).clamp(1, 10) as usize;

        // 1. 获取最近公告列表
        let announcements = self
            .client
            .get_announcements(code)
            .await
            .map_err(|e| te(format!("获取公告列表失败: {e}")))?;

        if announcements.is_empty() {
            return Ok(ToolResult::success("暂无公告记录"));
        }

        let mut results: Vec<String> = Vec::new();
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| te(format!("创建HTTP客户端失败: {e}")))?;

        for (i, ann) in announcements.iter().take(max_items).enumerate() {
            let title = &ann.title;
            let date = &ann.announce_date;
            let pdf_url = ann.pdf_url.as_ref();

            results.push(format!("\n===== 公告 {}: [{}] {} =====\n", i + 1, date, title));

            if let Some(url) = pdf_url {
                if url.is_empty() || url.len() < 10 {
                    results.push("  [无PDF链接]".into());
                    continue;
                }
                // 2. 下载PDF
                match http.get(url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let bytes = match resp.bytes().await {
                            Ok(b) => b,
                            Err(e) => {
                                results.push(format!("  [PDF下载失败: {e}]"));
                                continue;
                            },
                        };
                        // 3. 提取文本
                        match pdf_extract::extract_text_from_mem(&bytes) {
                            Ok(text) => {
                                let trimmed = text.trim();
                                // 截断到 4000 字避免 token 过大
                                let max_chars = 4000;
                                let excerpt = if trimmed.len() > max_chars {
                                    format!(
                                        "{}...\n[内容已截断至前{}字]",
                                        &trimmed[..max_chars],
                                        max_chars
                                    )
                                } else {
                                    trimmed.to_string()
                                };
                                if excerpt.is_empty() {
                                    results.push("  [PDF无可提取文本，可能为扫描件]".into());
                                } else {
                                    results.push(excerpt);
                                }
                            },
                            Err(e) => {
                                results.push(format!("  [PDF文本提取失败: {e}]"));
                            },
                        }
                    },
                    Ok(resp) => {
                        results.push(format!("  PDF下载失败, HTTP状态码: {}", resp.status()));
                    },
                    Err(e) => {
                        results.push(format!("  [PDF请求失败: {e}]"));
                    },
                }
            } else {
                results.push("  [无PDF链接]".into());
            }
        }

        Ok(ToolResult::success(results.join("\n")))
    }
}

// ── 30. StockSectorInfoTool ──
pub struct StockSectorInfoTool {
    pub client: Arc<AStockClient>,
}
impl StockSectorInfoTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockSectorInfoTool {
    fn name(&self) -> &str {
        "get_stock_sector_info"
    }
    fn description(&self) -> &str {
        "获取行业分类（申万一级/二级、概念板块标签）"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let r = self
            .client
            .get_sector_info(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string(&r).unwrap_or_default()))
    }
}

// ── 30. ComputeAttentionScoreTool ──
// 方案B: 综合多信号计算个股关注度评分（0=冷门 100=过热）
// 权重可通过 input.weights 覆盖，用于回测优化
pub struct ComputeAttentionScoreTool {
    pub client: Arc<AStockClient>,
}
impl ComputeAttentionScoreTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeAttentionScoreTool {
    fn name(&self) -> &str {
        "compute_attention_score"
    }
    fn description(&self) -> &str {
        "计算个股关注度评分 0-100（基于新闻量/研报/机构调研/换手率），越低越冷门"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
        "stock_code":{"type":"string","description":"6位股票代码"},
        "weights":{"type":"object","description":"可选权重覆盖，默认0.25/0.20/0.25/0.15/0.15","properties":{
            "turnover":{"type":"number","description":"换手率权重"},
            "news":{"type":"number","description":"新闻量权重"},
            "report":{"type":"number","description":"研报量权重"},
            "visit":{"type":"number","description":"机构调研权重"},
            "mcap":{"type":"number","description":"市值权重"}
        }}
    },"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;

        // 读取权重（支持覆盖，默认 = 回测初值）
        let w = |key: &str, def: f64| -> f64 {
            input["weights"][key]
                .as_f64()
                .unwrap_or(def)
                .clamp(0.0, 1.0)
        };
        let w_turnover = w("turnover", 0.25);
        let w_news = w("news", 0.20);
        let w_report = w("report", 0.25);
        let w_visit = w("visit", 0.15);
        let w_mcap = w("mcap", 0.15);
        // 归一化到总和=1.0
        let total = w_turnover + w_news + w_report + w_visit + w_mcap;
        let (w_turnover, w_news, w_report, w_visit, w_mcap) = if total > 0.0 {
            (
                w_turnover / total,
                w_news / total,
                w_report / total,
                w_visit / total,
                w_mcap / total,
            )
        } else {
            (0.25, 0.20, 0.25, 0.15, 0.15)
        };

        // 1. 行情数据（换手率、市值）
        let quote = self
            .client
            .get_quote(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        let turnover_rate = quote.turnover_rate;
        let market_cap = quote.total_mv.unwrap_or(0.0);

        // 2. 新闻量（近30条）
        let news_count = self
            .client
            .get_news(code, 10)
            .await
            .map(|v| v.len() as f64)
            .unwrap_or(0.0);

        // 3. 研报量
        let report_count = self
            .client
            .get_research_reports(code)
            .await
            .map(|v| v.len() as f64)
            .unwrap_or(0.0);

        // 4. 机构调研
        let visit_count = self
            .client
            .get_institutional_visits(code)
            .await
            .map(|v| v.len() as f64)
            .unwrap_or(0.0);

        // 5. 计算单项分（0-100）
        let score_turnover = (turnover_rate / 10.0 * 100.0).min(100.0);
        let score_news = (news_count / 5.0 * 100.0).min(100.0);
        let score_report = (report_count / 5.0 * 100.0).min(100.0);
        let score_visit = (visit_count / 3.0 * 100.0).min(100.0);
        let score_mcap = if market_cap > 0.0 {
            ((market_cap / 500.0) * 100.0).min(100.0)
        } else {
            50.0
        };

        let attention_score = (score_turnover * w_turnover
            + score_news * w_news
            + score_report * w_report
            + score_visit * w_visit
            + score_mcap * w_mcap) as u32;

        let result = serde_json::json!({
            "stock_code": code,
            "attention_score": attention_score.min(100),
            "detail": {
                "turnover_rate_pct": turnover_rate,
                "news_count_30d": news_count,
                "research_report_count": report_count,
                "institutional_visit_count": visit_count,
                "market_cap_billion": market_cap,
                "heat_label": if attention_score <= 30 { "冷门" } else if attention_score <= 60 { "正常" } else { "热门" }
            },
            "weights_used": {
                "turnover": w_turnover,
                "news": w_news,
                "report": w_report,
                "visit": w_visit,
                "mcap": w_mcap,
                "note": "可通过 input.weights 覆盖。回测时调用方可自动调优"
            }
        });
        Ok(ToolResult::success(result.to_string()))
    }
}

// ── 31. ComputeIndustryPositionTool ──
// 方案C: 行业竞争地位分析（同行对比 + 产能指标 + 排名）
pub struct ComputeIndustryPositionTool {
    pub client: Arc<AStockClient>,
}
impl ComputeIndustryPositionTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeIndustryPositionTool {
    fn name(&self) -> &str {
        "compute_industry_position"
    }
    fn description(&self) -> &str {
        "行业竞争地位分析：同行对比毛利率/ROE/负债率，计算产能指标（固定资产周转率/资本开支比）和排名"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;

        // 1. 获取行业信息
        let sector_info = self.client.get_sector_info(code).await.ok().flatten();

        // 2. 获取同行对比
        let peers = self.client.get_peers(code).await.unwrap_or_default();

        // 3. 获取财务数据
        let financials = self.client.get_financials(code).await.ok();
        let latest = financials.as_ref().and_then(|f| f.first());

        // 4. 计算产能/竞争指标
        let gm = latest.and_then(|f| f.gross_margin).unwrap_or(0.0);
        let roe = latest.and_then(|f| f.roe).unwrap_or(0.0);
        let debt_ratio = latest.and_then(|f| f.debt_ratio).unwrap_or(0.0);
        let rnd_ratio = latest.and_then(|f| f.net_margin).unwrap_or(0.0); // net_margin 作为研发密度近似
        let capex = latest.and_then(|f| f.capital_expenditure).unwrap_or(0.0);
        let ocf = latest.and_then(|f| f.operating_cash_flow).unwrap_or(0.0);
        let revenue = latest.and_then(|f| f.revenue).unwrap_or(0.0);

        // 资本开支/折旧：用 operating_cash_flow 近似折旧
        let capex_dep_ratio = if ocf > 0.0 {
            (capex / ocf).round()
        } else {
            0.0
        };

        // 同行对比排名
        let mut peer_gms: Vec<f64> = peers.iter().filter_map(|p| p.roe).collect();
        peer_gms.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let gm_rank = peer_gms
            .iter()
            .position(|&v| v <= roe)
            .map(|i| i + 1)
            .unwrap_or(0);

        let result = serde_json::json!({
            "stock_code": code,
            "sector": sector_info.as_ref().map(|s| s.sector_name.clone()).unwrap_or_default(),
            "sub_sector": sector_info.as_ref().map(|s| s.sub_sector.clone()).unwrap_or_default(),
            "competitive_position": {
                "gross_margin_pct": gm,
                "roe_pct": roe,
                "debt_ratio_pct": debt_ratio,
                "rnd_intensity": rnd_ratio,
                "gm_rank_in_peers": gm_rank,
                "total_peer_count": peers.len(),
            },
            "capacity_indicators": {
                "capex_depreciation_ratio": capex_dep_ratio,
                "revenue": revenue,
                "capex": capex,
                "signal": if capex_dep_ratio >= 3.0 {
                    "积极扩产（资本开支/折旧 > 3）"
                } else if capex_dep_ratio >= 1.5 {
                    "温和扩产"
                } else {
                    "维持性投入"
                }
            },
            "peer_summary": peers.iter().take(5).map(|p| {
                serde_json::json!({
                    "stock_code": p.stock_code,
                    "stock_name": p.stock_name,
                    "roe": p.roe,
                    "market_cap": p.market_cap,
                })
            }).collect::<Vec<_>>(),
        });
        Ok(ToolResult::success(result.to_string()))
    }
}

// ── 32. CheckExitSignalsTool ──
// Phase 3: 退出信号持续监控 — 检查个股是否触发退出条件
pub struct CheckExitSignalsTool {
    pub client: Arc<AStockClient>,
}
impl CheckExitSignalsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for CheckExitSignalsTool {
    fn name(&self) -> &str {
        "check_exit_signals"
    }
    fn description(&self) -> &str {
        "检查个股的退出信号：价格止损、技术替代新闻、产能过剩信号。返回 exit_urgency"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
        "stock_code":{"type":"string","description":"6位股票代码"},
        "entry_price":{"type":"number","description":"买入价（用于计算止损触发）"},
        "stop_loss_price":{"type":"number","description":"止损价"}
    },"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let entry_price = input["entry_price"].as_f64();
        let stop_loss_price = input["stop_loss_price"].as_f64();

        // 1. 获取当前行情
        let quote = self
            .client
            .get_quote(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        let price = quote.price;
        let change_pct_1m = quote.change_pct; // 当日涨跌幅（近似）

        // 2. 检查价格止损
        let stop_loss_hit = stop_loss_price.map(|sl| price < sl).unwrap_or(false);

        // 3. 搜索负面新闻
        let disruption_news = self
            .client
            .search_news(&format!("{} 技术替代 产能过剩 竞争", code), 5)
            .await
            .unwrap_or_default();
        let has_disruption_news = disruption_news.len() >= 2;

        // 4. 获取财务趋势（负债率和毛利率变化）
        let financials = self.client.get_financials(code).await.ok();
        let margin_declining = financials
            .as_ref()
            .and_then(|f| {
                if f.len() >= 2 {
                    let curr = f[0].gross_margin.unwrap_or(0.0);
                    let prev = f[1].gross_margin.unwrap_or(0.0);
                    Some(prev > 0.0 && curr < prev * 0.85) // 毛利率下降超过15%
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // 5. 综合判断退出紧迫度
        let (urgency, reasons) = if stop_loss_hit {
            ("exit_now", vec!["止损价已触发".to_string()])
        } else if has_disruption_news && margin_declining {
            ("exit_now", vec!["技术替代/竞争加剧 + 毛利率持续下降".to_string()])
        } else if has_disruption_news {
            ("caution", vec!["检测到技术替代或产能过剩相关新闻".to_string()])
        } else if margin_declining {
            ("caution", vec!["毛利率明显下降，关注竞争格局变化".to_string()])
        } else if entry_price
            .map(|ep| (price - ep) / ep < -0.15)
            .unwrap_or(false)
        {
            (
                "watch",
                vec![format!(
                    "距入场价下跌 {:.1}%，接近止损",
                    (price / entry_price.unwrap_or(price) - 1.0) * 100.0
                )],
            )
        } else {
            ("no_urgency", vec!["退出信号未触发".to_string()])
        };

        let result = serde_json::json!({
            "stock_code": code,
            "current_price": price,
            "change_pct_today": change_pct_1m,
            "stop_loss_hit": stop_loss_hit,
            "has_disruption_news": has_disruption_news,
            "margin_declining": margin_declining,
            "exit_urgency": urgency,
            "reasons": reasons,
            "updated_at": chrono::Utc::now().to_rfc3339()
        });
        Ok(ToolResult::success(result.to_string()))
    }
}

// ── 33. ComputeSerenityPerformanceTool ──
// 回馈闭环: 跟踪 Serenity 候选的推荐后表现
pub struct ComputeSerenityPerformanceTool {
    pub client: Arc<AStockClient>,
}
impl ComputeSerenityPerformanceTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for ComputeSerenityPerformanceTool {
    fn name(&self) -> &str {
        "compute_serenity_performance"
    }
    fn description(&self) -> &str {
        "计算 Serenity 候选股的推荐后表现：从 entry_date 到现在的收益率、最大回撤、波动率"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
        "stock_code":{"type":"string","description":"6位股票代码"},
        "recommend_date":{"type":"string","description":"推荐日期 YYYY-MM-DD"}
    },"required":["stock_code","recommend_date"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let date = input["recommend_date"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("recommend_date不能为空".into()))?;

        // 获取推荐日 K 线（前复权）确定入场价
        let entry_kline = self
            .client
            .get_klines(code, "daily", 30)
            .await
            .map_err(|e| te(e.to_string()))?;
        let entry_price = entry_kline
            .iter()
            .find(|k| k.date.starts_with(date))
            .map(|k| k.close)
            .or_else(|| entry_kline.last().map(|k| k.close))
            .unwrap_or(0.0);

        // 获取当前行情
        let current_quote = self
            .client
            .get_quote(code)
            .await
            .map_err(|e| te(e.to_string()))?;
        let current_price = current_quote.price;

        // 计算收益率
        let return_pct = if entry_price > 0.0 {
            (current_price - entry_price) / entry_price * 100.0
        } else {
            0.0
        };

        // 从 K 线计算最大回撤和波动率
        let max_drawdown: f64 = entry_kline
            .iter()
            .filter(|k| k.date.as_str() >= date)
            .fold((0.0_f64, entry_price), |(max_dd, peak), k| {
                let new_peak = peak.max(k.close);
                let dd = (new_peak - k.close) / new_peak * 100.0;
                (max_dd.max(dd), new_peak)
            })
            .0;
        let volatility = if entry_kline.len() >= 5 {
            let returns: Vec<f64> = entry_kline
                .windows(2)
                .map(|w| (w[1].close - w[0].close) / w[0].close)
                .collect();
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance =
                returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
            variance.sqrt() * 100.0
        } else {
            0.0
        };

        let result = serde_json::json!({
            "stock_code": code,
            "entry_price": entry_price,
            "current_price": current_price,
            "return_pct": (return_pct * 100.0).round() / 100.0,
            "max_drawdown_pct": (max_drawdown * 100.0_f64).round() / 100.0,
            "volatility_pct": (volatility * 100.0_f64).round() / 100.0,
            "days_held": entry_kline.iter().filter(|k| k.date.as_str() >= date).count(),
            "is_profitable": return_pct > 0.0,
        });
        Ok(ToolResult::success(result.to_string()))
    }
}

// ── 34. VerifyCatalystsTool ──
// 回馈闭环: 验证催化剂是否兑现
pub struct VerifyCatalystsTool {
    pub client: Arc<AStockClient>,
}
impl VerifyCatalystsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for VerifyCatalystsTool {
    fn name(&self) -> &str {
        "verify_catalysts"
    }
    fn description(&self) -> &str {
        "验证 Serenity 候选的催化剂是否兑现：搜索新闻确认事件是否发生"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
        "stock_code":{"type":"string","description":"6位股票代码"},
        "catalyst_descriptions":{"type":"array","items":{"type":"string"},"description":"催化剂描述列表"}
    },"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let descriptions: Vec<&str> = input["catalyst_descriptions"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let mut results = Vec::new();
        for desc in &descriptions {
            // 用搜索关键词找催化剂相关的新闻
            let news = self.client.search_news(desc, 5).await.unwrap_or_default();
            let found = news.iter().any(|n| {
                n.title.contains(*desc)
                    || n.summary.contains(*desc)
                    || desc
                        .chars()
                        .all(|c| n.title.contains(c) || n.summary.contains(c))
            });
            results.push(serde_json::json!({
                "description": desc,
                "verified": found,
                "evidence_count": news.len(),
                "top_match": news.first().map(|n| n.title.clone()).unwrap_or_default(),
            }));
        }

        let result = serde_json::json!({
            "stock_code": code,
            "catalysts_checked": results.len(),
            "verified_count": results.iter().filter(|r| r["verified"].as_bool().unwrap_or(false)).count(),
            "details": results,
        });
        Ok(ToolResult::success(result.to_string()))
    }
}

// ── 35. OptimizeAttentionWeightsTool ──
// 回馈闭环: 基于历史表现调优关注度评分权重
pub struct OptimizeAttentionWeightsTool {
    pub client: Arc<AStockClient>,
}
impl OptimizeAttentionWeightsTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for OptimizeAttentionWeightsTool {
    fn name(&self) -> &str {
        "optimize_attention_weights"
    }
    fn description(&self) -> &str {
        "基于历史候选表现调优 compute_attention_score 的权重。输入候选表现列表，输出最优权重组合"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{
        "samples":{"type":"array","description":"样本列表: [{attention_score, return_pct, ...}]","items":{
            "type":"object","properties":{
                "attention_score":{"type":"number"},"return_pct":{"type":"number"},
                "news_count":{"type":"number"},"report_count":{"type":"number"},
                "visit_count":{"type":"number"},"turnover_rate":{"type":"number"},"market_cap":{"type":"number"}
            }
        }}
    },"required":["samples"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let samples = input["samples"]
            .as_array()
            .ok_or_else(|| te("samples不能为空".into()))?;
        if samples.is_empty() {
            return Ok(ToolResult::success(r#"{"error":"样本为空"}"#.to_string()));
        }

        // 提取样本数据
        let scores: Vec<f64> = samples
            .iter()
            .filter_map(|s| s["attention_score"].as_f64())
            .collect();
        let returns: Vec<f64> = samples
            .iter()
            .filter_map(|s| s["return_pct"].as_f64())
            .collect();
        if scores.len() < 5 {
            return Ok(ToolResult::success(format!(
                r#"{{"error":"样本不足，需要至少5个样本","samples_count":{}}}"#,
                samples.len()
            )));
        }

        // 简单网格搜索: 测试 5 组权重维度
        let weight_sets: Vec<(&str, Vec<f64>)> = vec![
            ("默认", vec![0.25, 0.20, 0.25, 0.15, 0.15]),
            ("强趋势", vec![0.35, 0.15, 0.15, 0.10, 0.25]),
            ("弱信号", vec![0.15, 0.25, 0.30, 0.20, 0.10]),
            ("均衡", vec![0.20, 0.20, 0.20, 0.20, 0.20]),
            ("极致冷门", vec![0.15, 0.30, 0.30, 0.20, 0.05]),
        ];

        // 对每个权重组合计算与收益的负相关性（低关注度→高收益是理想的）
        let mut results: Vec<serde_json::Value> = weight_sets
            .iter()
            .map(|(name, ws)| {
                let weighted_scores: Vec<f64> = samples
                    .iter()
                    .map(|s| {
                        let turnover = s["turnover_rate"].as_f64().unwrap_or(0.0);
                        let news = s["news_count"].as_f64().unwrap_or(0.0);
                        let report = s["report_count"].as_f64().unwrap_or(0.0);
                        let visit = s["visit_count"].as_f64().unwrap_or(0.0);
                        let mcap = s["market_cap"].as_f64().unwrap_or(0.0);
                        let score = turnover / 10.0 * ws[0]
                            + news / 5.0 * ws[1]
                            + report / 5.0 * ws[2]
                            + visit / 3.0 * ws[3]
                            + (mcap / 500.0).min(1.0) * ws[4];
                        score * 100.0
                    })
                    .collect();

                // 计算相关系数: 低分数→高收益 = 好的权重
                let n = samples.len() as f64;
                let mean_x = weighted_scores.iter().sum::<f64>() / n;
                let mean_y = returns.iter().sum::<f64>() / n;
                let (num, den_x, den_y) = weighted_scores.iter().zip(returns.iter()).fold(
                    (0.0, 0.0, 0.0),
                    |(n, dx, dy), (&x, &y)| {
                        (
                            n + (x - mean_x) * (y - mean_y),
                            dx + (x - mean_x).powi(2),
                            dy + (y - mean_y).powi(2),
                        )
                    },
                );
                let correlation = if den_x > 0.0 && den_y > 0.0 {
                    num / (den_x.sqrt() * den_y.sqrt())
                } else {
                    0.0
                };

                // 理想的相关性是负的（低分数→高收益）
                let effectiveness = -correlation;

                serde_json::json!({
                    "name": name,
                    "weights": ws,
                    "correlation": (correlation * 100.0).round() / 100.0,
                    "effectiveness": (effectiveness * 100.0).round() / 100.0,
                })
            })
            .collect();

        // 按效果排序
        results.sort_by(|a, b| {
            b["effectiveness"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["effectiveness"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best = results.first();
        Ok(ToolResult::success(
            serde_json::json!({
                "best_weights": best.map(|b| b["weights"].clone()),
                "best_name": best.map(|b| b["name"].clone()),
                "best_correlation": best.map(|b| b["correlation"].clone()),
                "samples": results.len(),
                "note": "理想权重应使 attention_score 与 return 负相关（低关注→高收益）"
            })
            .to_string(),
        ))
    }
}

// ── 3b. StockFundamentalsReportTool (Phase 2) ──
// 工作流 t-fundamentals-data 节点调用此工具,生成预聚合的 markdown 基本面报告
// (含 health_score / valuation_state / 同比环比 / 估值带),供 a-fundamentals agent
// 启动时直接消费。避免 LLM 在大量原始财报上重复计算基础比率。
pub struct StockFundamentalsReportTool {
    pub client: Arc<AStockClient>,
}
impl StockFundamentalsReportTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockFundamentalsReportTool {
    fn name(&self) -> &str {
        "get_fundamentals_report_markdown"
    }
    fn description(&self) -> &str {
        "获取基本面分析报告(预聚合 markdown)：PE/PB/ROE/同比环比/估值带/0-100 健康度评分与质量等级。返回字符串,直接消费"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        // 1. 拉取实时行情
        let quote = self
            .client
            .get_quote(code)
            .await
            .map_err(|e| te(format!("get_quote 失败: {e}")))?;
        // 2. 拉取财务数据
        let financials = self
            .client
            .get_financials(code)
            .await
            .map_err(|e| te(format!("get_financials 失败: {e}")))?;
        // 3. 生成报告 (Phase 2 迁移后 FundamentalsAnalyzer 位于 astock-data)
        let report = axagent_astock_data::fundamentals_report::FundamentalsAnalyzer::generate(
            code,
            &quote,
            &financials,
        );
        // 4. Phase 3: 同步附加市场 Regime (避免单独增加 tool_node 破坏 9 节点网格)
        //    工作流引擎在 t-fundamentals-data 节点执行时,一并拉取 K 线 + 检测 regime,
        //    让 a-fundamentals 启动时直接消费完整的"基本面 + Regime"上下文。
        let regime_section = match self.client.get_klines(code, "daily", 120).await {
            Ok(klines) => {
                let regime = axagent_astock_data::regime::RegimeDetector::detect(&klines);
                let rules = regime
                    .triggered_rules
                    .iter()
                    .map(|r| format!("- {}", r))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "\n---\n\n## 当前市场 Regime (Phase 3)\n\n{} **{}**\n\n- 信心度: {}/100\n- Prompt 偏向: **{}**\n\n触发规则:\n{}\n\nexpert 在该 regime 下应据此切换分析 bias。\n",
                    regime.regime.emoji(),
                    regime.regime.label(),
                    regime.confidence,
                    regime.regime.prompt_bias(),
                    rules,
                )
            },
            Err(e) => {
                tracing::warn!(stock_code = %code, error = %e, "regime K 线拉取失败,跳过");
                String::new()
            },
        };
        Ok(ToolResult::success(format!("{}{}", report.to_markdown(), regime_section)))
    }
}

// ── 4. StockRegimeDetectTool (Phase 3) ──
// 借鉴 TradingAgents-CN "市场 regime 自适应 prompt" 机制:
// 此工具由 expert 主动调用,根据 20/60 日均线/布林带/波动率
// 判断市场状态(Bull/Bear/Sideways/Volatile),供 a-* expert 切换 prompt bias。
// 注:实际工作流预拉在 StockFundamentalsReportTool::call 中一并完成,
// 此工具供 expert 在其他分析场景中按需调用。
pub struct StockRegimeDetectTool {
    pub client: Arc<AStockClient>,
}
impl StockRegimeDetectTool {
    pub fn new(c: Arc<AStockClient>) -> Self {
        Self { client: c }
    }
}
#[async_trait]
impl Tool for StockRegimeDetectTool {
    fn name(&self) -> &str {
        "get_market_regime"
    }
    fn description(&self) -> &str {
        "市场 Regime 识别:综合 20/60 日均线/布林带/波动率/连涨连跌,判定当前市场状态 \
         (Bull/Bear/Sideways/Volatile),并给出 prompt_bias 建议"
    }
    fn input_schema(&self) -> Value {
        json!({"type":"object","properties":{"stock_code":{"type":"string","description":"6位股票代码"}},"required":["stock_code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Finance
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = input["stock_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| te("stock_code不能为空".into()))?;
        let klines = self
            .client
            .get_klines(code, "daily", 120)
            .await
            .map_err(|e| te(format!("get_klines 失败: {e}")))?;
        let report = axagent_astock_data::regime::RegimeDetector::detect(&klines);
        Ok(ToolResult::success(
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| format!("{:?}", report)),
        ))
    }
}

// ── Registration ──
pub fn register_stock_tools(
    registry: &mut crate::registry::ToolRegistry,
    client: Arc<AStockClient>,
) {
    registry.register_all(vec![
        Arc::new(StockQuoteTool::new(client.clone())),
        Arc::new(StockKlineTool::new(client.clone())),
        Arc::new(StockFinancialsTool::new(client.clone())),
        // Phase 2: 基本面报告工具,被 t-fundamentals-data 工作流节点调用
        Arc::new(StockFundamentalsReportTool::new(client.clone())),
        // Phase 3: Regime 识别工具,被 t-regime-detect 工作流节点调用
        Arc::new(StockRegimeDetectTool::new(client.clone())),
        Arc::new(StockNewsTool::new(client.clone())),
        Arc::new(StockMoneyFlowTool::new(client.clone())),
        Arc::new(StockHotStocksTool::new(client.clone())),
        Arc::new(StockIndustryRankTool::new(client.clone())),
        Arc::new(StockAnnouncementsTool::new(client.clone())),
        Arc::new(StockConsensusEPSTool::new(client.clone())),
        Arc::new(SearchStockTool::new(client.clone())),
        Arc::new(SearchNewsTool::new(client.clone())),
        Arc::new(ComputeScoringTool::new(client.clone())),
        Arc::new(ComputeValuationTool::new(client.clone())),
        Arc::new(ComputeRiskTool::new(client.clone())),
        Arc::new(StockBlockTradesTool::new(client.clone())),
        Arc::new(StockInstitutionalVisitsTool::new(client.clone())),
        Arc::new(StockPeersTool::new(client.clone())),
        Arc::new(StockResearchReportsTool::new(client.clone())),
        Arc::new(StockConceptBlocksTool::new(client.clone())),
        Arc::new(StockOptionPCRTool::new(client.clone())),
        Arc::new(StockCLSFlashTool::new(client.clone())),
        Arc::new(StockNorthBoundFlowTool::new(client.clone())),
        Arc::new(StockMarketDragonTigerTool::new(client.clone())),
        Arc::new(StockIndexQuotesTool::new(client.clone())),
        Arc::new(StockLockupTool::new(client.clone())),
        Arc::new(StockLockupBundleTool::new(client.clone())),
        Arc::new(StockShareholderTradesTool::new(client.clone())),
        Arc::new(StockDividendRecordsTool::new(client.clone())),
        Arc::new(StockNorthBoundHoldingTool::new(client.clone())),
        Arc::new(StockDragonTigerTool::new(client.clone())),
        Arc::new(StockMarginDataTool::new(client.clone())),
        Arc::new(AnnouncementContentTool::new(client.clone())),
        Arc::new(StockSectorInfoTool::new(client.clone())),
        Arc::new(ComputeAttentionScoreTool::new(client.clone())),
        Arc::new(ComputeIndustryPositionTool::new(client.clone())),
        Arc::new(CheckExitSignalsTool::new(client.clone())),
        Arc::new(ComputeSerenityPerformanceTool::new(client.clone())),
        Arc::new(VerifyCatalystsTool::new(client.clone())),
        Arc::new(OptimizeAttentionWeightsTool::new(client)),
    ]);
}
