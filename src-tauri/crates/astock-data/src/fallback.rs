//! 多级降级链配置

use crate::error::DataError;
use crate::types::StockQuote;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallbackStep {
    VendorQuote {
        vendor: String,
        timeout_ms: u64,
    },
    KlinesSynthesize {
        vendor: String,
        period: String,
        limit: u32,
    },
    FillFinancials {
        vendor: String,
    },
    Fail {
        reason: String,
    },
}

impl FallbackStep {
    pub fn vendor(v: impl Into<String>) -> Self {
        Self::VendorQuote {
            vendor: v.into(),
            timeout_ms: 8000,
        }
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
        Self {
            name: name.into(),
            steps: Vec::new(),
            total_timeout_ms: 30000,
        }
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
        Self::new("a_share_quote")
            .then(FallbackStep::vendor("tencent"))
            .then(FallbackStep::vendor("sina"))
            .then(FallbackStep::vendor("eastmoney"))
            .then(FallbackStep::vendor("xueqiu"))
            .then(FallbackStep::vendor("mootdx"))
            .with_total_timeout(20_000)
    }

    pub fn a_share_klines() -> Self {
        Self::new("a_share_klines")
            .then(FallbackStep::vendor("tencent"))
            .then(FallbackStep::vendor("sina"))
            .then(FallbackStep::vendor("eastmoney"))
            .then(FallbackStep::vendor("xueqiu"))
            .with_total_timeout(25_000)
    }

    pub fn a_share_financials() -> Self {
        Self::new("a_share_financials")
            .then(FallbackStep::VendorQuote {
                vendor: "tencent".into(),
                timeout_ms: 5000,
            })
            .then(FallbackStep::FillFinancials {
                vendor: "eastmoney".into(),
            })
            .then(FallbackStep::FillFinancials {
                vendor: "xueqiu".into(),
            })
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
            FallbackStep::VendorQuote {
                vendor: _,
                timeout_ms,
            } => {
                let _ = timeout_ms;
                match self.client.get_quote(code).await {
                    Ok(q) => {
                        if q.price > 0.0 && !q.name.is_empty() {
                            Ok(Some(q))
                        } else {
                            Ok(None)
                        }
                    },
                    Err(_) => Ok(None),
                }
            },
            FallbackStep::KlinesSynthesize {
                vendor,
                period,
                limit,
            } => {
                let _ = (vendor, period, limit);
                Ok(None)
            },
            FallbackStep::FillFinancials { vendor } => {
                let _ = vendor;
                Ok(None)
            },
            FallbackStep::Fail { reason } => Err(DataError::VendorError {
                vendor: "chain".into(),
                message: reason.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_a_share_quote_has_5_steps() {
        let c = FallbackChain::a_share_quote();
        assert_eq!(c.name, "a_share_quote");
        assert_eq!(c.steps.len(), 5);
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
        assert!(c
            .steps
            .iter()
            .any(|s| matches!(s, FallbackStep::FillFinancials { .. })));
    }

    #[test]
    fn fail_step_serializes() {
        let step = FallbackStep::Fail {
            reason: "test".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"type\":\"fail\""));
        assert!(json.contains("\"reason\":\"test\""));
    }
}
