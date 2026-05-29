use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct SkillTool;

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "Skill"
    }
    fn description(&self) -> &str {
        "加载并执行预注册的 Skill（领域任务模板）。Skill 封装了特定领域的知识、工具组合和操作流程。\
         调用后返回该 Skill 的完整指令——必须严格按指令逐步执行，按需使用其他工具。\
         不指定 args 时直接加载，指定时参数会注入到指令中。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "要加载的 Skill 名称"
                },
                "args": {
                    "type": "string",
                    "description": "传递给 Skill 的参数（可选）"
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
        &["SkillExecutor"]
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_name = input["skill"].as_str().unwrap_or("");
        let args = input["args"].as_str().unwrap_or("");

        if skill_name.is_empty() {
            return Err(ToolError::invalid_input("Skill name is required"));
        }

        let dirs = axagent_core::skill_dirs::skill_dirs();

        for (source_kind, dir) in &dirs {
            let skill_md = dir.join(skill_name).join("SKILL.md");
            if skill_md.exists()
                && let Ok(content) = std::fs::read_to_string(skill_md)
            {
                // 检查 skill-manifest.json 中的平台/版本约束
                let manifest_path = dir.join(skill_name).join("skill-manifest.json");
                if let Ok(manifest_str) = std::fs::read_to_string(&manifest_path)
                    && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_str)
                {
                    if let Some(platforms) = manifest["platforms"].as_array() {
                        let platform_list: Vec<&str> =
                            platforms.iter().filter_map(|p| p.as_str()).collect();
                        tracing::warn!(
                            "Skill '{}' declares platform constraints {:?} — verify compatibility",
                            skill_name,
                            platform_list
                        );
                    }
                    if let Some(requires) = manifest["requires_toolsets"].as_array() {
                        let requires_list: Vec<&str> =
                            requires.iter().filter_map(|r| r.as_str()).collect();
                        tracing::warn!(
                            "Skill '{}' requires toolsets {:?} — ensure they are available",
                            skill_name,
                            requires_list
                        );
                    }
                }

                let mut output = format!(
                    "# Skill: {}\n\n以下是从 SKILL.md 加载的技能指令。请严格按照这些指令执行任务，按需使用其他工具。\n\n---\n\n{}",
                    skill_name, content
                );
                if !args.is_empty() {
                    output.push_str(&format!("\n\n---\n**用户参数**: {}", args));
                    output.push_str("\n请将上述参数应用到技能指令中。");
                }

                return Ok(ToolResult {
                    content: output,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "args": args,
                        "source": "SKILL.md",
                        "source_kind": source_kind,
                        "source_dir": dir.to_string_lossy().to_string(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                });
            }

            let skill_md_alt = dir.join(format!("{}.md", skill_name));
            if skill_md_alt.exists()
                && let Ok(content) = std::fs::read_to_string(skill_md_alt)
            {
                let mut output = format!(
                    "# Skill: {}\n\n以下是从 SKILL.md 加载的技能指令。请严格按照这些指令执行任务，按需使用其他工具。\n\n---\n\n{}",
                    skill_name, content
                );
                if !args.is_empty() {
                    output.push_str(&format!("\n\n---\n**用户参数**: {}", args));
                    output.push_str("\n请将上述参数应用到技能指令中。");
                }

                return Ok(ToolResult {
                    content: output,
                    is_error: false,
                    truncated: false,
                    metadata: Some(serde_json::json!({
                        "skill_name": skill_name,
                        "args": args,
                        "source": "SKILL.md",
                        "source_kind": source_kind,
                        "source_dir": dir.to_string_lossy().to_string(),
                    })),
                    duration_ms: None,
                    progress: Vec::new(),
                });
            }
        }

        let available = dirs
            .iter()
            .filter_map(|(_, d)| std::fs::read_dir(d).ok())
            .flat_map(|rd| rd.filter_map(|e| e.ok()))
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let hint = if available.is_empty() {
            "(无)".to_string()
        } else {
            available.join(", ")
        };

        Err(ToolError::execution_failed(format!(
            "Skill '{}' 未找到。可用的 skills: {}",
            skill_name, hint
        )))
    }
}

// ── DiscoverSkills ──

pub struct DiscoverSkillsTool;

#[async_trait]
impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &str {
        "DiscoverSkills"
    }
    fn description(&self) -> &str {
        "通过名称/描述关键词搜索已安装的 Skill。扫描所有技能目录，返回匹配的技能名称和描述。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = i["query"].as_str().unwrap_or("").to_lowercase();
        let dirs = axagent_core::skill_dirs::skill_dirs();
        let mut results: Vec<(String, String)> = Vec::new();

        for (_kind, dir) in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let md = entry.path().join("SKILL.md");
                        let desc = if md.exists() {
                            std::fs::read_to_string(&md)
                                .ok()
                                .and_then(|c| c.lines().next().map(|l| l.to_string()))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        if (name.to_lowercase().contains(&q) || desc.to_lowercase().contains(&q))
                            && !results.iter().any(|(n, _)| n == &name)
                        {
                            results.push((name, desc));
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(ToolResult::success(format!("未找到匹配 '{}' 的 Skill", q)))
        } else {
            let mut out = format!("## 技能搜索: '{}'\n\n", q);
            for (n, d) in &results {
                out.push_str(&format!("- **{}**: {}\n", n, d));
            }
            out.push_str(&format!("\n共 {} 个技能。使用 Skill 工具加载。", results.len()));
            Ok(ToolResult::success(out))
        }
    }
}
