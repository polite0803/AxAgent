//! Prompt injection defense crate.
//!
//! Provides a 4-layer filtering pipeline that sanitizes user input
//! and labels external data before it enters the LLM context.

pub mod config;
pub mod detectors;
pub mod pipeline;
pub mod trust_labels;
pub mod wrappers;

pub use config::{DetectionResult, GuardConfig, GuardMode};
pub use pipeline::PromptGuardPipeline;

// ── Harness PromptGuard trait 实现 ──

use axagent_harness::PromptGuard as HarnessPromptGuard;

impl HarnessPromptGuard for PromptGuardPipeline {
    fn process_user_input(&self, input: &str) -> Result<String, String> {
        self.process_user_input(input)
    }

    fn process_external_data(&self, content: &str, source_label: &str, source_id: &str) -> String {
        let source_type = trust_labels::SourceType::from_label(source_label);
        self.process_external_data(content, source_type, source_id)
    }
}
