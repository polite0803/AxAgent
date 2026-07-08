use chrono::NaiveDate;

use axagent_astock_data::calendar::*;
use axagent_astock_data::indicators::*;
use axagent_astock_data::*;

#[test]
fn test_detect_market_type() {
    assert_eq!(detect_market_type("600519"), "main_sh");
    assert_eq!(detect_market_type("000001"), "main_sz");
    assert_eq!(detect_market_type("300750"), "chinext");
    assert_eq!(detect_market_type("688981"), "star");
    assert_eq!(detect_market_type("8xxxxx"), "bj");
    assert_eq!(detect_market_type("5xxxxx"), "unknown");
}

#[test]
fn test_price_limit_pct() {
    assert_eq!(get_price_limit_pct("main_sh"), 10.0);
    assert_eq!(get_price_limit_pct("main_sz"), 10.0);
    assert_eq!(get_price_limit_pct("chinext"), 20.0);
    assert_eq!(get_price_limit_pct("star"), 20.0);
    assert_eq!(get_price_limit_pct("bj"), 30.0);
}

#[test]
fn test_st_price_limit_pct() {
    assert_eq!(get_st_price_limit_pct(true, "main_sh"), 5.0);
    assert_eq!(get_st_price_limit_pct(false, "chinext"), 20.0);
}

#[test]
fn test_compute_indicators_basic() {
    let klines: Vec<KLine> = (0..30)
        .map(|i| {
            let price = 10.0 + i as f64 * 0.2;
            KLine {
                date: NaiveDate::from_ymd_opt(2026, 1, (i + 1) as u32)
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string(),
                open: price - 0.1,
                high: price + 0.3,
                low: price - 0.3,
                close: price,
                volume: 1000.0 + i as f64 * 10.0,
                amount: price * 1100.0,
                turnover_rate: None,
                adj_factor: None,
            }
        })
        .collect();

    let indicators = compute_indicators("TEST", &klines);
    assert!(indicators.ma5 > 0.0);
    assert!(indicators.ma20 > 0.0);
    assert!(!indicators.ma_alignment.is_empty());
    assert!(indicators.rsi6 >= 0.0 && indicators.rsi6 <= 100.0);
    assert!(!indicators.macd_signal.is_empty());
    assert!(!indicators.volume_signal.is_empty());
}

#[test]
fn test_is_trading_day() {
    // 周一（非节假日）
    assert!(is_trading_day(&NaiveDate::from_ymd_opt(2026, 5, 11).unwrap()));
    // 周六
    assert!(!is_trading_day(&NaiveDate::from_ymd_opt(2026, 5, 16).unwrap()));
    // 国庆
    assert!(!is_trading_day(&NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()));
    // 中秋
    assert!(!is_trading_day(&NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()));
}

#[test]
fn test_data_error_display() {
    let err = DataError::NotFound("600000".into());
    assert!(err.to_string().contains("600000"));
    let err = DataError::VendorError {
        vendor: "test".into(),
        message: "fail".into(),
    };
    assert!(err.to_string().contains("test"));
}

#[test]
fn test_stock_raw_data_serialization() {
    let raw = StockRawData {
        quote: StockQuote {
            code: "600519".into(),
            name: "茅台".into(),
            price: 1680.0,
            open: 1650.0,
            high: 1695.0,
            low: 1642.0,
            volume: 100.0,
            amount: 1000.0,
            change_pct: 2.35,
            turnover_rate: 0.5,
            pe: Some(35.0),
            pb: Some(12.0),
            total_mv: None,
            circulating_mv: None,
            limit_up: Some(1850.0),
            limit_down: Some(1500.0),
            is_st: false,
            pre_close: 1648.0,
            timestamp: "now".into(),
        },
        klines: vec![],
        financials: vec![],
        news: vec![],
        money_flow: None,
        dragon_tiger: vec![],
        lockup: vec![],
        margin_data: None,
        north_bound: None,
        sector_info: None,
        shareholder_trades: vec![],
        dividend_records: vec![],
        research_reports: vec![],
        consensus_eps: None,
        concept_blocks: None,
        announcements: vec![],
        block_trades: vec![],
        institutional_visits: vec![],
        peers: vec![],
        option_pcr: None,
    };
    let json = serde_json::to_string(&raw).unwrap();
    assert!(json.contains("600519"));
    assert!(json.contains("1680.0"));
    let back: StockRawData = serde_json::from_str(&json).unwrap();
    assert_eq!(back.quote.code, "600519");
}
