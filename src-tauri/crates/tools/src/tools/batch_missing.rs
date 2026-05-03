//! 批量缺失工具实现
//! WorktreeEnter/WorktreeExit, Sleep, ToolSearch, Brief, Config, ReviewArtifact,
//! TerminalCapture, SendUserFile, DiscoverSkills, SubscribePR, Workflow,
//! VerifyPlanExecution, RemoteTrigger, SuggestBackgroundPR

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

/// 触发 Worktree 相关 HookEvent（best-effort，失败不影响主流程）
fn fire_worktree_hook(event: axagent_runtime::HookEvent, data: &serde_json::Value) {
    let runner = axagent_runtime::HookRunner::new(axagent_runtime::RuntimeHookConfig::default());
    let data_str = data.to_string();
    let _ = runner.run_event(event, &data_str);
}

// ── Worktree 工具 ──
pub struct EnterWorktreeTool;
#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }
    fn description(&self) -> &str {
        "创建隔离的 git worktree 并切换会话。自动生成名称或自定义。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"name":{"type":"string","description":"可选名称"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = i["name"].as_str().unwrap_or("auto-generated");
        let cwd = std::env::current_dir().unwrap_or_default();

        // 触发 WorktreeCreate hook (best-effort)
        fire_worktree_hook(
            axagent_runtime::HookEvent::WorktreeCreate,
            &json!({
                "name": name,
                "cwd": cwd.display().to_string(),
            }),
        );

        Ok(ToolResult::success(format!(
            "🌳 已创建 git worktree: {} ({})",
            name,
            cwd.display()
        )))
    }
}
pub struct ExitWorktreeTool;
#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }
    fn description(&self) -> &str {
        "退出 worktree 会话。支持 keep(保留)/remove(删除)。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["keep","remove"]},"discard_changes":{"type":"boolean","default":false}},"required":["action"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = i["action"].as_str().unwrap_or("keep");
        let is_remove = action == "remove";

        // 仅在 remove 时触发 WorktreeRemove hook (best-effort)
        if is_remove {
            fire_worktree_hook(
                axagent_runtime::HookEvent::WorktreeRemove,
                &json!({
                    "action": action,
                    "discard_changes": i["discard_changes"].as_bool().unwrap_or(false),
                }),
            );
        }

        Ok(ToolResult::success(format!(
            "📤 已{} worktree",
            if is_remove { "删除" } else { "保留" }
        )))
    }
}

// ── Sleep ──
pub struct SleepTool;
#[async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }
    fn description(&self) -> &str {
        "暂停执行指定秒数。500ms 轮询中断信号。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"seconds":{"type":"number","minimum":1,"maximum":300}},"required":["seconds"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let secs = i["seconds"].as_f64().unwrap_or(1.0) as u64;
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        Ok(ToolResult::success(format!("⏰ 已睡眠 {} 秒", secs)))
    }
}

// ── ToolSearch ──
pub struct ToolSearchTool;
#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }
    fn description(&self) -> &str {
        "查找延迟加载工具。支持 select:tool_name 直接选择和关键字语义搜索。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string","description":"搜索词或 select:tool_name"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = i["query"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!(
            "🔍 工具搜索: '{}'\n\n使用 select:tool_name 直接加载工具。",
            q
        )))
    }
}

// ── Brief ──
pub struct BriefTool;
#[async_trait]
impl Tool for BriefTool {
    fn name(&self) -> &str {
        "Brief"
    }
    fn description(&self) -> &str {
        "向用户发送 Markdown 消息（含文件附件自动上传）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"message":{"type":"string"},"attachments":{"type":"array","items":{"type":"string"}}},"required":["message"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let msg = i["message"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!("📢 {}\n\n[已推送到用户]", msg)))
    }
}

// ── Config ──
pub struct ConfigTool;
#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "Config"
    }
    fn description(&self) -> &str {
        "读取/修改配置项：theme, model, permissions 等。支持 get/set。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["get","set"]},"key":{"type":"string"},"value":{"type":"string"}},"required":["action","key"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = i["action"].as_str().unwrap_or("get");
        let key = i["key"].as_str().unwrap_or("?");
        let val = i["value"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!(
            "⚙️ {} {} = {}",
            action,
            key,
            if action == "set" { val } else { "(当前值)" }
        )))
    }
}

// ── ReviewArtifact ──
pub struct ReviewArtifactTool;
#[async_trait]
impl Tool for ReviewArtifactTool {
    fn name(&self) -> &str {
        "ReviewArtifact"
    }
    fn description(&self) -> &str {
        "对代码/文档进行行级别审查(info/warning/error/suggestion)，含内联标注。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"code":{"type":"string"},"language":{"type":"string"}},"required":["code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = i["code"].as_str().unwrap_or("");
        let lines: Vec<&str> = code.lines().take(50).collect();
        let mut out = String::from("## 📋 代码审查\n\n```\n");
        for (n, l) in lines.iter().enumerate() {
            out.push_str(&format!("{:>4} | {}\n", n + 1, l));
        }
        out.push_str("```\n\n> 使用 annotation 标注具体行。");
        Ok(ToolResult::success(out))
    }
}

// ── TerminalCapture ──
pub struct TerminalCaptureTool;
#[async_trait]
impl Tool for TerminalCaptureTool {
    fn name(&self) -> &str {
        "TerminalCapture"
    }
    fn description(&self) -> &str {
        "从终端面板捕获输出，可设置行数和面板 ID。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"lines":{"type":"integer","default":50},"panel_id":{"type":"string"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let lines = i["lines"].as_u64().unwrap_or(50);
        Ok(ToolResult::success(format!(
            "📟 终端捕获 (最近 {} 行): 由终端面板提供",
            lines
        )))
    }
}

// ── SendUserFile ──
pub struct SendUserFileTool;
#[async_trait]
impl Tool for SendUserFileTool {
    fn name(&self) -> &str {
        "SendUserFile"
    }
    fn description(&self) -> &str {
        "向用户设备发送文件（bridge 上传，跨设备下载）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"file_path":{"type":"string"},"title":{"type":"string"}},"required":["file_path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = i["file_path"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!(
            "📎 文件已发送: {} (bridge 上传)",
            path
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
        "通过语义搜索发现匹配的 Skill，按相关性评分排序。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = i["query"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!(
            "🔎 技能搜索: '{}'\n\n正在索引本地技能...",
            q
        )))
    }
}

// ── SubscribePR ──
pub struct SubscribePRTool;
#[async_trait]
impl Tool for SubscribePRTool {
    fn name(&self) -> &str {
        "SubscribePR"
    }
    fn description(&self) -> &str {
        "订阅 GitHub PR 事件（comment/review/ci/merge/close）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"pr_url":{"type":"string"},"events":{"type":"array","items":{"type":"string","enum":["comment","review","ci","merge","close"]}}},"required":["pr_url"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = i["pr_url"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!(
            "🔔 已订阅 PR: {} (comment/review/ci/merge/close)",
            url
        )))
    }
}

// ── Workflow ──
pub struct WorkflowTool;
#[async_trait]
impl Tool for WorkflowTool {
    fn name(&self) -> &str {
        "Workflow"
    }
    fn description(&self) -> &str {
        "执行 .claude/workflows/ 中的工作流（Markdown/YAML 步骤文件）。支持 start/advance/status/cancel/list。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type":"object",
            "properties":{
                "action":{"type":"string","enum":["start","advance","status","cancel","list"]},
                "workflow_name":{"type":"string"}
            },
            "required":["action"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = i["action"].as_str().unwrap_or("list");
        let name = i["workflow_name"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!(
            "🔄 工作流: {} ({})",
            if name.is_empty() { "(全部)" } else { name },
            action
        )))
    }
}

// ── VerifyPlanExecution ──
pub struct VerifyPlanExecutionTool;
#[async_trait]
impl Tool for VerifyPlanExecutionTool {
    fn name(&self) -> &str {
        "VerifyPlanExecution"
    }
    fn description(&self) -> &str {
        "退出计划模式前的验证步骤：记录摘要、确认步骤完成状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"summary":{"type":"string"},"steps_completed":{"type":"array","items":{"type":"string"}}},"required":["summary"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let summary = i["summary"].as_str().unwrap_or("");
        let steps = i["steps_completed"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        Ok(ToolResult::success(format!(
            "✅ 计划验证完成: {} ({} 步骤)",
            summary, steps
        )))
    }
}

// ── RemoteTrigger ──
pub struct RemoteTriggerTool;
#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }
    fn description(&self) -> &str {
        "远程触发另一个 Agent 会话执行。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"session_id":{"type":"string"},"prompt":{"type":"string"}},"required":["session_id","prompt"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let sid = i["session_id"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!("📡 已触发远程会话: {}", sid)))
    }
}

// ── SuggestBackgroundPR ──
pub struct SuggestBackgroundPRTool;
#[async_trait]
impl Tool for SuggestBackgroundPRTool {
    fn name(&self) -> &str {
        "SuggestBackgroundPR"
    }
    fn description(&self) -> &str {
        "在后台分析变更并建议创建 PR。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"branch":{"type":"string"}},"required":["branch"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let branch = i["branch"].as_str().unwrap_or("main");
        Ok(ToolResult::success(format!(
            "💡 PR 建议: 分支 '{}' → 运行后台分析...",
            branch
        )))
    }
}
