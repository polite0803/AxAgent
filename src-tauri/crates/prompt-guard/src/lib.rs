//! Prompt injection defense crate.
//!
//! Provides a 4-layer filtering pipeline that sanitizes user input
//! and labels external data before it enters the LLM context.

pub mod config;
pub mod detectors;
pub mod pipeline;
pub mod trust_labels;
pub mod wrappers;

pub use config::GuardConfig;
pub use pipeline::PromptGuardPipeline;
