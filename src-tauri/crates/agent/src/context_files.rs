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
            .map(|f| format!("## Context: {} ({})\n\n{}\n", f.name, f.path.display(), f.content))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_file_format_equality() {
        assert_eq!(ContextFileFormat::AgentsMd, ContextFileFormat::AgentsMd);
        assert_ne!(ContextFileFormat::AgentsMd, ContextFileFormat::ClaudeMd);
        assert_ne!(ContextFileFormat::ClaudeMd, ContextFileFormat::AxAgentMemory);
    }

    #[test]
    fn test_context_file_format_serialization() {
        let format = ContextFileFormat::AgentsMd;
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ContextFileFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ContextFileFormat::AgentsMd);

        let format = ContextFileFormat::ClaudeMd;
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ContextFileFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ContextFileFormat::ClaudeMd);

        let format = ContextFileFormat::AxAgentMemory;
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ContextFileFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ContextFileFormat::AxAgentMemory);
    }

    #[test]
    fn test_context_file_serialization() {
        let cf = ContextFile {
            path: PathBuf::from("/project/AGENTS.md"),
            name: "AGENTS.md".to_string(),
            content: "test content".to_string(),
            format: ContextFileFormat::AgentsMd,
        };
        let json = serde_json::to_string(&cf).unwrap();
        let deserialized: ContextFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, PathBuf::from("/project/AGENTS.md"));
        assert_eq!(deserialized.name, "AGENTS.md");
        assert_eq!(deserialized.content, "test content");
        assert_eq!(deserialized.format, ContextFileFormat::AgentsMd);
    }

    #[test]
    fn test_context_file_result_serialization() {
        let result = ContextFileResult {
            files: vec![ContextFile {
                path: PathBuf::from("/project/CLAUDE.md"),
                name: "CLAUDE.md".to_string(),
                content: "claude content".to_string(),
                format: ContextFileFormat::ClaudeMd,
            }],
            combined_content: "combined".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ContextFileResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.files.len(), 1);
        assert_eq!(deserialized.combined_content, "combined");
    }

    #[tokio::test]
    async fn test_context_file_resolver_new() {
        let resolver = ContextFileResolver::new();
        assert!(resolver.cached().await.is_none());
    }

    #[tokio::test]
    async fn test_context_file_resolver_default() {
        let resolver = ContextFileResolver::default();
        assert!(resolver.cached().await.is_none());
    }

    #[tokio::test]
    async fn test_context_file_resolver_discover_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert!(result.files.is_empty());
        assert!(result.combined_content.is_empty());
    }

    #[tokio::test]
    async fn test_context_file_resolver_discover_with_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "agents content").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "AGENTS.md");
        assert_eq!(result.files[0].content, "agents content");
        assert_eq!(result.files[0].format, ContextFileFormat::AgentsMd);
    }

    #[tokio::test]
    async fn test_context_file_resolver_discover_with_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "claude content").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "CLAUDE.md");
        assert_eq!(result.files[0].format, ContextFileFormat::ClaudeMd);
    }

    #[tokio::test]
    async fn test_context_file_resolver_discover_with_axagent_memory() {
        let dir = tempfile::tempdir().unwrap();
        let axagent_dir = dir.path().join(".axagent");
        std::fs::create_dir_all(&axagent_dir).unwrap();
        std::fs::write(axagent_dir.join("memory.md"), "memory content").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, ".axagent/memory.md");
        assert_eq!(result.files[0].format, ContextFileFormat::AxAgentMemory);
    }

    #[tokio::test]
    async fn test_context_file_resolver_discover_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "agents").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "claude").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert_eq!(result.files.len(), 2);
    }

    #[tokio::test]
    async fn test_context_file_resolver_combined_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "agents content").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert!(result.combined_content.contains("agents content"));
        assert!(result.combined_content.contains("## Context:"));
    }

    #[tokio::test]
    async fn test_context_file_resolver_caching() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "cached content").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        let cached = resolver.cached().await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().files.len(), result.files.len());
    }

    #[tokio::test]
    async fn test_context_file_resolver_reload() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "original").unwrap();
        let resolver = ContextFileResolver::new();
        let result1 = resolver.discover(dir.path()).await;
        assert_eq!(result1.files[0].content, "original");
        std::fs::write(dir.path().join("AGENTS.md"), "updated").unwrap();
        let result2 = resolver.reload(dir.path()).await;
        assert_eq!(result2.files[0].content, "updated");
    }

    #[tokio::test]
    async fn test_context_file_resolver_skips_hidden_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let hidden_dir = dir.path().join(".hidden");
        std::fs::create_dir_all(&hidden_dir).unwrap();
        std::fs::write(hidden_dir.join("AGENTS.md"), "hidden").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_context_file_resolver_skips_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let nm_dir = dir.path().join("node_modules");
        std::fs::create_dir_all(&nm_dir).unwrap();
        std::fs::write(nm_dir.join("AGENTS.md"), "nm").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_context_file_resolver_skips_target_dir() {
        let dir = tempfile::tempdir().unwrap();
        let target_dir = dir.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("AGENTS.md"), "target").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_context_file_resolver_discovers_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let sub_dir = dir.path().join("src");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("AGENTS.md"), "subdir agents").unwrap();
        let resolver = ContextFileResolver::new();
        let result = resolver.discover(dir.path()).await;
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].content, "subdir agents");
    }
}
