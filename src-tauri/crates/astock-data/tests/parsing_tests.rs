use axagent_astock_data::*;

#[test]
fn test_stock_quote_serialization() {
    let quote = StockQuote {
        code: "600519".into(),
        name: "贵州茅台".into(),
        price: 1680.0,
        open: 1650.0,
        high: 1695.0,
        low: 1642.0,
        volume: 3820000.0,
        amount: 6400000000.0,
        change_pct: 2.35,
        turnover_rate: 0.52,
        pe: Some(35.5),
        pb: Some(12.3),
        total_mv: Some(2100000000000.0),
        circulating_mv: None,
        limit_up: Some(1848.0),
        limit_down: Some(1512.0),
        is_st: false,
        pre_close: 1648.0,
        timestamp: "2026-05-14T12:00:00Z".into(),
    };
    let json = serde_json::to_string(&quote).unwrap();
    let parsed: StockQuote = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.code, "600519");
    assert_eq!(parsed.price, 1680.0);
    // camelCase after serde rename
    assert_eq!(parsed.change_pct, 2.35);
    assert!(json.contains("changePct"));
    assert!(json.contains("600519"));
}

#[test]
fn test_kline_serialization() {
    let kline = KLine {
        date: "2026-05-14".into(),
        open: 1650.0,
        high: 1695.0,
        low: 1642.0,
        close: 1680.0,
        volume: 3820000.0,
        amount: 6400000000.0,
        turnover_rate: Some(0.52),
        adj_factor: None,
    };
    let json = serde_json::to_string(&kline).unwrap();
    let parsed: KLine = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.date, "2026-05-14");
    assert_eq!(parsed.close, 1680.0);
    assert!(json.contains("turnoverRate"));
}

#[test]
fn test_kline_period_em_codes() {
    assert_eq!(KLinePeriod::Min5.to_em_code(), "5");
    assert_eq!(KLinePeriod::Min15.to_em_code(), "15");
    assert_eq!(KLinePeriod::Min30.to_em_code(), "30");
    assert_eq!(KLinePeriod::Min60.to_em_code(), "60");
    assert_eq!(KLinePeriod::Daily.to_em_code(), "101");
    assert_eq!(KLinePeriod::Weekly.to_em_code(), "102");
    assert_eq!(KLinePeriod::Monthly.to_em_code(), "103");
}

#[test]
fn test_stock_quote_default_values() {
    // 验证可选字段为 None 时的序列化行为
    let quote = StockQuote {
        code: "000001".into(),
        name: "平安银行".into(),
        price: 10.5,
        open: 10.3,
        high: 10.6,
        low: 10.2,
        volume: 1000000.0,
        amount: 10500000.0,
        change_pct: -0.5,
        turnover_rate: 0.1,
        pe: None,
        pb: None,
        total_mv: None,
        circulating_mv: None,
        limit_up: None,
        limit_down: None,
        is_st: false,
        pre_close: 1648.0,
        timestamp: "2026-05-14T12:00:00Z".into(),
    };
    let json = serde_json::to_string(&quote).unwrap();
    let parsed: StockQuote = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.pe, None);
    assert_eq!(parsed.pb, None);
    assert_eq!(parsed.total_mv, None);
}

#[test]
fn test_stock_search_result_serialization() {
    let result = StockSearchResult {
        code: "600519".into(),
        name: "贵州茅台".into(),
        market: "SH".into(),
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: StockSearchResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.code, "600519");
    assert_eq!(parsed.market, "SH");
}
