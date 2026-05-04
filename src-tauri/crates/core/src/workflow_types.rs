//! Workflow type definitions
//!
//! This module defines the core types used in workflow execution,
//! including nodes, variables, triggers, and execution states.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub backoff_type: BackoffType,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_retries: 3,
            backoff_type: BackoffType::Exponential,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffType {
    Linear,
    Exponential,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub description: Option<String>,
    pub properties: Option<std::collections::HashMap<String, JsonSchemaProperty>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<JsonSchema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaProperty {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
    pub enum_values: Option<Vec<serde_json::Value>>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub var_type: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeBase {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: Position,
    pub retry: RetryConfig,
    pub timeout: Option<u64>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "schedule")]
    Schedule,
    #[serde(rename = "webhook")]
    Webhook,
    #[serde(rename = "event")]
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualTriggerConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTriggerConfig {
    pub cron: String,
    pub timezone: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTriggerConfig {
    pub path: String,
    pub method: String,
    pub auth_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTriggerConfig {
    pub event_type: String,
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    #[serde(rename = "researcher")]
    Researcher,
    #[serde(rename = "planner")]
    Planner,
    #[serde(rename = "developer")]
    Developer,
    #[serde(rename = "reviewer")]
    Reviewer,
    #[serde(rename = "synthesizer")]
    Synthesizer,
    #[serde(rename = "executor")]
    Executor,
    #[serde(rename = "coordinator")]
    Coordinator,
    #[serde(rename = "browser")]
    Browser,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Researcher => "researcher",
            AgentRole::Planner => "planner",
            AgentRole::Developer => "developer",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Synthesizer => "synthesizer",
            AgentRole::Executor => "executor",
            AgentRole::Coordinator => "coordinator",
            AgentRole::Browser => "browser",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "researcher" => Some(AgentRole::Researcher),
            "planner" => Some(AgentRole::Planner),
            "developer" => Some(AgentRole::Developer),
            "reviewer" => Some(AgentRole::Reviewer),
            "synthesizer" => Some(AgentRole::Synthesizer),
            "executor" => Some(AgentRole::Executor),
            "coordinator" => Some(AgentRole::Coordinator),
            "browser" => Some(AgentRole::Browser),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputMode {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "artifact")]
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub role: AgentRole,
    pub system_prompt: String,
    pub context_sources: Vec<String>,
    pub output_var: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Vec<String>,
    pub output_mode: OutputMode,
    pub agent_profile_id: Option<String>,
    pub agent_role_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AgentNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMNodeConfig {
    pub model: String,
    pub prompt: String,
    pub messages: Option<Vec<serde_json::Value>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<String>>,
    pub functions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LLMNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompareOperator {
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "ne")]
    Ne,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "notContains")]
    NotContains,
    #[serde(rename = "startsWith")]
    StartsWith,
    #[serde(rename = "endsWith")]
    EndsWith,
    #[serde(rename = "regexMatch")]
    RegexMatch,
    #[serde(rename = "isEmpty")]
    IsEmpty,
    #[serde(rename = "isNotEmpty")]
    IsNotEmpty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub var_path: String,
    pub operator: CompareOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNodeConfig {
    pub conditions: Vec<Condition>,
    pub logical_op: LogicalOperator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ConditionNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: String,
    pub title: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelNodeConfig {
    pub branches: Vec<Branch>,
    pub wait_for_all: bool,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ParallelNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    #[serde(rename = "forEach")]
    ForEach,
    #[serde(rename = "while")]
    While,
    #[serde(rename = "doWhile")]
    DoWhile,
    #[serde(rename = "until")]
    Until,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNodeConfig {
    pub loop_type: LoopType,
    pub items_var: Option<String>,
    pub iteratee_var: Option<String>,
    pub max_iterations: Option<u32>,
    pub continue_condition: Option<String>,
    pub continue_on_error: bool,
    pub body_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoopNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNodeConfig {
    pub merge_type: String,
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: MergeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    pub delay_type: String,
    pub seconds: u64,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DelayNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNodeConfig {
    pub tool_name: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ToolNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNodeConfig {
    pub language: String,
    pub code: String,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: CodeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNodeConfig {
    pub sub_workflow_id: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SubWorkflowNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentParserNodeConfig {
    pub input_var: String,
    pub parser_type: String,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentParserNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DocumentParserNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRetrieveNodeConfig {
    pub query: String,
    pub knowledge_base_id: String,
    pub top_k: u32,
    pub similarity_threshold: Option<f32>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRetrieveNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: VectorRetrieveNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationNodeConfig {
    pub assertions: Vec<ValidationAssertion>,
    pub on_fail: String,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationAssertion {
    #[serde(rename = "type")]
    pub assertion_type: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ValidationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNodeConfig {
    pub output_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: EndNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowNode {
    Trigger(TriggerNode),
    Agent(AgentNode),
    Llm(LLMNode),
    Condition(ConditionNode),
    Parallel(ParallelNode),
    Loop(LoopNode),
    Merge(MergeNode),
    Delay(DelayNode),
    Validation(ValidationNode),
    SubWorkflow(SubWorkflowNode),
    DocumentParser(DocumentParserNode),
    VectorRetrieve(VectorRetrieveNode),
    End(EndNode),
    // Legacy variants for backward compatibility during deserialization
    #[serde(rename = "tool")]
    Tool(ToolNode),
    #[serde(rename = "code")]
    Code(CodeNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: TriggerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "conditionTrue")]
    ConditionTrue,
    #[serde(rename = "conditionFalse")]
    ConditionFalse,
    #[serde(rename = "loopBack")]
    LoopBack,
    #[serde(rename = "parallelBranch")]
    ParallelBranch,
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub source_handle: Option<String>,
    pub target: String,
    pub target_handle: Option<String>,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnFailureAction {
    #[serde(rename = "abort")]
    Abort,
    #[serde(rename = "retryThenAbort")]
    RetryThenAbort,
    #[serde(rename = "runErrorBranch")]
    RunErrorBranch,
    #[serde(rename = "continueWithDefault")]
    ContinueWithDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub step_id: String,
    pub compensate_type: String,
    pub target_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorConfig {
    pub retry_policy: Option<RetryPolicy>,
    pub on_failure: OnFailureAction,
    pub error_branch: Option<Vec<String>>,
    pub compensation_steps: Option<Vec<CompensationStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkflowTemplateData {
    pub fn to_template_input(&self) -> WorkflowTemplateInput {
        WorkflowTemplateInput {
            name: self.name.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
            tags: self.tags.clone(),
            trigger_config: self.trigger_config.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            input_schema: self.input_schema.clone(),
            output_schema: self.output_schema.clone(),
            variables: self.variables.clone(),
            error_config: self.error_config.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateInput {
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<WorkflowTemplateData> for WorkflowTemplateResponse {
    fn from(data: WorkflowTemplateData) -> Self {
        Self {
            id: data.id,
            name: data.name,
            description: data.description,
            icon: data.icon,
            tags: data.tags,
            version: data.version,
            is_preset: data.is_preset,
            is_editable: data.is_editable,
            is_public: data.is_public,
            trigger_config: data.trigger_config,
            nodes: data.nodes,
            edges: data.edges,
            input_schema: data.input_schema,
            output_schema: data.output_schema,
            variables: data.variables,
            error_config: data.error_config,
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }
}

impl From<crate::entity::workflow_template::Model> for WorkflowTemplateResponse {
    fn from(model: crate::entity::workflow_template::Model) -> Self {
        let tags: Vec<String> = model
            .tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default();

        let trigger_config: Option<TriggerConfig> = model
            .trigger_config
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok());

        let nodes: Vec<WorkflowNode> = serde_json::from_str(&model.nodes).unwrap_or_default();
        let edges: Vec<WorkflowEdge> = serde_json::from_str(&model.edges).unwrap_or_default();
        let variables: Vec<Variable> = model
            .variables
            .as_ref()
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();
        let input_schema: Option<JsonSchema> = model
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let output_schema: Option<JsonSchema> = model
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let error_config: Option<ErrorConfig> = model
            .error_config
            .as_ref()
            .and_then(|e| serde_json::from_str(e).ok());

        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            icon: model.icon,
            tags,
            version: model.version,
            is_preset: model.is_preset,
            is_editable: model.is_editable,
            is_public: model.is_public,
            trigger_config,
            nodes,
            edges,
            input_schema,
            output_schema,
            variables,
            error_config,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFilter {
    pub is_preset: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: String,
    pub node_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Result of migrating a workflow from legacy Tool/Code nodes to Agent nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub workflow_id: String,
    pub migrated_nodes: Vec<NodeMigrationEntry>,
    pub unchanged: bool,
}

/// A single node migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMigrationEntry {
    pub node_id: String,
    pub from_type: String,
    pub to_skill_id: String,
    pub to_skill_name: String,
    pub status: String,
}

/// Workflow migrator that converts legacy Tool/Code nodes to AtomicSkill nodes
pub struct WorkflowMigrator;

impl WorkflowMigrator {
    /// Migrate a workflow Tool and Code nodes to Agent nodes.
    /// Returns a MigrationResult with details of what was migrated.
    #[allow(clippy::ptr_arg)]
    pub fn migrate(nodes: &mut Vec<WorkflowNode>) -> MigrationResult {
        let mut migrated_nodes = Vec::new();
        let mut has_changes = false;

        for node in nodes.iter_mut() {
            let new_node = match node {
                WorkflowNode::Tool(tool_node) => {
                    has_changes = true;
                    migrated_nodes.push(NodeMigrationEntry {
                        node_id: tool_node.base.id.clone(),
                        from_type: "tool".to_string(),
                        to_skill_id: String::new(),
                        to_skill_name: String::new(),
                        status: "migrated_to_agent".to_string(),
                    });
                    Some(WorkflowNode::Agent(AgentNode {
                        base: tool_node.base.clone(),
                        config: AgentNodeConfig {
                            role: AgentRole::Executor,
                            system_prompt: String::new(),
                            context_sources: Vec::new(),
                            output_var: tool_node.config.output_var.clone(),
                            model: None,
                            temperature: None,
                            max_tokens: None,
                            tools: vec![tool_node.config.tool_name.clone()],
                            output_mode: OutputMode::Text,
                            agent_profile_id: None,
                            agent_role_override: None,
                        },
                    }))
                },
                WorkflowNode::Code(code_node) => {
                    has_changes = true;
                    migrated_nodes.push(NodeMigrationEntry {
                        node_id: code_node.base.id.clone(),
                        from_type: "code".to_string(),
                        to_skill_id: String::new(),
                        to_skill_name: String::new(),
                        status: "migrated_to_agent".to_string(),
                    });
                    Some(WorkflowNode::Agent(AgentNode {
                        base: code_node.base.clone(),
                        config: AgentNodeConfig {
                            role: AgentRole::Executor,
                            system_prompt: String::new(),
                            context_sources: Vec::new(),
                            output_var: code_node.config.output_var.clone(),
                            model: None,
                            temperature: None,
                            max_tokens: None,
                            tools: Vec::new(),
                            output_mode: OutputMode::Text,
                            agent_profile_id: None,
                            agent_role_override: None,
                        },
                    }))
                },
                _ => None,
            };

            if let Some(new) = new_node {
                *node = new;
            }
        }

        MigrationResult {
            workflow_id: String::new(),
            migrated_nodes,
            unchanged: !has_changes,
        }
    }

    /// Check if a workflow contains legacy Tool or Code nodes
    pub fn has_legacy_nodes(nodes: &[WorkflowNode]) -> bool {
        nodes
            .iter()
            .any(|n| matches!(n, WorkflowNode::Tool(_) | WorkflowNode::Code(_)))
    }
}
