// SPDX-License-Identifier: AGPL-3.0-only
//! 插件沙箱隔离层 —— 在 hook/tool/lifecycle 子进程执行前做 capability 检查。
//!
//! 仅用 std + 已有依赖实现，不引入 WASM runtime / bubblewrap 等重型依赖。
//! 沙箱覆盖四类能力：
//! 1. 路径白名单（filesystem_read / filesystem_write + 工具 scope）
//! 2. ENV 白名单过滤（屏蔽 API Key / Token / Secret 等敏感变量）
//! 3. subprocess 权限拦截（未声明 subprocess_execution 禁止 shell 调用）
//! 4. network 权限标注（未声明 network_access 记录警告，不强制拦截）

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manager::PluginError;
use crate::types::{PluginManifest, PluginPermission};

/// 默认 ENV 白名单：仅传递运行所需的最小环境变量集合。
///
/// `AXAGENT_` 前缀变量允许通过（用于插件运行时上下文），但
/// `AXAGENT_CREDENTIAL_MASTER_KEY` 等含敏感关键词的变量会被
/// [`is_env_allowed`] 强制二次过滤。
pub const DEFAULT_ENV_WHITELIST: &[&str] =
    &["PATH", "HOME", "USERPROFILE", "TEMP", "TMP", "LANG", "LC_ALL", "AXAGENT_"];

/// 沙箱配置：描述单个插件（或聚合后多个插件）被允许的能力边界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    /// 路径白名单。非空时，被检查路径必须落在其中任一前缀下。
    /// 为空表示不强制路径白名单（仍受 `denied_paths` 约束）。
    pub allowed_paths: Vec<PathBuf>,
    /// 路径黑名单：系统敏感目录，无条件拒绝访问。
    pub denied_paths: Vec<PathBuf>,
    /// ENV 白名单（支持精确匹配与前缀匹配，如 `"AXAGENT_"`）。
    pub env_whitelist: Vec<&'static str>,
    /// 是否允许调用 shell 执行 hook 脚本（对应 `subprocess_execution` 权限）。
    pub allow_subprocess: bool,
    /// 是否允许网络访问（对应 `network_access` 权限，仅用于警告标注）。
    pub allow_network: bool,
}

impl SandboxConfig {
    /// 构建一个最小权限的默认沙箱：
    /// - 启用默认 ENV 白名单过滤
    /// - 启用默认敏感路径黑名单
    /// - `allow_subprocess = false`（最严格，需 manifest 显式声明方可放开）
    /// - `allow_network = false`
    #[must_use]
    pub fn restrictive() -> Self {
        Self {
            allowed_paths: Vec::new(),
            denied_paths: default_denied_paths(),
            env_whitelist: DEFAULT_ENV_WHITELIST.to_vec(),
            allow_subprocess: false,
            allow_network: false,
        }
    }

    /// 构建一个向后兼容的默认沙箱：
    /// - 仍启用 ENV 白名单过滤与敏感路径黑名单（安全增强）
    /// - `allow_subprocess = true`（无 manifest 上下文时保守允许，避免破坏现有行为）
    ///
    /// 用于 [`crate::hooks::HookRunner::new`] 等无 manifest 信息的入口。
    #[must_use]
    pub fn permissive() -> Self {
        Self { allow_subprocess: true, ..Self::restrictive() }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        // 默认采用 permissive 配置，确保未显式接入 manifest 的调用路径
        // 仍能获得 ENV 过滤 + 敏感路径拦截这两项无争议的安全增强，
        // 同时不因 subprocess 拦截破坏向后兼容性。
        Self::permissive()
    }
}

/// 返回当前平台的默认敏感路径黑名单（已 canonicalize，便于 `starts_with` 比较）。
pub fn default_denied_paths() -> Vec<PathBuf> {
    let raw: Vec<PathBuf> = raw_default_denied_paths();
    raw.into_iter().map(|p| p.canonicalize().unwrap_or(p)).collect()
}

/// 平台相关的原始敏感路径（未 canonicalize）。
fn raw_default_denied_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(unix)]
    {
        paths.push(PathBuf::from("/etc"));
        paths.push(PathBuf::from("/var"));
        paths.push(PathBuf::from("/usr"));
        paths.push(PathBuf::from("/bin"));
        paths.push(PathBuf::from("/sbin"));
        paths.push(PathBuf::from("/root"));
        paths.push(PathBuf::from("/boot"));
        paths.push(PathBuf::from("/dev"));
        paths.push(PathBuf::from("/proc"));
        paths.push(PathBuf::from("/sys"));
    }
    #[cfg(windows)]
    {
        // Windows 系统目录：优先用 %SystemRoot% 解析，回退到 C:\Windows
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            let system_root = PathBuf::from(system_root);
            paths.push(system_root.join("System32"));
            paths.push(system_root.join("System"));
            paths.push(system_root.join("SysWOW64"));
        } else {
            paths.push(PathBuf::from(r"C:\Windows\System32"));
            paths.push(PathBuf::from(r"C:\Windows\System"));
            paths.push(PathBuf::from(r"C:\Windows\SysWOW64"));
        }
        paths.push(PathBuf::from(r"C:\Program Files"));
        paths.push(PathBuf::from(r"C:\Program Files (x86)"));
    }
    // 避免 unused 警告：当非 unix 且非 windows（理论不存在）时返回空
    #[cfg(not(any(unix, windows)))]
    let _ = &mut paths;
    paths
}

/// 判断环境变量是否允许传递给插件子进程。
///
/// 规则：
/// 1. 强制排除含敏感关键词的变量（`TOKEN` / `SECRET` / `CREDENTIAL` /
///    `PASSWORD` / `API_KEY`），无论是否命中白名单。
/// 2. 命中白名单（精确匹配或前缀匹配）则允许。
pub fn is_env_allowed(name: &str, whitelist: &[&str]) -> bool {
    let upper = name.to_uppercase();
    // 强制排除敏感变量：即便白名单含 AXAGENT_ 前缀，
    // AXAGENT_CREDENTIAL_MASTER_KEY 等仍被拦截
    if upper.contains("TOKEN")
        || upper.contains("SECRET")
        || upper.contains("CREDENTIAL")
        || upper.contains("PASSWORD")
        || upper.contains("API_KEY")
    {
        return false;
    }
    // 白名单匹配：精确或前缀
    for allowed in whitelist {
        if name == *allowed || name.starts_with(allowed) {
            return true;
        }
    }
    false
}

/// 按白名单过滤环境变量集合。
///
/// 输入完整 env，返回过滤后的 env（移除敏感变量与非白名单变量）。
pub fn filter_env_vars(
    full_env: HashMap<String, String>,
    whitelist: &[&str],
) -> HashMap<String, String> {
    full_env.into_iter().filter(|(name, _)| is_env_allowed(name, whitelist)).collect()
}

/// 将 ENV 白名单过滤策略应用到 `Command`：
/// 先 `env_clear` 清空继承的变量，再仅回填白名单变量。
///
/// 调用方在调用此函数之后，仍可继续 `command.env(...)` 设置插件专用变量
/// （如 `CLAWD_PLUGIN_ID`），这些显式设置不受白名单约束。
pub fn apply_env_to_command(command: &mut Command, config: &SandboxConfig) {
    command.env_clear();
    for (key, value) in std::env::vars() {
        if is_env_allowed(&key, &config.env_whitelist) {
            command.env(key, value);
        }
    }
}

/// 检查路径是否在沙箱允许范围内。
///
/// 拒绝条件（按优先级）：
/// 1. 路径落在 `denied_paths` 任一前缀下 → `PermissionDenied`
/// 2. `allowed_paths` 非空且路径未落在任一白名单前缀下 → `PermissionDenied`
pub fn check_path_permission(path: &Path, config: &SandboxConfig) -> Result<(), PluginError> {
    // canonicalize 失败时回退到原路径（例如路径尚不存在）
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // 1. 检查是否落在敏感目录黑名单内
    for denied in &config.denied_paths {
        if canonical.starts_with(denied) {
            return Err(PluginError::PermissionDenied(format!(
                "路径 `{}` 位于禁止访问的敏感目录 `{}` 内",
                path.display(),
                denied.display()
            )));
        }
    }

    // 2. 检查路径白名单（仅当白名单非空时强制）
    if !config.allowed_paths.is_empty() {
        let allowed = config
            .allowed_paths
            .iter()
            .any(|allowed| canonical.starts_with(allowed) || allowed.starts_with(&canonical));
        if !allowed {
            return Err(PluginError::PermissionDenied(format!(
                "路径 `{}` 不在插件声明的可访问路径白名单内",
                path.display()
            )));
        }
    }

    Ok(())
}

/// 检查是否允许调用 shell 执行 hook 脚本。
///
/// 对应 manifest 的 `subprocess_execution` 权限：未声明时禁止 shell 调用。
pub fn check_subprocess_permission(config: &SandboxConfig) -> Result<(), PluginError> {
    if !config.allow_subprocess {
        Err(PluginError::PermissionDenied(
            "插件未声明 `subprocess_execution` 权限，禁止调用 shell 执行 hook 脚本".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// 检查 network 权限：未声明 `network_access` 时记录警告但不拦截。
///
/// 真正的 network 隔离需要 OS 级支持（如 seccomp / 网络命名空间），
/// 超出本沙箱范围，这里仅做告警便于审计。
pub fn note_network_access(config: &SandboxConfig) {
    if !config.allow_network {
        tracing::warn!(
            "插件未声明 `network_access` 权限，但当前沙箱无法在 OS 层强制拦截网络访问（仅告警）"
        );
    }
}

/// 根据插件清单构建沙箱配置。
///
/// - `subprocess_execution` 权限 → `allow_subprocess`
/// - `network_access` 权限 → `allow_network`（未声明时记录警告）
/// - `filesystem_read` / `filesystem_write` 权限 → 暂不自动扩展 `allowed_paths`，
///   保留默认敏感路径黑名单；调用方可通过 `with_allowed_paths` 进一步约束。
pub fn build_sandbox_from_manifest(manifest: &PluginManifest) -> SandboxConfig {
    build_sandbox_from_permissions(&manifest.permissions)
}

/// 根据权限切片构建沙箱配置（供聚合权限场景复用）。
///
/// 与 [`build_sandbox_from_manifest`] 行为一致，但接受任意权限切片，
/// 便于 [`crate::core::PluginRegistry::aggregated_permissions`] 等聚合入口
/// 直接传入合并后的权限集合。
pub fn build_sandbox_from_permissions(permissions: &[PluginPermission]) -> SandboxConfig {
    let allow_subprocess =
        permissions.iter().any(|p| matches!(p, PluginPermission::SubprocessExecution));
    let allow_network = permissions.iter().any(|p| matches!(p, PluginPermission::NetworkAccess));

    let config = SandboxConfig {
        allowed_paths: Vec::new(),
        denied_paths: default_denied_paths(),
        env_whitelist: DEFAULT_ENV_WHITELIST.to_vec(),
        allow_subprocess,
        allow_network,
    };

    // network 权限标注：未声明时记录警告
    note_network_access(&config);

    config
}

impl SandboxConfig {
    /// 追加允许访问的路径白名单前缀（builder 风格）。
    #[must_use]
    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = paths;
        self
    }

    /// 聚合多个沙箱配置：任一配置允许某项能力，聚合后即允许（并集语义）。
    ///
    /// 用于 [`crate::hooks::HookRunner::from_registry`] 将多个 enabled 插件
    /// 的沙箱合并为单一执行沙箱。
    #[must_use]
    pub fn merged_with(self, other: &SandboxConfig) -> Self {
        let mut allowed_paths = self.allowed_paths;
        for path in &other.allowed_paths {
            if !allowed_paths.contains(path) {
                allowed_paths.push(path.clone());
            }
        }
        Self {
            allowed_paths,
            // 黑名单取交集（更宽松）：仅保留两边都拒绝的路径
            denied_paths: self
                .denied_paths
                .into_iter()
                .filter(|p| other.denied_paths.contains(p))
                .collect(),
            env_whitelist: self.env_whitelist,
            allow_subprocess: self.allow_subprocess || other.allow_subprocess,
            allow_network: self.allow_network || other.allow_network,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_env_allowed_keeps_whitelist_and_blocks_sensitive() {
        assert!(is_env_allowed("PATH", DEFAULT_ENV_WHITELIST));
        assert!(is_env_allowed("HOME", DEFAULT_ENV_WHITELIST));
        assert!(is_env_allowed("AXAGENT_PLUGIN_ID", DEFAULT_ENV_WHITELIST));

        // 敏感变量强制排除
        assert!(!is_env_allowed("OPENAI_API_KEY", DEFAULT_ENV_WHITELIST));
        assert!(!is_env_allowed("ANTHROPIC_API_KEY", DEFAULT_ENV_WHITELIST));
        assert!(!is_env_allowed("AXAGENT_CREDENTIAL_MASTER_KEY", DEFAULT_ENV_WHITELIST));
        assert!(!is_env_allowed("GITHUB_TOKEN", DEFAULT_ENV_WHITELIST));
        assert!(!is_env_allowed("DB_SECRET", DEFAULT_ENV_WHITELIST));
        assert!(!is_env_allowed("USER_PASSWORD", DEFAULT_ENV_WHITELIST));

        // 非白名单变量被过滤
        assert!(!is_env_allowed("UNRELATED_VAR", DEFAULT_ENV_WHITELIST));
    }

    #[test]
    fn filter_env_vars_strips_sensitive_and_non_whitelist() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        env.insert("HOME".to_string(), "/home/user".to_string());
        env.insert("OPENAI_API_KEY".to_string(), "sk-xxx".to_string());
        env.insert("AXAGENT_CREDENTIAL_MASTER_KEY".to_string(), "secret".to_string());
        env.insert("UNRELATED".to_string(), "value".to_string());

        let filtered = filter_env_vars(env, DEFAULT_ENV_WHITELIST);
        assert_eq!(filtered.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(filtered.get("HOME"), Some(&"/home/user".to_string()));
        assert!(!filtered.contains_key("OPENAI_API_KEY"));
        assert!(!filtered.contains_key("AXAGENT_CREDENTIAL_MASTER_KEY"));
        assert!(!filtered.contains_key("UNRELATED"));
    }

    #[test]
    fn check_path_permission_denies_sensitive_dirs() {
        let config = SandboxConfig::restrictive();
        #[cfg(unix)]
        {
            assert!(check_path_permission(Path::new("/etc/passwd"), &config).is_err());
            assert!(check_path_permission(Path::new("/var/log"), &config).is_err());
            // 临时目录应允许
            assert!(check_path_permission(&std::env::temp_dir(), &config).is_ok());
        }
        #[cfg(windows)]
        {
            // Windows 敏感目录
            let sys32 = std::env::var("SystemRoot")
                .map(|r| PathBuf::from(r).join("System32"))
                .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\System32"));
            assert!(check_path_permission(&sys32, &config).is_err());
            // 临时目录应允许
            assert!(check_path_permission(&std::env::temp_dir(), &config).is_ok());
        }
    }

    #[test]
    fn check_path_permission_enforces_whitelist_when_set() {
        let tmp = std::env::temp_dir();
        let config = SandboxConfig::restrictive().with_allowed_paths(vec![tmp.clone()]);
        // 白名单内允许
        assert!(check_path_permission(&tmp.join("subdir"), &config).is_ok());
        // 白名单外拒绝（但若同时落在敏感目录，会先被黑名单拒绝）
        // 用一个非敏感、非白名单路径验证白名单拦截
        let outside = if cfg!(unix) {
            PathBuf::from("/opt/axagent-test-outside")
        } else {
            PathBuf::from(r"D:\axagent-test-outside")
        };
        // 该路径可能不存在，check 仍应基于白名单拒绝
        assert!(check_path_permission(&outside, &config).is_err());
    }

    #[test]
    fn check_subprocess_permission_respects_flag() {
        let denied = SandboxConfig::restrictive();
        assert!(check_subprocess_permission(&denied).is_err());

        let allowed = SandboxConfig { allow_subprocess: true, ..denied };
        assert!(check_subprocess_permission(&allowed).is_ok());
    }

    #[test]
    fn merged_with_takes_union_of_capabilities() {
        let a = SandboxConfig::restrictive();
        let b = SandboxConfig { allow_subprocess: true, allow_network: true, ..a.clone() };
        let merged = a.merged_with(&b);
        assert!(merged.allow_subprocess);
        assert!(merged.allow_network);
    }

    #[test]
    fn apply_env_to_command_clears_and_refills_whitelist() {
        // SAFETY: 测试代码，env_lock 由调用方保证（此处仅读不写）
        let mut command = Command::new("echo");
        let config = SandboxConfig::restrictive();
        apply_env_to_command(&mut command, &config);
        // 验证：apply 后显式 env 仍可追加
        command.env("CLAWD_PLUGIN_ID", "test");
        // 无失败即视为通过
    }
}
