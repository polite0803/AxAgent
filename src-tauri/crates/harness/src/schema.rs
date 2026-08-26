// SPDX-License-Identifier: AGPL-3.0-only

//! 协议层：强类型 Schema 系统
//!
//! 本模块定义工作流节点间交互的强类型契约（Contract），
//! 确保上游输出符合 Schema 才能流转到下游，告别"黑盒文本传递"。
//!
//! # 架构定位
//! - 定义在 harness 层（foundation），所有 crate 可通过 pub use 复用
//! - 不包含任何运行时执行逻辑，仅做纯数据定义
//! - 运行时校验逻辑由 rt-workflow 实现

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Schema 格式枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "format", rename_all = "camelCase")]
pub enum SchemaFormat {
    /// JSON Schema（最常用，与现有 JsonSchema 兼容）
    JsonSchema {
        #[serde(rename = "schema")]
        definition: serde_json::Value,
    },
    /// Protobuf（预留，用于高性能场景）
    Protobuf {
        #[serde(rename = "messageName")]
        message_name: String,
        #[serde(skip)]
        descriptor: Option<Vec<u8>>,
    },
    /// 纯类型标记（简单场景，如 string/number/boolean）
    Primitive {
        #[serde(rename = "type")]
        primitive_type: PrimitiveType,
    },
}

/// 基础类型枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Integer,
    Array,
    Object,
    Null,
}

impl SchemaFormat {
    /// 创建一个 JSON Schema 定义
    pub fn json_schema(schema: serde_json::Value) -> Self {
        SchemaFormat::JsonSchema { definition: schema }
    }

    /// 创建一个基础类型定义
    pub fn primitive(pt: PrimitiveType) -> Self {
        SchemaFormat::Primitive { primitive_type: pt }
    }

    /// 获取 Schema 的类型名称（用于调试日志）
    pub fn type_name(&self) -> &'static str {
        match self {
            SchemaFormat::JsonSchema { .. } => "json_schema",
            SchemaFormat::Protobuf { .. } => "protobuf",
            SchemaFormat::Primitive { .. } => "primitive",
        }
    }
}

/// 节点契约定义 —— 节点的"输入/输出接口"
///
/// # 设计理念
/// - 契约是节点的"门面"，定义节点能接受什么、能产出什么
/// - 运行时引擎在节点执行前后都会校验契约
/// - 校验失败则阻断流转，确保错误不扩散
///
/// # 示例
/// ```ignore
/// let contract = NodeContract::new()
///     .with_input(SchemaFormat::json_schema(json!({
///         "type": "object",
///         "properties": {
///             "query": {"type": "string"},
///             "max_results": {"type": "integer"}
///         },
///         "required": ["query"]
///     })))
///     .with_output(SchemaFormat::json_schema(json!({
///         "type": "array",
///         "items": {"type": "object"}
///     })));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeContract {
    /// 契约版本号（用于向后兼容）
    #[serde(default = "default_contract_version")]
    pub version: u32,
    /// 输入 Schema（None 表示接受任意输入）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<SchemaFormat>,
    /// 输出 Schema（None 表示输出不做校验）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<SchemaFormat>,
    /// 契约描述（人类可读，用于文档生成）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否启用严格模式（严格模式下 Schema 不匹配直接报错，否则尝试容错）
    #[serde(default = "default_true")]
    pub strict: bool,
}

fn default_contract_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

impl Default for NodeContract {
    fn default() -> Self {
        Self {
            version: default_contract_version(),
            input_schema: None,
            output_schema: None,
            description: None,
            strict: true,
        }
    }
}

impl NodeContract {
    /// 创建新的空契约
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置输入 Schema
    pub fn with_input(mut self, schema: SchemaFormat) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// 设置输出 Schema
    pub fn with_output(mut self, schema: SchemaFormat) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置严格模式
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// 输入 Schema 是否存在
    pub fn has_input(&self) -> bool {
        self.input_schema.is_some()
    }

    /// 输出 Schema 是否存在
    pub fn has_output(&self) -> bool {
        self.output_schema.is_some()
    }
}

// ── 预设契约模板 ──
//
// 为常见节点类型提供预设契约，避免用户重复定义。

impl NodeContract {
    /// Agent 节点预设：接受任意输入，输出 text/object
    pub fn agent_default() -> Self {
        Self::new().with_description("Agent 节点：接受自然语言输入，输出文本或结构化结果")
    }

    /// Tool 节点预设：接受结构化输入，输出结构化结果
    pub fn tool_default() -> Self {
        Self::new()
            .with_input(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "description": "工具调用参数"
            })))
            .with_output(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "description": "工具执行结果"
            })))
            .with_description("Tool 节点：接受参数映射，输出工具执行结果")
    }

    /// LLM 节点预设：接受 prompt 上下文，输出 text
    pub fn llm_default() -> Self {
        Self::new()
            .with_input(SchemaFormat::primitive(PrimitiveType::String))
            .with_output(SchemaFormat::primitive(PrimitiveType::String))
            .with_description("LLM 节点：接受 prompt，输出模型文本")
    }

    /// 条件节点预设：接受任意输入，输出 boolean 或路由决策
    pub fn condition_default() -> Self {
        Self::new()
            .with_output(SchemaFormat::primitive(PrimitiveType::Boolean))
            .with_description("Condition 节点：接受上下文，输出路由决策")
    }

    /// 循环节点预设：接受数组，输出聚合结果
    pub fn loop_default() -> Self {
        Self::new()
            .with_input(SchemaFormat::json_schema(serde_json::json!({
                "type": "array"
            })))
            .with_output(SchemaFormat::json_schema(serde_json::json!({
                "type": "array",
                "description": "聚合结果"
            })))
            .with_description("Loop 节点：接受数组，循环执行后输出聚合结果")
    }

    /// 并行节点预设：接受任意输入，输出分支聚合结果
    pub fn parallel_default() -> Self {
        Self::new()
            .with_output(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "description": "并行分支聚合结果"
            })))
            .with_description("Parallel 节点：接受上下文，并行执行后输出聚合结果")
    }

    /// 代码节点预设：接受变量映射，输出代码执行结果
    pub fn code_default() -> Self {
        Self::new()
            .with_output(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "description": "代码执行结果"
            })))
            .with_description("Code 节点：接受输入变量，输出代码执行结果")
    }

    /// HTTP 请求节点预设：接受请求参数，响应 HTTP 响应
    pub fn http_request_default() -> Self {
        Self::new()
            .with_input(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string"},
                    "headers": {"type": "object"},
                    "body": {}
                }
            })))
            .with_output(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "integer"},
                    "headers": {"type": "object"},
                    "body": {}
                }
            })))
            .with_description("HTTP Request 节点：接受请求配置，输出 HTTP 响应")
    }

    /// 结束节点预设：接受任意输入，输出最终结果
    pub fn end_default() -> Self {
        Self::new().with_description("End 节点：接受任意输入，标记工作流结束")
    }
}

// ── Schema 校验结果 ──

/// Schema 校验结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "valid", rename_all = "camelCase")]
pub enum SchemaValidationResult {
    /// 校验通过
    Valid,
    /// 校验失败
    Invalid {
        /// 错误列表
        errors: Vec<SchemaValidationError>,
    },
    /// 无 Schema（跳过校验）
    Skipped {
        #[serde(rename = "reason")]
        reason: String,
    },
}

/// 单个校验错误
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SchemaValidationError {
    /// 错误路径（JSON Pointer 格式，如 "/properties/query"）
    pub path: String,
    /// 错误信息
    pub message: String,
    /// 期望的类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<String>,
    /// 实际的类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<serde_json::Value>,
}

impl SchemaValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, SchemaValidationResult::Valid)
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, SchemaValidationResult::Invalid { .. })
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, SchemaValidationResult::Skipped { .. })
    }

    pub fn invalid(errors: Vec<SchemaValidationError>) -> Self {
        SchemaValidationResult::Invalid { errors }
    }

    pub fn skipped(reason: impl Into<String>) -> Self {
        SchemaValidationResult::Skipped { reason: reason.into() }
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_contract_default() {
        let contract = NodeContract::default();
        assert_eq!(contract.version, 1);
        assert!(contract.strict);
        assert!(contract.input_schema.is_none());
        assert!(contract.output_schema.is_none());
    }

    #[test]
    fn test_node_contract_builder() {
        let contract = NodeContract::new()
            .with_input(SchemaFormat::primitive(PrimitiveType::String))
            .with_output(SchemaFormat::primitive(PrimitiveType::Boolean))
            .with_description("测试契约")
            .with_strict(false);

        assert!(contract.has_input());
        assert!(contract.has_output());
        assert!(!contract.strict);
        assert_eq!(contract.description, Some("测试契约".to_string()));
    }

    #[test]
    fn test_preset_contracts() {
        // 验证所有预设契约都能创建
        let contracts = vec![
            ("Agent", NodeContract::agent_default()),
            ("Tool", NodeContract::tool_default()),
            ("LLM", NodeContract::llm_default()),
            ("Condition", NodeContract::condition_default()),
            ("Loop", NodeContract::loop_default()),
            ("Parallel", NodeContract::parallel_default()),
            ("Code", NodeContract::code_default()),
            ("HttpRequest", NodeContract::http_request_default()),
            ("End", NodeContract::end_default()),
        ];

        for (name, contract) in contracts {
            assert!(
                contract.has_input() || contract.has_output() || contract.description.is_some(),
                "{name} 预设契约应有描述或 Schema"
            );
        }
    }

    #[test]
    fn test_schema_format_type_name() {
        let json = SchemaFormat::json_schema(serde_json::json!({"type": "string"}));
        assert_eq!(json.type_name(), "json_schema");

        let proto =
            SchemaFormat::Protobuf { message_name: "TestMsg".to_string(), descriptor: None };
        assert_eq!(proto.type_name(), "protobuf");

        let prim = SchemaFormat::primitive(PrimitiveType::String);
        assert_eq!(prim.type_name(), "primitive");
    }

    #[test]
    fn test_validation_result() {
        let valid = SchemaValidationResult::Valid;
        assert!(valid.is_valid());

        let invalid = SchemaValidationResult::invalid(vec![SchemaValidationError {
            path: "/test".to_string(),
            message: "类型不匹配".to_string(),
            expected_type: Some("string".to_string()),
            actual_value: Some(serde_json::json!(42)),
        }]);
        assert!(invalid.is_invalid());

        let skipped = SchemaValidationResult::skipped("无 Schema");
        assert!(skipped.is_skipped());
    }

    #[test]
    fn test_contract_serialization() {
        let contract = NodeContract::new()
            .with_input(SchemaFormat::json_schema(serde_json::json!({
                "type": "object",
                "required": ["name"]
            })))
            .with_output(SchemaFormat::primitive(PrimitiveType::String))
            .with_description("序列化测试");

        let json = serde_json::to_string(&contract).unwrap();
        let deserialized: NodeContract = serde_json::from_str(&json).unwrap();

        assert_eq!(contract, deserialized);
    }
}
