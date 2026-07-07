// SPDX-License-Identifier: AGPL-3.0-only

//! Trigger 子系统：Schedule / Webhook / Event 触发器管理。
//!
//! TriggerManager 负责注册、注销和调度三种触发器类型：
//! - Schedule: 基于 cron 表达式的定时调度
//! - Webhook: 基于 HTTP 路径的入站请求路由
//! - Event: 基于事件总线的发布/订阅

pub mod event_bus;
pub mod scheduler;
pub mod webhook_server;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::work_engine::WorkEngine;

/// 触发器管理器 —— 生命周期与 WorkEngine 一致。
///
/// Schedule 触发器通过 tokio::spawn 创建常驻定时任务，
/// Webhook 通过内部 HTTP server 路由请求，
/// Event 通过内存事件总线匹配订阅。
pub struct TriggerManager {
    /// 工作流引擎引用（用于触发时调用 run_workflow）
    engine: Arc<RwLock<Option<Arc<WorkEngine>>>>,
    /// 活跃的定时任务句柄：workflow_id → JoinHandle
    active_schedules: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
    /// Webhook 路由映射：path → (workflow_id, method, response_mode)
    webhook_routes: RwLock<HashMap<String, WebhookRoute>>,
    /// 事件订阅映射：event_type → Vec<workflow_id>
    event_subscriptions: RwLock<HashMap<String, Vec<String>>>,
}

/// Webhook 路由条目
#[derive(Debug, Clone)]
pub struct WebhookRoute {
    pub workflow_id: String,
    pub method: String,
    /// "sync" | "async"
    pub response_mode: String,
    /// P0-6: HMAC-SHA256 共享密钥。若设置则要求请求必须带 X-Webhook-Signature。
    /// 设为 None 表示不校验（仅用于本地开发/调试；生产必须设置）。
    pub secret: Option<String>,
}

impl TriggerManager {
    /// 创建触发器管理器。engine 通过 `set_engine` 延迟注入。
    pub fn new() -> Self {
        Self {
            engine: Arc::new(RwLock::new(None)),
            active_schedules: RwLock::new(HashMap::new()),
            webhook_routes: RwLock::new(HashMap::new()),
            event_subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// 注入工作流引擎引用（WorkEngine 构造完成后调用）。
    pub async fn set_engine(&self, engine: Arc<WorkEngine>) {
        *self.engine.write().await = Some(engine);
    }

    /// 获取引擎引用，供子模块使用。
    pub(crate) async fn get_engine(&self) -> Option<Arc<WorkEngine>> {
        self.engine.read().await.clone()
    }

    // ── Schedule ──

    /// 注册定时触发器。重复注册同一 workflow_id 会先取消旧任务。
    pub async fn register_schedule(
        &self,
        workflow_id: &str,
        cron: &str,
        timezone: &str,
        input_params: Option<serde_json::Value>,
    ) -> Result<(), String> {
        self.unregister_schedule(workflow_id).await;
        let handle =
            scheduler::spawn_schedule(self, workflow_id, cron, timezone, input_params).await?;
        self.active_schedules.write().await.insert(workflow_id.to_string(), handle);
        Ok(())
    }

    /// 注销定时触发器。
    pub async fn unregister_schedule(&self, workflow_id: &str) {
        if let Some(handle) = self.active_schedules.write().await.remove(workflow_id) {
            handle.abort();
        }
    }

    // ── Webhook ──

    /// 注册 Webhook 路由。
    pub async fn register_webhook(
        &self,
        workflow_id: &str,
        path: &str,
        method: &str,
        response_mode: &str,
    ) {
        self.register_webhook_with_secret(workflow_id, path, method, response_mode, None).await;
    }

    /// P0-6: 注册带 HMAC 共享密钥的 webhook 路由。
    pub async fn register_webhook_with_secret(
        &self,
        workflow_id: &str,
        path: &str,
        method: &str,
        response_mode: &str,
        secret: Option<String>,
    ) {
        self.webhook_routes.write().await.insert(
            path.to_string(),
            WebhookRoute {
                workflow_id: workflow_id.to_string(),
                method: method.to_string(),
                response_mode: response_mode.to_string(),
                secret,
            },
        );
    }

    /// 注销 Webhook 路由。
    pub async fn unregister_webhook(&self, path: &str) {
        self.webhook_routes.write().await.remove(path);
    }

    /// 获取 Webhook 路由快照（供 webhook server 使用）。
    pub async fn get_webhook_routes(&self) -> HashMap<String, WebhookRoute> {
        self.webhook_routes.read().await.clone()
    }

    // ── Event ──

    /// 注册事件订阅。
    pub async fn register_event(&self, workflow_id: &str, event_type: &str) {
        let mut subs = self.event_subscriptions.write().await;
        subs.entry(event_type.to_string()).or_default().push(workflow_id.to_string());
    }

    /// 注销事件订阅。
    pub async fn unregister_event(&self, workflow_id: &str, event_type: &str) {
        let mut subs = self.event_subscriptions.write().await;
        if let Some(list) = subs.get_mut(event_type) {
            list.retain(|id| id != workflow_id);
            if list.is_empty() {
                subs.remove(event_type);
            }
        }
    }

    /// 发布事件，触发所有订阅该事件类型的工作流。
    pub async fn publish_event(&self, event_type: &str, payload: serde_json::Value) -> Vec<String> {
        let subs = self.event_subscriptions.read().await;
        let workflow_ids = subs.get(event_type).cloned().unwrap_or_default();
        let engine = self.get_engine().await;
        let mut triggered = Vec::new();

        if let Some(ref engine) = engine {
            for wf_id in &workflow_ids {
                let run_opts = crate::work_engine::RunOptions {
                    input: Some(payload.clone()),
                    ..Default::default()
                };
                match engine.run_workflow(wf_id, run_opts).await {
                    Ok(_) => {
                        triggered.push(wf_id.clone());
                    },
                    Err(e) => {
                        tracing::warn!(
                            workflow_id = %wf_id,
                            event_type = %event_type,
                            error = %e,
                            "事件触发工作流失败"
                        );
                    },
                }
            }
        }
        triggered
    }

    /// 启动 Webhook HTTP 服务器。
    pub async fn start_webhook_server(&self, bind_addr: &str) -> Result<(), String> {
        let bind_addr = bind_addr.to_string();
        let routes = self.get_webhook_routes().await;
        let engine = self.get_engine().await.ok_or_else(|| "引擎未就绪".to_string())?;

        webhook_server::serve(bind_addr, engine, routes).await
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}
