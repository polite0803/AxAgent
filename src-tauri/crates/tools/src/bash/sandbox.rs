// SPDX-License-Identifier: AGPL-3.0-only

//! 沙箱执行器
//!
//! 支持 Docker/podman 容器执行和进程级隔离。
//!
//! ## Phase 2 Task 2.2 加固要点（native fallback 路径）
//!
//! 原版 `build_native_command` 直接 `Command::new("bash").arg("-c").arg(cmd)`，
//! 继承了父进程的全部 env vars（可能含 AWS_ACCESS_KEY、SSH_AUTH_SOCK、
//! ~/.bashrc 自定义 alias 注入等），无 rlimit，无 safe-mode flags。
//!
//! 现在 native 路径额外做：
//!
//! 1. `env_clear()` + 显式白名单（`PATH/HOME/TMPDIR/LANG`）— 防止
//!    敏感 env 泄露到子进程。
//! 2. 注入 `set -euo pipefail` 包裹原命令（Unix 路径）— 让 bash 严格
//!    失败退出、未定义变量报错、pipe 任一段失败都返回非零。
//! 3. `RLIMIT_AS=256MB / RLIMIT_CPU=60s / RLIMIT_NOFILE=1024`
//    （Unix，pre_exec 阶段）— 防止内存/CPU/FD DoS。
//!
//! Windows 上 rlimit 不可移植，env_clear + safe flags 仍生效；rlimit
//! 限制在注释中标注为 future work。

use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 使用容器（Docker/Podman）
    pub use_container: bool,
    /// 容器镜像
    pub image: String,
    /// 允许的网络访问
    pub allow_network: bool,
    /// 挂载的卷列表
    pub volumes: Vec<(String, String)>,
    /// 内存限制 (MB)
    pub memory_limit_mb: Option<u64>,
    /// CPU 限制
    pub cpu_limit: Option<f64>,
}

/// 检测 Docker 是否可用
fn detect_docker_available() -> bool {
    std::process::Command::new("docker")
        .args(["info", "--format", "{{.OSType}}"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            use_container: detect_docker_available(),
            image: "alpine:latest".into(),
            allow_network: false,
            volumes: Vec::new(),
            memory_limit_mb: Some(512),
            cpu_limit: Some(0.5),
        }
    }
}

/// 沙箱执行器
pub struct SandboxRunner {
    config: SandboxConfig,
}

impl SandboxRunner {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 构建沙箱命令
    pub fn build_command(&self, cmd: &str, working_dir: &str) -> Command {
        if self.config.use_container {
            self.build_docker_command(cmd, working_dir)
        } else {
            self.build_native_command(cmd, working_dir)
        }
    }

    fn build_docker_command(&self, cmd: &str, working_dir: &str) -> Command {
        let mut command = Command::new("docker");
        command.arg("run");
        command.arg("--rm");
        command.arg("--network=none");
        command.arg("--read-only");
        command.arg(format!("--workdir={}", working_dir));
        command.arg("-v");
        command.arg(format!("{}:{}", working_dir, working_dir));

        if let Some(mem) = self.config.memory_limit_mb {
            command.arg(format!("--memory={}m", mem));
        }
        if let Some(cpu) = self.config.cpu_limit {
            command.arg(format!("--cpus={}", cpu));
        }

        command.arg(&self.config.image);
        command.arg("bash");
        command.arg("-c");
        command.arg(cmd);

        command
    }

    fn build_native_command(&self, cmd: &str, _working_dir: &str) -> Command {
        // SECURITY (Phase 2 Task 2.2): native 路径不再继承父 env，并注入
        // safe-mode flags + rlimit。env_clear + 白名单先做，确保子进程
        // 拿不到 AWS_*/SSH_AUTH_SOCK 等敏感变量。
        #[cfg(target_os = "windows")]
        {
            // FIXME(Windows): rlimit 无原生等价物；cmd.exe 也不支持
            // set -euo pipefail。env_clear + PATH 注入仍然生效。
            let mut command = Command::new("cmd");
            command.arg("/C");
            command.arg(cmd);
            apply_safe_env(&mut command);
            command
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 把原命令包进 `set -euo pipefail` 块，让 bash 严格模式生效：
            //  -e : 任一命令非零退出立即结束
            //  -u : 引用未定义变量报错
            //  -o pipefail : 管道中任一段失败返回非零
            let wrapped = format!("set -euo pipefail; {}", cmd);
            let mut command = Command::new("bash");
            command.arg("-c");
            command.arg(wrapped);
            apply_safe_env(&mut command);
            // pre_exec 在子进程 fork 后、exec 前运行。
            // 唯一允许的"不安全"：调 libc setrlimit，参数都是常量。
            // SAFETY: closure runs in forked child; only async-signal-safe
            // ops allowed (setrlimit qualifies).
            unsafe {
                command.pre_exec(|| {
                    install_rlimits();
                    Ok(())
                });
            }
            command
        }
    }

    /// 执行沙箱命令
    pub fn execute(&self, cmd: &str, working_dir: &str) -> std::io::Result<std::process::Output> {
        let mut command = self.build_command(cmd, working_dir);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.output()
    }
}

/// 清空继承的 env vars 并显式注入白名单。
///
/// PATH 是命令解析的唯一必需；HOME / TMPDIR 防止某些工具 panic；
/// LANG=C.UTF-8 让子进程输出稳定（grep / sort / awk 行为依赖 locale）。
fn apply_safe_env(cmd: &mut Command) {
    cmd.env_clear();
    cmd.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    cmd.env("HOME", "/tmp");
    cmd.env("TMPDIR", "/tmp");
    cmd.env("LANG", "C.UTF-8");
}

/// 在子进程 fork 后、exec 前安装 rlimit。
///
/// - `RLIMIT_AS=256MB` — 防止 malloc 炸弹
/// - `RLIMIT_CPU=60s` — 防止死循环 CPU 耗尽
/// - `RLIMIT_NOFILE=1024` — 防止 fd 耗尽
///
/// 注：Docker 路径的 rlimit 由 docker --memory/--cpus 提供，**不**走
/// 此函数。rlimit 在容器内部并不需要重复设置（容器是独立 PID 命名空间）。
#[cfg(unix)]
fn install_rlimits() {
    // SAFETY: 在 fork 后、exec 前调用；setrlimit 是 async-signal-safe。
    // RLIMIT_AS 256 MB
    let mut lim_as = libc::rlimit {
        rlim_cur: 256 * 1024 * 1024,
        rlim_max: 256 * 1024 * 1024,
    };
    // rlim_t 在不同 unix 上可能是 u32 / u64，做一次显式转换。
    #[allow(clippy::useless_conversion)]
    unsafe {
        lim_as.rlim_cur = lim_as.rlim_cur.into();
        lim_as.rlim_max = lim_as.rlim_max.into();
        let _ = libc::setrlimit(libc::RLIMIT_AS, &lim_as);
    }

    // RLIMIT_CPU 60s
    let mut lim_cpu = libc::rlimit {
        rlim_cur: 60,
        rlim_max: 60,
    };
    #[allow(clippy::useless_conversion)]
    unsafe {
        lim_cpu.rlim_cur = lim_cpu.rlim_cur.into();
        lim_cpu.rlim_max = lim_cpu.rlim_max.into();
        let _ = libc::setrlimit(libc::RLIMIT_CPU, &lim_cpu);
    }

    // RLIMIT_NOFILE 1024
    let mut lim_nofile = libc::rlimit {
        rlim_cur: 1024,
        rlim_max: 1024,
    };
    #[allow(clippy::useless_conversion)]
    unsafe {
        lim_nofile.rlim_cur = lim_nofile.rlim_cur.into();
        lim_nofile.rlim_max = lim_nofile.rlim_max.into();
        let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &lim_nofile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_detect_does_not_panic() {
        let available = super::detect_docker_available();
        assert!(available == true || available == false);
    }

    #[test]
    fn sandbox_config_default_is_safe() {
        let config = SandboxConfig::default();
        assert!(!config.allow_network);
        assert!(config.memory_limit_mb.is_some());
    }

    /// SECURITY (Phase 2 Task 2.2): native 路径必须 env_clear。
    /// 父进程设置 `AXAGENT_TEST_LEAK=secret123`，子进程不应该看见。
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn native_sandbox_does_not_leak_env() {
        unsafe { std::env::set_var("AXAGENT_TEST_LEAK", "secret123") };

        // 强制 use_container=false；用本地 runner。
        let runner = SandboxRunner::new(SandboxConfig {
            use_container: false,
            ..Default::default()
        });
        let mut cmd = runner.build_command("echo \"LEAK=$AXAGENT_TEST_LEAK\"", "/tmp");
        let output = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("spawn native sandbox command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("secret123"),
            "AXAGENT_TEST_LEAK should not be visible to child, got: {}",
            stdout
        );
        assert!(
            !stdout.contains("LEAK=secret"),
            "child should not see AXAGENT_TEST_LEAK value, got: {}",
            stdout
        );

        // 清理测试副作用
        unsafe { std::env::remove_var("AXAGENT_TEST_LEAK") };
    }

    /// SECURITY (Phase 2 Task 2.2): native 路径必须注入 set -euo pipefail。
    /// 验证方法：跑一个会引用未定义变量的命令，应该以非零退出。
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn native_sandbox_uses_strict_mode() {
        let runner = SandboxRunner::new(SandboxConfig {
            use_container: false,
            ..Default::default()
        });
        // set -u 让未定义变量 $UNDEFINED_VAR 触发 non-zero exit。
        let mut cmd = runner.build_command("echo $UNDEFINED_VAR", "/tmp");
        let output = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("spawn native sandbox command");
        assert!(
            !output.status.success(),
            "set -u should fail on undefined var, got exit={:?} stdout={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
    }

    /// SECURITY (Phase 2 Task 2.2): native 路径应设置白名单 env（PATH 可用）。
    /// 验证 `which sh` 在白名单 PATH 中能找到 /usr/bin/sh（PATH 应有 /usr/bin）。
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn native_sandbox_has_safe_path() {
        let runner = SandboxRunner::new(SandboxConfig {
            use_container: false,
            ..Default::default()
        });
        let mut cmd = runner.build_command("command -v sh", "/tmp");
        let output = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("spawn native sandbox command");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("sh"), "sh should be in PATH, got: {}", stdout);
    }

    /// SECURITY (Phase 2 Task 2.2): 验证 apply_safe_env 在所有平台上
    /// 都能正确清空 env 并设置白名单。Windows + Unix 通用。
    ///
    /// 通过 std::process::Command 的 get_envs() 拿到 env iter：
    /// env_clear 后 size() 应该恰好等于 4（PATH/HOME/TMPDIR/LANG）。
    #[test]
    fn apply_safe_env_clears_and_whitelists() {
        let mut cmd = Command::new("true");
        apply_safe_env(&mut cmd);
        let keys: Vec<String> = cmd
            .get_envs()
            .map(|(k, _v)| k.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            keys.len(),
            4,
            "expected 4 whitelisted env vars, got {}: {:?}",
            keys.len(),
            keys
        );
        for required in &["PATH", "HOME", "TMPDIR", "LANG"] {
            assert!(
                keys.contains(&required.to_string()),
                "missing {} in env, got: {:?}",
                required,
                keys
            );
        }
    }

    /// SECURITY (Phase 2 Task 2.2): build_native_command 在 native 路径
    /// 必须 env_clear（哪怕父进程有自定义 var）。
    /// Windows 路径走 cmd.exe，本测试只验证 builder 层 env 状态。
    #[test]
    fn build_native_command_clears_env() {
        let runner = SandboxRunner::new(SandboxConfig {
            use_container: false,
            ..Default::default()
        });
        let cmd = runner.build_command("true", "/tmp");
        let count = cmd.get_envs().count();
        // 4 = PATH/HOME/TMPDIR/LANG（apply_safe_env）
        assert_eq!(count, 4, "native cmd should have only 4 whitelisted env vars, got {}", count);
    }
}
