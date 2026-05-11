//! Re-exported from axagent-runtime-core.
pub use axagent_runtime_core::compact::{
    adaptive_compaction_config, cleanup_task_boundary, compact_session, decay_weight,
    detect_task_boundary, estimate_message_tokens, estimate_session_tokens,
    evaluate_compact_threshold, format_compact_summary, get_compact_continuation_message,
    should_compact, smart_compact, summarize_turn, CompactionConfig, CompactionResult,
};
