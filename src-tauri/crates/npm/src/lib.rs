pub mod registry;
pub mod tarball;
pub mod types;

pub use registry::NpmRegistry;
pub use types::{DistInfo, NpmError, PackageInfo, VersionInfo};

use std::path::Path;

use async_trait::async_trait;
use axagent_harness::NpmRegistryService as HarnessNpmRegistryService;

#[async_trait]
impl HarnessNpmRegistryService for NpmRegistry {
    async fn download_package(
        &self,
        name: &str,
        version: Option<&str>,
        dest: &Path,
    ) -> Result<(), String> {
        let info = self
            .fetch_package_info(name)
            .await
            .map_err(|e| e.to_string())?;
        let ver = NpmRegistry::resolve_version(&info, version).map_err(|e| e.to_string())?;
        self.download_and_extract(&ver.dist, dest)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
