//! Subprocess 包装 helper —— `safe_spawn` 统一 setsid。
//!
//! ## 背景（P1-3.7）
//!
//! 之前 BashTool 在 Windows 上漏 grandchild（sub-shell 起的子进程没
//! 跟随父进程被 kill）。类似问题大概率也存在于其他 tools 里的
//! `std::process::Command::new(...).spawn()` call sites。
//!
//! 修复策略：把所有 child 放到新进程组（Unix：`setsid`），未来
//! `kill_on_drop` 可以用 `kill(-pid, SIGKILL)` 一次性结束整个 group，
//! 包括 grandchild。
//!
//! ## Windows 限制
//!
//! Windows 没有 portable setsid。`std::os::windows::process` 提供
//! `creation_flags(JOB_OBJECT)` 等 API，但跨进程 group kill 仍是
//! `taskkill /T /F <pid>` 等 OS 调用，Rust std 没封装。所以 Windows
//! 路径本任务**不**做进程组隔离（注释里标了 future work）。Task 1.1
//! 已经在 BashTool 用了 `kill_on_drop` + `tokio::process::Command`，
//! 单独处理 tokio 路径的清理。

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

/// 在 Unix 上通过 setsid 把 child 放进新 session / process group，
/// 然后 spawn。Windows 上等价物缺失，fallback 到 plain spawn。
///
/// 用法：
/// ```ignore
/// let child = utils::spawn::safe_spawn(&mut Command::new("ls"))?;
/// ```
///
/// ## 设计点
///
/// - Unix：`unsafe { cmd.pre_exec(|| { libc::setsid(); Ok(()) }) }`
///   在 fork 之后、exec 之前运行。该 closure 必须只调 async-signal-safe
///   函数；`libc::setsid` 满足这个条件。
/// - Windows：直接 cmd.spawn()。Child 仍可能被 nested spawn 拉起
///   grandchild，但 BashTool 走 `tokio::process::Command` + `kill_on_drop`
///   单独处理。
/// - 函数返回 `std::io::Result<Child>`：caller 负责 wait/kill。本函数
///   只关心 spawn 阶段的进程组隔离。
/// - 接收 `&mut Command`（而非 owned）——`Command::args / .current_dir`
///   等 fluent 方法都返回 `&mut Self`，caller 写出
///   `Command::new(x).args(y).current_dir(z)` 时拿到的是 `&mut Command`。
///   接受 owned 反而逼 caller 写 `*cmd = Command::new(x); safe_spawn(cmd)`
///   两行，难看。
#[cfg(unix)]
pub fn safe_spawn(cmd: &mut Command) -> std::io::Result<std::process::Child> {
    // SAFETY: pre_exec closure runs in forked child, only async-signal-safe
    // ops allowed; setsid is async-signal-safe per POSIX.
    unsafe {
        cmd.pre_exec(|| {
            // 创建新 session + 新 process group，PID == PGID。
            // 之后任何 `kill(-pid, signal)` 都会向 group 全部成员发送。
            // errno 由 libc::setsid 直接返回；正常情况返回新 session id
            // （> 0），失败返回 -1。Rust 端无 error propagation 通道，
            // 失败时让 exec 失败即可（子进程会立即退出，shell 看得到）。
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()
}

/// Windows 上 setsid 不可用，fallback 到 plain spawn。Task 1.1 已经
/// 单独处理 BashTool 的 kill_on_drop + grandchild 清理，所以这个
/// fallback 在 tools crate 内的 std::process::Command::spawn 调用点上
/// 是可接受的。
#[cfg(not(unix))]
pub fn safe_spawn(cmd: &mut Command) -> std::io::Result<std::process::Child> {
    cmd.spawn()
}

#[cfg(test)]
#[allow(unused_imports)] // super::* and Command only used by cfg(unix) tests
mod tests {
    use super::*;

    /// 验证 Unix 上 safe_spawn 把 child 放到新 process group。
    ///
    /// 方法：跑 `sh -c 'echo $$; ps -o pgid='`。`$$` 是当前 shell 的 PID；
    /// setsid 之后 PID == PGID（因为新 session 第一个 process 就是 group
    /// leader）。如果两行相等则 setsid 生效。
    #[cfg(unix)]
    #[test]
    fn safe_spawn_creates_new_process_group() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo $$; ps -o pgid= -p $$");
        let child = safe_spawn(&mut cmd).expect("spawn sh");

        let output = child.wait_with_output().expect("wait child");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().split('\n').collect();
        assert_eq!(lines.len(), 2, "expected 2 lines (pid + pgid), got: {:?}", lines);
        let pid = lines[0].trim();
        let pgid = lines[1].trim();
        assert_eq!(pid, pgid, "PID should equal PGID after setsid");
    }

    /// 验证 safe_spawn 不会丢失普通 child 行为：能跑通、能拿到 exit code。
    #[cfg(unix)]
    #[test]
    fn safe_spawn_basic_exec_works() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("exit 7");
        let mut child = safe_spawn(&mut cmd).expect("spawn sh");
        let status = child.wait().expect("wait");
        assert_eq!(status.code(), Some(7));
    }
}
