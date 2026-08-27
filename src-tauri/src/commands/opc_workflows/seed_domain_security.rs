// SPDX-License-Identifier: AGPL-3.0-only

//! 信息安全（security）领域工作流种子化 — 4 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-sec-compliance:  合规审计（标准 → 审计 → 不合规整改分支 → 报告）
//! - wf-sec-incident:    安全事件（检测 → 严重度分支 → 紧急/标准响应 → 复盘）
//! - wf-sec-pentest:     渗透测试（范围 → 授权合规分支 → 执行 → 报告）
//! - wf-sec-threat-intel:威胁情报（收集 → 分析 → 高危行动分支）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cto-cto-ai-engineer";
/// security 领域模板版本（v4 丰富拓扑）
const SEC_TEMPLATE_VERSION: i32 = 4;

/// 种子化信息安全领域的全部工作流
pub(crate) async fn seed_domain_security_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-sec-compliance: 合规审计 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sec-compliance",
            "合规审计",
            "合规审计：定义合规标准，审计现状差距，不合规自动生成整改计划",
            "📋",
            vec!["opc".to_string(), "security".to_string()],
            SEC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：标准定义
                make_agent_node(
                    "a-comp-standard",
                    "标准定义",
                    "定义适用合规标准：法规、行业规范、内部制度，明确审计项。\
                     输出 JSON：{\"frameworks\":[], \"audit_items\":[{\"id\":\"\", \"requirement\":\"\", \"evidence\":\"\"}], \"scope\":\"\"}",
                    vec![td_desc("OpcSearchWiki", "检索合规标准要求")],
                    Some(PROFILE),
                    "a-comp-standard",
                    0.0,
                    180.0,
                ),
                // Agent：合规审计
                make_agent_node_full(
                    "a-comp-audit",
                    "审计",
                    "逐项审计合规现状：检查控制措施、收集证据、评估差距。\
                     输出 JSON：{\"results\":[{\"item\":\"\", \"status\":\"compliant|non_compliant|partial\", \"evidence\":\"\", \"gap\":\"\"}], }",
                    vec![],
                    Some(PROFILE),
                    "a-comp-audit",
                    vec![("standard", "a-comp-standard")],
                    vec!["a-comp-standard"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在不合规项
                make_condition_node(
                    "c-comp-findings",
                    "不合规判定",
                    vec![Condition {
                        var_path: "a-comp-audit.results".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 整改分支
                make_agent_node_full(
                    "a-comp-remediate",
                    "整改计划",
                    "为不合规项制定整改计划：措施、责任人、期限、验证方式。\
                     输出 JSON：{\"remediation\":[{\"item\":\"\", \"action\":\"\", \"owner\":\"\", \"due\":\"\", \"verify\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "通知合规整改责任人")],
                    Some(PROFILE),
                    "a-comp-remediate",
                    vec![("audit", "a-comp-audit")],
                    vec!["a-comp-audit"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-comp", "汇合", 0.0, 900.0),
                // Agent：审计报告
                make_agent_node_full(
                    "a-comp-report",
                    "报告",
                    "输出合规审计报告：符合率、发现项、风险等级与整改建议。\
                     输出 JSON：{\"compliance_rate\":0, \"findings\":[], \"risk_level\":\"\", \"recommendations\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-comp-report",
                    vec![("audit", "a-comp-audit"), ("remediate", "a-comp-remediate")],
                    vec!["a-comp-audit", "a-comp-remediate"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-standard", "trigger", "a-comp-standard"),
                edge("e-standard-audit", "a-comp-standard", "a-comp-audit"),
                edge("e-audit-findings", "a-comp-audit", "c-comp-findings"),
                edge_cond(
                    "e-findings-remediate",
                    "c-comp-findings",
                    "true",
                    "a-comp-remediate",
                    EdgeType::ConditionTrue,
                ),
                edge_cond("e-clean-merge", "c-comp-findings", "false", "m-comp", EdgeType::ConditionFalse),
                edge("e-remediate-merge", "a-comp-remediate", "m-comp"),
                edge("e-merge-report", "m-comp", "a-comp-report"),
                edge("e-report-end", "a-comp-report", "end"),
            ],
            vec![DomainInputField { key: "standard_name", label: "合规标准", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-sec-incident: 安全事件 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sec-incident",
            "安全事件",
            "安全事件：检测事件并定级，严重事件紧急响应，常规事件标准处理，事后复盘",
            "🚨",
            vec!["opc".to_string(), "security".to_string()],
            SEC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：事件检测
                make_agent_node(
                    "a-incident-detect",
                    "事件检测",
                    "检测并评估安全事件：影响范围、数据暴露、传播途径、严重级别。\
                     输出 JSON：{\"event\":\"\", \"severity\":\"critical|high|medium|low\", \"scope\":[], \"indicators\":[], \"evidence\":[]}",
                    vec![td_desc("OpcSearchWiki", "检索事件特征库")],
                    Some(PROFILE),
                    "a-incident-detect",
                    0.0,
                    180.0,
                ),
                // 条件：严重度判定
                make_condition_node(
                    "c-incident-critical",
                    "严重度判定",
                    vec![
                        Condition {
                            var_path: "a-incident-detect.severity".to_string(),
                            operator: CompareOperator::Eq,
                            value: serde_json::json!("critical"),
                        },
                        Condition {
                            var_path: "a-incident-detect.severity".to_string(),
                            operator: CompareOperator::Eq,
                            value: serde_json::json!("high"),
                        },
                    ],
                    LogicalOperator::Or,
                    0.0,
                    360.0,
                ),
                // 紧急响应
                make_agent_node_full(
                    "a-incident-emergency",
                    "紧急响应",
                    "启动紧急响应：隔离受感染系统、遏制扩散、通知管理层、应急处置。\
                     输出 JSON：{\"containment\":[], \"notifications\":[], \"actions\":[{\"step\":\"\", \"status\":\"done|pending\", \"owner\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "紧急通知管理层与安全团队")],
                    Some(PROFILE),
                    "a-incident-emergency",
                    vec![("detect", "a-incident-detect")],
                    vec!["a-incident-detect"],
                    -250.0,
                    540.0,
                ),
                // 标准响应
                make_agent_node_full(
                    "a-incident-respond",
                    "标准响应",
                    "执行标准事件响应流程：取证、清除、恢复、加固。\
                     输出 JSON：{\"steps\":[{\"phase\":\"\", \"actions\":[], \"owner\":\"\"}], }",
                    vec![],
                    Some(PROFILE),
                    "a-incident-respond",
                    vec![("detect", "a-incident-detect")],
                    vec!["a-incident-detect"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-incident", "汇合", 0.0, 720.0),
                // Agent：复盘
                make_agent_node_full(
                    "a-incident-review",
                    "复盘",
                    "事件复盘：时间线、根因、处置效果、改进项与预防措施。\
                     输出 JSON：{\"timeline\":[], \"root_cause\":\"\", \"improvements\":[], \"prevention\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-incident-review",
                    vec![("emergency", "a-incident-emergency"), ("respond", "a-incident-respond")],
                    vec!["a-incident-emergency", "a-incident-respond"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-detect", "trigger", "a-incident-detect"),
                edge("e-detect-critical", "a-incident-detect", "c-incident-critical"),
                edge_cond(
                    "e-critical-emergency",
                    "c-incident-critical",
                    "true",
                    "a-incident-emergency",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-standard-respond",
                    "c-incident-critical",
                    "false",
                    "a-incident-respond",
                    EdgeType::ConditionFalse,
                ),
                edge("e-emergency-merge", "a-incident-emergency", "m-incident"),
                edge("e-respond-merge", "a-incident-respond", "m-incident"),
                edge("e-merge-review", "m-incident", "a-incident-review"),
                edge("e-review-end", "a-incident-review", "end"),
            ],
            vec![DomainInputField { key: "incident_desc", label: "事件描述", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-sec-pentest: 渗透测试 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sec-pentest",
            "渗透测试",
            "渗透测试：定义测试范围，授权不合规自动补充，执行测试并输出漏洞报告",
            "🛡️",
            vec!["opc".to_string(), "security".to_string()],
            SEC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：范围定义
                make_agent_node(
                    "a-pentest-scope",
                    "范围定义",
                    "定义渗透测试范围：目标系统、测试类型、时间窗、授权状态。\
                     输出 JSON：{\"targets\":[], \"test_types\":[], \"window\":\"\", \"authorized\":true, }",
                    vec![td_desc("OpcSearchWiki", "检索测试方法论")],
                    Some(PROFILE),
                    "a-pentest-scope",
                    0.0,
                    180.0,
                ),
                // 条件：授权合规
                make_condition_node(
                    "c-pentest-auth",
                    "授权判定",
                    vec![Condition {
                        var_path: "a-pentest-scope.authorized".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 授权通过：执行测试
                make_agent_node_full(
                    "a-pentest-exec",
                    "执行",
                    "执行渗透测试：信息收集、漏洞探测、利用验证、痕迹清理。\
                     输出 JSON：{\"findings\":[{\"vuln\":\"\", \"severity\":\"critical|high|medium|low\", \"exploitability\":0, \"impact\":\"\", \"evidence\":\"\"}], }",
                    vec![],
                    Some(PROFILE),
                    "a-pentest-exec",
                    vec![("scope", "a-pentest-scope")],
                    vec!["a-pentest-scope"],
                    -250.0,
                    540.0,
                ),
                // 未授权：授权补充
                make_agent_node_full(
                    "a-pentest-authorize",
                    "授权补充",
                    "测试未获授权，列出所需授权材料与获取流程，暂停测试。\
                     输出 JSON：{\"required_authorizations\":[], }",
                    vec![td_desc("OpcSendNotification", "通知授权审批人")],
                    Some(PROFILE),
                    "a-pentest-authorize",
                    vec![("scope", "a-pentest-scope")],
                    vec!["a-pentest-scope"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-pentest", "汇合", 0.0, 720.0),
                // Agent：漏洞报告
                make_agent_node_full(
                    "a-pentest-report",
                    "报告",
                    "输出渗透测试报告：漏洞清单、风险评估、修复建议与复测计划。\
                     输出 JSON：{\"vulnerabilities\":[], \"risk_summary\":\"\", \"fix_recommendations\":[], \"retest_plan\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-pentest-report",
                    vec![("exec", "a-pentest-exec"), ("authorize", "a-pentest-authorize")],
                    vec!["a-pentest-exec", "a-pentest-authorize"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-scope", "trigger", "a-pentest-scope"),
                edge("e-scope-auth", "a-pentest-scope", "c-pentest-auth"),
                edge_cond("e-authorized-exec", "c-pentest-auth", "true", "a-pentest-exec", EdgeType::ConditionTrue),
                edge_cond(
                    "e-unauthorized-authorize",
                    "c-pentest-auth",
                    "false",
                    "a-pentest-authorize",
                    EdgeType::ConditionFalse,
                ),
                edge("e-exec-merge", "a-pentest-exec", "m-pentest"),
                edge("e-authorize-merge", "a-pentest-authorize", "m-pentest"),
                edge("e-merge-report", "m-pentest", "a-pentest-report"),
                edge("e-report-end", "a-pentest-report", "end"),
            ],
            vec![DomainInputField { key: "target_system", label: "目标系统", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-sec-threat-intel: 威胁情报 ───────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-sec-threat-intel",
            "威胁情报",
            "威胁情报：收集威胁情报，分析风险等级，高危威胁自动生成处置行动",
            "🕵️",
            vec!["opc".to_string(), "security".to_string()],
            SEC_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // 工具：情报收集
                make_tool_node(
                    "t-threat-collect",
                    "情报收集",
                    "OpcSearchWiki",
                    vec![("user_input", "trigger")],
                    "t-threat-collect",
                    0.0,
                    180.0,
                ),
                // Agent：威胁分析
                make_agent_node_full(
                    "a-threat-analyze",
                    "分析",
                    "分析威胁情报：攻击者画像、攻击向量、影响资产、风险等级。\
                     输出 JSON：{\"threats\":[{\"name\":\"\", \"actor\":\"\", \"vector\":\"\", \"affected\":[], \"risk\":\"high|medium|low\"}], }",
                    vec![td_desc("OpcSearchWiki", "检索威胁情报")],
                    Some(PROFILE),
                    "a-threat-analyze",
                    vec![("collect", "t-threat-collect.result")],
                    vec!["t-threat-collect"],
                    0.0,
                    360.0,
                ),
                // 条件：存在高危威胁
                make_condition_node(
                    "c-threat-high",
                    "高危判定",
                    vec![Condition {
                        var_path: "a-threat-analyze.threats".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 高危：处置行动
                make_agent_node_full(
                    "a-threat-act",
                    "行动",
                    "针对高危威胁制定处置行动：加固措施、检测规则、应急预案。\
                     输出 JSON：{\"actions\":[{\"threat\":\"\", \"action\":\"\", }",
                    vec![td_desc("OpcSendNotification", "通知安全运营团队高危威胁")],
                    Some(PROFILE),
                    "a-threat-act",
                    vec![("analyze", "a-threat-analyze")],
                    vec!["a-threat-analyze"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-threat", "汇合", 0.0, 900.0),
                // Agent：情报简报
                make_agent_node_full(
                    "a-threat-brief",
                    "情报简报",
                    "输出威胁情报简报：态势总结、关注清单、行动建议。\
                     输出 JSON：{\"summary\":\"\", \"watchlist\":[], \"recommendations\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-threat-brief",
                    vec![("analyze", "a-threat-analyze"), ("act", "a-threat-act")],
                    vec!["a-threat-analyze", "a-threat-act"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "t-threat-collect"),
                edge("e-collect-analyze", "t-threat-collect", "a-threat-analyze"),
                edge("e-analyze-high", "a-threat-analyze", "c-threat-high"),
                edge_cond("e-high-act", "c-threat-high", "true", "a-threat-act", EdgeType::ConditionTrue),
                edge_cond("e-low-merge", "c-threat-high", "false", "m-threat", EdgeType::ConditionFalse),
                edge("e-act-merge", "a-threat-act", "m-threat"),
                edge("e-merge-brief", "m-threat", "a-threat-brief"),
                edge("e-brief-end", "a-threat-brief", "end"),
            ],
            vec![DomainInputField { key: "intel_topic", label: "情报主题", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
