// SPDX-License-Identifier: AGPL-3.0-only
//! L2 簇路由规则 — 三层路由树第二层
//!
//! # 架构
//! ```text
//! L1 域路由 → [L2 簇路由] → 匹配具体功能集群
//!                  │
//!                  ├── 命中规则 → 直接返回 Cluster
//!                  └── 未命中 → 关键词推导（derive_cluster_for_passport）
//! ```
//!
//! # 与 L1 的关系
//! - L1 确定业务域（Domain）
//! - L2 在域内确定功能集群（Cluster）
//! - L3 由向量索引检索具体能力（Capability）

use crate::capability::CapabilityDomain;
use crate::capability_clusters::CapabilityCluster;
use crate::capability_router::CapabilityDiscoveryRequest;
use crate::domain_router::DomainRoutingResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── L2 簇路由规则 ──────────────────────────────────

/// L2 簇路由规则 — 在已确定业务域的基础上，进一步确定功能集群
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterRoutingRule {
    /// 规则 ID（唯一）
    pub rule_id: String,
    /// 规则名称（中文）
    pub rule_name: String,
    /// 所属业务域（L1 已确定的域）
    pub domain: CapabilityDomain,
    /// 目标集群 ID（对应 `CapabilityCluster::cluster_id`）
    pub target_cluster_id: String,
    /// 匹配关键词列表
    pub keywords: Vec<String>,
    /// 排除关键词列表（命中则不匹配）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_keywords: Vec<String>,
    /// 规则优先级
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 规则描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_priority() -> i32 {
    50
}

fn default_enabled() -> bool {
    true
}

impl ClusterRoutingRule {
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        domain: CapabilityDomain,
        target_cluster_id: impl Into<String>,
        keywords: Vec<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            domain,
            target_cluster_id: target_cluster_id.into(),
            keywords,
            exclude_keywords: vec![],
            priority: 50,
            enabled: true,
            description: None,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_exclude(mut self, keywords: Vec<String>) -> Self {
        self.exclude_keywords = keywords;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 测试 Query 是否命中此规则
    pub fn matches(&self, query: &str, domain: CapabilityDomain) -> bool {
        if !self.enabled {
            return false;
        }
        if self.domain != domain {
            return false;
        }

        let query_lower = query.to_lowercase();

        // 排除关键词
        if self.exclude_keywords.iter().any(|kw| query_lower.contains(&kw.to_lowercase())) {
            return false;
        }

        // 任一关键词命中
        self.keywords.iter().any(|kw| query_lower.contains(&kw.to_lowercase()))
    }
}

// ── L2 簇路由结果 ──────────────────────────────────

/// L2 簇路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterRoutingResult {
    /// 命中的业务域（来自 L1）
    pub domain: CapabilityDomain,
    /// 命中的集群
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster: Option<CapabilityCluster>,
    /// 命中的规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<ClusterRoutingRule>,
    /// 是否通过关键词推导（derive_cluster_for_passport）
    pub is_keyword_derived: bool,
    /// 是否兜底（无集群匹配，返回域内默认集群）
    pub is_fallback: bool,
    /// 置信度
    pub confidence: f64,
    /// 路由耗时（毫秒）
    pub elapsed_ms: u64,
}

impl ClusterRoutingResult {
    pub fn rule_hit(
        domain: CapabilityDomain,
        cluster: CapabilityCluster,
        rule: ClusterRoutingRule,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            domain,
            cluster: Some(cluster),
            matched_rule: Some(rule),
            is_keyword_derived: false,
            is_fallback: false,
            confidence: 1.0,
            elapsed_ms,
        }
    }

    pub fn keyword_derived(
        domain: CapabilityDomain,
        cluster: CapabilityCluster,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            domain,
            cluster: Some(cluster),
            matched_rule: None,
            is_keyword_derived: true,
            is_fallback: false,
            confidence: 0.8,
            elapsed_ms,
        }
    }

    pub fn fallback(
        domain: CapabilityDomain,
        default_cluster: CapabilityCluster,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            domain,
            cluster: Some(default_cluster),
            matched_rule: None,
            is_keyword_derived: false,
            is_fallback: true,
            confidence: 0.3,
            elapsed_ms,
        }
    }

    pub fn no_match(domain: CapabilityDomain, elapsed_ms: u64) -> Self {
        Self {
            domain,
            cluster: None,
            matched_rule: None,
            is_keyword_derived: false,
            is_fallback: true,
            confidence: 0.0,
            elapsed_ms,
        }
    }
}

// ── L2 簇路由接口 ──────────────────────────────────

/// L2 簇路由接口 — 在业务域内确定功能集群
///
/// # 路由流程
/// ```text
/// 1. 加载指定域的所有启用规则（按优先级排序）
/// 2. 逐条规则匹配用户 Query
///    - 命中 → 返回 Cluster + 规则信息
///    - 未命中 → 继续下一条
/// 3. 所有规则未命中 → 关键词推导（derive_cluster_for_passport 风格）
/// 4. 推导也未命中 → 返回域内默认集群
/// ```
#[async_trait]
pub trait ClusterRouter: Send + Sync {
    /// 执行 L2 簇路由
    ///
    /// # 参数
    /// - `query`: 用户原始输入
    /// - `l1_result`: L1 域路由结果（确定业务域）
    async fn route(&self, query: &str, l1_result: &DomainRoutingResult) -> ClusterRoutingResult;

    /// 从 CapabilityDiscoveryRequest 直接执行 L2 路由
    async fn route_from_request(
        &self,
        request: &CapabilityDiscoveryRequest,
        l1_result: &DomainRoutingResult,
    ) -> ClusterRoutingResult {
        self.route(&request.user_input, l1_result).await
    }

    /// 获取指定域的所有规则
    async fn list_rules(&self, domain: CapabilityDomain) -> Vec<ClusterRoutingRule>;

    /// 添加规则
    async fn add_rule(&self, rule: ClusterRoutingRule) -> Result<(), String>;

    /// 更新规则
    async fn update_rule(&self, rule: ClusterRoutingRule) -> Result<(), String>;

    /// 删除规则
    async fn remove_rule(&self, rule_id: &str) -> Result<(), String>;

    /// 获取指定域的默认集群
    async fn get_default_cluster(&self, domain: CapabilityDomain) -> Option<CapabilityCluster>;

    /// 设置指定域的默认集群
    async fn set_default_cluster(
        &self,
        domain: CapabilityDomain,
        cluster_id: &str,
    ) -> Result<(), String>;
}

// ── 通用关键词推导（复用 derive_cluster_for_passport 思路） ──

/// 基于关键词推导集群（L2 兜底逻辑）
///
/// 与 `derive_cluster_for_passport` 类似，但基于用户 Query 而非护照元数据。
/// 关键区别：**零命中返回 `None`**（不兜底到域内第一个集群），
/// 由调用方决定是降级到 L3 向量检索还是走 L2 fallback 语义。
pub fn derive_cluster_from_query(
    query: &str,
    domain: CapabilityDomain,
) -> Option<CapabilityCluster> {
    // 复用 capability_clusters 中的关键词评分逻辑（复用 best_cluster_by_keywords）
    let query_lower = query.to_lowercase();
    let tags: Vec<String> = vec![query_lower.clone()];
    let name = &query_lower;
    crate::capability_clusters::best_cluster_by_keywords(domain, &tags, name).copied()
}

// ── 默认簇路由器实现 ──────────────────────────────

/// L2 簇路由默认实现 — 规则 + 关键词推导 + 域内默认集群兜底
///
/// # 路由流程
/// 1. 加载指定域的全部启用规则，按优先级降序排序
/// 2. 逐条匹配用户 Query，命中即返回 `rule_hit`
/// 3. 全部未命中 → `derive_cluster_from_query` 关键词推导（置信度 0.8）
/// 4. 推导也未命中 → 域内第一个集群兜底（`fallback`，置信度 0.3）
/// 5. 域内无集群 → `no_match`
///
/// # 线程安全
/// 规则集合由 `tokio::sync::RwLock` 保护，满足 AGENTS.md 铁律 8。
pub struct ClusterRouterImpl {
    rules: tokio::sync::RwLock<Vec<ClusterRoutingRule>>,
}

impl ClusterRouterImpl {
    /// 创建默认实现（初始无规则，靠关键词推导 + 域内默认集群兜底）
    pub fn new() -> Self {
        Self { rules: tokio::sync::RwLock::new(Vec::new()) }
    }

    /// 以自定义规则集创建（用于测试或运行时覆盖）
    pub fn with_rules(rules: Vec<ClusterRoutingRule>) -> Self {
        Self { rules: tokio::sync::RwLock::new(rules) }
    }
}

impl Default for ClusterRouterImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClusterRouter for ClusterRouterImpl {
    async fn route(&self, query: &str, l1_result: &DomainRoutingResult) -> ClusterRoutingResult {
        let start = std::time::Instant::now();
        let domain = l1_result.domain;
        let rules = self.rules.read().await;

        // 过滤指定域的启用规则，按优先级降序排序
        let mut sorted: Vec<&ClusterRoutingRule> =
            rules.iter().filter(|r| r.domain == domain && r.enabled).collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.priority));

        for rule in sorted {
            if rule.matches(query, domain) {
                let elapsed = start.elapsed().as_millis() as u64;
                let cluster = crate::capability_clusters::find_cluster(&rule.target_cluster_id)
                    .copied()
                    .unwrap_or_else(|| {
                        // 规则指向的集群不存在时，退化为域内默认集群
                        crate::capability_clusters::clusters_by_domain(domain)
                            .first()
                            .copied()
                            .unwrap_or(
                                crate::capability_clusters::all_clusters()
                                    .first()
                                    .copied()
                                    .unwrap(),
                            )
                    });
                tracing::debug!(
                    rule_id = %rule.rule_id,
                    cluster_id = %cluster.cluster_id,
                    "L2 簇路由规则命中"
                );
                return ClusterRoutingResult::rule_hit(domain, cluster, rule.clone(), elapsed);
            }
        }

        // 规则未命中：关键词推导（零命中返回 None，不兜底到域内第一个集群）
        if let Some(cluster) = derive_cluster_from_query(query, domain) {
            let elapsed = start.elapsed().as_millis() as u64;
            tracing::debug!(cluster_id = %cluster.cluster_id, "L2 关键词推导命中");
            return ClusterRoutingResult::keyword_derived(domain, cluster, elapsed);
        }

        // 兜底：域内默认集群
        if let Some(default_cluster) =
            crate::capability_clusters::clusters_by_domain(domain).first()
        {
            let elapsed = start.elapsed().as_millis() as u64;
            tracing::debug!(cluster_id = %default_cluster.cluster_id, "L2 域内默认集群兜底");
            return ClusterRoutingResult::fallback(domain, *default_cluster, elapsed);
        }

        let elapsed = start.elapsed().as_millis() as u64;
        ClusterRoutingResult::no_match(domain, elapsed)
    }

    async fn list_rules(&self, domain: CapabilityDomain) -> Vec<ClusterRoutingRule> {
        self.rules.read().await.iter().filter(|r| r.domain == domain).cloned().collect()
    }

    async fn add_rule(&self, rule: ClusterRoutingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        if rules.iter().any(|r| r.rule_id == rule.rule_id) {
            return Err(format!("规则 ID 已存在: {}", rule.rule_id));
        }
        rules.push(rule);
        Ok(())
    }

    async fn update_rule(&self, rule: ClusterRoutingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        let idx = rules
            .iter()
            .position(|r| r.rule_id == rule.rule_id)
            .ok_or_else(|| format!("规则不存在: {}", rule.rule_id))?;
        rules[idx] = rule;
        Ok(())
    }

    async fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.rule_id != rule_id);
        if rules.len() == before {
            Err(format!("规则不存在: {}", rule_id))
        } else {
            Ok(())
        }
    }

    async fn get_default_cluster(&self, domain: CapabilityDomain) -> Option<CapabilityCluster> {
        crate::capability_clusters::clusters_by_domain(domain).first().copied()
    }

    async fn set_default_cluster(
        &self,
        domain: CapabilityDomain,
        cluster_id: &str,
    ) -> Result<(), String> {
        let target = crate::capability_clusters::find_cluster(cluster_id)
            .copied()
            .ok_or_else(|| format!("集群不存在: {}", cluster_id))?;
        if target.domain != domain {
            return Err(format!("集群 {} 不属于域 {}", cluster_id, domain.as_str()));
        }
        // 默认集群通过把规则优先级提升到最高实现；无规则时以域内第一个集群为默认
        let mut rules = self.rules.write().await;
        for rule in rules.iter_mut() {
            if rule.domain == domain {
                rule.priority = 10;
            }
        }
        rules.push(
            ClusterRoutingRule::new(
                format!("rule_default_{}_{}", domain.as_str(), target.cluster_id),
                "域默认集群",
                domain,
                target.cluster_id,
                Vec::new(),
            )
            .with_priority(5)
            .with_description("域内默认集群（优先级最低，兜底用）"),
        );
        Ok(())
    }
}
