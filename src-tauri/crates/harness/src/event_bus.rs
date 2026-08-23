// SPDX-License-Identifier: AGPL-3.0-only

//! 统一事件总线契约 —— harness foundation 层。
//!
//! 背景:agent / rt-workflow / orchestrator 三方各自维护独立的事件通道,
//! 跨 crate 桥接需手动转发。本模块定义统一的 `DomainEvent` 与 `EventBus`
//! trait,作为跨 crate 事件流的标准入口。
//!
//! ## 设计权衡
//!
//! - harness 是 foundation 层,**禁止依赖任何 axagent-* crate**(铁律 1)。
//!   因此 `DomainEvent` 不能直接引用 agent 的 `AgentEventType` 或
//!   rt-workflow 的事件枚举,改用 `category + kind + payload` 三元组:
//!   - `category`:粗粒度事件来源(agent / workflow / orchestration / system)
//!   - `kind`:细粒度事件类型字符串(如 `"TurnStarted"`、`"DecompositionStarted"`)
//!   - `payload`:`serde_json::Value`,三方各自把自己的事件结构序列化填入
//! - `EventBus` trait 使用 `#[async_trait]` 以支持 `Arc<dyn EventBus>` 动态分发,
//!   三方组件以 `Option<Arc<dyn EventBus>>` 字段注入,未注入时保持原有行为不变。
//! - `EventBusSubscription` 拆分为独立 trait,代表"一个订阅句柄",`recv()` 异步取事件。
//!
//! ## 三方桥接约定
//!
//! | 来源 crate       | category               | kind 来源                                   |
//! | ---------------- | ---------------------- | ------------------------------------------- |
//! | agent            | `EventCategory::Agent` | `AgentEventType::to_string()`              |
//! | rt-workflow      | `EventCategory::Workflow` | 触发器 `event_type` 字符串               |
//! | orchestrator     | `EventCategory::Orchestration` | `OrchestrationEvent` 变体名           |
//!
//! 三方"原有 event_bus 保留,统一总线是额外桥接":每次内部 emit 时,
//! 若 `event_bus` 字段非 None,额外 `publish` 一份 `DomainEvent` 到统一总线。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── DomainEvent ────────────────────────────────────────────────────────

/// 跨 crate 统一领域事件。
///
/// 三方各自把自己的事件结构序列化为 `payload`,用 `category + kind` 表征分类。
/// `trace_id` 可选,用于跨 crate 链路追踪。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainEvent {
    /// 事件来源分类(粗粒度)
    pub category: EventCategory,
    /// 事件类型字符串(细粒度,如 `"TurnStarted"`)
    pub kind: String,
    /// 事件负载(三方自定义结构序列化后的 JSON)
    pub payload: serde_json::Value,
    /// 事件产生时间(UTC)
    pub timestamp: DateTime<Utc>,
    /// 事件来源 crate 名(如 `"agent"`、`"rt-workflow"`、`"orchestrator"`)
    pub source: String,
    /// 可选链路追踪 ID
    pub trace_id: Option<String>,
}

impl DomainEvent {
    /// 创建一个新的 `DomainEvent`,自动填充当前 UTC 时间戳。
    pub fn new(
        category: EventCategory,
        kind: impl Into<String>,
        payload: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            category,
            kind: kind.into(),
            payload,
            timestamp: Utc::now(),
            source: source.into(),
            trace_id: None,
        }
    }

    /// 设置链路追踪 ID。
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

// ── EventCategory ──────────────────────────────────────────────────────

/// 领域事件来源分类。
///
/// 对应三方 crate + 系统级事件,用于订阅端按粗粒度过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// agent crate 的事件(AgentEventType 等)
    Agent,
    /// rt-workflow crate 的事件(触发器 / 节点执行等)
    Workflow,
    /// orchestrator crate 的事件(任务分解 / 状态迁移等)
    Orchestration,
    /// 系统级事件(初始化 / 关闭 / 跨 crate 桥接控制等)
    System,
}

impl EventCategory {
    /// 转为静态字符串,便于日志输出。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Workflow => "workflow",
            Self::Orchestration => "orchestration",
            Self::System => "system",
        }
    }
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── EventBusSubscription ───────────────────────────────────────────────

/// 事件订阅句柄。
///
/// 通过 `EventBus::subscribe()` 获取,每个订阅者拥有独立缓冲区,
/// 一方消费不影响其他订阅者。`recv()` 返回 `None` 表示发送端已关闭。
#[async_trait]
pub trait EventBusSubscription: Send {
    /// 异步等待并接收下一个事件;发送端全部断开时返回 `None`。
    async fn recv(&mut self) -> Option<DomainEvent>;
}

// ── EventBus ───────────────────────────────────────────────────────────

/// 统一事件总线 trait。
///
/// 实现方负责维护多订阅者的独立缓冲区与广播语义;
/// `publish` 在没有订阅者时应优雅返回(不 panic)。
///
/// 注:使用 `#[async_trait]` 而非 RPITIT(`-> impl Future`),
/// 以保证 trait 是 dyn-safe,可作 `Arc<dyn EventBus>` 注入到三方组件。
#[async_trait]
pub trait EventBus: Send + Sync + 'static {
    /// 发布一个领域事件到所有当前订阅者。
    ///
    /// 无订阅者 / 订阅者缓冲区满时均不 panic(由实现方优雅处理)。
    async fn publish(&self, event: DomainEvent);

    /// 创建一个新的订阅句柄。每个订阅者拥有独立的缓冲区与游标。
    async fn subscribe(&self) -> Box<dyn EventBusSubscription>;
}

// ── 测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_event_serialization_roundtrip() {
        let event = DomainEvent::new(
            EventCategory::Agent,
            "TurnStarted",
            serde_json::json!({"iteration": 1}),
            "agent",
        )
        .with_trace_id("trace-abc");

        let json = serde_json::to_string(&event).expect("serialize");
        let back: DomainEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.category, EventCategory::Agent);
        assert_eq!(back.kind, "TurnStarted");
        assert_eq!(back.source, "agent");
        assert_eq!(back.trace_id.as_deref(), Some("trace-abc"));
    }

    #[test]
    fn event_category_as_str_roundtrip() {
        assert_eq!(EventCategory::Agent.as_str(), "agent");
        assert_eq!(EventCategory::Workflow.as_str(), "workflow");
        assert_eq!(EventCategory::Orchestration.as_str(), "orchestration");
        assert_eq!(EventCategory::System.as_str(), "system");
    }

    #[test]
    fn event_category_serde_snake_case() {
        let json = serde_json::to_string(&EventCategory::Orchestration).expect("serialize");
        assert_eq!(json, "\"orchestration\"");
        let back: EventCategory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, EventCategory::Orchestration);
    }
}
