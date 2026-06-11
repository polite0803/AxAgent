// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: DistTags,
    pub versions: HashMap<String, VersionInfo>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DistTags {
    pub latest: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub dist: DistInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub shasum: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NpmError {
    #[error("package not found: {0}")]
    NotFound(String),
    #[error("version not found: {0}@{1}")]
    VersionNotFound(String, String),
    #[error("registry request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("tarball extraction failed: {0}")]
    ExtractFailed(#[from] std::io::Error),
}
