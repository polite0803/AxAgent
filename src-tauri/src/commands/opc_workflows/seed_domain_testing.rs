// SPDX-License-Identifier: AGPL-3.0-only

//! 测试与质量（testing）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-tst-automation: 自动化测试
//! - wf-tst-perf: 性能测试
//! - wf-tst-plan: 测试计划

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化测试与质量领域的全部工作流
pub(crate) async fn seed_domain_testing_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 自动化测试
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-tst-automation",
            "自动化测试",
            "编写和维护自动化测试脚本",
            "🤖",
            vec!["opc".to_string(), "testing".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-tauto-pick",
                    "选型",
                    "选择自动化框架和工具",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tauto-pick_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-tauto-write",
                    "编写",
                    "编写测试脚本并集成本地CI",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tauto-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-tauto-run",
                    "运行",
                    "运行测试并分析结果",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tauto-run_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-tauto-pick", "trigger", "a-tauto-pick"),
                edge("e-a-tauto-pick-a-tauto-write", "a-tauto-pick", "a-tauto-write"),
                edge("e-a-tauto-write-a-tauto-run", "a-tauto-write", "a-tauto-run"),
                edge("e-a-tauto-run-end", "a-tauto-run", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 性能测试
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-tst-perf",
            "性能测试",
            "负载测试和性能基准",
            "⚡",
            vec!["opc".to_string(), "testing".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-tperf-script",
                    "测试脚本",
                    "编写性能测试脚本和场景",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tperf-script_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-tperf-run",
                    "执行",
                    "执行负载测试并监控",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tperf-run_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-tperf-report",
                    "报告",
                    "输出性能报告和优化建议",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tperf-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-tperf-script", "trigger", "a-tperf-script"),
                edge("e-a-tperf-script-a-tperf-run", "a-tperf-script", "a-tperf-run"),
                edge("e-a-tperf-run-a-tperf-report", "a-tperf-run", "a-tperf-report"),
                edge("e-a-tperf-report-end", "a-tperf-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 测试计划
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-tst-plan",
            "测试计划",
            "制定完整测试策略和计划",
            "📋",
            vec!["opc".to_string(), "testing".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-tplan-analyze",
                    "需求分析",
                    "分析功能需求和技术规格",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tplan-analyze_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-tplan-design",
                    "测试设计",
                    "设计测试用例和测试场景",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tplan-design_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-tplan-review",
                    "评审",
                    "评审测试覆盖率和优先级",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tplan-review_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-tplan-analyze", "trigger", "a-tplan-analyze"),
                edge("e-a-tplan-analyze-a-tplan-design", "a-tplan-analyze", "a-tplan-design"),
                edge("e-a-tplan-design-a-tplan-review", "a-tplan-design", "a-tplan-review"),
                edge("e-a-tplan-review-end", "a-tplan-review", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
