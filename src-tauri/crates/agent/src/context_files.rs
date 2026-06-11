// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

const CONTEXT_FILE_NAMES: &[&str] = &["AGENTS.md", "CLAUDE.md", ".axagent/memory.md"];

const FILE_REF_SIZE_LIMIT: usize = 100 * 1024;
const URL_REF_SIZE_LIMIT: usize = 50 * 1024;
const URL_REF_TIMEOUT_SECS: u64 = 30;

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
            if path.exists()
                && let Ok(content) = std::fs::read_to_string(&path)
            {
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

pub async fn resolve_references(content: &str, base_dir: &Path) -> String {
    let content = resolve_file_references(content, base_dir);
    let content = resolve_url_references(&content).await;
    let content = resolve_skill_references(&content);
    strip_conditional_sections(&content)
}

fn resolve_file_references(content: &str, base_dir: &Path) -> String {
    let re = regex::Regex::new(r"@file:([^\s]+)").unwrap();
    re.replace_all(content, |caps: &regex::Captures| {
        let ref_path = &caps[1];
        let full_path = base_dir.join(ref_path);
        match std::fs::read_to_string(&full_path) {
            Ok(file_content) => {
                if file_content.len() > FILE_REF_SIZE_LIMIT {
                    format!(
                        "[Error: file '{}' exceeds {}KB size limit]",
                        ref_path,
                        FILE_REF_SIZE_LIMIT / 1024
                    )
                } else {
                    file_content
                }
            },
            Err(e) => format!("[Error reading file '{}': {}]", ref_path, e),
        }
    })
    .to_string()
}

async fn resolve_url_references(content: &str) -> String {
    let re = regex::Regex::new(r"@url:(https?://[^\s]+)").unwrap();
    let mut result = content.to_string();

    let caps: Vec<_> = re.captures_iter(content).collect();
    for cap in caps {
        let url = &cap[1];
        let placeholder = format!("@url:{}", url);
        let replacement = match fetch_url_content(url).await {
            Ok(body) => body,
            Err(e) => format!("[Error fetching URL '{}': {}]", url, e),
        };
        result = result.replacen(&placeholder, &replacement, 1);
    }

    result
}

async fn fetch_url_content(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(URL_REF_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(url).send().await.map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;

    if body.len() > URL_REF_SIZE_LIMIT {
        Err(format!(
            "response exceeds {}KB size limit ({} bytes)",
            URL_REF_SIZE_LIMIT / 1024,
            body.len()
        ))
    } else {
        Ok(body)
    }
}

fn resolve_skill_references(content: &str) -> String {
    let re = regex::Regex::new(r"@skill:([a-zA-Z0-9_-]+)").unwrap();
    let dirs = axagent_core::skill_dirs::skill_dirs();

    re.replace_all(content, |caps: &regex::Captures| {
        let skill_name = &caps[1];
        for (_kind, dir) in &dirs {
            let skill_md = dir.join(skill_name).join("SKILL.md");
            if skill_md.exists()
                && let Ok(md_content) = std::fs::read_to_string(&skill_md)
            {
                let first_line = md_content.lines().next().unwrap_or(skill_name);
                return first_line.to_string();
            }
        }
        format!("[Skill '{}' not found]", skill_name)
    })
    .to_string()
}

fn strip_conditional_sections(content: &str) -> String {
    let re = regex::Regex::new(r"<!--\s*if:(\w+):(\w+)\s*-->([\s\S]*?)<!--\s*endif\s*-->").unwrap();

    re.replace_all(content, |caps: &regex::Captures| {
        let condition_type = &caps[1];
        let condition_value = &caps[2];
        let inner_content = &caps[3];

        if evaluate_condition(condition_type, condition_value) {
            inner_content.to_string()
        } else {
            String::new()
        }
    })
    .to_string()
}

fn evaluate_condition(condition_type: &str, condition_value: &str) -> bool {
    match condition_type {
        "platform" => std::env::consts::OS == condition_value,
        "toolset" => is_toolset_available(condition_value),
        "personality" => std::env::var("AXAGENT_PERSONALITY")
            .unwrap_or_default()
            .eq(condition_value),
        _ => false,
    }
}

fn is_toolset_available(toolset: &str) -> bool {
    matches!(
        toolset,
        "web" | "file" | "shell" | "git" | "network" | "system" | "browser" | "database"
    )
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

    #[test]
    fn test_resolve_file_references_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hello world").unwrap();
        let content = "prefix @file:hello.txt suffix";
        let result = resolve_file_references(content, dir.path());
        assert_eq!(result, "prefix hello world suffix");
    }

    #[test]
    fn test_resolve_file_references_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let content = "prefix @file:missing.txt suffix";
        let result = resolve_file_references(content, dir.path());
        assert!(result.contains("[Error reading file 'missing.txt'"));
        assert!(result.contains("prefix"));
        assert!(result.contains("suffix"));
    }

    #[test]
    fn test_resolve_file_references_nested_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("data.txt"), "nested data").unwrap();
        let content = "@file:sub/data.txt";
        let result = resolve_file_references(content, dir.path());
        assert_eq!(result, "nested data");
    }

    #[test]
    fn test_resolve_file_references_size_limit() {
        let dir = tempfile::tempdir().unwrap();
        let big_content = "x".repeat(FILE_REF_SIZE_LIMIT + 1);
        std::fs::write(dir.path().join("big.txt"), &big_content).unwrap();
        let content = "@file:big.txt";
        let result = resolve_file_references(content, dir.path());
        assert!(result.contains("exceeds 100KB size limit"));
    }

    #[test]
    fn test_resolve_file_references_multiple() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "AAA").unwrap();
        std::fs::write(dir.path().join("b.txt"), "BBB").unwrap();
        let content = "@file:a.txt and @file:b.txt";
        let result = resolve_file_references(content, dir.path());
        assert_eq!(result, "AAA and BBB");
    }

    #[test]
    fn test_resolve_skill_references_not_found() {
        let content = "@skill:nonexistent-skill-xyz";
        let result = resolve_skill_references(content);
        assert!(result.contains("[Skill 'nonexistent-skill-xyz' not found]"));
    }

    #[test]
    fn test_strip_conditional_sections_platform_match() {
        let current_os = std::env::consts::OS;
        let content =
            format!("before<!-- if:platform:{} -->matched<!-- endif -->after", current_os);
        let result = strip_conditional_sections(&content);
        assert_eq!(result, "beforematchedafter");
    }

    #[test]
    fn test_strip_conditional_sections_platform_no_match() {
        let content = "before<!-- if:platform:nonexistent -->should be removed<!-- endif -->after";
        let result = strip_conditional_sections(&content);
        assert_eq!(result, "beforeafter");
    }

    #[test]
    fn test_strip_conditional_sections_toolset_available() {
        let content = "before<!-- if:toolset:web -->web content<!-- endif -->after";
        let result = strip_conditional_sections(content);
        assert_eq!(result, "beforeweb contentafter");
    }

    #[test]
    fn test_strip_conditional_sections_toolset_unavailable() {
        let content = "before<!-- if:toolset:nonexistent -->removed<!-- endif -->after";
        let result = strip_conditional_sections(content);
        assert_eq!(result, "beforeafter");
    }

    #[test]
    fn test_strip_conditional_sections_personality_no_match() {
        let content = "before<!-- if:personality:creative -->creative stuff<!-- endif -->after";
        let result = strip_conditional_sections(content);
        assert_eq!(result, "beforeafter");
    }

    #[test]
    fn test_strip_conditional_sections_unknown_type() {
        let content = "before<!-- if:unknown:value -->stuff<!-- endif -->after";
        let result = strip_conditional_sections(content);
        assert_eq!(result, "beforeafter");
    }

    #[test]
    fn test_strip_conditional_sections_multiline() {
        let current_os = std::env::consts::OS;
        let content = format!(
            "header\n<!-- if:platform:{} -->\nline1\nline2\n<!-- endif -->\nfooter",
            current_os
        );
        let result = strip_conditional_sections(&content);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("header"));
        assert!(result.contains("footer"));
    }

    #[test]
    fn test_evaluate_condition_platform() {
        let current_os = std::env::consts::OS;
        assert!(evaluate_condition("platform", current_os));
        assert!(!evaluate_condition("platform", "definitely_not_an_os"));
    }

    #[test]
    fn test_evaluate_condition_toolset() {
        assert!(evaluate_condition("toolset", "web"));
        assert!(evaluate_condition("toolset", "file"));
        assert!(evaluate_condition("toolset", "shell"));
        assert!(!evaluate_condition("toolset", "nonexistent"));
    }

    #[test]
    fn test_evaluate_condition_unknown() {
        assert!(!evaluate_condition("unknown_type", "value"));
    }

    #[tokio::test]
    async fn test_resolve_references_file_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ref.txt"), "resolved content").unwrap();
        let content = "Hello @file:ref.txt world";
        let result = resolve_references(&content, dir.path()).await;
        assert_eq!(result, "Hello resolved content world");
    }

    #[tokio::test]
    async fn test_resolve_references_mixed_with_conditionals() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.md"), "data payload").unwrap();
        let content = format!(
            "@file:data.md <!-- if:platform:{} -->platform-specific<!-- endif -->",
            std::env::consts::OS
        );
        let result = resolve_references(&content, dir.path()).await;
        assert!(result.contains("data payload"));
        assert!(result.contains("platform-specific"));
    }
}
