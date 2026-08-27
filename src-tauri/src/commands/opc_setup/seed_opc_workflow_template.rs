// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 需求发现工作流模板 — 持久化到 workflow_template 表
//!
//! 参考 stock_analysis_setup::seed_stock_analysis_workflow_template 模式
//!
//! v2 增强：增加技术社区扫描（Reddit/HN/GitHub）和需求价值评估
//!
//! 工作流结构：
//!   Trigger → Phase 1: 多平台扫描（技术社区 + 众包平台）
//!          → Phase 2: 需求价值评估
//!          → Phase 3: 现有数据收集
//!          → Phase 4: 并行 AgentNode(多角色分析)
//!          → Phase 5: 汇总决策 → End
//!
//! Agent 节点绑定 agent_profile_id 实现三要素：
//!   1. AgentRole.system_prompt      → 岗位/角色定义
//!   2. AgencyExpert.system_prompt   → 专家专业提示词
//!   3. AgentNodeConfig.system_prompt → 节点级任务指令（inline 嵌入）

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

// V4(2026-08-13): 添加 Phase 1 多平台扫描器 + Phase 2 需求价值评估
// V3(2026-08-13): 与股票分析工作流完全对齐
// V2: 初始版本
const TEMPLATE_VERSION: i32 = 4;

/// 种子化 OPC 需求发现工作流模板到数据库
pub(crate) async fn seed_opc_workflow_template(db: &DatabaseConnection) -> Result<(), String> {
    let existing = workflow_template::Entity::find_by_id(TEMPLATE_ID)
        .one(db)
        .await
        .map_err(|e| format!("查询工作流模板失败: {e}"))?;

    if let Some(existing) = existing {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!("[opc_setup] 模板已是最新版本 v{}，跳过种子化", existing.version);
            return Ok(());
        }
        tracing::info!(
            "[opc_setup] 更新需求发现工作流模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );
    }

    let now = chrono::Utc::now().timestamp_millis();

    // ── 工具定义 ──
    let mut tool_defs: Vec<ToolDef> = Vec::new();

    // OpcListProjects - 获取现有项目列表
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

    // OpcListCustomers - 获取客户列表
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

    // OpcListInvoices - 获取发票列表
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

    // OpcGetDashboard - 获取仪表盘数据
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

    // OpcListBlogPosts - 获取博客文章列表
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

    // OpcSearchWiki - 搜索知识库
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

    // OpcSendNotification - 发送通知
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

    // OpcListKpis - 获取 KPI 列表
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

    // ── v2 新增：多平台扫描器 ──

    // RedditScanner - Reddit 技术社区扫描
    tool_defs.push(ToolDef {
        name: "RedditScanner".into(),
        description: Some("扫描 Reddit 技术社区，获取相关需求线索".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "query".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("搜索关键词，如'AI tools'".into()),
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

    // HackerNewsScanner - HackerNews 扫描
    tool_defs.push(ToolDef {
        name: "HackerNewsScanner".into(),
        description: Some("扫描 HackerNews，获取技术趋势和需求线索".into()),
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

    // GitHubIssueScanner - GitHub Issue 扫描
    tool_defs.push(ToolDef {
        name: "GitHubIssueScanner".into(),
        description: Some("扫描 GitHub Issue，获取开源项目中的需求和 Bug".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "query".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("搜索关键词，如'feature request AI'".into()),
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

    // ZhumajieScanner - 猪八戒平台扫描
    tool_defs.push(ToolDef {
        name: "ZhumajieScanner".into(),
        description: Some("扫描猪八戒平台，获取外包需求线索".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "query".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("搜索关键词，如'小程序开发'".into()),
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

    // XianyuScanner - 闲鱼平台扫描
    tool_defs.push(ToolDef {
        name: "XianyuScanner".into(),
        description: Some("扫描闲鱼平台，获取二手需求线索".into()),
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

    // DemandValueEvaluator - 需求价值评估
    tool_defs.push(ToolDef {
        name: "DemandValueEvaluator".into(),
        description: Some("评估需求的商业价值，输出痛点分、市场缺口分、综合评分".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some({
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "demands".into(),
                    JsonSchemaProperty {
                        schema_type: "array".into(),
                        description: Some("需求列表，每项包含 id、title、description".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                );
                props.insert(
                    "min_score".into(),
                    JsonSchemaProperty {
                        schema_type: "number".into(),
                        description: Some("最低价值分阈值（0-100）".into()),
                        default: Some(serde_json::json!(50)),
                        enum_values: None,
                        format: None,
                    },
                );
                props
            }),
            required: Some(vec!["demands".into()]),
            items: None,
        }),
    });

    // ── 辅助函数 ──
    let tool_node = |id: &str,
                     title: &str,
                     tool_name: &str,
                     output_var: &str,
                     input_mapping: std::collections::HashMap<String, String>,
                     parent_id: Option<&str>,
                     x: f64,
                     y: f64|
     -> WorkflowNode {
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

    let agent = |id: &str,
                 title: &str,
                 expert_id: &str,
                 system_prompt: String,
                 tools: Vec<ToolDef>,
                 exposed_tools: Vec<String>,
                 context_sources: Vec<String>,
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
                    max_retries: 2,
                    base_delay_ms: 3000,
                    max_delay_ms: 30000,
                    backoff_type: BackoffType::Exponential,
                },
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt,
                context_sources,
                input_mapping: std::collections::HashMap::new(),
                output_var: id.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(16384),
                tools,
                exposed_tools,
                output_mode: OutputMode::Text,
                agent_profile_id: Some(format!("opc-{expert_id}")),
                max_tool_rounds: Some(3),
                execution_mode: None,
                rag_source_ids: Vec::new(),
                model_role: None,
                consistency_check: None,
                hallucination_guard: None,
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
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

    // ── 构建节点 ──
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    // Trigger - 开始需求发现
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".into(),
            title: "开始需求发现".into(),
            description: Some("手动触发需求发现流程".into()),
            position: Position { x: 560.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({"description": "{{需求描述}}"}),
        },
    }));

    // ═══════════════════════════════════════════════════════════════════════
    // Phase 1: 多平台需求扫描（ParallelNode: p-scanners）
    // ═══════════════════════════════════════════════════════════════════════
    // 并行执行 5 个扫描器，收集原始需求线索
    let scanner_configs: &[(&str, &str, &str, &str, f64)] = &[
        ("t-reddit-scan", "Reddit 扫描", "RedditScanner", "keywords", 60.0),
        ("t-hn-scan", "HackerNews 扫描", "HackerNewsScanner", "days", 140.0),
        ("t-github-scan", "GitHub Issue 扫描", "GitHubIssueScanner", "repos", 220.0),
        ("t-zhumajie-scan", "猪八戒扫描", "ZhumajieScanner", "category", 300.0),
        ("t-xianyu-scan", "闲鱼扫描", "XianyuScanner", "keyword", 380.0),
    ];

    let mut scanner_branches: Vec<Branch> = Vec::new();
    for (tool_id, title, tool_name, arg_key, y) in scanner_configs {
        let mut im = std::collections::HashMap::new();
        im.insert(arg_key.to_string(), "all".to_string());
        let tn = tool_node(
            tool_id,
            title,
            tool_name,
            &format!("{tool_id}_result"),
            im,
            Some("p-scanners"),
            60.0,
            *y,
        );
        nodes.push(tn);

        scanner_branches.push(Branch {
            id: format!("branch-{tool_id}"),
            title: title.to_string(),
            steps: vec![tool_id.to_string()],
            branch_timeout_ms: Some(120),
            degrade_strategy: DegradeStrategy::UseDefault,
        });
    }

    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-scanners".into(),
            title: "多平台需求扫描".into(),
            description: Some("并行扫描 Reddit/HackerNews/GitHub/猪八戒/闲鱼".into()),
            position: Position { x: 20.0, y: 60.0 },
            retry: RetryConfig::default(),
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: ParallelNodeConfig {
            branches: scanner_branches,
            wait_for_all: false,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: false,
            sub_graph: None,
        },
    }));

    // ═══════════════════════════════════════════════════════════════════════
    // Phase 2: 需求价值评估（ToolNode）
    // ═══════════════════════════════════════════════════════════════════════
    // 汇总扫描结果，评估需求商业价值
    let mut im_eval = std::collections::HashMap::new();
    im_eval.insert("scan_results".to_string(), "all".to_string());
    nodes.push(tool_node(
        "t-demand-eval",
        "需求价值评估",
        "DemandValueEvaluator",
        "evaluation_result",
        im_eval,
        None,
        60.0,
        460.0,
    ));

    // Phase 1: 数据收集层 (ToolNodes)
    // 获取现有项目数据
    let mut im_projects = std::collections::HashMap::new();
    im_projects.insert("status".to_string(), "all".to_string());
    nodes.push(tool_node(
        "t-projects",
        "获取现有项目",
        "OpcListProjects",
        "projects_data",
        im_projects,
        None,
        40.0,
        120.0,
    ));

    // 获取现有客户数据
    let mut im_customers = std::collections::HashMap::new();
    im_customers.insert("status".to_string(), "all".to_string());
    nodes.push(tool_node(
        "t-customers",
        "获取现有客户",
        "OpcListCustomers",
        "customers_data",
        im_customers,
        None,
        280.0,
        120.0,
    ));

    // 获取财务数据
    let mut im_invoices = std::collections::HashMap::new();
    im_invoices.insert("status".to_string(), "all".to_string());
    nodes.push(tool_node(
        "t-invoices",
        "获取财务数据",
        "OpcListInvoices",
        "invoices_data",
        im_invoices,
        None,
        520.0,
        120.0,
    ));

    // 获取仪表盘数据
    nodes.push(tool_node(
        "t-dashboard",
        "获取经营仪表盘",
        "OpcGetDashboard",
        "dashboard_data",
        std::collections::HashMap::new(),
        None,
        760.0,
        120.0,
    ));

    // 获取博客数据
    nodes.push(tool_node(
        "t-blog",
        "获取内容表现",
        "OpcListBlogPosts",
        "blog_data",
        std::collections::HashMap::new(),
        None,
        1000.0,
        120.0,
    ));

    // 获取 KPI 数据
    nodes.push(tool_node(
        "t-kpis",
        "获取关键指标",
        "OpcListKpis",
        "kpis_data",
        std::collections::HashMap::new(),
        None,
        1240.0,
        120.0,
    ));

    // Phase 2: 并行分析层 (AgentNodes) - 多角色并行分析
    // 营销增长分析 (CMO)
    let cmo_tools = vec![
        tool_defs[1].clone(), // OpcListCustomers
        tool_defs[4].clone(), // OpcListBlogPosts
        tool_defs[5].clone(), // OpcSearchWiki
        tool_defs[7].clone(), // OpcListKpis
    ];
    nodes.push(agent(
        "a-cmo-analysis",
        "营销增长分析",
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
        - 优先级建议（P0/P1/P2）"
            .to_string(),
        cmo_tools,
        vec![
            "OpcListCustomers".into(),
            "OpcListBlogPosts".into(),
            "OpcSearchWiki".into(),
            "OpcListKpis".into(),
        ],
        vec!["customers_data".into(), "blog_data".into(), "kpis_data".into()],
        40.0,
        280.0,
    ));

    // 产品需求分析 (CPO)
    let cpo_tools = vec![
        tool_defs[0].clone(), // OpcListProjects
        tool_defs[1].clone(), // OpcListCustomers
        tool_defs[5].clone(), // OpcSearchWiki
    ];
    nodes.push(agent(
        "a-cpo-analysis",
        "产品需求分析",
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
        - 技术依赖关系"
            .to_string(),
        cpo_tools,
        vec!["OpcListProjects".into(), "OpcListCustomers".into(), "OpcSearchWiki".into()],
        vec!["projects_data".into(), "customers_data".into()],
        280.0,
        280.0,
    ));

    // 技术可行性评估 (CTO)
    let cto_tools = vec![
        tool_defs[0].clone(), // OpcListProjects
        tool_defs[5].clone(), // OpcSearchWiki
    ];
    nodes.push(agent(
        "a-cto-analysis",
        "技术可行性评估",
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
        - 工时估算"
            .to_string(),
        cto_tools,
        vec!["OpcListProjects".into(), "OpcSearchWiki".into()],
        vec!["projects_data".into(), "kpis_data".into()],
        520.0,
        280.0,
    ));

    // 财务可行性评估 (CFO)
    let cfo_tools = vec![
        tool_defs[2].clone(), // OpcListInvoices
        tool_defs[3].clone(), // OpcGetDashboard
        tool_defs[7].clone(), // OpcListKpis
    ];
    nodes.push(agent(
        "a-cfo-analysis",
        "财务可行性评估",
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
        - 财务风险清单"
            .to_string(),
        cfo_tools,
        vec!["OpcListInvoices".into(), "OpcGetDashboard".into(), "OpcListKpis".into()],
        vec!["invoices_data".into(), "dashboard_data".into()],
        760.0,
        280.0,
    ));

    // 运营可行性评估 (COO)
    let coo_tools = vec![
        tool_defs[0].clone(), // OpcListProjects
        tool_defs[1].clone(), // OpcListCustomers
    ];
    nodes.push(agent(
        "a-coo-analysis",
        "运营可行性评估",
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
        - 资源调整建议"
            .to_string(),
        coo_tools,
        vec!["OpcListProjects".into(), "OpcListCustomers".into()],
        vec!["projects_data".into(), "customers_data".into()],
        1000.0,
        280.0,
    ));

    // Phase 3: 汇总决策层 (CEO)
    let ceo_tools = vec![
        tool_defs[3].clone(), // OpcGetDashboard
        tool_defs[6].clone(), // OpcSendNotification
    ];
    nodes.push(agent(
        "a-ceo-decision",
        "综合决策",
        "ceo-business-strategist",
        "你的任务：综合所有分析结果，做出最终决策。\n\n\
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
        - 风险缓解措施"
            .to_string(),
        ceo_tools,
        vec!["OpcGetDashboard".into(), "OpcSendNotification".into()],
        vec![
            "a-cmo-analysis".into(),
            "a-cpo-analysis".into(),
            "a-cto-analysis".into(),
            "a-cfo-analysis".into(),
            "a-coo-analysis".into(),
        ],
        600.0,
        480.0,
    ));

    // 通知节点 - 发送决策结果
    nodes.push(WorkflowNode::Notification(NotificationNode {
        base: WorkflowNodeBase {
            id: "n-notify".into(),
            title: "发送决策通知".into(),
            description: Some("将决策结果通知相关人员".into()),
            position: Position { x: 840.0, y: 480.0 },
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
            position: Position { x: 1080.0, y: 480.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: EndNodeConfig { output_var: Some("final_decision".into()) },
    }));

    // ── 构建边 ──
    // Trigger → 扫描器
    edges.push(edge("e-trigger-scanners", "trigger", "p-scanners"));

    // 扫描器 → 评估节点
    for (tool_id, _, _, _, _) in scanner_configs {
        edges.push(edge(&format!("e-{tool_id}-eval"), tool_id, "t-demand-eval"));
    }

    // 评估节点 → 数据收集层
    edges.push(edge("e-eval-projects", "t-demand-eval", "t-projects"));
    edges.push(edge("e-eval-customers", "t-demand-eval", "t-customers"));
    edges.push(edge("e-eval-invoices", "t-demand-eval", "t-invoices"));
    edges.push(edge("e-eval-dashboard", "t-demand-eval", "t-dashboard"));
    edges.push(edge("e-eval-blog", "t-demand-eval", "t-blog"));
    edges.push(edge("e-eval-kpis", "t-demand-eval", "t-kpis"));

    // 数据收集层 → 并行分析层
    edges.push(edge("e7", "t-projects", "a-cpo-analysis"));
    edges.push(edge("e8", "t-projects", "a-cto-analysis"));
    edges.push(edge("e9", "t-projects", "a-coo-analysis"));
    edges.push(edge("e10", "t-customers", "a-cmo-analysis"));
    edges.push(edge("e11", "t-customers", "a-cpo-analysis"));
    edges.push(edge("e12", "t-customers", "a-coo-analysis"));
    edges.push(edge("e13", "t-invoices", "a-cfo-analysis"));
    edges.push(edge("e14", "t-dashboard", "a-cfo-analysis"));
    edges.push(edge("e15", "t-dashboard", "a-ceo-decision"));
    edges.push(edge("e16", "t-blog", "a-cmo-analysis"));
    edges.push(edge("e17", "t-kpis", "a-cmo-analysis"));
    edges.push(edge("e18", "t-kpis", "a-cfo-analysis"));
    edges.push(edge("e19", "t-kpis", "a-cto-analysis"));

    // 并行分析层 → 汇总决策层
    edges.push(edge("e20", "a-cmo-analysis", "a-ceo-decision"));
    edges.push(edge("e21", "a-cpo-analysis", "a-ceo-decision"));
    edges.push(edge("e22", "a-cto-analysis", "a-ceo-decision"));
    edges.push(edge("e23", "a-cfo-analysis", "a-ceo-decision"));
    edges.push(edge("e24", "a-coo-analysis", "a-ceo-decision"));

    // 汇总决策 → 通知 → 结束
    edges.push(edge("e25", "a-ceo-decision", "n-notify"));
    edges.push(edge("e26", "n-notify", "end"));

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

    // 变量
    let variables_val = {
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

    // Tags - 序列化为 JSON 字符串
    let tags = serde_json::to_string(&vec![
        "opc".to_string(),
        "demand-discovery".to_string(),
        "preset".to_string(),
    ])
    .unwrap_or_default();

    // ── 写入数据库 ──
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.to_string()),
        cluster_id: Set(Some("opc".to_string())),
        route_path: Set(Some("/automation/opc/demand-discovery".to_string())),
        name: Set("OPC 需求发现".to_string()),
        description: Set(Some(
            "多角色并行分析（营销/产品/技术/财务/运营）→ CEO 综合决策 → 通知相关方".to_string(),
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

    tracing::info!("[opc_setup] 需求发现工作流模板已种子化 ({TEMPLATE_ID})");
    Ok(())
}
