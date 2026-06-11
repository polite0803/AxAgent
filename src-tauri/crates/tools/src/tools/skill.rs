// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::secure_store::SecureStore;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

static SKILL_INDEX: LazyLock<Mutex<SkillIndex>> = LazyLock::new(|| Mutex::new(SkillIndex::new()));

static AVAILABLE_TOOLSETS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn set_available_toolsets(toolsets: HashSet<String>) {
    if let Ok(mut guard) = AVAILABLE_TOOLSETS.lock() {
        *guard = toolsets;
    }
    if let Ok(mut index) = SKILL_INDEX.lock() {
        index.invalidate();
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RequiredEnvVar {
    name: String,
    prompt: String,
    help: String,
    required_for: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkillConfigSetting {
    key: String,
    description: String,
    default: Option<String>,
    prompt: String,
    setting_type: String,
}

#[derive(Clone)]
struct SkillIndexEntry {
    name: String,
    description: String,
    category: String,
    version: String,
    platforms: Vec<String>,
    tags: Vec<String>,
    requires_toolsets: Vec<String>,
    fallback_for_toolsets: Vec<String>,
    skill_dir: PathBuf,
    source_kind: String,
    required_environment_variables: Vec<RequiredEnvVar>,
    config_settings: Vec<SkillConfigSetting>,
}

struct SkillIndex {
    entries: Vec<SkillIndexEntry>,
    built_at: Option<Instant>,
}

type SkillMetadata = (
    String,
    String,
    String,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<RequiredEnvVar>,
    Vec<SkillConfigSetting>,
);

impl SkillIndex {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            built_at: None,
        }
    }

    fn ensure_built(&mut self) {
        let needs_rebuild = self.built_at.is_none_or(|t| t.elapsed().as_secs() > 300);
        if needs_rebuild {
            self.rebuild();
        }
    }

    fn invalidate(&mut self) {
        self.built_at = None;
    }

    fn is_toolset_available(toolset_name: &str) -> bool {
        AVAILABLE_TOOLSETS
            .lock()
            .map(|guard| guard.contains(toolset_name))
            .unwrap_or(false)
    }

    fn should_include_entry(entry: &SkillIndexEntry) -> bool {
        if !entry.platforms.is_empty()
            && !entry.platforms.contains(&std::env::consts::OS.to_string())
        {
            return false;
        }

        if !entry.requires_toolsets.is_empty()
            && !entry
                .requires_toolsets
                .iter()
                .all(|t| Self::is_toolset_available(t))
        {
            return false;
        }

        if !entry.fallback_for_toolsets.is_empty()
            && entry
                .fallback_for_toolsets
                .iter()
                .any(|t| Self::is_toolset_available(t))
        {
            return false;
        }

        true
    }

    fn rebuild(&mut self) {
        let dirs = axagent_core::skill_dirs::skill_dirs();
        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (source_kind, dir) in &dirs {
            if let Ok(dir_entries) = std::fs::read_dir(dir) {
                for entry in dir_entries.filter_map(|e| e.ok()) {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if seen.contains(&name) {
                        continue;
                    }
                    seen.insert(name.clone());

                    let skill_dir = entry.path();
                    let (
                        description,
                        category,
                        version,
                        platforms,
                        tags,
                        requires_toolsets,
                        fallback_for_toolsets,
                        required_env_vars,
                        config_settings,
                    ) = Self::extract_metadata(&skill_dir);

                    let candidate = SkillIndexEntry {
                        name,
                        description,
                        category,
                        version,
                        platforms,
                        tags,
                        requires_toolsets,
                        fallback_for_toolsets,
                        skill_dir,
                        source_kind: source_kind.to_string(),
                        required_environment_variables: required_env_vars,
                        config_settings,
                    };

                    if Self::should_include_entry(&candidate) {
                        entries.push(candidate);
                    }
                }
            }
        }

        self.entries = entries;
        self.built_at = Some(Instant::now());
    }

    fn extract_metadata(skill_dir: &Path) -> SkillMetadata {
        let mut description = String::new();
        let mut category = "general".to_string();
        let mut version = "1.0.0".to_string();
        let mut platforms = Vec::new();
        let mut tags = Vec::new();
        let mut requires_toolsets = Vec::new();
        let mut fallback_for_toolsets = Vec::new();
        let mut required_env_vars = Vec::new();
        let mut config_settings = Vec::new();

        let skill_md = skill_dir.join("SKILL.md");
        if let Ok(content) = std::fs::read_to_string(&skill_md) {
            if let Some(frontmatter) = Self::parse_frontmatter(&content) {
                description = frontmatter
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                version = frontmatter
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1.0.0")
                    .to_string();
                if let Some(p) = frontmatter.get("platforms").and_then(|v| v.as_array()) {
                    platforms = p
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
                if let Some(meta) = frontmatter.get("metadata") {
                    if let Some(cat) = meta
                        .get("hermes")
                        .and_then(|h| h.get("category"))
                        .and_then(|c| c.as_str())
                    {
                        category = cat.to_string();
                    }
                    if let Some(t) = meta
                        .get("hermes")
                        .and_then(|h| h.get("tags"))
                        .and_then(|ts| ts.as_array())
                    {
                        tags = t
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(rt) = meta
                        .get("hermes")
                        .and_then(|h| h.get("requires_toolsets"))
                        .and_then(|ts| ts.as_array())
                    {
                        requires_toolsets = rt
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(ft) = meta
                        .get("hermes")
                        .and_then(|h| h.get("fallback_for_toolsets"))
                        .and_then(|ts| ts.as_array())
                    {
                        fallback_for_toolsets = ft
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(rt) = meta
                        .get("axagent")
                        .and_then(|a| a.get("requires_toolsets"))
                        .and_then(|ts| ts.as_array())
                        && requires_toolsets.is_empty()
                    {
                        requires_toolsets = rt
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(ft) = meta
                        .get("axagent")
                        .and_then(|a| a.get("fallback_for_toolsets"))
                        .and_then(|ts| ts.as_array())
                        && fallback_for_toolsets.is_empty()
                    {
                        fallback_for_toolsets = ft
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(env_vars) = meta
                        .get("axagent")
                        .and_then(|a| a.get("required_environment_variables"))
                        .and_then(|v| v.as_array())
                    {
                        required_env_vars = Self::parse_env_vars_from_json(env_vars);
                    }
                    if required_env_vars.is_empty()
                        && let Some(env_vars) = meta
                            .get("hermes")
                            .and_then(|h| h.get("required_environment_variables"))
                            .and_then(|v| v.as_array())
                    {
                        required_env_vars = Self::parse_env_vars_from_json(env_vars);
                    }
                    if let Some(cs) = meta
                        .get("axagent")
                        .and_then(|a| a.get("config"))
                        .and_then(|v| v.as_array())
                    {
                        config_settings = Self::parse_config_settings_from_json(cs);
                    }
                    if config_settings.is_empty()
                        && let Some(cs) = meta
                            .get("hermes")
                            .and_then(|h| h.get("config"))
                            .and_then(|v| v.as_array())
                    {
                        config_settings = Self::parse_config_settings_from_json(cs);
                    }
                }
                if let Some(env_vars) = frontmatter
                    .get("required_environment_variables")
                    .and_then(|v| v.as_array())
                    && required_env_vars.is_empty()
                {
                    required_env_vars = Self::parse_env_vars_from_json(env_vars);
                }
            }
            if description.is_empty() {
                description = content
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.starts_with("---") && !l.starts_with('#'))
                    .unwrap_or("")
                    .to_string();
            }
        }

        let manifest_path = skill_dir.join("skill-manifest.json");
        if let Ok(manifest_str) = std::fs::read_to_string(&manifest_path)
            && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_str)
        {
            if description.is_empty() {
                description = manifest["description"].as_str().unwrap_or("").to_string();
            }
            if let Some(p) = manifest["platforms"].as_array() {
                platforms = p
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(t) = manifest["tags"].as_array() {
                tags = t
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(rt) = manifest["requires_toolsets"].as_array()
                && requires_toolsets.is_empty()
            {
                requires_toolsets = rt
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(ft) = manifest["fallback_for_toolsets"].as_array()
                && fallback_for_toolsets.is_empty()
            {
                fallback_for_toolsets = ft
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(env_vars) = manifest["required_environment_variables"].as_array()
                && required_env_vars.is_empty()
            {
                required_env_vars = Self::parse_env_vars_from_json(env_vars);
            }
            if let Some(cs) = manifest["config_settings"].as_array()
                && config_settings.is_empty()
            {
                config_settings = Self::parse_config_settings_from_json(cs);
            }
        }

        (
            description,
            category,
            version,
            platforms,
            tags,
            requires_toolsets,
            fallback_for_toolsets,
            required_env_vars,
            config_settings,
        )
    }

    fn parse_env_vars_from_json(arr: &[serde_json::Value]) -> Vec<RequiredEnvVar> {
        arr.iter()
            .filter_map(|v| {
                let obj = v.as_object()?;
                Some(RequiredEnvVar {
                    name: obj.get("name")?.as_str()?.to_string(),
                    prompt: obj
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    help: obj
                        .get("help")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    required_for: obj
                        .get("required_for")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect()
    }

    fn parse_config_settings_from_json(arr: &[serde_json::Value]) -> Vec<SkillConfigSetting> {
        arr.iter()
            .filter_map(|v| {
                let obj = v.as_object()?;
                Some(SkillConfigSetting {
                    key: obj.get("key")?.as_str()?.to_string(),
                    description: obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    default: obj
                        .get("default")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    prompt: obj
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    setting_type: obj
                        .get("setting_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string")
                        .to_string(),
                })
            })
            .collect()
    }

    fn parse_frontmatter(content: &str) -> Option<serde_json::Value> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return None;
        }
        let end = trimmed[3..].find("---")?;
        let yaml_str = &trimmed[3..3 + end];
        serde_yaml::from_str(yaml_str).ok()
    }

    fn list_skills(&mut self, category_filter: Option<&str>) -> Vec<SkillSummary> {
        self.ensure_built();
        self.entries
            .iter()
            .filter(|e| category_filter.is_none_or(|cf| e.category == cf))
            .map(|e| SkillSummary {
                name: e.name.clone(),
                description: e.description.clone(),
                category: e.category.clone(),
                version: e.version.clone(),
            })
            .collect()
    }

    fn find_skill(&mut self, name: &str) -> Option<(String, PathBuf)> {
        self.ensure_built();
        self.entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| (e.source_kind.clone(), e.skill_dir.clone()))
    }

    fn find_skill_entry(&mut self, name: &str) -> Option<&SkillIndexEntry> {
        self.ensure_built();
        self.entries.iter().find(|e| e.name == name)
    }

    fn all_entries(&mut self) -> &[SkillIndexEntry] {
        self.ensure_built();
        &self.entries
    }

    fn list_reference_files(&mut self, skill_name: &str) -> Vec<String> {
        self.ensure_built();
        let Some(entry) = self.entries.iter().find(|e| e.name == skill_name) else {
            return Vec::new();
        };
        let refs_dir = entry.skill_dir.join("references");
        if !refs_dir.exists() {
            return Vec::new();
        }
        std::fs::read_dir(&refs_dir)
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkillSummary {
    name: String,
    description: String,
    category: String,
    version: String,
}

// ── SkillsList (Level 0) ──

pub struct SkillsListTool;

#[async_trait]
impl Tool for SkillsListTool {
    fn name(&self) -> &str {
        "SkillsList"
    }
    fn description(&self) -> &str {
        "列出所有已安装技能的摘要信息（Level 0 — 渐进式披露）。\
         返回每个技能的名称、描述、类别和版本，不加载完整内容以节省 token。\
         可选按类别过滤。确定要使用的技能后，用 SkillView 加载完整内容。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "按类别过滤（可选）"
                }
            }
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let category_filter = input["category"].as_str();
        let mut index = SKILL_INDEX
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;
        let skills = index.list_skills(category_filter);

        if skills.is_empty() {
            let msg = if let Some(cat) = category_filter {
                format!("类别 '{}' 下没有已安装的技能", cat)
            } else {
                "没有已安装的技能".to_string()
            };
            return Ok(ToolResult::success(msg));
        }

        let mut out = String::from("## 已安装技能列表\n\n");
        for s in &skills {
            out.push_str(&format!(
                "- **{}** (v{}): {} [{}]\n",
                s.name, s.version, s.description, s.category
            ));
        }
        out.push_str(&format!(
            "\n共 {} 个技能。使用 SkillView 加载完整内容，使用 SkillReference 查看引用文件。",
            skills.len()
        ));

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "level": 0,
                "total_skills": skills.len(),
                "category_filter": category_filter,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── SkillView (Level 1) ──

pub struct SkillViewTool;

#[async_trait]
impl Tool for SkillViewTool {
    fn name(&self) -> &str {
        "SkillView"
    }
    fn description(&self) -> &str {
        "加载指定技能的完整 SKILL.md 内容（Level 1 — 渐进式披露）。\
         返回技能的完整指令和元数据。如需查看引用文件，使用 SkillReference (Level 2)。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "要查看的技能名称"
                },
                "args": {
                    "type": "string",
                    "description": "传递给技能的参数（可选）"
                }
            },
            "required": ["skill"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn aliases(&self) -> &[&str] {
        &["Skill"]
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_name = input["skill"].as_str().unwrap_or("");
        let args = input["args"].as_str().unwrap_or("");

        if skill_name.is_empty() {
            return Err(ToolError::invalid_input("Skill name is required"));
        }

        let (source_kind, skill_dir) = {
            let mut index = SKILL_INDEX
                .lock()
                .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;

            let Some((source_kind, skill_dir)) = index.find_skill(skill_name) else {
                let available: Vec<String> = index
                    .list_skills(None)
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                return Err(ToolError::execution_failed(format!(
                    "Skill '{}' 未找到。可用的 skills: {}",
                    skill_name,
                    if available.is_empty() {
                        "(无)".to_string()
                    } else {
                        available.join(", ")
                    }
                )));
            };
            (source_kind, skill_dir)
        };

        let refs = {
            let mut index = SKILL_INDEX
                .lock()
                .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;
            index.list_reference_files(skill_name)
        };

        let entry = {
            let mut index = SKILL_INDEX
                .lock()
                .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;
            index.find_skill_entry(skill_name).cloned()
        };

        let skill_md = skill_dir.join("SKILL.md");
        let content = if skill_md.exists() {
            std::fs::read_to_string(&skill_md).map_err(|e| {
                ToolError::execution_failed(format!("Failed to read SKILL.md: {}", e))
            })?
        } else {
            let alt = skill_dir.join(format!("{}.md", skill_name));
            if alt.exists() {
                std::fs::read_to_string(&alt).map_err(|e| {
                    ToolError::execution_failed(format!("Failed to read skill file: {}", e))
                })?
            } else {
                return Err(ToolError::execution_failed(format!(
                    "Skill '{}' 目录中没有 SKILL.md 文件",
                    skill_name
                )));
            }
        };

        let mut output = format!(
            "# Skill: {}\n\n以下是从 SKILL.md 加载的技能指令。请严格按照这些指令执行任务，按需使用其他工具。\n\n---\n\n{}",
            skill_name, content
        );

        if !refs.is_empty() {
            output.push_str(&format!(
                "\n\n---\n**引用文件**: {}。使用 SkillReference 查看具体文件内容。",
                refs.join(", ")
            ));
        }

        if !args.is_empty() {
            output.push_str(&format!("\n\n---\n**用户参数**: {}", args));
            output.push_str("\n请将上述参数应用到技能指令中。");
        }

        if let Some(ref e) = entry {
            let missing_env_vars: Vec<&RequiredEnvVar> = e
                .required_environment_variables
                .iter()
                .filter(|v| !is_env_var_set(&v.name))
                .collect();

            if !missing_env_vars.is_empty() {
                output.push_str("\n\n---\n⚠️ **缺少必需的环境变量**：\n\n");
                for var in &missing_env_vars {
                    output.push_str(&format!(
                        "- **{}**: {}{}\n",
                        var.name,
                        var.prompt,
                        if var.help.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", var.help)
                        }
                    ));
                }
                output
                    .push_str("\n请使用 SkillEnvCheck action=set 设置这些环境变量后再使用此技能。");
            }

            if !e.config_settings.is_empty() {
                let config_values = get_all_skill_config_values(skill_name);
                let mut has_any_config = false;
                for setting in &e.config_settings {
                    let resolved = config_values
                        .get(&setting.key)
                        .cloned()
                        .or(setting.default.clone());
                    if let Some(val) = resolved {
                        if !has_any_config {
                            output.push_str("\n\n---\n**技能配置值**：\n\n");
                            has_any_config = true;
                        }
                        output.push_str(&format!("- {}: {}\n", setting.key, val));
                    }
                }
                if has_any_config {
                    output.push_str("\n以上配置值已注入技能上下文，在执行技能时可直接使用。");
                }
            }
        }

        Ok(ToolResult {
            content: output,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "skill_name": skill_name,
                "args": args,
                "level": 1,
                "source": "SKILL.md",
                "source_kind": source_kind,
                "source_dir": skill_dir.to_string_lossy().to_string(),
                "reference_files": refs,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── SkillReference (Level 2) ──

pub struct SkillReferenceTool;

#[async_trait]
impl Tool for SkillReferenceTool {
    fn name(&self) -> &str {
        "SkillReference"
    }
    fn description(&self) -> &str {
        "加载技能的特定引用文件内容（Level 2 — 渐进式披露最深层）。\
         用于需要深入参考资料的复杂场景。路径相对于技能目录下的 references/ 文件夹。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "技能名称"
                },
                "path": {
                    "type": "string",
                    "description": "引用文件路径（相对于 references/ 目录）"
                }
            },
            "required": ["skill", "path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_name = input["skill"].as_str().unwrap_or("");
        let path = input["path"].as_str().unwrap_or("");

        if skill_name.is_empty() || path.is_empty() {
            return Err(ToolError::invalid_input("Both skill name and path are required"));
        }

        let mut index = SKILL_INDEX
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;

        let Some((_source_kind, skill_dir)) = index.find_skill(skill_name) else {
            return Err(ToolError::execution_failed(format!("Skill '{}' 未找到", skill_name)));
        };

        let ref_path = skill_dir.join("references").join(path);

        let canonical_ref = ref_path
            .canonicalize()
            .map_err(|_| ToolError::execution_failed(format!("引用文件 '{}' 不存在", path)))?;
        let canonical_base = skill_dir
            .join("references")
            .canonicalize()
            .map_err(|_| ToolError::execution_failed("技能 references 目录不存在"))?;

        if !canonical_ref.starts_with(&canonical_base) {
            return Err(ToolError::execution_failed("路径越界：引用文件必须在 references/ 目录内"));
        }

        let content = std::fs::read_to_string(&canonical_ref)
            .map_err(|e| ToolError::execution_failed(format!("读取引用文件失败: {}", e)))?;

        Ok(ToolResult {
            content: format!(
                "# Skill Reference: {}/references/{}\n\n---\n\n{}",
                skill_name, path, content
            ),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "skill_name": skill_name,
                "reference_path": path,
                "level": 2,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── DiscoverSkills (enhanced) ──

pub struct DiscoverSkillsTool;

#[async_trait]
impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &str {
        "DiscoverSkills"
    }
    fn description(&self) -> &str {
        "通过名称/描述/标签关键词搜索已安装的 Skill（Level 0 增强）。\
         扫描技能索引，返回匹配的技能摘要。确定目标后用 SkillView 加载完整内容。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词"
                }
            },
            "required": ["query"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = input["query"].as_str().unwrap_or("").to_lowercase();
        if q.is_empty() {
            return Err(ToolError::invalid_input("Query is required"));
        }

        let mut index = SKILL_INDEX
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;
        index.ensure_built();

        let results: Vec<&SkillIndexEntry> = index
            .entries
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q)
                    || e.description.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || e.category.to_lowercase().contains(&q)
            })
            .collect();

        if results.is_empty() {
            return Ok(ToolResult::success(format!("未找到匹配 '{}' 的 Skill", q)));
        }

        let mut out = format!("## 技能搜索: '{}'\n\n", q);
        for e in &results {
            out.push_str(&format!(
                "- **{}** (v{}): {} [{}]\n",
                e.name, e.version, e.description, e.category
            ));
        }
        out.push_str(&format!(
            "\n共 {} 个匹配。使用 SkillView 加载完整内容，使用 SkillReference 查看引用文件。",
            results.len()
        ));

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "level": 0,
                "query": q,
                "total_matches": results.len(),
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── Skill Bundles ──

static BUNDLE_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".axagent")
        .join("skill-bundles")
});

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkillBundle {
    name: String,
    #[serde(default)]
    description: String,
    skills: Vec<String>,
    #[serde(default)]
    instruction: String,
}

impl SkillBundle {
    fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .replace(' ', "-")
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
    }

    fn file_path(&self) -> std::path::PathBuf {
        BUNDLE_DIR.join(format!("{}.yaml", self.slug()))
    }

    fn save(&self) -> Result<(), ToolError> {
        std::fs::create_dir_all(&*BUNDLE_DIR).map_err(|e| {
            ToolError::execution_failed(format!("Failed to create bundle directory: {}", e))
        })?;
        let yaml = serde_yaml::to_string(self).map_err(|e| {
            ToolError::execution_failed(format!("Failed to serialize bundle: {}", e))
        })?;
        std::fs::write(self.file_path(), yaml)
            .map_err(|e| ToolError::execution_failed(format!("Failed to write bundle file: {}", e)))
    }

    fn delete(&self) -> Result<(), ToolError> {
        let path = self.file_path();
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| ToolError::execution_failed(format!("Failed to delete bundle: {}", e)))
        } else {
            Ok(())
        }
    }
}

fn load_all_bundles() -> Vec<SkillBundle> {
    let dir = &*BUNDLE_DIR;
    if !dir.exists() {
        return Vec::new();
    }
    std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "yaml" || ext == "yml")
                })
                .filter_map(|e| {
                    let content = std::fs::read_to_string(e.path()).ok()?;
                    serde_yaml::from_str::<SkillBundle>(&content).ok()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn find_bundle(name: &str) -> Option<SkillBundle> {
    let slug = name.to_lowercase().replace(' ', "-");
    load_all_bundles()
        .into_iter()
        .find(|b| b.slug() == slug || b.name == name)
}

pub struct SkillBundleListTool;

#[async_trait]
impl Tool for SkillBundleListTool {
    fn name(&self) -> &str {
        "SkillBundleList"
    }
    fn description(&self) -> &str {
        "列出所有已安装的技能包（Skill Bundles）。技能包将多个技能组合为单一命令。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let bundles = load_all_bundles();
        if bundles.is_empty() {
            return Ok(ToolResult::success("没有已安装的技能包。使用 SkillBundleCreate 创建。"));
        }
        let mut out = String::from("## 已安装技能包\n\n");
        for b in &bundles {
            out.push_str(&format!(
                "- **/{}**: {} (技能: {})\n",
                b.slug(),
                if b.description.is_empty() {
                    "(无描述)"
                } else {
                    &b.description
                },
                b.skills.join(", ")
            ));
        }
        Ok(ToolResult::success(out))
    }
}

pub struct SkillBundleCreateTool;

#[async_trait]
impl Tool for SkillBundleCreateTool {
    fn name(&self) -> &str {
        "SkillBundleCreate"
    }
    fn description(&self) -> &str {
        "创建技能包，将多个技能组合为单一斜杠命令。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "技能包名称" },
                "description": { "type": "string", "description": "描述（可选）" },
                "skills": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "包含的技能名称列表"
                },
                "instruction": { "type": "string", "description": "附加指令（可选）" }
            },
            "required": ["name", "skills"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input["name"].as_str().unwrap_or("").to_string();
        let skills: Vec<String> = input["skills"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        if name.is_empty() || skills.is_empty() {
            return Err(ToolError::invalid_input("name and skills are required"));
        }
        let bundle = SkillBundle {
            name,
            description: input["description"].as_str().unwrap_or("").to_string(),
            skills,
            instruction: input["instruction"].as_str().unwrap_or("").to_string(),
        };
        bundle.save()?;
        Ok(ToolResult::success(format!(
            "技能包 '{}' 已创建，包含 {} 个技能。使用 /{} 加载。",
            bundle.name,
            bundle.skills.len(),
            bundle.slug()
        )))
    }
}

pub struct SkillBundleLoadTool;

#[async_trait]
impl Tool for SkillBundleLoadTool {
    fn name(&self) -> &str {
        "SkillBundleLoad"
    }
    fn description(&self) -> &str {
        "加载技能包中的所有技能内容。包名冲突时优先于同名单个技能。缺失的技能自动跳过。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bundle": { "type": "string", "description": "技能包名称" },
                "args": { "type": "string", "description": "传递给所有技能的参数（可选）" }
            },
            "required": ["bundle"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let bundle_name = input["bundle"].as_str().unwrap_or("");
        let args = input["args"].as_str().unwrap_or("");
        if bundle_name.is_empty() {
            return Err(ToolError::invalid_input("Bundle name is required"));
        }

        let Some(bundle) = find_bundle(bundle_name) else {
            return Err(ToolError::execution_failed(format!(
                "技能包 '{}' 未找到。使用 SkillBundleList 查看可用技能包。",
                bundle_name
            )));
        };

        let mut index = SKILL_INDEX
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;

        let mut loaded = Vec::new();
        let mut skipped = Vec::new();

        for skill_name in &bundle.skills {
            let Some((_source_kind, skill_dir)) = index.find_skill(skill_name) else {
                skipped.push(skill_name.clone());
                continue;
            };
            let skill_md = skill_dir.join("SKILL.md");
            if let Ok(content) = std::fs::read_to_string(skill_md) {
                loaded.push((skill_name.clone(), content));
            } else {
                skipped.push(skill_name.clone());
            }
        }

        let mut output = format!("# Skill Bundle: {}\n\n", bundle.name);
        if !bundle.description.is_empty() {
            output.push_str(&format!("**描述**: {}\n\n", bundle.description));
        }
        if !bundle.instruction.is_empty() {
            output.push_str(&format!("**指令**: {}\n\n---\n\n", bundle.instruction));
        }

        for (name, content) in &loaded {
            output.push_str(&format!("## Skill: {}\n\n{}\n\n---\n\n", name, content));
        }

        if !skipped.is_empty() {
            output.push_str(&format!("**跳过的技能（未安装）**: {}\n", skipped.join(", ")));
        }

        if !args.is_empty() {
            output.push_str(&format!("\n**用户参数**: {}\n", args));
        }

        Ok(ToolResult {
            content: output,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "bundle_name": bundle.name,
                "loaded": loaded.len(),
                "skipped": skipped,
                "args": args,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

pub struct SkillBundleDeleteTool;

#[async_trait]
impl Tool for SkillBundleDeleteTool {
    fn name(&self) -> &str {
        "SkillBundleDelete"
    }
    fn description(&self) -> &str {
        "删除指定的技能包。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bundle": { "type": "string", "description": "技能包名称" }
            },
            "required": ["bundle"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let bundle_name = input["bundle"].as_str().unwrap_or("");
        if bundle_name.is_empty() {
            return Err(ToolError::invalid_input("Bundle name is required"));
        }
        let Some(bundle) = find_bundle(bundle_name) else {
            return Err(ToolError::execution_failed(format!("技能包 '{}' 未找到", bundle_name)));
        };
        bundle.delete()?;
        Ok(ToolResult::success(format!("技能包 '{}' 已删除", bundle_name)))
    }
}

// ── Skills Hub (agentskills.io) ──

static HUB_BASE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("skills")
        .join(".hub")
});

static HUB_SKILLS_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("skills")
});

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HubSkill {
    id: String,
    name: String,
    description: String,
    category: String,
    author: String,
    version: String,
    tags: Vec<String>,
    downloads: u64,
    rating: f64,
    readme_url: Option<String>,
    manifest_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HubSearchResult {
    skills: Vec<HubSkill>,
    total: usize,
    page: usize,
    page_size: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HubLockEntry {
    name: String,
    id: String,
    version: String,
    installed_at: String,
    source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct HubLockFile {
    entries: Vec<HubLockEntry>,
}

fn hub_api_url() -> String {
    std::env::var("AGENTSKILLS_API_URL")
        .unwrap_or_else(|_| "https://api.agentskills.io".to_string())
}

fn hub_api_key() -> Option<String> {
    std::env::var("AGENTSKILLS_API_KEY").ok()
}

fn lock_json_path() -> PathBuf {
    HUB_BASE_DIR.join("lock.json")
}

fn audit_log_path() -> PathBuf {
    HUB_BASE_DIR.join("audit.log")
}

fn quarantine_dir(name: &str) -> PathBuf {
    HUB_BASE_DIR.join("quarantine").join(name)
}

fn read_lock_file() -> HubLockFile {
    let path = lock_json_path();
    if !path.exists() {
        return HubLockFile::default();
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn write_lock_file(lock: &HubLockFile) -> Result<(), ToolError> {
    std::fs::create_dir_all(&*HUB_BASE_DIR).map_err(|e| {
        ToolError::execution_failed(format!("Failed to create .hub directory: {}", e))
    })?;
    let json = serde_json::to_string_pretty(lock).map_err(|e| {
        ToolError::execution_failed(format!("Failed to serialize lock.json: {}", e))
    })?;
    std::fs::write(lock_json_path(), json)
        .map_err(|e| ToolError::execution_failed(format!("Failed to write lock.json: {}", e)))
}

fn append_audit_log(entry: &str) -> Result<(), ToolError> {
    std::fs::create_dir_all(&*HUB_BASE_DIR).map_err(|e| {
        ToolError::execution_failed(format!("Failed to create .hub directory: {}", e))
    })?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let line = format!("[{}] {}\n", timestamp, entry);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_log_path())
        .map_err(|e| ToolError::execution_failed(format!("Failed to open audit.log: {}", e)))?;
    std::io::Write::write_all(&mut file, line.as_bytes())
        .map_err(|e| ToolError::execution_failed(format!("Failed to write audit.log: {}", e)))
}

async fn hub_search(query: &str, page: usize, page_size: usize) -> Result<HubSearchResult, String> {
    let url = format!(
        "{}/v1/skills/search?q={}&page={}&page_size={}",
        hub_api_url(),
        urlencoding::encode(query),
        page,
        page_size
    );
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(key) = hub_api_key() {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Skills Hub API error: {}", resp.status()));
    }
    resp.json::<HubSearchResult>()
        .await
        .map_err(|e| e.to_string())
}

async fn hub_get_skill(skill_id: &str) -> Result<HubSkill, String> {
    let url = format!("{}/v1/skills/{}", hub_api_url(), skill_id);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(key) = hub_api_key() {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Skills Hub API error: {}", resp.status()));
    }
    resp.json::<HubSkill>().await.map_err(|e| e.to_string())
}

async fn hub_download_skill(skill_id: &str) -> Result<Vec<u8>, String> {
    let url = format!("{}/v1/skills/{}/download", hub_api_url(), skill_id);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(key) = hub_api_key() {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Download failed: {}", resp.status()));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

async fn hub_publish_skill(name: &str, version: &str, content: &str) -> Result<String, String> {
    let url = format!("{}/v1/skills/publish", hub_api_url());
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": name,
        "version": version,
        "content": content,
    });
    let mut req = client.post(&url).json(&body);
    if let Some(key) = hub_api_key() {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Publish failed ({}): {}", status, body));
    }
    let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(result["id"].as_str().unwrap_or("unknown").to_string())
}

fn extract_zip_to_dir(data: &[u8], target_dir: &PathBuf) -> Result<(), ToolError> {
    const MAX_ZIP_SIZE: usize = 50 * 1024 * 1024;
    const MAX_SINGLE_FILE_SIZE: u64 = 10 * 1024 * 1024;
    if data.len() > MAX_ZIP_SIZE {
        return Err(ToolError::execution_failed(format!(
            "Zip archive too large: {} bytes (max {} bytes)",
            data.len(),
            MAX_ZIP_SIZE
        )));
    }
    let canonical_target = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.clone());
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ToolError::execution_failed(format!("Failed to read zip archive: {}", e)))?;
    std::fs::create_dir_all(target_dir).map_err(|e| {
        ToolError::execution_failed(format!("Failed to create target directory: {}", e))
    })?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ToolError::execution_failed(format!("Failed to read zip entry: {}", e)))?;
        if file.size() > MAX_SINGLE_FILE_SIZE {
            return Err(ToolError::execution_failed(format!(
                "File '{}' too large: {} bytes (max {} bytes)",
                file.name(),
                file.size(),
                MAX_SINGLE_FILE_SIZE
            )));
        }
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };
        let canonical_outpath = if let Ok(canon) = outpath.canonicalize() {
            canon
        } else {
            outpath.clone()
        };
        if !canonical_outpath.starts_with(&canonical_target) {
            return Err(ToolError::execution_failed(format!(
                "Zip Slip: path '{}' escapes target directory",
                outpath.display()
            )));
        }
        if file.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| {
                ToolError::execution_failed(format!("Failed to create directory: {}", e))
            })?;
        } else {
            if let Some(p) = outpath.parent()
                && !p.exists()
            {
                std::fs::create_dir_all(p).map_err(|e| {
                    ToolError::execution_failed(format!("Failed to create parent directory: {}", e))
                })?;
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| {
                ToolError::execution_failed(format!("Failed to create file: {}", e))
            })?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| ToolError::execution_failed(format!("Failed to write file: {}", e)))?;
        }
    }
    Ok(())
}

// ── SkillHubSearch ──

pub struct SkillHubSearchTool;

#[async_trait]
impl Tool for SkillHubSearchTool {
    fn name(&self) -> &str {
        "SkillHubSearch"
    }
    fn description(&self) -> &str {
        "搜索 agentskills.io 技能中心，查找可安装的技能。\
         返回匹配的技能列表（名称、描述、作者、评分、下载量等）。\
         找到目标技能后，使用 SkillHubInstall 安装。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词"
                },
                "page": {
                    "type": "integer",
                    "description": "页码（默认 1）"
                },
                "page_size": {
                    "type": "integer",
                    "description": "每页数量（默认 10）"
                }
            },
            "required": ["query"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Err(ToolError::invalid_input("query is required"));
        }
        let page = input["page"].as_u64().unwrap_or(1) as usize;
        let page_size = input["page_size"].as_u64().unwrap_or(10) as usize;

        let result = hub_search(query, page, page_size)
            .await
            .map_err(|e| ToolError::execution_failed(format!("Skills Hub 搜索失败: {}", e)))?;

        if result.skills.is_empty() {
            return Ok(ToolResult::success(format!(
                "在 agentskills.io 上未找到匹配 '{}' 的技能",
                query
            )));
        }

        let mut out = format!("## Skills Hub 搜索: '{}'\n\n", query);
        for s in &result.skills {
            out.push_str(&format!(
                "- **{}** (v{}) by {} — {} [⭐{:.1} 📥{}]\n  ID: `{}`  Category: {}\n\n",
                s.name, s.version, s.author, s.description, s.rating, s.downloads, s.id, s.category
            ));
        }
        out.push_str(&format!(
            "\n共 {} 个结果（第 {}/{} 页）。使用 SkillHubInstall 安装指定技能。",
            result.total,
            result.page,
            result.total.div_ceil(page_size)
        ));

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "query": query,
                "total": result.total,
                "page": result.page,
                "page_size": result.page_size,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── SkillHubInstall ──

pub struct SkillHubInstallTool;

#[async_trait]
impl Tool for SkillHubInstallTool {
    fn name(&self) -> &str {
        "SkillHubInstall"
    }
    fn description(&self) -> &str {
        "从 agentskills.io 安装技能。\
         下载技能到隔离区（quarantine）并展示 SKILL.md 供用户审查。\
         安装分两步：1) 首次调用下载到隔离区并返回内容摘要；\
         2) 使用 SkillHubReview 审查后批准或拒绝。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "技能 ID（从 SkillHubSearch 获取）"
                }
            },
            "required": ["skill_id"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_id = input["skill_id"].as_str().unwrap_or("");
        if skill_id.is_empty() {
            return Err(ToolError::invalid_input("skill_id is required"));
        }

        let skill_info = hub_get_skill(skill_id)
            .await
            .map_err(|e| ToolError::execution_failed(format!("获取技能信息失败: {}", e)))?;

        let skill_name = skill_info.name.clone();

        let final_dir = HUB_SKILLS_DIR.join(&skill_name);
        if final_dir.exists() {
            return Err(ToolError::execution_failed(format!(
                "技能 '{}' 已安装（存在于 {}），请先删除后再重新安装",
                skill_name,
                final_dir.display()
            )));
        }

        let q_dir = quarantine_dir(&skill_name);
        if q_dir.exists() {
            std::fs::remove_dir_all(&q_dir)
                .map_err(|e| ToolError::execution_failed(format!("清理旧隔离目录失败: {}", e)))?;
        }

        let data = hub_download_skill(skill_id)
            .await
            .map_err(|e| ToolError::execution_failed(format!("下载技能失败: {}", e)))?;

        extract_zip_to_dir(&data, &q_dir)?;

        let skill_md_path = find_skill_md(&q_dir);
        let skill_md_content = if let Some(md_path) = &skill_md_path {
            std::fs::read_to_string(md_path)
                .map_err(|e| ToolError::execution_failed(format!("读取 SKILL.md 失败: {}", e)))?
        } else {
            "(未找到 SKILL.md 文件)".to_string()
        };

        append_audit_log(&format!(
            "INSTALL_START skill_id={} name={} version={} -> quarantine",
            skill_id, skill_name, skill_info.version
        ))?;

        let out = format!(
            "## 技能已下载到隔离区\n\n\
             技能 **{}** (v{}) 已下载到:\n`{}`\n\n\
             ---\n### SKILL.md 内容预览\n\n{}\n\n\
             ---\n⚠️ **此技能当前处于隔离区，尚未正式安装。**\n\
             请审查以上内容后，使用 SkillHubReview 批准安装或拒绝删除。",
            skill_name,
            skill_info.version,
            q_dir.display(),
            skill_md_content
        );

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "skill_id": skill_id,
                "skill_name": skill_name,
                "version": skill_info.version,
                "status": "quarantined",
                "quarantine_path": q_dir.to_string_lossy().to_string(),
                "has_skill_md": skill_md_path.is_some(),
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

fn find_skill_md(dir: &PathBuf) -> Option<PathBuf> {
    let direct = dir.join("SKILL.md");
    if direct.exists() {
        return Some(direct);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                let nested = entry.path().join("SKILL.md");
                if nested.exists() {
                    return Some(nested);
                }
            }
        }
    }
    None
}

// ── SkillHubReview ──

pub struct SkillHubReviewTool;

#[async_trait]
impl Tool for SkillHubReviewTool {
    fn name(&self) -> &str {
        "SkillHubReview"
    }
    fn description(&self) -> &str {
        "审查隔离区中的技能，批准安装或拒绝删除。\
         action=approve: 将技能从隔离区移动到正式技能目录，更新 lock.json 和 audit.log。\
         action=reject: 从隔离区删除该技能，记录到 audit.log。\
         不指定 action 时仅展示隔离区技能的 SKILL.md 内容。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "技能名称"
                },
                "action": {
                    "type": "string",
                    "enum": ["approve", "reject"],
                    "description": "操作：approve（批准安装）或 reject（拒绝删除）。不指定则仅查看。"
                }
            },
            "required": ["name"]
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
        let name = input["name"].as_str().unwrap_or("");
        if name.is_empty() {
            return Err(ToolError::invalid_input("name is required"));
        }
        let action = input["action"].as_str();

        let q_dir = quarantine_dir(name);
        if !q_dir.exists() {
            return Err(ToolError::execution_failed(format!(
                "隔离区中未找到技能 '{}'。请先使用 SkillHubInstall 下载。",
                name
            )));
        }

        let skill_md_path = find_skill_md(&q_dir);
        let skill_md_content = if let Some(md_path) = &skill_md_path {
            std::fs::read_to_string(md_path)
                .map_err(|e| ToolError::execution_failed(format!("读取 SKILL.md 失败: {}", e)))?
        } else {
            "(未找到 SKILL.md 文件)".to_string()
        };

        match action {
            Some("approve") => {
                let final_dir = HUB_SKILLS_DIR.join(name);
                if final_dir.exists() {
                    return Err(ToolError::execution_failed(format!(
                        "技能 '{}' 已存在于正式目录，无法重复安装",
                        name
                    )));
                }

                let source_dir = determine_skill_source_dir(&q_dir);

                copy_dir_recursive(&source_dir, &final_dir)?;

                std::fs::remove_dir_all(&q_dir)
                    .map_err(|e| ToolError::execution_failed(format!("清理隔离目录失败: {}", e)))?;

                let mut lock = read_lock_file();
                lock.entries.push(HubLockEntry {
                    name: name.to_string(),
                    id: String::new(),
                    version: String::new(),
                    installed_at: chrono::Utc::now().to_rfc3339(),
                    source: "agentskills.io".to_string(),
                });
                write_lock_file(&lock)?;

                append_audit_log(&format!(
                    "INSTALL_APPROVE name={} -> {}",
                    name,
                    final_dir.display()
                ))?;

                Ok(ToolResult {
                    content: format!(
                        "✅ 技能 **{}** 已批准安装并移至:\n`{}`\n\n技能现在可以通过 SkillView 使用。",
                        name,
                        final_dir.display()
                    ),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "name": name,
                        "action": "approve",
                        "installed_path": final_dir.to_string_lossy().to_string(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            Some("reject") => {
                std::fs::remove_dir_all(&q_dir).map_err(|e| {
                    ToolError::execution_failed(format!("删除隔离区技能失败: {}", e))
                })?;

                append_audit_log(&format!("INSTALL_REJECT name={}", name))?;

                Ok(ToolResult {
                    content: format!("❌ 技能 **{}** 已被拒绝并从隔离区删除。", name),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "name": name,
                        "action": "reject",
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            _ => Ok(ToolResult {
                content: format!(
                    "## 隔离区技能审查: {}\n\n---\n\n{}\n\n---\n\
                         使用 action=approve 批准安装，或 action=reject 拒绝删除。",
                    name, skill_md_content
                ),
                is_error: false,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "name": name,
                    "action": "preview",
                    "has_skill_md": skill_md_path.is_some(),
                })),
                duration_ms: None,
                progress: Vec::new(),
            }),
        }
    }
}

fn determine_skill_source_dir(q_dir: &PathBuf) -> PathBuf {
    let skill_md = q_dir.join("SKILL.md");
    if skill_md.exists() {
        return q_dir.clone();
    }
    if let Ok(entries) = std::fs::read_dir(q_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() && entry.path().join("SKILL.md").exists() {
                return entry.path();
            }
        }
    }
    q_dir.clone()
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), ToolError> {
    std::fs::create_dir_all(dst)
        .map_err(|e| ToolError::execution_failed(format!("Failed to create directory: {}", e)))?;
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.filter_map(|e| e.ok()) {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| {
                    ToolError::execution_failed(format!("Failed to copy file: {}", e))
                })?;
            }
        }
    }
    Ok(())
}

// ── SkillHubPublish ──

pub struct SkillHubPublishTool;

#[async_trait]
impl Tool for SkillHubPublishTool {
    fn name(&self) -> &str {
        "SkillHubPublish"
    }
    fn description(&self) -> &str {
        "将本地技能发布到 agentskills.io 技能中心。\
         验证 SKILL.md 格式后调用 Hub API 发布。发布成功后记录到 audit.log。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "要发布的本地技能名称"
                }
            },
            "required": ["skill"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_name = input["skill"].as_str().unwrap_or("");
        if skill_name.is_empty() {
            return Err(ToolError::invalid_input("skill name is required"));
        }

        let (content, version, _description) = {
            let mut index = SKILL_INDEX
                .lock()
                .map_err(|_| ToolError::execution_failed("Failed to acquire skill index lock"))?;

            let Some((_source_kind, skill_dir)) = index.find_skill(skill_name) else {
                return Err(ToolError::execution_failed(format!(
                    "本地技能 '{}' 未找到",
                    skill_name
                )));
            };

            let skill_md_path = skill_dir.join("SKILL.md");
            if !skill_md_path.exists() {
                return Err(ToolError::execution_failed(format!(
                    "技能 '{}' 缺少 SKILL.md 文件，无法发布",
                    skill_name
                )));
            }

            let content = std::fs::read_to_string(&skill_md_path)
                .map_err(|e| ToolError::execution_failed(format!("读取 SKILL.md 失败: {}", e)))?;

            let (version, description) = extract_publish_metadata(&content);

            if description.is_empty() {
                return Err(ToolError::execution_failed(
                    "SKILL.md 缺少 description 字段（frontmatter 或正文首行），无法发布",
                ));
            }

            (content, version, description)
        };

        let published_id = hub_publish_skill(skill_name, &version, &content)
            .await
            .map_err(|e| ToolError::execution_failed(format!("发布失败: {}", e)))?;

        append_audit_log(&format!(
            "PUBLISH name={} version={} hub_id={}",
            skill_name, version, published_id
        ))?;

        Ok(ToolResult {
            content: format!(
                "✅ 技能 **{}** (v{}) 已成功发布到 agentskills.io！\nHub ID: `{}`",
                skill_name, version, published_id
            ),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "skill_name": skill_name,
                "version": version,
                "hub_id": published_id,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

fn extract_publish_metadata(content: &str) -> (String, String) {
    let mut version = "1.0.0".to_string();
    let mut description = String::new();

    let trimmed = content.trim_start();
    if trimmed.starts_with("---")
        && let Some(end) = trimmed[3..].find("---")
    {
        let yaml_str = &trimmed[3..3 + end];
        if let Ok(frontmatter) = serde_yaml::from_str::<serde_json::Value>(yaml_str) {
            version = frontmatter
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();
            description = frontmatter
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }

    if description.is_empty() {
        description = content
            .lines()
            .find(|l| {
                !l.trim().is_empty() && !l.trim().starts_with("---") && !l.trim().starts_with('#')
            })
            .unwrap_or("")
            .to_string();
    }

    (version, description)
}

// ── .env file helpers for F19 ──

static ENV_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join(".env")
});

fn read_env_file() -> std::collections::HashMap<String, String> {
    let path = &*ENV_FILE_PATH;
    if !path.exists() {
        return std::collections::HashMap::new();
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

fn write_env_file(map: &std::collections::HashMap<String, String>) -> Result<(), ToolError> {
    let path = &*ENV_FILE_PATH;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ToolError::execution_failed(format!("Failed to create .axagent directory: {}", e))
        })?;
    }
    let mut lines: Vec<String> = map.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    lines.sort();
    std::fs::write(path, lines.join("\n"))
        .map_err(|e| ToolError::execution_failed(format!("Failed to write .env file: {}", e)))
}

fn set_env_var(name: &str, value: &str) -> Result<(), ToolError> {
    if axagent_core::secure_store::is_secret_key(name) {
        let store = axagent_core::secure_store::CombinedSecureStore::with_default_paths();
        store.store_secret(name, value).map_err(|e| {
            ToolError::execution_failed(format!("Failed to store secret securely: {}", e))
        })?;
        Ok(())
    } else {
        let mut map = read_env_file();
        map.insert(name.to_string(), value.to_string());
        write_env_file(&map)
    }
}

fn is_env_var_set(name: &str) -> bool {
    if std::env::var(name).is_ok() {
        return true;
    }
    let map = read_env_file();
    map.contains_key(name) && !map.get(name).is_none_or(|v| v.is_empty())
}

// ── config.yaml helpers for F20 ──

static CONFIG_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("config.yaml")
});

fn read_config_yaml() -> serde_json::Value {
    let path = &*CONFIG_FILE_PATH;
    if !path.exists() {
        return serde_json::json!({});
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_yaml::from_str(&content).unwrap_or(serde_json::json!({}))
}

fn write_config_yaml(doc: &serde_json::Value) -> Result<(), ToolError> {
    let path = &*CONFIG_FILE_PATH;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ToolError::execution_failed(format!("Failed to create .axagent directory: {}", e))
        })?;
    }
    let yaml = serde_yaml::to_string(doc).map_err(|e| {
        ToolError::execution_failed(format!("Failed to serialize config.yaml: {}", e))
    })?;
    std::fs::write(path, yaml)
        .map_err(|e| ToolError::execution_failed(format!("Failed to write config.yaml: {}", e)))
}

fn get_skill_config_value(skill_name: &str, key: &str) -> Option<String> {
    let doc = read_config_yaml();
    doc.get("skills")
        .and_then(|s| s.get("config"))
        .and_then(|c| c.get(format!("{}.{}", skill_name, key)))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn set_skill_config_value(skill_name: &str, key: &str, value: &str) -> Result<(), ToolError> {
    let mut doc = read_config_yaml();
    if doc.is_null() || !doc.is_object() {
        doc = serde_json::json!({});
    }
    let obj = doc.as_object_mut().unwrap();
    if !obj.contains_key("skills") {
        obj.insert("skills".into(), serde_json::json!({}));
    }
    let skills = obj.get_mut("skills").unwrap().as_object_mut().unwrap();
    if !skills.contains_key("config") {
        skills.insert("config".into(), serde_json::json!({}));
    }
    let config = skills.get_mut("config").unwrap().as_object_mut().unwrap();
    config.insert(format!("{}.{}", skill_name, key), serde_json::Value::String(value.to_string()));
    write_config_yaml(&doc)
}

fn get_all_skill_config_values(skill_name: &str) -> std::collections::HashMap<String, String> {
    let doc = read_config_yaml();
    let mut result = std::collections::HashMap::new();
    let prefix = format!("{}.", skill_name);
    if let Some(config) = doc
        .get("skills")
        .and_then(|s| s.get("config"))
        .and_then(|c| c.as_object())
    {
        for (k, v) in config {
            if k.starts_with(&prefix)
                && let Some(val) = v.as_str()
            {
                result.insert(k[prefix.len()..].to_string(), val.to_string());
            }
        }
    }
    result
}

// ── SkillEnvCheckTool (F19) ──

pub struct SkillEnvCheckTool;

#[async_trait]
impl Tool for SkillEnvCheckTool {
    fn name(&self) -> &str {
        "SkillEnvCheck"
    }
    fn description(&self) -> &str {
        "检查和管理技能所需的环境变量（安全设置）。\
         action=check: 检查指定技能的必需环境变量，报告缺失项；\
         action=list: 列出所有技能及其环境变量需求；\
         action=set: 设置环境变量值（存储到 ~/.axagent/.env）。\
         不会在输出中暴露密钥值，仅显示是否已设置。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["check", "list", "set"],
                    "description": "操作: check（检查技能环境变量）、list（列出所有技能需求）、set（设置环境变量）"
                },
                "skill": {
                    "type": "string",
                    "description": "技能名称（check 和 set 操作需要）"
                },
                "name": {
                    "type": "string",
                    "description": "环境变量名称（set 操作需要）"
                },
                "value": {
                    "type": "string",
                    "description": "环境变量值（set 操作需要）"
                }
            },
            "required": ["action"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("");

        match action {
            "check" => {
                let skill_name = input["skill"].as_str().unwrap_or("");
                if skill_name.is_empty() {
                    return Err(ToolError::invalid_input(
                        "skill name is required for check action",
                    ));
                }

                let mut index = SKILL_INDEX.lock().map_err(|_| {
                    ToolError::execution_failed("Failed to acquire skill index lock")
                })?;

                let entry = index.find_skill_entry(skill_name).cloned();
                let Some(entry) = entry else {
                    return Err(ToolError::execution_failed(format!(
                        "Skill '{}' 未找到",
                        skill_name
                    )));
                };

                if entry.required_environment_variables.is_empty() {
                    return Ok(ToolResult::success(format!(
                        "技能 '{}' 不需要任何环境变量。",
                        skill_name
                    )));
                }

                let mut out = format!("## 技能 '{}' 环境变量检查\n\n", skill_name);
                let mut missing_count = 0;
                let mut set_count = 0;

                for var in &entry.required_environment_variables {
                    let is_set = is_env_var_set(&var.name);
                    if is_set {
                        set_count += 1;
                        out.push_str(&format!(
                            "- ✅ **{}**: 已设置{}",
                            var.name,
                            if var.required_for.is_empty() {
                                String::new()
                            } else {
                                format!(" (用途: {})", var.required_for)
                            }
                        ));
                    } else {
                        missing_count += 1;
                        out.push_str(&format!(
                            "- ❌ **{}**: 未设置 — {}{}",
                            var.name,
                            var.prompt,
                            if var.help.is_empty() {
                                String::new()
                            } else {
                                format!(" (帮助: {})", var.help)
                            }
                        ));
                    }
                    out.push('\n');
                }

                out.push_str(&format!(
                    "\n共 {} 个环境变量，{} 已设置，{} 缺失。使用 SkillEnvCheck action=set 设置缺失变量。",
                    entry.required_environment_variables.len(), set_count, missing_count
                ));

                Ok(ToolResult {
                    content: out,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "total_vars": entry.required_environment_variables.len(),
                        "set_count": set_count,
                        "missing_count": missing_count,
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "list" => {
                let mut index = SKILL_INDEX.lock().map_err(|_| {
                    ToolError::execution_failed("Failed to acquire skill index lock")
                })?;

                let entries = index.all_entries().to_vec();

                let mut out = String::from("## 技能环境变量需求列表\n\n");
                let mut skills_with_env = 0;

                for entry in &entries {
                    if entry.required_environment_variables.is_empty() {
                        continue;
                    }
                    skills_with_env += 1;
                    out.push_str(&format!("### {} (v{})\n\n", entry.name, entry.version));
                    for var in &entry.required_environment_variables {
                        let is_set = is_env_var_set(&var.name);
                        out.push_str(&format!(
                            "- {} **{}**: {}{}\n",
                            if is_set { "✅" } else { "❌" },
                            var.name,
                            var.prompt,
                            if var.required_for.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", var.required_for)
                            }
                        ));
                    }
                    out.push('\n');
                }

                if skills_with_env == 0 {
                    out.push_str("没有技能需要环境变量。\n");
                } else {
                    out.push_str(&format!("共 {} 个技能需要环境变量配置。\n", skills_with_env));
                }

                Ok(ToolResult {
                    content: out,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skills_with_env_vars": skills_with_env,
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "set" => {
                let name = input["name"].as_str().unwrap_or("");
                let value = input["value"].as_str().unwrap_or("");
                if name.is_empty() {
                    return Err(ToolError::invalid_input("name is required for set action"));
                }
                if value.is_empty() {
                    return Err(ToolError::invalid_input("value is required for set action"));
                }

                set_env_var(name, value)?;

                Ok(ToolResult {
                    content: format!(
                        "✅ 环境变量 '{}' 已设置。值已安全存储，不会在输出中显示。",
                        name
                    ),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "name": name,
                        "action": "set",
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            _ => Err(ToolError::invalid_input(format!(
                "未知 action '{}'，支持: check, list, set",
                action
            ))),
        }
    }
}

// ── SkillConfigTool (F20) ──

pub struct SkillConfigTool;

#[async_trait]
impl Tool for SkillConfigTool {
    fn name(&self) -> &str {
        "SkillConfig"
    }
    fn description(&self) -> &str {
        "管理技能的配置设置。\
         action=show: 显示指定技能的所有配置项及当前值；\
         action=set: 设置配置值（key 格式: 'skill.key'，存储到 ~/.axagent/config.yaml）；\
         action=get: 获取指定配置项的值；\
         action=migrate: 列出所有未配置的设置项，便于批量配置。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["show", "set", "get", "migrate"],
                    "description": "操作: show（显示配置）、set（设置配置）、get（获取配置）、migrate（列出未配置项）"
                },
                "skill": {
                    "type": "string",
                    "description": "技能名称（show/get/migrate 操作需要）"
                },
                "key": {
                    "type": "string",
                    "description": "配置键名（set/get 操作需要，set 时格式为 'skill.key'）"
                },
                "value": {
                    "type": "string",
                    "description": "配置值（set 操作需要）"
                }
            },
            "required": ["action"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("");

        match action {
            "show" => {
                let skill_name = input["skill"].as_str().unwrap_or("");
                if skill_name.is_empty() {
                    return Err(ToolError::invalid_input("skill name is required for show action"));
                }

                let mut index = SKILL_INDEX.lock().map_err(|_| {
                    ToolError::execution_failed("Failed to acquire skill index lock")
                })?;

                let entry = index.find_skill_entry(skill_name).cloned();
                let Some(entry) = entry else {
                    return Err(ToolError::execution_failed(format!(
                        "Skill '{}' 未找到",
                        skill_name
                    )));
                };

                if entry.config_settings.is_empty() {
                    return Ok(ToolResult::success(format!(
                        "技能 '{}' 没有可配置的设置项。",
                        skill_name
                    )));
                }

                let current_values = get_all_skill_config_values(skill_name);
                let mut out = format!("## 技能 '{}' 配置设置\n\n", skill_name);

                for setting in &entry.config_settings {
                    let current = current_values.get(&setting.key);
                    let display_value = current
                        .cloned()
                        .or(setting.default.clone())
                        .unwrap_or_else(|| "(未设置)".to_string());

                    out.push_str(&format!(
                        "- **{}** ({}): {}\n  描述: {}\n  当前值: {}\n\n",
                        setting.key,
                        setting.setting_type,
                        setting.prompt,
                        setting.description,
                        display_value
                    ));
                }

                out.push_str(&format!(
                    "共 {} 个配置项。使用 SkillConfig action=set 设置值（key 格式: '{}.key'）。",
                    entry.config_settings.len(),
                    skill_name
                ));

                Ok(ToolResult {
                    content: out,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "total_settings": entry.config_settings.len(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "set" => {
                let key = input["key"].as_str().unwrap_or("");
                let value = input["value"].as_str().unwrap_or("");
                if key.is_empty() || value.is_empty() {
                    return Err(ToolError::invalid_input(
                        "key and value are required for set action",
                    ));
                }

                let (skill_name, setting_key) = if key.contains('.') {
                    let mut parts = key.splitn(2, '.');
                    (parts.next().unwrap().to_string(), parts.next().unwrap().to_string())
                } else {
                    return Err(ToolError::invalid_input(
                        "key 格式应为 'skill.key'，例如 'my-skill.api_endpoint'",
                    ));
                };

                set_skill_config_value(&skill_name, &setting_key, value)?;

                Ok(ToolResult {
                    content: format!("✅ 配置项 '{}.{}' 已设置。", skill_name, setting_key),
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "key": setting_key,
                        "action": "set",
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            "get" => {
                let skill_name = input["skill"].as_str().unwrap_or("");
                let key = input["key"].as_str().unwrap_or("");
                if skill_name.is_empty() || key.is_empty() {
                    return Err(ToolError::invalid_input(
                        "skill and key are required for get action",
                    ));
                }

                let value = get_skill_config_value(skill_name, key);

                match value {
                    Some(v) => Ok(ToolResult {
                        content: format!("配置项 '{}.{}' = {}", skill_name, key, v),
                        is_error: false,
                        truncated: false,
                        metadata: Some(serde_json::json!({
                            "skill_name": skill_name,
                            "key": key,
                            "value": v,
                        })),
                        duration_ms: None,
                        progress: Vec::new(),
                    }),
                    None => {
                        let mut index = SKILL_INDEX.lock().map_err(|_| {
                            ToolError::execution_failed("Failed to acquire skill index lock")
                        })?;
                        if let Some(entry) = index.find_skill_entry(skill_name)
                            && let Some(setting) =
                                entry.config_settings.iter().find(|s| s.key == key)
                            && let Some(default) = &setting.default
                        {
                            return Ok(ToolResult {
                                content: format!(
                                    "配置项 '{}.{}' 未设置，默认值为: {}",
                                    skill_name, key, default
                                ),
                                is_error: false,
                                truncated: false,
                                metadata: Some(serde_json::json!({
                                    "skill_name": skill_name,
                                    "key": key,
                                    "default": default,
                                })),
                                duration_ms: None,
                                progress: Vec::new(),
                            });
                        }
                        Ok(ToolResult {
                            content: format!("配置项 '{}.{}' 未设置且无默认值。", skill_name, key),
                            is_error: false,
                            truncated: false,
                            metadata: Some(serde_json::json!({
                                "skill_name": skill_name,
                                "key": key,
                                "value": null,
                            })),
                            duration_ms: None,
                            progress: Vec::new(),
                        })
                    },
                }
            },
            "migrate" => {
                let skill_name = input["skill"].as_str().unwrap_or("");
                if skill_name.is_empty() {
                    return Err(ToolError::invalid_input(
                        "skill name is required for migrate action",
                    ));
                }

                let mut index = SKILL_INDEX.lock().map_err(|_| {
                    ToolError::execution_failed("Failed to acquire skill index lock")
                })?;

                let entry = index.find_skill_entry(skill_name).cloned();
                let Some(entry) = entry else {
                    return Err(ToolError::execution_failed(format!(
                        "Skill '{}' 未找到",
                        skill_name
                    )));
                };

                if entry.config_settings.is_empty() {
                    return Ok(ToolResult::success(format!(
                        "技能 '{}' 没有可配置的设置项，无需迁移。",
                        skill_name
                    )));
                }

                let current_values = get_all_skill_config_values(skill_name);
                let mut unconfigured = Vec::new();

                for setting in &entry.config_settings {
                    let has_value =
                        current_values.contains_key(&setting.key) || setting.default.is_some();
                    if !has_value {
                        unconfigured.push(setting);
                    }
                }

                if unconfigured.is_empty() {
                    return Ok(ToolResult::success(format!(
                        "技能 '{}' 的所有配置项均已设置。",
                        skill_name
                    )));
                }

                let mut out = format!("## 技能 '{}' 未配置项\n\n", skill_name);
                out.push_str("以下配置项尚未设置，请使用 SkillConfig action=set 逐项配置：\n\n");

                for (i, setting) in unconfigured.iter().enumerate() {
                    out.push_str(&format!(
                        "{}. **{}** ({}): {}\n   描述: {}\n   设置命令: SkillConfig action=set key=\"{}.{}\" value=\"<你的值>\"\n\n",
                        i + 1,
                        setting.key,
                        setting.setting_type,
                        setting.prompt,
                        setting.description,
                        skill_name,
                        setting.key
                    ));
                }

                out.push_str(&format!(
                    "共 {} 个未配置项（总计 {} 个配置项）。",
                    unconfigured.len(),
                    entry.config_settings.len()
                ));

                Ok(ToolResult {
                    content: out,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "unconfigured_count": unconfigured.len(),
                        "total_settings": entry.config_settings.len(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                })
            },
            _ => Err(ToolError::invalid_input(format!(
                "未知 action '{}'，支持: show, set, get, migrate",
                action
            ))),
        }
    }
}
