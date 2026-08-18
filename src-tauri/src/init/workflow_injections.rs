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

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::warn;

use axagent_agent::ProviderLlmBridge;
use axagent_harness::workflow_evolution::{
    EvolutionArtifactValidator, ExecutionFeedbackSink, SandboxValidationResult, ToolExecutionStats,
    WorkflowGenome, WorkflowLlmMutator, WorkflowSandbox,
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
        let variables_json =
            serde_json::to_string_pretty(&genome.variables).unwrap_or_else(|_| "[]".into());
        let failures_json =
            serde_json::to_string_pretty(failure_evidence).unwrap_or_else(|_| "[]".into());
        let successes_json =
            serde_json::to_string_pretty(success_evidence).unwrap_or_else(|_| "[]".into());

        // 方案 4A:针对性 prompt 增强 — 显式要求针对频繁失败节点调整
        // retry / timeout / continue_on_fail 字段,而非仅做拓扑重连。
        format!(
            "You are a workflow evolution expert. Given the current workflow genome and evidence,\n\
             suggest improvements by:\n\
             1. Patching node configs (retry / timeout / continue_on_fail) for nodes that appear\n\
                in failure_evidence. Increase `retry.max_retries` (up to 5) and enable\n\
                `retry.enabled` for high-failure-rate nodes. Increase `timeout` (up to 60 seconds)\n\
                for slow nodes. Set `continue_on_fail=true` only when the workflow should\n\
                tolerate the node's failure.\n\
             2. Rewiring edges only if a node is clearly misplaced.\n\
             DO NOT change node types, remove nodes, or rename node IDs.\n\
             DO NOT introduce duplicate node IDs or dangling edge endpoints.\n\n\
             Current nodes:\n{nodes_json}\n\n\
             Current edges:\n{edges_json}\n\n\
             Current variables:\n{variables_json}\n\n\
             Failure evidence:\n{failures_json}\n\n\
             Success evidence:\n{successes_json}\n\n\
             Respond with ONLY a JSON object (no markdown, no explanation):\n\
             {{\"nodes\": [...], \"edges\": [...], \"variables\": [...], \"changed_node_ids\": [...]}}\n\
             The \"nodes\" array must include ALL nodes (with patched configs), not just changed ones.\n\
             The \"changed_node_ids\" array lists ONLY the node IDs whose configs you modified —\n\
             unchanged nodes will be preserved from the original genome."
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

        // 方案 4A:variables 由 LLM 显式返回(缺失时保留原值)
        let new_variables = v
            .get("variables")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_else(|| original.variables.clone());

        Some(WorkflowGenome {
            template_id: original.template_id.clone(),
            name: original.name.clone(),
            nodes: new_nodes,
            edges: new_edges,
            variables: new_variables,
            fitness: original.fitness,
            generation: original.generation.saturating_add(1),
            // 方案 1B:LLM 显式声明变更的 node_id 列表
            changed_node_ids: v
                .get("changed_node_ids")
                .and_then(|n| n.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
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

/// 从文本中提取所有 `{{var_name}}` 形式的变量引用。
///
/// 手动扫描(非正则),匹配规则:
/// - 起始标记 `{{`
/// - 可选空白
/// - 标识符(首字符 `[a-zA-Z_]`,后续 `[a-zA-Z0-9_]*`)
/// - 可选空白
/// - 结束标记 `}}`
///
/// 不匹配时跳过 `{{`,从下一个位置继续扫描(避免死循环)。
fn extract_var_refs(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut refs = Vec::new();
    let mut i = 0;
    while let Some(start) = find_subslice(bytes, b"{{", i) {
        let mut pos = start + 2;
        // 跳过空白
        pos = skip_whitespace(bytes, pos);
        // 收集标识符
        let id_start = pos;
        if pos < bytes.len() && (bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_') {
            pos += 1;
            while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                pos += 1;
            }
            let id_end = pos;
            // 跳过空白
            pos = skip_whitespace(bytes, pos);
            // 匹配 `}}`
            if pos + 1 < bytes.len() && bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
                if let Ok(name) = std::str::from_utf8(&bytes[id_start..id_end]) {
                    refs.push(name.to_string());
                }
                i = pos + 2;
                continue;
            }
        }
        // 不匹配,从下一个字符继续扫描(避免在当前位置死循环)
        i = start + 1;
    }
    refs
}

/// 从 `from` 位置开始查找 `needle` 在 `haystack` 中的首个出现位置(字节下标)。
fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() || needle.is_empty() {
        return None;
    }
    haystack[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

/// 从 `pos` 开始跳过 ASCII 空白字符(空格/制表/换行/回车),返回新的位置。
fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    pos
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

// ── 方案 3A:wiring 层基因组加载器 ──

/// 从 DB 加载工作流模板并构造 `WorkflowGenome` 的 wiring 实现。
///
/// 委托 `WorkflowTemplateRepository::get_workflow_template`(harness trait),
/// 将 `WorkflowTemplateData` 的 `nodes` / `edges` / `variables`(JSON Value)
/// 反序列化出具体类型,组合成 `WorkflowGenome`。
///
/// trajectory 层通过 `WorkflowGenomeLoader` trait 拿到 genome,不直接依赖 dao。
pub struct DaoWorkflowGenomeLoader {
    repo: Arc<dyn axagent_harness::repositories::WorkflowTemplateRepository>,
}

impl DaoWorkflowGenomeLoader {
    pub fn new(repo: Arc<dyn axagent_harness::repositories::WorkflowTemplateRepository>) -> Self {
        Self { repo }
    }
}

impl axagent_harness::workflow_evolution::WorkflowGenomeLoader for DaoWorkflowGenomeLoader {
    fn load_genome(
        &self,
        template_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<WorkflowGenome>> + Send>> {
        let repo = self.repo.clone();
        let id = template_id.to_string();
        Box::pin(async move {
            let data = match repo.get_workflow_template(&id).await {
                Ok(Some(d)) => d,
                _ => return None,
            };

            // `WorkflowTemplateData.nodes / edges` 是 JSON 字符串,需先 parse 成 Value
            let nodes_value: serde_json::Value =
                serde_json::from_str(&data.nodes).unwrap_or(serde_json::Value::Array(vec![]));
            let edges_value: serde_json::Value =
                serde_json::from_str(&data.edges).unwrap_or(serde_json::Value::Array(vec![]));
            let variables_value: serde_json::Value = data
                .variables
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Array(vec![]));

            // 反序列化 nodes
            let nodes: Vec<axagent_harness::workflow_types::WorkflowNode> = nodes_value
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|n| {
                            serde_json::from_value::<axagent_harness::workflow_types::WorkflowNode>(
                                n.clone(),
                            )
                            .ok()
                        })
                        .collect()
                })
                .unwrap_or_default();

            // 反序列化 edges
            let edges: Vec<axagent_harness::workflow_types::WorkflowEdge> = edges_value
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| {
                            serde_json::from_value::<axagent_harness::workflow_types::WorkflowEdge>(
                                e.clone(),
                            )
                            .ok()
                        })
                        .collect()
                })
                .unwrap_or_default();

            // variables 保留为 JSON Vec(WorkflowGenome.variables 即 Vec<Value>)
            let variables: Vec<serde_json::Value> =
                variables_value.as_array().map(|arr| arr.to_vec()).unwrap_or_default();

            if nodes.is_empty() {
                tracing::warn!("[Evolver] template {id} has no nodes, genome loader returns None");
                return None;
            }

            Some(WorkflowGenome {
                template_id: data.id,
                name: data.name,
                nodes,
                edges,
                variables,
                fitness: 0.5,
                generation: 0,
                changed_node_ids: Vec::new(),
            })
        })
    }
}

// ── 方案 2A(简化版):拓扑可达性 + 变量引用校验沙箱 ──

/// 拓扑可达性 + 变量引用校验沙箱(不实际执行工作流)。
///
/// 比 `StructuralWorkflowSandbox` 更强:在结构校验基础上增加两项语义校验:
/// - 拓扑可达性:从 Trigger 节点出发,通过 edges BFS 遍历,所有节点必须可达
///   (捕获孤立节点 — LLM 变异可能误删边或新增孤立节点)
/// - 变量引用校验:nodes 的 config 字段中 `{{var_name}}` 形式引用的变量,
///   必须在 `genome.variables` 中定义(捕获悬空变量引用)
///
/// 不实际调用 WorkEngine,避免循环依赖(WorkEngine 持有 Evolver,Evolver 不应反向调用 WorkEngine)。
/// 仍属静态校验范畴,但不局限于字段级结构。
pub struct ReachabilityWorkflowSandbox;

impl ReachabilityWorkflowSandbox {
    pub fn new() -> Self {
        Self
    }

    /// 从 Trigger 节点出发做 BFS,返回不可达的节点 ID 列表。
    ///
    /// 无 Trigger 节点时,从第一个节点出发(降级策略)。
    fn find_unreachable_nodes(genome: &WorkflowGenome) -> Vec<String> {
        use std::collections::{HashMap, HashSet, VecDeque};
        if genome.nodes.is_empty() {
            return Vec::new();
        }

        // 构建邻接表:source -> [target]
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &genome.edges {
            adj.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
        }

        // 找入口节点:优先 Trigger,否则用第一个节点
        let start: &str = genome
            .nodes
            .iter()
            .find(|n| matches!(n, axagent_harness::workflow_types::WorkflowNode::Trigger(_)))
            .map(|n| n.base_id())
            .or_else(|| genome.nodes.first().map(|n| n.base_id()))
            .unwrap_or("");

        if start.is_empty() {
            return Vec::new();
        }

        // BFS
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        visited.insert(start);
        queue.push_back(start);
        while let Some(node) = queue.pop_front() {
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if visited.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }

        // 找不可达节点
        genome
            .nodes
            .iter()
            .map(|n| n.base_id())
            .filter(|id| !visited.contains(*id))
            .map(|id| id.to_string())
            .collect()
    }

    /// 扫描 nodes 的 config(序列化为 JSON),提取 `{{var_name}}` 引用,
    /// 检查是否在 `genome.variables` 中定义。
    ///
    /// 返回未定义的变量引用列表(去重)。手动扫描实现,不引入 `regex` crate 依赖。
    fn find_undefined_variable_refs(genome: &WorkflowGenome) -> Vec<String> {
        // 收集已定义变量名
        let defined: std::collections::HashSet<String> = genome
            .variables
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect();

        // 扫描每个 node 的 config JSON,手动提取 {{var}} 引用
        let mut undefined: std::collections::HashSet<String> = std::collections::HashSet::new();
        for node in &genome.nodes {
            let config_json = serde_json::to_string(node).unwrap_or_default();
            for name in extract_var_refs(&config_json) {
                if !defined.contains(&name) {
                    undefined.insert(name);
                }
            }
        }

        undefined.into_iter().collect()
    }

    /// 执行校验,返回错误列表(空 = 通过)。
    fn validate(genome: &WorkflowGenome) -> Vec<String> {
        // 先做结构校验(复用 StructuralWorkflowSandbox 逻辑)
        let mut errors = StructuralWorkflowSandbox::validate(genome);

        // 拓扑可达性
        let unreachable = Self::find_unreachable_nodes(genome);
        if !unreachable.is_empty() {
            errors.push(format!("unreachable nodes: {}", unreachable.join(", ")));
        }

        // 变量引用校验
        let undefined_vars = Self::find_undefined_variable_refs(genome);
        if !undefined_vars.is_empty() {
            errors.push(format!("undefined variable references: {}", undefined_vars.join(", ")));
        }

        errors
    }
}

#[async_trait]
impl WorkflowSandbox for ReachabilityWorkflowSandbox {
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

impl Default for ReachabilityWorkflowSandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// 结构校验沙箱:不实际执行工作流,仅做静态结构检查。
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
    pub(crate) fn validate(genome: &WorkflowGenome) -> Vec<String> {
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

/// 沙箱单次执行硬超时(秒)。
///
/// 即便静态校验 + 模拟执行都很快,也用 `tokio::time::timeout` 兜底,
/// 防止意外死循环或大量节点导致沙箱卡住进化主流程。
const SANDBOX_HARD_TIMEOUT_SECS: u64 = 5;

/// 节点 `timeout` 字段上限(秒)。超过视为 LLM 误生成。
const NODE_MAX_TIMEOUT_SECS: u64 = 300;

/// 节点 `retry.max_retries` 上限。超过视为 LLM 误生成。
const NODE_MAX_RETRIES: u32 = 10;

/// 工作流累积模拟执行时间上限(秒)。
///
/// 所有节点 `timeout` 之和超过此值视为执行链过长(LLM 可能误生成超长链)。
/// 不阻止进化,但会降低 `success_rate` 并记入 `execution_errors`。
const WORKFLOW_MAX_TOTAL_TIMEOUT_SECS: u64 = 3600;

/// 带有限试运行的沙箱(P2-8)。
///
/// 在 [`ReachabilityWorkflowSandbox`] 静态校验之上,新增"轻量模拟执行"层:
/// - **节点级配置合理性**:每个节点的 `timeout` ≤ 300s,`retry.max_retries` ≤ 10,
///   超过视为 LLM 误生成,降级 `success_rate` 并记入错误
/// - **累积执行时间上限**:所有节点 `timeout` 之和 ≤ 3600s,避免超长执行链
/// - **环检测**:edges 形成环且无 Loop 节点时,降级 `success_rate` 并警告
/// - **硬超时保护**:用 `tokio::time::timeout` 包装整体执行,5 秒内未完成视为异常
///
/// 仍属"有限试运行"范畴(不实际调用 LLM / 工具,避免副作用与循环依赖),
/// 但比纯静态校验能捕获更多运行时风险配置。
pub struct DryRunWorkflowSandbox;

impl DryRunWorkflowSandbox {
    pub fn new() -> Self {
        Self
    }

    /// 模拟执行:对每个节点检查配置合理性,返回 (错误列表, 累积模拟耗时_ms)。
    fn simulate_execution(genome: &WorkflowGenome) -> (Vec<String>, u64) {
        let mut errors = Vec::new();
        let mut total_timeout_secs: u64 = 0;

        for node in &genome.nodes {
            let base = node.base();
            let node_id = base.id.as_str();

            // 1. timeout 合理性
            if let Some(timeout_secs) = base.timeout {
                if timeout_secs > NODE_MAX_TIMEOUT_SECS {
                    errors.push(format!(
                        "node '{node_id}' timeout {timeout_secs}s exceeds {NODE_MAX_TIMEOUT_SECS}s"
                    ));
                }
                total_timeout_secs = total_timeout_secs.saturating_add(timeout_secs);
            }

            // 2. retry 合理性
            if base.retry.enabled && base.retry.max_retries > NODE_MAX_RETRIES {
                errors.push(format!(
                    "node '{node_id}' retry.max_retries {} exceeds {NODE_MAX_RETRIES}",
                    base.retry.max_retries
                ));
            }
        }

        // 3. 累积执行时间上限
        if total_timeout_secs > WORKFLOW_MAX_TOTAL_TIMEOUT_SECS {
            errors.push(format!(
                "cumulative timeout {total_timeout_secs}s exceeds {WORKFLOW_MAX_TOTAL_TIMEOUT_SECS}s"
            ));
        }

        // 4. 环检测:edges 形成环且无 Loop 节点时警告
        if Self::has_cycle_without_loop_node(genome) {
            errors.push(
                "edges form a cycle but no Loop node present (potential infinite loop)".to_string(),
            );
        }

        // 模拟耗时 = 累积 timeout(ms),仅用于报告(不影响 passed 判定)
        let simulated_ms = total_timeout_secs.saturating_mul(1000);
        (errors, simulated_ms)
    }

    /// 简单环检测:DFS 三色标记法。
    ///
    /// 若图中存在环且 nodes 中无 `WorkflowNode::Loop` 变体,返回 true。
    /// 有 Loop 节点时环是预期结构,跳过检测。
    fn has_cycle_without_loop_node(genome: &WorkflowGenome) -> bool {
        use std::collections::HashMap;

        // 若包含 Loop 节点,环是合法结构,直接返回 false
        if genome
            .nodes
            .iter()
            .any(|n| matches!(n, axagent_harness::workflow_types::WorkflowNode::Loop(_)))
        {
            return false;
        }

        // 构建邻接表
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &genome.edges {
            adj.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
        }

        // 三色:0=未访问,1=访问中(在递归栈),2=已完成
        let mut color: HashMap<&str, u8> = HashMap::new();
        for node in &genome.nodes {
            let id = node.base_id();
            color.entry(id).or_insert(0);
        }

        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            color: &mut HashMap<&'a str, u8>,
        ) -> bool {
            match color.get(node).copied().unwrap_or(0) {
                1 => return true,  // 找到环
                2 => return false, // 已完成,跳过
                _ => {},
            }
            color.insert(node, 1);
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if dfs(next, adj, color) {
                        return true;
                    }
                }
            }
            color.insert(node, 2);
            false
        }

        for node in &genome.nodes {
            let id = node.base_id();
            if color.get(id).copied().unwrap_or(0) == 0 && dfs(id, &adj, &mut color) {
                return true;
            }
        }

        false
    }
}

impl Default for DryRunWorkflowSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowSandbox for DryRunWorkflowSandbox {
    async fn execute(
        &self,
        genome: &WorkflowGenome,
        _test_input: &serde_json::Value,
    ) -> Result<SandboxValidationResult, String> {
        // 硬超时保护:整体执行 ≤ 5 秒,防止意外卡死
        let execution = async {
            // 1. 静态校验(复用 ReachabilityWorkflowSandbox 逻辑)
            let mut errors = ReachabilityWorkflowSandbox::validate(genome);

            // 2. 模拟执行层:节点级配置合理性 + 累积上限 + 环检测
            let (sim_errors, simulated_ms) = Self::simulate_execution(genome);
            errors.extend(sim_errors);

            if errors.is_empty() {
                SandboxValidationResult {
                    passed: true,
                    success_rate: 1.0,
                    execution_errors: Vec::new(),
                    avg_execution_time_ms: simulated_ms,
                }
            } else {
                // 多错误时按错误数比例降低 success_rate(避免硬编码)
                let err_count = errors.len() as f32;
                let success_rate = (1.0 / (1.0 + err_count)).min(0.99);
                SandboxValidationResult {
                    passed: false,
                    success_rate,
                    execution_errors: errors,
                    avg_execution_time_ms: simulated_ms,
                }
            }
        };

        match tokio::time::timeout(
            std::time::Duration::from_secs(SANDBOX_HARD_TIMEOUT_SECS),
            execution,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(_) => Ok(SandboxValidationResult {
                passed: false,
                success_rate: 0.0,
                execution_errors: vec![format!(
                    "sandbox hard timeout ({SANDBOX_HARD_TIMEOUT_SECS}s) exceeded"
                )],
                avg_execution_time_ms: SANDBOX_HARD_TIMEOUT_SECS * 1000,
            }),
        }
    }
}

// ── T4.4:计算型(Rhai)进化产物沙箱验证器 ──

/// Rhai 脚本最大长度(字符)。超过视为 LLM 误生成 / 潜在 DoS,拒绝执行。
const RHAI_SCRIPT_MAX_LEN: usize = 50_000;

/// 进化产物危险模式列表(T4.4)。
///
/// Rhai 是内存沙箱语言,本身不能直接执行系统命令 / 访问文件系统 / 发网络请求
/// (未注入对应模块)。这些模式命中说明产物含"尝试调用系统能力 / 破坏"的意图,
/// 即便 Rhai 编译也会失败,也应在第一道防线提前拦截并给出明确错误,
/// 与 `SkillSandboxExecutor::DANGEROUS_PATTERNS` 同思路。
/// 刻意保持精确(不做 `format` / `eval` / `download` 等宽泛匹配),避免误伤合法脚本。
const EVOLUTION_DANGEROUS_PATTERNS: &[&str] = &[
    // 系统命令 / 进程执行(Rhai 未注入)
    "std::process",
    "process::Command",
    "Command::new",
    // 文件系统破坏
    "std::fs",
    "fs::remove",
    "remove_dir_all",
    // 网络请求(Rhai 未注入)
    "reqwest",
    "http::Client",
    // 权限提升意图
    "setuid",
    "sudo -",
];

/// 计算型(Rhai)进化产物沙箱验证器(T4.4)。
///
/// 在 `GeneratedToolAdapter::call()` 真正执行 Rhai 脚本前调用,组合三道静态防线:
/// 1. **长度限制**:脚本超长(> `RHAI_SCRIPT_MAX_LEN`)拒绝,防超长脚本 DoS
/// 2. **自指熔断**:脚本内含 `/evolution/`、`evolution:workflow`、`self_evolution`
///    等保护关键词时拒绝,防进化产物递归调用系统能力(复用 `SelfReferenceProtection`)
/// 3. **危险模式**:命中 [`EVOLUTION_DANGEROUS_PATTERNS`] 中的危险意图片段时拒绝
///
/// 验证不通过 → 工具执行返回沙箱错误,产物不落地。wiring 层注入到进化工具执行路径。
pub struct SelfReferenceArtifactValidator {
    protected_keywords: Vec<String>,
}

impl SelfReferenceArtifactValidator {
    pub fn new() -> Self {
        // 复用认知路由的自指熔断保护关键词,保证与路由层同一套熔断语义
        let protected_keywords =
            axagent_harness::cognitive_router::SelfReferenceProtection::default()
                .protected_keywords;
        Self { protected_keywords }
    }
}

impl Default for SelfReferenceArtifactValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolutionArtifactValidator for SelfReferenceArtifactValidator {
    fn validate_code(&self, code: &str) -> Vec<String> {
        let mut violations = Vec::new();

        // 1. 长度限制
        if code.len() > RHAI_SCRIPT_MAX_LEN {
            violations.push(format!("脚本长度 {} 超过上限 {RHAI_SCRIPT_MAX_LEN} 字符", code.len()));
        }

        // 2. 自指熔断保护关键词
        for keyword in &self.protected_keywords {
            if code.contains(keyword) {
                violations.push(format!("脚本命中自指熔断保护关键词 '{keyword}'"));
            }
        }

        // 3. 危险模式
        for pattern in EVOLUTION_DANGEROUS_PATTERNS {
            if code.contains(pattern) {
                violations.push(format!("脚本命中危险模式 '{pattern}'"));
            }
        }

        violations
    }
}

// ── T5A.3:执行反馈闭环 wiring 实现 ──

/// 进化产物执行反馈接收器（T5A.3）。
///
/// 累计 `GeneratedToolAdapter::call` 上报的真实执行成败到
/// `AppState.evolution_execution_stats`（与 AppState 共享同一 Arc），
/// 作为贝叶斯决策器的「真实执行证据」（阶段四后置闭环）。
/// 与 [`ExecutionFeedbackSink`] 契约解耦：tools 层仅依赖 harness 契约，
/// 本实现位于 wiring 层，不破坏架构分层。
/// D3 持久化：持有可选数据库连接，`record` 更新内存后异步 upsert 落库，
/// 重启后由启动流程加载回内存，真实执行证据不丢失。
pub struct EvolutionFeedbackSinkImpl {
    /// 统计表（D2 会话隔离）：`conversation_id → tool_id → ToolExecutionStats`。
    /// 无会话上下文（`None`）落到 `""` 全局桶。
    stats: Arc<Mutex<HashMap<String, HashMap<String, ToolExecutionStats>>>>,
    /// 数据库连接（D3 持久化）：`Some` 时 `record` 同步更新内存并异步落库；
    /// `None` 时仅内存累计（纯测试 / 无 DB 上下文）。
    db: Option<axagent_harness::DatabaseConnection>,
}

impl EvolutionFeedbackSinkImpl {
    pub fn new(
        stats: Arc<Mutex<HashMap<String, HashMap<String, ToolExecutionStats>>>>,
        db: Option<axagent_harness::DatabaseConnection>,
    ) -> Self {
        Self { stats, db }
    }
}

impl ExecutionFeedbackSink for EvolutionFeedbackSinkImpl {
    fn record(&self, conversation_id: Option<&str>, tool_id: &str, success: bool) {
        // `record` 是同步回调（工具执行完成瞬间调用），统计表用 tokio Mutex 保护。
        // 临界区仅一次哈希表条目更新、不含 await，blocking_lock 无死锁风险
        //（同一时刻不会有其它任务持有该锁并等待本线程让出执行权）。
        let conv = conversation_id.unwrap_or("").to_string();
        let tool_id = tool_id.to_string();
        {
            let mut stats = self.stats.blocking_lock();
            let conv_stats = stats.entry(conv.clone()).or_default();
            let entry = conv_stats.entry(tool_id.clone()).or_default();
            entry.usage_count += 1;
            if success {
                entry.successes += 1;
            } else {
                entry.failures += 1;
            }
        }
        // D3 持久化：异步 upsert 到 DB（SQLite/PG 通用 UPSERT）。
        // 仅在实际执行路径调用（D4 假成功修复），落库前已释放 stats 锁，无跨 await 持锁。
        if let Some(db) = &self.db {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let db = db.clone();
                handle.spawn(async move {
                    if let Err(e) =
                        axagent_dao::repo::evolution_execution_stats::upsert_execution_feedback(
                            &db, &conv, &tool_id, success,
                        )
                        .await
                    {
                        tracing::warn!(
                            target: "evolution_feedback",
                            conversation_id = %conv,
                            tool_id = %tool_id,
                            error = %e,
                            "进化产物执行反馈落库失败"
                        );
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_json_object_simple() {
        let text = r#"{"a": 1, "b": 2}"#;
        let s = extract_first_json_object(text).expect("测试：text 应包含有效 JSON");
        assert_eq!(s, r#"{"a": 1, "b": 2}"#);
    }

    #[test]
    fn test_extract_first_json_object_with_prefix() {
        let text = "Here is the JSON:\n```json\n{\"nodes\": [], \"edges\": []}\n```";
        let s = extract_first_json_object(text).expect("测试：text 应包含有效 JSON");
        assert_eq!(s, r#"{"nodes": [], "edges": []}"#);
    }

    #[test]
    fn test_extract_first_json_object_with_nested_braces_in_string() {
        // 字符串中的 `{` 不应影响配对
        let text = r#"{"desc": "a {b} c", "x": 1}"#;
        let s = extract_first_json_object(text).expect("测试：text 应包含有效 JSON");
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
                {"id": "e1", "source": "n1", "sourceHandle": null,
                 "target": "n1", "targetHandle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = StructuralWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
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
                {"id": "e1", "source": "n1", "sourceHandle": null,
                 "target": "missing", "targetHandle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = StructuralWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
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
            changed_node_ids: Vec::new(),
        };
        let sandbox = StructuralWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
        assert!(!result.passed);
        assert!(result.execution_errors.iter().any(|e| e.contains("no nodes")));
    }

    // ── 方案 2A:extract_var_refs / ReachabilityWorkflowSandbox 单元测试 ──

    #[test]
    fn test_extract_var_refs_basic() {
        let text = r#"{"prompt": "Hello {{name}}, your score is {{score}}"}"#;
        let refs = extract_var_refs(text);
        assert!(refs.contains(&"name".to_string()));
        assert!(refs.contains(&"score".to_string()));
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_extract_var_refs_with_whitespace() {
        let text = r#"{"prompt": "{{  name  }} - {{ age }}"}"#;
        let refs = extract_var_refs(text);
        assert!(refs.contains(&"name".to_string()));
        assert!(refs.contains(&"age".to_string()));
    }

    #[test]
    fn test_extract_var_refs_no_match() {
        // 单 `{` 或 `}` 不应触发匹配,也不应死循环
        let refs = extract_var_refs("{ not a var } { ");
        assert!(refs.is_empty());
        let refs = extract_var_refs("}}} {{ ");
        assert!(refs.is_empty());
        let refs = extract_var_refs("{{ 123invalid }}");
        assert!(refs.is_empty(), "leading digit is not a valid identifier");
    }

    #[test]
    fn test_extract_var_refs_underscore_and_digits() {
        let text = "{{_user_id}} {{counter42}}";
        let refs = extract_var_refs(text);
        assert!(refs.contains(&"_user_id".to_string()));
        assert!(refs.contains(&"counter42".to_string()));
    }

    #[test]
    fn test_reachability_sandbox_single_node_self_loop() {
        // 单节点 + 自环边 → 可达,通过校验
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
                {"id": "e1", "source": "n1", "sourceHandle": null,
                 "target": "n1", "targetHandle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = ReachabilityWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
        assert!(result.passed, "expected pass, got: {:?}", result.execution_errors);
    }

    #[test]
    fn test_reachability_sandbox_isolated_node() {
        // 链: trigger -> delay;孤立节点: isolated(无入边/出边)→ 应失败
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "trigger",
                 "id": "t", "title": "trigger", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"type": "manual", "config": {}}},
                {"type": "delay",
                 "id": "d", "title": "delay", "position": {"x": 100, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}},
                {"type": "delay",
                 "id": "isolated", "title": "isolated", "position": {"x": 200, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [
                {"id": "e1", "source": "t", "sourceHandle": null,
                 "target": "d", "targetHandle": null,
                 "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = ReachabilityWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
        assert!(!result.passed, "expected fail (isolated node), got pass");
        assert!(
            result.execution_errors.iter().any(|e| e.contains("unreachable")),
            "expected unreachable error, got: {:?}",
            result.execution_errors
        );
    }

    #[test]
    fn test_reachability_sandbox_undefined_variable_ref() {
        // 引用了 {{missing_var}},但 variables 中只定义了 defined_var → 应失败
        // TriggerConfig.config 是 serde_json::Value,可以放任意 JSON
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "trigger",
                 "id": "t", "title": "trigger", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"type": "manual", "config": {"prompt": "Hello {{missing_var}}"}}}
            ],
            "edges": [],
            "variables": [{"name": "defined_var", "value": "ok"}],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = ReachabilityWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
        assert!(!result.passed, "expected fail (undefined var), got pass");
        assert!(
            result.execution_errors.iter().any(|e| e.contains("undefined variable")),
            "expected undefined var error, got: {:?}",
            result.execution_errors
        );
    }

    #[test]
    fn test_reachability_sandbox_defined_variable_ref_passes() {
        // 引用了 {{defined_var}},variables 中已定义 → 不应因变量引用失败
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "trigger",
                 "id": "t", "title": "trigger", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"type": "manual", "config": {"prompt": "Hello {{defined_var}}"}}}
            ],
            "edges": [],
            "variables": [{"name": "defined_var", "value": "world"}],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = ReachabilityWorkflowSandbox::new();
        let result = futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({})))
            .expect("测试：沙箱执行应成功");
        // 单节点 + 无边 → 可达(自身);变量引用已定义 → 不应失败
        assert!(result.passed, "expected pass, got: {:?}", result.execution_errors);
    }

    /// 辅助:构造带 timeout 的单节点 genome
    fn make_genome_with_timeout(timeout_secs: u64) -> WorkflowGenome {
        let json = format!(
            r#"{{
                "template_id": "t1",
                "name": "test",
                "nodes": [
                    {{"type": "delay",
                     "id": "n1", "title": "delay", "position": {{"x": 0, "y": 0}},
                     "retry": {{"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000}},
                     "timeout": {timeout_secs},
                     "enabled": true,
                     "config": {{"delay_type": "seconds", "seconds": 1, "until": null}}}}
                ],
                "edges": [],
                "variables": [],
                "fitness": 0.5,
                "generation": 0
            }}"#
        );
        serde_json::from_str(&json).expect("deserialize genome")
    }

    #[tokio::test]
    async fn test_dry_run_sandbox_passes_with_reasonable_timeout() {
        // timeout=10s ≤ 300s,无错误 → passed
        let genome = make_genome_with_timeout(10);
        let sandbox = DryRunWorkflowSandbox::new();
        let result =
            sandbox.execute(&genome, &serde_json::json!({})).await.expect("测试：异步操作应成功");
        assert!(result.passed, "expected pass, got: {:?}", result.execution_errors);
        assert_eq!(result.success_rate, 1.0);
        // 模拟耗时 = 10s = 10000ms
        assert_eq!(result.avg_execution_time_ms, 10_000);
    }

    #[tokio::test]
    async fn test_dry_run_sandbox_fails_with_excessive_timeout() {
        // timeout=400s > 300s 上限 → 失败,错误信息应包含 "exceeds 300s"
        let genome = make_genome_with_timeout(400);
        let sandbox = DryRunWorkflowSandbox::new();
        let result =
            sandbox.execute(&genome, &serde_json::json!({})).await.expect("测试：异步操作应成功");
        assert!(!result.passed, "expected fail");
        assert!(
            result.execution_errors.iter().any(|e| e.contains("exceeds 300s")),
            "expected timeout exceeds error, got: {:?}",
            result.execution_errors
        );
        // success_rate 应被降级(0 < rate < 1)
        assert!(result.success_rate > 0.0 && result.success_rate < 1.0);
    }

    #[tokio::test]
    async fn test_dry_run_sandbox_fails_with_excessive_retries() {
        // max_retries=20 > 10 上限 → 失败
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "delay",
                 "id": "n1", "title": "delay", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": true, "max_retries": 20, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = DryRunWorkflowSandbox::new();
        let result =
            sandbox.execute(&genome, &serde_json::json!({})).await.expect("测试：异步操作应成功");
        assert!(!result.passed, "expected fail");
        assert!(
            result.execution_errors.iter().any(|e| e.contains("retry.max_retries 20 exceeds 10")),
            "expected retry exceeds error, got: {:?}",
            result.execution_errors
        );
    }

    #[tokio::test]
    async fn test_dry_run_sandbox_detects_cycle_without_loop_node() {
        // 两个 delay 节点互相连接形成环,无 Loop 节点 → 应报 cycle 错误
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "delay",
                 "id": "n1", "title": "delay1", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}},
                {"type": "delay",
                 "id": "n2", "title": "delay2", "position": {"x": 100, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [
                {"id": "e1", "source": "n1", "sourceHandle": null, "target": "n2", "targetHandle": null, "edge_type": "direct", "label": null},
                {"id": "e2", "source": "n2", "sourceHandle": null, "target": "n1", "targetHandle": null, "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = DryRunWorkflowSandbox::new();
        let result =
            sandbox.execute(&genome, &serde_json::json!({})).await.expect("测试：异步操作应成功");
        assert!(!result.passed, "expected fail due to cycle");
        assert!(
            result.execution_errors.iter().any(|e| e.contains("cycle")),
            "expected cycle error, got: {:?}",
            result.execution_errors
        );
    }

    #[tokio::test]
    async fn test_dry_run_sandbox_allows_cycle_with_loop_node() {
        // 包含 Loop 节点 + 环 → 不应报 cycle 错误(环是 Loop 的预期结构)
        let json = r#"{
            "template_id": "t1",
            "name": "test",
            "nodes": [
                {"type": "loop",
                 "id": "loop1", "title": "loop", "position": {"x": 0, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"loop_type": "forEach", "items_var": "items", "iter_input_var": null, "iteratee_var": "item", "iter_output_var": "out"}},
                {"type": "delay",
                 "id": "n1", "title": "delay", "position": {"x": 100, "y": 0},
                 "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                 "enabled": true,
                 "config": {"delay_type": "seconds", "seconds": 1, "until": null}}
            ],
            "edges": [
                {"id": "e1", "source": "loop1", "sourceHandle": null, "target": "n1", "targetHandle": null, "edge_type": "direct", "label": null},
                {"id": "e2", "source": "n1", "sourceHandle": null, "target": "loop1", "targetHandle": null, "edge_type": "direct", "label": null}
            ],
            "variables": [],
            "fitness": 0.5,
            "generation": 0
        }"#;
        let genome: WorkflowGenome = serde_json::from_str(json).expect("deserialize genome");
        let sandbox = DryRunWorkflowSandbox::new();
        let result =
            sandbox.execute(&genome, &serde_json::json!({})).await.expect("测试：异步操作应成功");
        // 不应有 cycle 错误(Loop 节点允许环)
        assert!(
            !result.execution_errors.iter().any(|e| e.contains("cycle")),
            "Loop node should allow cycle, got: {:?}",
            result.execution_errors
        );
    }

    // ── T4.4:SelfReferenceArtifactValidator 单元测试 ──

    #[test]
    fn test_artifact_validator_passes_normal_rhai_script() {
        // 纯计算脚本(无保护关键词 / 无危险模式 / 长度正常)→ 通过
        let validator = SelfReferenceArtifactValidator::new();
        let violations = validator.validate_code("let x = input * 2;\nx + 1");
        assert!(violations.is_empty(), "正常脚本应通过, got: {violations:?}");
    }

    #[test]
    fn test_artifact_validator_rejects_self_reference_keyword() {
        // 命中自指熔断保护关键词(/evolution/)→ 拒绝
        let validator = SelfReferenceArtifactValidator::new();
        let violations = validator.validate_code("let route = \"/evolution/tool_x\";\nroute");
        assert!(
            violations.iter().any(|v| v.contains("/evolution/")),
            "应命中 /evolution/ 保护关键词, got: {violations:?}"
        );
    }

    #[test]
    fn test_artifact_validator_rejects_dangerous_pattern() {
        // 命中危险模式(process::Command)→ 拒绝
        let validator = SelfReferenceArtifactValidator::new();
        let violations =
            validator.validate_code("let p = \"process::Command::new(\\\"rm\\\")\";\np");
        assert!(
            violations.iter().any(|v| v.contains("process::Command")),
            "应命中危险模式 process::Command, got: {violations:?}"
        );
    }

    #[test]
    fn test_artifact_validator_rejects_oversized_script() {
        // 超长脚本(> RHAI_SCRIPT_MAX_LEN)→ 拒绝
        let validator = SelfReferenceArtifactValidator::new();
        let long_code = "let x = 1;\n".repeat(RHAI_SCRIPT_MAX_LEN / 10 + 1);
        let violations = validator.validate_code(&long_code);
        assert!(
            violations.iter().any(|v| v.contains("超过上限")),
            "应命中长度限制, got: {violations:?}"
        );
    }

    // ── T5A.3:EvolutionFeedbackSinkImpl 单元测试 ──

    /// 嵌套统计表类型别名（D2 会话隔离：会话 → 工具 → 统计）。
    type StatsMap = Arc<Mutex<HashMap<String, HashMap<String, ToolExecutionStats>>>>;

    /// 构造嵌套统计表（D2 会话隔离）的测试 sink（无 DB，仅内存累计）。
    fn make_sink() -> (StatsMap, EvolutionFeedbackSinkImpl) {
        let stats: StatsMap = Arc::new(Mutex::new(HashMap::new()));
        let sink = EvolutionFeedbackSinkImpl::new(stats.clone(), None);
        (stats, sink)
    }

    #[test]
    fn test_feedback_sink_accumulates_stats() {
        let (stats, sink) = make_sink();

        sink.record(Some("conv_a"), "tool_a", true);
        sink.record(Some("conv_a"), "tool_a", false);
        sink.record(Some("conv_a"), "tool_b", true);

        let snapshot = stats.blocking_lock().clone();
        let conv = snapshot.get("conv_a").expect("conv_a 应有会话桶");
        let a = conv.get("tool_a").copied().expect("tool_a 应有统计");
        assert_eq!(a.usage_count, 2);
        assert_eq!(a.successes, 1);
        assert_eq!(a.failures, 1);

        let b = conv.get("tool_b").copied().expect("tool_b 应有统计");
        assert_eq!(b.usage_count, 1);
        assert_eq!(b.successes, 1);
        assert_eq!(b.failures, 0);
    }

    /// D2 会话隔离：不同会话的统计互不影响，且无会话上下文落到全局桶。
    #[test]
    fn test_feedback_sink_isolates_conversations() {
        let (stats, sink) = make_sink();

        sink.record(Some("conv_a"), "tool_x", true);
        sink.record(Some("conv_b"), "tool_x", false);
        sink.record(None, "tool_x", true);

        let snapshot = stats.blocking_lock().clone();

        // 三个会话桶各自独立
        let a = snapshot.get("conv_a").and_then(|m| m.get("tool_x")).copied().unwrap();
        assert_eq!((a.usage_count, a.successes, a.failures), (1, 1, 0));

        let b = snapshot.get("conv_b").and_then(|m| m.get("tool_x")).copied().unwrap();
        assert_eq!((b.usage_count, b.successes, b.failures), (1, 0, 1));

        // 无会话上下文 → 全局桶 ""
        let g = snapshot.get("").and_then(|m| m.get("tool_x")).copied().unwrap();
        assert_eq!((g.usage_count, g.successes, g.failures), (1, 1, 0));

        // 断言无跨会话污染：conv_a 里只有 1 次统计
        assert_eq!(snapshot.get("conv_a").map(|m| m.len()).unwrap_or(0), 1);
    }
}
