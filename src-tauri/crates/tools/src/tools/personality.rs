use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

static PERSONALITIES_DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("personalities")
});

static ACTIVE_FILE: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("personalities")
        .join(".active")
});

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SoulFrontmatter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
}

fn parse_soul_md(name: &str, raw: &str) -> (SoulFrontmatter, String) {
    let trimmed = raw.trim_start();
    if trimmed.starts_with("---") {
        if let Some(end) = trimmed[3..].find("---") {
            let yaml_str = &trimmed[3..3 + end];
            let fm: SoulFrontmatter = serde_yaml::from_str(yaml_str).unwrap_or(SoulFrontmatter {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
            });
            let body = trimmed[3 + end + 3..].trim_start();
            return (fm, body.to_string());
        }
    }
    (
        SoulFrontmatter {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
        },
        raw.to_string(),
    )
}

fn serialize_soul_md(name: &str, version: &str, description: &str, content: &str) -> String {
    let fm = SoulFrontmatter {
        name: name.to_string(),
        version: version.to_string(),
        description: description.to_string(),
    };
    let yaml = serde_yaml::to_string(&fm).unwrap_or_default();
    format!("---\n{}---\n\n{}", yaml, content)
}

fn validate_personality_name(name: &str) -> Result<(), ToolError> {
    if name.is_empty() {
        return Err(ToolError::invalid_input("Personality name cannot be empty"));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(ToolError::invalid_input("Personality name contains invalid characters"));
    }
    if name.starts_with('.') {
        return Err(ToolError::invalid_input("Personality name cannot start with '.'"));
    }
    Ok(())
}

fn ensure_dir() -> Result<(), ToolError> {
    fs::create_dir_all(&*PERSONALITIES_DIR).map_err(|e| {
        ToolError::execution_failed(format!("Failed to create personalities directory: {}", e))
    })
}

fn list_names() -> Result<Vec<String>, ToolError> {
    ensure_dir()?;
    let mut names = Vec::new();
    let entries = fs::read_dir(&*PERSONALITIES_DIR)
        .map_err(|e| ToolError::execution_failed(format!("read_dir: {}", e)))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.join("SOUL.md").exists() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn load_personality(name: &str) -> Result<(SoulFrontmatter, String), ToolError> {
    validate_personality_name(name)?;
    let soul_path = PERSONALITIES_DIR.join(name).join("SOUL.md");
    if !soul_path.exists() {
        return Err(ToolError::execution_failed(format!("Personality '{}' not found", name)));
    }
    let raw = fs::read_to_string(&soul_path).map_err(|e| {
        ToolError::execution_failed(format!("Failed to read SOUL.md for '{}': {}", name, e))
    })?;
    Ok(parse_soul_md(name, &raw))
}

fn save_personality(
    name: &str,
    version: &str,
    description: &str,
    content: &str,
) -> Result<(), ToolError> {
    validate_personality_name(name)?;
    ensure_dir()?;
    let dir = PERSONALITIES_DIR.join(name);
    fs::create_dir_all(&dir)
        .map_err(|e| ToolError::execution_failed(format!("Failed to create directory: {}", e)))?;
    let soul_md = serialize_soul_md(name, version, description, content);
    fs::write(dir.join("SOUL.md"), soul_md)
        .map_err(|e| ToolError::execution_failed(format!("Failed to write SOUL.md: {}", e)))
}

fn delete_personality(name: &str) -> Result<(), ToolError> {
    validate_personality_name(name)?;
    let dir = PERSONALITIES_DIR.join(name);
    if !dir.exists() {
        return Err(ToolError::execution_failed(format!("Personality '{}' not found", name)));
    }
    fs::remove_dir_all(&dir).map_err(|e| {
        ToolError::execution_failed(format!("Failed to delete personality '{}': {}", name, e))
    })
}

fn get_active_name() -> Result<Option<String>, ToolError> {
    if !ACTIVE_FILE.exists() {
        return Ok(None);
    }
    let name = fs::read_to_string(&*ACTIVE_FILE).map_err(|e| {
        ToolError::execution_failed(format!("Failed to read active personality: {}", e))
    })?;
    let name = name.trim().to_string();
    if name.is_empty() {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

fn set_active(name: &str) -> Result<(), ToolError> {
    validate_personality_name(name)?;
    ensure_dir()?;
    let dir = PERSONALITIES_DIR.join(name);
    if !dir.exists() || !dir.join("SOUL.md").exists() {
        return Err(ToolError::execution_failed(format!(
            "Personality '{}' does not exist. Create it first.",
            name
        )));
    }
    fs::write(&*ACTIVE_FILE, name).map_err(|e| {
        ToolError::execution_failed(format!("Failed to write active personality: {}", e))
    })
}

pub struct PersonalityTool;

#[async_trait]
impl Tool for PersonalityTool {
    fn name(&self) -> &str {
        "Personality"
    }

    fn description(&self) -> &str {
        "管理 Agent 的人格/灵魂系统（SOUL.md）。\
         支持以下操作：\
         - list: 列出所有可用的人格\
         - current: 获取当前激活的人格\
         - switch: 切换到指定人格\
         - create: 创建新人格\
         - delete: 删除人格\
         人格内容会注入到系统提示词中，影响 Agent 的行为风格和偏好。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "current", "switch", "create", "delete"],
                    "description": "要执行的操作"
                },
                "name": {
                    "type": "string",
                    "description": "人格名称（switch/create/delete 时必需）"
                },
                "content": {
                    "type": "string",
                    "description": "SOUL.md 正文内容（create 时必需）"
                },
                "description": {
                    "type": "string",
                    "description": "人格描述（create 时可选）"
                },
                "version": {
                    "type": "string",
                    "description": "人格版本（create 时可选，默认 1.0.0）"
                }
            },
            "required": ["action"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("").to_lowercase();

        match action.as_str() {
            "list" => {
                let names = list_names()?;
                if names.is_empty() {
                    return Ok(ToolResult::success(
                        "没有已创建的人格。使用 action: \"create\" 创建新人格。",
                    ));
                }
                let active = get_active_name().ok().flatten();
                let mut out = String::from("## 可用人格列表\n\n");
                for name in &names {
                    let marker = if active.as_deref() == Some(name.as_str()) {
                        " ← 当前激活"
                    } else {
                        ""
                    };
                    let (fm, _) = load_personality(name).unwrap_or((
                        SoulFrontmatter {
                            name: name.clone(),
                            version: String::new(),
                            description: String::new(),
                        },
                        String::new(),
                    ));
                    out.push_str(&format!(
                        "- **{}** (v{}){}: {}\n",
                        name,
                        if fm.version.is_empty() {
                            "?"
                        } else {
                            &fm.version
                        },
                        marker,
                        if fm.description.is_empty() {
                            "(无描述)"
                        } else {
                            &fm.description
                        },
                    ));
                }
                out.push_str(&format!("\n共 {} 个人格。", names.len()));
                Ok(ToolResult {
                    content: out,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "personalities": names,
                        "active": active,
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "current" => {
                let active = get_active_name()?;
                match active {
                    Some(name) => {
                        let (fm, content) = load_personality(&name)?;
                        Ok(ToolResult {
                            content: format!(
                                "## 当前人格: {}\n\n**版本**: {}\n**描述**: {}\n\n---\n\n{}",
                                fm.name,
                                fm.version,
                                if fm.description.is_empty() {
                                    "(无)"
                                } else {
                                    &fm.description
                                },
                                content,
                            ),
                            is_error: false,
                            truncated: false,
                            metadata: Some(serde_json::json!({
                                "name": fm.name,
                                "version": fm.version,
                                "description": fm.description,
                                "content_length": content.len(),
                            })),
                            duration_ms: None,
                            progress: Vec::new(),
                        })
                    },
                    None => Ok(ToolResult::success(
                        "当前没有激活的人格。使用 action: \"switch\" 激活一个人格。",
                    )),
                }
            },
            "switch" => {
                let name = input["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err(ToolError::invalid_input("name 参数在 switch 操作中是必需的"));
                }
                set_active(&name)?;
                let (fm, _) = load_personality(&name)?;
                Ok(ToolResult {
                    content: format!(
                        "已切换到人格 '{}' (v{})。{}",
                        fm.name,
                        fm.version,
                        if fm.description.is_empty() {
                            String::new()
                        } else {
                            format!("描述: {}", fm.description)
                        },
                    ),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "active_personality": fm.name,
                        "version": fm.version,
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "create" => {
                let name = input["name"].as_str().unwrap_or("").to_string();
                let content = input["content"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err(ToolError::invalid_input("name 参数在 create 操作中是必需的"));
                }
                if content.is_empty() {
                    return Err(ToolError::invalid_input("content 参数在 create 操作中是必需的"));
                }
                let description = input["description"].as_str().unwrap_or("").to_string();
                let version = input["version"].as_str().unwrap_or("1.0.0").to_string();

                let existing = PERSONALITIES_DIR.join(&name).join("SOUL.md");
                if existing.exists() {
                    return Err(ToolError::execution_failed(format!(
                        "人格 '{}' 已存在。请先删除或使用不同名称。",
                        name
                    )));
                }

                save_personality(&name, &version, &description, &content)?;
                Ok(ToolResult {
                    content: format!(
                        "人格 '{}' (v{}) 已创建。使用 action: \"switch\" 将其激活。",
                        name, version
                    ),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "name": name,
                        "version": version,
                        "description": description,
                        "content_length": content.len(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "delete" => {
                let name = input["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err(ToolError::invalid_input("name 参数在 delete 操作中是必需的"));
                }
                let was_active = get_active_name().ok().flatten().is_some_and(|a| a == name);
                delete_personality(&name)?;
                if was_active {
                    let _ = fs::remove_file(&*ACTIVE_FILE);
                }
                Ok(ToolResult {
                    content: format!(
                        "人格 '{}' 已删除。{}",
                        name,
                        if was_active {
                            "该人格之前处于激活状态，已自动清除激活标记。"
                        } else {
                            ""
                        }
                    ),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "deleted": name,
                        "was_active": was_active,
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            _ => Err(ToolError::invalid_input(format!(
                "未知的 action: '{}'。支持: list, current, switch, create, delete",
                action
            ))),
        }
    }
}
