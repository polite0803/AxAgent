/// Auto-trigger indexing pipeline after cloud workspace sync.
///
/// When a cloud workspace is synced, this module automatically triggers:
/// 1. FileIndex scan (file metadata)
/// 2. AST Index (code semantics)
use std::path::Path;
use tracing::{info, warn};

use axagent_core::ast_index::AstIndex;
use axagent_core::file_index::{FileIndex, FileIndexConfig, CODE_EXTENSIONS};

/// Create FileIndex and AstIndex and index the workspace, all within a blocking task.
///
/// This runs the entire indexing pipeline in a blocking thread to avoid
/// Send/Sync issues with rusqlite::Connection.
pub async fn trigger_post_sync_indexing_for_cloud_workspace(
    workspace_path: &Path,
) -> IndexingReport {
    if !workspace_path.exists() {
        warn!("Workspace path does not exist: {}", workspace_path.display());
        return IndexingReport {
            skipped: true,
            reason: Some("workspace not found".to_string()),
            ..Default::default()
        };
    }

    let workspace_path = workspace_path.to_path_buf();

    tokio::task::spawn_blocking(move || index_workspace_blocking(&workspace_path))
        .await
        .unwrap_or_else(|e| IndexingReport {
            skipped: true,
            file_index_error: Some(format!("Task panicked: {}", e)),
            ..Default::default()
        })
}

/// Run the full indexing pipeline in a blocking thread.
///
/// This creates in-memory SQLite connections for FileIndex and AstIndex,
/// scans the workspace, and indexes all files.
fn index_workspace_blocking(workspace_path: &Path) -> IndexingReport {
    let mut report = IndexingReport::default();

    // Create FileIndex
    let file_index_conn = match rusqlite::Connection::open_in_memory() {
        Ok(conn) => conn,
        Err(e) => {
            report.file_index_error = Some(format!("Failed to create FileIndex connection: {}", e));
            report.skipped = true;
            return report;
        },
    };

    let file_index = match FileIndex::new(file_index_conn) {
        Ok(fi) => fi,
        Err(e) => {
            report.file_index_error = Some(format!("Failed to create FileIndex: {}", e));
            report.skipped = true;
            return report;
        },
    };

    // Step 1: FileIndex scan
    let config = FileIndexConfig::default();
    match file_index.scan_directory(workspace_path, &config) {
        Ok(count) => {
            report.files_indexed = count;
            info!("FileIndex scan completed: {} files indexed", count);
        },
        Err(e) => {
            warn!("FileIndex scan failed: {}", e);
            report.file_index_error = Some(e);
        },
    }

    // Step 2: AST Index
    let ast_index_conn = match rusqlite::Connection::open_in_memory() {
        Ok(conn) => conn,
        Err(e) => {
            warn!("Failed to create AstIndex connection: {}", e);
            report.ast_skipped = true;
            return report;
        },
    };

    let ast_index = match AstIndex::new(ast_index_conn) {
        Ok(ai) => ai,
        Err(e) => {
            warn!("Failed to create AstIndex: {}", e);
            report.ast_skipped = true;
            return report;
        },
    };

    let ast_report = index_code_files_with_ast_sync(&ast_index, workspace_path);
    report.ast_nodes_indexed = ast_report.ast_nodes_indexed;
    if let Some(e) = ast_report.ast_index_error {
        report.ast_index_error = Some(e);
    }

    report
}

/// Index all code files synchronously.
fn index_code_files_with_ast_sync(ast_index: &AstIndex, workspace_path: &Path) -> IndexingReport {
    let mut report = IndexingReport::default();

    let code_files = match scan_code_files_sync(workspace_path) {
        Ok(files) => files,
        Err(e) => {
            report.ast_index_error = Some(e);
            return report;
        },
    };

    info!("Found {} code files to index", code_files.len());

    let mut total_nodes = 0;
    for (file_path, content) in code_files {
        match ast_index.index_file(&file_path, &content) {
            Ok(nodes) => {
                total_nodes += nodes;
            },
            Err(e) => {
                warn!("Failed to index {}: {}", file_path, e);
            },
        }
    }

    report.ast_nodes_indexed = total_nodes;
    report
}

/// Synchronously scan for code files and read their content.
fn scan_code_files_sync(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    let extensions: Vec<String> = CODE_EXTENSIONS.iter().map(|e| e.to_string()).collect();
    scan_code_files_recursive(dir, &extensions, 0, &mut files)?;
    Ok(files)
}

fn scan_code_files_recursive(
    dir: &Path,
    extensions: &[String],
    depth: usize,
    files: &mut Vec<(String, String)>,
) -> Result<(), String> {
    if depth > 32 {
        return Ok(());
    }

    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {}", e))?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with('.') {
            continue;
        }

        let skip_dirs = [
            "target",
            "node_modules",
            ".git",
            "dist",
            "build",
            "__pycache__",
            ".venv",
            "vendor",
            ".next",
        ];
        if path.is_dir() && skip_dirs.contains(&name) {
            continue;
        }

        if path.is_dir() {
            scan_code_files_recursive(&path, extensions, depth + 1, files)?;
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            if extensions.contains(&ext) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let path_str = path.to_string_lossy().to_string();
                    files.push((path_str, content));
                }
            }
        }
    }

    Ok(())
}

/// Report of indexing operations triggered after sync.
#[derive(Debug, Default, serde::Serialize)]
pub struct IndexingReport {
    pub files_indexed: usize,
    pub ast_nodes_indexed: usize,
    pub file_index_error: Option<String>,
    pub ast_index_error: Option<String>,
    pub ast_skipped: bool,
    pub skipped: bool,
    pub reason: Option<String>,
}
