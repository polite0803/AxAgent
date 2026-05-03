//! 内置工具实现

pub mod agent;
pub mod bash;
pub mod batch_missing;
pub mod computer_use;
pub mod context;
pub mod cron;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod grep;
pub mod lsp;
pub mod messaging;
pub mod monitor;
pub mod plan;
pub mod push_notification;
pub mod repl;
pub mod skill;
pub mod task_system;
pub mod todo_write;
pub mod web_fetch;
pub mod web_search;

pub use todo_write::{AskUserQuestionTool, NotebookEditTool};

/// 注册所有内置工具到注册表
pub fn register_all(registry: &mut crate::registry::ToolRegistry) {
    registry.register_all(vec![
        // ── 核心文件操作 ──
        std::sync::Arc::new(file_read::FileReadTool),
        std::sync::Arc::new(file_write::FileWriteTool),
        std::sync::Arc::new(file_edit::FileEditTool),
        std::sync::Arc::new(glob::GlobTool),
        std::sync::Arc::new(grep::GrepTool),
        // ── Shell 和网络 ──
        std::sync::Arc::new(bash::BashTool),
        std::sync::Arc::new(web_fetch::WebFetchTool),
        std::sync::Arc::new(web_search::WebSearchTool),
        // ── 任务和提问 ──
        std::sync::Arc::new(todo_write::TodoWriteTool),
        std::sync::Arc::new(todo_write::AskUserQuestionTool),
        std::sync::Arc::new(todo_write::NotebookEditTool),
        // ── Agent 和 Skill ──
        std::sync::Arc::new(agent::AgentTool),
        std::sync::Arc::new(skill::SkillTool),
        // ── 计划模式 ──
        std::sync::Arc::new(plan::EnterPlanModeTool),
        std::sync::Arc::new(plan::ExitPlanModeTool),
        std::sync::Arc::new(batch_missing::VerifyPlanExecutionTool),
        // ── 桌面控制 ──
        std::sync::Arc::new(computer_use::ComputerUseTool),
        // ── 定时任务 ──
        std::sync::Arc::new(cron::CronCreateTool),
        std::sync::Arc::new(cron::CronDeleteTool),
        std::sync::Arc::new(cron::CronListTool),
        // ── Task 系统 ──
        std::sync::Arc::new(task_system::TaskCreateTool),
        std::sync::Arc::new(task_system::TaskGetTool),
        std::sync::Arc::new(task_system::TaskListTool),
        std::sync::Arc::new(task_system::TaskStopTool),
        std::sync::Arc::new(task_system::TaskUpdateTool),
        std::sync::Arc::new(task_system::TaskOutputTool),
        // ── Worktree ──
        std::sync::Arc::new(batch_missing::EnterWorktreeTool),
        std::sync::Arc::new(batch_missing::ExitWorktreeTool),
        // ── 工具管理 ──
        std::sync::Arc::new(batch_missing::SleepTool),
        std::sync::Arc::new(batch_missing::ToolSearchTool),
        std::sync::Arc::new(batch_missing::ConfigTool),
        std::sync::Arc::new(batch_missing::ReviewArtifactTool),
        std::sync::Arc::new(batch_missing::TerminalCaptureTool),
        std::sync::Arc::new(batch_missing::DiscoverSkillsTool),
        // ── 消息和文件 ──
        std::sync::Arc::new(batch_missing::BriefTool),
        std::sync::Arc::new(batch_missing::SendUserFileTool),
        std::sync::Arc::new(batch_missing::SubscribePRTool),
        std::sync::Arc::new(batch_missing::WorkflowTool),
        std::sync::Arc::new(batch_missing::RemoteTriggerTool),
        std::sync::Arc::new(batch_missing::SuggestBackgroundPRTool),
        // ── 通信: SendMessage + ListPeers + Team ──
        std::sync::Arc::new(messaging::SendMessageTool),
        std::sync::Arc::new(messaging::ListPeersTool),
        std::sync::Arc::new(messaging::TeamCreateTool),
        std::sync::Arc::new(messaging::TeamDeleteTool),
        // ── 开发工具 ──
        std::sync::Arc::new(lsp::LSPTool),
        std::sync::Arc::new(repl::REPLTool),
        // ── 监控和上下文 ──
        std::sync::Arc::new(monitor::MonitorTool),
        std::sync::Arc::new(context::CtxInspectTool),
        std::sync::Arc::new(context::SnipTool),
        // ── 通知 ──
        std::sync::Arc::new(push_notification::PushNotificationTool),
    ]);
}
