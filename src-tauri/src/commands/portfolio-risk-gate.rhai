// portfolio-risk-gate.rhai — P1-E13 组合风控门
//
// 在 portfolio-mgr 之后、rule-check 之前运行。
// 职责：
//   1. 单股仓位上限检查（PositionLimits::max_single_stock_pct）
//   2. 行业暴露上限检查（max_sector_exposure_pct）
//   3. 持仓数量上限检查（max_total_positions）
//   4. 风险档位否决（RiskTier::forbid_new_position）
//   5. 空头目标价强制卖出（BEARISH_TARGET_PRICE_RATIO=0.85）
//   6. 组合总仓位归一化（normalize_position_weights）
//
// 设计原则：
//   - 保留 portfolio-mgr 的所有输出字段，仅覆盖被调整的 action/positionPct/riskLevel
//   - 追加 risk_gate 元数据（adjusted/reasons/concentrationWarning 等）
//   - 保守降级：任何异常都不阻塞决策，透传 portfolio-mgr 原始结果
//
// 输入映射（input_mapping）：
//   pm_result       ← portfolio-mgr.result           （整个 portfolio-mgr 输出对象）
//   current_price   ← t-scoring.result.currentPrice   （当前价）
//   target_price    ← trader.content.targetPrice       （目标价，用于空头检测）
//   stock_code      ← stock_code                       （工作流变量，core.rs 注入）
//   stock_sector    ← stock_sector                     （工作流变量，core.rs 注入）
//   holdings_json   ← holdings_json                    （工作流变量，core.rs 注入）
//   portfolio_cash  ← portfolio_cash                   （工作流变量，core.rs 注入）

// ── Rhai 全局辅助函数（必须位于全局作用域，Rhai 不允许在 try/块内定义函数）──
fn present(x) { type_of(x) != "()" }

fn safe_str(x) {
    if !present(x) { return ""; }
    if type_of(x) == "string" { x } else { "" }
}

fn safe_f64(x, fallback) {
    if !present(x) { return fallback; }
    if type_of(x) == "f64" { x } else { fallback }
}

fn safe_opt_f64(x) {
    if !present(x) { return (); }
    if type_of(x) == "f64" { x } else { () }
}

// P1-E13 修复: 返回 Option<&str> 而非 &str
// 避免 safe_str 把未设置的 stock_sector 转为 ""，
// 导致 Rust 侧 Option<&str> 收到 Some("") 误判为有效行业。
// 未设置或空字符串 → () (即 Rust None)
fn safe_opt_str(x) {
    if !present(x) { return (); }
    if type_of(x) != "string" { return (); }
    if x == "" { return (); }
    x
}

fn safe_bool(x, fallback) {
    if !present(x) { return fallback; }
    if type_of(x) == "bool" { x } else { fallback }
}

// 将 reasons 数组拼接为分号分隔的字符串
fn join_reasons(arr) {
    if type_of(arr) != "array" { return ""; }
    let text = "";
    for i in 0..arr.len() {
        let r = arr[i];
        if type_of(r) == "string" {
            if text == "" {
                text = r;
            } else {
                text = `${text}; ${r}`;
            }
        }
    }
    text
}

// ── 防御性兜底：任何 Rhai 运行时错误都返回可解析的保守决策 ──
let gate_result = #{
    "action": "观望",
    "positionPct": 0.0,
    "confidence": 50.0,
    "riskLevel": "中风险",
    "reasoning": "portfolio-risk-gate 兜底：未执行",
};

try {

// 1. 读取 portfolio-mgr 结果
if !present(pm_result) || type_of(pm_result) != "map" {
    // portfolio-mgr 无输出 — 保守降级
    gate_result = #{
        "action": "观望",
        "positionPct": 0.0,
        "confidence": 50.0,
        "riskLevel": "中风险",
        "reasoning": "portfolio-risk-gate: portfolio-mgr 无输出，保守降级",
        "risk_gate": #{
            "adjusted": true,
            "originalAction": "",
            "originalPositionPct": 0.0,
            "reasons": ["portfolio-mgr 无输出"],
            "concentrationWarning": "",
            "topHoldingPct": 0.0,
            "maxSectorPct": 0.0,
        },
    };
} else {

let orig_action = safe_str(pm_result["action"]);
let orig_position_pct = safe_f64(pm_result["positionPct"], 0.0);
let orig_risk_level = safe_str(pm_result["riskLevel"]);

// 2. 调用 pm_portfolio_risk_gate 纯函数（注册在共享 Rhai Engine 中）
let gate_json = pm_portfolio_risk_gate(
    orig_action,
    orig_position_pct,
    orig_risk_level,
    safe_f64(current_price, 0.0),
    safe_opt_f64(target_price),
    safe_str(stock_code),
    safe_opt_str(stock_sector),
    safe_str(holdings_json),
    safe_f64(portfolio_cash, 0.0)
);

let gate = json_parse(gate_json);

if type_of(gate) != "map" {
    // 解析失败 — 透传 portfolio-mgr 结果，追加错误标记
    gate_result = pm_result;
    gate_result["risk_gate"] = #{
        "adjusted": false,
        "originalAction": orig_action,
        "originalPositionPct": orig_position_pct,
        "reasons": ["风控门 JSON 解析失败，未做调整"],
        "concentrationWarning": "",
        "topHoldingPct": 0.0,
        "maxSectorPct": 0.0,
    };
} else {
    let final_action = safe_str(gate["action"]);
    let final_position_pct = safe_f64(gate["positionPct"], orig_position_pct);
    let adjusted = safe_bool(gate["adjusted"], false);
    let concentration_warning = safe_str(gate["concentrationWarning"]);
    let top_holding_pct = safe_f64(gate["topHoldingPct"], 0.0);
    let max_sector_pct = safe_f64(gate["maxSectorPct"], 0.0);
    let reasons = gate["reasons"];

    // 3. 输出：保留 portfolio-mgr 的所有字段，覆盖被调整的字段，追加 risk_gate 元数据
    //    Rhai 的 map 赋值是值拷贝，修改 gate_result 不影响 pm_result
    gate_result = pm_result;
    gate_result["action"] = final_action;
    gate_result["positionPct"] = (final_position_pct * 10.0).round() / 10.0;
    gate_result["positionPctNormalized"] = (final_position_pct * 10.0).round() / 10.0;

    // 如果风控门调整了决策，追加 reasoning 说明
    if adjusted {
        let reasons_text = join_reasons(reasons);
        let orig_reasoning = safe_str(pm_result["reasoning"]);
        if reasons_text != "" {
            gate_result["reasoning"] = `${orig_reasoning} | [风控门] ${reasons_text}`;
        }
    }

    gate_result["risk_gate"] = #{
        "adjusted": adjusted,
        "originalAction": safe_str(gate["originalAction"]),
        "originalPositionPct": safe_f64(gate["originalPositionPct"], 0.0),
        "reasons": reasons,
        "concentrationWarning": concentration_warning,
        "topHoldingPct": top_holding_pct,
        "maxSectorPct": max_sector_pct,
    };
}

} // end if pm_result present

}
catch (err) {
    // 异常兜底：如果 pm_result 存在则透传，否则返回保守决策
    if present(pm_result) && type_of(pm_result) == "map" {
        gate_result = pm_result;
        gate_result["risk_gate"] = #{
            "adjusted": false,
            "originalAction": "",
            "originalPositionPct": 0.0,
            "reasons": ["风控门异常: " + err],
            "concentrationWarning": "",
            "topHoldingPct": 0.0,
            "maxSectorPct": 0.0,
        };
    } else {
        gate_result = #{
            "action": "观望",
            "positionPct": 0.0,
            "confidence": 50.0,
            "riskLevel": "中风险",
            "reasoning": "portfolio-risk-gate 异常且 portfolio-mgr 无输出: " + err,
            "risk_gate": #{
                "adjusted": false,
                "originalAction": "",
                "originalPositionPct": 0.0,
                "reasons": ["风控门异常: " + err],
                "concentrationWarning": "",
                "topHoldingPct": 0.0,
                "maxSectorPct": 0.0,
            },
        };
    }
}

gate_result
