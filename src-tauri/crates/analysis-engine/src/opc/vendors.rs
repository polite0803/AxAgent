// OPC 行业数据接入层 —— Vendor 体系
// 对齐 astock-data 的 StockVendor + FallbackChain 降级模式

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::data_service::{OpcDataService, TimeRange};
use super::error::OpcResult;

// ── OpIndustryVendor trait（对齐 StockVendor） ─────────────────

/// 行业数据供应商 trait
#[async_trait]
pub trait OpIndustryVendor: Send + Sync {
    fn name(&self) -> &str;

    async fn fetch(
        &self,
        industry_id: &str,
        data_domain: &str,
        query: &serde_json::Value,
    ) -> OpcResult<Option<serde_json::Value>>;

    async fn health_check(&self, industry_id: &str) -> OpcResult<bool>;
}

// ── Vendor 健康状态 ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VendorHealthState {
    consecutive_failures: u32,
    degraded: bool,
}

const DEGRADE_THRESHOLD: u32 = 3;

// ── DbVendor ────────────────────────────────────────────────────

/// DbVendor：读 opc_* 业务表
pub struct DbVendor {
    data_service: Arc<dyn OpcDataService>,
}

impl DbVendor {
    pub fn new(data_service: Arc<dyn OpcDataService>) -> Self {
        Self { data_service }
    }
}

#[async_trait]
impl OpIndustryVendor for DbVendor {
    fn name(&self) -> &str {
        "db"
    }

    async fn fetch(
        &self,
        _industry_id: &str,
        data_domain: &str,
        _query: &serde_json::Value,
    ) -> OpcResult<Option<serde_json::Value>> {
        let range = TimeRange::days(30);
        let from = range.start;
        let to = range.end;
        let value = match data_domain {
            "invoice_count" => self.data_service.count_invoices(&[], from, to).await?.to_string(),
            "invoice_amount" => {
                self.data_service.aggregate_invoice_amounts(&[], from, to).await?.total.to_string()
            },
            "customer_count" => self.data_service.count_customers(&[], from, to).await?.to_string(),
            "project_count" => self.data_service.count_projects(&[], from, to).await?.to_string(),
            "project_budget" => {
                self.data_service.aggregate_project_budgets(&[], from, to).await?.total.to_string()
            },
            _ => return Ok(None),
        };
        Ok(Some(serde_json::json!({ "domain": data_domain, "value": value })))
    }

    async fn health_check(&self, _industry_id: &str) -> OpcResult<bool> {
        Ok(true)
    }
}

// ── CacheVendor ─────────────────────────────────────────────────

/// CacheVendor：磁盘缓存（JSON 文件，key 带 `opc:{industry_id}:` 前缀）
pub struct CacheVendor {
    cache_dir: PathBuf,
}

impl CacheVendor {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    fn cache_path(&self, industry_id: &str, data_domain: &str) -> PathBuf {
        self.cache_dir.join(format!("opc:{industry_id}:{data_domain}.json"))
    }
}

#[async_trait]
impl OpIndustryVendor for CacheVendor {
    fn name(&self) -> &str {
        "cache"
    }

    async fn fetch(
        &self,
        industry_id: &str,
        data_domain: &str,
        _query: &serde_json::Value,
    ) -> OpcResult<Option<serde_json::Value>> {
        let path = self.cache_path(industry_id, data_domain);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };
        Ok(serde_json::from_str(&raw).ok())
    }

    async fn health_check(&self, _industry_id: &str) -> OpcResult<bool> {
        Ok(true)
    }
}

// ── WebVendor ───────────────────────────────────────────────────

/// WebVendor：外部检索（占位）
pub struct WebVendor;

#[async_trait]
impl OpIndustryVendor for WebVendor {
    fn name(&self) -> &str {
        "web"
    }

    async fn fetch(
        &self,
        _industry_id: &str,
        _data_domain: &str,
        _query: &serde_json::Value,
    ) -> OpcResult<Option<serde_json::Value>> {
        Ok(None)
    }

    async fn health_check(&self, _industry_id: &str) -> OpcResult<bool> {
        Ok(false)
    }
}

// ── FileVendor ──────────────────────────────────────────────────

/// FileVendor：本机资产文件（占位）
pub struct FileVendor;

#[async_trait]
impl OpIndustryVendor for FileVendor {
    fn name(&self) -> &str {
        "file"
    }

    async fn fetch(
        &self,
        _industry_id: &str,
        _data_domain: &str,
        _query: &serde_json::Value,
    ) -> OpcResult<Option<serde_json::Value>> {
        Ok(None)
    }

    async fn health_check(&self, _industry_id: &str) -> OpcResult<bool> {
        Ok(false)
    }
}

// ── OpIndustryClient ────────────────────────────────────────────

/// 数据源配置（对应 analysis.yaml 中的 data_sources）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDataSource {
    pub id: String,
    pub chain: Vec<String>,
    #[serde(default)]
    pub quality_precheck: bool,
}

/// 行业数据客户端：按行业包 data_sources 路由 + 降级 + 健康追踪
pub struct OpIndustryClient {
    industry_id: String,
    sources: Vec<AnalysisDataSource>,
    vendors: HashMap<String, Arc<dyn OpIndustryVendor>>,
    health: Mutex<HashMap<String, VendorHealthState>>,
}

impl OpIndustryClient {
    pub fn new(
        industry_id: String,
        sources: Vec<AnalysisDataSource>,
        vendors: HashMap<String, Arc<dyn OpIndustryVendor>>,
    ) -> Self {
        Self { industry_id, sources, vendors, health: Mutex::new(HashMap::new()) }
    }

    /// 取某数据源的数据：按 chain 依次尝试（降级），首个命中返回
    pub async fn fetch(
        &self,
        source_id: &str,
        data_domain: &str,
        query: &serde_json::Value,
    ) -> OpcResult<serde_json::Value> {
        let Some(source) = self.sources.iter().find(|s| s.id == source_id) else {
            return Ok(serde_json::Value::Null);
        };
        for vendor_name in &source.chain {
            let Some(vendor) = self.vendors.get(vendor_name) else { continue };
            let vname = vendor.name().to_string();
            if self.is_degraded(&vname) {
                continue;
            }
            if !vendor.health_check(&self.industry_id).await.unwrap_or(false) {
                self.record_failure(&vname);
                continue;
            }
            match vendor.fetch(&self.industry_id, data_domain, query).await {
                Ok(Some(value)) => {
                    self.record_success(&vname);
                    return Ok(value);
                },
                Ok(None) => {
                    self.record_success(&vname);
                    continue;
                },
                Err(_) => {
                    self.record_failure(&vname);
                    continue;
                },
            }
        }
        Ok(serde_json::Value::Null)
    }

    /// 质量预检
    pub async fn precheck(&self) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for source in self.sources.iter().filter(|s| s.quality_precheck) {
            let mut passed = false;
            let mut vendors = Vec::new();
            for vendor_name in &source.chain {
                let Some(vendor) = self.vendors.get(vendor_name) else { continue };
                let vname = vendor.name().to_string();
                if self.is_degraded(&vname) {
                    vendors.push(serde_json::json!({ "vendor": vname, "reachable": false, "degraded": true }));
                    continue;
                }
                let reachable = vendor.health_check(&self.industry_id).await.unwrap_or(false);
                if reachable {
                    passed = true;
                }
                vendors.push(serde_json::json!({ "vendor": vname, "reachable": reachable }));
            }
            out.push(serde_json::json!({
                "sourceId": source.id,
                "chain": source.chain,
                "passed": passed,
                "insufficient": !passed,
                "vendors": vendors,
            }));
        }
        out
    }

    /// 各 vendor 健康状态
    pub fn health_snapshot(&self) -> HashMap<String, serde_json::Value> {
        let health = self.health.lock();
        health
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::json!({ "degraded": v.degraded, "consecutiveFailures": v.consecutive_failures }),
                )
            })
            .collect()
    }

    fn is_degraded(&self, vendor_name: &str) -> bool {
        self.health.lock().get(vendor_name).map(|s| s.degraded).unwrap_or(false)
    }

    fn record_success(&self, vendor_name: &str) {
        let mut health = self.health.lock();
        let entry = health
            .entry(vendor_name.to_string())
            .or_insert(VendorHealthState { consecutive_failures: 0, degraded: false });
        entry.consecutive_failures = 0;
        entry.degraded = false;
    }

    fn record_failure(&self, vendor_name: &str) {
        let mut health = self.health.lock();
        let entry = health
            .entry(vendor_name.to_string())
            .or_insert(VendorHealthState { consecutive_failures: 0, degraded: false });
        entry.consecutive_failures += 1;
        if entry.consecutive_failures >= DEGRADE_THRESHOLD {
            entry.degraded = true;
            tracing::warn!(
                "[opc-data] 行业 {} vendor {vendor_name} 连续 {} 次失败，已降级",
                self.industry_id,
                entry.consecutive_failures
            );
        }
    }
}
