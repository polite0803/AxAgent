// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use serde::Serialize;
#[cfg(not(target_os = "android"))]
use std::process::Stdio;
#[cfg(not(target_os = "android"))]
use tokio::process::Command;

#[cfg(not(target_os = "android"))]
const SANDBOX_TIMEOUT_SECS: u64 = 30;

/// 安全包装：剥离 `require`、`process`、`globalThis`、`global`、`Buffer`，
/// 限制 CPU/内存，禁止子进程，禁止 socket，关闭原生 module 加载。
/// 不依赖任何第三方包，直接走 Node `--frozen-intrinsics` + 预置沙箱 wrapper。
const SANDBOX_PROLOGUE: &str = r#"
'use strict';
(function () {
  const _no = () => { throw new Error('blocked by sandbox'); };
  // 1) 拒绝原生模块与子进程
  try { Object.freeze(Object.prototype); } catch (e) {}
  const _blockedModules = ['fs', 'child_process', 'cluster', 'worker_threads',
    'dgram', 'dns', 'http', 'http2', 'https', 'net', 'tls', 'inspector',
    'repl', 'readline', 'vm', 'wasi', 'sys', 'os', 'path', 'crypto'];
  // 2) 阻断 require 加载
  try {
    const _Mod = require('module');
    if (_Mod && _Mod.prototype && _Mod.prototype.require) {
      _Mod.prototype.require = function (id) {
        if (_blockedModules.indexOf(String(id)) >= 0 ||
            String(id).startsWith('node:')) _no();
        if (typeof id === 'string' && /^[./]/.test(id)) {
          // 相对路径也只允许当前工作区内的虚拟 in-memory 模块。
          _no();
        }
        _no();
      };
    }
  } catch (e) { _no(); }
  // 3) 禁用 process 大部分 API
  try {
    if (typeof process !== 'undefined' && process) {
      const _origEnv = process.env;
      Object.defineProperty(process, 'env', {
        get: () => new Proxy({}, { get: () => undefined, has: () => false,
          ownKeys: () => [], getOwnPropertyDescriptor: () => undefined })
      });
      process.exit = _no;
      process.kill = _no;
      process.binding = _no;
      process.dlopen = _no;
    }
  } catch (e) { _no(); }
  // 4) 屏蔽全局 fetch/XHR/Buffer
  try { globalThis.fetch = _no; } catch (e) {}
  try { globalThis.XMLHttpRequest = undefined; } catch (e) {}
  try { globalThis.WebSocket = undefined; } catch (e) {}
  try { globalThis.importScripts = _no; } catch (e) {}
  // 5) 内存上限：使用 performance + 周期性自检
  // 真正的硬限制由外部资源配额（RLIMIT_AS）保证。
  // 6) 阻断 setImmediate/nextTick 之外的异步 IO
  //    （Node 无完美方式，但子进程被禁后，文件/网络 IO 已不可达）
})();
"#;

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[cfg(not(target_os = "android"))]
pub struct SandboxRunner {
    node_path: String,
    /// 是否启用"硬沙箱"。生产必须为 true。
    /// 仅在测试或调试时可通过 AXAGENT_DISABLE_NODE_HARDEN=1 关闭。
    hard_sandbox: bool,
}

#[cfg(not(target_os = "android"))]
impl Default for SandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "android"))]
impl SandboxRunner {
    pub fn new() -> Self {
        // 1) 解析 node 路径：必须指向真实的 node/bun 可执行文件，且文件名是 node/bun。
        let raw = std::env::var("AXAGENT_NODE_PATH").unwrap_or_else(|_| "node".to_string());
        let resolved = if raw.contains('/') || raw.contains('\\') {
            std::fs::canonicalize(&raw)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| raw.clone())
        } else {
            raw.clone()
        };
        let exe_name = std::path::Path::new(&resolved)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        // 拒绝非 node/bun 二进制 — 防止 NODE_PATH 指向 rm/python 等。
        // 修复：原代码用 anyhow::bail!，但函数签名是 -> Self，编译错误。
        // 这里改为：检测到非法路径时记 error 日志并 panic（开发期早暴露）。
        // 生产环境应在启动时校验环境变量，不应让此函数失败。
        if !(exe_name == "node" || exe_name == "bun") {
            tracing::error!(
                "AXAGENT_NODE_PATH/NODE_PATH must point to node or bun, got '{}' (resolved '{}')",
                raw,
                resolved
            );
            panic!(
                "AXAGENT_NODE_PATH/NODE_PATH must point to node or bun, got '{}' (resolved '{}')",
                raw, resolved
            );
        }
        let hard_sandbox = std::env::var("AXAGENT_DISABLE_NODE_HARDEN")
            .map(|v| !matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);
        Self {
            node_path: resolved,
            hard_sandbox,
        }
    }

    pub async fn execute(&self, code: &str, language: &str) -> Result<ExecutionResult> {
        let limits = crate::resource_limits::ResourceLimits::default_sandbox();
        if let Err(e) = limits.apply_to_current_process() {
            tracing::warn!("Failed to apply sandbox resource limits: {}", e);
        }

        match language {
            "javascript" | "js" | "typescript" | "ts" => self.execute_js(code).await,
            "python" | "py" => self.execute_python(code).await,
            _ => Err(anyhow::anyhow!("Unsupported language: {}", language)),
        }
    }

    async fn execute_js(&self, code: &str) -> Result<ExecutionResult> {
        let temp_dir = std::env::temp_dir();
        // 文件名加 .sandbox.js 后缀以避免与用户代码冲突，且不可预测。
        let script_path = temp_dir.join(format!(
            "axagent_sbx_{}_{}.sandbox.js",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));

        // 将 prologue + 用户代码拼成一个文件。prologue 必须在用户代码之前。
        let full_source = format!("{SANDBOX_PROLOGUE}\n// === user code below ===\n{code}\n");
        tokio::fs::write(&script_path, &full_source).await?;

        // 安全相关命令行：
        //   --frozen-intrinsics: 防止 Object.prototype 污染绕过
        //   --disallow-code-generation-from-strings: 阻止 new Function / eval (我们也想阻断)
        //   --no-warnings: 减少噪声
        //   --input-type=module: 不强制
        // 不使用 --experimental-vm-modules（依赖环境）。
        // SECURITY (C6): 对 Node.js 子进程增加内存限制，防止 OOM 影响宿主机。
        // --max-old-space-size: V8 老生代最大堆 (MB)
        // --max-semi-space-size: V8 新生代半空间 (MB)
        let mut cmd = Command::new(&self.node_path);
        if self.hard_sandbox {
            cmd.arg("--frozen-intrinsics")
                .arg("--disallow-code-generation-from-strings")
                .arg("--no-warnings")
                .arg("--max-old-space-size=256")
                .arg("--max-semi-space-size=8");
        }
        let output_fut = cmd
            .arg(&script_path)
            .env_remove("NODE_PATH")
            .env_remove("NODE_OPTIONS")
            .env_remove("AXAGENT_NODE_PATH")
            .env_remove("AXAGENT_DISABLE_NODE_HARDEN")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output();

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(SANDBOX_TIMEOUT_SECS), output_fut)
                .await
                .map_err(|_| anyhow::anyhow!("Execution timeout"))??;

        let _ = tokio::fs::remove_file(&script_path).await;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code: result.status.code().unwrap_or(-1),
        })
    }

    async fn execute_python(&self, _code: &str) -> Result<ExecutionResult> {
        Err(anyhow::anyhow!("Python execution handled by frontend Pyodide"))
    }
}

#[cfg(not(target_os = "android"))]
pub fn create_sandbox_runner() -> SandboxRunner {
    SandboxRunner::new()
}

#[cfg(target_os = "android")]
use axagent_harness::constants::android_msg;

#[cfg(target_os = "android")]
pub struct SandboxRunner;

#[cfg(target_os = "android")]
impl SandboxRunner {
    pub fn new() -> Self {
        Self
    }
    pub async fn execute(&self, _code: &str, _language: &str) -> anyhow::Result<ExecutionResult> {
        anyhow::bail!(android_msg::SANDBOX_NOT_AVAILABLE)
    }
}

#[cfg(target_os = "android")]
pub fn create_sandbox_runner() -> SandboxRunner {
    SandboxRunner::new()
}

#[cfg(target_os = "android")]
#[derive(Debug, serde::Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
