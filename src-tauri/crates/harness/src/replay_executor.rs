// SPDX-License-Identifier: AGPL-3.0-only

//! ReplayExecutor — 轨迹回放与回归测试契约。
//!
//! 提供"加载 golden trajectory → 逐 step 对比 → 生成回归报告"的能力。
//!
//! ## 核心概念
//!
//! - **Golden Trajectory**：已录制的高质量轨迹，作为回归基准
//! - **Replay**：在相同输入下重新执行/对比，检测行为偏差
//! - **Deviation**：回放行为与 golden 的差异（role/content/tool_call/tool_result）
//! - **Regression Suite**：一组 golden trajectory，批量回放生成汇总报告
//!
//! ## 设计原则
//!
//! - `replay_trajectory` 是纯函数，不依赖外部资源，可独立单元测试
//! - `ReplayExecutor` trait 接受 `&Trajectory`（而非 trajectory_id），让消费者自行加载
//! - 实现方（如 `TrajectoryReplayer`）可额外提供 `replay_by_id` 方法封装加载逻辑

use serde::{Deserialize, Serialize};

use crate::trajectory_types::{Trajectory, TrajectoryOutcome, TrajectoryStep};

// ── 偏差类型 ──────────────────────────────────────────────────────────

/// 回放过程中检测到的偏差类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviationKind {
    /// 步骤角色不匹配（如 golden 是 Assistant 但 current 是 User）
    RoleMismatch,
    /// 步骤内容不匹配（strict 模式下完全匹配，否则相似度低于阈值）
    ContentMismatch,
    /// golden 有 tool_calls 但 current 缺少
    ToolCallMissing,
    /// current 有多余的 tool_calls
    ToolCallExtra,
    /// tool_call 名称不匹配
    ToolCallNameMismatch,
    /// tool_call 参数不匹配
    ToolCallArgumentsMismatch,
    /// tool 执行结果不匹配
    ToolResultMismatch,
    /// tool 执行出错（golden 成功但回放失败）
    ToolResultError,
    /// 步数不匹配（current 比 golden 少或多）
    StepCountMismatch,
}

impl DeviationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoleMismatch => "role_mismatch",
            Self::ContentMismatch => "content_mismatch",
            Self::ToolCallMissing => "tool_call_missing",
            Self::ToolCallExtra => "tool_call_extra",
            Self::ToolCallNameMismatch => "tool_call_name_mismatch",
            Self::ToolCallArgumentsMismatch => "tool_call_arguments_mismatch",
            Self::ToolResultMismatch => "tool_result_mismatch",
            Self::ToolResultError => "tool_result_error",
            Self::StepCountMismatch => "step_count_mismatch",
        }
    }
}

impl std::fmt::Display for DeviationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── 单步偏差 ──────────────────────────────────────────────────────────

/// 单步回放偏差记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDeviation {
    /// 偏差发生的步骤索引（基于 golden trajectory）
    pub step_index: usize,
    /// 偏差类型
    pub kind: DeviationKind,
    /// golden 中的值
    pub golden_value: String,
    /// 当前回放的值
    pub current_value: String,
    /// 人类可读的详细描述
    pub detail: String,
}

// ── 回放配置 ──────────────────────────────────────────────────────────

/// 回放配置选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOptions {
    /// 严格模式：true=内容完全匹配；false=相似度对比
    pub strict: bool,
    /// 内容相似度阈值（0.0-1.0，非 strict 模式下生效）
    pub content_similarity_threshold: f64,
    /// 最大允许偏差数（超过则 passed=false，0=零容忍）
    pub max_deviations: usize,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self { strict: true, content_similarity_threshold: 0.8, max_deviations: 0 }
    }
}

// ── 回放报告 ──────────────────────────────────────────────────────────

/// 单条轨迹回放报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayReport {
    /// 被回放的 golden trajectory ID
    pub trajectory_id: String,
    /// 是否通过回归（deviations 数 <= max_deviations 且 outcome 一致）
    pub passed: bool,
    /// 回放评分（0.0-1.0，基于 ReplayContextExt::evaluate）
    pub evaluation: f64,
    /// golden trajectory 总步数
    pub total_steps: usize,
    /// 匹配成功的步数
    pub matched_steps: usize,
    /// 检测到的偏差列表
    pub deviations: Vec<StepDeviation>,
    /// 回放耗时（毫秒）
    pub duration_ms: u64,
    /// 开始时间戳（毫秒）
    pub started_at: i64,
    /// 结束时间戳（毫秒）
    pub finished_at: i64,
}

// ── Golden Trajectory 标记 ────────────────────────────────────────────

/// 标记为回归基准的 golden trajectory。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenTrajectory {
    /// golden 轨迹
    pub trajectory: Trajectory,
    /// 期望的执行结果
    pub expected_outcome: TrajectoryOutcome,
    /// 标签（如 "code_generation"、"refactor"、"bug_fix"）
    pub tags: Vec<String>,
}

impl GoldenTrajectory {
    pub fn new(trajectory: Trajectory, expected_outcome: TrajectoryOutcome) -> Self {
        Self { trajectory, expected_outcome, tags: Vec::new() }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

// ── 回归样本集 ────────────────────────────────────────────────────────

/// 回归样本集 — 一组 golden trajectory 用于批量回放。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionSuite {
    /// 样本集名称
    pub name: String,
    /// golden trajectory 列表
    pub golden_trajectories: Vec<GoldenTrajectory>,
    /// 回放配置
    pub options: ReplayOptions,
}

impl RegressionSuite {
    pub fn new(name: String, options: ReplayOptions) -> Self {
        Self { name, golden_trajectories: Vec::new(), options }
    }

    pub fn with_golden(mut self, golden: GoldenTrajectory) -> Self {
        self.golden_trajectories.push(golden);
        self
    }

    pub fn len(&self) -> usize {
        self.golden_trajectories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.golden_trajectories.is_empty()
    }
}

// ── 回归样本集结果 ────────────────────────────────────────────────────

/// 批量回放结果汇总。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionSuiteResult {
    /// 样本集名称
    pub suite_name: String,
    /// 总数
    pub total: usize,
    /// 通过数
    pub passed: usize,
    /// 失败数
    pub failed: usize,
    /// 每条轨迹的回放报告
    pub reports: Vec<ReplayReport>,
    /// 平均评分
    pub avg_evaluation: f64,
    /// 总耗时（毫秒）
    pub duration_ms: u64,
}

// ── ReplayExecutor trait ──────────────────────────────────────────────

/// 轨迹回放执行器契约。
///
/// 消费者（agent/runtime）通过此 trait 执行回归测试，无需依赖 trajectory crate。
#[async_trait::async_trait]
pub trait ReplayExecutor: Send + Sync {
    /// 回放单条 golden trajectory，生成回归报告。
    ///
    /// `current` 是当前行为轨迹（由调用方录制或构造），
    /// 与 `golden` 对比生成偏差报告。
    async fn replay(
        &self,
        golden: &Trajectory,
        current: &Trajectory,
        options: &ReplayOptions,
    ) -> Result<ReplayReport, String>;

    /// 批量回放回归样本集。
    ///
    /// 默认实现逐个回放，子类型可覆盖为并行执行。
    async fn replay_suite(
        &self,
        suite: &RegressionSuite,
        current_trajectories: &[Trajectory],
    ) -> Result<RegressionSuiteResult, String> {
        if suite.golden_trajectories.len() != current_trajectories.len() {
            return Err(format!(
                "golden_trajectories count ({}) != current_trajectories count ({})",
                suite.golden_trajectories.len(),
                current_trajectories.len()
            ));
        }

        let started = chrono::Utc::now().timestamp_millis();
        let mut reports = Vec::with_capacity(suite.golden_trajectories.len());
        let mut passed = 0usize;

        for (golden, current) in suite.golden_trajectories.iter().zip(current_trajectories.iter()) {
            let report = self.replay(&golden.trajectory, current, &suite.options).await?;
            if report.passed {
                passed += 1;
            }
            reports.push(report);
        }

        let finished = chrono::Utc::now().timestamp_millis();
        let total = suite.golden_trajectories.len();
        let failed = total - passed;
        let avg_evaluation = if reports.is_empty() {
            0.0
        } else {
            reports.iter().map(|r| r.evaluation).sum::<f64>() / reports.len() as f64
        };

        Ok(RegressionSuiteResult {
            suite_name: suite.name.clone(),
            total,
            passed,
            failed,
            reports,
            avg_evaluation,
            duration_ms: (finished - started) as u64,
        })
    }
}

// ── 纯函数：轨迹对比逻辑 ──────────────────────────────────────────────

/// 对比 golden 和 current 轨迹，生成偏差列表。
///
/// 这是 `ReplayExecutor::replay` 的核心逻辑，抽取为纯函数便于独立测试。
/// 不依赖任何外部资源，不执行真实工具。
pub fn compare_trajectories(
    golden: &Trajectory,
    current: &Trajectory,
    options: &ReplayOptions,
) -> Vec<StepDeviation> {
    let mut deviations = Vec::new();
    let golden_len = golden.steps.len();
    let current_len = current.steps.len();

    // 步数不匹配
    if golden_len != current_len {
        deviations.push(StepDeviation {
            step_index: golden_len.min(current_len),
            kind: DeviationKind::StepCountMismatch,
            golden_value: golden_len.to_string(),
            current_value: current_len.to_string(),
            detail: format!("golden has {golden_len} steps but current has {current_len} steps"),
        });
    }

    // 逐 step 对比
    for (idx, (g_step, c_step)) in golden.steps.iter().zip(current.steps.iter()).enumerate() {
        compare_step(idx, g_step, c_step, options, &mut deviations);
    }

    deviations
}

/// 对比单个步骤，将偏差追加到 `deviations`。
fn compare_step(
    step_index: usize,
    golden: &TrajectoryStep,
    current: &TrajectoryStep,
    options: &ReplayOptions,
    deviations: &mut Vec<StepDeviation>,
) {
    // 角色对比
    if golden.role != current.role {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::RoleMismatch,
            golden_value: format!("{:?}", golden.role),
            current_value: format!("{:?}", current.role),
            detail: format!(
                "step {step_index} role mismatch: golden={:?} current={:?}",
                golden.role, current.role
            ),
        });
    }

    // 内容对比
    if !content_matches(&golden.content, &current.content, options) {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::ContentMismatch,
            golden_value: golden.content.chars().take(200).collect(),
            current_value: current.content.chars().take(200).collect(),
            detail: format!("step {step_index} content mismatch"),
        });
    }

    // tool_calls 对比
    compare_tool_calls(step_index, golden, current, deviations);

    // tool_results 对比（结构对比，不执行真实工具）
    compare_tool_results(step_index, golden, current, deviations);
}

/// 内容匹配检查：strict 模式完全匹配，否则相似度对比。
fn content_matches(golden: &str, current: &str, options: &ReplayOptions) -> bool {
    if options.strict {
        golden == current
    } else {
        let similarity = text_similarity(golden, current);
        similarity >= options.content_similarity_threshold
    }
}

/// 简单的文本相似度（基于 Jaccard 字符 bigram）。
///
/// 返回 0.0-1.0 的相似度分数。空字符串视为完全匹配。
fn text_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let bigrams_a: std::collections::HashSet<&str> =
        a.as_bytes().windows(2).filter_map(|w| std::str::from_utf8(w).ok()).collect();
    let bigrams_b: std::collections::HashSet<&str> =
        b.as_bytes().windows(2).filter_map(|w| std::str::from_utf8(w).ok()).collect();

    if bigrams_a.is_empty() && bigrams_b.is_empty() {
        return 1.0;
    }

    let intersection = bigrams_a.intersection(&bigrams_b).count();
    let union = bigrams_a.union(&bigrams_b).count();

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

/// 对比 tool_calls。
fn compare_tool_calls(
    step_index: usize,
    golden: &TrajectoryStep,
    current: &TrajectoryStep,
    deviations: &mut Vec<StepDeviation>,
) {
    let golden_calls = golden.tool_calls.as_deref().unwrap_or(&[]);
    let current_calls = current.tool_calls.as_deref().unwrap_or(&[]);

    // golden 有 tool_calls 但 current 缺少
    if !golden_calls.is_empty() && current_calls.is_empty() {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::ToolCallMissing,
            golden_value: format!("{} tool calls", golden_calls.len()),
            current_value: "0 tool calls".to_string(),
            detail: format!("step {step_index} golden has tool_calls but current has none"),
        });
        return;
    }

    // current 有多余 tool_calls
    if golden_calls.is_empty() && !current_calls.is_empty() {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::ToolCallExtra,
            golden_value: "0 tool calls".to_string(),
            current_value: format!("{} tool calls", current_calls.len()),
            detail: format!("step {step_index} current has extra tool_calls not in golden"),
        });
        return;
    }

    // 逐个对比
    for (i, (g, c)) in golden_calls.iter().zip(current_calls.iter()).enumerate() {
        if g.name != c.name {
            deviations.push(StepDeviation {
                step_index,
                kind: DeviationKind::ToolCallNameMismatch,
                golden_value: g.name.clone(),
                current_value: c.name.clone(),
                detail: format!("step {step_index} tool_call[{i}] name mismatch"),
            });
        }
        if g.arguments != c.arguments {
            deviations.push(StepDeviation {
                step_index,
                kind: DeviationKind::ToolCallArgumentsMismatch,
                golden_value: g.arguments.chars().take(200).collect(),
                current_value: c.arguments.chars().take(200).collect(),
                detail: format!("step {step_index} tool_call[{i}] arguments mismatch"),
            });
        }
    }

    // golden 比 current 多
    if golden_calls.len() > current_calls.len() {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::ToolCallMissing,
            golden_value: format!("{} tool calls", golden_calls.len()),
            current_value: format!("{} tool calls", current_calls.len()),
            detail: format!(
                "step {step_index} golden has {} more tool_calls than current",
                golden_calls.len() - current_calls.len()
            ),
        });
    }

    // current 比 golden 多
    if current_calls.len() > golden_calls.len() {
        deviations.push(StepDeviation {
            step_index,
            kind: DeviationKind::ToolCallExtra,
            golden_value: format!("{} tool calls", golden_calls.len()),
            current_value: format!("{} tool calls", current_calls.len()),
            detail: format!(
                "step {step_index} current has {} extra tool_calls not in golden",
                current_calls.len() - golden_calls.len()
            ),
        });
    }
}

/// 对比 tool_results（结构对比）。
fn compare_tool_results(
    step_index: usize,
    golden: &TrajectoryStep,
    current: &TrajectoryStep,
    deviations: &mut Vec<StepDeviation>,
) {
    let golden_results = golden.tool_results.as_deref().unwrap_or(&[]);
    let current_results = current.tool_results.as_deref().unwrap_or(&[]);

    for (i, (g, c)) in golden_results.iter().zip(current_results.iter()).enumerate() {
        // is_error 状态对比
        if g.is_error != c.is_error {
            deviations.push(StepDeviation {
                step_index,
                kind: DeviationKind::ToolResultError,
                golden_value: format!("is_error={}", g.is_error),
                current_value: format!("is_error={}", c.is_error),
                detail: format!(
                    "step {step_index} tool_result[{i}] error state mismatch: golden is_error={} current is_error={}",
                    g.is_error, c.is_error
                ),
            });
            continue;
        }

        // output 对比（非 error 情况下）
        if !g.is_error && g.output != c.output {
            deviations.push(StepDeviation {
                step_index,
                kind: DeviationKind::ToolResultMismatch,
                golden_value: g.output.chars().take(200).collect(),
                current_value: c.output.chars().take(200).collect(),
                detail: format!("step {step_index} tool_result[{i}] output mismatch"),
            });
        }
    }
}

/// 根据偏差列表和配置生成回放报告。
///
/// 调用方负责传入 `deviations`（由 `compare_trajectories` 生成），
/// 此函数计算评分、matched_steps、passed 等汇总字段。
pub fn build_replay_report(
    golden: &Trajectory,
    deviations: Vec<StepDeviation>,
    options: &ReplayOptions,
    started_at: i64,
    finished_at: i64,
) -> ReplayReport {
    let total_steps = golden.steps.len();
    let deviation_count = deviations.len();

    // 计算匹配步数：无偏差的步骤数
    let deviated_steps: std::collections::HashSet<usize> =
        deviations.iter().map(|d| d.step_index).collect();
    let matched_steps =
        golden.steps.iter().enumerate().filter(|(idx, _)| !deviated_steps.contains(idx)).count();

    // 评分：基于偏差数和匹配率
    let match_rate = if total_steps > 0 {
        matched_steps as f64 / total_steps as f64
    } else {
        1.0
    };
    let deviation_penalty = (deviation_count as f64 * 0.1).min(0.5);
    let evaluation = (match_rate - deviation_penalty).clamp(0.0, 1.0);

    // passed 判定：偏差数 <= max_deviations
    let passed = deviation_count <= options.max_deviations;

    ReplayReport {
        trajectory_id: golden.id.clone(),
        passed,
        evaluation,
        total_steps,
        matched_steps,
        deviations,
        duration_ms: (finished_at - started_at) as u64,
        started_at,
        finished_at,
    }
}

// ── 单元测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory_types::{
        MessageRole, ToolCall, TrajectoryQuality, TrajectoryStep, TrajectoryToolResult,
    };

    fn make_step(role: MessageRole, content: &str) -> TrajectoryStep {
        TrajectoryStep {
            timestamp_ms: 0,
            role,
            content: content.to_string(),
            reasoning: None,
            tool_calls: None,
            tool_results: None,
        }
    }

    fn make_trajectory(id: &str, steps: Vec<TrajectoryStep>) -> Trajectory {
        Trajectory {
            id: id.to_string(),
            session_id: "test_session".to_string(),
            user_id: "test_user".to_string(),
            agent_name: None,
            topic: "test".to_string(),
            summary: "test trajectory".to_string(),
            outcome: TrajectoryOutcome::Success,
            duration_ms: 1000,
            quality: TrajectoryQuality::default(),
            value_score: 0.8,
            patterns: vec![],
            steps,
            rewards: vec![],
            created_at: chrono::Utc::now(),
            replay_count: 0,
            last_replay_at: None,
        }
    }

    #[test]
    fn test_compare_identical_trajectories() {
        let steps = vec![
            make_step(MessageRole::User, "hello"),
            make_step(MessageRole::Assistant, "hi there"),
        ];
        let golden = make_trajectory("g1", steps.clone());
        let current = make_trajectory("c1", steps);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.is_empty(), "identical trajectories should have no deviations");

        let report = build_replay_report(&golden, deviations, &options, 1000, 1001);
        assert!(report.passed);
        assert_eq!(report.total_steps, 2);
        assert_eq!(report.matched_steps, 2);
        assert!((report.evaluation - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compare_role_mismatch() {
        let golden = make_trajectory("g1", vec![make_step(MessageRole::User, "hello")]);
        let current = make_trajectory("c1", vec![make_step(MessageRole::Assistant, "hello")]);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].kind, DeviationKind::RoleMismatch);
    }

    #[test]
    fn test_compare_content_mismatch_strict() {
        let golden = make_trajectory("g1", vec![make_step(MessageRole::User, "hello world")]);
        let current = make_trajectory("c1", vec![make_step(MessageRole::User, "hello")]);
        let options = ReplayOptions::default(); // strict=true

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.iter().any(|d| d.kind == DeviationKind::ContentMismatch));
    }

    #[test]
    fn test_compare_content_match_non_strict() {
        let golden = make_trajectory("g1", vec![make_step(MessageRole::User, "hello world")]);
        let current = make_trajectory("c1", vec![make_step(MessageRole::User, "hello world")]);
        let options =
            ReplayOptions { strict: false, content_similarity_threshold: 0.5, max_deviations: 0 };

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.is_empty(), "identical content should match in non-strict mode");
    }

    #[test]
    fn test_compare_step_count_mismatch() {
        let golden = make_trajectory(
            "g1",
            vec![
                make_step(MessageRole::User, "a"),
                make_step(MessageRole::Assistant, "b"),
                make_step(MessageRole::User, "c"),
            ],
        );
        let current = make_trajectory("c1", vec![make_step(MessageRole::User, "a")]);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.iter().any(|d| d.kind == DeviationKind::StepCountMismatch));
    }

    #[test]
    fn test_compare_tool_call_missing() {
        let golden_step = TrajectoryStep {
            timestamp_ms: 0,
            role: MessageRole::Assistant,
            content: "calling tool".to_string(),
            reasoning: None,
            tool_calls: Some(vec![ToolCall {
                id: "tc1".to_string(),
                name: "read_file".to_string(),
                arguments: r#"{"path":"test.rs"}"#.to_string(),
            }]),
            tool_results: None,
        };
        let current_step = make_step(MessageRole::Assistant, "calling tool");

        let golden = make_trajectory("g1", vec![golden_step]);
        let current = make_trajectory("c1", vec![current_step]);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.iter().any(|d| d.kind == DeviationKind::ToolCallMissing));
    }

    #[test]
    fn test_compare_tool_call_name_mismatch() {
        let make_tc = |name: &str| TrajectoryStep {
            timestamp_ms: 0,
            role: MessageRole::Assistant,
            content: "calling tool".to_string(),
            reasoning: None,
            tool_calls: Some(vec![ToolCall {
                id: "tc1".to_string(),
                name: name.to_string(),
                arguments: "{}".to_string(),
            }]),
            tool_results: None,
        };

        let golden = make_trajectory("g1", vec![make_tc("read_file")]);
        let current = make_trajectory("c1", vec![make_tc("write_file")]);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.iter().any(|d| d.kind == DeviationKind::ToolCallNameMismatch));
    }

    #[test]
    fn test_compare_tool_result_error_mismatch() {
        let make_step_with_result = |is_error: bool| TrajectoryStep {
            timestamp_ms: 0,
            role: MessageRole::Tool,
            content: "".to_string(),
            reasoning: None,
            tool_calls: None,
            tool_results: Some(vec![TrajectoryToolResult {
                tool_use_id: "tc1".to_string(),
                tool_name: "read_file".to_string(),
                output: "content".to_string(),
                is_error,
            }]),
        };

        let golden = make_trajectory("g1", vec![make_step_with_result(false)]);
        let current = make_trajectory("c1", vec![make_step_with_result(true)]);
        let options = ReplayOptions::default();

        let deviations = compare_trajectories(&golden, &current, &options);
        assert!(deviations.iter().any(|d| d.kind == DeviationKind::ToolResultError));
    }

    #[test]
    fn test_build_report_passed_with_max_deviations() {
        let golden = make_trajectory("g1", vec![make_step(MessageRole::User, "a")]);
        let deviations = vec![StepDeviation {
            step_index: 0,
            kind: DeviationKind::ContentMismatch,
            golden_value: "a".to_string(),
            current_value: "b".to_string(),
            detail: "mismatch".to_string(),
        }];
        let options = ReplayOptions {
            max_deviations: 1, // 允许 1 个偏差
            ..Default::default()
        };

        let report = build_replay_report(&golden, deviations, &options, 0, 1);
        assert!(report.passed, "should pass with 1 deviation <= max_deviations=1");
        assert_eq!(report.matched_steps, 0);
    }

    #[test]
    fn test_build_report_failed_exceeds_max_deviations() {
        let golden = make_trajectory("g1", vec![make_step(MessageRole::User, "a")]);
        let deviations = vec![StepDeviation {
            step_index: 0,
            kind: DeviationKind::ContentMismatch,
            golden_value: "a".to_string(),
            current_value: "b".to_string(),
            detail: "mismatch".to_string(),
        }];
        let options = ReplayOptions::default(); // max_deviations=0

        let report = build_replay_report(&golden, deviations, &options, 0, 1);
        assert!(!report.passed, "should fail with 1 deviation > max_deviations=0");
    }

    #[test]
    fn test_text_similarity_identical() {
        let sim = text_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_text_similarity_empty() {
        assert!((text_similarity("", "") - 1.0).abs() < 0.01);
        assert!((text_similarity("a", "") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_regression_suite_builder() {
        let traj = make_trajectory("g1", vec![make_step(MessageRole::User, "hello")]);
        let golden = GoldenTrajectory::new(traj, TrajectoryOutcome::Success)
            .with_tags(vec!["test".to_string()]);
        assert_eq!(golden.tags.len(), 1);

        let suite = RegressionSuite::new("test_suite".to_string(), ReplayOptions::default())
            .with_golden(golden);
        assert_eq!(suite.len(), 1);
        assert!(!suite.is_empty());
        assert_eq!(suite.name, "test_suite");
    }
}
