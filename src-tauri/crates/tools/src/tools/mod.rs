//! 内置工具实现

use std::collections::HashSet;

pub mod agent;
pub mod agent_memory;
pub mod bash;
pub mod batch_missing;
pub mod browser;
pub mod ci;
pub mod computer_use;
pub mod context;
pub mod cron;
pub mod database;
pub mod devops;
pub mod document;
pub mod export;
pub mod file_edit;
pub mod file_read;
pub mod file_system;
pub mod file_write;
pub mod git;
pub mod glob;
pub mod grep;
pub mod integration;
pub mod knowledge;
pub mod lsp;
pub mod media;
pub mod media_delivery;
pub mod messaging;
pub mod migration_tool;
pub mod misc;
pub mod monitor;
pub mod network;
pub mod obsidian;
pub mod ocr;
pub mod personality;
pub mod plan;
pub mod push_notification;
pub mod repl;
pub mod rpc;
pub mod skill;
pub mod storage;
pub mod system_info;
pub mod task_system;
pub mod testing;
pub mod todo_write;
pub mod web_fetch;
pub mod web_search;
pub mod workspace;
pub mod worktree;

pub use todo_write::{AskUserQuestionTool, NotebookEditTool};

/// 注册所有内置工具到注册表
pub fn register_all(registry: &mut crate::registry::ToolRegistry) {
    registry.register_all(vec![
        // ── 核心文件操作 ──
        std::sync::Arc::new(file_read::FileReadTool),
        std::sync::Arc::new(file_write::FileWriteTool),
        std::sync::Arc::new(file_edit::FileEditTool),
        std::sync::Arc::new(file_system::ListDirectoryTool),
        std::sync::Arc::new(file_system::DeleteFileTool),
        std::sync::Arc::new(file_system::CreateDirectoryTool),
        std::sync::Arc::new(file_system::FileExistsTool),
        std::sync::Arc::new(file_system::GetFileInfoTool),
        std::sync::Arc::new(file_system::MoveFileTool),
        std::sync::Arc::new(glob::GlobTool),
        std::sync::Arc::new(grep::GrepTool),
        // ── Shell 和网络 ──
        std::sync::Arc::new(bash::BashTool),
        std::sync::Arc::new(web_fetch::WebFetchTool),
        std::sync::Arc::new(web_search::WebSearchTool),
        // ── 网络工具 ──
        std::sync::Arc::new(network::HttpRequestTool),
        std::sync::Arc::new(network::PingTool),
        std::sync::Arc::new(network::DnsLookupTool),
        std::sync::Arc::new(network::JsonApiTool),
        std::sync::Arc::new(network::RssReaderTool),
        std::sync::Arc::new(network::GraphQLTool),
        std::sync::Arc::new(network::WebSocketTool),
        // ── 任务和提问 ──
        std::sync::Arc::new(todo_write::TodoWriteTool),
        std::sync::Arc::new(todo_write::AskUserQuestionTool),
        std::sync::Arc::new(todo_write::NotebookEditTool),
        // ── Agent 和 Skill ──
        std::sync::Arc::new(agent::AgentTool),
        std::sync::Arc::new(skill::SkillsListTool),
        std::sync::Arc::new(skill::SkillViewTool),
        std::sync::Arc::new(skill::SkillReferenceTool),
        std::sync::Arc::new(skill::SkillBundleListTool),
        std::sync::Arc::new(skill::SkillBundleCreateTool),
        std::sync::Arc::new(skill::SkillBundleLoadTool),
        std::sync::Arc::new(skill::SkillBundleDeleteTool),
        // ── 计划模式 ──
        std::sync::Arc::new(plan::EnterPlanModeTool),
        std::sync::Arc::new(plan::ExitPlanModeTool),
        std::sync::Arc::new(plan::VerifyPlanExecutionTool),
        // ── 人格系统 ──
        std::sync::Arc::new(personality::PersonalityTool),
        // ── 桌面控制 ──
        std::sync::Arc::new(computer_use::ComputerUseTool),
        // ── 浏览器 ──
        std::sync::Arc::new(browser::BrowserNavigateTool),
        std::sync::Arc::new(browser::BrowserScreenshotTool),
        std::sync::Arc::new(browser::BrowserClickTool),
        std::sync::Arc::new(browser::BrowserFillTool),
        std::sync::Arc::new(browser::BrowserTypeTool),
        std::sync::Arc::new(browser::BrowserExtractTextTool),
        std::sync::Arc::new(browser::BrowserExtractAllTool),
        std::sync::Arc::new(browser::BrowserGetContentTool),
        std::sync::Arc::new(browser::BrowserSelectTool),
        std::sync::Arc::new(browser::BrowserWaitForTool),
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
        std::sync::Arc::new(worktree::EnterWorktreeTool),
        std::sync::Arc::new(worktree::ExitWorktreeTool),
        // ── 系统工具 ──
        std::sync::Arc::new(batch_missing::SleepTool),
        std::sync::Arc::new(batch_missing::ToolSearchTool),
        std::sync::Arc::new(batch_missing::ConfigTool),
        std::sync::Arc::new(batch_missing::ReviewArtifactTool),
        std::sync::Arc::new(batch_missing::TerminalCaptureTool),
        std::sync::Arc::new(skill::DiscoverSkillsTool),
        // ── Skills Hub (agentskills.io) ──
        std::sync::Arc::new(skill::SkillHubSearchTool),
        std::sync::Arc::new(skill::SkillHubInstallTool),
        std::sync::Arc::new(skill::SkillHubReviewTool),
        std::sync::Arc::new(skill::SkillHubPublishTool),
        std::sync::Arc::new(skill::SkillEnvCheckTool),
        std::sync::Arc::new(skill::SkillConfigTool),
        // ── 消息和文件 ──
        std::sync::Arc::new(batch_missing::BriefTool),
        std::sync::Arc::new(batch_missing::SendUserFileTool),
        std::sync::Arc::new(batch_missing::SubscribePRTool),
        std::sync::Arc::new(batch_missing::WorkflowTool),
        std::sync::Arc::new(agent::RemoteTriggerTool),
        std::sync::Arc::new(agent::SuggestBackgroundPRTool),
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
        std::sync::Arc::new(context::ContextResolveTool),
        // ── 知识库 ──
        std::sync::Arc::new(knowledge::ListKnowledgeBasesTool),
        std::sync::Arc::new(knowledge::SearchKnowledgeTool),
        std::sync::Arc::new(knowledge::CreateKnowledgeEntityTool),
        std::sync::Arc::new(knowledge::CreateKnowledgeFlowTool),
        std::sync::Arc::new(knowledge::CreateKnowledgeInterfaceTool),
        std::sync::Arc::new(knowledge::AddKnowledgeDocumentTool),
        // ── 存储管理 ──
        std::sync::Arc::new(storage::GetStorageInfoTool),
        std::sync::Arc::new(storage::ListStorageFilesTool),
        std::sync::Arc::new(storage::UploadStorageFileTool),
        std::sync::Arc::new(storage::DownloadStorageFileTool),
        std::sync::Arc::new(storage::DeleteStorageFileTool),
        // ── 系统信息 ──
        std::sync::Arc::new(system_info::GetSystemInfoTool),
        std::sync::Arc::new(system_info::ListProcessesTool),
        // ── Git ──
        std::sync::Arc::new(git::GitStatusTool),
        std::sync::Arc::new(git::GitDiffTool),
        std::sync::Arc::new(git::GitCommitTool),
        std::sync::Arc::new(git::GitLogTool),
        std::sync::Arc::new(git::GitBranchTool),
        std::sync::Arc::new(git::GitReviewTool),
        // ── OCR ──
        std::sync::Arc::new(ocr::OcrImageTool),
        std::sync::Arc::new(ocr::OcrDetectLangsTool),
        // ── Obsidian ──
        std::sync::Arc::new(obsidian::ObsidianGetVaultsTool),
        std::sync::Arc::new(obsidian::ObsidianListFilesTool),
        std::sync::Arc::new(obsidian::ObsidianReadFileTool),
        // ── 导出与格式 ──
        std::sync::Arc::new(document::ExportWordTool),
        std::sync::Arc::new(document::RenderMarkdownTool),
        std::sync::Arc::new(document::ExportPdfTool),
        std::sync::Arc::new(document::ExportXlsxTool),
        std::sync::Arc::new(document::ExportPptxTool),
        std::sync::Arc::new(document::ReadXlsxTool),
        std::sync::Arc::new(document::ReadPptxTool),
        std::sync::Arc::new(export::PdfInfoTool),
        std::sync::Arc::new(export::DetectEncodingTool),
        // ── 远程文件 ──
        std::sync::Arc::new(misc::RemoteFileUploadTool),
        std::sync::Arc::new(misc::RemoteFileListTool),
        std::sync::Arc::new(misc::RemoteFileDeleteTool),
        // ── 缓存管理 ──
        std::sync::Arc::new(misc::CacheInfoTool),
        std::sync::Arc::new(misc::CacheClearTool),
        // ── 工作区记忆 ──
        std::sync::Arc::new(workspace::WorkspaceReadTool),
        std::sync::Arc::new(workspace::WorkspaceWriteTool),
        // ── Agent 记忆 ──
        std::sync::Arc::new(agent_memory::SessionSearchTool),
        std::sync::Arc::new(agent_memory::MemoryFlushTool),
        std::sync::Arc::new(agent_memory::AgentCheckpointTool),
        std::sync::Arc::new(agent_memory::AgentStatusTool),
        std::sync::Arc::new(agent_memory::AgentRememberTool),
        // ── AI 媒体 ──
        std::sync::Arc::new(media::GenerateImageTool),
        std::sync::Arc::new(media::GenerateChartConfigTool),
        std::sync::Arc::new(media::SequentialThinkingTool),
        std::sync::Arc::new(media::Base64ImageTool),
        // ── 媒体智能投递 ──
        std::sync::Arc::new(media_delivery::MediaDetectTool),
        std::sync::Arc::new(media_delivery::MediaDeliverTool),
        std::sync::Arc::new(media_delivery::MediaPreviewTool),
        // ── 外部集成 ──
        std::sync::Arc::new(integration::DifyListBasesTool),
        std::sync::Arc::new(integration::DifySearchTool),
        // ── 通知 ──
        std::sync::Arc::new(push_notification::PushNotificationTool),
        // ── 数据库管理 ──
        std::sync::Arc::new(database::DatabaseQueryTool),
        std::sync::Arc::new(database::DatabaseListTablesTool),
        std::sync::Arc::new(database::DatabaseMigrationStatusTool),
        // ── 测试运行 ──
        std::sync::Arc::new(testing::RunTestsTool),
        std::sync::Arc::new(testing::RunLinterTool),
        std::sync::Arc::new(testing::RunTestCoverageTool),
        // ── CI/CD ──
        std::sync::Arc::new(ci::CiStatusTool),
        std::sync::Arc::new(ci::CiTriggerTool),
        std::sync::Arc::new(ci::CiListWorkflowsTool),
        // ── DevOps ──
        std::sync::Arc::new(devops::SecurityAuditTool),
        std::sync::Arc::new(devops::DeadCodeDetectTool),
        std::sync::Arc::new(devops::BundleAnalyzeTool),
        std::sync::Arc::new(devops::IssueCreateTool),
        std::sync::Arc::new(devops::IssueListTool),
        // ── RPC ──
        std::sync::Arc::new(rpc::RpcTool),
        std::sync::Arc::new(rpc::RpcCallTool),
        // ── 迁移工具 ──
        std::sync::Arc::new(migration_tool::MigrationTool),
    ]);

    let available_toolsets: HashSet<String> = registry
        .list_tools()
        .iter()
        .map(|t| format!("{:?}", t.category).to_lowercase())
        .collect();
    skill::set_available_toolsets(available_toolsets);
}
