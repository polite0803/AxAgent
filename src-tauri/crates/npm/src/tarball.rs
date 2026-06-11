// SPDX-License-Identifier: AGPL-3.0-only

//! npm 包 tarball (.tgz) 下载与解压

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

/// 将 .tgz 字节流解压到 dest 目录
pub fn extract_tarball(data: &[u8], dest: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dest)?;
    let cursor = Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

/// npm 包解压后通常有一层外层的 package/ 目录
/// 读取 dest 下的顶层目录：
/// - 如果只有 1 个目录，返回该目录路径
/// - 如果有多个或没有，返回 None (即 dest 本身就是根)
pub fn detect_package_root(dest: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    let entries: Vec<_> = fs::read_dir(dest)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    if entries.len() == 1 {
        let single_dir = entries
            .into_iter()
            .next()
            .expect("entries.len() == 1 was checked above")
            .path();
        if single_dir.join("plugin.json").exists()
            || single_dir.join(".claude-plugin").exists()
            || single_dir.join("SKILL.md").exists()
            || single_dir.join("package.json").exists()
        {
            return Ok(Some(single_dir));
        }
    }
    Ok(None)
}
