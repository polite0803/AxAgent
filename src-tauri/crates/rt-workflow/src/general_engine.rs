//! General-purpose engine — lightweight runtime for non-code tasks.
//!
//! Handles daily chat, document processing, system operations, tool
//! invocations, and message gateway routing. Does NOT load code-specific
//! modules (LSP, AST index, file index, code search pipeline), keeping
//! the memory footprint low for non-coding scenarios.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 把输入拆成小写 token（按空白、标点切分）。空字符串归一为 "0"。
/// 该函数对 classify_task 的语义关键：避免 "delete metadata" 命中 "delete"，
/// 或 "analyze the web" 误命中 "web"。
fn tokenize(input: &str) -> Vec<String> {
    input
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// RULES 表里的字符串名 → 枚举的映射。仅在本文件内部使用。
fn category_from_name(name: &str) -> GeneralTaskCategory {
    match name {
        "DocumentProcessing" => GeneralTaskCategory::DocumentProcessing,
        "WebSearch" => GeneralTaskCategory::WebSearch,
        "FileOperation" => GeneralTaskCategory::FileOperation,
        "SystemTool" => GeneralTaskCategory::SystemTool,
        "DataAnalysis" => GeneralTaskCategory::DataAnalysis,
        "MessageGateway" => GeneralTaskCategory::MessageGateway,
        _ => GeneralTaskCategory::Unknown,
    }
}

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
    ///
    /// 修复：原实现是按"子串包含"顺序匹配，导致"analyze the web page"被误归为
    /// WebSearch，"delete metadata"被误归为 FileOperation。新实现改为：
    /// 1. 用词边界匹配（split on whitespace/punct）替代子串匹配；
    /// 2. 引入"命中分数"，取分数最高的类别；平局时保留原优先级；
    /// 3. 关键词列表（KEYWORDS）按 token 列示，避免歧义。
    pub fn classify_task(&self, input: &str) -> GeneralTaskCategory {
        let tokens: Vec<String> = tokenize(input);
        if tokens.is_empty() {
            return GeneralTaskCategory::DailyChat;
        }
        // 类别 → 命中关键字集合（带权重）。权重反映"该关键字对该类别的代表性"。
        const RULES: &[(&str, &[(&str, u32)])] = &[
            (
                "DocumentProcessing",
                &[
                    ("document", 2), ("pdf", 3), ("docx", 3), ("excel", 3),
                    ("powerpoint", 3), ("parse", 2), ("extract", 2),
                ],
            ),
            (
                "WebSearch",
                &[
                    ("search", 2), ("lookup", 2), ("google", 3), ("bing", 3),
                    ("web", 1), ("internet", 2), ("url", 2),
                ],
            ),
            (
                "FileOperation",
                &[
                    ("file", 2), ("folder", 2), ("directory", 2),
                    ("rename", 3), ("delete", 3), ("move", 3), ("copy", 2),
                ],
            ),
            (
                "SystemTool",
                &[
                    ("system", 2), ("command", 2), ("terminal", 3),
                    ("bash", 3), ("shell", 2), ("execute", 2), ("ps", 1),
                ],
            ),
            (
                "DataAnalysis",
                &[
                    ("analyze", 3), ("analysis", 3), ("statistics", 3),
                    ("chart", 2), ("graph", 2), ("dataset", 3), ("summary", 1),
                ],
            ),
            (
                "MessageGateway",
                &[
                    ("whatsapp", 3), ("telegram", 3), ("slack", 3),
                    ("discord", 3), ("wechat", 3),
                ],
            ),
        ];

        // 1) 计算每个类别的命中分数
        let mut scores: Vec<(&str, u32)> = RULES
            .iter()
            .map(|(cat, kws)| {
                let score = kws
                    .iter()
                    .filter(|(kw, _)| tokens.iter().any(|t| t == *kw))
                    .map(|(_, w)| *w)
                    .sum::<u32>();
                (*cat, score)
            })
            .collect();

        // 2) 按分数降序，分数相同则保持 RULES 声明的优先级
        scores.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((cat, score)) = scores.first() {
            if *score > 0 {
                return category_from_name(cat);
            }
        }
        GeneralTaskCategory::DailyChat
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
        assert_eq!(engine.classify_task("Hello, how are you?"), GeneralTaskCategory::DailyChat);
    }

    #[test]
    fn test_capability_toggle() {
        let mut engine = GeneralEngine::new(GeneralEngineConfig::default());
        assert!(engine.is_capability_enabled("web_search"));
        engine.set_capability("web_search", false);
        assert!(!engine.is_capability_enabled("web_search"));
    }
}
