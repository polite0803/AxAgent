//! 股票分析专家与工作流模板种子化。
//!
//! 子模块：
//! - seed_stock_analysis: 股票分析主工作流模板种子
//! - seed_serenity: Serenity 瓶颈筛选工作流模板种子

pub mod seed_serenity;
pub mod seed_stock_analysis;
pub mod seed_variables;

// 股票分析专家/角色/Profile 自动种子化到 agency_experts/agent_roles/agent_profiles 表。
// 使用 include_str! 编译期嵌入 .md 内容，打包后无需文件 I/O。

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_setup;
use axagent_dao::repo;
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
];

struct StockRoleDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    system_prompt: &'static str,
    max_concurrent: i32,
    timeout_seconds: i64,
}

/// AxInvest 专属业务岗位 — 证券投资负责人。
///
/// 与 `agent_roles`（抽象执行器：stock-analyst / debater / trader 等）正交：
/// - BusinessRole 表达「在组织里担什么责」——证券投资决策的责任与合规边界
/// - AgentRole 表达「怎么干活」——执行器类型
///
/// 上游 agent_executor 4 层 prompt 拼接顺序（高 → 低）：
///   BusinessRole → AgentRole → Expert → 节点 inline
/// 本业务岗位的 system_prompt 作为最外层身份提示词注入。
const STOCK_BUSINESS_ROLE_ID: &str = "stock-investment-lead";

struct StockBusinessRoleDef {
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

const STOCK_BUSINESS_ROLE: StockBusinessRoleDef = StockBusinessRoleDef {
    id: STOCK_BUSINESS_ROLE_ID,
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
    system_prompt: "你是证券投资负责人，领导多专家团队进行 A 股投资分析与决策。所有结论须基于公开市场数据与已验证的研究方法论，杜绝内幕信息与市场操纵行为。决策以风险调整后收益最大化为目标，对每一条建议承担可追溯的合规责任。在分析过程中：1) 优先采纳已交叉验证的数据与证据；2) 对不确定性显式标注并量化置信度；3) 在多空辩论分歧时，要求辩手给出可证伪的判定条件；4) 对所有 LLM 输出保持怀疑，发现幻觉或与事实不符时立即标注 untrusted。",
    icon: "📈",
    color: "#dc2626",
};

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
            "get_stock_kline",
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
            "get_social_sentiment",
            "get_stock_news",
            "get_stock_money_flow",
            "get_stock_option_pcr",
            "get_stock_dragon_tiger",
            "get_north_bound_flow",
            "search_stock",
        ],
    ),
    (
        "news-analyst",
        &[
            "get_stock_news",
            "get_stock_announcements",
            "get_cls_flash",
            "get_stock_option_pcr",
            "search_stock",
        ],
    ),
    (
        "fundamentals-analyst",
        &[
            "get_stock_financials",
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
            "get_stock_money_flow",
            "get_stock_dragon_tiger",
            "get_north_bound_flow",
            "get_stock_institutional_visits",
            "search_stock",
        ],
    ),
    (
        "lockup-watcher",
        &[
            "get_stock_lockup_bundle",
            "get_stock_lockup",
            "get_stock_shareholder_trades",
            "get_stock_margin_data",
            "get_stock_announcements",
            "get_stock_block_trades",
            "search_stock",
        ],
    ),
    (
        "research-analyst",
        &[
            "get_stock_consensus_eps",
            "get_stock_financials",
            "get_stock_news",
            "get_stock_research_reports",
            "get_stock_institutional_visits",
            "search_stock",
        ],
    ),
    (
        "sector-analyst",
        &[
            "get_industry_ranking",
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
    // P2: 新增 get_announcement_content 调用 cninfo PDF 全文解析，突破标题级别信息局限
    (
        "catalyst-analyst",
        &[
            "get_stock_news",
            "get_stock_announcements",
            "get_announcement_content",
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
];

pub async fn ensure_stock_analysis_experts_seeded(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    // 先执行 Serenity 种子，独立 try 避免被前序步骤阻塞
    tracing::warn!("[stock_analysis_setup] === 开始种子 Serenity 模板 ===");
    if let Err(e) = seed_serenity_screening_workflow_template(db).await {
        tracing::error!("[stock_analysis_setup] Serenity 模板种子失败 (非致命): {e}");
    }
    tracing::warn!("[stock_analysis_setup] === Serenity 模板种子完成 ===");

    seed_agency_experts(db).await?;
    seed_agent_roles(db).await?;
    seed_business_role(db).await?;
    seed_agent_profiles(db).await?;
    seed_stock_analysis_workflow_template(db).await?;
    seed_reflection_workflow_template(db).await?;
    // seed_debate_subworkflow(db).await?;  // 辩论子工作流未引用，暂不种子化
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
            .map_err(|e| e.to_string())?
            .is_some()
        {
            active.update(db).await.map_err(|e| e.to_string())?;
        } else {
            active.insert(db).await.map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tracing::info!("[stock_analysis_setup] 已种子化/更新 {count} 个 agent_roles");
    Ok(())
}

/// 种子化 AxInvest 专属业务岗位 `stock-investment-lead`（证券投资负责人）。
///
/// 该岗位的 system_prompt 作为最外层身份提示词，通过上游 agent_executor 4 层
/// prompt 拼接（BusinessRole → AgentRole → Expert → 节点 inline）注入到所有
/// 股票专家 AgentProfile 的运行时上下文中。详见 STOCK_BUSINESS_ROLE 注释。
async fn seed_business_role(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    let r = STOCK_BUSINESS_ROLE;
    let responsibilities: Vec<String> = r.responsibilities.iter().map(|s| s.to_string()).collect();
    let certifications: Vec<String> =
        r.required_certifications.iter().map(|s| s.to_string()).collect();
    let domains: Vec<String> = r.active_domains.iter().map(|s| s.to_string()).collect();
    // managed_expert_ids 留空——股票专家众多且会动态增减，由前端按 source_dir="stock-analysis" 聚合
    repo::business_role::upsert_business_role(
        db,
        r.id,
        r.name,
        Some(r.description),
        Some(&responsibilities),
        Some(r.decision_authority),
        None,
        None,
        Some(&certifications),
        Some(&domains),
        r.system_prompt,
        Some(r.icon),
        Some(r.color),
        "stock-analysis",
        100,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("种子业务岗位失败: {e}"))
    })?;
    tracing::info!("[stock_analysis_setup] 已种子化/更新业务岗位 {} ({})", r.id, r.name);
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
            agent_role: Set(Some(role_id.into())),
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
            business_role_id: Set(Some(STOCK_BUSINESS_ROLE_ID.into())),
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
        let td_announce = ToolDef {
            name: "get_announcement_content".into(),
            description: Some("获取公司公告PDF全文内容，用于事后查阅分析期间发布的新公告".into()),
            parameters: Some(axagent_harness::workflow_types::JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some(
                    [(
                        "stock_code".into(),
                        axagent_harness::workflow_types::JsonSchemaProperty {
                            schema_type: "string".into(),
                            description: Some("6位股票代码".into()),
                            default: None,
                            enum_values: None,
                            format: None,
                        },
                    )]
                    .into_iter()
                    .collect(),
                ),
                required: Some(vec!["stock_code".into()]),
                items: None,
            }),
        };
        vec![td_kline, td_announce]
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
                    enabled: true,
                    match_threshold: 0.4,
                }),
                fallback_model: None,
                task_scene: None,
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
    // v1: 重新种子化
    // v2: 删除 sub-analysis SubWorkflowNode，改由 run_reflection_workflow
    //     从 stock_analyses.blackboard_snapshot 加载记忆注入 sub-analysis 变量
    // v3: 修复 reflection-agent input_mapping（trigger→变量名）;
    //     store-ref operation 改为 upsert 避免重复 row;
    //     params_suggestion schema 统一为数组格式;
    //     新增 implementation_tier/code_diff_proposal 闭环字段;
    //     新增 hindsight_date 变量注入
    // v4: 接入 reflection-validator CodeNode（原死代码）;
    //     DAG: reflection-agent → reflection-validator → store-ref;
    //     store-ref input_var 改为 reflection-validated;
    //     validator Rhai 脚本 params_suggestion 改为数组格式
    const REFLECTION_TEMPLATE_VERSION: i32 = 4;

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
