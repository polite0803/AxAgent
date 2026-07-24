// SPDX-License-Identifier: AGPL-3.0-only

//! `BroadcastEventBus` —— 基于 `tokio::sync::broadcast` 的统一事件总线默认实现。
//!
//! 作为 consumer 层(runtime-core)对 harness `EventBus` trait 的标准实现,
//! 供 wiring 层在初始化时实例化并注入到 agent / rt-workflow / orchestrator 三方。
//!
//! ## 行为特性
//!
//! - **多订阅者独立缓冲区**:每个 `subscribe()` 返回独立 `Receiver`,
//!   一方消费不影响其他订阅者,符合 broadcast 语义。
//! - **无订阅者优雅处理**:`publish` 在没有订阅者时静默丢弃事件,不 panic。
//! - **缓冲区满优雅处理**:订阅者 lag 时跳过事件,记 warn 日志,不 panic;
//!   其他订阅者不受影响。
//! - **关闭语义**:当所有 `Sender` 克隆被 drop 时,`recv()` 返回 `None`。
//!
//! ## 依赖方向
//!
//! runtime-core 是 consumer 层,仅依赖 `axagent-harness`(获取 trait 与 DTO)
//! 与 tokio broadcast;不依赖任何实现层 crate,符合铁律 2。

use async_trait::async_trait;
use axagent_harness::{DomainEvent, EventBus, EventBusSubscription};
use tokio::sync::broadcast;

// ── BroadcastEventBus ──────────────────────────────────────────────────

/// 基于 `tokio::sync::broadcast` 的统一事件总线实现。
///
/// 构造函数:`BroadcastEventBus::new(buffer_size)`,buffer_size 是每个订阅者
/// 的独立缓冲区容量。推荐 wiring 层使用 1024。
///
/// 该结构体可被克隆(`Clone`),所有克隆共享同一个底层 channel。
/// 但通常以 `Arc<dyn EventBus>` 形式注入到三方组件,无需克隆。
#[derive(Clone)]
pub struct BroadcastEventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl BroadcastEventBus {
    /// 创建一个新的事件总线,每个订阅者拥有 `buffer_size` 容量的独立缓冲区。
    ///
    /// `buffer_size` 为 0 会触发 tokio 内部 panic(不允许),
    /// 调用方应使用合理值(推荐 1024)。
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self { sender }
    }

    /// 返回当前活跃订阅者数量(主要用于测试与监控)。
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[async_trait]
impl EventBus for BroadcastEventBus {
    async fn publish(&self, event: DomainEvent) {
        // tokio broadcast::Sender::send 返回 Result<usize, SendError<DomainEvent>>:
        // - Ok(n): n 个订阅者收到事件(注意:即使某些订阅者 lag 也返回 Ok)
        // - Err(NoSubscribers): 无活跃订阅者,事件被丢弃
        // 两种情况均不 panic,符合"无订阅者优雅处理"要求。
        // 订阅者 lag 时不会让 send 失败,而是在订阅者后续 recv 时返回 Lagged。
        match self.sender.send(event) {
            Ok(_n) => {
                // 成功广播给 n 个订阅者(部分可能 lag,由订阅者自行处理)
            },
            Err(_) => {
                // 无订阅者,事件被丢弃,这是预期行为,trace 级日志即可
                tracing::trace!("BroadcastEventBus: 无订阅者,事件已丢弃");
            },
        }
    }

    async fn subscribe(&self) -> Box<dyn EventBusSubscription> {
        let receiver = self.sender.subscribe();
        Box::new(BroadcastSubscription { receiver })
    }
}

impl std::fmt::Debug for BroadcastEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroadcastEventBus")
            .field("receiver_count", &self.receiver_count())
            .finish_non_exhaustive()
    }
}

// ── BroadcastSubscription ──────────────────────────────────────────────

/// `BroadcastEventBus` 的订阅句柄,封装 `broadcast::Receiver<DomainEvent>`。
///
/// 每个实例拥有独立缓冲区,`recv()` 在订阅者 lag 时跳过丢失事件并继续等待,
/// 发送端全部断开时返回 `None`。
pub struct BroadcastSubscription {
    receiver: broadcast::Receiver<DomainEvent>,
}

#[async_trait]
impl EventBusSubscription for BroadcastSubscription {
    async fn recv(&mut self) -> Option<DomainEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    // 当前订阅者落后了 count 个事件,跳过它们继续等下一个
                    // 不 panic,不传播错误,仅 warn 日志
                    tracing::warn!(
                        lagged_count = count,
                        "BroadcastSubscription: 订阅者缓冲区已满,跳过丢失事件"
                    );
                    continue;
                },
                Err(broadcast::error::RecvError::Closed) => {
                    // 所有 Sender 已 drop,通道关闭
                    return None;
                },
            }
        }
    }
}

// ── 测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::EventCategory;

    #[tokio::test]
    async fn publish_without_subscribers_does_not_panic() {
        // 无订阅者时 publish 应静默返回,不 panic
        let bus = BroadcastEventBus::new(16);
        let event =
            DomainEvent::new(EventCategory::System, "test", serde_json::Value::Null, "test");
        bus.publish(event).await;
        assert_eq!(bus.receiver_count(), 0);
    }

    #[tokio::test]
    async fn single_subscriber_receives_event() {
        let bus = BroadcastEventBus::new(16);
        let mut sub = bus.subscribe().await;

        let event = DomainEvent::new(
            EventCategory::Agent,
            "TurnStarted",
            serde_json::json!({"iteration": 1}),
            "agent",
        );
        bus.publish(event.clone()).await;

        let received = sub.recv().await.expect("should receive event");
        assert_eq!(received.category, EventCategory::Agent);
        assert_eq!(received.kind, "TurnStarted");
        assert_eq!(received.source, "agent");
    }

    #[tokio::test]
    async fn multiple_subscribers_each_receive_independently() {
        let bus = BroadcastEventBus::new(16);
        let mut sub1 = bus.subscribe().await;
        let mut sub2 = bus.subscribe().await;

        let event = DomainEvent::new(
            EventCategory::Workflow,
            "NodeCompleted",
            serde_json::json!({"node_id": "n1"}),
            "rt-workflow",
        );
        bus.publish(event).await;

        let r1 = sub1.recv().await.expect("sub1 should receive");
        let r2 = sub2.recv().await.expect("sub2 should receive");
        assert_eq!(r1.kind, "NodeCompleted");
        assert_eq!(r2.kind, "NodeCompleted");
    }

    #[tokio::test]
    async fn returns_none_when_bus_dropped() {
        let bus = BroadcastEventBus::new(16);
        let mut sub = bus.subscribe().await;
        drop(bus);
        let result = sub.recv().await;
        assert!(result.is_none(), "recv should return None when bus dropped");
    }

    #[tokio::test]
    async fn dyn_event_bus_trait_object_works() {
        // 验证可作为 Arc<dyn EventBus> 注入到三方组件
        let bus: std::sync::Arc<dyn EventBus> = std::sync::Arc::new(BroadcastEventBus::new(16));
        let mut sub = bus.subscribe().await;
        bus.publish(DomainEvent::new(
            EventCategory::Orchestration,
            "DecompositionStarted",
            serde_json::Value::Null,
            "orchestrator",
        ))
        .await;
        let received = sub.recv().await.expect("should receive");
        assert_eq!(received.category, EventCategory::Orchestration);
        assert_eq!(received.source, "orchestrator");
    }
}
