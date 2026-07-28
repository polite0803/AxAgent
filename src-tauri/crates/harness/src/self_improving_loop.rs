// SPDX-License-Identifier: AGPL-3.0-only

//! 自改进循环（Self-Improving Loop）契约层
//!
//! 定义"生成 → 自我评估 → 识别不足 → 注入改进提示 → 重新生成"的回合制契约。
//! 基座（agent crate）提供通用执行器 [`SelfImprovementExecutor`]，
//! 业务层（AxInvest / AxOPC / AxSim 等）通过实现 [`SelfImprovingRound`] trait
//! 注入领域评估逻辑，复用基座的迭代策略与收敛/逃逸机制。
//!
//! 设计原则：仅 trait + 纯 DTO，不含业务逻辑。所有实现都在下游 crate。

use serde::{Deserialize, Serialize};

/// 一轮"执行 + 评估 + 决策"的完整回合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResult {
    /// 回合序号（由执行器填充，业务层实现时填 0 即可）
    pub round: u32,
    /// 本轮最终输出文本
    pub output: String,
    /// 本轮评估结果（由执行器在评估后回填）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<RoundEvaluation>,
    /// 本轮执行轨迹（think/act/observe/reflect 步骤摘要）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<RoundStep>,
}

/// 单步轨迹摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundStep {
    pub index: u32,
    /// 步骤类型："think" | "act" | "observe" | "reflect" 等
    pub kind: String,
    pub summary: String,
    pub tokens_used: u32,
}

/// 一轮输出的自我评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundEvaluation {
    /// 质量分数，范围 0.0 ~ 1.0，越高越好
    pub score: f64,
    /// 评估置信度，范围 0.0 ~ 1.0
    pub confidence: f64,
    /// 识别到的不足/遗漏（驱动下一轮 Refine）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<String>,
    /// 本轮的亮点/可复用经验
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strengths: Vec<String>,
    /// LLM 原始评估文本（用于调试/审计）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw_assessment: String,
    /// 下一轮改进方向（由 executor 在 Refine 分支从 `NextAction::Refine { direction }`
    /// 提取并写入；`execute_round` 通过 `prev_evaluation.next_direction` 读取，
    /// 与 `gaps` 一起作为下一轮的改进提示）。Redirect 分支不填充此字段
    /// （`prev_evaluation` 被置 None，下一轮从零开始）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_direction: Option<String>,
}

/// 下一轮的动作决策
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NextAction {
    /// 接受当前输出，结束循环
    Accept,
    /// 根据给定方向继续改进（下一轮 execute_round 会通过 prev_evaluation 感知）
    Refine { direction: String },
    /// 完全换方向重做（prev_evaluation 重置，下一轮从零开始）
    Redirect { reason: String },
    /// 需要人类介入
    Escalate { message: String },
}

/// 自改进循环契约 —— 由业务层实现，基座提供通用执行器
///
/// 实现者只需关注"如何执行一轮"和"如何评估一轮"，
/// 收敛检测、最大轮数限制、连续无进展逃逸等策略由
/// [`crate::self_improving_loop::SelfImprovementExecutor`]（agent crate）统一管理。
#[async_trait::async_trait]
pub trait SelfImprovingRound: Send + Sync {
    /// 执行一轮：接收任务描述 + 上一轮评估（首轮为 None），输出本轮结果
    async fn execute_round(
        &mut self,
        task: &str,
        prev_evaluation: Option<&RoundEvaluation>,
    ) -> Result<RoundResult, Box<dyn std::error::Error + Send>>;

    /// 自我评估本轮输出质量
    async fn evaluate_round(
        &self,
        task: &str,
        result: &RoundResult,
    ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>>;

    /// 根据评估决定下一步动作
    async fn decide_next(
        &self,
        task: &str,
        result: &RoundResult,
        evaluation: &RoundEvaluation,
    ) -> Result<NextAction, Box<dyn std::error::Error + Send>>;
}

/// 自改进循环相关错误
#[derive(Debug, thiserror::Error)]
pub enum LoopError {
    /// 超过最大执行轮数
    #[error("Exceeded maximum rounds: {0}")]
    MaxRoundsExceeded(u32),
    /// 业务层主动请求人类介入
    #[error("Escalated to human: {0}")]
    Escalated(String),
    /// 内部错误（业务层 trait 方法返回的 Box<dyn Error>）
    #[error("Internal error: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_result_serialization() {
        let result = RoundResult {
            round: 1,
            output: "test output".to_string(),
            evaluation: None,
            trace: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: RoundResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.round, 1);
        assert_eq!(deserialized.output, "test output");
        assert!(deserialized.evaluation.is_none());
    }

    #[test]
    fn test_round_evaluation_next_direction_default_none() {
        // 旧版本 JSON（无 next_direction 字段）反序列化时应默认为 None
        let json_old = r#"{
            "score": 0.8,
            "confidence": 0.7,
            "gaps": [],
            "strengths": []
        }"#;
        let eval: RoundEvaluation = serde_json::from_str(json_old).unwrap();
        assert_eq!(eval.score, 0.8);
        assert!(eval.next_direction.is_none());
    }

    #[test]
    fn test_next_action_equality() {
        assert_eq!(NextAction::Accept, NextAction::Accept);
        assert_ne!(NextAction::Accept, NextAction::Escalate { message: "x".into() });
    }

    #[test]
    fn test_loop_error_display() {
        let err = LoopError::MaxRoundsExceeded(5);
        assert!(err.to_string().contains("5"));
        let err = LoopError::Escalated("manual".to_string());
        assert!(err.to_string().contains("manual"));
    }
}
