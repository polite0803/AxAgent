// SPDX-License-Identifier: AGPL-3.0-only
//! G3.3 news-to-cross-market-analysis 工作流模板种子
//!
//! ## 模板用途
//!
//! 输入新闻文本 → 产业链关键词匹配 → 跨市场传导映射 → 综合输出受影响标的清单
//! （A 股 / 美股 / 港股），对齐 DojoAgents 宣传场景 2「新闻到模拟组合」的传导映射环节。
//!
//! ## DAG 结构
//!
//! ```text
//! trigger (Manual，输入 news_text)
//!   → map-news-to-chain   (Code 节点，调 map_news_to_cross_market_stocks MCP 工具)
//!   → propagate-impact    (Code 节点，对每条命中链调 get_industry_chain_propagation)
//!   → synthesize-result   (Agent 节点，LLM 综合传导结果，输出标准化 JSON）
//!   → end
//! ```
//!
//! Code 节点完成纯数据预处理；Agent 节点负责语义增强（理由 / 信心 / 持续性判断）。

use axagent_entities::workflow_template;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, CodeNode, CodeNodeConfig, EdgeType, EndNode, EndNodeConfig,
    JsonSchema, JsonSchemaProperty, OutputMode, Position, RetryConfig, ToolDef, TriggerConfig,
    TriggerNode, TriggerType, Variable, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
};

const TEMPLATE_ID: &str = "news-to-cross-market-analysis";
const TEMPLATE_VERSION: i32 = 1;

/// 种子化 news-to-cross-market-analysis 工作流模板
pub async fn seed_news_cross_market_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();

    // ── 触发器：手动触发，要求输入 news_text ──
    let manual_cfg_value = serde_json::json!({
        "input_params": {
            "news_text": {
                "type": "string",
                "description": "待分析的新闻正文（中文，≥10 字符）",
                "required": true
            }
        }
    });
    let trigger_config =
        TriggerConfig { trigger_type: TriggerType::Manual, config: manual_cfg_value.clone() };

    // ── Agent 可用工具：2 个产业链 MCP 工具 ──
    let agent_tools: Vec<ToolDef> = vec![
        ToolDef {
            name: "map_news_to_cross_market_stocks".into(),
            description: Some(
                "将新闻正文映射到预定义产业链（5 条链：AI 算力 / 半导体 / 光模块 / 新能源车 / 消费电子），返回命中链 + 激活节点 + 传导结果".into(),
            ),
            parameters: Some(JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "news_text".into(),
                        JsonSchemaProperty {
                            schema_type: "string".into(),
                            description: Some("新闻正文（≥10 字符）".into()),
                            default: None,
                            enum_values: None,
                            format: None,
                        },
                    );
                    m
                }),
                required: Some(vec!["news_text".into()]),
                items: None,
            }),
        },
        ToolDef {
            name: "get_industry_chain_propagation".into(),
            description: Some(
                "对指定产业链的起始节点做正向 / 负向 / 中性传导，返回下游受影响节点列表（含 A 股 / 美股 / 港股代码）".into(),
            ),
            parameters: Some(JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "chain_id".into(),
                        JsonSchemaProperty {
                            schema_type: "string".into(),
                            description: Some("产业链 ID：ai_compute / semiconductor / optical_module / nev / consumer_electronics".into()),
                            default: None,
                            enum_values: Some(vec![
                                "ai_compute".into(),
                                "semiconductor".into(),
                                "optical_module".into(),
                                "nev".into(),
                                "consumer_electronics".into(),
                            ]),
                            format: None,
                        },
                    );
                    m.insert(
                        "start_node_id".into(),
                        JsonSchemaProperty {
                            schema_type: "string".into(),
                            description: Some("起始节点 ID（如 gpu / lithium_mining / panel）".into()),
                            default: None,
                            enum_values: None,
                            format: None,
                        },
                    );
                    m.insert(
                        "direction".into(),
                        JsonSchemaProperty {
                            schema_type: "string".into(),
                            description: Some("影响方向：positive / negative / neutral".into()),
                            default: Some(serde_json::json!("neutral")),
                            enum_values: Some(vec![
                                "positive".into(),
                                "negative".into(),
                                "neutral".into(),
                            ]),
                            format: None,
                        },
                    );
                    m
                }),
                required: Some(vec!["chain_id".into(), "start_node_id".into()]),
                items: None,
            }),
        },
    ];

    // ── 节点定义 ──
    let nodes: Vec<WorkflowNode> = vec![
        // 1. 触发器：手动触发，传入 news_text
        WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                id: "trigger".into(),
                title: "手动触发：输入新闻".into(),
                description: Some("Manual 触发，输入 news_text 进行产业链传导分析".into()),
                position: Position { x: 20.0, y: 20.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: trigger_config.clone(),
        }),
        // 2. Agent 节点：综合分析输出标准化传导结果
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: "synthesize-result".into(),
                title: "LLM 综合传导结果".into(),
                description: Some(
                    "调用产业链 MCP 工具，结合 LLM 语义判断，输出受影响标的清单（A 股 / 美股 / 港股）"
                        .into(),
                ),
                position: Position { x: 20.0, y: 180.0 },
                retry: RetryConfig { enabled: true, max_retries: 1, ..Default::default() },
                timeout: Some(300), // 5 分钟超时
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: r#"基于用户输入的新闻文本，识别产业链传导路径，输出跨市场受影响标的清单。

执行步骤：
1. 调用 map_news_to_cross_market_stocks 工具（news_text 来自触发器变量）
2. 对每条命中链，调用 get_industry_chain_propagation 工具获取完整传导路径

请根据产业链传导分析方法论完成任务，输出 JSON 结果。"#.into(),
                context_sources: vec!["trigger".into()],
                input_mapping: std::collections::HashMap::new(),
                output_var: "cross-market-output".into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(4096),
                tools: agent_tools,
                exposed_tools: vec![],
                output_mode: OutputMode::Json,
                agent_profile_id: Some("stock-industry-chain-analyzer".into()),
                max_tool_rounds: Some(6),
                execution_mode: Some("react".into()),
                rag_source_ids: vec![],
                model_role: None,
                consistency_check: None,
                hallucination_guard: None,
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
            },
        }),
        // 3. 终止节点
        WorkflowNode::End(EndNode {
            base: WorkflowNodeBase {
                id: "end".into(),
                title: "结束".into(),
                description: None,
                position: Position { x: 20.0, y: 340.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: EndNodeConfig { output_var: Some("cross-market-output".into()) },
        }),
    ];

    let edges: Vec<WorkflowEdge> = vec![
        WorkflowEdge {
            id: "e-trigger-agent".into(),
            source: "trigger".into(),
            source_handle: None,
            target: "synthesize-result".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-agent-end".into(),
            source: "synthesize-result".into(),
            source_handle: None,
            target: "end".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
    ];

    // ── 模板变量（运行时由触发器填充）──
    let variables: Vec<Variable> = vec![Variable {
        name: "news_text".into(),
        var_type: "string".into(),
        value: serde_json::json!(""),
        description: Some("待分析的新闻正文（≥10 字符）".into()),
        is_secret: false,
    }];

    // ── 版本检查与快照 ──
    let existing =
        workflow_template::Entity::find_by_id(TEMPLATE_ID).one(db).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    if let Some(ref existing) = existing {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[stock_analysis_setup] {TEMPLATE_ID} 模板已是最新 v{}，跳过种子化",
                existing.version
            );
            return Ok(());
        }
        let ver_id = format!("{TEMPLATE_ID}_v{}", existing.version);
        let ver_existing = axagent_entities::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        if ver_existing.is_none() {
            let snapshot = axagent_entities::workflow_template_version::ActiveModel {
                id: Set(ver_id.clone()),
                template_id: Set(TEMPLATE_ID.into()),
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
                created_at: Set(now),
            };
            snapshot.insert(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            tracing::info!("[stock_analysis_setup] {TEMPLATE_ID} 模板旧版本快照已保存: {ver_id}");
        }
    }

    let nodes_json = serde_json::to_string(&nodes).map_err(|e| format!("序列化节点失败: {e}"))?;
    let edges_json = serde_json::to_string(&edges).map_err(|e| format!("序列化边失败: {e}"))?;
    let variables_json =
        serde_json::to_string(&variables).map_err(|e| format!("序列化变量失败: {e}"))?;
    let trigger_config_json =
        serde_json::to_string(&trigger_config).map_err(|e| format!("序列化触发器配置失败: {e}"))?;
    let tags = serde_json::to_string(&[
        "产业链传导".to_string(),
        "跨市场分析".to_string(),
        "G3".to_string(),
    ])
    .map_err(|e| format!("序列化标签失败: {e}"))?;

    // 先删再插
    let _ = workflow_template::Entity::delete_by_id(TEMPLATE_ID).exec(db).await;
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.into()),
        cluster_id: Set(Some("cross-market".to_string())),
        route_path: Set(Some("/finance/cross-market/news".to_string())),
        name: Set("新闻→跨市场传导分析".into()),
        description: Set(Some(
            "输入新闻正文，匹配预定义产业链（AI 算力 / 半导体 / 光模块 / 新能源车 / 消费电子），\
             调用 MCP 工具计算传导路径，LLM 综合输出跨市场受影响标的清单（A 股 / 美股 / 港股）"
                .into(),
        )),
        icon: Set("🔗".into()),
        tags: Set(Some(tags)),
        version: Set(TEMPLATE_VERSION),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(trigger_config_json)),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some(variables_json)),
        error_config: Set(None),
        composite_source: Set(None),
        tool_defs: Set(None),
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| format!("写入 {TEMPLATE_ID} 模板失败: {e}"))?;

    tracing::info!(
        "[stock_analysis_setup] G3.3 news-to-cross-market-analysis 模板已创建 (v{TEMPLATE_VERSION})"
    );
    Ok(())
}

/// 抑制未使用警告（CodeNode 等 import 暂未使用，保留供后续扩展）
#[allow(dead_code)]
fn _touch_unused() {
    let _ = CodeNode {
        base: WorkflowNodeBase {
            id: String::new(),
            title: String::new(),
            description: None,
            position: Position { x: 0.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: CodeNodeConfig {
            language: String::new(),
            code: String::new(),
            output_var: String::new(),
            tool_name: None,
            execute_directly: false,
            input_mapping: std::collections::HashMap::new(),
        },
    };
}
