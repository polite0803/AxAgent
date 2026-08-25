// SPDX-License-Identifier: AGPL-3.0-only

//! 守卫条件评估器
//!
//! 使用 Rhai 脚本引擎评估状态转移的守卫条件表达式。
//! 守卫条件是动态业务规则，决定何时允许状态转移。
//!
//! # 架构位置
//! - 实现层：rt-workflow（hybrid 层）
//! - 依赖：harness::business_state_machine（FSM 定义）+ rhai（脚本引擎）
//! - 被 FsmExecutor 调用，在状态转移前评估

use axagent_harness::business_state_machine::{FsmContext, FsmTransitionError, StateTransition};
use rhai::{Dynamic, Engine, Map};
use std::sync::Arc;

/// 守卫条件评估器
///
/// 负责评估状态转移的守卫条件表达式。
/// 使用 Rhai 作为嵌入式脚本语言，支持复杂的业务规则定义。
pub struct GuardEvaluator {
    /// Rhai 引擎（线程安全，可复用）
    engine: Arc<Engine>,
}

impl GuardEvaluator {
    /// 创建新的守卫条件评估器
    pub fn new() -> Self {
        let engine = Engine::new();
        Self { engine: Arc::new(engine) }
    }

    /// 评估守卫条件
    ///
    /// # 参数
    /// - `transition`: 状态转移规则（包含守卫条件表达式）
    /// - `context`: 运行时上下文（变量、用户角色等）
    ///
    /// # 返回
    /// - `Ok(true)`: 守卫条件满足，允许转移
    /// - `Ok(false)`: 守卫条件不满足，阻止转移
    /// - `Err(FsmTransitionError::GuardFailed)`: 表达式执行错误
    pub fn evaluate(
        &self,
        transition: &StateTransition,
        context: &FsmContext,
    ) -> Result<bool, FsmTransitionError> {
        // 如果没有守卫条件，直接允许
        let guard_expr = match &transition.guard_expr {
            Some(expr) => expr,
            None => return Ok(true),
        };

        // 准备上下文环境
        let mut scope_map = Map::new();

        // 注入用户角色
        if let Some(role) = &context.user_role {
            scope_map.insert("user_role".into(), Dynamic::from(role.clone()));
        }

        // 注入事件
        if let Some(event) = &context.event {
            scope_map.insert("event".into(), Dynamic::from(event.clone()));
        }

        // 注入事件数据
        if let Some(event_data) = &context.event_data {
            let data_dynamic = json_to_dynamic(event_data);
            scope_map.insert("event_data".into(), data_dynamic);
        }

        // 注入自定义变量
        let mut variables_map = Map::new();
        for (key, value) in &context.variables {
            variables_map.insert(key.into(), json_to_dynamic(value));
        }
        scope_map.insert("variables".into(), Dynamic::from(variables_map));

        // 执行守卫条件表达式
        let mut scope = rhai::Scope::new();

        // 将上下文变量注入到 Rhai 作用域
        scope.push("user_role", scope_map.get("user_role").cloned().unwrap_or(Dynamic::UNIT));
        scope.push("event", scope_map.get("event").cloned().unwrap_or(Dynamic::UNIT));
        scope.push("event_data", scope_map.get("event_data").cloned().unwrap_or(Dynamic::UNIT));

        // 创建 variables 作为 Rhai Map
        let vars_map =
            scope_map.get("variables").cloned().unwrap_or_else(|| Dynamic::from(Map::new()));
        scope.push("variables", vars_map);

        let result = self.engine.eval_with_scope::<bool>(&mut scope, guard_expr);

        match result {
            Ok(allowed) => Ok(allowed),
            Err(e) => Err(FsmTransitionError::GuardFailed {
                transition_id: transition.id.clone(),
                reason: format!("守卫条件表达式执行错误: {e}"),
            }),
        }
    }

    /// 批量评估多个转移的守卫条件
    ///
    /// # 参数
    /// - `transitions`: 候选的状态转移列表
    /// - `context`: 运行时上下文
    ///
    /// # 返回
    /// 返回第一个守卫条件满足的转移，用于冲突解决
    pub fn evaluate_transitions<'a>(
        &self,
        transitions: &'a [&StateTransition],
        context: &FsmContext,
    ) -> Option<&'a StateTransition> {
        for transition in transitions {
            // 如果没有守卫条件，直接返回
            if !transition.has_guard() {
                return Some(transition);
            }

            // 评估守卫条件
            match self.evaluate(transition, context) {
                Ok(true) => return Some(transition),
                Ok(false) => continue,
                Err(e) => {
                    tracing::warn!("守卫条件评估错误: {e}");
                    continue;
                },
            }
        }
        None
    }
}

impl Default for GuardEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GuardEvaluator {
    fn clone(&self) -> Self {
        Self { engine: self.engine.clone() }
    }
}

// ── 辅助函数 ──

/// 将 serde_json::Value 转换为 Rhai Dynamic
fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::from(n.to_string())
            }
        },
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<Dynamic> = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(items)
        },
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (key, value) in obj {
                map.insert(key.as_str().into(), json_to_dynamic(value));
            }
            Dynamic::from(map)
        },
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::business_state_machine::StateTransition;

    #[test]
    fn test_guard_without_expression() {
        let evaluator = GuardEvaluator::new();
        let transition = StateTransition::new("a", "b");
        let context = FsmContext::new();

        // 没有守卫条件，应该直接允许
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_guard_role_check() {
        let evaluator = GuardEvaluator::new();
        let transition = StateTransition::new("a", "b")
            .with_guard_expr("user_role == \"manager\"")
            .with_guard_description("需要管理员角色");

        // 管理员角色，应该允许
        let context = FsmContext::new().with_user_role("manager");
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 普通角色，应该拒绝
        let context = FsmContext::new().with_user_role("user");
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_guard_variable_check() {
        let evaluator = GuardEvaluator::new();
        let transition = StateTransition::new("a", "b")
            .with_guard_expr("variables.amount > 1000")
            .with_guard_description("金额必须大于1000");

        // 金额足够，应该允许
        let context = FsmContext::new().with_variable("amount", serde_json::json!(1500));
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 金额不足，应该拒绝
        let context = FsmContext::new().with_variable("amount", serde_json::json!(500));
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_guard_complex_expression() {
        let evaluator = GuardEvaluator::new();
        let transition = StateTransition::new("a", "b")
            .with_guard_expr("user_role == \"manager\" && variables.amount > 1000")
            .with_guard_description("管理员且金额大于1000");

        // 条件全部满足
        let context = FsmContext::new()
            .with_user_role("manager")
            .with_variable("amount", serde_json::json!(2000));
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 只满足部分条件
        let context = FsmContext::new()
            .with_user_role("user")
            .with_variable("amount", serde_json::json!(2000));
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_guard_invalid_expression() {
        let evaluator = GuardEvaluator::new();
        let transition = StateTransition::new("a", "b").with_guard_expr("invalid expression {{{}");

        let context = FsmContext::new();
        let result = evaluator.evaluate(&transition, &context);
        assert!(result.is_err());
        match result.unwrap_err() {
            FsmTransitionError::GuardFailed { reason, .. } => {
                assert!(reason.contains("执行错误"));
            },
            other => panic!("预期 GuardFailed，实际: {other:?}"),
        }
    }

    #[test]
    fn test_guard_has_expression() {
        let transition = StateTransition::new("a", "b");
        assert!(!transition.has_guard());

        let transition = transition.with_guard_expr("true");
        assert!(transition.has_guard());
    }
}
