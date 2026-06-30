//! 最小测试：用 rhai v1.25.0 编译 bottleneck-calc.rhai v9，确认语法问题。
use rhai::{Engine, Scope};

#[test]
fn bottleneck_calc_v9_compiles() {
    let code = include_str!("../../../src/commands/bottleneck-calc.rhai");
    let mut engine = Engine::new();
    engine.set_max_expr_depths(1024, 1024);
    engine.register_fn("clamp", |v: f64, min: f64, max: f64| -> f64 {
        if v < min {
            min
        } else if v > max {
            max
        } else {
            v
        }
    });
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    engine.register_fn("json_parse", |_s: &str| -> rhai::Dynamic { rhai::Dynamic::UNIT });
    let mut scope = Scope::new();
    // 注入脚本需要的输入变量（空值即可，只测编译）
    scope.push_constant("baseline_semi", ());
    scope.push_constant("baseline_battery", ());
    scope.push_constant("baseline_chem", ());
    scope.push_constant("baseline_med", ());
    scope.push_constant("baseline_aero", ());
    scope.push_constant("baseline_consumer_elec", ());
    scope.push_constant("baseline_auto", ());
    scope.push_constant("chain_analysis", ());
    scope.push_constant("industry_ranking", ());

    match engine.compile(code) {
        Ok(_) => eprintln!("=== PARSE OK ==="),
        Err(e) => panic!("编译失败: {e}"),
    }
}

fn portfolio_mgr_scope() -> Scope<'static> {
    let mut s = Scope::new();
    // 注入 portfolio-mgr.rhai 需要的所有输入变量（空值即可，只测编译）
    s.push_constant("market_regime_prior", 0.50f64);
    s.push_constant("factor_weights", ());
    s.push_constant("screening_source", ());
    s.push_constant("risk_volatility", 35.0f64);
    s.push_constant("risk_drawdown", 25.0f64);
    s.push_constant("risk_sharpe", 0.3f64);
    s.push_constant("risk_roe", 10.0f64);
    s.push_constant("risk_gross_margin", 25.0f64);
    s.push_constant("risk_debt_ratio", 50.0f64);
    s.push_constant("risk_revenue_growth", 10.0f64);
    s.push_constant("risk_pe", 20.0f64);
    s.push_constant("overall_risk", ());
    s.push_constant("totalScore", 60.0f64);
    s.push_constant("consensusScore", 55.0f64);
    s.push_constant("catalyst_level", ());
    s.push_constant("valuation_dcf_upside", ());
    s.push_constant("valuation_graham_upside", ());
    s.push_constant("valuation_fscore", ());
    s.push_constant("dqi_score", 70.0f64);
    s.push_constant("trader_target_price", ());
    s.push_constant("trader_stop_loss", ());
    s.push_constant("current_price", 20.0f64);
    s.push_constant("trader_time_horizon", ());
    s.push_constant("trader_holding_days", ());
    s.push_constant("market_regime_state", ());
    s.push_constant("risk_disagreement", ());
    s.push_constant("serenity_context", ());
    s
}

#[test]
fn portfolio_mgr_v54_compiles() {
    let code = include_str!("../../../src/commands/portfolio-mgr.rhai");
    let engine = portfolio_mgr_engine();
    let scope = portfolio_mgr_scope();
    match engine.compile_with_scope(&scope, code) {
        Ok(_) => eprintln!("=== PORTFOLIO-MGR PARSE OK ==="),
        Err(e) => panic!("编译 portfolio-mgr 失败: {e}"),
    }
}

fn portfolio_mgr_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(2048, 2048);
    engine.register_fn("clamp", |v: f64, min: f64, max: f64| -> f64 {
        if v < min {
            min
        } else if v > max {
            max
        } else {
            v
        }
    });
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    engine.register_fn("type_of", |v: rhai::Dynamic| -> String {
        // 简化: () 类型返回 "()", 其他返回 "f64"
        if v.is_unit() {
            "()".to_string()
        } else {
            "f64".to_string()
        }
    });
    engine.register_fn("present", |v: rhai::Dynamic| -> bool { !v.is_unit() });
    engine.register_fn("print", |_s: &str| {});
    engine.register_fn("to_string", |v: f64| -> String { v.to_string() });
    engine.register_fn("to_string", |v: i64| -> String { v.to_string() });
    engine
}

#[test]
fn data_verifier_compiles() {
    let code = include_str!("../../../src/commands/data-verifier.rhai");
    let mut engine = Engine::new();
    engine.set_max_expr_depths(2048, 2048);
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    engine.register_fn("json_parse", |_s: &str| -> rhai::Dynamic { rhai::Dynamic::UNIT });
    engine.register_fn("type_of", |v: rhai::Dynamic| -> String {
        if v.is_unit() {
            "()".to_string()
        } else {
            "f64".to_string()
        }
    });
    engine.register_fn("present", |v: rhai::Dynamic| -> bool { !v.is_unit() });
    engine.register_fn("print", |_s: &str| {});
    engine.register_fn("to_string", |v: f64| -> String { v.to_string() });
    engine.register_fn("to_string", |v: i64| -> String { v.to_string() });
    let mut scope = Scope::new();
    scope.push_constant("candidates", ());
    scope.push_constant("candidates_direct", ());
    scope.push_constant("tool_calls_made", ());
    match engine.compile_with_scope(&scope, code) {
        Ok(_) => eprintln!("=== DATA-VERIFIER PARSE OK ==="),
        Err(e) => panic!("编译 data-verifier 失败: {e}"),
    }
}

#[test]
fn consistency_check_compiles() {
    let code = include_str!("../../../src/commands/consistency-check.rhai");
    let mut engine = Engine::new();
    engine.set_max_expr_depths(2048, 2048);
    engine.register_fn("type_of", |v: rhai::Dynamic| -> String {
        if v.is_unit() {
            "()".to_string()
        } else {
            "f64".to_string()
        }
    });
    engine.register_fn("present", |v: rhai::Dynamic| -> bool { !v.is_unit() });
    engine.register_fn("print", |_s: &str| {});
    engine.register_fn("to_string", |v: f64| -> String { v.to_string() });
    engine.register_fn("to_string", |v: i64| -> String { v.to_string() });
    let mut scope = Scope::new();
    scope.push_constant("chain_node_trend1", ());
    scope.push_constant("chain_node_trend2", ());
    scope.push_constant("chain_node_trend3", ());
    scope.push_constant("chain_node_trend4", ());
    scope.push_constant("chain_node_trend5", ());
    scope.push_constant("bottleneck_trend1", ());
    scope.push_constant("bottleneck_trend2", ());
    scope.push_constant("bottleneck_trend3", ());
    scope.push_constant("bottleneck_trend4", ());
    scope.push_constant("bottleneck_trend5", ());
    scope.push_constant("trend_names", ());
    match engine.compile_with_scope(&scope, code) {
        Ok(_) => eprintln!("=== CONSISTENCY-CHECK PARSE OK ==="),
        Err(e) => panic!("编译 consistency-check 失败: {e}"),
    }
}
