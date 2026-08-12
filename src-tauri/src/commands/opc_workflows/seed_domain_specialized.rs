// SPDX-License-Identifier: AGPL-3.0-only

//! 专业服务（specialized）领域工作流种子化 — 10 个工作流
//!
//! 生成的工作流：
//! - wf-spc-change-mgmt: 变更管理
//! - wf-spc-data-privacy: 数据隐私合规
//! - wf-spc-esg: ESG报告
//! - wf-spc-grant: 项目申请
//! - wf-spc-hire: 招聘流程
//! - wf-spc-legal-review: 合同审查
//! - wf-spc-localization: 本地化
//! - wf-spc-m-a: 并购整合
//! - wf-spc-onboard: 员工入职
//! - wf-spc-supply-chain: 供应链优化

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化专业服务领域的全部工作流
pub(crate) async fn seed_domain_specialized_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // wf-spc-change-mgmt: 变更管理
    if seed_domain_template(db, build_domain_template(
        "wf-spc-change-mgmt", "变更管理", "评估变革对组织、流程、人员的影响 → 制定分阶段变革实施和沟通计划 → 监督执行并收集反馈调整", "🔄",
        vec!["opc".to_string(), "specialized".to_string()],
        "opc-ceo-ceo-business-strategist",
        vec![
            make_trigger(250.0, 0.0),
            make_agent_node("a-change-impact", "影响评估", "评估变革对组织、流程、人员的影响", vec![], Some("opc-ceo-ceo-business-strategist"), "a-change-impact_result", 250.0, 150.0),
            make_agent_node("a-change-plan", "变革计划", "制定分阶段变革实施和沟通计划", vec![], Some("opc-ceo-ceo-business-strategist"), "a-change-plan_result", 250.0, 350.0),
            make_agent_node("a-change-exec", "变革执行", "监督执行并收集反馈调整", vec![], Some("opc-ceo-ceo-business-strategist"), "a-change-exec_result", 250.0, 550.0),
            make_end(250.0, 750.0),
        ],
        vec![
            edge("e-trigger-a-change-impact", "trigger", "a-change-impact"),
            edge("e-a-change-impact-a-change-plan", "a-change-impact", "a-change-plan"),
            edge("e-a-change-plan-a-change-exec", "a-change-plan", "a-change-exec"),
            edge("e-a-change-exec-end", "a-change-exec", "end"),
        ],
    )).await? {
        seeded += 1;
    }

    // wf-spc-data-privacy: 数据隐私合规
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-data-privacy",
            "数据隐私合规",
            "审计数据采集、存储、处理流程 → 识别合规差距和风险等级 → 实施整改措施并验证",
            "🔒",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-privacy-audit",
                    "隐私审计",
                    "审计数据采集、存储、处理流程",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-privacy-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-privacy-gap",
                    "差距分析",
                    "识别合规差距和风险等级",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-privacy-gap_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-privacy-fix",
                    "整改实施",
                    "实施整改措施并验证",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-privacy-fix_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-privacy-audit", "trigger", "a-privacy-audit"),
                edge("e-a-privacy-audit-a-privacy-gap", "a-privacy-audit", "a-privacy-gap"),
                edge("e-a-privacy-gap-a-privacy-fix", "a-privacy-gap", "a-privacy-fix"),
                edge("e-a-privacy-fix-end", "a-privacy-fix", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-esg: ESG报告
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-esg",
            "ESG报告",
            "收集环境、社会、治理数据 → 计算ESG关键指标和评分 → 生成ESG报告和改善路线图",
            "🌱",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-esg-collect",
                    "ESG数据收集",
                    "收集环境、社会、治理数据",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-esg-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-esg-measure",
                    "ESG指标计算",
                    "计算ESG关键指标和评分",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-esg-measure_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-esg-report",
                    "ESG报告生成",
                    "生成ESG报告和改善路线图",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-esg-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-esg-collect", "trigger", "a-esg-collect"),
                edge("e-a-esg-collect-a-esg-measure", "a-esg-collect", "a-esg-measure"),
                edge("e-a-esg-measure-a-esg-report", "a-esg-measure", "a-esg-report"),
                edge("e-a-esg-report-end", "a-esg-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-grant: 项目申请
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-grant",
            "项目申请",
            "研究适合的项目和资助机构 → 撰写项目申请书和预算 → 最终审核并提交申请",
            "📝",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-grant-research",
                    "项目研究",
                    "研究适合的项目和资助机构",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-grant-research_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-grant-write",
                    "撰写申请",
                    "撰写项目申请书和预算",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-grant-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-grant-submit",
                    "审核提交",
                    "最终审核并提交申请",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-grant-submit_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-grant-research", "trigger", "a-grant-research"),
                edge("e-a-grant-research-a-grant-write", "a-grant-research", "a-grant-write"),
                edge("e-a-grant-write-a-grant-submit", "a-grant-write", "a-grant-submit"),
                edge("e-a-grant-submit-end", "a-grant-submit", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-hire: 招聘流程
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-hire",
            "招聘流程",
            "撰写职位描述和要求 → 筛选简历、安排面试 → 综合评估候选人、产出报告",
            "🎯",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-hire-jd",
                    "职位描述",
                    "撰写职位描述和要求",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-hire-jd_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-hire-screen",
                    "简历筛选",
                    "筛选简历、安排面试",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-hire-screen_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-hire-evaluate",
                    "综合评估",
                    "综合评估候选人、产出报告",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-hire-evaluate_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-hire-jd", "trigger", "a-hire-jd"),
                edge("e-a-hire-jd-a-hire-screen", "a-hire-jd", "a-hire-screen"),
                edge("e-a-hire-screen-a-hire-evaluate", "a-hire-screen", "a-hire-evaluate"),
                edge("e-a-hire-evaluate-end", "a-hire-evaluate", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-legal-review: 合同审查
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-legal-review",
            "合同审查",
            "提交合同文档和背景说明 → 审查关键条款、风险点、合规性 → 输出审查意见和修改建议",
            "⚖️",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-legal-upload",
                    "合同提交",
                    "提交合同文档和背景说明",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-legal-upload_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-legal-review",
                    "条款审查",
                    "审查关键条款、风险点、合规性",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-legal-review_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-legal-report",
                    "审查报告",
                    "输出审查意见和修改建议",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-legal-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-legal-upload", "trigger", "a-legal-upload"),
                edge("e-a-legal-upload-a-legal-review", "a-legal-upload", "a-legal-review"),
                edge("e-a-legal-review-a-legal-report", "a-legal-review", "a-legal-report"),
                edge("e-a-legal-report-end", "a-legal-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-localization: 本地化
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-localization",
            "本地化",
            "审计需要本地化的内容和功能 → 翻译内容、适配格式和规范 → 验证本地化质量和一致性",
            "🌍",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-locale-audit",
                    "本地化审计",
                    "审计需要本地化的内容和功能",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-locale-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-locale-translate",
                    "翻译适配",
                    "翻译内容、适配格式和规范",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-locale-translate_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-locale-verify",
                    "质量验证",
                    "验证本地化质量和一致性",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-locale-verify_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-locale-audit", "trigger", "a-locale-audit"),
                edge("e-a-locale-audit-a-locale-translate", "a-locale-audit", "a-locale-translate"),
                edge(
                    "e-a-locale-translate-a-locale-verify",
                    "a-locale-translate",
                    "a-locale-verify",
                ),
                edge("e-a-locale-verify-end", "a-locale-verify", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-m-a: 并购整合
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-m-a",
            "并购整合",
            "审计目标公司业务、技术、团队 → 制定100天整合计划 → 执行整合并监控关键指标",
            "🤝",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ma-audit",
                    "尽职审计",
                    "审计目标公司业务、技术、团队",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-ma-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ma-plan",
                    "整合计划",
                    "制定100天整合计划",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-ma-plan_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ma-exec",
                    "整合执行",
                    "执行整合并监控关键指标",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-ma-exec_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ma-audit", "trigger", "a-ma-audit"),
                edge("e-a-ma-audit-a-ma-plan", "a-ma-audit", "a-ma-plan"),
                edge("e-a-ma-plan-a-ma-exec", "a-ma-plan", "a-ma-exec"),
                edge("e-a-ma-exec-end", "a-ma-exec", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-onboard: 员工入职
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-onboard",
            "员工入职",
            "制定入职计划和任务清单 → 开通账号、配置设备、访问权限 → 公司介绍、团队介绍、首周任务",
            "📋",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-onboard-plan",
                    "入职计划",
                    "制定入职计划和任务清单",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-onboard-plan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-onboard-setup",
                    "账号配置",
                    "开通账号、配置设备、访问权限",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-onboard-setup_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-onboard-orient",
                    "入职引导",
                    "公司介绍、团队介绍、首周任务",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-onboard-orient_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-onboard-plan", "trigger", "a-onboard-plan"),
                edge("e-a-onboard-plan-a-onboard-setup", "a-onboard-plan", "a-onboard-setup"),
                edge("e-a-onboard-setup-a-onboard-orient", "a-onboard-setup", "a-onboard-orient"),
                edge("e-a-onboard-orient-end", "a-onboard-orient", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-spc-supply-chain: 供应链优化
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spc-supply-chain",
            "供应链优化",
            "审计采购、库存、物流各环节 → 制定降本增效方案 → 实施优化并跟踪KPI",
            "📦",
            vec!["opc".to_string(), "specialized".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-sc-audit",
                    "供应链审计",
                    "审计采购、库存、物流各环节",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sc-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-sc-optimize",
                    "优化方案",
                    "制定降本增效方案",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sc-optimize_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-sc-implement",
                    "实施跟踪",
                    "实施优化并跟踪KPI",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sc-implement_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-sc-audit", "trigger", "a-sc-audit"),
                edge("e-a-sc-audit-a-sc-optimize", "a-sc-audit", "a-sc-optimize"),
                edge("e-a-sc-optimize-a-sc-implement", "a-sc-optimize", "a-sc-implement"),
                edge("e-a-sc-implement-end", "a-sc-implement", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
