//! 核心轨迹数据结构 —— 由 axagent-harness 定义

// DTO + 固有方法 + 构造器 由 axagent-harness 提供
pub use axagent_harness::trajectory_types::{
    CompressedTrajectory, ExportFormat, MessageRole, RLTrainingEntry, ReplayContext,
    RewardSignal, RewardType, ToolCall, ToolResult, Trajectory, TrajectoryBuilder,
    TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern, TrajectoryQuality,
    TrajectoryQuery, TrajectoryStep, TrainingConfig,
};
