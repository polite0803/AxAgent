// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 17 个领域的专家定义（编译期嵌入 .md 文件）
//!
//! 每个领域包含：
//! - 专家列表（id, name, prompt_content）
//! - 专家 → 工具白名单
//!
//! 注意：角色对应岗位，在 agent 节点中可以为空。
//! 专家（Expert）是核心，Profile 可以只绑定专家，不绑定角色。

// ── 学术研究（academic）— 2 个专家 ──

pub const ACADEMIC_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "academic-literature-reviewer",
        "文献综述专家",
        include_str!(
            "../../../agency_experts/opc/domains/academic/academic-literature-reviewer.md"
        ),
    ),
    (
        "academic-research-designer",
        "研究方案设计师",
        include_str!("../../../agency_experts/opc/domains/academic/academic-research-designer.md"),
    ),
];

pub const ACADEMIC_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("academic-literature-reviewer", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("academic-research-designer", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
];

// ── 设计（design）— 4 个专家 ──

pub const DESIGN_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "design-ux-researcher",
        "用户研究专家",
        include_str!("../../../agency_experts/opc/domains/design/design-ux-researcher.md"),
    ),
    (
        "design-prototype-designer",
        "原型设计专家",
        include_str!("../../../agency_experts/opc/domains/design/design-prototype-designer.md"),
    ),
    (
        "design-system-architect",
        "设计系统架构师",
        include_str!("../../../agency_experts/opc/domains/design/design-system-architect.md"),
    ),
    (
        "design-accessibility-auditor",
        "无障碍审计专家",
        include_str!("../../../agency_experts/opc/domains/design/design-accessibility-auditor.md"),
    ),
];

pub const DESIGN_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("design-ux-researcher", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("design-prototype-designer", &["FileRead", "FileWrite", "WebSearch"]),
    ("design-system-architect", &["FileRead", "FileWrite", "WebSearch"]),
    ("design-accessibility-auditor", &["Bash", "FileRead", "FileWrite", "WebSearch"]),
];

// ── 工程开发（engineering）— 13 个专家 ──

pub const ENGINEERING_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "engineering-technical-writer",
        "技术文档工程师",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-technical-writer.md"
        ),
    ),
    (
        "engineering-cross-language-migrator",
        "跨语言迁移专家",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-cross-language-migrator.md"
        ),
    ),
    (
        "engineering-senior-developer",
        "高级工程师",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-senior-developer.md"
        ),
    ),
    (
        "engineering-tech-lead",
        "技术负责人",
        include_str!("../../../agency_experts/opc/domains/engineering/engineering-tech-lead.md"),
    ),
    (
        "engineering-quality-engineer",
        "质量工程师",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-quality-engineer.md"
        ),
    ),
    (
        "engineering-refactor-consultant",
        "重构顾问",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-refactor-consultant.md"
        ),
    ),
    (
        "engineering-onboarding-specialist",
        "入职培训专家",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-onboarding-specialist.md"
        ),
    ),
    (
        "engineering-performance-engineer",
        "性能工程师",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-performance-engineer.md"
        ),
    ),
    (
        "engineering-db-migration-expert",
        "数据库迁移专家",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-db-migration-expert.md"
        ),
    ),
    (
        "engineering-code-reviewer",
        "代码审查专家",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-code-reviewer.md"
        ),
    ),
    (
        "engineering-devops-engineer",
        "DevOps 工程师",
        include_str!(
            "../../../agency_experts/opc/domains/engineering/engineering-devops-engineer.md"
        ),
    ),
    (
        "engineering-architect",
        "系统架构师",
        include_str!("../../../agency_experts/opc/domains/engineering/engineering-architect.md"),
    ),
    (
        "engineering-api-designer",
        "API 设计专家",
        include_str!("../../../agency_experts/opc/domains/engineering/engineering-api-designer.md"),
    ),
];

pub const ENGINEERING_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("engineering-technical-writer", &["FileRead", "FileWrite", "WebSearch"]),
    (
        "engineering-cross-language-migrator",
        &["FileRead", "FileWrite", "Bash", "Grep", "WebSearch"],
    ),
    ("engineering-senior-developer", &["FileRead", "FileWrite", "Bash", "Grep"]),
    ("engineering-tech-lead", &["FileRead", "FileWrite", "Grep", "WebSearch"]),
    ("engineering-quality-engineer", &["Bash", "FileRead", "FileWrite", "Grep"]),
    ("engineering-refactor-consultant", &["Bash", "FileRead", "Grep", "FileWrite", "WebSearch"]),
    ("engineering-onboarding-specialist", &["FileRead", "FileWrite", "Bash", "Grep"]),
    ("engineering-performance-engineer", &["Bash", "FileRead", "Grep", "FileWrite"]),
    ("engineering-db-migration-expert", &["Bash", "FileRead", "FileWrite", "Grep"]),
    ("engineering-code-reviewer", &["Bash", "FileRead", "Grep", "FileWrite"]),
    ("engineering-devops-engineer", &["Bash", "FileRead", "FileWrite", "Grep"]),
    ("engineering-architect", &["FileRead", "FileWrite", "Grep", "WebSearch"]),
    ("engineering-api-designer", &["FileRead", "FileWrite", "Grep", "WebSearch"]),
];

// ── 金融财务（finance）— 3 个专家 ──

pub const FINANCE_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "finance-investment-advisor",
        "投资顾问",
        include_str!("../../../agency_experts/opc/domains/finance/finance-investment-advisor.md"),
    ),
    (
        "finance-risk-analyzer",
        "风险分析师",
        include_str!("../../../agency_experts/opc/domains/finance/finance-risk-analyzer.md"),
    ),
    (
        "finance-financial-modeler",
        "财务建模师",
        include_str!("../../../agency_experts/opc/domains/finance/finance-financial-modeler.md"),
    ),
];

pub const FINANCE_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "finance-investment-advisor",
        &["OpcGetDashboard", "OpcListKpis", "WebSearch", "FileRead", "FileWrite"],
    ),
    ("finance-risk-analyzer", &["OpcGetDashboard", "OpcListKpis", "WebSearch", "FileRead"]),
    (
        "finance-financial-modeler",
        &["FileRead", "FileWrite", "OpcGetFinancialReport", "OpcListKpis", "WebSearch"],
    ),
];

// ── 游戏开发（gamedev）— 3 个专家 ──

pub const GAMEDEV_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "gamedev-game-tester",
        "游戏测试工程师",
        include_str!("../../../agency_experts/opc/domains/gamedev/gamedev-game-tester.md"),
    ),
    (
        "gamedev-game-developer",
        "游戏开发工程师",
        include_str!("../../../agency_experts/opc/domains/gamedev/gamedev-game-developer.md"),
    ),
    (
        "gamedev-game-designer",
        "游戏设计师",
        include_str!("../../../agency_experts/opc/domains/gamedev/gamedev-game-designer.md"),
    ),
];

pub const GAMEDEV_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("gamedev-game-tester", &["Bash", "FileRead", "FileWrite"]),
    ("gamedev-game-developer", &["FileRead", "FileWrite", "Bash", "Grep"]),
    ("gamedev-game-designer", &["FileRead", "FileWrite", "WebSearch", "Bash"]),
];

// ── 地理信息（gis）— 4 个专家 ──

pub const GIS_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "gis-report-compiler",
        "GIS 报告编撰师",
        include_str!("../../../agency_experts/opc/domains/gis/gis-report-compiler.md"),
    ),
    (
        "gis-planning-advisor",
        "空间规划顾问",
        include_str!("../../../agency_experts/opc/domains/gis/gis-planning-advisor.md"),
    ),
    (
        "gis-mapping-specialist",
        "GIS 制图专家",
        include_str!("../../../agency_experts/opc/domains/gis/gis-mapping-specialist.md"),
    ),
    (
        "gis-data-analyst",
        "GIS 数据分析师",
        include_str!("../../../agency_experts/opc/domains/gis/gis-data-analyst.md"),
    ),
];

pub const GIS_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("gis-report-compiler", &["FileRead", "FileWrite", "WebSearch"]),
    ("gis-planning-advisor", &["FileRead", "WebSearch", "OpcSearchWiki"]),
    ("gis-mapping-specialist", &["FileRead", "FileWrite", "WebSearch"]),
    ("gis-data-analyst", &["FileRead", "FileWrite", "WebSearch", "Bash"]),
];

// ── 市场营销（marketing）— 10 个专家 ──

pub const MARKETING_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "marketing-community-manager",
        "社区运营专家",
        include_str!(
            "../../../agency_experts/opc/domains/marketing/marketing-community-manager.md"
        ),
    ),
    (
        "marketing-product-marketer",
        "产品营销专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-product-marketer.md"),
    ),
    (
        "marketing-brand-strategist",
        "品牌策略专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-brand-strategist.md"),
    ),
    (
        "marketing-influencer-manager",
        "KOL 营销专家",
        include_str!(
            "../../../agency_experts/opc/domains/marketing/marketing-influencer-manager.md"
        ),
    ),
    (
        "marketing-analytics-expert",
        "营销分析专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-analytics-expert.md"),
    ),
    (
        "marketing-email-marketer",
        "邮件营销专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-email-marketer.md"),
    ),
    (
        "marketing-seo-specialist",
        "SEO 优化专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-seo-specialist.md"),
    ),
    (
        "marketing-social-media-manager",
        "社交媒体专家",
        include_str!(
            "../../../agency_experts/opc/domains/marketing/marketing-social-media-manager.md"
        ),
    ),
    (
        "marketing-content-creator",
        "内容创作专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-content-creator.md"),
    ),
    (
        "marketing-campaign-planner",
        "市场活动策划专家",
        include_str!("../../../agency_experts/opc/domains/marketing/marketing-campaign-planner.md"),
    ),
];

pub const MARKETING_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("marketing-community-manager", &["WebSearch", "FileRead", "FileWrite", "OpcSendNotification"]),
    ("marketing-product-marketer", &["WebSearch", "FileRead", "FileWrite"]),
    ("marketing-brand-strategist", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("marketing-influencer-manager", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("marketing-analytics-expert", &["OpcListKpis", "OpcGetDashboard", "WebSearch", "FileRead"]),
    ("marketing-email-marketer", &["FileRead", "FileWrite", "WebSearch", "OpcListCustomers"]),
    ("marketing-seo-specialist", &["WebSearch", "FileRead", "FileWrite"]),
    ("marketing-social-media-manager", &["WebSearch", "FileRead", "FileWrite"]),
    ("marketing-content-creator", &["FileRead", "FileWrite", "WebSearch"]),
    ("marketing-campaign-planner", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
];

// ── 付费媒体（paidmedia）— 2 个专家 ──

pub const PAIDMEDIA_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "paidmedia-ad-optimizer",
        "广告优化专家",
        include_str!("../../../agency_experts/opc/domains/paidmedia/paidmedia-ad-optimizer.md"),
    ),
    (
        "paidmedia-media-planner",
        "媒体策划专家",
        include_str!("../../../agency_experts/opc/domains/paidmedia/paidmedia-media-planner.md"),
    ),
];

pub const PAIDMEDIA_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("paidmedia-ad-optimizer", &["WebSearch", "FileRead", "FileWrite", "OpcGetDashboard"]),
    ("paidmedia-media-planner", &["WebSearch", "FileRead", "FileWrite", "OpcListKpis"]),
];

// ── 项目管理（pm）— 3 个专家 ──

pub const PM_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "pm-project-reporter",
        "项目报告专家",
        include_str!("../../../agency_experts/opc/domains/pm/pm-project-reporter.md"),
    ),
    (
        "pm-risk-manager",
        "风险管理专家",
        include_str!("../../../agency_experts/opc/domains/pm/pm-risk-manager.md"),
    ),
    (
        "pm-project-planner",
        "项目规划专家",
        include_str!("../../../agency_experts/opc/domains/pm/pm-project-planner.md"),
    ),
];

pub const PM_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("pm-project-reporter", &["OpcListProjects", "OpcListKpis", "OpcGetDashboard", "FileWrite"]),
    ("pm-risk-manager", &["OpcListProjects", "OpcGetDashboard", "FileRead", "FileWrite"]),
    ("pm-project-planner", &["OpcListProjects", "OpcCreateProject", "FileRead", "FileWrite"]),
];

// ── 产品管理（product）— 3 个专家 ──

pub const PRODUCT_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "product-product-launcher",
        "产品发布专家",
        include_str!("../../../agency_experts/opc/domains/product/product-product-launcher.md"),
    ),
    (
        "product-product-designer",
        "产品设计师",
        include_str!("../../../agency_experts/opc/domains/product/product-product-designer.md"),
    ),
    (
        "product-product-manager",
        "产品经理",
        include_str!("../../../agency_experts/opc/domains/product/product-product-manager.md"),
    ),
];

pub const PRODUCT_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "product-product-launcher",
        &["OpcListProjects", "OpcListLandingPages", "FileRead", "FileWrite", "WebSearch"],
    ),
    ("product-product-designer", &["FileRead", "FileWrite", "WebSearch"]),
    (
        "product-product-manager",
        &["OpcListProjects", "OpcCreateProject", "FileRead", "FileWrite", "WebSearch"],
    ),
];

// ── 销售（sales）— 5 个专家 ──

pub const SALES_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "sales-ops-analyst",
        "销售运营专家",
        include_str!("../../../agency_experts/opc/domains/sales/sales-ops-analyst.md"),
    ),
    (
        "sales-account-manager",
        "客户管理专家",
        include_str!("../../../agency_experts/opc/domains/sales/sales-account-manager.md"),
    ),
    (
        "sales-negotiator",
        "销售谈判专家",
        include_str!("../../../agency_experts/opc/domains/sales/sales-negotiator.md"),
    ),
    (
        "sales-prospector",
        "销售拓展专家",
        include_str!("../../../agency_experts/opc/domains/sales/sales-prospector.md"),
    ),
    (
        "sales-lead-generator",
        "线索生成专家",
        include_str!("../../../agency_experts/opc/domains/sales/sales-lead-generator.md"),
    ),
];

pub const SALES_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("sales-ops-analyst", &["OpcListKpis", "OpcGetDashboard", "OpcListCustomers", "FileRead"]),
    (
        "sales-account-manager",
        &["OpcListCustomers", "OpcCreateCustomer", "OpcListInvoices", "FileRead"],
    ),
    ("sales-negotiator", &["FileRead", "FileWrite", "OpcListCustomers", "OpcListInvoices"]),
    ("sales-prospector", &["WebSearch", "OpcListCustomers", "FileRead"]),
    ("sales-lead-generator", &["WebSearch", "OpcListCustomers", "FileRead", "FileWrite"]),
];

// ── 安全（security）— 4 个专家 ──

pub const SECURITY_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "security-patch-manager",
        "安全补丁管理专家",
        include_str!("../../../agency_experts/opc/domains/security/security-patch-manager.md"),
    ),
    (
        "security-incident-responder",
        "安全事件响应专家",
        include_str!("../../../agency_experts/opc/domains/security/security-incident-responder.md"),
    ),
    (
        "security-compliance-auditor",
        "安全合规审计专家",
        include_str!("../../../agency_experts/opc/domains/security/security-compliance-auditor.md"),
    ),
    (
        "security-risk-assessor",
        "安全风险评估专家",
        include_str!("../../../agency_experts/opc/domains/security/security-risk-assessor.md"),
    ),
];

pub const SECURITY_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("security-patch-manager", &["Bash", "FileRead", "WebSearch", "OpcSendNotification"]),
    ("security-incident-responder", &["Bash", "FileRead", "Grep", "WebSearch"]),
    ("security-compliance-auditor", &["FileRead", "WebSearch", "OpcSearchWiki"]),
    ("security-risk-assessor", &["Bash", "FileRead", "Grep", "WebSearch"]),
];

// ── 空间数据（spatial）— 2 个专家 ──

pub const SPATIAL_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "spatial-visualization-expert",
        "空间可视化专家",
        include_str!("../../../agency_experts/opc/domains/spatial/spatial-visualization-expert.md"),
    ),
    (
        "spatial-data-analyzer",
        "空间数据分析师",
        include_str!("../../../agency_experts/opc/domains/spatial/spatial-data-analyzer.md"),
    ),
];

pub const SPATIAL_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("spatial-visualization-expert", &["FileRead", "FileWrite", "WebSearch"]),
    ("spatial-data-analyzer", &["FileRead", "FileWrite", "Bash", "WebSearch"]),
];

// ── 专业领域（specialized）— 10 个专家 ──

pub const SPECIALIZED_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "specialized-ethics-reviewer",
        "伦理审查专家",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-ethics-reviewer.md"
        ),
    ),
    (
        "specialized-research-analyst",
        "研究分析师",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-research-analyst.md"
        ),
    ),
    (
        "specialized-change-manager",
        "变更管理专家",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-change-manager.md"
        ),
    ),
    (
        "specialized-training-specialist",
        "培训专家",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-training-specialist.md"
        ),
    ),
    (
        "specialized-legal-advisor",
        "法律顾问",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-legal-advisor.md"
        ),
    ),
    (
        "specialized-data-scientist",
        "数据科学家",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-data-scientist.md"
        ),
    ),
    (
        "specialized-quality-assurance",
        "质量保证专家",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-quality-assurance.md"
        ),
    ),
    (
        "specialized-implementation-engineer",
        "实施工程师",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-implementation-engineer.md"
        ),
    ),
    (
        "specialized-technical-architect",
        "技术架构师",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-technical-architect.md"
        ),
    ),
    (
        "specialized-domain-consultant",
        "专业咨询顾问",
        include_str!(
            "../../../agency_experts/opc/domains/specialized/specialized-domain-consultant.md"
        ),
    ),
];

pub const SPECIALIZED_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("specialized-ethics-reviewer", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("specialized-research-analyst", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
    ("specialized-change-manager", &["FileRead", "FileWrite", "WebSearch", "OpcSendNotification"]),
    ("specialized-training-specialist", &["FileRead", "FileWrite", "WebSearch"]),
    ("specialized-legal-advisor", &["WebSearch", "FileRead", "OpcSearchWiki"]),
    ("specialized-data-scientist", &["Bash", "FileRead", "FileWrite", "WebSearch"]),
    ("specialized-quality-assurance", &["Bash", "FileRead", "FileWrite", "Grep"]),
    ("specialized-implementation-engineer", &["FileRead", "FileWrite", "Bash", "Grep"]),
    ("specialized-technical-architect", &["FileRead", "FileWrite", "WebSearch", "Grep"]),
    ("specialized-domain-consultant", &["WebSearch", "FileRead", "FileWrite", "OpcSearchWiki"]),
];

// ── 战略规划（strategy）— 2 个专家 ──

pub const STRATEGY_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "strategy-implementation-planner",
        "战略实施规划师",
        include_str!(
            "../../../agency_experts/opc/domains/strategy/strategy-implementation-planner.md"
        ),
    ),
    (
        "strategy-business-analyst",
        "商业分析专家",
        include_str!("../../../agency_experts/opc/domains/strategy/strategy-business-analyst.md"),
    ),
];

pub const STRATEGY_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "strategy-implementation-planner",
        &["OpcListProjects", "OpcCreateProject", "FileRead", "FileWrite", "WebSearch"],
    ),
    (
        "strategy-business-analyst",
        &["WebSearch", "FileRead", "FileWrite", "OpcGetDashboard", "OpcListKpis"],
    ),
];

// ── 技术支持（support）— 3 个专家 ──

pub const SUPPORT_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "support-knowledge-base-writer",
        "知识库管理专家",
        include_str!(
            "../../../agency_experts/opc/domains/support/support-knowledge-base-writer.md"
        ),
    ),
    (
        "support-technical-specialist",
        "技术支持专家",
        include_str!("../../../agency_experts/opc/domains/support/support-technical-specialist.md"),
    ),
    (
        "support-ticket-manager",
        "工单管理专家",
        include_str!("../../../agency_experts/opc/domains/support/support-ticket-manager.md"),
    ),
];

pub const SUPPORT_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("support-knowledge-base-writer", &["FileRead", "FileWrite", "OpcSearchWiki", "WebSearch"]),
    ("support-technical-specialist", &["Bash", "FileRead", "Grep", "WebSearch", "OpcSearchWiki"]),
    ("support-ticket-manager", &["FileRead", "FileWrite", "WebSearch", "OpcSearchWiki"]),
];

// ── 测试（testing）— 3 个专家 ──

pub const TESTING_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "testing-test-analyst",
        "测试分析专家",
        include_str!("../../../agency_experts/opc/domains/testing/testing-test-analyst.md"),
    ),
    (
        "testing-test-executor",
        "测试执行专家",
        include_str!("../../../agency_experts/opc/domains/testing/testing-test-executor.md"),
    ),
    (
        "testing-test-planner",
        "测试规划专家",
        include_str!("../../../agency_experts/opc/domains/testing/testing-test-planner.md"),
    ),
];

pub const TESTING_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("testing-test-analyst", &["Bash", "FileRead", "FileWrite", "OpcListKpis"]),
    ("testing-test-executor", &["Bash", "FileRead", "FileWrite", "Grep"]),
    ("testing-test-planner", &["FileRead", "FileWrite", "Grep", "WebSearch"]),
];
