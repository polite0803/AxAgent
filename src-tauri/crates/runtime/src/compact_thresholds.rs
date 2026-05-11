//! Re-exported from axagent-runtime-core.
pub use axagent_runtime_core::compact_thresholds::{
    recommended_compaction_config, should_auto_compact, should_reactive_compact,
    AutoCompactTracking, CompactThresholdState, AUTOCOMPACT_BUFFER_TOKENS,
    ERROR_THRESHOLD_BUFFER_TOKENS, MANUAL_COMPACT_BUFFER_TOKENS,
    MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES, WARNING_THRESHOLD_BUFFER_TOKENS,
};
