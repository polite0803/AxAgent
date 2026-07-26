// SPDX-License-Identifier: AGPL-3.0-only
//! G12 Task/Pipeline 契约系统
//!
//! 为工作流任务定义结构化契约，明确输入/输出 schema、验收标准和 SLA。
//! 三种 harness profile 覆盖 AxInvest 主要任务类型：
//!
//! ## 三种 harness profile
//!
//! | profile              | 适用场景                     | 验收标准                       |
//! |---------------------|------------------------------|--------------------------------|
//! | `tool_orchestrated` | 工具编排任务（数据采集/分析） | 工具调用成功率 + 输出字段完整   |
//! | `artifact_synthesis`| 产物合成任务（报告/摘要生成） | 字数/格式/关键信息覆盖率       |
//! | `portfolio_task`    | 组合管理任务（调仓/风控）     | 决策一致性 + 风险约束满足       |
//!
//! ## 使用方式
//!
//! ```ignore
//! use axagent_rt_workflow::task_contract::{TaskContract, HarnessProfile};
//!
//! let contract = TaskContract::new("daily-market-scan")
//!     .with_profile(HarnessProfile::ToolOrchestrated)
//!     .with_input_schema(json!({"type": "object", "properties": {...}}))
//!     .with_output_schema(json!({"type": "object", "properties": {...}}))
//!     .with_sla_max_seconds(300);
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 任务 harness profile 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessProfile {
    /// 工具编排任务（数据采集/分析）
    ToolOrchestrated,
    /// 产物合成任务（报告/摘要生成）
    ArtifactSynthesis,
    /// 组合管理任务（调仓/风控）
    PortfolioTask,
}

impl HarnessProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolOrchestrated => "tool_orchestrated",
            Self::ArtifactSynthesis => "artifact_synthesis",
            Self::PortfolioTask => "portfolio_task",
        }
    }

    /// 默认验收标准
    pub fn default_acceptance_criteria(&self) -> AcceptanceCriteria {
        match self {
            Self::ToolOrchestrated => AcceptanceCriteria {
                min_tool_success_rate: 0.8,
                required_output_fields: vec![],
                min_word_count: None,
                max_word_count: None,
                required_format: None,
                max_position_pct: None,
                require_risk_constraints: false,
            },
            Self::ArtifactSynthesis => AcceptanceCriteria {
                min_tool_success_rate: 0.0,
                required_output_fields: vec!["summary".to_string()],
                min_word_count: Some(200),
                max_word_count: Some(5000),
                required_format: Some("markdown".to_string()),
                max_position_pct: None,
                require_risk_constraints: false,
            },
            Self::PortfolioTask => AcceptanceCriteria {
                min_tool_success_rate: 0.0,
                required_output_fields: vec!["action".to_string(), "position_pct".to_string()],
                min_word_count: None,
                max_word_count: None,
                required_format: None,
                max_position_pct: Some(100.0),
                require_risk_constraints: true,
            },
        }
    }
}

/// 验收标准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    /// 工具调用最低成功率（0.0-1.0）
    pub min_tool_success_rate: f64,
    /// 必需的输出字段
    pub required_output_fields: Vec<String>,
    /// 最小字数（产物合成任务）
    pub min_word_count: Option<usize>,
    /// 最大字数（产物合成任务）
    pub max_word_count: Option<usize>,
    /// 必需的输出格式（如 "markdown" / "json"）
    pub required_format: Option<String>,
    /// 最大仓位百分比（组合管理任务）
    pub max_position_pct: Option<f64>,
    /// 是否要求满足风险约束
    pub require_risk_constraints: bool,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成并通过验收
    Accepted,
    /// 已完成但未通过验收
    Rejected,
    /// 执行失败
    Failed,
    /// 超时
    Timeout,
}

impl ContractStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Accepted | Self::Rejected | Self::Failed | Self::Timeout)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// 验收结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceResult {
    /// 是否通过
    pub passed: bool,
    /// 失败原因（未通过时）
    pub failures: Vec<String>,
    /// 实际测得的指标
    pub metrics: HashMap<String, Value>,
}

impl AcceptanceResult {
    pub fn passed() -> Self {
        Self { passed: true, failures: vec![], metrics: HashMap::new() }
    }

    pub fn failed(failures: Vec<String>) -> Self {
        Self { passed: false, failures, metrics: HashMap::new() }
    }
}

/// 任务契约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContract {
    /// 任务 ID
    pub task_id: String,
    /// 任务名称
    pub task_name: String,
    /// harness profile
    pub profile: HarnessProfile,
    /// 输入 schema（JSON Schema）
    pub input_schema: Option<Value>,
    /// 输出 schema（JSON Schema）
    pub output_schema: Option<Value>,
    /// 验收标准
    pub acceptance_criteria: AcceptanceCriteria,
    /// SLA：最大执行秒数
    pub sla_max_seconds: Option<u64>,
    /// 重试配置
    pub max_retries: u32,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 任务状态
    pub status: ContractStatus,
    /// 执行开始时间
    pub started_at: Option<i64>,
    /// 执行结束时间
    pub finished_at: Option<i64>,
    /// 实际输出
    pub output: Option<Value>,
    /// 验收结果
    pub acceptance_result: Option<AcceptanceResult>,
    /// 错误信息
    pub error: Option<String>,
}

impl TaskContract {
    /// 创建新的任务契约
    pub fn new(task_name: impl Into<String>) -> Self {
        let task_name = task_name.into();
        let task_id = format!("task_{}_{}", now_ms(), sanitize_name(&task_name));
        Self {
            task_id,
            task_name,
            profile: HarnessProfile::ToolOrchestrated,
            input_schema: None,
            output_schema: None,
            acceptance_criteria: HarnessProfile::ToolOrchestrated.default_acceptance_criteria(),
            sla_max_seconds: None,
            max_retries: 0,
            created_at: now_ms(),
            status: ContractStatus::Pending,
            started_at: None,
            finished_at: None,
            output: None,
            acceptance_result: None,
            error: None,
        }
    }

    /// 设置 harness profile（同时更新默认验收标准）
    pub fn with_profile(mut self, profile: HarnessProfile) -> Self {
        self.acceptance_criteria = profile.default_acceptance_criteria();
        self.profile = profile;
        self
    }

    /// 设置输入 schema
    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// 设置输出 schema
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// 自定义验收标准
    pub fn with_acceptance_criteria(mut self, criteria: AcceptanceCriteria) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    /// 设置 SLA
    pub fn with_sla_max_seconds(mut self, seconds: u64) -> Self {
        self.sla_max_seconds = Some(seconds);
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// 标记开始执行
    pub fn mark_started(&mut self) {
        self.status = ContractStatus::Running;
        self.started_at = Some(now_ms());
    }

    /// 标记完成（待验收）
    pub fn mark_completed(&mut self, output: Value) {
        self.output = Some(output);
        self.finished_at = Some(now_ms());
        // 自动验收
        let result = self.validate_output();
        self.acceptance_result = Some(result.clone());
        if result.passed {
            self.status = ContractStatus::Accepted;
        } else {
            self.status = ContractStatus::Rejected;
        }
    }

    /// 标记失败
    pub fn mark_failed(&mut self, error: String) {
        self.error = Some(error);
        self.status = ContractStatus::Failed;
        self.finished_at = Some(now_ms());
    }

    /// 标记超时
    pub fn mark_timeout(&mut self) {
        self.status = ContractStatus::Timeout;
        self.finished_at = Some(now_ms());
        self.error = Some("Task exceeded SLA timeout".to_string());
    }

    /// 验收输出
    pub fn validate_output(&self) -> AcceptanceResult {
        let output = match &self.output {
            Some(o) => o,
            None => {
                return AcceptanceResult::failed(vec!["输出为空".to_string()]);
            },
        };

        let mut failures = Vec::new();
        let mut metrics = HashMap::new();

        // 检查必需字段
        for field in &self.acceptance_criteria.required_output_fields {
            if output.get(field).is_none() {
                failures.push(format!("缺少必需字段: {field}"));
            }
        }

        // 检查字数（如果是字符串输出）
        if let Some(min) = self.acceptance_criteria.min_word_count {
            if let Some(text) = output.as_str() {
                let word_count = text.split_whitespace().count();
                metrics.insert("word_count".to_string(), json!(word_count));
                if word_count < min {
                    failures.push(format!("字数 {word_count} 低于最小要求 {min}"));
                }
            }
        }

        if let Some(max) = self.acceptance_criteria.max_word_count {
            if let Some(text) = output.as_str() {
                let word_count = text.split_whitespace().count();
                if word_count > max {
                    failures.push(format!("字数 {word_count} 超过最大限制 {max}"));
                }
            }
        }

        // 检查仓位百分比
        if let Some(max_pct) = self.acceptance_criteria.max_position_pct {
            if let Some(pct) = output.get("position_pct").and_then(|v| v.as_f64()) {
                metrics.insert("position_pct".to_string(), json!(pct));
                if pct > max_pct {
                    failures.push(format!("仓位 {pct}% 超过最大限制 {max_pct}%"));
                }
            }
        }

        // 检查工具成功率
        if self.acceptance_criteria.min_tool_success_rate > 0.0 {
            if let Some(rate) = output.get("tool_success_rate").and_then(|v| v.as_f64()) {
                metrics.insert("tool_success_rate".to_string(), json!(rate));
                if rate < self.acceptance_criteria.min_tool_success_rate {
                    failures.push(format!(
                        "工具成功率 {rate:.2} 低于最小要求 {:.2}",
                        self.acceptance_criteria.min_tool_success_rate
                    ));
                }
            }
        }

        if failures.is_empty() {
            AcceptanceResult { passed: true, failures: vec![], metrics }
        } else {
            AcceptanceResult { passed: false, failures, metrics }
        }
    }

    /// 检查是否超时
    pub fn is_timed_out(&self) -> bool {
        if let (Some(started), Some(sla)) = (self.started_at, self.sla_max_seconds) {
            let elapsed = (now_ms() - started) / 1000;
            return elapsed as u64 > sla;
        }
        false
    }

    /// 执行耗时（秒）
    pub fn elapsed_seconds(&self) -> Option<u64> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(((end - start) / 1000) as u64),
            (Some(start), None) => Some(((now_ms() - start) / 1000) as u64),
            _ => None,
        }
    }
}

/// 当前 Unix 毫秒时间戳
fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// 清理名称用于生成 task_id
fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(32)
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_profile_str() {
        assert_eq!(HarnessProfile::ToolOrchestrated.as_str(), "tool_orchestrated");
        assert_eq!(HarnessProfile::ArtifactSynthesis.as_str(), "artifact_synthesis");
        assert_eq!(HarnessProfile::PortfolioTask.as_str(), "portfolio_task");
    }

    #[test]
    fn test_default_acceptance_criteria() {
        let tool_criteria = HarnessProfile::ToolOrchestrated.default_acceptance_criteria();
        assert_eq!(tool_criteria.min_tool_success_rate, 0.8);

        let artifact_criteria = HarnessProfile::ArtifactSynthesis.default_acceptance_criteria();
        assert_eq!(artifact_criteria.min_word_count, Some(200));
        assert_eq!(artifact_criteria.max_word_count, Some(5000));
        assert_eq!(artifact_criteria.required_format.as_deref(), Some("markdown"));

        let portfolio_criteria = HarnessProfile::PortfolioTask.default_acceptance_criteria();
        assert_eq!(portfolio_criteria.max_position_pct, Some(100.0));
        assert!(portfolio_criteria.require_risk_constraints);
    }

    #[test]
    fn test_contract_creation() {
        let contract = TaskContract::new("daily-market-scan")
            .with_profile(HarnessProfile::ToolOrchestrated)
            .with_sla_max_seconds(300)
            .with_max_retries(3);

        assert_eq!(contract.task_name, "daily-market-scan");
        assert_eq!(contract.profile, HarnessProfile::ToolOrchestrated);
        assert_eq!(contract.sla_max_seconds, Some(300));
        assert_eq!(contract.max_retries, 3);
        assert_eq!(contract.status, ContractStatus::Pending);
        assert!(contract.task_id.starts_with("task_"));
    }

    #[test]
    fn test_status_transitions() {
        let mut contract = TaskContract::new("test");
        assert_eq!(contract.status, ContractStatus::Pending);

        contract.mark_started();
        assert_eq!(contract.status, ContractStatus::Running);
        assert!(contract.started_at.is_some());

        contract.mark_failed("error".to_string());
        assert_eq!(contract.status, ContractStatus::Failed);
        assert!(contract.finished_at.is_some());
        assert_eq!(contract.error.as_deref(), Some("error"));
    }

    #[test]
    fn test_validation_accepted() {
        let mut contract = TaskContract::new("test").with_profile(HarnessProfile::PortfolioTask);
        contract.mark_started();
        contract.mark_completed(json!({
            "action": "buy",
            "position_pct": 50.0
        }));
        assert_eq!(contract.status, ContractStatus::Accepted);
        assert!(contract.acceptance_result.as_ref().unwrap().passed);
    }

    #[test]
    fn test_validation_rejected_missing_field() {
        let mut contract = TaskContract::new("test").with_profile(HarnessProfile::PortfolioTask);
        contract.mark_started();
        contract.mark_completed(json!({
            "action": "buy"
            // 缺少 position_pct
        }));
        assert_eq!(contract.status, ContractStatus::Rejected);
        assert!(!contract.acceptance_result.as_ref().unwrap().passed);
    }

    #[test]
    fn test_validation_rejected_position_exceeded() {
        let mut contract = TaskContract::new("test").with_profile(HarnessProfile::PortfolioTask);
        contract.mark_started();
        contract.mark_completed(json!({
            "action": "buy",
            "position_pct": 150.0  // 超过 100% 限制
        }));
        assert_eq!(contract.status, ContractStatus::Rejected);
    }

    #[test]
    fn test_validation_artifact_word_count() {
        let mut contract =
            TaskContract::new("test").with_profile(HarnessProfile::ArtifactSynthesis);

        // 短文本应被拒绝（少于 200 字）
        contract.mark_started();
        contract.mark_completed(json!("short text"));
        assert_eq!(contract.status, ContractStatus::Rejected);

        // 长文本应通过
        let long_text = "word ".repeat(300);
        let mut contract2 =
            TaskContract::new("test").with_profile(HarnessProfile::ArtifactSynthesis);
        contract2.mark_started();
        contract2.mark_completed(json!(long_text.trim()));
        assert_eq!(contract2.status, ContractStatus::Accepted);
    }

    #[test]
    fn test_timeout_detection() {
        let mut contract = TaskContract::new("test")
            .with_profile(HarnessProfile::ToolOrchestrated)
            .with_sla_max_seconds(0);

        // SLA=0 秒，启动后立即超时
        contract.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(contract.is_timed_out());

        contract.mark_timeout();
        assert_eq!(contract.status, ContractStatus::Timeout);
    }

    #[test]
    fn test_elapsed_seconds() {
        let mut contract = TaskContract::new("test");
        assert_eq!(contract.elapsed_seconds(), None);

        contract.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(contract.elapsed_seconds().is_some());

        contract.mark_completed(json!({}));
        assert!(contract.elapsed_seconds().is_some());
    }

    #[test]
    fn test_status_terminal_and_success() {
        assert!(ContractStatus::Accepted.is_terminal());
        assert!(ContractStatus::Rejected.is_terminal());
        assert!(ContractStatus::Failed.is_terminal());
        assert!(ContractStatus::Timeout.is_terminal());
        assert!(!ContractStatus::Pending.is_terminal());
        assert!(!ContractStatus::Running.is_terminal());

        assert!(ContractStatus::Accepted.is_success());
        assert!(!ContractStatus::Rejected.is_success());
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Daily Market Scan"), "dailymarketscan");
        assert_eq!(sanitize_name("task-123"), "task-123");
        assert_eq!(sanitize_name("中文任务"), "");
        assert_eq!(sanitize_name("a_b_c"), "a_b_c");
    }
}
