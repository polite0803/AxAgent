//! DomainGate — Per-Vendor 并发门控
//!
//! 限制同一供应商（域名）的并发请求数量，避免被限流。
//! 每个供应商有独立的信号量容量，获取许可证后等待直到有空位。
//! 许可证在 DomainGuard Drop 时自动释放。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// 支持的供应商及其并发上限
const CAPACITIES: &[(&str, usize)] = &[
    ("eastmoney", 3),   // 东方财富：最核心，但限流最严
    ("tencent", 5),     // 腾讯财经：稳定，可宽松
    ("baidu_stock", 3), // 百度股市通：有隐性限流
    ("ths", 4),         // 同花顺：适中
    ("akshare", 4),     // AKShare
    ("mootdx", 3),      // 通达信TCP：连接数不宜过多
    ("sina", 5),        // 新浪财经
    ("iwencai", 3),     // 问财：需 API Key
    ("cninfo", 3),      // 巨潮资讯
    ("neodata", 3),     // NeoData API
];

/// Per-Vendor 并发门控
pub struct DomainGate {
    gates: HashMap<&'static str, Arc<Semaphore>>,
    default: Arc<Semaphore>,
}

/// 门控许可证 — Drop 时自动归还信号量
pub struct DomainGuard {
    _permit: Option<OwnedSemaphorePermit>,
}

impl DomainGuard {
    fn new(permit: OwnedSemaphorePermit) -> Self {
        Self {
            _permit: Some(permit),
        }
    }
}

impl DomainGate {
    pub fn new() -> Self {
        let mut gates = HashMap::new();
        for (name, capacity) in CAPACITIES {
            gates.insert(*name, Arc::new(Semaphore::new(*capacity)));
        }
        Self {
            gates,
            default: Arc::new(Semaphore::new(5)),
        }
    }

    /// 获取指定供应商的并发许可证（异步等待直到有空位）
    pub async fn acquire(&self, vendor_name: &str) -> DomainGuard {
        let sem = self.gates.get(vendor_name).unwrap_or(&self.default);
        let permit = sem
            .clone()
            .acquire_owned()
            .await
            .expect("DomainGate semaphore closed");
        DomainGuard::new(permit)
    }

    /// 尝试获取许可证，不等待，失败返回 None
    pub fn try_acquire(&self, vendor_name: &str) -> Option<DomainGuard> {
        let sem = self.gates.get(vendor_name).unwrap_or(&self.default);
        sem.clone()
            .try_acquire_owned()
            .ok()
            .map(|p| DomainGuard::new(p))
    }
}

impl Default for DomainGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_release() {
        let gate = DomainGate::new();
        let guard = gate.acquire("eastmoney").await;
        drop(guard); // 显式释放
        let guard2 = gate.acquire("eastmoney").await;
        assert!(guard2._permit.is_some());
    }

    #[tokio::test]
    async fn test_capacity_limits() {
        let gate = DomainGate::new();
        // 东方财富只有 3 个许可证
        let g1 = gate.acquire("eastmoney").await;
        let g2 = gate.acquire("eastmoney").await;
        let g3 = gate.acquire("eastmoney").await;
        // 第4个尝试应该获取不到（非阻塞）
        assert!(gate.try_acquire("eastmoney").is_none());
        drop(g1);
        // 释放后能再从非阻塞获取到
        assert!(gate.try_acquire("eastmoney").is_some());
    }

    #[test]
    fn test_default_vendor() {
        let gate = DomainGate::new();
        assert!(gate.try_acquire("unknown_vendor").is_some());
    }
}
