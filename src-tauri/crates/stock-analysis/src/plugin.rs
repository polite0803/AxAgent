use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 用户自定义分析师
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomAnalyst {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    /// 分类: "analyst" | "debator" | "risk" | "manager"
    pub category: String,
    pub source_path: String,
}

/// 自定义分析师插件管理器
pub struct AnalystPluginManager {
    plugin_dir: PathBuf,
}

impl AnalystPluginManager {
    /// 创建插件管理器，`base_dir` 为 agency_experts/stock-analysis/
    pub fn new(base_dir: &str) -> Self {
        let plugin_dir = PathBuf::from(base_dir).join("custom");
        fs::create_dir_all(&plugin_dir).ok();
        Self { plugin_dir }
    }

    /// 扫描 custom/ 目录下的 .md 文件，发现所有自定义分析师
    pub fn discover_custom_analysts(&self) -> Vec<CustomAnalyst> {
        let mut analysts = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.plugin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(analyst) = Self::parse_custom_file(&path, &content) {
                            analysts.push(analyst);
                        }
                    }
                }
            }
        }
        analysts
    }

    /// 解析单个 Markdown 文件（YAML frontmatter + Markdown body）
    fn parse_custom_file(path: &std::path::Path, content: &str) -> Option<CustomAnalyst> {
        let (frontmatter, body) = if let Some(rest) = content.strip_prefix("---") {
            if let Some(end) = rest.find("\n---") {
                (rest[..end].to_string(), rest[end + 4..].trim().to_string())
            } else {
                (String::new(), content.to_string())
            }
        } else {
            (String::new(), content.to_string())
        };

        let mut name = String::new();
        let mut description = String::new();
        let mut category = String::from("analyst");
        let mut disabled = false;

        for line in frontmatter.lines() {
            let trimmed = line.trim();
            if let Some(v) = trimmed.strip_prefix("name:") {
                name = v.trim().to_string();
            } else if let Some(v) = trimmed.strip_prefix("description:") {
                description = v.trim().to_string();
            } else if let Some(v) = trimmed.strip_prefix("category:") {
                category = v.trim().to_string();
            } else if let Some(v) = trimmed.strip_prefix("disabled:") {
                disabled = v.trim().parse().unwrap_or(false);
            }
        }

        if name.is_empty() || disabled {
            return None;
        }

        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Some(CustomAnalyst {
            id,
            name,
            description,
            system_prompt: body,
            category,
            source_path: path.display().to_string(),
        })
    }

    /// 将自定义分析师的提示词合并到基础提示词映射中
    pub fn merge_prompts(
        base_prompts: &HashMap<String, String>,
        custom_analysts: &[CustomAnalyst],
    ) -> HashMap<String, String> {
        let mut merged = base_prompts.clone();
        for analyst in custom_analysts {
            merged.insert(analyst.id.clone(), analyst.system_prompt.clone());
        }
        merged
    }
}
