use axagent_harness::ToolRegistry;
use std::sync::Arc;

use crate::action_executor::ActionError;
use crate::reasoning_state::ActionType;
use crate::self_verifier::validate_json_output;
use crate::thought_chain::Action;

/// 执行前校验错误
#[derive(Debug, thiserror::Error)]
pub enum PreValidationError {
    #[error("缺少工具名称")]
    MissingToolName,
    #[error("工具 '{0}' 未注册")]
    ToolNotFound(String),
    #[error("工具 '{tool}' 的参数不符合 schema: {errors:?}")]
    SchemaMismatch { tool: String, errors: Vec<String> },
    #[error("破坏性操作 '{0}' 需要用户确认")]
    DestructiveActionRequiresConfirmation(String),
}

/// 执行前校验器
///
/// 在工具调用前校验：
/// 1. 工具是否存在
/// 2. 参数是否符合工具的 JSON Schema
/// 3. 破坏性操作是否需要确认
pub struct PreExecutionValidator {
    tool_registry: Arc<dyn ToolRegistry>,
}

impl PreExecutionValidator {
    pub fn new(tool_registry: Arc<dyn ToolRegistry>) -> Self {
        Self { tool_registry }
    }

    /// 校验一个 Action 是否可安全执行
    ///
    /// 返回 `Ok(())` 表示通过，`Err(PreValidationError)` 表示校验失败。
    pub fn validate_action(&self, action: &Action) -> Result<(), PreValidationError> {
        match action.action_type {
            ActionType::ToolCall => self.validate_tool_call(action),
            _ => Ok(()),
        }
    }

    fn validate_tool_call(&self, action: &Action) -> Result<(), PreValidationError> {
        let tool_name = action
            .tool_name
            .as_deref()
            .ok_or(PreValidationError::MissingToolName)?;

        let tool = self
            .tool_registry
            .find(tool_name)
            .ok_or_else(|| PreValidationError::ToolNotFound(tool_name.to_string()))?;

        // 参数 schema 校验
        if let Some(ref input) = action.tool_input {
            let schema = tool.input_schema();
            if schema.is_object() && !schema.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                let input_str = serde_json::to_string(input).unwrap_or_default();
                let validation = validate_json_output(&input_str, Some(&schema));
                if !validation.schema_compliant {
                    return Err(PreValidationError::SchemaMismatch {
                        tool: tool_name.to_string(),
                        errors: validation.errors,
                    });
                }
            }
        }

        // 破坏性操作确认检查
        if tool.is_destructive() && !action.requires_confirmation {
            return Err(PreValidationError::DestructiveActionRequiresConfirmation(
                tool_name.to_string(),
            ));
        }

        Ok(())
    }
}

impl From<PreValidationError> for ActionError {
    fn from(err: PreValidationError) -> Self {
        match err {
            PreValidationError::MissingToolName => {
                ActionError::InvalidAction("缺少工具名称".to_string())
            },
            PreValidationError::ToolNotFound(name) => {
                ActionError::InvalidAction(format!("工具 '{}' 未注册", name))
            },
            PreValidationError::SchemaMismatch { tool, errors } => ActionError::InvalidAction(
                format!("工具 '{}' 参数不符合 schema: {}", tool, errors.join(", ")),
            ),
            PreValidationError::DestructiveActionRequiresConfirmation(name) => {
                ActionError::PermissionDenied(format!("破坏性操作 '{}' 需要用户确认", name))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_tools::registry::ToolRegistry as ConcreteToolRegistry;

    #[test]
    fn test_missing_tool_name() {
        let registry = Arc::new(ConcreteToolRegistry::new()) as Arc<dyn ToolRegistry>;
        let validator = PreExecutionValidator::new(registry);

        let action = Action {
            action_type: ActionType::ToolCall,
            tool_name: None,
            tool_input: None,
            llm_prompt: None,
            requires_confirmation: false,
        };

        let result = validator.validate_action(&action);
        assert!(matches!(result, Err(PreValidationError::MissingToolName)));
    }

    #[test]
    fn test_tool_not_found() {
        let registry = Arc::new(ConcreteToolRegistry::new()) as Arc<dyn ToolRegistry>;
        let validator = PreExecutionValidator::new(registry);

        let action = Action {
            action_type: ActionType::ToolCall,
            tool_name: Some("nonexistent_tool".to_string()),
            tool_input: Some(serde_json::json!({})),
            llm_prompt: None,
            requires_confirmation: false,
        };

        let result = validator.validate_action(&action);
        assert!(matches!(result, Err(PreValidationError::ToolNotFound(_))));
    }

    #[test]
    fn test_non_tool_actions_pass_through() {
        let registry = Arc::new(ConcreteToolRegistry::new()) as Arc<dyn ToolRegistry>;
        let validator = PreExecutionValidator::new(registry);

        let action = Action::llm_call("test prompt");
        assert!(validator.validate_action(&action).is_ok());

        let action = Action::user_confirm("confirm?");
        assert!(validator.validate_action(&action).is_ok());
    }
}
