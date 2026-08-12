// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 需求发现工作流模板 — 持久化到 workflow_template 表
//!
//! 参考 stock_analysis_setup::seed_stock_analysis_workflow_template 模式
//!
//! 工作流结构（与股票分析一致）：
//!   Trigger → [ParallelNode(p-analysts)] → CEO 决策 → End
//!
//! 每个分析师配对一个 (Tool + Agent) 节点：
//!   ToolNode 在左列获取数据，AgentNode 在右列分析
//!
//! Agent 节点绑定 agent_profile_id 实现三要素：
//!   1. AgentRole.system_prompt      → 岗位/角色定义
//!   2. AgencyExpert.system_prompt   → 专家专业提示词
//!   3. AgentNodeConfig.system_prompt → 节点级任务指令（inline 嵌入）

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::opc_setup;
use axagent_entities::workflow_template;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, BackoffType, Branch, DegradeStrategy, EdgeType, EndNode,
    EndNodeConfig, JsonSchema, JsonSchemaProperty, MergeStrategy, NotificationNode,
    NotificationNodeConfig, OutputMode, ParallelNode, ParallelNodeConfig, Position, RetryConfig,
    ToolDef, ToolNode, ToolNodeConfig, TriggerConfig, TriggerNode, TriggerType, WorkflowEdge,
    WorkflowNode, WorkflowNodeBase,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

const TEMPLATE_ID: &str = "opc-demand-discovery";

// V3(2026-08-13): 与股票分析工作流完全对齐 — 单一 ParallelNode 容器、(Tool+Agent) 配对布局
const TEMPLATE_VERSION: i32 = 3;

/// Profile → 工具白名单（与 mod.rs 中的 PROFILE_TOOLS 保持一致）
const PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "ceo-business-strategist",
        &[
            "OpcGetDashboard",
            "OpcListKpis",
            "OpcListInvoices",
            "OpcListCustomers",
            "OpcListProjects",
            "OpcSearchWiki",
        ],
    ),
    ("cto-ai-engineer", &["OpcListProjects", "OpcListKpis", "OpcSearchWiki"]),
    (
        "cfo-financial-analyst",
        &["OpcListInvoices", "OpcListCustomers", "OpcGetDashboard", "OpcListKpis", "OpcSearchWiki"],
    ),
    (
        "coo-operations-manager",
        &[
            "OpcListProjects",
            "OpcListCustomers",
            "OpcListInvoices",
            "OpcGetDashboard",
            "OpcSearchWiki",
        ],
    ),
    (
        "cmo-content-strategist",
        &["OpcListCustomers", "OpcListBlogPosts", "OpcGetDashboard", "OpcSearchWiki"],
    ),
    ("cpo-product-manager", &["OpcListProjects", "OpcListCustomers", "OpcSearchWiki"]),
];

/// 种子化 OPC 需求发现工作流模板到数据库
pub(crate) async fn seed_opc_workflow_template(db: &DatabaseConnection) -> Result<(), String> {
    // 升级前保留旧模板的变量自定义值
    let mut old_variables: Option<String> = None;

    if let Some(existing) =
        workflow_template::Entity::find_by_id(TEMPLATE_ID).one(db).await.map_err(|e| {
            ErrorResponse::new(opc_setup::INTERNAL).with_detail(format!("查询工作流模板失败: {e}"))
        })?
    {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[opc_setup] 模板已是最新版本 v{}，跳过种子化以保留用户修改",
                existing.version
            );
            return Ok(());
        }
        tracing::info!(
            "[opc_setup] 更新需求发现工作流模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );

        // 写版本快照
        let ver_id = format!("{}_v{}", TEMPLATE_ID, existing.version);
        if axagent_entities::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(opc_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
            .is_none()
        {
            let snapshot = axagent_entities::workflow_template_version::ActiveModel {
                id: Set(ver_id.clone()),
                template_id: Set(TEMPLATE_ID.to_string()),
                name: Set(existing.name.clone()),
                description: Set(existing.description.clone()),
                icon: Set(existing.icon.clone()),
                tags: Set(existing.tags.clone()),
                version: Set(existing.version),
                is_preset: Set(existing.is_preset),
                is_editable: Set(existing.is_editable),
                is_public: Set(existing.is_public),
                trigger_config: Set(existing.trigger_config.clone()),
                nodes: Set(existing.nodes.clone()),
                edges: Set(existing.edges.clone()),
                input_schema: Set(existing.input_schema.clone()),
                output_schema: Set(existing.output_schema.clone()),
                variables: Set(existing.variables.clone()),
                error_config: Set(existing.error_config.clone()),
                created_at: Set(chrono::Utc::now().timestamp_millis()),
            };
            snapshot.insert(db).await.map_err(|e| {
                ErrorResponse::new(opc_setup::INTERNAL)
                    .with_detail(format!("写入版本快照失败: {e}"))
            })?;
            tracing::info!("[opc_setup] 旧版本快照已保存: {ver_id}");
        }
        old_variables = existing.variables.clone();
    }

    let now = chrono::Utc::now().timestamp_millis();

    // ── 辅助函数 ──

    // ToolNode 创建器（与股票分析一致：带 parent_id 和网格布局支持）
    let tool_node = |id: &str,
                     title: &str,
                     tool_name: &str,
                     output_var: &str,
                     arg_key: &str,
                     parent_id: Option<&str>,
                     x: f64,
                     y: f64|
     -> WorkflowNode {
        let mut input_mapping = std::collections::HashMap::new();
        input_mapping.insert(arg_key.to_string(), "all".to_string());
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("获取数据: {tool_name}")),
                position: Position { x, y },
                retry: RetryConfig {
                    enabled: true,
                    max_retries: 2,
                    base_delay_ms: 1000,
                    max_delay_ms: 10000,
                    backoff_type: BackoffType::Exponential,
                },
                timeout: None,
                enabled: true,
                parent_id: parent_id.map(String::from),
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: tool_name.into(),
                input_mapping,
                output_var: output_var.into(),
            },
        })
    };

    // AgentNode 创建器（与股票分析一致：完善配置）
    let agent = |id: &str,
                 title: &str,
                 expert_id: &str,
                 parent_id: Option<&str>,
                 x: f64,
                 y: f64|
     -> WorkflowNode {
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("需求发现: {expert_id}")),
                position: Position { x, y },
                retry: RetryConfig {
                    enabled: true,
                    max_retries: 3,
                    base_delay_ms: 3000,
                    max_delay_ms: 60000,
                    backoff_type: BackoffType::Exponential,
                },
                timeout: None,
                enabled: true,
                parent_id: parent_id.map(String::from),
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: String::new(),
                context_sources: vec![],
                input_mapping: std::collections::HashMap::new(),
                output_var: id.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(32768),
                tools: vec![],
                exposed_tools: vec![],
                output_mode: OutputMode::Text,
                agent_profile_id: Some(format!("opc-{expert_id}")),
                max_tool_rounds: Some(2),
                execution_mode: None,
                rag_source_ids: Vec::new(),
                model_role: None,
                consistency_check: None,
                hallucination_guard: None,
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: Some(300),
            },
        })
    };

    let edge = |id: &str, source: &str, target: &str| -> WorkflowEdge {
        WorkflowEdge {
            id: id.into(),
            source: source.into(),
            source_handle: None,
            target: target.into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        }
    };

    // 从 ToolDef 列表生成工具 prompt 片段
    fn tool_prompt(tools: &[ToolDef]) -> String {
        if tools.is_empty() {
            return String::new();
        }
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        format!(
            "\n\n你可以调用以下工具获取最新数据：{}。请先调用相关工具获取数据，再基于返回结果进行分析。",
            names.join("、")
        )
    }

    // ── 工具定义 ──
    let mut tool_defs: Vec<ToolDef> = Vec::new();

    // OpcListProjects
    tool_defs.push(ToolDef {
        name: "OpcListProjects".into(),
        description: Some("获取现有项目列表：状态、进度、客户".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "status".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("项目状态: active/completed/on_hold".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: None,
            items: None,
        }),
    });

    // OpcListCustomers
    tool_defs.push(ToolDef {
        name: "OpcListCustomers".into(),
        description: Some("获取客户列表：状态、价值、来源".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "status".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("客户状态: active/potential/inactive".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: None,
            items: None,
        }),
    });

    // OpcListInvoices
    tool_defs.push(ToolDef {
        name: "OpcListInvoices".into(),
        description: Some("获取发票列表：金额、状态、逾期".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "status".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("发票状态: paid/pending/overdue".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: None,
            items: None,
        }),
    });

    // OpcGetDashboard
    tool_defs.push(ToolDef {
        name: "OpcGetDashboard".into(),
        description: Some("获取经营仪表盘：收入、成本、利润、关键指标".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::new()),
            required: None,
            items: None,
        }),
    });

    // OpcListBlogPosts
    tool_defs.push(ToolDef {
        name: "OpcListBlogPosts".into(),
        description: Some("获取博客文章列表：阅读量、互动、转化".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::new()),
            required: None,
            items: None,
        }),
    });

    // OpcSearchWiki
    tool_defs.push(ToolDef {
        name: "OpcSearchWiki".into(),
        description: Some("搜索内部知识库：方案、文档、经验".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "query".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("搜索关键词".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: Some(vec!["query".into()]),
            items: None,
        }),
    });

    // OpcSendNotification
    tool_defs.push(ToolDef {
        name: "OpcSendNotification".into(),
        description: Some("发送内部通知：决策提醒、任务分配".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "message".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("通知内容".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props.insert(
                    "priority".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("优先级: low/medium/high".into()),
                        default: Some(serde_json::Value::String("medium".into())),
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: Some(vec!["message".into()]),
            items: None,
        }),
    });

    // OpcListKpis
    tool_defs.push(ToolDef {
        name: "OpcListKpis".into(),
        description: Some("获取关键绩效指标：目标、实际、趋势".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::new()),
            required: None,
            items: None,
        }),
    });

    // 工具名 → ToolDef 映射
    let tool_def_map: std::collections::HashMap<&str, ToolDef> = [
        ("OpcListProjects", tool_defs[0].clone()),
        ("OpcListCustomers", tool_defs[1].clone()),
        ("OpcListInvoices", tool_defs[2].clone()),
        ("OpcGetDashboard", tool_defs[3].clone()),
        ("OpcListBlogPosts", tool_defs[4].clone()),
        ("OpcSearchWiki", tool_defs[5].clone()),
        ("OpcSendNotification", tool_defs[6].clone()),
        ("OpcListKpis", tool_defs[7].clone()),
    ]
    .into_iter()
    .collect();

    // ── 构建节点 ──
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    // Trigger
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".into(),
            title: "开始需求发现".into(),
            description: Some("手动触发需求发现流程".into()),
            position: Position { x: 520.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({"description": "{{demand_description}}"}),
        },
    }));

    // ═══════════════════════════════════════════════════════════════════════
    // 【装饰节点 / Decorative Container】p-analysts
    // ═══════════════════════════════════════════════════════════════════════
    // 语义：视觉分组容器，包裹 6 组 (Tool + Agent) 子节点
    // 调度：容器本身在引擎中立即 Completed（不参与流程控制）
    //      - wait_for_all=true, aggregation=All: 等所有子节点完成后聚合
    //      - auto_input_from_parent=false: 不自动从父节点拉数据
    //      - 实际依赖通过显式 edge 表达
    //
    // 与股票分析一致：3×N 网格布局，Tool 在左列，Agent 在右列
    // ═══════════════════════════════════════════════════════════════════════

    // 分析师配置：(ToolID, ToolTitle, ToolName, ArgKey, AnalystID, AnalystTitle, ExpertID)
    let analyst_configs: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
        (
            "t-customers-data",
            "获取客户数据",
            "OpcListCustomers",
            "status",
            "a-cmo-analysis",
            "营销增长分析",
            "cmo-content-strategist",
        ),
        (
            "t-projects-data",
            "获取项目数据",
            "OpcListProjects",
            "status",
            "a-cpo-analysis",
            "产品需求分析",
            "cpo-product-manager",
        ),
        (
            "t-tech-data",
            "获取技术数据",
            "OpcListProjects",
            "status",
            "a-cto-analysis",
            "技术可行性评估",
            "cto-ai-engineer",
        ),
        (
            "t-finance-data",
            "获取财务数据",
            "OpcListInvoices",
            "status",
            "a-cfo-analysis",
            "财务可行性评估",
            "cfo-financial-analyst",
        ),
        (
            "t-ops-data",
            "获取运营数据",
            "OpcListProjects",
            "status",
            "a-coo-analysis",
            "运营可行性评估",
            "coo-operations-manager",
        ),
        (
            "t-dashboard-data",
            "获取仪表盘",
            "OpcGetDashboard",
            "",
            "a-ceo-analysis",
            "综合分析",
            "ceo-business-strategist",
        ),
    ];

    // 分析师提示词
    let analyst_prompts: &[(&str, &str)] = &[
        (
            "cmo-content-strategist",
            "你的任务：分析当前市场机会和增长方向。\n\n\
重要原则：\n\
1. 基于现有客户数据和内容表现，识别市场趋势\n\
2. 分析客户画像，识别未满足的需求\n\
3. 评估现有内容渠道的ROI，给出优化建议\n\
4. 输出结构化的增长分析报告\n\n\
输出格式：\n\
- 市场趋势判断\n\
- 目标客户画像\n\
- 增长机会点\n\
- 优先级建议（P0/P1/P2）",
        ),
        (
            "cpo-product-manager",
            "你的任务：分析产品方向和需求优先级。\n\n\
重要原则：\n\
1. 基于现有项目进度和客户反馈，识别产品缺口\n\
2. 分析用户故事和使用场景，提炼核心需求\n\
3. 评估需求优先级（价值/成本/风险）\n\
4. 输出产品路线图建议\n\n\
输出格式：\n\
- 需求清单（用户故事）\n\
- 优先级矩阵\n\
- MVP 范围建议\n\
- 技术依赖关系",
        ),
        (
            "cto-ai-engineer",
            "你的任务：评估技术实现可行性和风险。\n\n\
重要原则：\n\
1. 分析现有技术栈和项目依赖\n\
2. 评估新技术需求的实现复杂度\n\
3. 识别技术风险和约束\n\
4. 给出技术方案建议\n\n\
输出格式：\n\
- 技术可行性评估\n\
- 实现路径建议\n\
- 风险清单\n\
- 工时估算",
        ),
        (
            "cfo-financial-analyst",
            "你的任务：评估财务可行性和投资回报。\n\n\
重要原则：\n\
1. 基于现有财务数据和经营状况，评估资金可用性\n\
2. 估算新项目的收入预测和成本结构\n\
3. 计算关键财务指标（ROI、回收期、盈亏平衡点）\n\
4. 给出财务决策建议\n\n\
输出格式：\n\
- 财务可行性评估\n\
- 投资回报分析\n\
- 现金流预测\n\
- 财务风险清单",
        ),
        (
            "coo-operations-manager",
            "你的任务：评估运营可行性和交付能力。\n\n\
重要原则：\n\
1. 分析现有项目负载和资源占用\n\
2. 评估新项目对运营资源的影响\n\
3. 识别运营瓶颈和风险\n\
4. 给出运营调整建议\n\n\
输出格式：\n\
- 运营资源评估\n\
- 交付能力分析\n\
- 运营风险清单\n\
- 资源调整建议",
        ),
        (
            "ceo-business-strategist",
            "你的任务：综合所有维度分析结果，给出战略建议。\n\n\
重要原则：\n\
1. 汇总各维度分析结论（营销/产品/技术/财务/运营）\n\
2. 识别各维度的关键洞察和矛盾点\n\
3. 从战略层面评估需求的商业价值\n\
4. 给出优先级建议和资源分配方案\n\n\
输出格式：\n\
- 战略价值评估\n\
- 关键成功因素\n\
- 资源需求清单\n\
- 优先级建议",
        ),
    ];

    // model_role 映射
    let analyst_model_roles: &[(&str, &str)] = &[
        ("a-cmo-analysis", "cmo"),
        ("a-cpo-analysis", "cpo"),
        ("a-cto-analysis", "cto"),
        ("a-cfo-analysis", "cfo"),
        ("a-coo-analysis", "coo"),
        ("a-ceo-analysis", "ceo"),
    ];

    // ── Phase 1: p-analysts 容器 — 6 组 (Tool + Agent) ──
    // 3×2 网格布局（与股票分析一致）
    //   col_x = [40, 520, 1000]
    //   Tool x = col_x[col], Agent x = col_x[col] + 240
    //   row_y = 100 + row * 180
    let col_x = [40.0_f64, 520.0, 1000.0];
    let row_y_base = 100.0;
    let row_dy = 180.0;

    let mut analyst_branches: Vec<Branch> = Vec::with_capacity(analyst_configs.len());
    for (i, (tool_id, tool_title, tool_name, arg_key, analyst_id, analyst_title, expert_id)) in
        analyst_configs.iter().enumerate()
    {
        let col = i % 3;
        let row = i / 3;
        let x_tool = col_x[col];
        let y = row_y_base + row as f64 * row_dy;

        // 创建 ToolNode（左列）
        nodes.push(tool_node(
            tool_id,
            tool_title,
            tool_name,
            tool_id,
            arg_key,
            Some("p-analysts"),
            x_tool,
            y,
        ));
        edges.push(edge(&format!("e-trigger-{tool_id}"), "trigger", tool_id));

        // 创建 AgentNode（右列）
        let x_agent = col_x[col] + 240.0;
        let mut an = agent(analyst_id, analyst_title, expert_id, Some("p-analysts"), x_agent, y);

        if let WorkflowNode::Agent(ref mut a) = an {
            // 设置 context_sources（对应 ToolNode 的输出）
            a.config.context_sources = vec![tool_id.to_string()];

            // 设置 model_role
            a.config.model_role = analyst_model_roles
                .iter()
                .find(|(aid, _)| aid == analyst_id)
                .map(|(_, role)| role.to_string());

            // 设置 tools 和 exposed_tools（从 PROFILE_TOOLS 获取）
            let profile_tools = PROFILE_TOOLS
                .iter()
                .find(|(aid, _)| aid == expert_id)
                .map(|(_, tools)| tools.to_vec())
                .unwrap_or_default();
            a.config.tools =
                profile_tools.iter().filter_map(|tn| tool_def_map.get(tn).cloned()).collect();
            a.config.exposed_tools = profile_tools.iter().map(|s| s.to_string()).collect();

            // 设置 system_prompt
            let base_prompt = analyst_prompts
                .iter()
                .find(|(aid, _)| aid == expert_id)
                .map(|(_, prompt)| prompt.to_string())
                .unwrap_or_else(|| "分析需求并提供建议".to_string());

            a.config.system_prompt = format!(
                "你的任务: {analyst_title}\n\n重要原则：\n\
1. 如果上游数据节点返回为空，请主动调用可用工具获取补充数据。\n\
2. 如果经过补充获取仍然无法获得某些数据，请在分析报告中诚实标记该维度数据获取失败的状态，并评估该缺失对分析结论的影响程度。\n\
3. 始终针对需求给出明确的观点和论据。\n\
{base_prompt}",
            );
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));

            // 注入需求描述变量
            a.config.input_mapping = std::collections::HashMap::from([(
                "demand_description".into(),
                "demand_description".into(),
            )]);
        }
        nodes.push(an);

        // ToolNode → AgentNode 的边
        edges.push(edge(&format!("e-{tool_id}-{analyst_id}"), tool_id, analyst_id));

        analyst_branches.push(Branch {
            id: format!("branch-{analyst_id}"),
            title: analyst_title.to_string(),
            steps: vec![tool_id.to_string(), analyst_id.to_string()],
            branch_timeout_ms: None,
            degrade_strategy: DegradeStrategy::UseDefault,
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // p-analysts 容器
    // ═══════════════════════════════════════════════════════════════════════
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-analysts".into(),
            title: "多维度分析师分组".into(),
            description: Some("营销/产品/技术/财务/运营/综合六维度分析".into()),
            position: Position { x: 20.0, y: 80.0 },
            retry: RetryConfig::default(),
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: ParallelNodeConfig {
            branches: analyst_branches,
            wait_for_all: true,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: false,
            sub_graph: None,
        },
    }));

    // 前端验证要求容器节点有至少一条入边
    edges.push(edge("e-trigger-p-analysts", "trigger", "p-analysts"));

    // ═══════════════════════════════════════════════════════════════════════
    // Phase 2: CEO 综合决策（独立节点，等待所有分析师完成）
    // ═══════════════════════════════════════════════════════════════════════
    let analyst_ids: Vec<&str> = analyst_configs.iter().map(|(_, _, _, _, id, _, _)| *id).collect();

    let ceo_tools = PROFILE_TOOLS
        .iter()
        .find(|(aid, _)| aid == &"ceo-business-strategist")
        .map(|(_, tools)| tools.to_vec())
        .unwrap_or_default();

    let ceo_y = row_y_base + 2.0 * row_dy + 40.0;
    let mut ceo_an =
        agent("a-ceo-decision", "综合决策", "ceo-business-strategist", None, 400.0, ceo_y);

    if let WorkflowNode::Agent(ref mut a) = ceo_an {
        // 所有分析师的输出作为上下文
        a.config.context_sources = analyst_ids.iter().map(|id| id.to_string()).collect();
        a.config.model_role = Some("ceo".into());
        a.config.tools = ceo_tools.iter().filter_map(|tn| tool_def_map.get(tn).cloned()).collect();
        a.config.exposed_tools = ceo_tools.iter().map(|s| s.to_string()).collect();
        a.config.max_tool_rounds = Some(2);

        let ceo_prompt = "你的任务：综合所有分析结果，做出最终决策。\n\n\
重要原则：\n\
1. 汇总各维度分析结论（营销/产品/技术/财务/运营）\n\
2. 识别各维度的关键洞察和矛盾点\n\
3. 做出 go/no-go 决策，明确支持条件\n\
4. 输出行动计划和责任分配\n\n\
输出格式：\n\
- 决策结论（GO/NO-GO/CONDITIONAL GO）\n\
- 关键决策因素\n\
- 行动清单（按优先级）\n\
- 责任分配和时间线\n\
- 风险缓解措施";

        a.config.system_prompt = format!(
            "你的任务: 综合决策\n\n重要原则：\n\
1. 汇总各维度分析结论（营销/产品/技术/财务/运营）\n\
2. 识别各维度的关键洞察和矛盾点\n\
3. 做出 go/no-go 决策，明确支持条件\n\
4. 输出行动计划和责任分配\n\
{ceo_prompt}",
        );
        a.config.system_prompt =
            format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
        a.config.input_mapping = std::collections::HashMap::new();
    }
    nodes.push(ceo_an);

    // 所有分析师 → CEO 的边
    for analyst_id in &analyst_ids {
        edges.push(edge(&format!("e-{analyst_id}-ceo"), analyst_id, "a-ceo-decision"));
    }

    // 通知节点
    nodes.push(WorkflowNode::Notification(NotificationNode {
        base: WorkflowNodeBase {
            id: "n-notify".into(),
            title: "发送决策通知".into(),
            description: Some("将决策结果通知相关人员".into()),
            position: Position { x: 680.0, y: ceo_y },
            retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: NotificationNodeConfig {
            channel: "internal".into(),
            message: "需求发现流程完成，决策结果已生成".into(),
            webhook_url: None,
            recipients: vec!["ceo".into(), "cto".into(), "cfo".into()],
            subject: Some("OPC 需求发现决策通知".into()),
            enabled: true,
            output_var: "notification_result".into(),
        },
    }));

    // End 节点
    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end".into(),
            title: "完成".into(),
            description: Some("需求发现流程完成".into()),
            position: Position { x: 920.0, y: ceo_y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: EndNodeConfig { output_var: Some("final_decision".into()) },
    }));

    // CEO → 通知 → 结束
    edges.push(edge("e-ceo-notify", "a-ceo-decision", "n-notify"));
    edges.push(edge("e-notify-end", "n-notify", "end"));

    // ── 序列化并保存 ──
    let nodes_json = serde_json::to_string(&nodes).map_err(|e| format!("序列化节点失败: {e}"))?;
    let edges_json = serde_json::to_string(&edges).map_err(|e| format!("序列化边失败: {e}"))?;
    let tool_defs_val =
        serde_json::to_string(&tool_defs).map_err(|e| format!("序列化工具定义失败: {e}"))?;

    // 输入 Schema
    let input_schema_val = {
        let mut props = std::collections::HashMap::new();
        props.insert(
            "description".into(),
            JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("需求描述（可选）".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        let schema = JsonSchema {
            schema_type: "object".into(),
            description: Some("需求发现输入".into()),
            properties: Some(props),
            required: None,
            items: None,
        };
        serde_json::to_string(&schema).unwrap_or_default()
    };

    // 输出 Schema
    let output_schema_val = {
        let mut props = std::collections::HashMap::new();
        props.insert(
            "decision".into(),
            JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("决策结论: GO/NO-GO/CONDITIONAL GO".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        props.insert(
            "actions".into(),
            JsonSchemaProperty {
                schema_type: "array".into(),
                description: Some("行动清单".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        let schema = JsonSchema {
            schema_type: "object".into(),
            description: Some("需求发现输出".into()),
            properties: Some(props),
            required: Some(vec!["decision".into()]),
            items: None,
        };
        serde_json::to_string(&schema).unwrap_or_default()
    };

    // 变量（优先保留旧版本的用户自定义值）
    let variables_val = if let Some(ref old) = old_variables {
        old.clone()
    } else {
        let vars = vec![
            serde_json::json!({
                "name": "demand_description",
                "description": "需求描述",
                "value": "",
                "type": "string",
            }),
            serde_json::json!({
                "name": "priority_threshold",
                "description": "优先级阈值",
                "value": "P1",
                "type": "string",
            }),
            serde_json::json!({
                "name": "budget_limit",
                "description": "预算上限",
                "value": "50000",
                "type": "number",
            }),
        ];
        serde_json::to_string(&vars).unwrap_or_default()
    };

    // 错误配置
    let error_config_val = serde_json::json!({
        "on_error": "stop",
        "max_retries": 1,
        "fallback_to_previous": true,
    })
    .to_string();

    // Tags
    let tags = serde_json::to_string(&vec![
        "opc".to_string(),
        "demand-discovery".to_string(),
        "preset".to_string(),
    ])
    .unwrap_or_default();

    // ── 写入数据库 ──
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.to_string()),
        name: Set("OPC 需求发现".to_string()),
        description: Set(Some(
            "多维度并行分析（营销/产品/技术/财务/运营/综合）→ CEO 决策 → 通知相关方".to_string(),
        )),
        icon: Set("lightbulb".into()),
        tags: Set(Some(tags)),
        version: Set(TEMPLATE_VERSION),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(
            serde_json::to_string(&TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({"description": "{{demand_description}}"}),
            })
            .unwrap_or_default(),
        )),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(Some(input_schema_val)),
        output_schema: Set(Some(output_schema_val)),
        variables: Set(Some(variables_val)),
        error_config: Set(Some(error_config_val)),
        composite_source: Set(None),
        tool_defs: Set(Some(tool_defs_val)),
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| format!("写入工作流模板失败: {e}"))?;

    tracing::info!("[opc_setup] 需求发现工作流模板已种子化 v{TEMPLATE_VERSION} ({TEMPLATE_ID})");
    Ok(())
}
