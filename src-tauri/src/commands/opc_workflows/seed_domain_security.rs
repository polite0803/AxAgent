// SPDX-License-Identifier: AGPL-3.0-only

//! 安全与合规（security）领域工作流种子化 — 4 个工作流
//!
//! 生成的工作流：
//! - wf-sec-compliance: 合规审计
//! - wf-sec-incident: 安全事件响应
//! - wf-sec-pentest: 渗透测试
//! - wf-sec-threat-intel: 威胁情报

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化安全与合规领域的全部工作流
pub(crate) async fn seed_domain_security_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 合规审计
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sec-compliance",
            "合规审计",
            "检查安全合规标准和差距",
            "✅",
            vec!["opc".to_string(), "security".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-comp-standard",
                    "标准对照",
                    "确定适用的安全标准和框架",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-comp-standard_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-comp-audit",
                    "审计",
                    "逐项检查合规性",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-comp-audit_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-comp-report",
                    "报告",
                    "输出合规报告和整改计划",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-comp-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-comp-standard", "trigger", "a-comp-standard"),
                edge("e-a-comp-standard-a-comp-audit", "a-comp-standard", "a-comp-audit"),
                edge("e-a-comp-audit-a-comp-report", "a-comp-audit", "a-comp-report"),
                edge("e-a-comp-report-end", "a-comp-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 安全事件响应
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sec-incident",
            "安全事件响应",
            "检测、分析和响应安全事件",
            "🚨",
            vec!["opc".to_string(), "security".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-incident-detect",
                    "检测",
                    "确认安全事件类型和范围",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-incident-detect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-incident-respond",
                    "响应",
                    "执行应急响应和止损措施",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-incident-respond_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-incident-review",
                    "复盘",
                    "事故复盘和改进计划",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-incident-review_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-incident-detect", "trigger", "a-incident-detect"),
                edge(
                    "e-a-incident-detect-a-incident-respond",
                    "a-incident-detect",
                    "a-incident-respond",
                ),
                edge(
                    "e-a-incident-respond-a-incident-review",
                    "a-incident-respond",
                    "a-incident-review",
                ),
                edge("e-a-incident-review-end", "a-incident-review", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 渗透测试
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sec-pentest",
            "渗透测试",
            "对应用和基础设施进行渗透测试",
            "🔓",
            vec!["opc".to_string(), "security".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-pentest-scope",
                    "范围确定",
                    "确定测试范围和目标",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-pentest-scope_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-pentest-exec",
                    "执行",
                    "执行渗透测试并记录发现",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-pentest-exec_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-pentest-report",
                    "报告",
                    "输出漏洞报告和修复建议",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-pentest-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-pentest-scope", "trigger", "a-pentest-scope"),
                edge("e-a-pentest-scope-a-pentest-exec", "a-pentest-scope", "a-pentest-exec"),
                edge("e-a-pentest-exec-a-pentest-report", "a-pentest-exec", "a-pentest-report"),
                edge("e-a-pentest-report-end", "a-pentest-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 4: 威胁情报
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sec-threat-intel",
            "威胁情报",
            "收集和分析最新安全威胁情报",
            "🕵️",
            vec!["opc".to_string(), "security".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-threat-collect",
                    "情报收集",
                    "收集行业威胁情报和安全公告",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-threat-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-threat-analyze",
                    "分析",
                    "评估威胁影响和风险级别",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-threat-analyze_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-threat-act",
                    "行动",
                    "制定防护措施和更新策略",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-threat-act_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-threat-collect", "trigger", "a-threat-collect"),
                edge("e-a-threat-collect-a-threat-analyze", "a-threat-collect", "a-threat-analyze"),
                edge("e-a-threat-analyze-a-threat-act", "a-threat-analyze", "a-threat-act"),
                edge("e-a-threat-act-end", "a-threat-act", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
