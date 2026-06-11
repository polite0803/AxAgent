// SPDX-License-Identifier: AGPL-3.0-only

//! Rhai 脚本引擎适配器契约。
//!
//! 提供 Rhai 脚本的编译和执行能力，用于工作流中的动态脚本节点。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde_json::Value as JsonValue;

/// Rhai 脚本引擎适配器契约
///
/// 封装 Rhai 脚本的批量编译和按名执行能力。
/// 实现方（`axagent-tools::rhai_engine`）管理内部脚本缓存。
pub trait RhaiEngineAdapter: fmt::Debug + Send + Sync {
    /// 批量注册并编译脚本（在工作流初始化时调用）
    ///
    /// `scripts`：脚本定义数组，每个元素为 `{ "tool_name": "...", "code": "..." }`
    fn register_scripts(&self, scripts: &[JsonValue]);

    /// 执行已注册的指定脚本
    ///
    /// - `script_name`：要执行的脚本名称（与注册时的 `tool_name` 对应）
    /// - `args`：输入参数
    /// - `tool_fns`：可被脚本调用的工具函数映射，key=工具名，value= `(name, args) -> Result`
    fn execute_script(
        &self,
        script_name: &str,
        args: JsonValue,
        tool_fns: &HashMap<String, RhaiToolFn>,
    ) -> Result<JsonValue, String>;
}

/// Rhai 可调用工具函数
///
/// 签名：`(工具名, JSON参数) -> Result<JSON结果, 错误信息>`
pub type RhaiToolFn = Arc<dyn Fn(String, JsonValue) -> Result<JsonValue, String> + Send + Sync>;

/// 空实现 — 总是失败（Rhai 引擎未配置）
#[derive(Debug)]
pub struct NoopRhaiEngineAdapter;

impl RhaiEngineAdapter for NoopRhaiEngineAdapter {
    fn register_scripts(&self, _scripts: &[JsonValue]) {}

    fn execute_script(
        &self,
        _script_name: &str,
        _args: JsonValue,
        _tool_fns: &HashMap<String, RhaiToolFn>,
    ) -> Result<JsonValue, String> {
        Err("Rhai engine is not configured".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_never_succeeds_on_execute() {
        let adapter = NoopRhaiEngineAdapter;
        adapter.register_scripts(&[]);
        let result = adapter.execute_script("test", JsonValue::Null, &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
    }
}
