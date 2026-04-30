//! General-purpose engine — lightweight runtime for non-code tasks.
//!
//! Handles daily chat, document processing, system operations, tool
//! invocations, and message gateway routing. Does NOT load code-specific
//! modules (LSP, AST index, file index, code search pipeline), keeping
//! the memory footprint low for non-coding scenarios.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A general-purpose task category that the engine can handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralTaskCategory {
    DailyChat,
    DocumentProcessing,
    FileOperation,
    SystemTool,
    WebSearch,
    DataAnalysis,
    MessageGateway,
    Unknown,
}

/// Result of a general engine task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralTaskResult {
    pub category: GeneralTaskCategory,
    pub summary: String,
    pub tool_calls: Vec<String>,
    pub estimated_tokens: u32,
    pub duration_ms: u64,
}

/// Configuration for the general engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralEngineConfig {
    pub max_iterations: usize,
    pub enable_document_parser: bool,
    pub enable_web_search: bool,
    pub enable_system_tools: bool,
    pub enable_message_gateway: bool,
    pub auto_compaction_threshold_tokens: u32,
}

impl Default for GeneralEngineConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            enable_document_parser: true,
            enable_web_search: true,
            enable_system_tools: true,
            enable_message_gateway: true,
            auto_compaction_threshold_tokens: 100_000,
        }
    }
}

/// The general-purpose engine — handles all non-code tasks.
pub struct GeneralEngine {
    config: GeneralEngineConfig,
    active_capabilities: HashMap<String, bool>,
}

impl GeneralEngine {
    pub fn new(config: GeneralEngineConfig) -> Self {
        let mut capabilities = HashMap::new();
        capabilities.insert("document_parser".to_string(), config.enable_document_parser);
        capabilities.insert("web_search".to_string(), config.enable_web_search);
        capabilities.insert("system_tools".to_string(), config.enable_system_tools);
        capabilities.insert("message_gateway".to_string(), config.enable_message_gateway);

        Self {
            config,
            active_capabilities: capabilities,
        }
    }

    /// Detect the type of general task from user input.
    pub fn classify_task(&self, input: &str) -> GeneralTaskCategory {
        let lowered = input.to_lowercase();

        if lowered.contains("document")
            || lowered.contains("pdf")
            || lowered.contains("docx")
            || lowered.contains("excel")
            || lowered.contains("powerpoint")
            || lowered.contains("parse")
        {
            GeneralTaskCategory::DocumentProcessing
        } else if lowered.contains("search")
            || lowered.contains("find")
            || lowered.contains("lookup")
            || lowered.contains("google")
            || lowered.contains("web")
        {
            GeneralTaskCategory::WebSearch
        } else if lowered.contains("file")
            || lowered.contains("folder")
            || lowered.contains("directory")
            || lowered.contains("rename")
            || lowered.contains("delete")
            || lowered.contains("move")
        {
            GeneralTaskCategory::FileOperation
        } else if lowered.contains("system")
            || lowered.contains("command")
            || lowered.contains("terminal")
            || lowered.contains("bash")
            || lowered.contains("shell")
            || lowered.contains("execute")
        {
            GeneralTaskCategory::SystemTool
        } else if lowered.contains("analyze")
            || lowered.contains("data")
            || lowered.contains("statistics")
            || lowered.contains("chart")
            || lowered.contains("graph")
        {
            GeneralTaskCategory::DataAnalysis
        } else if lowered.contains("whatsapp")
            || lowered.contains("telegram")
            || lowered.contains("slack")
            || lowered.contains("discord")
            || lowered.contains("wechat")
        {
            GeneralTaskCategory::MessageGateway
        } else {
            GeneralTaskCategory::DailyChat
        }
    }

    /// Check if a capability is enabled.
    pub fn is_capability_enabled(&self, capability: &str) -> bool {
        self.active_capabilities
            .get(capability)
            .copied()
            .unwrap_or(false)
    }

    /// Enable or disable a specific capability.
    pub fn set_capability(&mut self, capability: &str, enabled: bool) {
        self.active_capabilities
            .insert(capability.to_string(), enabled);
    }

    /// Get a summary of active capabilities.
    pub fn active_capabilities_summary(&self) -> Vec<String> {
        self.active_capabilities
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &GeneralEngineConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_document_task() {
        let engine = GeneralEngine::new(GeneralEngineConfig::default());
        assert_eq!(
            engine.classify_task("Parse this PDF document"),
            GeneralTaskCategory::DocumentProcessing
        );
    }

    #[test]
    fn test_classify_chat_task() {
        let engine = GeneralEngine::new(GeneralEngineConfig::default());
        assert_eq!(
            engine.classify_task("Hello, how are you?"),
            GeneralTaskCategory::DailyChat
        );
    }

    #[test]
    fn test_capability_toggle() {
        let mut engine = GeneralEngine::new(GeneralEngineConfig::default());
        assert!(engine.is_capability_enabled("web_search"));
        engine.set_capability("web_search", false);
        assert!(!engine.is_capability_enabled("web_search"));
    }
}
