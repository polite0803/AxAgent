// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
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
