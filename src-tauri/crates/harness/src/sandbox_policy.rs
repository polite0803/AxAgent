// SPDX-License-Identifier: AGPL-3.0-only

//! 沙箱策略 — 权威 DTO（PLAN-codex-parity P0-1a）
//!
//! 对标 codex 的 sandbox mode 三档语义：
//! - `ReadOnly`：只读，禁止一切写入与网络
//! - `WorkspaceWrite`：仅工作区可写，网络默认关闭
//! - `DangerFullAccess`：完全访问（与沙箱功能引入前的行为一致）
//!
//! 所有平台的沙箱实现（Windows restricted token / Linux unshare+seccomp /
//! macOS seatbelt）统一消费本类型。此前项目内 4 处同名 `SandboxConfig`
//! （runtime-core / plugins / tools / tools::bash）属历史分层，后续逐期
//! 收敛到本定义；本模块只新增、不删除旧定义，避免一次性大迁移破坏回归。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 沙箱模式（对标 codex sandbox mode）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    /// 只读：子进程对文件系统仅有读权限，禁止网络
    #[default]
    ReadOnly,
    /// 工作区可写：仅 [`SandboxPolicy::workspace_cwd`] 及其子路径可写，网络默认关闭
    WorkspaceWrite,
    /// 完全访问：不施加沙箱限制（行为与未启用沙箱一致）
    DangerFullAccess,
}

impl SandboxMode {
    /// 是否需要为该模式启动受限子进程
    #[must_use]
    pub fn requires_restriction(self) -> bool {
        matches!(self, Self::ReadOnly | Self::WorkspaceWrite)
    }

    /// 解析 settings 存储的模式字符串（kebab-case），未识别值回退 `DangerFullAccess`。
    #[must_use]
    pub fn from_mode_str(mode: &str) -> Self {
        match mode {
            "read-only" => Self::ReadOnly,
            "workspace-write" => Self::WorkspaceWrite,
            _ => Self::DangerFullAccess,
        }
    }
}

/// 沙箱策略
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    /// 工作区根目录（WorkspaceWrite 模式下的唯一可写路径；其余模式仅作 cwd 语义）
    pub workspace_cwd: PathBuf,
    /// 是否允许网络访问（ReadOnly / WorkspaceWrite 语义上默认 false）
    pub network_access: bool,
}

impl SandboxPolicy {
    #[must_use]
    pub fn read_only(workspace_cwd: impl Into<PathBuf>) -> Self {
        Self {
            mode: SandboxMode::ReadOnly,
            workspace_cwd: workspace_cwd.into(),
            network_access: false,
        }
    }

    #[must_use]
    pub fn workspace_write(workspace_cwd: impl Into<PathBuf>) -> Self {
        Self {
            mode: SandboxMode::WorkspaceWrite,
            workspace_cwd: workspace_cwd.into(),
            network_access: false,
        }
    }

    #[must_use]
    pub fn danger_full_access(workspace_cwd: impl Into<PathBuf>) -> Self {
        Self {
            mode: SandboxMode::DangerFullAccess,
            workspace_cwd: workspace_cwd.into(),
            network_access: true,
        }
    }

    /// 从 settings 的 `sandbox_mode` 字符串构造策略（PLAN-codex-parity P0-1c）。
    ///
    /// - `read-only` → [`SandboxPolicy::read_only`]
    /// - `workspace-write` → [`SandboxPolicy::workspace_write`]
    /// - 其他（含 `danger-full-access` / 未识别值）→ [`SandboxPolicy::danger_full_access`]
    #[must_use]
    pub fn from_mode_str(mode: &str, workspace_cwd: impl Into<PathBuf>) -> Self {
        match SandboxMode::from_mode_str(mode) {
            SandboxMode::ReadOnly => Self::read_only(workspace_cwd),
            SandboxMode::WorkspaceWrite => Self::workspace_write(workspace_cwd),
            SandboxMode::DangerFullAccess => Self::danger_full_access(workspace_cwd),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SandboxMode, SandboxPolicy};
    use std::path::Path;

    #[test]
    fn mode_serializes_kebab_case() {
        assert_eq!(serde_json::to_string(&SandboxMode::ReadOnly).unwrap(), "\"read-only\"");
        assert_eq!(
            serde_json::to_string(&SandboxMode::WorkspaceWrite).unwrap(),
            "\"workspace-write\""
        );
        assert_eq!(
            serde_json::to_string(&SandboxMode::DangerFullAccess).unwrap(),
            "\"danger-full-access\""
        );
        let back: SandboxMode = serde_json::from_str("\"workspace-write\"").unwrap();
        assert_eq!(back, SandboxMode::WorkspaceWrite);
    }

    #[test]
    fn policy_serializes_camel_case() {
        let policy = SandboxPolicy::workspace_write("D:/ws");
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("\"mode\":\"workspace-write\""), "{json}");
        assert!(json.contains("\"workspaceCwd\""), "{json}");
        assert!(json.contains("\"networkAccess\":false"), "{json}");
    }

    #[test]
    fn default_mode_is_read_only() {
        assert_eq!(SandboxMode::default(), SandboxMode::ReadOnly);
    }

    #[test]
    fn convenience_constructors_match_mode() {
        let p = Path::new("D:/ws");
        assert_eq!(SandboxPolicy::read_only(p).mode, SandboxMode::ReadOnly);
        assert_eq!(SandboxPolicy::workspace_write(p).mode, SandboxMode::WorkspaceWrite);
        assert_eq!(SandboxPolicy::danger_full_access(p).mode, SandboxMode::DangerFullAccess);
        assert!(SandboxPolicy::danger_full_access(p).network_access);
    }

    #[test]
    fn from_mode_str_maps_settings_values() {
        let p = Path::new("D:/ws");
        assert_eq!(SandboxPolicy::from_mode_str("read-only", p).mode, SandboxMode::ReadOnly);
        assert_eq!(
            SandboxPolicy::from_mode_str("workspace-write", p).mode,
            SandboxMode::WorkspaceWrite
        );
        // 默认档与未识别值都回退 DangerFullAccess（零回归语义）
        assert_eq!(
            SandboxPolicy::from_mode_str("danger-full-access", p).mode,
            SandboxMode::DangerFullAccess
        );
        assert_eq!(SandboxPolicy::from_mode_str("", p).mode, SandboxMode::DangerFullAccess);
        assert_eq!(SandboxPolicy::from_mode_str("garbage", p).mode, SandboxMode::DangerFullAccess);
        // workspace_cwd 透传
        assert_eq!(SandboxPolicy::from_mode_str("read-only", "D:/ws").workspace_cwd, p);
    }
}
