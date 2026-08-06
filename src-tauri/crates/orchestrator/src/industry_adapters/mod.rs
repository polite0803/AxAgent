// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业适配器模块
//!
//! 为 9 大垂直行业提供统一的接口，将基础编排引擎、反思系统、
//! 进化算法和 RL 优化能力接入到行业业务场景中。
//!
//! # 架构
//!
//! ```text
//! IndustryAdapter (trait)
//!     ↓ 实现
//! ├── AiResearchAdapter
//! ├── SoftwareDevAdapter
//! ├── FinanceAdapter
//! ├── SalesAdapter
//! ├── ContentMediaAdapter
//! ├── ConsultingAdapter
//! ├── AccountingAdapter
//! ├── EcommerceAdapter
//! └── EducationAdapter
//!     ↓ 注册到
//! IndustryAdapterRegistry
//! ```

pub mod base_adapter;
pub mod types;

use async_trait::async_trait;
use std::sync::Arc;

use super::dynamic_subgraph::GeneratedSubGraph;
use super::types::OrchestrationError;
use crate::industry_adapters::types::{
    AcceptanceCriterion, EvolutionConstraints, IndustryContext, IndustryLearningConfig,
    MissionType, PresetWorkflowStep, ReflectionTemplate,
};

/// 行业适配器核心 trait
///
/// 每个行业实现此 trait，提供行业特定的：
/// - 动态任务分解策略
/// - 反思模板
/// - 进化约束
/// - 验收标准定义
#[async_trait]
pub trait IndustryAdapter: Send + Sync {
    /// 行业唯一标识
    fn industry_id(&self) -> &str;

    /// 行业显示名称
    fn industry_name(&self) -> &str;

    /// 将用户意图分解为动态任务 DAG
    ///
    /// # 参数
    /// - `mission`: 用户任务描述
    /// - `context`: 行业上下文信息
    ///
    /// # 返回
    /// - 成功: 可执行的动态子图
    /// - 失败: 错误信息
    async fn decompose_mission(
        &self,
        mission: &str,
        context: &IndustryContext,
    ) -> Result<GeneratedSubGraph, OrchestrationError>;

    /// 检测任务类型
    ///
    /// # 参数
    /// - `mission`: 用户任务描述
    ///
    /// # 返回
    /// - 识别出的任务类型
    fn detect_mission_type(&self, mission: &str) -> MissionType;

    /// 获取行业特定反思模板
    fn reflection_template(&self) -> &ReflectionTemplate;

    /// 获取行业特定进化约束
    fn evolution_constraints(&self) -> &EvolutionConstraints;

    /// 获取行业特定验收标准定义
    fn acceptance_criteria(&self) -> &[AcceptanceCriterion];

    /// 获取行业学习配置
    fn learning_config(&self) -> &IndustryLearningConfig;

    /// 获取行业预设工作流步骤
    ///
    /// 返回行业的标准工作流步骤模板，用于初始化工作流编排。
    /// 默认实现返回空列表，行业适配器可覆盖此方法。
    fn preset_steps(&self) -> Vec<PresetWorkflowStep> {
        Vec::new()
    }
}

/// 行业适配器注册表
///
/// 管理所有行业适配器的实例，提供按 ID 查找功能。
pub struct IndustryAdapterRegistry {
    adapters: Vec<Arc<dyn IndustryAdapter>>,
}

impl IndustryAdapterRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }

    /// 注册行业适配器
    pub fn register(&mut self, adapter: Arc<dyn IndustryAdapter>) {
        self.adapters.push(adapter);
    }

    /// 按行业 ID 查找适配器
    pub fn get(&self, industry_id: &str) -> Option<&Arc<dyn IndustryAdapter>> {
        self.adapters.iter().find(|a| a.industry_id() == industry_id)
    }

    /// 获取所有已注册行业 ID 列表
    pub fn list_industries(&self) -> Vec<&str> {
        self.adapters.iter().map(|a| a.industry_id()).collect()
    }

    /// 获取所有已注册行业适配器引用
    pub fn all(&self) -> &[Arc<dyn IndustryAdapter>] {
        &self.adapters
    }

    /// 获取已注册行业数量
    pub fn count(&self) -> usize {
        self.adapters.len()
    }
}

impl Default for IndustryAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 导出基础适配器和配置加载入口（P0-1-A：工厂函数已随硬编码删除）
pub use base_adapter::BaseIndustryAdapter;
