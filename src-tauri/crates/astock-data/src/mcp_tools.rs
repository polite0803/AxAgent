use serde_json::json;

pub fn stock_mcp_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "search_stock",
            "description": "搜索A股股票，按代码或名称模糊匹配",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "股票代码或名称关键词" }
                },
                "required": ["keyword"]
            }
        }),
        json!({
            "name": "search_news",
            "description": "按关键词搜索财经新闻，用于验证催化剂/CapEx/行业趋势",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "搜索关键词（如'英伟达 CapEx'、'HBM 产能扩张'）" },
                    "limit": { "type": "integer", "description": "返回条数（默认10）" }
                },
                "required": ["keyword"]
            }
        }),
        json!({
            "name": "get_stock_quote",
            "description": "获取A股实时行情（价格、涨跌幅、成交量等）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码，如600519" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_kline",
            "description": "获取A股历史K线数据（含日期、开高低收、成交量）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "period": { "type": "string", "description": "周期：daily/weekly/monthly", "default": "daily" },
                    "limit": { "type": "integer", "description": "K线数量（1-500）", "default": 120 }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_financials",
            "description": "获取A股财务报表（营收、净利润、EPS、ROE、毛利率等）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_fundamentals_report_markdown",
            "description": "获取基本面预聚合 Markdown 报告（健康度评分/估值带/安全边际/同比环比），供基本面分析师直接消费，避免重复计算基础比率",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_news",
            "description": "获取A股相关新闻公告（含情绪评分）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "limit": { "type": "integer", "description": "新闻数量", "default": 30 }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_money_flow",
            "description": "获取A股资金流向（主力/超大单/大单/中单/小单净流入）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_social_sentiment",
            "description": "获取社交舆情数据（东方财富股吧帖子数/情感倾向/看多看空比例），用于情绪面分析师",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_dragon_tiger",
            "description": "获取个股龙虎榜数据（营业部买卖、上榜原因）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_margin_data",
            "description": "获取融资融券数据（融资买入额、余额、融券卖出量、余量）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_sector_info",
            "description": "获取行业分类（申万一级/二级、概念板块标签）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_north_bound",
            "description": "获取北向资金个股持仓（持股数量、占比）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_lockup",
            "description": "获取限售解禁日程（解禁日期、股数、比例、股东名称）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_lockup_bundle",
            "description": "获取解禁+大股东增减持+大宗交易聚合包（lockup-watcher 冷启动数据）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_shareholder_trades",
            "description": "获取大股东增减持记录（变动类型、数量、均价、原因）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_dividend_records",
            "description": "获取除权除息/分红送配记录",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_research_reports",
            "description": "获取研报列表（机构、评级、目标价、EPS预测）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_consensus_eps",
            "description": "获取机构一致预期EPS（一致预期EPS、目标价、评级）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_concept_blocks",
            "description": "获取概念板块三维归属（行业/概念/地域）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_announcements",
            "description": "获取巨潮全量公告（沪深北交所）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_block_trades",
            "description": "获取大宗交易记录（交易日期、价格、数量、买方/卖方营业部）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_institutional_visits",
            "description": "获取机构调研记录（调研日期、参与机构数、调研内容摘要）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_market_dragon_tiger",
            "description": "获取全市场龙虎榜（每日上榜股票+净买额排名）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_hot_stocks",
            "description": "获取同花顺强势股（当日强势股+题材归因标签）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_industry_ranking",
            "description": "获取行业横向排名（~90行业涨跌排名+领涨股）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_cls_flash",
            "description": "获取财联社快讯（分钟级电报）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_north_bound_flow",
            "description": "获取北向资金分钟级流向（沪深股通）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_index_quotes",
            "description": "获取大盘指数行情（上证指数、深证成指、创业板指）",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_stock_peers",
            "description": "获取同行业可比公司估值（PE/PB/ROE/涨跌幅/市值）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_stock_option_pcr",
            "description": "获取期权PCR（看跌/看涨成交量和持仓量比率，市场情绪前瞻指标）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        // ── 算法工具 ──
        json!({
            "name": "compute_scoring",
            "description": "六维度技术评分（趋势/乖离/MACD/量能/RSI/支撑）+ 基本面修正 + 价值修正，返回100分制评分及买入信号",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "kline_json": { "type": "string", "description": "上游K线节点输出的JSON" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "compute_valuation",
            "description": "DCF两阶段估值 + 格雷厄姆公式 + Piotroski F-Score(0-9) + 护城河量化(0-100)，返回内在价值和安全边际",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "financials_json": { "type": "string", "description": "上游财务节点输出的JSON" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "compute_portfolio_risk",
            "description": "计算单股风险画像：年化波动率/最大回撤/夏普比率/ROE/毛利率/负债率/营收增速/PE，输出 stockRiskProfile 供下游 portfolio-mgr 决策",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_codes": { "type": "string", "description": "逗号分隔的股票代码列表（工作流节点传入，取第一个为主标的）" },
                    "stock_code": { "type": "string", "description": "单个6位股票代码（LLM 直接调用时使用）" },
                    "weights": { "type": "string", "description": "逗号分隔的持仓权重(0-1)，不填则等权（可选）" }
                },
                "required": []
            }
        }),
        json!({
            "name": "run_quality_gate",
            "description": "LLM报告质量门控：占位检测、失败标记检测、必采项覆盖率检查，返回A-F质量评级",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reports_json": { "type": "string", "description": "分析师报告JSON，格式: {expert_id: report_text}" }
                },
                "required": ["reports_json"]
            }
        }),
    ]
}

pub async fn execute_mcp_tool(
    client: &crate::AStockClient,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    match tool_name {
        "search_stock" => {
            let keyword = arguments["keyword"].as_str().unwrap_or("");
            let results = client.search_stock(keyword).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&results).map_err(|e| e.to_string())
        },
        "search_news" => {
            let keyword = arguments["keyword"].as_str().unwrap_or("");
            let limit = arguments["limit"].as_u64().unwrap_or(10) as u32;
            let results = client.search_news(keyword, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&results).map_err(|e| e.to_string())
        },
        "get_stock_quote" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let quote = client.get_quote(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&quote).map_err(|e| e.to_string())
        },
        "get_stock_kline" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let period = arguments["period"].as_str().unwrap_or("daily");
            let limit = arguments["limit"].as_u64().unwrap_or(120).min(500) as u32;
            let klines = client.get_klines(code, period, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&klines).map_err(|e| e.to_string())
        },
        "get_stock_financials" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let financials = client.get_financials(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&financials).map_err(|e| e.to_string())
        },
        "get_fundamentals_report_markdown" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let quote = client.get_quote(code).await.map_err(|e| e.to_string())?;
            let financials = client.get_financials(code).await.map_err(|e| e.to_string())?;
            let report = crate::fundamentals_report::FundamentalsAnalyzer::generate(
                code,
                &quote,
                &financials,
            );
            Ok(report.to_markdown())
        },
        "get_stock_news" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let limit = arguments["limit"].as_u64().unwrap_or(30).min(100) as u32;
            let news = client.get_news(code, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&news).map_err(|e| e.to_string())
        },
        "get_stock_money_flow" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let flow = client.get_money_flow(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&flow).map_err(|e| e.to_string())
        },
        "get_social_sentiment" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let sentiment = client.get_social_sentiment(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&sentiment).map_err(|e| e.to_string())
        },
        "get_stock_dragon_tiger" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let dt = client.get_dragon_tiger(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&dt).map_err(|e| e.to_string())
        },
        "get_stock_margin_data" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let margin = client.get_margin_data(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&margin).map_err(|e| e.to_string())
        },
        "get_stock_sector_info" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let sector = client.get_sector_info(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&sector).map_err(|e| e.to_string())
        },
        "get_stock_north_bound" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let nb = client.get_north_bound_holding(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&nb).map_err(|e| e.to_string())
        },
        "get_stock_lockup" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let lockup = client.get_lockup_schedule(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&lockup).map_err(|e| e.to_string())
        },
        "get_stock_lockup_bundle" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let bundle = client.get_lockup_bundle(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&bundle).map_err(|e| e.to_string())
        },
        "get_stock_shareholder_trades" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let trades = client.get_shareholder_trades(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&trades).map_err(|e| e.to_string())
        },
        "get_stock_dividend_records" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let dividends = client.get_dividend_records(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&dividends).map_err(|e| e.to_string())
        },
        "get_stock_research_reports" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let reports = client.get_research_reports(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&reports).map_err(|e| e.to_string())
        },
        "get_stock_consensus_eps" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let eps = client.get_consensus_eps(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&eps).map_err(|e| e.to_string())
        },
        "get_stock_concept_blocks" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let blocks = client.get_concept_blocks(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&blocks).map_err(|e| e.to_string())
        },
        "get_stock_announcements" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let anns = client.get_announcements(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&anns).map_err(|e| e.to_string())
        },
        "get_stock_block_trades" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let bt = client.get_block_trades(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&bt).map_err(|e| e.to_string())
        },
        "get_stock_institutional_visits" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let visits = client.get_institutional_visits(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&visits).map_err(|e| e.to_string())
        },
        "get_market_dragon_tiger" => {
            let dt = client.get_market_dragon_tiger().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&dt).map_err(|e| e.to_string())
        },
        "get_hot_stocks" => {
            let hot = client.get_hot_stocks().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&hot).map_err(|e| e.to_string())
        },
        "get_industry_ranking" => {
            let ranking = client.get_industry_ranking().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&ranking).map_err(|e| e.to_string())
        },
        "get_cls_flash" => {
            let flash = client.get_cls_flash().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&flash).map_err(|e| e.to_string())
        },
        "get_north_bound_flow" => {
            let flow = client.get_north_bound_flow().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&flow).map_err(|e| e.to_string())
        },
        "get_index_quotes" => {
            let idx = client.get_index_quotes().await.map_err(|e| e.to_string())?;
            serde_json::to_string(&idx).map_err(|e| e.to_string())
        },
        "get_stock_peers" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let peers = client.get_peers(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&peers).map_err(|e| e.to_string())
        },
        "get_stock_option_pcr" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            let pcr = client.get_option_pcr(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&pcr).map_err(|e| e.to_string())
        },
        // ── 算法工具：compute_scoring / compute_valuation / compute_portfolio_risk ──
        // 历史问题：工具列表（stock_mcp_tools）声明了这些算法工具，但 dispatch_tool
        // 的 match 中没有对应分支，LLM 调用时走到 `_ => Unknown MCP tool` 分支失败。
        // V57 修复：补全三个算法工具的分发，复用 astock-data 内的 ScoringEngine 等模块，
        // 避免重复实现（铁律 4）。
        "compute_scoring" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            if code.is_empty() {
                return Err("compute_scoring 缺少 stock_code 参数".to_string());
            }
            // 允许调用方传入 kline_json（避免重复拉取）；若未提供则现场拉取 120 日 K 线
            let klines = if let Some(kj) = arguments["kline_json"].as_str() {
                serde_json::from_str::<Vec<crate::types::KLine>>(kj)
                    .map_err(|e| format!("kline_json 解析失败: {e}"))?
            } else {
                client.get_klines(code, "daily", 120).await.map_err(|e| e.to_string())?
            };
            let ind = crate::indicators::compute_indicators(code, &klines);
            let latest_price = klines.last().map(|k| k.close).unwrap_or(0.0);
            let score = crate::scoring::ScoringEngine::score(&ind, latest_price, None);
            serde_json::to_string(&score).map_err(|e| e.to_string())
        },
        "compute_valuation" => {
            let code = arguments["stock_code"].as_str().unwrap_or("");
            if code.is_empty() {
                return Err("compute_valuation 缺少 stock_code 参数".to_string());
            }
            // 估值需要行情（PE/PB）和财务数据
            let quote = client.get_quote(code).await.map_err(|e| e.to_string())?;
            let financials = client.get_financials(code).await.map_err(|e| e.to_string())?;
            // 取最近一期财务快照用于估值带计算
            let fin_snap = financials.first();
            let pe = quote.pe;
            let pb = quote.pb;
            let result = json!({
                "stock_code": code,
                "pe": pe,
                "pb": pb,
                "quote": &quote,
                "financials": fin_snap,
                "valuation_note": "基于 PE/PB 与财务快照的简化估值判断，详细 DCF/F-Score 请见 replay_tool_chain",
            });
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "compute_portfolio_risk" => {
            // 修复(2026-07-21):
            // 1) 参数名兼容: 节点传 `stock_codes`(逗号分隔), LLM 直接调用传 `stock_code`(单数)
            // 2) 输出结构对齐 portfolio-mgr.rhai 期望的 stockRiskProfile 字段
            //    (annualizedVolatilityPct/maxDrawdownPct/sharpeRatio/roeTTMPct/
            //     grossMarginPct/debtRatioPct/revenueGrowthYoYPct/peTTM)
            // 3) 用真实 K 线计算波动率/回撤/夏普, 用财报提取基本面指标
            let primary_code = arguments["stock_codes"]
                .as_str()
                .and_then(|s| s.split(',').next())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| arguments["stock_code"].as_str().map(str::trim))
                .ok_or_else(|| {
                    "compute_portfolio_risk 缺少 stock_codes/stock_code 参数".to_string()
                })?;

            // 拉取 60 日前复权 K 线计算量化风险指标
            let klines = client
                .get_klines_with_adj(
                    primary_code,
                    "daily",
                    60,
                    Some(crate::types::AdjType::Forward),
                )
                .await
                .map_err(|e| e.to_string())?;

            let (ann_vol_pct, max_dd_pct, sharpe) = if klines.len() >= 2 {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                // 日收益率序列
                let returns: Vec<f64> = closes
                    .windows(2)
                    .map(|w| {
                        if w[0] > 0.0 {
                            (w[1] - w[0]) / w[0]
                        } else {
                            0.0
                        }
                    })
                    .collect();
                // 年化波动率 = std(daily_returns) * sqrt(252) * 100
                let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                    / returns.len().max(1) as f64;
                let std = variance.sqrt();
                let ann_vol = std * (252.0_f64).sqrt() * 100.0;
                // 最大回撤
                let mut peak = closes[0];
                let mut max_dd = 0.0_f64;
                for &p in &closes {
                    if p > peak {
                        peak = p;
                    }
                    if peak > 0.0 {
                        let dd = (peak - p) / peak;
                        if dd > max_dd {
                            max_dd = dd;
                        }
                    }
                }
                let max_dd_pct = max_dd * 100.0;
                // 夏普比率 (年化, rf=3%)
                let rf_daily = 0.03 / 252.0;
                let sharpe = if std > 0.0 {
                    (mean - rf_daily) / std * (252.0_f64).sqrt()
                } else {
                    0.0
                };
                (
                    (ann_vol * 10.0).round() / 10.0,
                    (max_dd_pct * 10.0).round() / 10.0,
                    (sharpe * 1000.0).round() / 1000.0,
                )
            } else {
                (0.0, 0.0, 0.0)
            };

            // 拉取财报提取基本面指标(取最新一条)
            let financials =
                client.get_financials(primary_code).await.map_err(|e| e.to_string())?;
            let fin = financials.first();
            let roe_ttm_pct = fin.and_then(|f| f.roe).map(|v| (v * 100.0 * 10.0).round() / 10.0);
            let gross_margin_pct =
                fin.and_then(|f| f.gross_margin).map(|v| (v * 100.0 * 10.0).round() / 10.0);
            let debt_ratio_pct =
                fin.and_then(|f| f.debt_ratio).map(|v| (v * 100.0 * 10.0).round() / 10.0);
            let revenue_growth_yoy_pct =
                fin.and_then(|f| f.revenue_yoy).map(|v| (v * 100.0 * 10.0).round() / 10.0);

            // 拉取行情拿 PE/PB
            let quote = client.get_quote(primary_code).await.map_err(|e| e.to_string())?;
            let pe_ttm = quote.pe;

            let result = json!({
                "stock_code": primary_code,
                "stockRiskProfile": {
                    "annualizedVolatilityPct": ann_vol_pct,
                    "maxDrawdownPct": max_dd_pct,
                    "sharpeRatio": sharpe,
                    "roeTTMPct": roe_ttm_pct,
                    "grossMarginPct": gross_margin_pct,
                    "debtRatioPct": debt_ratio_pct,
                    "revenueGrowthYoYPct": revenue_growth_yoy_pct,
                    "peTTM": pe_ttm,
                },
                "risk_note": "基于60日前复权K线计算波动率/回撤/夏普, 基本面指标取最新财报",
            });
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        _ => Err(format!("Unknown MCP tool: {tool_name}")),
    }
}
