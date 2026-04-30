//! Three-level recall pipeline for precision code search.
//!
//! Orchestrates L1 (file metadata filter) → L2 (AST semantic match) → L3 (vector
//! similarity sort) in a configurable pipeline. Each level can be independently
//! enabled, and results are incrementally narrowed at each step.
//!
//! # Architecture
//!
//! The pipeline executes three stages sequentially:
//!
//! - L1 (File Index Filter): Filters by code extension, modification time,
//!   and path pattern, reducing the candidate set to 10-30% of total files.
//! - L2 (AST Semantic Match): Searches function/class/interface names and
//!   call edges, scoring by relevance to narrow to 1-5% of files.
//! - L3 (Vector Similarity Sort): Vector search only within L2 candidates,
//!   producing a hybrid score (AST * 0.4 + Vector * 0.6) and final top-K ranking.
//!

use serde::{Deserialize, Serialize};

use crate::ast_index::AstIndex;
use crate::file_index::FileIndex;

pub type VectorSearchFn = dyn Fn(&[&str], &str) -> Vec<(String, f32)>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub l1_enabled: bool,
    pub l2_enabled: bool,
    pub l3_enabled: bool,
    pub l1_code_extensions: Vec<String>,
    pub l2_limit: usize,
    pub l3_limit: usize,
    pub ast_weight: f32,
    pub vector_weight: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            l1_enabled: true,
            l2_enabled: true,
            l3_enabled: true,
            l1_code_extensions: crate::file_index::CODE_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            l2_limit: 50,
            l3_limit: 10,
            ast_weight: 0.4,
            vector_weight: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub file_path: String,
    pub ast_score: f32,
    pub vector_score: Option<f32>,
    pub combined_score: f32,
    pub matched_definitions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RecallPipeline<'a> {
    file_index: &'a FileIndex,
    ast_index: &'a AstIndex,
    config: PipelineConfig,
}

impl<'a> RecallPipeline<'a> {
    pub fn new(file_index: &'a FileIndex, ast_index: &'a AstIndex, config: PipelineConfig) -> Self {
        Self { file_index, ast_index, config }
    }

    /// Execute the full three-level recall for a query.
    ///
    /// If L3 is disabled, vector_score will be None and combined_score
    /// will equal ast_score alone.
    pub fn execute(
        &self,
        query: &str,
        vector_search_fn: Option<&VectorSearchFn>,
    ) -> Result<Vec<RecallResult>, String> {
        // ── L1: File metadata filter ────────────────────────────────────
        let l1_files: Vec<String> = if self.config.l1_enabled {
            let extensions: Vec<&str> = self.config.l1_code_extensions.iter().map(|s| s.as_str()).collect();
            let entries = self.file_index.filter_by_extension(&extensions)?;
            entries.into_iter().map(|e| e.path).collect()
        } else {
            Vec::new()
        };

        if l1_files.is_empty() && self.config.l1_enabled {
            return Ok(Vec::new());
        }

        // ── L2: AST semantic match ──────────────────────────────────────
        let ast_scored = if self.config.l2_enabled {
            self.score_ast_matches(query, &l1_files)?
        } else {
            l1_files.into_iter().map(|f| (f, 0.0f32, Vec::new())).collect()
        };

        // ── L3: Vector similarity sort ──────────────────────────────────
        let mut results: Vec<RecallResult> = if self.config.l3_enabled {
            if let Some(vs_fn) = vector_search_fn {
                let candidates: Vec<&str> = ast_scored.iter().map(|(f, _, _)| f.as_str()).collect();
                let vector_scores = vs_fn(&candidates, query);

                ast_scored
                    .into_iter()
                    .map(|(file, ast_score, defs)| {
                        let v_score = vector_scores
                            .iter()
                            .find(|(f, _)| f == &file)
                            .map(|(_, s)| *s);
                        let combined = if let Some(vs) = v_score {
                            self.config.ast_weight * ast_score + self.config.vector_weight * vs
                        } else {
                            ast_score
                        };
                        RecallResult {
                            file_path: file,
                            ast_score,
                            vector_score: v_score,
                            combined_score: combined,
                            matched_definitions: defs,
                        }
                    })
                    .collect()
            } else {
                ast_scored
                    .into_iter()
                    .map(|(file, ast_score, defs)| RecallResult {
                        file_path: file,
                        ast_score,
                        vector_score: None,
                        combined_score: ast_score,
                        matched_definitions: defs,
                    })
                    .collect()
            }
        } else {
            ast_scored
                .into_iter()
                .map(|(file, ast_score, defs)| RecallResult {
                    file_path: file,
                    ast_score,
                    vector_score: None,
                    combined_score: ast_score,
                    matched_definitions: defs,
                })
                .collect()
        };

        // Sort by combined score descending
        results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal));

        let limit = if self.config.l3_enabled { self.config.l3_limit } else { self.config.l2_limit };
        results.truncate(limit);

        Ok(results)
    }

    /// Score files based on AST match relevance to the query.
    fn score_ast_matches(
        &self,
        query: &str,
        l1_files: &[String],
    ) -> Result<Vec<(String, f32, Vec<String>)>, String> {
        let mut scored: Vec<(String, f32, Vec<String>)> = Vec::new();

        // Search functions
        if let Ok(fns) = self.ast_index.search_functions(query, self.config.l2_limit) {
            for f in &fns {
                if l1_files.is_empty() || l1_files.contains(&f.file_path) {
                    self.upsert_score(&mut scored, &f.file_path, 1.0, f.name.clone());
                }
            }
        }

        // Search classes
        if let Ok(cls) = self.ast_index.search_classes(query, self.config.l2_limit) {
            for c in &cls {
                if l1_files.is_empty() || l1_files.contains(&c.file_path) {
                    self.upsert_score(&mut scored, &c.file_path, 0.8, c.name.clone());
                }
            }
        }

        // Search all definitions in files
        if let Ok(paths) = self.ast_index.search_all(query, self.config.l2_limit) {
            for p in &paths {
                if l1_files.is_empty() || l1_files.contains(p) {
                    self.upsert_score(&mut scored, p, 0.5, query.to_string());
                }
            }
        }

        // Search call edges
        if let Ok(edges) = self.ast_index.find_callers(query) {
            for e in &edges {
                if l1_files.is_empty() || l1_files.contains(&e.caller_file) {
                    self.upsert_score(&mut scored, &e.caller_file, 0.6, format!("calls_{}", e.callee_name));
                }
            }
        }

        // Normalize scores to 0..1 range
        let max_score = scored.iter().map(|(_, s, _)| *s).fold(0.0f32, f32::max);
        if max_score > 0.0 {
            for (_, score, _) in &mut scored {
                *score /= max_score;
            }
        }

        Ok(scored)
    }

    fn upsert_score(&self, scored: &mut Vec<(String, f32, Vec<String>)>, file_path: &str, score: f32, def_name: String) {
        if let Some(existing) = scored.iter_mut().find(|(f, _, _)| f == file_path) {
            existing.1 += score;
            existing.2.push(def_name);
        } else {
            scored.push((file_path.to_string(), score, vec![def_name]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> (FileIndex, AstIndex) {
        let conn = Connection::open_in_memory().unwrap();
        let fi = FileIndex::new(conn).unwrap();
        let ai_conn = Connection::open_in_memory().unwrap();
        let ai = AstIndex::new(ai_conn).unwrap();
        (fi, ai)
    }

    #[test]
    fn test_pipeline_l1_l2_only() {
        let (fi, ai) = setup();
        fi.upsert("/src/main.rs", "rs", 1024, 1000).unwrap();
        fi.upsert("/src/lib.rs", "rs", 2048, 2000).unwrap();
        fi.upsert("/app.ts", "ts", 512, 500).unwrap();

        ai.index_file("/src/main.rs", "fn calculate() -> u32 { 42 }\nfn render() {}").unwrap();
        ai.index_file("/src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        ai.index_file("/app.ts", "function hello() { console.log('hi'); }").unwrap();

        let pipeline = RecallPipeline::new(
            &fi,
            &ai,
            PipelineConfig {
                l3_enabled: false,
                ..Default::default()
            },
        );

        let results = pipeline.execute("calculate", None).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.file_path.contains("main.rs")));
    }

    #[test]
    fn test_pipeline_empty_query() {
        let (fi, ai) = setup();
        fi.upsert("/src/main.rs", "rs", 1024, 1000).unwrap();
        ai.index_file("/src/main.rs", "fn main() {}").unwrap();

        let pipeline = RecallPipeline::new(&fi, &ai, PipelineConfig::default());
        let results = pipeline.execute("nonexistent_function_xyz", None).unwrap();
        assert!(results.is_empty());
    }
}
