// SPDX-License-Identifier: AGPL-3.0-only

//! DTO 序列化契约测试
//!
//! 本模块对所有核心数据传输对象（DTO）进行序列化/反序列化往返测试，
//! 确保字段语义在跨进程传输（Tauri IPC）过程中不丢失。
//!
//! 测试覆盖范围：
//! - Agent 相关 DTO：AgentExecuteRequest, AgentResult, AgentPlan 等
//! - Conversation 相关 DTO：ConversationMessage, TokenUsage, ContentBlock 等
//! - Workflow 相关 DTO：WorkflowNodeBase, RetryConfig, Variable 等
//! - Session 相关 DTO：SessionInfo 等
//!
//! 如果某个 DTO 的序列化/反序列化往返测试失败，说明：
//! 1. 字段被意外添加/删除
//! 2. #[serde(default)] 属性导致 None 被替换为默认值
//! 3. skip_serializing_if 属性导致字段被意外跳过

use axagent_harness::agent::{
    AgentCapability, AgentExecuteRequest, AgentInfo, AgentPlan, AgentResult, PlanStep,
};
use axagent_harness::conversation_model::{
    ContentBlock, ConversationMessage, SessionInfo, TokenUsage,
};
use axagent_harness::types::MessageRole;
use axagent_harness::workflow_types::{
    BackoffType, CompensationConfig, CompensationStrategy, NodeKind, Position, RetryConfig,
    Variable, WorkflowNodeBase,
};

// ============================================================================
// 通用序列化契约测试宏
// ============================================================================

/// DTO 序列化契约测试宏
///
/// 确保结构体在序列化/反序列化往返后字段语义保持不变。
/// 使用 JSON 字符串对比，避免依赖 PartialEq 实现。
macro_rules! serialization_contract {
    ($test_name:ident, $struct_type:ty, $instance:expr) => {
        #[test]
        fn $test_name() {
            let original = $instance;

            // 第一次序列化为 JSON
            let json_first = serde_json::to_string(&original).unwrap_or_else(|e| {
                panic!("序列化失败: {} - 类型: {}", e, stringify!($struct_type))
            });

            // 反序列化回来
            let restored: $struct_type = serde_json::from_str(&json_first).unwrap_or_else(|e| {
                panic!(
                    "反序列化失败: {} - 类型: {}\nJSON: {}",
                    e,
                    stringify!($struct_type),
                    json_first
                )
            });

            // 再次序列化
            let json_second = serde_json::to_string(&restored).unwrap_or_else(|e| {
                panic!("二次序列化失败: {} - 类型: {}", e, stringify!($struct_type))
            });

            // 对比两次 JSON 是否完全一致
            assert_eq!(
                json_first,
                json_second,
                "序列化往返测试失败: {} 字段语义在序列化/反序列化过程中发生变化。\n\
                 原始 JSON: {}\n恢复后 JSON: {}",
                stringify!($struct_type),
                json_first,
                json_second
            );
        }
    };
}

// ============================================================================
// Agent 相关 DTO 测试
// ============================================================================

// AgentExecuteRequest - 包含 Optional 字段（关键测试点）
serialization_contract!(
    test_agent_execute_request_with_none,
    AgentExecuteRequest,
    AgentExecuteRequest { goal: "test goal".to_string(), context: None, max_steps: None }
);

// AgentExecuteRequest - 包含所有字段
serialization_contract!(
    test_agent_execute_request_full,
    AgentExecuteRequest,
    AgentExecuteRequest {
        goal: "full test goal".to_string(),
        context: Some("some context".to_string()),
        max_steps: Some(10),
    }
);

// AgentResult
serialization_contract!(
    test_agent_result_success,
    AgentResult,
    AgentResult {
        output: "Hello World".to_string(),
        success: true,
        steps_taken: 5,
        session_id: None
    }
);

serialization_contract!(
    test_agent_result_failure,
    AgentResult,
    AgentResult { output: "".to_string(), success: false, steps_taken: 0, session_id: None }
);

// AgentPlan
serialization_contract!(
    test_agent_plan_single_step,
    AgentPlan,
    AgentPlan {
        steps: vec![PlanStep {
            description: "Step 1".to_string(),
            agent: Some("planner".to_string()),
        }],
    }
);

serialization_contract!(
    test_agent_plan_multiple_steps,
    AgentPlan,
    AgentPlan {
        steps: vec![
            PlanStep {
                description: "Analyze the problem".to_string(),
                agent: Some("analyst".to_string()),
            },
            PlanStep { description: "Search for solutions".to_string(), agent: None },
            PlanStep {
                description: "Implement the fix".to_string(),
                agent: Some("coder".to_string()),
            },
        ],
    }
);

// AgentCapability
serialization_contract!(
    test_agent_capability,
    AgentCapability,
    AgentCapability {
        name: "code_generation".to_string(),
        description: "Generate code from natural language".to_string(),
    }
);

// AgentInfo
serialization_contract!(
    test_agent_info,
    AgentInfo,
    AgentInfo {
        name: "TestAgent".to_string(),
        description: "A test agent".to_string(),
        capabilities: vec![AgentCapability {
            name: "search".to_string(),
            description: "Search the web".to_string(),
        }],
    }
);

// ============================================================================
// Conversation 相关 DTO 测试
// ============================================================================

// TokenUsage - 包含 Optional 字段（cache_miss_input_tokens）
serialization_contract!(test_token_usage_default, TokenUsage, TokenUsage::default());

serialization_contract!(
    test_token_usage_with_values,
    TokenUsage,
    TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 10,
        cache_read_input_tokens: 20,
        cache_miss_input_tokens: None,
    }
);

serialization_contract!(
    test_token_usage_with_cache_miss,
    TokenUsage,
    TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_miss_input_tokens: Some(5),
    }
);

// ContentBlock - 三种变体
serialization_contract!(
    test_content_block_text,
    ContentBlock,
    ContentBlock::Text { text: "Hello World".to_string() }
);

serialization_contract!(
    test_content_block_tool_use,
    ContentBlock,
    ContentBlock::ToolUse {
        id: "call_123".to_string(),
        name: "read_file".to_string(),
        input: r#"{"path": "/tmp/test.txt"}"#.to_string(),
    }
);

serialization_contract!(
    test_content_block_tool_result,
    ContentBlock,
    ContentBlock::ToolResult {
        tool_use_id: "call_123".to_string(),
        tool_name: "read_file".to_string(),
        output: "File contents...".to_string(),
        is_error: false,
    }
);

serialization_contract!(
    test_content_block_tool_result_error,
    ContentBlock,
    ContentBlock::ToolResult {
        tool_use_id: "call_456".to_string(),
        tool_name: "write_file".to_string(),
        output: "Permission denied".to_string(),
        is_error: true,
    }
);

// ConversationMessage - 包含 usage 可选字段
serialization_contract!(
    test_conversation_message_with_usage,
    ConversationMessage,
    ConversationMessage {
        role: MessageRole::Assistant,
        blocks: vec![ContentBlock::Text { text: "Here's the result.".to_string() }],
        usage: Some(TokenUsage {
            input_tokens: 50,
            output_tokens: 30,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        }),
    }
);

serialization_contract!(
    test_conversation_message_without_usage,
    ConversationMessage,
    ConversationMessage {
        role: MessageRole::User,
        blocks: vec![ContentBlock::Text { text: "Hello".to_string() }],
        usage: None,
    }
);

serialization_contract!(
    test_conversation_message_tool_call,
    ConversationMessage,
    ConversationMessage {
        role: MessageRole::Assistant,
        blocks: vec![
            ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "search".to_string(),
                input: r#"{"query": "test"}"#.to_string(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                tool_name: "search".to_string(),
                output: "Found results...".to_string(),
                is_error: false,
            },
        ],
        usage: None,
    }
);

// SessionInfo
serialization_contract!(
    test_session_info,
    SessionInfo,
    SessionInfo {
        session_id: "sess_123".to_string(),
        user_id: "user_456".to_string(),
        title: Some("Test Session".to_string()),
        created_at: 1700000000,
        updated_at: 1700003600,
        token_usage: Some(TokenUsage::default()),
    }
);

serialization_contract!(
    test_session_info_minimal,
    SessionInfo,
    SessionInfo {
        session_id: "sess_min".to_string(),
        user_id: "user_min".to_string(),
        title: None,
        created_at: 1700000000,
        updated_at: 1700000000,
        token_usage: None,
    }
);

// ============================================================================
// Workflow 相关 DTO 测试
// ============================================================================

// Position
serialization_contract!(test_position_default, Position, Position::default());

serialization_contract!(test_position_with_values, Position, Position { x: 100.5, y: 200.3 });

// BackoffType - 枚举单独测试
#[test]
fn test_backoff_type_roundtrip() {
    let variants = vec![BackoffType::Linear, BackoffType::Exponential, BackoffType::Fixed];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap_or_else(|e| panic!("序列化失败: {}", e));
        let restored: BackoffType = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("反序列化失败: {} - JSON: {}", e, json));
        let json_second = serde_json::to_string(&restored).unwrap();
        assert_eq!(json, json_second, "BackoffType 枚举序列化往返失败");
    }
}

// RetryConfig
serialization_contract!(test_retry_config_default, RetryConfig, RetryConfig::default());

serialization_contract!(
    test_retry_config_custom,
    RetryConfig,
    RetryConfig {
        enabled: true,
        max_retries: 5,
        backoff_type: BackoffType::Linear,
        base_delay_ms: 500,
        max_delay_ms: 5000,
    }
);

// CompensationStrategy - 枚举单独测试
#[test]
fn test_compensation_strategy_roundtrip() {
    let variants = vec![
        CompensationStrategy::SkipWithWarning,
        CompensationStrategy::Rollback,
        CompensationStrategy::Escalate,
    ];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap_or_else(|e| panic!("序列化失败: {}", e));
        let restored: CompensationStrategy = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("反序列化失败: {} - JSON: {}", e, json));
        let json_second = serde_json::to_string(&restored).unwrap();
        assert_eq!(json, json_second, "CompensationStrategy 枚举序列化往返失败");
    }
}

// CompensationConfig
serialization_contract!(
    test_compensation_config_skip,
    CompensationConfig,
    CompensationConfig {
        strategy: CompensationStrategy::SkipWithWarning,
        compensation_nodes: vec![],
    }
);

serialization_contract!(
    test_compensation_config_rollback,
    CompensationConfig,
    CompensationConfig {
        strategy: CompensationStrategy::Rollback,
        compensation_nodes: vec!["node_1".to_string(), "node_2".to_string()],
    }
);

// NodeKind - 枚举单独测试
#[test]
fn test_node_kind_roundtrip() {
    let variants = vec![
        NodeKind::Input,
        NodeKind::Output,
        NodeKind::Tool,
        NodeKind::Agent,
        NodeKind::Condition,
        NodeKind::Loop,
        NodeKind::Container,
        NodeKind::Storage,
    ];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap_or_else(|e| panic!("序列化失败: {}", e));
        let restored: NodeKind = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("反序列化失败: {} - JSON: {}", e, json));
        let json_second = serde_json::to_string(&restored).unwrap();
        assert_eq!(json, json_second, "NodeKind 枚举序列化往返失败");
    }
}

// Variable
serialization_contract!(
    test_variable_simple,
    Variable,
    Variable {
        name: "simple_var".to_string(),
        var_type: "string".to_string(),
        value: serde_json::json!("hello"),
        description: None,
        is_secret: false,
    }
);

serialization_contract!(
    test_variable_complex,
    Variable,
    Variable {
        name: "complex_var".to_string(),
        var_type: "object".to_string(),
        value: serde_json::json!({
            "nested": true,
            "items": [1, 2, 3],
            "metadata": {
                "created": "2024-01-01",
                "version": 2
            }
        }),
        description: Some("A complex variable with nested structure".to_string()),
        is_secret: true,
    }
);

// WorkflowNodeBase
serialization_contract!(
    test_workflow_node_base_default,
    WorkflowNodeBase,
    WorkflowNodeBase {
        id: "node_1".to_string(),
        title: "Test Node".to_string(),
        description: None,
        position: Position::default(),
        retry: RetryConfig::default(),
        timeout: None,
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
);

serialization_contract!(
    test_workflow_node_base_full,
    WorkflowNodeBase,
    WorkflowNodeBase {
        id: "node_full".to_string(),
        title: "Full Node".to_string(),
        description: Some("A fully configured node".to_string()),
        position: Position { x: 100.0, y: 200.0 },
        retry: RetryConfig {
            enabled: true,
            max_retries: 3,
            backoff_type: BackoffType::Exponential,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        },
        timeout: Some(60000),
        enabled: true,
        parent_id: Some("parent_1".to_string()),
        compensation: Some(CompensationConfig {
            strategy: CompensationStrategy::Rollback,
            compensation_nodes: vec!["child_1".to_string()],
        }),
        continue_on_fail: true,
    }
);

// ============================================================================
// 边界情况测试
// ============================================================================

#[test]
fn test_empty_string_roundtrip() {
    let original = AgentExecuteRequest { goal: String::new(), context: None, max_steps: None };
    let json = serde_json::to_string(&original).unwrap();
    let restored: AgentExecuteRequest = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

#[test]
fn test_large_numbers_roundtrip() {
    let original = TokenUsage {
        input_tokens: u32::MAX,
        output_tokens: u32::MAX,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_miss_input_tokens: Some(u32::MAX),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: TokenUsage = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

#[test]
fn test_special_characters_in_strings() {
    let original = AgentExecuteRequest {
        goal: "目标：分析 \"代码\" 和 '文本'，包括\n换行和\t制表符以及\\转义字符".to_string(),
        context: Some("特殊字符：<>&\"'\\n\\t".to_string()),
        max_steps: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: AgentExecuteRequest = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

#[test]
fn test_unicode_characters() {
    let original = AgentResult {
        output: "你好世界 🌍 مرحبا العالم こんにちは世界".to_string(),
        success: true,
        steps_taken: 1,
        session_id: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: AgentResult = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

#[test]
fn test_nested_content_blocks() {
    let original = ConversationMessage {
        role: MessageRole::Assistant,
        blocks: vec![
            ContentBlock::Text { text: "Let me search for that.".to_string() },
            ContentBlock::ToolUse {
                id: "search_1".to_string(),
                name: "web_search".to_string(),
                input: r#"{"query": "test query"}"#.to_string(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "search_1".to_string(),
                tool_name: "web_search".to_string(),
                output: r#"{"results": ["result1", "result2"]}"#.to_string(),
                is_error: false,
            },
            ContentBlock::Text { text: "Here are the results.".to_string() },
        ],
        usage: Some(TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        }),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: ConversationMessage = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

#[test]
fn test_variable_with_json_value_roundtrip() {
    let original = Variable {
        name: "config".to_string(),
        var_type: "json".to_string(),
        value: serde_json::json!({
            "timeout": 30,
            "retries": [1, 2, 3],
            "options": {
                "verbose": true,
                "format": "json"
            }
        }),
        description: Some("Runtime configuration".to_string()),
        is_secret: false,
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: Variable = serde_json::from_str(&json).unwrap();
    let json_second = serde_json::to_string(&restored).unwrap();
    assert_eq!(json, json_second);
}

// ============================================================================
// 语义保持验证（关键 None 值不应被意外替换）
// ============================================================================

/// 验证 None 字段在序列化后仍然存在（不会被 default 替换）
#[test]
fn test_none_fields_preserved_in_json() {
    let request = AgentExecuteRequest { goal: "test".to_string(), context: None, max_steps: None };
    let json: serde_json::Value = serde_json::to_value(&request).unwrap();

    // context 和 maxSteps 应该存在且为 null
    assert!(json.get("context").is_some(), "context 字段应该存在于 JSON 中");
    assert_eq!(json.get("context").unwrap(), &serde_json::Value::Null, "context 字段应该为 null");
    assert!(json.get("maxSteps").is_some(), "maxSteps 字段应该存在于 JSON 中");
    assert_eq!(json.get("maxSteps").unwrap(), &serde_json::Value::Null, "maxSteps 字段应该为 null");
}

/// 验证 Some 值在序列化后保持不变
#[test]
fn test_some_fields_preserved_in_json() {
    let request = AgentExecuteRequest {
        goal: "test".to_string(),
        context: Some("important context".to_string()),
        max_steps: Some(42),
    };
    let json: serde_json::Value = serde_json::to_value(&request).unwrap();

    assert_eq!(
        json.get("context").unwrap(),
        &serde_json::json!("important context"),
        "context 值应该保持不变"
    );
    assert_eq!(json.get("maxSteps").unwrap(), &serde_json::json!(42), "maxSteps 值应该保持不变");
}

/// 验证 TokenUsage 的可选字段行为
#[test]
fn test_token_usage_optional_field_behavior() {
    // 无 cache_miss
    let usage_no_miss = TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_miss_input_tokens: None,
    };
    let json: serde_json::Value = serde_json::to_value(usage_no_miss).unwrap();
    // cache_miss_input_tokens 应该存在且为 null
    assert!(json.get("cacheMissInputTokens").is_some());
    assert_eq!(json.get("cacheMissInputTokens").unwrap(), &serde_json::Value::Null);

    // 有 cache_miss
    let usage_with_miss = TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_miss_input_tokens: Some(5),
    };
    let json: serde_json::Value = serde_json::to_value(usage_with_miss).unwrap();
    assert_eq!(json.get("cacheMissInputTokens").unwrap(), &serde_json::json!(5));
}
