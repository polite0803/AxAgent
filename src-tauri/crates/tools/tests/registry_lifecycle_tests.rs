// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for ToolRegistry state management: disable/enable/unregister.

use axagent_tools::registry::ToolRegistry;
use axagent_tools::{Tool, ToolCategory, ToolError, ToolInfo};

/// A minimal mock tool for registry tests.
struct MockTool {
    name: &'static str,
    category: ToolCategory,
    description: &'static str,
}

impl MockTool {
    fn new(name: &'static str, category: ToolCategory) -> Self {
        Self {
            name,
            category,
            description: "mock tool for testing",
        }
    }
}

impl Tool for MockTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        self.description
    }
    fn category(&self) -> ToolCategory {
        self.category
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        Ok(serde_json::json!({"result": "ok"}))
    }
    fn tool_type(&self) -> axagent_harness::ToolType {
        axagent_harness::ToolType::Builtin
    }
}

// ---------------------------------------------------------------------------
// ToolRegistry: register & find
// ---------------------------------------------------------------------------

#[test]
fn test_registry_register_and_find() {
    let mut reg = ToolRegistry::new();
    let tool = std::sync::Arc::new(MockTool::new("mock-a", ToolCategory::General));

    assert!(!reg.contains("mock-a"));
    reg.register(tool);
    assert!(reg.contains("mock-a"));
    assert!(reg.find("mock-a").is_some());
}

#[test]
fn test_registry_register_all_and_list() {
    let mut reg = ToolRegistry::new();
    let tools: Vec<std::sync::Arc<dyn Tool>> = vec![
        std::sync::Arc::new(MockTool::new("t1", ToolCategory::General)),
        std::sync::Arc::new(MockTool::new("t2", ToolCategory::FileSystem)),
        std::sync::Arc::new(MockTool::new("t3", ToolCategory::Network)),
    ];
    reg.register_all(tools);
    assert_eq!(reg.list_all().len(), 3);
    assert_eq!(reg.len(), 3);
}

// ---------------------------------------------------------------------------
// ToolRegistry: disable / enable
// ---------------------------------------------------------------------------

#[test]
fn test_registry_disable_tool() {
    let mut reg = ToolRegistry::new();
    let tool = std::sync::Arc::new(MockTool::new("toggle-me", ToolCategory::General));
    reg.register(tool);

    // Initially enabled
    assert!(reg.find("toggle-me").is_some());

    // Disable
    reg.disable("toggle-me");
    // Should still be in registry but find returns None
    assert!(reg.find("toggle-me").is_none());

    // Re-enable
    reg.enable("toggle-me");
    assert!(reg.find("toggle-me").is_some());
}

#[test]
fn test_registry_disable_nonexistent_tool() {
    let mut reg = ToolRegistry::new();
    // Should not panic
    reg.disable("does-not-exist");
}

#[test]
fn test_registry_enable_nonexistent_tool() {
    let mut reg = ToolRegistry::new();
    reg.enable("does-not-exist"); // should not panic
}

#[test]
fn test_registry_disable_category() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("f1", ToolCategory::FileSystem)));
    reg.register(std::sync::Arc::new(MockTool::new("f2", ToolCategory::FileSystem)));
    reg.register(std::sync::Arc::new(MockTool::new("g1", ToolCategory::General)));

    reg.disable_category(ToolCategory::FileSystem);
    // FileSystem tools should be gone from find
    assert!(reg.find("f1").is_none());
    assert!(reg.find("f2").is_none());
    // General tool should remain
    assert!(reg.find("g1").is_some());
}

// ---------------------------------------------------------------------------
// ToolRegistry: unregister
// ---------------------------------------------------------------------------

#[test]
fn test_registry_unregister_tool() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("remove-me", ToolCategory::General)));
    assert!(reg.contains("remove-me"));

    reg.unregister("remove-me");
    assert!(!reg.contains("remove-me"));
    assert!(reg.find("remove-me").is_none());
}

#[test]
fn test_registry_unregister_nonexistent() {
    let mut reg = ToolRegistry::new();
    reg.unregister("ghost"); // should not panic
}

// ---------------------------------------------------------------------------
// ToolRegistry: by_category
// ---------------------------------------------------------------------------

#[test]
fn test_registry_by_category() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("a1", ToolCategory::General)));
    reg.register(std::sync::Arc::new(MockTool::new("a2", ToolCategory::General)));
    reg.register(std::sync::Arc::new(MockTool::new("fs1", ToolCategory::FileSystem)));

    let general_tools = reg.by_category(ToolCategory::General);
    assert_eq!(general_tools.len(), 2);

    let fs_tools = reg.by_category(ToolCategory::FileSystem);
    assert_eq!(fs_tools.len(), 1);
}

// ---------------------------------------------------------------------------
// ToolRegistry: is_empty / total_registered
// ---------------------------------------------------------------------------

#[test]
fn test_registry_empty() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.total_registered(), 0);
}

#[test]
fn test_registry_not_empty() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("x", ToolCategory::General)));
    assert!(!reg.is_empty());
    assert_eq!(reg.total_registered(), 1);
}

// ---------------------------------------------------------------------------
// ToolRegistry: register & find
// ---------------------------------------------------------------------------

#[test]
fn test_registry_register_and_find() {
    let mut reg = ToolRegistry::new();
    let tool = std::sync::Arc::new(MockTool::new("mock-a", ToolCategory::General));

    assert!(!reg.contains("mock-a"));
    reg.register(tool);
    assert!(reg.contains("mock-a"));
    assert!(reg.find("mock-a").is_some());
}

#[test]
fn test_registry_register_all_and_list() {
    let mut reg = ToolRegistry::new();
    let tools: Vec<std::sync::Arc<dyn axagent_tools::tool::Tool>> = vec![
        std::sync::Arc::new(MockTool::new("t1", ToolCategory::General)),
        std::sync::Arc::new(MockTool::new("t2", ToolCategory::FileSystem)),
        std::sync::Arc::new(MockTool::new("t3", ToolCategory::Network)),
    ];
    reg.register_all(tools);
    assert_eq!(reg.list_all().len(), 3);
    assert_eq!(reg.len(), 3);
}

// ---------------------------------------------------------------------------
// ToolRegistry: disable / enable
// ---------------------------------------------------------------------------

#[test]
fn test_registry_disable_tool() {
    let mut reg = ToolRegistry::new();
    let tool = std::sync::Arc::new(MockTool::new("toggle-me", ToolCategory::General));
    reg.register(tool);

    // Initially enabled
    assert!(reg.find("toggle-me").is_some());

    // Disable
    reg.disable("toggle-me");
    // Should still be in registry but find returns None
    assert!(reg.find("toggle-me").is_none());

    // Re-enable
    reg.enable("toggle-me");
    assert!(reg.find("toggle-me").is_some());
}

#[test]
fn test_registry_disable_nonexistent_tool() {
    let mut reg = ToolRegistry::new();
    // Should not panic
    reg.disable("does-not-exist");
}

#[test]
fn test_registry_enable_nonexistent_tool() {
    let mut reg = ToolRegistry::new();
    reg.enable("does-not-exist"); // should not panic
}

#[test]
fn test_registry_disable_category() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("f1", ToolCategory::FileSystem)));
    reg.register(std::sync::Arc::new(MockTool::new("f2", ToolCategory::FileSystem)));
    reg.register(std::sync::Arc::new(MockTool::new("g1", ToolCategory::General)));

    reg.disable_category(ToolCategory::FileSystem);
    // FileSystem tools should be gone from find
    assert!(reg.find("f1").is_none());
    assert!(reg.find("f2").is_none());
    // General tool should remain
    assert!(reg.find("g1").is_some());
}

// ---------------------------------------------------------------------------
// ToolRegistry: unregister
// ---------------------------------------------------------------------------

#[test]
fn test_registry_unregister_tool() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("remove-me", ToolCategory::General)));
    assert!(reg.contains("remove-me"));

    reg.unregister("remove-me");
    assert!(!reg.contains("remove-me"));
    assert!(reg.find("remove-me").is_none());
}

#[test]
fn test_registry_unregister_nonexistent() {
    let mut reg = ToolRegistry::new();
    reg.unregister("ghost"); // should not panic
}

// ---------------------------------------------------------------------------
// ToolRegistry: by_category
// ---------------------------------------------------------------------------

#[test]
fn test_registry_by_category() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("a1", ToolCategory::General)));
    reg.register(std::sync::Arc::new(MockTool::new("a2", ToolCategory::General)));
    reg.register(std::sync::Arc::new(MockTool::new("fs1", ToolCategory::FileSystem)));

    let general_tools = reg.by_category(ToolCategory::General);
    assert_eq!(general_tools.len(), 2);

    let fs_tools = reg.by_category(ToolCategory::FileSystem);
    assert_eq!(fs_tools.len(), 1);
}

// ---------------------------------------------------------------------------
// ToolRegistry: is_empty / total_registered
// ---------------------------------------------------------------------------

#[test]
fn test_registry_empty() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.total_registered(), 0);
}

#[test]
fn test_registry_not_empty() {
    let mut reg = ToolRegistry::new();
    reg.register(std::sync::Arc::new(MockTool::new("x", ToolCategory::General)));
    assert!(!reg.is_empty());
    assert_eq!(reg.total_registered(), 1);
}
