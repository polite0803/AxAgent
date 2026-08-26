// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板进化 trait 契约(三层 trait 之二:Evolver)
//!
//! 把工作流模板视为"基因组",复用 GEPA 遗传算法(选择/交叉/变异)。
//! 不重定义 `SkillGenome`,新建 `WorkflowGenome` 同构结构。
//!
//! 异步执行:进化流程在后台 `tokio::spawn`,不阻塞工作流引擎主流程。

use crate::reflection_types::Reflection;
use crate::trajectory_types::GeneratedTool;
use crate::workflow_reflection::{WorkflowPattern, WorkflowRunStatus};
use crate::workflow_types::{WorkflowEdge, WorkflowNode};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 工作流基因组 ──

/// 工作流模板的可进化表示(同构于 `SkillGenome`)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowGenome {
    pub template_id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: Vec<serde_json::Value>,
    pub fitness: f32,
    pub generation: u32,
    /// 方案 1B:LLM 显式标注本次变异修改的 node_id 列表(空 = 未声明)。
    ///
    /// 合并器按此 mask 选择性替换原 genome 的节点,未声明的节点保留原值,
    /// 避免 LLM"误伤"健康节点。LLM 在 prompt 中被要求输出 `changed_node_ids`。
    #[serde(default)]
    pub changed_node_ids: Vec<String>,
}

// ── 进化配置(与 trajectory::EvolutionConfig 字段对齐,便于转换) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub max_generations: usize,
    pub convergence_threshold: f64,
    pub use_llm_mutation: bool,
    pub use_execution_validation: bool,
    pub validation_rounds: usize,
    pub auto_trigger_consecutive_failures: u32,
    pub auto_trigger_min_usages: u32,
    pub auto_trigger_success_threshold: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            elite_count: 4,
            mutation_rate: 0.15,
            crossover_rate: 0.7,
            max_generations: 50,
            convergence_threshold: 0.95,
            use_llm_mutation: true,
            use_execution_validation: true,
            validation_rounds: 3,
            auto_trigger_consecutive_failures: 3,
            auto_trigger_min_usages: 3,
            auto_trigger_success_threshold: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionPopulation {
    pub generation: u32,
    pub individuals: Vec<WorkflowGenome>,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionStats {
    pub generation: u32,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
    pub converged: bool,
}

// ── 进化结果 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowModification {
    pub template_id: String,
    pub generation: u32,
    pub original: WorkflowGenome,
    pub evolved: WorkflowGenome,
    pub fitness_delta: f32,
    pub changes: Vec<GenomeChange>,
    pub validation: SandboxValidationResult,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenomeChange {
    /// `node` 装箱以避免 `large_enum_variant`。
    NodeAdded {
        node: Box<WorkflowNode>,
        position: GenomePosition,
    },
    NodeRemoved {
        node_id: String,
    },
    /// `new_node` 装箱以避免 `large_enum_variant`。
    NodeReplaced {
        node_id: String,
        new_node: Box<WorkflowNode>,
    },
    EdgeAdded {
        edge: WorkflowEdge,
    },
    EdgeRemoved {
        from: String,
        to: String,
    },
    EdgeRewired {
        from: String,
        original_to: String,
        new_to: String,
    },
    ConfigPatched {
        node_id: String,
        patch: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomePosition {
    pub after_node: Option<String>,
    pub before_node: Option<String>,
    pub branch_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxValidationResult {
    pub passed: bool,
    pub success_rate: f32,
    pub execution_errors: Vec<String>,
    pub avg_execution_time_ms: u64,
}

// ── Trait 契约 ──

/// 工作流模板进化器:基于反思结果与历史执行,进化工作流模板。
///
/// **三层 trait 之二**(Reflector / Evolver / Optimizer)。
///
/// 实现方:trajectory crate(复用 `SkillEvolutionEngine` 的遗传算子 + `LlmEvolutionProvider`)。
/// 调用方:wiring 层(批量后台任务)、命令层(`commands/evolution*`)。
///
/// 执行方式:异步(`tokio::spawn`),进化完成后通过事件/回调通知。
#[async_trait]
pub trait WorkflowEvolver: Send + Sync + std::any::Any {
    /// 初始化种群(从模板生成初始基因组)。
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String>;

    /// 进化一代。
    ///
    /// `reflections` 为近期反思结果,用于计算适应度与变异方向。
    async fn evolve_generation(
        &self,
        population: &mut EvolutionPopulation,
        reflections: &[Reflection],
    ) -> Result<WorkflowGenome, String>;

    /// 完整进化流程(到收敛或 `max_generations`)。
    ///
    /// 建议在 `tokio::spawn` 中调用,避免阻塞主流程。
    async fn run(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String>;

    /// 自动触发判定(基于近期失败率与使用次数)。
    async fn should_auto_evolve(&self, template_id: &str) -> Result<bool, String>;

    /// 记录一次反思结果(由 wiring 层在每次 reflect 完成后调用)。
    ///
    /// 实现方应保留近期反思的 (quality_score, status) 元组,供 `should_auto_evolve`
    /// 判定是否达到自动进化阈值。**不返回错误**:反思记录是辅助数据,失败不应影响
    /// 工作流主流程(实现方内部日志吞掉错误即可)。
    async fn record_reflection(
        &self,
        template_id: &str,
        quality_score: u8,
        status: WorkflowRunStatus,
    );

    /// 注入 LLM 变异 provider。
    async fn set_llm_provider(&self, provider: Arc<dyn WorkflowLlmMutator>) -> Result<(), String>;

    /// 注入沙箱验证器。
    async fn set_sandbox(&self, sandbox: Arc<dyn WorkflowSandbox>) -> Result<(), String>;

    /// 注入基因组加载器(wiring 层从 DB 加载模板后构造 `WorkflowGenome`)。
    ///
    /// 未注入时,`initialize` 退化为占位(单个体空 genome);注入后,种群初始化
    /// 才能基于真实模板生成多个个体(扰动 + 交叉)。
    async fn set_genome_loader(&self, loader: Arc<dyn WorkflowGenomeLoader>) -> Result<(), String>;

    /// 状态查询。
    async fn get_stats(&self) -> Result<EvolutionStats, String>;
    async fn is_running(&self) -> Result<bool, String>;
}

/// 工作流基因组加载器:从 DB / 模板表加载并构造 `WorkflowGenome`。
///
/// **wiring 层**实现(委托 `WorkflowTemplateRepository`),trajectory 层只
/// 通过此 trait 拿到 genome,不直接依赖 dao / entities。
pub trait WorkflowGenomeLoader: Send + Sync {
    /// 按 `template_id` 加载基因组(可选返回 None,如模板不存在)。
    ///
    /// 实现方应从 `WorkflowTemplateData.nodes / edges / variables` 反序列化
    /// 出 `WorkflowNode` / `WorkflowEdge` 列表,组合成 `WorkflowGenome`。
    fn load_genome(
        &self,
        template_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<WorkflowGenome>> + Send>>;
}

/// 工作流 LLM 变异器:扩展 `LlmEvolutionProvider` 支持工作流节点语义。
///
/// 实现方可内部委托 `axagent_harness::trajectory_types::LlmEvolutionProvider`,
/// 并将 `ProcedureStep` 与 `WorkflowNode` 互转。
#[async_trait]
pub trait WorkflowLlmMutator: Send + Sync {
    async fn generate_mutation(
        &self,
        genome: &WorkflowGenome,
        failure_evidence: &[WorkflowPattern],
        success_evidence: &[WorkflowPattern],
    ) -> Result<WorkflowGenome, String>;

    async fn evaluate_quality(&self, genome: &WorkflowGenome, context: &str)
    -> Result<f32, String>;
}

/// 工作流沙箱:执行进化后的模板以验证可行性。
///
/// 实现方可委托 `trajectory::sandbox_executor::SkillSandboxExecutor`,
/// 将工作流模板转换为技能 steps 后执行。
#[async_trait]
pub trait WorkflowSandbox: Send + Sync + std::any::Any {
    async fn execute(
        &self,
        genome: &WorkflowGenome,
        test_input: &serde_json::Value,
    ) -> Result<SandboxValidationResult, String>;
}

/// 编排型进化产物执行器(T4.3)。
///
/// `GeneratedToolAdapter` 对 `WorkflowDag` 类型产物调用该 trait 执行,
/// 将 `WorkflowGenome` 交由 rt-workflow 引擎(或沙箱)真正运行。
///
/// 依赖方向:trajectory / tools 仅依赖本契约;wiring 层将 `WorkEngine`
/// 包装为 `Arc<dyn WorkflowDagExecutor>` 注入,不打破 harness 分层。
#[async_trait]
pub trait WorkflowDagExecutor: Send + Sync + std::any::Any {
    /// 执行编排型进化产物。
    ///
    /// `genome` 为产物映射出的 `WorkflowGenome`,`input` 为工具调用入参。
    /// 返回执行后的精简结果(`Workflow.output` 或节点结果聚合)。
    async fn execute(
        &self,
        genome: &WorkflowGenome,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

/// 进化产物代码沙箱验证器(T4.4)。
///
/// 计算型(`RhaiScript`)产物在 `GeneratedToolAdapter::call()` 真正执行前,
/// 先调用本 trait 验证代码安全性:
/// - 脚本/代码长度限制(防超长脚本 DoS)
/// - 自指熔断关键词检测(防进化产物递归调用系统能力,复用 `SelfReferenceProtection`)
/// - 危险模式检测(与 `SkillSandboxExecutor` 同思路的第一道防线)
///
/// 返回违规原因列表;空列表 = 通过,允许执行。
/// 依赖方向:tools 仅依赖本契约;wiring 层将安全策略实现注入,不打破 harness 分层。
/// 与 [`WorkflowDagExecutor`] 对称:均为"进化产物执行链"的 wiring 注入接缝。
pub trait EvolutionArtifactValidator: Send + Sync + std::any::Any {
    /// 验证进化产物代码是否安全。返回违规原因列表(空 = 通过)。
    fn validate_code(&self, code: &str) -> Vec<String>;
}

/// 将进化产物(`GeneratedTool`)映射为可执行的 `WorkflowGenome`(T4.3)。
///
/// 映射规则:
/// - 优先:产物 `code` 本身是 `WorkflowGenome` 的 JSON(LLM 结构化输出直接落库),
///   直接反序列化并补齐模板元信息。
/// - 兜底:产物 `code` 是自定义 DAG 描述(含 `nodes`/`edges`/`variables` 键),
///   将其节点映射为 `WorkflowNode`(默认 `delay` 类型不可用,按 `kind` 字段映射)。
///
/// 产物 `id` 用作 `template_id`,保证每次进化产物有独立可溯源模板 ID。
pub fn workflow_genome_from_generated(tool: &GeneratedTool) -> Result<WorkflowGenome, String> {
    // 1. 优先:code 直接是 WorkflowGenome JSON
    if let Ok(genome) = serde_json::from_str::<WorkflowGenome>(&tool.code) {
        return Ok(genome);
    }

    // 2. 兜底:code 是自定义 DAG 描述(含 nodes/edges/variables 键)
    let value: serde_json::Value =
        serde_json::from_str(&tool.code).map_err(|e| format!("进化产物 code 非 JSON: {e}"))?;
    let nodes_value = value.get("nodes").ok_or("进化产物缺少 nodes")?;
    let edges_value = value.get("edges").unwrap_or(&serde_json::Value::Null);
    let variables_value = value.get("variables").unwrap_or(&serde_json::Value::Null);

    let nodes: Vec<WorkflowNode> = nodes_value
        .as_array()
        .ok_or("nodes 必须是数组")?
        .iter()
        .map(|n| {
            // 节点 JSON 直接反序列化为 WorkflowNode(带 type tag 的 enum)
            serde_json::from_value::<WorkflowNode>(n.clone())
                .map_err(|e| format!("节点反序列化失败: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let edges: Vec<WorkflowEdge> = edges_value
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| {
                    serde_json::from_value::<WorkflowEdge>(e.clone())
                        .map_err(|err| format!("边反序列化失败: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))?;

    let variables: Vec<serde_json::Value> = variables_value.as_array().cloned().unwrap_or_default();

    Ok(WorkflowGenome {
        template_id: tool.id.clone(),
        name: tool.name.clone(),
        nodes,
        edges,
        variables,
        fitness: 0.0,
        generation: 0,
        changed_node_ids: Vec::new(),
    })
}

// ── 基础校验工具(方案 1A) ──

/// 对基因组做最小一致性校验:node id 不重复 / edge 引用有效 / variable name 不重复。
///
/// 用于进化器在替换 LLM 变异结果前做快速结构检查(轻量,不调用 LLM)。
/// 返回错误列表(空 = 通过)。**不**包含 nodes 非空 / variables 数量上限等业务约束,
/// 后者由沙箱 [`WorkflowSandbox`] 实现(如 `StructuralWorkflowSandbox`)负责。
pub fn validate_genome_basic(genome: &WorkflowGenome) -> Vec<String> {
    let mut errors = Vec::new();

    // 1. node id 不重复
    let mut seen_node_ids = std::collections::HashSet::new();
    for node in &genome.nodes {
        let id = node.base_id();
        if !seen_node_ids.insert(id.to_string()) {
            errors.push(format!("duplicate node id: {id}"));
        }
    }

    // 2. edge source/target 必须引用已存在的节点 id
    let node_ids: std::collections::HashSet<&str> =
        genome.nodes.iter().map(|n| n.base_id()).collect();
    for edge in &genome.edges {
        if !node_ids.contains(edge.source.as_str()) {
            errors.push(format!("edge.source '{}' not in nodes", edge.source));
        }
        if !node_ids.contains(edge.target.as_str()) {
            errors.push(format!("edge.target '{}' not in nodes", edge.target));
        }
    }

    // 3. variable name 不重复(若 Value 是对象且含 "name" 字段)
    //    `WorkflowGenome.variables` 类型为 `Vec<serde_json::Value>`,
    //    实际承载 `Variable` 结构,这里宽容地从 JSON 提取 name。
    let mut seen_var_names = std::collections::HashSet::new();
    for v in &genome.variables {
        if let Some(name) = v.get("name").and_then(|n| n.as_str())
            && !seen_var_names.insert(name.to_string())
        {
            errors.push(format!("duplicate variable name: {name}"));
        }
    }

    errors
}

// ── 节点级 diff 合并(方案 1B) ──

/// 按 `changed_node_ids` mask 选择性合并 `original` 与 `mutated`。
///
/// - mask 中声明的 node_id → 取 `mutated` 的版本(允许 LLM 修改)
/// - 未声明的 node_id → 保留 `original` 的版本(避免 LLM 误伤健康节点)
/// - edges / variables / changed_node_ids → 取 `mutated`(允许 LLM 重连 / 调整变量)
/// - fitness / generation → 取 `mutated`(LLM 变异后的版本号)
///
/// 若 `mutated.changed_node_ids` 为空(未声明),退化为整体替换(向后兼容批次 A/B)。
pub fn merge_genome_by_mask(original: &WorkflowGenome, mutated: &WorkflowGenome) -> WorkflowGenome {
    // mask 为空 → 整体替换(向后兼容)
    if mutated.changed_node_ids.is_empty() {
        return mutated.clone();
    }

    let mask: std::collections::HashSet<&str> =
        mutated.changed_node_ids.iter().map(|s| s.as_str()).collect();

    // nodes:按 mask 选择性合并
    let mut merged_nodes: Vec<WorkflowNode> = Vec::with_capacity(original.nodes.len());
    // 先放入 original 中未在 mask 内的节点
    for node in &original.nodes {
        if !mask.contains(node.base_id()) {
            merged_nodes.push(node.clone());
        }
    }
    // 再放入 mutated 中 mask 内的节点(顺序可能变化,但保持 mutated 的拓扑)
    for node in &mutated.nodes {
        if mask.contains(node.base_id()) {
            merged_nodes.push(node.clone());
        }
    }

    // 若 mask 中的 node_id 在 mutated 中不存在(LLM 声明删除),则该节点不会出现
    // 若 mask 中的 node_id 在 original 中不存在(LLM 声明新增),则该节点会出现

    WorkflowGenome {
        template_id: mutated.template_id.clone(),
        name: mutated.name.clone(),
        nodes: merged_nodes,
        edges: mutated.edges.clone(),
        variables: mutated.variables.clone(),
        fitness: mutated.fitness,
        generation: mutated.generation,
        changed_node_ids: mutated.changed_node_ids.clone(),
    }
}

#[cfg(test)]
mod validate_genome_basic_tests {
    use super::*;

    /// 用 JSON 反序列化构造 genome(避免手工构造 WorkflowNodeBase 全字段)。
    ///
    /// `WorkflowNode` 用 `#[serde(tag="type", rename_all="camelCase")]` + `#[serde(flatten)]` base,
    /// 因此 JSON 形式为 `{"type":"delay", ...平铺字段}`(非 `{"Delay":{...}}` 的 newtype 形式)。
    fn make_genome(
        node_ids: &[&str],
        edges: &[(&str, &str)],
        variables: &[&str],
    ) -> WorkflowGenome {
        use serde_json::json;
        let nodes: Vec<crate::workflow_types::WorkflowNode> = node_ids
            .iter()
            .map(|id| {
                json!({
                    "type": "delay",
                    "id": id,
                    "title": format!("node-{id}"),
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"delay_type": "seconds", "seconds": 1, "until": null}
                })
            })
            .map(|v| serde_json::from_value(v).expect("deserialize node"))
            .collect();
        let edges: Vec<crate::workflow_types::WorkflowEdge> = edges
            .iter()
            .map(|(s, t)| {
                json!({
                    "id": format!("{s}-{t}"),
                    "source": s,
                    "sourceHandle": null,
                    "target": t,
                    "targetHandle": null,
                    "edge_type": "direct",
                    "label": null
                })
            })
            .map(|v| serde_json::from_value(v).expect("deserialize edge"))
            .collect();
        let variables: Vec<serde_json::Value> =
            variables.iter().map(|n| json!({"name": n, "value": 0})).collect();
        WorkflowGenome {
            template_id: "test".into(),
            name: "test".into(),
            nodes,
            edges,
            variables,
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        }
    }

    #[test]
    fn passes_on_valid_genome() {
        let g = make_genome(&["n1", "n2"], &[("n1", "n2")], &["v1"]);
        assert!(validate_genome_basic(&g).is_empty());
    }

    #[test]
    fn catches_duplicate_node_id() {
        let g = make_genome(&["n1", "n1"], &[], &["v1"]);
        let errs = validate_genome_basic(&g);
        assert!(errs.iter().any(|e| e.contains("duplicate node id")));
    }

    #[test]
    fn catches_dangling_edge() {
        let g = make_genome(&["n1"], &[("n1", "missing")], &["v1"]);
        let errs = validate_genome_basic(&g);
        assert!(errs.iter().any(|e| e.contains("not in nodes")));
    }

    #[test]
    fn catches_duplicate_variable_name() {
        let g = make_genome(&["n1"], &[], &["v1", "v1"]);
        let errs = validate_genome_basic(&g);
        assert!(errs.iter().any(|e| e.contains("duplicate variable name")));
    }
}

#[cfg(test)]
mod merge_genome_by_mask_tests {
    use super::*;

    fn make_genome_with_mask(
        node_ids: &[&str],
        variables: Vec<serde_json::Value>,
        mask: Vec<&str>,
    ) -> WorkflowGenome {
        use serde_json::json;
        let nodes: Vec<crate::workflow_types::WorkflowNode> = node_ids
            .iter()
            .map(|id| {
                json!({
                    "type": "delay",
                    "id": id,
                    "title": format!("node-{id}"),
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"delay_type": "seconds", "seconds": 1, "until": null}
                })
            })
            .map(|v| serde_json::from_value(v).expect("deserialize node"))
            .collect();
        WorkflowGenome {
            template_id: "test".into(),
            name: "test".into(),
            nodes,
            edges: vec![],
            variables,
            fitness: 0.5,
            generation: 0,
            changed_node_ids: mask.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn empty_mask_falls_back_to_wholesale_replace() {
        let original = make_genome_with_mask(&["n1", "n2"], vec![], vec![]);
        let mutated = make_genome_with_mask(&["n1"], vec![], vec![]);
        let merged = merge_genome_by_mask(&original, &mutated);
        // mask 为空 → 整体替换,merged.nodes == mutated.nodes
        assert_eq!(merged.nodes.len(), 1);
        assert_eq!(merged.nodes[0].base_id(), "n1");
    }

    #[test]
    fn mask_preserves_unmentioned_nodes() {
        // original 有 n1, n2, n3;mutated 只声明改了 n2
        let original = make_genome_with_mask(&["n1", "n2", "n3"], vec![], vec![]);
        let mut mutated = make_genome_with_mask(&["n2"], vec![], vec!["n2"]);
        mutated.nodes[0] = serde_json::from_value(serde_json::json!({
            "type": "delay",
            "id": "n2",
            "title": "mutated-n2",
            "position": {"x": 0, "y": 0},
            "retry": {"enabled": true, "max_retries": 5, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
            "enabled": true,
            "config": {"delay_type": "seconds", "seconds": 10, "until": null}
        })).expect("deserialize mutated node");

        let merged = merge_genome_by_mask(&original, &mutated);
        // n1, n3 保留 original 版本,n2 取 mutated 版本
        assert_eq!(merged.nodes.len(), 3);
        let n2 = merged.nodes.iter().find(|n| n.base_id() == "n2").expect("n2 present");
        assert_eq!(n2.base().title, "mutated-n2");
        assert_eq!(n2.base().retry.max_retries, 5);
        let n1 = merged.nodes.iter().find(|n| n.base_id() == "n1").expect("n1 present");
        assert_eq!(n1.base().title, "node-n1"); // 保留 original
    }

    #[test]
    fn mask_allows_node_deletion() {
        // mutated 声明改了 n2,但 mutated.nodes 中没有 n2 → 视为删除
        let original = make_genome_with_mask(&["n1", "n2", "n3"], vec![], vec![]);
        let mutated = make_genome_with_mask(&[], vec![], vec!["n2"]);

        let merged = merge_genome_by_mask(&original, &mutated);
        // n1, n3 保留,n2 被删除
        assert_eq!(merged.nodes.len(), 2);
        assert!(merged.nodes.iter().all(|n| n.base_id() != "n2"));
    }

    #[test]
    fn mask_allows_node_addition() {
        // mutated 声明新增 n4(不在 original 中)
        let original = make_genome_with_mask(&["n1"], vec![], vec![]);
        let mutated = make_genome_with_mask(&["n4"], vec![], vec!["n4"]);

        let merged = merge_genome_by_mask(&original, &mutated);
        // n1 保留,n4 新增
        assert_eq!(merged.nodes.len(), 2);
        assert!(merged.nodes.iter().any(|n| n.base_id() == "n1"));
        assert!(merged.nodes.iter().any(|n| n.base_id() == "n4"));
    }
}

// ── T4.3:GeneratedTool → WorkflowGenome 转换(分层执行的编排型产物入口) ──

#[cfg(test)]
mod workflow_genome_from_generated_tests {
    use super::*;
    use crate::trajectory_types::EvolutionArtifactKind;

    /// code 直接是 WorkflowGenome JSON(带 template_id/name/nodes/...) → 直接路径
    #[test]
    fn direct_genome_json_path() {
        let tool = GeneratedTool::with_artifact_kind(
            "wf_direct",
            &serde_json::json!({
                "template_id": "wf-direct",
                "name": "wf_direct",
                "nodes": [{
                    "type": "delay",
                    "id": "d1",
                    "title": "delay-1",
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"delay_type": "seconds", "seconds": 1, "until": null}
                }],
                "edges": [],
                "variables": [],
                "fitness": 0.0,
                "generation": 0,
                "changed_node_ids": []
            })
            .to_string(),
            "编排型工具",
            EvolutionArtifactKind::WorkflowDag,
        );

        let genome = workflow_genome_from_generated(&tool).expect("直接 JSON 路径应成功");
        assert_eq!(genome.template_id, "wf-direct");
        assert_eq!(genome.name, "wf_direct");
        assert_eq!(genome.nodes.len(), 1);
        assert_eq!(genome.nodes[0].base_id(), "d1");
    }

    /// code 是自定义 DAG 描述(仅 nodes/edges/variables,无 genome 外壳字段) → 兜底路径
    #[test]
    fn fallback_custom_dag_description_path() {
        let tool = GeneratedTool::with_artifact_kind(
            "wf_fallback",
            &serde_json::json!({
                "nodes": [{
                    "type": "delay",
                    "id": "d1",
                    "title": "delay-1",
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 3, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"delay_type": "seconds", "seconds": 1, "until": null}
                }],
                "edges": [],
                "variables": []
            })
            .to_string(),
            "编排型工具(自定义描述)",
            EvolutionArtifactKind::WorkflowDag,
        );

        let genome = workflow_genome_from_generated(&tool).expect("兜底路径应成功");
        // 兜底路径下 template_id/name 回退到 tool 字段(id 为 UUID,仅断言非空)
        assert!(!genome.template_id.is_empty());
        assert_eq!(genome.name, "wf_fallback");
        assert_eq!(genome.nodes.len(), 1);
        assert_eq!(genome.variables.len(), 0);
    }

    /// code 非 JSON → 明确错误,不静默
    #[test]
    fn non_json_code_returns_error() {
        let tool = GeneratedTool::with_artifact_kind(
            "wf_bad",
            "not-json-at-all",
            "非法编排型工具",
            EvolutionArtifactKind::WorkflowDag,
        );

        let err = workflow_genome_from_generated(&tool).unwrap_err();
        assert!(err.contains("非 JSON"), "错误信息应说明 code 非 JSON: {err}");
    }
}

/// 进化产物运行时执行统计（贝叶斯证据输入，阶段四后置闭环）。
///
/// 由 wiring 层按 `tool_id` 累计，作为「真实执行反馈」的证据源，
/// 供 `EvolutionDecider` 重建后验（与 `DecisionEvidence` 的「按模式推断成败」对照，
/// 真实执行结果优先）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionStats {
    /// 已执行次数。
    pub usage_count: u32,
    /// 真实成功次数。
    pub successes: u32,
    /// 真实失败次数。
    pub failures: u32,
}

/// 进化产物执行反馈回传契约（阶段四后置闭环）。
///
/// `GeneratedToolAdapter::call` 执行完成（成功/失败）后调用，
/// wiring 层实现把真实执行结果累计到进化产物的贝叶斯证据。
///
/// 与 [`WorkflowDagExecutor`] / [`EvolutionArtifactValidator`] 对称，
/// 均为"进化产物执行链"的 wiring 注入接缝，tools 仅依赖本契约，
/// 不打破 harness 分层。
pub trait ExecutionFeedbackSink: Send + Sync + std::any::Any {
    /// 上报一次进化产物执行结果。
    ///
    /// `conversation_id` 为当前执行所属会话（`None` 表示无会话上下文，
    /// 如纯 tools 层测试），`tool_id` 为产物标识(`GeneratedTool.id`)，
    /// `success` 为真实成败。
    /// 实现方须线程安全（wiring 层用 `tokio::sync::Mutex` 保护统计表）。
    fn record(&self, conversation_id: Option<&str>, tool_id: &str, success: bool);
}
