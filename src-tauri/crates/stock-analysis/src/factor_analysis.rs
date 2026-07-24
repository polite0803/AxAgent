//! 因子分析体系 — 因子注册表 + IC/IR 计算 + 权重优化
//!
//! ## 设计
//!
//! 当前 `hit_rate_backtest` 的 9 因子 IC 计算是 Spearman 硬编码，无因子组合优化。
//! 本模块提供：
//!
//! 1. **因子注册表**：注册/发现因子（因子的计算公式、所属类别、默认权重）
//! 2. **IC 计算**：Spearman Rank IC + 截面 IC + 时序 IC
//! 3. **IR 计算**：IC 均值 / IC 标准差（因子稳定性）
//! 4. **权重优化**：IC 加权 → IR 加权 → 等权
//! 5. **因子暴露度**：股票在各因子上的得分

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 因子类别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FactorCategory {
    /// 动量类（Momentum）
    Momentum,
    /// 反转类（Reversal）
    Reversal,
    /// 价值类（Value）
    Value,
    /// 质量类（Quality）
    Quality,
    /// 成长类（Growth）
    Growth,
    /// 技术类（Technical）
    Technical,
    /// 情绪类（Sentiment）
    Sentiment,
    /// 资金流类（CapitalFlow）
    CapitalFlow,
    /// 风险类（Risk）
    Risk,
}

/// 因子定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorDefinition {
    /// 因子标识（如 "pe_ttm", "mom_20d"）
    pub id: String,
    /// 因子名称（如 "市盈率(TTM)"）
    pub name: String,
    /// 因子类别
    pub category: FactorCategory,
    /// 因子方向：true=值越大越好，false=值越小越好
    pub higher_is_better: bool,
    /// 描述
    pub description: String,
    /// 默认权重（0.0-1.0，所有因子之和不要求=1.0）
    pub default_weight: f64,
    /// 是否启用
    pub enabled: bool,
}

/// 因子 IC 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorIC {
    /// 因子 id
    pub factor_id: String,
    /// Spearman Rank IC
    pub spearman_ic: f64,
    /// Pearson IC
    pub pearson_ic: f64,
    /// IC 标准差（稳定性）
    pub ic_std: f64,
    /// IR（IC 均值 / IC 标准差）
    pub ir: f64,
    /// 计算所用样本数
    pub sample_count: u32,
    /// 窗口（天数）
    pub window_days: u32,
    /// 是否显著（|IR| > 0.5）
    pub is_significant: bool,
}

/// 因子暴露度 — 股票在某个因子上的标准化分数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorExposure {
    pub stock_code: String,
    pub stock_name: String,
    /// factor_id → z-score
    pub scores: HashMap<String, f64>,
    /// 综合得分（加权求和）
    pub composite_score: f64,
}

/// 因子权重优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorWeightResult {
    /// factor_id → 优化后权重
    pub weights: HashMap<String, f64>,
    /// 优化方法： "ic_weighted" | "ir_weighted" | "equal_weight"
    pub method: String,
    /// 预期 IC（加权后）
    pub expected_ic: f64,
    /// 预期 IR
    pub expected_ir: f64,
}

/// 因子注册表
#[derive(Debug, Clone)]
pub struct FactorRegistry {
    factors: HashMap<String, FactorDefinition>,
    /// 历史 IC 记录（factor_id → [IC 序列]）
    ic_history: HashMap<String, Vec<FactorIC>>,
}

impl Default for FactorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FactorRegistry {
    pub fn new() -> Self {
        let mut registry = Self { factors: HashMap::new(), ic_history: HashMap::new() };
        registry.register_defaults();
        registry
    }

    /// 注册一个因子
    pub fn register(&mut self, factor: FactorDefinition) {
        self.factors.insert(factor.id.clone(), factor);
    }

    /// 获取所有因子
    pub fn all_factors(&self) -> Vec<&FactorDefinition> {
        self.factors.values().filter(|f| f.enabled).collect()
    }

    /// 获取因子定义
    pub fn get(&self, id: &str) -> Option<&FactorDefinition> {
        self.factors.get(id)
    }

    /// 记录一次 IC 计算
    pub fn record_ic(&mut self, ic: FactorIC) {
        self.ic_history.entry(ic.factor_id.clone()).or_default().push(ic);
    }

    /// 获取因子 IC 历史
    pub fn ic_series(&self, factor_id: &str) -> Vec<&FactorIC> {
        self.ic_history.get(factor_id).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// 计算 IC 加权权重
    pub fn compute_ic_weighted_weights(&self) -> FactorWeightResult {
        let factors = self.all_factors();
        if factors.is_empty() {
            return FactorWeightResult {
                weights: HashMap::new(),
                method: "ic_weighted".into(),
                expected_ic: 0.0,
                expected_ir: 0.0,
            };
        }

        // 取每个因子最新的 IC
        let mut ics: Vec<(String, f64)> = Vec::new();
        for f in &factors {
            if let Some(series) = self.ic_history.get(&f.id) {
                if let Some(latest) = series.last() {
                    ics.push((f.id.clone(), latest.spearman_ic.abs()));
                }
            }
        }

        let total: f64 = ics.iter().map(|(_, ic)| ic).sum();
        let weights: HashMap<String, f64> = if total > 0.0 {
            ics.iter().map(|(id, ic)| (id.clone(), ic / total)).collect()
        } else {
            // 降级到等权
            let w = 1.0 / factors.len() as f64;
            factors.iter().map(|f| (f.id.clone(), w)).collect()
        };

        FactorWeightResult {
            expected_ic: ics.iter().map(|(_, ic)| ic).sum::<f64>() / ics.len().max(1) as f64,
            expected_ir: 0.0,
            method: "ic_weighted".into(),
            weights,
        }
    }

    /// 计算 IR 加权权重
    pub fn compute_ir_weighted_weights(&self) -> FactorWeightResult {
        let factors = self.all_factors();
        if factors.is_empty() {
            return FactorWeightResult {
                weights: HashMap::new(),
                method: "ir_weighted".into(),
                expected_ic: 0.0,
                expected_ir: 0.0,
            };
        }

        let mut irs: Vec<(String, f64)> = Vec::new();
        for f in &factors {
            if let Some(series) = self.ic_history.get(&f.id) {
                if series.len() >= 3 {
                    let ic_values: Vec<f64> = series.iter().map(|ic| ic.spearman_ic).collect();
                    let mean = ic_values.iter().sum::<f64>() / ic_values.len() as f64;
                    let variance = if ic_values.len() > 1 {
                        ic_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                            / (ic_values.len() - 1) as f64
                    } else {
                        0.0
                    };
                    let std = variance.sqrt();
                    let ir = if std > 0.0 { mean / std } else { 0.0 };
                    irs.push((f.id.clone(), ir.abs()));
                }
            }
        }

        let total: f64 = irs.iter().map(|(_, ir)| ir).sum();
        let weights = if total > 0.0 {
            irs.iter().map(|(id, ir)| (id.clone(), ir / total)).collect()
        } else {
            self.compute_ic_weighted_weights().weights
        };

        FactorWeightResult {
            expected_ic: 0.0,
            expected_ir: irs.iter().map(|(_, ir)| ir).sum::<f64>() / irs.len().max(1) as f64,
            method: "ir_weighted".into(),
            weights,
        }
    }

    /// 注册 12 个默认因子
    fn register_defaults(&mut self) {
        let defaults = vec![
            FactorDefinition {
                id: "mom_20d".into(),
                name: "20日动量".into(),
                category: FactorCategory::Momentum,
                higher_is_better: true,
                description: "过去20个交易日的累计收益率".into(),
                default_weight: 0.15,
                enabled: true,
            },
            FactorDefinition {
                id: "mom_60d".into(),
                name: "60日动量".into(),
                category: FactorCategory::Momentum,
                higher_is_better: true,
                description: "过去60个交易日的累计收益率".into(),
                default_weight: 0.10,
                enabled: true,
            },
            FactorDefinition {
                id: "rev_5d".into(),
                name: "5日反转".into(),
                category: FactorCategory::Reversal,
                higher_is_better: false,
                description: "过去5个交易日收益率（超跌反弹因子）".into(),
                default_weight: 0.08,
                enabled: true,
            },
            FactorDefinition {
                id: "pe_ttm".into(),
                name: "市盈率(TTM)".into(),
                category: FactorCategory::Value,
                higher_is_better: false,
                description: "滚动市盈率".into(),
                default_weight: 0.10,
                enabled: true,
            },
            FactorDefinition {
                id: "pb".into(),
                name: "市净率".into(),
                category: FactorCategory::Value,
                higher_is_better: false,
                description: "市净率".into(),
                default_weight: 0.08,
                enabled: true,
            },
            FactorDefinition {
                id: "roe".into(),
                name: "ROE".into(),
                category: FactorCategory::Quality,
                higher_is_better: true,
                description: "净资产收益率".into(),
                default_weight: 0.08,
                enabled: true,
            },
            FactorDefinition {
                id: "profit_yoy".into(),
                name: "净利润同比".into(),
                category: FactorCategory::Growth,
                higher_is_better: true,
                description: "净利润同比增长率".into(),
                default_weight: 0.10,
                enabled: true,
            },
            FactorDefinition {
                id: "revenue_yoy".into(),
                name: "营收同比".into(),
                category: FactorCategory::Growth,
                higher_is_better: true,
                description: "营业收入同比增长率".into(),
                default_weight: 0.08,
                enabled: true,
            },
            FactorDefinition {
                id: "rsi_14".into(),
                name: "RSI(14)".into(),
                category: FactorCategory::Technical,
                higher_is_better: true,
                description: "14日相对强弱指数".into(),
                default_weight: 0.08,
                enabled: true,
            },
            FactorDefinition {
                id: "volume_ratio".into(),
                name: "量比".into(),
                category: FactorCategory::CapitalFlow,
                higher_is_better: true,
                description: "当日成交量 / 5日均量".into(),
                default_weight: 0.05,
                enabled: true,
            },
            FactorDefinition {
                id: "main_inflow".into(),
                name: "主力净流入".into(),
                category: FactorCategory::CapitalFlow,
                higher_is_better: true,
                description: "主力资金净流入额".into(),
                default_weight: 0.05,
                enabled: true,
            },
            FactorDefinition {
                id: "turnover_rate".into(),
                name: "换手率".into(),
                category: FactorCategory::Risk,
                higher_is_better: false,
                description: "换手率（高换手=高风险）".into(),
                default_weight: 0.05,
                enabled: true,
            },
        ];

        for f in defaults {
            self.register(f);
        }
    }
}
