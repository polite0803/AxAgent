use chrono::{Datelike, Duration, NaiveDate, Timelike, Weekday};
use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

/// 远程拉取的 A 股节假日缓存(YYYY-MM-DD 格式)。
/// 启动时由 `init_holiday_calendar` 异步填充,is_trading_day 优先查这里。
/// 用 RwLock 允许运行时刷新(每年初拉一次新年日历)。
static REMOTE_HOLIDAYS: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 把东方财富返回的 "20250101" 格式转换为 "2025-01-01"
fn em_date_to_iso(s: &str) -> String {
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

/// 判断是否为A股交易日。
///
/// 优先级:
/// 1. 远程节假日缓存(启动时由 init_holiday_calendar 异步填充,优先于硬编码)
/// 2. 周末判断(Sat/Sun)
/// 3. 调休工作日(周末但上班)
/// 4. 2025-2026 年硬编码节假日
///
/// 缺陷 A 修复:2027 年及以后不再依赖硬编码,而是依赖远程缓存(由东方财富 API 拉取 365 天滚动)。
pub fn is_trading_day(date: &NaiveDate) -> bool {
    let date_str = date.format("%Y-%m-%d").to_string();

    // 1) 远程节假日缓存命中 → 非交易日
    if let Ok(cache) = REMOTE_HOLIDAYS.read() {
        if cache.contains(&date_str) {
            return false;
        }
    }

    // 调休工作日（周末但上班）——优先于周末判断
    let workdays = [
        "2025-02-08", // 春节调休
        "2025-04-27", // 清明调休
        "2025-09-28", // 中秋调休
        "2025-10-11", // 国庆调休
    ];
    if workdays.contains(&date_str.as_str()) {
        return true;
    }

    let w = date.weekday();
    // 周末不交易
    if w == Weekday::Sat || w == Weekday::Sun {
        return false;
    }

    // 硬编码2025-2026年A股节假日（简化版，不含临时休市）
    let holidays = [
        // 2025年
        "2025-01-01", // 元旦
        "2025-01-28",
        "2025-01-29",
        "2025-01-30",
        "2025-01-31",
        "2025-02-03",
        "2025-02-04", // 春节
        "2025-04-04",
        "2025-04-07", // 清明
        "2025-05-01",
        "2025-05-02",
        "2025-05-05", // 劳动节
        "2025-06-02", // 端午
        "2025-09-15",
        "2025-09-16", // 中秋
        "2025-10-01",
        "2025-10-02",
        "2025-10-03",
        "2025-10-06",
        "2025-10-07",
        "2025-10-08", // 国庆
        // 2026年
        "2026-01-01",
        "2026-01-02",
        "2026-02-16",
        "2026-02-17",
        "2026-02-18",
        "2026-02-19",
        "2026-02-20",
        "2026-02-23",
        "2026-02-24",
        "2026-03-23",
        "2026-03-24",
        "2026-04-20",
        "2026-04-21",
        "2026-05-18",
        "2026-05-19",
        "2026-06-08",
        "2026-06-09",
        "2026-09-21",
        "2026-09-22",
        "2026-09-28",
        "2026-09-29",
        "2026-09-30",
        "2026-10-01",
        "2026-10-02",
    ];

    let date_str2 = date.format("%Y-%m-%d").to_string();
    if holidays.contains(&date_str2.as_str()) {
        return false;
    }

    true
}

/// 查找指定日期之前最近的交易日(若 date 本身是交易日,返回 date)
/// 用于 as-of 模式下周末/节假日的 fallback: 用户选择回看周六,
/// 真实数据应该取周五收盘。
pub fn previous_trading_day(date: NaiveDate) -> NaiveDate {
    let mut candidate = date;
    for _ in 0..14 {
        // 14 天上限(春节+调休极端情况)
        if is_trading_day(&candidate) {
            return candidate;
        }
        candidate -= Duration::days(1);
    }
    // 兜底:连续14天都不是交易日(极端硬编码漏洞),返回原 date
    date
}

/// 查找指定日期之后最近的交易日(若 date 本身是交易日,返回 date)
pub fn next_trading_day(date: NaiveDate) -> NaiveDate {
    let mut candidate = date;
    for _ in 0..14 {
        if is_trading_day(&candidate) {
            return candidate;
        }
        candidate += Duration::days(1);
    }
    date
}

/// 判断当前是否为交易时间
pub fn is_trading_time() -> bool {
    let now = chrono::Utc::now();
    let today = now.date_naive();

    if !is_trading_day(&today) {
        return false;
    }

    // 获取北京时间（UTC+8）
    let hour = (now.hour() + 8) % 24;
    let minute = now.minute();

    // 上午 9:30-11:30
    if hour == 9 && minute >= 30 {
        return true;
    }
    if hour == 10 {
        return true;
    }
    if hour == 11 && minute <= 30 {
        return true;
    }

    // 下午 13:00-15:00
    if hour == 13 {
        return true;
    }
    if hour == 14 {
        return true;
    }
    if hour == 15 && minute == 0 {
        return true;
    }

    false
}

/// 判断当前是否为午休时间
pub fn is_lunch_break() -> bool {
    let now = chrono::Utc::now();
    let hour = (now.hour() + 8) % 24;
    hour == 11 || hour == 12
}

/// 从东方财富 API 获取最新交易日历
pub async fn fetch_holiday_calendar() -> Result<Vec<String>, String> {
    let url = "https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=RPTA_WEB_TRADE_CALENDAR&columns=TRADE_DATE,IS_TRADING_DAY&pageSize=365&pageNumber=1";
    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("获取交易日历失败: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析失败: {}", e))?;

    let holidays: Vec<String> = json["result"]["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|d| {
            let is_trading = d["IS_TRADING_DAY"].as_str().unwrap_or("1") == "0";
            let date_raw = d["TRADE_DATE"].as_str().unwrap_or("");
            let date = em_date_to_iso(date_raw);
            if !date.is_empty() && is_trading {
                Some(date)
            } else {
                None
            }
        })
        .collect();

    Ok(holidays)
}

/// 把远程拉取的节假日写入全局缓存。
/// 启动时调用一次,失败也不影响 is_trading_day(会 fallback 到硬编码 2025-2026)。
/// 返回写入的节假日条数。
pub fn populate_remote_holidays(holidays: Vec<String>) -> usize {
    let count = holidays.len();
    if let Ok(mut cache) = REMOTE_HOLIDAYS.write() {
        cache.clear();
        cache.extend(holidays);
    }
    count
}

/// 启动时初始化节假日缓存(异步,fire-and-forget)。
/// 不阻塞主流程,失败时静默退化(走硬编码)。
pub async fn init_holiday_calendar() -> Result<usize, String> {
    let holidays = fetch_holiday_calendar().await?;
    Ok(populate_remote_holidays(holidays))
}

/// 仅供测试:清空远程节假日缓存,恢复纯硬编码模式
#[cfg(test)]
pub fn clear_remote_holidays_for_test() {
    if let Ok(mut cache) = REMOTE_HOLIDAYS.write() {
        cache.clear();
    }
}

/// 获取距离下一个交易时间的描述
pub fn next_trading_time_desc() -> String {
    if is_trading_time() {
        return "交易中".to_string();
    }
    if is_lunch_break() {
        return "午休中，13:00恢复".to_string();
    }

    let now = chrono::Utc::now();
    let hour = (now.hour() + 8) % 24;
    if hour < 9 || (hour == 9 && now.minute() < 30) {
        return "距开盘".to_string();
    }
    if hour >= 15 {
        return "已收盘".to_string();
    }
    "非交易时段".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekend_not_trading() {
        let sat = NaiveDate::from_ymd_opt(2025, 6, 7).unwrap(); // 周六
        let sun = NaiveDate::from_ymd_opt(2025, 6, 8).unwrap(); // 周日
        assert!(!is_trading_day(&sat));
        assert!(!is_trading_day(&sun));
    }

    #[test]
    fn test_weekday_is_trading() {
        let mon = NaiveDate::from_ymd_opt(2025, 6, 9).unwrap(); // 周一，非节假日
        assert!(is_trading_day(&mon));
    }

    #[test]
    fn test_holiday_not_trading() {
        let new_year = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(); // 元旦
        assert!(!is_trading_day(&new_year));
    }

    #[test]
    fn test_national_day_holiday() {
        let nd = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(); // 国庆
        assert!(!is_trading_day(&nd));
    }

    #[test]
    fn test_workday_override() {
        let workday = NaiveDate::from_ymd_opt(2025, 2, 8).unwrap(); // 春节调休上班
        assert!(is_trading_day(&workday));
    }

    /// 缺陷 A 修复:远程节假日缓存优先于硬编码
    #[test]
    fn test_remote_holiday_overrides_default() {
        clear_remote_holidays_for_test();
        // 2027-01-01 元旦 — 硬编码 2025-2026 没覆盖,默认会被认为"是交易日"
        let future = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert!(is_trading_day(&future), "默认(无远程缓存)2027 元旦应被当作交易日");
        // 注入远程缓存后,应被识别为节假日
        let n = populate_remote_holidays(vec!["2027-01-01".to_string()]);
        assert_eq!(n, 1);
        assert!(!is_trading_day(&future), "远程缓存应优先于硬编码");
        // 清理,不影响其他测试
        clear_remote_holidays_for_test();
    }

    #[test]
    fn test_em_date_to_iso() {
        assert_eq!(em_date_to_iso("20250101"), "2025-01-01");
        assert_eq!(em_date_to_iso("20261231"), "2026-12-31");
        // 非数字原样返回
        assert_eq!(em_date_to_iso("2025-01-01"), "2025-01-01");
        assert_eq!(em_date_to_iso(""), "");
    }
}
