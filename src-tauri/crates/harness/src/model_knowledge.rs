// SPDX-License-Identifier: AGPL-3.0-only

//! Model knowledge provider — dependency inversion boundary for model metadata.
//!
//! `axagent-kit` implements this trait; `axagent-providers` consumes it
//! without depending on `kit` (or `core` which re-exports kit).

/// Provider of model knowledge (context window, etc.).
///
/// Implementations supply model metadata that providers need
/// (e.g. context window sizes for different models).
pub trait ModelKnowledgeProvider: Send + Sync {
    /// Return the context window size (max input tokens) for a known model.
    /// Returns `None` if the model is not recognized.
    fn get_model_context_window(&self, model_id: &str) -> Option<u32>;
}
