// SPDX-License-Identifier: AGPL-3.0-only

//! 设计与创意（design）领域工作流种子化 — 4 个工作流
//!
//! 生成的工作流：
//! - wf-des-accessibility: 无障碍审计
//! - wf-des-design-system: 设计系统
//! - wf-des-prototype: 原型设计
//! - wf-des-ux-research: 用户研究

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化设计与创意领域的全部工作流
pub(crate) async fn seed_domain_design_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 无障碍审计
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-des-accessibility",
            "无障碍审计",
            "审计和修复产品无障碍问题",
            "♿",
            vec!["opc".to_string(), "design".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-a11y-scan",
                    "扫描",
                    "使用工具扫描无障碍问题",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-a11y-scan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-a11y-report",
                    "报告",
                    "分类报告问题严重程度",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-a11y-report_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-a11y-fix",
                    "修复",
                    "优先级修复关键无障碍问题",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-a11y-fix_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-a11y-scan", "trigger", "a-a11y-scan"),
                edge("e-a-a11y-scan-a-a11y-report", "a-a11y-scan", "a-a11y-report"),
                edge("e-a-a11y-report-a-a11y-fix", "a-a11y-report", "a-a11y-fix"),
                edge("e-a-a11y-fix-end", "a-a11y-fix", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 设计系统
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-des-design-system",
            "设计系统",
            "搭建和维护统一的设计系统",
            "📐",
            vec!["opc".to_string(), "design".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ds-audit",
                    "审计",
                    "审计现有设计元件和模式",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ds-audit_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ds-components",
                    "组件库",
                    "构建核心组件库和规范文档",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ds-components_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ds-doc",
                    "文档",
                    "输出设计系统使用文档",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ds-doc_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ds-audit", "trigger", "a-ds-audit"),
                edge("e-a-ds-audit-a-ds-components", "a-ds-audit", "a-ds-components"),
                edge("e-a-ds-components-a-ds-doc", "a-ds-components", "a-ds-doc"),
                edge("e-a-ds-doc-end", "a-ds-doc", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 原型设计
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-des-prototype",
            "原型设计",
            "从线框图到交互原型",
            "🎨",
            vec!["opc".to_string(), "design".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-proto-wireframe",
                    "线框图",
                    "绘制页面结构和布局线框图",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-proto-wireframe_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-proto-mockup",
                    "高保真",
                    "设计高保真模型和设计稿",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-proto-mockup_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-proto-interact",
                    "交互原型",
                    "制作可点击交互原型",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-proto-interact_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-proto-wireframe", "trigger", "a-proto-wireframe"),
                edge("e-a-proto-wireframe-a-proto-mockup", "a-proto-wireframe", "a-proto-mockup"),
                edge("e-a-proto-mockup-a-proto-interact", "a-proto-mockup", "a-proto-interact"),
                edge("e-a-proto-interact-end", "a-proto-interact", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 4: 用户研究
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-des-ux-research",
            "用户研究",
            "用户访谈、可用性测试和洞察",
            "👥",
            vec!["opc".to_string(), "design".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ux-plan",
                    "研究计划",
                    "确定研究目标和用户招募标准",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ux-plan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ux-conduct",
                    "执行",
                    "执行用户访谈或可用性测试",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ux-conduct_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ux-report",
                    "研究报告",
                    "输出研究洞察和设计建议",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-ux-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ux-plan", "trigger", "a-ux-plan"),
                edge("e-a-ux-plan-a-ux-conduct", "a-ux-plan", "a-ux-conduct"),
                edge("e-a-ux-conduct-a-ux-report", "a-ux-conduct", "a-ux-report"),
                edge("e-a-ux-report-end", "a-ux-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
