use serde_json::json;

// ── G14 DojoSDK 工具执行器（trait + 全局注册器） ─────────────────────────
//
// astock-data 是 implementor 层级，不能依赖 quant（consumer）/
// stock-analysis（implementor，但反向依赖会循环）/ tools（hybrid）。
// 因此 DojoSDK 工具的执行逻辑通过 trait 抽象，由 main crate 实现并注册。
//
// 调用顺序：
// 1. main crate 启动时调用 `register_dojo_sdk_executor(impl)` 注册实现
// 2. LLM 通过 MCP 协议调用 `dojo_*` / `sector_precomputed_*` 工具
// 3. `execute_mcp_tool` 命中 DojoSDK 工具时调用 `with_dojo_sdk_executor`
// 4. 实现内部路由到 quant / stock-analysis / tools 等具体 crate

/// DojoSDK 工具执行器 trait
///
/// 实现方需在 `execute` 中根据 `tool_name` 路由到具体的 SDK 功能。
/// 返回 JSON 字符串（与 `execute_mcp_tool` 一致）。
#[async_trait::async_trait]
pub trait DojoSdkExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, String>;
}

/// 全局 DojoSdkExecutor 注册器（OnceLock 保证一次性注册）
static DOJO_SDK_EXECUTOR: std::sync::OnceLock<Box<dyn DojoSdkExecutor>> =
    std::sync::OnceLock::new();

/// 注册全局 DojoSdkExecutor（启动时调用一次）
///
/// 重复调用会被忽略（OnceLock 语义）。建议在 `init::services` 中注册。
pub fn register_dojo_sdk_executor(executor: Box<dyn DojoSdkExecutor>) {
    let _ = DOJO_SDK_EXECUTOR.set(executor);
}

/// 检查 DojoSdkExecutor 是否已注册
pub fn has_dojo_sdk_executor() -> bool {
    DOJO_SDK_EXECUTOR.get().is_some()
}

/// 判断工具名是否属于 DojoSDK 工具集
pub fn is_dojo_sdk_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "sector_precomputed_sector_alpha_factors_daily"
            | "dojo_run_quant_backtest"
            | "dojo_get_skill_content"
            | "dojo_list_skills"
            | "dojo_get_paper_portfolio"
            | "dojo_list_market_mainlines"
            | "dojo_create_plan"
            | "dojo_execute_plan"
            | "dojo_revise_plan"
    )
}

/// 委托 DojoSDK 工具到已注册的执行器
async fn with_dojo_sdk_executor(
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    match DOJO_SDK_EXECUTOR.get() {
        Some(executor) => executor.execute(tool_name, arguments).await,
        None => Err(format!(
            "DojoSDK 工具 '{tool_name}' 需要注册 DojoSdkExecutor 才能执行（启动时调用 register_dojo_sdk_executor）"
        )),
    }
}

pub fn stock_mcp_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "search_stock",
            "description": "搜索A股股票。keyword 必须是完整的中文名称（如'中国卫通'、'紫金矿业'）或 6 位数字代码（如'601698'），禁止传入拼音片段（如'zi'jin'、'zhongguo'）或中英混合片段。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "完整中文名称或6位数字代码（如'中国卫通'、'601698'）。禁止拼音片段。" }
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
        // ── G1 跨市场数据接入：美股/港股/外汇/基准指数 ──
        json!({
            "name": "get_international_stock_quote",
            "description": "获取美股/港股实时行情（价格/涨跌/成交量/市值）。支持 AAPL/00700.HK/TSLA/BABA 等代码格式",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "国际股票代码：AAPL / TSLA / 00700 / 00700.HK / BABA.US" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_international_stock_kline",
            "description": "获取美股/港股历史 K 线（开高低收/成交量）。支持 daily/weekly/monthly 周期",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "国际股票代码：AAPL / 00700 / TSLA.US" },
                    "period": { "type": "string", "description": "周期：daily/weekly/monthly", "default": "daily" },
                    "limit": { "type": "integer", "description": "K线数量（1-1000）", "default": 120 }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "get_benchmark_kline",
            "description": "获取基准指数 K 线（标普500/纳指/恒生/上证等），用于跨市场对比分析",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "benchmark_code": { "type": "string", "description": "基准指数代码：SPX/IXIC/DJI/HSI/HSCEI（国际）或 000001.SH/399001/399006/000300（A股）" },
                    "period": { "type": "string", "description": "周期：daily/weekly/monthly", "default": "daily" },
                    "limit": { "type": "integer", "description": "K线数量（1-1000）", "default": 120 }
                },
                "required": ["benchmark_code"]
            }
        }),
        json!({
            "name": "get_forex_kline",
            "description": "获取外汇 K 线（USD/CNY、HKD/CNY 等），用于跨市场汇率风险分析",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pair": { "type": "string", "description": "外汇对：USD/CNY、HKD/CNY、EUR/USD、USD/JPY" },
                    "period": { "type": "string", "description": "周期：daily/weekly/monthly", "default": "daily" },
                    "limit": { "type": "integer", "description": "K线数量（1-1000）", "default": 120 }
                },
                "required": ["pair"]
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
        // ── Serenity 瓶颈筛选工具集（V58 补全，对接 astock-data 已有 API）──
        // 历史问题：seed_serenity.rs 注册了 7 个 ToolDef schema 但无 Rust 实现，
        // 运行时 ToolResolver 三级匹配全部落空，t-baseline-*/t-signal-* 14 个 ToolNode
        // 全部失败，c-bottleneck-trend* CodeNode 因上游缺失也失败。
        // 修复：在 execute_mcp_tool 中补全 7 个 match 分支，对接 astock-data 已有 API。
        json!({
            "name": "compute_industry_position",
            "description": "行业竞争地位分析：拉取个股及同行业可比公司，计算毛利率/ROE/负债率/R&D强度行业排名、CapEx/折旧比",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "compute_bottleneck_signals",
            "description": "瓶颈信号计算：基于多期财报计算存货周转天数变化、毛利率同比趋势、CapEx/折旧比，识别供给瓶颈迹象",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "compute_attention_score",
            "description": "计算个股关注度评分 0-100，越低越冷门，验证低关注度因子（覆盖研报数+新闻热度+换手率+共识差）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "check_exit_signals",
            "description": "检查个股退出信号：技术替代新闻、毛利率趋势、产能过剩、新进入者、需求放缓。返回 overall_exit_urgency",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "entry_price": { "type": "number", "description": "买入价（可选，用于计算止损触发）" },
                    "stop_loss_price": { "type": "number", "description": "止损价（可选）" }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "verify_catalysts",
            "description": "验证 Serenity 候选的催化剂是否兑现：基于近期新闻公告匹配催化剂描述",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "catalyst_descriptions": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "催化剂描述列表"
                    }
                },
                "required": ["stock_code"]
            }
        }),
        json!({
            "name": "compute_serenity_performance",
            "description": "计算 Serenity 候选推荐后表现：相对推荐日的涨跌幅、相对大盘超额收益、持有天数",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6位股票代码" },
                    "recommend_date": { "type": "string", "description": "推荐日期 YYYY-MM-DD" }
                },
                "required": ["stock_code", "recommend_date"]
            }
        }),
        json!({
            "name": "optimize_attention_weights",
            "description": "基于历史样本调优关注度评分权重：输入样本（attention_score + 实际表现），输出建议权重",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "samples": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "attention_score": { "type": "number" },
                                "actual_return_pct": { "type": "number" }
                            }
                        },
                        "description": "历史样本列表"
                    }
                },
                "required": ["samples"]
            }
        }),
        // G3 产业链相关 MCP 工具（get_industry_chain_propagation /
        // map_news_to_cross_market_stocks）已于 P2-8 阶段迁至
        // `axagent_analysis_engine::mcp_tools`。本 crate 不再注册这两个工具，
        // 调用方需通过 `axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools()`
        // 获取并合并到工具列表中。
        // ── G14 DojoSDK 工具集 ──────────────────────────────────────────
        json!({
            "name": "sector_precomputed_sector_alpha_factors_daily",
            "description": "DojoSDK: 行业 alpha 因子日频数据。返回一级行业（申万）alpha 因子序列，含 size/value/momentum/reversal/volatility/liquidity 6 类因子值。可选 date 范围与行业过滤。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start_date": { "type": "string", "description": "起始日期 YYYY-MM-DD（默认近 30 日）" },
                    "end_date": { "type": "string", "description": "结束日期 YYYY-MM-DD（默认今日）" },
                    "industry": { "type": "string", "description": "可选行业过滤（如 '银行'、'半导体'）；不传 = 全行业" },
                    "factors": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "需要返回的因子列表，可选: size/value/momentum/reversal/volatility/liquidity；默认全部"
                    }
                }
            }
        }),
        json!({
            "name": "dojo_run_quant_backtest",
            "description": "DojoSDK: 运行量化策略回测。内置 5 套策略 (ma_cross/macd/rsi/boll/turtle)，返回完整 BacktestResult 含交易记录、净值曲线、Sharpe/MaxDD 等指标。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "6 位股票代码" },
                    "strategy": { "type": "string", "enum": ["ma_cross","macd","rsi","boll","turtle"], "description": "内置策略名" },
                    "start_date": { "type": "string", "description": "回测起始日期 YYYY-MM-DD" },
                    "end_date": { "type": "string", "description": "回测结束日期 YYYY-MM-DD" },
                    "initial_capital": { "type": "number", "description": "初始资金（默认 100000）" },
                    "params": { "type": "object", "description": "策略参数（如 {fast:5, slow:20}），可选" }
                },
                "required": ["stock_code", "strategy", "start_date", "end_date"]
            }
        }),
        json!({
            "name": "dojo_get_skill_content",
            "description": "DojoSDK: 获取指定 SKILL 的完整内容（含 frontmatter + 正文）。优先走 SkillPromptCache 缓存；缓存未命中则扫描 skill_dirs。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "skill_name": { "type": "string", "description": "skill 名称（目录名，如 stock-pick / industry-chain-analysis / risk-management / market-mainline）" }
                },
                "required": ["skill_name"]
            }
        }),
        json!({
            "name": "dojo_list_skills",
            "description": "DojoSDK: 列出当前所有可用 SKILL（含内置 4 个 + 用户自定义），返回 [{name, description, version, source_kind}]。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "include_external": { "type": "boolean", "description": "是否包含外部目录（claude/trae/codebuddy 等），默认 true" }
                }
            }
        }),
        json!({
            "name": "dojo_get_paper_portfolio",
            "description": "DojoSDK: 获取模拟观察组合详情（含持仓 + 实时盈亏）。组合状态 active/closed/archived。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "portfolio_id": { "type": "string", "description": "组合 ID" }
                },
                "required": ["portfolio_id"]
            }
        }),
        json!({
            "name": "dojo_list_market_mainlines",
            "description": "DojoSDK: 列出最近 N 天市场主线（按强度降序）。可选 category 过滤。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "days": { "type": "integer", "description": "最近天数（默认 7）" },
                    "category": { "type": "string", "description": "主题大类过滤（可选）" }
                }
            }
        }),
        json!({
            "name": "dojo_create_plan",
            "description": "DojoSDK G19: 创建分层执行计划。基于目标拆分为多阶段（Phase）+ 多任务（Task），支持任务间依赖与角色分配。复用 HierarchicalPlanner。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "goal": { "type": "string", "description": "计划目标描述（如'分析半导体产业链投资机会'）" },
                    "phases": {
                        "type": "array",
                        "description": "阶段数组，每个阶段包含多个任务",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "阶段名称" },
                                "description": { "type": "string", "description": "阶段描述" },
                                "dependencies": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "依赖的阶段 ID（可空，从 1 开始计数：1=第一Phase）"
                                },
                                "tasks": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "description": { "type": "string", "description": "任务描述" },
                                            "action_type": {
                                                "type": "string",
                                                "description": "动作类型：agent/llm/tool/shell",
                                                "default": "agent"
                                            },
                                            "parameters": { "type": "object", "description": "任务参数（JSON）" },
                                            "dependencies": {
                                                "type": "array",
                                                "items": { "type": "string" },
                                                "description": "依赖任务 ID（同阶段内）"
                                            },
                                            "max_retries": { "type": "integer", "default": 3 },
                                            "assigned_role": {
                                                "type": "string",
                                                "description": "分配角色（analyst/implementer/reviewer）"
                                            }
                                        },
                                        "required": ["description", "action_type"]
                                    }
                                }
                            },
                            "required": ["name", "description", "tasks"]
                        }
                    }
                },
                "required": ["goal", "phases"]
            }
        }),
        json!({
            "name": "dojo_execute_plan",
            "description": "DojoSDK G19: 启动/继续执行已创建的计划。返回当前进度与下一批可执行任务。复用 HierarchicalPlanner。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "plan_id": { "type": "string", "description": "计划 ID（由 create_plan 返回）" },
                    "action": {
                        "type": "string",
                        "enum": ["start", "pause", "resume", "cancel", "progress", "next_tasks", "complete_task", "fail_task"],
                        "default": "start",
                        "description": "执行动作：start=开始执行, pause=暂停, resume=继续, cancel=取消, progress=查询进度, next_tasks=获取下一批可执行任务, complete_task=标记任务完成, fail_task=标记任务失败"
                    },
                    "task_id": { "type": "string", "description": "complete_task/fail_task 必填" },
                    "result": { "type": "object", "description": "complete_task 时附带的任务结果" },
                    "error": { "type": "string", "description": "fail_task 时的错误信息" }
                },
                "required": ["plan_id", "action"]
            }
        }),
        json!({
            "name": "dojo_revise_plan",
            "description": "DojoSDK G19: 修订计划（重规划）。支持 Retry/Skip/Insert/Remove/Reorder/AddPhase/ModifyTask 七种动作。复用 HierarchicalPlanner.replan。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "plan_id": { "type": "string", "description": "计划 ID" },
                    "reason": {
                        "type": "string",
                        "description": "重规划原因（StepFailed/ResourceConstraint/GoalChanged/NewDependencyDiscovered/ManualIntervention）"
                    },
                    "actions": {
                        "type": "array",
                        "description": "重规划动作数组",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["Retry", "Skip", "Insert", "Remove", "Reorder", "AddPhase", "ModifyTask"]
                                },
                                "task_id": { "type": "string", "description": "Retry/Skip/Remove/Reorder/ModifyTask 必填" },
                                "phase_id": { "type": "string", "description": "Insert 必填" },
                                "modified_parameters": { "type": "object", "description": "Retry 时修改参数" },
                                "reason": { "type": "string", "description": "Skip/Remove 原因" },
                                "task": { "type": "object", "description": "Insert 时新任务定义" },
                                "position": { "type": "integer", "description": "Insert/Reorder/AddPhase 位置" },
                                "new_position": { "type": "integer", "description": "Reorder 新位置" },
                                "phase": { "type": "object", "description": "AddPhase 新阶段" },
                                "modifications": { "type": "object", "description": "ModifyTask 修改字段" }
                            },
                            "required": ["type"]
                        }
                    },
                    "rollback_to_version": {
                        "type": "integer",
                        "description": "可选：回滚到指定版本（不传则执行 actions）"
                    }
                },
                "required": ["plan_id", "reason"]
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
        // 修复(2026-07-22): GLM-5.2 偶传 {"stock_code": 600887} (数字)
        // 修复(2026-07-30): GLM-5.2 偶传 {"argument": 601398} (泛化键名)
        // 修复(2026-07-30 第2版): GLM-5.2 偶传 {"argument": {"stock_code": "601899"}} (嵌套对象)
        let from_key = |key: &str| -> String {
            match &args[key] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            }
        };
        let code = from_key("stock_code");
        if !code.is_empty() {
            return code;
        }
        // 兼容 "argument" 为字符串/数字/嵌套对象三种形态
        match &args["argument"] {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Object(obj) => {
                // GLM-5.2 偶传 {"argument": {"stock_code": "601899"}}
                if let Some(v) = obj.get("stock_code") {
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            },
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

    // G14: DojoSDK 工具集优先委托给已注册的 DojoSdkExecutor
    // （astock-data 不能直接依赖 quant/stock-analysis/tools，故走 trait 注入）
    if is_dojo_sdk_tool(tool_name) {
        return with_dojo_sdk_executor(tool_name, arguments).await;
    }

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
            | "optimize_attention_weights"
            | "get_forex_kline"
            | "get_benchmark_kline"
            // G3 industry_chain 工具已迁至 axagent_analysis_engine::mcp_tools
            | "dojo_run_quant_backtest"
            | "dojo_get_skill_content"
            | "dojo_list_skills"
            | "dojo_get_paper_portfolio"
            | "dojo_list_market_mainlines"
            | "dojo_create_plan"
            | "dojo_execute_plan"
            | "dojo_revise_plan"
            | "sector_precomputed_sector_alpha_factors_daily"
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
            if keyword.trim().is_empty() {
                return Err(
                    "search_stock 缺少 keyword 参数，请传入完整中文名称或6位数字代码".to_string()
                );
            }
            // 拼音片段检测在 AStockClient::search_stock 底层统一处理
            // （覆盖 Tauri 命令 + MCP 工具两条路径，避免重复逻辑）
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
        // ── G1 跨市场数据接入 ──
        "get_international_stock_quote" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            if code.is_empty() {
                return Err("get_international_stock_quote 缺少 stock_code 参数".to_string());
            }
            // 国际代码直接走 international vendor 路径
            let quote = if crate::is_international_code(code) {
                client.get_international_quote(code).await.map_err(|e| e.to_string())?
            } else {
                // 兼容：传入 A 股代码时走默认路由
                client.get_quote(code).await.map_err(|e| e.to_string())?
            };
            serde_json::to_string(&quote).map_err(|e| e.to_string())
        },
        "get_international_stock_kline" => {
            let code = parse_code(arguments);
            let code = code.as_str();
            if code.is_empty() {
                return Err("get_international_stock_kline 缺少 stock_code 参数".to_string());
            }
            let period = arguments["period"].as_str().unwrap_or("daily");
            let limit = arguments["limit"].as_u64().unwrap_or(120).min(1000) as u32;
            let klines = if crate::is_international_code(code) {
                client
                    .get_international_klines(code, period, limit, None)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                client.get_klines(code, period, limit).await.map_err(|e| e.to_string())?
            };
            serde_json::to_string(&klines).map_err(|e| e.to_string())
        },
        "get_benchmark_kline" => {
            let benchmark = parse_str(arguments, "benchmark_code");
            if benchmark.trim().is_empty() {
                return Err("get_benchmark_kline 缺少 benchmark_code 参数".to_string());
            }
            let period = arguments["period"].as_str().unwrap_or("daily");
            let limit = arguments["limit"].as_u64().unwrap_or(120).min(1000) as u32;
            let klines = client
                .get_benchmark_klines(&benchmark, period, limit)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&klines).map_err(|e| e.to_string())
        },
        "get_forex_kline" => {
            let pair = parse_str(arguments, "pair");
            if pair.trim().is_empty() {
                return Err("get_forex_kline 缺少 pair 参数（如 USD/CNY）".to_string());
            }
            let period = arguments["period"].as_str().unwrap_or("daily");
            let limit = arguments["limit"].as_u64().unwrap_or(120).min(1000) as u32;
            let klines =
                client.get_forex_klines(&pair, period, limit).await.map_err(|e| e.to_string())?;
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
            // 尝试解析可选的估值配置参数
            let valuation_config = arguments
                .get("valuation_config")
                .and_then(|v| serde_json::from_value::<ValuationConfig>(v.clone()).ok());

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
                compute_dcf(&financials, total_shares, current_price, valuation_config.as_ref());

            // ── 格雷厄姆内在价值（先计算，作为 DCF fallback） ──
            let graham_value =
                compute_graham_value(&financials, current_price, valuation_config.as_ref());

            // ── 安全边际：优先使用 DCF，不可用时 fallback 到格雷厄姆 ──
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
            } else if graham_value > 0.0 && current_price > 0.0 {
                // DCF 不可用时，使用格雷厄姆估值作为 fallback
                let mos = ((graham_value - current_price) / graham_value) * 100.0;
                let level = if mos > 30.0 {
                    "充足(格雷厄姆)"
                } else if mos > 15.0 {
                    "适中(格雷厄姆)"
                } else if mos > 0.0 {
                    "不足(格雷厄姆)"
                } else {
                    "无（高估风险）"
                };
                (mos, level)
            } else {
                (0.0, "无法计算")
            };

            // ── 所有者收益率 ──
            // compute_owner_earnings 返回元，total_mv 为亿元，需统一单位
            let oe_yield =
                if let (Some(mv), Some(oe)) = (total_mv, compute_owner_earnings(&financials)) {
                    if mv > 0.0 {
                        // oe(元) / (mv(亿元) * 1_0000_0000) = 收益率
                        (oe / (mv * 1_0000_0000.0)) * 100.0
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
                // P3-C8: 夏普比率统一走 harness 实现（样本方差 n-1，A 股 244 天年化）。
                // 修复历史 bug: 原实现误用总体方差（n 分母），且 252/244 混用导致
                // 与 stock-analysis/risk.rs 的 Sharpe 结果分叉。
                // 保留 rf=3% 作为 A 股长期无风险利率近似（10 年期国债中枢）。
                let rf_daily = 0.03 / axagent_harness::indicators::A_SHARE_TRADING_DAYS_PER_YEAR;
                let sharpe = axagent_harness::indicators::sharpe_ratio_with_annualization(
                    &returns,
                    rf_daily,
                    axagent_harness::indicators::A_SHARE_TRADING_DAYS_PER_YEAR,
                );
                // 年化波动率: 复用 harness stddev_sample 保持算法一致（样本方差 n-1）
                let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                let std = axagent_harness::indicators::stddev_sample(&returns, mean);
                let ann_vol =
                    std * axagent_harness::indicators::A_SHARE_TRADING_DAYS_PER_YEAR.sqrt() * 100.0;
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
        // ── Serenity 瓶颈筛选工具集（V58 补全）──
        // 输出契约与 bottleneck-calc.rhai 期望字段对齐：
        //   competitive_position.gross_margin_pct / roe_pct / debt_ratio_pct / rnd_intensity
        //   capacity_indicators.signal / sector
        "compute_industry_position" => {
            let code = parse_code(arguments);
            let result = compute_industry_position_impl(client, &code).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "compute_bottleneck_signals" => {
            let code = parse_code(arguments);
            let result = compute_bottleneck_signals_impl(client, &code).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        // 输出契约与 mapper_prompt attention_metrics 字段对齐：
        //   coverage_change_3m / search_heat / relative_volume / consensus_gap / attention_score
        "compute_attention_score" => {
            let code = parse_code(arguments);
            let result = compute_attention_score_impl(client, &code).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        // 输出契约与 mapper_prompt exit_signals 字段对齐：
        //   technology_disruption_risk / capacity_oversupply_risk / new_entrant_risk
        //   demand_slowdown_risk / overall_exit_urgency
        "check_exit_signals" => {
            let code = parse_code(arguments);
            let entry_price = arguments["entry_price"].as_f64();
            let stop_loss_price = arguments["stop_loss_price"].as_f64();
            let result =
                check_exit_signals_impl(client, &code, entry_price, stop_loss_price).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "verify_catalysts" => {
            let code = parse_code(arguments);
            let catalysts: Vec<String> = arguments["catalyst_descriptions"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let result = verify_catalysts_impl(client, &code, &catalysts).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "compute_serenity_performance" => {
            let code = parse_code(arguments);
            let recommend_date = parse_str(arguments, "recommend_date");
            let result = compute_serenity_performance_impl(client, &code, &recommend_date).await?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "optimize_attention_weights" => {
            let samples = arguments["samples"].as_array().cloned().unwrap_or_default();
            let result = optimize_attention_weights_impl(&samples);
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        // P1-2 修复(2026-08-09): run_quality_gate 原只有 schema 声明、dispatch 无实现分支，
        // LLM 调用必走 Unknown MCP tool。现接入 astock-data::quality::run_quality_gate。
        // 输入: {reports_json: "{expert_id: report_text}"}，输出: {grade, summary, warnings}。
        "run_quality_gate" => {
            let reports_json = arguments["reports_json"].as_str().ok_or_else(|| {
                "run_quality_gate 缺少 reports_json 参数（{expert_id: report_text} JSON）"
                    .to_string()
            })?;
            let reports: std::collections::HashMap<String, String> =
                serde_json::from_str(reports_json)
                    .map_err(|e| format!("reports_json 解析失败: {e}"))?;
            let check = crate::quality::run_quality_gate(&reports);
            serde_json::to_string(&serde_json::json!({
                "grade": format!("{:?}", check.grade),
                "summary": check.summary,
                "warnings": check.warnings,
            }))
            .map_err(|e| e.to_string())
        },
        // G3 产业链相关工具（get_industry_chain_propagation /
        // map_news_to_cross_market_stocks）已于 P2-8 阶段迁至
        // `axagent_analysis_engine::mcp_tools::execute_industry_chain_tool`。
        // 调用方需在调用 astock-data::mcp_tools::execute_mcp_tool 之前，
        // 先尝试 axagent_analysis_engine::mcp_tools::execute_industry_chain_tool。
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
    }
    // L2: 流动比率提升
    if let (Some(curr_cr), Some(prev_cr)) = (curr.current_ratio, prev.and_then(|p| p.current_ratio))
    {
        if curr_cr >= prev_cr {
            score += 1;
        }
    } else if curr.current_ratio.unwrap_or(0.0) >= 1.0 {
        score += 1;
    }
    // L3: 无新股增发 — 用 net_profit/eps 比值近似股本变化；比值下降视为股本增加
    // 股本 = net_profit / eps，若股本增长则视为可能增发
    if let (Some(curr_np), Some(curr_eps), Some(prev_np), Some(prev_eps)) =
        (curr.net_profit, curr.eps, prev.and_then(|p| p.net_profit), prev.and_then(|p| p.eps))
    {
        if curr_eps > 0.0 && prev_eps > 0.0 {
            let curr_shares_approx = (curr_np / curr_eps).abs();
            let prev_shares_approx = (prev_np / prev_eps).abs();
            if curr_shares_approx <= prev_shares_approx * 1.05 {
                // 股本变化在 5% 以内视为无显著增发
                score += 1;
            }
        }
    } else if prev.is_none() {
        // 只有一期数据，检查当前资产负债率是否健康
        if curr.debt_ratio.unwrap_or(100.0) < 50.0 {
            score += 1;
        }
    }

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
            } else if curr_rev > prev_rev {
                score += 1; // 营收增长近似替代周转率提升
            }
        } else if curr_rev > prev_rev {
            score += 1; // 营收增长近似替代周转率提升
        }
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

    // 3. 低负债 (20分) — 使用多期平均负债率，避免单期异常
    let debt_values: Vec<f64> = financials.iter().take(5).filter_map(|r| r.debt_ratio).collect();
    let debt_count = debt_values.len() as f64;
    let avg_debt = if debt_count > 0.0 {
        debt_values.iter().sum::<f64>() / debt_count
    } else {
        100.0
    };
    if avg_debt < 20.0 {
        score += 20;
    } else if avg_debt < 40.0 {
        score += 15;
    } else if avg_debt < 60.0 {
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
///
/// 估值参数说明：
/// - 永续增长率 `PERPETUAL_GROWTH = 3%`：接近长期通胀率，Gordon 增长模型标准假设
/// - 折现率 `DISCOUNT_RATE = 10%`：A 股权益风险溢价合理区间（无风险利率 3% + 风险溢价 7%）
/// - 默认增长率 `DEFAULT_GROWTH = 8%`：当无营收同比数据时使用的保守假设
/// - 增长率区间 `[MIN_GROWTH, MAX_GROWTH] = [2%, 30%]`：限制异常值
const PERPETUAL_GROWTH: f64 = 0.03;
const DISCOUNT_RATE: f64 = 0.10;
const DEFAULT_GROWTH: f64 = 0.08;
const MIN_GROWTH: f64 = 0.02;
const MAX_GROWTH: f64 = 0.30;
const FORECAST_YEARS: i32 = 5;

/// 估值运行时配置（可由前端设置页下发）
///
/// 当 `Some(config)` 传入时使用自定义值，否则回退到模块级常量。
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuationConfig {
    pub perpetual_growth: Option<f64>,
    pub discount_rate: Option<f64>,
    pub default_growth: Option<f64>,
    pub min_growth: Option<f64>,
    pub max_growth: Option<f64>,
    pub forecast_years: Option<i32>,
    pub bond_yield: Option<f64>,
}

impl ValuationConfig {
    fn perpetual_growth(&self) -> f64 {
        self.perpetual_growth.unwrap_or(PERPETUAL_GROWTH)
    }
    fn discount_rate(&self) -> f64 {
        self.discount_rate.unwrap_or(DISCOUNT_RATE)
    }
    fn default_growth(&self) -> f64 {
        self.default_growth.unwrap_or(DEFAULT_GROWTH)
    }
    fn min_growth(&self) -> f64 {
        self.min_growth.unwrap_or(MIN_GROWTH)
    }
    fn max_growth(&self) -> f64 {
        self.max_growth.unwrap_or(MAX_GROWTH)
    }
    fn forecast_years(&self) -> i32 {
        self.forecast_years.unwrap_or(FORECAST_YEARS)
    }
    fn bond_yield(&self) -> f64 {
        self.bond_yield.unwrap_or(4.4)
    }
}

fn compute_dcf(
    financials: &[FinancialReport],
    total_shares: Option<f64>,
    _current_price: f64,
    config: Option<&ValuationConfig>,
) -> (f64, f64, f64) {
    if financials.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let cfg = config.copied().unwrap_or_default();
    let perpetual_growth = cfg.perpetual_growth();
    let discount_rate = cfg.discount_rate();
    let default_growth = cfg.default_growth();
    let min_growth = cfg.min_growth();
    let max_growth = cfg.max_growth();
    let forecast_years = cfg.forecast_years();

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
    let growth = latest
        .revenue_yoy
        .map(|y| (y / 100.0).clamp(min_growth, max_growth))
        .unwrap_or(default_growth);

    let dcf_two_stage = |fcf_ps: f64, g: f64, p: f64, d: f64| -> f64 {
        let mut pv = 0.0;
        let mut current_fcf = fcf_ps;
        for year in 1..=forecast_years {
            current_fcf *= 1.0 + g;
            pv += current_fcf / (1.0 + d).powi(year);
        }
        let terminal_fcf = current_fcf * (1.0 + p);
        let terminal_spread = (d - p).max(0.001);
        let terminal_value = terminal_fcf / terminal_spread;
        let terminal_pv = terminal_value / (1.0 + d).powi(forecast_years);
        pv + terminal_pv
    };

    // 保守档：增长率打 6 折，永续增长率打 7 折
    let low_growth = (growth * 0.6_f64).max(min_growth / 2.0);
    let low_perpetual = (perpetual_growth * 0.7_f64).max(min_growth / 2.0);
    let low = dcf_two_stage(fcf_per_share, low_growth, low_perpetual, discount_rate);

    // 中性档：使用原始增长率和永续增长率
    let mid_growth = growth.max(min_growth / 2.0);
    let mid = dcf_two_stage(fcf_per_share, mid_growth, perpetual_growth, discount_rate);

    // 乐观档：增长率放大 1.5 倍，永续增长率放大 1.3 倍
    let high_growth = (growth * 1.5_f64).clamp(min_growth, max_growth);
    let high_perpetual = (perpetual_growth * 1.3_f64).min(0.05);
    let high = dcf_two_stage(fcf_per_share, high_growth, high_perpetual, discount_rate);

    (low, mid, high)
}

/// 格雷厄姆内在价值公式：V = EPS × (8.5 + 2g) × 4.4 / Y
/// g 为未来7-10年预期增长率，Y 为AAA企业债收益率基准
fn compute_graham_value(
    financials: &[FinancialReport],
    current_price: f64,
    config: Option<&ValuationConfig>,
) -> f64 {
    if financials.is_empty() || current_price <= 0.0 {
        return 0.0;
    }
    let cfg = config.copied().unwrap_or_default();
    let bond_yield = cfg.bond_yield();

    let latest = &financials[0];
    let eps = latest.eps.unwrap_or(0.0);
    if eps <= 0.0 {
        return 0.0;
    }
    // profit_yoy 是百分比值（如 15.0 表示 15%），需要转换为小数形式
    // 格雷厄姆公式中 g 应为小数（如 0.15），封顶 30% = 0.30
    let g = latest.profit_yoy.map(|y| (y / 100.0).clamp(0.0, 0.30)).unwrap_or(0.05);
    let value = eps * (8.5 + 2.0 * g) * 4.4 / bond_yield;
    value.max(0.0)
}

/// 巴菲特所有者收益（元）
/// 注意：vendor 返回的财务数据单位均为"元"，无需缩放
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

// ═══════════════════════════════════════════════════════════════════════════
// Serenity 瓶颈筛选工具集（V58 补全）
// ═══════════════════════════════════════════════════════════════════════════
// 历史：seed_serenity.rs 注册了 7 个 ToolDef 但无实现，运行时全部失败。
// 修复：对接 astock-data 已有 API（get_financials/get_peers/get_quote 等），
// 输出契约严格对齐 bottleneck-calc.rhai / mapper_prompt 期望字段。

/// compute_industry_position：行业竞争地位分析
/// 输出契约（bottleneck-calc.rhai 期望）：
///   sector / competitive_position.{gross_margin_pct, roe_pct, debt_ratio_pct,
///   rnd_intensity, gm_rank_in_peers, total_peer_count} / capacity_indicators.signal
async fn compute_industry_position_impl(
    client: &crate::AStockClient,
    stock_code: &str,
) -> Result<serde_json::Value, String> {
    let financials = client.get_financials(stock_code).await.map_err(|e| e.to_string())?;
    let peers = client.get_peers(stock_code).await.map_err(|e| e.to_string())?;
    let sector_info = client.get_sector_info(stock_code).await.map_err(|e| e.to_string())?;

    let latest = financials.first();
    let gm_pct = latest.and_then(|f| f.gross_margin).unwrap_or(0.0);
    let roe_pct = latest.and_then(|f| f.roe).unwrap_or(0.0);
    let debt_ratio_pct = latest.and_then(|f| f.debt_ratio).unwrap_or(0.0);
    // R&D 强度无直接 API：用 (revenue - net_profit * 10) / revenue 近似
    // 保守估算：若 net_profit 为负或为 0，按营收 5% 默认值
    let rnd_intensity = latest
        .and_then(|f| {
            f.revenue.and_then(|rev| {
                if rev > 0.0 {
                    f.net_profit.map(|np| {
                        let rnd = (rev - np * 10.0).max(0.0);
                        (rnd / rev * 100.0).clamp(0.0, 30.0)
                    })
                } else {
                    None
                }
            })
        })
        .unwrap_or(5.0);
    // CapEx/折旧比：用 capex / (total_assets * 0.05) 近似（5% 折旧率）
    let capex_dep_ratio = latest
        .and_then(|f| {
            f.capital_expenditure.and_then(|capex| {
                f.total_assets.and_then(|ta| {
                    if ta > 0.0 {
                        Some((capex / (ta * 0.05)).clamp(0.0, 10.0))
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or(0.0);
    let capex_signal = if capex_dep_ratio >= 3.0 {
        "积极扩产"
    } else if capex_dep_ratio >= 1.5 {
        "温和扩张"
    } else if capex_dep_ratio >= 1.0 {
        "维持投入"
    } else {
        "收缩投入"
    };

    // 在 peers 中按 ROE 排序计算个股排名（仅含有 ROE 数据的）
    let mut peer_roes: Vec<(String, f64)> =
        peers.iter().filter_map(|p| p.roe.map(|r| (p.stock_code.clone(), r))).collect();
    // 加入目标股本身
    if roe_pct > 0.0 || !peer_roes.is_empty() {
        peer_roes.push((stock_code.to_string(), roe_pct));
    }
    peer_roes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let total_peer_count = peer_roes.len() as u32;
    let gm_rank_in_peers = peer_roes
        .iter()
        .position(|(code, _)| code == stock_code)
        .map(|i| (i + 1) as u32)
        .unwrap_or(0);

    let sector_full = match sector_info.as_ref() {
        None => "未知行业".to_string(),
        Some(si) if si.sub_sector.is_empty() => si.sector_name.clone(),
        Some(si) => format!("{}/{}", si.sector_name, si.sub_sector),
    };

    let result = json!({
        "stock_code": stock_code,
        "sector": sector_full,
        "competitive_position": {
            "gross_margin_pct": round2(gm_pct),
            "roe_pct": round2(roe_pct),
            "debt_ratio_pct": round2(debt_ratio_pct),
            "rnd_intensity": round2(rnd_intensity),
            "gm_rank_in_peers": gm_rank_in_peers,
            "total_peer_count": total_peer_count
        },
        "capacity_indicators": {
            "capex_dep_ratio": round2(capex_dep_ratio),
            "signal": capex_signal,
            "expansion_intensity": if capex_dep_ratio >= 3.0 { "high" }
                else if capex_dep_ratio >= 1.5 { "medium" }
                else { "low" }
        },
        "peer_count": peers.len(),
        "summary": format!(
            "行业:{sector_full} | 毛利率:{gm_pct:.1}% | ROE:{roe_pct:.1}% | 负债率:{debt_ratio_pct:.1}% | CapEx/折旧:{capex_dep_ratio:.2}({capex_signal})"
        )
    });
    Ok(result)
}

/// compute_bottleneck_signals：瓶颈信号计算
/// 输出契约（mapper_prompt 期望）：
///   inventory_turnover.{days_latest, days_yoy_change, signal}
///   gross_margin_trend.{latest, yoy_change, direction}
///   capex.{capex_dep_ratio, signal}
async fn compute_bottleneck_signals_impl(
    client: &crate::AStockClient,
    stock_code: &str,
) -> Result<serde_json::Value, String> {
    let financials = client.get_financials(stock_code).await.map_err(|e| e.to_string())?;

    if financials.is_empty() {
        return Ok(json!({
            "stock_code": stock_code,
            "inventory_turnover": null,
            "gross_margin_trend": null,
            "capex": null,
            "summary": "无财务数据"
        }));
    }

    let latest = &financials[0];
    let prev = financials.get(1);

    // 存货周转：用 revenue / total_assets 近似周转率，反推天数
    // 注意：astock-data 无存货字段，用资产周转率作为代理指标
    let asset_turnover_latest = latest
        .revenue
        .zip(latest.total_assets)
        .filter(|(_, ta)| *ta > 0.0)
        .map(|(rev, ta)| rev / ta);
    let asset_turnover_prev = prev
        .and_then(|p| p.revenue.zip(p.total_assets))
        .filter(|(_, ta)| *ta > 0.0)
        .map(|(rev, ta)| rev / ta);
    let days_latest =
        asset_turnover_latest.map(|t| if t > 0.0 { 365.0 / t } else { 0.0 }).unwrap_or(0.0);
    let days_yoy_change = match (asset_turnover_latest, asset_turnover_prev) {
        (Some(cl), Some(pv)) => {
            let dl = if cl > 0.0 { 365.0 / cl } else { 0.0 };
            let dp = if pv > 0.0 { 365.0 / pv } else { 0.0 };
            dl - dp
        },
        _ => 0.0,
    };
    let inventory_signal = if days_yoy_change > 30.0 {
        "accumulating"
    } else if days_yoy_change < -30.0 {
        "decelerating"
    } else {
        "stable"
    };

    // 毛利率趋势
    let gm_latest = latest.gross_margin.unwrap_or(0.0);
    let gm_yoy_change = prev.and_then(|p| p.gross_margin).map(|g| gm_latest - g).unwrap_or(0.0);
    let gm_direction = if gm_yoy_change > 1.0 {
        "expanding"
    } else if gm_yoy_change < -1.0 {
        "contracting"
    } else {
        "stable"
    };

    // CapEx/折旧比
    let capex_dep_ratio = latest
        .capital_expenditure
        .zip(latest.total_assets)
        .filter(|(_, ta)| *ta > 0.0)
        .map(|(capex, ta)| (capex / (ta * 0.05)).clamp(0.0, 10.0))
        .unwrap_or(0.0);
    let capex_signal = if capex_dep_ratio >= 3.0 {
        "积极扩产"
    } else if capex_dep_ratio >= 1.5 {
        "温和扩张"
    } else if capex_dep_ratio >= 1.0 {
        "维持投入"
    } else {
        "收缩投入"
    };

    let result = json!({
        "stock_code": stock_code,
        "inventory_turnover": {
            "days_latest": round2(days_latest),
            "days_yoy_change": round2(days_yoy_change),
            "signal": inventory_signal
        },
        "gross_margin_trend": {
            "latest": round2(gm_latest),
            "yoy_change": round2(gm_yoy_change),
            "direction": gm_direction
        },
        "capex": {
            "capex_dep_ratio": round2(capex_dep_ratio),
            "signal": capex_signal
        },
        "summary": format!(
            "存货周转天数:{days_latest:.0}天(同比{days_yoy_change:+.0}天,{inventory_signal}) | 毛利率:{gm_latest:.1}%({gm_direction}) | CapEx/折旧:{capex_dep_ratio:.2}({capex_signal})"
        )
    });
    Ok(result)
}

/// compute_attention_score：关注度评分
/// 输出契约（mapper_prompt attention_metrics 期望）：
///   attention_score / coverage_change_3m / search_heat / relative_volume / consensus_gap
async fn compute_attention_score_impl(
    client: &crate::AStockClient,
    stock_code: &str,
) -> Result<serde_json::Value, String> {
    // 并行拉取 4 类数据
    let (reports, news, quote, visits) = tokio::join!(
        client.get_research_reports(stock_code),
        client.get_news(stock_code, 30),
        client.get_quote(stock_code),
        client.get_institutional_visits(stock_code),
    );
    let reports = reports.map_err(|e| e.to_string()).unwrap_or_default();
    let news = news.map_err(|e| e.to_string()).unwrap_or_default();
    let quote = quote.map_err(|e| e.to_string()).ok();
    let visits = visits.map_err(|e| e.to_string()).unwrap_or_default();

    // 研报覆盖度（最近 90 天）
    let now = chrono::Utc::now();
    let cutoff_90d = now - chrono::Duration::days(90);
    let recent_reports: Vec<_> = reports
        .iter()
        .filter(|r| {
            chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", r.publish_date))
                .map(|dt| dt.with_timezone(&chrono::Utc) > cutoff_90d)
                .unwrap_or(false)
        })
        .collect();
    let research_count = recent_reports.len();
    let coverage_change_3m = if research_count == 0 {
        "无机构覆盖".to_string()
    } else if research_count < 3 {
        format!("近 3 月 {research_count} 篇研报（低覆盖）")
    } else if research_count < 8 {
        format!("近 3 月 {research_count} 篇研报（正常）")
    } else {
        format!("近 3 月 {research_count} 篇研报（高覆盖）")
    };

    // 新闻热度
    let news_count_30d = news.len();
    let search_heat = if news_count_30d < 10 {
        "冷门"
    } else if news_count_30d < 30 {
        "正常"
    } else {
        "热门"
    };

    // 换手率相对市场（A 股均值约 2%）
    let turnover = quote.as_ref().map(|q| q.turnover_rate).unwrap_or(0.0);
    let relative_volume = if turnover < 1.0 {
        format!("低于均值 {:.0}%", (2.0 - turnover).max(0.0) * 50.0)
    } else if turnover < 3.0 {
        "正常".to_string()
    } else {
        format!("高于均值 {:.0}%", (turnover - 2.0) * 50.0)
    };

    // 共识差：研报评级 vs 当前价
    // 简化：用研报数量 + 平均目标价 vs 当前价判断
    let avg_target = reports.iter().filter_map(|r| r.target_price).next(); // 取最新一篇的目标价
    let current_price = quote.as_ref().map(|q| q.price).unwrap_or(0.0);
    let consensus_gap = match (avg_target, current_price > 0.0) {
        (Some(target), true) => {
            let gap_pct = (target - current_price) / current_price * 100.0;
            if gap_pct > 30.0 {
                "明显低估"
            } else if gap_pct > 10.0 {
                "合理偏低"
            } else if gap_pct > -10.0 {
                "合理"
            } else {
                "高估"
            }
        },
        _ => "无研报共识",
    };

    // 机构调研数
    let visit_count = visits.len();

    // 综合关注度评分 0-100（越低越冷门）
    // 权重：研报覆盖 35% + 新闻热度 25% + 换手率 25% + 机构调研 15%
    let research_score = (research_count as f64 * 8.0).min(40.0); // 0-40
    let news_score = (news_count_30d as f64 * 1.5).min(30.0); // 0-30
    let turnover_score = (turnover * 10.0).min(20.0); // 0-20
    let visit_score = (visit_count as f64 * 3.0).min(10.0); // 0-10
    let attention_score =
        (research_score + news_score + turnover_score + visit_score).round() as u32;

    let result = json!({
        "stock_code": stock_code,
        "attention_score": attention_score.min(100),
        "coverage_change_3m": coverage_change_3m,
        "search_heat": search_heat,
        "relative_volume": relative_volume,
        "consensus_gap": consensus_gap,
        "components": {
            "research_coverage": research_count,
            "news_count_30d": news_count_30d,
            "visit_count": visit_count,
            "turnover_rate_pct": round2(turnover),
            "current_price": round2(current_price),
            "avg_target_price": avg_target.map(round2)
        },
        "summary": format!(
            "关注度评分:{}/100 | 研报:{research_count}篇 | 新闻:{news_count_30d}条 | 换手率:{turnover:.2}% | 机构调研:{visit_count}次",
            attention_score.min(100)
        )
    });
    Ok(result)
}

/// check_exit_signals：退出信号检查
/// 输出契约（mapper_prompt exit_signals 期望）：
///   technology_disruption_risk / capacity_oversupply_risk / new_entrant_risk
///   demand_slowdown_risk / overall_exit_urgency
async fn check_exit_signals_impl(
    client: &crate::AStockClient,
    stock_code: &str,
    entry_price: Option<f64>,
    stop_loss_price: Option<f64>,
) -> Result<serde_json::Value, String> {
    let (financials, news, quote) = tokio::join!(
        client.get_financials(stock_code),
        client.get_news(stock_code, 30),
        client.get_quote(stock_code),
    );
    let financials = financials.map_err(|e| e.to_string()).unwrap_or_default();
    let news = news.map_err(|e| e.to_string()).unwrap_or_default();
    let quote = quote.map_err(|e| e.to_string()).ok();

    let latest = financials.first();
    let prev = financials.get(1);

    // 1. 技术替代风险：扫描近期新闻关键词
    let disruption_keywords = ["替代", "颠覆", "新技术", "突破", "新一代", "替代品", "颠覆性"];
    let disruption_hits = news
        .iter()
        .filter(|n| {
            disruption_keywords.iter().any(|k| n.title.contains(k) || n.summary.contains(k))
        })
        .count();
    let technology_disruption_risk = if disruption_hits >= 3 {
        "高 - 近期有技术替代报道"
    } else if disruption_hits >= 1 {
        "中 - 个别替代相关新闻"
    } else {
        "低 - 暂无技术替代报道"
    };

    // 2. 产能过剩风险：存货周转天数变化 + CapEx 强度
    let asset_turnover_change = match (latest, prev) {
        (Some(l), Some(p)) => {
            let lt =
                l.revenue.zip(l.total_assets).filter(|(_, ta)| *ta > 0.0).map(|(r, ta)| r / ta);
            let pt =
                p.revenue.zip(p.total_assets).filter(|(_, ta)| *ta > 0.0).map(|(r, ta)| r / ta);
            match (lt, pt) {
                (Some(l), Some(p)) => Some(l - p),
                _ => None,
            }
        },
        _ => None,
    };
    let capex_dep_ratio = latest
        .and_then(|l| {
            l.capital_expenditure
                .zip(l.total_assets)
                .filter(|(_, ta)| *ta > 0.0)
                .map(|(capex, ta)| (capex / (ta * 0.05)).clamp(0.0, 10.0))
        })
        .unwrap_or(0.0);
    let capacity_oversupply_risk = match asset_turnover_change {
        Some(change) if change < -0.1 && capex_dep_ratio > 2.0 => "高 - 周转率下滑且 CapEx 高强度",
        Some(change) if change < -0.1 => "中 - 周转率下滑",
        Some(_) if capex_dep_ratio > 3.0 => "中 - CapEx 强度偏高，关注产能释放",
        _ => "低 - 周转率稳定",
    };

    // 3. 新进入者风险：壁垒评估（ROE + 毛利率）
    let roe = latest.and_then(|f| f.roe).unwrap_or(0.0);
    let gm = latest.and_then(|f| f.gross_margin).unwrap_or(0.0);
    let debt = latest.and_then(|f| f.debt_ratio).unwrap_or(50.0);
    let new_entrant_risk = if gm > 50.0 && roe > 15.0 && debt < 40.0 {
        "低 - 高毛利+高ROE+低负债，行业壁垒高"
    } else if gm > 30.0 || roe > 10.0 {
        "中 - 中等壁垒"
    } else {
        "高 - 低毛利/低ROE，壁垒薄弱"
    };

    // 4. 需求放缓风险：营收同比
    let revenue_yoy = latest.and_then(|f| f.revenue_yoy).unwrap_or(0.0);
    let profit_yoy = latest.and_then(|f| f.profit_yoy).unwrap_or(0.0);
    let demand_slowdown_risk = if revenue_yoy < -10.0 || profit_yoy < -20.0 {
        "高 - 营收或利润显著下滑"
    } else if revenue_yoy < 0.0 || profit_yoy < 0.0 {
        "中 - 营收或利润负增长"
    } else if revenue_yoy < 10.0 {
        "低 - 增速放缓但未负增长"
    } else {
        "低 - 营收稳健增长"
    };

    // 5. 价格止损触发
    let current_price = quote.as_ref().map(|q| q.price).unwrap_or(0.0);
    let price_stop_triggered = match (entry_price, stop_loss_price, current_price > 0.0) {
        (Some(entry), Some(stop), true) => {
            // 触发条件：当前价 <= 止损价 或 当前价 < 买入价 * 0.85（默认 -15%）
            let dynamic_stop = if stop > 0.0 { stop } else { entry * 0.85 };
            current_price <= dynamic_stop
        },
        _ => false,
    };

    // 综合退出紧迫度
    let risk_count = [
        technology_disruption_risk.starts_with("高"),
        capacity_oversupply_risk.starts_with("高"),
        new_entrant_risk.starts_with("高"),
        demand_slowdown_risk.starts_with("高"),
    ]
    .iter()
    .filter(|&&x| x)
    .count();
    let medium_count = [
        technology_disruption_risk.starts_with("中"),
        capacity_oversupply_risk.starts_with("中"),
        new_entrant_risk.starts_with("中"),
        demand_slowdown_risk.starts_with("中"),
    ]
    .iter()
    .filter(|&&x| x)
    .count();
    let overall_exit_urgency = if price_stop_triggered || risk_count >= 2 {
        "exit_now"
    } else if risk_count >= 1 || medium_count >= 2 {
        "caution"
    } else if medium_count >= 1 {
        "watch"
    } else {
        "no_urgency"
    };

    let result = json!({
        "stock_code": stock_code,
        "technology_disruption_risk": technology_disruption_risk,
        "capacity_oversupply_risk": capacity_oversupply_risk,
        "new_entrant_risk": new_entrant_risk,
        "demand_slowdown_risk": demand_slowdown_risk,
        "overall_exit_urgency": overall_exit_urgency,
        "price_check": {
            "current_price": round2(current_price),
            "entry_price": entry_price.map(round2),
            "stop_loss_price": stop_loss_price.map(round2),
            "stop_triggered": price_stop_triggered
        },
        "summary": format!(
            "退出紧迫度:{overall_exit_urgency} | 高风险{risk_count}项, 中风险{medium_count}项"
        )
    });
    Ok(result)
}

/// verify_catalysts：催化剂验证
async fn verify_catalysts_impl(
    client: &crate::AStockClient,
    stock_code: &str,
    catalysts: &[String],
) -> Result<serde_json::Value, String> {
    let news = client.get_news(stock_code, 50).await.map_err(|e| e.to_string())?;

    let verified: Vec<serde_json::Value> = catalysts
        .iter()
        .map(|c| {
            // 从催化剂描述中提取关键词（按空格/逗号分割）
            let keywords: Vec<&str> =
                c.split([' ', ',', '，', '/']).filter(|s| s.chars().count() >= 2).collect();
            // 在新闻标题中匹配关键词
            let hits: Vec<&str> = news
                .iter()
                .filter(|n| keywords.iter().any(|k| n.title.contains(k) || n.summary.contains(k)))
                .map(|n| n.title.as_str())
                .collect();
            let hit_count = hits.len();
            let status = if hit_count >= 3 {
                "confirmed"
            } else if hit_count >= 1 {
                "partial"
            } else {
                "unverified"
            };
            let confidence = if hit_count >= 3 {
                85
            } else if hit_count >= 1 {
                60
            } else {
                30
            };
            json!({
                "catalyst": c,
                "status": status,
                "evidence": hits.iter().take(3).collect::<Vec<_>>(),
                "hit_count": hit_count,
                "confidence": confidence
            })
        })
        .collect();
    let unverified_count =
        verified.iter().filter(|v| v["status"].as_str() == Some("unverified")).count() as u32;
    let total = verified.len() as u32;
    let confirmed = total - unverified_count;

    let result = json!({
        "stock_code": stock_code,
        "verified": verified,
        "total_count": total,
        "confirmed_count": confirmed,
        "unverified_count": unverified_count,
        "summary": format!("{confirmed}/{total} 催化剂已验证（{unverified_count} 未验证）")
    });
    Ok(result)
}

/// compute_serenity_performance：Serenity 候选推荐后表现
async fn compute_serenity_performance_impl(
    client: &crate::AStockClient,
    stock_code: &str,
    recommend_date: &str,
) -> Result<serde_json::Value, String> {
    if recommend_date.is_empty() {
        return Err("compute_serenity_performance 缺少 recommend_date 参数".to_string());
    }
    let recommend_dt = chrono::NaiveDate::parse_from_str(recommend_date, "%Y-%m-%d")
        .map_err(|e| format!("recommend_date 格式错误（应为 YYYY-MM-DD）: {e}"))?;
    let today = chrono::Utc::now().date_naive();
    let holding_days = (today - recommend_dt).num_days();

    if holding_days <= 0 {
        return Ok(json!({
            "stock_code": stock_code,
            "recommend_date": recommend_date,
            "return_pct": 0.0,
            "outperform_pct": 0.0,
            "holding_days": 0,
            "hit_target": false,
            "hit_stop": false,
            "status": "future_date"
        }));
    }

    // 拉取推荐日至今的 K 线（日 K，足够覆盖 1 年内）
    let limit = (holding_days as u32 + 30).min(500);
    let klines = client.get_klines(stock_code, "daily", limit).await.map_err(|e| e.to_string())?;
    // 找到推荐日附近的 K 线
    let baseline = klines.iter().find(|k| k.date.starts_with(recommend_date));
    let latest = klines.last();
    let return_pct = match (baseline, latest) {
        (Some(base), Some(last)) if base.close > 0.0 => {
            (last.close - base.close) / base.close * 100.0
        },
        _ => 0.0,
    };

    // 大盘基准（上证指数）
    let index_quotes = client.get_index_quotes().await.map_err(|e| e.to_string())?;
    let sh_index = index_quotes.iter().find(|q| q.code.starts_with("000001"));
    let index_change_pct = sh_index.map(|q| q.change_pct).unwrap_or(0.0);
    // outperform_pct = return_pct - 当日大盘涨跌幅
    // 注：这里用今日大盘涨跌幅作为近似（推荐日至今的累计需要历史 K 线，
    //     简化处理：仅用今日大盘涨跌幅作为相对参考）
    let outperform_pct = return_pct - index_change_pct;

    // 止盈止损触发（默认 +30% 止盈，-15% 止损）
    let hit_target = return_pct >= 30.0;
    let hit_stop = return_pct <= -15.0;
    let status = if hit_target {
        "hit_target"
    } else if hit_stop {
        "hit_stop"
    } else if holding_days > 90 {
        "expired"
    } else {
        "active"
    };

    let result = json!({
        "stock_code": stock_code,
        "recommend_date": recommend_date,
        "return_pct": round2(return_pct),
        "outperform_pct": round2(outperform_pct),
        "holding_days": holding_days,
        "hit_target": hit_target,
        "hit_stop": hit_stop,
        "status": status,
        "baseline_price": baseline.map(|b| round2(b.close)),
        "latest_price": latest.map(|l| round2(l.close)),
        "summary": format!(
            "持有{holding_days}天 | 涨幅:{return_pct:.2}% | 超额:{outperform_pct:+.2}% | 状态:{status}"
        )
    });
    Ok(result)
}

/// optimize_attention_weights：基于历史样本调优关注度权重
/// 纯算法实现，不调 API
fn optimize_attention_weights_impl(samples: &Vec<serde_json::Value>) -> serde_json::Value {
    let sample_count = samples.len();
    if sample_count == 0 {
        return json!({
            "weights": {
                "coverage_weight": 0.35,
                "search_weight": 0.25,
                "volume_weight": 0.25,
                "gap_weight": 0.15
            },
            "expected_accuracy": 0.5,
            "sample_count": 0,
            "summary": "无样本输入，返回默认权重"
        });
    }

    // 简化策略：根据 attention_score 与 actual_return_pct 的相关性反推权重
    // 高 attention_score 应对应低 actual_return（Serenity 假说：低关注度 → 高弹性）
    // 用样本统计验证假说强度
    let mut high_attn_returns: Vec<f64> = Vec::new();
    let mut low_attn_returns: Vec<f64> = Vec::new();
    let mut total_score = 0.0;
    let mut total_return = 0.0;
    for s in samples {
        let attn = s["attention_score"].as_f64().unwrap_or(50.0);
        let ret = s["actual_return_pct"].as_f64().unwrap_or(0.0);
        total_score += attn;
        total_return += ret;
        if attn >= 50.0 {
            high_attn_returns.push(ret);
        } else {
            low_attn_returns.push(ret);
        }
    }
    let _avg_attn = total_score / sample_count as f64;
    let avg_return = total_return / sample_count as f64;
    let low_avg = if !low_attn_returns.is_empty() {
        low_attn_returns.iter().sum::<f64>() / low_attn_returns.len() as f64
    } else {
        0.0
    };
    let high_avg = if !high_attn_returns.is_empty() {
        high_attn_returns.iter().sum::<f64>() / high_attn_returns.len() as f64
    } else {
        0.0
    };

    // 假说验证：低关注度组平均收益是否高于高关注度组
    let hypothesis_valid = low_avg > high_avg;
    let spread = (low_avg - high_avg).abs();
    // 假说越显著，coverage_weight 越大
    let coverage_weight = if hypothesis_valid && spread > 5.0 {
        0.45
    } else if hypothesis_valid {
        0.35
    } else {
        0.25
    };
    let search_weight = 0.20;
    let volume_weight = 0.20;
    let gap_weight = 1.0 - coverage_weight - search_weight - volume_weight;

    // 期望准确率：用样本均值偏离度近似（粗略指标）
    let return_std = {
        let mean = avg_return;
        let var = samples
            .iter()
            .filter_map(|s| s["actual_return_pct"].as_f64())
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / sample_count as f64;
        var.sqrt()
    };
    let expected_accuracy = if return_std > 0.0 {
        (1.0 / (1.0 + return_std / 20.0)).clamp(0.3, 0.9)
    } else {
        0.5
    };

    json!({
        "weights": {
            "coverage_weight": round2(coverage_weight),
            "search_weight": round2(search_weight),
            "volume_weight": round2(volume_weight),
            "gap_weight": round2(gap_weight)
        },
        "expected_accuracy": round2(expected_accuracy),
        "sample_count": sample_count,
        "hypothesis_validation": {
            "low_attention_avg_return_pct": round2(low_avg),
            "high_attention_avg_return_pct": round2(high_avg),
            "hypothesis_valid": hypothesis_valid,
            "spread_pct": round2(spread)
        },
        "summary": format!(
            "样本:{sample_count} | 低关注度组均值收益:{low_avg:.2}% | 高关注度组:{high_avg:.2}% | 假说验证:{hypothesis_valid}"
        )
    })
}
