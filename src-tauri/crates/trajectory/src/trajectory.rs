// SPDX-License-Identifier: AGPL-3.0-only

//! 核心轨迹数据结构 —— 由 axagent-harness 定义

// DTO + 固有方法 + 构造器 由 axagent-harness 提供
pub use axagent_harness::trajectory_types::{
    ExportFormat, MessageRole, RLTrainingEntry, RewardSignal, RewardType, ToolCall, ToolResult,
    TrainingConfig, Trajectory, TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern,
    TrajectoryQuality, TrajectoryQuery, TrajectoryStep,
};
