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
