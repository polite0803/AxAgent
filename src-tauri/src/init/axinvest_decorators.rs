// SPDX-License-Identifier: AGPL-3.0-only

//! AxInvest 专属反思/进化装饰器（wiring 层）。
//!
//! 设计目的：上游 AxAgent 的自我进化与反思能力增强后，本地股票工作流需要
//! 「在保留上游基础能力的同时叠加 AxInvest 专属语义」——装饰器模式是最小侵入方案。
//!
//! ## 两个装饰器
//!
//! - [`AxInvestReflectorDecorator`]：包装默认 `WorkflowReflector`，在反思完成后
//!   扫描 `WorkflowExecutionRecord.nodes[].output` 中的 `__untrusted: true` 标记，
//!   把不可信节点列表追加到 `Reflection.metadata.axinvest_untrusted_nodes`，
//!   让下游 reflection-comparator.rhai / evolver 能感知数据质量异常。
//!
//! - [`AxInvestEvolverDecorator`]：包装默认 `WorkflowEvolver`，对 `stock-*` 前缀
//!   模板的 `should_auto_evolve` 强制返回 false。理由：stock-analysis 模板失败率
//!   容易超过 50% 触发自动进化，但业务调优依赖人工专家经验，自动进化会破坏
//!   业务节奏。`record_reflection` 仍正常记录，仅阻断自动变异。
//!
//! ## 架构合规
//!
//! - 装饰器位于 wiring 层（`src/init/`），不改 harness / trajectory 任何代码
//! - 通过 `Arc<dyn WorkflowReflector>` / `Arc<dyn WorkflowEvolver>` trait object
//!   天然支持包装器模式
//! - AxInvest 装饰器不重复定义任何 DTO，仅在 `Reflection.metadata` 上叠加字段

use std::sync::Arc;

use async_trait::async_trait;

use axagent_harness::workflow_evolution::{
    EvolutionPopulation, EvolutionStats, WorkflowGenome, WorkflowModification,
};
use axagent_harness::workflow_reflection::{
    NodeExecutionSnapshot, WorkflowExecutionRecord, WorkflowPattern,
};
use axagent_harness::{Reflection, WorkflowEvolver, WorkflowReflector};

// ───────────────────────── Reflector 装饰器 ─────────────────────────

/// AxInvest 专属反思装饰器。
///
/// 包装上游 `WorkflowReflectorImpl`，在反思结果上叠加 AxInvest 业务语义：
/// 扫描所有节点输出中的 `__untrusted: true` 标记，追加到 metadata。
pub struct AxInvestReflectorDecorator {
    inner: Arc<dyn WorkflowReflector>,
}

impl AxInvestReflectorDecorator {
    pub fn new(inner: Arc<dyn WorkflowReflector>) -> Self {
        Self { inner }
    }

    /// 扫描节点输出中的 `__untrusted: true` 标记。
    ///
    /// 兼容多种 output 形态：
    /// - JSON 对象 `{"__untrusted": true, ...}`
    /// - JSON 字符串（内部再解析为对象）
    /// - output 包裹在 `{"content": "..."}` 中（tool result 标准包装）
    fn scan_untrusted_nodes(nodes: &[NodeExecutionSnapshot]) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for n in nodes {
            if let Some(output) = &n.output
                && output_is_untrusted(output)
            {
                out.push(serde_json::json!({
                    "node_id": n.node_id,
                    "node_type": n.node_type,
                    "error": n.error,
                }));
            }
        }
        out
    }
}

/// 判断 output 是否标记为 `__untrusted`。
///
/// 多形态兼容：
/// 1. output 是对象 `{"__untrusted": true}`
/// 2. output 是字符串，内部是 JSON 对象
/// 3. output 是 `{"content": "<json string>"}` 包装
fn output_is_untrusted(output: &serde_json::Value) -> bool {
    // 形态 1：直接对象
    if let Some(flag) = output.get("__untrusted").and_then(|v| v.as_bool()) {
        return flag;
    }
    // 形态 3：content 包装
    if let Some(content) = output.get("content").and_then(|v| v.as_str()) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(flag) = parsed.get("__untrusted").and_then(|v| v.as_bool()) {
                return flag;
            }
        }
    }
    // 形态 2：字符串内嵌 JSON
    if let Some(s) = output.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            if let Some(flag) = parsed.get("__untrusted").and_then(|v| v.as_bool()) {
                return flag;
            }
        }
    }
    false
}

#[async_trait]
impl WorkflowReflector for AxInvestReflectorDecorator {
    async fn reflect(&self, record: &WorkflowExecutionRecord) -> Result<Reflection, String> {
        let mut reflection = self.inner.reflect(record).await?;

        // 扫描 __untrusted 节点
        let untrusted = Self::scan_untrusted_nodes(&record.nodes);
        if !untrusted.is_empty() {
            let metadata = reflection.metadata.take().unwrap_or_else(|| serde_json::json!({}));
            // 在 metadata 上叠加 axinvest_untrusted_nodes 字段
            let mut metadata_obj = match metadata {
                serde_json::Value::Object(map) => map,
                other => {
                    // 非 object（罕见）→ 包装到 { "inner": other }
                    let mut map = serde_json::Map::new();
                    map.insert("inner".to_string(), other);
                    map
                },
            };
            metadata_obj.insert(
                "axinvest_untrusted_nodes".to_string(),
                serde_json::Value::Array(untrusted),
            );
            reflection.metadata = Some(serde_json::Value::Object(metadata_obj));
        }
        Ok(reflection)
    }

    async fn reflect_node(
        &self,
        record: &WorkflowExecutionRecord,
        failed_node: &NodeExecutionSnapshot,
    ) -> Result<Reflection, String> {
        // 节点级反思直接委托，不做 AxInvest 专属处理
        self.inner.reflect_node(record, failed_node).await
    }

    async fn reflect_batch(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<Reflection>, String> {
        // 批量反思委托，但每个 reflection 仍走 reflect() 的装饰逻辑
        let mut out = Vec::with_capacity(records.len());
        for r in records {
            out.push(self.reflect(r).await?);
        }
        Ok(out)
    }

    async fn aggregate_patterns(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<WorkflowPattern>, String> {
        // 模式聚合直接委托
        self.inner.aggregate_patterns(records).await
    }

    async fn get_history(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> Result<Vec<Reflection>, String> {
        // 历史查询直接委托
        self.inner.get_history(workflow_id, limit).await
    }
}

// ───────────────────────── Evolver 装饰器 ─────────────────────────

/// AxInvest 专属进化装饰器。
///
/// 包装上游 `WorkflowEvolverImpl`，对 `stock-*` 前缀模板的自动进化做保护性阻断。
/// `record_reflection` 仍正常透传，仅 `should_auto_evolve` 对 stock 模板强制 false。
pub struct AxInvestEvolverDecorator {
    inner: Arc<dyn WorkflowEvolver>,
}

impl AxInvestEvolverDecorator {
    pub fn new(inner: Arc<dyn WorkflowEvolver>) -> Self {
        Self { inner }
    }

    /// 判断模板是否受 AxInvest 自动进化保护。
    ///
    /// 命中 `stock-` 前缀的模板（如 `stock-analysis`、`stock-decision`）受保护，
    /// 避免业务调优被自动变异破坏。
    fn is_protected_template(template_id: &str) -> bool {
        template_id.starts_with("stock-")
    }
}

#[async_trait]
impl WorkflowEvolver for AxInvestEvolverDecorator {
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String> {
        self.inner.initialize(template_id).await
    }

    async fn evolve_generation(
        &self,
        population: &mut EvolutionPopulation,
        reflections: &[Reflection],
    ) -> Result<WorkflowGenome, String> {
        self.inner.evolve_generation(population, reflections).await
    }

    async fn run(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String> {
        // 受保护模板理论上不会进入 run（should_auto_evolve 已拦截），
        // 但防御性再检查一次，避免上游绕过 should_auto_evolve 直接调 run
        if Self::is_protected_template(template_id) {
            tracing::warn!(
                "[AxInvest] 拒绝对 stock 模板 {} 执行自动变异（防御性拦截）",
                template_id
            );
            // 返回一个空 modification 表示无变更
            return Err(format!(
                "stock 模板 {} 受 AxInvest 自动进化保护，拒绝自动变异",
                template_id
            ));
        }
        self.inner.run(template_id, reflections).await
    }

    async fn should_auto_evolve(&self, template_id: &str) -> Result<bool, String> {
        if Self::is_protected_template(template_id) {
            tracing::info!(
                "[AxInvest] 自动进化保护：stock 模板 {} 跳过自动变异（业务调优依赖人工）",
                template_id
            );
            return Ok(false);
        }
        self.inner.should_auto_evolve(template_id).await
    }

    async fn record_reflection(
        &self,
        template_id: &str,
        quality_score: u8,
        status: axagent_harness::WorkflowRunStatus,
    ) {
        // 仍记录反思（用于 evolver 内部失败率统计），但不触发自动进化
        self.inner.record_reflection(template_id, quality_score, status).await;
    }

    async fn set_llm_provider(
        &self,
        mutator: Arc<dyn axagent_harness::WorkflowLlmMutator>,
    ) -> Result<(), String> {
        self.inner.set_llm_provider(mutator).await
    }

    async fn set_sandbox(
        &self,
        sandbox: Arc<dyn axagent_harness::WorkflowSandbox>,
    ) -> Result<(), String> {
        self.inner.set_sandbox(sandbox).await
    }

    async fn set_genome_loader(
        &self,
        loader: Arc<dyn axagent_harness::WorkflowGenomeLoader>,
    ) -> Result<(), String> {
        self.inner.set_genome_loader(loader).await
    }

    async fn get_stats(&self) -> Result<EvolutionStats, String> {
        self.inner.get_stats().await
    }

    async fn is_running(&self) -> Result<bool, String> {
        self.inner.is_running().await
    }
}

// ───────────────────────── 单元测试 ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_is_untrusted_direct_object() {
        let v = serde_json::json!({"__untrusted": true, "report": "..."});
        assert!(output_is_untrusted(&v));
    }

    #[test]
    fn test_output_is_untrusted_content_wrapper() {
        let v = serde_json::json!({
            "content": "{\"__untrusted\": true, \"verdict\": {\"position_pct\": 50}}"
        });
        assert!(output_is_untrusted(&v));
    }

    #[test]
    fn test_output_is_untrusted_string_form() {
        let v = serde_json::json!("{\"__untrusted\": true}");
        assert!(output_is_untrusted(&v));
    }

    #[test]
    fn test_output_is_untrusted_false_when_absent() {
        let v = serde_json::json!({"report": "正常输出"});
        assert!(!output_is_untrusted(&v));
    }

    #[test]
    fn test_is_protected_template() {
        assert!(AxInvestEvolverDecorator::is_protected_template("stock-analysis"));
        assert!(AxInvestEvolverDecorator::is_protected_template("stock-decision"));
        assert!(!AxInvestEvolverDecorator::is_protected_template("agent-chat"));
        assert!(!AxInvestEvolverDecorator::is_protected_template("workflow-x"));
    }
}
