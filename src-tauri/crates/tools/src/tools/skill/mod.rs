// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::disallowed_types)]

use std::collections::HashSet;

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;

pub mod env_config;
use axagent_kit::secure_store::SecureStore;
pub use env_config::{SkillConfigTool, SkillEnvCheckTool};
use parking_lot::Mutex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

static SKILL_INDEX: LazyLock<Mutex<SkillIndex>> = LazyLock::new(|| Mutex::new(SkillIndex::new()));

static AVAILABLE_TOOLSETS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn set_available_toolsets(toolsets: HashSet<String>) {
    let mut guard = AVAILABLE_TOOLSETS.lock();
    *guard = toolsets;

    let mut index = SKILL_INDEX.lock();
    index.invalidate();
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
    domain: ToolDomain,
    required_environment_variables: Vec<RequiredEnvVar>,
    config_settings: Vec<SkillConfigSetting>,
}

struct SkillIndex {
    entries: Vec<SkillIndexEntry>,
    built_at: Option<Instant>,
}

type SkillMetadata = (
    String,                  // description
    String,                  // category
    String,                  // version
    Vec<String>,             // platforms
    Vec<String>,             // tags
    Vec<String>,             // requires_toolsets
    Vec<String>,             // fallback_for_toolsets
    Vec<RequiredEnvVar>,     // required_env_vars
    Vec<SkillConfigSetting>, // config_settings
    String,                  // domain
);

impl SkillIndex {
    fn new() -> Self {
        Self { entries: Vec::new(), built_at: None }
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
        AVAILABLE_TOOLSETS.lock().contains(toolset_name)
    }

    fn should_include_entry(entry: &SkillIndexEntry) -> bool {
        if !entry.platforms.is_empty()
            && !entry.platforms.contains(&std::env::consts::OS.to_string())
        {
            return false;
        }

        if !entry.requires_toolsets.is_empty()
            && !entry.requires_toolsets.iter().all(|t| Self::is_toolset_available(t))
        {
            return false;
        }

        if !entry.fallback_for_toolsets.is_empty()
            && entry.fallback_for_toolsets.iter().any(|t| Self::is_toolset_available(t))
        {
            return false;
        }

        true
    }

    fn rebuild(&mut self) {
        let dirs = axagent_kit::skill_dirs::skill_dirs();
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
                        mut version,
                        platforms,
                        tags,
                        requires_toolsets,
                        fallback_for_toolsets,
                        required_env_vars,
                        config_settings,
                        domain,
                    ) = Self::extract_metadata(&skill_dir);

                    // 运行时只对本应用自身的技能进行版本检测；
                    // 其它应用的技能（claude/codebuddy/workbuddy/trae/agents 等）不检测版本，使用默认值。
                    // 判断依据取 kit::skill_dirs::self_source_kind()，fork 改项目名后自动适配。
                    if source_kind != axagent_kit::skill_dirs::self_source_kind() {
                        version = "1.0.0".to_string();
                    }

                    // 兼容历史旧值 core/invest/opc;未知值兜底 General
                    let domain_enum = domain.parse().unwrap_or(ToolDomain::General);

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
                        domain: domain_enum,
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
        let mut domain = "general".to_string();

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
                domain = frontmatter
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general")
                    .to_string();
                if let Some(p) = frontmatter.get("platforms").and_then(|v| v.as_array()) {
                    platforms = p.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                }
                if let Some(meta) = frontmatter.get("metadata") {
                    if let Some(cat) =
                        meta.get("hermes").and_then(|h| h.get("category")).and_then(|c| c.as_str())
                    {
                        category = cat.to_string();
                    }
                    if let Some(t) =
                        meta.get("hermes").and_then(|h| h.get("tags")).and_then(|ts| ts.as_array())
                    {
                        tags = t.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(rt) = meta
                        .get("hermes")
                        .and_then(|h| h.get("requires_toolsets"))
                        .and_then(|ts| ts.as_array())
                    {
                        requires_toolsets =
                            rt.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(ft) = meta
                        .get("hermes")
                        .and_then(|h| h.get("fallback_for_toolsets"))
                        .and_then(|ts| ts.as_array())
                    {
                        fallback_for_toolsets =
                            ft.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(rt) = meta
                        .get("axagent")
                        .and_then(|a| a.get("requires_toolsets"))
                        .and_then(|ts| ts.as_array())
                        && requires_toolsets.is_empty()
                    {
                        requires_toolsets =
                            rt.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(ft) = meta
                        .get("axagent")
                        .and_then(|a| a.get("fallback_for_toolsets"))
                        .and_then(|ts| ts.as_array())
                        && fallback_for_toolsets.is_empty()
                    {
                        fallback_for_toolsets =
                            ft.iter().filter_map(|v| v.as_str().map(String::from)).collect();
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
                    if let Some(cs) =
                        meta.get("axagent").and_then(|a| a.get("config")).and_then(|v| v.as_array())
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
                if let Some(env_vars) =
                    frontmatter.get("required_environment_variables").and_then(|v| v.as_array())
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
                platforms = p.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
            if let Some(t) = manifest["tags"].as_array() {
                tags = t.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
            if let Some(rt) = manifest["requires_toolsets"].as_array()
                && requires_toolsets.is_empty()
            {
                requires_toolsets =
                    rt.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
            if let Some(ft) = manifest["fallback_for_toolsets"].as_array()
                && fallback_for_toolsets.is_empty()
            {
                fallback_for_toolsets =
                    ft.iter().filter_map(|v| v.as_str().map(String::from)).collect();
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
            domain,
        )
    }

    fn parse_env_vars_from_json(arr: &[serde_json::Value]) -> Vec<RequiredEnvVar> {
        arr.iter()
            .filter_map(|v| {
                let obj = v.as_object()?;
                Some(RequiredEnvVar {
                    name: obj.get("name")?.as_str()?.to_string(),
                    prompt: obj.get("prompt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    help: obj.get("help").and_then(|v| v.as_str()).unwrap_or("").to_string(),
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
                    default: obj.get("default").and_then(|v| v.as_str()).map(String::from),
                    prompt: obj.get("prompt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
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
                domain: e.domain.as_str().to_string(),
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
    domain: String,
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
         返回每个技能的名称、描述、类别、版本和领域，不加载完整内容以节省 token。\
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let category_filter = input["category"].as_str();
        let mut index = SKILL_INDEX.lock();
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
                "- **{}** (v{}): {} [{}] [域:{}]\n",
                s.name, s.version, s.description, s.category, s.domain
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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
            let mut index = SKILL_INDEX.lock();

            let Some((source_kind, skill_dir)) = index.find_skill(skill_name) else {
                let available: Vec<String> =
                    index.list_skills(None).iter().map(|s| s.name.clone()).collect();
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
            let mut index = SKILL_INDEX.lock();
            index.list_reference_files(skill_name)
        };

        let entry = {
            let mut index = SKILL_INDEX.lock();
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
                    let resolved =
                        config_values.get(&setting.key).cloned().or(setting.default.clone());
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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

        let mut index = SKILL_INDEX.lock();

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

// ── DiscoverSkills（统一能力检索引擎）──
//
// 此前只查文件系统 SKILL_INDEX，且是全量子串扫描、无打分、无 top-k（命中多少
// 返回多少）—— 与能力护照索引互不相通，Tool/Toolchain/Template 搜不到。
// 现在同时检索两套目录、统一打分排序、top-k 截断。

/// 单条检索命中（来源统一抽象，屏蔽护照 / 文件系统技能的差异）。
struct DiscoverHit {
    /// 展示名（护照用 capability_id，技能用目录名）
    name: String,
    description: String,
    /// 类别 / 域标签（仅展示用）
    category: String,
    version: String,
    /// 相关性得分（越高越相关）
    score: f64,
    /// 来源目录：`capability`（护照全集）或 `skill`（文件系统 SKILL.md）
    source: &'static str,
}

/// 对一段候选文本打分：query 按空白分词，逐词匹配加权求和。
///
/// 权重设计：名称命中 > 标签命中 > 描述命中；前缀命中额外加权。
/// 任一词都未命中则 0 分（视为不匹配，直接淘汰）。
fn score_text(query_terms: &[String], name: &str, tags: &[String], description: &str) -> f64 {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();
    let tags_lower: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();

    let mut total = 0.0;
    for term in query_terms {
        let mut term_score = 0.0;
        if name_lower.contains(term) {
            term_score += 5.0;
            if name_lower.starts_with(term) {
                term_score += 3.0;
            }
        }
        if tags_lower.iter().any(|t| t.contains(term)) {
            term_score += 3.0;
        }
        if desc_lower.contains(term) {
            term_score += 2.0;
        }
        if term_score == 0.0 {
            // 任一词完全无命中 → 整体不匹配（AND 语义，避免弱相关噪音挤占 top-k）
            return 0.0;
        }
        total += term_score;
    }
    total
}

fn split_query_terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// 检索护照全集（Tool / Toolchain / Template / Skill / KnowledgeBase）。
async fn discover_from_passports(
    terms: &[String],
    domain_filter: Option<&str>,
) -> Vec<DiscoverHit> {
    let Some(indexer) = super::capability_shared::capability_indexer() else {
        return Vec::new();
    };
    let passports = indexer.list_passports().await;

    passports
        .into_iter()
        .filter(|p| p.is_user_visible())
        .filter(|p| domain_filter.is_none_or(|d| p.domain.as_str() == d))
        .filter_map(|p| {
            let summary = p.summary.as_deref().unwrap_or(&p.description);
            let mut score = score_text(terms, &p.name, &p.tags, summary);
            if score > 0.0 && p.sub_category.to_lowercase().contains(&terms.join(" ")) {
                score += 1.0;
            }
            (score > 0.0).then(|| DiscoverHit {
                name: p.capability_id.clone(),
                description: summary.to_string(),
                category: p.domain.as_str().to_string(),
                version: p.version.clone().unwrap_or_default(),
                score,
                source: "capability",
            })
        })
        .collect()
}

/// 检索文件系统技能（SKILL.md 目录 —— 护照未覆盖的部分）。
fn discover_from_skill_index(terms: &[String], domain_filter: Option<&str>) -> Vec<DiscoverHit> {
    let mut index = SKILL_INDEX.lock();
    index.ensure_built();

    index
        .entries
        .iter()
        .filter(|e| domain_filter.is_none_or(|d| e.domain.as_str() == d))
        .filter_map(|e| {
            let score = score_text(terms, &e.name, &e.tags, &e.description);
            (score > 0.0).then(|| DiscoverHit {
                name: e.name.clone(),
                description: e.description.clone(),
                category: e.category.clone(),
                version: e.version.clone(),
                score,
                source: "skill",
            })
        })
        .collect()
}

pub struct DiscoverSkillsTool;

#[async_trait]
impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &str {
        "DiscoverSkills"
    }
    fn description(&self) -> &str {
        "按关键词统一检索全部可发现能力（Level 0 — 统一检索引擎）。\
         覆盖能力护照全集（Tool/Toolchain/Template/Skill/KnowledgeBase）与文件系统技能，\
         多词 AND 匹配 + 相关性打分 + top-k 截断。\
         确定目标后：护照能力用 CapabilityView 展开定义，文件系统技能用 SkillView 加载正文。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词（多词为 AND 语义）"
                },
                "top_k": {
                    "type": "integer",
                    "description": "返回条数上限，缺省 10，最大 50"
                },
                "domain": {
                    "type": "string",
                    "description": "按域过滤（可选，如 general / invest / opc）"
                }
            },
            "required": ["query"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap_or("").trim().to_string();
        if query.is_empty() {
            return Err(ToolError::invalid_input("Query is required"));
        }
        let top_k = input["top_k"].as_u64().unwrap_or(10).clamp(1, 50) as usize;
        let domain_filter = input["domain"].as_str().filter(|d| !d.trim().is_empty());
        let terms = split_query_terms(&query);

        let mut hits = discover_from_passports(&terms, domain_filter).await;
        hits.extend(discover_from_skill_index(&terms, domain_filter));
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let total_matches = hits.len();
        let shown: Vec<DiscoverHit> = hits.into_iter().take(top_k).collect();

        if shown.is_empty() {
            return Ok(ToolResult::success(format!("未找到匹配 '{query}' 的能力或技能")));
        }

        let mut out = format!("## 能力检索: '{query}'（按相关性排序）\n\n");
        for (rank, hit) in shown.iter().enumerate() {
            let version_note = if hit.version.is_empty() {
                String::new()
            } else {
                format!(" (v{})", hit.version)
            };
            out.push_str(&format!(
                "{}. **{}**{} [{} 分] [{}|来源:{}]: {}\n",
                rank + 1,
                hit.name,
                version_note,
                hit.score as u64,
                hit.category,
                hit.source,
                hit.description
            ));
        }
        out.push_str(&format!(
            "\n共 {total_matches} 个匹配，显示前 {} 条。\
             护照能力（来源 capability）用 CapabilityView 展开、CapabilityLoad 加载；\
             文件系统技能（来源 skill）用 SkillView 加载正文。",
            shown.len()
        ));

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "level": 0,
                "query": query,
                "total_matches": total_matches,
                "returned": shown.len(),
                "top_k": top_k,
                "domain_filter": domain_filter,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

#[cfg(test)]
mod discover_tests {
    use super::{score_text, split_query_terms};

    #[test]
    fn multi_word_query_is_and_semantics() {
        // 多词查询为 AND 语义：每个词都必须命中，否则整体为 0（淘汰弱相关噪音）
        let terms = split_query_terms("stock analysis");
        // 两词均命中（stock 在 name、analysis 在 desc）→ 应 > 0
        assert!(score_text(&terms, "stock-analyzer", &[], "does analysis") > 0.0);

        // 仅命中部分词（"quantum" 完全无命中）→ AND 语义下应被淘汰为 0
        let partial = split_query_terms("stock quantum");
        assert_eq!(score_text(&partial, "stock-analyzer", &[], "does analysis"), 0.0);
    }

    #[test]
    fn name_match_outranks_description_match() {
        let terms = split_query_terms("scan");
        let by_name = score_text(&terms, "port-scanner", &[], "unrelated");
        let by_desc = score_text(&terms, "unrelated", &[], "can scan ports");
        assert!(by_name > by_desc);
    }

    #[test]
    fn prefix_hit_scores_higher() {
        let terms = split_query_terms("port");
        let prefix = score_text(&terms, "port-scanner", &[], "");
        let contains = score_text(&terms, "report-scanner", &[], "");
        assert!(prefix > contains);
    }

    #[test]
    fn case_insensitive_matching() {
        let terms = split_query_terms("SCAN");
        assert!(score_text(&terms, "port-scanner", &[], "") > 0.0);
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
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml" || ext == "yml"))
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
    load_all_bundles().into_iter().find(|b| b.slug() == slug || b.name == name)
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input["name"].as_str().unwrap_or("").to_string();
        let skills: Vec<String> = input["skills"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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

        let mut index = SKILL_INDEX.lock();

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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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

// ── Skills Hub — 本地隔离区管理（无网络依赖） ──

static HUB_BASE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("skills")
        .join(".hub")
});

static HUB_SKILLS_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join("skills")
});

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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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
                "隔离区中未找到技能 '{}'。请先将 SKILL.md 放入隔离区后重试。",
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
                    source: "local".to_string(),
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

// ── .env file helpers for F19 ──

static ENV_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join(".env")
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
    if axagent_kit::secure_store::is_secret_key(name) {
        let store = axagent_kit::secure_store::CombinedSecureStore::with_default_paths();
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

pub(crate) fn is_env_var_set(name: &str) -> bool {
    if std::env::var(name).is_ok() {
        return true;
    }
    let map = read_env_file();
    map.contains_key(name) && !map.get(name).is_none_or(|v| v.is_empty())
}

// ── config.yaml helpers for F20 ──

static CONFIG_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join("config.yaml")
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
    let obj = doc.as_object_mut().expect("技能配置：doc 应已确保为 object");
    if !obj.contains_key("skills") {
        obj.insert("skills".into(), serde_json::json!({}));
    }
    let skills = obj
        .get_mut("skills")
        .expect("技能配置：skills 键应已确保存在")
        .as_object_mut()
        .expect("技能配置：skills 应为 object");
    if !skills.contains_key("config") {
        skills.insert("config".into(), serde_json::json!({}));
    }
    let config = skills
        .get_mut("config")
        .expect("技能配置：config 键应已确保存在")
        .as_object_mut()
        .expect("技能配置：config 应为 object");
    config.insert(format!("{}.{}", skill_name, key), serde_json::Value::String(value.to_string()));
    write_config_yaml(&doc)
}

pub(crate) fn get_all_skill_config_values(
    skill_name: &str,
) -> std::collections::HashMap<String, String> {
    let doc = read_config_yaml();
    let mut result = std::collections::HashMap::new();
    let prefix = format!("{}.", skill_name);
    if let Some(config) =
        doc.get("skills").and_then(|s| s.get("config")).and_then(|c| c.as_object())
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
