use super::multi_turn::{CodeBlock, FileReference, FileType, ReferenceType, SkillFile};
use regex::Regex;
use std::path::Path;

pub struct SkillPackageParser;

impl SkillPackageParser {
    pub fn parse_files(files: Vec<(String, String)>) -> Vec<SkillFile> {
        files
            .into_iter()
            .map(|(path, content)| Self::parse_file(&path, &content))
            .collect()
    }

    pub fn parse_file(path: &str, content: &str) -> SkillFile {
        let file_type = Self::infer_file_type(path);
        let code_blocks = Self::extract_code_blocks(content);
        let references = Self::extract_references(path, content, &file_type);

        SkillFile {
            path: path.to_string(),
            file_type,
            content: content.to_string(),
            code_blocks,
            references,
        }
    }

    fn infer_file_type(path: &str) -> FileType {
        let path_obj = Path::new(path);
        if let Some(ext) = path_obj.extension() {
            FileType::from_extension(ext.to_str().unwrap_or(""))
        } else {
            FileType::Text
        }
    }

    pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        let mut block_counter = 0;

        while i < lines.len() {
            let line = lines[i];
            if line.trim().starts_with("```") {
                let language = line.trim().trim_start_matches("```").trim();
                let lang = if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                };

                let start_idx = i;
                let mut end_idx = start_idx + 1;
                while end_idx < lines.len() && !lines[end_idx].trim().starts_with("```") {
                    end_idx += 1;
                }

                let block_content: String = lines[start_idx + 1..end_idx.min(lines.len())]
                    .join("\n");

                let block_id = format!("cb_{}", block_counter);
                blocks.push(CodeBlock {
                    id: block_id,
                    language: lang,
                    content: block_content,
                    start_line: start_idx,
                    end_line: end_idx.min(lines.len()),
                });

                block_counter += 1;
                i = end_idx + 1;
                continue;
            }
            i += 1;
        }

        blocks
    }

    pub fn extract_references(path: &str, content: &str, file_type: &FileType) -> Vec<FileReference> {
        let mut refs = Vec::new();

        match file_type {
            FileType::Python => {
                refs.extend(Self::extract_python_imports(path, content));
            }
            FileType::JavaScript | FileType::TypeScript => {
                refs.extend(Self::extract_js_imports(path, content));
            }
            FileType::Markdown => {
                refs.extend(Self::extract_markdown_refs(path, content));
            }
            FileType::Json | FileType::Yaml => {
                refs.extend(Self::extract_config_refs(path, content));
            }
            _ => {}
        }

        refs
    }

    fn extract_python_imports(file_path: &str, content: &str) -> Vec<FileReference> {
        let mut refs = Vec::new();
        let import_re = Regex::new(r#"^\s*(?:import|from)\s+([a-zA-Z_][a-zA-Z0-9_\.]+)"#).unwrap();
        let from_re = Regex::new(r#"^\s*from\s+([a-zA-Z_][a-zA-Z0-9_\.]+)\s+import"#).unwrap();

        for line in content.lines() {
            if let Some(caps) = from_re.captures(line) {
                let module = caps.get(1).unwrap().as_str();
                if !module.starts_with('.') {
                    refs.push(FileReference {
                        from_file: file_path.to_string(),
                        to_file: module.to_string(),
                        to_function: None,
                        reference_type: ReferenceType::Import,
                    });
                }
            } else if let Some(caps) = import_re.captures(line) {
                let module = caps.get(1).unwrap().as_str();
                refs.push(FileReference {
                    from_file: file_path.to_string(),
                    to_file: module.to_string(),
                    to_function: None,
                    reference_type: ReferenceType::Import,
                });
            }
        }

        refs
    }

    fn extract_js_imports(file_path: &str, content: &str) -> Vec<FileReference> {
        let mut refs = Vec::new();
        let require_re = Regex::new(r#"require\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap();
        let import_re = Regex::new(r#"import\s+(?:(?:\{[^}]+\}|\w+)\s+from\s+)?["']([^"']+)["']"#).unwrap();

        for line in content.lines() {
            if let Some(caps) = require_re.captures(line) {
                let module = caps.get(1).unwrap().as_str();
                if !module.starts_with('.') && !module.starts_with("node:") {
                    refs.push(FileReference {
                        from_file: file_path.to_string(),
                        to_file: module.to_string(),
                        to_function: None,
                        reference_type: ReferenceType::Import,
                    });
                }
            }
            if let Some(caps) = import_re.captures(line) {
                let module = caps.get(1).unwrap().as_str();
                if !module.starts_with('.') && !module.starts_with("node:") {
                    refs.push(FileReference {
                        from_file: file_path.to_string(),
                        to_file: module.to_string(),
                        to_function: None,
                        reference_type: ReferenceType::Import,
                    });
                }
            }
        }

        refs
    }

    fn extract_markdown_refs(file_path: &str, content: &str) -> Vec<FileReference> {
        let mut refs = Vec::new();
        let file_link_re = Regex::new(r#"\[([^\]]+)\]\(([^\)]+)\)"#).unwrap();
        let code_ref_re = Regex::new(r#"```(\w+)\s+([^\s]+)"#).unwrap();

        for cap in file_link_re.captures_iter(content) {
            let link_text = cap.get(1).unwrap().as_str();
            let link_target = cap.get(2).unwrap().as_str();

            if !link_target.starts_with("http") && !link_target.starts_with('#') {
                refs.push(FileReference {
                    from_file: file_path.to_string(),
                    to_file: link_target.to_string(),
                    to_function: None,
                    reference_type: ReferenceType::Reference,
                });
            }

            if link_text.contains("function") || link_text.contains("method") {
                refs.push(FileReference {
                    from_file: file_path.to_string(),
                    to_file: link_target.to_string(),
                    to_function: Some(link_text.to_string()),
                    reference_type: ReferenceType::Calls,
                });
            }
        }

        for cap in code_ref_re.captures_iter(content) {
            let lang = cap.get(1).unwrap().as_str();
            let ref_name = cap.get(2).unwrap().as_str();

            if lang == "python" || lang == "javascript" || lang == "typescript" {
                refs.push(FileReference {
                    from_file: file_path.to_string(),
                    to_file: ref_name.to_string(),
                    to_function: None,
                    reference_type: ReferenceType::Reference,
                });
            }
        }

        refs
    }

    fn extract_config_refs(file_path: &str, content: &str) -> Vec<FileReference> {
        let mut refs = Vec::new();
        let ref_re = Regex::new(r#"\$\{(\w+)\}|ref:\s*([^\s]+)"#).unwrap();

        for cap in ref_re.captures_iter(content) {
            let reference = cap.get(1).or(cap.get(2)).map(|m| m.as_str());
            if let Some(reference) = reference {
                refs.push(FileReference {
                    from_file: file_path.to_string(),
                    to_file: reference.to_string(),
                    to_function: None,
                    reference_type: ReferenceType::Reference,
                });
            }
        }

        refs
    }

    pub fn build_file_graph(files: &[SkillFile]) -> FileGraph {
        let mut graph = FileGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        };

        for file in files {
            graph.nodes.push(FileNode {
                path: file.path.clone(),
                file_type: file.file_type.clone(),
                code_block_count: file.code_blocks.len(),
                reference_count: file.references.len(),
            });

            for r#ref in &file.references {
                graph.edges.push(FileEdge {
                    from: r#ref.from_file.clone(),
                    to: r#ref.to_file.clone(),
                    edge_type: format!("{:?}", r#ref.reference_type),
                });
            }
        }

        graph
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileGraph {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<FileEdge>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileNode {
    pub path: String,
    pub file_type: FileType,
    pub code_block_count: usize,
    pub reference_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}

pub fn infer_content_type(language: &Option<String>) -> &'static str {
    match language.as_deref().map(|s| s.to_lowercase()).as_deref() {
        Some("python") | Some("py") => "script",
        Some("javascript") | Some("js") => "script",
        Some("typescript") | Some("ts") => "script",
        Some("bash") | Some("sh") | Some("shell") => "script",
        Some("ruby") | Some("rb") => "script",
        Some("go") | Some("golang") => "script",
        Some("rust") | Some("rs") => "script",
        Some("yaml") | Some("yml") => "config",
        Some("json") => "config",
        Some("toml") | Some("ini") | Some("conf") => "config",
        Some("xml") => "config",
        Some("sql") => "query",
        Some("markdown") | Some("md") => "documentation",
        _ => "code_block",
    }
}
