// SPDX-License-Identifier: AGPL-3.0-only

//! 产品管理（product）领域工作流种子化 — 3 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-prod-launch:  产品发布（计划 → 准备 → 就绪判定 → 执行/补就绪分支 → 审批）
//! - wf-prod-roadmap: 产品路线图（需求收集 → 优先级排序 → 依赖冲突分支 → 发布）
//! - wf-prod-spec:    产品规格书（需求分析 → 编写 → 评审 → 修订分支）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cpo-cpo-product-manager";
/// product 领域模板版本（v4 丰富拓扑）
const PROD_TEMPLATE_VERSION: i32 = 4;

/// 种子化产品管理领域的全部工作流
pub(crate) async fn seed_domain_product_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-prod-launch: 产品发布 ────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-prod-launch",
            "产品发布",
            "产品发布：制定发布计划，检查发布就绪度，未就绪自动补项，经审批后执行发布",
            "🚀",
            vec!["opc".to_string(), "product".to_string()],
            PROD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：发布计划
                make_agent_node(
                    "a-launch-plan",
                    "发布计划",
                    "制定发布计划：范围、时间窗口、发布策略（灰度/全量）、回滚方案。\
                     输出 JSON：{\"scope\":[], \"window\":\"\", \"strategy\":\"\", \"rollback\":\"\", \"checklist\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-launch-plan",
                    0.0,
                    180.0,
                ),
                // Agent：发布准备
                make_agent_node_full(
                    "a-launch-prep",
                    "发布准备",
                    "执行发布准备：功能验证、文档更新、客服培训、监控告警配置，评估就绪度。\
                     输出 JSON：{\"prepared_items\":[], \"pending\":[], \"ready\":true, \"gaps\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-launch-prep",
                    vec![("plan", "a-launch-plan")],
                    vec!["a-launch-plan"],
                    0.0,
                    360.0,
                ),
                // 条件：就绪判定
                make_condition_node(
                    "c-launch-ready",
                    "就绪判定",
                    vec![Condition {
                        var_path: "a-launch-prep.ready".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 未就绪：补充准备
                make_agent_node_full(
                    "a-launch-fill",
                    "就绪补充",
                    "发布存在缺口，补齐未准备好的项并更新检查清单。\
                     输出 JSON：{\"completed\":[], \"ready\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-launch-fill",
                    vec![("prep", "a-launch-prep")],
                    vec!["a-launch-prep"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-launch", "汇合", 0.0, 900.0),
                // 人工审批
                make_approval_node(
                    "ap-launch",
                    "发布审批",
                    "发布就绪，请审批是否按计划执行发布",
                    Some("release_manager"),
                    86400,
                    "ap-launch",
                    0.0,
                    1080.0,
                ),
                // Agent：执行发布
                make_agent_node_full(
                    "a-launch-exec",
                    "执行发布",
                    "执行发布：按策略发布、监控指标、异常处理、发布后确认。\
                     输出 JSON：{\"status\":\"success|partial|rolled_back\", \"metrics\":{}, \"issues\":[], \"post_checks\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-launch-exec",
                    vec![("prep", "a-launch-prep"), ("fill", "a-launch-fill"), ("approval", "ap-launch")],
                    vec!["a-launch-prep", "a-launch-fill", "ap-launch"],
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-launch-plan"),
                edge("e-plan-prep", "a-launch-plan", "a-launch-prep"),
                edge("e-prep-ready", "a-launch-prep", "c-launch-ready"),
                edge_cond("e-not-ready-fill", "c-launch-ready", "false", "a-launch-fill", EdgeType::ConditionFalse),
                edge_cond("e-ready-merge", "c-launch-ready", "true", "m-launch", EdgeType::ConditionTrue),
                edge("e-fill-merge", "a-launch-fill", "m-launch"),
                edge("e-merge-approval", "m-launch", "ap-launch"),
                edge("e-approval-exec", "ap-launch", "a-launch-exec"),
                edge("e-exec-end", "a-launch-exec", "end"),
            ],
            vec![DomainInputField { key: "release_version", label: "发布版本", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-prod-roadmap: 产品路线图 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-prod-roadmap",
            "产品路线图",
            "产品路线图：收集需求来源，优先级排序，依赖冲突自动调整，发布路线图",
            "🗺️",
            vec!["opc".to_string(), "product".to_string()],
            PROD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // 工具：需求收集
                make_tool_node(
                    "t-road-collect",
                    "需求收集",
                    "OpcListContacts",
                    vec![("user_input", "trigger")],
                    "t-road-collect",
                    0.0,
                    180.0,
                ),
                // Agent：优先级排序
                make_agent_node_full(
                    "a-road-prioritize",
                    "优先级排序",
                    "汇总需求并按价值/成本/风险排序，标注依赖关系与冲突。\
                     输出 JSON：{\"items\":[{\"id\":\"\", \"title\":\"\", \"value\":0, \"cost\":0, \"priority\":\"P0|P1|P2\", \"dependencies\":[]}], \"conflict_count\":0}",
                    vec![td_desc("OpcListContacts", "获取需求相关联系人")],
                    Some(PROFILE),
                    "a-road-prioritize",
                    vec![("contacts", "t-road-collect.result")],
                    vec!["t-road-collect"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在依赖冲突
                make_condition_node(
                    "c-road-conflict",
                    "依赖冲突判定",
                    vec![Condition {
                        var_path: "a-road-prioritize.items".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 冲突：依赖调整
                make_agent_node_full(
                    "a-road-resolve",
                    "依赖调整",
                    "调整依赖冲突：重排优先级、拆分需求、标注阻塞关系。\
                     输出 JSON：{\"adjustments\":[{\"item\":\"\", \"change\":\"\", \"reason\":\"\"}], \"resolved\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-road-resolve",
                    vec![("items", "a-road-prioritize")],
                    vec!["a-road-prioritize"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-road", "汇合", 0.0, 900.0),
                // Agent：路线图发布
                make_agent_node_full(
                    "a-road-publish",
                    "发布",
                    "生成产品路线图：时间轴、里程碑、主题分组。\
                     输出 JSON：{\"horizon\":[{\"phase\":\"\", \"themes\":[], \"items\":[]}], \"milestones\":[], \"note\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-road-publish",
                    vec![("items", "a-road-prioritize"), ("resolve", "a-road-resolve")],
                    vec!["a-road-prioritize", "a-road-resolve"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "t-road-collect"),
                edge("e-collect-prioritize", "t-road-collect", "a-road-prioritize"),
                edge("e-prioritize-conflict", "a-road-prioritize", "c-road-conflict"),
                edge_cond(
                    "e-conflict-resolve",
                    "c-road-conflict",
                    "true",
                    "a-road-resolve",
                    EdgeType::ConditionTrue,
                ),
                edge_cond("e-clean-merge", "c-road-conflict", "false", "m-road", EdgeType::ConditionFalse),
                edge("e-resolve-merge", "a-road-resolve", "m-road"),
                edge("e-merge-publish", "m-road", "a-road-publish"),
                edge("e-publish-end", "a-road-publish", "end"),
            ],
            vec![DomainInputField { key: "horizon", label: "规划周期", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-prod-spec: 产品规格书 ────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-prod-spec",
            "产品规格书",
            "产品规格书：分析需求，编写规格书，评审未通过自动修订直至通过",
            "📋",
            vec!["opc".to_string(), "product".to_string()],
            PROD_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：需求分析
                make_agent_node(
                    "a-spec-req",
                    "需求分析",
                    "分析需求：用户故事、业务规则、验收标准、边界与异常场景。\
                     输出 JSON：{\"stories\":[], \"business_rules\":[], \"acceptance\":[], \"edge_cases\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-spec-req",
                    0.0,
                    180.0,
                ),
                // Agent：规格编写
                make_agent_node_full(
                    "a-spec-write",
                    "编写",
                    "编写产品规格书：功能描述、交互说明、数据要求、非功能要求。\
                     输出 JSON：{\"functional\":[], \"interaction\":\"\", \"data\":\"\", \"non_functional\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-spec-write",
                    vec![("req", "a-spec-req")],
                    vec!["a-spec-req"],
                    0.0,
                    360.0,
                ),
                // Agent：评审
                make_agent_node_full(
                    "a-spec-review",
                    "评审",
                    "评审规格书：完整性、一致性、可实现性、可测试性。\
                     输出 JSON：{\"passed\":false, \"issues\":[{\"severity\":\"\", \"description\":\"\", \"suggestion\":\"\"}], \"blocker_count\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-spec-review",
                    vec![("spec", "a-spec-write")],
                    vec!["a-spec-write"],
                    0.0,
                    540.0,
                ),
                // 条件：评审是否通过
                make_condition_node(
                    "c-spec-passed",
                    "评审判定",
                    vec![Condition {
                        var_path: "a-spec-review.passed".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    720.0,
                ),
                // 未通过：修订
                make_agent_node_full(
                    "a-spec-revise",
                    "修订",
                    "按评审意见修订规格书，消除所有阻塞性问题。\
                     输出 JSON：{\"revisions\":[], \"passed\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-spec-revise",
                    vec![("review", "a-spec-review"), ("spec", "a-spec-write")],
                    vec!["a-spec-review", "a-spec-write"],
                    -250.0,
                    900.0,
                ),
                make_merge_node("m-spec", "汇合", 0.0, 1080.0),
                // Agent：定稿
                make_agent_node_full(
                    "a-spec-final",
                    "定稿",
                    "规格书通过评审，整理定稿版本与变更记录。\
                     输出 JSON：{\"final_version\":\"\", \"changelog\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-spec-final",
                    vec![("review", "a-spec-review"), ("revise", "a-spec-revise")],
                    vec!["a-spec-review", "a-spec-revise"],
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-req", "trigger", "a-spec-req"),
                edge("e-req-write", "a-spec-req", "a-spec-write"),
                edge("e-write-review", "a-spec-write", "a-spec-review"),
                edge("e-review-passed", "a-spec-review", "c-spec-passed"),
                edge_cond("e-fail-revise", "c-spec-passed", "false", "a-spec-revise", EdgeType::ConditionFalse),
                edge_cond("e-pass-merge", "c-spec-passed", "true", "m-spec", EdgeType::ConditionTrue),
                edge("e-revise-merge", "a-spec-revise", "m-spec"),
                edge("e-merge-final", "m-spec", "a-spec-final"),
                edge("e-final-end", "a-spec-final", "end"),
            ],
            vec![DomainInputField { key: "feature_name", label: "功能名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
