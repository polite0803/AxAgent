// SPDX-License-Identifier: AGPL-3.0-only

//! 游戏开发（gamedev）领域工作流种子化 — 3 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-gd-concept:   游戏概念设计（概念 → 设计 → 完整性判定 → 文档/补充分支 → 审批）
//! - wf-gd-prototype: 游戏原型（核心机制 → 玩法测试 → 可玩性判定 → 迭代优化循环）
//! - wf-gd-qa:        游戏测试（功能/平衡/体验测试 → 严重缺陷判定 → 修复排期）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cto-cto-ai-engineer";
/// gamedev 领域模板版本（v4 丰富拓扑）
const GD_TEMPLATE_VERSION: i32 = 4;

/// 种子化游戏开发领域的全部工作流
pub(crate) async fn seed_domain_gamedev_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-gd-concept: 游戏概念设计 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gd-concept",
            "游戏概念设计",
            "游戏概念设计：生成核心玩法概念，设计机制/关卡/角色，设计不完整自动补充，文档经审批",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            GD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：概念生成
                make_agent_node(
                    "a-gd-idea",
                    "概念生成",
                    "生成游戏核心玩法与概念：类型、题材、核心循环、目标受众。\
                     输出 JSON：{\"genre\":\"\", \"theme\":\"\", \"core_loop\":\"\", \"audience\":\"\", \"unique_mechanic\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-idea",
                    0.0,
                    180.0,
                ),
                // Agent：游戏设计
                make_agent_node_full(
                    "a-gd-design",
                    "游戏设计",
                    "设计游戏机制、关卡结构、角色与成长系统，评估设计完整性。\
                     输出 JSON：{\"mechanics\":[], \"levels\":[{\"name\":\"\", \"objective\":\"\", \"features\":[]}], \"characters\":[], \"complete\":true, \"gaps\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-design",
                    vec![("idea", "a-gd-idea")],
                    vec!["a-gd-idea"],
                    0.0,
                    360.0,
                ),
                // 条件：设计完整性
                make_condition_node(
                    "c-gd-complete",
                    "完整性判定",
                    vec![Condition {
                        var_path: "a-gd-design.complete".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不完整分支：设计补充
                make_agent_node_full(
                    "a-gd-fill",
                    "设计补充",
                    "设计存在缺口，补齐缺失的机制/关卡/角色定义。\
                     输出 JSON：{\"filled_gaps\":[{\"area\":\"\", \"design\":\"\"}], \"complete\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-fill",
                    vec![("design", "a-gd-design")],
                    vec!["a-gd-design"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-gd", "汇合", 0.0, 900.0),
                // Agent：设计文档
                make_agent_node_full(
                    "a-gd-doc",
                    "文档",
                    "编写完整游戏设计文档（GDD）：愿景、核心循环、系统、内容路线图。\
                     输出 JSON：{\"vision\":\"\", \"core_systems\":[], \"content_roadmap\":[], \"appendices\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-doc",
                    vec![("design", "a-gd-design"), ("fill", "a-gd-fill")],
                    vec!["a-gd-design", "a-gd-fill"],
                    0.0,
                    1080.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-gd-concept",
                    "设计审批",
                    "游戏设计文档已完成，请主创审批",
                    Some("creative_director"),
                    86400,
                    "ap-gd-concept",
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-idea", "trigger", "a-gd-idea"),
                edge("e-idea-design", "a-gd-idea", "a-gd-design"),
                edge("e-design-complete", "a-gd-design", "c-gd-complete"),
                edge_cond("e-incomplete-fill", "c-gd-complete", "false", "a-gd-fill", EdgeType::ConditionFalse),
                edge_cond("e-complete-merge", "c-gd-complete", "true", "m-gd", EdgeType::ConditionTrue),
                edge("e-fill-merge", "a-gd-fill", "m-gd"),
                edge("e-merge-doc", "m-gd", "a-gd-doc"),
                edge("e-doc-approval", "a-gd-doc", "ap-gd-concept"),
                edge("e-approval-end", "ap-gd-concept", "end"),
            ],
            vec![DomainInputField { key: "game_idea", label: "游戏创意", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-gd-prototype: 游戏原型 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gd-prototype",
            "游戏原型",
            "游戏原型：实现核心玩法，测试可玩性，不达标自动迭代优化直至可玩",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            GD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：核心机制
                make_agent_node(
                    "a-gd-proto-core",
                    "核心机制",
                    "实现核心玩法与控制：操作手感、规则闭环、最小可玩内容。\
                     输出 JSON：{\"controls\":[], \"mechanics_impl\":\"\", \"playable\":true, \"known_issues\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-proto-core",
                    0.0,
                    180.0,
                ),
                // Agent：玩法测试
                make_agent_node_full(
                    "a-gd-proto-test",
                    "玩法测试",
                    "测试核心机制可玩性：操作反馈、乐趣点、挫败点、留存意愿。\
                     输出 JSON：{\"fun_score\":0, \"friction_points\":[], \"verdict\":\"pass|needs_iteration\", \"test_notes\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-proto-test",
                    vec![("core", "a-gd-proto-core")],
                    vec!["a-gd-proto-core"],
                    0.0,
                    360.0,
                ),
                // 条件：可玩性判定
                make_condition_node(
                    "c-gd-playable",
                    "可玩性判定",
                    vec![Condition {
                        var_path: "a-gd-proto-test.verdict".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!("pass"),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不达标分支：迭代优化
                make_agent_node_full(
                    "a-gd-proto-iterate",
                    "迭代优化",
                    "根据测试反馈优化核心机制：调整参数、修复挫败点、增强反馈。\
                     输出 JSON：{\"changes\":[{\"issue\":\"\", \"fix\":\"\", \"expected_effect\":\"\"}], \"iteration_notes\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-proto-iterate",
                    vec![("test", "a-gd-proto-test")],
                    vec!["a-gd-proto-test"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-gd-proto", "汇合", 0.0, 900.0),
                // Agent：原型结论
                make_agent_node_full(
                    "a-gd-proto-verdict",
                    "原型结论",
                    "汇总测试与迭代结果，输出原型验收结论与后续开发建议。\
                     输出 JSON：{\"accepted\":true, \"summary\":\"\", \"next_steps\":[], \"risks\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-proto-verdict",
                    vec![("test", "a-gd-proto-test"), ("iterate", "a-gd-proto-iterate")],
                    vec!["a-gd-proto-test", "a-gd-proto-iterate"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-core", "trigger", "a-gd-proto-core"),
                edge("e-core-test", "a-gd-proto-core", "a-gd-proto-test"),
                edge("e-test-playable", "a-gd-proto-test", "c-gd-playable"),
                edge_cond(
                    "e-fail-iterate",
                    "c-gd-playable",
                    "false",
                    "a-gd-proto-iterate",
                    EdgeType::ConditionFalse,
                ),
                edge_cond("e-pass-merge", "c-gd-playable", "true", "m-gd-proto", EdgeType::ConditionTrue),
                edge("e-iterate-merge", "a-gd-proto-iterate", "m-gd-proto"),
                edge("e-merge-verdict", "m-gd-proto", "a-gd-proto-verdict"),
                edge("e-verdict-end", "a-gd-proto-verdict", "end"),
            ],
            vec![DomainInputField { key: "prototype_scope", label: "原型范围", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-gd-qa: 游戏测试 ──────────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gd-qa",
            "游戏测试",
            "游戏测试：功能/平衡/体验三轮测试，发现严重缺陷自动生成修复排期",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            GD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：功能测试
                make_agent_node(
                    "a-gd-qa-functional",
                    "功能测试",
                    "执行功能测试：核心流程、边界条件、异常输入、兼容性。\
                     输出 JSON：{\"defects\":[{\"id\":\"\", \"severity\":\"critical|major|minor\", \"area\":\"\", \"repro\":\"\", \"expected\":\"\"}], \"critical_count\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-qa-functional",
                    0.0,
                    180.0,
                ),
                // Agent：平衡测试
                make_agent_node_full(
                    "a-gd-qa-balance",
                    "平衡测试",
                    "测试数值平衡：成长曲线、资源产出/消耗、职业/角色强弱。\
                     输出 JSON：{\"balance_issues\":[{\"system\":\"\", \"issue\":\"\", \"data\":\"\", \"suggestion\":\"\"}], \"overall_balance\":\"good|needs_tuning\"}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-qa-balance",
                    vec![("functional", "a-gd-qa-functional")],
                    vec!["a-gd-qa-functional"],
                    0.0,
                    360.0,
                ),
                // Agent：体验测试
                make_agent_node_full(
                    "a-gd-qa-ux",
                    "体验测试",
                    "评估游戏体验：新手引导、信息呈现、操作流畅度、情感反馈。\
                     输出 JSON：{\"ux_issues\":[{\"area\":\"\", \"problem\":\"\", \"impact\":\"\", \"suggestion\":\"\"}], \"experience_score\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-qa-ux",
                    vec![("balance", "a-gd-qa-balance")],
                    vec!["a-gd-qa-balance"],
                    0.0,
                    540.0,
                ),
                // 条件：存在严重缺陷
                make_condition_node(
                    "c-gd-qa-critical",
                    "严重度判定",
                    vec![Condition {
                        var_path: "a-gd-qa-functional.critical_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    720.0,
                ),
                // 严重缺陷分支：修复排期
                make_agent_node_full(
                    "a-gd-qa-fix",
                    "修复排期",
                    "为严重缺陷制定修复排期：优先级、责任人、验证标准。\
                     输出 JSON：{\"fix_plan\":[{\"defect\":\"\", \"priority\":\"P0|P1\", \"owner\":\"\", \"due\":\"\", \"verify\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "通知开发团队严重缺陷")],
                    Some(PROFILE),
                    "a-gd-qa-fix",
                    vec![("functional", "a-gd-qa-functional")],
                    vec!["a-gd-qa-functional"],
                    -250.0,
                    900.0,
                ),
                make_merge_node("m-gd-qa", "汇合", 0.0, 1080.0),
                // Agent：测试报告
                make_agent_node_full(
                    "a-gd-qa-report",
                    "测试报告",
                    "汇总功能/平衡/体验测试结果与修复计划，输出发布建议。\
                     输出 JSON：{\"release_verdict\":\"go|no_go|conditional\", \"summary\":\"\", \"defect_summary\":{\"critical\":0, \"major\":0, \"minor\":0}, \"recommendations\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gd-qa-report",
                    vec![("functional", "a-gd-qa-functional"), ("balance", "a-gd-qa-balance"), ("ux", "a-gd-qa-ux"), ("fix", "a-gd-qa-fix")],
                    vec!["a-gd-qa-functional", "a-gd-qa-balance", "a-gd-qa-ux", "a-gd-qa-fix"],
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-functional", "trigger", "a-gd-qa-functional"),
                edge("e-functional-balance", "a-gd-qa-functional", "a-gd-qa-balance"),
                edge("e-balance-ux", "a-gd-qa-balance", "a-gd-qa-ux"),
                edge("e-ux-critical", "a-gd-qa-ux", "c-gd-qa-critical"),
                edge_cond("e-critical-fix", "c-gd-qa-critical", "true", "a-gd-qa-fix", EdgeType::ConditionTrue),
                edge_cond("e-clean-merge", "c-gd-qa-critical", "false", "m-gd-qa", EdgeType::ConditionFalse),
                edge("e-fix-merge", "a-gd-qa-fix", "m-gd-qa"),
                edge("e-merge-report", "m-gd-qa", "a-gd-qa-report"),
                edge("e-report-end", "a-gd-qa-report", "end"),
            ],
            vec![DomainInputField { key: "build_version", label: "构建版本", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
