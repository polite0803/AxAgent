// SPDX-License-Identifier: AGPL-3.0-only

//! 类型化事件派发总线 —— P2 事件化（缺陷 #3）地基本体。
//!
//! 在统一广播总线（`event_bus.rs`）之上，新增**有裁决语义**的派发机制：
//! 订阅者按 [`DispatchMode`] 被调用，可阅读 / 改写 / 拒绝事件，支撑把策略、
//! 审批、遥测等横切逻辑剥离为独立插件（经能力注册表 `event.dispatch` 接缝挂载）。
//!
//! 与 `event_bus.rs`（广播 / stream 订阅）的区别：本模块是**命令式派发**，
//! 订阅者以 `Arc<dyn EventSubscriber>` 回调形式注册，可对事件做改写 / 拒绝，
//! 而非被动接收拷贝。二者互补：广播用于"观测"，派发用于"拦截/改写"。
//!
//! ## 四派发模式
//!
//! | 模式 | 语义 | 用途 |
//! |------|------|------|
//! | [`DispatchMode::Emit`] | 广播，订阅者只读 | 遥测 / 审计 |
//! | [`DispatchMode::Waterfall`] | 按注册顺序，订阅者可改写 payload 或拒绝（中断） | 策略 / 审批（`agent/request`） |
//! | [`DispatchMode::Parallel`] | 订阅者并发调用（只读） | 并行观测 |
//! | [`DispatchMode::Serial`] | 按注册顺序调用（只读） | 顺序观测 |
//!
//! harness 为 foundation 层，禁止依赖任何 axagent-* crate；事件统一用
//! [`DomainEvent`]（category + kind + payload 三元组），订阅者经
//! [`EventMatcher`] 按 category / kind 过滤。

#![allow(clippy::disallowed_types)]

use crate::EffectScope;
use crate::event_bus::{DomainEvent, EventCategory};
use crate::reversible_effect::EffectHandle;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// 派发模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchMode {
    /// 广播：订阅者只读，互不影响。
    Emit,
    /// 瀑布：按注册顺序调用，订阅者可改写 payload 或拒绝（中断后续）。
    Waterfall,
    /// 并行：订阅者并发调用（只读），全部完成后汇总。
    Parallel,
    /// 串行：按注册顺序调用（只读）。
    Serial,
}

/// 订阅者裁决。
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriberVerdict {
    /// 继续，不改写。
    Continue,
    /// 改写 payload 后继续（仅 [`DispatchMode::Waterfall`] 生效）。
    Rewrite(Value),
    /// 拒绝并中断处理链（仅 [`DispatchMode::Waterfall`] 生效）。
    Reject,
}

/// 事件匹配器：按来源分类 + 类型字符串过滤订阅者。
///
/// `None` 表示不限制该维度（匹配全部）。
#[derive(Debug, Clone)]
pub struct EventMatcher {
    /// 事件来源分类；`None` 匹配所有分类。
    pub category: Option<EventCategory>,
    /// 事件类型字符串（如 `"agent/request"`、`"llm/stream"`）；`None` 匹配所有类型。
    pub kind: Option<String>,
}

impl EventMatcher {
    /// 匹配所有事件。
    pub fn any() -> Self {
        Self { category: None, kind: None }
    }

    /// 仅匹配某来源分类。
    pub fn category(category: EventCategory) -> Self {
        Self { category: Some(category), kind: None }
    }

    /// 仅匹配某事件类型字符串。
    pub fn kind(kind: impl Into<String>) -> Self {
        Self { category: None, kind: Some(kind.into()) }
    }

    /// 判断事件是否匹配本订阅过滤器。
    pub fn matches(&self, event: &DomainEvent) -> bool {
        if let Some(c) = self.category
            && c != event.category
        {
            return false;
        }
        if let Some(k) = &self.kind
            && *k != event.kind
        {
            return false;
        }
        true
    }
}

/// 类型化事件订阅者。
///
/// 事件以只读 `&DomainEvent` 传入（支持并发共享）；需要改写 / 拒绝时
/// 通过 [`SubscriberVerdict`] 返回值声明，由总线统一应用。
#[async_trait]
pub trait EventSubscriber: Send + Sync + 'static {
    /// 处理一个事件并返回裁决。
    async fn handle(&self, event: &DomainEvent) -> SubscriberVerdict;
}

/// 一次派发的汇总结果。
#[derive(Debug, Clone, Default)]
pub struct DispatchOutcome {
    /// Waterfall 模式下被订阅者改写的最终 payload（未改写则为 `None`）。
    pub rewritten: Option<Value>,
    /// 是否被订阅者拒绝（Waterfall 中断）。
    pub rejected: bool,
    /// 实际参与回调的订阅者数。
    pub invoked: usize,
}

struct Subscription {
    id: u64,
    matcher: EventMatcher,
    subscriber: Arc<dyn EventSubscriber>,
}

struct Inner {
    next_id: u64,
    subscriptions: Vec<Subscription>,
}

/// 类型化事件派发总线。
///
/// 维护订阅者集合，支持四派发模式；订阅返回可撤销 [`EffectHandle`]，
/// 撤销后不再参与后续派发（供插件挂载 / 卸载热插拔）。
#[derive(Clone)]
pub struct EventDispatchBus {
    inner: Arc<RwLock<Inner>>,
    effects: EffectScope,
}

impl Default for EventDispatchBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatchBus {
    /// 构造一个空总线。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner { next_id: 0, subscriptions: Vec::new() })),
            effects: EffectScope::new(),
        }
    }

    /// 注册一个订阅者，返回可撤销句柄。
    ///
    /// 撤销后该订阅者即从总线移除，不再收到后续事件。
    pub fn subscribe(
        &self,
        matcher: EventMatcher,
        subscriber: Arc<dyn EventSubscriber>,
    ) -> EffectHandle {
        let id = {
            let mut g = self.inner.write().unwrap_or_else(|e| e.into_inner());
            let id = g.next_id;
            g.next_id += 1;
            g.subscriptions.push(Subscription { id, matcher, subscriber });
            id
        };
        let inner = self.inner.clone();
        self.effects.register(format!("event-subscriber:{id}"), move || {
            let mut g = inner.write().unwrap_or_else(|e| e.into_inner());
            g.subscriptions.retain(|s| s.id != id);
        })
    }

    /// 当前订阅者数量（含未匹配当前事件的）。
    pub fn subscriber_count(&self) -> usize {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).subscriptions.len()
    }

    /// 是否有订阅者可能匹配该事件（热路径读取前的廉价短路）。
    ///
    /// 用于流式路径：构造/序列化 payload 前先判断有没有匹配订阅者，
    /// 避免无订阅时仍为每个 chunk 做 JSON 序列化的开销。
    pub fn would_dispatch(&self, event: &DomainEvent) -> bool {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .subscriptions
            .iter()
            .any(|s| s.matcher.matches(event))
    }

    /// 按派发模式派发一个事件。
    ///
    /// 仅匹配的订阅者参与回调；无匹配订阅者时优雅返回（不 panic）。
    pub async fn dispatch(&self, event: &mut DomainEvent, mode: DispatchMode) -> DispatchOutcome {
        let subs: Vec<Arc<dyn EventSubscriber>> = {
            let g = self.inner.read().unwrap_or_else(|e| e.into_inner());
            g.subscriptions
                .iter()
                .filter(|s| s.matcher.matches(event))
                .map(|s| s.subscriber.clone())
                .collect()
        };
        let mut outcome = DispatchOutcome::default();
        match mode {
            DispatchMode::Emit | DispatchMode::Serial => {
                for s in &subs {
                    s.handle(event).await;
                    outcome.invoked += 1;
                }
            },
            DispatchMode::Waterfall => {
                for s in &subs {
                    match s.handle(event).await {
                        SubscriberVerdict::Continue => {},
                        SubscriberVerdict::Rewrite(payload) => {
                            event.payload = payload;
                            outcome.rewritten = Some(event.payload.clone());
                        },
                        SubscriberVerdict::Reject => {
                            outcome.rejected = true;
                            outcome.invoked += 1;
                            break;
                        },
                    }
                    outcome.invoked += 1;
                }
            },
            DispatchMode::Parallel => {
                let event_ref: &DomainEvent = event;
                let results =
                    futures::future::join_all(subs.iter().map(|s| s.handle(event_ref))).await;
                outcome.invoked = results.len();
            },
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatContent;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 测试订阅者：按返回策略响应事件。
    struct VerdictSub {
        verdict: SubscriberVerdict,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventSubscriber for VerdictSub {
        async fn handle(&self, _event: &DomainEvent) -> SubscriberVerdict {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.verdict.clone()
        }
    }

    fn event() -> DomainEvent {
        DomainEvent::new(EventCategory::Agent, "agent/request", serde_json::json!({"q": 1}), "test")
    }

    fn sub(verdict: SubscriberVerdict) -> Arc<dyn EventSubscriber> {
        Arc::new(VerdictSub { verdict, calls: Arc::new(AtomicUsize::new(0)) })
    }

    #[tokio::test]
    async fn emit_is_readonly_and_invokes_all() {
        let bus = EventDispatchBus::new();
        bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        let mut e = event();
        let outcome = bus.dispatch(&mut e, DispatchMode::Emit).await;
        assert_eq!(outcome.invoked, 1);
        assert!(!outcome.rejected);
        assert!(outcome.rewritten.is_none());
    }

    #[tokio::test]
    async fn waterfall_rejects_and_stops() {
        let bus = EventDispatchBus::new();
        let _h1 = bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        let _h2 = bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Reject));
        let _h3 = bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        let mut e = event();
        let outcome = bus.dispatch(&mut e, DispatchMode::Waterfall).await;
        assert!(outcome.rejected);
        assert_eq!(outcome.invoked, 2, "第二个订阅者拒绝后中断，第三个不再被调用");
    }

    #[tokio::test]
    async fn waterfall_rewrites_payload() {
        let bus = EventDispatchBus::new();
        let _h1 = bus.subscribe(
            EventMatcher::any(),
            sub(SubscriberVerdict::Rewrite(serde_json::json!({"q": 42}))),
        );
        let mut e = event();
        let outcome = bus.dispatch(&mut e, DispatchMode::Waterfall).await;
        assert_eq!(outcome.rewritten.as_ref(), Some(&serde_json::json!({"q": 42})));
        assert_eq!(e.payload, serde_json::json!({"q": 42}));
    }

    #[tokio::test]
    async fn matcher_filters_by_kind() {
        let bus = EventDispatchBus::new();
        let _h = bus.subscribe(EventMatcher::kind("llm/stream"), sub(SubscriberVerdict::Continue));
        let mut e = event(); // kind = "agent/request"
        let outcome = bus.dispatch(&mut e, DispatchMode::Emit).await;
        assert_eq!(outcome.invoked, 0, "kind 不匹配的订阅者不应触发");
    }

    #[tokio::test]
    async fn would_dispatch_short_circuits_by_matcher() {
        let bus = EventDispatchBus::new();
        // 无订阅 → 假
        assert!(!bus.would_dispatch(&event()));
        // 仅匹配 llm/stream → agent/request 事件不应触发派发
        let _h = bus.subscribe(EventMatcher::kind("llm/stream"), sub(SubscriberVerdict::Continue));
        assert!(!bus.would_dispatch(&event()), "kind 不匹配不应触发");
        // 补一个匹配 agent/request 的订阅者 → 真
        let _h2 =
            bus.subscribe(EventMatcher::kind("agent/request"), sub(SubscriberVerdict::Continue));
        assert!(bus.would_dispatch(&event()));
        // llm/stream 事件仍应匹配（订阅了 llm/stream）
        let llm =
            DomainEvent::new(EventCategory::Agent, "llm/stream", serde_json::json!({}), "agent");
        assert!(bus.would_dispatch(&llm));
    }

    #[tokio::test]
    async fn parallel_invokes_all_concurrently() {
        let bus = EventDispatchBus::new();
        bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        let mut e = event();
        let outcome = bus.dispatch(&mut e, DispatchMode::Parallel).await;
        assert_eq!(outcome.invoked, 3);
    }

    #[tokio::test]
    async fn unsubscribe_removes_subscriber() {
        let bus = EventDispatchBus::new();
        let handle = bus.subscribe(EventMatcher::any(), sub(SubscriberVerdict::Continue));
        assert_eq!(bus.subscriber_count(), 1);
        handle.undo();
        assert_eq!(bus.subscriber_count(), 0);
        let mut e = event();
        let outcome = bus.dispatch(&mut e, DispatchMode::Emit).await;
        assert_eq!(outcome.invoked, 0);
    }

    /// 端到端模拟 provider_adapter 的 agent/request 瀑布拦截：
    /// 订阅者改写 ChatRequest（在最后一条消息追加合规 prompt），
    /// 反序列化回 ChatRequest 后验证改写生效。
    #[tokio::test]
    async fn agent_request_waterfall_rewrites_chat_request() {
        use crate::types::{ChatMessage, ChatRequest};

        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("帮我写一份合同".into()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: true,
            ..Default::default()
        };

        // 订阅者：把合规声明追加到最后一条用户消息
        let bus = EventDispatchBus::new();
        let _h = bus.subscribe(EventMatcher::kind("agent/request"), Arc::new(ComplianceSub));

        // 复刻 provider_adapter 的派发流程
        let mut event = DomainEvent::new(
            EventCategory::Agent,
            "agent/request",
            serde_json::to_value(&req).unwrap(),
            "agent",
        );
        let outcome = bus.dispatch(&mut event, DispatchMode::Waterfall).await;
        assert!(!outcome.rejected, "改写订阅者不应拒绝请求");

        let rewritten: ChatRequest = serde_json::from_value(outcome.rewritten.unwrap()).unwrap();
        let last = rewritten.messages.last().unwrap();
        match &last.content {
            ChatContent::Text(t) => {
                assert!(t.contains("【合规声明】"), "改写后应追加合规声明: {t}")
            },
            ChatContent::Multipart(_) => panic!("文本消息不应被改写成 multipart"),
        }
    }

    /// 合规订阅者：给最后一条消息追加合规声明。
    struct ComplianceSub;

    #[async_trait]
    impl EventSubscriber for ComplianceSub {
        async fn handle(&self, event: &DomainEvent) -> SubscriberVerdict {
            let mut req: crate::types::ChatRequest =
                serde_json::from_value(event.payload.clone()).unwrap();
            if let Some(last) = req.messages.last_mut()
                && let ChatContent::Text(t) = &mut last.content
            {
                t.push_str("\n【合规声明】本回复仅用于合法用途。");
            }
            SubscriberVerdict::Rewrite(serde_json::to_value(req).unwrap())
        }
    }
}
