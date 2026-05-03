//! SkillTool - Skill 执行调度工具

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
        "执行一个已注册的 Skill。Skill 是预定义的任务模板，封装了特定领域的知识和工具组合。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "要执行的 Skill 名称"
                },
                "args": {
                    "type": "string",
                    "description": "传递给 Skill 的参数"
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
        let skill_name = input["skill"].as_str().unwrap();
        let args = input["args"].as_str().unwrap_or("");

        let mut output = format!("## Skill 执行: {}\n\n", skill_name);
        if !args.is_empty() {
            output.push_str(&format!("**参数**: {}\n\n", args));
        }
        output.push_str("[Skill 执行结果将由上层调度器处理]\n");

        Ok(ToolResult::success(output))
    }
}
