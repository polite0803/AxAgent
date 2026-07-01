// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 错误上下文 —— 在 Error Workflow 中通过 $error / _error 变量访问。
///
/// 当节点执行失败且配置了 RunErrorBranch 或 error_workflow_id 时，
/// 引擎构造此上下文并注入到 ExecutionState 变量中，供错误处理
/// 工作流引用失败节点的详细信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub failed_node_id: String,
    pub failed_node_name: String,
    pub error_code: String,
    pub error_message: String,
    pub workflow_id: String,
    pub execution_id: String,
    pub timestamp: i64,
    pub last_output: Option<Value>,
}

impl ErrorContext {
    pub fn new(
        node_id: String,
        node_name: String,
        error_code: String,
        error_message: String,
        workflow_id: String,
        execution_id: String,
        last_output: Option<Value>,
    ) -> Self {
        Self {
            failed_node_id: node_id,
            failed_node_name: node_name,
            error_code,
            error_message,
            workflow_id,
            execution_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
            last_output,
        }
    }

    /// 获取可在模板中引用的变量名。
    pub const fn variable_name() -> &'static str {
        "_error"
    }

    /// 将错误上下文序列化为 Value，注入到 variables 中。
    pub fn to_variable(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}
