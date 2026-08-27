//! 股票分析专家与工作流模板种子化。
//!
//! 子模块：
//! - seed_stock_analysis: 股票分析主工作流模板种子
//! - seed_serenity: Serenity 瓶颈筛选工作流模板种子
//! - seed_daily_market_events: G4 每日市场主线提炼工作流模板种子
//! - seed_screenshot_portfolio_diagnosis: G6 截图持仓诊断工作流模板种子
//! - seed_news_cross_market: G3.3 新闻→跨市场传导分析工作流模板种子
//!
//! 注：Multi-Agent 固定角色（analyst/implementer/reviewer）种子化已迁移到上游
//! `commands/multi_agent_setup/seed_multi_agent_roles`，本模块不再负责。

pub mod seed_concept_index;
pub mod seed_daily_market_events;
pub mod seed_news_cross_market;
pub mod seed_screenshot_portfolio_diagnosis;
pub mod seed_serenity;
pub mod seed_stock_analysis;
pub mod seed_variables;

// 仅测试构建：seed 工具声明 ↔ 运行时解析空间 一致性校验
#[cfg(test)]
mod seed_consistency_tests;

// 股票分析专家/角色/Profile 自动种子化到 agency_experts/agent_roles/agent_profiles 表。
// 使用 include_str! 编译期嵌入 .md 内容，打包后无需文件 I/O。

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_setup;
use axagent_dao::repo;
use seed_daily_market_events::seed_daily_market_events_template;
use seed_screenshot_portfolio_diagnosis::seed_screenshot_portfolio_diagnosis_template;
use seed_serenity::seed_serenity_screening_workflow_template;
use seed_stock_analysis::seed_stock_analysis_workflow_template;

/// 编译期嵌入的专家提示词（include_str 确保打包后可用）
const EMBEDDED_PROMPTS: &[(&str, &str)] = &[
    ("market-analyst", include_str!("../../../agency_experts/stock-analysis/market-analyst.md")),
    (
        "sentiment-analyst",
        include_str!("../../../agency_experts/stock-analysis/sentiment-analyst.md"),
    ),
    ("news-analyst", include_str!("../../../agency_experts/stock-analysis/news-analyst.md")),
    (
        "fundamentals-analyst",
        include_str!("../../../agency_experts/stock-analysis/fundamentals-analyst.md"),
    ),
    ("policy-analyst", include_str!("../../../agency_experts/stock-analysis/policy-analyst.md")),
    (
        "hot-money-tracker",
        include_str!("../../../agency_experts/stock-analysis/hot-money-tracker.md"),
    ),
    ("lockup-watcher", include_str!("../../../agency_experts/stock-analysis/lockup-watcher.md")),
    (
        "research-analyst",
        include_str!("../../../agency_experts/stock-analysis/research-analyst.md"),
    ),
    ("sector-analyst", include_str!("../../../agency_experts/stock-analysis/sector-analyst.md")),
    ("bull-researcher", include_str!("../../../agency_experts/stock-analysis/bull-researcher.md")),
    ("bear-researcher", include_str!("../../../agency_experts/stock-analysis/bear-researcher.md")),
    ("bull-r2", include_str!("../../../agency_experts/stock-analysis/bull-r2.md")),
    ("bear-r2", include_str!("../../../agency_experts/stock-analysis/bear-r2.md")),
    ("bull-r3", include_str!("../../../agency_experts/stock-analysis/bull-r3.md")),
    ("bear-r3", include_str!("../../../agency_experts/stock-analysis/bear-r3.md")),
    (
        "aggressive-debator",
        include_str!("../../../agency_experts/stock-analysis/aggressive-debator.md"),
    ),
    (
        "conservative-debator",
        include_str!("../../../agency_experts/stock-analysis/conservative-debator.md"),
    ),
    ("neutral-debator", include_str!("../../../agency_experts/stock-analysis/neutral-debator.md")),
    (
        "research-manager",
        include_str!("../../../agency_experts/stock-analysis/research-manager.md"),
    ),
    ("trader", include_str!("../../../agency_experts/stock-analysis/trader.md")),
    (
        "value-investor",
        include_str!("../../../agency_experts/stock-analysis/custom/value-investor.md"),
    ),
    (
        "data-quality-inspector",
        include_str!("../../../agency_experts/stock-analysis/data-quality-inspector.md"),
    ),
    (
        "quality-fallback",
        include_str!("../../../agency_experts/stock-analysis/quality-fallback.md"),
    ),
    ("rule-checker", include_str!("../../../agency_experts/stock-analysis/rule-checker.md")),
    (
        "catalyst-analyst",
        include_str!("../../../agency_experts/stock-analysis/catalyst-analyst.md"),
    ),
    (
        "debate-convergence",
        include_str!("../../../agency_experts/stock-analysis/debate-convergence.md"),
    ),
    (
        "risk-convergence",
        include_str!("../../../agency_experts/stock-analysis/risk-convergence.md"),
    ),
    ("reflection", include_str!("../../../agency_experts/stock-analysis/reflection.md")),
    // ── Serenity 瓶颈分析 4 专家 ──
    ("trend-scanner", include_str!("../../../agency_experts/stock-analysis/trend-scanner.md")),
    (
        "chain-decomposer",
        include_str!("../../../agency_experts/stock-analysis/chain-decomposer.md"),
    ),
    (
        "chokepoint-identifier",
        include_str!("../../../agency_experts/stock-analysis/chokepoint-identifier.md"),
    ),
    (
        "candidate-mapper",
        include_str!("../../../agency_experts/stock-analysis/candidate-mapper.md"),
    ),
    // ── P2: 借鉴 TradingAgents 的新分析师 ──
    (
        "social-media-analyst",
        include_str!("../../../agency_experts/stock-analysis/social-media-analyst.md"),
    ),
    (
        "volume-price-analyst",
        include_str!("../../../agency_experts/stock-analysis/volume-price-analyst.md"),
    ),
    // ── 简化模板升级：3 个新专家 ──
    (
        "market-synthesizer",
        include_str!("../../../agency_experts/stock-analysis/market-synthesizer.md"),
    ),
    (
        "industry-chain-analyzer",
        include_str!("../../../agency_experts/stock-analysis/industry-chain-analyzer.md"),
    ),
    (
        "screenshot-diagnoser",
        include_str!("../../../agency_experts/stock-analysis/screenshot-diagnoser.md"),
    ),
    // ── 事件驱动模板：仓位规划与止损复查 ──
    (
        "position-planner",
        include_str!("../../../agency_experts/stock-analysis/position-planner.md"),
    ),
    (
        "stop-loss-reviewer",
        include_str!("../../../agency_experts/stock-analysis/stop-loss-reviewer.md"),
    ),
];

const EXPERT_ROLE_MAP: &[(&str, &str)] = &[
    ("market-analyst", "stock-analyst"),
    ("sentiment-analyst", "stock-analyst"),
    ("news-analyst", "stock-analyst"),
    ("fundamentals-analyst", "stock-analyst"),
    ("policy-analyst", "stock-analyst"),
    ("hot-money-tracker", "stock-analyst"),
    ("lockup-watcher", "stock-analyst"),
    ("research-analyst", "stock-analyst"),
    ("sector-analyst", "stock-analyst"),
    ("bull-researcher", "debater"),
    ("bear-researcher", "debater"),
    ("bull-r2", "debater"),
    ("bear-r2", "debater"),
    ("bull-r3", "debater"),
    ("bear-r3", "debater"),
    ("aggressive-debator", "risk-evaluator"),
    ("conservative-debator", "risk-evaluator"),
    ("neutral-debator", "risk-evaluator"),
    ("research-manager", "decision-maker"),
    ("trader", "trader"),
    ("value-investor", "stock-analyst"),
    ("data-quality-inspector", "stock-analyst"),
    ("quality-fallback", "decision-maker"),
    ("rule-checker", "risk-evaluator"),
    ("catalyst-analyst", "stock-analyst"),
    ("debate-convergence", "debater"),
    ("risk-convergence", "risk-evaluator"),
    ("reflection", "decision-maker"),
    // ── Serenity 瓶颈分析师 ──
    ("trend-scanner", "stock-analyst"),
    ("chain-decomposer", "stock-analyst"),
    ("chokepoint-identifier", "stock-analyst"),
    ("candidate-mapper", "stock-analyst"),
    ("social-media-analyst", "stock-analyst"),
    ("volume-price-analyst", "stock-analyst"),
    // ── 简化模板升级：3 个新专家角色映射 ──
    ("market-synthesizer", "stock-analyst"),
    ("industry-chain-analyzer", "stock-analyst"),
    ("screenshot-diagnoser", "stock-analyst"),
    // ── 事件驱动模板：仓位规划与止损复查角色映射 ──
    ("position-planner", "decision-maker"),
    ("stop-loss-reviewer", "decision-maker"),
];

struct StockRoleDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    system_prompt: &'static str,
    max_concurrent: i32,
    timeout_seconds: i64,
}

/// AxInvest 专属角色 — 证券投资负责人。
///
/// 本角色（stock-investment-lead）seed 进 agent_roles，作为股票专家 profile 的
/// agent_role 引用，其 system_prompt 注入最外层身份（投资决策责任与合规边界）。
const STOCK_AGENT_ROLE_ID: &str = "stock-investment-lead";

struct StockAgentRoleDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    responsibilities: &'static [&'static str],
    decision_authority: &'static str,
    required_certifications: &'static [&'static str],
    active_domains: &'static [&'static str],
    system_prompt: &'static str,
    icon: &'static str,
    color: &'static str,
}

const STOCK_AGENT_ROLE: StockAgentRoleDef = StockAgentRoleDef {
    id: STOCK_AGENT_ROLE_ID,
    name: "证券投资负责人",
    description: "领导多专家团队进行 A 股证券投资分析与决策，对决策合规性与风险调整后收益负责",
    responsibilities: &[
        "组织多专家团队完成 A 股标的的多维度分析",
        "评估投资风险与仓位边界，制定风险调整后收益最大化方案",
        "决策买入 / 持有 / 卖出动作，维护决策链路的可追溯性",
        "确保分析过程遵循监管要求与合规边界",
    ],
    decision_authority: r#"{"max_position_pct":100,"scopes":["stock-analysis","portfolio-mgmt","risk-assessment"]}"#,
    required_certifications: &["证券从业资格", "5 年 A 股研究经验"],
    // 修复: 原 "stock-analysis"/"finance" 均非合法 ToolDomain 字符串（parse_domain_str
    // 只认 core/general/devops/ai_media/invest/opc），解析为全集 → 分析师 AgentNode 经
    // get_chat_tools_for_domains 只拿到 MCP 工具，丢失所有非-MCP 本地工具（含 invest 域）。
    // 改为合法域: invest（投资域）+ core/general（通用能力），使领域过滤真正生效且不过窄。
    active_domains: &["invest", "core", "general"],
    // 分层原则：
    // - Role: 身份 + 职责 + 权限 + 合规边界（通用、稳定）
    // - Expert: 方法论 + 评分体系 + 输出格式（专业、可演进）
    system_prompt: "你是证券投资负责人，领导多专家团队进行 A 股投资分析与决策。\
    \n\n职责：组织多维度分析，评估风险与收益，决策买入/持有/卖出。\
    \n权限：对所有投资建议承担可追溯的合规责任。\
    \n合规：杜绝内幕信息与市场操纵，所有结论基于公开数据。\
    \n目标：以风险调整后收益最大化为目标。",
    icon: "📈",
    color: "#dc2626",
};

/// 投研办公室子岗位 — 对应 INVESTMENT_OFFICE_TEMPLATE 中的 6 个房间。
///
/// 这些岗位作为 `stock-investment-lead` 的下属存在
/// （reports_to = STOCK_AGENT_ROLE_ID），用于：
/// - AddMemberModal 的角色下拉中可按房间选择对应角色
/// - 角色 system_prompt 注入到 dispatcher 路由上下文，引导 LLM 将股票相关
///   消息路由到合适房间（如「查询行情」→ data-lead，「下单」→ trading-lead）
///
/// 颜色与 sceneTemplates.ts 中的房间 color 保持一致，前端卡片与 Sprite
/// 渲染时通过 role 反查颜色，无需再维护 ROLE_COLORS 映射表。
const STOCK_AGENT_SUB_ROLES: &[StockAgentRoleDef] = &[
    StockAgentRoleDef {
        id: "stock-research-lead",
        name: "投研负责人",
        description: "领导行业研究、基本面分析与研报撰写，对应办公室「投研室」",
        responsibilities: &[
            "组织行业景气度跟踪与上下游调研",
            "统筹基本面分析（财务 / 估值 / 成长性）",
            "撰写深度研报并标注证据链与置信度",
        ],
        decision_authority: r#"{"max_position_pct":50,"scopes":["research","fundamental-analysis"]}"#,
        required_certifications: &["证券从业资格", "3 年行业研究经验"],
        active_domains: &["invest", "core"],
        system_prompt: "你是投研负责人，专注行业景气度跟踪、基本面深度分析与研报撰写。所有结论必须标注证据来源（公告/财报/调研/数据接口）与置信度（high/medium/low）。对不确定性显式标注 data_gaps，禁止编造未公开数据。在办公室中常驻「投研室」，对接消息涉及行业研究、基本面、研报、公告解读时主动接手。",
        icon: "🔬",
        color: "#1677ff",
    },
    StockAgentRoleDef {
        id: "stock-data-lead",
        name: "数据负责人",
        description: "对接 astock-data 行情/财务/新闻接口，对应办公室「数据室」",
        responsibilities: &[
            "对接 astock-data MCP 工具集（行情/K线/财务/新闻）",
            "校验数据质量并标注 dqi_score",
            "为其他角色提供数据上下文与回测样本",
        ],
        decision_authority: r#"{"max_position_pct":0,"scopes":["data-query","data-quality"]}"#,
        required_certifications: &["证券从业资格", "熟悉量化数据接口"],
        active_domains: &["invest", "core", "general"],
        system_prompt: "你是数据负责人，对接 astock-data MCP 工具集，提供行情、K线、财务、新闻等数据查询与质量校验。返回结果必须包含数据时间戳、来源、dqi_score；数据缺失或异常时显式标注 untrusted=true 并触发 weights collapse。在办公室中常驻「数据室」，消息涉及查询行情/财务/新闻数据时主动接手。",
        icon: "📡",
        color: "#13c2c2",
    },
    StockAgentRoleDef {
        id: "stock-meeting-host",
        name: "晨会主持",
        description: "组织晨会、投研会议与多空辩论，对应办公室「会议室」",
        responsibilities: &[
            "组织每日晨会议题与市场主线提炼",
            "主持多空辩论与同行评估",
            "汇总分歧并形成会议纪要",
        ],
        decision_authority: r#"{"max_position_pct":0,"scopes":["meeting","debate"]}"#,
        required_certifications: &["证券从业资格", "2 年投研经验"],
        active_domains: &["invest", "core"],
        system_prompt: "你是晨会主持，组织每日晨会议题、市场主线提炼与多空辩论。所有议题须基于已验证数据，对分歧观点要求辩手给出可证伪的判定条件。会议纪要须包含：议题 / 主线 / 分歧 / 多空观点 / 决议。在办公室中常驻「会议室」，消息涉及晨会议题、主线提炼、辩论组织时主动接手。",
        icon: "🎤",
        color: "#722ed1",
    },
    StockAgentRoleDef {
        id: "stock-strategy-lead",
        name: "策略负责人",
        description: "策略研发、回测与组合优化，对应办公室「策略室」",
        responsibilities: &[
            "研发并验证投资策略（趋势/价值/量化）",
            "对接 quant crate 进行回测与 walkforward 验证",
            "输出策略列表与建议仓位上限",
        ],
        decision_authority: r#"{"max_position_pct":80,"scopes":["strategy","backtest","portfolio"]}"#,
        required_certifications: &["证券从业资格", "3 年策略研发经验"],
        active_domains: &["invest", "core"],
        system_prompt: "你是策略负责人，负责研发、回测与组合优化。对接 quant crate 进行 walkforward 验证，输出策略列表与建议仓位上限。所有策略须附回测报告（年化收益/最大回撤/夏普/胜率），禁止推荐未回测的策略。在办公室中常驻「策略室」，消息涉及策略研发、回测、组合优化时主动接手。",
        icon: "🎯",
        color: "#eb2f96",
    },
    StockAgentRoleDef {
        id: "stock-trading-lead",
        name: "交易负责人",
        description: "执行下单、止损止盈与 T+1 涨跌停合规检查，对应办公室「交易室」",
        responsibilities: &[
            "制定入场/出场/分批方案",
            "执行 T+1、涨跌停、停牌合规检查",
            "对接 paper_portfolio 模拟成交记录",
        ],
        decision_authority: r#"{"max_position_pct":100,"scopes":["trading","execution","paper-portfolio"]}"#,
        required_certifications: &["证券从业资格", "熟悉 A 股交易规则"],
        active_domains: &["invest", "core"],
        system_prompt: "你是交易负责人，制定入场/出场/分批方案并执行 T+1、涨跌停、停牌合规检查。所有交易指令须附合规检查结果与 paper_portfolio 记录。禁止违反 T+1 与涨跌停规则。在办公室中常驻「交易室」（投研办公室默认房间），消息涉及下单、改单、撤单、止损止盈时主动接手。",
        icon: "⚡",
        color: "#f5222d",
    },
    StockAgentRoleDef {
        id: "stock-risk-lead",
        name: "风控负责人",
        description: "风险评估、压力测试与合规边界，对应办公室「风控室」",
        responsibilities: &[
            "识别投资风险（系统性/行业/个股）并量化评估",
            "组织压力测试与情景分析",
            "对违规操作触发 weights collapse 与仓位上限",
        ],
        decision_authority: r#"{"max_position_pct":100,"scopes":["risk","stress-test","compliance"]}"#,
        required_certifications: &["证券从业资格", "FRM 或 3 年风控经验"],
        active_domains: &["invest", "core"],
        system_prompt: "你是风控负责人，识别投资风险并量化评估，组织压力测试与情景分析。对数据质量 F 级（dqi_score<25）触发 weights collapse：position_pct=0、confidence×0.5、action 降级为「观望」。对所有建议保留合规审计追溯链。在办公室中常驻「风控室」，消息涉及回撤、压测、行业暴露、相关性、合规时主动接手。",
        icon: "🛡️",
        color: "#fa8c16",
    },
];

const STOCK_ROLES: &[StockRoleDef] = &[
    StockRoleDef {
        id: "stock-analyst",
        name: "股票分析师",
        description: "A股多维分析",
        system_prompt: "你是专业的 A 股分析师，基于行情数据、财务数据、新闻资讯等对股票进行深度分析。",
        // 修复 Defect #6: 提升到 14 以容纳 11 个 a-* + value-investor + data-quality-inspector + catalyst-analyst
        // + social-media-analyst + volume-price-analyst（共 14 个 stock-analyst 角色节点），留 1 槽位余量。
        max_concurrent: 15,
        timeout_seconds: 600,
    },
    StockRoleDef {
        id: "debater",
        name: "辩论研究员",
        description: "多空辩论",
        system_prompt: "你是投资辩论研究员，从多/空角度审视分析结论。",
        max_concurrent: 2,
        timeout_seconds: 300,
    },
    StockRoleDef {
        id: "risk-evaluator",
        name: "风险评估师",
        description: "风险评估",
        system_prompt: "你是风险评估师，识别投资中的各类风险并量化评估。",
        max_concurrent: 4,
        timeout_seconds: 300,
    },
    StockRoleDef {
        id: "trader",
        name: "交易员",
        description: "制定交易执行方案",
        system_prompt: "你是 A 股交易员，制定具体入场/出场/仓位方案，遵守 T+1、涨跌停规则。",
        max_concurrent: 1,
        timeout_seconds: 300,
    },
    StockRoleDef {
        id: "decision-maker",
        name: "决策者",
        description: "最终投资决策",
        system_prompt: "你是投资决策者，综合所有分析结果做出最终决策。",
        max_concurrent: 1,
        timeout_seconds: 300,
    },
];

/// Profile → 工具映射（模块级，模板 seed 和 agent_profiles seed 共用）
pub(crate) static PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "market-analyst",
        &[
            // P0 修复(2026-07-22): 移除 get_stock_kline——上游 t-market-data 已获取并通过
            // context_sources 注入，LLM 重新调用会重复获取数据 + 可能传入空 stock_code。
            "get_stock_quote",
            "compute_scoring",
            "compute_kdj",
            "compute_obv",
            "search_stock",
        ],
    ),
    (
        "sentiment-analyst",
        &[
            // P0 修复(2026-07-22): 移除 get_social_sentiment——上游 t-sentiment-data 已获取。
            // 保留 get_stock_news/get_stock_money_flow：a-sentiment 的 context_sources 只有
            // t-sentiment-data，看不到 t-news-data/t-hotmoney-data 的数据，LLM 主动调用是合理补充。
            "get_stock_news",
            "get_stock_money_flow",
            "get_stock_option_pcr",
            "get_stock_dragon_tiger",
            // 2026-08-01 恢复 get_north_bound_flow：净流入停披但成交额仍披露（v3 返回成交额，
            // timestamp 标注非净流入），北向成交活跃度仍是资金面信号。
            "get_north_bound_flow",
            "get_stock_margin_data",
            "get_stock_quote",
            "search_stock",
        ],
    ),
    (
        "news-analyst",
        &[
            // P0 修复(2026-07-22): 移除 get_stock_news——上游 t-news-data 已获取。
            "get_stock_announcements",
            "get_cls_flash",
            "get_stock_option_pcr",
            "search_stock",
        ],
    ),
    (
        "fundamentals-analyst",
        &[
            // V63 修复(2026-07-23): 移除 get_stock_financials——上游 t-fundamentals-data
            // 用 get_fundamentals_report_markdown 已预聚合所有关键财务指标（含
            // PE/PB/ROE/毛利率/净利率/资产负债率/FCF收益率/同比增速 + 商誉/应收账款）。
            // LLM 再调 get_stock_financials 会返回多期原始财报 JSON（每期 ~20 字段），
            // 与预聚合报告数据重复 → input tokens 膨胀 → output 超 max_tokens 截断 →
            // VERDICT 标签被切掉。与 a-market-analyst 移除 get_stock_kline 同一模式。
            "compute_valuation",
            "get_stock_consensus_eps",
            "get_stock_institutional_visits",
            "get_stock_peers",
            "search_stock",
        ],
    ),
    ("policy-analyst", &["search_news", "get_stock_news", "get_cls_flash", "search_stock"]),
    (
        "hot-money-tracker",
        &[
            // P0 修复(2026-07-22): 移除 get_stock_money_flow——上游 t-hotmoney-data 已获取。
            // 2026-07-25 修复: 补充 get_stock_margin_data（融资融券）——lockup-watcher
            // 虽也有此工具，但 hot-money-tracker 的分析需要融资融券作为真金白银信号，
            // 且 lockup-watcher 不保证在 hot-money-tracker 之前运行。
            "get_stock_dragon_tiger",
            // 2026-08-01 恢复 get_north_bound_flow（v3 返回成交额，净流入停披但成交额仍披露）
            "get_north_bound_flow",
            "get_stock_institutional_visits",
            "get_stock_margin_data",
            "search_stock",
        ],
    ),
    (
        "lockup-watcher",
        &[
            // V63 修复(2026-07-23): 移除 get_stock_lockup / get_stock_shareholder_trades /
            //   get_stock_block_trades——上游 t-lockup-data 调用 get_stock_lockup_bundle
            //   已返回 {lockup_schedule, shareholder_trades, block_trades} 三个字段的
            //   bundled JSON。LLM 再分别调用这三个工具会获取完全相同的数据，
            //   三份重复 JSON 注入 messages → input tokens 膨胀 3 倍 → output 截断。
            // 保留 get_stock_margin_data（bundle 不含融资融券）和
            //   get_stock_announcements（bundle 不含公告）作为补充数据源。
            "get_stock_margin_data",
            "get_stock_announcements",
            "search_stock",
        ],
    ),
    (
        "research-analyst",
        &[
            // V63 修复(2026-07-23): 移除 get_stock_financials——研报分析师的核心数据是
            // 上游 t-research-data 预拉的研报列表（含分析师评级/EPS预测/目标价），
            // 不需要原始财报 JSON。get_stock_financials 返回多期原始财报（每期 ~20 字段），
            // 与研报中的财务预测数据重复 → input tokens 膨胀 → output 截断 → VERDICT 丢失。
            // 与 fundamentals-analyst 移除 get_stock_financials 同一模式。
            "get_stock_consensus_eps",
            "get_stock_news",
            "get_stock_institutional_visits",
            "search_stock",
        ],
    ),
    (
        "sector-analyst",
        &[
            // P0 修复(2026-07-22): 移除 get_industry_ranking——上游 t-sector-data 已获取。
            "get_hot_stocks",
            "get_stock_quote",
            "get_stock_concept_blocks",
            "get_stock_peers",
            "search_stock",
        ],
    ),
    ("bull-researcher", &["compute_scoring", "compute_valuation", "search_stock"]),
    ("bear-researcher", &["compute_scoring", "compute_valuation", "search_stock"]),
    // v16: R2 质询型辩手也需要 compute_scoring / compute_valuation 来核实对方论据中的
    // 技术评分与估值结论，否则质询问题缺乏数据支撑，容易产出空泛内容。
    ("bull-r2", &["compute_scoring", "compute_valuation", "search_stock"]),
    ("bear-r2", &["compute_scoring", "compute_valuation", "search_stock"]),
    // R3 最终反驳型辩手同样需要 compute_scoring / compute_valuation 来核实对方 R2 质询
    // 背后的技术指标与估值假设，否则"逐条回应"会沦为文本辩论。
    ("bull-r3", &["compute_scoring", "compute_valuation", "search_stock"]),
    ("bear-r3", &["compute_scoring", "compute_valuation", "search_stock"]),
    ("aggressive-debator", &["compute_portfolio_risk", "search_stock"]),
    ("conservative-debator", &["compute_portfolio_risk", "search_stock"]),
    ("neutral-debator", &["compute_portfolio_risk", "search_stock"]),
    (
        "research-manager",
        &["compute_scoring", "compute_valuation", "compute_portfolio_risk", "search_stock"],
    ),
    ("trader", &["get_stock_quote", "compute_scoring", "search_stock"]),
    (
        "value-investor",
        &[
            "get_stock_financials",
            "compute_valuation",
            "get_stock_consensus_eps",
            "get_stock_institutional_visits",
            "get_stock_peers",
            "search_stock",
        ],
    ),
    // ── P3 (real-nodes): 数据质量检查员 + 规则检查员 ──
    // data-quality-inspector 只需阅读上游分析师报告（context_sources 注入），
    // 不需要外部工具调用
    ("data-quality-inspector", &["search_stock"]),
    // quality-fallback: 数据降级时的保守决策，只需少量查询
    ("quality-fallback", &["get_stock_quote", "get_stock_kline", "compute_scoring"]),
    // rule-checker 需要读取技术指标与估值/风控结果
    (
        "rule-checker",
        &["compute_scoring", "compute_valuation", "compute_portfolio_risk", "search_stock"],
    ),
    // ── Catalyst & Narrative Analyst ──
    // 需要读取新闻/公告做催化剂判断 + K线/量价做机构行为分析
    // P0 修复(2026-07-22): 移除未实现的 get_announcement_content（PDF 全文解析为 P2 功能，尚未落地）
    (
        "catalyst-analyst",
        &[
            "get_stock_news",
            "get_stock_announcements",
            "get_stock_concept_blocks",
            "get_stock_peers",
            "get_stock_kline",
            "get_stock_quote",
            "search_stock",
        ],
    ),
    // ── Serenity 瓶颈分析 4 专家工具映射 ──
    // trend-scanner: 扫描宏观数据发现产业趋势，需全天候监控类工具
    (
        "trend-scanner",
        &[
            "get_hot_stocks",
            "get_industry_ranking",
            "get_cls_flash",
            "get_stock_concept_blocks",
            // 2026-08-01 恢复 get_north_bound_flow（v3 返回成交额，净流入停披但成交额仍披露）
            "get_north_bound_flow",
            "get_market_dragon_tiger",
            "search_stock",
        ],
    ),
    // chain-decomposer: 拆解产业链，需行业/概念/同业数据
    (
        "chain-decomposer",
        &[
            "get_stock_concept_blocks",
            "get_stock_peers",
            "get_stock_news",
            "get_industry_ranking",
            "search_stock",
        ],
    ),
    // chokepoint-identifier: 验证瓶颈假设，需财务/研报数据
    (
        "chokepoint-identifier",
        &[
            "get_stock_financials",
            "get_stock_research_reports",
            "get_stock_consensus_eps",
            "get_stock_peers",
            "get_stock_news",
            "search_stock",
        ],
    ),
    // candidate-mapper: 映射候选公司，需财务/估值/调研数据
    (
        "candidate-mapper",
        &[
            "get_stock_financials",
            "get_stock_quote",
            "compute_valuation",
            "get_stock_institutional_visits",
            "get_stock_research_reports",
            "get_stock_news",
            "search_stock",
        ],
    ),
    // ── 简化模板升级：3 个新专家工具映射 ──
    // market-synthesizer: 市场主线综合，需多源数据采集 + 持久化
    (
        "market-synthesizer",
        &[
            "get_hot_stocks",
            "get_cls_flash",
            "get_dragon_tiger_list",
            "get_north_flow",
            "market_mainline_batch_upsert",
        ],
    ),
    // industry-chain-analyzer: 产业链传导，需新闻 + 产业链追踪
    (
        "industry-chain-analyzer",
        &[
            "get_stock_news",
            "get_cls_flash",
            "get_stock_concept_blocks",
            "trace_industry_chain",
            "search_stock",
        ],
    ),
    // screenshot-diagnoser: 持仓截图诊断，需基础分析工具
    (
        "screenshot-diagnoser",
        &["compute_portfolio_risk", "get_stock_quote", "get_stock_peers", "search_stock"],
    ),
    // ── 事件驱动模板：仓位规划与止损复查 ──
    // position-planner: 仓位规划，需基础行情 + 资金分配
    ("position-planner", &["get_stock_quote", "get_account_info", "get_stock_risk_metrics"]),
    // stop-loss-reviewer: 止损复查，需波动率 + 风险指标
    ("stop-loss-reviewer", &["get_stock_quote", "get_stock_risk_metrics", "compute_volatility"]),
];

pub async fn ensure_stock_analysis_experts_seeded(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    // 先执行 Serenity 种子，独立 try 避免被前序步骤阻塞
    tracing::info!("[stock_analysis_setup] === 开始种子 Serenity 模板 ===");
    if let Err(e) = seed_serenity_screening_workflow_template(db).await {
        tracing::error!("[stock_analysis_setup] Serenity 模板种子失败 (非致命): {e}");
    }
    tracing::info!("[stock_analysis_setup] === Serenity 模板种子完成 ===");

    seed_agency_experts(db).await?;
    seed_agent_roles(db).await?;
    seed_stock_agent_roles(db).await?;
    seed_agent_profiles(db).await?;

    // 股票分析核心工作流模板 — 失败不阻塞主流程（独立 try）
    // 原因：如果前置专家种子化失败，? 操作符会直接 return，
    // 导致工作流模板永远不会被种子化，编辑器打开时无内容显示
    tracing::info!("[stock_analysis_setup] === 开始种子股票分析工作流模板 ===");
    if let Err(e) = seed_stock_analysis_workflow_template(db).await {
        tracing::error!("[stock_analysis_setup] 股票分析工作流模板种子失败 (非致命): {e}");
    }
    tracing::info!("[stock_analysis_setup] === 股票分析工作流模板种子完成 ===");

    if let Err(e) = seed_reflection_workflow_template(db).await {
        tracing::error!("[stock_analysis_setup] 反思工作流模板种子失败 (非致命): {e}");
    }
    // seed_debate_subworkflow(db).await?;  // 辩论子工作流未引用，暂不种子化

    // P2-2: 决策事件总线订阅方模板 — 失败不阻塞主流程（独立 try）
    // 两个模板都订阅 "decision.completed" 事件，由 stock_workflow/core.rs 的
    // publish_event 自动触发，实现决策→仓位规划/止损复查的联动编排。
    tracing::info!("[stock_analysis_setup] === 开始种子决策事件订阅模板 ===");
    if let Err(e) = seed_auto_position_plan_template(db).await {
        tracing::error!("[stock_analysis_setup] auto-position-plan 模板种子失败 (非致命): {e}");
    }
    if let Err(e) = seed_auto_stop_loss_review_template(db).await {
        tracing::error!("[stock_analysis_setup] auto-stop-loss-review 模板种子失败 (非致命): {e}");
    }
    tracing::info!("[stock_analysis_setup] === 决策事件订阅模板种子完成 ===");

    // G4: daily-market-events 每日市场主线提炼模板 — 失败不阻塞主流程
    tracing::info!("[stock_analysis_setup] === 开始种子 G4 市场主线模板 ===");
    if let Err(e) = seed_daily_market_events_template(db).await {
        tracing::error!("[stock_analysis_setup] daily-market-events 模板种子失败 (非致命): {e}");
    }
    tracing::info!("[stock_analysis_setup] === G4 市场主线模板种子完成 ===");

    // G6: screenshot-portfolio-diagnosis 截图持仓诊断模板 — 失败不阻塞主流程
    tracing::info!("[stock_analysis_setup] === 开始种子 G6 截图诊断模板 ===");
    if let Err(e) = seed_screenshot_portfolio_diagnosis_template(db).await {
        tracing::error!(
            "[stock_analysis_setup] screenshot-portfolio-diagnosis 模板种子失败 (非致命): {e}"
        );
    }
    tracing::info!("[stock_analysis_setup] === G6 截图诊断模板种子完成 ===");

    // G3.3: news-to-cross-market-analysis 新闻→跨市场传导分析模板 — 失败不阻塞主流程
    tracing::info!("[stock_analysis_setup] === 开始种子 G3.3 跨市场传导分析模板 ===");
    if let Err(e) = seed_news_cross_market::seed_news_cross_market_template(db).await {
        tracing::error!(
            "[stock_analysis_setup] news-to-cross-market-analysis 模板种子失败 (非致命): {e}"
        );
    }
    tracing::info!("[stock_analysis_setup] === G3.3 跨市场传导分析模板种子完成 ===");

    // stock-pipeline: 股票全业务管道模板 — 失败不阻塞主流程
    tracing::info!("[stock_analysis_setup] === 开始种子 stock-pipeline 模板 ===");
    if let Err(e) = crate::commands::stock_pipeline::seed_stock_pipeline_template(db).await {
        tracing::error!("[stock_analysis_setup] stock-pipeline 模板种子失败 (非致命): {e}");
    }
    tracing::info!("[stock_analysis_setup] === stock-pipeline 模板种子完成 ===");
    Ok(())
}

/// 将股票分析 DAG 作为工作流模板持久化到 workflow_templates 表。
/// 模板中的 system_prompt 使用 {{stock_code}} / {{stock_name}} / {{data_ctx}} 占位符，
/// 运行时由 run_stock_workflow 替换为实际行情数据。
///
/// ───────────────────────────────────────────────────────────────────────
/// 【装饰节点模式 / Decorative Container Pattern】
/// ───────────────────────────────────────────────────────────────────────
/// 本模板中以下三个"容器节点"是**纯视觉装饰**，不参与实际流程控制：
///
///   1. `p-analysts`       (ParallelNode)  包裹 9 组 (Tool + Agent)
///   2. `debate-bull-bear` (DebateNode)    包裹 6 个真实辩手 (bull-r1..r3, bear-r1..r3)
///   3. `p-risk-assess`    (ParallelNode)  包裹 3 个风险偏好 Agent
///
/// 关键约定：
///   • 容器在引擎中**立即 Completed**，不等子节点
///   • 实际依赖通过**显式 edge** 表达，不依赖容器的调度语义
///   • `parent_id` 字段仅供前端编辑器嵌套渲染，**运行时调度忽略**
///   • 子节点的 context_sources 直接指向"父节点"（容器）的 id，
///     但因为容器瞬时完成，运行时等同于"等触发边到齐即可启动"
///
/// 为什么需要这种设计？
///   前端画布需要把多组节点画在一个可折叠的分组框内，单纯靠 edge
///   拓扑无法表达"视觉从属关系"。容器节点是"调度语义 + 视觉语义"
///   的解耦产物：调度走 edge，视觉走 parent_id。
///
/// 维护警示：
///   任何把"等下游数据"的节点直接连到容器都是错的——容器返回的是
///   配置元数据而非子节点输出。正确接法是连到最后一个真实子节点
///   （如 value-investor 应连到 `bear-r{debate_max_rounds}`，详见 P0 修复）。
/// ───────────────────────────────────────────────────────────────────────
async fn seed_agency_experts(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    use axagent_entities::agency_experts;
    use sea_orm::{ActiveModelTrait, EntityTrait, NotSet, Set};

    let mut count = 0u32;
    for &(expert_id, content) in EMBEDDED_PROMPTS {
        let (name, desc, body, color) = parse_expert_md(content, expert_id);
        let agency_id = format!("agency-stock-analysis-{expert_id}");
        let now = chrono::Utc::now().timestamp();
        let active = agency_experts::ActiveModel {
            id: Set(agency_id.clone()),
            name: Set(name),
            description: Set(if desc.is_empty() { None } else { Some(desc) }),
            category: Set("finance".into()),
            system_prompt: Set(body),
            color: Set(color),
            source_dir: Set("stock-analysis".into()),
            is_enabled: Set(1),
            imported_at: Set(now),
            recommended_workflows: Set(None),
            recommended_tools: Set(None),
            active_domains: Set(None),
            seniority: NotSet,
            specialties: NotSet,
            parent_role_id: NotSet,
            success_rate: NotSet,
            avg_latency_ms: NotSet,
            avg_token_cost: NotSet,
        };
        // v24: 改为 UPSERT — 已存在则 update，确保 .md 改动和新增的 R3 专家能同步到 DB
        // 历史版本: 已存在则 continue 跳过,导致 .md 改动 / 新增 .md 文件 (bull-r3/bear-r3) 不写库,
        // 前端看到的是旧版 prompt,输出与代码不同步。
        if agency_experts::Entity::find_by_id(&agency_id)
            .one(db)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
            .is_some()
        {
            active.update(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        } else {
            active.insert(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        }
        count += 1;
    }
    tracing::info!("[stock_analysis_setup] 已种子化/更新 {count} 个 agency_experts");
    Ok(())
}

async fn seed_agent_roles(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    // v24: 去掉"已存在则跳过"短路 — 无条件调 upsert_agent_role,确保 STOCK_ROLES 改动
    // (尤其是新增的 role) 能同步到 DB。
    for role in STOCK_ROLES {
        repo::agent_role::upsert_agent_role(
            db,
            role.id,
            role.name,
            Some(role.description),
            role.system_prompt,
            &[],
            &[],
            role.max_concurrent,
            role.timeout_seconds,
            "stock-analysis",
        )
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        count += 1;
    }
    tracing::info!("[stock_analysis_setup] 已种子化/更新 {count} 个 agent_roles");
    Ok(())
}

/// 种子化 AxInvest 专属角色 `stock-investment-lead`（证券投资负责人）
/// 及其 6 个下属子岗位（投研/数据/会议/策略/交易/风控 负责人）。
///
/// 顶层 leader 的 system_prompt 作为最外层身份提示词，通过上游 agent_executor 4 层
/// prompt 拼接（AgentRole → Expert → 节点 inline）注入到所有
/// 股票专家 AgentProfile 的运行时上下文中。详见 STOCK_AGENT_ROLE 注释。
///
/// 6 个子岗位对应 INVESTMENT_OFFICE_TEMPLATE 中的 6 个房间，作为 AddMemberModal
/// 的角色下拉候选项，让投研办公室成员添加时可按房间选角色。
async fn seed_stock_agent_roles(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    // 顶层 leader
    upsert_stock_agent_role(db, &STOCK_AGENT_ROLE, None, 100).await?;
    // 6 个子岗位（投研办公室房间负责人），全部 reports_to = leader
    let mut count = 1u32;
    for sub in STOCK_AGENT_SUB_ROLES {
        upsert_stock_agent_role(db, sub, Some(STOCK_AGENT_ROLE_ID), 200 + count as i32).await?;
        count += 1;
    }
    tracing::info!("[stock_analysis_setup] 已种子化/更新 {} 个角色（1 leader + 6 子岗位）", count);
    Ok(())
}

/// 单个 StockAgentRoleDef 的 upsert 包装，避免重复样板代码。
async fn upsert_stock_agent_role(
    db: &sea_orm::DatabaseConnection,
    r: &StockAgentRoleDef,
    reports_to: Option<&str>,
    sort_order: i32,
) -> Result<(), String> {
    let responsibilities: Vec<String> = r.responsibilities.iter().map(|s| s.to_string()).collect();
    let certifications: Vec<String> =
        r.required_certifications.iter().map(|s| s.to_string()).collect();
    let domains: Vec<String> = r.active_domains.iter().map(|s| s.to_string()).collect();
    // managed_expert_ids 留空——股票专家众多且会动态增减，由前端按 source_dir="stock-analysis" 聚合
    repo::agent_role::upsert_agent_role_ext(
        db,
        r.id,
        r.name,
        Some(r.description),
        r.system_prompt,
        &[],
        &domains,
        3,
        600,
        "stock-analysis",
        Some(&serde_json::to_string(&responsibilities).unwrap_or_default()),
        Some(r.decision_authority),
        reports_to,
        None,
        Some(&serde_json::to_string(&certifications).unwrap_or_default()),
        Some(r.icon),
        Some(r.color),
        true,
        sort_order,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("种子角色失败: {e}"))
    })?;
    tracing::info!("[stock_analysis_setup] 已种子化/更新角色岗位 {} ({})", r.id, r.name);
    Ok(())
}

async fn seed_agent_profiles(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    use axagent_entities::agent_profiles;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    // Profile → 工具映射（从模块级 PROFILE_TOOLS 构建）
    let profile_tools: std::collections::HashMap<&str, &[&str]> =
        PROFILE_TOOLS.iter().cloned().collect();

    let mut count = 0u32;
    for &(expert_id, role_id) in EXPERT_ROLE_MAP {
        let profile_id = format!("stock-{expert_id}");

        let tools_json = profile_tools
            .get(expert_id)
            .map(|tools| serde_json::to_string(tools).unwrap_or_default());
        let now = chrono::Utc::now().timestamp_millis();
        let active = agent_profiles::ActiveModel {
            id: Set(profile_id.clone()),
            name: Set(format!("📈 {}", expert_id_to_display(expert_id))),
            description: Set(Some(format!("股票分析专家 — {}", role_id_to_display(role_id)))),
            category: Set("stock-analysis".into()),
            icon: Set("📈".into()),
            source: Set("stock-analysis".into()),
            tags: Set(None),
            suggested_provider_id: Set(None),
            suggested_model_id: Set(None),
            suggested_temperature: Set(None),
            suggested_max_tokens: Set(None),
            search_enabled: Set(None),
            recommend_permission_mode: Set(None),
            recommended_tools: Set(tools_json),
            disallowed_tools: Set(None),
            recommended_workflows: Set(None),
            sort_order: Set(0),
            is_enabled: Set(1),
            expert_id: Set(Some(format!("agency-stock-analysis-{expert_id}"))),
            // v218: 岗位即角色——agent_role 指向证券投资负责人（已并入 agent_roles），
            // 其 system_prompt 作为最外层身份注入；stock-analyst 等执行器标签原未入表，不再使用。
            agent_role: Set(Some(STOCK_AGENT_ROLE_ID.into())),
            created_at: Set(now),
            updated_at: Set(now),
        };
        // v24: 改为 UPSERT — 已存在则 update,确保 PROFILE_TOOLS 改动和新增 expert (bull-r3/bear-r3) 同步到 DB
        if agent_profiles::Entity::find_by_id(&profile_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("查询 profile 失败: {e}"))
            })?
            .is_some()
        {
            active.update(db).await.map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("更新 profile 失败: {e}"))
            })?;
        } else {
            active.insert(db).await.map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("插入 profile 失败: {e}"))
            })?;
        }
        count += 1;
    }
    tracing::info!("[stock_analysis_setup] 已种子化/更新 {count} 个 agent_profiles");
    Ok(())
}

pub(crate) fn parse_expert_md(
    content: &str,
    fallback: &str,
) -> (String, String, String, Option<String>) {
    let mut name = String::new();
    let mut desc = String::new();
    let mut color: Option<String> = None;
    let body = if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let fm = &rest[..end];
            for line in fm.lines() {
                // title: 作为 name: 的别名（多份 .md 沿用 old frontmatter 习惯）
                if let Some(v) = line.trim().strip_prefix("name:") {
                    name = v.trim().into();
                } else if let Some(v) = line.trim().strip_prefix("title:") {
                    if name.is_empty() {
                        name = v.trim().into();
                    }
                } else if let Some(v) = line.trim().strip_prefix("description:") {
                    desc = v.trim().into();
                } else if let Some(v) = line.trim().strip_prefix("color:") {
                    let c = v.trim();
                    if !c.is_empty() {
                        color = Some(c.into());
                    }
                }
            }
            rest[end + 4..].trim().to_string()
        } else {
            content.to_string()
        }
    } else {
        content.to_string()
    };
    if name.is_empty() {
        name = expert_id_to_display(fallback);
    }
    (name, desc, body, color)
}

pub(crate) fn expert_id_to_display(id: &str) -> String {
    match id {
        "market-analyst" => "市场技术分析师".to_string(),
        "sentiment-analyst" => "情绪面分析师".to_string(),
        "news-analyst" => "消息面分析师".to_string(),
        "fundamentals-analyst" => "基本面分析师".to_string(),
        "policy-analyst" => "政策面分析师".to_string(),
        "hot-money-tracker" => "资金面追踪".to_string(),
        "lockup-watcher" => "筹码限售观察".to_string(),
        "research-analyst" => "研报分析师".to_string(),
        "sector-analyst" => "板块题材分析师".to_string(),
        "bull-researcher" => "多方研究员".to_string(),
        "bear-researcher" => "空方研究员".to_string(),
        "aggressive-debator" => "激进风险评估".to_string(),
        "conservative-debator" => "保守风险评估".to_string(),
        "neutral-debator" => "中性风险评估".to_string(),
        "research-manager" => "研究经理".to_string(),
        "trader" => "交易员".to_string(),
        "value-investor" => "价值投资者（巴菲特框架）".to_string(),
        "catalyst-analyst" => "催化剂与叙事分析师".to_string(),
        // ── Serenity 瓶颈分析师 ──
        "trend-scanner" => "产业趋势扫描器".to_string(),
        "chain-decomposer" => "产业链拆解师".to_string(),
        "chokepoint-identifier" => "瓶颈鉴定师".to_string(),
        "candidate-mapper" => "候选公司映射器".to_string(),
        o => o.to_string(),
    }
}

pub(crate) fn role_id_to_display(id: &str) -> String {
    match id {
        "stock-analyst" => "股票分析师".to_string(),
        "debater" => "辩论研究员".to_string(),
        "risk-evaluator" => "风险评估师".to_string(),
        "trader" => "交易员".to_string(),
        "decision-maker" => "决策者".to_string(),
        "reflection" => "投资复盘官".to_string(),
        o => o.to_string(),
    }
}

/// 构建分析师 input_mapping：为每个分析师注入 bull_score/bear_score/consensus_score
/// 例如 a-market-analyst → 【market_bull_score】:75 【market_bear_score】:25
///
/// 路径规则（V29 修复）：AgentNode 输出包裹在 {role, content: <json_string>, ...} 中，
/// resolve_var_path 遇到 Value::String 会自动 from_str 解析后再继续下钻，
/// 因此必须用 `.content.field` 路径访问 AgentNode 业务字段。
pub(crate) fn build_analyst_input_mapping(
    a_ids: &[&str],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    for aid in a_ids {
        // a-market-analyst → market, a-sentiment → sentiment, etc.
        let prefix = aid.strip_prefix("a-").unwrap_or(aid);
        map.insert(format!("{prefix}_bull_score"), format!("{aid}.content.bull_score"));
        map.insert(format!("{prefix}_bear_score"), format!("{aid}.content.bear_score"));
        // consensus_score = bull - bear（聚合分数）
        map.insert(format!("{prefix}_consensus"), format!("{aid}.content.consensus_score"));
    }
    // 为所有辩论/风险节点注入历史反思教训
    map.insert("stock_lessons".into(), "stock_lessons".into());
    map
}

/// 合并新模板变量与旧模板变量的值。
/// 对于同名的变量，保留旧变量的 value（用户的修改），字段定义以新模板为准。
pub(crate) fn merge_variable_values(
    new_variables_json: &str,
    old_variables_json: &str,
) -> Result<String, String> {
    let new_vars: Vec<serde_json::Value> =
        serde_json::from_str(new_variables_json).map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("解析新变量失败: {e}"))
        })?;
    let old_vars: Vec<serde_json::Value> =
        serde_json::from_str(old_variables_json).map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("解析旧变量失败: {e}"))
        })?;

    // 变量迁移映射表：旧名称 → 新名称（模板升级时变量被重命名的情况）
    //
    // 老 UI 用的 camelCase 命名在 stock-analysis 模板 v15→v19 升级时统一改为 snake_case
    // 并补全前缀（agent_/tool_/rule_/pos_/value_/monitor_/kline_/news_/vendor_）。
    // 旧用户在设置面板调整过的值会留在 DB 的 workflow_template.variables 列里，
    // 升级时如果新模板没有同 key 的变量就会被丢弃。这里建立别名映射，
    // 升级时把旧 key 的 value 复制到新 key 上，避免用户调参失效。
    const RENAME_MAP: &[(&str, &str)] = &[
        // 分析流程
        ("analysis_maxDebateRounds", "debate_rounds"),
        ("analysis_maxConcurrent", "max_concurrent"),
        // 数据源
        ("analysis_klinePeriod", "kline_period"),
        ("analysis_klineLimit", "kline_limit"),
        ("analysis_newsLimit", "news_limit"),
        // Agent / Tool
        ("analysis_temperature", "agent_temperature"),
        ("analysis_maxTokens", "agent_max_tokens"),
        ("analysis_timeoutSecs", "agent_timeout_secs"),
        ("tool_timeoutSecs", "tool_timeout_secs"),
        ("tool_retryMax", "tool_retry_max"),
        // 规则
        ("rule_rsiOverbought", "rule_rsi_overbought"),
        ("rule_rsiOversold", "rule_rsi_oversold"),
        ("rule_biasLimit", "rule_bias_limit_pct"),
        ("rule_volumeSignalBlock", "rule_volume_signal_block"),
        ("rule_bearLowScore", "rule_bear_low_score"),
        ("rule_autoStopLossPct", "rule_auto_stop_loss_pct"),
        // 仓位
        ("pos_maxSingleStockPct", "pos_max_single_pct"),
        ("pos_maxTotalPositions", "pos_max_total"),
        ("pos_maxSectorExposurePct", "pos_max_sector_pct"),
        // 估值
        ("value_dcfGrowthRate", "value_dcf_growth_rate"),
        ("value_dcfPerpetualRate", "value_dcf_perpetual_rate"),
        ("value_dcfDiscountRate", "value_dcf_discount_rate"),
        ("value_moatThreshold", "value_moat_threshold"),
        ("value_fScoreBuyThreshold", "value_fscore_buy"),
        ("value_safetyMarginMin", "value_safety_margin"),
        // 监控
        ("monitor_pollIntervalSecs", "monitor_poll_interval_secs"),
        ("monitor_changePctThreshold", "monitor_change_pct"),
        ("monitor_turnoverThreshold", "monitor_turnover"),
        ("monitor_alertCooldownSecs", "monitor_alert_cooldown_secs"),
        // 跨股票聚合器（P2 配置入口）
        ("aggregator_windowSecs", "aggregator_window_secs"),
        ("aggregator_minSignalCount", "aggregator_min_signal_count"),
        ("aggregator_cooldownSecs", "aggregator_cooldown_secs"),
        ("aggregator_minStrength", "aggregator_min_strength"),
    ];

    // 构建旧变量名 → value 的映射（处理重命名别名）
    let old_values: std::collections::HashMap<String, serde_json::Value> = old_vars
        .into_iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?;
            let value = v.get("value")?.clone();
            // 主名称
            let mut entries = vec![(name.to_string(), value.clone())];
            // 如果该变量有重命名别名，也加入映射
            for (old, new) in RENAME_MAP {
                if *new == name {
                    entries.push((old.to_string(), value.clone()));
                }
            }
            Some(entries)
        })
        .flatten()
        .collect();

    // 合并：新变量定义 + 旧变量值（如有）
    let merged: Vec<serde_json::Value> = new_vars
        .into_iter()
        .map(|mut v| {
            if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
                if let Some(old_val) = old_values.get(name) {
                    v["value"] = old_val.clone();
                }
            }
            v
        })
        .collect();

    serde_json::to_string(&merged).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL)
            .with_detail(format!("序列化合变量失败: {e}"))
            .to_string()
    })
}

// seed_debate_subworkflow: 辩论已通过 DebateNode 容器直接嵌入主模板，旧独立模板已移除

/// 种子化反思复盘工作流模板（stock-reflection）。
///
/// 与 stock-analysis 同款：用 Rust 类型（`WorkflowNode` / `WorkflowNodeBase` /
/// `WorkflowNodeConfig::*`）构造节点，再 `serde_json::to_string` 序列化入库。
/// 这样编译器会强制要求所有必填字段（id/title/position/retry/enabled…），
/// 避免 `serde_json::json!()` 裸写漏字段导致反序列化静默失败、编辑器看不到节点。
///
/// 运行时 portfolio-manager 通过 `{{actual_outcome}}` 变量切换到反思模式。
async fn seed_reflection_workflow_template(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    use axagent_entities::workflow_template;
    use axagent_harness::workflow_types::{
        AgentNode, AgentNodeConfig, CodeNode, CodeNodeConfig, EdgeType, OutputMode, Position,
        RetryConfig, StorageNode, StorageNodeConfig, ToolDef, TriggerConfig, TriggerNode,
        TriggerType, Variable, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let now = chrono::Utc::now().timestamp_millis();

    // ── 反思 Agent 可用工具定义（仅 K 线 + 公告全文，不暴露交易类工具）──
    let refl_tools: Vec<ToolDef> = {
        let mut kline_props = std::collections::HashMap::new();
        kline_props.insert(
            "stock_code".into(),
            axagent_harness::workflow_types::JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("6位股票代码".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        kline_props.insert(
            "period".into(),
            axagent_harness::workflow_types::JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("K线周期: daily(日线)/weekly(周线)/monthly(月线)".into()),
                default: Some(serde_json::json!("daily")),
                enum_values: None,
                format: None,
            },
        );
        kline_props.insert(
            "limit".into(),
            axagent_harness::workflow_types::JsonSchemaProperty {
                schema_type: "integer".into(),
                description: Some("K线数量".into()),
                default: Some(serde_json::json!(120)),
                enum_values: None,
                format: None,
            },
        );
        let td_kline = ToolDef {
            name: "get_stock_kline".into(),
            description: Some("获取K线数据：OHLCV，可指定周期和数量，用于事后对比走势".into()),
            parameters: Some(axagent_harness::workflow_types::JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some(kline_props),
                required: Some(vec!["stock_code".into()]),
                items: None,
            }),
        };
        // P0 修复(2026-07-22): 移除未实现的 get_announcement_content 工具引用
        // 该工具在 mcp_tools.rs 中未注册 dispatch，调用会触发 "Unknown MCP tool" 错误
        vec![td_kline]
    };

    // ── CodeNode: 定量对比脚本（sub-analysis → reflection-comparator → reflection-agent）──
    let comparator_code = include_str!("../reflection-comparator.rhai").to_string();
    let comparator_node = WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "reflection-comparator".into(),
            title: "预测vs实际定量对比".into(),
            description: Some("对比分析师预测与实际走势，输出结构化偏差报告".into()),
            position: Position { x: 20.0, y: 260.0 },
            retry: RetryConfig::default(),
            timeout: Some(10),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: true, // 对比失败不阻塞反思
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: comparator_code,
            output_var: "reflection-comparator".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: [
                ("trader_action", "sub-analysis.trader.content.action"),
                ("trader_target_price", "sub-analysis.trader.content.targetPrice"),
                ("trader_confidence", "sub-analysis.trader.content.confidence"),
                ("portfolio_action", "sub-analysis.portfolio-mgr.action"),
                ("portfolio_posterior", "sub-analysis.portfolio-mgr.posterior"),
                ("debate_consensus", "sub-analysis.debate-convergence.content.consensus_score"),
                ("total_score", "sub-analysis.t-scoring.result.totalScore"),
                ("raw_return_pct", "raw_return_pct"),
                ("alpha_return_pct", "alpha_return_pct"),
                ("holding_days", "holding_days"),
                ("original_time_horizon", "original_time_horizon"),
                ("original_holding_days", "original_holding_days"),
                // __untrusted 标记（从子工作流各 Agent 节点提取）
                ("u_trader", "sub-analysis.trader.__untrusted"),
                ("u_research_mgr", "sub-analysis.research-mgr.__untrusted"),
                ("u_catalyst", "sub-analysis.a-catalyst.__untrusted"),
                ("u_debate_cnv", "sub-analysis.debate-convergence.__untrusted"),
                ("u_data_quality", "sub-analysis.data-quality.__untrusted"),
                ("u_risk_cnv", "sub-analysis.risk-convergence.__untrusted"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        },
    });

    // ── CodeNode: 反思输出硬裁决验证层（reflection-agent → reflection-validator → store-ref）──
    // [P1-#1 修复] 原 reflection_validator.rhai 是死代码，DAG 未引用。
    // 现接入为 DAG 节点，在 reflection-agent 之后、store-ref 之前执行。
    // 验证 7 字段类型/枚举值/长度，自动修正 verdict 枚举、截断 lesson_summary、
    // 补全 missed_signals 数组等（R-302/R-303/R-304/R-305 硬裁决规则）。
    let validator_code = include_str!("../reflection_validator.rhai").to_string();
    let validator_node = WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "reflection-validator".into(),
            title: "反思输出硬裁决验证".into(),
            description: Some("验证 reflection-agent 输出的字段类型/枚举值/长度，自动修正".into()),
            position: Position { x: 20.0, y: 460.0 },
            retry: RetryConfig::default(),
            timeout: Some(5),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: true, // 验证失败不阻塞落盘
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: validator_code,
            output_var: "reflection-validated".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: [("reflection_input", "reflection")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        },
    });

    // ── 节点定义（与 stock-analysis 同款：Rust 类型构造，编译期校验必填字段）──
    let nodes: Vec<WorkflowNode> = vec![
        // 1. 触发器：手动模式，传入 stock_code / as_of_date / actual_outcome / reflection_depth
        WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                id: "trigger".into(),
                title: "反思复盘触发器".into(),
                description: Some("触发反思复盘工作流，传入 stock_code / as_of_date".into()),
                position: Position { x: 20.0, y: 20.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({
                    "description": "as-of 重放: 选择历史日期对分析结果进行反思复盘",
                    "required_params": ["as_of_date", "stock_code"],
                    "param_schema": {
                        "as_of_date": { "type": "date", "description": "原始分析日期，决定数据时间锚点" },
                        "stock_code": { "type": "string", "description": "股票代码" }
                    }
                }),
            },
        }),
        // 2. 定量对比 CodeNode + 3. 反思复盘 Agent + 4. 硬裁决验证
        //    注: comparator_node / validator_node 在 nodes vec 外部构造(见前文),这里追加到 vec 末尾
        //    [v2] 删除 sub-analysis SubWorkflowNode — 不再重跑完整 stock-analysis DAG，
        //    改由 run_reflection_workflow 从 stock_analyses.blackboard_snapshot 加载记忆，
        //    构造名为 "sub-analysis" 的变量注入工作流（context_sources / input_mapping 路径不变）。
        comparator_node,
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: "reflection-agent".into(),
                title: "反思复盘".into(),
                description: Some("基于实际走势+偏差报告+数据工具做反思复盘".into()),
                position: Position { x: 20.0, y: 380.0 },
                retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
                timeout: Some(600),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: "你的任务：对历史股票分析进行反思复盘。\n\
                    目标股票代码: {{stock_code}}，股票名称: {{stock_name}}\n\
                    实际走势结果: {{actual_outcome}}（非空 → 反思模式）\n\
                    ——结构化 outcome 变量（v008 C3 借鉴:硬数字,避免 LLM 脑补）——\n\
                    原始收益率: {{raw_return_pct}}%\n\
                    相对基准超额: {{alpha_return_pct}}%\n\
                    实际持有天数: {{holding_days}} 天\n\
                    基准名称: {{benchmark_name}}\n\
                    反思深度: {{reflection_depth}}（light = 简要；deep = 详细推理链）\n\n\
                    ——定量偏差报告（reflection-comparator 输出）——\n\
                    详见下方【输入上下文】的 deviation_report 字段。\n\
                    包含方向匹配度(direction_match)/收益分类(return_category)/时间维度检查。\n\
                    分析前务必先阅读，direction_match=false 说明方向误判需深入分析错因。\n\n\
                    历史反思教训（避免重蹈覆辙）:\n\
                    {{stock_lessons}}\n\n\
                    可用工具：\n\
                    - get_stock_kline: 获取实际操作期间的K线数据，对比预测走势与实际价格运动\n\
                    - get_announcement_content: 获取分析日期之后发布的新公告PDF全文，\n\
                      用于检查是否有影响走势的关键公告被遗漏\n\n\
                    使用工具的原则：\n\
                    1. 先分析 deviation_report 中的定量发现，确认方向是否一致\n\
                    2. 如有必要，调用 get_stock_kline 查看实际K线走势验证\n\
                    3. 如果公告数据在原始分析后发生变化，调用 get_announcement_content 查阅\n\
                    4. 工具调用结论应与定量对比报告交叉验证\n\n\
                    重要原则：\n\
                    1. 必须严格基于 actual_outcome 提供的实际走势与上游分析结论做对比，识别错因。\n\
                    2. 结合 deviation_report 的定量发现验证而非替代 LLM 判断。\n\
                    3. 严禁输出空结果或只列 data_gaps。\n\
                    4. 强制简短：lesson_summary 字段必须 ≤200 字符、≤2 句。\n\
                    5. 反思深度=deep 时给出可执行的检查清单（具体指标阈值、信号确认步骤）。\n\
                    6. 用 verdict 字段标记本次反思判定（correct/partial/wrong 三选一）。\n\
                    7. 如果复盘发现本可优化决策，在 alpha_cited 字段说明关键 alpha 信号。\n\
                    8. 不要输出交易决策（买入/卖出/持有），不要输出 confidence/positionPct。\n\n\
                    你必须输出严格 JSON 格式（不要 Markdown 代码块，不要多余文本），字段如下：\n\
                    {\n\
                      \"verdict\": \"correct | partial | wrong\",\n\
                      \"alpha_cited\": \"引用本次未被重视但事后证明重要的 alpha 信号\",\n\
                      \"lesson_summary\": \"≤200 字符、≤2 句简短总结\",\n\
                      \"what_went_wrong\": \"哪里判断错了，简要说明\",\n\
                      \"missed_signals\": [\"被忽略的信号1\", \"被忽略的信号2\"],\n\
                      \"fix_for_future\": \"下次如何避免同样的错误\",\n\
                      \"implementation_tier\": \"L1 | L2 | L3\",\n\
                      \"code_diff_proposal\": \"具体修改方案描述（L1简述 / L2-L3含文件路径和代码段）\",\n\
                      \"params_suggestion\": [\n\
                        {\n\
                          \"param\": \"参数名\",\n\
                          \"current_value\": \"当前值\",\n\
                          \"suggested_value\": \"建议值\",\n\
                          \"reason\": \"调整原因\"\n\
                        }\n\
                      ]\n\
                    }"
                .into(),
                context_sources: vec!["sub-analysis".into(), "reflection-comparator".into()],
                input_mapping: [
                    // [BUGFIX] source 应为变量名而非节点 ID "trigger"。
                    // 这些变量已在 run_reflection_workflow 的 variables vec 中顶层注入,
                    // 用变量名才能正确从 context.variables 取到 string 值,
                    // 否则 map_inputs 会把整个 trigger 节点输出对象当变量值传递。
                    ("stock_code".to_string(), "stock_code".to_string()),
                    ("stock_name".to_string(), "stock_name".to_string()),
                    ("actual_outcome".to_string(), "actual_outcome".to_string()),
                    ("reflection_depth".to_string(), "reflection_depth".to_string()),
                    ("raw_return_pct".to_string(), "raw_return_pct".to_string()),
                    ("alpha_return_pct".to_string(), "alpha_return_pct".to_string()),
                    ("holding_days".to_string(), "holding_days".to_string()),
                    ("benchmark_name".to_string(), "benchmark_name".to_string()),
                    ("stock_lessons".to_string(), "stock_lessons".to_string()),
                    ("hindsight_date".to_string(), "hindsight_date".to_string()),
                    ("deviation_report".to_string(), "reflection-comparator".to_string()),
                ]
                .into_iter()
                .collect(),
                output_var: "reflection".into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(32768),
                tools: refl_tools,
                exposed_tools: vec![],
                output_mode: OutputMode::Json,
                agent_profile_id: Some("stock-reflection".into()),
                max_tool_rounds: Some(3), // 限制工具调用轮数，防止过度拉数据
                execution_mode: None,
                // 从 stock_reflections 记忆空间检索语义相似的历史反思
                rag_source_ids: vec!["memory:stock_reflections".into()],
                model_role: Some("decision-maker".into()),
                consistency_check: Some(axagent_harness::ConsistencyCheckConfig {
                    enabled: true,
                    mode: axagent_harness::ConsistencyMode::SameModelRepeated,
                    secondary_model: None,
                    deviation_threshold: 0.3,
                }),
                hallucination_guard: Some(axagent_harness::HallucinationGuardConfig {
                    enabled: false,
                    match_threshold: 0.4,
                }),
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
            },
        }),
        // 4. 硬裁决验证：reflection-agent → reflection-validator → store-ref
        //    [P1-#1] 接入原死代码 reflection_validator.rhai，自动修正字段类型/枚举值/长度
        validator_node,
        // 5. 反思记录持久化：写入 stock_reflections 表供后续查询/复盘
        WorkflowNode::Storage(StorageNode {
            base: WorkflowNodeBase {
                id: "store-ref".into(),
                title: "反思记录持久化".into(),
                description: Some("写入反思记录到 stock_reflections 表".into()),
                position: Position { x: 20.0, y: 500.0 },
                retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
                timeout: Some(30),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: StorageNodeConfig {
                backend: "sqlite".into(),
                // [BUGFIX] 改为 upsert：B3 路径下 run_reflection_workflow 已 UPDATE
                // pending row（通过 pending_id 匹配），store-ref 不应再 INSERT 重复 row。
                // upsert 语义：若 pending row 存在则 UPDATE，否则 INSERT。
                operation: "upsert".into(),
                // [P1-#1] 使用验证后的输出（reflection-validator 节点 output_var）
                input_var: "reflection-validated".into(),
                collection: "stock_reflections".into(),
                key_var: None,
                output_var: "storage-result".into(),
            },
        }),
    ];

    let edges: Vec<WorkflowEdge> = vec![
        // [v2] trigger → reflection-comparator 直连（删除 sub-analysis 中间节点）
        WorkflowEdge {
            id: "e-trigger-comparator".into(),
            source: "trigger".into(),
            source_handle: None,
            target: "reflection-comparator".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-comparator-reflection".into(),
            source: "reflection-comparator".into(),
            source_handle: None,
            target: "reflection-agent".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        // [P1-#1] reflection-agent → reflection-validator → store-ref
        WorkflowEdge {
            id: "e-reflection-validator".into(),
            source: "reflection-agent".into(),
            source_handle: None,
            target: "reflection-validator".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-validator-store".into(),
            source: "reflection-validator".into(),
            source_handle: None,
            target: "store-ref".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
    ];

    let variables: Vec<Variable> = vec![
        Variable {
            name: "actual_outcome".into(),
            var_type: "string".into(),
            value: serde_json::Value::String("".into()),
            description: Some("实际走势结果，如 '30天跌8% → 失败'，非空时触发反思模式".into()),
            is_secret: false,
        },
        Variable {
            name: "reflection_depth".into(),
            var_type: "string".into(),
            value: serde_json::Value::String("light".into()),
            description: Some("反思深度：light(简要) / deep(详细推理链)".into()),
            is_secret: false,
        },
    ];

    // serenity-reflection 模板版本。
    const REFLECTION_TEMPLATE_VERSION: i32 = 1;

    // 版本检查：已有同版本或更新的记录则跳过
    if let Some(ref existing) =
        axagent_entities::workflow_template::Entity::find_by_id("stock-reflection")
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
    {
        if existing.version >= REFLECTION_TEMPLATE_VERSION {
            tracing::info!(
                "[stock_analysis_setup] 反思模板已是最新 v{}，跳过种子化",
                existing.version
            );
            return Ok(());
        }
        // 旧版本 → 保存快照
        let ver_id = format!("stock-reflection_v{}", existing.version);
        if axagent_entities::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
            .is_none()
        {
            use crate::commands::error::ErrorResponse;
            use sea_orm::ActiveModelTrait;
            let snapshot = axagent_entities::workflow_template_version::ActiveModel {
                id: Set(ver_id.clone()),
                template_id: Set("stock-reflection".to_string()),
                name: Set(existing.name.clone()),
                description: Set(existing.description.clone()),
                icon: Set(existing.icon.clone()),
                tags: Set(existing.tags.clone()),
                version: Set(existing.version),
                is_preset: Set(existing.is_preset),
                is_editable: Set(existing.is_editable),
                is_public: Set(existing.is_public),
                trigger_config: Set(existing.trigger_config.clone()),
                nodes: Set(existing.nodes.clone()),
                edges: Set(existing.edges.clone()),
                input_schema: Set(existing.input_schema.clone()),
                output_schema: Set(existing.output_schema.clone()),
                variables: Set(existing.variables.clone()),
                error_config: Set(existing.error_config.clone()),
                created_at: Set(chrono::Utc::now().timestamp_millis()),
            };
            snapshot.insert(db).await.map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("写入版本快照失败: {e}"))
            })?;
            tracing::info!("[stock_analysis_setup] 反思模板旧版本快照已保存: {ver_id}");
        }
    }

    // 走 stock-analysis 同款序列化路径：编译期校验 + 字段齐全
    let nodes_json = serde_json::to_string(&nodes).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化反思节点失败: {e}"))
    })?;
    let edges_json = serde_json::to_string(&edges).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化反思边失败: {e}"))
    })?;
    let variables_json = serde_json::to_string(&variables).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化反思变量失败: {e}"))
    })?;
    let tags_json = serde_json::to_string(&["stock", "reflection", "A股"]).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化反思标签失败: {e}"))
    })?;

    // 先删再插，避免 SeaORM .save() 对已存在记录的 update 失败
    let _ = workflow_template::Entity::delete_by_id("stock-reflection").exec(db).await;
    workflow_template::ActiveModel {
        id: Set("stock-reflection".to_string()),
        cluster_id: Set(None),
        route_path: Set(None),
        name: Set("A股反思复盘".to_string()),
        description: Set(Some(
            "嵌套 stock-analysis 子工作流的 as-of 重放，注入实际走势结果后反思".to_string(),
        )),
        icon: Set("search".into()),
        tags: Set(Some(tags_json)),
        version: Set(REFLECTION_TEMPLATE_VERSION),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(
            serde_json::to_string(&TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({
                    "description": "as-of 重放: 选择历史日期对分析结果进行反思复盘",
                    "required_params": ["as_of_date", "stock_code"],
                    "param_schema": {
                        "as_of_date": { "type": "date", "description": "原始分析日期，决定数据时间锚点" },
                        "stock_code": { "type": "string", "description": "股票代码" }
                    }
                }),
            })
            .map_err(|e| ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化触发器配置失败: {e}")))?,
        )),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some(variables_json)),
        error_config: Set(None),
        composite_source: Set(None),
        tool_defs: Set(None),
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("写入反思模板失败: {e}")))?;

    tracing::info!(
        "[stock_analysis_setup] 反思复盘工作流模板已创建 (stock-reflection, SubWorkflowNode 嵌套)"
    );
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// P2-2: 决策事件总线订阅方模板
//
// 两个模板都订阅 stock_workflow/core.rs 在决策落库后发布的 "decision.completed"
// 事件。事件 payload 字段（camelCase）:
//   { analysisId, stockCode, stockName, action, decisionJson, asOfDate,
//     parentAnalysisId, timestamp }
//
// publish_event → engine.run_workflow(wf_id, RunOptions { input: payload })
// → start_workflow 把 payload 存入 state.input_params
// → 节点执行时 merged_vars 合并顺序：deps_results → context_sources
//    → state.variables → state.input_params（兜底）
// → AgentNode 通过 input_mapping 引用 payload 字段（value = payload 字段名）
// ───────────────────────────────────────────────────────────────────────────

/// 事件订阅型决策联动模板的公共字段配置。
struct EventTriggeredTemplateSpec {
    /// 模板 ID（也是 workflow_id，注册到 TriggerManager）
    template_id: &'static str,
    /// 模板显示名
    name: &'static str,
    /// 模板描述
    description: &'static str,
    /// 图标
    icon: &'static str,
    /// Agent 节点 ID（用于前端节点标题国际化）
    agent_node_id: &'static str,
    /// Agent 节点标题
    agent_node_title: &'static str,
    /// Agent 系统提示词（支持 {{stock_code}} 等占位符）
    agent_system_prompt: &'static str,
    /// Agent 输出变量名
    output_var: &'static str,
    /// 模板版本
    version: i32,
    /// 标签
    tags: &'static [&'static str],
    /// Agent Profile ID（用于绑定 Role + Expert）
    agent_profile_id: Option<&'static str>,
    /// 模型角色（用于注入 A 股约束等）
    model_role: Option<&'static str>,
}

/// 内部辅助函数：构建并持久化一个事件订阅型决策联动工作流模板。
///
/// 模板结构：TriggerNode(Event) → AgentNode → EndNode
/// - TriggerNode 配置 EventTriggerConfig { event_type: "decision.completed" }
/// - AgentNode 通过 input_mapping 引用 payload 字段，输出 JSON 结果
/// - EndNode 终止工作流
///
/// 顶层 `trigger_config` 字段也写入 EventTriggerConfig，供 trigger_recovery.rs
/// 在进程重启时恢复事件订阅到 TriggerManager。
async fn seed_event_triggered_decision_template(
    db: &sea_orm::DatabaseConnection,
    spec: EventTriggeredTemplateSpec,
) -> Result<(), String> {
    use axagent_entities::workflow_template;
    use axagent_harness::workflow_types::{
        AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, EventTriggerConfig,
        OutputMode, Position, RetryConfig, TriggerConfig, TriggerNode, TriggerType, WorkflowEdge,
        WorkflowNode, WorkflowNodeBase,
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let now = chrono::Utc::now().timestamp_millis();

    // 事件触发器配置（同时用于 TriggerNode 和顶层 trigger_config 字段）
    let event_cfg =
        EventTriggerConfig { event_type: "decision.completed".to_string(), filter: None };
    let event_cfg_value = serde_json::to_value(&event_cfg).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化事件配置失败: {e}"))
    })?;
    let trigger_config =
        TriggerConfig { trigger_type: TriggerType::Event, config: event_cfg_value.clone() };

    // ── 节点定义 ──
    let nodes: Vec<WorkflowNode> = vec![
        // 1. 触发器：订阅 decision.completed 事件
        WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                id: "trigger".into(),
                title: "决策事件触发器".into(),
                description: Some("订阅 decision.completed 事件，决策落库后自动触发".into()),
                position: Position { x: 20.0, y: 20.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: trigger_config.clone(),
        }),
        // 2. Agent 节点：基于决策 payload 推理输出
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: spec.agent_node_id.into(),
                title: spec.agent_node_title.into(),
                description: Some(spec.description.into()),
                position: Position { x: 20.0, y: 180.0 },
                retry: RetryConfig { enabled: true, max_retries: 1, ..Default::default() },
                timeout: Some(300),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: spec.agent_system_prompt.into(),
                // 引用触发器节点输出（含 status/trigger_type/config/timestamp），
                // 实际决策数据通过 input_mapping + input_params 兜底注入。
                context_sources: vec!["trigger".into()],
                input_mapping: [
                    ("stock_code".to_string(), "stockCode".to_string()),
                    ("stock_name".to_string(), "stockName".to_string()),
                    ("action".to_string(), "action".to_string()),
                    ("decision_json".to_string(), "decisionJson".to_string()),
                    ("as_of_date".to_string(), "asOfDate".to_string()),
                    ("analysis_id".to_string(), "analysisId".to_string()),
                ]
                .into_iter()
                .collect(),
                output_var: spec.output_var.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(4096),
                tools: vec![],
                exposed_tools: vec![],
                output_mode: OutputMode::Json,
                agent_profile_id: spec.agent_profile_id.map(|id| id.into()),
                max_tool_rounds: Some(1),
                execution_mode: None,
                rag_source_ids: vec![],
                model_role: spec.model_role.map(|r| r.into()),
                consistency_check: None,
                hallucination_guard: None,
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
            },
        }),
        // 3. 终止节点
        WorkflowNode::End(EndNode {
            base: WorkflowNodeBase {
                id: "end".into(),
                title: "结束".into(),
                description: None,
                position: Position { x: 20.0, y: 340.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: EndNodeConfig { output_var: Some(spec.output_var.into()) },
        }),
    ];

    let edges: Vec<WorkflowEdge> = vec![
        WorkflowEdge {
            id: "e-trigger-agent".into(),
            source: "trigger".into(),
            source_handle: None,
            target: spec.agent_node_id.into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-agent-end".into(),
            source: spec.agent_node_id.into(),
            source_handle: None,
            target: "end".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
    ];

    // ── 版本检查与快照（与 seed_reflection_workflow_template 同款逻辑）──
    if let Some(ref existing) =
        workflow_template::Entity::find_by_id(spec.template_id).one(db).await.map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
        })?
    {
        if existing.version >= spec.version {
            tracing::info!(
                "[stock_analysis_setup] {} 模板已是最新 v{}，跳过种子化",
                spec.template_id,
                existing.version
            );
            return Ok(());
        }
        // 旧版本 → 保存快照
        let ver_id = format!("{}_v{}", spec.template_id, existing.version);
        if axagent_entities::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
            .is_none()
        {
            let snapshot = axagent_entities::workflow_template_version::ActiveModel {
                id: Set(ver_id.clone()),
                template_id: Set(spec.template_id.to_string()),
                name: Set(existing.name.clone()),
                description: Set(existing.description.clone()),
                icon: Set(existing.icon.clone()),
                tags: Set(existing.tags.clone()),
                version: Set(existing.version),
                is_preset: Set(existing.is_preset),
                is_editable: Set(existing.is_editable),
                is_public: Set(existing.is_public),
                trigger_config: Set(existing.trigger_config.clone()),
                nodes: Set(existing.nodes.clone()),
                edges: Set(existing.edges.clone()),
                input_schema: Set(existing.input_schema.clone()),
                output_schema: Set(existing.output_schema.clone()),
                variables: Set(existing.variables.clone()),
                error_config: Set(existing.error_config.clone()),
                created_at: Set(chrono::Utc::now().timestamp_millis()),
            };
            snapshot.insert(db).await.map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("写入版本快照失败: {e}"))
            })?;
            tracing::info!(
                "[stock_analysis_setup] {} 模板旧版本快照已保存: {ver_id}",
                spec.template_id
            );
        }
    }

    // 序列化节点/边/标签
    let nodes_json = serde_json::to_string(&nodes).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化节点失败: {e}"))
    })?;
    let edges_json = serde_json::to_string(&edges).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化边失败: {e}"))
    })?;
    let tags_json = serde_json::to_string(spec.tags).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化标签失败: {e}"))
    })?;
    let trigger_config_json = serde_json::to_string(&trigger_config).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化触发器配置失败: {e}"))
    })?;

    // 先删再插，避免 .save() 对已存在记录的 update 失败
    let _ = workflow_template::Entity::delete_by_id(spec.template_id).exec(db).await;
    workflow_template::ActiveModel {
        id: Set(spec.template_id.to_string()),
        cluster_id: Set(None),
        route_path: Set(None),
        name: Set(spec.name.to_string()),
        description: Set(Some(spec.description.to_string())),
        icon: Set(spec.icon.into()),
        tags: Set(Some(tags_json)),
        version: Set(spec.version),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(trigger_config_json)),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(None),
        error_config: Set(None),
        composite_source: Set(None),
        tool_defs: Set(None),
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL)
            .with_detail(format!("写入 {} 模板失败: {e}", spec.template_id))
    })?;

    tracing::info!(
        "[stock_analysis_setup] 决策事件订阅模板已创建: {} ({})",
        spec.template_id,
        spec.name
    );
    Ok(())
}

/// P2-2: 自动仓位规划模板。
///
/// 订阅 `decision.completed` 事件，决策落库后自动触发，基于决策的 action /
/// confidence / positionPct 等字段生成分批建仓 / 止损位 / 止盈位 / 资金分配
/// 方案。输出 JSON 结果到工作流执行历史，供前端展示与后续追溯。
async fn seed_auto_position_plan_template(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    let spec = EventTriggeredTemplateSpec {
        template_id: "auto-position-plan",
        name: "自动仓位规划",
        description: "订阅决策完成事件，自动生成分批建仓/止损/止盈/资金分配方案",
        icon: "wallet",
        agent_node_id: "position-planner",
        agent_node_title: "仓位规划助手",
        version: 1,
        tags: &["stock", "position", "auto", "A股"],
        agent_system_prompt: r#"基于上游交易决策，输出结构化的仓位执行方案。

【输入决策上下文】
- 股票代码: {{stock_code}}
- 股票名称: {{stock_name}}
- 决策动作: {{action}}（买入/增持/持有/减持/卖出/观望）
- 决策详情(JSON): {{decision_json}}
- 分析日期: {{as_of_date}}
- 分析记录 ID: {{analysis_id}}

请根据仓位规划方法论完成任务，输出 JSON 结果。"#,
        output_var: "position-plan",
        agent_profile_id: Some("stock-position-planner"),
        model_role: Some("decision-maker"),
    };
    seed_event_triggered_decision_template(db, spec).await
}

/// P2-2: 自动止损复查模板。
///
/// 订阅 `decision.completed` 事件，对决策的止损合理性进行独立复查，
/// 输出复查结论与调整建议。与 auto-position-plan 形成双视角对照。
async fn seed_auto_stop_loss_review_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    let spec = EventTriggeredTemplateSpec {
        template_id: "auto-stop-loss-review",
        name: "自动止损复查",
        description: "订阅决策完成事件，独立复查止损位合理性并输出调整建议",
        icon: "shield",
        agent_node_id: "stop-loss-reviewer",
        agent_node_title: "止损复查助手",
        version: 1,
        tags: &["stock", "risk", "auto", "A股"],
        agent_system_prompt: r#"对上游交易决策的止损合理性进行风控视角的二次审视。

【输入决策上下文】
- 股票代码: {{stock_code}}
- 股票名称: {{stock_name}}
- 决策动作: {{action}}
- 决策详情(JSON): {{decision_json}}
- 分析日期: {{as_of_date}}
- 分析记录 ID: {{analysis_id}}

请根据止损复查方法论完成任务，输出 JSON 结果。"#,
        output_var: "stop-loss-review",
        agent_profile_id: Some("stock-stop-loss-reviewer"),
        model_role: Some("decision-maker"),
    };
    seed_event_triggered_decision_template(db, spec).await
}
