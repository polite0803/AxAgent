//! Vendor Schema 校验 —— 必填字段容错 + 字段质量评分

use crate::types::*;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldStatus {
    Ok,
    Missing,
    OptionalMissing,
    Invalid(String),
}

impl FieldStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, FieldStatus::Ok)
    }
    pub fn is_blocking(&self) -> bool {
        matches!(self, FieldStatus::Missing | FieldStatus::Invalid(_))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub ok: bool,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn passed() -> Self {
        Self {
            ok: true,
            ..Default::default()
        }
    }

    pub fn block(missing: Vec<String>) -> Self {
        Self {
            ok: false,
            missing,
            ..Default::default()
        }
    }

    pub fn add_warning(&mut self, w: impl Into<String>) {
        self.warnings.push(w.into());
    }

    pub fn quality_score(&self) -> f64 {
        let miss_penalty = self.missing.len() as f64 * 0.3;
        let warn_penalty = self.warnings.len() as f64 * 0.05;
        (1.0 - miss_penalty - warn_penalty).max(0.0)
    }
}

pub fn validate_quote(q: &StockQuote) -> ValidationReport {
    let mut r = ValidationReport::default();

    if q.code.is_empty() {
        r.missing.push("code".into());
    }
    if q.price <= 0.0 {
        r.missing.push(format!("price<=0 ({})", q.price));
    }
    if q.name.is_empty() {
        r.missing.push("name empty".into());
    }
    if q.pre_close <= 0.0 {
        r.add_warning(format!("pre_close={} (可能首日上市)", q.pre_close));
    }
    if q.pre_close > 0.0 {
        let change = (q.price - q.pre_close) / q.pre_close;
        if change.abs() > 0.30 {
            r.add_warning(format!("change_pct 异常: {:.1}%", change * 100.0));
        }
    }
    if q.turnover_rate < 0.0 || q.turnover_rate > 100.0 {
        r.add_warning(format!("turnover_rate 越界: {}", q.turnover_rate));
    }

    r.ok = r.missing.is_empty();
    r
}

pub fn validate_kline(k: &KLine) -> ValidationReport {
    let mut r = ValidationReport::default();
    if k.date.is_empty() {
        r.missing.push("date empty".into());
    } else if k.date.len() != 10 {
        r.add_warning(format!("date 格式非标准: {}", k.date));
    }
    if k.open < 0.0 || k.high < 0.0 || k.low < 0.0 || k.close < 0.0 {
        r.missing.push("OHLC 含负值".into());
    }
    if k.high < k.low {
        r.missing.push(format!("high<low ({},{})", k.high, k.low));
    }
    if k.close > 0.0 {
        if k.close > k.high {
            r.add_warning(format!("close>high ({},{})", k.close, k.high));
        }
        if k.close < k.low {
            r.add_warning(format!("close<low ({},{})", k.close, k.low));
        }
    }
    r.ok = r.missing.is_empty();
    r
}

pub fn validate_klines(ks: &[KLine]) -> ValidationReport {
    if ks.is_empty() {
        return ValidationReport::block(vec!["klines empty".into()]);
    }
    let mut combined = ValidationReport::passed();
    for (i, k) in ks.iter().enumerate() {
        let r = validate_kline(k);
        if !r.missing.is_empty() {
            for m in r.missing {
                combined.missing.push(format!("[{}] {}", i, m));
            }
        }
        for w in r.warnings {
            combined.add_warning(format!("[{}] {}", i, w));
        }
    }
    combined.ok = combined.missing.is_empty();
    combined
}

pub fn validate_financial(f: &FinancialReport) -> ValidationReport {
    let mut r = ValidationReport::default();
    if f.stock_code.is_empty() {
        r.missing.push("stock_code empty".into());
    }
    if f.report_date.is_empty() {
        r.missing.push("report_date empty".into());
    }
    if !f.has_valid_data() {
        r.missing.push("全部核心财务字段为空".into());
    }
    if let Some(pe) = f.eps {
        if !(-100.0..=1000.0).contains(&pe) {
            r.add_warning(format!("eps 异常: {}", pe));
        }
    }
    if let Some(roe) = f.roe {
        if !(-100.0..=200.0).contains(&roe) {
            r.add_warning(format!("roe 异常: {}", roe));
        }
    }
    r.ok = r.missing.is_empty();
    r
}

pub fn validate_financials(fs: &[FinancialReport]) -> ValidationReport {
    if fs.is_empty() {
        return ValidationReport::block(vec!["financials empty".into()]);
    }
    let mut combined = ValidationReport::passed();
    let mut has_valid = false;
    for (i, f) in fs.iter().enumerate() {
        let r = validate_financial(f);
        if r.ok {
            has_valid = true;
        }
        if !r.missing.is_empty() {
            for m in r.missing {
                combined.missing.push(format!("[{}] {}", i, m));
            }
        }
        for w in r.warnings {
            combined.add_warning(format!("[{}] {}", i, w));
        }
    }
    if !has_valid {
        combined.missing.push("无任何有效记录".into());
    }
    combined.ok = combined.missing.is_empty();
    combined
}

pub fn validate_news(n: &NewsItem) -> ValidationReport {
    let mut r = ValidationReport::default();
    if n.title.is_empty() {
        r.missing.push("title empty".into());
    }
    if n.url.is_empty() {
        r.add_warning("url empty");
    }
    if n.publish_time.is_empty() {
        r.add_warning("publish_time empty");
    }
    r.ok = r.missing.is_empty();
    r
}

pub fn validate_news_batch(items: &[NewsItem]) -> ValidationReport {
    if items.is_empty() {
        return ValidationReport::passed();
    }
    let mut combined = ValidationReport::passed();
    for (i, n) in items.iter().enumerate() {
        let r = validate_news(n);
        for m in r.missing {
            combined.missing.push(format!("[{}] {}", i, m));
        }
        for w in r.warnings {
            combined.add_warning(format!("[{}] {}", i, w));
        }
    }
    combined.ok = combined.missing.is_empty();
    combined
}

pub fn pick_better<T, F>(a: &Option<T>, b: &Option<T>, quality: F) -> Option<T>
where
    F: Fn(&T) -> f64,
    T: Clone,
{
    match (a, b) {
        (Some(va), Some(vb)) => {
            let qa = quality(va);
            let qb = quality(vb);
            if qa >= qb {
                Some(va.clone())
            } else {
                Some(vb.clone())
            }
        },
        (Some(va), None) => Some(va.clone()),
        (None, Some(vb)) => Some(vb.clone()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote(price: f64, name: &str) -> StockQuote {
        StockQuote {
            code: "600519".into(),
            name: name.into(),
            price,
            pre_close: 100.0,
            open: 100.0,
            high: 102.0,
            low: 99.0,
            volume: 1000.0,
            amount: 100000.0,
            change_pct: 0.5,
            turnover_rate: 0.3,
            pe: Some(35.0),
            pb: Some(12.0),
            total_mv: Some(2.0e12),
            circulating_mv: None,
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: "2026-01-15 14:00:00".into(),
        }
    }

    #[test]
    fn quote_ok() {
        let r = validate_quote(&sample_quote(101.0, "茅台"));
        assert!(r.ok);
        assert!(r.missing.is_empty());
    }

    #[test]
    fn quote_block_on_zero_price() {
        let r = validate_quote(&sample_quote(0.0, "茅台"));
        assert!(!r.ok);
        assert!(r.missing.iter().any(|m| m.contains("price<=0")));
    }

    #[test]
    fn quote_block_on_empty_name() {
        let r = validate_quote(&sample_quote(101.0, ""));
        assert!(!r.ok);
        assert!(r.missing.iter().any(|m| m.contains("name")));
    }

    #[test]
    fn quote_warn_on_abnormal_change() {
        let mut q = sample_quote(200.0, "茅台");
        q.pre_close = 100.0;
        let r = validate_quote(&q);
        assert!(r.ok);
        assert!(r.warnings.iter().any(|w| w.contains("change_pct")));
    }

    #[test]
    fn kline_block_on_high_lt_low() {
        let k = KLine {
            date: "2026-01-15".into(),
            open: 10.0,
            high: 9.0,
            low: 11.0,
            close: 10.0,
            volume: 100.0,
            amount: 1000.0,
            turnover_rate: None,
            adj_factor: None,
        };
        let r = validate_kline(&k);
        assert!(!r.ok);
        assert!(r.missing.iter().any(|m| m.contains("high<low")));
    }

    #[test]
    fn klines_empty_is_blocked() {
        let r = validate_klines(&[]);
        assert!(!r.ok);
    }

    #[test]
    fn financials_block_when_all_empty() {
        let f = FinancialReport {
            stock_code: "600519".into(),
            report_date: "2025-09-30".into(),
            revenue: None,
            net_profit: None,
            eps: None,
            bps: None,
            roe: None,
            debt_ratio: None,
            gross_margin: None,
            net_margin: None,
            revenue_yoy: None,
            profit_yoy: None,
            total_assets: None,
            operating_cash_flow: None,
            capital_expenditure: None,
            free_cash_flow: None,
            current_ratio: None,
            quick_ratio: None,
        };
        let r = validate_financial(&f);
        assert!(!r.ok);
    }

    #[test]
    fn financials_pass_with_one_field() {
        let f = FinancialReport {
            stock_code: "600519".into(),
            report_date: "2025-09-30".into(),
            revenue: Some(1e10),
            net_profit: None,
            eps: None,
            bps: None,
            roe: None,
            debt_ratio: None,
            gross_margin: None,
            net_margin: None,
            revenue_yoy: None,
            profit_yoy: None,
            total_assets: None,
            operating_cash_flow: None,
            capital_expenditure: None,
            free_cash_flow: None,
            current_ratio: None,
            quick_ratio: None,
        };
        let r = validate_financial(&f);
        assert!(r.ok);
    }

    #[test]
    fn pick_better_chooses_higher_quality() {
        let qa = sample_quote(100.0, "A");
        let qb = sample_quote(200.0, "B");
        let pick = pick_better(&Some(qa.clone()), &Some(qb.clone()), |q| q.price);
        assert_eq!(pick.unwrap().name, "B");
        let pick = pick_better(&Some(qa.clone()), &Some(qb.clone()), |q| -q.price);
        assert_eq!(pick.unwrap().name, "A");
        let pick = pick_better(&None, &Some(qb.clone()), |_q| 0.0);
        assert_eq!(pick.unwrap().name, "B");
        let pick: Option<StockQuote> = pick_better(&None, &None, |_q| 0.0);
        assert!(pick.is_none());
    }

    #[test]
    fn quality_score_decreases_with_issues() {
        let mut r = ValidationReport::passed();
        let s0 = r.quality_score();
        r.missing.push("x".into());
        r.missing.push("y".into());
        r.add_warning("w");
        let s1 = r.quality_score();
        assert!(s1 < s0);
        assert!(s0 > 0.99);
    }
}
