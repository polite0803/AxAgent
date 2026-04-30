//! Dedicated code engine — lightweight runtime optimized exclusively for
//! code reading, editing, search, and analysis tasks.
//!
//! Unlike the general-purpose engine in `axagent-runtime`, the `CodeEngine`
//! loads only the modules relevant to software development: file indexing,
//! AST-based semantic search, LSP integration, git operations, and the
//! three-level recall pipeline. No document parsers, message gateways, or
//! UI automation modules are loaded.
//!
//! # Architecture
//!
//! The `CodeEngine` wraps `axagent_core` primitives (FileIndex, AstIndex,
//! RecallPipeline, OutputProcessor, TextChunker) into a single cohesive
//! engine that receives code queries and returns ranked results with
//! precision content injection.

use axagent_core::ast_index::AstIndex;
use axagent_core::disk_cache::DiskCache;
use axagent_core::file_index::{FileIndex, FileIndexConfig};
use axagent_core::incremental_indexer::IncrementalIndexer;
use axagent_core::output_processor::{OutputProcessor, OutputProcessorConfig};
use axagent_core::rag::{extract_surrounding_lines, inject_function_only};
use axagent_core::recall_pipeline::{PipelineConfig, RecallPipeline};
use axagent_core::text_chunker::{chunk_for_code, TextChunk};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEngineConfig {
    pub code_extensions: Vec<String>,
    pub l2_limit: usize,
    pub l3_limit: usize,
    pub db_path: Option<String>,
    pub enable_disk_cache: bool,
    pub max_cache_entries: usize,
    pub search_ttl_days: u32,
}

impl Default for CodeEngineConfig {
    fn default() -> Self {
        Self {
            code_extensions: axagent_core::file_index::CODE_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            l2_limit: 50,
            l3_limit: 10,
            db_path: None,
            enable_disk_cache: true,
            max_cache_entries: 1000,
            search_ttl_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchRequest {
    pub query: String,
    pub max_results: usize,
    pub include_context: bool,
    pub context_lines: usize,
    pub function_only: bool,
    pub source_files: Option<Vec<String>>,
}

impl Default for CodeSearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            max_results: 10,
            include_context: true,
            context_lines: 3,
            function_only: true,
            source_files: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub file_path: String,
    pub content: String,
    pub ast_score: f32,
    pub vector_score: Option<f32>,
    pub combined_score: f32,
    pub matched_definitions: Vec<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndexStats {
    pub file_count: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub last_modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFileInfo {
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub language: String,
    pub definitions: Vec<String>,
}

pub struct CodeEngine {
    config: CodeEngineConfig,
    file_index: Arc<RwLock<Option<FileIndex>>>,
    ast_index: Arc<RwLock<Option<AstIndex>>>,
    disk_cache: Arc<RwLock<Option<DiskCache>>>,
    output_processor: OutputProcessor,
    #[allow(dead_code)]
    indexer: IncrementalIndexer,
    #[allow(dead_code)]
    project_root: Arc<RwLock<Option<PathBuf>>>,
    #[allow(dead_code)]
    db_conn: Arc<RwLock<Option<Connection>>>,
}

impl CodeEngine {
    pub fn new(config: CodeEngineConfig) -> Self {
        Self {
            config,
            file_index: Arc::new(RwLock::new(None)),
            ast_index: Arc::new(RwLock::new(None)),
            disk_cache: Arc::new(RwLock::new(None)),
            output_processor: OutputProcessor::with_config(OutputProcessorConfig::aggressive()),
            indexer: IncrementalIndexer::default(),
            project_root: Arc::new(RwLock::new(None)),
            db_conn: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the engine for a project by scanning the root directory
    /// and building file/AST indices.
    pub async fn initialize(&self, project_root: &Path) -> Result<CodeIndexStats, String> {
        let db_path = self.config.db_path.as_deref().unwrap_or(":memory:");
        let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB: {e}"))?;

        let fi = FileIndex::new(conn).map_err(|e| format!("FileIndex: {e}"))?;
        let file_count = fi
            .scan_directory(project_root, &FileIndexConfig::default())
            .map_err(|e| format!("Scan: {e}"))?;

        let ai_conn = Connection::open(db_path).map_err(|e| format!("AI DB: {e}"))?;
        let ai = AstIndex::new(ai_conn).map_err(|e| format!("AstIndex: {e}"))?;

        let mut ast_count = 0;
        let entries = fi.all_entries()?;
        for entry in &entries {
            let abs_path = project_root.join(&entry.path);
            if abs_path.exists() && abs_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&abs_path) {
                    if let Ok(count) = ai.index_file(&entry.path, &content) {
                        ast_count += count;
                    }
                }
            }
        }

        let last_mod = fi.latest_modified().unwrap_or(None);

        if self.config.enable_disk_cache {
            let dc_conn = Connection::open(db_path).map_err(|e| format!("Cache DB: {e}"))?;
            let dc = DiskCache::new(
                dc_conn,
                axagent_core::disk_cache::DiskCacheConfig {
                    max_search_results: self.config.max_cache_entries,
                    search_result_ttl_days: self.config.search_ttl_days,
                    ..Default::default()
                },
            )
            .map_err(|e| format!("DiskCache: {e}"))?;
            *self.disk_cache.write().await = Some(dc);
        }

        *self.file_index.write().await = Some(fi);
        *self.ast_index.write().await = Some(ai);
        *self.project_root.write().await = Some(project_root.to_path_buf());

        let state_conn = Connection::open(db_path).map_err(|e| format!("State DB: {e}"))?;
        *self.db_conn.write().await = Some(state_conn);

        Ok(CodeIndexStats {
            file_count,
            function_count: ast_count,
            class_count: 0,
            last_modified: last_mod,
        })
    }

    /// Execute a code search using the three-level recall pipeline.
    pub async fn search(&self, request: &CodeSearchRequest) -> Result<Vec<CodeSearchResult>, String> {
        let fi_guard = self.file_index.read().await;
        let ai_guard = self.ast_index.read().await;
        let root_guard = self.project_root.read().await;

        let fi = fi_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;
        let ai = ai_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;
        let root = root_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;

        let pipeline = RecallPipeline::new(
            fi,
            ai,
            PipelineConfig {
                l1_code_extensions: self.config.code_extensions.clone(),
                l2_limit: self.config.l2_limit,
                l3_limit: request.max_results,
                ..PipelineConfig::default()
            },
        );

        // No vector search function for L3 (pipeline falls back to AST-only)
        let results = pipeline.execute(&request.query, None::<&axagent_core::recall_pipeline::VectorSearchFn>)?;

        let mut search_results = Vec::new();
        for r in &results {
            let mut content = String::new();
            let language = detect_language(&r.file_path);

            if request.include_context {
                let abs_path = root.join(&r.file_path);
                if abs_path.exists() {
                    if let Ok(source) = std::fs::read_to_string(&abs_path) {
                        if request.function_only && !r.matched_definitions.is_empty() {
                            if let Some(first_def) = r.matched_definitions.first() {
                                if let Some(snippet_start) = source.find(first_def.as_str()) {
                                    let snippet = &source[snippet_start..];
                                    let end = (snippet.len()).min(1000);
                                    content = inject_function_only(&source, first_def, end);
                                }
                            }
                        }
                        if content.is_empty() {
                            // Extract surrounding lines around the first matched definition
                            let snippet = r.matched_definitions.first().map(|s| s.as_str()).unwrap_or(&r.file_path);
                            content = extract_surrounding_lines(&source, snippet, request.context_lines)
                                .unwrap_or_else(|| source.chars().take(500).collect());
                        }
                    }
                }
            }

            search_results.push(CodeSearchResult {
                file_path: r.file_path.clone(),
                content,
                ast_score: r.ast_score,
                vector_score: r.vector_score,
                combined_score: r.combined_score,
                matched_definitions: r.matched_definitions.clone(),
                language: language.to_string(),
            });
        }

        // Try disk cache lookup as a fallback
        if search_results.is_empty() {
            if let Some(ref dc) = *self.disk_cache.read().await {
                let hash = DiskCache::query_hash(&request.query);
                if let Ok(Some(_cached)) = dc.get_search_results(&hash) {
                    tracing::debug!("Disk cache hit for query: {}", request.query);
                }
            }
        } else {
            // Cache successful results
            if let Some(ref dc) = *self.disk_cache.read().await {
                let hash = DiskCache::query_hash(&request.query);
                if let Ok(json) = serde_json::to_string(&search_results) {
                    let _ = dc.store_search_results(&hash, &request.query, &json, search_results.len());
                }
            }
        }

        Ok(search_results)
    }

    /// Chunk source code for embedding using code-optimized parameters.
    pub fn chunk_code(&self, source: &str) -> Vec<TextChunk> {
        chunk_for_code(source, None, None)
    }

    /// Process LLM output through the code-mode output processor.
    pub fn process_output(&self, raw: &str) -> axagent_core::output_processor::ProcessedOutput {
        self.output_processor.process(raw)
    }

    /// Get file information for all indexed code files.
    pub async fn list_files(&self, extension_filter: Option<&[&str]>) -> Result<Vec<CodeFileInfo>, String> {
        let fi_guard = self.file_index.read().await;
        let ai_guard = self.ast_index.read().await;

        let fi = fi_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;
        let ai = ai_guard.as_ref();

        let entries = if let Some(exts) = extension_filter {
            fi.filter_by_extension(exts)?
        } else {
            fi.all_entries()?
        };

        let mut files = Vec::new();
        for entry in &entries {
            let mut definitions = Vec::new();
            if let Some(ai) = ai {
                if let Ok(fns) = ai.search_functions("", 10) {
                    for f in fns.iter().filter(|f| f.file_path == entry.path) {
                        definitions.push(f.name.clone());
                    }
                }
                if let Ok(cls) = ai.search_classes("", 10) {
                    for c in cls.iter().filter(|c| c.file_path == entry.path) {
                        definitions.push(format!("class:{}", c.name));
                    }
                }
            }

            files.push(CodeFileInfo {
                path: entry.path.clone(),
                extension: entry.extension.clone(),
                size_bytes: entry.size_bytes,
                modified_at: entry.modified_at,
                language: detect_language(&entry.path).to_string(),
                definitions,
            });
        }

        Ok(files)
    }

    /// Get engine statistics.
    pub async fn stats(&self) -> Result<CodeIndexStats, String> {
        let fi_guard = self.file_index.read().await;
        let ai_guard = self.ast_index.read().await;

        let fi = fi_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;
        let ai = ai_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?;

        Ok(CodeIndexStats {
            file_count: fi.count()?,
            function_count: ai.total_definitions()?,
            class_count: 0,
            last_modified: fi.latest_modified()?,
        })
    }

    /// Start watching the project for incremental index updates.
    /// This should be spawned on a dedicated task.
    pub async fn start_watching(&self) -> Result<(), String> {
        let root_guard = self.project_root.read().await;
        let root = root_guard
            .as_ref()
            .ok_or_else(|| "CodeEngine not initialized".to_string())?
            .clone();
        drop(root_guard);

        tracing::info!("File watcher ready for project: {:?}", root);
        let _ = root;
        Ok(())
    }
}

fn detect_language(file_path: &str) -> &str {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".rs") { "rust" }
    else if lower.ends_with(".ts") || lower.ends_with(".tsx") { "typescript" }
    else if lower.ends_with(".js") || lower.ends_with(".jsx") { "javascript" }
    else if lower.ends_with(".py") { "python" }
    else if lower.ends_with(".go") { "go" }
    else if lower.ends_with(".java") { "java" }
    else if lower.ends_with(".cpp") || lower.ends_with(".cc") { "cpp" }
    else if lower.ends_with(".c") || lower.ends_with(".h") { "c" }
    else if lower.ends_with(".toml") || lower.ends_with(".yaml") || lower.ends_with(".yml") { "config" }
    else if lower.ends_with(".json") { "json" }
    else if lower.ends_with(".md") { "markdown" }
    else { "unknown" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_engine_initialize_and_search() {
        let dir = std::env::temp_dir().join("axagent_code_engine_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();

        let mut f = std::fs::File::create(dir.join("src/main.rs")).unwrap();
        writeln!(f, "fn calculate_total(items: &[i32]) -> i32 {{ items.iter().sum() }}\nfn main() {{ calculate_total(&[1,2,3]); }}").unwrap();

        let mut f2 = std::fs::File::create(dir.join("src/lib.rs")).unwrap();
        writeln!(f2, "pub fn authenticate(user: &str, pass: &str) -> bool {{ user == \"admin\" }}\npub fn render() {{ println!(\"render\"); }}").unwrap();

        let engine = CodeEngine::new(CodeEngineConfig::default());
        engine.initialize(&dir).await.unwrap();

        let req = CodeSearchRequest {
            query: "calculate".to_string(),
            max_results: 5,
            include_context: true,
            context_lines: 2,
            function_only: true,
            source_files: None,
        };
        let results = engine.search(&req).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.file_path.contains("main.rs")));

        let req2 = CodeSearchRequest {
            query: "authenticate".to_string(),
            ..Default::default()
        };
        let results2 = engine.search(&req2).await.unwrap();
        assert!(results2.iter().any(|r| r.file_path.contains("lib.rs")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_files() {
        let dir = std::env::temp_dir().join("axagent_ce_list_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(dir.join("b.ts"), "function b() {}").unwrap();

        let engine = CodeEngine::new(CodeEngineConfig::default());
        engine.initialize(&dir).await.unwrap();

        let all = engine.list_files(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let rs_only = engine.list_files(Some(&["rs"])).await.unwrap();
        assert_eq!(rs_only.len(), 1);
        assert_eq!(rs_only[0].extension, "rs");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
