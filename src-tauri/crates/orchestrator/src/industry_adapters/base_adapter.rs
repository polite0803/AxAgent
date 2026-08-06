// SPDX-License-Identifier: AGPL-3.0-only

//! 基础行业适配器实现
//!
//! 提供通用的 IndustryAdapter 实现，包含：
//! - 动态任务分解的基础逻辑
//! - 任务类型检测的关键词匹配
//! - 反思模板、进化约束、验收标准的配置注入
//!
//! 9 个行业适配器均继承此基础实现，只需提供行业特定配置。

use async_trait::async_trait;

use crate::dynamic_subgraph::{DynamicSubGraph, GeneratedSubGraph};
use crate::industry_adapters::{
    IndustryAdapter,
    types::{
        AcceptanceCriterion, EvolutionConstraints, IndustryContext, IndustryLearningConfig,
        MissionType, PresetWorkflowStep, ReflectionTemplate,
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

    /// 从行业包配置（`learning.yaml` 的 `adapter:` 段，已转 JSON）构造适配器。
    ///
    /// P0-1-A：消灭 Rust 硬编码 9 行业配置。可解析 `reflection_template` /
    /// `evolution_constraints` / `acceptance_criteria` 三段（字段对齐
    /// `types.rs` serde），缺失段保持默认值；`adapter` 段整体缺失（旧包）时
    /// 返回默认适配器（向后兼容）。mission_keywords 用默认映射（9 行业原配置
    /// 均未自定义），preset_steps 留 P0-1-B 迁移。
    pub fn from_config_json(
        industry_id: &str,
        industry_name: &str,
        cfg: &serde_json::Value,
    ) -> Result<Self, String> {
        let mut adapter = Self::new(industry_id, industry_name);
        if let Some(rt) = cfg.get("reflection_template") {
            let template: ReflectionTemplate = serde_json::from_value(rt.clone())
                .map_err(|e| format!("reflection_template 解析失败: {e}"))?;
            adapter = adapter.with_reflection_template(template);
        }
        if let Some(ec) = cfg.get("evolution_constraints") {
            let constraints: EvolutionConstraints = serde_json::from_value(ec.clone())
                .map_err(|e| format!("evolution_constraints 解析失败: {e}"))?;
            adapter = adapter.with_evolution_constraints(constraints);
        }
        if let Some(ac) = cfg.get("acceptance_criteria") {
            let criteria: Vec<AcceptanceCriterion> = serde_json::from_value(ac.clone())
                .map_err(|e| format!("acceptance_criteria 解析失败: {e}"))?;
            adapter = adapter.with_acceptance_criteria(criteria);
        }
        Ok(adapter)
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
