// Tarball download and extraction — to be implemented in Task 3.

use std::path::{Path, PathBuf};

/// Placeholder — will be implemented in Task 3
pub fn extract_tarball(_data: &[u8], _dest: &Path) -> Result<(), std::io::Error> {
    Ok(())
}

/// Placeholder — will be implemented in Task 3
pub fn detect_package_root(_dest: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    Ok(None)
}
