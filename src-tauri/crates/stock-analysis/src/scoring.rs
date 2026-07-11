//! 100 分制客观评分引擎（已下沉到 axagent-astock-data crate）
//!
//! 保持向后兼容的 re-export 层。

pub use axagent_astock_data::scoring::{ObjectiveScore, ScoreBands, ScoringEngine, ScoringWeights};
