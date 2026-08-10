// SPDX-License-Identifier: AGPL-3.0-only

//! 技能系统增强 (P1-10)
//!
//! 借鉴 Hermes Agent 的技能管理：
//! - 三级披露（摘要→详情→执行）
//! - 条件激活（上下文条件触发）
//! - 技能 Bundle（打包分发）
//! - 使用统计（追踪和分析）
//! - 策展（精选和推荐）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 三级披露（渐进式加载）
// ---------------------------------------------------------------------------

/// 技能披露级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillDisclosureLevel {
    /// Level 0: 摘要（仅名称、描述、类别）
    Summary,
    /// Level 1: 详情（包括参数、示例、触发条件）
    Details,
    /// Level 2: 完整内容（包括实际指令、工具列表、完整实现）
    Full,
}

/// Level 0: 技能摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub quality_score: f64,
    pub success_rate: f64,
    pub total_usages: u32,
    pub tags: Vec<String>,
}

/// Level 1: 技能详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetails {
    pub summary: SkillSummary,
    pub parameters: Vec<SkillParameter>,
    pub examples: Vec<SkillExample>,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub required_tools: Vec<String>,
    pub platforms: Vec<String>,
    pub dependencies: Vec<String>,
    pub last_updated: String,
}

/// 技能参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// 技能示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    pub input: String,
    pub output: String,
    pub description: Option<String>,
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub condition_type: TriggerConditionType,
    pub value: String,
    pub priority: u32,
}

/// 触发条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerConditionType {
    /// 关键词匹配
    Keyword,
    /// 正则匹配
    Regex,
    /// 意图匹配
    Intent,
    /// 实体匹配
    Entity,
    /// 上下文条件
    Context,
}

// ---------------------------------------------------------------------------
// 条件激活
// ---------------------------------------------------------------------------

/// 条件激活配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalActivation {
    /// 是否启用条件激活
    pub enabled: bool,
    /// 激活规则列表
    pub activation_rules: Vec<ActivationRule>,
    /// 失活规则列表（满足则不激活）
    pub deactivation_rules: Vec<ActivationRule>,
}

impl Default for ConditionalActivation {
    fn default() -> Self {
        Self { enabled: false, activation_rules: Vec::new(), deactivation_rules: Vec::new() }
    }
}

/// 激活规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationRule {
    /// 规则 ID
    pub id: String,
    /// 规则类型
    pub rule_type: ActivationRuleType,
    /// 规则条件
    pub condition: String,
    /// 优先级（数字越大优先级越高）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
}

/// 激活规则类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationRuleType {
    /// 关键词匹配
    KeywordMatch,
    /// 正则表达式匹配
    RegexMatch,
    /// 上下文检查
    ContextCheck,
    /// 用户意图识别
    IntentDetection,
    /// 时间窗口
    TimeWindow,
    /// 工具可用性
    ToolAvailability,
    /// 模型能力
    ModelCapability,
}

// ---------------------------------------------------------------------------
// 技能 Bundle
// ---------------------------------------------------------------------------

/// 技能 Bundle（打包分发）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBundle {
    /// Bundle ID
    pub id: String,
    /// Bundle 名称
    pub name: String,
    /// Bundle 描述
    pub description: String,
    /// Bundle 版本
    pub version: String,
    /// 包含的技能 ID 列表
    pub skill_ids: Vec<String>,
    /// 依赖的 Bundle
    pub dependencies: Vec<String>,
    /// Bundle 元数据
    pub metadata: BundleMetadata,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// Bundle 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub author: String,
    pub tags: Vec<String>,
    pub category: String,
    pub install_count: u32,
    pub rating: f64,
    pub license: String,
}

impl SkillBundle {
    /// 创建 Bundle
    pub fn new(name: &str, skill_ids: Vec<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("bundle-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            description: format!("Bundle containing {} skills", skill_ids.len()),
            version: "1.0.0".to_string(),
            skill_ids,
            dependencies: Vec::new(),
            metadata: BundleMetadata {
                author: "system".to_string(),
                tags: Vec::new(),
                category: "general".to_string(),
                install_count: 0,
                rating: 0.0,
                license: "MIT".to_string(),
            },
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 添加技能
    pub fn add_skill(&mut self, skill_id: &str) {
        if !self.skill_ids.contains(&skill_id.to_string()) {
            self.skill_ids.push(skill_id.to_string());
            self.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }

    /// 移除技能
    pub fn remove_skill(&mut self, skill_id: &str) {
        if let Some(pos) = self.skill_ids.iter().position(|id| id == skill_id) {
            self.skill_ids.remove(pos);
            self.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }

    /// Bundle 大小
    pub fn size(&self) -> usize {
        self.skill_ids.len()
    }
}

// ---------------------------------------------------------------------------
// 使用统计
// ---------------------------------------------------------------------------

/// 技能使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsageStats {
    /// 技能 ID
    pub skill_id: String,
    /// 总使用次数
    pub total_usages: u64,
    /// 成功次数
    pub successful_usages: u64,
    /// 失败次数
    pub failed_usages: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f64,
    /// 总执行时间（毫秒）
    pub total_execution_time_ms: u64,
    /// 最后使用时间
    pub last_used_at: Option<String>,
    /// 按天统计
    pub daily_stats: HashMap<String, DailyUsageStats>,
    /// 按场景统计
    pub scenario_stats: HashMap<String, u64>,
}

impl SkillUsageStats {
    pub fn new(skill_id: &str) -> Self {
        Self {
            skill_id: skill_id.to_string(),
            total_usages: 0,
            successful_usages: 0,
            failed_usages: 0,
            success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            total_execution_time_ms: 0,
            last_used_at: None,
            daily_stats: HashMap::new(),
            scenario_stats: HashMap::new(),
        }
    }

    /// 记录一次使用
    pub fn record_usage(&mut self, success: bool, duration_ms: u64, scenario: &str) {
        self.total_usages += 1;
        self.total_execution_time_ms += duration_ms;

        if success {
            self.successful_usages += 1;
        } else {
            self.failed_usages += 1;
        }

        // 更新成功率
        self.success_rate = if self.total_usages > 0 {
            self.successful_usages as f64 / self.total_usages as f64
        } else {
            0.0
        };

        // 更新平均执行时间
        self.avg_execution_time_ms = if self.total_usages > 0 {
            self.total_execution_time_ms as f64 / self.total_usages as f64
        } else {
            0.0
        };

        // 更新最后使用时间
        self.last_used_at = Some(chrono::Utc::now().to_rfc3339());

        // 更新每日统计
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let daily = self.daily_stats.entry(today).or_insert(DailyUsageStats::default());
        daily.total += 1;
        if success {
            daily.success += 1;
        } else {
            daily.failure += 1;
        }

        // 更新场景统计
        *self.scenario_stats.entry(scenario.to_string()).or_insert(0) += 1;
    }
}

/// 每日使用统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyUsageStats {
    pub total: u64,
    pub success: u64,
    pub failure: u64,
}

// ---------------------------------------------------------------------------
// 策展
// ---------------------------------------------------------------------------

/// 技能策展集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCuratedCollection {
    /// 集合 ID
    pub id: String,
    /// 集合名称
    pub name: String,
    /// 集合描述
    pub description: String,
    /// 分类
    pub category: CuratedCategory,
    /// 推荐的技能 ID 列表（按推荐顺序）
    pub recommended_skill_ids: Vec<String>,
    /// 评分标准
    pub rating_criteria: RatingCriteria,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 策展分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuratedCategory {
    /// 新手入门
    Beginner,
    /// 高级功能
    Advanced,
    /// 生产力
    Productivity,
    /// 开发工具
    Development,
    /// 数据分析
    DataAnalysis,
    /// 内容创作
    ContentCreation,
    /// 自动化
    Automation,
}

/// 评分标准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingCriteria {
    /// 最低成功率
    pub min_success_rate: f64,
    /// 最小使用次数
    pub min_usages: u64,
    /// 最低质量分数
    pub min_quality_score: f64,
    /// 最大平均执行时间（毫秒）
    pub max_avg_execution_time_ms: f64,
}

impl Default for RatingCriteria {
    fn default() -> Self {
        Self {
            min_success_rate: 0.7,
            min_usages: 10,
            min_quality_score: 0.5,
            max_avg_execution_time_ms: 30000.0,
        }
    }
}

impl SkillCuratedCollection {
    /// 创建策展集合
    pub fn new(name: &str, category: CuratedCategory) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("curated-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            description: String::new(),
            category,
            recommended_skill_ids: Vec::new(),
            rating_criteria: RatingCriteria::default(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 推荐技能
    pub fn recommend_skill(&mut self, skill_id: &str, stats: &SkillUsageStats) -> bool {
        // 检查评分标准
        if stats.success_rate < self.rating_criteria.min_success_rate {
            return false;
        }
        if stats.total_usages < self.rating_criteria.min_usages {
            return false;
        }

        // 添加到推荐列表（如果不在）
        if !self.recommended_skill_ids.contains(&skill_id.to_string()) {
            self.recommended_skill_ids.push(skill_id.to_string());
            self.updated_at = chrono::Utc::now().to_rfc3339();
            return true;
        }

        false
    }

    /// 获取推荐数量
    pub fn recommendation_count(&self) -> usize {
        self.recommended_skill_ids.len()
    }
}

/// 技能推荐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecommendation {
    pub skill_id: String,
    pub skill_summary: SkillSummary,
    pub recommendation_score: f64,
    pub recommendation_reason: String,
    pub suggested_use_case: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_summary() {
        let summary = SkillSummary {
            id: "1".to_string(),
            name: "代码审查".to_string(),
            description: "审查代码质量".to_string(),
            category: "development".to_string(),
            version: "1.0.0".to_string(),
            quality_score: 0.8,
            success_rate: 0.95,
            total_usages: 100,
            tags: vec!["code".to_string()],
        };

        assert_eq!(summary.name, "代码审查");
        assert!(summary.success_rate > 0.9);
    }

    #[test]
    fn test_bundle() {
        let mut bundle =
            SkillBundle::new("开发工具集", vec!["skill-1".to_string(), "skill-2".to_string()]);
        assert_eq!(bundle.size(), 2);

        bundle.add_skill("skill-3");
        assert_eq!(bundle.size(), 3);

        bundle.remove_skill("skill-1");
        assert_eq!(bundle.size(), 2);
    }

    #[test]
    fn test_usage_stats() {
        let mut stats = SkillUsageStats::new("skill-1");
        assert_eq!(stats.total_usages, 0);

        stats.record_usage(true, 1000, "code_review");
        assert_eq!(stats.total_usages, 1);
        assert_eq!(stats.successful_usages, 1);
        assert_eq!(stats.success_rate, 1.0);

        stats.record_usage(false, 2000, "code_review");
        assert_eq!(stats.total_usages, 2);
        assert_eq!(stats.failed_usages, 1);
        assert!(stats.success_rate < 1.0);
    }

    #[test]
    fn test_curated_collection() {
        let mut collection =
            SkillCuratedCollection::new("推荐开发技能", CuratedCategory::Development);
        assert_eq!(collection.category, CuratedCategory::Development);

        let mut stats = SkillUsageStats::new("skill-1");
        stats.record_usage(true, 500, "test");

        // 不应该推荐（使用次数不够）
        assert!(!collection.recommend_skill("skill-1", &stats));

        // 增加使用次数
        for _ in 0..10 {
            stats.record_usage(true, 500, "test");
        }

        // 现在应该可以推荐
        assert!(collection.recommend_skill("skill-1", &stats));
        assert_eq!(collection.recommendation_count(), 1);
    }

    #[test]
    fn test_conditional_activation_default() {
        let activation = ConditionalActivation::default();
        assert!(!activation.enabled);
        assert!(activation.activation_rules.is_empty());
    }
}
