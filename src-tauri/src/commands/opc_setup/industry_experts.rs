// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 12 个行业的专家定义（编译期嵌入 .md 文件）
//!
//! 每个行业包含：
//! - 专家列表（id, name, prompt_content）
//! - 专家 → 工具白名单
//!
//! 注意：角色对应岗位，在 agent 节点中可以为空。
//! 专家（Expert）是核心，Profile 可以只绑定专家，不绑定角色。

// ── 会计与财务管理 ──

pub const ACCOUNTING_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "accounting-financial-clerk",
        "财务专员",
        include_str!(
            "../../../agency_experts/opc/industries/accounting/accounting-financial-clerk.md"
        ),
    ),
    (
        "accounting-financial-approver",
        "财务审批人",
        include_str!(
            "../../../agency_experts/opc/industries/accounting/accounting-financial-approver.md"
        ),
    ),
    (
        "accounting-financial-assistant",
        "财务助理",
        include_str!(
            "../../../agency_experts/opc/industries/accounting/accounting-financial-assistant.md"
        ),
    ),
    (
        "accounting-financial-analyst",
        "财务分析师",
        include_str!(
            "../../../agency_experts/opc/industries/accounting/accounting-financial-analyst.md"
        ),
    ),
];

pub const ACCOUNTING_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("accounting-financial-clerk", &["OpcCreateInvoice", "OpcListCustomers", "OpcListInvoices"]),
    ("accounting-financial-approver", &["OpcListInvoices", "OpcGetFinancialReport"]),
    ("accounting-financial-assistant", &["OpcSendNotification", "OpcListInvoices"]),
    ("accounting-financial-analyst", &["OpcGetFinancialReport", "OpcRecordKpi", "OpcListKpis"]),
];

// ── 金融投资 ──

pub const FINANCE_INVEST_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "finance-market-analyst",
        "市场分析师",
        include_str!(
            "../../../agency_experts/opc/industries/finance_invest/finance-market-analyst.md"
        ),
    ),
    (
        "finance-industry-researcher",
        "行业研究员",
        include_str!(
            "../../../agency_experts/opc/industries/finance_invest/finance-industry-researcher.md"
        ),
    ),
    (
        "finance-asset-allocator",
        "资产配置专家",
        include_str!(
            "../../../agency_experts/opc/industries/finance_invest/finance-asset-allocator.md"
        ),
    ),
    (
        "finance-trade-executor",
        "交易执行专家",
        include_str!(
            "../../../agency_experts/opc/industries/finance_invest/finance-trade-executor.md"
        ),
    ),
    (
        "finance-portfolio-reviewer",
        "投资回顾专家",
        include_str!(
            "../../../agency_experts/opc/industries/finance_invest/finance-portfolio-reviewer.md"
        ),
    ),
];

pub const FINANCE_INVEST_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("finance-market-analyst", &["OpcGetDashboard", "OpcListKpis", "OpcListCustomers"]),
    ("finance-industry-researcher", &["OpcSearchWiki", "OpcListProjects"]),
    ("finance-asset-allocator", &["OpcGetFinancialReport", "OpcGetDashboard"]),
    ("finance-trade-executor", &["OpcSendNotification", "OpcGetDashboard"]),
    ("finance-portfolio-reviewer", &["OpcGetFinancialReport", "OpcRecordKpi"]),
];

// ── 游戏开发 ──

pub const GAME_DEV_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "game-concept-designer",
        "游戏概念设计师",
        include_str!("../../../agency_experts/opc/industries/game_dev/game-concept-designer.md"),
    ),
    (
        "game-prototype-developer",
        "原型开发专家",
        include_str!("../../../agency_experts/opc/industries/game_dev/game-prototype-developer.md"),
    ),
    (
        "game-content-designer",
        "内容设计师",
        include_str!("../../../agency_experts/opc/industries/game_dev/game-content-designer.md"),
    ),
    (
        "game-qa-expert",
        "游戏QA专家",
        include_str!("../../../agency_experts/opc/industries/game_dev/game-qa-expert.md"),
    ),
    (
        "game-operations-expert",
        "游戏运营专家",
        include_str!("../../../agency_experts/opc/industries/game_dev/game-operations-expert.md"),
    ),
];

pub const GAME_DEV_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("game-concept-designer", &["WebSearch"]),
    ("game-prototype-developer", &["FileWrite", "WebSearch"]),
    ("game-content-designer", &["FileWrite"]),
    ("game-qa-expert", &["FileRead", "WebSearch"]),
    ("game-operations-expert", &["WebSearch"]),
];

// ── 设计 ──

pub const DESIGN_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "design-ui-designer",
        "UI设计师",
        include_str!("../../../agency_experts/opc/industries/design/design-ui-designer.md"),
    ),
    (
        "design-brand-strategist",
        "品牌设计师",
        include_str!("../../../agency_experts/opc/industries/design/design-brand-strategist.md"),
    ),
    (
        "design-system-architect",
        "设计系统专家",
        include_str!("../../../agency_experts/opc/industries/design/design-system-architect.md"),
    ),
];

pub const DESIGN_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("design-ui-designer", &["FileWrite", "OpcListProjects"]),
    ("design-brand-strategist", &["FileWrite", "WebSearch"]),
    ("design-system-architect", &["FileWrite", "FileRead"]),
];

// ── 电子商务 ──

pub const ECOMMERCE_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "ecommerce-product-scout",
        "选品专家",
        include_str!("../../../agency_experts/opc/industries/ecommerce/ecommerce-product-scout.md"),
    ),
    (
        "ecommerce-competitor-analyst",
        "竞品分析师",
        include_str!(
            "../../../agency_experts/opc/industries/ecommerce/ecommerce-competitor-analyst.md"
        ),
    ),
    (
        "ecommerce-marketing-planner",
        "营销专家",
        include_str!(
            "../../../agency_experts/opc/industries/ecommerce/ecommerce-marketing-planner.md"
        ),
    ),
];

pub const ECOMMERCE_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("ecommerce-product-scout", &["WebSearch", "OpcListProducts"]),
    ("ecommerce-competitor-analyst", &["WebSearch", "OpcListCustomers"]),
    ("ecommerce-marketing-planner", &["OpcCreateBlogPost", "WebSearch", "OpcListLandingPages"]),
];

// ── 教育培训 ──

pub const EDUCATION_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "education-curriculum-designer",
        "课程设计师",
        include_str!(
            "../../../agency_experts/opc/industries/education/education-curriculum-designer.md"
        ),
    ),
    (
        "education-content-creator",
        "内容创作专家",
        include_str!(
            "../../../agency_experts/opc/industries/education/education-content-creator.md"
        ),
    ),
    (
        "education-assessment-expert",
        "评估专家",
        include_str!(
            "../../../agency_experts/opc/industries/education/education-assessment-expert.md"
        ),
    ),
];

pub const EDUCATION_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("education-curriculum-designer", &["OpcCreateProject", "FileWrite"]),
    ("education-content-creator", &["FileWrite", "OpcCreateBlogPost"]),
    ("education-assessment-expert", &["OpcRecordKpi", "OpcListKpis", "FileRead"]),
];

// ── 地理信息 ──

pub const GEOSPATIAL_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "geospatial-data-analyst",
        "地理数据分析师",
        include_str!(
            "../../../agency_experts/opc/industries/geospatial/geospatial-data-analyst.md"
        ),
    ),
    (
        "geospatial-visualization-expert",
        "可视化专家",
        include_str!(
            "../../../agency_experts/opc/industries/geospatial/geospatial-visualization-expert.md"
        ),
    ),
    (
        "geospatial-planning-advisor",
        "规划顾问",
        include_str!(
            "../../../agency_experts/opc/industries/geospatial/geospatial-planning-advisor.md"
        ),
    ),
];

pub const GEOSPATIAL_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("geospatial-data-analyst", &["WebSearch", "FileRead"]),
    ("geospatial-visualization-expert", &["FileWrite", "WebSearch"]),
    ("geospatial-planning-advisor", &["OpcSearchWiki", "WebSearch"]),
];

// ── 行业咨询 ──

pub const INDUSTRY_CONSULTING_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "industry-research-analyst",
        "行业研究员",
        include_str!(
            "../../../agency_experts/opc/industries/industry_consulting/industry-research-analyst.md"
        ),
    ),
    (
        "industry-strategy-advisor",
        "战略顾问",
        include_str!(
            "../../../agency_experts/opc/industries/industry_consulting/industry-strategy-advisor.md"
        ),
    ),
    (
        "industry-implementation-coach",
        "实施教练",
        include_str!(
            "../../../agency_experts/opc/industries/industry_consulting/industry-implementation-coach.md"
        ),
    ),
];

pub const INDUSTRY_CONSULTING_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("industry-research-analyst", &["WebSearch", "OpcSearchWiki"]),
    ("industry-strategy-advisor", &["OpcGetDashboard", "OpcSearchWiki"]),
    ("industry-implementation-coach", &["OpcCreateProject", "OpcAddMilestone", "OpcListProjects"]),
];

// ── 项目管理 ──

pub const PROJECT_MANAGEMENT_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "project-planning-expert",
        "项目规划专家",
        include_str!(
            "../../../agency_experts/opc/industries/project_management/project-planning-expert.md"
        ),
    ),
    (
        "project-monitoring-analyst",
        "项目监控分析师",
        include_str!(
            "../../../agency_experts/opc/industries/project_management/project-monitoring-analyst.md"
        ),
    ),
    (
        "project-delivery-manager",
        "交付管理专家",
        include_str!(
            "../../../agency_experts/opc/industries/project_management/project-delivery-manager.md"
        ),
    ),
];

pub const PROJECT_MANAGEMENT_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("project-planning-expert", &["OpcCreateProject", "OpcAddMilestone", "OpcListProjects"]),
    ("project-monitoring-analyst", &["OpcListProjects", "OpcGetDashboard"]),
    ("project-delivery-manager", &["OpcListProjects", "OpcSendNotification"]),
];

// ── 销售增长 ──

pub const SALES_GROWTH_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "sales-lead-generator",
        "线索生成专家",
        include_str!("../../../agency_experts/opc/industries/sales_growth/sales-lead-generator.md"),
    ),
    (
        "sales-negotiation-expert",
        "谈判专家",
        include_str!(
            "../../../agency_experts/opc/industries/sales_growth/sales-negotiation-expert.md"
        ),
    ),
    (
        "sales-growth-analyst",
        "增长分析师",
        include_str!("../../../agency_experts/opc/industries/sales_growth/sales-growth-analyst.md"),
    ),
];

pub const SALES_GROWTH_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("sales-lead-generator", &["OpcListCustomers", "OpcCreateCustomer", "WebSearch"]),
    ("sales-negotiation-expert", &["OpcListInvoices", "OpcCreateInvoice"]),
    ("sales-growth-analyst", &["OpcGetDashboard", "OpcListKpis", "OpcRecordKpi"]),
];

// ── 安全合规 ──

pub const SECURITY_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "security-compliance-auditor",
        "合规审计师",
        include_str!(
            "../../../agency_experts/opc/industries/security/security-compliance-auditor.md"
        ),
    ),
    (
        "security-incident-responder",
        "事件响应专家",
        include_str!(
            "../../../agency_experts/opc/industries/security/security-incident-responder.md"
        ),
    ),
    (
        "security-risk-assessor",
        "风险评估专家",
        include_str!("../../../agency_experts/opc/industries/security/security-risk-assessor.md"),
    ),
];

pub const SECURITY_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("security-compliance-auditor", &["OpcSearchWiki", "FileRead"]),
    ("security-incident-responder", &["OpcSendNotification", "OpcGetDashboard"]),
    ("security-risk-assessor", &["OpcSearchWiki", "OpcGetDashboard"]),
];

// ── 软件开发 ──

pub const SOFTWARE_DEV_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "software-architect",
        "软件架构师",
        include_str!("../../../agency_experts/opc/industries/software_dev/software-architect.md"),
    ),
    (
        "software-developer",
        "开发专家",
        include_str!("../../../agency_experts/opc/industries/software_dev/software-developer.md"),
    ),
    (
        "software-quality-expert",
        "质量专家",
        include_str!(
            "../../../agency_experts/opc/industries/software_dev/software-quality-expert.md"
        ),
    ),
];

pub const SOFTWARE_DEV_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("software-architect", &["OpcSearchWiki", "FileRead", "WebSearch"]),
    ("software-developer", &["FileWrite", "FileRead"]),
    ("software-quality-expert", &["FileRead", "WebSearch"]),
];
