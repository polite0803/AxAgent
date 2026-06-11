// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

/// Represents the type of workspace storage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum WorkspaceType {
    #[default]
    Local,
    Cloud,
}

/// Unified workspace URI supporting local and cloud schemes.
///
/// Supported schemes:
/// - `local:///absolute/path/to/workspace`  →  local filesystem
/// - `s3://bucket/path/to/workspace`       →  S3-compatible storage
/// - `webdav://host/path/to/workspace`     →  WebDAV storage
/// - Plain path (backward compat)          →  local filesystem
#[derive(Debug, Clone)]
pub struct WorkspaceUri {
    pub scheme: String,
    /// For S3: bucket name; for WebDAV: host
    pub authority: String,
    /// Normalized path within the bucket/host
    pub path: String,
    /// Original string for display
    pub raw: String,
}

impl WorkspaceUri {
    pub fn parse(uri: &str) -> Result<Self, String> {
        // Detect scheme
        if let Some(pos) = uri.find("://") {
            let scheme = uri[..pos].to_lowercase();
            let rest = &uri[pos + 3..];

            match scheme.as_str() {
                "s3" | "webdav" | "cos" | "oss" | "obs" | "bos" => {
                    Self::parse_cloud(&scheme, rest, uri)
                },
                "local" => Self::parse_local(rest, uri),
                other => Err(format!("Unsupported workspace scheme: {}", other)),
            }
        } else {
            // Plain path → local
            Self::parse_local(uri, uri)
        }
    }

    fn parse_cloud(scheme: &str, rest: &str, raw: &str) -> Result<Self, String> {
        let (authority, path) = if let Some(slash) = rest.find('/') {
            let a = rest[..slash].to_string();
            let p = rest[slash..].to_string();
            (a, p)
        } else {
            (rest.to_string(), "/".to_string())
        };

        let path = normalize_cloud_path(&path);

        Ok(Self {
            scheme: scheme.to_string(),
            authority,
            path,
            raw: raw.to_string(),
        })
    }

    fn parse_local(path_str: &str, raw: &str) -> Result<Self, String> {
        let path = normalize_local_path(path_str);
        Ok(Self {
            scheme: "local".to_string(),
            authority: String::new(),
            path,
            raw: raw.to_string(),
        })
    }

    pub fn is_cloud(&self) -> bool {
        self.scheme != "local"
    }

    pub fn is_local(&self) -> bool {
        self.scheme == "local"
    }

    /// Generate a local cache path for this cloud workspace.
    pub fn cache_path(&self, cache_base: &Path) -> PathBuf {
        if self.is_local() {
            return PathBuf::from(&self.path);
        }
        // Hash the URI to create a unique cache directory
        let hash = format!("{:x}", md5::compute(&self.raw));
        let bucket_or_host = if self.authority.is_empty() {
            "unknown"
        } else {
            &self.authority
        };
        cache_base.join(format!("{}_{}", bucket_or_host, &hash[..8]))
    }

    /// Build an S3 key prefix for files in this workspace.
    pub fn s3_key_prefix(&self) -> String {
        self.path.trim_start_matches('/').to_string()
    }

    /// Get the local path for this workspace (only valid for local URIs).
    pub fn local_path(&self) -> Option<PathBuf> {
        if self.is_local() {
            Some(PathBuf::from(&self.path))
        } else {
            None
        }
    }
}

fn normalize_cloud_path(path: &str) -> String {
    let path = path.trim_start_matches('/');
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    }
}

fn normalize_local_path(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        ".".to_string()
    } else {
        path.to_string()
    }
}

/// S3-compatible provider aliases (normalised scheme).
pub fn normalise_cloud_scheme(scheme: &str) -> &str {
    match scheme {
        "cos" | "s3" => "s3",
        "oss" => "s3",
        "obs" => "s3",
        "bos" => "s3",
        _ => scheme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_s3_uri() {
        let uri = WorkspaceUri::parse("s3://my-bucket/projects/demo").unwrap();
        assert_eq!(uri.scheme, "s3");
        assert_eq!(uri.authority, "my-bucket");
        assert_eq!(uri.path, "projects/demo");
        assert!(uri.is_cloud());
    }

    #[test]
    fn parses_cos_uri() {
        let uri = WorkspaceUri::parse("cos://my-cos-bucket/data").unwrap();
        assert_eq!(uri.scheme, "cos");
        assert_eq!(uri.authority, "my-cos-bucket");
    }

    #[test]
    fn parses_local_uri() {
        let uri = WorkspaceUri::parse("local:///home/user/workspace").unwrap();
        assert_eq!(uri.scheme, "local");
        assert_eq!(uri.path, "/home/user/workspace");
        assert!(uri.is_local());
    }

    #[test]
    fn parses_plain_path_as_local() {
        let uri = WorkspaceUri::parse("/home/user/project").unwrap();
        assert_eq!(uri.scheme, "local");
        assert!(uri.is_local());
    }

    #[test]
    fn rejects_unknown_scheme() {
        let err = WorkspaceUri::parse("ftp://host/path").unwrap_err();
        assert!(err.contains("Unsupported"));
    }
}
