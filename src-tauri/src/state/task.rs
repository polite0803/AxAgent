//! Task domain state.
//!
//! Owns the background-task bookkeeping: the central task manager, the
//! individual `JoinHandle` slots for the named long-running tasks
//! (auto-backup, webdav sync, API server, trajectory cleanup), the global
//! shutdown token, and the per-stream cancel/coordination maps used by
//! streaming endpoints.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub struct TaskState {
    pub task_manager: Arc<axagent_runtime::task_manager::TaskManager>,
    pub auto_backup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub webdav_sync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub api_server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub trajectory_cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub shutdown_token: CancellationToken,
    pub close_to_tray: Arc<AtomicBool>,
    pub stream_cancel_flags: Arc<DashMap<String, Arc<AtomicBool>>>,
    pub agent_permission_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_ask_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_always_allowed:
        Arc<Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>>,
    pub agent_prompters:
        Arc<Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>>,
}

#[allow(dead_code)]
impl TaskState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_manager: Arc<axagent_runtime::task_manager::TaskManager>,
        auto_backup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        webdav_sync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        api_server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        trajectory_cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        shutdown_token: CancellationToken,
        close_to_tray: Arc<AtomicBool>,
        stream_cancel_flags: Arc<DashMap<String, Arc<AtomicBool>>>,
        agent_permission_senders: Arc<
            Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
        >,
        agent_ask_senders: Arc<
            Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
        >,
        agent_always_allowed: Arc<
            Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
        >,
        agent_prompters: Arc<
            Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>,
        >,
    ) -> Self {
        Self {
            task_manager,
            auto_backup_handle,
            webdav_sync_handle,
            api_server_handle,
            trajectory_cleanup_handle,
            shutdown_token,
            close_to_tray,
            stream_cancel_flags,
            agent_permission_senders,
            agent_ask_senders,
            agent_always_allowed,
            agent_prompters,
        }
    }
}
