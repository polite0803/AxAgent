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
            "name": "get_stock_policy_news",
            "description": "获取政策相关新闻（基于股票所属行业做关键词搜索：政策/规划/通知/补贴）",
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
        // #4: 股权质押数据工具
        // 前置工具：LLM 在调用 detect_pledge_risk（tools/finance.rs）前应先调用本工具获取 pledge_pct。
        // 输出字段：pledge_ratio（大股东质押总比例%）、pledge_shares（质押股数）、
        //           pledge_count（质押笔数）、controlling_pledge_ratio（控股股东质押比例%）、
        //           risk_level（安全/低风险/中风险/高风险/极高风险）
        json!({
            "name": "get_stock_pledge_data",
            "description": "获取股权质押数据（大股东质押比例/质押股数/控股股东质押比例/风险等级），用于质押风险评估",
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
            "description": "六维度技术评分（趋势/乖离/MACD/量能/RSI/支撑）+ 基本面修正 + 价值修正，返回100分制评分、买入信号、完整技术指标(ma5/ma20/bias_ma5/macd_dif/rsi14/boll_upper等)和最新价",
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
    // 辅助函数:兼容 LLM 传入数字或字符串类型的 stock_code
    // 修复(2026-07-22): GLM-5.2 偶尔传入 {"stock_code": 600887} (数字) 而非
    // {"stock_code": "600887"} (字符串),导致 as_str() 返回 None → 空字符串。
    let parse_code = |args: &serde_json::Value| -> String {
        match &args["stock_code"] {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        }
    };
    // 同上,兼容 keyword 的数字/字符串类型
    let parse_str = |args: &serde_json::Value, key: &str| -> String {
        match &args[key] {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        }
    };

    // P0 修复(2026-07-22): 对需要 stock_code 的工具统一做空值预检，
    // 避免空字符串传给 vendor 后触发 6 vendor × 2 轮无效重试（浪费 ~3 分钟）。
    // 根因：Agent 节点 LLM 流式 tool_call arguments 反序列化失败时 stock_code 为空，
    // parse_code 返回空字符串 → to_em_secid("") → "0." → vendor 全部失败 → 重试。
    // 以下工具不需要 stock_code（用 keyword 或无参数），排除在预检之外。
    if !matches!(
        tool_name,
        "search_stock"
            | "search_news"
            | "get_market_dragon_tiger"
            | "get_hot_stocks"
            | "get_industry_ranking"
            | "get_cls_flash"
            | "get_north_bound_flow"
            | "get_index_quotes"
            | "compute_portfolio_risk"
    ) {
        let code = parse_code(arguments);
        if code.is_empty() {
            tracing::warn!(
                tool = tool_name,
                args = %arguments,
                "stock_code 为空（LLM 参数解析失败），快速失败避免无效重试"
            );
            return Err(format!(
                "工具 '{}' 缺少 stock_code 参数（LLM 参数解析失败，arguments={}）",
                tool_name, arguments
            ));
        }
    }

    match tool_name {
        "search_stock" => {
            let keyword = parse_str(arguments, "keyword");
            // P0 修复(2026-07-22): 校验 keyword，避免 LLM 传入拼音片段或空字符串。
            // 日志显示 GLM-5.2 曾生成 "zhong'g"/"中国wei"/"中国卫tong" 等拼音片段，
            // 导致所有 vendor 搜索失败并重试，浪费 ~14 秒。
            if keyword.trim().is_empty() {
                return Err(
                    "search_stock 缺少 keyword 参数，请传入完整中文名称或6位数字代码".to_string()
                );
            }
            let keyword = keyword.as_str();
            let results = client.search_stock(keyword).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&results).map_err(|e| e.to_string())
        },
        "search_news" => {
            let keyword = parse_str(arguments, "keyword");
            let keyword = keyword.as_str();
            let limit = arguments["limit"].as_u64().unwrap_or(10) as u32;
            let results = client.search_news(keyword, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&results).map_err(|e| e.to_string())
        },
        "get_stock_quote" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let quote = client.get_quote(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&quote).map_err(|e| e.to_string())
        },
        "get_stock_kline" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let period = arguments["period"].as_str().unwrap_or("daily");
            let limit = arguments["limit"].as_u64().unwrap_or(120).min(500) as u32;
            let klines = client.get_klines(code, period, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&klines).map_err(|e| e.to_string())
        },
        "get_stock_financials" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let financials = client.get_financials(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&financials).map_err(|e| e.to_string())
        },
        "get_fundamentals_report_markdown" => {
            let code = parse_code(arguments);
            let code = code.as_str();
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
            let code = parse_code(arguments);
            let code = code.as_str();
            let limit = arguments["limit"].as_u64().unwrap_or(30).min(100) as u32;
            let news = client.get_news(code, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&news).map_err(|e| e.to_string())
        },
        "get_stock_policy_news" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let limit = arguments["limit"].as_u64().unwrap_or(30).min(100) as u32;
            let news = client.get_policy_news(code, limit).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&news).map_err(|e| e.to_string())
        },
        "get_stock_money_flow" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let flow = client.get_money_flow(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&flow).map_err(|e| e.to_string())
        },
        "get_social_sentiment" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let sentiment = client.get_social_sentiment(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&sentiment).map_err(|e| e.to_string())
        },
        "get_stock_dragon_tiger" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let dt = client.get_dragon_tiger(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&dt).map_err(|e| e.to_string())
        },
        "get_stock_margin_data" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let margin = client.get_margin_data(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&margin).map_err(|e| e.to_string())
        },
        "get_stock_sector_info" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let sector = client.get_sector_info(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&sector).map_err(|e| e.to_string())
        },
        "get_stock_north_bound" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let nb = client.get_north_bound_holding(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&nb).map_err(|e| e.to_string())
        },
        "get_stock_lockup" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let lockup = client.get_lockup_schedule(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&lockup).map_err(|e| e.to_string())
        },
        "get_stock_lockup_bundle" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let bundle = client.get_lockup_bundle(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&bundle).map_err(|e| e.to_string())
        },
        "get_stock_shareholder_trades" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let trades = client.get_shareholder_trades(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&trades).map_err(|e| e.to_string())
        },
        "get_stock_dividend_records" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let dividends = client.get_dividend_records(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&dividends).map_err(|e| e.to_string())
        },
        "get_stock_research_reports" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let reports = client.get_research_reports(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&reports).map_err(|e| e.to_string())
        },
        "get_stock_consensus_eps" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let eps = client.get_consensus_eps(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&eps).map_err(|e| e.to_string())
        },
        "get_stock_concept_blocks" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let blocks = client.get_concept_blocks(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&blocks).map_err(|e| e.to_string())
        },
        "get_stock_announcements" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let anns = client.get_announcements(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&anns).map_err(|e| e.to_string())
        },
        "get_stock_block_trades" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let bt = client.get_block_trades(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&bt).map_err(|e| e.to_string())
        },
        "get_stock_institutional_visits" => {
            let code = parse_code(arguments);
            let code = code.as_str();
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
            let code = parse_code(arguments);
            let code = code.as_str();
            let peers = client.get_peers(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&peers).map_err(|e| e.to_string())
        },
        "get_stock_option_pcr" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let pcr = client.get_option_pcr(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&pcr).map_err(|e| e.to_string())
        },
        // #4: 股权质押数据 — LLM 可先调用本工具拿到 pledge_pct,
        // 再调用 detect_pledge_risk (tools/finance.rs) 做阈值判断。
        "get_stock_pledge_data" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            let pledge = client.get_pledge_data(code).await.map_err(|e| e.to_string())?;
            serde_json::to_string(&pledge).map_err(|e| e.to_string())
        },
        // ── 算法工具：compute_scoring / compute_valuation / compute_portfolio_risk ──
        // 历史问题：工具列表（stock_mcp_tools）声明了这些算法工具，但 dispatch_tool
        // 的 match 中没有对应分支，LLM 调用时走到 `_ => Unknown MCP tool` 分支失败。
        // V57 修复：补全三个算法工具的分发，复用 astock-data 内的 ScoringEngine 等模块，
        // 避免重复实现（铁律 4）。
        "compute_scoring" => {
            let code = parse_code(arguments);
            let code = code.as_str();
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
            // #7 修复(2026-07-22): 原实现只返回 ObjectiveScore 评分结构,
            // 缺少 totalScore/currentPrice/indicators/factor_backtest 字段,
            // 导致下游 input_mapping 引用(t-scoring.result.indicators.rsi14 等)全部为 null,
            // LLM 报告中 MA5/MA20/bias_ma5 等技术指标缺失。
            //
            // 修复: 用 json! 构造扩展返回结构,既保留原 ObjectiveScore 字段(向后兼容),
            // 又追加 totalScore(别名)/currentPrice/indicators/factor_backtest(占位)。
            let score_json = serde_json::to_value(&score).map_err(|e| e.to_string())?;
            let ind_json = serde_json::to_value(&ind).map_err(|e| e.to_string())?;
            // P0 根因修复(2026-07-22): 返回 kline_json 供下游 trader 节点通过 input_mapping
            // 引用，避免 trader 重新调用 get_stock_kline（原设计导致 LLM 生成空 stock_code
            // 的 tool_call，触发 6 vendor × 2 轮无效重试，浪费 3.4 分钟）。
            // kline_json 是 120 根日 K 线的 JSON 数组，trader 可直接传给 compute_atr /
            // compute_kelly / compute_mc 等纯计算工具。
            let kline_json = serde_json::to_value(&klines).map_err(|e| e.to_string())?;
            let result = serde_json::json!({
                // ── 原 ObjectiveScore 字段(flatten 等价,向后兼容) ──
                "total": score_json["total"],
                "trendScore": score_json["trendScore"],
                "deviationScore": score_json["deviationScore"],
                "macdScore": score_json["macdScore"],
                "volumeScore": score_json["volumeScore"],
                "rsiScore": score_json["rsiScore"],
                "supportScore": score_json["supportScore"],
                "bollScore": score_json["bollScore"],
                "fundamentalAdjustment": score_json["fundamentalAdjustment"],
                "signal": score_json["signal"],
                "signalCode": score_json["signalCode"],
                // ── #7 新增: 别名 + 原始指标 + 占位字段 ──
                "totalScore": score_json["total"], // 别名,供 input_mapping 引用
                "currentPrice": latest_price,       // 最新收盘价
                "indicators": ind_json,             // 完整技术指标(ma5/ma20/bias_ma5/macd_dif/rsi14/boll_upper 等)
                // kline_json: 120 根日 K 线原始数据，供 trader 节点的 ATR/Kelly/MC 工具使用
                "kline_json": kline_json,
                // factor_backtest 占位: 因子回测引擎未实现,下游 portfolio-mgr.rhai
                // 会 fallback 到等权,不会因 null 报错。
                "factor_backtest": {
                    "factors": serde_json::json!({}),
                    "note": "factor backtest engine not implemented, using equal weights fallback"
                }
            });
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "compute_valuation" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            if code.is_empty() {
                return Err("compute_valuation 缺少 stock_code 参数".to_string());
            }
            // 估值需要行情（PE/PB/总市值）和财务数据
            let quote = client.get_quote(code).await.map_err(|e| e.to_string())?;
            let financials = client.get_financials(code).await.map_err(|e| e.to_string())?;
            let current_price = quote.price;
            let pe = quote.pe;
            let pb = quote.pb;
            let total_mv = quote.total_mv;
            let total_shares = if current_price > 0.0 {
                total_mv.map(|mv| mv / current_price / 1_0000_0000.0)
            } else {
                None
            };

            // ── Piotroski F-Score (0-9) ──
            let f_score = compute_f_score(&financials);
            let f_score_level = match f_score {
                7..=9 => "优秀(7-9)",
                5..=6 => "良好(5-6)",
                3..=4 => "一般(3-4)",
                _ => "弱(0-2)",
            };

            // ── 护城河量化评分 (0-100) ──
            let (moat_score, moat_level) = compute_moat_score(&financials, pe, pb);

            // ── DCF 两阶段估值 ──
            let (dcf_low, dcf_mid, dcf_high) =
                compute_dcf(&financials, total_shares, current_price);

            // ── 安全边际 ──
            let (mos_pct, mos_level) = if dcf_mid > 0.0 && current_price > 0.0 {
                let mos = ((dcf_mid - current_price) / dcf_mid) * 100.0;
                let level = if mos > 30.0 {
                    "充足"
                } else if mos > 15.0 {
                    "适中"
                } else if mos > 0.0 {
                    "不足"
                } else {
                    "无（高估风险）"
                };
                (mos, level)
            } else {
                (0.0, "无法计算")
            };

            // ── 格雷厄姆内在价值 ──
            let graham_value = compute_graham_value(&financials, current_price);

            // ── 所有者收益率 ──
            let oe_yield =
                if let (Some(mv), Some(oe)) = (total_mv, compute_owner_earnings(&financials)) {
                    if mv > 0.0 {
                        (oe / mv) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

            // ── 综合估值判断 ──
            let value_signal = {
                let mut score = 0u32;
                if mos_pct > 20.0 {
                    score += 30;
                } else if mos_pct > 10.0 {
                    score += 20;
                } else if mos_pct > 0.0 {
                    score += 10;
                }
                score += f_score.min(9) * 5;
                score += moat_score.min(100) / 5;
                if oe_yield > 5.0 {
                    score += 20;
                } else if oe_yield > 3.0 {
                    score += 10;
                }
                match score {
                    60.. => "低估",
                    45.. => "合理偏低",
                    30.. => "合理",
                    15.. => "偏高",
                    _ => "高估",
                }
            };

            let result = json!({
                "stock_code": code,
                "current_price": current_price,
                "pe": pe,
                "pb": pb,
                "total_mv": total_mv,
                "dcf_valuation": {
                    "low": round2(dcf_low),
                    "mid": round2(dcf_mid),
                    "high": round2(dcf_high),
                },
                "graham_intrinsic_value": round2(graham_value),
                "margin_of_safety": {
                    "pct": round1(mos_pct),
                    "level": mos_level,
                },
                "piotroski_f_score": {
                    "score": f_score,
                    "max": 9,
                    "level": f_score_level,
                },
                "moat": {
                    "score": moat_score,
                    "max": 100,
                    "level": moat_level,
                },
                "owner_earnings_yield_pct": round1(oe_yield),
                "value_signal": value_signal,
                "summary": format!(
                    "内在价值(DCF中性)≈{:.2}元 | 格雷厄姆值≈{:.2}元 | 安全边际{:.0}%({}) | F-Score={}/9({}) | 护城河{}/100({}) | OE收益率{:.1}% | 综合判断:{}",
                    dcf_mid, graham_value, mos_pct, mos_level, f_score, f_score_level, moat_score, moat_level, oe_yield, value_signal
                ),
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
            let roe_ttm_pct = fin.and_then(|f| f.roe).map(|v| (v * 10.0).round() / 10.0);
            let gross_margin_pct =
                fin.and_then(|f| f.gross_margin).map(|v| (v * 10.0).round() / 10.0);
            let debt_ratio_pct = fin.and_then(|f| f.debt_ratio).map(|v| (v * 10.0).round() / 10.0);
            let revenue_growth_yoy_pct =
                fin.and_then(|f| f.revenue_yoy).map(|v| (v * 10.0).round() / 10.0);

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

// ── 估值计算辅助函数 ──────────────────────────────────────────────────────

use axagent_harness::market_data::FinancialReport;

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Piotroski F-Score (0-9)
///  profitability(4): 正ROE, 正经营现金流, ROE同比增长, 现金流>净利润
///  leverage(3): 长期负债不增, 流动比率提升, 无新股增发
///  efficiency(2): 毛利率提升, 资产周转率提升
fn compute_f_score(financials: &[FinancialReport]) -> u32 {
    if financials.is_empty() {
        return 0;
    }
    let curr = &financials[0];
    let prev = financials.get(1);
    let mut score = 0u32;

    // P1: 正 ROE（roe 是百分比值，>0 即正 ROE）
    if curr.roe.unwrap_or(0.0) > 0.0 {
        score += 1;
    }
    // P2: 正经营现金流
    if curr.operating_cash_flow.unwrap_or(0.0) > 0.0 {
        score += 1;
    }
    // P3: ROE 同比增长
    if let (Some(curr_roe), Some(prev_roe)) = (curr.roe, prev.and_then(|p| p.roe)) {
        if curr_roe > prev_roe {
            score += 1;
        }
    } else if curr.roe.unwrap_or(0.0) > 0.0 && prev.is_none() {
        score += 1; // 仅一期且为正 ROE 也算通过
    }
    // P4: 经营现金流 > 净利润（应计质量）
    let np = curr.net_profit.unwrap_or(0.0);
    let ocf = curr.operating_cash_flow;
    if let (Some(ocf_val), np_val) = (ocf, np) {
        if ocf_val > np_val {
            score += 1;
        }
    }

    // L1: 长期负债/资产负债率不增
    if let (Some(curr_dr), Some(prev_dr)) = (curr.debt_ratio, prev.and_then(|p| p.debt_ratio)) {
        if curr_dr <= prev_dr {
            score += 1;
        }
    } else {
        score += 1; // 无法对比时给通过
    }
    // L2: 流动比率提升
    if let (Some(curr_cr), Some(prev_cr)) = (curr.current_ratio, prev.and_then(|p| p.current_ratio))
    {
        if curr_cr >= prev_cr {
            score += 1;
        }
    } else if curr.current_ratio.unwrap_or(1.5) >= 1.0 {
        score += 1;
    }
    // L3: 无新股增发 — 无直接数据，跳过（用负债率指标替代为已覆盖）
    // A股财报数据不包含增发信息，此项默认给通过
    score += 1;

    // E1: 毛利率提升
    if let (Some(curr_gm), Some(prev_gm)) = (curr.gross_margin, prev.and_then(|p| p.gross_margin)) {
        if curr_gm > prev_gm {
            score += 1;
        }
    } else if curr.gross_margin.unwrap_or(0.0) > 20.0 {
        score += 1;
    }
    // E2: 资产周转率提升 — 用 revenue / total_assets 近似；无 total_assets 时用营收同比增长代替
    if let (Some(curr_rev), Some(prev_rev)) = (curr.revenue, prev.and_then(|p| p.revenue)) {
        if let (Some(curr_ta), Some(prev_ta)) =
            (curr.total_assets, prev.and_then(|p| p.total_assets))
        {
            if curr_ta > 0.0 && prev_ta > 0.0 {
                let curr_tat = curr_rev / curr_ta;
                let prev_tat = prev_rev / prev_ta;
                if curr_tat > prev_tat {
                    score += 1;
                }
            }
        } else if curr_rev > prev_rev {
            score += 1; // 营收增长近似替代周转率提升
        }
    } else {
        score += 1; // 无法对比时给通过
    }

    score.min(9)
}

/// 护城河量化评分 (0-100)
fn compute_moat_score(
    financials: &[FinancialReport],
    pe: Option<f64>,
    _pb: Option<f64>,
) -> (u32, &'static str) {
    if financials.is_empty() {
        return (0, "无");
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
    let gm_values: Vec<f64> = financials.iter().take(5).filter_map(|r| r.gross_margin).collect();
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

    let level = if score >= 70 {
        "宽阔"
    } else if score >= 40 {
        "狭窄"
    } else {
        "无"
    };
    (score, level)
}

/// DCF 两阶段估值（保守/中性/乐观三档）
fn compute_dcf(
    financials: &[FinancialReport],
    total_shares: Option<f64>,
    _current_price: f64,
) -> (f64, f64, f64) {
    if financials.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let latest = &financials[0];
    // vendor 返回的财务数据单位均为"元"，无需缩放
    // 优先用 free_cash_flow；其次 operating_cash_flow - capex；最后用 net_profit * 0.90 估算
    let fcf = latest
        .free_cash_flow
        .or_else(|| {
            latest
                .operating_cash_flow
                .and_then(|ocf| latest.capital_expenditure.map(|capex| ocf - capex))
        })
        .unwrap_or_else(|| latest.net_profit.unwrap_or(0.0) * 0.90);

    let shares = match total_shares {
        Some(s) if s > 0.0 => s,
        _ => return (0.0, 0.0, 0.0),
    };

    if fcf <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let fcf_per_share = fcf / shares; // 元/股

    // 用营收同比增速作为 growth_rate 参考；默认 8%
    let growth = latest.revenue_yoy.map(|y| (y / 100.0).clamp(0.02, 0.30)).unwrap_or(0.08);
    let perpetual = 0.03;
    let discount = 0.10;

    let dcf_two_stage = |fcf_ps: f64, g: f64, p: f64, d: f64| -> f64 {
        let mut pv = 0.0;
        let mut current_fcf = fcf_ps;
        for year in 1..=5 {
            current_fcf *= 1.0 + g;
            pv += current_fcf / (1.0 + d).powi(year);
        }
        let terminal_fcf = current_fcf * (1.0 + p);
        let terminal_spread = (d - p).max(0.001);
        let terminal_value = terminal_fcf / terminal_spread;
        let terminal_pv = terminal_value / (1.0 + d).powi(5);
        pv + terminal_pv
    };

    let low = dcf_two_stage(
        fcf_per_share,
        (growth * 0.6_f64).max(0.01),
        (perpetual * 0.7_f64).max(0.01),
        discount,
    );
    let mid = dcf_two_stage(fcf_per_share, growth.max(0.01), perpetual, discount);
    let high = dcf_two_stage(
        fcf_per_share,
        (growth * 1.5_f64).clamp(0.02, 0.30),
        (perpetual * 1.3_f64).min(0.05),
        discount,
    );

    (low, mid, high)
}

/// 格雷厄姆内在价值公式：V = EPS × (8.5 + 2g) × 4.4 / Y
/// g 为未来7-10年预期增长率，Y 为AAA企业债收益率（取4.4%为基准）
fn compute_graham_value(financials: &[FinancialReport], current_price: f64) -> f64 {
    if financials.is_empty() || current_price <= 0.0 {
        return 0.0;
    }
    let latest = &financials[0];
    let eps = latest.eps.unwrap_or(0.0);
    if eps <= 0.0 {
        return 0.0;
    }
    // 用利润同比作为增长参考；vendor 返回的 profit_yoy 是百分比值（如 15.0 表示 15%）
    // 格雷厄姆公式中 g 直接用百分比值（8.5 + 2*g），封顶 30%
    let g = latest.profit_yoy.map(|y| y.clamp(0.0, 30.0)).unwrap_or(5.0);
    let bond_yield = 4.4;
    let value = eps * (8.5 + 2.0 * g) * 4.4 / bond_yield;
    value.max(0.0)
}

/// 巴菲特所有者收益（亿元）
fn compute_owner_earnings(financials: &[FinancialReport]) -> Option<f64> {
    if financials.is_empty() {
        return None;
    }
    let f = &financials[0];
    // vendor 返回的财务数据单位均为"元"，无需缩放
    if let (Some(ocf), Some(capex)) = (f.operating_cash_flow, f.capital_expenditure) {
        Some((ocf - capex).max(0.0))
    } else if let Some(fcf) = f.free_cash_flow {
        Some(fcf.max(0.0))
    } else {
        let net = f.net_profit.unwrap_or(0.0);
        let debt_ratio = f.debt_ratio.unwrap_or(50.0);
        // debt_ratio 是百分比值，>60% 为高负债
        let factor = if debt_ratio > 60.0 {
            0.85
        } else if debt_ratio > 40.0 {
            0.90
        } else {
            0.95
        };
        Some((net * factor).max(0.0))
    }
}
