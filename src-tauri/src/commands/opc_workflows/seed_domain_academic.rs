// SPDX-License-Identifier: AGPL-3.0-only

//! 学术研究（academic）领域工作流种子化 — 2 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-acd-literature: 文献综述（检索 → 筛选 → 充足性判定 → 逐篇精读循环 → 综述 → 审批）
//! - wf-acd-research:   研究方案（问题定义 → 方法设计 → 可行性判定 → 调整/计划 → 审批）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{
    CompareOperator, Condition, EdgeType, LogicalOperator, LoopType,
};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-ceo-ceo-business-strategist";
/// academic 领域模板版本（v4 丰富拓扑）
const ACD_TEMPLATE_VERSION: i32 = 4;

/// 种子化学术研究领域的全部工作流
pub(crate) async fn seed_domain_academic_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-acd-literature: 文献综述 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-acd-literature",
            "文献综述",
            "文献综述：检索目标领域文献，筛选高质量来源，逐篇精读提取要点，综合撰写综述，经学术审批",
            "📚",
            vec!["opc".to_string(), "academic".to_string()],
            ACD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // 工具：文献检索
                make_tool_node(
                    "t-lit-search",
                    "文献检索",
                    "OpcSearchWiki",
                    vec![("user_input", "trigger")],
                    "t-lit-search",
                    0.0,
                    180.0,
                ),
                // Agent：文献筛选
                make_agent_node_full(
                    "a-lit-screen",
                    "文献筛选",
                    "筛选检索结果：按相关度、时效、权威性评估，剔除低质量文献，输出精读清单。\
                     输出 JSON：{\"papers\":[{\"title\":\"\", \"authors\":\"\", \"year\":0, \"relevance\":0, \"key_claims\":[]}], \"total\":0}",
                    vec![td_desc("OpcSearchWiki", "检索学术文献")],
                    Some(PROFILE),
                    "a-lit-screen",
                    vec![("search", "t-lit-search.result")],
                    vec!["t-lit-search"],
                    0.0,
                    360.0,
                ),
                // 条件：文献是否充足
                make_condition_node(
                    "c-lit-sufficient",
                    "文献充足性",
                    vec![Condition {
                        var_path: "a-lit-screen.total".to_string(),
                        operator: CompareOperator::Gte,
                        value: serde_json::json!(5),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不足分支：扩展检索
                make_agent_node_full(
                    "a-lit-expand",
                    "扩展检索",
                    "文献不足 5 篇，扩大检索范围：放宽时间窗、补充关键词、纳入综述型文献。\
                     输出 JSON：{\"new_queries\":[], \"expanded_papers\":[], \"total\":0}",
                    vec![td_desc("OpcSearchWiki", "扩展检索")],
                    Some(PROFILE),
                    "a-lit-expand",
                    vec![("screen", "a-lit-screen")],
                    vec!["a-lit-screen"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-lit", "汇合", 0.0, 900.0),
                // Loop：逐篇精读
                make_loop_node(
                    "l-lit-read",
                    "逐篇精读",
                    LoopType::ForEach,
                    Some("a-lit-screen"),
                    Some("paper_item"),
                    Some("l-lit-read"),
                    Some("l-lit-read__partial"),
                    Some(50),
                    vec!["a-lit-read".to_string()],
                    0.0,
                    1080.0,
                ),
                // Loop body：单篇精读
                make_agent_node_full(
                    "a-lit-read",
                    "单篇精读",
                    "精读当前文献：提取研究问题、方法、数据、结论与局限。\
                     输出 JSON：{\"title\":\"\", \"question\":\"\", \"method\":\"\", \"findings\":[], \"limitations\":[], \"contribution\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-lit-read",
                    vec![("paper", "paper_item")],
                    vec!["a-lit-screen"],
                    250.0,
                    1080.0,
                ),
                // Agent：综述撰写
                make_agent_node_full(
                    "a-lit-synthesize",
                    "综述撰写",
                    "综合全部精读笔记，撰写综述：研究脉络、主题聚类、争议点、研究空白。\
                     输出 JSON：{\"overview\":\"\", \"themes\":[{\"topic\":\"\", \"findings\":[], \"debates\":[]}], \"research_gaps\":[], \"references\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-lit-synthesize",
                    vec![("notes", "l-lit-read.items")],
                    vec!["l-lit-read"],
                    0.0,
                    1260.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-lit",
                    "综述审批",
                    "文献综述已完成，请审核内容与引用质量",
                    Some("advisor"),
                    86400,
                    "ap-lit",
                    0.0,
                    1440.0,
                ),
                make_end(0.0, 1620.0),
            ],
            vec![
                edge("e-trigger-search", "trigger", "t-lit-search"),
                edge("e-search-screen", "t-lit-search", "a-lit-screen"),
                edge("e-screen-sufficient", "a-lit-screen", "c-lit-sufficient"),
                edge_cond(
                    "e-insufficient-expand",
                    "c-lit-sufficient",
                    "false",
                    "a-lit-expand",
                    EdgeType::ConditionFalse,
                ),
                edge_cond(
                    "e-sufficient-merge",
                    "c-lit-sufficient",
                    "true",
                    "m-lit",
                    EdgeType::ConditionTrue,
                ),
                edge("e-expand-merge", "a-lit-expand", "m-lit"),
                edge("e-merge-loop", "m-lit", "l-lit-read"),
                edge("e-loop-synthesize", "l-lit-read", "a-lit-synthesize"),
                edge("e-synthesize-approval", "a-lit-synthesize", "ap-lit"),
                edge("e-approval-end", "ap-lit", "end"),
            ],
            vec![DomainInputField { key: "research_topic", label: "研究主题", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-acd-research: 研究方案 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-acd-research",
            "研究方案",
            "研究方案：定义研究问题与假设，设计方法论，可行性不足自动调整，经学术审批",
            "🔬",
            vec!["opc".to_string(), "academic".to_string()],
            ACD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：研究问题
                make_agent_node(
                    "a-research-question",
                    "研究问题",
                    "定义研究问题、假设与预期贡献，明确研究边界。\
                     输出 JSON：{\"question\":\"\", \"hypotheses\":[], \"contribution\":\"\", \"scope\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-research-question",
                    0.0,
                    180.0,
                ),
                // Agent：方法论设计
                make_agent_node_full(
                    "a-research-method",
                    "方法论",
                    "设计研究方法：数据来源、采集方案、分析方法与验证策略。\
                     输出 JSON：{\"design\":\"\", \"data_sources\":[], \"analysis\":\"\", \"validation\":\"\", \"feasible\":true, \"risk_points\":[]}",
                    vec![td_desc("OpcSearchWiki", "检索同类研究的方法论")],
                    Some(PROFILE),
                    "a-research-method",
                    vec![("question", "a-research-question")],
                    vec!["a-research-question"],
                    0.0,
                    360.0,
                ),
                // 条件：方法可行性
                make_condition_node(
                    "c-research-feasible",
                    "可行性判定",
                    vec![Condition {
                        var_path: "a-research-method.feasible".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不可行分支：方法调整
                make_agent_node_full(
                    "a-research-adjust",
                    "方法调整",
                    "方法论存在不可行风险，提出替代方案：简化设计、更换数据源、调整范围。\
                     输出 JSON：{\"alternatives\":[{\"option\":\"\", \"pros\":[], \"cons\":[]}], \"recommended\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-research-adjust",
                    vec![("method", "a-research-method")],
                    vec!["a-research-method"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-research", "汇合", 0.0, 900.0),
                // Agent：研究计划
                make_agent_node_full(
                    "a-research-plan",
                    "研究计划",
                    "制定研究时间表、里程碑、资源需求与阶段交付物。\
                     输出 JSON：{\"timeline\":[{\"phase\":\"\", \"duration\":\"\", \"deliverable\":\"\"}], \"resources\":[], \"milestones\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-research-plan",
                    vec![("method", "a-research-method"), ("adjust", "a-research-adjust")],
                    vec!["a-research-method", "a-research-adjust"],
                    0.0,
                    1080.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-research",
                    "方案审批",
                    "研究方案与计划已完成，请审核",
                    Some("advisor"),
                    86400,
                    "ap-research",
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-question", "trigger", "a-research-question"),
                edge("e-question-method", "a-research-question", "a-research-method"),
                edge("e-method-feasible", "a-research-method", "c-research-feasible"),
                edge_cond(
                    "e-infeasible-adjust",
                    "c-research-feasible",
                    "false",
                    "a-research-adjust",
                    EdgeType::ConditionFalse,
                ),
                edge_cond(
                    "e-feasible-merge",
                    "c-research-feasible",
                    "true",
                    "m-research",
                    EdgeType::ConditionTrue,
                ),
                edge("e-adjust-merge", "a-research-adjust", "m-research"),
                edge("e-merge-plan", "m-research", "a-research-plan"),
                edge("e-plan-approval", "a-research-plan", "ap-research"),
                edge("e-approval-end", "ap-research", "end"),
            ],
            vec![DomainInputField { key: "research_field", label: "研究领域", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
