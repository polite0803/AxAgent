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
        "加载并执行一个已注册的 Skill。Skill 是预定义的任务模板，封装了特定领域的知识和工具组合。调用此工具后，你会收到该 Skill 的完整指令，请严格按照指令逐步执行。"
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
            if skill_md.exists() {
                if let Ok(content) = std::fs::read_to_string(skill_md) {
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
                    });
                }
            }

            let skill_md_alt = dir.join(format!("{}.md", skill_name));
            if skill_md_alt.exists() {
                if let Ok(content) = std::fs::read_to_string(skill_md_alt) {
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
                    });
                }
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
