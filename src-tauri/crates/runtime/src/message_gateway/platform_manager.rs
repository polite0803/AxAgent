use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use axagent_core::platform_config::PlatformConfig;
use crate::message_gateway::platforms;
use crate::message_gateway::platforms::PlatformAdapter;
use crate::message_gateway::session_router::SessionRouter;

#[async_trait::async_trait]
pub trait PlatformMessageCallback: Send + Sync {
    async fn on_message(
        &self,
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        chat_id: &str,
        text: &str,
    ) -> Option<String>;
}

pub struct PlatformManager {
    adapters: RwLock<HashMap<String, Arc<dyn PlatformAdapter>>>,
    session_router: RwLock<SessionRouter>,
    running_adapters: RwLock<Vec<String>>,
    message_callback: RwLock<Option<Arc<dyn PlatformMessageCallback>>>,
}

impl PlatformManager {
    pub fn new() -> Self {
        let mut adapters: HashMap<String, Arc<dyn PlatformAdapter>> = HashMap::new();

        adapters.insert(
            "telegram".to_string(),
            Arc::new(platforms::telegram::TelegramAdapter::new()),
        );
        adapters.insert(
            "discord".to_string(),
            Arc::new(platforms::discord::DiscordAdapter::new()),
        );
        adapters.insert(
            "wechat".to_string(),
            Arc::new(platforms::wechat::WeChatAdapter::new()),
        );
        adapters.insert(
            "feishu".to_string(),
            Arc::new(platforms::feishu::FeishuAdapter::new()),
        );
        adapters.insert(
            "qq".to_string(),
            Arc::new(platforms::qq::QQAdapter::new()),
        );
        adapters.insert(
            "dingtalk".to_string(),
            Arc::new(platforms::dingtalk::DingtalkAdapter::new()),
        );
        adapters.insert(
            "slack".to_string(),
            Arc::new(platforms::slack::SlackAdapter::new()),
        );
        adapters.insert(
            "whatsapp".to_string(),
            Arc::new(platforms::whatsapp::WhatsAppAdapter::new()),
        );

        Self {
            adapters: RwLock::new(adapters),
            session_router: RwLock::new(SessionRouter::new()),
            running_adapters: RwLock::new(Vec::new()),
            message_callback: RwLock::new(None),
        }
    }

    pub async fn reconcile(
        &self,
        config: &PlatformConfig,
    ) -> anyhow::Result<PlatformReconcileReport> {
        let adapters = self.adapters.read().await;
        let mut running = self.running_adapters.write().await;
        let mut report = PlatformReconcileReport::default();

        for (name, adapter) in adapters.iter() {
            if adapter.is_enabled(config) {
                if !running.contains(name) {
                    match adapter.start(config).await {
                        Ok(()) => {
                            running.push(name.clone());
                            report.started.push(name.clone());
                            tracing::info!("[PlatformManager] {} started", name);
                        }
                        Err(e) => {
                            report
                                .errors
                                .push((name.clone(), format!("start failed: {}", e)));
                            tracing::error!("[PlatformManager] {} start failed: {}", name, e);
                        }
                    }
                }
            } else if running.contains(name) {
                match adapter.stop().await {
                    Ok(()) => {
                        running.retain(|n| n != name);
                        report.stopped.push(name.clone());
                        tracing::info!("[PlatformManager] {} stopped", name);
                    }
                    Err(e) => {
                        report
                            .errors
                            .push((name.clone(), format!("stop failed: {}", e)));
                        tracing::error!("[PlatformManager] {} stop failed: {}", name, e);
                    }
                }
            }
        }

        Ok(report)
    }

    pub async fn stop_all(&self) -> anyhow::Result<()> {
        let adapters = self.adapters.read().await;
        let mut running = self.running_adapters.write().await;

        for name in running.iter() {
            if let Some(adapter) = adapters.get(name) {
                let _ = adapter.stop().await;
            }
        }
        running.clear();
        Ok(())
    }

    pub async fn get_adapter(&self, name: &str) -> Option<Arc<dyn PlatformAdapter>> {
        let adapters = self.adapters.read().await;
        adapters.get(name).cloned()
    }

    pub async fn get_running_adapters(&self) -> Vec<String> {
        self.running_adapters.read().await.clone()
    }

    pub async fn set_message_callback(&self, callback: Arc<dyn PlatformMessageCallback>) {
        crate::message_gateway::platforms::set_message_callback(callback.clone());
        let mut cb = self.message_callback.write().await;
        *cb = Some(callback);
    }

    pub async fn get_message_callback(&self) -> Option<Arc<dyn PlatformMessageCallback>> {
        self.message_callback.read().await.clone()
    }

    pub async fn link_agent_session(
        &self,
        platform: &str,
        user_id: &str,
        agent_session_id: &str,
    ) -> Option<()> {
        let mut router = self.session_router.write().await;
        router.link_agent_session(platform, user_id, agent_session_id)
    }

    pub async fn get_statuses(
        &self,
        config: &PlatformConfig,
    ) -> Vec<PlatformAdapterStatus> {
        struct AdapterInfo {
            name: String,
            enabled: bool,
            in_running: bool,
            adapter: Arc<dyn PlatformAdapter>,
            active_sessions: i32,
        }

        let infos: Vec<AdapterInfo> = {
            let adapters = self.adapters.read().await;
            let running = self.running_adapters.read().await;
            let session_router = self.session_router.read().await;
            let active_sessions = session_router.list_active_sessions();

            adapters
                .iter()
                .map(|(name, adapter)| {
                    let active_count = active_sessions
                        .iter()
                        .filter(|s| s.platform == *name)
                        .count() as i32;
                    AdapterInfo {
                        name: name.clone(),
                        enabled: adapter.is_enabled(config),
                        in_running: running.contains(name),
                        adapter: adapter.clone(),
                        active_sessions: active_count,
                    }
                })
                .collect()
        };

        let mut statuses: Vec<PlatformAdapterStatus> = Vec::with_capacity(infos.len() + 1);

        for info in infos {
            let connected = if info.enabled && info.in_running {
                info.adapter.is_connected().await
            } else {
                false
            };
            statuses.push(PlatformAdapterStatus {
                name: info.name,
                enabled: info.enabled,
                connected,
                last_activity: None,
                active_sessions: info.active_sessions,
            });
        }

        statuses.push(PlatformAdapterStatus {
            name: "api_server".to_string(),
            enabled: config.api_server_enabled,
            connected: false,
            last_activity: None,
            active_sessions: 0,
        });

        statuses
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PlatformReconcileReport {
    pub started: Vec<String>,
    pub stopped: Vec<String>,
    pub errors: Vec<(String, String)>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlatformAdapterStatus {
    pub name: String,
    pub enabled: bool,
    pub connected: bool,
    pub last_activity: Option<i64>,
    pub active_sessions: i32,
}
