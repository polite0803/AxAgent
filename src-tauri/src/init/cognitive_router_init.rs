// SPDX-License-Identifier: AGPL-3.0-only

//! 认知编排器初始化
//!
//! 认知编排器由主工作流 + L1/L2/L3 三个子工作流模板组成。
//! 所有模板存储在 workflow_templates 表中，通过 is_preset=true + 标签 "cognitive_router"
//! + visibility=SystemOnly 实现与业务工作流的隔离。
//!
//! 架构（三层分层路由）：
//! ```text
//! cognitive_router_main (主工作流, ~20 节点)
//!     ├── SubWorkflowNode → cognitive_l1_domain_router (L1 域路由, ~10 节点)
//!     ├── SubWorkflowNode → cognitive_l2_cluster_router (L2 簇路由, ~8 节点)
//!     └── SubWorkflowNode → cognitive_l3_capability_router (L3 能力路由, ~15 节点)
//! ```

use sea_orm::DatabaseConnection;

use axagent_dao::repo::workflow_template::{
    build_active_model_from_data, get_workflow_template, insert_workflow_template,
};
use axagent_harness::workflow_types::{
    BackoffType, CompareOperator, Condition, ConditionNode, ConditionNodeConfig,
    DataTransformerNode, DataTransformerNodeConfig, EndNode, EndNodeConfig, LlmClassifierNode,
    LlmClassifierNodeConfig, LogicalOperator, Position, RetryConfig, SubWorkflowNode,
    SubWorkflowNodeConfig, SwitchCase, SwitchNode, SwitchNodeConfig, ValidationAssertion,
    ValidationNode, ValidationNodeConfig, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
    WorkflowTemplateData,
};

/// 认知编排器模板 ID 常量
pub const COGNITIVE_ROUTER_MAIN_ID: &str = "cognitive_router_main";
pub const COGNITIVE_L1_DOMAIN_ROUTER_ID: &str = "cognitive_l1_domain_router";
pub const COGNITIVE_L2_CLUSTER_ROUTER_ID: &str = "cognitive_l2_cluster_router";
pub const COGNITIVE_L3_CAPABILITY_ROUTER_ID: &str = "cognitive_l3_capability_router";

/// 认知编排器标签（用于隔离和筛选）
pub const COGNITIVE_ROUTER_TAG: &str = "cognitive_router";

/// 初始化认知编排器（4 个工作流模板，共 ~53 个节点）
pub async fn ensure_cognitive_router_templates(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 创建 L1 域路由子工作流（~10 节点）
    ensure_template(
        db,
        COGNITIVE_L1_DOMAIN_ROUTER_ID,
        "L1 域路由",
        "识别用户输入所属的业务域（规则匹配 + LLM 兜底 + 置信度评估）",
        build_l1_router_nodes(),
        build_l1_router_edges(),
    )
    .await?;

    // 2. 创建 L2 簇路由子工作流（~8 节点）
    ensure_template(
        db,
        COGNITIVE_L2_CLUSTER_ROUTER_ID,
        "L2 簇路由",
        "在业务域内匹配具体的能力簇（规则匹配 + LLM 兜底）",
        build_l2_router_nodes(),
        build_l2_router_edges(),
    )
    .await?;

    // 3. 创建 L3 能力路由子工作流（~15 节点）
    ensure_template(
        db,
        COGNITIVE_L3_CAPABILITY_ROUTER_ID,
        "L3 能力路由",
        "选择具体的执行策略（RAR 检索 + 图谱路由 + 熔断检查 + 执行模式决策）",
        build_l3_router_nodes(),
        build_l3_router_edges(),
    )
    .await?;

    // 4. 创建主认知编排器工作流（~20 节点）
    ensure_template(
        db,
        COGNITIVE_ROUTER_MAIN_ID,
        "认知编排器",
        "系统级路由编排器，协调 L1/L2/L3 三层路由，支持熔断保护和物理隔离",
        build_main_router_nodes(),
        build_main_router_edges(),
    )
    .await?;

    tracing::info!("[cognitive_router] 认知编排器初始化完成（4 个工作流模板，共约 53 个节点）");
    Ok(())
}

/// 确保模板存在（不存在则创建，存在则跳过）
async fn ensure_template(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: &str,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
) -> Result<(), String> {
    let existing = get_workflow_template(db, id).await.map_err(|e| e.to_string())?;

    if existing.is_some() {
        tracing::debug!("[cognitive_router] 模板 {} 已存在，跳过", id);
        return Ok(());
    }

    tracing::info!(
        "[cognitive_router] 创建模板: {} ({} 节点, {} 边)",
        name,
        nodes.len(),
        edges.len()
    );

    let now = chrono::Utc::now().timestamp_millis();
    let template = WorkflowTemplateData {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        icon: "🤖".to_string(),
        tags: vec![COGNITIVE_ROUTER_TAG.to_string()],
        version: 1,
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: axagent_harness::capability::Visibility::SystemOnly,
        trigger_config: None,
        nodes,
        edges,
        input_schema: None,
        output_schema: None,
        variables: Vec::new(),
        error_config: None,
        error_workflow_id: None,
        tool_defs: Vec::new(),
        mission_hash: None,
        created_at: now,
        updated_at: now,
    };

    let active = build_active_model_from_data(&template);
    insert_workflow_template(db, active).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════

fn base(id: &str, title: &str, pos: Position) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.to_string(),
        title: title.to_string(),
        description: None,
        position: pos,
        retry: RetryConfig {
            enabled: true,
            max_retries: 2,
            backoff_type: BackoffType::Exponential,
            base_delay_ms: 500,
            max_delay_ms: 8000,
        },
        timeout: Some(30000),
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

fn simple_base(id: &str, title: &str, pos: Position) -> WorkflowNodeBase {
    let mut b = base(id, title, pos);
    b.retry = RetryConfig::default();
    b.timeout = None;
    b
}

/// 构建单个条件（表达式形式：var_path + 运算符 + 值）
fn cond(var_path: &str, operator: CompareOperator, value: serde_json::Value) -> Condition {
    Condition { var_path: var_path.to_string(), operator, value }
}

/// 构建表达式断言（用于 Validation 节点的规则）
fn assertion(expression: &str) -> ValidationAssertion {
    ValidationAssertion {
        assertion_type: "expression".to_string(),
        expected: None,
        actual: None,
        expression: Some(expression.to_string()),
    }
}

/// 构建条件节点（默认 And 逻辑，不启用 LLM 路由）
fn condition_node(
    id: &str,
    title: &str,
    pos: Position,
    conditions: Vec<Condition>,
) -> WorkflowNode {
    condition_node_op(id, title, pos, conditions, LogicalOperator::And)
}

/// 构建条件节点（可指定 And/Or 逻辑）
fn condition_node_op(
    id: &str,
    title: &str,
    pos: Position,
    conditions: Vec<Condition>,
    logical_op: LogicalOperator,
) -> WorkflowNode {
    WorkflowNode::Condition(ConditionNode {
        base: simple_base(id, title, pos),
        config: ConditionNodeConfig {
            conditions,
            logical_op,
            judge_by_llm: None,
            routing_prompt: None,
            routing_model: None,
            confidence_threshold: None,
        },
    })
}

/// 构建验证节点（规则列表 + on_fail 目标）
fn validation_node(
    id: &str,
    title: &str,
    pos: Position,
    rules: Vec<&str>,
    on_fail: &str,
) -> WorkflowNode {
    WorkflowNode::Validation(ValidationNode {
        base: base(id, title, pos),
        config: ValidationNodeConfig {
            assertions: rules.into_iter().map(assertion).collect(),
            on_fail: on_fail.to_string(),
            max_retries: 0,
        },
    })
}

/// 构建子工作流调用节点
fn sub_workflow_node(
    id: &str,
    title: &str,
    pos: Position,
    sub_workflow_id: &str,
    input_mapping: Vec<(&str, &str)>,
    output_var: &str,
) -> WorkflowNode {
    WorkflowNode::SubWorkflow(SubWorkflowNode {
        base: base(id, title, pos),
        config: SubWorkflowNodeConfig {
            sub_workflow_id: sub_workflow_id.to_string(),
            input_mapping: input_mapping
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            output_var: output_var.to_string(),
            is_async: false,
            sub_graph: None,
        },
    })
}

/// 构建 LLM 分类器节点
#[allow(clippy::too_many_arguments)]
fn llm_classifier_node(
    id: &str,
    title: &str,
    pos: Position,
    categories: Vec<&str>,
    categories_var: Option<&str>,
    prompt: &str,
    input_var: &str,
    output_var: &str,
    confidence_threshold: Option<f64>,
    fallback_label: Option<&str>,
) -> WorkflowNode {
    WorkflowNode::LlmClassifier(LlmClassifierNode {
        base: base(id, title, pos),
        config: LlmClassifierNodeConfig {
            categories: categories.into_iter().map(str::to_string).collect(),
            categories_var: categories_var.map(str::to_string),
            prompt: prompt.to_string(),
            model: None,
            input_var: input_var.to_string(),
            output_var: output_var.to_string(),
            confidence_threshold,
            fallback_label: fallback_label.map(str::to_string),
            consistency_check: None,
        },
    })
}

/// 构建结束节点（带输出变量：结束节点把指定变量的值作为工作流最终输出）
fn end_node_with_var(
    id: &str,
    title: &str,
    pos: Position,
    output_var: Option<&str>,
) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: simple_base(id, title, pos),
        config: EndNodeConfig { output_var: output_var.map(str::to_string) },
    })
}

/// 构建 DataTransformer 节点（Rhai 表达式，可写 output_var）——
/// 认知编排器用它实现 L1/L2 规则匹配并归一化输出 `{category, confidence}` 对象。
fn data_transformer_node(
    id: &str,
    title: &str,
    pos: Position,
    input_var: &str,
    expression: &str,
    output_var: &str,
) -> WorkflowNode {
    WorkflowNode::DataTransformer(DataTransformerNode {
        base: simple_base(id, title, pos),
        config: DataTransformerNodeConfig {
            input_var: input_var.to_string(),
            expression: expression.to_string(),
            output_var: output_var.to_string(),
        },
    })
}

/// 构建直连边
fn direct_edge(id: &str, source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        source_handle: None,
        target: target.to_string(),
        target_handle: None,
        edge_type: axagent_harness::workflow_types::EdgeType::Direct,
        label: None,
    }
}

/// 构建条件边（true/false 分支）
fn branch_edge(
    id: &str,
    source: &str,
    target: &str,
    true_branch: bool,
    label: &str,
) -> WorkflowEdge {
    use axagent_harness::workflow_types::EdgeType;
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        source_handle: None,
        target: target.to_string(),
        target_handle: None,
        edge_type: if true_branch {
            EdgeType::ConditionTrue
        } else {
            EdgeType::ConditionFalse
        },
        label: Some(label.to_string()),
    }
}

// ═══════════════════════════════════════════════════════════════
// 主认知编排器工作流（~20 节点）
// ═══════════════════════════════════════════════════════════════

fn build_main_router_nodes() -> Vec<WorkflowNode> {
    vec![
        // 1. L1 子工作流调用
        //    h3：动态分类目录注入口 —— 把能力基座构建的 __l1_categories 透传给 L1 子工作流。
        sub_workflow_node(
            "call_l1",
            "调用 L1 域路由",
            Position { x: 250.0, y: 80.0 },
            COGNITIVE_L1_DOMAIN_ROUTER_ID,
            vec![("user_input", "user_input"), ("__l1_categories", "__l1_categories")],
            "l1_result",
        ),
        // 2. L1 结果验证
        validation_node(
            "validate_l1",
            "L1 结果验证",
            Position { x: 250.0, y: 200.0 },
            vec!["l1_result.category is not null", "l1_result.confidence >= 0.5"],
            "fallback",
        ),
        // 3. L1 低置信度兜底
        condition_node(
            "l1_low_conf",
            "L1 低置信度检查",
            Position { x: 100.0, y: 280.0 },
            vec![cond("l1_result.confidence", CompareOperator::Lt, serde_json::json!(0.7))],
        ),
        // 4. L1 兜底（LLM 重新分类，结果覆盖 l1_result 供下游统一消费）
        llm_classifier_node(
            "l1_fallback",
            "L1 LLM 兜底分类",
            Position { x: 100.0, y: 380.0 },
            vec![
                "general",
                "devops",
                "ai_media",
                "finance",
                "automation",
                "data_analysis",
                "content_creation",
                "communication",
            ],
            Some("__l1_categories"),
            "你是一个业务域分类器。根据用户输入，将其归类到最合适的业务域。\n\n业务域列表：\n- general: 通用能力（文件读写、Shell、文本、网络、搜索、文档、配置等兜底通用能力）\n- devops: 运维（CI/CD、部署、监控告警、安全审计、容器编排）\n- ai_media: AI 媒体（图像、视频、音频的生成与处理）\n- finance: 金融（行情、交易、风控、组合管理）\n- automation: 自动化（RPA、定时任务、工作流编排）\n- data_analysis: 数据分析（SQL 查询、可视化、ETL/数据清洗）\n- content_creation: 内容创作（写作、设计、排版）\n- communication: 通信（IM、邮件、推送通知）\n\n用户输入：{user_input}\n\n请返回 JSON 格式：{\"label\": \"xxx\", \"confidence\": 0.xx}",
            "user_input",
            "l1_result",
            Some(0.6),
            Some("general"),
        ),
        // 5. L2 子工作流调用（l1_domain 取 l1_result.category；透传动态目录 __l2_categories）
        sub_workflow_node(
            "call_l2",
            "调用 L2 簇路由",
            Position { x: 250.0, y: 480.0 },
            COGNITIVE_L2_CLUSTER_ROUTER_ID,
            vec![
                ("user_input", "user_input"),
                ("l1_domain", "l1_result.category"),
                ("__l2_categories", "__l2_categories"),
            ],
            "l2_result",
        ),
        // 6. L2 结果验证
        validation_node(
            "validate_l2",
            "L2 结果验证",
            Position { x: 250.0, y: 600.0 },
            vec!["l2_result.category is not null", "l2_result.confidence >= 0.4"],
            "fallback",
        ),
        // 7. L2 低置信度检查
        condition_node(
            "l2_low_conf",
            "L2 低置信度检查",
            Position { x: 100.0, y: 680.0 },
            vec![cond("l2_result.confidence", CompareOperator::Lt, serde_json::json!(0.6))],
        ),
        // 8. L2 兜底
        llm_classifier_node(
            "l2_fallback",
            "L2 LLM 兜底分类",
            Position { x: 100.0, y: 780.0 },
            vec![
                "file_management",
                "config_management",
                "code_development",
                "ci_cd",
                "monitoring",
                "trading",
                "market_data",
                "analysis",
                "order_management",
                "refund_processing",
                "writing",
                "translation",
                "learning",
                "memory",
                "ticket_handling",
            ],
            Some("__l2_categories"),
            "你是一个能力簇分类器。在已识别的业务域下，选择最合适的能力簇。\n\n业务域：{l1_domain}\n用户输入：{user_input}\n\n请返回 JSON：{\"cluster\": \"xxx\", \"confidence\": 0.xx}",
            "user_input",
            "l2_fallback_result",
            Some(0.5),
            Some("general"),
        ),
        // 9. L3 子工作流调用（RAR + 图谱 + 熔断；透传字符串 l1/l2 category）
        sub_workflow_node(
            "call_l3",
            "调用 L3 能力路由",
            Position { x: 250.0, y: 880.0 },
            COGNITIVE_L3_CAPABILITY_ROUTER_ID,
            vec![
                ("user_input", "user_input"),
                ("l1_domain", "l1_result.category"),
                ("l2_cluster", "l2_result.category"),
            ],
            "l3_result",
        ),
        // 10. 熔断检查
        condition_node(
            "circuit_breaker",
            "熔断检查",
            Position { x: 500.0, y: 880.0 },
            vec![cond("l3_result.is_circuit_broken", CompareOperator::Eq, serde_json::json!(true))],
        ),
        // 11. 熔断触发结束（仍输出 l3_result，is_circuit_broken=true 供上层决策消费）
        end_node_with_var(
            "circuit_broken_end",
            "熔断保护结束",
            Position { x: 700.0, y: 880.0 },
            Some("l3_result"),
        ),
        // 12. 执行模式决策
        WorkflowNode::Switch(SwitchNode {
            base: simple_base("execution_mode", "执行模式决策", Position { x: 250.0, y: 1000.0 }),
            config: SwitchNodeConfig {
                input_var: "l3_result.execution_mode".to_string(),
                cases: vec![
                    SwitchCase { value: "direct".to_string(), label: "直接执行".to_string() },
                    SwitchCase {
                        value: "workflow".to_string(), label: "工作流执行".to_string()
                    },
                    SwitchCase { value: "delegate".to_string(), label: "代理执行".to_string() },
                    SwitchCase { value: "ask".to_string(), label: "问答模式".to_string() },
                    SwitchCase { value: "plan".to_string(), label: "计划模式".to_string() },
                    SwitchCase { value: "act".to_string(), label: "行动模式".to_string() },
                    SwitchCase { value: "clarify".to_string(), label: "澄清选择".to_string() },
                ],
                default_case: Some("ask".to_string()),
                match_mode: "exact".to_string(),
                use_llm: None,
                llm_prompt: None,
                llm_model: None,
                // 执行模式决策仅用于分支跳转，最终决策由 EndNode 以 l3_result 输出；
                // output_var 留空避免产生未被消费的 final_decision 死变量。
                output_var: String::new(),
            },
        }),
        // 13. 最终结束节点（输出 l3_result，作为主 DAG 的最终路由决策）
        end_node_with_var("end", "路由完成", Position { x: 250.0, y: 1120.0 }, Some("l3_result")),
    ]
}

fn build_main_router_edges() -> Vec<WorkflowEdge> {
    vec![
        direct_edge("e1", "call_l1", "validate_l1"),
        direct_edge("e2", "validate_l1", "l1_low_conf"),
        branch_edge("e3", "l1_low_conf", "l1_fallback", true, "低置信度"),
        branch_edge("e4", "l1_low_conf", "call_l2", false, "通过"),
        direct_edge("e5", "l1_fallback", "call_l2"),
        direct_edge("e6", "call_l2", "validate_l2"),
        direct_edge("e7", "validate_l2", "l2_low_conf"),
        branch_edge("e8", "l2_low_conf", "l2_fallback", true, "低置信度"),
        branch_edge("e9", "l2_low_conf", "call_l3", false, "通过"),
        direct_edge("e10", "l2_fallback", "call_l3"),
        direct_edge("e11", "call_l3", "circuit_breaker"),
        branch_edge("e12", "circuit_breaker", "circuit_broken_end", true, "熔断触发"),
        branch_edge("e13", "circuit_breaker", "execution_mode", false, "正常"),
        direct_edge("e14", "execution_mode", "end"),
    ]
}

// ═══════════════════════════════════════════════════════════════
// L1 域路由子工作流（~10 节点）
// ═══════════════════════════════════════════════════════════════

/// L1 规则匹配 Rhai 表达式 —— 仅依赖 `input`（user_input），命中返回 `{hit, category}`。
///
/// 分类目录与 `CapabilityDomain` 枚举及 `list_l1_categories` 动态目录保持命名一致：
/// 订单/退款等电商操作归入 `automation`（自动化域），知识库/学习/记忆归入 `general`（通用域），
/// 否则规则命中的域不在动态目录内会导致 L2 分类目录为空。
const L1_RULE_EXPRESSION: &str = r#"
if input.contains("股票") || input.contains("基金") || input.contains("投资") || input.contains("行情") {
    #{ "hit": true, "category": "finance" }
} else if input.contains("订单") || input.contains("退款") || input.contains("发货") || input.contains("物流") {
    #{ "hit": true, "category": "automation" }
} else if input.contains("写") || input.contains("翻译") || input.contains("润色") || input.contains("生成") {
    #{ "hit": true, "category": "content_creation" }
} else if input.contains("文件") || input.contains("目录") || input.contains("读写") {
    #{ "hit": true, "category": "general" }
} else if input.contains("部署") || input.contains("监控") || input.contains("CI/CD") || input.contains("Docker") {
    #{ "hit": true, "category": "devops" }
} else if input.contains("知识库") || input.contains("学习") || input.contains("记忆") {
    #{ "hit": true, "category": "general" }
} else {
    #{ "hit": false, "category": "" }
}
"#;

/// L1 规则命中归一化 —— 将 `{hit, category}` 转换为统一的 `{category, confidence}` 结构。
const L1_RULE_NORMALIZE_EXPRESSION: &str = r#"
#{ "category": l1_rule_result.category, "confidence": 0.95 }
"#;

fn build_l1_router_nodes() -> Vec<WorkflowNode> {
    vec![
        // 1. 关键词规则匹配（Rhai 表达式 → {hit, category}）
        data_transformer_node(
            "l1_rule_match",
            "L1 规则匹配",
            Position { x: 100.0, y: 80.0 },
            "user_input",
            L1_RULE_EXPRESSION,
            "l1_rule_result",
        ),
        // 2. 规则命中检查
        condition_node(
            "l1_rule_hit",
            "规则命中检查",
            Position { x: 100.0, y: 200.0 },
            vec![cond("l1_rule_result.hit", CompareOperator::Eq, serde_json::json!(true))],
        ),
        // 3. 规则命中归一化（{category, confidence} → l1_result）
        data_transformer_node(
            "l1_rule_normalize",
            "规则命中归一化",
            Position { x: 300.0, y: 200.0 },
            "l1_rule_result",
            L1_RULE_NORMALIZE_EXPRESSION,
            "l1_result",
        ),
        // 4. 规则命中结束（输出 l1_result）
        end_node_with_var(
            "l1_rule_end",
            "规则命中结束",
            Position { x: 500.0, y: 200.0 },
            Some("l1_result"),
        ),
        // 5. LLM 兜底分类（动态分类目录注入口：categories_var=__l1_categories）
        llm_classifier_node(
            "l1_llm",
            "L1 LLM 分类",
            Position { x: 100.0, y: 320.0 },
            vec![
                "general",
                "devops",
                "ai_media",
                "finance",
                "automation",
                "data_analysis",
                "content_creation",
                "communication",
            ],
            Some("__l1_categories"),
            "你是一个业务域分类器。根据用户输入，将其归类到最合适的业务域。\n\n业务域列表：\n- general: 通用能力（文件读写、Shell、文本、网络、搜索、文档、配置等兜底通用能力）\n- devops: 运维（CI/CD、部署、监控告警、安全审计、容器编排）\n- ai_media: AI 媒体（图像、视频、音频的生成与处理）\n- finance: 金融（行情、交易、风控、组合管理）\n- automation: 自动化（RPA、定时任务、工作流编排）\n- data_analysis: 数据分析（SQL 查询、可视化、ETL/数据清洗）\n- content_creation: 内容创作（写作、设计、排版）\n- communication: 通信（IM、邮件、推送通知）\n\n用户输入：{user_input}\n\n返回 JSON：{\"label\": \"业务域\", \"confidence\": 0.xx}",
            "user_input",
            "l1_llm_result",
            Some(0.6),
            Some("general"),
        ),
        // 6. LLM 结果置信度检查
        condition_node(
            "l1_llm_conf",
            "LLM 置信度检查",
            Position { x: 100.0, y: 440.0 },
            vec![cond("l1_llm_result.confidence", CompareOperator::Gte, serde_json::json!(0.6))],
        ),
        // 7. LLM 成功（输出 l1_llm_result，含 category/confidence）
        end_node_with_var(
            "l1_llm_end",
            "L1 LLM 分类成功",
            Position { x: 300.0, y: 440.0 },
            Some("l1_llm_result"),
        ),
        // 8. LLM 低置信度（仍输出结果，主 DAG 的 l1_low_conf 会二次兜底）
        end_node_with_var(
            "l1_low_conf_end",
            "L1 LLM 低置信度",
            Position { x: 100.0, y: 560.0 },
            Some("l1_llm_result"),
        ),
    ]
}

fn build_l1_router_edges() -> Vec<WorkflowEdge> {
    vec![
        direct_edge("e1", "l1_rule_match", "l1_rule_hit"),
        branch_edge("e2", "l1_rule_hit", "l1_rule_normalize", true, "规则命中"),
        branch_edge("e3", "l1_rule_hit", "l1_llm", false, "规则未命中"),
        direct_edge("e4", "l1_rule_normalize", "l1_rule_end"),
        direct_edge("e5", "l1_llm", "l1_llm_conf"),
        branch_edge("e6", "l1_llm_conf", "l1_llm_end", true, "高置信度"),
        branch_edge("e7", "l1_llm_conf", "l1_low_conf_end", false, "低置信度"),
    ]
}

// ═══════════════════════════════════════════════════════════════
// L2 簇路由子工作流（~8 节点）
// ═══════════════════════════════════════════════════════════════

/// L2 规则匹配 Rhai 表达式 —— 依赖 `input`（user_input）+ `l1_domain`。
const L2_RULE_EXPRESSION: &str = r#"
if l1_domain == "finance" && (input.contains("技术面") || input.contains("K线") || input.contains("均线")) {
    #{ "hit": true, "category": "stock_tech" }
} else if l1_domain == "finance" && (input.contains("基本面") || input.contains("PE") || input.contains("ROE")) {
    #{ "hit": true, "category": "stock_fundamental" }
} else if l1_domain == "finance" && (input.contains("新闻") || input.contains("舆情") || input.contains("公告")) {
    #{ "hit": true, "category": "stock_news" }
} else if l1_domain == "automation" && (input.contains("退款") || input.contains("退货")) {
    #{ "hit": true, "category": "refund_processing" }
} else if l1_domain == "automation" && (input.contains("订单") || input.contains("修改")) {
    #{ "hit": true, "category": "order_modify" }
} else if l1_domain == "devops" && (input.contains("部署") || input.contains("上线")) {
    #{ "hit": true, "category": "deployment" }
} else if l1_domain == "devops" && (input.contains("监控") || input.contains("告警")) {
    #{ "hit": true, "category": "monitoring" }
} else {
    #{ "hit": false, "category": "" }
}
"#;

/// L2 规则命中归一化 —— 将 `{hit, category}` 转换为统一的 `{category, confidence}` 结构。
const L2_RULE_NORMALIZE_EXPRESSION: &str = r#"
#{ "category": l2_rule_result.category, "confidence": 0.95 }
"#;

fn build_l2_router_nodes() -> Vec<WorkflowNode> {
    vec![
        // 1. L2 规则匹配（Rhai 表达式，依赖 l1_domain + user_input → {hit, category}）
        data_transformer_node(
            "l2_rule_match",
            "L2 规则匹配",
            Position { x: 100.0, y: 80.0 },
            "user_input",
            L2_RULE_EXPRESSION,
            "l2_rule_result",
        ),
        // 2. L2 规则命中检查
        condition_node(
            "l2_rule_hit",
            "L2 规则命中",
            Position { x: 100.0, y: 200.0 },
            vec![cond("l2_rule_result.hit", CompareOperator::Eq, serde_json::json!(true))],
        ),
        // 3. 规则命中归一化（{category, confidence} → l2_result）
        data_transformer_node(
            "l2_rule_normalize",
            "规则命中归一化",
            Position { x: 300.0, y: 200.0 },
            "l2_rule_result",
            L2_RULE_NORMALIZE_EXPRESSION,
            "l2_result",
        ),
        // 4. 规则命中结束（输出 l2_result）
        end_node_with_var(
            "l2_rule_end",
            "规则命中结束",
            Position { x: 500.0, y: 200.0 },
            Some("l2_result"),
        ),
        // 5. L2 LLM 兜底分类（动态分类目录注入口：categories_var=__l2_categories）
        llm_classifier_node(
            "l2_llm",
            "L2 LLM 分类",
            Position { x: 100.0, y: 320.0 },
            vec![
                "file_management",
                "config_management",
                "code_development",
                "ci_cd",
                "monitoring",
                "deployment",
                "trading",
                "market_data",
                "stock_tech",
                "stock_fundamental",
                "stock_news",
                "stock_risk",
                "analysis",
                "order_management",
                "refund_processing",
                "shipping",
                "marketing",
                "writing",
                "translation",
                "polishing",
                "learning",
                "memory",
                "knowledge_base",
                "ticket_handling",
            ],
            Some("__l2_categories"),
            "你是一个能力簇分类器。在已识别的业务域内选择最合适的能力簇。\n\n业务域：{l1_domain}\n用户输入：{user_input}\n\n返回 JSON：{\"label\": \"能力簇\", \"confidence\": 0.xx}",
            "user_input",
            "l2_llm_result",
            Some(0.5),
            Some("general"),
        ),
        // 6. L2 LLM 结果置信度
        condition_node(
            "l2_llm_conf",
            "L2 LLM 置信度",
            Position { x: 100.0, y: 440.0 },
            vec![cond("l2_llm_result.confidence", CompareOperator::Gte, serde_json::json!(0.4))],
        ),
        // 7. L2 LLM 成功（输出 l2_llm_result，含 category/confidence）
        end_node_with_var(
            "l2_llm_end",
            "L2 LLM 分类成功",
            Position { x: 300.0, y: 440.0 },
            Some("l2_llm_result"),
        ),
        // 8. L2 低置信度（仍输出结果，主 DAG 的 l2_low_conf 会二次兜底）
        end_node_with_var(
            "l2_low_conf_end",
            "L2 LLM 低置信度",
            Position { x: 100.0, y: 560.0 },
            Some("l2_llm_result"),
        ),
    ]
}

fn build_l2_router_edges() -> Vec<WorkflowEdge> {
    vec![
        direct_edge("e1", "l2_rule_match", "l2_rule_hit"),
        branch_edge("e2", "l2_rule_hit", "l2_rule_normalize", true, "规则命中"),
        branch_edge("e3", "l2_rule_hit", "l2_llm", false, "规则未命中"),
        direct_edge("e4", "l2_rule_normalize", "l2_rule_end"),
        direct_edge("e5", "l2_llm", "l2_llm_conf"),
        branch_edge("e6", "l2_llm_conf", "l2_llm_end", true, "高置信度"),
        branch_edge("e7", "l2_llm_conf", "l2_low_conf_end", false, "低置信度"),
    ]
}

// ═══════════════════════════════════════════════════════════════
// L3 能力路由子工作流（~8 节点）
// ═══════════════════════════════════════════════════════════════
//
// 由系统能力承接核心路由逻辑：
//   - system_rar_retriever：RAR 向量检索，返回 `{candidates: [...], count: N}`
//     （候选对象含 id/name 等字段，系统能力内部已过滤编排器自身等系统能力）
//   - system_workflow_graph_router：图谱路径规划，输入选中能力，返回
//     `{route_path, capability_id, confidence, execution_mode, circuit_broken, reason}`
//
// 熔断策略（双层）：
//   1. 系统能力层：RAR 检索/图谱路由返回的 circuit_broken 字段（自指自检在系统能力内部完成）
//   2. DAG 层：normalize_l3 归一化时用 Rhai 对 capability_id 做编排器关键字二次防线

/// L3 RAR 候选选择 prompt（LLM 从 RAR 检索候选中选 1，输出 JSON 含 label/confidence/execution_mode）
const L3_SELECT_PROMPT: &str = r#"你是一个 L3 工作流选择器。根据用户输入，从候选工作流中选择最匹配的一个。

用户输入：{user_input}
业务域：{l1_domain}
能力簇：{l2_cluster}
候选项：{rar_candidates}

请返回 JSON：{"label": "候选id", "confidence": 0.xx, "execution_mode": "direct|workflow|delegate|ask|plan|act"}
只输出 JSON，不要包含任何其他内容。"#;

/// L3 结果归一化 Rhai 表达式 —— 汇总选中能力 + 图谱路径，输出统一决策对象。
///
/// 字段来源：
/// - `selected_capability`：有候选路径（rar_llm_select 选中）或无候选路径（l2_cluster 兜底）
/// - `graph_path`：system_workflow_graph_router 返回的路径对象
/// - `l1_domain` / `l2_cluster`：上游透传
///
/// 自指熔断二次防线：capability_id 命中编排器关键字（cognitive_router / orchestrator /
/// system_ 前缀）时强制 is_circuit_broken=true。
///
/// 候选列表透传：`rar_candidates.candidates` 原样注入输出的 `candidates` 字段，
/// 供主 DAG clarify 分支展示 Top2 候选给用户选择（含 id/name/description/score）。
/// RAR 无候选时兜底构造单候选（以 l2_cluster / l1_domain 为 ID），保证 clarify 分支有项可展示。
///
/// 观测字段透传：`is_llm_fallback` / `fallback_path` / `stage_records` 从图谱路由输出
/// 原样透传，供 cognitive_query 填充响应中的观测视图。
const L3_NORMALIZE_EXPRESSION: &str = r#"
let cap = selected_capability;
let self_ref = cap.contains("cognitive_router") || cap.contains("orchestrator") || cap.starts_with("system_");
let mode = if graph_path.execution_mode == () { "ask" } else { graph_path.execution_mode };
let broken = graph_path.circuit_broken == true || self_ref;
let reason = if self_ref { "self_reference" } else if graph_path.circuit_broken == true { graph_path.reason } else { "" };
let llm_fb = if graph_path.is_llm_fallback == () { false } else { graph_path.is_llm_fallback };
let fb_path = if graph_path.fallback_path == () { "" } else { graph_path.fallback_path };
let stages = if graph_path.stage_records == () { [] } else { graph_path.stage_records };
let cands = if rar_candidates == () || rar_candidates.candidates == () || rar_candidates.candidates.len() == 0 {
    let fb_id = if l2_cluster == "" { l1_domain } else { l2_cluster };
    [#{ "id": fb_id, "name": fb_id, "description": "", "score": graph_path.confidence }]
} else {
    rar_candidates.candidates
};
#{
    "route_path": graph_path.route_path,
    "domain": l1_domain,
    "cluster": l2_cluster,
    "capability_id": cap,
    "confidence": graph_path.confidence,
    "is_circuit_broken": broken,
    "execution_mode": mode,
    "reason": reason,
    "candidates": cands,
    "is_llm_fallback": llm_fb,
    "fallback_path": fb_path,
    "stage_records": stages
}
"#;

fn build_l3_router_nodes() -> Vec<WorkflowNode> {
    vec![
        // 1. RAR 向量检索（系统能力，返回 {candidates, count}）
        sub_workflow_node(
            "l3_rar_search",
            "RAR 向量检索",
            Position { x: 100.0, y: 80.0 },
            "system_rar_retriever",
            vec![
                ("user_input", "user_input"),
                ("l1_domain", "l1_domain"),
                ("l2_cluster", "l2_cluster"),
            ],
            "rar_candidates",
        ),
        // 2. RAR 结果检查（count > 0 → 有候选走 LLM 选择；否则走簇兜底）
        condition_node(
            "rar_has_results",
            "RAR 结果检查",
            Position { x: 100.0, y: 200.0 },
            vec![cond("rar_candidates.count", CompareOperator::Gt, serde_json::json!(0))],
        ),
        // 3. RAR LLM 选择（5 选 1；动态目录注入口 = RAR 候选列表）
        llm_classifier_node(
            "rar_llm_select",
            "RAR LLM 选择",
            Position { x: 100.0, y: 320.0 },
            vec!["general", "assistant", "search", "analysis", "automation"],
            Some("rar_candidates.candidates"),
            L3_SELECT_PROMPT,
            "user_input",
            "rar_selected",
            Some(0.5),
            None,
        ),
        // 4. 有候选路径：提取选中能力 id（rar_selected.category → selected_capability）
        data_transformer_node(
            "condense_selected",
            "提取选中能力",
            Position { x: 320.0, y: 320.0 },
            "rar_selected",
            "rar_selected.category",
            "selected_capability",
        ),
        // 5. 无候选路径：以 L2 能力簇作为兜底能力（避免引用未定义的 rar_selected）
        data_transformer_node(
            "fallback_cluster",
            "簇级兜底",
            Position { x: 320.0, y: 200.0 },
            "l2_cluster",
            "l2_cluster",
            "selected_capability",
        ),
        // 6. 图谱路由（系统能力，输入选中能力，返回路径 + 熔断标记）
        sub_workflow_node(
            "graph_router",
            "图谱路由",
            Position { x: 200.0, y: 440.0 },
            "system_workflow_graph_router",
            vec![
                ("selected_capability", "selected_capability"),
                ("l1_domain", "l1_domain"),
                ("l2_cluster", "l2_cluster"),
                ("user_input", "user_input"),
            ],
            "graph_path",
        ),
        // 7. 结果归一化（汇总 + 自指熔断二次防线 → l3_result）
        data_transformer_node(
            "normalize_l3",
            "L3 结果归一化",
            Position { x: 200.0, y: 560.0 },
            "graph_path",
            L3_NORMALIZE_EXPRESSION,
            "l3_result",
        ),
        // 8. L3 完成结束（输出 l3_result，主 DAG 的 circuit_breaker / execution_mode 消费）
        end_node_with_var(
            "l3_success",
            "L3 完成",
            Position { x: 200.0, y: 680.0 },
            Some("l3_result"),
        ),
    ]
}

fn build_l3_router_edges() -> Vec<WorkflowEdge> {
    vec![
        direct_edge("e1", "l3_rar_search", "rar_has_results"),
        branch_edge("e2", "rar_has_results", "rar_llm_select", true, "有候选"),
        branch_edge("e3", "rar_has_results", "fallback_cluster", false, "无候选"),
        direct_edge("e4", "rar_llm_select", "condense_selected"),
        direct_edge("e5", "condense_selected", "graph_router"),
        direct_edge("e6", "fallback_cluster", "graph_router"),
        direct_edge("e7", "graph_router", "normalize_l3"),
        direct_edge("e8", "normalize_l3", "l3_success"),
    ]
}
