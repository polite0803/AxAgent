// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理（pm）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-pm-risk: 风险管理
//! - wf-pm-sprint: Sprint规划
//! - wf-pm-status: 项目状态报告

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化项目管理领域的全部工作流
pub(crate) async fn seed_domain_pm_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 风险管理
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-pm-risk",
            "风险管理",
            "识别、评估和应对项目风险",
            "⚠️",
            vec!["opc".to_string(), "pm".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-risk-identify",
                    "风险识别",
                    "识别技术和业务风险",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-risk-identify_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-risk-assess",
                    "评估",
                    "评估影响和概率",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-risk-assess_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-risk-respond",
                    "应对",
                    "制定风险应对策略",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-risk-respond_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-risk-identify", "trigger", "a-risk-identify"),
                edge("e-a-risk-identify-a-risk-assess", "a-risk-identify", "a-risk-assess"),
                edge("e-a-risk-assess-a-risk-respond", "a-risk-assess", "a-risk-respond"),
                edge("e-a-risk-respond-end", "a-risk-respond", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: Sprint规划
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-pm-sprint",
            "Sprint规划",
            "迭代冲刺规划和任务分配",
            "📋",
            vec!["opc".to_string(), "pm".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-sprint-backlog",
                    "Backlog梳理",
                    "梳理和估算待办项",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sprint-backlog_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-sprint-plan",
                    "冲刺规划",
                    "确定冲刺目标和任务分配",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sprint-plan_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-sprint-review",
                    "冲刺回顾",
                    "回顾冲刺成果和改进点",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sprint-review_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-sprint-backlog", "trigger", "a-sprint-backlog"),
                edge("e-a-sprint-backlog-a-sprint-plan", "a-sprint-backlog", "a-sprint-plan"),
                edge("e-a-sprint-plan-a-sprint-review", "a-sprint-plan", "a-sprint-review"),
                edge("e-a-sprint-review-end", "a-sprint-review", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 项目状态报告
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-pm-status",
            "项目状态报告",
            "生成项目周报和状态更新",
            "📊",
            vec!["opc".to_string(), "pm".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-status-collect",
                    "数据收集",
                    "收集团队进展和指标",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-status-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-status-write",
                    "报告撰写",
                    "撰写项目状态报告",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-status-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-status-distribute",
                    "分发",
                    "发送报告并安排跟进",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-status-distribute_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-status-collect", "trigger", "a-status-collect"),
                edge("e-a-status-collect-a-status-write", "a-status-collect", "a-status-write"),
                edge(
                    "e-a-status-write-a-status-distribute",
                    "a-status-write",
                    "a-status-distribute",
                ),
                edge("e-a-status-distribute-end", "a-status-distribute", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
