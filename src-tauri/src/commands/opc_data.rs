// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业数据接入层命令
//!
//! 与股票业务同架构：从 analysis-engine 引擎的 opc::vendors 模块
//! 获取 OpIndustryVendor / OpIndustryClient 实现，命令层只负责调用。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_analysis_engine::opc::*;

use crate::commands::opc_workflows::IndustryAnalysisConfig;

// ── 行业包加载 ─────────────────────────────────────────────────

/// 读取行业包 analysis.yaml（命令直读入口）
pub fn load_analysis_config(dir: &std::path::Path) -> Result<IndustryAnalysisConfig, String> {
    let path = dir.join("analysis.yaml");
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    serde_yaml::from_str(&raw).map_err(|e| format!("解析 {} 失败: {e}", path.display()))
}

// ── Vendor 注册中心（命令层构造，传入引擎） ──────────────────

/// 构造内建 vendor 集合并注册到 HashMap
#[allow(dead_code)]
pub fn build_vendor_map(
    data_service: Arc<dyn OpcDataService>,
    cache_dir: std::path::PathBuf,
) -> HashMap<String, Arc<dyn OpIndustryVendor>> {
    let mut vendors: HashMap<String, Arc<dyn OpIndustryVendor>> = HashMap::new();

    vendors.insert("db".to_string(), Arc::new(DbVendor::new(data_service)));
    vendors.insert("cache".to_string(), Arc::new(CacheVendor::new(cache_dir)));
    vendors.insert("web".to_string(), Arc::new(WebVendor));
    vendors.insert("file".to_string(), Arc::new(FileVendor));

    vendors
}

// ── OpIndustryClient 构造（从引擎） ──────────────────────────

/// 从行业包配置构造 OpIndustryClient
#[allow(dead_code)]
pub fn create_client(
    industry_id: String,
    config: &IndustryAnalysisConfig,
    vendors: HashMap<String, Arc<dyn OpIndustryVendor>>,
) -> OpIndustryClient {
    let sources: Vec<AnalysisDataSource> = config
        .data_sources
        .iter()
        .map(|s| AnalysisDataSource {
            id: s.id.clone(),
            chain: s.chain.clone(),
            quality_precheck: s.quality_precheck,
        })
        .collect();

    OpIndustryClient::new(industry_id, sources, vendors)
}
