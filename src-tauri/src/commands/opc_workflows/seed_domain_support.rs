// SPDX-License-Identifier: AGPL-3.0-only

//! 客户支持（support）领域工作流种子化 — 3 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-sup-faq:          FAQ知识库（问题收集 → 撰写 → 覆盖完整性分支 → 发布）
//! - wf-sup-satisfaction: 满意度管理（调查设计 → 发送 → 逐反馈分析循环 → 改进报告）
//! - wf-sup-ticket:       工单处理（分类 → 严重度分支 → 紧急/常规处理 → 跟进闭环）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{
    CompareOperator, Condition, EdgeType, LogicalOperator, LoopType,
};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-coo-coo-operations-manager";
/// support 领域模板版本（v4 丰富拓扑）
const SUP_TEMPLATE_VERSION: i32 = 4;

/// 种子化客户支持领域的全部工作流
pub(crate) async fn seed_domain_support_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-sup-faq: FAQ知识库 ───────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sup-faq",
            "FAQ知识库",
            "FAQ知识库：收集高频问题，撰写答案，覆盖不完整自动补充，发布知识库",
            "📖",
            vec!["opc".to_string(), "support".to_string()],
            SUP_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：问题收集
                make_agent_node(
                    "a-faq-collect",
                    "问题收集",
                    "收集高频用户问题：工单主题、咨询记录、社区反馈、客服标注。\
                     输出 JSON：{\"questions\":[{\"question\":\"\", \"frequency\":0, \"category\":\"\"}], \"total\":0}",
                    vec![td_desc("OpcListContacts", "获取用户反馈联系人")],
                    Some(PROFILE),
                    "a-faq-collect",
                    0.0,
                    180.0,
                ),
                // Agent：答案撰写
                make_agent_node_full(
                    "a-faq-write",
                    "撰写",
                    "为每个高频问题撰写清晰答案：步骤说明、截图指引、相关链接。\
                     输出 JSON：{\"entries\":[{\"question\":\"\", \"answer\":\"\", \"steps\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-faq-write",
                    vec![("questions", "a-faq-collect")],
                    vec!["a-faq-collect"],
                    0.0,
                    360.0,
                ),
                // 条件：覆盖完整性
                make_condition_node(
                    "c-faq-coverage",
                    "覆盖判定",
                    vec![Condition {
                        var_path: "a-faq-write.entries".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不完整：答案补充
                make_agent_node_full(
                    "a-faq-fill",
                    "答案补充",
                    "答案覆盖不完整，补充缺失条目并校对现有答案。\
                     输出 JSON：{\"added\":[], \"coverage\":1.0}",
                    vec![],
                    Some(PROFILE),
                    "a-faq-fill",
                    vec![("write", "a-faq-write")],
                    vec!["a-faq-write"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-faq", "汇合", 0.0, 900.0),
                // Agent：发布
                make_agent_node_full(
                    "a-faq-publish",
                    "发布",
                    "整理 FAQ 知识库并发布：分类、检索优化、更新机制。\
                     输出 JSON：{\"published\":true, \"categories\":[], \"update_policy\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-faq-publish",
                    vec![("write", "a-faq-write"), ("fill", "a-faq-fill")],
                    vec!["a-faq-write", "a-faq-fill"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "a-faq-collect"),
                edge("e-collect-write", "a-faq-collect", "a-faq-write"),
                edge("e-write-coverage", "a-faq-write", "c-faq-coverage"),
                edge_cond("e-incomplete-fill", "c-faq-coverage", "false", "a-faq-fill", EdgeType::ConditionFalse),
                edge_cond("e-ok-merge", "c-faq-coverage", "true", "m-faq", EdgeType::ConditionTrue),
                edge("e-fill-merge", "a-faq-fill", "m-faq"),
                edge("e-merge-publish", "m-faq", "a-faq-publish"),
                edge("e-publish-end", "a-faq-publish", "end"),
            ],
            vec![DomainInputField { key: "product_name", label: "产品名称", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-sup-satisfaction: 满意度管理 ─────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sup-satisfaction",
            "满意度管理",
            "满意度管理：设计调查问卷，发送用户，逐反馈分析，输出满意度改进报告",
            "😊",
            vec!["opc".to_string(), "support".to_string()],
            SUP_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：调查设计
                make_agent_node(
                    "a-sat-design",
                    "调查设计",
                    "设计满意度调查：问题维度（服务/时效/质量）、评分量表、开放问题。\
                     输出 JSON：{\"dimensions\":[], \"questions\":[], \"scale\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索满意度调查最佳实践")],
                    Some(PROFILE),
                    "a-sat-design",
                    0.0,
                    180.0,
                ),
                // 工具：联系人发送
                make_tool_node(
                    "t-sat-send",
                    "发送调查",
                    "OpcListContacts",
                    vec![("user_input", "a-sat-design")],
                    "t-sat-send",
                    0.0,
                    360.0,
                ),
                // Loop：逐反馈分析
                make_loop_node(
                    "l-sat-feedback",
                    "逐反馈分析",
                    LoopType::ForEach,
                    Some("t-sat-send"),
                    Some("feedback_item"),
                    Some("l-sat-feedback"),
                    Some("l-sat-feedback__partial"),
                    Some(100),
                    vec!["a-sat-feedback".to_string()],
                    0.0,
                    540.0,
                ),
                // Loop body：单条反馈分析
                make_agent_node_full(
                    "a-sat-feedback",
                    "反馈分析",
                    "分析当前反馈：满意度评分、痛点、表扬点、改进建议。\
                     输出 JSON：{\"score\":0, \"pain_points\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-sat-feedback",
                    vec![("feedback", "feedback_item")],
                    vec!["a-sat-design"],
                    250.0,
                    540.0,
                ),
                // Agent：改进报告
                make_agent_node_full(
                    "a-sat-analyze",
                    "改进报告",
                    "汇总全部反馈：满意度分布、趋势、痛点聚类、改进计划。\
                     输出 JSON：{\"avg_score\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-sat-analyze",
                    vec![("feedbacks", "l-sat-feedback.items")],
                    vec!["l-sat-feedback"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-design", "trigger", "a-sat-design"),
                edge("e-design-send", "a-sat-design", "t-sat-send"),
                edge("e-send-loop", "t-sat-send", "l-sat-feedback"),
                edge("e-loop-analyze", "l-sat-feedback", "a-sat-analyze"),
                edge("e-analyze-end", "a-sat-analyze", "end"),
            ],
            vec![DomainInputField {
                key: "survey_period",
                label: "调查周期",
                field_type: "string",
                required: false,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-sup-ticket: 工单处理 ─────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sup-ticket",
            "工单处理",
            "工单处理：分类定级，紧急工单优先处理，常规工单按流程解决，跟进闭环",
            "🎫",
            vec!["opc".to_string(), "support".to_string()],
            SUP_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：工单分类
                make_agent_node(
                    "a-ticket-categorize",
                    "工单分类",
                    "工单分类与定级：问题类型、影响范围、紧急程度。\
                     输出 JSON：{\"category\":\"\", \"severity\":\"critical|high|normal|low\", }",
                    vec![td_desc("OpcListContacts", "查询工单客户信息")],
                    Some(PROFILE),
                    "a-ticket-categorize",
                    0.0,
                    180.0,
                ),
                // 条件：紧急程度
                make_condition_node(
                    "c-ticket-urgent",
                    "紧急判定",
                    vec![
                        Condition {
                            var_path: "a-ticket-categorize.severity".to_string(),
                            operator: CompareOperator::Eq,
                            value: serde_json::json!("critical"),
                        },
                        Condition {
                            var_path: "a-ticket-categorize.severity".to_string(),
                            operator: CompareOperator::Eq,
                            value: serde_json::json!("high"),
                        },
                    ],
                    LogicalOperator::Or,
                    0.0,
                    360.0,
                ),
                // 紧急处理
                make_agent_node_full(
                    "a-ticket-urgent-solve",
                    "紧急处理",
                    "紧急工单优先处理：快速定位、临时规避、升级上报、全程跟进。\
                     输出 JSON：{\"solution\":\"\", }",
                    vec![td_desc("OpcSendNotification", "紧急通知值班工程师")],
                    Some(PROFILE),
                    "a-ticket-urgent-solve",
                    vec![("ticket", "a-ticket-categorize")],
                    vec!["a-ticket-categorize"],
                    -250.0,
                    540.0,
                ),
                // 常规处理
                make_agent_node_full(
                    "a-ticket-solve",
                    "常规处理",
                    "常规工单按流程解决：诊断、方案、执行、验证。\
                     输出 JSON：{\"diagnosis\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-ticket-solve",
                    vec![("ticket", "a-ticket-categorize")],
                    vec!["a-ticket-categorize"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-ticket", "汇合", 0.0, 720.0),
                // Agent：跟进闭环
                make_agent_node_full(
                    "a-ticket-follow",
                    "跟进闭环",
                    "跟进工单闭环：客户确认、回访、知识沉淀、统计。\
                     输出 JSON：{\"confirmed\":true, }",
                    vec![],
                    Some(PROFILE),
                    "a-ticket-follow",
                    vec![("urgent", "a-ticket-urgent-solve"), ("solve", "a-ticket-solve")],
                    vec!["a-ticket-urgent-solve", "a-ticket-solve"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-categorize", "trigger", "a-ticket-categorize"),
                edge("e-categorize-urgent", "a-ticket-categorize", "c-ticket-urgent"),
                edge_cond(
                    "e-urgent-solve",
                    "c-ticket-urgent",
                    "true",
                    "a-ticket-urgent-solve",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-normal-solve",
                    "c-ticket-urgent",
                    "false",
                    "a-ticket-solve",
                    EdgeType::ConditionFalse,
                ),
                edge("e-urgent-merge", "a-ticket-urgent-solve", "m-ticket"),
                edge("e-solve-merge", "a-ticket-solve", "m-ticket"),
                edge("e-merge-follow", "m-ticket", "a-ticket-follow"),
                edge("e-follow-end", "a-ticket-follow", "end"),
            ],
            vec![DomainInputField {
                key: "ticket_id",
                label: "工单号",
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
