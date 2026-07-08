// SPDX-License-Identifier: AGPL-3.0-only

//! 共享路径工具函数
//!
//! `matches_glob` 和 `is_within_workspace` 在 file_read / file_write / classifier
//! 中曾经各自重复定义，现在统一在此模块中维护。

use std::path::{Component, Path};

/// 简易 glob 匹配：仅支持 `*`（匹配单个路径段）。
/// 模式与文本按 `/` 或 `\` 切段比较：每个段必须相等或为 `*`；
/// 模式可作为前缀匹配更深的路径（但不能跨段匹配，如 `/etc` 不应匹配 `/etcfoo`）。
/// 手动 split 而非依赖 Path::components()，确保 Linux CI 上也能正确识别 `\` 为分隔符。
pub fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern_trim = pattern.trim_end_matches(['/', '\\']);

    let p_segs: Vec<&str> = pattern_trim
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    let t_segs: Vec<&str> = text.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    if p_segs.len() > t_segs.len() {
        return false;
    }
    for (i, p_seg) in p_segs.iter().enumerate() {
        if *p_seg == "*" {
            continue;
        }
        if *p_seg != t_segs[i] {
            return false;
        }
    }
    true
}

/// 判断 `path` 是否在 `workspace` 内（按组件比较，已 normalize）。
pub fn is_within_workspace(path: &Path, workspace: &Path) -> bool {
    let p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let w = std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    path_components_starts_with(&p, &w)
}

fn path_components_starts_with(p: &Path, prefix: &Path) -> bool {
    let p_comp: Vec<Component> = p.components().collect();
    let w_comp: Vec<Component> = prefix.components().collect();
    if p_comp.len() < w_comp.len() {
        return false;
    }
    p_comp.iter().take(w_comp.len()).eq(w_comp.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_star() {
        assert!(matches_glob("/home/*/.ssh/", "/home/alice/.ssh/id_rsa"));
        assert!(matches_glob("/etc/shadow", "/etc/shadow"));
        assert!(!matches_glob("/etc/shadow", "/etc/passwd"));
        // 路径段必须严格匹配，/etc 不应误判 /etcfoo
        assert!(!matches_glob("/etc", "/etcfoo/x"));
    }

    #[test]
    fn glob_windows_paths() {
        assert!(matches_glob("C:\\Windows", "C:\\Windows\\System32\\drivers"));
        assert!(!matches_glob("C:\\Windows", "C:\\WindowsApps\\something"));
    }

    #[test]
    fn workspace_components_strict() {
        use std::path::PathBuf;
        let p = PathBuf::from("/workspace/inside.txt");
        let w = PathBuf::from("/workspace");
        assert!(is_within_workspace(&p, &w));
        let p2 = PathBuf::from("/workspace_evil/x");
        assert!(!is_within_workspace(&p2, &w));
    }
}
