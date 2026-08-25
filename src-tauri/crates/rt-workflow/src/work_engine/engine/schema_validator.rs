// SPDX-License-Identifier: AGPL-3.0-only

//! 运行时 Schema 校验器
//!
//! 本模块实现基于 NodeContract 的运行时数据校验，
//! 确保节点间传递的数据符合强类型 Schema 约束。
//!
//! # 架构位置
//! - 实现层：rt-workflow（hybrid 层）
//! - 依赖：harness::schema（契约定义）
//! - 被 node_executor 调用，在节点执行前后执行校验

use std::collections::HashMap;

use axagent_harness::schema::{
    NodeContract, PrimitiveType, SchemaFormat, SchemaValidationError, SchemaValidationResult,
};
use axagent_harness::workflow_types::WorkflowNode;
use jsonschema::Validator;

/// Schema 校验器
///
/// 负责在节点执行前后校验输入/输出数据是否符合契约定义。
/// 校验失败时返回详细的错误信息，供上层决定是否重试或熔断。
#[derive(Debug, Default)]
pub struct SchemaValidator {
    /// 已注册的节点契约映射（node_type → contract）
    contracts: HashMap<String, NodeContract>,
}

impl SchemaValidator {
    /// 创建新的校验器
    pub fn new() -> Self {
        Self { contracts: HashMap::new() }
    }

    /// 注册节点契约
    pub fn register_contract(&mut self, node_type: impl Into<String>, contract: NodeContract) {
        self.contracts.insert(node_type.into(), contract);
    }

    /// 为 WorkflowNode 自动注册预设契约
    pub fn register_default_contracts(&mut self) {
        self.register_contract("agent", NodeContract::agent_default());
        self.register_contract("tool", NodeContract::tool_default());
        self.register_contract("llm", NodeContract::llm_default());
        self.register_contract("condition", NodeContract::condition_default());
        self.register_contract("loop", NodeContract::loop_default());
        self.register_contract("parallel", NodeContract::parallel_default());
        self.register_contract("code", NodeContract::code_default());
        self.register_contract("httpRequest", NodeContract::http_request_default());
        self.register_contract("end", NodeContract::end_default());
    }

    /// 校验节点输入
    pub fn validate_input(
        &self,
        node: &WorkflowNode,
        input: &serde_json::Value,
    ) -> SchemaValidationResult {
        let node_type = get_node_type(node);
        let contract = match self.contracts.get(node_type) {
            Some(c) => c,
            None => {
                return SchemaValidationResult::skipped(format!("节点类型 {node_type} 无注册契约"));
            },
        };

        match &contract.input_schema {
            Some(schema) => validate_against_schema(input, schema, "input"),
            None => SchemaValidationResult::Valid,
        }
    }

    /// 校验节点输出
    pub fn validate_output(
        &self,
        node: &WorkflowNode,
        output: &serde_json::Value,
    ) -> SchemaValidationResult {
        let node_type = get_node_type(node);
        let contract = match self.contracts.get(node_type) {
            Some(c) => c,
            None => {
                return SchemaValidationResult::skipped(format!("节点类型 {node_type} 无注册契约"));
            },
        };

        match &contract.output_schema {
            Some(schema) => validate_against_schema(output, schema, "output"),
            None => SchemaValidationResult::Valid,
        }
    }

    /// 获取节点契约
    pub fn get_contract(&self, node_type: &str) -> Option<&NodeContract> {
        self.contracts.get(node_type)
    }
}

// ── 内部工具函数 ──

/// 获取节点类型字符串
fn get_node_type(node: &WorkflowNode) -> &'static str {
    match node {
        WorkflowNode::Trigger(_) => "trigger",
        WorkflowNode::Agent(_) => "agent",
        WorkflowNode::Llm(_) => "llm",
        WorkflowNode::Condition(_) => "condition",
        WorkflowNode::Parallel(_) => "parallel",
        WorkflowNode::Loop(_) => "loop",
        WorkflowNode::Merge(_) => "merge",
        WorkflowNode::Delay(_) => "delay",
        WorkflowNode::Validation(_) => "validation",
        WorkflowNode::SubWorkflow(_) => "subWorkflow",
        WorkflowNode::WorkflowRef(_) => "workflowRef",
        WorkflowNode::DocumentParser(_) => "documentParser",
        WorkflowNode::VectorRetrieve(_) => "vectorRetrieve",
        WorkflowNode::End(_) => "end",
        WorkflowNode::HttpRequest(_) => "httpRequest",
        WorkflowNode::Switch(_) => "switch",
        WorkflowNode::DatabaseQuery(_) => "databaseQuery",
        WorkflowNode::Notification(_) => "notification",
        WorkflowNode::Approval(_) => "approval",
        WorkflowNode::FileOperation(_) => "fileOperation",
        WorkflowNode::DataTransformer(_) => "dataTransformer",
        WorkflowNode::WebhookSend(_) => "webhookSend",
        WorkflowNode::Logging(_) => "logging",
        WorkflowNode::LlmClassifier(_) => "llmClassifier",
        WorkflowNode::Aggregator(_) => "aggregator",
        WorkflowNode::Email(_) => "email",
        WorkflowNode::Debate(_) => "debate",
        WorkflowNode::Swarm(_) => "swarm",
        WorkflowNode::MultiAgent(_) => "multiAgent",
        WorkflowNode::Storage(_) => "storage",
        WorkflowNode::Tool(_) => "tool",
        WorkflowNode::Code(_) => "code",
    }
}

/// 针对 Schema 格式校验数据
fn validate_against_schema(
    value: &serde_json::Value,
    schema: &SchemaFormat,
    direction: &str,
) -> SchemaValidationResult {
    match schema {
        SchemaFormat::JsonSchema { definition } => {
            validate_json_schema(value, definition, direction)
        },
        SchemaFormat::Primitive { primitive_type } => {
            validate_primitive(value, primitive_type, direction)
        },
        SchemaFormat::Protobuf { message_name, .. } => {
            // Protobuf 校验需要 descriptor，预留接口
            SchemaValidationResult::skipped(format!(
                "Protobuf Schema '{message_name}' 需要运行时校验器支持"
            ))
        },
    }
}

/// 校验 JSON Schema（使用专业 jsonschema crate）
fn validate_json_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    direction: &str,
) -> SchemaValidationResult {
    // 创建验证器
    let validator = match Validator::new(schema) {
        Ok(v) => v,
        Err(e) => {
            return SchemaValidationResult::skipped(format!("{direction} Schema 解析失败: {e}"));
        },
    };

    // 执行校验
    match validator.validate(value) {
        Ok(()) => SchemaValidationResult::Valid,
        Err(err) => {
            // 将 jsonschema::ValidationError 转换为 SchemaValidationError
            let path = err.instance_path().to_string().replace('\'', "").replace('"', "");
            let message = format!("{direction} 校验失败: {err}");
            let schema_error = SchemaValidationError {
                path: if path.is_empty() {
                    "/".to_string()
                } else {
                    path
                },
                message,
                expected_type: Some(err.schema_path().to_string()),
                actual_value: None,
            };
            SchemaValidationResult::invalid(vec![schema_error])
        },
    }
}

/// 校验基础类型
fn validate_primitive(
    value: &serde_json::Value,
    expected: &PrimitiveType,
    direction: &str,
) -> SchemaValidationResult {
    let actual = detect_primitive_type(value);
    if actual == *expected {
        SchemaValidationResult::Valid
    } else {
        SchemaValidationResult::invalid(vec![SchemaValidationError {
            path: "/".to_string(),
            message: format!("{direction} 基础类型不匹配: 期望 {expected:?}, 实际 {actual:?}"),
            expected_type: Some(format!("{expected:?}")),
            actual_value: Some(value.clone()),
        }])
    }
}

/// 检测基础类型
fn detect_primitive_type(value: &serde_json::Value) -> PrimitiveType {
    match value {
        serde_json::Value::Null => PrimitiveType::Null,
        serde_json::Value::Bool(_) => PrimitiveType::Boolean,
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                PrimitiveType::Integer
            } else {
                PrimitiveType::Number
            }
        },
        serde_json::Value::String(_) => PrimitiveType::String,
        serde_json::Value::Array(_) => PrimitiveType::Array,
        serde_json::Value::Object(_) => PrimitiveType::Object,
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::schema::NodeContract;
    use axagent_harness::workflow_types::{
        Position, RetryConfig, ToolNode, ToolNodeConfig, WorkflowNodeBase,
    };

    fn create_test_node() -> WorkflowNode {
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: "test_tool".to_string(),
                title: "Test Tool".to_string(),
                description: None,
                position: Position::default(),
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: "search".to_string(),
                input_mapping: HashMap::new(),
                output_var: "result".to_string(),
            },
        })
    }

    #[test]
    fn test_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(validator.contracts.is_empty());
    }

    #[test]
    fn test_register_contract() {
        let mut validator = SchemaValidator::new();
        validator.register_contract("tool", NodeContract::tool_default());
        assert!(validator.get_contract("tool").is_some());
        assert!(validator.get_contract("agent").is_none());
    }

    #[test]
    fn test_register_default_contracts() {
        let mut validator = SchemaValidator::new();
        validator.register_default_contracts();
        assert!(validator.get_contract("agent").is_some());
        assert!(validator.get_contract("tool").is_some());
        assert!(validator.get_contract("llm").is_some());
        assert!(validator.get_contract("end").is_some());
    }

    #[test]
    fn test_validate_input_no_contract() {
        let validator = SchemaValidator::new();
        let node = create_test_node();
        let result = validator.validate_input(&node, &serde_json::json!({}));
        assert!(result.is_skipped());
    }

    #[test]
    fn test_validate_input_with_contract_valid() {
        let mut validator = SchemaValidator::new();
        validator.register_default_contracts();
        let node = create_test_node();

        // Tool 节点接受 object 输入
        let result = validator.validate_input(&node, &serde_json::json!({"query": "test"}));
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_output_valid() {
        let mut validator = SchemaValidator::new();
        validator.register_default_contracts();
        let node = create_test_node();

        let result = validator.validate_output(&node, &serde_json::json!({"status": "ok"}));
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_json_schema_type_mismatch() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        // 正确的对象
        let valid = serde_json::json!({
            "name": "John",
            "age": 30
        });
        let result = validate_json_schema(&valid, &schema, "test");
        assert!(result.is_valid());

        // 缺少必填字段
        let invalid = serde_json::json!({
            "age": 30
        });
        let result = validate_json_schema(&invalid, &schema, "test");
        assert!(result.is_invalid());
    }

    #[test]
    fn test_validate_primitive_string() {
        let schema = SchemaFormat::Primitive { primitive_type: PrimitiveType::String };

        let result = validate_against_schema(&serde_json::json!("hello"), &schema, "test");
        assert!(result.is_valid());

        let result = validate_against_schema(&serde_json::json!(42), &schema, "test");
        assert!(result.is_invalid());
    }

    #[test]
    fn test_validate_primitive_integer() {
        let schema = SchemaFormat::Primitive { primitive_type: PrimitiveType::Integer };

        let result = validate_against_schema(&serde_json::json!(42), &schema, "test");
        assert!(result.is_valid());

        let result = validate_against_schema(&serde_json::json!(3.14), &schema, "test");
        assert!(result.is_invalid());
    }

    #[test]
    fn test_nested_schema_validation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "email": {"type": "string"}
                    },
                    "required": ["name", "email"]
                }
            },
            "required": ["user"]
        });

        // 嵌套对象验证
        let valid = serde_json::json!({
            "user": {
                "name": "John",
                "email": "john@test.com"
            }
        });
        let result = validate_json_schema(&valid, &schema, "test");
        assert!(result.is_valid());

        // 嵌套对象缺少必填
        let invalid = serde_json::json!({
            "user": {
                "name": "John"
            }
        });
        let result = validate_json_schema(&invalid, &schema, "test");
        assert!(result.is_invalid());
    }
}
