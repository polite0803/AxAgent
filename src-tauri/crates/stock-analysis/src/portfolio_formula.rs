//! P3-1: portfolio-mgr 核心数学公式的 Rust 实现（可测试、可复用）。
//!
//! 这些函数原在 portfolio-mgr.rhai 中内联计算，迁移到 Rust 层后：
//! 1. 可通过 `register_fn` 注入 Rhai Engine（Rhai 脚本只做规则编排）
//! 2. 可写单元测试验证边界条件
//! 3. 可被 Rust 层其他模块复用（如 WFO 校准目标函数）
//!
//! 所有函数均为纯函数，无副作用，无异步。

use serde::{Deserialize, Serialize};

// ── 证据缩放 ──

/// 非线性证据缩放系数。
///
/// 旧 (Rhai 内联): `clamp(0.20 + total_weight * 0.25, 0.20, 0.50)` — 纯线性
/// 新 (P2-1): `0.10 + 0.45 * sqrt(total_weight / max_weight)` — sqrt 曲线
///
/// 特性:
/// - total_weight=0.3: ~0.31  vs 旧 0.15 → 低证据量适度放大（单因子信号仍需合理权重）
/// - total_weight=0.8: ~0.44  vs 旧 0.40 → 中证据量略高
/// - total_weight=1.4: ~0.55  vs 旧 0.50 → 满证据推高到 0.55
///
/// # 参数
/// - `total_weight`: 实际激活因子的权重和
/// - `max_weight`: 理论最大权重和（所有因子默认权重之和 = 1.44）
pub fn compute_evidence_scale(total_weight: f64, max_weight: f64) -> f64 {
    if total_weight < 0.3 || max_weight <= 0.0 {
        0.20 // 单因子保持保守（修改: 从 0.15 微升至 0.20，避免过度压制 weak signal）
    } else {
        let normalized = (total_weight / max_weight).min(1.0);
        0.10 + 0.45 * normalized.sqrt()
    }
}

// ── 凯利赔率 ──

/// 凯利赔率：`odds = (targetPrice - currentPrice) / (currentPrice - stopLoss)`
///
/// P0-1 新增：当 trader 数据缺失时，基于 posterior 强度使用保守赔率 fallback。
/// fallback 设计依据：A 股趋势突破后的盈亏比经验分布，posterior≥0.70 时对应 2.5x。
pub fn compute_kelly_odds(
    trader_target_price: Option<f64>,
    trader_stop_loss: Option<f64>,
    current_price: Option<f64>,
    posterior: f64,
) -> (f64, &'static str) {
    // 优先使用 trader 数据
    if let (Some(tp), Some(sl), Some(cp)) = (trader_target_price, trader_stop_loss, current_price) {
        if sl > 0.0 && cp > sl && tp > cp {
            let profit = tp - cp;
            let loss = cp - sl;
            if loss > 0.0 {
                let odds = (profit / loss).clamp(0.0, 10.0);
                return (odds, "trader");
            }
        }
        // trader 数据存在但不满足看多条件：
        // - targetPrice <= currentPrice（空头预测，不应做多）
        // - stopLoss >= currentPrice（无效止损，数据异常）
        // - stopLoss <= 0.0（垃圾数据）
        // → odds=0，不使用 fallback（trader 已给出明确信息）
        return (0.0, "trader_看空");
    }
    // 真正缺失 trader 数据时才使用波动率 fallback
    let odds = if posterior >= 0.70 {
        2.5
    } else if posterior >= 0.60 {
        2.0
    } else if posterior >= 0.50 {
        1.5
    } else {
        0.0
    };
    (odds, "波动率fallback")
}

// ── 凯利仓位 ──

/// 凯利仓位计算：`f* = (p × (odds + 1) - 1) / odds` → 半凯利 → 成本扣减 → 风险上限
///
/// # 参数
/// - `posterior`: 后验概率 P(上涨|证据)
/// - `odds`: 赔率（收益/亏损比）
/// - `cost_pct`: 交易成本率（默认 0.01 = 1%）
/// - `risk_level`: 风险等级（"极高风险"/"高风险"/"中风险"/其他）
///
/// # 返回
/// 建议仓位百分比（0.0 ~ 95.0）
pub fn compute_kelly_position(posterior: f64, odds: f64, cost_pct: f64, risk_level: &str) -> f64 {
    if posterior < 0.50 || odds <= 0.001 {
        return 0.0;
    }
    let q = 1.0 - posterior;
    let kelly_raw = (posterior * odds - q) / odds;
    if kelly_raw <= 0.0001 {
        return 0.0;
    }
    // 半凯利 + 交易成本扣减
    let position = kelly_raw / 2.0 * 100.0 * (1.0 - cost_pct);
    let position = position.clamp(0.0, 95.0);
    // 风险等级仓位上限
    apply_risk_cap(position, risk_level)
}

/// 根据风险等级施加仓位上限
fn apply_risk_cap(position: f64, risk_level: &str) -> f64 {
    match risk_level {
        "极高风险" | "极高" => position.min(10.0),
        "高风险" | "高" => position.min(35.0),
        "中风险" | "中" => position.min(50.0),
        _ => position, // 低风险不 cap
    }
}

// ── 风险分类（算法版）──

/// 基于量化指标的算法风险分类。
///
/// 输入指标来自 t-risk 节点（stockRiskProfile）：
/// - `volatility`: 年化波动率 (%)
/// - `sharpe`: 夏普比率
/// - `drawdown`: 最大回撤 (%)
/// - `roe`: ROE (%)
/// - `debt`: 负债率 (%)
/// - `growth`: 营收增长率 (%)
///
/// 返回 "极高风险" / "高风险" / "低风险" / "中风险"
///
/// V54 放宽阈值适配 A 股（高波动+高负债+低增长的特征）
pub fn classify_risk(
    volatility: Option<f64>,
    sharpe: Option<f64>,
    drawdown: Option<f64>,
    roe: Option<f64>,
    debt: Option<f64>,
    growth: Option<f64>,
) -> String {
    // 默认阈值（V54 A 股校准值）
    let vol = volatility.unwrap_or(0.0);
    let sp = sharpe.unwrap_or(0.0);
    let dd = drawdown.unwrap_or(0.0);
    let r = roe.unwrap_or(0.0);
    let d = debt.unwrap_or(0.0);
    let g = growth.unwrap_or(0.0);

    // 极高风险
    if (d > 85.0 && g < 0.0) || (vol > 60.0 && sp < -1.5) {
        return "极高风险".into();
    }

    // 高风险
    if (vol > 40.0 || sp < 0.0 || dd > 45.0) && (r < 5.0 || d > 65.0) {
        return "高风险".into();
    }
    if vol > 35.0 && sp < 0.3 && r < 8.0 && g < 5.0 {
        return "高风险".into();
    }

    // 低风险
    if vol < 25.0 && sp > 0.5 && dd < 30.0 && r > 8.0 && d < 55.0 && g > 3.0 {
        return "低风险".into();
    }

    "中风险".into()
}

// ── 因子协方差衰减 ──

/// 因子协方差衰减：对高相关因子对做权重降权，减少信号重复计数。
///
/// 朴素贝叶斯假设因子条件独立，但以下因子对不满足：
/// - f3(催化剂) ↔ f11(PACE 情绪)：同源公告数据，重叠约 70%
/// - f1(技术面) ↔ f9(资金面)：趋势-资金共振
///
/// # 参数
/// - `f1_weight`: 技术面因子权重
/// - `f3_weight`: 催化剂因子权重
/// - `f9_weight`: 资金面因子权重
/// - `f11_weight`: PACE 情绪因子权重
///
/// # 返回
/// `(f9_weight_after, f11_weight_after)` — 衰减后的权重
pub fn apply_covariance_decay(
    f1_weight: f64,
    f3_weight: f64,
    f9_weight: f64,
    f11_weight: f64,
) -> (f64, f64) {
    let mut f9 = f9_weight;
    let mut f11 = f11_weight;

    // f3 ↔ f11: 公告数据重叠 → f11 降权 35%
    if f3_weight > 0.0 && f11_weight > 0.0 {
        f11 *= 0.65;
    }
    // f1 ↔ f9: 趋势-资金共振 → f9 降权 25%
    if f1_weight > 0.0 && f9_weight > 0.0 {
        f9 *= 0.75;
    }

    (f9, f11)
}

// ── 决策动作 ──

/// 根据 effective_posterior 和仓位计算决策动作。
///
/// # 参数
/// - `effective_posterior`: 含 risk_bias 的后验概率 [0, 1]
/// - `position_pct`: 计算后的仓位 [0, 100]
/// - `buy_threshold`: 买入阈值（默认 0.63）
/// - `increase_threshold`: 增持阈值（默认 0.53）
/// - `hold_threshold`: 持有阈值（默认 0.48）
/// - `watch_threshold`: 观望阈值（默认 0.38）
/// - `reduce_threshold`: 减持阈值（默认 0.30）
/// - `pos_buy_min`: 买入所需最小仓位（默认 15.0%）
/// - `pos_increase_min`: 增持所需最小仓位（默认 10.0%）
///
/// # 返回
/// "买入" / "增持" / "持有" / "观望" / "减持" / "卖出"
#[allow(clippy::too_many_arguments)]
pub fn compute_action(
    effective_posterior: f64,
    position_pct: f64,
    buy_threshold: f64,
    increase_threshold: f64,
    hold_threshold: f64,
    watch_threshold: f64,
    reduce_threshold: f64,
    pos_buy_min: f64,
    pos_increase_min: f64,
) -> String {
    if effective_posterior >= buy_threshold && position_pct >= pos_buy_min {
        "买入".into()
    } else if effective_posterior >= increase_threshold && position_pct >= pos_increase_min {
        "增持".into()
    } else if effective_posterior >= hold_threshold {
        "持有".into()
    } else if effective_posterior >= watch_threshold {
        "观望".into()
    } else if effective_posterior >= reduce_threshold {
        "减持".into()
    } else {
        "卖出".into()
    }
}

/// 计算 risk_bias（根据风险等级的行为阈值偏移）
pub fn compute_risk_bias(risk_level: &str) -> f64 {
    match risk_level {
        "极高风险" | "极高" => -0.15,
        "高风险" | "高" => -0.08,
        "低风险" | "低" => 0.05,
        _ => 0.0,
    }
}

/// 应用风控否决：根据风险等级限制决策动作。
///
/// 返回 (final_action, was_downgraded, note)
pub fn apply_risk_veto(action: &str, risk_level: &str) -> (String, bool, String) {
    if matches!(risk_level, "极高风险" | "极高") && matches!(action, "买入" | "增持" | "持有")
    {
        return ("观望".into(), true, "极高风险风控否决：禁止持仓".into());
    }
    if matches!(risk_level, "高风险" | "高") && matches!(action, "买入" | "增持") {
        return ("持有".into(), true, "高风险风控否决：禁止加仓".into());
    }
    (action.to_string(), false, String::new())
}
// ── P3-2: Portfolio-mgr 参数集（WFO 校准目标）──

/// portfolio-mgr 的可校准参数集合。
///
/// 这些参数当前在 portfolio-mgr.rhai 中硬编码为 V56 校准的默认值。
/// 通过 WFO 定期跑 param scan，可自动寻找适合当前市场状态的最优参数。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioMgrParamSet {
    pub buy_threshold: f64,
    pub increase_threshold: f64,
    pub hold_threshold: f64,
    pub watch_threshold: f64,
    pub reduce_threshold: f64,
    pub cap_extreme: f64,
    pub cap_high: f64,
    pub cap_mid: f64,
}

impl PortfolioMgrParamSet {
    /// V56 校准默认值
    pub fn v56_default() -> Self {
        /* ... */
        Self {
            buy_threshold: 0.63,
            increase_threshold: 0.53,
            hold_threshold: 0.48,
            watch_threshold: 0.38,
            reduce_threshold: 0.30,
            cap_extreme: 10.0,
            cap_high: 35.0,
            cap_mid: 50.0,
        }
    }
    pub fn conservative() -> Self {
        /* ... */
        Self {
            buy_threshold: 0.68,
            increase_threshold: 0.58,
            hold_threshold: 0.52,
            watch_threshold: 0.42,
            reduce_threshold: 0.35,
            cap_extreme: 8.0,
            cap_high: 28.0,
            cap_mid: 40.0,
        }
    }
    pub fn aggressive() -> Self {
        /* ... */
        Self {
            buy_threshold: 0.58,
            increase_threshold: 0.48,
            hold_threshold: 0.42,
            watch_threshold: 0.32,
            reduce_threshold: 0.25,
            cap_extreme: 15.0,
            cap_high: 45.0,
            cap_mid: 60.0,
        }
    }
    pub fn default_grid() -> Vec<Self> {
        vec![
            Self::conservative(),
            Self {
                buy_threshold: 0.65,
                increase_threshold: 0.55,
                hold_threshold: 0.50,
                watch_threshold: 0.40,
                reduce_threshold: 0.32,
                ..Self::v56_default()
            },
            Self::v56_default(),
            Self {
                buy_threshold: 0.60,
                increase_threshold: 0.50,
                hold_threshold: 0.45,
                watch_threshold: 0.35,
                reduce_threshold: 0.28,
                ..Self::v56_default()
            },
            Self::aggressive(),
            Self {
                buy_threshold: 0.70,
                increase_threshold: 0.60,
                hold_threshold: 0.55,
                watch_threshold: 0.45,
                reduce_threshold: 0.35,
                cap_extreme: 5.0,
                cap_high: 20.0,
                cap_mid: 30.0,
            },
            Self {
                buy_threshold: 0.55,
                increase_threshold: 0.45,
                hold_threshold: 0.40,
                watch_threshold: 0.30,
                reduce_threshold: 0.22,
                cap_extreme: 20.0,
                cap_high: 50.0,
                cap_mid: 70.0,
            },
            Self { buy_threshold: 0.66, increase_threshold: 0.56, ..Self::v56_default() },
            Self { buy_threshold: 0.60, increase_threshold: 0.50, ..Self::v56_default() },
        ]
    }
}

// ── Path 2: 从 LLM 反思输出解析 ParamSet ──

/// 从 reflection 的 `parameter_suggestions_json` 解析 `PortfolioMgrParamSet`。
///
/// LLM 输出的格式可以是：
/// - 完整 JSON 对象: `{"buy_threshold":0.60, "cap_high":30.0, ...}`
/// - 部分 JSON: `{"buy_threshold":0.60}`（未指定的字段用默认值填充）
/// - 数组格式（兼容旧格式）: `[{"key":"buy_threshold","value":0.60}, ...]`
///
/// 返回 `None` 表示解析失败（非法的 JSON 或空输入）。
pub fn try_parse_param_suggestion(json_str: &str) -> Option<PortfolioMgrParamSet> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() || trimmed == "null" || trimmed == "{}" {
        return None;
    }

    // 尝试解析为完整对象或数组
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // 顶层是数组 → 数组格式: [{"key":"buy_threshold","value":0.60}, ...]
        if let Some(arr) = v.as_array() {
            return parse_key_value_array(arr);
        }
        // 顶层是对象
        if let Some(obj) = v.as_object() {
            // 检查是否有结构化字段名
            let has_structured = obj.keys().any(|k| {
                matches!(
                    k.as_str(),
                    "buy_threshold"
                        | "increase_threshold"
                        | "hold_threshold"
                        | "cap_extreme"
                        | "cap_high"
                        | "cap_mid"
                )
            });
            if has_structured {
                let def = PortfolioMgrParamSet::v56_default();
                return Some(PortfolioMgrParamSet {
                    buy_threshold: obj
                        .get("buy_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.buy_threshold),
                    increase_threshold: obj
                        .get("increase_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.increase_threshold),
                    hold_threshold: obj
                        .get("hold_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.hold_threshold),
                    watch_threshold: obj
                        .get("watch_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.watch_threshold),
                    reduce_threshold: obj
                        .get("reduce_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.reduce_threshold),
                    cap_extreme: obj
                        .get("cap_extreme")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(def.cap_extreme),
                    cap_high: obj.get("cap_high").and_then(|v| v.as_f64()).unwrap_or(def.cap_high),
                    cap_mid: obj.get("cap_mid").and_then(|v| v.as_f64()).unwrap_or(def.cap_mid),
                });
            }
            // 对象内藏数组: {"params":[{"key":"buy_threshold","value":0.60}, ...]}
            if let Some(arr) = obj.values().find_map(|v| v.as_array()) {
                return parse_key_value_array(arr);
            }
        }
    }

    // 尝试整段解析为 PortfolioMgrParamSet
    serde_json::from_str::<PortfolioMgrParamSet>(trimmed).ok()
}

/// 解析 key-value 数组: [{"key":"buy_threshold","value":0.60}, ...]
fn parse_key_value_array(arr: &[serde_json::Value]) -> Option<PortfolioMgrParamSet> {
    let mut p = PortfolioMgrParamSet::v56_default();
    for entry in arr {
        if let Some(e) = entry.as_object() {
            if let (Some(k), Some(v)) =
                (e.get("key").and_then(|k| k.as_str()), e.get("value").and_then(|v| v.as_f64()))
            {
                match k {
                    "buy_threshold" | "buyThreshold" => p.buy_threshold = v,
                    "increase_threshold" | "increaseThreshold" => p.increase_threshold = v,
                    "hold_threshold" | "holdThreshold" => p.hold_threshold = v,
                    "watch_threshold" | "watchThreshold" => p.watch_threshold = v,
                    "reduce_threshold" | "reduceThreshold" => p.reduce_threshold = v,
                    "cap_extreme" | "capExtreme" => p.cap_extreme = v,
                    "cap_high" | "capHigh" => p.cap_high = v,
                    "cap_mid" | "capMid" => p.cap_mid = v,
                    _ => {},
                }
            }
        }
    }
    Some(p)
}

// ── Path 1: 基于历史反思的参数校准评分 ──

/// 对一组参数进行历史回溯评分，返回 (score, 说明)。
///
/// 评分规则（基于反思 verdict）：
/// - reflection.verdict = "correct" 且参数与当前一致 → +1
/// - reflection.verdict = "wrong"   且参数有差异  → +0.5（证明调整方向正确）
/// - reflection.verdict = "wrong"   且参数无修改  → -0.5（未修正错误）
///
/// 实际使用时应取所有已有反思的加权均值。
pub fn score_param_set(
    params: &PortfolioMgrParamSet,
    suggestions: &[(String /* verdict */, Option<PortfolioMgrParamSet>)],
) -> f64 {
    if suggestions.is_empty() {
        return 0.0;
    }
    let default = PortfolioMgrParamSet::v56_default();
    let total: f64 = suggestions
        .iter()
        .map(|(verdict, suggested)| {
            let suggested = suggested.as_ref().unwrap_or(&default);
            let changed = params != suggested;
            match verdict.as_str() {
                "correct" if !changed => 1.0, // 正确决策未改参数 → 当前参数正确
                "correct" => 0.3,             // 正确但改了参数 → 可能是噪声
                "wrong" if changed => 0.5,    // 错误决策有改参数尝试 → 方向对
                "wrong" => -0.5,              // 错误但没改 → 参数有问题
                "partial" if changed => 0.2,  // 部分正确有改 → 有点用
                "partial" => 0.0,             // 部分正确没改 → 中性
                _ => 0.0,
            }
        })
        .sum();
    (total * 100.0).round() / 100.0 / suggestions.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── evidence_scale ──

    #[test]
    fn evidence_scale_low_weight() {
        let result = compute_evidence_scale(0.2, 1.44);
        assert!((result - 0.20).abs() < 0.01, "low weight: got {result}");
    }

    #[test]
    fn evidence_scale_mid_weight() {
        let result = compute_evidence_scale(0.7, 1.44);
        // 0.10 + 0.45 * sqrt(0.7/1.44) = 0.10 + 0.45 * 0.697 = ~0.414
        assert!(result > 0.35 && result < 0.45, "mid weight out of range: {result}");
    }

    #[test]
    fn evidence_scale_full_weight() {
        let result = compute_evidence_scale(1.44, 1.44);
        assert!((result - 0.55).abs() < 0.01, "full weight: got {result}");
    }

    #[test]
    fn evidence_scale_zero_max() {
        let result = compute_evidence_scale(0.5, 0.0);
        assert!((result - 0.20).abs() < 0.01, "zero max: got {result}");
    }

    // ── 凯利赔率 ──

    #[test]
    fn kelly_odds_trader_data() {
        let (odds, source) = compute_kelly_odds(Some(25.0), Some(18.0), Some(20.0), 0.6);
        assert!((odds - 2.5).abs() < 0.01, "trader odds: got {odds}");
        assert_eq!(source, "trader");
    }

    #[test]
    fn kelly_odds_no_trader_high_posterior() {
        let (odds, source) = compute_kelly_odds(None, None, None, 0.75);
        assert!((odds - 2.5).abs() < 0.01, "fallback high: got {odds}");
        assert_eq!(source, "波动率fallback");
    }

    #[test]
    fn kelly_odds_no_trader_low_posterior() {
        let (odds, source) = compute_kelly_odds(None, None, None, 0.45);
        assert!((odds - 0.0).abs() < 0.01, "fallback low: got {odds}");
        assert_eq!(source, "波动率fallback");
    }

    #[test]
    fn kelly_odds_bearish_case() {
        // targetPrice < currentPrice → trader 看空，odds=0，不用 fallback
        let (odds, source) = compute_kelly_odds(Some(8.0), Some(7.0), Some(20.0), 0.6);
        assert!((odds - 0.0).abs() < 0.01, "bearish odds: got {odds}");
        assert_eq!(source, "trader_看空");
    }

    #[test]
    fn kelly_odds_stop_loss_greater_than_price() {
        // stopLoss >= currentPrice → 无效数据，odds=0
        let (odds, source) = compute_kelly_odds(Some(25.0), Some(22.0), Some(20.0), 0.6);
        assert!((odds - 0.0).abs() < 0.01, "invalid stop loss: got {odds}");
        assert_eq!(source, "trader_看空");
    }

    // ── 凯利仓位 ──

    #[test]
    fn kelly_position_normal() {
        let pos = compute_kelly_position(0.65, 2.5, 0.01, "低风险");
        // kelly_raw = (0.65*2.5-0.35)/2.5 = 0.51
        // half=0.255, ×100=25.5, ×(1-0.01)=25.245
        assert!((pos - 25.245).abs() < 0.01, "kelly pos: got {pos}");
    }

    #[test]
    fn kelly_position_low_posterior() {
        let pos = compute_kelly_position(0.45, 2.5, 0.01, "低风险");
        assert_eq!(pos, 0.0, "low posterior should give zero");
    }

    #[test]
    fn kelly_position_high_risk_cap() {
        let pos = compute_kelly_position(0.70, 5.0, 0.01, "高风险");
        // kelly_raw=(0.7*5-0.3)/5=0.64, half=0.32, ×100×0.99=31.68, cap 35
        assert!((pos - 31.68).abs() < 0.1, "kelly high risk: got {pos}");
        assert!(pos <= 35.0, "high risk exceeded cap: {pos}");
    }

    #[test]
    fn kelly_position_extreme_risk_cap() {
        let pos = compute_kelly_position(0.70, 5.0, 0.01, "极高风险");
        assert!(pos <= 10.0, "extreme risk exceeded cap: {pos}");
    }

    // ── 风险分类 ──

    #[test]
    fn risk_classify_low() {
        let level =
            classify_risk(Some(20.0), Some(1.2), Some(15.0), Some(12.0), Some(40.0), Some(10.0));
        assert_eq!(level, "低风险", "should be low risk");
    }

    #[test]
    fn risk_classify_high_vol_sharpe() {
        let level =
            classify_risk(Some(50.0), Some(-0.2), Some(35.0), Some(3.0), Some(70.0), Some(2.0));
        assert_eq!(level, "高风险", "should be high risk");
    }

    #[test]
    fn risk_classify_extreme_debt() {
        let level =
            classify_risk(Some(30.0), Some(0.5), Some(20.0), Some(5.0), Some(90.0), Some(-5.0));
        assert_eq!(level, "极高风险", "should be extreme risk");
    }

    #[test]
    fn risk_classify_none_data() {
        let level = classify_risk(None, None, None, None, None, None);
        assert_eq!(level, "中风险", "no data should default to mid risk");
    }

    // ── 协方差衰减 ──

    #[test]
    fn covariance_decay_f3_f11() {
        // f1_weight=0.15 也会触发 f1↔f9 衰减，f9 从 0.08 → 0.06
        let (f9, f11) = apply_covariance_decay(0.15, 0.20, 0.08, 0.08);
        assert!((f11 - 0.08 * 0.65).abs() < 0.001, "f11 decay: got {f11}");
        assert!((f9 - 0.08 * 0.75).abs() < 0.001, "f9 decayed by f1↔f9: got {f9}");
    }

    #[test]
    fn covariance_decay_f1_f9() {
        let (f9, f11) = apply_covariance_decay(0.15, 0.0, 0.08, 0.0);
        assert!((f9 - 0.08 * 0.75).abs() < 0.001, "f9 decay: got {f9}");
    }

    #[test]
    fn covariance_decay_no_overlap() {
        let (f9, f11) = apply_covariance_decay(0.0, 0.0, 0.08, 0.0);
        assert!((f9 - 0.08).abs() < 0.001, "f9 no decay: got {f9}");
        assert!((f11 - 0.0).abs() < 0.001, "f11 no decay: got {f11}");
    }

    // ── 决策动作 ──

    #[test]
    fn action_buy() {
        let a = compute_action(0.70, 20.0, 0.63, 0.53, 0.48, 0.38, 0.30, 15.0, 10.0);
        assert_eq!(a, "买入");
    }

    #[test]
    fn action_buy_insufficient_position() {
        let a = compute_action(0.70, 5.0, 0.63, 0.53, 0.48, 0.38, 0.30, 15.0, 10.0);
        // effective_posterior >= 0.63 但 position < 15%，降级到下一个匹配阈值
        // next: 0.63 >= 0.53 但 position(5) < pos_increase_min(10) → 跳过
        // 0.70 >= 0.48 → 持有
        assert_eq!(a, "持有");
    }

    #[test]
    fn action_hold() {
        let a = compute_action(0.55, 10.0, 0.63, 0.53, 0.48, 0.38, 0.30, 15.0, 10.0);
        assert_eq!(a, "增持");
    }

    #[test]
    fn action_sell() {
        let a = compute_action(0.25, 0.0, 0.63, 0.53, 0.48, 0.38, 0.30, 15.0, 10.0);
        assert_eq!(a, "卖出");
    }

    // ── 风险否决 ──

    #[test]
    fn risk_veto_high_risk_buy() {
        let (action, was_down, _) = apply_risk_veto("买入", "高风险");
        assert_eq!(action, "持有");
        assert!(was_down);
    }

    #[test]
    fn risk_veto_extreme_risk_hold() {
        let (action, was_down, _) = apply_risk_veto("持有", "极高风险");
        assert_eq!(action, "观望");
        assert!(was_down);
    }

    #[test]
    fn risk_veto_low_risk_hold() {
        let (action, was_down, _) = apply_risk_veto("持有", "低风险");
        assert_eq!(action, "持有");
        assert!(!was_down);
    }

    // ── Path 2: try_parse_param_suggestion ──

    #[test]
    fn parse_full_object() {
        let result =
            try_parse_param_suggestion(r#"{"buy_threshold":0.70,"cap_high":20.0}"#).unwrap();
        assert!((result.buy_threshold - 0.70).abs() < 0.001);
        assert!((result.cap_high - 20.0).abs() < 0.001);
        // 未指定的字段应使用 V56 默认
        assert!((result.increase_threshold - 0.53).abs() < 0.001);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(try_parse_param_suggestion("").is_none());
        assert!(try_parse_param_suggestion("null").is_none());
        assert!(try_parse_param_suggestion("{}").is_none());
    }

    #[test]
    fn parse_array_format() {
        let result = try_parse_param_suggestion(
            r#"[{"key":"buy_threshold","value":0.60},{"key":"cap_high","value":30.0}]"#,
        )
        .unwrap();
        assert!((result.buy_threshold - 0.60).abs() < 0.001);
        assert!((result.cap_high - 30.0).abs() < 0.001);
        assert!((result.hold_threshold - 0.48).abs() < 0.001); // default
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(try_parse_param_suggestion("not json").is_none());
        assert!(try_parse_param_suggestion("123").is_none());
    }

    // ── Path 1: score_param_set ──

    #[test]
    fn score_no_suggestions() {
        let p = PortfolioMgrParamSet::v56_default();
        assert!((score_param_set(&p, &[])).abs() < 0.001);
    }

    #[test]
    fn score_correct_no_change() {
        let p = PortfolioMgrParamSet::v56_default();
        let suggestions = vec![("correct".into(), None)];
        assert!((score_param_set(&p, &suggestions) - 1.0).abs() < 0.001);
    }

    #[test]
    fn score_wrong_no_change_negative() {
        let p = PortfolioMgrParamSet::v56_default();
        let suggestions = vec![("wrong".into(), None)];
        assert!((score_param_set(&p, &suggestions) - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn score_wrong_with_change_positive() {
        // 当前活跃参数
        let p = PortfolioMgrParamSet { buy_threshold: 0.60, ..PortfolioMgrParamSet::v56_default() };
        // 反思建议的参数（与活跃参数不同 → "有改"）
        let suggested =
            PortfolioMgrParamSet { buy_threshold: 0.55, ..PortfolioMgrParamSet::v56_default() };
        let suggestions = vec![("wrong".into(), Some(suggested))];
        let score = score_param_set(&p, &suggestions);
        assert!((score - 0.5).abs() < 0.001, "expected 0.5, got {score}");
    }
}
