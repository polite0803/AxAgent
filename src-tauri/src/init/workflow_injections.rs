// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流进化 wiring 层注入实现(优化 4-b / 4-c)。
//!
//! 在 wiring 层(主 crate `axagent`)创建两个 trait 实现,启动时通过
//! `set_llm_provider` / `set_sandbox` 注入到 `WorkflowEvolverImpl`:
//!
//! - [`ProviderWorkflowLlmMutator`][]:委托 `ProviderLlmBridge` 调用 LLM,
//!   把 `WorkflowGenome` 序列化为 prompt,解析 LLM 返回的 JSON 重建基因组。
//!   LLM 调用失败 / 解析失败时返回原 genome(保守策略,不破坏模板)。
//!
//! - [`StructuralWorkflowSandbox`][]:不实际执行工作流(避免副作用),
//!   仅做静态结构校验:节点非空、edges 引用有效、variables 数量合理。
//!   全部通过 → passed;否则 → 列出错误。比 evolver 内置的占位逻辑更严格。

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use axagent_agent::ProviderLlmBridge;
use axagent_harness::workflow_evolution::{
    SandboxValidationResult, WorkflowGenome, WorkflowLlmMutator, WorkflowSandbox,
};
use axagent_harness::workflow_reflection::WorkflowPattern;

/// LLM 调用超时(秒)。变异是后台任务,但过长会占用 spawn 资源。
const LLM_TIMEOUT_SECS: u64 = 60;

/// 基于 `ProviderLlmBridge` 的工作流 LLM 变异器实现。
///
/// 设计原则:
/// - **保守策略**:LLM 调用 / JSON 解析失败时返回原 genome,不破坏模板
/// - **JSON-only prompt**:明确要求 LLM 返回纯 JSON(无 markdown 包裹),
///   同时用手动提取兜底,兼容模型偶发的不规范输出
/// - **不修改 nodes 类型**:仅修改 config / 重排 edges,避免破坏类型契约
pub struct ProviderWorkflowLlmMutator {
    bridge: Arc<ProviderLlmBridge>,
}

impl ProviderWorkflowLlmMutator {
    pub fn new(bridge: ProviderLlmBridge) -> Self {
        Self { bridge: Arc::new(bridge) }
    }

    /// 构造变异 prompt:把 genome + 失败模式 + 成功模式序列化为 JSON。
    fn build_mutation_prompt(
        genome: &WorkflowGenome,
        failure_evidence: &[WorkflowPattern],
        success_evidence: &[WorkflowPattern],
    ) -> String {
        let nodes_json =
            serde_json::to_string_pretty(&genome.nodes).unwrap_or_else(|_| "[]".into());
        let edges_json =
            serde_json::to_string_pretty(&genome.edges).unwrap_or_else(|_| "[]".into());
        let failures_json =
            serde_json::to_string_pretty(failure_evidence).unwrap_or_else(|_| "[]".into());
        let successes_json =
            serde_json::to_string_pretty(success_evidence).unwrap_or_else(|_| "[]".into());

        format!(
            "You are a workflow evolution expert. Given the current workflow genome and evidence,\n\
             suggest improvements by modifying node configs and rewiring edges.\n\
             DO NOT change node types or remove nodes. Only patch configs and rewire edges.\n\n\
             Current nodes:\n{nodes_json}\n\n\
             Current edges:\n{edges_json}\n\n\
             Failure evidence:\n{failures_json}\n\n\
             Success evidence:\n{successes_json}\n\n\
             Respond with ONLY a JSON object (no markdown, no explanation):\n\
             {{\"nodes\": [...], \"edges\": [...]}}"
        )
    }

    /// 解析 LLM 返回的 JSON,提取 nodes/edges 数组并重建 genome。
    /// 失败时返回 `None`,调用方降级到原 genome。
    fn parse_mutation_response(text: &str, original: &WorkflowGenome) -> Option<WorkflowGenome> {
        // 优先直接解析;失败则手动提取首个 {...} 子串
        let json_str = if serde_json::from_str::<serde_json::Value>(text).is_ok() {
            text.to_string()
        } else {
            extract_first_json_object(text)?
        };

        let v: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        let nodes = v.get("nodes").and_then(|n| n.as_array())?;
        let edges = v.get("edges").and_then(|n| n.as_array())?;

        let new_nodes = nodes
            .iter()
            .filter_map(|n| {
                serde_json::from_value::<axagent_harness::workflow_types::WorkflowNode>(n.clone())
                    .ok()
            })
            .collect::<Vec<_>>();
        let new_edges = edges
            .iter()
            .filter_map(|e| {
                serde_json::from_value::<axagent_harness::workflow_types::WorkflowEdge>(e.clone())
                    .ok()
            })
            .collect::<Vec<_>>();

        if new_nodes.is_empty() {
            // 不允许返回空 nodes(可能 LLM 误判)
            return None;
        }

        Some(WorkflowGenome {
            template_id: original.template_id.clone(),
            name: original.name.clone(),
            nodes: new_nodes,
            edges: new_edges,
            variables: original.variables.clone(),
            fitness: original.fitness,
            generation: original.generation.saturating_add(1),
        })
    }
}

/// 从文本中提取首个 `{...}` JSON 对象子串(基于括号配对,非正则)。
///
/// 失败返回 `None`。用于兜底解析 LLM 偶发包裹 markdown / 额外文本的情况。
fn extract_first_json_object(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    // 从 start 开始配对计数,找到匹配的 `}`
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        match b {
            b'\\' if in_string => escape = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=i].to_string());
                }
            },
            _ => {},
        }
    }
    None
}

/// 从文本中提取首个浮点数(0.0-1.0)。手动扫描,无正则依赖。
fn extract_first_float(text: &str) -> Option<f32> {
    let mut start = None;
    let mut end = 0;
    for (i, c) in text.chars().enumerate() {
        let is_digit = c.is_ascii_digit();
        let is_dot = c == '.';
        let is_sign = (c == '-' || c == '+') && start.is_none();
        if (is_digit || is_dot || is_sign) && start.is_none() {
            start = Some(i);
            end = i + 1;
        } else if is_digit || is_dot {
            if start.is_some() {
                end = i + 1;
            }
        } else if start.is_some() {
            break;
        }
    }
    let s = start?;
    text[s..end].parse::<f32>().ok()
}

#[async_trait]
impl WorkflowLlmMutator for ProviderWorkflowLlmMutator {
    async fn generate_mutation(
        &self,
        genome: &WorkflowGenome,
        failure_evidence: &[WorkflowPattern],
        success_evidence: &[WorkflowPattern],
    ) -> Result<WorkflowGenome, String> {
        let prompt = Self::build_mutation_prompt(genome, failure_evidence, success_evidence);
        let system = "You are a workflow evolution expert. Respond with ONLY valid JSON.";

        // 带超时的 LLM 调用,避免变异任务长时间占用 spawn
        let bridge = self.bridge.clone();
        let call_fut = bridge.call_llm(system, &prompt);
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(LLM_TIMEOUT_SECS), call_fut).await;

        let text = match result {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                warn!("[Evolver] LLM mutation call failed: {e}");
                return Ok(genome.clone());
            },
            Err(_) => {
                warn!("[Evolver] LLM mutation timed out after {LLM_TIMEOUT_SECS}s");
                return Ok(genome.clone());
            },
        };

        match Self::parse_mutation_response(&text, genome) {
            Some(new_genome) => Ok(new_genome),
            None => {
                warn!(
                    "[Evolver] Failed to parse LLM mutation response (len={}), falling back to original genome",
                    text.len()
                );
                Ok(genome.clone())
            },
        }
    }

    async fn evaluate_quality(
        &self,
        genome: &WorkflowGenome,
        context: &str,
    ) -> Result<f32, String> {
        let nodes_summary =
            genome.nodes.iter().map(|n| n.base_id().to_string()).collect::<Vec<_>>().join(", ");
        let prompt = format!(
            "Evaluate the quality of this workflow on a scale from 0.0 to 1.0.\n\n\
             Workflow name: {}\n\
             Nodes: {nodes_summary}\n\
             Context: {context}\n\n\
             Respond with ONLY a number between 0.0 and 1.0.",
            genome.name
        );

        let bridge = self.bridge.clone();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(LLM_TIMEOUT_SECS),
            bridge.call_llm("You are a workflow quality evaluator.", &prompt),
        )
        .await;

        let text = match result {
            Ok(Ok(text)) => text.trim().to_string(),
            Ok(Err(e)) => {
                warn!("[Evolver] LLM quality eval failed: {e}");
                return Ok(0.5);
            },
            Err(_) => {
                warn!("[Evolver] LLM quality eval timed out");
                return Ok(0.5);
            },
        };

        // 解析首个浮点数,clamp 到 [0.0, 1.0]
        let score = extract_first_float(&text).unwrap_or(0.5).clamp(0.0, 1.0);
        Ok(score)
    }
}

/// 结构校验沙箱:不执行工作流,仅做静态结构检查。
///
/// 比内置的"占位通过"更严格,能在进化阶段捕获明显的结构错误:
/// - nodes 非空
/// - edges 的 source/target 都引用了存在的节点
/// - variables 数量合理(<= 1000,避免 LLM 误生成海量变量)
///
/// 全部通过 → `passed=true, success_rate=1.0`;否则 → `passed=false`,
/// 错误明细记入 `execution_errors`。
pub struct StructuralWorkflowSandbox;

impl StructuralWorkflowSandbox {
    pub fn new() -> Self {
        Self
    }

    /// 执行结构校验,返回错误列表(空 = 通过)。
    fn validate(genome: &WorkflowGenome) -> Vec<String> {
        // 复用 harness 提供的基础校验(node id 不重复 / edge 引用有效 / variable name 不重复)
        let mut errors = axagent_harness::workflow_evolution::validate_genome_basic(genome);

        // 额外业务约束:nodes 非空 + variables 数量合理
        if genome.nodes.is_empty() {
            errors.push("genome has no nodes".to_string());
        }
        if genome.variables.len() > 1000 {
            errors.push(format!(
                "variables count {} exceeds 1000 (likely malformed)",
                genome.variables.len()
            ));
        }

        errors
    }
}

impl Default for StructuralWorkflowSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowSandbox for StructuralWorkflowSandbox {
    async fn execute(
        &self,
        genome: &WorkflowGenome,
        _test_input: &serde_json::Value,
    ) -> Result<SandboxValidationResult, String> {
        let errors = Self::validate(genome);
        if errors.is_empty() {
            Ok(SandboxValidationResult {
                passed: true,
                success_rate: 1.0,
                execution_errors: Vec::new(),
                avg_execution_time_ms: 0,
            })
        } else {
            Ok(SandboxValidationResult {
                passed: false,
                success_rate: 0.0,
                execution_errors: errors,
                avg_execution_time_ms: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_json_object_simple() {
        let text = r#"{"a": 1, "b": 2}"#;
        let s = extract_first_json_object(text).unwrap();
        assert_eq!(s, r#"{"a": 1, "b": 2}"#);
    }

    #[test]
    fn test_extract_first_json_object_with_prefix() {
        let text = "Here is the JSON:\n```json\n{\"nodes\": [], \"edges\": []}\n```";
        let s = extract_first_json_object(text).unwrap();
        assert_eq!(s, r#"{"nodes": [], "edges": []}"#);
    }

    #[test]
    fn test_extract_first_json_object_with_nested_braces_in_string() {
        // 字符串中的 `{` 不应影响配对
        let text = r#"{"desc": "a {b} c", "x": 1}"#;
        let s = extract_first_json_object(text).unwrap();
        assert_eq!(s, r#"{"desc": "a {b} c", "x": 1}"#);
    }

    #[test]
    fn test_extract_first_json_object_no_object() {
        assert!(extract_first_json_object("no json here").is_none());
    }

    #[test]
    fn test_extract_first_float() {
        assert_eq!(extract_first_float("0.75"), Some(0.75));
        assert_eq!(extract_first_float("score: 0.9 ok"), Some(0.9));
        assert_eq!(extract_first_float("0"), Some(0.0));
        assert_eq!(extract_first_float("no number"), None);
    }

    #[test]
    fn test_structural_sandbox_valid_genome() {
        // 用 JSON 反序列化构造 WorkflowGenome(避免手工构造 WorkflowNodeBase 全字段)
        // WorkflowNode 用 #[serde(tag="type", rename_all="camelCase")] + #[serde(flatten)] base
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "delay",
                 "id": "n1", "title": "delay", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [
                {"id": "e1", "source": "n1", "source_handle": null,
                 "target": "n1", "target_handle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = StructuralWorkflowSandbox::new();
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
        assert!(result.passed, "expected pass, got errors: {:?}", result.execution_errors);
        assert_eq!(result.success_rate, 1.0);
    }

    #[test]
    fn test_structural_sandbox_dangling_edge() {
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "delay",
                 "id": "n1", "title": "delay", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [
                {"id": "e1", "source": "n1", "source_handle": null,
                 "target": "missing", "target_handle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = StructuralWorkflowSandbox::new();
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
        assert!(!result.passed);
        assert!(result.execution_errors.iter().any(|e| e.contains("missing")));
    }

    #[test]
    fn test_structural_sandbox_empty_nodes() {
        let genome = WorkflowGenome {
            template_id: "t1".into(),
            name: "empty".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: Vec::new(),
            fitness: 0.5,
            generation: 0,
        };
        let sandbox = StructuralWorkflowSandbox::new();
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
        assert!(!result.passed);
        assert!(result.execution_errors.iter().any(|e| e.contains("no nodes")));
    }
}
