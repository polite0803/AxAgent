// SPDX-License-Identifier: AGPL-3.0-only

//! 基础行业适配器实现
//!
//! 提供通用的 IndustryAdapter 实现，包含：
//! - 动态任务分解的基础逻辑
//! - 任务类型检测的关键词匹配
//! - 反思模板、进化约束、验收标准的配置注入
//!
//! 9 个行业适配器均继承此基础实现，只需提供行业特定配置。

use std::sync::Arc;

use async_trait::async_trait;

use crate::dynamic_subgraph::{DynamicSubGraph, GeneratedSubGraph};
use crate::industry_adapters::{
    IndustryAdapter,
    types::{
        AcceptanceCriterion, DependencyType, EvolutionConstraints, ForbiddenOptimization,
        IndustryContext, IndustryLearningConfig, MissionType, PresetWorkflowStep, ProtectedStep,
        QualityThresholds, QualityWeights, ReflectionCheckpoint, ReflectionTemplate,
        StepDependency,
    },
};
use crate::types::{DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask};

/// 基础行业适配器
///
/// 通用实现，通过注入行业特定配置来适配不同行业。
pub struct BaseIndustryAdapter {
    industry_id: String,
    industry_name: String,
    reflection_template: ReflectionTemplate,
    evolution_constraints: EvolutionConstraints,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    learning_config: IndustryLearningConfig,
    /// 预设工作流步骤
    preset_steps: Vec<PresetWorkflowStep>,
    /// 任务类型关键词映射
    mission_keywords: Vec<(MissionType, Vec<String>)>,
}

impl BaseIndustryAdapter {
    pub fn new(industry_id: impl Into<String>, industry_name: impl Into<String>) -> Self {
        let id = industry_id.into();
        Self {
            industry_id: id.clone(),
            industry_name: industry_name.into(),
            reflection_template: ReflectionTemplate::default(),
            evolution_constraints: EvolutionConstraints::default(),
            acceptance_criteria: Vec::new(),
            learning_config: IndustryLearningConfig::default(),
            preset_steps: Vec::new(),
            mission_keywords: Self::default_keywords(),
        }
    }

    pub fn with_reflection_template(mut self, template: ReflectionTemplate) -> Self {
        self.reflection_template = template;
        self
    }

    pub fn with_evolution_constraints(mut self, constraints: EvolutionConstraints) -> Self {
        self.evolution_constraints = constraints;
        self
    }

    pub fn with_acceptance_criteria(mut self, criteria: Vec<AcceptanceCriterion>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    pub fn with_learning_config(mut self, config: IndustryLearningConfig) -> Self {
        self.learning_config = config;
        self
    }

    /// 注入预设工作流步骤
    pub fn with_preset_steps(mut self, steps: Vec<PresetWorkflowStep>) -> Self {
        self.preset_steps = steps;
        self
    }

    pub fn with_mission_keywords(mut self, keywords: Vec<(MissionType, Vec<String>)>) -> Self {
        self.mission_keywords = keywords;
        self
    }

    /// 默认的任务类型关键词映射
    fn default_keywords() -> Vec<(MissionType, Vec<String>)> {
        vec![
            (
                MissionType::Research,
                vec![
                    "调研",
                    "研究",
                    "分析",
                    "论文",
                    "benchmark",
                    "评测",
                    "对比",
                    "survey",
                    "research",
                    "analysis",
                    "investigate",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Generation,
                vec![
                    "生成", "创建", "写", "设计", "撰写", "generate", "create", "write", "design",
                    "produce",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Review,
                vec![
                    "审查", "评估", "检查", "审核", "review", "evaluate", "check", "audit",
                    "inspect",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Fix,
                vec![
                    "修复", "优化", "重构", "bug", "fix", "refactor", "optimize", "improve",
                    "resolve",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Planning,
                vec![
                    "规划",
                    "计划",
                    "架构",
                    "strategy",
                    "plan",
                    "design",
                    "architecture",
                    "roadmap",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Monitoring,
                vec![
                    "监控",
                    "运维",
                    "部署",
                    "monitor",
                    "deploy",
                    "ops",
                    "maintenance",
                    "operations",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            (
                MissionType::Reporting,
                vec!["报告", "总结", "输出", "report", "summary", "document", "output", "deliver"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            ),
            (
                MissionType::Consultation,
                vec!["咨询", "建议", "讨论", "consult", "advise", "discuss", "help", "assist"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            ),
        ]
    }

    /// 基于关键词匹配检测任务类型
    fn detect_mission_type_by_keywords(&self, mission: &str) -> MissionType {
        let lower_mission = mission.to_lowercase();

        for (mission_type, keywords) in &self.mission_keywords {
            for keyword in keywords {
                if lower_mission.contains(&keyword.to_lowercase()) {
                    return *mission_type;
                }
            }
        }

        // 默认返回 Consultation
        MissionType::Consultation
    }

    /// 使用预设步骤分解任务
    ///
    /// 根据行业预设步骤模板生成初始工作流：
    /// 1. 筛选骨架步骤（必须包含）
    /// 2. 根据任务类型和任务描述动态添加可选步骤
    /// 3. 按依赖关系排序生成子任务列表
    /// 4. 传递多智能体配置和并行支持
    fn decompose_with_preset_steps(
        &self,
        mission: &str,
        mission_type: MissionType,
    ) -> Result<GeneratedSubGraph, OrchestrationError> {
        let skeleton_steps: Vec<&PresetWorkflowStep> =
            self.preset_steps.iter().filter(|s| s.is_skeleton).collect();

        // 根据任务类型和任务描述动态选择可选步骤
        let optional_steps = self.select_optional_steps_dynamic(mission_type, mission);

        // 合并步骤
        let mut all_steps = skeleton_steps.clone();
        all_steps.extend(optional_steps.iter());

        // 按 order 排序
        all_steps.sort_by_key(|s| s.order);

        // 生成子任务（传递多智能体配置和并行支持）
        let sub_tasks: Vec<SubTask> = all_steps
            .iter()
            .map(|step| {
                let task_id = format!("{}_{}_{}", self.industry_id, step.id, uuid::Uuid::new_v4());
                let goal = if step.is_skeleton {
                    format!("[骨架] {}", step.goal)
                } else {
                    format!("[可选] {}", step.goal)
                };
                let mut task = SubTask::new(task_id, step.name.clone(), goal, step.role.clone());

                // 传递依赖
                task.dependencies = step.depends_on.clone();

                // 传递多智能体配置
                if step.multi_agent
                    && let Some(ref mode) = step.coordination_mode
                {
                    task = task.with_multi_agent(mode, step.max_rounds);
                }

                // 传递并行支持
                if step.parallel_supported {
                    task = task.with_parallel();
                }

                task
            })
            .collect();

        // 计算最大并行数（根据支持并行的步骤数量动态计算）
        let parallel_count = all_steps.iter().filter(|s| s.parallel_supported).count();
        let max_parallel = if parallel_count > 0 {
            parallel_count as u32
        } else {
            1
        };

        let plan = DecompositionPlan {
            mission: mission.to_string(),
            strategy: OrchestrationStrategy::Ordered,
            sub_tasks,
            max_parallel,
            max_replans: 5,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let mut generator = DynamicSubGraph::new();
        generator.generate(&plan)
    }

    /// 根据任务类型选择可选步骤（支持动态选择）
    ///
    /// 选择策略：
    /// 1. 基础规则：根据 MissionType 映射默认步骤
    /// 2. 动态调整：根据任务描述关键词和复杂度动态增减步骤
    /// 3. 智能推荐：基于历史执行模式（预留接口）
    fn select_optional_steps_by_mission_type(
        &self,
        mission_type: MissionType,
    ) -> Vec<&PresetWorkflowStep> {
        let optional_steps: Vec<&PresetWorkflowStep> =
            self.preset_steps.iter().filter(|s| s.is_optional).collect();

        // 1. 基础规则：根据任务类型选择默认步骤
        match mission_type {
            MissionType::Generation => {
                // 生成类任务：添加文档编写、监控配置
                let steps: Vec<&str> = vec!["documentation", "monitoring_setup"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Review | MissionType::Fix => {
                // 审查/修复类任务：添加安全审计、性能优化
                let steps: Vec<&str> = vec!["security_audit", "performance_opt"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Planning => {
                // 规划类任务：添加文档编写
                let steps: Vec<&str> = vec!["documentation"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Monitoring => {
                // 监控类任务：添加监控配置
                let steps: Vec<&str> = vec!["monitoring_setup"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Research => {
                // 研究类任务：添加文档编写（用于记录研究成果）
                let steps: Vec<&str> = vec!["documentation"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Reporting => {
                // 报告类任务：添加文档编写
                let steps: Vec<&str> = vec!["documentation"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
            MissionType::Consultation => {
                // 咨询类任务：根据需要添加性能优化
                let steps: Vec<&str> = vec!["performance_opt"];
                self.filter_steps_by_ids(&optional_steps, &steps)
            },
        }
    }

    /// 根据任务类型和任务描述动态选择可选步骤
    ///
    /// 增强版：在基础规则之上叠加关键词匹配和复杂度评估
    pub fn select_optional_steps_dynamic(
        &self,
        mission_type: MissionType,
        mission: &str,
    ) -> Vec<&PresetWorkflowStep> {
        let optional_steps: Vec<&PresetWorkflowStep> =
            self.preset_steps.iter().filter(|s| s.is_optional).collect();

        // 1. 获取基础步骤
        let base_steps = self.select_optional_steps_by_mission_type(mission_type);
        let base_ids: Vec<&str> = base_steps.iter().map(|s| s.id.as_str()).collect();

        // 2. 关键词分析：检测任务描述中的关键信号
        let task_lower = mission.to_lowercase();
        let signals = Self::detect_task_signals(&task_lower);

        // 3. 基于信号动态添加额外步骤
        let mut selected_ids = base_ids;

        // 安全相关信号 → 添加安全审计
        if (signals.contains(&"security")
            || signals.contains(&"vulnerability")
            || signals.contains(&"安全"))
            && !selected_ids.contains(&"security_audit")
        {
            selected_ids.push("security_audit");
        }

        // 性能相关信号 → 添加性能优化
        if (signals.contains(&"performance")
            || signals.contains(&"slow")
            || signals.contains(&"性能")
            || signals.contains(&"优化"))
            && !selected_ids.contains(&"performance_opt")
        {
            selected_ids.push("performance_opt");
        }

        // 文档相关信号 → 添加文档编写
        if (signals.contains(&"documentation")
            || signals.contains(&"doc")
            || signals.contains(&"文档")
            || signals.contains(&"api"))
            && !selected_ids.contains(&"documentation")
        {
            selected_ids.push("documentation");
        }

        // 部署相关信号 → 添加监控配置
        if (signals.contains(&"deploy")
            || signals.contains(&"release")
            || signals.contains(&"部署")
            || signals.contains(&"上线"))
            && !selected_ids.contains(&"monitoring_setup")
        {
            selected_ids.push("monitoring_setup");
        }

        // 4. 复杂度评估：任务越长越复杂，需要更多可选步骤
        let complexity = Self::assess_complexity(mission, &signals);
        if complexity >= 3 {
            // 高复杂度：添加所有可选步骤
            selected_ids = optional_steps.iter().map(|s| s.id.as_str()).collect();
        } else if complexity >= 2 {
            // 中复杂度：确保至少有安全审计和性能优化
            if !selected_ids.contains(&"security_audit") {
                selected_ids.push("security_audit");
            }
            if !selected_ids.contains(&"performance_opt") {
                selected_ids.push("performance_opt");
            }
        }

        // 5. 过滤并返回选中的步骤
        self.filter_steps_by_ids(&optional_steps, &selected_ids)
    }

    /// 根据 ID 列表过滤步骤
    fn filter_steps_by_ids<'a>(
        &self,
        steps: &[&'a PresetWorkflowStep],
        ids: &[&str],
    ) -> Vec<&'a PresetWorkflowStep> {
        steps.iter().filter(|s| ids.contains(&s.id.as_str())).cloned().collect()
    }

    /// 检测任务描述中的信号关键词
    fn detect_task_signals(task: &str) -> Vec<&str> {
        let signal_patterns: Vec<(&str, &[&str])> = vec![
            ("security", &["security", "safe", "protect", "安全", "漏洞", "防护"]),
            ("performance", &["performance", "fast", "slow", "speed", "性能", "优化", "慢"]),
            ("documentation", &["documentation", "doc", "readme", "文档", "api", "指南"]),
            ("deployment", &["deploy", "release", "ship", "部署", "上线", "发布"]),
            ("refactoring", &["refactor", "cleanup", "重构", "整理"]),
            ("testing", &["test", "coverage", "测试", "用例"]),
            ("integration", &["integrate", "connect", "集成", "对接"]),
        ];

        let mut signals = Vec::new();
        for (signal, keywords) in signal_patterns {
            if keywords.iter().any(|kw| task.contains(kw)) {
                signals.push(signal);
            }
        }

        signals
    }

    /// 评估任务复杂度
    fn assess_complexity(task: &str, signals: &[&str]) -> u32 {
        let mut complexity = 1;

        // 基于任务长度
        let char_count = task.chars().count();
        if char_count > 100 {
            complexity += 1;
        }
        if char_count > 200 {
            complexity += 1;
        }

        // 基于信号数量
        let signal_count = signals.len() as u32;
        if signal_count >= 3 {
            complexity += 1;
        }
        if signal_count >= 5 {
            complexity += 1;
        }

        complexity.min(5) // 最高 5 级
    }
}

#[async_trait]
impl IndustryAdapter for BaseIndustryAdapter {
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
        let mission_type = self.detect_mission_type_by_keywords(mission);

        // 如果有预设步骤，使用预设步骤生成初始工作流
        if !self.preset_steps.is_empty() {
            return self.decompose_with_preset_steps(mission, mission_type);
        }

        // 基础实现：创建一个包含单个任务的简单分解计划
        let sub_task_id = format!("{}_{}", self.industry_id, uuid::Uuid::new_v4());

        let plan = DecompositionPlan {
            mission: mission.to_string(),
            strategy: OrchestrationStrategy::Ordered,
            sub_tasks: vec![SubTask::new(
                sub_task_id,
                format!("{}任务", mission_type.as_str()),
                mission.to_string(),
                "worker".to_string(),
            )],
            max_parallel: 1,
            max_replans: 3,
            replan_count: 0,
            created_at: chrono::Utc::now(),
        };

        let mut generator = DynamicSubGraph::new();
        generator.generate(&plan)
    }

    fn preset_steps(&self) -> Vec<PresetWorkflowStep> {
        self.preset_steps.clone()
    }

    fn detect_mission_type(&self, mission: &str) -> MissionType {
        self.detect_mission_type_by_keywords(mission)
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

// ── 9 个行业适配器工厂函数 ──────────────────────────────────────────

/// 获取所有行业适配器
pub fn create_all_adapters() -> Vec<Arc<dyn IndustryAdapter>> {
    vec![
        Arc::new(create_ai_research_adapter()),
        Arc::new(create_software_dev_adapter()),
        Arc::new(create_finance_invest_adapter()),
        Arc::new(create_sales_growth_adapter()),
        Arc::new(create_content_media_adapter()),
        Arc::new(create_industry_consulting_adapter()),
        Arc::new(create_accounting_adapter()),
        Arc::new(create_ecommerce_adapter()),
        Arc::new(create_education_adapter()),
    ]
}

/// AI 科技研究适配器
pub fn create_ai_research_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("ai-research", "AI 科技研究报告")
        .with_reflection_template(ReflectionTemplate {
            id: "ai-research-default".to_string(),
            name: "AI 研究反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.3,
                output_quality: 0.3,
                efficiency: 0.2,
                cost_efficiency: 0.2,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "accuracy".to_string(),
                    name: "研究准确性".to_string(),
                    dimension: "accuracy".to_string(),
                    description: "评估论文分析和技术解读的准确性".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "coverage".to_string(),
                    name: "调研覆盖度".to_string(),
                    dimension: "coverage".to_string(),
                    description: "评估技术调研的广度和深度".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "actionability".to_string(),
                    name: "可操作性".to_string(),
                    dimension: "actionability".to_string(),
                    description: "评估研究结论的实践指导价值".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次 AI 研究的准确性和深度".to_string(),
                "本次研究是否覆盖了关键技术方向？".to_string(),
                "研究结论是否具有实际应用价值？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints {
            protected_steps: vec![],
            step_dependencies: vec![],
            min_steps: 2,
            max_steps: 15,
            must_follow_order: false,
            forbidden_optimizations: vec![],
            quality_thresholds: QualityThresholds {
                min_accuracy: 0.8,
                min_success_rate: 0.7,
                min_quality_score: 0.6,
            },
        })
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "ai-research-accuracy".to_string(),
                name: "研究准确性".to_string(),
                description: "论文和技术解读是否准确".to_string(),
                dimension: "accuracy".to_string(),
                threshold: 0.85,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "ai-research-coverage".to_string(),
                name: "研究覆盖度".to_string(),
                description: "是否覆盖指定的研究范围".to_string(),
                dimension: "coverage".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "ai-research-actionable".to_string(),
                name: "可操作性".to_string(),
                description: "结论是否可以指导实际应用".to_string(),
                dimension: "actionability".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 软件开发适配器
pub fn create_software_dev_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("software-dev", "软件开发完整流程")
        .with_reflection_template(ReflectionTemplate {
            id: "software-dev-default".to_string(),
            name: "软件开发反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.25,
                output_quality: 0.3,
                efficiency: 0.2,
                cost_efficiency: 0.25,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "coding".to_string(),
                    name: "代码质量".to_string(),
                    dimension: "quality".to_string(),
                    description: "评估代码的可读性、健壮性和可维护性".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "testing".to_string(),
                    name: "测试覆盖率".to_string(),
                    dimension: "coverage".to_string(),
                    description: "评估单元测试和集成测试的覆盖情况".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "documentation".to_string(),
                    name: "文档完整性".to_string(),
                    dimension: "documentation".to_string(),
                    description: "评估技术文档和 API 文档的完整度".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次代码实现的质量和规范".to_string(),
                "代码是否通过了必要的测试？".to_string(),
                "技术文档是否完整清晰？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints {
            protected_steps: vec![
                ProtectedStep {
                    step_id: "code_review".to_string(),
                    reason: "代码质量是核心竞争力".to_string(),
                },
                ProtectedStep {
                    step_id: "security_audit".to_string(),
                    reason: "安全性不可妥协".to_string(),
                },
                ProtectedStep {
                    step_id: "testing".to_string(),
                    reason: "测试是质量保障".to_string(),
                },
            ],
            step_dependencies: vec![
                StepDependency {
                    from: "requirements".to_string(),
                    to: "architecture".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "architecture".to_string(),
                    to: "coding".to_string(),
                    dep_type: DependencyType::Hard,
                },
                StepDependency {
                    from: "coding".to_string(),
                    to: "code_review".to_string(),
                    dep_type: DependencyType::Soft,
                },
            ],
            min_steps: 5,
            max_steps: 25,
            must_follow_order: true,
            forbidden_optimizations: vec![
                ForbiddenOptimization {
                    optimization_type: "skip_testing".to_string(),
                    reason: "不允许跳过测试阶段".to_string(),
                },
                ForbiddenOptimization {
                    optimization_type: "merge_review_and_coding".to_string(),
                    reason: "实现和评估必须分离".to_string(),
                },
            ],
            quality_thresholds: QualityThresholds {
                min_accuracy: 0.8,
                min_success_rate: 0.7,
                min_quality_score: 0.6,
            },
        })
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "sd-code-quality".to_string(),
                name: "代码质量".to_string(),
                description: "代码符合编码规范，无明显 bug".to_string(),
                dimension: "quality".to_string(),
                threshold: 0.8,
                is_critical: true,
                weight: 0.35,
            },
            AcceptanceCriterion {
                id: "sd-test-coverage".to_string(),
                name: "测试覆盖率".to_string(),
                description: "核心模块测试覆盖率不低于 70%".to_string(),
                dimension: "coverage".to_string(),
                threshold: 0.7,
                is_critical: true,
                weight: 0.25,
            },
            AcceptanceCriterion {
                id: "sd-security".to_string(),
                name: "安全性".to_string(),
                description: "无高危漏洞，符合 OWASP Top 10".to_string(),
                dimension: "security".to_string(),
                threshold: 0.9,
                is_critical: true,
                weight: 0.2,
            },
            AcceptanceCriterion {
                id: "sd-documentation".to_string(),
                name: "文档完整性".to_string(),
                description: "设计文档和 API 文档齐全".to_string(),
                dimension: "documentation".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.2,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
        .with_preset_steps(crate::industry_adapters::types::get_software_dev_preset_steps())
}

/// 金融投资适配器
pub fn create_finance_invest_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("finance-invest", "金融投资分析")
        .with_reflection_template(ReflectionTemplate {
            id: "finance-invest-default".to_string(),
            name: "金融投资反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.2,
                output_quality: 0.35,
                efficiency: 0.15,
                cost_efficiency: 0.3,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "analysis-accuracy".to_string(),
                    name: "分析准确性".to_string(),
                    dimension: "accuracy".to_string(),
                    description: "评估财务分析和投资建议的准确性".to_string(),
                    weight: 0.5,
                },
                ReflectionCheckpoint {
                    id: "risk-assessment".to_string(),
                    name: "风险评估".to_string(),
                    dimension: "risk".to_string(),
                    description: "评估风险识别和控制的有效性".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "actionability".to_string(),
                    name: "可操作性".to_string(),
                    dimension: "actionability".to_string(),
                    description: "评估建议的可执行性".to_string(),
                    weight: 0.2,
                },
            ],
            prompts: vec![
                "请评估本次财务分析的准确性".to_string(),
                "投资建议的风险是否充分考虑？".to_string(),
                "建议是否具有可操作性？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints {
            protected_steps: vec![
                ProtectedStep {
                    step_id: "risk_assessment".to_string(),
                    reason: "风险控制是金融核心".to_string(),
                },
                ProtectedStep {
                    step_id: "compliance_check".to_string(),
                    reason: "合规性要求".to_string(),
                },
            ],
            step_dependencies: vec![],
            min_steps: 3,
            max_steps: 20,
            must_follow_order: false,
            forbidden_optimizations: vec![ForbiddenOptimization {
                optimization_type: "skip_risk_assessment".to_string(),
                reason: "不允许跳过风险评估".to_string(),
            }],
            quality_thresholds: QualityThresholds {
                min_accuracy: 0.85,
                min_success_rate: 0.75,
                min_quality_score: 0.7,
            },
        })
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "fi-analysis-accuracy".to_string(),
                name: "分析准确性".to_string(),
                description: "财务分析和投资建议准确无误".to_string(),
                dimension: "accuracy".to_string(),
                threshold: 0.85,
                is_critical: true,
                weight: 0.5,
            },
            AcceptanceCriterion {
                id: "fi-risk-control".to_string(),
                name: "风险控制".to_string(),
                description: "已识别并评估主要风险".to_string(),
                dimension: "risk".to_string(),
                threshold: 0.9,
                is_critical: true,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "fi-actionable".to_string(),
                name: "可操作性".to_string(),
                description: "建议清晰可执行".to_string(),
                dimension: "actionability".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.2,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 销售增长适配器
pub fn create_sales_growth_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("sales-growth", "销售增长流程")
        .with_reflection_template(ReflectionTemplate {
            id: "sales-growth-default".to_string(),
            name: "销售增长反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.25,
                output_quality: 0.25,
                efficiency: 0.3,
                cost_efficiency: 0.2,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "conversion".to_string(),
                    name: "转化率".to_string(),
                    dimension: "conversion".to_string(),
                    description: "评估线索到成交的转化效率".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "lead-quality".to_string(),
                    name: "线索质量".to_string(),
                    dimension: "quality".to_string(),
                    description: "评估目标客户匹配度".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "response-speed".to_string(),
                    name: "响应速度".to_string(),
                    dimension: "speed".to_string(),
                    description: "评估跟进和响应的及时性".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次销售活动的转化效果".to_string(),
                "线索质量是否达标？".to_string(),
                "跟进响应是否及时？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints::default())
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "sg-conversion".to_string(),
                name: "转化率".to_string(),
                description: "线索转化率达到目标".to_string(),
                dimension: "conversion".to_string(),
                threshold: 0.7,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "sg-lead-quality".to_string(),
                name: "线索质量".to_string(),
                description: "目标客户符合 ICP".to_string(),
                dimension: "quality".to_string(),
                threshold: 0.8,
                is_critical: false,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "sg-response-time".to_string(),
                name: "响应时效".to_string(),
                description: "首次响应时间不超过 24 小时".to_string(),
                dimension: "speed".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 内容媒体适配器
pub fn create_content_media_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("content-media", "内容营销流程")
        .with_reflection_template(ReflectionTemplate {
            id: "content-media-default".to_string(),
            name: "内容媒体反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.2,
                output_quality: 0.35,
                efficiency: 0.2,
                cost_efficiency: 0.25,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "content-quality".to_string(),
                    name: "内容质量".to_string(),
                    dimension: "quality".to_string(),
                    description: "评估内容的原创性和价值".to_string(),
                    weight: 0.5,
                },
                ReflectionCheckpoint {
                    id: "seo".to_string(),
                    name: "SEO 效果".to_string(),
                    dimension: "seo".to_string(),
                    description: "评估搜索引擎优化效果".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "engagement".to_string(),
                    name: "用户互动".to_string(),
                    dimension: "engagement".to_string(),
                    description: "评估内容引发的用户互动".to_string(),
                    weight: 0.2,
                },
            ],
            prompts: vec![
                "请评估本次内容创作的质量".to_string(),
                "SEO 优化是否到位？".to_string(),
                "内容能引发用户互动吗？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints::default())
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "cm-content-quality".to_string(),
                name: "内容质量".to_string(),
                description: "内容原创且有价值".to_string(),
                dimension: "quality".to_string(),
                threshold: 0.8,
                is_critical: true,
                weight: 0.5,
            },
            AcceptanceCriterion {
                id: "cm-seo".to_string(),
                name: "SEO 优化".to_string(),
                description: "关键词和元数据优化到位".to_string(),
                dimension: "seo".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "cm-engagement".to_string(),
                name: "互动潜力".to_string(),
                description: "标题和开头能吸引读者".to_string(),
                dimension: "engagement".to_string(),
                threshold: 0.6,
                is_critical: false,
                weight: 0.2,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 行业咨询适配器
pub fn create_industry_consulting_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("industry-consulting", "行业咨询流程")
        .with_reflection_template(ReflectionTemplate {
            id: "industry-consulting-default".to_string(),
            name: "行业咨询反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.25,
                output_quality: 0.3,
                efficiency: 0.2,
                cost_efficiency: 0.25,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "insight-depth".to_string(),
                    name: "洞察深度".to_string(),
                    dimension: "depth".to_string(),
                    description: "分析是否深入到业务本质".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "recommendation".to_string(),
                    name: "建议质量".to_string(),
                    dimension: "quality".to_string(),
                    description: "建议是否具体可执行".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "data-support".to_string(),
                    name: "数据支撑".to_string(),
                    dimension: "data".to_string(),
                    description: "结论是否有充分数据支撑".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次咨询分析的深度".to_string(),
                "建议是否具体可行？".to_string(),
                "结论是否有数据支撑？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints::default())
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "ic-insight-depth".to_string(),
                name: "洞察深度".to_string(),
                description: "分析深入业务本质".to_string(),
                dimension: "depth".to_string(),
                threshold: 0.8,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "ic-recommendation".to_string(),
                name: "建议质量".to_string(),
                description: "建议具体可执行".to_string(),
                dimension: "quality".to_string(),
                threshold: 0.75,
                is_critical: true,
                weight: 0.35,
            },
            AcceptanceCriterion {
                id: "ic-data-support".to_string(),
                name: "数据支撑".to_string(),
                description: "结论有充分数据支撑".to_string(),
                dimension: "data".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.25,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 会计适配器
pub fn create_accounting_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("accounting", "会计流程")
        .with_reflection_template(ReflectionTemplate {
            id: "accounting-default".to_string(),
            name: "会计反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.3,
                output_quality: 0.3,
                efficiency: 0.2,
                cost_efficiency: 0.2,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "accuracy".to_string(),
                    name: "数据准确性".to_string(),
                    dimension: "accuracy".to_string(),
                    description: "评估财务数据的准确性".to_string(),
                    weight: 0.5,
                },
                ReflectionCheckpoint {
                    id: "compliance".to_string(),
                    name: "合规性".to_string(),
                    dimension: "compliance".to_string(),
                    description: "评估是否符合会计准则".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "efficiency".to_string(),
                    name: "效率".to_string(),
                    dimension: "efficiency".to_string(),
                    description: "评估流程执行效率".to_string(),
                    weight: 0.2,
                },
            ],
            prompts: vec![
                "请评估本次财务处理的准确性".to_string(),
                "是否符合会计准则？".to_string(),
                "流程是否高效？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints {
            protected_steps: vec![ProtectedStep {
                step_id: "compliance_check".to_string(),
                reason: "会计合规性是硬性要求".to_string(),
            }],
            step_dependencies: vec![],
            min_steps: 2,
            max_steps: 15,
            must_follow_order: false,
            forbidden_optimizations: vec![ForbiddenOptimization {
                optimization_type: "skip_compliance".to_string(),
                reason: "不允许跳过合规检查".to_string(),
            }],
            quality_thresholds: QualityThresholds {
                min_accuracy: 0.95,
                min_success_rate: 0.9,
                min_quality_score: 0.8,
            },
        })
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "ac-accuracy".to_string(),
                name: "数据准确性".to_string(),
                description: "财务数据准确无误".to_string(),
                dimension: "accuracy".to_string(),
                threshold: 0.95,
                is_critical: true,
                weight: 0.5,
            },
            AcceptanceCriterion {
                id: "ac-compliance".to_string(),
                name: "合规性".to_string(),
                description: "符合会计准则和法规".to_string(),
                dimension: "compliance".to_string(),
                threshold: 0.95,
                is_critical: true,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "ac-efficiency".to_string(),
                name: "效率".to_string(),
                description: "流程执行高效".to_string(),
                dimension: "efficiency".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.2,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 电商适配器
pub fn create_ecommerce_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("ecommerce", "电商运营流程")
        .with_reflection_template(ReflectionTemplate {
            id: "ecommerce-default".to_string(),
            name: "电商反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.25,
                output_quality: 0.25,
                efficiency: 0.25,
                cost_efficiency: 0.25,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "conversion-rate".to_string(),
                    name: "转化率".to_string(),
                    dimension: "conversion".to_string(),
                    description: "评估访客到客户的转化".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "customer-satisfaction".to_string(),
                    name: "客户满意度".to_string(),
                    dimension: "satisfaction".to_string(),
                    description: "评估客户购买体验".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "profit-margin".to_string(),
                    name: "利润率".to_string(),
                    dimension: "profitability".to_string(),
                    description: "评估盈利能力".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次电商活动的转化率".to_string(),
                "客户满意度如何？".to_string(),
                "利润率是否达标？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints::default())
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "ec-conversion".to_string(),
                name: "转化率".to_string(),
                description: "页面转化率达到目标".to_string(),
                dimension: "conversion".to_string(),
                threshold: 0.6,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "ec-satisfaction".to_string(),
                name: "客户满意度".to_string(),
                description: "客户体验良好".to_string(),
                dimension: "satisfaction".to_string(),
                threshold: 0.8,
                is_critical: false,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "ec-profit".to_string(),
                name: "利润率".to_string(),
                description: "保持合理利润率".to_string(),
                dimension: "profitability".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}

/// 教育适配器
pub fn create_education_adapter() -> BaseIndustryAdapter {
    BaseIndustryAdapter::new("education", "教育流程")
        .with_reflection_template(ReflectionTemplate {
            id: "education-default".to_string(),
            name: "教育反思模板".to_string(),
            quality_weights: QualityWeights {
                task_completion: 0.3,
                output_quality: 0.25,
                efficiency: 0.2,
                cost_efficiency: 0.25,
            },
            checkpoints: vec![
                ReflectionCheckpoint {
                    id: "learning-outcomes".to_string(),
                    name: "学习成果".to_string(),
                    dimension: "outcomes".to_string(),
                    description: "评估学员知识掌握情况".to_string(),
                    weight: 0.4,
                },
                ReflectionCheckpoint {
                    id: "engagement".to_string(),
                    name: "参与度".to_string(),
                    dimension: "engagement".to_string(),
                    description: "评估学员课堂参与度".to_string(),
                    weight: 0.3,
                },
                ReflectionCheckpoint {
                    id: "practical-application".to_string(),
                    name: "实践应用".to_string(),
                    dimension: "application".to_string(),
                    description: "评估知识转化为实践能力".to_string(),
                    weight: 0.3,
                },
            ],
            prompts: vec![
                "请评估本次教学的学习成果".to_string(),
                "学员参与度如何？".to_string(),
                "知识能否转化为实践能力？".to_string(),
            ],
            ..Default::default()
        })
        .with_evolution_constraints(EvolutionConstraints::default())
        .with_acceptance_criteria(vec![
            AcceptanceCriterion {
                id: "ed-outcomes".to_string(),
                name: "学习成果".to_string(),
                description: "学员掌握核心知识点".to_string(),
                dimension: "outcomes".to_string(),
                threshold: 0.75,
                is_critical: true,
                weight: 0.4,
            },
            AcceptanceCriterion {
                id: "ed-engagement".to_string(),
                name: "参与度".to_string(),
                description: "学员积极参与互动".to_string(),
                dimension: "engagement".to_string(),
                threshold: 0.7,
                is_critical: false,
                weight: 0.3,
            },
            AcceptanceCriterion {
                id: "ed-application".to_string(),
                name: "实践应用".to_string(),
                description: "知识可应用于实际场景".to_string(),
                dimension: "application".to_string(),
                threshold: 0.65,
                is_critical: false,
                weight: 0.3,
            },
        ])
        .with_learning_config(IndustryLearningConfig::default())
}
