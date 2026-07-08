use rhai::{Dynamic, Engine, Scope};

#[test]
fn test_push_complex_map() {
    // Reproduce what code_executor does: push a complex map with nested data
    let engine = Engine::new();
    let mut scope = Scope::new();

    // Create a complex map like the baseline data
    let mut map = rhai::Map::new();

    let mut cp = rhai::Map::new();
    cp.insert("debt_ratio_pct".into(), Dynamic::from(0.0_f64));
    cp.insert("gm_rank_in_peers".into(), Dynamic::from(0_i64));
    cp.insert("gross_margin_pct".into(), Dynamic::from(0.0_f64));
    cp.insert("rnd_intensity".into(), Dynamic::from(0.0_f64));
    cp.insert("roe_pct".into(), Dynamic::from(0.0_f64));
    cp.insert("total_peer_count".into(), Dynamic::from(0_i64));

    map.insert("competitive_position".into(), Dynamic::from(cp));
    map.insert("sector".into(), Dynamic::from(""));
    map.insert("stock_code".into(), Dynamic::from("002371"));
    map.insert("peer_summary".into(), Dynamic::from(rhai::Array::new()));

    scope.push_dynamic("baseline_semi", Dynamic::from(map));

    // Verify the variable exists
    let result: Result<bool, _> = engine.eval_with_scope(&mut scope, "baseline_semi != ()");
    eprintln!("push_dynamic complex map check: {:?}", result);
    match result {
        Ok(v) => assert!(v, "baseline_semi should not be ()"),
        Err(e) => panic!("FAIL: {e}"),
    }
}

#[test]
fn test_push_complex_via_push_constant() {
    let engine = Engine::new();
    let mut scope = Scope::new();

    // Same complex map, but push via push_constant
    let mut map = rhai::Map::new();
    let mut cp = rhai::Map::new();
    cp.insert("gross_margin_pct".into(), Dynamic::from(0.0_f64));
    cp.insert("roe_pct".into(), Dynamic::from(0.0_f64));
    map.insert("competitive_position".into(), Dynamic::from(cp));
    map.insert("sector".into(), Dynamic::from(""));
    map.insert("stock_code".into(), Dynamic::from("002371"));

    let dyn_val: Dynamic = map.into();
    scope.push_constant("baseline_semi", dyn_val);

    // Verify
    let result: Result<bool, _> = engine.eval_with_scope(&mut scope, "baseline_semi != ()");
    eprintln!("push_constant complex map check: {:?}", result);
    match result {
        Ok(v) => assert!(v, "baseline_semi should not be ()"),
        Err(e) => panic!("FAIL: {e}"),
    }
}

#[test]
fn test_multiple_complex_pushes() {
    let engine = Engine::new();
    let mut scope = Scope::new();

    // Push 9 variables (simulating the actual code_executor flow)
    let vars = [
        "baseline_semi",
        "baseline_battery",
        "baseline_chem",
        "baseline_med",
        "baseline_aero",
        "baseline_consumer_elec",
        "baseline_auto",
        "chain_analysis",
        "industry_ranking",
    ];

    for v in &vars {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(*v));
        map.insert("value".into(), Dynamic::from(0.0_f64));
        scope.push_dynamic(*v, Dynamic::from(map));
    }

    // Verify ALL exist
    for v in &vars {
        let code = format!("{v} != ()");
        let result: Result<bool, _> = engine.eval_with_scope(&mut scope, &code);
        eprintln!("{v}: {:?}", result);
        match result {
            Ok(is_true) => assert!(is_true, "{v} should not be ()"),
            Err(e) => panic!("{v} NOT FOUND: {e}"),
        }
    }
}
