// SPDX-License-Identifier: AGPL-3.0-only

//! 股票行业编排模块
//!
//! 为股票业务场景提供基于 Orchestrator 的动态编排能力，包括：
//! - 股票行业适配器 (StockIndustryAdapter)
//! - 分析流水线编排策略 (Pipeline)
//! - 多空辩论编排策略 (Debate)
//! - 股票领域特定的反思模板、进化约束和验收标准
//!
//! # 编排流程
//!
//! ## Pipeline：全链路投资分析
//!
//! ```text
//! 数据获取 → 技术面分析 → 基本面分析 → 资金面分析 → 信号聚合 → 决策生成
//! ```
//!
//! ## Debate：多空辩论
//!
//! ```text
//! 多头分析师 ←→ 空头分析师 → 仲裁者裁决 → 最终决策
//! ```

use async_trait::async_trait;
use std::sync::Arc;

use axagent_harness::{
    industry_orchestration::types::{
        AcceptanceCriterion, DependencyType, EvolutionConstraints, ForbiddenOptimization,
        ProtectedStep, QualityThresholds, QualityWeights, ReflectionCheckpoint, ReflectionTemplate,
        StepDependency,
    },
    DecompositionPlan, DynamicSubGraph, GeneratedSubGraph, IndustryAdapter, IndustryContext,
    IndustryLearningConfig, MissionType, OrchestrationError, OrchestrationStrategy, SubTask,
};

// ── 股票行业适配器 ──────────────────────────────────────────────

/// 股票行业适配器
///
/// 为股票业务场景提供动态编排能力，支持：
/// - 全链路投资分析流水线
/// - 多空辩论决策机制
/// - 领域特定的反思和进化配置
pub struct StockIndustryAdapter {
    industry_id: String,
    industry_name: String,
    reflection_template: ReflectionTemplate,
    evolution_constraints: EvolutionConstraints,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    learning_config: IndustryLearningConfig,
}

impl StockIndustryAdapter {
    /// 创建股票行业适配器
    pub fn new() -> Self {
        Self {
            industry_id: "stock-invest".to_string(),
            industry_name: "股票投资分析".to_string(),
            reflection_template: Self::stock_reflection_template(),
            evolution_constraints: Self::stock_evolution_constraints(),
            acceptance_criteria: Self::stock_acceptance_criteria(),
            learning_config: Self::stock_learning_config(),
        }
    }

    /// 股票分析反思模板
    fn stock_reflection_template() -> ReflectionTemplate {
        ReflectionTemplate {
            id: "stock-invest-default".to_string(),
            name: "股票投资反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.2,
                output_quality: 0.35,
                efficiency: 0.15,
                cost_efficiency: 0.3,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "signal-accuracy".to_string(),
                    name: "信号准确性".to_string(),
                    dimension: "accuracy".to_string(),
                    description: "评估技术指标和信号的准确程度".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "risk-assessment".to_string(),
                    name: "风险评估".to_string(),
                    dimension: "risk".to_string(),
                    description: "评估风险识别和控制措施的有效性".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "decision-quality".to_string(),
                    name: "决策质量".to_string(),
                    dimension: "quality".to_string(),
                    description: "评估投资决策的合理性和可执行性".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次股票分析的信号准确性".to_string(),
                "风险评估是否充分？".to_string(),
                "投资决策是否具有可操作性？".to_string(),
            ],
            ..Default::default()
        }
    }

    /// 股票分析进化约束
    fn stock_evolution_constraints() -> EvolutionConstraints {
        EvolutionConstraints {
            protected_steps: vec![
                ProtectedStep {
                    step_id: "data_fetch".to_string(),
                    reason: "数据获取是分析基础，不可跳过".to_string(),
                },
                ProtectedStep {
                    step_id: "risk_assessment".to_string(),
                    reason: "风险控制是投资核心，必须执行".to_string(),
                },
                ProtectedStep {
                    step_id: "compliance_check".to_string(),
                    reason: "合规性检查不可妥协".to_string(),
                },
            ],
            step_dependencies: vec![
                StepDependency {
                    from: "data_fetch".to_string(),
                    to: "technical_analysis".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "data_fetch".to_string(),
                    to: "fundamental_analysis".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "technical_analysis".to_string(),
                    to: "signal_aggregation".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "fundamental_analysis".to_string(),
                    to: "signal_aggregation".to_string(),
                    dep_type: DependencyType::Soft,
                },
                StepDependency {
                    from: "signal_aggregation".to_string(),
                    to: "risk_assessment".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "risk_assessment".to_string(),
                    to: "decision_generation".to_string(),
                    dep_type: DependencyType::Hard,
                },
            ],
            min_steps: 4,
            max_steps: 20,
            must_follow_order: true,
            forbidden_optimizations: vec![
                ForbiddenOptimization {
                    optimization_type: "skip_data_fetch".to_string(),
                    reason: "不允许跳过数据获取".to_string(),
                },
                ForbiddenOptimization {
                    optimization_type: "skip_risk_assessment".to_string(),
                    reason: "不允许跳过风险评估".to_string(),
                },
                ForbiddenOptimization {
                    optimization_type: "merge_analysis_and_decision".to_string(),
                    reason: "分析和决策必须分离".to_string(),
                },
            ],
            quality_thresholds: QualityThresholds {
                min_accuracy: 0.85,
                min_success_rate: 0.75,
                min_quality_score: 0.7,
            },
        }
    }

    /// 股票分析验收标准
    fn stock_acceptance_criteria() -> Vec<AcceptanceCriterion> {
        vec![
            AcceptanceCriterion {
                id: "si-signal-accuracy".to_string(),
                name: "信号准确性".to_string(),
                description: "技术指标和交易信号准确可靠".to_string(),
                dimension: "accuracy".to_string(),
                threshold: 0.85,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "si-risk-control".to_string(),
                name: "风险控制".to_string(),
                description: "已识别并评估所有主要风险".to_string(),
                dimension: "risk".to_string(),
                threshold: 0.9,
                is_critical: true,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "si-decision-quality".to_string(),
                name: "决策质量".to_string(),
                description: "投资决策合理且可执行".to_string(),
                dimension: "quality".to_string(),
                threshold: 0.75,
                is_critical: true,
                weight: 0.2,
            },
            AcceptanceCriterion {
                id: "si-analysis-depth".to_string(),
                name: "分析深度".to_string(),
                description: "覆盖技术面、基本面、资金面多维度".to_string(),
                dimension: "depth".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.1,
            },
        ]
    }

    /// 股票学习配置
    fn stock_learning_config() -> IndustryLearningConfig {
        IndustryLearningConfig::default()
    }

    /// 检测股票分析任务类型
    fn detect_stock_mission_type(&self, mission: &str) -> MissionType {
        let lower = mission.to_lowercase();

        // 股票特定关键词匹配
        let research_keywords =
            ["分析", "研究", "研报", "调研", "分析", "analysis", "research", "report"];
        let decision_keywords =
            ["买入", "卖出", "持有", "减仓", "加仓", "buy", "sell", "hold", "position"];
        let review_keywords = ["复盘", "审查", "评估", "检查", "review", "evaluate", "check"];

        for kw in &research_keywords {
            if lower.contains(kw) {
                return MissionType::Research;
            }
        }
        for kw in &decision_keywords {
            if lower.contains(kw) {
                return MissionType::Planning;
            }
        }
        for kw in &review_keywords {
            if lower.contains(kw) {
                return MissionType::Review;
            }
        }

        MissionType::Consultation
    }

    /// 构建全链路分析 Pipeline 子图
    fn build_analysis_pipeline(
        &self,
        mission: &str,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        let sub_tasks = vec![
            SubTask::new(
                "data_fetch".to_string(),
                "数据获取".to_string(),
                "获取股票行情、财报、资金流向等数据".to_string(),
                "data_agent".to_string(),
            ),
            SubTask::new(
                "technical_analysis".to_string(),
                "技术面分析".to_string(),
                "技术指标计算、K线形态识别、趋势判断".to_string(),
                "technical_agent".to_string(),
            ),
            SubTask::new(
                "fundamental_analysis".to_string(),
                "基本面分析".to_string(),
                "财务报表分析、估值计算、行业对比".to_string(),
                "fundamental_agent".to_string(),
            ),
            SubTask::new(
                "capital_flow_analysis".to_string(),
                "资金面分析".to_string(),
                "主力资金流向、龙虎榜、融资融券分析".to_string(),
                "capital_agent".to_string(),
            ),
            SubTask::new(
                "signal_aggregation".to_string(),
                "信号聚合".to_string(),
                "整合技术、基本面、资金面信号".to_string(),
                "aggregator_agent".to_string(),
            ),
            SubTask::new(
                "risk_assessment".to_string(),
                "风险评估".to_string(),
                "识别风险因素、计算风险指标".to_string(),
                "risk_agent".to_string(),
            ),
            SubTask::new(
                "decision_generation".to_string(),
                "决策生成".to_string(),
                "生成买入/卖出/持有建议".to_string(),
                "decision_agent".to_string(),
            ),
        ];

        let plan = DecompositionPlan {
            mission: mission.to_string(),
            strategy: OrchestrationStrategy::Pipeline,
            sub_tasks,
            max_parallel: 2, // 技术面和基本面可并行
            max_replans: 3,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let mut generator = DynamicSubGraph::new();
        generator.generate(&plan)
    }

    /// 构建多空辩论 Debate 子图
    fn build_debate_strategy(
        &self,
        mission: &str,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        let sub_tasks = vec![
            SubTask::new(
                "bull_analyst".to_string(),
                "多头分析师".to_string(),
                "从多头角度分析，寻找上涨理由和买入信号".to_string(),
                "bull_agent".to_string(),
            ),
            SubTask::new(
                "bear_analyst".to_string(),
                "空头分析师".to_string(),
                "从空头角度分析，寻找下跌风险和卖出信号".to_string(),
                "bear_agent".to_string(),
            ),
            SubTask::new(
                "arbitrator".to_string(),
                "仲裁者".to_string(),
                "综合多空观点，做出最终裁决".to_string(),
                "arbiter_agent".to_string(),
            ),
        ];

        let plan = DecompositionPlan {
            mission: mission.to_string(),
            strategy: OrchestrationStrategy::Debate,
            sub_tasks,
            max_parallel: 2, // 多空并行
            max_replans: 2,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let mut generator = DynamicSubGraph::new();
        generator.generate(&plan)
    }

    /// 智能选择编排策略
    ///
    /// 根据任务特征自动选择最合适的编排策略
    fn select_strategy(&self, mission: &str) -> OrchestrationStrategy {
        let lower = mission.to_lowercase();

        // 关键词触发辩论策略
        let debate_keywords = ["辩论", "多空", "bull", "bear", "debate", "争议"];
        for kw in debate_keywords {
            if lower.contains(kw) {
                return OrchestrationStrategy::Debate;
            }
        }

        // 默认使用流水线策略
        OrchestrationStrategy::Pipeline
    }
}

impl Default for StockIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IndustryAdapter for StockIndustryAdapter {
    fn industry_id(&self) -> &str {
        &self.industry_id
    }

    fn industry_name(&self) -> &str {
        &self.industry_name
    }

    async fn decompose_mission(
        &self,
        mission: &str,
        _context: &IndustryContext,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        let strategy = self.select_strategy(mission);

        match strategy {
            OrchestrationStrategy::Debate => self.build_debate_strategy(mission),
            _ => self.build_analysis_pipeline(mission),
        }
    }

    fn detect_mission_type(&self, mission: &str) -> MissionType {
        self.detect_stock_mission_type(mission)
    }

    fn reflection_template(&self) -> &ReflectionTemplate {
        &self.reflection_template
    }

    fn evolution_constraints(&self) -> &EvolutionConstraints {
        &self.evolution_constraints
    }

    fn acceptance_criteria(&self) -> &[AcceptanceCriterion] {
        &self.acceptance_criteria
    }

    fn learning_config(&self) -> &IndustryLearningConfig {
        &self.learning_config
    }
}

// ── 工厂函数 ───────────────────────────────────────────────────

/// 创建股票行业适配器
pub fn create_stock_industry_adapter() -> Arc<dyn IndustryAdapter> {
    Arc::new(StockIndustryAdapter::new())
}

/// 注册股票适配器到注册表
pub fn register_stock_adapter(registry: &mut axagent_harness::IndustryAdapterRegistry) {
    registry.register(create_stock_industry_adapter());
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_has_correct_id() {
        let adapter = StockIndustryAdapter::new();
        assert_eq!(adapter.industry_id(), "stock-invest");
        assert_eq!(adapter.industry_name(), "股票投资分析");
    }

    #[test]
    fn detects_research_mission() {
        let adapter = StockIndustryAdapter::new();
        let mission_type = adapter.detect_mission_type("请分析贵州茅台的投资价值");
        assert_eq!(mission_type, MissionType::Research);
    }

    #[test]
    fn detects_planning_mission() {
        let adapter = StockIndustryAdapter::new();
        let mission_type = adapter.detect_mission_type("给出买入或卖出建议");
        assert_eq!(mission_type, MissionType::Planning);
    }

    #[test]
    fn selects_debate_strategy() {
        let adapter = StockIndustryAdapter::new();
        let strategy = adapter.select_strategy("对贵州茅台进行多空辩论");
        assert_eq!(strategy, OrchestrationStrategy::Debate);
    }

    #[test]
    fn selects_pipeline_strategy() {
        let adapter = StockIndustryAdapter::new();
        let strategy = adapter.select_strategy("分析比亚迪的基本面");
        assert_eq!(strategy, OrchestrationStrategy::Pipeline);
    }

    #[tokio::test]
    async fn decomposes_into_pipeline() {
        let adapter = StockIndustryAdapter::new();
        let context = IndustryContext::default();
        let result = adapter.decompose_mission("分析宁德时代的技术面和基本面", &context).await;

        assert!(result.is_ok(), "Pipeline 分解应成功");
        let subgraph = result.unwrap();
        assert!(subgraph.nodes.len() >= 5, "Pipeline 至少应有 5 个节点");
    }

    #[tokio::test]
    async fn decomposes_into_debate() {
        let adapter = StockIndustryAdapter::new();
        let context = IndustryContext::default();
        let result = adapter.decompose_mission("对招商银行进行多空辩论分析", &context).await;

        assert!(result.is_ok(), "Debate 分解应成功");
        let subgraph = result.unwrap();
        assert!(subgraph.nodes.len() >= 3, "Debate 至少应有 3 个节点（多、空、仲裁）");
    }

    #[test]
    fn has_protected_steps() {
        let adapter = StockIndustryAdapter::new();
        let constraints = adapter.evolution_constraints();

        assert_eq!(constraints.protected_steps.len(), 3);
        assert!(constraints.protected_steps.iter().any(|s| s.step_id == "data_fetch"));
        assert!(constraints.protected_steps.iter().any(|s| s.step_id == "risk_assessment"));
    }

    #[test]
    fn has_acceptance_criteria() {
        let adapter = StockIndustryAdapter::new();
        let criteria = adapter.acceptance_criteria();

        assert_eq!(criteria.len(), 4);
        // 信号准确性是关键标准
        let signal_criterion = criteria.iter().find(|c| c.id == "si-signal-accuracy");
        assert!(signal_criterion.is_some());
        assert!(signal_criterion.unwrap().is_critical);
    }

    #[test]
    fn has_reflection_checkpoints() {
        let adapter = StockIndustryAdapter::new();
        let template = adapter.reflection_template();

        assert_eq!(template.checkpoints.len(), 3);
        assert_eq!(template.checkpoints[0].dimension, "accuracy");
    }
}
