// SPDX-License-Identifier: AGPL-3.0-only
//! 路径地址与路由图 — 三层路由树的地址编码与全局路径规划
//!
//! # 路径地址格式
//! `/{domain}/{cluster}/{capability}`
//! - `domain`: `CapabilityDomain::as_str()` 输出,如 `core`、`ai_media`
//! - `cluster`: 集群的 `path_segment`,如 `file_ops`、`image_gen`
//! - `capability`: `capability_id` 去掉 `{kind}:` 前缀后的简短 ID
//!
//! 示例:
//! - `/core/file_ops/read_file`
//! - `/finance/trading/execute_order`
//! - `/ai_media/image_gen/text_to_image`
//!
//! # 路由图
//! `RoutingGraph` 是 L1 → L2 → L3 的有向无环图(DAG)邻接表,
//! 设计期用于全局路径规划与 System Prompt 注入。

use crate::capability::CapabilityDomain;
use crate::capability_clusters::{all_clusters, find_cluster};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;

// ── 路径地址 ──────────────────────────────────────

/// 路径地址 — 唯一标识一个能力在三层路由树中的位置
///
/// 格式: `/{domain}/{cluster}/{capability}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct RoutingPath {
    /// L1: 业务域
    pub domain: CapabilityDomain,
    /// L2: 集群 path_segment(如 `"file_ops"`)
    pub cluster: String,
    /// L3: 能力简短 ID(去掉 `{kind}:` 前缀,如 `"read_file"`)
    pub capability: String,
}

impl RoutingPath {
    /// 解析路径字符串为 `RoutingPath`
    ///
    /// # 格式
    /// `/{domain}/{cluster}/{capability}`,必须以 `/` 开头,恰好 3 段。
    ///
    /// # 错误
    /// - 不以 `/` 开头
    /// - 段数不为 3
    /// - domain 段无法识别为合法 `CapabilityDomain`
    pub fn parse(path: &str) -> Result<Self, String> {
        let trimmed = path.trim();
        if !trimmed.starts_with('/') {
            return Err(format!("路径必须以 '/' 开头,实际: {path}"));
        }
        let segments: Vec<&str> = trimmed[1..].split('/').collect();
        if segments.len() != 3 {
            return Err(format!(
                "路径必须恰好 3 段(domain/cluster/capability),实际 {} 段: {path}",
                segments.len()
            ));
        }
        let domain = parse_domain(segments[0])
            .ok_or_else(|| format!("无法识别的 domain 段: '{}'", segments[0]))?;
        Ok(RoutingPath {
            domain,
            cluster: segments[1].to_string(),
            capability: segments[2].to_string(),
        })
    }

    /// 拼接完整路径字符串 `/{domain}/{cluster}/{capability}`
    pub fn to_path_string(&self) -> String {
        format!("/{}/{}/{}", self.domain.as_str(), self.cluster, self.capability)
    }

    /// 拼接 L1+L2 路径字符串 `/{domain}/{cluster}`(不含 L3 capability)
    pub fn l1_l2_string(&self) -> String {
        format!("/{}/{}", self.domain.as_str(), self.cluster)
    }

    /// 从完整 capability_id 提取简短段(去掉 `{kind}:` 前缀)
    ///
    /// 示例:`"tool:read_file"` → `"read_file"`,`"workflow:data_pipeline"` → `"data_pipeline"`
    /// 若无 `:` 前缀,原样返回。
    pub fn simplify_capability_id(capability_id: &str) -> String {
        match capability_id.find(':') {
            Some(pos) => capability_id[pos + 1..].to_string(),
            None => capability_id.to_string(),
        }
    }

    /// 从护照元数据构造路径地址
    ///
    /// # 参数
    /// - `domain`: 能力所属域
    /// - `cluster_path_segment`: 集群的 path_segment(由 `derive_cluster_for_passport` 推导)
    /// - `capability_id`: 完整能力 ID(如 `"tool:read_file"`)
    pub fn from_passport(
        domain: CapabilityDomain,
        cluster_path_segment: &str,
        capability_id: &str,
    ) -> Self {
        RoutingPath {
            domain,
            cluster: cluster_path_segment.to_string(),
            capability: Self::simplify_capability_id(capability_id),
        }
    }
}

/// 将字符串解析为 `CapabilityDomain`（兼容历史旧值 core/invest/opc）
pub(crate) fn parse_domain(s: &str) -> Option<CapabilityDomain> {
    s.parse().ok()
}

// ── 路由图(L1 → L2 → L3 DAG 邻接表) ────────────────

/// 路由图 — L1 → L2 → L3 的有向无环图邻接表
///
/// 设计期全局路径规划使用,可生成邻接表文本注入 System Prompt。
/// 节点规模:L1(8 业务域 + 1 系统域) + L2(~27) + L3(N) ≈ 200 节点上限。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingGraph {
    /// L1 → L2 邻接:domain → 该域下所有集群 ID(如 `"general_file_ops"`)
    #[serde(default)]
    pub domain_to_clusters: HashMap<CapabilityDomain, Vec<String>>,
    /// L2 → L3 邻接:cluster_id → 该集群下所有能力 ID(如 `"tool:read_file"`)
    #[serde(default)]
    pub cluster_to_capabilities: HashMap<String, Vec<String>>,
}

impl RoutingGraph {
    /// 创建空路由图
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建包含全部 L2 集群定义(来自 `all_clusters()`)的初始路由图
    ///
    /// 仅预填 L1 → L2 边,L3 节点待 `add_capability` 填充。
    pub fn with_all_clusters() -> Self {
        let mut graph = Self::new();
        for cluster in all_clusters() {
            graph
                .domain_to_clusters
                .entry(cluster.domain)
                .or_default()
                .push(cluster.cluster_id.to_string());
        }
        graph
    }

    /// 添加能力节点到路由图
    ///
    /// # 参数
    /// - `domain`: 能力所属域
    /// - `cluster`: 集群 ID(如 `"general_file_ops"`),若不在 L2 清单中也会注册到 L1→L2 边
    /// - `capability_id`: 完整能力 ID(如 `"tool:read_file"`)
    pub fn add_capability(&mut self, domain: CapabilityDomain, cluster: &str, capability_id: &str) {
        // L1 → L2 边(去重)
        let clusters = self.domain_to_clusters.entry(domain).or_default();
        if !clusters.iter().any(|c| c == cluster) {
            clusters.push(cluster.to_string());
        }
        // L2 → L3 边(去重)
        let caps = self.cluster_to_capabilities.entry(cluster.to_string()).or_default();
        if !caps.iter().any(|c| c == capability_id) {
            caps.push(capability_id.to_string());
        }
    }

    /// 删除能力节点(仅移除 L3 节点及其 L2→L3 边,保留 L1→L2 结构)
    pub fn remove_capability(&mut self, capability_id: &str) {
        for caps in self.cluster_to_capabilities.values_mut() {
            caps.retain(|c| c != capability_id);
        }
    }

    /// 查询某集群下的所有能力 ID
    pub fn capabilities_in_cluster(&self, cluster_id: &str) -> Vec<String> {
        self.cluster_to_capabilities.get(cluster_id).cloned().unwrap_or_default()
    }

    /// 查询某域下的所有集群 ID
    pub fn clusters_in_domain(&self, domain: CapabilityDomain) -> Vec<String> {
        self.domain_to_clusters.get(&domain).cloned().unwrap_or_default()
    }

    /// 生成 L1+L2 层邻接表文本(注入 System Prompt 用)
    ///
    /// # 输出格式
    /// ```text
    /// core: file_ops, text_ops, system_ops, config_ops
    /// general: search, summary, translation
    /// finance: market_data, trading, risk_control, portfolio
    /// ...
    /// ```
    ///
    /// 输出 path_segment(而非完整 cluster_id),保持简洁。
    /// 按 CapabilityDomain 枚举顺序输出,保证确定性。
    pub fn to_adjacency_text(&self) -> String {
        let mut lines = Vec::new();

        // 按 all_clusters() 顺序遍历 domain,保证确定性
        let mut seen_domains: Vec<CapabilityDomain> = Vec::new();
        for cluster in all_clusters() {
            if !seen_domains.contains(&cluster.domain) {
                seen_domains.push(cluster.domain);
            }
        }
        // 补充图中存在但 all_clusters 未覆盖的 domain(自定义集群场景)
        for domain in self.domain_to_clusters.keys() {
            if !seen_domains.contains(domain) {
                seen_domains.push(*domain);
            }
        }

        for domain in &seen_domains {
            let cluster_ids = match self.domain_to_clusters.get(domain) {
                Some(ids) => ids,
                None => continue,
            };
            if cluster_ids.is_empty() {
                continue;
            }
            // 将 cluster_id 转为 path_segment(更简洁,便于 LLM 理解)
            let segments: Vec<String> = cluster_ids
                .iter()
                .map(|cid| {
                    find_cluster(cid)
                        .map(|c| c.path_segment.to_string())
                        .unwrap_or_else(|| cid.clone())
                })
                .collect();
            let mut line = String::new();
            let _ = write!(line, "{}: {}", domain.as_str(), segments.join(", "));
            lines.push(line);
        }
        lines.join("\n")
    }
}
