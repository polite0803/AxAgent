//! 多级降级链配置

use crate::error::DataError;
use crate::types::StockQuote;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallbackStep {
    VendorQuote { vendor: String, timeout_ms: u64 },
    KlinesSynthesize { vendor: String, period: String, limit: u32 },
    FillFinancials { vendor: String },
    Fail { reason: String },
}

impl FallbackStep {
    pub fn vendor(v: impl Into<String>) -> Self {
        Self::VendorQuote { vendor: v.into(), timeout_ms: 8000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackChain {
    pub name: String,
    pub steps: Vec<FallbackStep>,
    pub total_timeout_ms: u64,
}

impl FallbackChain {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), steps: Vec::new(), total_timeout_ms: 30000 }
    }

    pub fn then(mut self, step: FallbackStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_total_timeout(mut self, ms: u64) -> Self {
        self.total_timeout_ms = ms;
        self
    }

    pub fn a_share_quote() -> Self {
        // 与 lib.rs default_routing().quote 保持一致
        Self::new("a_share_quote")
            .then(FallbackStep::vendor("tencent"))
            .then(FallbackStep::vendor("mootdx"))
            .then(FallbackStep::vendor("sina"))
            .then(FallbackStep::vendor("xueqiu"))
            .then(FallbackStep::vendor("eastmoney"))
            .then(FallbackStep::vendor("neodata"))
            .with_total_timeout(20_000)
    }

    pub fn a_share_klines() -> Self {
        // 与 lib.rs default_routing().klines 保持一致
        Self::new("a_share_klines")
            .then(FallbackStep::vendor("tencent"))
            .then(FallbackStep::vendor("xueqiu"))
            .then(FallbackStep::vendor("mootdx"))
            .then(FallbackStep::vendor("eastmoney"))
            .then(FallbackStep::vendor("browser_eastmoney"))
            .with_total_timeout(25_000)
    }

    pub fn a_share_financials() -> Self {
        // 与 lib.rs default_routing().financials 保持一致
        Self::new("a_share_financials")
            .then(FallbackStep::FillFinancials { vendor: "eastmoney".into() })
            .then(FallbackStep::FillFinancials { vendor: "browser_eastmoney".into() })
            .then(FallbackStep::FillFinancials { vendor: "xueqiu".into() })
            .then(FallbackStep::FillFinancials { vendor: "akshare".into() })
            .then(FallbackStep::FillFinancials { vendor: "neodata".into() })
            .with_total_timeout(30_000)
    }
}

pub struct ChainExecutor {
    pub client: Arc<crate::AStockClient>,
}

impl ChainExecutor {
    pub fn new(client: Arc<crate::AStockClient>) -> Self {
        Self { client }
    }

    pub async fn execute_quote_chain(
        &self,
        code: &str,
        chain: &FallbackChain,
    ) -> Result<StockQuote, DataError> {
        use std::time::Duration;
        let total = Duration::from_millis(chain.total_timeout_ms);
        let inner = async {
            for step in &chain.steps {
                match self.try_step_quote(code, step).await {
                    Ok(Some(q)) => return Ok(q),
                    Ok(None) => {
                        tracing::debug!("[chain:{}] 步骤 {:?} 跳过,尝试下一源", chain.name, step);
                    },
                    Err(e) => {
                        tracing::warn!("[chain:{}] 步骤失败: {}", chain.name, e);
                    },
                }
            }
            Err(DataError::VendorError {
                vendor: "chain".into(),
                message: format!("链 {} 全部失败", chain.name),
            })
        };
        match tokio::time::timeout(total, inner).await {
            Ok(result) => result,
            Err(_) => Err(DataError::VendorError {
                vendor: "chain".into(),
                message: format!("链 {} 总超时", chain.name),
            }),
        }
    }

    async fn try_step_quote(
        &self,
        code: &str,
        step: &FallbackStep,
    ) -> Result<Option<StockQuote>, DataError> {
        match step {
            FallbackStep::VendorQuote { vendor: _, timeout_ms } => {
                // 修复 M-RES-3: 原 timeout_ms 字段未使用，导致单步无超时控制。
                // 现在用 tokio::time::timeout 包装 vendor.get_quote，
                // 超时则记 warn 并传播 VendorError。
                let timeout_dur = std::time::Duration::from_millis(*timeout_ms);
                match tokio::time::timeout(timeout_dur, self.client.get_quote(code)).await {
                    Ok(Ok(q)) => {
                        if q.price > 0.0 && !q.name.is_empty() {
                            Ok(Some(q))
                        } else {
                            Ok(None)
                        }
                    },
                    // 修复 P0-A3: 原 `Err(_) => Ok(None)` 把 401/DNS/panic 全部吞为
                    // "OK 但空"，回退链调试黑洞。改为显式传播错误。
                    Ok(Err(e)) => Err(e),
                    Err(_) => {
                        tracing::warn!(
                            "[chain] vendor get_quote 超时 (code={}, timeout_ms={})",
                            code,
                            timeout_ms
                        );
                        Err(DataError::VendorError {
                            vendor: "chain".into(),
                            message: format!("vendor get_quote 超时 (timeout_ms={})", timeout_ms),
                        })
                    },
                }
            },
            FallbackStep::KlinesSynthesize { vendor, period, limit } => {
                let _ = (vendor, period, limit);
                Ok(None)
            },
            FallbackStep::FillFinancials { vendor } => {
                let _ = vendor;
                Ok(None)
            },
            FallbackStep::Fail { reason } => {
                Err(DataError::VendorError { vendor: "chain".into(), message: reason.clone() })
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_a_share_quote_has_6_steps() {
        let c = FallbackChain::a_share_quote();
        assert_eq!(c.name, "a_share_quote");
        assert_eq!(c.steps.len(), 6);
        assert_eq!(c.total_timeout_ms, 20_000);
        match &c.steps[0] {
            FallbackStep::VendorQuote { vendor, .. } => assert_eq!(vendor, "tencent"),
            _ => panic!("第一个 step 应该是 VendorQuote"),
        }
    }

    #[test]
    fn chain_builder_then_returns_self() {
        let c = FallbackChain::new("test")
            .then(FallbackStep::vendor("a"))
            .then(FallbackStep::vendor("b"));
        assert_eq!(c.steps.len(), 2);
    }

    #[test]
    fn chain_financials_uses_fill_step() {
        let c = FallbackChain::a_share_financials();
        assert!(c.steps.iter().any(|s| matches!(s, FallbackStep::FillFinancials { .. })));
    }

    #[test]
    fn fail_step_serializes() {
        let step = FallbackStep::Fail { reason: "test".into() };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"type\":\"fail\""));
        assert!(json.contains("\"reason\":\"test\""));
    }
}
