// SPDX-License-Identifier: AGPL-3.0-only

//! 专业服务（specialized）领域工作流种子化 — 10 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-spc-change-mgmt:  变更管理（影响评估 → 重大影响分支 → 变革计划 → 执行反馈）
//! - wf-spc-data-privacy: 数据隐私合规（审计 → 高风险分支 → 整改 → 验证）
//! - wf-spc-esg:          ESG报告（数据收集 → 完整性分支 → 测量 → 报告）
//! - wf-spc-grant:        项目申请（研究 → 指南合规分支 → 撰写 → 提交）
//! - wf-spc-hire:         招聘流程（JD → 筛选 → 达标分支 → 评估 → 面试）
//! - wf-spc-legal-review: 合同审查（上传 → 审查 → 重大风险分支 → 报告）
//! - wf-spc-localization: 本地化（审计 → 逐内容翻译循环 → 验证）
//! - wf-spc-m-a:          并购整合（审计 → 整合风险分支 → 计划 → 执行）
//! - wf-spc-onboard:      员工入职（计划 → 准备完整性分支 → 环境准备 → 引导）
//! - wf-spc-supply-chain: 供应链优化（审计 → 瓶颈分支 → 优化 → 实施）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{
    CompareOperator, Condition, EdgeType, LogicalOperator, LoopType,
};
use sea_orm::DatabaseConnection;

const CEO: &str = "opc-ceo-ceo-business-strategist";
const CFO: &str = "opc-cfo-cfo-financial-analyst";
const COO: &str = "opc-coo-coo-operations-manager";
/// specialized 领域模板版本（v4 丰富拓扑）
const SPC_TEMPLATE_VERSION: i32 = 4;

/// 种子化专业服务领域的全部工作流
pub(crate) async fn seed_domain_specialized_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-spc-change-mgmt: 变更管理 ────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-change-mgmt",
            "变更管理",
            "变更管理：评估变革影响，重大变革生成详细计划，执行并收集反馈调整",
            "🔄",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：影响评估
                make_agent_node(
                    "a-change-impact",
                    "影响评估",
                    "评估变革对组织、流程、人员的影响范围与程度。\
                     输出 JSON：{\"impact_level\":\"high|medium|low\", \"affected_org\":[], \"affected_processes\":[], \"people_impact\":\"\", \"risks\":[]}",
                    vec![td_desc("OpcSearchWiki", "检索同类变革案例")],
                    Some(CEO),
                    "a-change-impact",
                    0.0,
                    180.0,
                ),
                // 条件：影响重大
                make_condition_node(
                    "c-change-major",
                    "影响判定",
                    vec![Condition {
                        var_path: "a-change-impact.impact_level".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!("high"),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 重大：详细变革计划
                make_agent_node_full(
                    "a-change-plan",
                    "变革计划",
                    "制定分阶段变革实施与沟通计划：阶段目标、沟通策略、培训、里程碑。\
                     输出 JSON：{\"phases\":[{\"phase\":\"\", \"goal\":\"\", \"actions\":[], \"milestone\":\"\"}], \"communication\":\"\", \"training\":[]}",
                    vec![],
                    Some(CEO),
                    "a-change-plan",
                    vec![("impact", "a-change-impact")],
                    vec!["a-change-impact"],
                    -250.0,
                    540.0,
                ),
                // 一般：轻量计划
                make_agent_node_full(
                    "a-change-light",
                    "轻量计划",
                    "变革影响一般，制定轻量实施计划。\
                     输出 JSON：{\"actions\":[], \"owner\":\"\", \"timeline\":\"\"}",
                    vec![],
                    Some(CEO),
                    "a-change-light",
                    vec![("impact", "a-change-impact")],
                    vec!["a-change-impact"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-change", "汇合", 0.0, 720.0),
                // Agent：变革执行
                make_agent_node_full(
                    "a-change-exec",
                    "变革执行",
                    "监督变革执行，收集反馈并调整计划。\
                     输出 JSON：{\"progress\":0, \"feedback\":[], \"adjustments\":[], \"blockers\":[]}",
                    vec![td_desc("OpcSendNotification", "通知变革相关方")],
                    Some(CEO),
                    "a-change-exec",
                    vec![("plan", "a-change-plan"), ("light", "a-change-light")],
                    vec!["a-change-plan", "a-change-light"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-impact", "trigger", "a-change-impact"),
                edge("e-impact-major", "a-change-impact", "c-change-major"),
                edge_cond("e-major-plan", "c-change-major", "true", "a-change-plan", EdgeType::ConditionTrue),
                edge_cond("e-light-plan", "c-change-major", "false", "a-change-light", EdgeType::ConditionFalse),
                edge("e-plan-merge", "a-change-plan", "m-change"),
                edge("e-light-merge", "a-change-light", "m-change"),
                edge("e-merge-exec", "m-change", "a-change-exec"),
                edge("e-exec-end", "a-change-exec", "end"),
            ],
            vec![DomainInputField { key: "change_desc", label: "变革描述", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-data-privacy: 数据隐私合规 ───────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-data-privacy",
            "数据隐私合规",
            "数据隐私合规：审计数据流程，高风险差距紧急整改，验证整改效果",
            "🔒",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：隐私审计
                make_agent_node(
                    "a-privacy-audit",
                    "隐私审计",
                    "审计数据采集、存储、处理、共享流程：授权、加密、留存、跨境。\
                     输出 JSON：{\"findings\":[{\"area\":\"\", \"gap\":\"\", \"risk\":\"high|medium|low\"}], \"high_risk_count\":0}",
                    vec![td_desc("OpcSearchWiki", "检索隐私法规要求")],
                    Some(CFO),
                    "a-privacy-audit",
                    0.0,
                    180.0,
                ),
                // 条件：是否存在高风险差距
                make_condition_node(
                    "c-privacy-high",
                    "风险判定",
                    vec![Condition {
                        var_path: "a-privacy-audit.high_risk_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 高风险：紧急整改
                make_agent_node_full(
                    "a-privacy-fix",
                    "紧急整改",
                    "优先整改高风险差距：立即行动、责任到人、限期完成。\
                     输出 JSON：{\"remediation\":[{\"gap\":\"\", \"action\":\"\", \"owner\":\"\", \"deadline\":\"\"}], \"risk_after\":\"low\"}",
                    vec![td_desc("OpcSendNotification", "通知隐私整改责任人")],
                    Some(CFO),
                    "a-privacy-fix",
                    vec![("audit", "a-privacy-audit")],
                    vec!["a-privacy-audit"],
                    -250.0,
                    540.0,
                ),
                // 低风险：常规整改
                make_agent_node_full(
                    "a-privacy-normal",
                    "常规整改",
                    "中低风险差距按计划整改。\
                     输出 JSON：{\"plan\":[{\"gap\":\"\", \"action\":\"\", \"due\":\"\"}]}",
                    vec![],
                    Some(CFO),
                    "a-privacy-normal",
                    vec![("audit", "a-privacy-audit")],
                    vec!["a-privacy-audit"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-privacy", "汇合", 0.0, 720.0),
                // Agent：整改验证
                make_agent_node_full(
                    "a-privacy-verify",
                    "整改验证",
                    "验证整改效果：复测、证据收集、合规状态更新。\
                     输出 JSON：{\"verified\":true, \"remaining_gaps\":[], \"compliance_status\":\"\"}",
                    vec![],
                    Some(CFO),
                    "a-privacy-verify",
                    vec![("fix", "a-privacy-fix"), ("normal", "a-privacy-normal")],
                    vec!["a-privacy-fix", "a-privacy-normal"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-audit", "trigger", "a-privacy-audit"),
                edge("e-audit-high", "a-privacy-audit", "c-privacy-high"),
                edge_cond("e-high-fix", "c-privacy-high", "true", "a-privacy-fix", EdgeType::ConditionTrue),
                edge_cond("e-low-normal", "c-privacy-high", "false", "a-privacy-normal", EdgeType::ConditionFalse),
                edge("e-fix-merge", "a-privacy-fix", "m-privacy"),
                edge("e-normal-merge", "a-privacy-normal", "m-privacy"),
                edge("e-merge-verify", "m-privacy", "a-privacy-verify"),
                edge("e-verify-end", "a-privacy-verify", "end"),
            ],
            vec![DomainInputField { key: "data_scope", label: "数据范围", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-esg: ESG报告 ─────────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-esg",
            "ESG报告",
            "ESG报告：收集环境/社会/治理数据，数据不完整自动补充，测量指标并输出报告",
            "🌱",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：数据收集
                make_agent_node(
                    "a-esg-collect",
                    "数据收集",
                    "收集 ESG 数据：环境排放、能耗、社会指标、治理结构。\
                     输出 JSON：{\"environment\":{}, \"social\":{}, \"governance\":{}, \"data_complete\":true, \"missing\":[]}",
                    vec![td_desc("OpcSearchWiki", "检索 ESG 披露框架")],
                    Some(CEO),
                    "a-esg-collect",
                    0.0,
                    180.0,
                ),
                // 条件：数据完整性
                make_condition_node(
                    "c-esg-complete",
                    "完整性判定",
                    vec![Condition {
                        var_path: "a-esg-collect.data_complete".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 不完整：数据补充
                make_agent_node_full(
                    "a-esg-fill",
                    "数据补充",
                    "补齐缺失数据：估算、补充来源、说明口径。\
                     输出 JSON：{\"filled\":[], \"data_complete\":true}",
                    vec![],
                    Some(CEO),
                    "a-esg-fill",
                    vec![("collect", "a-esg-collect")],
                    vec!["a-esg-collect"],
                    -250.0,
                    540.0,
                ),
                make_merge_node("m-esg", "汇合", 0.0, 720.0),
                // Agent：指标测量
                make_agent_node_full(
                    "a-esg-measure",
                    "指标测量",
                    "计算 ESG 关键指标：排放强度、能耗效率、多元比例、合规率。\
                     输出 JSON：{\"metrics\":{}, \"trends\":{}, \"benchmarks\":{}}",
                    vec![],
                    Some(CEO),
                    "a-esg-measure",
                    vec![("collect", "a-esg-collect"), ("fill", "a-esg-fill")],
                    vec!["a-esg-collect", "a-esg-fill"],
                    0.0,
                    900.0,
                ),
                // Agent：报告
                make_agent_node_full(
                    "a-esg-report",
                    "报告",
                    "输出 ESG 报告：指标披露、目标达成、改进计划。\
                     输出 JSON：{\"summary\":\"\", \"disclosures\":[], \"targets\":[], \"improvements\":[]}",
                    vec![],
                    Some(CEO),
                    "a-esg-report",
                    vec![("measure", "a-esg-measure")],
                    vec!["a-esg-measure"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "a-esg-collect"),
                edge("e-collect-complete", "a-esg-collect", "c-esg-complete"),
                edge_cond("e-incomplete-fill", "c-esg-complete", "false", "a-esg-fill", EdgeType::ConditionFalse),
                edge_cond("e-ok-merge", "c-esg-complete", "true", "m-esg", EdgeType::ConditionTrue),
                edge("e-fill-merge", "a-esg-fill", "m-esg"),
                edge("e-merge-measure", "m-esg", "a-esg-measure"),
                edge("e-measure-report", "a-esg-measure", "a-esg-report"),
                edge("e-report-end", "a-esg-report", "end"),
            ],
            vec![DomainInputField { key: "report_year", label: "报告年度", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-grant: 项目申请 ──────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-grant",
            "项目申请",
            "项目申请：研究资助机会，申请书不符合指南自动修改，定稿提交",
            "📄",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：资助研究
                make_agent_node(
                    "a-grant-research",
                    "资助研究",
                    "研究资助机会：资助方、金额、申请条件、截止日期、匹配度。\
                     输出 JSON：{\"opportunities\":[{\"funder\":\"\", \"amount\":0, \"deadline\":\"\", \"fit\":0}], }",
                    vec![td_desc("OpcSearchWiki", "检索资助机会")],
                    Some(CEO),
                    "a-grant-research",
                    0.0,
                    180.0,
                ),
                // Agent：申请撰写
                make_agent_node_full(
                    "a-grant-write",
                    "申请撰写",
                    "撰写申请材料：项目概述、预算、时间线、预期成果，对照指南检查。\
                     输出 JSON：{\"application\":\"\", \"budget\":0, }",
                    vec![],
                    Some(CEO),
                    "a-grant-write",
                    vec![("research", "a-grant-research")],
                    vec!["a-grant-research"],
                    0.0,
                    360.0,
                ),
                // 条件：指南合规
                make_condition_node(
                    "c-grant-compliant",
                    "指南合规判定",
                    vec![Condition {
                        var_path: "a-grant-write.application".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不合规：申请修改
                make_agent_node_full(
                    "a-grant-revise",
                    "申请修改",
                    "对照资助指南修改申请材料。\
                     输出 JSON：{\"revisions\":[], \"compliant\":true}",
                    vec![],
                    Some(CEO),
                    "a-grant-revise",
                    vec![("write", "a-grant-write")],
                    vec!["a-grant-write"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-grant", "汇合", 0.0, 900.0),
                // Agent：提交
                make_agent_node_full(
                    "a-grant-submit",
                    "提交",
                    "完成申请定稿并提交：材料核对、渠道确认、提交确认。\
                     输出 JSON：{\"submitted\":true, }",
                    vec![],
                    Some(CEO),
                    "a-grant-submit",
                    vec![("write", "a-grant-write"), ("revise", "a-grant-revise")],
                    vec!["a-grant-write", "a-grant-revise"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-research", "trigger", "a-grant-research"),
                edge("e-research-write", "a-grant-research", "a-grant-write"),
                edge("e-write-compliant", "a-grant-write", "c-grant-compliant"),
                edge_cond(
                    "e-violation-revise",
                    "c-grant-compliant",
                    "false",
                    "a-grant-revise",
                    EdgeType::ConditionFalse,
                ),
                edge_cond("e-ok-merge", "c-grant-compliant", "true", "m-grant", EdgeType::ConditionTrue),
                edge("e-revise-merge", "a-grant-revise", "m-grant"),
                edge("e-merge-submit", "m-grant", "a-grant-submit"),
                edge("e-submit-end", "a-grant-submit", "end"),
            ],
            vec![DomainInputField { key: "project_topic", label: "项目主题", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-hire: 招聘流程 ──────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-hire",
            "招聘流程",
            "招聘流程：编写岗位 JD，筛选简历，候选不达标扩大筛选，评估并安排面试",
            "👥",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：JD 编写
                make_agent_node(
                    "a-hire-jd",
                    "岗位JD",
                    "编写岗位 JD：职责、要求、加分项、薪资范围、评估标准。\
                     输出 JSON：{\"title\":\"\", \"responsibilities\":[], \"requirements\":[], \"evaluation_criteria\":[]}",
                    vec![],
                    Some(COO),
                    "a-hire-jd",
                    0.0,
                    180.0,
                ),
                // Agent：简历筛选
                make_agent_node_full(
                    "a-hire-screen",
                    "简历筛选",
                    "按 JD 筛选简历：匹配度评分、优劣势、推荐进入评估。\
                     输出 JSON：{\"candidates\":[{\"name\":\"\", \"match\":0, \"strengths\":[], \"concerns\":[]}], }",
                    vec![td_desc("OpcListContacts", "查询候选人联系信息")],
                    Some(COO),
                    "a-hire-screen",
                    vec![("jd", "a-hire-jd")],
                    vec!["a-hire-jd"],
                    0.0,
                    360.0,
                ),
                // 条件：达标候选数量
                make_condition_node(
                    "c-hire-qualified",
                    "候选达标判定",
                    vec![Condition {
                        var_path: "a-hire-screen.candidates".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 达标：评估
                make_agent_node_full(
                    "a-hire-evaluate",
                    "评估",
                    "深度评估候选：技能考核、文化匹配、薪酬预期。\
                     输出 JSON：{\"evaluations\":[{\"name\":\"\", \"score\":0, \"recommendation\":\"advance|hold|reject\", \"interview_questions\":[]}], }",
                    vec![],
                    Some(COO),
                    "a-hire-evaluate",
                    vec![("screen", "a-hire-screen")],
                    vec!["a-hire-screen"],
                    -250.0,
                    720.0,
                ),
                // 不达标：扩大筛选
                make_agent_node_full(
                    "a-hire-expand",
                    "扩大筛选",
                    "达标候选不足，扩大渠道：放宽条件、主动寻源、内推。\
                     输出 JSON：{\"expanded_pool\":[], \"additional_candidates\":[]}",
                    vec![td_desc("OpcSearchWiki", "检索招聘渠道")],
                    Some(COO),
                    "a-hire-expand",
                    vec![("screen", "a-hire-screen")],
                    vec!["a-hire-screen"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-hire", "汇合", 0.0, 900.0),
                // Agent：面试安排
                make_agent_node_full(
                    "a-hire-interview",
                    "面试安排",
                    "安排面试：面试官、时间、环节、评估表。\
                     输出 JSON：{\"schedule\":[{\"candidate\":\"\", \"interviewers\":[], }",
                    vec![],
                    Some(COO),
                    "a-hire-interview",
                    vec![("evaluate", "a-hire-evaluate"), ("expand", "a-hire-expand")],
                    vec!["a-hire-evaluate", "a-hire-expand"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-jd", "trigger", "a-hire-jd"),
                edge("e-jd-screen", "a-hire-jd", "a-hire-screen"),
                edge("e-screen-qualified", "a-hire-screen", "c-hire-qualified"),
                edge_cond("e-ok-evaluate", "c-hire-qualified", "true", "a-hire-evaluate", EdgeType::ConditionTrue),
                edge_cond("e-no-expand", "c-hire-qualified", "false", "a-hire-expand", EdgeType::ConditionFalse),
                edge("e-evaluate-merge", "a-hire-evaluate", "m-hire"),
                edge("e-expand-merge", "a-hire-expand", "m-hire"),
                edge("e-merge-interview", "m-hire", "a-hire-interview"),
                edge("e-interview-end", "a-hire-interview", "end"),
            ],
            vec![DomainInputField { key: "position", label: "职位", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-legal-review: 合同审查 ───────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-legal-review",
            "合同审查",
            "合同审查：上传合同文本，逐条审查条款，重大风险升级报告",
            "⚖️",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：合同上传
                make_agent_node(
                    "a-legal-upload",
                    "合同上传",
                    "整理合同文本与背景：合同类型、双方、金额、期限、相关附件。\
                     输出 JSON：{\"contract_type\":\"\", \"parties\":[], \"amount\":0, }",
                    vec![td_desc("OpcSearchWiki", "检索合同模板与法规")],
                    Some(CFO),
                    "a-legal-upload",
                    0.0,
                    180.0,
                ),
                // Agent：条款审查
                make_agent_node_full(
                    "a-legal-review",
                    "条款审查",
                    "逐条审查合同条款：责任、赔偿、违约、保密、知识产权、终止。\
                     输出 JSON：{\"clauses\":[{\"no\":\"\", \"summary\":\"\", }",
                    vec![],
                    Some(CFO),
                    "a-legal-review",
                    vec![("upload", "a-legal-upload")],
                    vec!["a-legal-upload"],
                    0.0,
                    360.0,
                ),
                // 条件：重大风险
                make_condition_node(
                    "c-legal-risk",
                    "风险判定",
                    vec![Condition {
                        var_path: "a-legal-review.clauses".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 重大风险：升级
                make_agent_node_full(
                    "a-legal-escalate",
                    "风险升级",
                    "发现重大合同风险，升级至法务负责人：风险条款、影响、谈判建议。\
                     输出 JSON：{\"escalation\":[], }",
                    vec![td_desc("OpcSendNotification", "通知法务负责人重大风险")],
                    Some(CFO),
                    "a-legal-escalate",
                    vec![("review", "a-legal-review")],
                    vec!["a-legal-review"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-legal", "汇合", 0.0, 900.0),
                // Agent：审查报告
                make_agent_node_full(
                    "a-legal-report",
                    "审查报告",
                    "输出合同审查报告：条款风险、修改建议、签署意见。\
                     输出 JSON：{\"risk_level\":\"\", \"suggestions\":[], }",
                    vec![],
                    Some(CFO),
                    "a-legal-report",
                    vec![("review", "a-legal-review"), ("escalate", "a-legal-escalate")],
                    vec!["a-legal-review", "a-legal-escalate"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-upload", "trigger", "a-legal-upload"),
                edge("e-upload-review", "a-legal-upload", "a-legal-review"),
                edge("e-review-risk", "a-legal-review", "c-legal-risk"),
                edge_cond(
                    "e-high-escalate",
                    "c-legal-risk",
                    "true",
                    "a-legal-escalate",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-low-merge",
                    "c-legal-risk",
                    "false",
                    "m-legal",
                    EdgeType::ConditionFalse,
                ),
                edge("e-escalate-merge", "a-legal-escalate", "m-legal"),
                edge("e-merge-report", "m-legal", "a-legal-report"),
                edge("e-report-end", "a-legal-report", "end"),
            ],
            vec![DomainInputField {
                key: "contract_name",
                label: "合同名称",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-localization: 本地化 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-localization",
            "本地化",
            "本地化：审计内容本地化需求，逐内容翻译循环，质量验证后发布",
            "🌐",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：本地化审计
                make_agent_node(
                    "a-locale-audit",
                    "本地化审计",
                    "审计本地化需求：目标语言、内容清单、文化适配点、术语表。\
                     输出 JSON：{\"languages\":[], \"contents\":[{\"id\":\"\", \"type\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索目标市场文化规范")],
                    Some(COO),
                    "a-locale-audit",
                    0.0,
                    180.0,
                ),
                // Loop：逐内容翻译
                make_loop_node(
                    "l-locale-translate",
                    "逐内容翻译",
                    LoopType::ForEach,
                    Some("a-locale-audit"),
                    Some("content_item"),
                    Some("l-locale-translate"),
                    Some("l-locale-translate__partial"),
                    Some(50),
                    vec!["a-locale-translate".to_string()],
                    0.0,
                    360.0,
                ),
                // Loop body：单内容翻译
                make_agent_node_full(
                    "a-locale-translate",
                    "内容翻译",
                    "翻译当前内容：准确传达、文化适配、术语一致、风格统一。\
                     输出 JSON：{\"id\":\"\", }",
                    vec![],
                    Some(COO),
                    "a-locale-translate",
                    vec![("content", "content_item")],
                    vec!["a-locale-audit"],
                    250.0,
                    360.0,
                ),
                // Agent：质量验证
                make_agent_node_full(
                    "a-locale-verify",
                    "质量验证",
                    "验证翻译质量：术语一致性、文化敏感性、格式正确性、母语复核。\
                     输出 JSON：{\"quality_score\":0, }",
                    vec![],
                    Some(COO),
                    "a-locale-verify",
                    vec![("translations", "l-locale-translate.items")],
                    vec!["l-locale-translate"],
                    0.0,
                    720.0,
                ),
                make_end(0.0, 900.0),
            ],
            vec![
                edge("e-trigger-audit", "trigger", "a-locale-audit"),
                edge("e-audit-loop", "a-locale-audit", "l-locale-translate"),
                edge("e-loop-verify", "l-locale-translate", "a-locale-verify"),
                edge("e-verify-end", "a-locale-verify", "end"),
            ],
            vec![DomainInputField {
                key: "target_lang",
                label: "目标语言",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-m-a: 并购整合 ────────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-m-a",
            "并购整合",
            "并购整合：审计标的与协同点，整合风险高发详细计划，执行整合并跟踪",
            "🤝",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：并购审计
                make_agent_node(
                    "a-ma-audit",
                    "并购审计",
                    "审计并购标的：财务、运营、文化、技术、协同价值。\
                     输出 JSON：{\"financial\":\"\", \"synergies\":[], }",
                    vec![td_desc("OpcSearchWiki", "检索标的公司信息")],
                    Some(CEO),
                    "a-ma-audit",
                    0.0,
                    180.0,
                ),
                // 条件：整合风险
                make_condition_node(
                    "c-ma-risk",
                    "风险判定",
                    vec![Condition {
                        var_path: "a-ma-audit.synergies".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 高风险：详细整合计划
                make_agent_node_full(
                    "a-ma-plan",
                    "整合计划",
                    "制定整合计划：组织、系统、流程、文化、时间表。\
                     输出 JSON：{\"integration_plan\":[], }",
                    vec![],
                    Some(CEO),
                    "a-ma-plan",
                    vec![("audit", "a-ma-audit")],
                    vec!["a-ma-audit"],
                    -250.0,
                    540.0,
                ),
                // 低风险：快速整合
                make_agent_node_full(
                    "a-ma-fast",
                    "快速整合",
                    "风险较低，制定快速整合方案。\
                     输出 JSON：{\"fast_track\":[], }",
                    vec![],
                    Some(CEO),
                    "a-ma-fast",
                    vec![("audit", "a-ma-audit")],
                    vec!["a-ma-audit"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-ma", "汇合", 0.0, 720.0),
                // Agent：整合执行
                make_agent_node_full(
                    "a-ma-exec",
                    "整合执行",
                    "执行整合：里程碑跟踪、问题处理、协同价值兑现。\
                     输出 JSON：{\"progress\":0, }",
                    vec![],
                    Some(CEO),
                    "a-ma-exec",
                    vec![("plan", "a-ma-plan"), ("fast", "a-ma-fast")],
                    vec!["a-ma-plan", "a-ma-fast"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-audit", "trigger", "a-ma-audit"),
                edge("e-audit-risk", "a-ma-audit", "c-ma-risk"),
                edge_cond("e-high-plan", "c-ma-risk", "true", "a-ma-plan", EdgeType::ConditionTrue),
                edge_cond(
                    "e-low-fast",
                    "c-ma-risk",
                    "false",
                    "a-ma-fast",
                    EdgeType::ConditionFalse,
                ),
                edge("e-plan-merge", "a-ma-plan", "m-ma"),
                edge("e-fast-merge", "a-ma-fast", "m-ma"),
                edge("e-merge-exec", "m-ma", "a-ma-exec"),
                edge("e-exec-end", "a-ma-exec", "end"),
            ],
            vec![DomainInputField {
                key: "target_company",
                label: "标的公司",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-onboard: 员工入职 ────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-onboard",
            "员工入职",
            "员工入职：制定入职计划，环境准备不完整自动补齐，执行入职引导",
            "🎉",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：入职计划
                make_agent_node(
                    "a-onboard-plan",
                    "入职计划",
                    "制定入职计划：首日安排、培训课程、导师分配、里程碑。\
                     输出 JSON：{\"day1\":[], \"training\":[], }",
                    vec![],
                    Some(COO),
                    "a-onboard-plan",
                    0.0,
                    180.0,
                ),
                // Agent：环境准备
                make_agent_node_full(
                    "a-onboard-setup",
                    "环境准备",
                    "准备入职环境：账号、设备、权限、工位、资料。\
                     输出 JSON：{\"prepared\":[], \"pending\":[], \"ready\":true}",
                    vec![],
                    Some(COO),
                    "a-onboard-setup",
                    vec![("plan", "a-onboard-plan")],
                    vec!["a-onboard-plan"],
                    0.0,
                    360.0,
                ),
                // 条件：准备完整
                make_condition_node(
                    "c-onboard-ready",
                    "准备判定",
                    vec![Condition {
                        var_path: "a-onboard-setup.ready".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不完整：补准备
                make_agent_node_full(
                    "a-onboard-fill",
                    "补齐准备",
                    "补齐未准备的环境项。\
                     输出 JSON：{\"completed\":[], \"ready\":true}",
                    vec![],
                    Some(COO),
                    "a-onboard-fill",
                    vec![("setup", "a-onboard-setup")],
                    vec!["a-onboard-setup"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-onboard", "汇合", 0.0, 900.0),
                // Agent：入职引导
                make_agent_node_full(
                    "a-onboard-orient",
                    "入职引导",
                    "执行入职引导：欢迎、介绍、培训、首次任务、反馈收集。\
                     输出 JSON：{\"orientation\":[], }",
                    vec![],
                    Some(COO),
                    "a-onboard-orient",
                    vec![("setup", "a-onboard-setup"), ("fill", "a-onboard-fill")],
                    vec!["a-onboard-setup", "a-onboard-fill"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-onboard-plan"),
                edge("e-plan-setup", "a-onboard-plan", "a-onboard-setup"),
                edge("e-setup-ready", "a-onboard-setup", "c-onboard-ready"),
                edge_cond(
                    "e-no-fill",
                    "c-onboard-ready",
                    "false",
                    "a-onboard-fill",
                    EdgeType::ConditionFalse,
                ),
                edge_cond(
                    "e-ok-merge",
                    "c-onboard-ready",
                    "true",
                    "m-onboard",
                    EdgeType::ConditionTrue,
                ),
                edge("e-fill-merge", "a-onboard-fill", "m-onboard"),
                edge("e-merge-orient", "m-onboard", "a-onboard-orient"),
                edge("e-orient-end", "a-onboard-orient", "end"),
            ],
            vec![DomainInputField {
                key: "employee_name",
                label: "员工姓名",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spc-supply-chain: 供应链优化 ─────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spc-supply-chain",
            "供应链优化",
            "供应链优化：审计供应链效率，存在瓶颈重点优化，实施优化方案",
            "📦",
            vec!["opc".to_string(), "specialized".to_string()],
            SPC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：供应链审计
                make_agent_node(
                    "a-sc-audit",
                    "供应链审计",
                    "审计供应链：采购、库存、物流、交付时效、成本结构。\
                     输出 JSON：{\"metrics\":{}, \"bottlenecks\":[{\"area\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索供应链最佳实践")],
                    Some(COO),
                    "a-sc-audit",
                    0.0,
                    180.0,
                ),
                // 条件：存在瓶颈
                make_condition_node(
                    "c-sc-bottleneck",
                    "瓶颈判定",
                    vec![Condition {
                        var_path: "a-sc-audit.bottlenecks".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 有瓶颈：重点优化
                make_agent_node_full(
                    "a-sc-optimize",
                    "重点优化",
                    "针对瓶颈制定优化方案：流程再造、供应商调整、库存策略。\
                     输出 JSON：{\"optimizations\":[{\"bottleneck\":\"\", }",
                    vec![],
                    Some(COO),
                    "a-sc-optimize",
                    vec![("audit", "a-sc-audit")],
                    vec!["a-sc-audit"],
                    -250.0,
                    540.0,
                ),
                // 无瓶颈：常规优化
                make_agent_node_full(
                    "a-sc-tune",
                    "常规优化",
                    "无重大瓶颈，制定持续改进方案。\
                     输出 JSON：{\"improvements\":[], }",
                    vec![],
                    Some(COO),
                    "a-sc-tune",
                    vec![("audit", "a-sc-audit")],
                    vec!["a-sc-audit"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-sc", "汇合", 0.0, 720.0),
                // Agent：实施
                make_agent_node_full(
                    "a-sc-implement",
                    "实施",
                    "实施优化方案：排期、负责人、效果跟踪。\
                     输出 JSON：{\"implementation\":[], }",
                    vec![],
                    Some(COO),
                    "a-sc-implement",
                    vec![("optimize", "a-sc-optimize"), ("tune", "a-sc-tune")],
                    vec!["a-sc-optimize", "a-sc-tune"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-audit", "trigger", "a-sc-audit"),
                edge("e-audit-bottleneck", "a-sc-audit", "c-sc-bottleneck"),
                edge_cond(
                    "e-bottleneck-optimize",
                    "c-sc-bottleneck",
                    "true",
                    "a-sc-optimize",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-clean-tune",
                    "c-sc-bottleneck",
                    "false",
                    "a-sc-tune",
                    EdgeType::ConditionFalse,
                ),
                edge("e-optimize-merge", "a-sc-optimize", "m-sc"),
                edge("e-tune-merge", "a-sc-tune", "m-sc"),
                edge("e-merge-implement", "m-sc", "a-sc-implement"),
                edge("e-implement-end", "a-sc-implement", "end"),
            ],
            vec![DomainInputField {
                key: "chain_scope",
                label: "供应链范围",
                field_type: "string",
                required: false,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
