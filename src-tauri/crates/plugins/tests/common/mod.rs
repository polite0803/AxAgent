// SPDX-License-Identifier: AGPL-3.0-only

//! 测试工具和 fixture 生成器。
//!
//! 多个 test binary 共享同一份工具模块,但每个 test 只用其中一部分,
//! 因此允许 `common` 模块内出现未使用的辅助函数。

#![allow(dead_code)]

use axagent_plugins::PluginManifest;
use serde_json::json;
use std::path::{Path, PathBuf};

/// 创建测试用临时目录并在 test 完成后自动清理。
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("测试：系统时间应晚于 UNIX EPOCH")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("plugin-test-{label}-{nanos}"));
        std::fs::create_dir_all(&path).expect("failed to create temp dir");
        TempDir { path }
    }

    /// 在临时目录中写入 plugin.json 并返回完整路径。
    pub fn write_plugin_json(&self, manifest_json: &str) -> PathBuf {
        let plugin_dir = self.path.join("plugin");
        std::fs::create_dir_all(&plugin_dir).expect("failed to create plugin dir");
        let manifest_path = plugin_dir.join("plugin.json");
        std::fs::write(&manifest_path, manifest_json).expect("failed to write plugin.json");
        plugin_dir
    }

    /// 写入一个恶意 plugin.json（语法错误或 JSON 不合法）。
    pub fn write_malformed_plugin_json(&self) -> PathBuf {
        let plugin_dir = self.path.join("malformed-plugin");
        std::fs::create_dir_all(&plugin_dir).expect("failed to create malformed dir");
        let manifest_path = plugin_dir.join("plugin.json");
        std::fs::write(&manifest_path, r#"{"name": "bad, "version": "0.1"}"#)
            .expect("failed to write malformed json");
        plugin_dir
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// 生成一个最小有效的 plugin.json content
pub fn minimal_plugin_json(name: &str, version: &str) -> String {
    let json = json!({
        "name": name,
        "version": version,
        "description": format!("Test plugin: {}", name),
        "mcp_servers": [],
        "skills": [],
        "agents": [],
        "permissions": [],
        "defaultEnabled": false
    });
    json.to_string()
}

/// 尝试从 fixture 目录加载 manifest 并返回 result。
pub fn load_manifest(dir: &Path) -> Result<PluginManifest, axagent_plugins::PluginError> {
    axagent_plugins::load_plugin_from_directory(dir)
}
