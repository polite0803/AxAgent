// SPDX-License-Identifier: AGPL-3.0-only

//! 轻量实体抽取器 —— 从用户输入提取关键实体（P1，无 LLM 成本）。
//!
//! 服务于「语义解析 → 意图向量 + 关键实体」规范步骤的实体部分：
//! 认知编排/能力发现命中 `Template` 能力时，凭其 `placeholders`（占位符定义）
//! 从用户输入提取实际值（如 `{{target_ip}}` → "192.168.1.1"），
//! 产出"可实例化提示"而非让认知层手工解析。
//!
//! 实现为纯正则规则（ip / date_range / number / email / url / id），
//! 确定性、零依赖、毫秒级；不引入 LLM 调用（避免运行时边界与成本问题）。

use crate::capability::PlaceholderDef;
use serde::{Deserialize, Serialize};

/// 提取出的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityEntity {
    /// 占位符名（对应 PlaceholderDef.name）
    pub name: String,
    /// 从输入中提取到的值
    pub value: String,
    /// 实体类型（ip / date_range / number / email / url / id）
    pub entity_type: String,
    /// 该占位符的说明（取自 PlaceholderDef）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// 从用户输入按占位符定义提取实体。
///
/// 每个占位符按 `placeholder_type` 匹配对应正则，取第一个命中；
/// 匹配不到时跳过（该占位符留给认知层/用户后续填充）。
pub fn extract_entities(input: &str, placeholders: &[PlaceholderDef]) -> Vec<CapabilityEntity> {
    if placeholders.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    for p in placeholders {
        if let Some(value) = match_placeholder(input, &p.placeholder_type) {
            result.push(CapabilityEntity {
                name: p.name.clone(),
                value,
                entity_type: p.placeholder_type.clone(),
                description: p.description.clone(),
            });
        }
    }
    result
}

/// 按占位符类型匹配输入，返回第一个命中的值。
fn match_placeholder(input: &str, placeholder_type: &str) -> Option<String> {
    match placeholder_type {
        "ip" => {
            // IPv4：四段 0-255
            let re = regex::Regex::new(
                r"(?i)\b(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}\b",
            )
            .ok()?;
            re.find(input).map(|m| m.as_str().to_string())
        },
        "date_range" => {
            // 日期区间：YYYY-MM-DD ~ YYYY-MM-DD（支持 ~ / - / 至 分隔）
            let re = regex::Regex::new(
                r"(?i)\d{4}[-/]\d{1,2}[-/]\d{1,2}\s*(~|-|至|到)\s*\d{4}[-/]\d{1,2}[-/]\d{1,2}",
            )
            .ok()?;
            re.find(input).map(|m| m.as_str().trim().to_string())
        },
        "number" => {
            // 数字（含小数）
            let re = regex::Regex::new(r"-?\d+(\.\d+)?").ok()?;
            re.find(input).map(|m| m.as_str().to_string())
        },
        "email" => {
            let re = regex::Regex::new(r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}").ok()?;
            re.find(input).map(|m| m.as_str().to_string())
        },
        "url" => {
            let re = regex::Regex::new(r"(?i)https?://[^\s<>]+").ok()?;
            re.find(input).map(|m| {
                m.as_str().trim_end_matches(|c: char| c.is_ascii_punctuation()).to_string()
            })
        },
        // string / enum / 其他未知类型：取第一个非空词（宽松兜底）
        _ => input.split_whitespace().next().filter(|s| !s.is_empty()).map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ph(name: &str, t: &str) -> PlaceholderDef {
        PlaceholderDef {
            name: name.to_string(),
            placeholder_type: t.to_string(),
            description: String::new(),
        }
    }

    #[test]
    fn extracts_ip() {
        let out = extract_entities("扫描 192.168.1.5 的开放端口", &[ph("target_ip", "ip")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "192.168.1.5");
        assert_eq!(out[0].entity_type, "ip");
    }

    #[test]
    fn extracts_date_range() {
        let out = extract_entities(
            "统计 2026-01-01 ~ 2026-03-31 的收益",
            &[ph("date_range", "date_range")],
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].value.contains("2026-01-01") && out[0].value.contains("2026-03-31"));
    }

    #[test]
    fn extracts_number() {
        let out = extract_entities("预算 5000 元", &[ph("budget", "number")]);
        assert_eq!(out[0].value, "5000");
    }

    #[test]
    fn skips_unmatched() {
        let out = extract_entities("没有任何 IP 的任务", &[ph("target_ip", "ip")]);
        assert!(out.is_empty(), "无 IP 时应跳过占位符");
    }

    #[test]
    fn multiple_placeholders() {
        let defs = vec![ph("target_ip", "ip"), ph("date_range", "date_range")];
        let out = extract_entities("扫描 10.0.0.1 从 2026-01-01 至 2026-02-01", &defs);
        assert_eq!(out.len(), 2);
    }
}
