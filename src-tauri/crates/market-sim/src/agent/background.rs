//! 后台 Agent —— 持续运行的监听型 Agent，不主动交易但收集市场数据。
//!
//! BackgroundAgent 模拟"始终在线"的监控进程，与事件驱动的 Agent 不同：
//! - 无需显式 WakeupAfter，框架确保其定期获知市场状态
//! - 用于模拟数据推送服务、风控监控、实时计算等基础设施

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::*;

/// 后台监控 Agent
pub struct BackgroundAgent {
    id: String,
    /// 监控间隔（ns）
    interval_ns: SimTimestamp,
    /// 观察到的报价更新次数
    quote_updates: u64,
}

impl BackgroundAgent {
    /// 创建后台监控 Agent
    pub fn new(id: impl Into<String>, interval_ns: SimTimestamp) -> Self {
        Self { id: id.into(), interval_ns, quote_updates: 0 }
    }
}

impl SimAgent for BackgroundAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Background"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Custom("background".into())
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.interval_ns),
        ]
    }

    fn on_message(&mut self, msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::QuoteReply(_) => {
                self.quote_updates += 1;
                Vec::new()
            },
            _ => Vec::new(),
        }
    }

    fn on_wakeup(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.interval_ns),
        ]
    }

    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }
}
