// SPDX-License-Identifier: AGPL-3.0-only
//! G6 screenshot-portfolio-diagnosis 工作流模板种子
//!
//! ## 模板用途
//!
//! 对齐 DojoAgents 宣传场景 3「截图持仓诊断」：
//! 用户上传券商 App / 同花顺 / 东方财富 / 雪球 / 通达信持仓截图，
//! Agent 调用 `screenshot_diagnosis_create_from_image` 工具完成
//! OCR + 结构化持仓解析 + 7 项风险指标计算 + LLM 中文诊断说明 + 持久化。
//!
//! ## DAG 结构
//!
//! ```text
//! trigger (Manual: image_base64 + source_app + provider_id + model_id)
//!   → diagnose-screenshot (Agent 节点，调用 screenshot_diagnosis_create_from_image 工具)
//!   → end
//! ```
//!
//! Agent 节点拿到工具返回的 screenshot_diagnoses::Model 后，
//! 直接当作工作流输出，由前端展示。
//! 用户在前端点击「一键转为模拟观察组合」按钮时，
//! 再单独调 `screenshot_diagnosis_to_paper_portfolio` Tauri 命令完成 G2 联动。

use axagent_entities::workflow_template;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, JsonSchema, JsonSchemaProperty,
    OutputMode, Position, RetryConfig, ToolDef, TriggerConfig, TriggerNode, TriggerType, Variable,
    WorkflowEdge, WorkflowNode, WorkflowNodeBase,
};

const TEMPLATE_ID: &str = "screenshot-portfolio-diagnosis";
const TEMPLATE_VERSION: i32 = 1;

/// 种子化 screenshot-portfolio-diagnosis 工作流模板
pub async fn seed_screenshot_portfolio_diagnosis_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();

    // ── 触发器：手动触发，传入截图 base64 + 供应商 + 模型 ──
    let trigger_config = TriggerConfig {
        trigger_type: TriggerType::Manual,
        config: serde_json::json!({
            "description": "上传持仓截图，自动 OCR + 结构化解析 + 风险诊断",
            "required_params": ["image_base64", "provider_id", "model_id"],
            "param_schema": {
                "image_base64": {
                    "type": "string",
                    "description": "持仓截图 base64（支持 data:image/png;base64,XXX 或裸 base64）"
                },
                "source_app": {
                    "type": "string",
                    "description": "截图来源 App（同花顺/东方财富/雪球/通达信/其他），可选"
                },
                "provider_id": { "type": "string", "description": "LLM 供应商 ID" },
                "model_id": { "type": "string", "description": "视觉模型 ID（如 gpt-4o）" }
            }
        }),
    };

    // ── Agent 可用工具：1 个截图诊断工具 ──
    let mut image_props = std::collections::HashMap::new();
    image_props.insert(
        "image_base64".into(),
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some("持仓截图 base64（data:image/png;base64,XXX 或裸 base64）".into()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    image_props.insert(
        "source_app".into(),
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some("截图来源 App（同花顺/东方财富/雪球/通达信/其他）".into()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    image_props.insert(
        "provider_id".into(),
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some("LLM 供应商 ID".into()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    image_props.insert(
        "model_id".into(),
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some("视觉模型 ID（如 gpt-4o）".into()),
            default: None,
            enum_values: None,
            format: None,
        },
    );

    let agent_tools: Vec<ToolDef> = vec![ToolDef {
        name: "screenshot_diagnosis_create_from_image".into(),
        description: Some(
            "上传持仓截图自动诊断：OCR + 结构化持仓解析 + 7 项风险指标 + LLM 中文诊断说明 + 持久化"
                .into(),
        ),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(image_props),
            required: Some(vec!["image_base64".into(), "provider_id".into(), "model_id".into()]),
            items: None,
        }),
    }];

    // ── 节点定义 ──
    let nodes: Vec<WorkflowNode> = vec![
        // 1. 触发器：手动上传截图
        WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                id: "trigger".into(),
                title: "上传持仓截图".into(),
                description: Some("手动触发：传入截图 base64 + 供应商 + 模型".into()),
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
        // 2. Agent 节点：调用 screenshot_diagnosis_create_from_image 工具完成全流程
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: "diagnose-screenshot".into(),
                title: "截图持仓诊断".into(),
                description: Some(
                    "调用 screenshot_diagnosis_create_from_image 工具完成 OCR + 结构化 + 风险诊断 + 持久化".into(),
                ),
                position: Position { x: 20.0, y: 180.0 },
                retry: RetryConfig { enabled: true, max_retries: 0, ..Default::default() },
                timeout: Some(600), // 10 分钟超时（视觉模型 + LLM 双调用）
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: r#"调用 `screenshot_diagnosis_create_from_image` 工具完成截图诊断全流程。

【输入参数】
- image_base64: {{image_base64}}
- source_app: {{source_app}}（可选，若空传 null）
- provider_id: {{provider_id}}
- model_id: {{model_id}}

请根据持仓截图诊断方法论完成任务，直接输出工具返回的 JSON 结果。"#.into(),
                context_sources: vec!["trigger".into()],
                input_mapping: [
                    ("image_base64".to_string(), "image_base64".to_string()),
                    ("source_app".to_string(), "source_app".to_string()),
                    ("provider_id".to_string(), "provider_id".to_string()),
                    ("model_id".to_string(), "model_id".to_string()),
                ]
                .into_iter()
                .collect(),
                output_var: "diagnosis-result".into(),
                model: None,
                temperature: Some(0.0),
                max_tokens: Some(4096),
                tools: agent_tools,
                exposed_tools: vec![],
                output_mode: OutputMode::Json,
                agent_profile_id: Some("stock-screenshot-diagnoser".into()),
                max_tool_rounds: Some(2), // 限制为 2 轮（1 轮工具调用 + 1 轮输出）
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
            config: EndNodeConfig { output_var: Some("diagnosis-result".into()) },
        }),
    ];

    let edges: Vec<WorkflowEdge> = vec![
        WorkflowEdge {
            id: "e-trigger-agent".into(),
            source: "trigger".into(),
            source_handle: None,
            target: "diagnose-screenshot".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-agent-end".into(),
            source: "diagnose-screenshot".into(),
            source_handle: None,
            target: "end".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
    ];

    // ── 模板变量（运行时由触发器填充）──
    let variables: Vec<Variable> = vec![
        Variable {
            name: "image_base64".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(String::new()),
            description: Some("持仓截图 base64".into()),
            is_secret: false,
        },
        Variable {
            name: "source_app".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(String::new()),
            description: Some("截图来源 App（可选）".into()),
            is_secret: false,
        },
        Variable {
            name: "provider_id".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(String::new()),
            description: Some("LLM 供应商 ID".into()),
            is_secret: false,
        },
        Variable {
            name: "model_id".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(String::new()),
            description: Some("视觉模型 ID".into()),
            is_secret: false,
        },
    ];

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
    let tags =
        serde_json::to_string(&["截图诊断".to_string(), "持仓分析".to_string(), "G6".to_string()])
            .map_err(|e| format!("序列化标签失败: {e}"))?;

    // 先删再插
    let _ = workflow_template::Entity::delete_by_id(TEMPLATE_ID).exec(db).await;
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.into()),
        cluster_id: Set(Some("portfolio".to_string())),
        route_path: Set(Some("/finance/portfolio/diagnosis".to_string())),
        name: Set("截图持仓诊断".into()),
        description: Set(Some(
            "上传券商 App 持仓截图，自动 OCR + 结构化解析 + 7 项风险指标 + LLM 中文诊断说明，可一键转为模拟观察组合（G2 联动）".into(),
        )),
        icon: Set("📸".into()),
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
        "[stock_analysis_setup] G6 screenshot-portfolio-diagnosis 模板已创建 (v{TEMPLATE_VERSION})"
    );
    Ok(())
}
