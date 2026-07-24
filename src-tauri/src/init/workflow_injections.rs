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
            changed_node_ids: Vec::new(),
        };
        let sandbox = StructuralWorkflowSandbox::new();
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
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
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
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
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
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
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
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
        let result =
            futures::executor::block_on(sandbox.execute(&genome, &serde_json::json!({}))).unwrap();
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
        let result = sandbox.execute(&genome, &serde_json::json!({})).await.unwrap();
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
        let result = sandbox.execute(&genome, &serde_json::json!({})).await.unwrap();
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
        let result = sandbox.execute(&genome, &serde_json::json!({})).await.unwrap();
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
        let result = sandbox.execute(&genome, &serde_json::json!({})).await.unwrap();
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
        let result = sandbox.execute(&genome, &serde_json::json!({})).await.unwrap();
        // 不应有 cycle 错误(Loop 节点允许环)
        assert!(
            !result.execution_errors.iter().any(|e| e.contains("cycle")),
            "Loop node should allow cycle, got: {:?}",
            result.execution_errors
        );
    }
}
