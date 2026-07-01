// SPDX-License-Identifier: AGPL-3.0-only

//! 插件发现测试 — 从 fixtures 加载 manifest。

mod common;

use common::{TempDir, load_manifest, minimal_plugin_json};

#[test]
fn discover_valid_plugin_from_directory() {
    let tmp = TempDir::new("discover-valid");
    let manifest_json = minimal_plugin_json("hello-world", "1.0.0");
    let plugin_dir = tmp.write_plugin_json(&manifest_json);

    let result = load_manifest(&plugin_dir);
    assert!(result.is_ok(), "Expected valid manifest, got: {:?}", result.err());
    let manifest = result.unwrap();
    assert_eq!(manifest.name, "hello-world");
    assert_eq!(manifest.version, "1.0.0");
}

#[test]
fn discover_plugin_with_mcp_servers() {
    let tmp = TempDir::new("discover-mcp");
    let json = r#"{
        "name": "mcp-plugin",
        "version": "2.0.0",
        "description": "Plugin with MCP servers",
        "mcp_servers": [
            {"name": "filesystem", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem"]}
        ],
        "skills": [],
        "agents": [],
        "permissions": [],
        "defaultEnabled": false
    }"#;
    let plugin_dir = tmp.write_plugin_json(json);

    let result = load_manifest(&plugin_dir);
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.mcp_servers.len(), 1);
    assert_eq!(manifest.mcp_servers[0].name, "filesystem");
}

#[test]
fn reject_missing_manifest() {
    let tmp = TempDir::new("discover-no-manifest");
    // 空目录 — 没有 plugin.json
    let empty_dir = tmp.path.join("empty-plugin");
    std::fs::create_dir_all(&empty_dir).unwrap();

    let result = load_manifest(&empty_dir);
    assert!(result.is_err(), "Expected error for missing manifest");
}

#[test]
fn reject_malformed_json() {
    let tmp = TempDir::new("discover-malformed");
    let bad_dir = tmp.write_malformed_plugin_json();

    let result = load_manifest(&bad_dir);
    assert!(result.is_err(), "Expected error for malformed JSON");
}

#[test]
fn discover_multiple_plugins_from_same_fixture_dir() {
    let tmp = TempDir::new("discover-multi");

    let plugin_a = tmp.write_plugin_json(&minimal_plugin_json("plugin-a", "0.1.0"));
    let plugin_b_json = r#"{
        "name": "plugin-b",
        "version": "0.2.0",
        "description": "Second plugin",
        "mcp_servers": [],
        "skills": [],
        "agents": [],
        "permissions": [],
        "defaultEnabled": false
    }"#;
    let plugin_b_dir = tmp.path.join("plugin-b");
    std::fs::create_dir_all(&plugin_b_dir).unwrap();
    std::fs::write(plugin_b_dir.join("plugin.json"), plugin_b_json).unwrap();

    let a = load_manifest(&plugin_a).unwrap();
    let b = load_manifest(&plugin_b_dir).unwrap();
    assert_eq!(a.name, "plugin-a");
    assert_eq!(b.name, "plugin-b");
    assert_ne!(a.name, b.name);
}
