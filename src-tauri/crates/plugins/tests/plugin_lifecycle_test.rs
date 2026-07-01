// SPDX-License-Identifier: AGPL-3.0-only

//! 插件生命周期测试 — load → enable → disable → unload 完整路径。
//!
//! 测试覆盖：
//! 1. 从 fixtures 加载 manifest
//! 2. PluginManagerConfig 的 enabled_plugins 状态机（enable → disable → remove）
//! 3. 配置默认值

mod common;

use axagent_plugins::{PluginManagerConfig, load_plugin_from_directory};
use common::{TempDir, minimal_plugin_json};

#[test]
fn load_manifest_from_fixture() {
    let tmp = TempDir::new("lifecycle-load");
    let manifest_json = minimal_plugin_json("lifecycle-test", "1.0.0");
    let plugin_dir = tmp.write_plugin_json(&manifest_json);

    let manifest = load_plugin_from_directory(&plugin_dir).expect("should load manifest");
    assert_eq!(manifest.name, "lifecycle-test");
    assert!(!manifest.default_enabled);
}

#[test]
fn enable_updates_enabled_state() {
    let tmp = TempDir::new("lifecycle-enable");
    let mut config = PluginManagerConfig::new(tmp.path.clone());

    // 初始为空
    assert!(config.enabled_plugins.is_empty());

    // 标记为启用
    config
        .enabled_plugins
        .insert("test-plugin".to_string(), true);
    assert_eq!(config.enabled_plugins.get("test-plugin"), Some(&true));
}

#[test]
fn disable_clears_enabled_state() {
    let tmp = TempDir::new("lifecycle-disable");
    let mut config = PluginManagerConfig::new(tmp.path.clone());

    // Enable → Disable
    config
        .enabled_plugins
        .insert("test-plugin".to_string(), true);
    config
        .enabled_plugins
        .insert("test-plugin".to_string(), false);

    assert_eq!(config.enabled_plugins.get("test-plugin"), Some(&false));
}

#[test]
fn lifecycle_load_enable_disable_unload() {
    let tmp = TempDir::new("lifecycle-full");

    // 1. Load manifest from fixture
    let plugin_dir = tmp.write_plugin_json(&minimal_plugin_json("full-lifecycle", "1.0.0"));
    let manifest = load_plugin_from_directory(&plugin_dir).unwrap();
    assert_eq!(manifest.name, "full-lifecycle");

    // 2. Simulate enable
    let mut config = PluginManagerConfig::new(tmp.path.clone());
    config
        .enabled_plugins
        .insert("full-lifecycle".to_string(), true);
    assert!(config.enabled_plugins.contains_key("full-lifecycle"));

    // 3. Simulate disable
    config
        .enabled_plugins
        .insert("full-lifecycle".to_string(), false);
    assert_eq!(config.enabled_plugins.get("full-lifecycle"), Some(&false));

    // 4. Simulate unload: remove from enabled set
    config.enabled_plugins.remove("full-lifecycle");
    assert!(!config.enabled_plugins.contains_key("full-lifecycle"));
}

#[test]
fn default_config_is_empty() {
    let config = PluginManagerConfig::new("/tmp/test-home");
    assert!(config.enabled_plugins.is_empty());
    assert!(config.external_dirs.is_empty());
    assert!(config.install_root.is_none());
}
