// regime-weights.rhai — 基于市场状态的因子权重调节（决策二选 C 轻量实现）
//
// 输入：
//   market_regime_state: "bull"/"bear"/"neutral"/"bull_high_vol"/"bear_high_vol"
//   market_regime_prior: f64 (0.4-0.65)
//
// 输出：factor_weights JSON（与 portfolio-mgr 的 input_mapping f_weights 格式兼容）
//   {trend: {weight, regime_mod}, consensus: {weight, regime_mod}, ...}
//
// 原理：
//   牛市 → 趋势+资金面权重↑（趋势跟踪有效）
//   熊市 → 估值+风险权重↑（防御优先），筹码面权重↑（解禁减持在熊市杀伤力更大）
//   高波动 → 所有因子降权，风控优先
//
// P2 修复: 补充 f7(trade_signal)/f9(money_flow)/f10(chip) 的 regime 调节参数
//   trade_signal: 熊市降权（LLM 在熊市易过度悲观），高波动降权（LLM 判断可靠性下降）
//   money_flow: 牛市升权（趋势跟踪有效），熊市降权（资金面萎缩）
//   chip: 熊市升权（解禁减持在熊市杀伤力更大），高波动升权（筹码稳定性更重要）

fn present(x) { type_of(x) != "()" }

let state = if present(market_regime_state) { market_regime_state } else { "neutral" };
let prior_val = if present(market_regime_prior) { market_regime_prior } else { 0.50 };

// ── 各因子基权重 + 市场状态调节系数 ──
// P2 修复新增: money_flow (f9), chip (f10), trade_signal (f7)
let factors = #{
    "trend": #{ "base": 0.15, "bull": 1.4, "bear": 0.7, "neutral": 1.0, "volatile": 0.6 },
    "consensus": #{ "base": 0.25, "bull": 1.0, "bear": 1.0, "neutral": 1.0, "volatile": 0.8 },
    "catalyst": #{ "base": 0.20, "bull": 1.1, "bear": 0.8, "neutral": 1.2, "volatile": 0.7 },
    "risk": #{ "base": 0.15, "bull": 0.8, "bear": 1.5, "neutral": 1.0, "volatile": 1.8 },
    "valuation": #{ "base": 0.15, "bull": 0.7, "bear": 1.4, "neutral": 1.1, "volatile": 1.2 },
    "data_quality": #{ "base": 0.15, "bull": 1.0, "bear": 1.2, "neutral": 1.0, "volatile": 1.3 },
    // P2 新增的三个因子
    "trade_signal": #{ "base": 0.15, "bull": 1.1, "bear": 0.8, "neutral": 1.0, "volatile": 0.7 },
    "money_flow": #{ "base": 0.08, "bull": 1.3, "bear": 0.7, "neutral": 1.0, "volatile": 0.5 },
    "chip": #{ "base": 0.08, "bull": 0.8, "bear": 1.3, "neutral": 1.0, "volatile": 1.5 },
};

// 解析状态（含高波动后缀处理）
let regime_key = if state == "bull_high_vol" { "volatile" }
    else if state == "bear_high_vol" { "volatile" }
    else if state == "bull" || state == "bear" || state == "neutral" { state }
    else { "neutral" };

// P2 修复: 更新因子迭代列表，新增 trade_signal/money_flow/chip
let factor_names = ["trend", "consensus", "catalyst", "risk", "valuation", "data_quality",
                    "trade_signal", "money_flow", "chip"];

// 计算调节后的因子权重
let result = #{};
let total_weight = 0.0;
for name in factor_names {
    let f = factors[name];
    let base = f["base"];
    let mod_val = f[regime_key];
    let adjusted = base * mod_val;
    result[name] = #{ "weight": (adjusted * 1000.0).round() / 1000.0, "regime_mod": mod_val };
    total_weight = total_weight + adjusted;
}

// 增加先验概率
result["prior"] = prior_val;
result["regime"] = state;

#{ "factor_weights": result, "total_weight": (total_weight * 1000.0).round() / 1000.0 }
