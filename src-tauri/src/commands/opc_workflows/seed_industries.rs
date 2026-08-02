// SPDX-License-Identifier: AGPL-3.0-only

//! 9 垂直领域行业工作流模板
//!
//! 每个工作流绑定公司角色 profile_id，驱动对应领域的业务流程。
//!
//! Profile ID 格式: opc-{role}-{expert}
//! 例如: opc-cto-cto-ai-engineer (CTO 角色 + AI Engineer 专家)
//!
//! 9 个行业:
//! 1. AI科技    - CTO 视角: 需求→调研→原型→部署
//! 2. 软件开发  - CTO 视角: 需求→架构→编码→测试→部署
//! 3. 金融投资  - CFO 视角: 财务分析→投资建议→执行
//! 4. 销售增长  - CMO 视角: 线索→触达→签约→交付
//! 5. 内容媒体  - CMO 视角: 选题→创作→发布→推广
//! 6. 行业咨询  - CEO 视角: 调研→分析→方案→交付
//! 7. 会计财务  - CFO 视角: 发票→审批→收款→报表
//! 8. 品牌电商  - CPO 视角: 选品→设计→上架→推广
//! 9. 教育培训  - COO 视角: 课程→制作→学员→评估

use axagent_harness::util_fns::now_ts;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

use super::{check_template_version, make_base, upsert_template, OPC_TEMPLATE_VERSION};

pub async fn seed_industry_workflows(db: &DatabaseConnection) -> Result<(), String> {
    seed_ai_research(db).await?;
    seed_software_dev(db).await?;
    seed_finance_investment(db).await?;
    seed_sales_growth(db).await?;
    seed_content_media(db).await?;
    seed_industry_consulting(db).await?;
    seed_accounting(db).await?;
    seed_ecommerce(db).await?;
    seed_education(db).await?;
    Ok(())
}

/// 构建 Agent 节点的辅助函数
fn agent_node(id: &str, title: &str, desc: &str, profile_id: &str, prompt: &str, x: f64, y: f64, output_var: &str) -> WorkflowNode {
    WorkflowNode::Agent(AgentNode {
        base: make_base(id, title, desc, x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.into(),
            context_sources: vec![],
            output_var: output_var.into(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            exposed_tools: vec![],
            output_mode: OutputMode::Json,
            agent_profile_id: Some(profile_id.into()),
            max_tool_rounds: Some(10),
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: Some("opc-worker".to_string()),
            consistency_check: None,
            hallucination_guard: Some(axagent_harness::hallucination_guard::HallucinationGuardConfig {
                enabled: true, match_threshold: 0.4,
            }),
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
            input_mapping: std::collections::HashMap::new(),
        },
    })
}

fn agent_node_with_input(
    id: &str, title: &str, desc: &str, profile_id: &str, prompt: &str,
    x: f64, y: f64, output_var: &str,
    input_mapping: std::collections::HashMap<String, String>,
) -> WorkflowNode {
    let mut node = agent_node(id, title, desc, profile_id, prompt, x, y, output_var);
    if let WorkflowNode::Agent(ref mut a) = node {
        a.config.input_mapping = input_mapping;
    }
    node
}

fn edge(id: &str, src: &str, tgt: &str) -> WorkflowEdge {
    WorkflowEdge { id: id.into(), source: src.into(), source_handle: None, target: tgt.into(), target_handle: None, edge_type: EdgeType::Direct, label: None }
}

fn end_node(id: &str, title: &str, x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: make_base(id, title, "", x, y),
        config: EndNodeConfig { output_var: None },
    })
}

fn trigger_node(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: make_base("trigger", "手动启动", "用户选择后启动工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

// ═══════════════════════════════════════════════════════════════════
// 1. AI 科技与研究 — CTO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_ai_research(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-ai-research";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-req", "需求分析", "分析 AI 研究需求", "opc-cto-cto-ai-engineer",
            "分析 AI 研究需求，明确研究范围、技术路线和评估标准。输出 JSON {requirements, tech_areas, success_criteria}", 250.0, 150.0, "req_result"),
        agent_node_with_input("a-survey", "技术调研", "调研相关 AI 技术方案", "opc-cto-cto-ai-engineer",
            "调研相关 AI 技术方案和最新进展。评估各方案的成熟度、成本和可行性。输出 JSON {solutions, comparison, recommendation}", 250.0, 350.0, "survey_result",
            [("requirements".into(), "a-req.result".into())].into()),
        agent_node_with_input("a-prototype", "原型验证", "搭建原型验证可行性", "opc-cto-cto-ai-engineer",
            "基于调研结果搭建最小原型，验证关键假设。输出 JSON {prototype_status, findings, risks}", 250.0, 550.0, "proto_result",
            [("solution".into(), "a-survey.result.recommendation".into())].into()),
        agent_node_with_input("a-report", "研究报告", "生成 AI 研究报告", "opc-ceo-ceo-business-strategist",
            "生成 AI 研究报告：摘要、技术分析、可行性评估、实施建议。输出 Markdown 格式报告。", 250.0, 750.0, "report_result",
            [("survey".into(), "a-survey.result".into()), ("prototype".into(), "a-prototype.result".into())].into()),
        end_node("end", "完成", 250.0, 950.0),
    ];
    let edges = vec![edge("e1","trigger","a-req"), edge("e2","a-req","a-survey"), edge("e3","a-survey","a-prototype"), edge("e4","a-prototype","a-report"), edge("e5","a-report","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "AI 科技研究报告".into(),
        description: Some("AI 技术与应用研究报告：需求分析 → 技术调研 → 原型验证 → 报告输出。CTO 视角，含防幻觉。".into()),
        icon: "🤖".into(), tags: vec!["ai".into(),"research".into(),"tech".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 2. 软件开发 — CTO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_software_dev(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-software-dev";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-req", "需求分析", "分析软件需求", "opc-cto-cto-ai-engineer", "分析需求，确定功能范围和技术约束。输出 JSON {requirements, scope, constraints}", 250.0, 150.0, "req_result"),
        agent_node_with_input("a-arch", "架构设计", "设计系统架构", "opc-cto-cto-ai-engineer", "设计系统架构，选择技术栈。输出 JSON {architecture, tech_stack, modules}", 250.0, 350.0, "arch_result",
            [("req".into(), "a-req.result".into())].into()),
        agent_node_with_input("a-code", "编码实现", "实现核心功能", "opc-cto-cto-ai-engineer", "按设计实现核心功能模块。输出 JSON {modules_implemented, code_summary, issues}", 250.0, 550.0, "code_result",
            [("arch".into(), "a-arch.result".into())].into()),
        agent_node_with_input("a-test", "测试验证", "编写并执行测试", "opc-cto-cto-ai-engineer", "编写测试用例，执行测试。输出 JSON {test_results, coverage, bugs}", 250.0, 750.0, "test_result",
            [("code".into(), "a-code.result".into())].into()),
        end_node("end", "完成", 250.0, 950.0),
    ];
    let edges = vec![edge("e1","trigger","a-req"), edge("e2","a-req","a-arch"), edge("e3","a-arch","a-code"), edge("e4","a-code","a-test"), edge("e5","a-test","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "软件开发流程".into(),
        description: Some("一人公司软件开发：需求 → 架构 → 编码 → 测试。CTO 视角，含防幻觉。".into()),
        icon: "💻".into(), tags: vec!["dev".into(),"software".into(),"tech".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 3. 金融投资 — CFO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_finance_investment(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-finance-invest";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-report", "财务分析", "生成财务报表", "opc-cfo-cfo-financial-analyst",
            "使用 OpcGetFinancialReport 和 OpcGetDashboard 生成财务状况分析。输出 JSON {revenue, profit, cashflow, risks}", 250.0, 150.0, "report_result"),
        agent_node_with_input("a-advice", "投资建议", "生成投资建议", "opc-cfo-cfo-financial-analyst",
            "基于财务分析生成投资建议：推荐金额、风险等级、建议说明。输出 JSON {recommended_amount, risk_level, advice}", 250.0, 350.0, "advice_result",
            [("report".into(), "a-report.result".into())].into()),
        end_node("end", "完成", 250.0, 550.0),
    ];
    let edges = vec![edge("e1","trigger","a-report"), edge("e2","a-report","a-advice"), edge("e3","a-advice","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "金融投资分析".into(),
        description: Some("财务分析 → 投资建议。CFO 视角，基于实际经营数据生成可执行的财务建议。".into()),
        icon: "📈".into(), tags: vec!["finance".into(),"investment".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 4. 销售增长 — CMO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_sales_growth(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-sales-growth";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-lead", "线索获取", "分析获取潜在线索", "opc-cmo-cmo-content-strategist",
            "使用 OpcListCustomers 分析现有客户数据，识别潜在客户。输出 JSON {leads, insights}", 250.0, 150.0, "lead_result"),
        agent_node_with_input("a-outreach", "触达跟进", "制定触达方案", "opc-cmo-cmo-content-strategist",
            "基于线索信息制定个触达方案。输出 JSON {strategy, channels, timeline}", 250.0, 350.0, "outreach_result",
            [("leads".into(), "a-lead.result".into())].into()),
        agent_node_with_input("a-close", "签约转化", "推动签约转化", "opc-ceo-ceo-business-strategist",
            "推动线索转化签约。使用 OpcCreateCustomer 创建客户记录。输出 JSON {conversions, next_steps}", 250.0, 550.0, "close_result",
            [("leads".into(), "a-lead.result".into())].into()),
        end_node("end", "完成", 250.0, 750.0),
    ];
    let edges = vec![edge("e1","trigger","a-lead"), edge("e2","a-lead","a-outreach"), edge("e3","a-outreach","a-close"), edge("e4","a-close","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "销售增长流程".into(),
        description: Some("线索获取 → 触达跟进 → 签约转化。CMO + CEO 交叉协作。".into()),
        icon: "🚀".into(), tags: vec!["sales".into(),"growth".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 5. 内容与媒体 — CMO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_content_media(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-content-media";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-topic", "选题策划", "确定内容主题", "opc-cmo-cmo-content-strategist",
            "分析市场和客户数据，策划内容主题。输出 JSON {topic, audience, outline}", 250.0, 150.0, "topic_result"),
        agent_node_with_input("a-create", "内容创作", "创作内容", "opc-cmo-cmo-content-strategist",
            "根据选题创作内容。使用 OpcCreateBlogPost 发布博客。输出 JSON {post_id, status, summary}", 250.0, 350.0, "create_result",
            [("topic".into(), "a-topic.result".into())].into()),
        agent_node_with_input("a-landing", "创建落地页", "创建营销落地页", "opc-cmo-cmo-content-strategist",
            "使用 OpcCreateLandingPage 为内容创建落地页。输出 JSON {page_id, url, status}", 250.0, 550.0, "landing_result",
            [("content".into(), "a-create.result".into())].into()),
        end_node("end", "完成", 250.0, 750.0),
    ];
    let edges = vec![edge("e1","trigger","a-topic"), edge("e2","a-topic","a-create"), edge("e3","a-create","a-landing"), edge("e4","a-landing","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "内容营销流程".into(),
        description: Some("选题策划 → 内容创作 → 落地页发布。CMO 视角，含博客和落地页创建。".into()),
        icon: "📝".into(), tags: vec!["content".into(),"media".into(),"marketing".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 6. 行业咨询 — CEO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_industry_consulting(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-industry-consulting";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-research", "行业调研", "调研目标行业", "opc-ceo-ceo-business-strategist",
            "调研行业现状、趋势和竞争格局。输出 JSON {industry_overview, trends, opportunities}", 250.0, 150.0, "research_result"),
        agent_node_with_input("a-analysis", "方案设计", "设计解决方案", "opc-ceo-ceo-business-strategist",
            "基于调研结果设计解决方案。输出 JSON {solution, roadmap, resources}", 250.0, 350.0, "analysis_result",
            [("research".into(), "a-research.result".into())].into()),
        agent_node_with_input("a-deliver", "交付报告", "生成咨询报告", "opc-ceo-ceo-business-strategist",
            "生成咨询报告：发现、建议、行动计划。输出 Markdown 报告。", 250.0, 550.0, "deliver_result",
            [("solution".into(), "a-analysis.result".into())].into()),
        end_node("end", "完成", 250.0, 750.0),
    ];
    let edges = vec![edge("e1","trigger","a-research"), edge("e2","a-research","a-analysis"), edge("e3","a-analysis","a-deliver"), edge("e4","a-deliver","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "行业咨询流程".into(),
        description: Some("行业调研 → 方案设计 → 交付报告。CEO 视角，战略级分析。".into()),
        icon: "💼".into(), tags: vec!["consulting".into(),"strategy".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 7. 会计财务 — CFO 视角（升级版发票审批）
// ═══════════════════════════════════════════════════════════════════

async fn seed_accounting(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-accounting";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-create", "创建发票", "创建发票草稿", "opc-cfo-cfo-financial-analyst",
            "根据用户信息创建发票。使用 OpcCreateInvoice。输出 JSON {invoice_id, number, total}", 250.0, 150.0, "create_result"),
        WorkflowNode::Approval(ApprovalNode {
            base: make_base("approval", "财务审批", "审批发票", 250.0, 350.0),
            config: ApprovalNodeConfig {
                message: "请审批发票。24小时超时自动拒绝。".into(),
                approver: Some("manager".into()), timeout_secs: 86400,
                timeout_action: "auto_reject".into(), output_var: "approval_result".into(),
            },
        }),
        agent_node_with_input("a-notify", "通知客户", "通知客户发票已开具", "opc-cfo-cfo-financial-analyst",
            "发票已审批通过。使用 OpcSendNotification 通知客户。输出 JSON {notification_status}", 450.0, 550.0, "notify_result",
            [("invoice".into(), "a-create.result".into())].into()),
        agent_node_with_input("a-report", "登记报表", "记录 KPI 到报表", "opc-cfo-cfo-financial-analyst",
            "使用 OpcRecordKpi 记录发票相关的关键指标。输出 JSON {kpi_status}", 250.0, 750.0, "report_result",
            [("invoice".into(), "a-create.result".into())].into()),
        end_node("end", "完成", 250.0, 950.0),
    ];
    let edges = vec![
        edge("e1","trigger","a-create"), edge("e2","a-create","approval"),
        WorkflowEdge { id: "e3".into(), source: "approval".into(), source_handle: Some("true".into()), target: "a-notify".into(), target_handle: None, edge_type: EdgeType::ConditionTrue, label: None },
        WorkflowEdge { id: "e4".into(), source: "approval".into(), source_handle: Some("false".into()), target: "end".into(), target_handle: None, edge_type: EdgeType::ConditionFalse, label: None },
        edge("e5","a-notify","a-report"), edge("e6","a-report","end"),
    ];

    let data = WorkflowTemplateData {
        id: id.into(), name: "会计财务流程".into(),
        description: Some("创建发票 → 审批(ApprovalNode) → 通知客户 → 登记KPI。CFO 视角。".into()),
        icon: "🧾".into(), tags: vec!["finance".into(),"accounting".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 8. 品牌电商 — CPO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_ecommerce(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-ecommerce";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-product", "选品分析", "分析选品方向", "opc-cpo-cpo-product-manager",
            "分析市场和客户需求，确定产品方向。输出 JSON {product_ideas, market_fit, pricing}", 250.0, 150.0, "product_result"),
        agent_node_with_input("a-page", "上架落地页", "创建产品落地页", "opc-cpo-cpo-product-manager",
            "使用 OpcCreateLandingPage 创建产品落地页。输出 JSON {page_id, url}", 250.0, 350.0, "page_result",
            [("product".into(), "a-product.result".into())].into()),
        agent_node_with_input("a-customer", "客户管理", "创建客户并关联产品", "opc-coo-coo-operations-manager",
            "使用 OpcCreateCustomer 创建客户记录，关联产品信息。输出 JSON {customer_id, product_ref}", 250.0, 550.0, "customer_result",
            [("page".into(), "a-page.result".into())].into()),
        end_node("end", "完成", 250.0, 750.0),
    ];
    let edges = vec![edge("e1","trigger","a-product"), edge("e2","a-product","a-page"), edge("e3","a-page","a-customer"), edge("e4","a-customer","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "电商运营流程".into(),
        description: Some("选品分析 → 上架落地页 → 客户管理。CPO + COO 交叉协作。".into()),
        icon: "🛍️".into(), tags: vec!["ecommerce".into(),"product".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// 9. 教育培训 — COO 视角
// ═══════════════════════════════════════════════════════════════════

async fn seed_education(db: &DatabaseConnection) -> Result<(), String> {
    let id = "workflow-education";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? { return Ok(()); }
    let now = now_ts();

    let nodes = vec![
        trigger_node(250.0, 0.0),
        agent_node("a-curriculum", "课程设计", "设计课程大纲", "opc-coo-coo-operations-manager",
            "设计课程目标、大纲和评估方式。输出 JSON {title, objectives, modules, duration}", 250.0, 150.0, "curriculum_result"),
        agent_node_with_input("a-content", "内容制作", "制作课程内容", "opc-coo-coo-operations-manager",
            "制作课程内容讲义、练习和测验。输出 JSON {content_summary, modules_completed}", 250.0, 350.0, "content_result",
            [("curriculum".into(), "a-curriculum.result".into())].into()),
        agent_node_with_input("a-enroll", "学员管理", "创建学员记录", "opc-cmo-cmo-content-strategist",
            "使用 OpcCreateCustomer 创建学员信息，设置学习路径。输出 JSON {enrollment_id, student_count}", 250.0, 550.0, "enroll_result",
            [("course".into(), "a-content.result".into())].into()),
        end_node("end", "完成", 250.0, 750.0),
    ];
    let edges = vec![edge("e1","trigger","a-curriculum"), edge("e2","a-curriculum","a-content"), edge("e3","a-content","a-enroll"), edge("e4","a-enroll","end")];

    let data = WorkflowTemplateData {
        id: id.into(), name: "教育培训流程".into(),
        description: Some("课程设计 → 内容制作 → 学员管理。COO + CMO 协作。".into()),
        icon: "🎓".into(), tags: vec!["education".into(),"training".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![], error_config: None,
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}
