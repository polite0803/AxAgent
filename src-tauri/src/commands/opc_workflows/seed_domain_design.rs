// SPDX-License-Identifier: AGPL-3.0-only

//! 设计与创意（design）领域工作流种子化 — 4 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-des-accessibility:  无障碍审计（扫描 → 分类 → 严重问题逐项修复循环 → 验证报告）
//! - wf-des-design-system:  设计系统（审计 → 规范 → 一致性判定 → 组件打磨循环 → 文档 → 审批）
//! - wf-des-prototype:      原型设计（线框 → 低保真自检 → 高保真/结构调整分支 → 审批）
//! - wf-des-ux-research:    用户研究（研究设计 → 知识检索 → 逐用户分析循环 → 洞察报告）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{
    CompareOperator, Condition, EdgeType, LogicalOperator, LoopType,
};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cpo-cpo-product-manager";
/// design 领域模板版本（v4 丰富拓扑）
const DES_TEMPLATE_VERSION: i32 = 4;

/// 种子化设计与创意领域的全部工作流
pub(crate) async fn seed_domain_design_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-des-accessibility: 无障碍审计 ─────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-des-accessibility",
            "无障碍审计",
            "无障碍审计：扫描产品无障碍问题，按严重度分类，严重问题逐项修复，输出验证报告",
            "♿",
            vec!["opc".to_string(), "design".to_string()],
            DES_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：扫描
                make_agent_node(
                    "a-a11y-scan",
                    "扫描",
                    "扫描产品的无障碍问题：对比度、可访问名称、键盘导航、屏幕阅读器兼容性。\
                     输出 JSON：{\"issues\":[{\"element\":\"\", \"type\":\"contrast|aria|keyboard|screen_reader\", \"severity\":\"high|medium|low\", \"description\":\"\"}], \"high_count\":0}",
                    vec![td_desc("OpcSearchWiki", "检索无障碍规范 WCAG")],
                    Some(PROFILE),
                    "a-a11y-scan",
                    0.0,
                    180.0,
                ),
                // Agent：问题分类
                make_agent_node_full(
                    "a-a11y-report",
                    "问题分类",
                    "将扫描结果按严重程度与影响面分类排序，标注修复优先级。\
                     输出 JSON：{\"categories\":[{\"severity\":\"\", \"count\":0, \"items\":[]}], \"recommended_order\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-a11y-report",
                    vec![("scan", "a-a11y-scan")],
                    vec!["a-a11y-scan"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在严重问题
                make_condition_node(
                    "c-a11y-critical",
                    "严重度判定",
                    vec![Condition {
                        var_path: "a-a11y-scan.high_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // Loop：逐项修复严重问题
                make_loop_node(
                    "l-a11y-fix",
                    "逐项修复",
                    LoopType::ForEach,
                    Some("a-a11y-scan"),
                    Some("issue_item"),
                    Some("l-a11y-fix"),
                    Some("l-a11y-fix__partial"),
                    Some(50),
                    vec!["a-a11y-fix".to_string()],
                    -250.0,
                    720.0,
                ),
                // Loop body：单项修复
                make_agent_node_full(
                    "a-a11y-fix",
                    "单项修复",
                    "针对当前无障碍问题给出具体修复方案：修改建议、实现方式、验收标准。\
                     输出 JSON：{\"element\":\"\", \"fix\":\"\", \"implementation\":\"\", \"acceptance\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-a11y-fix",
                    vec![("issue", "issue_item")],
                    vec!["a-a11y-scan"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-a11y", "汇合", 0.0, 900.0),
                // Agent：验证报告
                make_agent_node_full(
                    "a-a11y-verify",
                    "验证报告",
                    "汇总修复结果，输出无障碍审计报告：剩余问题、合规状态、后续建议。\
                     输出 JSON：{\"compliance\":\"wcag_aa|partial|fail\", \"remaining\":[], \"summary\":\"\", \"next_steps\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-a11y-verify",
                    vec![("report", "a-a11y-report"), ("fixes", "l-a11y-fix.items")],
                    vec!["a-a11y-report", "l-a11y-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-scan", "trigger", "a-a11y-scan"),
                edge("e-scan-report", "a-a11y-scan", "a-a11y-report"),
                edge("e-report-critical", "a-a11y-report", "c-a11y-critical"),
                edge_cond("e-critical-loop", "c-a11y-critical", "true", "l-a11y-fix", EdgeType::ConditionTrue),
                edge_cond("e-clean-merge", "c-a11y-critical", "false", "m-a11y", EdgeType::ConditionFalse),
                edge("e-loop-merge", "l-a11y-fix", "m-a11y"),
                edge("e-merge-verify", "m-a11y", "a-a11y-verify"),
                edge("e-verify-end", "a-a11y-verify", "end"),
            ],
            vec![DomainInputField { key: "product_url", label: "产品地址", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-des-design-system: 设计系统 ──────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-des-design-system",
            "设计系统",
            "设计系统：审计现有设计元件，构建组件规范，一致性不足逐组件打磨，输出文档并审批",
            "📐",
            vec!["opc".to_string(), "design".to_string()],
            DES_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：元件审计
                make_agent_node(
                    "a-ds-audit",
                    "元件审计",
                    "审计现有设计元件与模式：颜色、字体、间距、组件用法，识别不一致项。\
                     输出 JSON：{\"tokens\":[], \"patterns\":[{\"name\":\"\", \"usage\":[], \"inconsistencies\":[]}], \"inconsistency_count\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-ds-audit",
                    0.0,
                    180.0,
                ),
                // Agent：组件规范
                make_agent_node_full(
                    "a-ds-components",
                    "组件规范",
                    "构建核心组件库与规范：设计令牌、组件变体、状态与使用规则。\
                     输出 JSON：{\"tokens\":[{\"name\":\"\", \"value\":\"\", \"usage\":\"\"}], \"components\":[{\"name\":\"\", \"variants\":[], \"states\":[], \"rules\":[]}]}",
                    vec![],
                    Some(PROFILE),
                    "a-ds-components",
                    vec![("audit", "a-ds-audit")],
                    vec!["a-ds-audit"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在不一致
                make_condition_node(
                    "c-ds-consistency",
                    "一致性判定",
                    vec![Condition {
                        var_path: "a-ds-audit.inconsistency_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // Loop：逐组件打磨
                make_loop_node(
                    "l-ds-polish",
                    "组件打磨",
                    LoopType::ForEach,
                    Some("a-ds-components"),
                    Some("component_item"),
                    Some("l-ds-polish"),
                    Some("l-ds-polish__partial"),
                    Some(30),
                    vec!["a-ds-polish".to_string()],
                    -250.0,
                    720.0,
                ),
                // Loop body：单组件打磨
                make_agent_node_full(
                    "a-ds-polish",
                    "组件打磨",
                    "针对当前组件对照规范打磨：补齐变体/状态、修正用法、标注依赖。\
                     输出 JSON：{\"component\":\"\", \"gaps\":[], \"refinements\":[], \"dependencies\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-ds-polish",
                    vec![("component", "component_item")],
                    vec!["a-ds-components"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-ds", "汇合", 0.0, 900.0),
                // Agent：使用文档
                make_agent_node_full(
                    "a-ds-doc",
                    "文档输出",
                    "输出设计系统使用文档：快速上手、组件清单、最佳实践与常见陷阱。\
                     输出 JSON：{\"getting_started\":\"\", \"component_index\":[], \"best_practices\":[], \"pitfalls\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-ds-doc",
                    vec![("components", "a-ds-components"), ("polish", "l-ds-polish.items")],
                    vec!["a-ds-components", "l-ds-polish"],
                    0.0,
                    1080.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-ds",
                    "设计系统审批",
                    "设计系统文档已完成，请设计负责人审批",
                    Some("design_lead"),
                    86400,
                    "ap-ds",
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-audit", "trigger", "a-ds-audit"),
                edge("e-audit-components", "a-ds-audit", "a-ds-components"),
                edge("e-components-consistency", "a-ds-components", "c-ds-consistency"),
                edge_cond("e-inconsistent-loop", "c-ds-consistency", "true", "l-ds-polish", EdgeType::ConditionTrue),
                edge_cond("e-consistent-merge", "c-ds-consistency", "false", "m-ds", EdgeType::ConditionFalse),
                edge("e-loop-merge", "l-ds-polish", "m-ds"),
                edge("e-merge-doc", "m-ds", "a-ds-doc"),
                edge("e-doc-approval", "a-ds-doc", "ap-ds"),
                edge("e-approval-end", "ap-ds", "end"),
            ],
            vec![DomainInputField { key: "product_name", label: "产品名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-des-prototype: 原型设计 ──────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-des-prototype",
            "原型设计",
            "原型设计：绘制线框图，自检信息架构，通过后高保真制作交互原型，经审批交付",
            "🎨",
            vec!["opc".to_string(), "design".to_string()],
            DES_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：线框图
                make_agent_node(
                    "a-proto-wireframe",
                    "线框图",
                    "绘制页面结构与布局线框图，标注信息层级与交互入口。\
                     输出 JSON：{\"screens\":[{\"name\":\"\", \"layout\":\"\", \"elements\":[], \"interactions\":[]}], \"ia\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-proto-wireframe",
                    0.0,
                    180.0,
                ),
                // 条件：信息架构自检
                make_condition_node(
                    "c-proto-ia",
                    "架构自检",
                    vec![Condition {
                        var_path: "a-proto-wireframe.screens".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 通过：高保真
                make_agent_node_full(
                    "a-proto-mockup",
                    "高保真设计",
                    "基于线框图设计高保真模型：视觉规范、组件状态、响应式适配。\
                     输出 JSON：{\"mockups\":[{\"screen\":\"\", \"style\":\"\", \"components\":[], \"responsive\":\"\"}], \"design_tokens\":{}}",
                    vec![],
                    Some(PROFILE),
                    "a-proto-mockup",
                    vec![("wireframe", "a-proto-wireframe")],
                    vec!["a-proto-wireframe"],
                    -250.0,
                    540.0,
                ),
                // 未通过：结构调整
                make_agent_node_full(
                    "a-proto-restructure",
                    "结构调整",
                    "信息架构不完整，梳理用户流程与页面关系，重构线框图。\
                     输出 JSON：{\"issues\":[], \"new_flow\":\"\", \"updated_screens\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-proto-restructure",
                    vec![("wireframe", "a-proto-wireframe")],
                    vec!["a-proto-wireframe"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-proto", "汇合", 0.0, 720.0),
                // Agent：交互原型
                make_agent_node_full(
                    "a-proto-interact",
                    "交互原型",
                    "制作可点击交互原型：页面流转、状态切换、异常路径。\
                     输出 JSON：{\"flows\":[{\"name\":\"\", \"steps\":[], \"transitions\":[]}], \"edge_cases\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-proto-interact",
                    vec![("mockup", "a-proto-mockup"), ("restructure", "a-proto-restructure")],
                    vec!["a-proto-mockup", "a-proto-restructure"],
                    0.0,
                    900.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-proto",
                    "原型审批",
                    "交互原型已完成，请产品负责人审批",
                    Some("product_owner"),
                    86400,
                    "ap-proto",
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-wireframe", "trigger", "a-proto-wireframe"),
                edge("e-wireframe-ia", "a-proto-wireframe", "c-proto-ia"),
                edge_cond("e-ia-mockup", "c-proto-ia", "true", "a-proto-mockup", EdgeType::ConditionTrue),
                edge_cond(
                    "e-ia-restructure",
                    "c-proto-ia",
                    "false",
                    "a-proto-restructure",
                    EdgeType::ConditionFalse,
                ),
                edge("e-mockup-merge", "a-proto-mockup", "m-proto"),
                edge("e-restructure-merge", "a-proto-restructure", "m-proto"),
                edge("e-merge-interact", "m-proto", "a-proto-interact"),
                edge("e-interact-approval", "a-proto-interact", "ap-proto"),
                edge("e-approval-end", "ap-proto", "end"),
            ],
            vec![DomainInputField { key: "feature_brief", label: "功能简报", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-des-ux-research: 用户研究 ────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-des-ux-research",
            "用户研究",
            "用户研究：设计研究方案，检索背景资料，逐用户分析访谈结果，输出洞察报告",
            "👥",
            vec!["opc".to_string(), "design".to_string()],
            DES_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：研究计划
                make_agent_node(
                    "a-ux-plan",
                    "研究计划",
                    "确定研究目标、问题、用户招募标准与研究方法（访谈/可用性测试）。\
                     输出 JSON：{\"goals\":[], \"questions\":[], \"recruiting\":{\"criteria\":[], \"count\":0}, \"method\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-ux-plan",
                    0.0,
                    180.0,
                ),
                // 工具：检索背景资料
                make_tool_node(
                    "t-ux-search",
                    "背景资料检索",
                    "OpcSearchWiki",
                    vec![("user_input", "a-ux-plan")],
                    "t-ux-search",
                    0.0,
                    360.0,
                ),
                // Loop：逐用户分析
                make_loop_node(
                    "l-ux-users",
                    "逐用户分析",
                    LoopType::ForEach,
                    Some("t-ux-search"),
                    Some("user_item"),
                    Some("l-ux-users"),
                    Some("l-ux-users__partial"),
                    Some(30),
                    vec!["a-ux-user".to_string()],
                    0.0,
                    540.0,
                ),
                // Loop body：单用户洞察
                make_agent_node_full(
                    "a-ux-user",
                    "用户洞察",
                    "分析当前用户访谈/测试记录：行为、痛点、动机、需求与引用。\
                     输出 JSON：{\"user\":\"\", \"behaviors\":[], \"pain_points\":[], \"motivations\":[], \"quotes\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-ux-user",
                    vec![("user", "user_item")],
                    vec!["a-ux-plan", "t-ux-search"],
                    250.0,
                    540.0,
                ),
                // Agent：研究报告
                make_agent_node_full(
                    "a-ux-report",
                    "研究报告",
                    "汇总全部用户洞察：主题聚类、用户画像、需求优先级与设计建议。\
                     输出 JSON：{\"themes\":[{\"topic\":\"\", \"evidence\":[], \"frequency\":0}], \"personas\":[], \"recommendations\":[{\"suggestion\":\"\", \"priority\":\"P0|P1|P2\", \"rationale\":\"\"}]}",
                    vec![],
                    Some(PROFILE),
                    "a-ux-report",
                    vec![("insights", "l-ux-users.items")],
                    vec!["l-ux-users"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-ux-plan"),
                edge("e-plan-search", "a-ux-plan", "t-ux-search"),
                edge("e-search-loop", "t-ux-search", "l-ux-users"),
                edge("e-loop-report", "l-ux-users", "a-ux-report"),
                edge("e-report-end", "a-ux-report", "end"),
            ],
            vec![DomainInputField { key: "research_goal", label: "研究目标", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
