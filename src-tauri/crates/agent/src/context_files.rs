use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

const CONTEXT_FILE_NAMES: &[&str] = &["AGENTS.md", "CLAUDE.md", ".axagent/memory.md"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: PathBuf,
    pub name: String,
    pub content: String,
    pub format: ContextFileFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextFileFormat {
    AgentsMd,
    ClaudeMd,
    AxAgentMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFileResult {
    pub files: Vec<ContextFile>,
    pub combined_content: String,
}

pub struct ContextFileResolver {
    cache: Arc<RwLock<Option<ContextFileResult>>>,
}

impl Default for ContextFileResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextFileResolver {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn discover(&self, project_root: &Path) -> ContextFileResult {
        let mut files = Vec::new();

        Self::discover_in_dir(project_root, &mut files);

        Self::discover_subdirs(project_root, &mut files);

        let combined_content = files
            .iter()
            .map(|f| {
                format!(
                    "## Context: {} ({})\n\n{}\n",
                    f.name,
                    f.path.display(),
                    f.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        let result = ContextFileResult {
            files,
            combined_content,
        };
        *self.cache.write().await = Some(result.clone());
        result
    }

    fn discover_in_dir(dir: &Path, files: &mut Vec<ContextFile>) {
        for &name in CONTEXT_FILE_NAMES {
            let path = if name == ".axagent/memory.md" {
                dir.join(".axagent").join("memory.md")
            } else {
                dir.join(name)
            };
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let format = match name {
                        "AGENTS.md" => ContextFileFormat::AgentsMd,
                        "CLAUDE.md" => ContextFileFormat::ClaudeMd,
                        _ => ContextFileFormat::AxAgentMemory,
                    };
                    files.push(ContextFile {
                        path: path.clone(),
                        name: name.to_string(),
                        content,
                        format,
                    });
                }
            }
        }
    }

    fn discover_subdirs(root: &Path, files: &mut Vec<ContextFile>) {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                    Self::discover_in_dir(&entry.path(), files);
                }
            }
        }
    }

    pub async fn reload(&self, project_root: &Path) -> ContextFileResult {
        *self.cache.write().await = None;
        self.discover(project_root).await
    }

    pub async fn cached(&self) -> Option<ContextFileResult> {
        self.cache.read().await.clone()
    }
}
