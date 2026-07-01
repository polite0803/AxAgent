// SPDX-License-Identifier: AGPL-3.0-only

//! 事件总线 —— 内存级发布/订阅。
//!
//! 通过 TriggerManager 维护 event_type → workflow_id 映射，
//! publish() 遍历订阅者并调用 engine.run_workflow。

use std::collections::HashMap;
use std::sync::Arc;

use super::TriggerManager;

/// 事件总线 —— 轻量封装 TriggerManager 的 publish/subscribe 能力。
pub struct EventBus {
    manager: Arc<TriggerManager>,
}

impl EventBus {
    pub fn new(manager: Arc<TriggerManager>) -> Self {
        Self { manager }
    }

    /// 订阅事件。
    pub async fn subscribe(&self, workflow_id: &str, event_type: &str) {
        self.manager.register_event(workflow_id, event_type).await;
    }

    /// 取消订阅。
    pub async fn unsubscribe(&self, workflow_id: &str, event_type: &str) {
        self.manager.unregister_event(workflow_id, event_type).await;
    }

    /// 发布事件，返回成功触发的工作流 ID 列表。
    pub async fn publish(&self, event_type: &str, payload: serde_json::Value) -> Vec<String> {
        self.manager.publish_event(event_type, payload).await
    }

    /// 获取当前所有事件订阅的快照。
    pub async fn subscriptions(&self) -> HashMap<String, Vec<String>> {
        self.manager.event_subscriptions.read().await.clone()
    }
}
