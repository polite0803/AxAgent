use std::path::{Path, PathBuf};

use tracing::info;

use crate::tarball;
use crate::types::{DistInfo, NpmError, PackageInfo, VersionInfo};

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

pub struct NpmRegistry {
    registry_url: String,
    client: reqwest::Client,
}

impl NpmRegistry {
    pub fn new() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY.to_string(),
            client: reqwest::Client::builder()
                .user_agent("axagent-npm/0.1.0")
                .build()
                .expect("reqwest client build"),
        }
    }

    /// 解析包名: "@scope/name@version" → ("@scope/name", Option<"version">)
    /// 也支持无 scope: "plain-package@1.0.0"
    pub fn parse_package_spec(spec: &str) -> (&str, Option<&str>) {
        if let Some(at_pos) = spec.rfind('@') {
            if at_pos > 0 {
                let name = &spec[..at_pos];
                let version = &spec[at_pos + 1..];
                if !version.is_empty() && !version.contains('/') {
                    return (name, Some(version));
                }
            }
        }
        (spec, None)
    }

    /// 将 npm 包名转换为 registry URL path
    /// @scope/name → @scope%2Fname
    pub fn package_path(name: &str) -> String {
        name.replace('/', "%2F")
    }

    /// GET /<package> → PackageInfo
    pub async fn fetch_package_info(&self, name: &str) -> Result<PackageInfo, NpmError> {
        let path = Self::package_path(name);
        let url = format!("{}/{}", self.registry_url, path);
        info!("npm: fetching package info from {}", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(NpmError::NotFound(name.to_string()));
        }

        let info: PackageInfo = response.error_for_status()?.json().await?;
        Ok(info)
    }

    /// 解析版本 latest 或 semver
    pub fn resolve_version<'a>(
        info: &'a PackageInfo,
        version: Option<&str>,
    ) -> Result<&'a VersionInfo, NpmError> {
        let version_str = version.unwrap_or("latest");
        let semver = if version_str == "latest" {
            &info.dist_tags.latest
        } else {
            version_str
        };
        info.versions.get(semver).ok_or_else(|| {
            NpmError::VersionNotFound(info.name.clone(), semver.to_string())
        })
    }

    /// 下载 tarball 流式解压到 dest，返回插件根目录
    pub async fn download_and_extract(
        &self,
        dist: &DistInfo,
        dest: &Path,
    ) -> Result<PathBuf, NpmError> {
        info!("npm: downloading tarball from {}", dist.tarball);
        let response = self
            .client
            .get(&dist.tarball)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;
        tarball::extract_tarball(&bytes, dest)?;
        let root = tarball::detect_package_root(dest)?;
        Ok(root.unwrap_or_else(|| dest.to_path_buf()))
    }
}

impl Default for NpmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scoped_package_latest() {
        let (name, version) = NpmRegistry::parse_package_spec("@clawd/ths");
        assert_eq!(name, "@clawd/ths");
        assert_eq!(version, None);
    }

    #[test]
    fn parse_scoped_package_with_version() {
        let (name, version) = NpmRegistry::parse_package_spec("@clawd/stock@1.2.0");
        assert_eq!(name, "@clawd/stock");
        assert_eq!(version, Some("1.2.0"));
    }

    #[test]
    fn parse_plain_package_latest() {
        let (name, version) = NpmRegistry::parse_package_spec("my-plugin");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, None);
    }

    #[test]
    fn parse_plain_package_with_version() {
        let (name, version) = NpmRegistry::parse_package_spec("my-plugin@2.0.0");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, Some("2.0.0"));
    }

    #[test]
    fn parse_scoped_package_with_semver_tag() {
        let (name, version) = NpmRegistry::parse_package_spec("@scope/pkg@beta");
        assert_eq!(name, "@scope/pkg");
        assert_eq!(version, Some("beta"));
    }

    #[test]
    fn package_path_scoped() {
        assert_eq!(NpmRegistry::package_path("@clawd/ths"), "@clawd%2Fths");
    }

    #[test]
    fn package_path_plain() {
        assert_eq!(NpmRegistry::package_path("lodash"), "lodash");
    }
}
