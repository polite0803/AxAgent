// SPDX-License-Identifier: AGPL-3.0-only

//! 软件测试（testing）领域工作流种子化 — 3 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-tst-automation: 自动化测试（用例选择 → 脚本编写 → 通过判定 → 运行/修复分支）
//! - wf-tst-perf:       性能测试（脚本设计 → 执行 → 达标判定 → 报告/瓶颈分析分支）
//! - wf-tst-plan:       测试计划（需求分析 → 计划设计 → 评审 → 修订分支）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cto-cto-ai-engineer";
/// testing 领域模板版本（v4 丰富拓扑）
const TST_TEMPLATE_VERSION: i32 = 4;

/// 种子化软件测试领域的全部工作流
pub(crate) async fn seed_domain_testing_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-tst-automation: 自动化测试 ───────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-tst-automation",
            "自动化测试",
            "自动化测试：选择高价值用例，编写脚本，脚本失败自动修复后运行",
            "🤖",
            vec!["opc".to_string(), "testing".to_string()],
            TST_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：用例选择
                make_agent_node(
                    "a-tauto-pick",
                    "用例选择",
                    "选择自动化用例：回归价值、执行频率、稳定性、维护成本。\
                     输出 JSON：{\"cases\":[{\"id\":\"\", \"priority\":\"P0|P1|P2\", \"value\":0, \"flaky\":false}], }",
                    vec![],
                    Some(PROFILE),
                    "a-tauto-pick",
                    0.0,
                    180.0,
                ),
                // Agent：脚本编写
                make_agent_node_full(
                    "a-tauto-write",
                    "脚本编写",
                    "编写自动化测试脚本：定位器、断言、数据准备、等待策略。\
                     输出 JSON：{\"scripts\":[{\"case\":\"\", \"script\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-tauto-write",
                    vec![("cases", "a-tauto-pick")],
                    vec!["a-tauto-pick"],
                    0.0,
                    360.0,
                ),
                // 条件：脚本是否通过
                make_condition_node(
                    "c-tauto-pass",
                    "通过判定",
                    vec![Condition {
                        var_path: "a-tauto-write.scripts".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 通过：运行
                make_agent_node_full(
                    "a-tauto-run",
                    "运行",
                    "运行自动化测试套件：执行、收集结果、失败分析、报告。\
                     输出 JSON：{\"passed\":0, \"failed\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-tauto-run",
                    vec![("scripts", "a-tauto-write")],
                    vec!["a-tauto-write"],
                    -250.0,
                    720.0,
                ),
                // 失败：脚本修复
                make_agent_node_full(
                    "a-tauto-fix",
                    "脚本修复",
                    "脚本编写失败，修复：定位器失效、断言错误、环境依赖。\
                     输出 JSON：{\"fixes\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-tauto-fix",
                    vec![("scripts", "a-tauto-write")],
                    vec!["a-tauto-write"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-tauto", "汇合", 0.0, 900.0),
                // Agent：测试结论
                make_agent_node_full(
                    "a-tauto-summary",
                    "结论",
                    "汇总自动化测试结果：通过率、遗留缺陷、维护建议。\
                     输出 JSON：{\"pass_rate\":0, \"flaky_cases\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-tauto-summary",
                    vec![("run", "a-tauto-run"), ("fix", "a-tauto-fix")],
                    vec!["a-tauto-run", "a-tauto-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-pick", "trigger", "a-tauto-pick"),
                edge("e-pick-write", "a-tauto-pick", "a-tauto-write"),
                edge("e-write-pass", "a-tauto-write", "c-tauto-pass"),
                edge_cond("e-ok-run", "c-tauto-pass", "true", "a-tauto-run", EdgeType::ConditionTrue),
                edge_cond("e-fail-fix", "c-tauto-pass", "false", "a-tauto-fix", EdgeType::ConditionFalse),
                edge("e-run-merge", "a-tauto-run", "m-tauto"),
                edge("e-fix-merge", "a-tauto-fix", "m-tauto"),
                edge("e-merge-summary", "m-tauto", "a-tauto-summary"),
                edge("e-summary-end", "a-tauto-summary", "end"),
            ],
            vec![DomainInputField { key: "module_name", label: "测试模块", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-tst-perf: 性能测试 ───────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-tst-perf",
            "性能测试",
            "性能测试：设计压测脚本，执行测试，不达标自动定位瓶颈",
            "⚡",
            vec!["opc".to_string(), "testing".to_string()],
            TST_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：脚本设计
                make_agent_node(
                    "a-tperf-script",
                    "脚本设计",
                    "设计性能测试脚本：场景、并发模型、负载曲线、监控指标。\
                     输出 JSON：{\"scenarios\":[], \"concurrency\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-tperf-script",
                    0.0,
                    180.0,
                ),
                // Agent：执行
                make_agent_node_full(
                    "a-tperf-run",
                    "执行",
                    "执行性能测试：压测、资源监控、结果采集。\
                     输出 JSON：{\"rt_p95\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-tperf-run",
                    vec![("script", "a-tperf-script")],
                    vec!["a-tperf-script"],
                    0.0,
                    360.0,
                ),
                // 条件：性能达标
                make_condition_node(
                    "c-tperf-pass",
                    "达标判定",
                    vec![Condition {
                        var_path: "a-tperf-run.rt_p95".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 达标：报告
                make_agent_node_full(
                    "a-tperf-report",
                    "报告",
                    "输出性能测试报告：指标汇总、达标情况、容量建议。\
                     输出 JSON：{\"metrics\":{}, \"verdict\":\"pass\", }",
                    vec![],
                    Some(PROFILE),
                    "a-tperf-report",
                    vec![("run", "a-tperf-run")],
                    vec!["a-tperf-run"],
                    -250.0,
                    720.0,
                ),
                // 不达标：瓶颈分析
                make_agent_node_full(
                    "a-tperf-bottleneck",
                    "瓶颈分析",
                    "性能不达标，定位瓶颈：CPU/内存/IO/锁/慢查询。\
                     输出 JSON：{\"bottlenecks\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-tperf-bottleneck",
                    vec![("run", "a-tperf-run")],
                    vec!["a-tperf-run"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-tperf", "汇合", 0.0, 900.0),
                // Agent：结论
                make_agent_node_full(
                    "a-tperf-conclusion",
                    "结论",
                    "汇总性能测试结论与优化建议。\
                     输出 JSON：{\"verdict\":\"\", \"optimizations\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-tperf-conclusion",
                    vec![("report", "a-tperf-report"), ("bottleneck", "a-tperf-bottleneck")],
                    vec!["a-tperf-report", "a-tperf-bottleneck"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-script", "trigger", "a-tperf-script"),
                edge("e-script-run", "a-tperf-script", "a-tperf-run"),
                edge("e-run-pass", "a-tperf-run", "c-tperf-pass"),
                edge_cond(
                    "e-ok-report",
                    "c-tperf-pass",
                    "true",
                    "a-tperf-report",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-fail-bottleneck",
                    "c-tperf-pass",
                    "false",
                    "a-tperf-bottleneck",
                    EdgeType::ConditionFalse,
                ),
                edge("e-report-merge", "a-tperf-report", "m-tperf"),
                edge("e-bottleneck-merge", "a-tperf-bottleneck", "m-tperf"),
                edge("e-merge-conclusion", "m-tperf", "a-tperf-conclusion"),
                edge("e-conclusion-end", "a-tperf-conclusion", "end"),
            ],
            vec![DomainInputField {
                key: "endpoint",
                label: "压测目标",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-tst-plan: 测试计划 ───────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-tst-plan",
            "测试计划",
            "测试计划：分析需求确定测试范围，设计测试策略，评审未通过自动修订",
            "📝",
            vec!["opc".to_string(), "testing".to_string()],
            TST_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：需求分析
                make_agent_node(
                    "a-tplan-analyze",
                    "需求分析",
                    "分析需求确定测试范围：功能点、优先级、风险项、测试环境。\
                     输出 JSON：{\"scope\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-tplan-analyze",
                    0.0,
                    180.0,
                ),
                // Agent：计划设计
                make_agent_node_full(
                    "a-tplan-design",
                    "计划设计",
                    "设计测试计划：测试类型、用例规模、资源排期、准入准出标准。\
                     输出 JSON：{\"strategy\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-tplan-design",
                    vec![("analyze", "a-tplan-analyze")],
                    vec!["a-tplan-analyze"],
                    0.0,
                    360.0,
                ),
                // Agent：评审
                make_agent_node_full(
                    "a-tplan-review",
                    "评审",
                    "评审测试计划：覆盖率、资源可行性、风险覆盖、标准明确性。\
                     输出 JSON：{\"passed\":false, }",
                    vec![],
                    Some(PROFILE),
                    "a-tplan-review",
                    vec![("design", "a-tplan-design")],
                    vec!["a-tplan-design"],
                    0.0,
                    540.0,
                ),
                // 条件：评审是否通过
                make_condition_node(
                    "c-tplan-passed",
                    "评审判定",
                    vec![Condition {
                        var_path: "a-tplan-review.passed".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    720.0,
                ),
                // 未通过：计划修订
                make_agent_node_full(
                    "a-tplan-revise",
                    "计划修订",
                    "按评审意见修订测试计划。\
                     输出 JSON：{\"revisions\":[], \"passed\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-tplan-revise",
                    vec![("review", "a-tplan-review"), ("design", "a-tplan-design")],
                    vec!["a-tplan-review", "a-tplan-design"],
                    -250.0,
                    900.0,
                ),
                make_merge_node("m-tplan", "汇合", 0.0, 1080.0),
                // Agent：定稿
                make_agent_node_full(
                    "a-tplan-final",
                    "定稿",
                    "测试计划定稿：版本、审批记录、发布说明。\
                     输出 JSON：{\"final_version\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-tplan-final",
                    vec![("review", "a-tplan-review"), ("revise", "a-tplan-revise")],
                    vec!["a-tplan-review", "a-tplan-revise"],
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-analyze", "trigger", "a-tplan-analyze"),
                edge("e-analyze-design", "a-tplan-analyze", "a-tplan-design"),
                edge("e-design-review", "a-tplan-design", "a-tplan-review"),
                edge("e-review-passed", "a-tplan-review", "c-tplan-passed"),
                edge_cond(
                    "e-fail-revise",
                    "c-tplan-passed",
                    "false",
                    "a-tplan-revise",
                    EdgeType::ConditionFalse,
                ),
                edge_cond(
                    "e-pass-merge",
                    "c-tplan-passed",
                    "true",
                    "m-tplan",
                    EdgeType::ConditionTrue,
                ),
                edge("e-revise-merge", "a-tplan-revise", "m-tplan"),
                edge("e-merge-final", "m-tplan", "a-tplan-final"),
                edge("e-final-end", "a-tplan-final", "end"),
            ],
            vec![DomainInputField {
                key: "release_scope",
                label: "版本范围",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
