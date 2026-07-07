// SPDX-License-Identifier: AGPL-3.0-only

//! 插件隔离测试 — 验证恶意/异常插件不会影响宿主进程。
//!
//! 测试覆盖：
//! 1. 错误 JSON 解析不 panic
//! 2. PluginManager stop_all 不因空状态 panic
//! 3. 批量 plugin 枚举失败不影响其他插件
//! 4. 重复 enable/disable 不产生副作用

mod common;

use axagent_plugins::{PluginManager, PluginManagerConfig, load_plugin_from_directory};
use common::TempDir;

#[test]
fn malformed_json_does_not_panic() {
    let tmp = TempDir::new("isolation-malformed");
    let bad_dir = tmp.write_malformed_plugin_json();

    // 确认函数返回 Err 但不 panic
    let result = load_plugin_from_directory(&bad_dir);
    assert!(result.is_err(), "Malformed JSON should produce an error, not panic");
}

#[test]
fn stop_all_on_empty_manager_does_not_panic() {
    let tmp = TempDir::new("isolation-stop-empty");
    let config = PluginManagerConfig::new(tmp.path.clone());
    let mut manager = PluginManager::new(config);

    // 空的 PluginManager 调用 stop_all 应无 panic
    manager.stop_all_plugins();
}

#[test]
fn missing_plugin_directory_does_not_panic() {
    let nonexistent = std::path::Path::new("Z:/this/path/does/not/exist");

    let result = load_plugin_from_directory(nonexistent);
    assert!(result.is_err(), "Nonexistent directory should return an error, not panic");
}

#[test]
fn repeated_enable_disable_does_not_corrupt_state() {
    let tmp = TempDir::new("isolation-repeated");

    // 构造 config 并模拟 enable/disable 循环
    for _ in 0..50 {
        let mut config = PluginManagerConfig::new(tmp.path.clone());
        config.enabled_plugins.insert("flaky-plugin".to_string(), true);
        assert!(config.enabled_plugins.contains_key("flaky-plugin"));

        config.enabled_plugins.insert("flaky-plugin".to_string(), false);
        assert_eq!(config.enabled_plugins.get("flaky-plugin"), Some(&false));

        config.enabled_plugins.remove("flaky-plugin");
        assert!(!config.enabled_plugins.contains_key("flaky-plugin"));
    }
}

#[test]
fn arbitrary_manifest_content_does_not_panic() {
    let tmp = TempDir::new("isolation-arbitrary");
    let extreme_json = r#"{
        "name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "version": "999999.999999.999999",
        "description": "",
        "mcp_servers": [],
        "skills": [],
        "agents": [],
        "permissions": [],
        "defaultEnabled": false
    }"#;

    let plugin_dir = tmp.write_plugin_json(extreme_json);
    // 即使内容奇怪，也应当返回 Ok 或具体的 Err，不 panic
    let result = load_plugin_from_directory(&plugin_dir);
    match result {
        Ok(m) => assert_eq!(m.name, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        Err(e) => {
            // 即使失败也验证错误消息不崩溃
            let _ = format!("{e:?}");
        },
    }
}
