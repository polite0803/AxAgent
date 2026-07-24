//! 策略参数优化器 — 网格搜索 + 贝叶斯优化 + 遗传算法
//!
//! 在 Walk-Forward 验证之上，自动搜索最优参数组合。
//! 当前实现网格搜索（GridSearch），贝叶斯和遗传算法为框架预留。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::engine::{BacktestConfig, BacktestEngine};
use crate::error::QuantError;
use crate::strategy::Strategy;
use crate::types::Bar;

/// 参数搜索空间定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamSpace {
    /// 参数名 → 候选值列表
    #[serde(flatten)]
    pub params: HashMap<String, Vec<serde_json::Value>>,
}

/// 一次参数扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamTrialResult {
    /// 参数组合
    pub params: HashMap<String, serde_json::Value>,
    /// 回测结果摘要
    pub total_return: f64,
    pub sharpe: f64,
    pub max_drawdown_pct: f64,
    pub win_rate: f64,
    pub total_trades: u32,
    /// Walk-Forward 综合得分（如有）
    pub wf_score: Option<f64>,
    /// 排名（1=最优）
    pub rank: u32,
}

/// 参数扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamScanResult {
    /// 策略名称
    pub strategy_name: String,
    /// 搜索空间
    pub space: ParamSpace,
    /// 所有 trial 结果（按综合得分降序）
    pub trials: Vec<ParamTrialResult>,
    /// 最优参数
    pub best_params: HashMap<String, serde_json::Value>,
    /// 最优指标
    pub best_metrics: ParamTrialResult,
    /// 搜索方法
    pub method: String,
    /// 耗时（ms）
    pub duration_ms: u64,
}

/// 参数优化器
pub struct ParamOptimizer;

impl ParamOptimizer {
    /// 网格搜索
    ///
    /// - `strategy_factory`: 接收参数 JSON 返回策略实例的闭包
    /// - `space`: 搜索空间
    /// - `klines`: 回测 K 线
    /// - `config_template`: 回测配置模板
    pub async fn grid_search(
        strategy_factory: &dyn Fn(HashMap<String, serde_json::Value>) -> Box<dyn Strategy>,
        space: ParamSpace,
        klines: Vec<Bar>,
        config_template: BacktestConfig,
    ) -> Result<ParamScanResult, QuantError> {
        let start = std::time::Instant::now();

        // 1. 生成所有参数组合
        let combinations = Self::cartesian_product(&space);
        if combinations.is_empty() {
            return Err(QuantError::Multi("参数搜索空间为空".into()));
        }

        let total = combinations.len();
        tracing::info!("[ParamOptimizer] 网格搜索开始: {total} 个组合");

        // 2. 逐个跑回测
        let mut trials: Vec<ParamTrialResult> = Vec::with_capacity(total);

        for (i, params) in combinations.into_iter().enumerate() {
            let mut strategy = strategy_factory(params.clone());
            let engine = BacktestEngine::new(config_template.clone());
            let result = engine.run(&mut *strategy, klines.clone()).await?;

            trials.push(ParamTrialResult {
                params,
                total_return: result.total_return,
                sharpe: result.sharpe,
                max_drawdown_pct: result.max_drawdown_pct,
                win_rate: result.win_rate,
                total_trades: result.total_trades as u32,
                wf_score: None,
                rank: 0,
            });

            if (i + 1) % 50 == 0 || i == total - 1 {
                tracing::info!(
                    "[ParamOptimizer] 进度: {}/{} ({:.0}%)",
                    i + 1,
                    total,
                    (i + 1) as f64 / total as f64 * 100.0
                );
            }
        }

        // 3. 按综合得分降序排列
        trials.sort_by(|a, b| {
            let score_a = a.sharpe.max(0.0) * 0.4
                + a.total_return.max(0.0) * 30.0
                + (1.0 - a.max_drawdown_pct) * 0.2
                + a.win_rate * 0.1;
            let score_b = b.sharpe.max(0.0) * 0.4
                + b.total_return.max(0.0) * 30.0
                + (1.0 - b.max_drawdown_pct) * 0.2
                + b.win_rate * 0.1;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. 赋排名
        for (i, trial) in trials.iter_mut().enumerate() {
            trial.rank = (i + 1) as u32;
        }

        let best = trials[0].clone();
        let duration_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            "[ParamOptimizer] 网格搜索完成: {total} 组合, 最优 Sharpe={:.3}, 耗时={}ms",
            best.sharpe,
            duration_ms
        );

        Ok(ParamScanResult {
            strategy_name: "grid_search".into(),
            space,
            trials: trials.clone(),
            best_params: best.params.clone(),
            best_metrics: best,
            method: "grid_search".into(),
            duration_ms,
        })
    }

    /// 生成笛卡尔积参数组合
    fn cartesian_product(space: &ParamSpace) -> Vec<HashMap<String, serde_json::Value>> {
        let param_names: Vec<&String> = space.params.keys().collect();
        let param_values: Vec<&Vec<serde_json::Value>> = space.params.values().collect();

        if param_names.is_empty() {
            return vec![HashMap::new()];
        }

        // 递归构建笛卡尔积
        let mut results = Vec::new();
        Self::cartesian_recursive(
            &param_names,
            &param_values,
            0,
            &mut HashMap::new(),
            &mut results,
        );
        results
    }

    fn cartesian_recursive(
        names: &[&String],
        values: &[&Vec<serde_json::Value>],
        depth: usize,
        current: &mut HashMap<String, serde_json::Value>,
        results: &mut Vec<HashMap<String, serde_json::Value>>,
    ) {
        if depth >= names.len() {
            results.push(current.clone());
            return;
        }

        let name = names[depth];
        for val in values[depth] {
            current.insert(name.to_string(), val.clone());
            Self::cartesian_recursive(names, values, depth + 1, current, results);
        }
        current.remove(name.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cartesian_product_two_params() {
        let mut space = ParamSpace { params: HashMap::new() };
        space.params.insert("fast".into(), vec![serde_json::json!(5), serde_json::json!(10)]);
        space.params.insert("slow".into(), vec![serde_json::json!(20), serde_json::json!(60)]);

        let combos = ParamOptimizer::cartesian_product(&space);
        assert_eq!(combos.len(), 4); // 2×2
    }

    #[test]
    fn test_cartesian_product_single_param() {
        let mut space = ParamSpace { params: HashMap::new() };
        space.params.insert(
            "period".into(),
            vec![serde_json::json!(10), serde_json::json!(20), serde_json::json!(30)],
        );

        let combos = ParamOptimizer::cartesian_product(&space);
        assert_eq!(combos.len(), 3);
    }

    #[test]
    fn test_cartesian_product_empty() {
        let space = ParamSpace { params: HashMap::new() };
        let combos = ParamOptimizer::cartesian_product(&space);
        assert_eq!(combos.len(), 1); // 空组合也算一种
    }
}
