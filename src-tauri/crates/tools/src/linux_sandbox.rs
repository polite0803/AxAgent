// SPDX-License-Identifier: AGPL-3.0-only

//! Linux OS 级沙箱（PLAN-codex-parity P0-1c）
//!
//! 基于 `unshare` 用户命名空间实现受限子进程执行：
//! - `--user --map-root-user`：无特权用户命名空间（无需 root）；
//! - `--mount --ipc --pid --uts --fork`：隔离挂载表 / IPC / 进程表 / 主机名；
//! - `--net`（`policy.network_access == false` 时）：独立网络命名空间，
//!   无任何网络接口（仅 loopback down），等价于断网。
//!
//! ## 当前边界（v1，如实界定）
//! - **文件系统写保护未实现**：`ReadOnly` / `WorkspaceWrite` 目前在文件写入
//!   方面同等宽松（unshare --mount 本身不阻断写）。阶段 2 计划接 Landlock
//!   （kernel LSM，无特权可用）补齐写路径隔离，见 PLAN-codex-parity P0-1。
//! - 依赖系统 `unshare` 二进制（util-linux）；缺失或 user namespace 被禁用
//!   （如某些容器环境）时 spawn 显式报错，**不做静默降级**。
//!
//! 非 Linux 平台本模块不编译；Bash 工具侧由 cfg 分支处理。

use std::path::Path;

use axagent_harness::{SandboxMode, SandboxPolicy};

/// 沙箱化子进程：RAII 兜底——Drop 时 `kill_on_drop` 终止进程，保证
/// 超时/取消不残留进程（tokio `kill_on_drop(true)` + 显式 `start_kill` 加速）。
pub struct SandboxedChild {
    child: std::mem::ManuallyDrop<tokio::process::Child>,
}

/// 与 Windows 侧 `win_sandbox::SandboxedOutput` 字段完全对齐，
/// 上层 `bash.rs` 的等待/格式化逻辑对两个平台无差别消费。
pub struct SandboxedOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl SandboxedChild {
    /// 终止进程（幂等：已退出的进程 start_kill 无副作用）。
    pub fn terminate(&mut self) {
        let _ = self.child.start_kill();
    }

    /// 等待进程退出并收集全部输出。
    ///
    /// 用 `ptr::read` 绕过 Rust 禁止从 Drop 类型 move 字段的限制（E0509），
    /// 配合 `mem::forget(self)` 阻止 self 的 Drop 被调用。
    pub async fn wait_with_output(self) -> Result<SandboxedOutput, String> {
        // SAFETY: SandboxedChild 实现了 Drop，编译器禁止从 self move 字段。
        // ptr::read 绕过此限制，ManuallyDrop::into_inner 取出内部 Child，
        // forget(self) 阻止 self Drop 再次 start_kill + drop ManuallyDrop。
        let child_man_drop = unsafe {
            std::ptr::read(&self.child as *const std::mem::ManuallyDrop<tokio::process::Child>)
        };
        let child = std::mem::ManuallyDrop::into_inner(child_man_drop);
        std::mem::forget(self);

        let output =
            child.wait_with_output().await.map_err(|e| format!("沙箱命令执行异常: {e}"))?;
        Ok(SandboxedOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

impl Drop for SandboxedChild {
    fn drop(&mut self) {
        // kill_on_drop(true) 已兜底；显式 start_kill 让超时路径立即终止
        // 而不是等到 Drop 完成后再由 runtime 回收。
        let _ = self.child.start_kill();
        // 手动释放 ManuallyDrop 内部 Child（否则 pipe 句柄泄漏）
        // SAFETY: self.child 在 drop 期间未被 move，是有效的 ManuallyDrop
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.child);
        }
    }
}

/// 以沙箱策略启动 `unshare ... bash -c <command>`。
///
/// 返回的 [`SandboxedChild`] 具备 RAII 兜底：超时/取消时 Drop 即终止进程。
pub fn spawn_sandboxed(
    policy: &SandboxPolicy,
    command: &str,
    cwd: &Path,
) -> Result<SandboxedChild, String> {
    match policy.mode {
        SandboxMode::ReadOnly | SandboxMode::WorkspaceWrite => {
            spawn_namespaced(policy, command, cwd)
        },
        SandboxMode::DangerFullAccess => {
            Err("DangerFullAccess 不应进入沙箱路径（调用方负责走直通分支）".to_string())
        },
    }
}

fn spawn_namespaced(
    policy: &SandboxPolicy,
    command: &str,
    cwd: &Path,
) -> Result<SandboxedChild, String> {
    let mut cmd = tokio::process::Command::new("unshare");
    cmd.args(["--user", "--map-root-user", "--mount", "--ipc", "--pid", "--uts", "--fork"]);
    // 网络封锁：network_access=false（ReadOnly / WorkspaceWrite 默认）时
    // 独立 network namespace = 无接口断网。
    if !policy.network_access {
        cmd.arg("--net");
    }
    cmd.args(["bash", "-c", command]);

    cmd.current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // 环境白名单：不继承父进程完整 env（防凭据泄露），与 Windows 侧同思路。
    cmd.env_clear();
    for key in ["PATH", "HOME", "LANG", "LC_ALL", "TERM", "TMPDIR", "USER", "SHELL"] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Linux 沙箱需要系统 `unshare`（util-linux），未找到该命令".to_string()
        } else {
            format!("unshare 启动失败（user namespace 可能被禁用）: {e}")
        }
    })?;

    Ok(SandboxedChild { child: std::mem::ManuallyDrop::new(child) })
}
