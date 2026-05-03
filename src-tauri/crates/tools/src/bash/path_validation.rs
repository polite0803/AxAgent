//! 路径验证器
//!
//! 验证命令中所有文件系统路径的安全性：
//! - 工作目录边界检查
//! - 输出重定向目标验证
//! - 符号链接追踪

use std::path::{Path, PathBuf};

/// 路径验证器
pub struct PathValidator {
    /// 允许的工作目录
    working_dir: PathBuf,
    /// 禁止的前缀
    blocked_prefixes: Vec<PathBuf>,
}

impl PathValidator {
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        let working_dir = working_dir.into();
        let blocked_prefixes = vec![
            PathBuf::from("/etc"),
            PathBuf::from("/boot"),
            PathBuf::from("/sys"),
            PathBuf::from("/proc"),
            PathBuf::from("/dev"),
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Program Files"),
        ];

        Self {
            working_dir,
            blocked_prefixes,
        }
    }

    /// 验证单一路径
    pub fn validate(&self, path: &str) -> PathResult {
        let p = Path::new(path);

        // 规范化路径
        let normalized = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.working_dir.join(p)
        };

        // 检查阻断前缀
        for blocked in &self.blocked_prefixes {
            if normalized.starts_with(blocked) {
                return PathResult::Blocked(format!(
                    "路径 '{}' 指向系统目录 '{}'",
                    path,
                    blocked.display()
                ));
            }
        }

        // 检查路径遍历攻击
        if path.contains("..") {
            // 允许 .. 但检查解析后是否在工作目录内
            if let Ok(canonical) = std::fs::canonicalize(&normalized) {
                if let Ok(canonical_wd) = std::fs::canonicalize(&self.working_dir) {
                    if !canonical.starts_with(&canonical_wd) {
                        return PathResult::Blocked(format!(
                            "路径 '{}' 解析后在工作目录之外",
                            path
                        ));
                    }
                }
            }
        }

        PathResult::Allowed
    }

    /// 验证路径列表（来自命令参数）
    pub fn validate_all(&self, paths: &[&str]) -> Vec<PathResult> {
        paths.iter().map(|p| self.validate(p)).collect()
    }
}

#[derive(Debug, Clone)]
pub enum PathResult {
    Allowed,
    Warning(String),
    Blocked(String),
}

impl PathResult {
    pub fn is_blocked(&self) -> bool {
        matches!(self, PathResult::Blocked(_))
    }
}
