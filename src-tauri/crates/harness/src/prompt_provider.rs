// SPDX-License-Identifier: AGPL-3.0-only

//! Prompt provider trait — abstracts the multi-language prompt registry.
//!
//! This trait decouples runtime-core (and other consumers) from axagent-kit,
//! allowing them to resolve localized prompt templates through the harness
//! without depending on the implementation crate.

use std::collections::HashMap;

/// Language identifier for prompt resolution.
///
/// This enum is owned by the harness to avoid consumers depending on
/// axagent-kit directly. Implementation crates map their internal
/// language representation to/from this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptLang {
    /// Simplified Chinese (default)
    ZhCN,
    /// English (fallback)
    EnUS,
}

/// Provides localized prompt templates.
///
/// Implementations (typically axagent-kit's PromptRegistry) look up
/// templates by key + language. Consumers (runtime-core, agent, etc.)
/// depend on this trait through the harness.
pub trait PromptProvider: Send + Sync {
    /// Get a prompt template by key and language.
    ///
    /// Returns the static string for the given key.
    /// If the key is not found, implementations should return "".
    fn get(&self, key: &str, lang: PromptLang) -> &'static str;

    /// Get a prompt template with simple positional argument substitution.
    ///
    /// Arguments are substituted for `{0}`, `{1}`, `{2}`, etc.
    fn format(&self, key: &str, lang: PromptLang, args: &[&str]) -> String {
        let template = self.get(key, lang);
        let mut result = template.to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        result
    }

    /// Get all language variants for a given key.
    fn get_all_languages(&self, key: &str) -> HashMap<String, &'static str>;
}

/// A no-op PromptProvider that returns empty strings for all keys.

/// Static PromptProvider backed by a compiled-in registry.
///
/// Used when a real registry is available at compile time
/// (e.g., from axagent-kit's PromptRegistry).
pub struct StaticPromptProvider {
    get_fn: fn(&str, PromptLang) -> &'static str,
    all_fn: fn(&str) -> HashMap<String, &'static str>,
}

impl StaticPromptProvider {
    pub fn new(
        get_fn: fn(&str, PromptLang) -> &'static str,
        all_fn: fn(&str) -> HashMap<String, &'static str>,
    ) -> Self {
        Self { get_fn, all_fn }
    }
}

impl PromptProvider for StaticPromptProvider {
    fn get(&self, key: &str, lang: PromptLang) -> &'static str {
        (self.get_fn)(key, lang)
    }

    fn get_all_languages(&self, key: &str) -> HashMap<String, &'static str> {
        (self.all_fn)(key)
    }
}

// Re-export NoopPromptProvider from test_support for runtime-core usage
pub use crate::test_support::NoopPromptProvider;
