// SPDX-License-Identifier: AGPL-3.0-only

//! 荐股策略包契约层 — YAML 自然语言策略包格式
//!
//! 让用户通过 YAML 文件描述策略包，配置现有 Rust 策略实现的参数，
//! 而不需要修改代码重新编译。
//!
//! ## 设计目标
//!
//! 1. **可读性**：YAML 格式，用户可以直接编辑
//! 2. **安全性**：不执行任意代码，只配置参数
//! 3. **可扩展**：支持引用现有 Rust 策略实现（通过 `strategy_id`）
//! 4. **可组合**：一个策略包包含多个策略条目，每个条目独立配置
//!
//! ## YAML 示例
//!
//! ```yaml
//! name: "短线趋势策略包"
//! description: "基于 MA 金叉和成交量放大的短线趋势跟踪"
//! version: "1.0.0"
//! author: "AxInvest"
//! min_confidence: 65
//! max_picks: 8
//! strategies:
//!   - id: "trend_short"
//!     strategy_id: "trend"
//!     style: "trend"
//!     period: "short"
//!     enabled: true
//!     weight: 1.2
//!     params:
//!       ma_short: 5
//!       ma_long: 20
//!       volume_ratio: 1.5
//!   - id: "trend_ultra_short"
//!     strategy_id: "trend"
//!     style: "trend"
//!     period: "ultra_short"
//!     enabled: true
//!     weight: 1.0
//!     params:
//!       ma_short: 3
//!       ma_long: 10
//!       volume_ratio: 2.0
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 策略风格（与 `stock-analysis::recommender::types::Style` 对齐）
///
/// 下沉到 harness 层避免 consumer 直接依赖 stock-analysis。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyPackStyle {
    /// 趋势跟踪
    Trend,
    /// 价值低估
    Value,
    /// 资金驱动
    Capital,
    /// 超跌反弹
    Reversion,
    /// 候选池兜底
    Watchlist,
    /// Serenity 瓶颈分析
    Serenity,
}

impl StrategyPackStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyPackStyle::Trend => "trend",
            StrategyPackStyle::Value => "value",
            StrategyPackStyle::Capital => "capital",
            StrategyPackStyle::Reversion => "reversion",
            StrategyPackStyle::Watchlist => "watchlist",
            StrategyPackStyle::Serenity => "serenity",
        }
    }
}

/// 持有周期（与 `stock-analysis::recommender::types::Period` 对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyPackPeriod {
    /// 超短线 1-3 天
    #[serde(alias = "ultra_short")]
    UltraShort,
    /// 短线 1-2 周
    Short,
    /// 中线 3-8 周
    Mid,
    /// 长线 3 个月+
    Long,
}

impl StrategyPackPeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyPackPeriod::UltraShort => "ultra_short",
            StrategyPackPeriod::Short => "short",
            StrategyPackPeriod::Mid => "mid",
            StrategyPackPeriod::Long => "long",
        }
    }
}

/// 单个策略条目（YAML 中 `strategies[]` 的一项）
///
/// 引用现有 Rust 策略实现，配置其参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPackStrategyEntry {
    /// 策略条目 ID（同一包内唯一，用于启停和权重调整）
    pub id: String,
    /// 引用的 Rust 策略实现 ID（如 "trend" / "value" / "capital" / "reversion"）
    pub strategy_id: String,
    /// 风格（与 strategy_id 对应，用于分组展示）
    pub style: StrategyPackStyle,
    /// 周期
    pub period: StrategyPackPeriod,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 权重（0.0-2.0，影响 confidence 计算）
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// 策略参数（key-value，由具体策略实现解释）
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    /// 可选的最低置信度覆盖（None 则使用包级 min_confidence）
    pub min_confidence: Option<u8>,
}

fn default_true() -> bool {
    true
}

fn default_weight() -> f64 {
    1.0
}

/// 策略包规格（完整 YAML 文件的反序列化目标）
///
/// 对应一个 YAML 文件，包含元信息和策略列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPackSpec {
    /// 包名称
    pub name: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 版本号（语义化版本）
    #[serde(default = "default_version")]
    pub version: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 包级最低置信度（0-100，被 strategies[].min_confidence 覆盖）
    #[serde(default = "default_min_confidence")]
    pub min_confidence: u8,
    /// 包级最大推荐数量
    #[serde(default = "default_max_picks")]
    pub max_picks: usize,
    /// 策略列表
    pub strategies: Vec<StrategyPackStrategyEntry>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_min_confidence() -> u8 {
    60
}

fn default_max_picks() -> usize {
    10
}

impl StrategyPackSpec {
    /// 从 YAML 字符串解析
    pub fn from_yaml(yaml: &str) -> Result<Self, StrategyPackError> {
        serde_yaml::from_str(yaml).map_err(StrategyPackError::YamlParse)
    }

    /// 序列化为 YAML 字符串
    pub fn to_yaml(&self) -> Result<String, StrategyPackError> {
        serde_yaml::to_string(self).map_err(StrategyPackError::YamlSerialize)
    }

    /// 从 JSON 字符串解析（前端 API 用）
    pub fn from_json(json: &str) -> Result<Self, StrategyPackError> {
        serde_json::from_str(json).map_err(StrategyPackError::JsonParse)
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, StrategyPackError> {
        serde_json::to_string(self).map_err(StrategyPackError::JsonSerialize)
    }

    /// 校验策略包规格
    ///
    /// - 策略列表非空
    /// - 每个策略的 id 唯一
    /// - weight 在 [0.0, 2.0]
    /// - min_confidence 在 [0, 100]
    pub fn validate(&self) -> Result<(), StrategyPackError> {
        if self.strategies.is_empty() {
            return Err(StrategyPackError::EmptyStrategies);
        }
        let mut seen_ids = std::collections::HashSet::new();
        for entry in &self.strategies {
            if !seen_ids.insert(&entry.id) {
                return Err(StrategyPackError::DuplicateStrategyId(entry.id.clone()));
            }
            if !(0.0..=2.0).contains(&entry.weight) {
                return Err(StrategyPackError::InvalidWeight(entry.id.clone(), entry.weight));
            }
            if let Some(mc) = entry.min_confidence
                && mc > 100
            {
                return Err(StrategyPackError::InvalidMinConfidence(entry.id.clone(), mc));
            }
        }
        if self.min_confidence > 100 {
            return Err(StrategyPackError::InvalidMinConfidence(
                "_pack".to_string(),
                self.min_confidence,
            ));
        }
        if self.max_picks == 0 {
            return Err(StrategyPackError::InvalidMaxPicks(self.max_picks));
        }
        Ok(())
    }

    /// 获取启用的策略条目
    pub fn enabled_entries(&self) -> impl Iterator<Item = &StrategyPackStrategyEntry> {
        self.strategies.iter().filter(|e| e.enabled)
    }
}

/// 策略包清单（轻量级元信息，用于列表展示）
///
/// 不包含完整策略列表，仅含概要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPackManifest {
    /// 包 ID（文件名或数据库 ID）
    pub id: String,
    /// 包名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: String,
    /// 策略数量
    pub strategy_count: usize,
    /// 启用策略数量
    pub enabled_count: usize,
    /// 是否启用（包级开关）
    pub enabled: bool,
    /// 来源路径（文件系统路径或 "builtin"）
    pub source: String,
}

/// 已加载的策略包（含运行时状态）
///
/// 包装 `StrategyPackSpec`，附加运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPack {
    /// 清单信息
    pub manifest: StrategyPackManifest,
    /// 完整规格
    pub spec: StrategyPackSpec,
}

/// 策略包错误类型
#[derive(Debug, thiserror::Error)]
pub enum StrategyPackError {
    #[error("YAML 解析失败: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("YAML 序列化失败: {0}")]
    YamlSerialize(serde_yaml::Error),
    #[error("JSON 解析失败: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("JSON 序列化失败: {0}")]
    JsonSerialize(serde_json::Error),
    #[error("策略列表为空")]
    EmptyStrategies,
    #[error("策略 ID 重复: {0}")]
    DuplicateStrategyId(String),
    #[error("策略 {0} 的权重 {1} 超出 [0.0, 2.0] 范围")]
    InvalidWeight(String, f64),
    #[error("策略 {0} 的最低置信度 {1} 超出 [0, 100] 范围")]
    InvalidMinConfidence(String, u8),
    #[error("最大推荐数量 {0} 必须大于 0")]
    InvalidMaxPicks(usize),
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("文件读取失败: {0}")]
    FileRead(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
name: "短线趋势策略包"
description: "基于 MA 金叉和成交量放大的短线趋势跟踪"
version: "1.0.0"
author: "AxInvest"
minConfidence: 65
maxPicks: 8
strategies:
  - id: "trend_short"
    strategyId: "trend"
    style: "trend"
    period: "short"
    enabled: true
    weight: 1.2
    params:
      maShort: 5
      maLong: 20
      volumeRatio: 1.5
  - id: "trend_ultra_short"
    strategyId: "trend"
    style: "trend"
    period: "ultra_short"
    enabled: false
    weight: 1.0
    params:
      maShort: 3
      maLong: 10
"#;

    #[test]
    fn parse_yaml_ok() {
        let spec = StrategyPackSpec::from_yaml(SAMPLE_YAML).expect("parse yaml");
        assert_eq!(spec.name, "短线趋势策略包");
        assert_eq!(spec.min_confidence, 65);
        assert_eq!(spec.max_picks, 8);
        assert_eq!(spec.strategies.len(), 2);
        assert_eq!(spec.strategies[0].id, "trend_short");
        assert_eq!(spec.strategies[0].style, StrategyPackStyle::Trend);
        assert_eq!(spec.strategies[0].period, StrategyPackPeriod::Short);
        assert!((spec.strategies[0].weight - 1.2).abs() < 1e-9);
        assert!(spec.strategies[0].enabled);
        assert!(!spec.strategies[1].enabled);
    }

    #[test]
    fn parse_yaml_period_alias() {
        let yaml = r#"
name: "test"
strategies:
  - id: "s1"
    strategyId: "trend"
    style: "trend"
    period: "ultra_short"
"#;
        let spec = StrategyPackSpec::from_yaml(yaml).expect("parse");
        assert_eq!(spec.strategies[0].period, StrategyPackPeriod::UltraShort);
    }

    #[test]
    fn validate_ok() {
        let spec = StrategyPackSpec::from_yaml(SAMPLE_YAML).expect("parse");
        spec.validate().expect("should be valid");
    }

    #[test]
    fn validate_empty_strategies() {
        let yaml = r#"
name: "empty"
strategies: []
"#;
        let spec = StrategyPackSpec::from_yaml(yaml).expect("parse");
        assert!(matches!(spec.validate(), Err(StrategyPackError::EmptyStrategies)));
    }

    #[test]
    fn validate_duplicate_id() {
        let yaml = r#"
name: "dup"
strategies:
  - id: "s1"
    strategyId: "trend"
    style: "trend"
    period: "short"
  - id: "s1"
    strategyId: "trend"
    style: "trend"
    period: "mid"
"#;
        let spec = StrategyPackSpec::from_yaml(yaml).expect("parse");
        assert!(matches!(spec.validate(), Err(StrategyPackError::DuplicateStrategyId(_))));
    }

    #[test]
    fn validate_weight_out_of_range() {
        let yaml = r#"
name: "bad"
strategies:
  - id: "s1"
    strategyId: "trend"
    style: "trend"
    period: "short"
    weight: 3.0
"#;
        let spec = StrategyPackSpec::from_yaml(yaml).expect("parse");
        assert!(matches!(spec.validate(), Err(StrategyPackError::InvalidWeight(_, _))));
    }

    #[test]
    fn roundtrip_yaml() {
        let spec = StrategyPackSpec::from_yaml(SAMPLE_YAML).expect("parse");
        let yaml = spec.to_yaml().expect("serialize");
        let spec2 = StrategyPackSpec::from_yaml(&yaml).expect("parse again");
        assert_eq!(spec.name, spec2.name);
        assert_eq!(spec.strategies.len(), spec2.strategies.len());
        assert_eq!(spec.strategies[0].id, spec2.strategies[0].id);
    }

    #[test]
    fn roundtrip_json() {
        let spec = StrategyPackSpec::from_yaml(SAMPLE_YAML).expect("parse");
        let json = spec.to_json().expect("to json");
        let spec2 = StrategyPackSpec::from_json(&json).expect("from json");
        assert_eq!(spec.name, spec2.name);
        assert_eq!(spec.strategies.len(), spec2.strategies.len());
    }

    #[test]
    fn enabled_entries_filters_disabled() {
        let spec = StrategyPackSpec::from_yaml(SAMPLE_YAML).expect("parse");
        let enabled: Vec<_> = spec.enabled_entries().collect();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "trend_short");
    }

    #[test]
    fn default_values() {
        let yaml = r#"
name: "minimal"
strategies:
  - id: "s1"
    strategyId: "trend"
    style: "trend"
    period: "short"
"#;
        let spec = StrategyPackSpec::from_yaml(yaml).expect("parse");
        assert_eq!(spec.version, "1.0.0");
        assert_eq!(spec.min_confidence, 60);
        assert_eq!(spec.max_picks, 10);
        assert!(spec.strategies[0].enabled);
        assert!((spec.strategies[0].weight - 1.0).abs() < 1e-9);
    }
}
