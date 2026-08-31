// SPDX-License-Identifier: AGPL-3.0-only
//! CapabilityBrowse — 能力树的逐层下钻（渐进式披露 L0 的导航层）。
//!
//! # 解决什么
//!
//! `<capability-index>` 目录受 token 预算封顶，能力一多就截断，且一次性摊开
//! 全量条目是 O(N) 枚举。树形导航把全局检索拆成多次边界清晰的局部决策：
//!
//! ```text
//! CapabilityBrowse {}                  → 列出全部域（L1 路由）
//! CapabilityBrowse { path: "invest" }  → 该域下的集群列表（L2 路由）
//! CapabilityBrowse { path: "invest/industry" } → 集群内能力摘要（精确定位）
//! ```
//!
//! 每层只返回当前层的候选 —— O(log N) 定位，替代全量扫描。
//! 分组依据与 `<capability-index>` 共用 `CapabilityPassportDto::cluster_label()`，
//! 两处口径不会漂移。

use super::capability_shared::capability_indexer;
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolErrorKind, ToolResult};
use async_trait::async_trait;
use axagent_harness::error_codes::capability::NOT_FOUND as CAPABILITY_NOT_FOUND;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// 单层返回条数上限（域/集群列表一般远小于此）。
const MAX_ENTRIES_PER_LEVEL: usize = 50;

pub struct CapabilityBrowseTool;

#[async_trait]
impl Tool for CapabilityBrowseTool {
    fn name(&self) -> &str {
        "CapabilityBrowse"
    }

    fn description(&self) -> &str {
        "逐层浏览能力树（渐进式披露 L0 导航层）：不传 path 列出全部域；\
         传 'domain' 列出该域下的集群；传 'domain/cluster' 列出集群内的能力摘要。\
         每层只返回当前层候选，适合能力较多时替代全量目录定位目标。\
         已知确切 ID 时直接用 CapabilityView，无需逐层浏览。"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "浏览路径：空=全部域；'domain'=该域的集群；'domain/cluster'=集群内能力"
                }
            }
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let raw_path = input["path"].as_str().unwrap_or("").trim().to_string();
        let path_parts: Vec<&str> =
            raw_path.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();

        let Some(indexer) = capability_indexer() else {
            return Err(browse_failed("能力索引器尚未初始化，无法浏览能力树"));
        };

        // 与目录/定义层同口径：不可见能力在任何层级都不可见
        let passports: Vec<_> =
            indexer.list_passports().await.into_iter().filter(|p| p.is_user_visible()).collect();

        match path_parts.as_slice() {
            // 第 1 层：域列表（含各域集群数，供下钻决策）
            [] => {
                // BTreeSet 天然去重 + 有序，避免线性 contains 扫描
                let mut domains: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
                for p in &passports {
                    domains.entry(p.domain.as_str()).or_default().insert(p.cluster_label());
                }

                if domains.is_empty() {
                    return Ok(ToolResult::success("能力树为空（当前无可发现能力）"));
                }

                let mut out =
                    String::from("## 能力域\n\n逐层浏览：传 'domain' 查看域下的集群。\n\n");
                for (domain, clusters) in &domains {
                    let count = clusters.len();
                    out.push_str(&format!("- **{domain}**（{count} 个集群）\n"));
                }
                Ok(text_result(out, json!({"level": "domains", "count": domains.len()})))
            },

            // 第 2 层：域下的集群列表
            [domain] => {
                let mut clusters: BTreeMap<String, Vec<&axagent_harness::CapabilityPassportDto>> =
                    BTreeMap::new();
                for p in passports.iter().filter(|p| p.domain.as_str() == *domain) {
                    clusters.entry(p.cluster_label().to_string()).or_default().push(p);
                }

                if clusters.is_empty() {
                    return Err(not_found(&format!("域 '{domain}' 下无可发现能力")));
                }

                let mut out = format!(
                    "## 域 '{domain}' 下的集群\n\n逐层浏览：传 '{domain}/<cluster>' 查看集群内的能力。\n\n"
                );
                for (cluster, members) in clusters.iter().take(MAX_ENTRIES_PER_LEVEL) {
                    out.push_str(&format!("- **{cluster}**（{} 项能力）\n", members.len()));
                }
                Ok(text_result(
                    out,
                    json!({"level": "clusters", "domain": domain, "count": clusters.len()}),
                ))
            },

            // 第 3 层：集群内的能力摘要
            [domain, cluster] => {
                let members: Vec<_> = passports
                    .iter()
                    .filter(|p| p.domain.as_str() == *domain && p.cluster_label() == *cluster)
                    .collect();

                if members.is_empty() {
                    return Err(not_found(&format!("域 '{domain}' 下未找到集群 '{cluster}'")));
                }

                let mut out = format!(
                    "## 集群 '{domain}/{cluster}'（{} 项能力）\n\n\
                     确定目标后用 CapabilityView 展开定义、CapabilityLoad 加载。\n\n",
                    members.len()
                );
                for p in members.iter().take(MAX_ENTRIES_PER_LEVEL) {
                    let summary = p.summary.as_deref().unwrap_or(&p.description);
                    out.push_str(&format!("- **{}** {}: {}\n", p.capability_id, p.name, summary));
                }
                Ok(text_result(
                    out,
                    json!({"level": "capabilities", "domain": domain, "cluster": cluster,
                           "count": members.len()}),
                ))
            },

            _ => Err(ToolError::invalid_input_for(
                "CapabilityBrowse",
                "path 最多两级：'domain' 或 'domain/cluster'",
            )),
        }
    }
}

fn text_result(content: String, metadata: Value) -> ToolResult {
    ToolResult {
        content,
        is_error: false,
        truncated: false,
        metadata: Some(metadata),
        duration_ms: None,
        progress: Vec::new(),
    }
}

fn not_found(message: &str) -> ToolError {
    ToolError {
        message: message.to_string(),
        kind: ToolErrorKind::NotFound,
        error_code: CAPABILITY_NOT_FOUND.to_string(),
    }
}

fn browse_failed(message: &str) -> ToolError {
    ToolError {
        message: message.to_string(),
        kind: ToolErrorKind::ExecutionFailed,
        error_code: CAPABILITY_NOT_FOUND.to_string(),
    }
}
