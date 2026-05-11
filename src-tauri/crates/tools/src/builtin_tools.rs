//! 全局状态管理（精简版）
//!
//! 所有工具定义和 handler 已迁移至 tools/*.rs 下的 Tool trait 实现。
//! 此文件仅保留少数必需的全局状态访问函数。
//! TODO: 后续将这些全局状态迁移到 ToolContext 或 AppState 中。

use sea_orm::DatabaseConnection;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;

// ── 数据库路径 ────────────────────────────────────────────────────────────

static GLOBAL_DB_PATH: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

pub fn set_global_db_path(path: &str) {
    let mut db_path = GLOBAL_DB_PATH.write().expect("GLOBAL_DB_PATH poisoned");
    *db_path = Some(path.to_string());
}

pub fn get_global_db_path() -> Option<String> {
    let db_path = GLOBAL_DB_PATH.read().expect("GLOBAL_DB_PATH poisoned");
    db_path.clone()
}

// ── SeaORM 数据库连接 ─────────────────────────────────────────────────────

static GLOBAL_SEA_DB: LazyLock<RwLock<Option<Arc<DatabaseConnection>>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_global_sea_db(db: Arc<DatabaseConnection>) {
    let mut sea_db = GLOBAL_SEA_DB.write().expect("GLOBAL_SEA_DB poisoned");
    *sea_db = Some(db);
}

pub fn get_global_sea_db() -> Option<Arc<DatabaseConnection>> {
    let sea_db = GLOBAL_SEA_DB.read().expect("GLOBAL_SEA_DB poisoned");
    sea_db.clone()
}

// ── 子 Agent 运行器 ───────────────────────────────────────────────────────

pub type SubAgentRunner = Arc<
    dyn Fn(
            String,
            String,
            String,
            String,
            String,
        )
            -> Pin<Box<dyn Future<Output = std::result::Result<(String, String), String>> + Send>>
        + Send
        + Sync,
>;

static GLOBAL_SUB_AGENT_RUNNER: LazyLock<RwLock<Option<SubAgentRunner>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_global_sub_agent_runner(runner: SubAgentRunner) {
    let mut r = GLOBAL_SUB_AGENT_RUNNER
        .write()
        .expect("GLOBAL_SUB_AGENT_RUNNER poisoned");
    *r = Some(runner);
}

pub fn get_global_sub_agent_runner() -> Option<SubAgentRunner> {
    let r = GLOBAL_SUB_AGENT_RUNNER
        .read()
        .expect("GLOBAL_SUB_AGENT_RUNNER poisoned");
    r.clone()
}

// ── 当前会话 ID ───────────────────────────────────────────────────────────

static GLOBAL_CURRENT_CONVERSATION_ID: LazyLock<RwLock<Option<String>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_current_conversation_id(id: &str) {
    let mut cid = GLOBAL_CURRENT_CONVERSATION_ID
        .write()
        .expect("GLOBAL_CURRENT_CONVERSATION_ID poisoned");
    *cid = Some(id.to_string());
}

pub fn get_current_conversation_id() -> Option<String> {
    let cid = GLOBAL_CURRENT_CONVERSATION_ID
        .read()
        .expect("GLOBAL_CURRENT_CONVERSATION_ID poisoned");
    cid.clone()
}

// ── 待处理子 Agent 卡片 ───────────────────────────────────────────────────

pub type PendingSubAgentCard = (String, String, String);

static PENDING_SUB_AGENT_CARDS: LazyLock<RwLock<std::collections::HashMap<String, PendingSubAgentCard>>> =
    LazyLock::new(|| RwLock::new(std::collections::HashMap::new()));

pub fn store_pending_sub_agent_card(
    parent_id: &str,
    child_id: &str,
    agent_type: &str,
    description: &str,
) {
    let mut m = PENDING_SUB_AGENT_CARDS
        .write()
        .expect("PENDING_SUB_AGENT_CARDS poisoned");
    m.insert(
        parent_id.to_string(),
        (child_id.to_string(), agent_type.to_string(), description.to_string()),
    );
}

pub fn take_pending_sub_agent_card(parent_id: &str) -> Option<PendingSubAgentCard> {
    let mut m = PENDING_SUB_AGENT_CARDS
        .write()
        .expect("PENDING_SUB_AGENT_CARDS poisoned");
    m.remove(parent_id)
}

// ── Fork 上下文 ───────────────────────────────────────────────────────────

pub fn store_fork_context(parent_id: &str, description: &str, prompt: &str) {
    let data = axagent_runtime_core::fork_bridge::ForkSessionData {
        parent_conversation_id: parent_id.to_string(),
        description: description.to_string(),
        prompt: prompt.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        parent_system_prompt: Vec::new(),
        parent_messages_json: String::new(),
        child_system_prompt: Some(axagent_runtime_core::fork_bridge::build_fork_child_prompt(prompt)),
    };
    axagent_runtime_core::fork_bridge::store_fork_session(data);
}

pub fn has_fork_context(parent_id: &str) -> bool {
    axagent_runtime_core::fork_bridge::has_fork_session(parent_id)
}
