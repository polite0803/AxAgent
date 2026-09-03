// SPDX-License-Identifier: AGPL-3.0-only

//! 全局事件总线 —— 跨 Engine 的事件订阅/分发通道。
//!
//! 通过 `tokio::sync::broadcast` 实现单进程内多引擎间的事件路由。
//! 不同的 Engine（WorkEngine / AgentCoordinator / Swarm）各自持有一个
//! `SubscriptionHandle`，通过 `GlobalEventBus::global()` 共享同一个
//! 广播通道。
//!
//! # 示例
//!
//! ```rust,ignore
//! let mut bus = GlobalEventBus::new(1024);
//! let mut rx1 = bus.subscribe("engine-a");
//! let mut rx2 = bus.subscribe("engine-b");
//! bus.emit(GlobalEngineEvent {
//!     source: "engine-a".into(),
//!     event_type: "node_completed".into(),
//!     payload: serde_json::json!({"node_id": "n1"}),
//! });
//! ```

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 全局引擎事件的统一 payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEngineEvent {
    /// 事件来源引擎标识符（如 "workflow:exec_123"、"agent:coord_1"）
    pub source: String,
    /// 事件类型（如 "node_completed"、"agent_started"、"swarm_task_assigned"）
    pub event_type: String,
    /// 事件负载
    pub payload: serde_json::Value,
    /// 关联的执行/会话 ID
    pub correlation_id: Option<String>,
}

/// 全局事件总线 —— 基于 broadcast channel 的跨 Engine 事件路由
pub struct GlobalEventBus {
    tx: broadcast::Sender<GlobalEngineEvent>,
}

impl GlobalEventBus {
    /// 创建新的全局事件总线
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// 订阅全局事件
    pub fn subscribe(&self) -> broadcast::Receiver<GlobalEngineEvent> {
        self.tx.subscribe()
    }

    /// 发布事件到所有订阅者
    pub fn emit(
        &self,
        event: GlobalEngineEvent,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<GlobalEngineEvent>> {
        self.tx.send(event)
    }

    /// 获取当前订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

/// 便利的引擎事件订阅句柄 —— 持有 receiver 并过滤指定 source 的事件
pub struct EngineEventSubscription {
    source_filter: Option<String>,
    receiver: broadcast::Receiver<GlobalEngineEvent>,
}

impl EngineEventSubscription {
    /// 创建新的订阅句柄
    pub fn new(rx: broadcast::Receiver<GlobalEngineEvent>) -> Self {
        Self { source_filter: None, receiver: rx }
    }

    /// 按来源过滤
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_filter = Some(source.into());
        self
    }

    /// 接收下一个匹配的事件（阻塞等待）
    pub async fn recv(&mut self) -> Option<GlobalEngineEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if let Some(ref filter) = self.source_filter {
                        if &event.source == filter || event.source.starts_with(filter.as_str()) {
                            return Some(event);
                        }
                        // 不匹配，继续接收
                        continue;
                    }
                    return Some(event);
                },
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("GlobalEventBus lagged by {} events", n);
                    continue;
                },
            }
        }
    }

    /// 尝试接收下一个事件（非阻塞）
    pub fn try_recv(&mut self) -> Option<GlobalEngineEvent> {
        loop {
            match self.receiver.try_recv() {
                Ok(event) => {
                    if let Some(ref filter) = self.source_filter {
                        if &event.source == filter || event.source.starts_with(filter.as_str()) {
                            return Some(event);
                        }
                        continue;
                    }
                    return Some(event);
                },
                Err(broadcast::error::TryRecvError::Empty) => return None,
                Err(broadcast::error::TryRecvError::Closed) => return None,
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    tracing::warn!("GlobalEventBus lagged by {} events", n);
                    continue;
                },
            }
        }
    }
}

// ── 全局单例 ──

use std::sync::OnceLock;

static GLOBAL_BUS: OnceLock<GlobalEventBus> = OnceLock::new();

/// 获取或初始化全局事件总线（默认容量 4096）
pub fn global_event_bus() -> &'static GlobalEventBus {
    GLOBAL_BUS.get_or_init(|| GlobalEventBus::new(4096))
}

/// 用指定容量初始化全局事件总线（仅首次调用有效）
pub fn init_global_event_bus(capacity: usize) -> &'static GlobalEventBus {
    GLOBAL_BUS.get_or_init(|| GlobalEventBus::new(capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_global_event_bus_basic() {
        let bus = GlobalEventBus::new(64);
        let mut rx = bus.subscribe();

        bus.emit(GlobalEngineEvent {
            source: "test_engine".into(),
            event_type: "test_event".into(),
            payload: serde_json::json!({"key": "value"}),
            correlation_id: None,
        })
        .expect("测试应成功");

        let event = rx.recv().await.expect("测试：异步操作应成功");
        assert_eq!(event.source, "test_engine");
        assert_eq!(event.event_type, "test_event");
    }

    #[tokio::test]
    async fn test_engine_event_subscription() {
        let bus = GlobalEventBus::new(64);
        let mut sub = EngineEventSubscription::new(bus.subscribe()).with_source("engine_a");

        bus.emit(GlobalEngineEvent {
            source: "engine_b".into(),
            event_type: "ignored".into(),
            payload: serde_json::json!({}),
            correlation_id: None,
        })
        .expect("测试应成功");

        bus.emit(GlobalEngineEvent {
            source: "engine_a".into(),
            event_type: "processed".into(),
            payload: serde_json::json!({}),
            correlation_id: None,
        })
        .expect("测试应成功");

        let event = sub.recv().await.expect("测试：异步操作应成功");
        assert_eq!(event.source, "engine_a");
        assert_eq!(event.event_type, "processed");
    }

    #[test]
    fn test_global_singleton() {
        let _ = init_global_event_bus(128);
        let bus = global_event_bus();
        assert!(bus.subscriber_count() == 0);
    }
}
