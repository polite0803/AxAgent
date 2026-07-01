// SPDX-License-Identifier: AGPL-3.0-only

pub mod rhai_eval;
pub mod template;

pub use rhai_eval::{RhaiEvalError, resolve_expression};
pub use template::{TemplateError, resolve_template, resolve_value_templates};

use serde_json::Value;
use std::collections::HashMap;

/// 表达式求值上下文 —— 注入每个节点执行时的求值环境
#[derive(Debug, Clone)]
pub struct ExpressionContext {
    /// 全局变量：$vars.xxx（即 ExecutionState.variables）
    pub variables: HashMap<String, Value>,
    /// 节点输出：$node["NodeName"].output.field
    pub node_outputs: HashMap<String, Value>,
    /// 当前节点的输入参数：$input.xxx
    pub input_params: Value,
    /// 环境变量：$env.xxx
    pub env: HashMap<String, String>,
}

impl ExpressionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            node_outputs: HashMap::new(),
            input_params: Value::Null,
            env: std::env::vars().collect(),
        }
    }
}
