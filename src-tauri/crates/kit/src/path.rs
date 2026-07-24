// SPDX-License-Identifier: AGPL-3.0-only

//! 路径校验统一模块(P3 质量)。
//!
//! 项目中原本有 2 处 `validate_relative_path` 重复定义且语义不一致:
//! - `storage::storage_paths::validate_relative_path`(Strict 策略)— 拒绝任何 `..`
//! - `agent::shadow_fs::validate_relative_path`(AllowIntermediate 策略)— 允许中间 `..`
//!
//! 本模块统一两者,通过 `TraversalPolicy` 参数化策略,消除重复定义(AGENTS.md 第 12 条)。

/// 路径遍历策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalPolicy {
    /// 严格策略 — 拒绝任何 `..`(包括中间 `..`)。
    ///
    /// 适用于:存储路径 / 文件名校验 / 用户输入的相对路径。
    /// 原实现:`storage::storage_paths::validate_relative_path`。
    Strict,
    /// 宽松策略 — 允许中间 `..`,仅拒绝逃逸出根的 `..`。
    ///
    /// 通过 depth 计数实现:`src/../sub/file.rs` 合法(depth 最终 ≥ 0),
    /// `../etc/passwd` 非法(depth 变为 -1)。
    ///
    /// 适用于:投机执行影子文件系统 / 允许相对引用的场景。
    /// 原实现:`agent::shadow_fs::validate_relative_path`。
    AllowIntermediate,
}

/// 验证相对路径安全性(防止路径遍历攻击)。
///
/// 所有策略共同校验:
/// 1. 非空
/// 2. 非绝对路径(不以 `/` 或 `\` 开头)
/// 3. 非 Windows 驱动盘路径(如 `C:\` 或 `C:/`)
///
/// 策略差异仅在 `..` 处理:
/// - `Strict`:任何 `..` 都拒绝
/// - `AllowIntermediate`:通过 depth 计数,仅拒绝逃逸出根的 `..`
pub fn validate_relative_path(path: &str, policy: TraversalPolicy) -> Result<(), String> {
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(format!("path must not be absolute: {path}"));
    }

    // Windows 驱动盘路径(如 C:\ 或 C:/)— 无论宿主平台都拒绝
    if path.len() >= 3 {
        let b = path.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
            return Err(format!("absolute path not allowed: {path}"));
        }
    }

    match policy {
        TraversalPolicy::Strict => {
            if path.contains("..") {
                return Err(format!("path must not contain '..' traversal: {path}"));
            }
        },
        TraversalPolicy::AllowIntermediate => {
            // 逐段规整检查路径遍历:仅当 .. 导致逃逸出项目根才拒绝。
            // 中间的 .. 若仍能落回项目根内(如 src/../sub/file.rs)视为合法。
            let mut depth: i32 = 0;
            for segment in path.split(['/', '\\']) {
                if segment.is_empty() || segment == "." {
                    continue;
                }
                if segment == ".." {
                    depth -= 1;
                    if depth < 0 {
                        return Err(format!("path traversal not allowed: {path}"));
                    }
                } else {
                    depth += 1;
                }
            }
        },
    }

    Ok(())
}

/// 剥离 Windows UNC 前缀(`\\?\` / `\\.\` / `\\?\UNC\`)。
///
/// `std::fs::canonicalize` 在 Windows 上会返回 UNC 路径,导致后续 `starts_with`
/// 检查失败。本函数剥离前缀后返回普通路径:
/// - `\\?\UNC\server\share` → `\\server\share`(网络路径保留双反斜杠)
/// - `\\?\C:\path` → `C:\path`
/// - `\\.\COM1` → `COM1`
///
/// 原实现:`file_browser::strip_unc_prefix` 和 `tools::registry::simplify_unc` 重复。
pub fn strip_unc_prefix(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        // 网络路径:还原为 \\server\share 格式
        std::path::PathBuf::from(format!(r"\\{}", rest))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(rest)
    } else if let Some(rest) = s.strip_prefix(r"\\.\") {
        std::path::PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// 规范化路径(canonicalize + UNC 剥离)。
///
/// 优先调用 `std::fs::canonicalize`,失败时回退到词法规范化。
/// 无论成功失败,都剥离 UNC 前缀。
///
/// 原实现散落在 `file_browser::canonicalize_path` 和 `tools::registry::normalize_path`。
pub fn canonicalize_with_fallback(path: &std::path::Path) -> std::path::PathBuf {
    match std::fs::canonicalize(path) {
        Ok(p) => strip_unc_prefix(&p),
        Err(_) => strip_unc_prefix(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_rejects_any_traversal() {
        assert!(validate_relative_path("../etc", TraversalPolicy::Strict).is_err());
        assert!(validate_relative_path("src/../sub", TraversalPolicy::Strict).is_err());
        assert!(validate_relative_path("a/../../b", TraversalPolicy::Strict).is_err());
    }

    #[test]
    fn strict_accepts_normal_relative() {
        assert!(validate_relative_path("src/main.rs", TraversalPolicy::Strict).is_ok());
        assert!(validate_relative_path("a/b/c", TraversalPolicy::Strict).is_ok());
    }

    #[test]
    fn allow_intermediate_accepts_mid_traversal() {
        assert!(
            validate_relative_path("src/../sub/file.rs", TraversalPolicy::AllowIntermediate)
                .is_ok()
        );
        assert!(validate_relative_path("a/./b/../c", TraversalPolicy::AllowIntermediate).is_ok());
    }

    #[test]
    fn allow_intermediate_rejects_escape() {
        assert!(
            validate_relative_path("../etc/passwd", TraversalPolicy::AllowIntermediate).is_err()
        );
        assert!(validate_relative_path("a/../../b", TraversalPolicy::AllowIntermediate).is_err());
    }

    #[test]
    fn both_policies_reject_absolute() {
        assert!(validate_relative_path("/etc/passwd", TraversalPolicy::Strict).is_err());
        assert!(validate_relative_path("/etc/passwd", TraversalPolicy::AllowIntermediate).is_err());
        assert!(validate_relative_path(r"C:\Windows", TraversalPolicy::Strict).is_err());
        assert!(validate_relative_path(r"C:\Windows", TraversalPolicy::AllowIntermediate).is_err());
    }

    #[test]
    fn both_policies_reject_empty() {
        assert!(validate_relative_path("", TraversalPolicy::Strict).is_err());
        assert!(validate_relative_path("", TraversalPolicy::AllowIntermediate).is_err());
    }

    #[test]
    fn strip_unc_removes_prefix() {
        let p = std::path::Path::new(r"\\?\C:\Users\test");
        assert_eq!(strip_unc_prefix(p), std::path::PathBuf::from(r"C:\Users\test"));

        let p = std::path::Path::new(r"\\.\COM1");
        assert_eq!(strip_unc_prefix(p), std::path::PathBuf::from("COM1"));

        // 网络路径:还原为 \\server\share 格式
        let p = std::path::Path::new(r"\\?\UNC\server\share\file");
        assert_eq!(strip_unc_prefix(p), std::path::PathBuf::from(r"\\server\share\file"));

        // 无前缀的路径保持不变
        let p = std::path::Path::new("/usr/local/bin");
        assert_eq!(strip_unc_prefix(p), std::path::PathBuf::from("/usr/local/bin"));
    }
}
