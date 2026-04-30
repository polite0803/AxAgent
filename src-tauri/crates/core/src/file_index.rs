//! Lightweight file directory index for pre-filtering during code search.
//!
//! Stores four metadata fields per file (path, extension, size, modification time)
//! in a SQLite table, enabling sub-millisecond filtering before more expensive
//! AST or vector operations.
//!
//! # Architecture
//!
//! - Index is rebuilt or updated via `scan_directory()`
//! - Queries use `filter_by_extension()`, `filter_by_modified_since()`, `filter_by_size_range()`
//! - Results are returned as `FileEntry` structs sorted by modification time descending

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_at: u64,
}

#[derive(Debug, Clone)]
pub struct FileIndexConfig {
    pub max_depth: usize,
    pub include_hidden: bool,
    pub exclude_patterns: Vec<String>,
}

impl Default for FileIndexConfig {
    fn default() -> Self {
        Self {
            max_depth: 32,
            include_hidden: false,
            exclude_patterns: vec![
                "target/".to_string(),
                "node_modules/".to_string(),
                ".git/".to_string(),
                "dist/".to_string(),
                "build/".to_string(),
                "__pycache__/".to_string(),
                ".venv/".to_string(),
                "vendor/".to_string(),
                ".next/".to_string(),
            ],
        }
    }
}

pub struct FileIndex {
    pub(crate) conn: Connection,
}

impl std::fmt::Debug for FileIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileIndex").finish_non_exhaustive()
    }
}

impl FileIndex {
    pub fn new(conn: Connection) -> Result<Self, String> {
        let index = Self { conn };
        index.ensure_table()?;
        Ok(index)
    }

    fn ensure_table(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS file_index (
                    path TEXT PRIMARY KEY,
                    extension TEXT NOT NULL DEFAULT '',
                    size_bytes INTEGER NOT NULL DEFAULT 0,
                    modified_at INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_file_ext ON file_index(extension);
                CREATE INDEX IF NOT EXISTS idx_file_modified ON file_index(modified_at);",
            )
            .map_err(|e| format!("Failed to create file_index table: {e}"))
    }

    /// Scan a directory recursively, storing metadata for all matching files.
    pub fn scan_directory(
        &self,
        root: &Path,
        config: &FileIndexConfig,
    ) -> Result<usize, String> {
        let mut count = 0;
        self.scan_recursive(root, root, config, 0, &mut count)?;

        self.conn
            .execute("DELETE FROM file_index WHERE path NOT LIKE ?1", params![format!("{}%", root.display())])
            .map_err(|e| format!("Failed to clean stale entries: {e}"))?;

        Ok(count)
    }

    fn scan_recursive(
        &self,
        root: &Path,
        current: &Path,
        config: &FileIndexConfig,
        depth: usize,
        count: &mut usize,
    ) -> Result<(), String> {
        if depth > config.max_depth {
            return Ok(());
        }

        let entries = std::fs::read_dir(current).map_err(|e| format!("read_dir {current:?}: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name.starts_with('.') && !config.include_hidden {
                continue;
            }

            if path.is_dir() {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let rel_str = rel.to_string_lossy();
                if config.exclude_patterns.iter().any(|p| rel_str.contains(p.as_str())) {
                    continue;
                }
                self.scan_recursive(root, &path, config, depth + 1, count)?;
            } else if path.is_file() {
                let metadata = std::fs::metadata(&path).map_err(|e| format!("metadata {path:?}: {e}"))?;
                let size = metadata.len();
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();

                let path_str = path.to_string_lossy().to_string();

                self.conn
                    .execute(
                        "INSERT OR REPLACE INTO file_index (path, extension, size_bytes, modified_at) VALUES (?1, ?2, ?3, ?4)",
                        params![path_str, ext, size, modified],
                    )
                    .map_err(|e| format!("insert {path_str}: {e}"))?;

                *count += 1;
            }
        }
        Ok(())
    }

    /// Filter entries by file extension (e.g. "rs", "ts", "py").
    pub fn filter_by_extension(&self, extensions: &[&str]) -> Result<Vec<FileEntry>, String> {
        if extensions.is_empty() {
            return self.all_entries();
        }
        let placeholders: Vec<String> = extensions.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!(
            "SELECT path, extension, size_bytes, modified_at FROM file_index WHERE extension IN ({}) ORDER BY modified_at DESC",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| format!("prepare: {e}"))?;
        let params: Vec<&dyn rusqlite::types::ToSql> = extensions.iter().map(|e| e as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(FileEntry {
                path: row.get(0)?,
                extension: row.get(1)?,
                size_bytes: row.get(2)?,
                modified_at: row.get(3)?,
            })
        }).map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Filter entries modified after the given Unix timestamp.
    pub fn filter_by_modified_since(&self, timestamp: u64) -> Result<Vec<FileEntry>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, extension, size_bytes, modified_at FROM file_index WHERE modified_at > ?1 ORDER BY modified_at DESC")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![timestamp], |row| {
                Ok(FileEntry {
                    path: row.get(0)?,
                    extension: row.get(1)?,
                    size_bytes: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Filter entries by file size range (inclusive).
    pub fn filter_by_size_range(&self, min_bytes: u64, max_bytes: u64) -> Result<Vec<FileEntry>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, extension, size_bytes, modified_at FROM file_index WHERE size_bytes BETWEEN ?1 AND ?2 ORDER BY modified_at DESC")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![min_bytes, max_bytes], |row| {
                Ok(FileEntry {
                    path: row.get(0)?,
                    extension: row.get(1)?,
                    size_bytes: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Search by partial path match (LIKE pattern).
    pub fn search_by_path(&self, pattern: &str) -> Result<Vec<FileEntry>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, extension, size_bytes, modified_at FROM file_index WHERE path LIKE ?1 ORDER BY modified_at DESC LIMIT 100")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![format!("%{pattern}%")], |row| {
                Ok(FileEntry {
                    path: row.get(0)?,
                    extension: row.get(1)?,
                    size_bytes: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Override or add a single file entry.
    pub fn upsert(&self, path: &str, extension: &str, size_bytes: u64, modified_at: u64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO file_index (path, extension, size_bytes, modified_at) VALUES (?1, ?2, ?3, ?4)",
                params![path, extension, size_bytes, modified_at],
            )
            .map_err(|e| format!("upsert {path}: {e}"))?;
        Ok(())
    }

    /// Remove a single file entry.
    pub fn remove(&self, path: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM file_index WHERE path = ?1", params![path])
            .map_err(|e| format!("remove {path}: {e}"))?;
        Ok(())
    }

    /// Remove all entries whose paths start with the given prefix.
    pub fn remove_by_prefix(&self, prefix: &str) -> Result<usize, String> {
        let count = self
            .conn
            .execute("DELETE FROM file_index WHERE path LIKE ?1", params![format!("{prefix}%")])
            .map_err(|e| format!("remove prefix {prefix}: {e}"))?;
        Ok(count)
    }

    /// Get the last modification timestamp in the index.
    pub fn latest_modified(&self) -> Result<Option<u64>, String> {
        self.conn
            .query_row(
                "SELECT MAX(modified_at) FROM file_index",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("latest_modified: {e}"))
    }

    /// Get total file count.
    pub fn count(&self) -> Result<usize, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get::<_, i64>(0))
            .map(|c| c as usize)
            .map_err(|e| format!("count: {e}"))
    }

    /// Return all entries.
    pub fn all_entries(&self) -> Result<Vec<FileEntry>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, extension, size_bytes, modified_at FROM file_index ORDER BY modified_at DESC LIMIT 5000")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(FileEntry {
                    path: row.get(0)?,
                    extension: row.get(1)?,
                    size_bytes: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }
}

/// Recommended source code extensions for filtering.
pub const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp",
    "swift", "kt", "scala", "rb", "php", "cs", "vue", "svelte", "sql",
    "toml", "yaml", "yml", "json", "md", "css", "html", "sh", "bash", "zsh",
    "proto", "graphql", "prisma", "tf", "dockerfile",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn test_index() -> FileIndex {
        let conn = Connection::open_in_memory().unwrap();
        FileIndex::new(conn).unwrap()
    }

    #[test]
    fn test_upsert_and_filter() {
        let idx = test_index();
        idx.upsert("/src/main.rs", "rs", 1024, 1000).unwrap();
        idx.upsert("/src/lib.rs", "rs", 2048, 2000).unwrap();
        idx.upsert("/app.ts", "ts", 512, 500).unwrap();

        let rs = idx.filter_by_extension(&["rs"]).unwrap();
        assert_eq!(rs.len(), 2);

        let ts = idx.filter_by_extension(&["ts"]).unwrap();
        assert_eq!(ts.len(), 1);

        let since = idx.filter_by_modified_since(1500).unwrap();
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].path, "/src/lib.rs");
    }

    #[test]
    fn test_count_and_remove() {
        let idx = test_index();
        idx.upsert("/a.rs", "rs", 100, 1).unwrap();
        idx.upsert("/b.rs", "rs", 200, 2).unwrap();
        assert_eq!(idx.count().unwrap(), 2);

        idx.remove("/a.rs").unwrap();
        assert_eq!(idx.count().unwrap(), 1);

        idx.remove_by_prefix("/").unwrap();
        assert_eq!(idx.count().unwrap(), 0);
    }
}
