//! 模拟 Agent 系统 — SimAgent trait + AgentContext + 消息类型。
//!
//! ## 设计原则
//!
//! - Agent 之间**只通过消息通信**，不直接调用对方的方法（ABIDES 风格）
//! - Agent 通过 `AgentContext` 与 Kernel 交互（发消息、设唤醒定时器）
//! - Agent **不能**直接访问订单簿——必须向 ExchangeAgent 发消息
//! - Phase 2 中 Agent trait 为同步接口（纯算法，无 I/O）

use crate::types::*;
use serde::{Deserialize, Serialize};

// ── Agent 类型 ──

/// Agent 类别标记
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// 交易所 Agent（维护订单簿，撮合交易）
    Exchange,
    /// 做市商（双边报价，赚取价差）
    MarketMaker,
    /// 动量交易者（追涨杀跌）
    Momentum,
    /// 价值投资者（基于基本面估值交易）
    Value,
    /// 噪声交易者（随机下单，无信息含量）
    Noise,
    /// 机构投资者（大单拆分，TWAP/VWAP 执行）
    Institutional,
    /// Rhai 脚本定义的 Agent（用户自定义行为）
    Rhai,
    /// 用户主策略 Agent（AxInvest 策略的模拟接入点）
    Strategy,
    /// 事件驱动 Agent（P2-C9：基于新闻/公告/财报事件触发交易）
    EventDriven,
    /// 自定义类型
    Custom(String),
}

impl AgentType {
    pub fn as_str(&self) -> &str {
        match self {
            AgentType::Exchange => "exchange",
            AgentType::MarketMaker => "market_maker",
            AgentType::Momentum => "momentum",
            AgentType::Value => "value",
            AgentType::Noise => "noise",
            AgentType::Institutional => "institutional",
            AgentType::Rhai => "rhai",
            AgentType::Strategy => "strategy",
            AgentType::EventDriven => "event_driven",
            AgentType::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── 消息类型 ──

/// Agent 间消息信封
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// 消息来源 Agent ID
    pub source: String,
    /// 消息目标 Agent ID
    pub target: String,
    /// 消息发送时刻（模拟时间戳）
    pub sent_at: SimTimestamp,
    /// 消息体
    pub body: MessageBody,
}

impl AgentMessage {
    pub fn new(source: String, target: String, sent_at: SimTimestamp, body: MessageBody) -> Self {
        Self { source, target, sent_at, body }
    }
}

/// 消息体 —— Agent 之间传递的业务语义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageBody {
    // ── 订单操作（→ ExchangeAgent） ──
    /// 提交限价单
    SubmitLimit(LimitOrder),
    /// 提交市价单
    SubmitMarket(MarketOrder),
    /// 撤单
    CancelOrder(OrderId),

    // ── 成交通知（← ExchangeAgent） ──
    /// 订单成交通知
    OrderFilled { order_id: OrderId, fill: FillResult },
    /// 撤单确认
    OrderCancelled { order_id: OrderId, remaining: Quantity },
    /// 限价单已挂单确认
    ///
    /// 修复 P0-6: 增加 `side` 字段，让做市商等 Agent 能区分挂单方向，
    /// 从而正确更新 bid_order_id / ask_order_id。
    OrderPlaced { order_id: OrderId, side: OrderSide },

    // ── 行情查询 ──
    /// 请求订单簿快照（→ ExchangeAgent）
    RequestQuote,
    /// 订单簿快照回复（← ExchangeAgent）
    QuoteReply(BookSnapshot),

    // ── 外部事件（P2-C9：Kernel → 广播给所有 Agent） ──
    /// 外部事件通知（新闻/公告/财报/市场冲击）
    ///
    /// 由 `SimKernel::inject_event` 注入，Kernel 按事件 scheduled_at
    /// 广播给所有已注册 Agent。EventDrivenAgent 收到后根据事件情感
    /// 和影响强度决定交易行为。
    ExternalEvent(crate::events::ExternalEvent),

    // ── 系统消息 ──
    /// 定时唤醒（Kernel → Agent 自唤醒）
    Wakeup,
    /// 模拟结束通知
    SimEnd,
}

// ── Agent 动作 ──

/// Agent 在处理消息后返回的动作列表
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// 向另一个 Agent 发送消息
    SendMessage { target: String, body: MessageBody },
    /// 延迟后唤醒自己（延迟相对于当前模拟时间）
    WakeupAfter(SimTimestamp),
    /// 向多个 Agent 广播同一消息
    Broadcast { targets: Vec<String>, body: MessageBody },
}

// ── Agent 上下文 ──

/// Kernel 提供给 Agent 的运行时上下文
///
/// Agent 通过此结构感知当前模拟时间、发送消息、设置定时器。
/// Agent 不能直接访问订单簿或其他 Agent 的内部状态。
pub struct AgentContext {
    /// 当前模拟时间
    pub current_time: SimTimestamp,
    /// 股票代码
    pub stock_code: String,
    /// 参考价格（分）
    pub reference_price: Price,
    /// Agent 自身 ID（即消息目标 Agent）
    agent_id: String,
    /// 修复 P0-5: 当前消息的来源 Agent ID。
    ///
    /// Kernel 在 `make_ctx` 时从 `event.message.source` 填充。
    /// 对于 on_init / on_sim_end 等非消息触发场景为 None。
    ///
    /// ExchangeAgent 处理 CancelOrder / RequestQuote 等不含来源 ID 的消息时，
    /// 需通过此字段获取通知目标（提交请求的 Agent），避免把通知发给自己。
    message_source: Option<String>,
    /// 动作缓冲区（Agent 的返回值通过此 Vec 传递给 Kernel）
    actions: Vec<AgentAction>,
}

impl AgentContext {
    pub fn new(
        current_time: SimTimestamp,
        stock_code: String,
        reference_price: Price,
        agent_id: String,
        message_source: Option<String>,
    ) -> Self {
        Self {
            current_time,
            stock_code,
            reference_price,
            agent_id,
            message_source,
            actions: Vec::new(),
        }
    }

    /// 向目标 Agent 发送消息
    pub fn send(&mut self, target: &str, body: MessageBody) {
        self.actions.push(AgentAction::SendMessage { target: target.to_string(), body });
    }

    /// 延迟后唤醒自己（ns）
    pub fn wakeup_after(&mut self, delay_ns: SimTimestamp) {
        self.actions.push(AgentAction::WakeupAfter(delay_ns));
    }

    /// 向多个 Agent 广播
    pub fn broadcast(&mut self, targets: Vec<String>, body: MessageBody) {
        self.actions.push(AgentAction::Broadcast { targets, body });
    }

    /// 获取 Agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// 修复 P0-5: 获取当前消息的来源 Agent ID。
    ///
    /// 用于 ExchangeAgent 处理 CancelOrder / RequestQuote 等不含来源 ID 的消息时，
    /// 确定通知目标（即提交请求的 Agent）。
    pub fn message_source(&self) -> Option<&str> {
        self.message_source.as_deref()
    }

    /// 消费累积的动作列表（由 Kernel 调用）
    pub fn drain_actions(&mut self) -> Vec<AgentAction> {
        std::mem::take(&mut self.actions)
    }
}

// ── Agent Trait ──

/// 模拟 Agent 接口
///
/// 所有市场参与者（交易所、做市商、策略等）实现此 trait。
/// Phase 2 为同步接口——Agent 行为是纯算法，无网络 I/O。
///
/// # 生命周期
///
/// 1. `on_init(ctx)` — 模拟开始前初始化
/// 2. `on_message(msg, ctx)` — 收到消息时处理
/// 3. `on_wakeup(ctx)` — 定时器到期时处理
/// 4. `on_sim_end(ctx)` — 模拟结束时清理
pub trait SimAgent: Send + Sync {
    /// Agent 唯一标识
    fn id(&self) -> &str;

    /// Agent 显示名
    fn name(&self) -> &str {
        self.id()
    }

    /// Agent 类型
    fn agent_type(&self) -> AgentType;

    /// 该 Agent 在模拟中产生的成交历史（默认空；策略类 Agent 可重写以暴露模拟成交）
    fn trade_history(&self) -> &[TradeRecord] {
        &[]
    }

    /// 交易所统计（仅 ExchangeAgent 重写）
    ///
    /// 返回 (total_orders, total_trades, total_volume)。
    /// 其他 Agent 默认返回 (0, 0, 0)。
    /// 修复 C4.2: 之前 Kernel.collect_results 硬编码返回 (0, 0, 0)，
    /// 导致 SimStats 永远是 0。改为通过 trait 暴露真实统计。
    fn exchange_stats(&self) -> (u64, u64, Quantity) {
        (0, 0, 0)
    }

    /// 模拟结束时的最终中间价（仅 ExchangeAgent 重写）
    ///
    /// 修复 M-RES-12: 原 Kernel.collect_results 用所有 trade 的均价近似 final_mid_price，
    /// 这不是真正的 mid price。改为通过 trait 让 ExchangeAgent 暴露 OrderBook 的
    /// best_bid + best_ask 均值。其他 Agent 默认返回 None。
    fn final_mid_price(&self) -> Option<f64> {
        None
    }

    /// 模拟初始化（在第一个事件之前调用一次）
    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }

    /// 收到消息时回调
    fn on_message(&mut self, msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction>;

    /// 定时唤醒回调（经由 `WakeupAfter` 触发）
    fn on_wakeup(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }

    /// 模拟结束通知
    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }
}
