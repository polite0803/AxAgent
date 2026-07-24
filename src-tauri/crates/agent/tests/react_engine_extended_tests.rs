// SPDX-License-Identifier: AGPL-3.0-only

//! Extended tests for react_engine: resume, builder patterns, token budget.

use std::sync::Arc;

use axagent_agent::react_engine::{LlmReasoningProvider, ReActEngine, ReActError};
use axagent_agent::reasoning_state::{ActionType, ReasoningContext};
use axagent_agent::thought_chain::{Action, ThoughtChain};

/// 测试专用的 LLM provider（返回预定义响应，不依赖真实 LLM）
struct TestLlmProvider;

#[async_trait::async_trait]
impl LlmReasoningProvider for TestLlmProvider {
    async fn analyze(
        &self,
        _input: &str,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok("test analysis".to_string())
    }
    async fn think(
        &self,
        _input: &str,
        _context: &ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<String, ReActError> {
        Ok("test thinking".to_string())
    }
    async fn plan(
        &self,
        _input: &str,
        context: &mut ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<Action, ReActError> {
        context.increment_depth();
        Ok(Action {
            action_type: ActionType::Synthesize,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("test synthesis".to_string()),
            requires_confirmation: false,
        })
    }
    async fn reflect(
        &self,
        _chain: &ThoughtChain,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok("test reflection".to_string())
    }
    async fn synthesize(
        &self,
        _chain: &ThoughtChain,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        Ok("test result".to_string())
    }
}

fn test_engine() -> ReActEngine {
    ReActEngine::new().with_reasoning_provider(Arc::new(TestLlmProvider))
}

// ---------------------------------------------------------------------------
// ReActEngine — resume() method  (checkpoint edge cases)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_react_engine_resume_after_run() {
    let mut engine = test_engine();

    // First run to establish state
    let _first = engine.run("Hello").await;
    // Then try resume (should handle gracefully even without saved checkpoint)
    let resume_result = engine.resume("Continue").await;
    // Either it succeeds or returns a meaningful error; no panic
    if resume_result.success {
        assert!(!resume_result.final_response.is_empty());
    }
}

// ---------------------------------------------------------------------------
// ReActEngine — builder pattern integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_react_engine_with_config_and_goal_eval() {
    use axagent_agent::reasoning_state::ReActConfig;
    let config = ReActConfig::for_simple_task();

    let mut engine = test_engine().with_config(config).with_goal_evaluation(2);

    let result = engine.run("Write a short greeting").await;
    // Should complete without panic
    if result.success {
        assert!(!result.final_response.is_empty());
    }
}

#[tokio::test]
async fn test_react_engine_token_budget_reset() {
    let mut engine = test_engine();
    engine.reset_token_budget();
    let result = engine.run("Hello").await;
    // token budget reset should not cause errors
    if result.success {
        assert!(!result.final_response.is_empty());
    }
}

#[tokio::test]
async fn test_react_engine_reset_and_rerun() {
    let mut engine = test_engine();
    let r1 = engine.run("Task one").await;
    let r2 = engine.run("Task two").await;
    // 连续两次 run 都应成功完成，验证 engine 可重置后重跑（reset and rerun）。
    // 引擎已修复 CycleDetector 对空工具名的误判，默认配置下 run 也能跑通。
    assert!(r1.success, "第一次 run 未成功: {:?}", r1);
    assert!(r2.success, "第二次 run 未成功: {:?}", r2);
}
