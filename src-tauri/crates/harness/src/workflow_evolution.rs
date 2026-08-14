// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板进化 trait 契约(三层 trait 之二:Evolver)
//!
//! 把工作流模板视为"基因组",复用 GEPA 遗传算法(选择/交叉/变异)。
//! 不重定义 `SkillGenome`,新建 `WorkflowGenome` 同构结构。
//!
//! 异步执行:进化流程在后台 `tokio::spawn`,不阻塞工作流引擎主流程。

use crate::reflection_types::Reflection;
use crate::workflow_reflection::{WorkflowPattern, WorkflowRunStatus};
use crate::workflow_types::{WorkflowEdge, WorkflowNode};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 工作流基因组 ──

/// 工作流模板的可进化表示(同构于 `SkillGenome`)。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct EvolutionPopulation {
    pub generation: u32,
    pub individuals: Vec<WorkflowGenome>,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: u32,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub fitness_history: Vec<f32>,
    pub converged: bool,
}

// ── 进化结果 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
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
