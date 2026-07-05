// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for ToolRegistry state management: disable/enable/unregister.

use async_trait::async_trait;
use axagent_tools::registry::ToolRegistry;
use axagent_tools::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use std::sync::Arc;

/// A minimal mock tool for registry tests.
struct MockRegistryTool;

#[async_trait]
impl Tool for MockRegistryTool {
    fn name(&self) -> &str {
        "mock-registry-tool"
    }
    fn description(&self) -> &str {
        "mock tool for registry testing"
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    async fn call(
        &self,
        _input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("ok"))
    }
}

/// A named mock tool that's distinguishable by name.
struct NamedMockTool {
    name: &'static str,
    cat: ToolCategory,
}

#[async_trait]
impl Tool for NamedMockTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "named mock tool"
    }
    fn category(&self) -> ToolCategory {
        self.cat
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    async fn call(
        &self,
        _input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("ok"))
    }
}

// ---------------------------------------------------------------------------
// ToolRegistry: register & find
// ---------------------------------------------------------------------------

#[test]
fn test_registry_lifecycle_register_and_find() {
    let mut reg = ToolRegistry::new();
    let tool: Arc<dyn Tool> = Arc::new(MockRegistryTool);

    assert!(!reg.contains("mock-registry-tool"));
    reg.register(tool);
    assert!(reg.contains("mock-registry-tool"));
    assert!(reg.find("mock-registry-tool").is_some());
}

#[test]
fn test_registry_lifecycle_register_all_and_list() {
    let mut reg = ToolRegistry::new();
    reg.register_all(vec![
        Arc::new(NamedMockTool {
            name: "lifecycle-a",
            cat: ToolCategory::FileRead,
        }) as Arc<dyn Tool>,
        Arc::new(NamedMockTool {
            name: "lifecycle-b",
            cat: ToolCategory::FileWrite,
        }),
        Arc::new(NamedMockTool {
            name: "lifecycle-c",
            cat: ToolCategory::Network,
        }),
    ]);
    assert_eq!(reg.list_all().len(), 3);
    assert_eq!(reg.len(), 3);
}

// ---------------------------------------------------------------------------
// ToolRegistry: disable / enable
// ---------------------------------------------------------------------------

#[test]
fn test_registry_lifecycle_disable_then_enable() {
    let mut reg = ToolRegistry::new();
    let tool: Arc<dyn Tool> = Arc::new(NamedMockTool {
        name: "reg-toggle",
        cat: ToolCategory::Shell,
    });
    reg.register(tool);

    // Initially enabled
    assert!(reg.find("reg-toggle").is_some());

    // Disable
    reg.disable("reg-toggle");
    assert!(reg.find("reg-toggle").is_none(), "Disabled tool should not be found");

    // Re-enable
    reg.enable("reg-toggle");
    assert!(reg.find("reg-toggle").is_some(), "Re-enabled tool should be found");
}

#[test]
fn test_registry_lifecycle_disable_nonexistent() {
    let mut reg = ToolRegistry::new();
    reg.disable("does-not-exist"); // should not panic
}

#[test]
fn test_registry_lifecycle_enable_nonexistent() {
    let mut reg = ToolRegistry::new();
    reg.enable("does-not-exist"); // should not panic
}

#[test]
fn test_registry_lifecycle_disable_category() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(NamedMockTool {
        name: "cat-f1",
        cat: ToolCategory::FileRead,
    }) as Arc<dyn Tool>);
    reg.register(Arc::new(NamedMockTool {
        name: "cat-f2",
        cat: ToolCategory::FileRead,
    }) as Arc<dyn Tool>);
    reg.register(Arc::new(NamedMockTool {
        name: "cat-shell",
        cat: ToolCategory::Shell,
    }) as Arc<dyn Tool>);

    reg.disable_category(ToolCategory::FileRead);
    assert!(reg.find("cat-f1").is_none(), "FileRead tool should be disabled");
    assert!(reg.find("cat-f2").is_none(), "FileRead tool should be disabled");
    assert!(reg.find("cat-shell").is_some(), "Shell tool should remain");
}

// ---------------------------------------------------------------------------
// ToolRegistry: unregister
// ---------------------------------------------------------------------------

#[test]
fn test_registry_lifecycle_unregister() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(NamedMockTool {
        name: "unreg-me",
        cat: ToolCategory::Shell,
    }) as Arc<dyn Tool>);
    assert!(reg.contains("unreg-me"));

    reg.unregister("unreg-me");
    assert!(!reg.contains("unreg-me"));
    assert!(reg.find("unreg-me").is_none());
}

#[test]
fn test_registry_lifecycle_unregister_nonexistent() {
    let mut reg = ToolRegistry::new();
    reg.unregister("ghost"); // should not panic
}

// ---------------------------------------------------------------------------
// ToolRegistry: by_category
// ---------------------------------------------------------------------------

#[test]
fn test_registry_lifecycle_by_category() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(NamedMockTool {
        name: "bc-a1",
        cat: ToolCategory::Shell,
    }) as Arc<dyn Tool>);
    reg.register(Arc::new(NamedMockTool {
        name: "bc-a2",
        cat: ToolCategory::Shell,
    }) as Arc<dyn Tool>);
    reg.register(Arc::new(NamedMockTool {
        name: "bc-net",
        cat: ToolCategory::Network,
    }) as Arc<dyn Tool>);

    let shell_tools = reg.by_category(ToolCategory::Shell);
    assert_eq!(shell_tools.len(), 2);

    let net_tools = reg.by_category(ToolCategory::Network);
    assert_eq!(net_tools.len(), 1);
}

// ---------------------------------------------------------------------------
// ToolRegistry: is_empty / total_registered
// ---------------------------------------------------------------------------

#[test]
fn test_registry_lifecycle_empty_state() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.total_registered(), 0);
}

#[test]
fn test_registry_lifecycle_not_empty() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(NamedMockTool {
        name: "only-one",
        cat: ToolCategory::Shell,
    }) as Arc<dyn Tool>);
    assert!(!reg.is_empty());
    assert_eq!(reg.total_registered(), 1);
}
