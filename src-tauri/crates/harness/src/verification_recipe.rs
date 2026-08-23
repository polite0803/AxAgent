// SPDX-License-Identifier: AGPL-3.0-only

//! 验证 Recipe 系统 (P2-13)
//!
//! 借鉴 Hermes Agent 的验证配方：
//! - VerificationRecipe: 预定义的验证配方
//! - VerificationStep: 验证步骤
//! - VerificationReport: 验证报告

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 验证配方
// ---------------------------------------------------------------------------

/// 验证配方
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecipe {
    /// 配方 ID
    pub id: String,
    /// 配方名称
    pub name: String,
    /// 配方描述
    pub description: String,
    /// 配方类型
    pub recipe_type: VerificationRecipeType,
    /// 触发条件
    pub trigger: VerificationTrigger,
    /// 验证步骤列表
    pub steps: Vec<VerificationStep>,
    /// 预期结果
    pub expected_outcome: ExpectedOutcome,
    /// 严重程度
    pub severity: VerificationSeverity,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 配方类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRecipeType {
    /// 代码质量
    CodeQuality,
    /// 安全检查
    SecurityCheck,
    /// 性能验证
    PerformanceCheck,
    /// 功能验证
    FunctionalCheck,
    /// 集成验证
    IntegrationCheck,
    /// 回归测试
    RegressionTest,
    /// 合规检查
    ComplianceCheck,
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationTrigger {
    /// 触发类型
    pub trigger_type: VerificationTriggerType,
    /// 条件值
    pub condition: String,
    /// 是否需要用户确认
    pub require_user_confirmation: bool,
}

/// 触发类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationTriggerType {
    /// 每次执行
    Always,
    /// 检测到特定工具使用
    ToolUsage,
    /// 检测到文件修改
    FileModification,
    /// 定时触发
    Scheduled,
    /// 手动触发
    Manual,
    /// 错误发生后
    OnError,
}

/// 验证步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: VerificationStepType,
    /// 检查命令或表达式
    pub check_command: String,
    /// 期望的结果模式
    pub expected_pattern: Option<String>,
    /// 失败后的操作
    pub on_failure: FailureAction,
    /// 是否可选
    pub optional: bool,
}

/// 步骤类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStepType {
    /// 运行命令
    RunCommand,
    /// 检查文件
    CheckFile,
    /// 检查 API
    CheckApi,
    /// 代码分析
    CodeAnalysis,
    /// 单元测试
    UnitTest,
    /// 集成测试
    IntegrationTest,
    /// 静态分析
    StaticAnalysis,
    /// 手动检查
    ManualCheck,
}

/// 失败后的操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureAction {
    /// 停止执行
    Stop,
    /// 继续但标记警告
    ContinueWithWarning,
    /// 重试
    Retry,
    /// 升级到人工
    EscalateToHuman,
}

/// 预期结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    /// 成功条件
    pub success_condition: String,
    /// 失败条件
    pub failure_condition: String,
    /// 通过标准（如 80% 步骤通过）
    pub pass_criteria: PassCriteria,
}

/// 通过标准
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassCriteria {
    /// 所有步骤必须通过
    AllSteps,
    /// 必须通过关键步骤
    CriticalStepsOnly,
    /// 通过率超过阈值
    ThresholdPassRate,
}

/// 严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl VerificationRecipe {
    /// 创建新配方
    pub fn new(name: &str, recipe_type: VerificationRecipeType) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("recipe-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            description: String::new(),
            recipe_type,
            trigger: VerificationTrigger {
                trigger_type: VerificationTriggerType::Manual,
                condition: String::new(),
                require_user_confirmation: false,
            },
            steps: Vec::new(),
            expected_outcome: ExpectedOutcome {
                success_condition: "所有步骤通过".to_string(),
                failure_condition: "任何关键步骤失败".to_string(),
                pass_criteria: PassCriteria::CriticalStepsOnly,
            },
            severity: VerificationSeverity::Medium,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 添加步骤
    pub fn add_step(&mut self, step: VerificationStep) {
        self.steps.push(step);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// 获取关键步骤
    pub fn critical_steps(&self) -> Vec<&VerificationStep> {
        self.steps.iter().filter(|s| !s.optional).collect()
    }

    /// 获取可选步骤
    pub fn optional_steps(&self) -> Vec<&VerificationStep> {
        self.steps.iter().filter(|s| s.optional).collect()
    }

    /// 总步骤数
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

// ---------------------------------------------------------------------------
// 预定义配方
// ---------------------------------------------------------------------------

impl VerificationRecipe {
    /// 代码质量检查配方
    pub fn code_quality() -> Self {
        let mut recipe = Self::new("代码质量检查", VerificationRecipeType::CodeQuality);
        recipe.description = "检查代码是否符合质量标准".to_string();
        recipe.severity = VerificationSeverity::High;
        recipe.trigger = VerificationTrigger {
            trigger_type: VerificationTriggerType::FileModification,
            condition: "*.rs,*.ts,*.tsx".to_string(),
            require_user_confirmation: false,
        };

        recipe.add_step(VerificationStep {
            id: "step-1".to_string(),
            name: "代码格式化".to_string(),
            description: "检查代码是否格式化".to_string(),
            step_type: VerificationStepType::RunCommand,
            check_command: "cargo fmt --check".to_string(),
            expected_pattern: None,
            on_failure: FailureAction::ContinueWithWarning,
            optional: false,
        });

        recipe.add_step(VerificationStep {
            id: "step-2".to_string(),
            name: "代码检查".to_string(),
            description: "运行 clippy 检查".to_string(),
            step_type: VerificationStepType::RunCommand,
            check_command: "cargo check".to_string(),
            expected_pattern: None,
            on_failure: FailureAction::Stop,
            optional: false,
        });

        recipe
    }

    /// 安全检查配方
    pub fn security_check() -> Self {
        let mut recipe = Self::new("安全检查", VerificationRecipeType::SecurityCheck);
        recipe.description = "检查代码是否存在安全问题".to_string();
        recipe.severity = VerificationSeverity::Critical;

        recipe.add_step(VerificationStep {
            id: "step-1".to_string(),
            name: "依赖审计".to_string(),
            description: "检查依赖是否有已知漏洞".to_string(),
            step_type: VerificationStepType::RunCommand,
            check_command: "cargo audit".to_string(),
            expected_pattern: None,
            on_failure: FailureAction::EscalateToHuman,
            optional: false,
        });

        recipe.add_step(VerificationStep {
            id: "step-2".to_string(),
            name: "密钥泄露检查".to_string(),
            description: "检查代码中是否有硬编码密钥".to_string(),
            step_type: VerificationStepType::StaticAnalysis,
            check_command: "grep -r 'api_key\\|secret\\|password' --include='*.rs' .".to_string(),
            expected_pattern: None,
            on_failure: FailureAction::Stop,
            optional: false,
        });

        recipe
    }
}

// ---------------------------------------------------------------------------
// 验证报告
// ---------------------------------------------------------------------------

/// 验证报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// 报告 ID
    pub id: String,
    /// 配方 ID
    pub recipe_id: String,
    /// 配方名称
    pub recipe_name: String,
    /// 执行时间
    pub executed_at: String,
    /// 总步骤数
    pub total_steps: usize,
    /// 通过步骤数
    pub passed_steps: usize,
    /// 失败步骤数
    pub failed_steps: usize,
    /// 跳过步骤数
    pub skipped_steps: usize,
    /// 是否全部通过
    pub all_passed: bool,
    /// 关键步骤是否通过
    pub critical_passed: bool,
    /// 步骤结果详情
    pub step_results: Vec<StepResult>,
    /// 执行耗时（毫秒）
    pub execution_time_ms: u64,
    /// 总体评级
    pub rating: VerificationRating,
    /// 建议
    pub recommendations: Vec<String>,
}

/// 步骤结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub status: StepStatus,
    pub output: String,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// 验证评级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRating {
    Excellent,
    Good,
    Fair,
    Poor,
    Failed,
}

impl VerificationReport {
    /// 创建报告
    pub fn new(recipe: &VerificationRecipe) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("report-{}", uuid::Uuid::new_v4()),
            recipe_id: recipe.id.clone(),
            recipe_name: recipe.name.clone(),
            executed_at: now,
            total_steps: recipe.steps.len(),
            passed_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            all_passed: true,
            critical_passed: true,
            step_results: Vec::new(),
            execution_time_ms: 0,
            rating: VerificationRating::Fair,
            recommendations: Vec::new(),
        }
    }

    /// 添加步骤结果
    pub fn add_result(&mut self, result: StepResult) {
        match result.status {
            StepStatus::Passed => self.passed_steps += 1,
            StepStatus::Failed => self.failed_steps += 1,
            StepStatus::Skipped => self.skipped_steps += 1,
            StepStatus::Error => self.failed_steps += 1,
        }

        self.step_results.push(result);
        self.update_rating();
    }

    /// 更新评级
    fn update_rating(&mut self) {
        let pass_rate = if self.total_steps > 0 {
            self.passed_steps as f64 / self.total_steps as f64
        } else {
            0.0
        };

        self.all_passed = self.failed_steps == 0;
        self.critical_passed = self.failed_steps == 0; // 简化处理

        self.rating = if self.all_passed && pass_rate >= 0.9 {
            VerificationRating::Excellent
        } else if pass_rate >= 0.7 {
            VerificationRating::Good
        } else if pass_rate >= 0.5 {
            VerificationRating::Fair
        } else if pass_rate > 0.0 {
            VerificationRating::Poor
        } else {
            VerificationRating::Failed
        };

        // 更新建议
        self.recommendations.clear();
        if !self.all_passed {
            self.recommendations.push("修复失败的步骤后重新运行验证".to_string());
        }
        if self.rating == VerificationRating::Poor || self.rating == VerificationRating::Failed {
            self.recommendations.push("验证结果较差，建议优先处理".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_creation() {
        let recipe = VerificationRecipe::code_quality();
        assert!(recipe.enabled);
        assert!(!recipe.steps.is_empty());
        assert_eq!(recipe.recipe_type, VerificationRecipeType::CodeQuality);
    }

    #[test]
    fn test_recipe_steps() {
        let recipe = VerificationRecipe::code_quality();
        assert!(recipe.step_count() >= 2);
        assert!(!recipe.critical_steps().is_empty());
        assert!(recipe.optional_steps().is_empty());
    }

    #[test]
    fn test_report_generation() {
        let recipe = VerificationRecipe::code_quality();
        let mut report = VerificationReport::new(&recipe);

        report.add_result(StepResult {
            step_id: "step-1".to_string(),
            step_name: "格式化".to_string(),
            status: StepStatus::Passed,
            output: "All formatted".to_string(),
            duration_ms: 100,
            error_message: None,
        });

        report.add_result(StepResult {
            step_id: "step-2".to_string(),
            step_name: "检查".to_string(),
            status: StepStatus::Passed,
            output: "No errors".to_string(),
            duration_ms: 500,
            error_message: None,
        });

        assert_eq!(report.passed_steps, 2);
        assert!(report.all_passed);
        assert!(matches!(report.rating, VerificationRating::Excellent));
    }

    #[test]
    fn test_report_with_failure() {
        let recipe = VerificationRecipe::code_quality();
        let mut report = VerificationReport::new(&recipe);

        report.add_result(StepResult {
            step_id: "step-1".to_string(),
            step_name: "格式化".to_string(),
            status: StepStatus::Passed,
            output: "OK".to_string(),
            duration_ms: 100,
            error_message: None,
        });

        report.add_result(StepResult {
            step_id: "step-2".to_string(),
            step_name: "检查".to_string(),
            status: StepStatus::Failed,
            output: "Found errors".to_string(),
            duration_ms: 300,
            error_message: Some("编译错误".to_string()),
        });

        assert_eq!(report.passed_steps, 1);
        assert_eq!(report.failed_steps, 1);
        assert!(!report.all_passed);
        assert!(!report.recommendations.is_empty());
    }
}
