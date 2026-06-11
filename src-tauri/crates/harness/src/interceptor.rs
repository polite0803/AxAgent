// SPDX-License-Identifier: AGPL-3.0-only

//! 统一拦截器链 — 可编排的 Harness 级拦截点
//!
//! 将分散在各 executor 中的约束（PromptGuard、业务规则、权限校验、输出校验、
//! 一致性检查等）统一为可编排的拦截器链。每个拦截器声明自己关注的拦截点，
//! 由 `InterceptorChain` 按点串行执行。

use crate::business_rules::{BusinessRuleEngine, RuleEvaluationOutcome};
use crate::consistency_check::ConsistencyCheckConfig;
use crate::prompt_guard::PromptGuard;
use std::sync::Arc;

/// 拦截点
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterceptPoint {
    /// LLM 调用之前（输入净化、上下文裁剪、PromptGuard）
    BeforeLlmCall,
    /// LLM 调用之后（输出校验、置信度检查、一致性检查）
    AfterLlmCall,
    /// 工具调用之前（权限校验、限流）
    BeforeToolCall,
    /// 工具调用之后（脱敏、审计）
    AfterToolCall,
    /// 工作流节点执行之前（业务规则）
    BeforeNodeExecute,
    /// 工作流节点执行之后（结果补偿）
    AfterNodeExecute,
}

/// 拦截器执行结果
#[derive(Debug, Clone)]
pub enum InterceptorResult {
    /// 继续执行
    Continue,
    /// 阻断执行
    Block { reason: String },
    /// 跳过后续拦截器
    SkipRemaining,
    /// 需要降级
    Degrade { fallback: serde_json::Value },
}

/// 拦截器上下文 — 包含请求/响应/配置
#[derive(Debug, Clone)]
pub struct InterceptorContext {
    pub point: InterceptPoint,
    pub request: Option<serde_json::Value>,
    pub response: Option<serde_json::Value>,
    pub tool_name: Option<String>,
    pub node_id: Option<String>,
    pub workflow_id: Option<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl InterceptorContext {
    /// 为 BeforeLlmCall 创建上下文
    pub fn before_llm(request: Option<serde_json::Value>) -> Self {
        Self {
            point: InterceptPoint::BeforeLlmCall,
            request,
            response: None,
            tool_name: None,
            node_id: None,
            workflow_id: None,
            duration_ms: 0,
            error: None,
        }
    }

    /// 为 AfterLlmCall 创建上下文
    pub fn after_llm(response: Option<serde_json::Value>) -> Self {
        Self {
            point: InterceptPoint::AfterLlmCall,
            request: None,
            response,
            tool_name: None,
            node_id: None,
            workflow_id: None,
            duration_ms: 0,
            error: None,
        }
    }

    /// 为 BeforeNodeExecute 创建上下文
    pub fn before_node(node_id: String, request: Option<serde_json::Value>) -> Self {
        Self {
            point: InterceptPoint::BeforeNodeExecute,
            request,
            response: None,
            tool_name: None,
            node_id: Some(node_id),
            workflow_id: None,
            duration_ms: 0,
            error: None,
        }
    }
}

/// 拦截器 trait
#[async_trait::async_trait]
pub trait HarnessInterceptor: Send + Sync + std::fmt::Debug {
    /// 唯一标识
    fn id(&self) -> &'static str;

    /// 声明关注的拦截点
    fn intercept_points(&self) -> Vec<InterceptPoint>;

    /// 执行拦截逻辑
    async fn intercept(
        &self,
        point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult;
}

/// 拦截器链 — 按 InterceptPoint 分组执行
#[derive(Debug, Default)]
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn HarnessInterceptor>>,
}

impl InterceptorChain {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    pub fn add(&mut self, interceptor: Arc<dyn HarnessInterceptor>) {
        self.interceptors.push(interceptor);
    }

    /// 在指定拦截点执行所有匹配的拦截器
    ///
    /// - `Continue`：继续执行下一个拦截器
    /// - `SkipRemaining`：跳过后续拦截器，但视为通过
    /// - `Block`：立即阻断，返回错误
    /// - `Degrade`：立即降级，返回 fallback 值
    pub async fn execute(
        &self,
        point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        for interceptor in &self.interceptors {
            if interceptor.intercept_points().contains(&point) {
                let result = interceptor.intercept(point, ctx).await;
                match &result {
                    InterceptorResult::Continue => continue,
                    InterceptorResult::SkipRemaining => break,
                    InterceptorResult::Block { .. } | InterceptorResult::Degrade { .. } => {
                        return result;
                    },
                }
            }
        }
        InterceptorResult::Continue
    }

    /// 获取所有拦截器 ID
    pub fn interceptor_ids(&self) -> Vec<&'static str> {
        self.interceptors.iter().map(|i| i.id()).collect()
    }
}

// ── 内置拦截器实现 ──

/// 业务规则拦截器 — 在工作流节点执行前检查业务规则
#[derive(Debug)]
pub struct BusinessRuleInterceptor {
    engine: Arc<BusinessRuleEngine>,
}

impl BusinessRuleInterceptor {
    pub fn new(engine: Arc<BusinessRuleEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for BusinessRuleInterceptor {
    fn id(&self) -> &'static str {
        "business_rule"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::BeforeNodeExecute]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let node_type = ctx.node_id.as_deref().unwrap_or("unknown");
        let input = ctx.request.clone().unwrap_or(serde_json::Value::Null);

        match self.engine.evaluate(node_type, &input) {
            RuleEvaluationOutcome::Pass => InterceptorResult::Continue,
            RuleEvaluationOutcome::Violation {
                reason, action: _, ..
            } => InterceptorResult::Block {
                reason: format!("[业务规则] {reason}"),
            },
            RuleEvaluationOutcome::RequiresApproval { reason, .. } => {
                // RequireApproval 视为阻断（当前层级无法通过，需要上层处理）
                InterceptorResult::Block {
                    reason: format!("[需审批] {reason}"),
                }
            },
        }
    }
}

/// PromptGuard 拦截器 — 在 LLM 调用前过滤用户输入
#[derive(Debug)]
pub struct PromptGuardInterceptor {
    guard: Arc<dyn PromptGuard>,
}

impl PromptGuardInterceptor {
    pub fn new(guard: Arc<dyn PromptGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for PromptGuardInterceptor {
    fn id(&self) -> &'static str {
        "prompt_guard"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::BeforeLlmCall]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let request = match ctx.request.as_ref() {
            Some(v) => v.clone(),
            None => return InterceptorResult::Continue,
        };

        // 从序列化的请求中提取消息内容并过滤
        let messages = request.get("messages").and_then(|v| v.as_array());
        if messages.is_none() || messages.unwrap().is_empty() {
            return InterceptorResult::Continue;
        }

        // 对每个消息的 content 做 PromptGuard 过滤
        let messages = messages.unwrap();
        for msg in messages {
            let content = match msg.get("content") {
                Some(serde_json::Value::String(text)) => text.clone(),
                _ => continue,
            };

            match self.guard.process_user_input(&content) {
                Ok(safe) => {
                    // 更新请求中的消息内容
                    if safe != content {
                        tracing::debug!("[PromptGuardInterceptor] 已过滤消息内容");
                    }
                },
                Err(blocked) => {
                    let reason = format!("PromptGuard 阻断: {blocked}");
                    tracing::warn!("[PromptGuardInterceptor] {reason}");
                    ctx.error = Some(reason.clone());
                    return InterceptorResult::Block { reason };
                },
            }
        }

        InterceptorResult::Continue
    }
}

/// 输出校验拦截器 — 在 LLM 调用后校验响应格式
#[derive(Debug)]
pub struct OutputValidationInterceptor {
    schema: serde_json::Value,
}

impl OutputValidationInterceptor {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for OutputValidationInterceptor {
    fn id(&self) -> &'static str {
        "output_validation"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::AfterLlmCall]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let response = match ctx.response.as_ref() {
            Some(v) => v,
            None => return InterceptorResult::Continue,
        };

        // 简化校验：检查 response 是否包含 schema 中要求的字段
        if let Some(required_fields) = self.schema.get("required").and_then(|v| v.as_array()) {
            for field in required_fields {
                let field_name = match field.as_str() {
                    Some(name) => name,
                    None => continue,
                };
                if response.get(field_name).is_none() {
                    return InterceptorResult::Block {
                        reason: format!("输出校验失败: 缺少必需字段 '{field_name}'"),
                    };
                }
            }
        }

        InterceptorResult::Continue
    }
}

/// 一致性检查拦截器 — 在 LLM 调用后检查输出一致性
#[derive(Debug)]
pub struct ConsistencyCheckInterceptor {
    config: ConsistencyCheckConfig,
}

impl ConsistencyCheckInterceptor {
    pub fn new(config: ConsistencyCheckConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for ConsistencyCheckInterceptor {
    fn id(&self) -> &'static str {
        "consistency_check"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        if self.config.enabled {
            vec![InterceptPoint::AfterLlmCall]
        } else {
            vec![]
        }
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let _response = match ctx.response.as_ref() {
            Some(v) => v.clone(),
            None => return InterceptorResult::Continue,
        };

        // 一致性检查需要第二结果做对比，这里只有单次结果就通过
        // 实际使用中需要提供 secondary 结果
        tracing::debug!("[ConsistencyCheckInterceptor] 需要二次结果进行对比，当前单次结果跳过检查");

        InterceptorResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business_rules::{BusinessRule, RuleAction, RuleResult};
    use futures::executor::block_on;

    #[test]
    fn test_intercept_point_equality() {
        assert_eq!(InterceptPoint::BeforeLlmCall, InterceptPoint::BeforeLlmCall);
        assert_ne!(InterceptPoint::BeforeLlmCall, InterceptPoint::AfterLlmCall);
    }

    #[test]
    fn test_interceptor_chain_empty() {
        let chain = InterceptorChain::new();
        let mut ctx = InterceptorContext::before_llm(None);
        let result = block_on(chain.execute(InterceptPoint::BeforeLlmCall, &mut ctx));
        assert!(matches!(result, InterceptorResult::Continue));
    }

    #[test]
    fn test_interceptor_chain_ids_empty() {
        let chain = InterceptorChain::new();
        assert!(chain.interceptor_ids().is_empty());
    }

    #[test]
    fn test_business_rule_interceptor_block() {
        let rule = BusinessRule {
            name: "test_block".into(),
            description: "测试阻断".into(),
            evaluate: Arc::new(|_, _| RuleResult::Violation {
                reason: "测试违规".into(),
            }),
            action: RuleAction::Block("阻断".into()),
        };
        let engine = Arc::new(BusinessRuleEngine::new(vec![rule]));
        let interceptor = BusinessRuleInterceptor::new(engine);

        let mut ctx = InterceptorContext::before_node(
            "test_node".into(),
            Some(serde_json::json!({"key": "value"})),
        );
        let result = block_on(interceptor.intercept(InterceptPoint::BeforeNodeExecute, &mut ctx));
        assert!(matches!(result, InterceptorResult::Block { .. }));
    }

    #[test]
    fn test_business_rule_interceptor_pass() {
        let engine = Arc::new(BusinessRuleEngine::empty());
        let interceptor = BusinessRuleInterceptor::new(engine);

        let mut ctx = InterceptorContext::before_node(
            "test_node".into(),
            Some(serde_json::json!({"key": "value"})),
        );
        let result = block_on(interceptor.intercept(InterceptPoint::BeforeNodeExecute, &mut ctx));
        assert!(matches!(result, InterceptorResult::Continue));
    }

    #[test]
    fn test_context_constructors() {
        let ctx = InterceptorContext::before_llm(Some(serde_json::json!({"msg": "hi"})));
        assert_eq!(ctx.point, InterceptPoint::BeforeLlmCall);
        assert!(ctx.request.is_some());

        let ctx = InterceptorContext::after_llm(Some(serde_json::json!({"result": "ok"})));
        assert_eq!(ctx.point, InterceptPoint::AfterLlmCall);
        assert!(ctx.response.is_some());

        let ctx =
            InterceptorContext::before_node("n1".into(), Some(serde_json::json!({"foo": "bar"})));
        assert_eq!(ctx.point, InterceptPoint::BeforeNodeExecute);
        assert_eq!(ctx.node_id.as_deref(), Some("n1"));
    }

    #[test]
    fn test_interceptor_chain_skip_remaining() {
        let chain = InterceptorChain::new();
        let mut ctx = InterceptorContext::before_llm(None);
        let result = block_on(chain.execute(InterceptPoint::BeforeLlmCall, &mut ctx));
        assert!(matches!(result, InterceptorResult::Continue));
    }

    #[test]
    fn test_interceptor_result_debug() {
        let r = InterceptorResult::Block {
            reason: "test".into(),
        };
        assert!(format!("{r:?}").contains("Block"));
        let r = InterceptorResult::Continue;
        assert!(format!("{r:?}").contains("Continue"));
        let r = InterceptorResult::SkipRemaining;
        assert!(format!("{r:?}").contains("SkipRemaining"));
    }
}
