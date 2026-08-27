//! 股票管道工作流模板种子化
//!
//! 管道流程（Agent 驱动）：
//! 1. 触发：手动触发
//! 2. 股票发现（CodeNode）：获取候选股 + 持仓股
//! 3. 股票筛选（AgentNode, stock-analyst）：分析师评估候选股优先级
//! 4. 并行分析（ParallelNode）：
//!    a. 新候选股分析（CodeNode）：调用单股分析工作流
//!    b. 持仓再评估（CodeNode）：调用单股分析工作流
//! 5. 风险评估（AgentNode, risk-evaluator）：风险评估师综合评估
//! 6. 投资决策（AgentNode, decision-maker）：决策者最终决策
//! 7. 汇总报告（AgentNode, decision-maker）：生成最终报告
//! 8. 结束

use axagent_entities::workflow_template;
use axagent_harness::capability::Visibility;
use axagent_harness::hallucination_guard::HallucinationGuardConfig;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, BackoffType, Branch, CodeNode, CodeNodeConfig, EdgeType, EndNode,
    EndNodeConfig, JsonSchema, JsonSchemaProperty, MergeStrategy, OutputMode, ParallelNode,
    ParallelNodeConfig, Position, RetryConfig, ToolDef, TriggerConfig, TriggerNode, TriggerType,
    Variable, WorkflowEdge, WorkflowNode, WorkflowNodeBase, WorkflowTemplateData,
};
use sea_orm::EntityTrait;

const TEMPLATE_ID: &str = "stock-pipeline";
const TEMPLATE_VERSION: i32 = 2;

/// 工具参数 Schema 构建辅助函数
fn tool_params_schema(properties: Vec<(&str, &str)>, required: Vec<&str>) -> JsonSchema {
    let mut props = std::collections::HashMap::new();
    for (name, desc) in properties {
        props.insert(
            name.to_string(),
            JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some(desc.into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
    }
    JsonSchema {
        schema_type: "object".into(),
        description: None,
        properties: Some(props),
        required: Some(required.into_iter().map(|s| s.to_string()).collect()),
        items: None,
    }
}

/// 股票管道工具定义（供 Agent 节点调用）——使用 LazyLock 延迟初始化
static PIPELINE_TOOLS: std::sync::LazyLock<Vec<ToolDef>> = std::sync::LazyLock::new(|| {
    vec![
        ToolDef {
            name: "get_stock_quote".into(),
            description: Some("获取股票实时行情".into()),
            parameters: Some(tool_params_schema(
                vec![("stock_code", "股票代码")],
                vec!["stock_code"],
            )),
        },
        ToolDef {
            name: "get_stock_financials".into(),
            description: Some("获取股票财务数据".into()),
            parameters: Some(tool_params_schema(
                vec![("stock_code", "股票代码")],
                vec!["stock_code"],
            )),
        },
        ToolDef {
            name: "compute_valuation".into(),
            description: Some("计算股票估值指标".into()),
            parameters: Some(tool_params_schema(
                vec![("stock_code", "股票代码")],
                vec!["stock_code"],
            )),
        },
        ToolDef {
            name: "run_single_analysis".into(),
            description: Some("对单只股票进行完整分析（调用股票分析工作流）".into()),
            parameters: Some(tool_params_schema(
                vec![("stock_code", "股票代码"), ("stock_name", "股票名称")],
                vec!["stock_code"],
            )),
        },
        ToolDef {
            name: "get_position_list".into(),
            description: Some("获取当前持仓列表".into()),
            parameters: None,
        },
    ]
});

/// 种子化股票管道工作流模板
pub async fn seed_stock_pipeline_template(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
    if let Some(existing) = workflow_template::Entity::find_by_id(TEMPLATE_ID)
        .one(db)
        .await
        .map_err(|e| format!("查询工作流模板失败: {e}"))?
    {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!("[stock_pipeline] 模板已是最新版本 v{}，跳过种子化", existing.version);
            return Ok(());
        }
        tracing::info!(
            "[stock_pipeline] 更新股票管道工作流模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );
    }

    let now = chrono::Utc::now().timestamp_millis();

    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    let make_base =
        |id: &str, title: &str, desc: Option<&str>, x: f64, y: f64| -> WorkflowNodeBase {
            WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: desc.map(|d| d.into()),
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
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            }
        };

    let make_agent = |id: &str,
                      title: &str,
                      expert_id: &str,
                      system_prompt: &str,
                      tools: Vec<ToolDef>,
                      x: f64,
                      y: f64|
     -> WorkflowNode {
        WorkflowNode::Agent(AgentNode {
            base: make_base(id, title, Some(title), x, y),
            config: AgentNodeConfig {
                system_prompt: system_prompt.into(),
                context_sources: vec![],
                input_mapping: [
                    ("stock_codes".to_string(), "stock_codes".to_string()),
                    ("holdings".to_string(), "holdings".to_string()),
                    ("candidates".to_string(), "candidates".to_string()),
                ]
                .into_iter()
                .collect(),
                output_var: id.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(32768),
                tools,
                exposed_tools: vec![],
                output_mode: OutputMode::Text,
                agent_profile_id: Some(format!("stock-{expert_id}")),
                max_tool_rounds: Some(5),
                execution_mode: None,
                rag_source_ids: vec![],
                model_role: None,
                consistency_check: None,
                hallucination_guard: Some(HallucinationGuardConfig {
                    enabled: false,
                    match_threshold: 0.4,
                }),
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: Some(300),
            },
        })
    };

    // ── 1. Trigger 节点 ──
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: make_base("trigger", "开始股票管道", Some("每日自动或手动触发管道"), 520.0, 0.0),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    }));

    // ── 2. 股票发现节点 (CodeNode) ──
    nodes.push(WorkflowNode::Code(CodeNode {
        base: make_base("discovery", "股票发现", Some("获取候选股 + 持仓股列表"), 520.0, 100.0),
        config: CodeNodeConfig {
            language: "rust".into(),
            code: "discover_candidates".into(),
            output_var: "discovery_result".into(),
            tool_name: Some("stock_pipeline_discover".into()),
            execute_directly: true,
            input_mapping: std::collections::HashMap::new(),
        },
    }));

    edges.push(WorkflowEdge {
        id: "e-trigger-discovery".into(),
        source: "trigger".into(),
        source_handle: None,
        target: "discovery".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 3. 股票筛选 Agent (market-synthesizer) ──
    nodes.push(make_agent(
        "screener",
        "股票筛选",
        "market-synthesizer",
        "你是股票分析师。请对候选股和持仓股进行优先级评估：
1. 对候选股进行多维度分析（技术面、基本面、资金面、情绪面）
2. 对持仓股进行再评估，给出持有/减仓/清仓建议
3. 输出筛选后的候选股列表和持仓评估结果，按优先级排序",
        vec![PIPELINE_TOOLS[0].clone(), PIPELINE_TOOLS[1].clone(), PIPELINE_TOOLS[2].clone()],
        520.0,
        220.0,
    ));

    edges.push(WorkflowEdge {
        id: "e-discovery-screener".into(),
        source: "discovery".into(),
        source_handle: None,
        target: "screener".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 4. 并行分析容器 ──
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: make_base("p-analyze", "并行分析", Some("新候选股分析 + 持仓再评估"), 520.0, 340.0),
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "branch-new-analyses".into(),
                    title: "新候选股分析".into(),
                    steps: vec!["analyze_new".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: Default::default(),
                },
                Branch {
                    id: "branch-reassess".into(),
                    title: "持仓再评估".into(),
                    steps: vec!["reassess_holdings".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: Default::default(),
                },
            ],
            wait_for_all: true,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: false,
            sub_graph: None,
        },
    }));

    edges.push(WorkflowEdge {
        id: "e-screener-p-analyze".into(),
        source: "screener".into(),
        source_handle: None,
        target: "p-analyze".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 5. 新候选股分析节点 (CodeNode - 调用单股分析工作流) ──
    nodes.push(WorkflowNode::Code(CodeNode {
        base: make_base(
            "analyze_new",
            "新候选股分析",
            Some("并发调用 run_single_stock_analysis 分析候选股"),
            200.0,
            460.0,
        ),
        config: CodeNodeConfig {
            language: "rust".into(),
            code: "analyze_stocks_batch".into(),
            output_var: "new_analyses".into(),
            tool_name: Some("stock_pipeline_analyze".into()),
            execute_directly: true,
            input_mapping: {
                let mut m = std::collections::HashMap::new();
                m.insert("stock_codes".into(), "screener".into());
                m
            },
        },
    }));

    edges.push(WorkflowEdge {
        id: "e-p-analyze-analyze_new".into(),
        source: "p-analyze".into(),
        source_handle: None,
        target: "analyze_new".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 6. 持仓再评估节点 (CodeNode) ──
    nodes.push(WorkflowNode::Code(CodeNode {
        base: make_base(
            "reassess_holdings",
            "持仓再评估",
            Some("获取持仓股并并发调用 run_single_stock_analysis 再评估"),
            840.0,
            460.0,
        ),
        config: CodeNodeConfig {
            language: "rust".into(),
            code: "reassess_holdings_batch".into(),
            output_var: "reassessed".into(),
            tool_name: Some("stock_pipeline_reassess".into()),
            execute_directly: true,
            input_mapping: {
                let mut m = std::collections::HashMap::new();
                m.insert("holdings".into(), "screener".into());
                m
            },
        },
    }));

    edges.push(WorkflowEdge {
        id: "e-p-analyze-reassess".into(),
        source: "p-analyze".into(),
        source_handle: None,
        target: "reassess_holdings".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 7. 风险评估 Agent (risk-convergence) ──
    nodes.push(make_agent(
        "risk_eval",
        "风险评估",
        "risk-convergence",
        "你是风险评估师。请综合评估所有分析结果中的风险：
1. 识别每只股票的主要风险点（政策风险、行业风险、公司风险、技术风险）
2. 评估持仓组合的整体风险敞口
3. 给出风险等级（低/中/高）和应对建议",
        vec![PIPELINE_TOOLS[0].clone(), PIPELINE_TOOLS[1].clone()],
        520.0,
        580.0,
    ));

    edges.push(WorkflowEdge {
        id: "e-analyze_new-risk_eval".into(),
        source: "analyze_new".into(),
        source_handle: None,
        target: "risk_eval".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    edges.push(WorkflowEdge {
        id: "e-reassess-risk_eval".into(),
        source: "reassess_holdings".into(),
        source_handle: None,
        target: "risk_eval".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 8. 投资决策 Agent (research-manager) ──
    nodes.push(make_agent(
        "decision",
        "投资决策",
        "research-manager",
        "你是投资决策者。请综合所有分析和风险评估结果，做出最终投资决策：
1. 对每只候选股给出明确建议（买入/观望/放弃）
2. 对持仓股给出操作建议（加仓/持有/减仓/清仓）
3. 确定仓位分配方案
4. 给出关键价位（入场价、目标价、止损价）",
        vec![PIPELINE_TOOLS[0].clone(), PIPELINE_TOOLS[2].clone()],
        520.0,
        700.0,
    ));

    edges.push(WorkflowEdge {
        id: "e-risk_eval-decision".into(),
        source: "risk_eval".into(),
        source_handle: None,
        target: "decision".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 9. 汇总报告 Agent (quality-fallback) ──
    nodes.push(make_agent(
        "report",
        "汇总报告",
        "quality-fallback",
        "你是投资报告撰写人。请将所有分析结果、风险评估和投资决策汇总成一份完整的每日投资报告：
1. 市场概况（大盘走势、板块热点）
2. 候选股分析汇总（TOP 推荐 + 分析要点）
3. 持仓评估汇总（持仓股表现 + 调整建议）
4. 风险提示（主要风险点 + 应对策略）
5. 操作计划（具体交易指令 + 时间表）",
        vec![],
        520.0,
        820.0,
    ));

    edges.push(WorkflowEdge {
        id: "e-decision-report".into(),
        source: "decision".into(),
        source_handle: None,
        target: "report".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 10. End 节点 ──
    nodes.push(WorkflowNode::End(EndNode {
        base: make_base("end", "结束", Some("管道执行完成"), 520.0, 940.0),
        config: EndNodeConfig { output_var: Some("pipeline_result".into()) },
    }));

    edges.push(WorkflowEdge {
        id: "e-report-end".into(),
        source: "report".into(),
        source_handle: None,
        target: "end".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // ── 构建 input_schema ──
    let input_schema = {
        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "max_candidates".into(),
            JsonSchemaProperty {
                schema_type: "integer".into(),
                description: Some("候选股最大数量".into()),
                default: Some(serde_json::json!(5)),
                enum_values: None,
                format: None,
            },
        );
        properties.insert(
            "new_analysis_concurrency".into(),
            JsonSchemaProperty {
                schema_type: "integer".into(),
                description: Some("新候选股分析并发数".into()),
                default: Some(serde_json::json!(2)),
                enum_values: None,
                format: None,
            },
        );
        properties.insert(
            "holdings_reassess_concurrency".into(),
            JsonSchemaProperty {
                schema_type: "integer".into(),
                description: Some("持仓再评估并发数".into()),
                default: Some(serde_json::json!(2)),
                enum_values: None,
                format: None,
            },
        );
        properties.insert(
            "as_of_date".into(),
            JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("指定分析日期（可选）".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        Some(JsonSchema {
            schema_type: "object".into(),
            description: Some("股票管道输入参数".into()),
            properties: Some(properties),
            required: None,
            items: None,
        })
    };

    // ── 构建 variables ──
    let variables = vec![
        Variable {
            name: "max_candidates".into(),
            var_type: "integer".into(),
            value: serde_json::json!(5),
            description: Some("候选股最大数量".into()),
            is_secret: false,
        },
        Variable {
            name: "new_analysis_concurrency".into(),
            var_type: "integer".into(),
            value: serde_json::json!(2),
            description: Some("新候选股分析并发数".into()),
            is_secret: false,
        },
        Variable {
            name: "holdings_reassess_concurrency".into(),
            var_type: "integer".into(),
            value: serde_json::json!(2),
            description: Some("持仓再评估并发数".into()),
            is_secret: false,
        },
        Variable {
            name: "as_of_date".into(),
            var_type: "string".into(),
            value: serde_json::Value::Null,
            description: Some("指定分析日期".into()),
            is_secret: false,
        },
    ];

    // ── 构建 WorkflowTemplateData ──
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "股票全业务管道".to_string(),
        description: Some("Agent 驱动的每日自动发现 + 筛选 + 分析 + 决策管道".to_string()),
        icon: "📈".to_string(),
        cluster_id: Some("pipeline".to_string()),
        route_path: Some("/finance/pipeline/stock-pipeline".to_string()),
        tags: vec!["stock".to_string(), "pipeline".to_string(), "agent".to_string()],
        version: TEMPLATE_VERSION,
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema,
        output_schema: None,
        variables,
        error_config: None,
        error_workflow_id: None,
        tool_defs: Vec::new(), // Rhai 脚本工具（顶层注册）
        mission_hash: None,
        created_at: now,
        updated_at: now,
    };

    upsert_template(db, template_data).await?;

    tracing::info!("[stock_pipeline] 股票管道工作流模板 v{TEMPLATE_VERSION} 种子化完成");
    Ok(())
}

/// 将 WorkflowTemplateData 写入或更新到数据库
async fn upsert_template(
    db: &sea_orm::DatabaseConnection,
    data: WorkflowTemplateData,
) -> Result<(), String> {
    use sea_orm::*;

    let tags_json = serde_json::to_string(&data.tags).unwrap_or_default();
    let nodes_json = serde_json::to_string(&data.nodes).map_err(|e| format!("nodes json: {e}"))?;
    let edges_json = serde_json::to_string(&data.edges).map_err(|e| format!("edges json: {e}"))?;
    let vars_json = serde_json::to_string(&data.variables).unwrap_or_default();
    let trigger_json = data.trigger_config.as_ref().and_then(|t| serde_json::to_string(t).ok());
    let input_json = data.input_schema.as_ref().and_then(|s| serde_json::to_string(s).ok());
    let output_json = data.output_schema.as_ref().and_then(|s| serde_json::to_string(s).ok());

    let am = workflow_template::ActiveModel {
        id: Set(data.id.clone()),
        cluster_id: Set(data.cluster_id.clone()),
        route_path: Set(data.route_path.clone()),
        name: Set(data.name),
        description: Set(data.description),
        icon: Set(data.icon),
        tags: Set(Some(tags_json)),
        version: Set(data.version),
        is_preset: Set(data.is_preset),
        is_editable: Set(data.is_editable),
        is_public: Set(data.is_public),
        trigger_config: Set(trigger_json),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(input_json),
        output_schema: Set(output_json),
        variables: Set(Some(vars_json)),
        error_config: Set(None),
        composite_source: Set(None),
        mission_hash: Set(data.mission_hash),
        tool_defs: Set(None),
        created_at: Set(data.created_at),
        updated_at: Set(data.updated_at),
    };

    workflow_template::Entity::insert(am)
        .on_conflict(
            sea_query::OnConflict::column(workflow_template::Column::Id)
                .update_column(workflow_template::Column::Name)
                .update_column(workflow_template::Column::Description)
                .update_column(workflow_template::Column::Icon)
                .update_column(workflow_template::Column::Tags)
                .update_column(workflow_template::Column::Version)
                .update_column(workflow_template::Column::Nodes)
                .update_column(workflow_template::Column::Edges)
                .update_column(workflow_template::Column::InputSchema)
                .update_column(workflow_template::Column::OutputSchema)
                .update_column(workflow_template::Column::Variables)
                .update_column(workflow_template::Column::ErrorConfig)
                .update_column(workflow_template::Column::UpdatedAt)
                .to_owned(),
        )
        .exec_without_returning(db)
        .await
        .map_err(|e| format!("upsert template: {e}"))?;

    Ok(())
}
