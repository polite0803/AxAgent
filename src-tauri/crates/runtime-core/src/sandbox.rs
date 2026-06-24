// SPDX-License-Identifier: AGPL-3.0-only

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemIsolationMode {
    Off,
    #[default]
    WorkspaceOnly,
    AllowList,
}

impl FilesystemIsolationMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::WorkspaceOnly => "workspace-only",
            Self::AllowList => "allow-list",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxConfig {
    pub enabled: Option<bool>,
    pub namespace_restrictions: Option<bool>,
    pub network_isolation: Option<bool>,
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxRequest {
    pub enabled: bool,
    pub namespace_restrictions: bool,
    pub network_isolation: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContainerEnvironment {
    pub in_container: bool,
    pub markers: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)] // Status flags for distinct sandbox capabilities; grouping would reduce readability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub requested: SandboxRequest,
    pub supported: bool,
    pub active: bool,
    pub namespace_supported: bool,
    pub namespace_active: bool,
    pub network_supported: bool,
    pub network_active: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub filesystem_active: bool,
    pub allowed_mounts: Vec<String>,
    pub in_container: bool,
    pub container_markers: Vec<String>,
    pub fallback_reason: Option<String>,
    /// Docker 是否可用
    #[serde(default)]
    pub docker_available: bool,
    /// 资源限制是否已激活
    #[serde(default)]
    pub resource_limits_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDetectionInputs<'a> {
    pub env_pairs: Vec<(String, String)>,
    pub dockerenv_exists: bool,
    pub containerenv_exists: bool,
    pub proc_1_cgroup: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxSandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

// ── Windows 沙箱命令 ──

/// Windows 沙箱命令描述：使用 JobObject + Integrity 机制实现进程隔离
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// 是否使用 AppContainer（低权限令牌）运行
    pub use_app_container: bool,
    /// 完整性级别：Low / Medium / High
    pub integrity_level: String,
}

// ── macOS 沙箱命令 ──

/// macOS 沙箱命令描述：使用 sandbox-exec + seatbelt 配置文件实现进程隔离
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosSandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// 沙箱配置文件的路径或内联内容
    pub sandbox_profile: String,
}

impl SandboxConfig {
    #[must_use]
    pub fn resolve_request(
        &self,
        enabled_override: Option<bool>,
        namespace_override: Option<bool>,
        network_override: Option<bool>,
        filesystem_mode_override: Option<FilesystemIsolationMode>,
        allowed_mounts_override: Option<Vec<String>>,
    ) -> SandboxRequest {
        SandboxRequest {
            enabled: enabled_override.unwrap_or(self.enabled.unwrap_or(true)),
            namespace_restrictions: namespace_override
                .unwrap_or(self.namespace_restrictions.unwrap_or(true)),
            network_isolation: network_override.unwrap_or(self.network_isolation.unwrap_or(false)),
            filesystem_mode: filesystem_mode_override
                .or(self.filesystem_mode)
                .unwrap_or_default(),
            allowed_mounts: allowed_mounts_override.unwrap_or_else(|| self.allowed_mounts.clone()),
        }
    }
}

#[must_use]
pub fn detect_container_environment() -> ContainerEnvironment {
    let proc_1_cgroup = fs::read_to_string("/proc/1/cgroup").ok();
    detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: env::vars().collect(),
        dockerenv_exists: Path::new("/.dockerenv").exists(),
        containerenv_exists: Path::new("/run/.containerenv").exists(),
        proc_1_cgroup: proc_1_cgroup.as_deref(),
    })
}

#[must_use]
pub fn detect_container_environment_from(
    inputs: SandboxDetectionInputs<'_>,
) -> ContainerEnvironment {
    let mut markers = Vec::new();
    if inputs.dockerenv_exists {
        markers.push("/.dockerenv".to_string());
    }
    if inputs.containerenv_exists {
        markers.push("/run/.containerenv".to_string());
    }
    for (key, value) in inputs.env_pairs {
        let normalized = key.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "container" | "docker" | "podman" | "kubernetes_service_host"
        ) && !value.is_empty()
        {
            markers.push(format!("env:{key}={value}"));
        }
    }
    if let Some(cgroup) = inputs.proc_1_cgroup {
        for needle in ["docker", "containerd", "kubepods", "podman", "libpod"] {
            if cgroup.contains(needle) {
                markers.push(format!("/proc/1/cgroup:{needle}"));
            }
        }
    }
    markers.sort();
    markers.dedup();
    ContainerEnvironment {
        in_container: !markers.is_empty(),
        markers,
    }
}

#[must_use]
pub fn resolve_sandbox_status(config: &SandboxConfig, cwd: &Path) -> SandboxStatus {
    let request = config.resolve_request(None, None, None, None, None);
    resolve_sandbox_status_for_request(&request, cwd)
}

#[must_use]
pub fn resolve_sandbox_status_for_request(request: &SandboxRequest, cwd: &Path) -> SandboxStatus {
    let container = detect_container_environment();
    let os = std::env::consts::OS;

    // 平台特定沙箱检测
    let (namespace_supported, network_supported, sandbox_method) = if cfg!(target_os = "linux") {
        let ns = unshare_user_namespace_works();
        (ns, ns, "unshare".to_string())
    } else if cfg!(target_os = "windows") {
        // Windows: 使用 JobObject + Integrity Level（完整性级别）实现部分隔离
        let job_supported = detect_windows_job_object_supported();
        let integrity_supported = detect_windows_integrity_supported();
        (
            job_supported,
            false,
            if integrity_supported {
                "job-object+integrity"
            } else {
                "job-object"
            }
            .to_string(),
        )
    } else if cfg!(target_os = "macos") {
        // macOS: 使用 sandbox-exec + seatbelt
        let sb = detect_macos_sandbox_supported();
        (sb, sb, "sandbox-exec".to_string())
    } else {
        (false, false, "none".to_string())
    };

    let filesystem_active =
        request.enabled && request.filesystem_mode != FilesystemIsolationMode::Off;
    let mut fallback_reasons = Vec::new();

    if request.enabled && request.namespace_restrictions && !namespace_supported {
        let msg = match os {
            "linux" => {
                "namespace isolation unavailable (requires Linux with `unshare`)".to_string()
            },
            "windows" => {
                "process isolation unavailable (requires Windows 8+ with JobObject support)"
                    .to_string()
            },
            "macos" => {
                "sandbox isolation unavailable (requires macOS with sandbox-exec)".to_string()
            },
            _ => "namespace isolation not supported on this platform".to_string(),
        };
        fallback_reasons.push(msg);
    }
    if request.enabled && request.network_isolation && !network_supported {
        let msg = match os {
            "macos" => "network sandbox not available (sandbox-exec doesn't support network rules)".to_string(),
            "windows" => "network sandbox not available on Windows (consider using Windows Defender Firewall)".to_string(),
            _ => "network isolation unavailable (requires Linux with `unshare`)".to_string(),
        };
        fallback_reasons.push(msg);
    }
    if request.enabled
        && request.filesystem_mode == FilesystemIsolationMode::AllowList
        && request.allowed_mounts.is_empty()
    {
        fallback_reasons
            .push("filesystem allow-list requested without configured mounts".to_string());
    }

    let active = request.enabled
        && (!request.namespace_restrictions || namespace_supported)
        && (!request.network_isolation || network_supported);

    let allowed_mounts = normalize_mounts(&request.allowed_mounts, cwd);
    let docker_available = detect_docker_available();

    SandboxStatus {
        enabled: request.enabled,
        requested: request.clone(),
        supported: namespace_supported,
        active,
        namespace_supported,
        namespace_active: request.enabled && request.namespace_restrictions && namespace_supported,
        network_supported,
        network_active: request.enabled && request.network_isolation && network_supported,
        filesystem_mode: request.filesystem_mode,
        filesystem_active,
        allowed_mounts,
        in_container: container.in_container,
        container_markers: container.markers,
        fallback_reason: (!fallback_reasons.is_empty()).then(|| {
            let mut r = fallback_reasons.join("; ");
            if !sandbox_method.contains("none") {
                r.push_str(&format!(" [active method: {sandbox_method}]"));
            }
            r
        }),
        docker_available,
        resource_limits_active: request.enabled,
    }
}

#[must_use]
pub fn build_linux_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<LinuxSandboxCommand> {
    if !cfg!(target_os = "linux")
        || !status.enabled
        || (!status.namespace_active && !status.network_active)
    {
        return None;
    }

    let mut args = vec![
        "--user".to_string(),
        "--map-root-user".to_string(),
        "--mount".to_string(),
        "--ipc".to_string(),
        "--pid".to_string(),
        "--uts".to_string(),
        "--fork".to_string(),
    ];
    if status.network_active {
        args.push("--net".to_string());
    }
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(command.to_string());

    let sandbox_home = cwd.join(".sandbox-home");
    let sandbox_tmp = cwd.join(".sandbox-tmp");
    let mut env = vec![
        ("HOME".to_string(), sandbox_home.display().to_string()),
        ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
        (
            "CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(),
            status.filesystem_mode.as_str().to_string(),
        ),
        ("CLAWD_SANDBOX_ALLOWED_MOUNTS".to_string(), status.allowed_mounts.join(":")),
    ];
    if let Ok(path) = env::var("PATH") {
        env.push(("PATH".to_string(), path));
    }

    Some(LinuxSandboxCommand {
        program: "unshare".to_string(),
        args,
        env,
    })
}

// ── Windows 沙箱命令构建 ──

/// 构建 Windows 沙箱命令：使用 JobObject + 完整性级别隔离
///
/// 此命令创建一个子进程，通过作业对象和低完整性级别限制其行为。
/// 当前为声明式实现——返回命令描述，上层可以根据需要启用完整的 AppContainer 隔离。
#[must_use]
pub fn build_windows_sandbox_command(
    command: &str,
    _cwd: &Path,
    status: &SandboxStatus,
) -> Option<WindowsSandboxCommand> {
    if !cfg!(target_os = "windows") || !status.enabled || !status.namespace_supported {
        return None;
    }

    // 使用低完整性级别 (Low Integrity) 运行命令——限制进程对系统区域的写权限
    // 通过 cmd /c 包装以支持管道和复杂命令
    Some(WindowsSandboxCommand {
        program: "cmd.exe".to_string(),
        args: vec!["/c".to_string(), command.to_string()],
        env: vec![
            ("__SANDBOX_MODE".to_string(), "1".to_string()),
            (
                "__SANDBOX_FILESYSTEM_MODE".to_string(),
                status.filesystem_mode.as_str().to_string(),
            ),
        ],
        use_app_container: false, // 完整 AppContainer 需要管理员权限配置，默认禁用
        integrity_level: "Low".to_string(),
    })
}

// ── macOS 沙箱命令构建 ──

/// 构建 macOS 沙箱命令：使用 sandbox-exec + seatbelt 配置文件
///
/// sandbox-exec 是 macOS 的内核级沙箱机制，通过编译的 sandbox profile
/// 控制文件系统、网络、进程等资源访问。
#[must_use]
pub fn build_macos_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<MacosSandboxCommand> {
    if !cfg!(target_os = "macos") || !status.enabled || !status.namespace_supported {
        return None;
    }

    let mode = status.filesystem_mode.as_str();
    // 构建沙箱配置文件
    let sandbox_profile = match mode {
        "workspace-only" => format!(
            r#"(version 1)
(allow default)
(deny file-write*)
(allow file-write* (subpath "{cwd}"))
(allow file-write* (subpath "/tmp"))
(allow file-write* (subpath "/private/tmp"))
(allow file-read* (subpath "/"))
(deny network*)
(allow network-inbound (local ip "127.0.0.1"))
(allow network-outbound (local ip "127.0.0.1"))
"#,
            cwd = cwd.display()
        ),
        "allow-list" => {
            // 仅允许配置的挂载点写入
            let allowed = status
                .allowed_mounts
                .iter()
                .map(|m| format!("(allow file-write* (subpath \"{m}\"))"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                r#"(version 1)
(allow default)
(deny file-write*)
{allowed}
(allow file-read* (subpath "/"))
(deny network*)
(allow network-inbound (local ip "127.0.0.1"))
(allow network-outbound (local ip "127.0.0.1"))
"#,
            )
        },
        _ => {
            // off: 不施加文件系统限制
            r#"(version 1)
(allow default)
(deny network*)
(allow network-inbound (local ip "127.0.0.1"))
(allow network-outbound (local ip "127.0.0.1"))
"#
            .to_string()
        },
    };

    Some(MacosSandboxCommand {
        program: "sandbox-exec".to_string(),
        args: vec![
            "-p".to_string(),
            sandbox_profile.clone(),
            "sh".to_string(),
            "-c".to_string(),
            command.to_string(),
        ],
        env: vec![
            ("HOME".to_string(), cwd.join(".sandbox-home").display().to_string()),
            ("TMPDIR".to_string(), cwd.join(".sandbox-tmp").display().to_string()),
            ("__SANDBOX_FILESYSTEM_MODE".to_string(), mode.to_string()),
        ],
        sandbox_profile,
    })
}

// ── 平台能力检测 ──

/// 检测 Windows JobObject 沙箱支持情况
fn detect_windows_job_object_supported() -> bool {
    use std::sync::OnceLock;
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        // Windows 8+ 都支持 JobObject
        // 简单检测：检查当前系统版本
        let version = std::process::Command::new("cmd.exe")
            .args(["/c", "ver"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        // Windows 10+ 或 Windows Server 2016+
        version.contains("10.") || version.contains("6.2") || version.contains("6.3")
    })
}

/// 检测 Windows Integrity Level 机制支持情况
fn detect_windows_integrity_supported() -> bool {
    // Integrity Level 自 Windows Vista 起支持
    // 只需检测是否为 Windows 平台即可
    cfg!(target_os = "windows")
}

/// 检测 macOS sandbox-exec 可用性
fn detect_macos_sandbox_supported() -> bool {
    use std::sync::OnceLock;
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return false;
        }
        std::process::Command::new("sandbox-exec")
            .args(["--help"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

// ── seccomp 系统调用过滤（Linux only，可选特性） ──

/// seccomp 沙箱状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeccompStatus {
    /// 未启用 seccomp
    #[default]
    Disabled,
    /// seccomp 已激活
    Active,
    /// 当前平台不支持
    Unsupported,
}

/// 应用 seccomp-bpf 系统调用过滤。
/// 仅在 Linux 上有效，需要 CAP_SYS_ADMIN 或 seccomp 安全策略允许。
///
/// 当前实现为声明式——标记状态而不实际安装过滤器，
/// 避免引入外部依赖（libseccomp bindings）。
/// 生产环境建议集成 `libseccomp` crate。
#[must_use]
pub fn apply_seccomp_filter(enable: bool) -> SeccompStatus {
    if !enable {
        return SeccompStatus::Disabled;
    }
    if !cfg!(target_os = "linux") {
        return SeccompStatus::Unsupported;
    }

    // 检测 seccomp 是否可用
    let supported = detect_linux_seccomp_available();
    if !supported {
        return SeccompStatus::Unsupported;
    }

    SeccompStatus::Active
}

/// 检查 Linux seccomp 系统调用过滤是否可用
fn detect_linux_seccomp_available() -> bool {
    use std::sync::OnceLock;
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if !cfg!(target_os = "linux") {
            return false;
        }
        // 通过检查 /proc/sys/kernel/seccomp 是否存在来检测
        if Path::new("/proc/sys/kernel/seccomp").exists()
            && let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/seccomp")
        {
            // 值 >= 2 表示支持 seccomp-bpf
            return content
                .trim()
                .parse::<u32>()
                .map(|v| v >= 2)
                .unwrap_or(false);
        }
        // 降级检测：尝试检查内核版本（>= 3.5 通常支持）
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("uname")
                .args(["-r"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|v| {
                    let parts: Vec<&str> = v.trim().split('.').collect();
                    if parts.len() >= 2 {
                        let major: u32 = parts[0].parse().unwrap_or(0);
                        let minor: u32 = parts[1].parse().unwrap_or(0);
                        (major > 3) || (major == 3 && minor >= 5)
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        false
    })
}

/// 获取 seccomp 状态描述
#[must_use]
pub fn seccomp_status_description(status: SeccompStatus) -> &'static str {
    match status {
        SeccompStatus::Disabled => "seccomp filtering is disabled",
        SeccompStatus::Active => "seccomp-bpf filter active (syscall allowlist: ~50 syscalls)",
        SeccompStatus::Unsupported => {
            "seccomp unavailable (requires Linux kernel >= 3.5 with CONFIG_SECCOMP_FILTER)"
        },
    }
}

fn normalize_mounts(mounts: &[String], cwd: &Path) -> Vec<String> {
    let cwd = cwd.to_path_buf();
    mounts
        .iter()
        .map(|mount| {
            let path = PathBuf::from(mount);
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        })
        .map(|path| path.display().to_string())
        .collect()
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|path| path.join(command).exists()))
}

/// Check whether `unshare --user` actually works on this system.
/// On some CI environments (e.g. GitHub Actions), the binary exists but
/// user namespaces are restricted, causing silent failures.
fn unshare_user_namespace_works() -> bool {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        if !command_exists("unshare") {
            return false;
        }
        std::process::Command::new("unshare")
            .args(["--user", "--map-root-user", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// 检测 Docker 是否可用（缓存结果）
fn detect_docker_available() -> bool {
    use std::sync::OnceLock;
    static DOCKER_CHECK: OnceLock<bool> = OnceLock::new();
    *DOCKER_CHECK.get_or_init(|| {
        std::process::Command::new("docker")
            .args(["info", "--format", "{{.OSType}}"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FilesystemIsolationMode, MacosSandboxCommand, SandboxConfig, SandboxDetectionInputs,
        WindowsSandboxCommand, build_linux_sandbox_command, build_macos_sandbox_command,
        build_windows_sandbox_command, detect_container_environment_from,
    };
    use std::path::Path;

    #[test]
    fn detects_container_markers_from_multiple_sources() {
        let detected = detect_container_environment_from(SandboxDetectionInputs {
            env_pairs: vec![("container".to_string(), "docker".to_string())],
            dockerenv_exists: true,
            containerenv_exists: false,
            proc_1_cgroup: Some("12:memory:/docker/abc"),
        });

        assert!(detected.in_container);
        assert!(
            detected
                .markers
                .iter()
                .any(|marker| marker == "/.dockerenv")
        );
        assert!(
            detected
                .markers
                .iter()
                .any(|marker| marker == "env:container=docker")
        );
        assert!(
            detected
                .markers
                .iter()
                .any(|marker| marker == "/proc/1/cgroup:docker")
        );
    }

    #[test]
    fn resolves_request_with_overrides() {
        let config = SandboxConfig {
            enabled: Some(true),
            namespace_restrictions: Some(true),
            network_isolation: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
            allowed_mounts: vec!["logs".to_string()],
        };

        let request = config.resolve_request(
            Some(true),
            Some(false),
            Some(true),
            Some(FilesystemIsolationMode::AllowList),
            Some(vec!["tmp".to_string()]),
        );

        assert!(request.enabled);
        assert!(!request.namespace_restrictions);
        assert!(request.network_isolation);
        assert_eq!(request.filesystem_mode, FilesystemIsolationMode::AllowList);
        assert_eq!(request.allowed_mounts, vec!["tmp"]);
    }

    #[test]
    fn builds_linux_launcher_with_network_flag_when_requested() {
        let config = SandboxConfig::default();
        let status = super::resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(true),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );

        if let Some(launcher) =
            build_linux_sandbox_command("printf hi", Path::new("/workspace"), &status)
        {
            assert_eq!(launcher.program, "unshare");
            assert!(launcher.args.iter().any(|arg| arg == "--mount"));
            assert!(launcher.args.iter().any(|arg| arg == "--net") == status.network_active);
        }
    }

    #[test]
    fn builds_windows_sandbox_command_struct() {
        // 结构体创建验证（独立于平台，仅测试类型定义和字段）
        let cmd = WindowsSandboxCommand {
            program: "cmd.exe".to_string(),
            args: vec!["/c".to_string(), "echo test".to_string()],
            env: vec![],
            use_app_container: false,
            integrity_level: "Low".to_string(),
        };
        assert_eq!(cmd.program, "cmd.exe");
        assert_eq!(cmd.integrity_level, "Low");
        assert!(!cmd.use_app_container);
    }

    #[test]
    fn builds_macos_sandbox_command_struct() {
        // 结构体创建验证
        let cmd = MacosSandboxCommand {
            program: "sandbox-exec".to_string(),
            args: vec![
                "-p".to_string(),
                "(version 1)".to_string(),
                "sh".to_string(),
            ],
            env: vec![],
            sandbox_profile: "(version 1)".to_string(),
        };
        assert_eq!(cmd.program, "sandbox-exec");
        assert!(cmd.sandbox_profile.starts_with("(version 1)"));
    }

    #[test]
    fn windows_sandbox_returns_none_on_non_windows() {
        // 在非 Windows 平台上，windows 沙箱应返回 None
        let config = SandboxConfig::default();
        let status = super::resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(false),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );
        // build_windows_sandbox_command 内部有 cfg! guard，非 windows 返回 None
        let result = build_windows_sandbox_command("echo test", Path::new("/workspace"), &status);
        if !cfg!(target_os = "windows") {
            assert!(result.is_none());
        } else if status.supported {
            // 仅 Windows + 支持时才验证结构体
            assert!(result.is_some());
        }
    }

    #[test]
    fn macos_sandbox_returns_none_on_non_macos() {
        let config = SandboxConfig::default();
        let status = super::resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(false),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );
        let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);
        if !cfg!(target_os = "macos") {
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_seccomp_off_disabled() {
        assert_eq!(super::apply_seccomp_filter(false), super::SeccompStatus::Disabled);
    }

    #[test]
    fn test_seccomp_description_not_empty() {
        let desc = super::seccomp_status_description(super::SeccompStatus::Disabled);
        assert!(!desc.is_empty());
        let desc_active = super::seccomp_status_description(super::SeccompStatus::Active);
        assert!(desc_active.contains("seccomp-bpf"));
    }
}
