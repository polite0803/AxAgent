// SPDX-License-Identifier: AGPL-3.0-only

//! Mission decomposition strategies.
//!
//! Defines the [`MissionDecomposer`] trait for decomposing a high-level
//! mission into structured [`SubTask`]s, along with two implementations:
//!
//! - [`RuleBasedDecomposer`] — keyword matching (original, fast, no deps)
//! - [`LlmBasedDecomposer`] — LLM-driven decomposition via structured JSON output
//!
//! The [`OrchestratorExecutor`] defaults to [`RuleBasedDecomposer`] and
//! can be injected with [`LlmBasedDecomposer`] when a provider is available.

use std::sync::Arc;

use crate::types::{DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask};
use axagent_harness::llm_execution::LlmExecutionService;
use axagent_harness::provider::{ProviderAdapter, ProviderRequestContext};
use axagent_harness::types::ChatContent;
use serde::Deserialize;

// ── Trait ──────────────────────────────────────────────────────────────

/// Strategy for decomposing a mission into sub-tasks.
pub trait MissionDecomposer: Send + Sync {
    /// Decompose a mission into a [`DecompositionPlan`].
    fn decompose(
        &self,
        mission: &str,
        strategy: OrchestrationStrategy,
    ) -> Result<DecompositionPlan, OrchestrationError>;
}

// ── Rule-based (fallback) ─────────────────────────────────────────────

/// 关键词匹配的规则化分解器（**默认兜底实现**）。
///
/// 检测常见术语 — review、refactor、design — 并生成固定模板子任务 DAG。
/// 快速、确定性、无外部依赖。
///
/// **架构定位**：
/// - 作为 `OrchestratorExecutor` 的**默认 decomposer**，无需 LLM 即可工作。
/// - 作为 [`LlmBasedDecomposer`] 的**降级兜底**：当 LLM 调用失败、响应解析失败、
///   或返回空结果时，自动回退到本规则分解器，保证 mission 分解链路始终可用。
/// - 生产环境如需语义化分解，应通过 wiring 层注入 [`LlmBasedDecomposer`]。
///
/// **注意**：本 decomposer **不返回 Err**，规则匹配失败时走 default 分支
/// 生成通用的 analyze→implement→review 三段式 DAG，确保调用方总能拿到合法 plan。
pub struct RuleBasedDecomposer;

impl RuleBasedDecomposer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RuleBasedDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionDecomposer for RuleBasedDecomposer {
    fn decompose(
        &self,
        mission: &str,
        strategy: OrchestrationStrategy,
    ) -> Result<DecompositionPlan, OrchestrationError> {
        let mission_lower = mission.to_lowercase();
        let mut plan = DecompositionPlan::new(mission.to_string(), strategy);

        let phase_count = if mission_lower.contains("review")
            || mission_lower.contains("audit")
            || mission_lower.contains("inspect")
        {
            plan.sub_tasks.push(SubTask::new(
                "analyze".to_string(),
                "Analyze".to_string(),
                format!("Analyze the codebase/documents for: {}", mission),
                "researcher".to_string(),
            ));

            plan.sub_tasks.push(
                SubTask::new(
                    "review".to_string(),
                    "Review".to_string(),
                    "Review findings from analysis, identify issues".to_string(),
                    "reviewer".to_string(),
                )
                .with_dependencies(vec!["analyze".to_string()]),
            );

            plan.sub_tasks.push(
                SubTask::new(
                    "report".to_string(),
                    "Report".to_string(),
                    "Compile review findings into structured report".to_string(),
                    "synthesizer".to_string(),
                )
                .with_dependencies(vec!["review".to_string()]),
            );

            3
        } else if mission_lower.contains("refactor")
            || mission_lower.contains("rewrite")
            || mission_lower.contains("restructure")
        {
            plan.sub_tasks.push(SubTask::new(
                "analyze".to_string(),
                "Analyze".to_string(),
                format!("Analyze current code structure for: {}", mission),
                "researcher".to_string(),
            ));

            plan.sub_tasks.push(
                SubTask::new(
                    "plan".to_string(),
                    "Plan Refactor".to_string(),
                    "Create refactoring plan with migration steps".to_string(),
                    "planner".to_string(),
                )
                .with_dependencies(vec!["analyze".to_string()]),
            );

            plan.sub_tasks.push(
                SubTask::new(
                    "implement".to_string(),
                    "Implement".to_string(),
                    "Execute the refactoring changes".to_string(),
                    "developer".to_string(),
                )
                .with_dependencies(vec!["plan".to_string()]),
            );

            plan.sub_tasks.push(
                SubTask::new(
                    "verify".to_string(),
                    "Verify".to_string(),
                    "Verify refactored code works correctly".to_string(),
                    "reviewer".to_string(),
                )
                .with_dependencies(vec!["implement".to_string()]),
            );

            4
        } else if mission_lower.contains("design")
            || mission_lower.contains("architect")
            || mission_lower.contains("plan")
        {
            plan.sub_tasks.push(SubTask::new(
                "research".to_string(),
                "Research".to_string(),
                format!("Research requirements and constraints for: {}", mission),
                "researcher".to_string(),
            ));

            plan.sub_tasks.push(
                SubTask::new(
                    "design".to_string(),
                    "Design".to_string(),
                    "Create the design/architecture".to_string(),
                    "planner".to_string(),
                )
                .with_dependencies(vec!["research".to_string()]),
            );

            plan.sub_tasks.push(
                SubTask::new(
                    "review".to_string(),
                    "Review Design".to_string(),
                    "Review the design for completeness and correctness".to_string(),
                    "reviewer".to_string(),
                )
                .with_dependencies(vec!["design".to_string()]),
            );

            3
        } else {
            plan.sub_tasks.push(SubTask::new(
                "analyze".to_string(),
                "Analyze Requirements".to_string(),
                format!("Analyze and understand: {}", mission),
                "researcher".to_string(),
            ));

            plan.sub_tasks.push(
                SubTask::new(
                    "implement".to_string(),
                    "Implement".to_string(),
                    format!("Implement the solution for: {}", mission),
                    "developer".to_string(),
                )
                .with_dependencies(vec!["analyze".to_string()]),
            );

            plan.sub_tasks.push(
                SubTask::new(
                    "review".to_string(),
                    "Review".to_string(),
                    "Review the implementation for correctness".to_string(),
                    "reviewer".to_string(),
                )
                .with_dependencies(vec!["implement".to_string()]),
            );

            3
        };

        plan.max_parallel = match strategy {
            OrchestrationStrategy::FanOut => phase_count as u32,
            _ => 2,
        };

        Ok(plan)
    }
}

// ── LLM-based ──────────────────────────────────────────────────────────

/// LLM-driven mission decomposition.
///
/// Sends the mission to an LLM with a structured prompt that asks for a
/// JSON array of sub-tasks. Falls back to [`RuleBasedDecomposer`] on
/// any failure (network error, parse error, empty response).
///
/// **架构约束（设计时工具）**：本 decomposer 在拆任务时会调用 LLM，违反
/// 「运行时不调 LLM」的稳定性铁律。仅应在**设计时**（如 mission 编译、
/// workflow_template 设计阶段）使用；**运行时**必须直接使用已编译的
/// `workflow_template`，不得调用本 decomposer。详见 AGENTS.md「运行时边界」。
pub struct LlmBasedDecomposer {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    llm_service: Arc<dyn LlmExecutionService>,
    fallback: RuleBasedDecomposer,
    /// 业务岗位/专家清单 brief（由调用方在构造时 async 获取后注入）。
    /// 为 None 时 prompt 中不插入清单 section。
    roles_and_experts_brief: Option<String>,
}

impl LlmBasedDecomposer {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        llm_service: Arc<dyn LlmExecutionService>,
    ) -> Self {
        Self {
            adapter,
            ctx,
            llm_service,
            fallback: RuleBasedDecomposer,
            roles_and_experts_brief: None,
        }
    }

    /// 注入业务岗位/专家清单 brief（由调用方 async 获取后传入）。
    ///
    /// brief 格式建议：
    /// ```text
    /// === 可用业务岗位 ===
    /// - CEO：负责战略决策与资源分配
    /// - CTO：负责技术决策与团队管理
    ///
    /// === 可用专家 ===
    /// - securities_analyst：证券分析师，擅长股票/债券估值
    /// - lawyer：律师，擅长合同审查/合规
    /// ```
    pub fn with_roles_and_experts_brief(mut self, brief: impl Into<String>) -> Self {
        self.roles_and_experts_brief = Some(brief.into());
        self
    }
}

/// Expected JSON structure from the LLM decomposition response.
#[derive(Debug, Deserialize)]
struct LlmSubTask {
    id: String,
    name: String,
    description: String,
    role: String,
    dependencies: Vec<String>,
}

impl MissionDecomposer for LlmBasedDecomposer {
    fn decompose(
        &self,
        mission: &str,
        strategy: OrchestrationStrategy,
    ) -> Result<DecompositionPlan, OrchestrationError> {
        // 业务岗位/专家清单 section（仅在调用方注入 brief 时插入）
        let roles_section = self
            .roles_and_experts_brief
            .as_ref()
            .map(|brief| format!("\n## Available Business Roles & Experts\n{brief}\n"))
            .unwrap_or_default();

        let prompt = format!(
            r#"You are a task decomposition engine. Given a mission and an orchestration strategy, break it into 2–8 sub-tasks.

## Rules
- Each sub-task must have a **short unique id** (snake_case, e.g. "parse_config")
- Each sub-task must have a **role** from: Researcher, Developer, Reviewer, Planner, Synthesizer, Executor, Coordinator, Browser
- If strategy is "ordered" or "pipeline": add sequential dependencies
- If strategy is "fan_out" or "race": keep dependencies minimal
- If strategy is "debate": assign the last sub-task as adjudicator, all others feed into it
- If strategy is "dynamic": let the LLM decide the topology naturally
- 若提供了「Available Business Roles & Experts」清单，在 description 中说明该子任务
  建议由哪个业务岗位/专家负责（如 "建议由证券分析师执行"），便于后续分配 AgentProfile。
{roles_section}
## Mission
{mission}

## Strategy
{strategy}

Respond with ONLY a JSON object:
{{
  "sub_tasks": [
    {{
      "id": "unique_id",
      "name": "Human-readable name",
      "description": "Detailed description for the worker agent",
      "role": "Researcher|Developer|Reviewer|Planner|Synthesizer|Executor|Coordinator|Browser",
      "dependencies": ["dependency_id_1"]
    }}
  ]
}}
"#,
            roles_section = roles_section,
            mission = mission,
            strategy = strategy.as_str(),
        );

        let config = axagent_harness::llm_execution::LlmCallConfig::default();
        let request = axagent_harness::types::ChatRequest {
            model: String::new(),
            messages: vec![
                axagent_harness::types::ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(
                        "You are a precise task decomposition engine. Output only valid JSON."
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                axagent_harness::types::ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            top_p: None,
            max_tokens: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
            response_format: None,
        };

        let request_json = match serde_json::to_value(&request) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to serialize ChatRequest, falling back to rule-based");
                return self.fallback.decompose(mission, strategy);
            },
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.llm_service.execute(&*self.adapter, &self.ctx, request_json, &config).await
            })
        });

        let response_text = match result {
            Ok(r) => r.content,
            Err(e) => {
                tracing::warn!(error = %e, "LLM decompose failed, falling back to rule-based");
                return self.fallback.decompose(mission, strategy);
            },
        };

        // Try to parse JSON from the response
        let json_text = extract_json(&response_text);
        let llm_sub_tasks: Vec<LlmSubTask> = match serde_json::from_str::<serde_json::Value>(
            &json_text,
        ) {
            Ok(val) => {
                let tasks = val
                    .get("sub_tasks")
                    .and_then(|v| serde_json::from_value::<Vec<LlmSubTask>>(v.clone()).ok());
                match tasks {
                    Some(t) if !t.is_empty() => t,
                    _ => {
                        tracing::warn!(
                            "LLM decompose returned empty or invalid sub_tasks, falling back to rule-based"
                        );
                        return self.fallback.decompose(mission, strategy);
                    },
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "LLM decompose JSON parse failed, falling back to rule-based");
                return self.fallback.decompose(mission, strategy);
            },
        };

        // Validate and limit sub-tasks
        if llm_sub_tasks.len() > 8 {
            tracing::warn!(
                count = llm_sub_tasks.len(),
                "LLM returned too many sub-tasks, capping at 8"
            );
        }

        let mut plan = DecompositionPlan::new(mission.to_string(), strategy);
        for (i, lst) in llm_sub_tasks.into_iter().take(8).enumerate() {
            let role = match lst.role.to_lowercase().as_str() {
                "researcher" => "researcher".to_string(),
                "developer" => "developer".to_string(),
                "reviewer" => "reviewer".to_string(),
                "planner" => "planner".to_string(),
                "synthesizer" => "synthesizer".to_string(),
                "executor" => "executor".to_string(),
                "coordinator" => "coordinator".to_string(),
                "browser" => "browser".to_string(),
                _ => "developer".to_string(),
            };

            let id = if lst.id.is_empty() {
                format!("task_{}", i)
            } else {
                lst.id
            };

            let mut sub_task = SubTask::new(id, lst.name, lst.description, role);

            // Validate dependencies — only reference existing task ids
            let valid_deps: Vec<String> = lst
                .dependencies
                .into_iter()
                .filter(|dep| plan.sub_tasks.iter().any(|t| t.id == *dep))
                .collect();

            if !valid_deps.is_empty() {
                sub_task = sub_task.with_dependencies(valid_deps);
            }

            plan.sub_tasks.push(sub_task);
        }

        if plan.sub_tasks.is_empty() {
            tracing::warn!("LLM decompose produced empty plan, falling back to rule-based");
            return self.fallback.decompose(mission, strategy);
        }

        plan.max_parallel = match strategy {
            OrchestrationStrategy::FanOut => plan.sub_tasks.len() as u32,
            _ => 2.min(plan.sub_tasks.len() as u32),
        };

        tracing::info!(
            sub_tasks = plan.sub_tasks.len(),
            strategy = strategy.as_str(),
            "LLM-driven decomposition complete"
        );

        Ok(plan)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Extract the first JSON object or array from a text blob.
/// Handles markdown code fences, leading/trailing text, etc.
fn extract_json(text: &str) -> String {
    let text = text.trim();

    // Remove markdown code fences
    let text = if text.starts_with("```") {
        text.lines().skip(1).filter(|l| !l.trim().starts_with("```")).collect::<Vec<_>>().join("\n")
    } else {
        text.to_string()
    };

    // Find the first `{` and last `}`
    let text = text.trim();
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        text[start..=end].to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_review() {
        let decomposer = RuleBasedDecomposer::new();
        let plan =
            decomposer.decompose("Review the API design", OrchestrationStrategy::Ordered).unwrap();
        assert_eq!(plan.sub_tasks.len(), 3);
        assert!(plan.sub_tasks.iter().any(|t| t.id == "review"));
    }

    #[test]
    fn test_rule_based_refactor() {
        let decomposer = RuleBasedDecomposer::new();
        let plan = decomposer
            .decompose("Refactor database layer", OrchestrationStrategy::Ordered)
            .unwrap();
        assert_eq!(plan.sub_tasks.len(), 4);
        assert!(plan.sub_tasks.iter().any(|t| t.id == "verify"));
    }

    #[test]
    fn test_rule_based_design() {
        let decomposer = RuleBasedDecomposer::new();
        let plan =
            decomposer.decompose("Design new architecture", OrchestrationStrategy::Debate).unwrap();
        assert_eq!(plan.sub_tasks.len(), 3);
        assert!(plan.sub_tasks.iter().any(|t| t.id == "design"));
    }

    #[test]
    fn test_rule_based_default() {
        let decomposer = RuleBasedDecomposer::new();
        let plan =
            decomposer.decompose("Fix the login bug", OrchestrationStrategy::Ordered).unwrap();
        assert_eq!(plan.sub_tasks.len(), 3);
    }

    #[test]
    fn test_extract_json_from_code_fence() {
        let input = r#"```json
{"sub_tasks": [{"id": "a", "name": "A", "description": "desc", "role": "Developer", "dependencies": []}]}
```"#;
        let extracted = extract_json(input);
        assert!(extracted.starts_with('{'));
        assert!(extracted.ends_with('}'));
        assert!(extracted.contains("sub_tasks"));
    }

    #[test]
    fn test_extract_json_raw() {
        let input = r#"{"sub_tasks": [{"id": "a", "name": "A", "description": "desc", "role": "Developer", "dependencies": []}]}"#;
        let extracted = extract_json(input);
        assert_eq!(extracted, input);
    }
}
