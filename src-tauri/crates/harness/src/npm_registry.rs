//! NPM Registry 服务契约。
//!
//! 提供 npm 包安装能力（下载、解压）。
//! 实现方（`axagent-npm`）调用 npm registry 的 REST API。

use async_trait::async_trait;
use std::fmt;
use std::path::Path;

/// NPM Registry 服务契约
///
/// - `download_package`：从 npm registry 下载指定版本包到目标目录
#[async_trait]
pub trait NpmRegistryService: fmt::Debug + Send + Sync {
    /// 从 npm registry 下载指定版本包到目标目录
    async fn download_package(
        &self,
        name: &str,
        version: Option<&str>,
        dest: &Path,
    ) -> Result<(), String>;
}

/// 空实现 — 总是失败（npm 包安装不可用）
#[derive(Debug)]
pub struct NoopNpmRegistryService;

#[async_trait]
impl NpmRegistryService for NoopNpmRegistryService {
    async fn download_package(
        &self,
        _name: &str,
        _version: Option<&str>,
        _dest: &Path,
    ) -> Result<(), String> {
        Err("npm registry service is not configured".to_string())
    }
}

/// 解析 npm 包规范字符串，返回 (package_name, optional_version)。
/// 支持格式："pkg"、"pkg@1.0.0"、"@scope/pkg"、"@scope/pkg@1.0.0"
pub fn parse_npm_package_spec(spec: &str) -> (&str, Option<&str>) {
    if let Some(at_pos) = spec.rfind('@')
        && at_pos > 0
    {
        let name = &spec[..at_pos];
        let version = &spec[at_pos + 1..];
        if !version.is_empty() && !version.contains('/') {
            return (name, Some(version));
        }
    }
    (spec, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scoped_package_with_version() {
        assert_eq!(parse_npm_package_spec("@scope/pkg@1.0.0"), ("@scope/pkg", Some("1.0.0")));
    }

    #[test]
    fn parses_unscoped_package() {
        assert_eq!(parse_npm_package_spec("lodash"), ("lodash", None));
    }

    #[test]
    fn parses_package_with_version() {
        assert_eq!(parse_npm_package_spec("lodash@4.17.21"), ("lodash", Some("4.17.21")));
    }

    #[test]
    fn noop_always_errors() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let service = NoopNpmRegistryService;
        let result =
            rt.block_on(service.download_package("test", None, std::path::Path::new("/tmp")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
    }
}
