#[cfg(not(target_os = "android"))]
use crate::ast_index::AstIndex;
#[cfg(not(target_os = "android"))]
use crate::file_index::{FileIndex, FileIndexConfig};
#[cfg(not(target_os = "android"))]
use notify::{Event, EventKind, RecursiveMode, Watcher};
#[cfg(not(target_os = "android"))]
use std::cell::RefCell;
#[cfg(not(target_os = "android"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "android"))]
use std::sync::mpsc;
#[cfg(not(target_os = "android"))]
use std::time::{Duration, Instant};

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub debounce_ms: u64,
    pub code_extensions: Vec<String>,
}

#[cfg(not(target_os = "android"))]
impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            code_extensions: crate::file_index::CODE_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

#[cfg(not(target_os = "android"))]
pub struct IncrementalIndexer {
    watch_config: WatchConfig,
    file_index_config: FileIndexConfig,
    last_event: RefCell<Instant>,
}

#[cfg(not(target_os = "android"))]
impl IncrementalIndexer {
    pub fn new(watch_config: WatchConfig, file_index_config: FileIndexConfig) -> Self {
        Self {
            watch_config,
            file_index_config,
            last_event: RefCell::new(Instant::now()),
        }
    }

    pub fn watch_and_index(
        &self,
        root: &Path,
        file_index: &FileIndex,
        ast_index: &AstIndex,
    ) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        let root = root.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory {:?}: {e}", root))?;

        tracing::info!("File watcher started for {:?}", root);

        for event in rx {
            self.handle_event(&event, &root, file_index, ast_index);
        }

        Ok(())
    }

    fn handle_event(
        &self,
        event: &Event,
        root: &Path,
        file_index: &FileIndex,
        ast_index: &AstIndex,
    ) {
        if self.should_skip(event, root) {
            return;
        }

        let now = Instant::now();
        if now.duration_since(*self.last_event.borrow())
            < Duration::from_millis(self.watch_config.debounce_ms)
        {
            return;
        }
        *self.last_event.borrow_mut() = now;

        let paths = &event.paths;
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in paths {
                    if let Err(e) = self.reindex_file(path, root, file_index, ast_index) {
                        tracing::warn!("Failed to reindex {:?}: {e}", path);
                    }
                }
            },
            EventKind::Remove(_) => {
                for path in paths {
                    if let Ok(rel) = path.strip_prefix(root) {
                        let rel_str = rel.to_string_lossy().to_string();
                        let _ = ast_index.remove_file(&rel_str);
                        let _ = file_index.remove(&rel_str);
                        tracing::debug!("Removed index entry for {:?}", rel);
                    }
                }
            },
            _ => {},
        }
    }

    fn should_skip(&self, event: &Event, root: &Path) -> bool {
        for path in &event.paths {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy();
                for pattern in &self.file_index_config.exclude_patterns {
                    if rel_str.contains(pattern.as_str()) {
                        return true;
                    }
                }
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if self.watch_config.code_extensions.iter().any(|e| e == ext) {
                        return false;
                    }
                }
                if path.extension().is_none() && path.is_file() {
                    return false;
                }
            }
        }
        true
    }

    fn reindex_file(
        &self,
        path: &PathBuf,
        root: &Path,
        file_index: &FileIndex,
        ast_index: &AstIndex,
    ) -> Result<(), String> {
        if !path.is_file() {
            return Ok(());
        }

        let metadata = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
        let size = metadata.len();
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("strip prefix: {e}"))?;
        let rel_str = rel.to_string_lossy().to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        file_index.upsert(&rel_str, &ext, size, modified)?;

        let content = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
        ast_index.index_file(&rel_str, &content)?;

        tracing::debug!("Reindexed {:?} ({} bytes)", rel, size);
        Ok(())
    }

    pub fn initial_scan_and_watch(
        &self,
        root: &Path,
        file_index: &FileIndex,
        ast_index: &AstIndex,
    ) -> Result<(), String> {
        let file_count = file_index.scan_directory(root, &self.file_index_config)?;
        tracing::info!("Initial file scan complete: {} files indexed", file_count);

        let mut ast_count = 0;
        let entries = file_index.all_entries()?;
        for entry in &entries {
            let abs_path = root.join(&entry.path);
            if abs_path.exists() && abs_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&abs_path) {
                    match ast_index.index_file(&entry.path, &content) {
                        Ok(count) => ast_count += count,
                        Err(e) => {
                            tracing::debug!("AST index skipped for {}: {e}", entry.path);
                        },
                    }
                }
            }
        }
        tracing::info!("Initial AST index complete: {} definitions", ast_count);

        self.watch_and_index(root, file_index, ast_index)
    }
}

#[cfg(not(target_os = "android"))]
impl Default for IncrementalIndexer {
    fn default() -> Self {
        Self::new(WatchConfig::default(), FileIndexConfig::default())
    }
}

#[cfg(not(target_os = "android"))]
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_reindex_file() {
        let dir = std::env::temp_dir().join("axagent_incremental_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let test_file = dir.join("test.rs");
        let mut f = fs::File::create(&test_file).unwrap();
        writeln!(f, "fn hello() {{ println!(\"hi\"); }}").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        let fi = FileIndex::new(conn).unwrap();
        let ai_conn = Connection::open_in_memory().unwrap();
        let ai = AstIndex::new(ai_conn).unwrap();

        let indexer = IncrementalIndexer::default();
        indexer.reindex_file(&test_file, &dir, &fi, &ai).unwrap();

        let results = ai.search_functions("hello", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");

        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(target_os = "android")]
use std::path::Path;

#[cfg(target_os = "android")]
#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub debounce_ms: u64,
    pub code_extensions: Vec<String>,
}

#[cfg(target_os = "android")]
impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            code_extensions: Vec::new(),
        }
    }
}

#[cfg(target_os = "android")]
pub struct IncrementalIndexer;

#[cfg(target_os = "android")]
impl IncrementalIndexer {
    pub fn new(
        _watch_config: WatchConfig,
        _file_index_config: crate::file_index::FileIndexConfig,
    ) -> Self {
        Self
    }

    pub fn watch_and_index(
        &self,
        _root: &Path,
        _file_index: &crate::file_index::FileIndex,
        _ast_index: &crate::ast_index::AstIndex,
    ) -> Result<(), String> {
        Err("Incremental indexing is not available on Android".to_string())
    }

    pub fn initial_scan_and_watch(
        &self,
        _root: &Path,
        _file_index: &crate::file_index::FileIndex,
        _ast_index: &crate::ast_index::AstIndex,
    ) -> Result<(), String> {
        Err("Incremental indexing is not available on Android".to_string())
    }
}

#[cfg(target_os = "android")]
impl Default for IncrementalIndexer {
    fn default() -> Self {
        Self
    }
}
