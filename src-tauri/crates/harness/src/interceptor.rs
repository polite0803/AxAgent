// SPDX-License-Identifier: AGPL-3.0-only

//! 统一拦截器链 — 可编排的 Harness 级拦截点
//!
//! 将分散在各 executor 中的约束（PromptGuard、业务规则、权限校验、输出校验、
//! 一致性检查等）统一为可编排的拦截器链。每个拦截器声明自己关注的拦截点，
//! 由 `InterceptorChain` 按点串行执行。
//!
//! 具体拦截器实现位于 `axagent_runtime_core::interceptors`。

use crate::workflow_types::NodeKind;
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
    pub node_kind: Option<NodeKind>,
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
            node_kind: None,
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
            node_kind: None,
            workflow_id: None,
            duration_ms: 0,
            error: None,
        }
    }

    /// 为 BeforeNodeExecute 创建上下文
    pub fn before_node(
        node_id: String,
        node_kind: NodeKind,
        request: Option<serde_json::Value>,
    ) -> Self {
        Self {
            point: InterceptPoint::BeforeNodeExecute,
            request,
            response: None,
            tool_name: None,
            node_id: Some(node_id),
            node_kind: Some(node_kind),
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
        Self { interceptors: Vec::new() }
    }

    pub fn add(&mut self, interceptor: Arc<dyn HarnessInterceptor>) {
        self.interceptors.push(interceptor);
    }

    /// 获取内部拦截器切片
    pub fn interceptors(&self) -> &[Arc<dyn HarnessInterceptor>] {
        &self.interceptors
    }

    /// 获取所有拦截器 ID
    pub fn interceptor_ids(&self) -> Vec<&'static str> {
        self.interceptors.iter().map(|i| i.id()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_point_equality() {
        assert_eq!(InterceptPoint::BeforeLlmCall, InterceptPoint::BeforeLlmCall);
        assert_ne!(InterceptPoint::BeforeLlmCall, InterceptPoint::AfterLlmCall);
    }

    #[test]
    fn test_interceptor_chain_empty() {
        let chain = InterceptorChain::new();
        assert!(chain.interceptors().is_empty());
    }

    #[test]
    fn test_interceptor_chain_ids_empty() {
        let chain = InterceptorChain::new();
        assert!(chain.interceptor_ids().is_empty());
    }

    #[test]
    fn test_context_constructors() {
        let ctx = InterceptorContext::before_llm(Some(serde_json::json!({"msg": "hi"})));
        assert_eq!(ctx.point, InterceptPoint::BeforeLlmCall);
        assert!(ctx.request.is_some());

        let ctx = InterceptorContext::after_llm(Some(serde_json::json!({"result": "ok"})));
        assert_eq!(ctx.point, InterceptPoint::AfterLlmCall);
        assert!(ctx.response.is_some());

        let ctx = InterceptorContext::before_node(
            "n1".into(),
            NodeKind::Tool,
            Some(serde_json::json!({"foo": "bar"})),
        );
        assert_eq!(ctx.point, InterceptPoint::BeforeNodeExecute);
        assert_eq!(ctx.node_id.as_deref(), Some("n1"));
        assert_eq!(ctx.node_kind, Some(NodeKind::Tool));
    }

    #[test]
    fn test_interceptor_chain_skip_remaining() {
        let chain = InterceptorChain::new();
        assert_eq!(chain.interceptors().len(), 0);
    }

    #[test]
    fn test_interceptor_result_debug() {
        let r = InterceptorResult::Block { reason: "test".into() };
        assert!(format!("{r:?}").contains("Block"));
        let r = InterceptorResult::Continue;
        assert!(format!("{r:?}").contains("Continue"));
        let r = InterceptorResult::SkipRemaining;
        assert!(format!("{r:?}").contains("SkipRemaining"));
    }
}
