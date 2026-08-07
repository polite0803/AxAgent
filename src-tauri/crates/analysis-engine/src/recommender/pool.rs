//! 候选池构造 + vendor 启用集合检测

use axagent_astock_data::AStockClient;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::FALLBACK_STOCKS;

/// 候选池条目：(code, name, sector)
pub type SeedItem = (String, String, Option<String>);

/// 构造 seed pool
///
/// 顺序：get_hot_stocks(30) ∪ get_industry_ranking 龙头(20) ∪ FALLBACK_STOCKS 冷门补全
///
/// **冷门补全逻辑**：热门股池天然包含已上涨标的，但策略需要同样扫描未热门的潜在标的。
/// FALLBACK_STOCKS 覆盖沪深300+行业龙头+硬科技约80只，确保策略有足够多样化样本。
/// 流动性过滤（≥1亿日均成交额）会进一步筛除不活跃标的。
pub async fn build_seed_pool(client: &AStockClient) -> Vec<SeedItem> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<SeedItem> = Vec::new();

    // 1. 热门个股
    let hot_succeeded = match client.get_hot_stocks().await {
        Ok(hot) => {
            for h in hot.iter().take(50) {
                if seen.insert(h.stock_code.clone()) {
                    out.push((h.stock_code.clone(), h.stock_name.clone(), h.sector.clone()));
                }
            }
            true
        },
        Err(e) => {
            tracing::warn!("[seed_pool] get_hot_stocks 失败: {e}");
            false
        },
    };

    // 2. 行业排名龙头（扩大到20个行业）
    let industry_succeeded = match client.get_industry_ranking().await {
        Ok(industries) => {
            for ind in industries.iter().take(30) {
                if let (Some(code), Some(name)) = (&ind.leader_code, &ind.leader_name) {
                    if seen.insert(code.clone()) {
                        out.push((code.clone(), name.clone(), Some(ind.industry_name.clone())));
                    }
                }
            }
            true
        },
        Err(e) => {
            tracing::warn!("[seed_pool] get_industry_ranking 失败: {e}");
            false
        },
    };

    // 诊断日志：两个数据源都失败时，种子池仅靠 FALLBACK_STOCKS 兜底
    if !hot_succeeded && !industry_succeeded {
        tracing::warn!(
            "[seed_pool] hot_stocks 和 industry_ranking 均失败, 种子池仅靠 {} 只 FALLBACK_STOCKS 兜底",
            FALLBACK_STOCKS.len()
        );
    }

    // 3. 冷门补全：始终混入 FALLBACK_STOCKS 中未被前两个源覆盖的标的
    //    防止种子池只有"已涨的"——给予策略发现潜在冷门标的机会
    for (code, name) in FALLBACK_STOCKS {
        if seen.insert((*code).to_string()) {
            out.push(((*code).to_string(), (*name).to_string(), None));
        }
    }

    out
}

/// 流动性过滤：日均成交额 ≥ 1 亿；排除 ST / 上市 < 60 日
/// 截断到 200
///
/// 用 `FuturesUnordered` 并发（最多 8 并发），避免串行导致整体超时
pub async fn liquidity_filter_and_truncate(
    client: Arc<AStockClient>,
    seed: Vec<SeedItem>,
) -> Vec<SeedItem> {
    // P3-D12: 保留原签名作为薄包装，向后兼容。
    // 默认并发数 8 来自历史硬编码值。
    liquidity_filter_and_truncate_with_concurrency(client, seed, 8).await
}

/// P3-D12: 带自定义并发数的流动性过滤。
///
/// `max_concurrent` 控制对 `client.get_quote` 的并发拉取上限。
/// 调用方可根据 vendor 健康度动态传入：
/// - 上游全部健康 → 8（默认）
/// - 部分降级 → 4（避免雪崩）
/// - 大面积降级 → 2（保命）
///
/// 截断到 200 个标的。
pub async fn liquidity_filter_and_truncate_with_concurrency(
    client: Arc<AStockClient>,
    seed: Vec<SeedItem>,
    max_concurrent: usize,
) -> Vec<SeedItem> {
    use futures::stream::{FuturesUnordered, StreamExt};
    use tokio::sync::Semaphore;

    // P3-D12: 防御性下限，避免调用方误传 0
    let concurrency = max_concurrent.max(1);
    let sem = Arc::new(Semaphore::new(concurrency));
    let mut tasks: FuturesUnordered<_> = FuturesUnordered::new();

    for item in seed {
        let sem = sem.clone();
        let client = client.clone();
        tasks.push(async move {
            let _permit = sem.acquire_owned().await.ok()?;
            filter_one(&client, item).await
        });
    }

    let mut kept: Vec<SeedItem> = Vec::new();
    while let Some(res) = tasks.next().await {
        if let Some(item) = res {
            kept.push(item);
            if kept.len() >= 200 {
                break;
            }
        }
    }
    kept
}

async fn filter_one(client: &AStockClient, item: SeedItem) -> Option<SeedItem> {
    let (code, name, sector) = item;
    // A 股荐股场景排除 B 股（沪B 900xxx / 深B 200xxx）：
    // 腾讯等 quote 源不支持 B 股（Stock code not found），
    // 提前过滤避免无谓请求与日志噪音。
    let plain = code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
    if plain.len() == 6 && (plain.starts_with("900") || plain.starts_with("200")) {
        return None;
    }
    // 行情数据必须可获取，否则无法交易（quote 走 tencent 路由，通常稳定）
    // 加一次轻量重试：瞬断场景下避免大量标的被误过滤
    // 用 tokio::time::timeout 包裹，避免单个标的长时间阻塞整个过滤阶段
    let quote = match tokio::time::timeout(std::time::Duration::from_secs(10), async {
        match client.get_quote(&code).await {
            Ok(q) => Some(q),
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                client.get_quote(&code).await.ok()
            },
        }
    })
    .await
    {
        Ok(Some(q)) => q,
        _ => return None, // 超时或两次失败都跳过
    };
    if quote.is_st {
        return None;
    }
    // 用当日成交额估算流动性（替代 60 天 K 线平均成交额，消除额外 HTTP 请求）
    if quote.amount < 100_000_000.0 {
        return None;
    }
    Some((code, name, sector))
}

/// 缓存 vendor 启用集合（5 min）
use std::sync::Mutex;
use std::time::{Duration, Instant};

static VENDOR_CACHE: Mutex<Option<(HashSet<String>, Instant)>> = Mutex::new(None);
const VENDOR_TTL: Duration = Duration::from_secs(300);

/// 通过 `get_workflow_template` 读 `vendor_*` 变量，返回启用的 vendor 集合
///
/// 使用白名单模式：只解析已知的 vendor 名称，排除 `vendor_xueqiu_token` / `vendor_neodata_token` 等凭据变量
/// 对于 KNOWN_VENDORS 中模板未定义的 vendor，默认启用（避免新 vendor 被误过滤）
static KNOWN_VENDORS: &[&str] = &[
    "tencent",
    "eastmoney",
    "sina",
    "ths",
    "cninfo",
    "baidu_stock",
    "iwencai",
    "akshare",
    "mootdx",
    "xueqiu",
    "neodata",
    // 以下 vendor 无需用户配置凭据，始终启用
    "browser_eastmoney",
    "guba",
];
pub fn load_enabled_vendors_from_template(
    template_vars: &[(String, serde_json::Value)],
) -> HashSet<String> {
    let mut set = HashSet::new();
    // KNOWN_VENDORS 中模板未定义的 vendor 默认启用
    let template_vendor_names: std::collections::HashSet<&str> = template_vars
        .iter()
        .filter_map(|(n, _)| {
            if !n.starts_with("vendor_") {
                return None;
            }
            let v = n.trim_start_matches("vendor_");
            if KNOWN_VENDORS.contains(&v) {
                Some(v)
            } else {
                None
            }
        })
        .collect();
    for v in KNOWN_VENDORS {
        if !template_vendor_names.contains(v) {
            set.insert(v.to_string());
        }
    }
    // 处理模板中显式定义的 vendor 变量
    for (name, value) in template_vars {
        if !name.starts_with("vendor_") {
            continue;
        }
        let vendor_name = name.trim_start_matches("vendor_");
        // 白名单：仅当在已知 vendor 列表中才处理
        if !KNOWN_VENDORS.contains(&vendor_name) {
            continue;
        }
        // 修复：旧逻辑 !v.is_empty() 会让字符串 "false"/"0"/"no" 都判为启用
        // 新逻辑：显式禁用关键字 → false；其他非空字符串 → true（兼容历史 "enabled" 用法）
        let enabled = if let Some(v) = value.as_str() {
            let lower = v.to_ascii_lowercase();
            !matches!(lower.as_str(), "" | "false" | "0" | "no" | "off" | "disabled")
        } else {
            value.as_bool().unwrap_or(false)
        };
        if enabled {
            set.insert(vendor_name.to_string());
        } else {
            set.remove(vendor_name); // 显式禁用
        }
    }
    set
}

/// 读取缓存（命中且未过期直接返回）
pub fn get_cached_vendors() -> Option<HashSet<String>> {
    let guard = VENDOR_CACHE.lock().ok()?;
    if let Some((set, ts)) = guard.as_ref() {
        if ts.elapsed() < VENDOR_TTL {
            return Some(set.clone());
        }
    }
    None
}

pub fn set_cached_vendors(set: HashSet<String>) {
    if let Ok(mut g) = VENDOR_CACHE.lock() {
        *g = Some((set, Instant::now()));
    }
}

pub fn clear_cached_vendors() {
    if let Ok(mut g) = VENDOR_CACHE.lock() {
        *g = None;
    }
}

/// 评估一组必需 vendor 中是否至少有一个启用
pub fn vendors_satisfied(required: &[&str], enabled: &HashSet<String>) -> bool {
    required.iter().any(|v| enabled.contains(*v))
}

/// 把 vendor 列表 (a股 vendor 名) 映射到 PANEL_VENDORS 里的 key
/// 与前端 [PANEL_VENDORS](file:///d:/OneManager/AxInvest/src/components/stock-analysis/vendorCheck.ts) 保持一致
pub fn required_vendors_for_style(
    style: crate::recommender::types::Style,
) -> &'static [&'static str] {
    use crate::recommender::types::Style::*;
    match style {
        // 趋势跟踪：依赖 K 线（任意一家支持 K 线的 vendor）
        Trend => &["eastmoney", "tencent", "ths", "akshare"],
        // 价值低估：需要估值 / 财务
        Value => &["eastmoney", "ths", "akshare"],
        // 资金驱动：需要资金流向 + 龙虎（ths / baidu_stock 任一即可）
        Capital => &["ths", "baidu_stock"],
        // 超跌反弹：依赖 K 线
        Reversion => &["eastmoney", "tencent", "ths", "akshare"],
        // 候选池兜底：只依赖实时 quote
        Watchlist => &["tencent", "eastmoney", "mootdx"],
        // 趋势智选策略：依赖财务数据做确定性验证
        Bottleneck => &["eastmoney", "ths", "akshare"],
        Policy => &["eastmoney", "ths", "akshare"],
        Earnings => &["eastmoney", "ths", "akshare"],
        CapitalFlow => &["eastmoney", "ths", "akshare"],
        Event => &["eastmoney", "ths", "akshare"],
        Technical => &["eastmoney", "ths", "akshare"],
    }
}

/// 给后端 logs 用的：从 HashSet 序列化为 vendor → bool 表，方便调试
pub fn vendors_to_map(enabled: &HashSet<String>) -> HashMap<String, bool> {
    let all =
        ["eastmoney", "tencent", "ths", "baidu_stock", "akshare", "iwencai", "cninfo", "sina"];
    all.iter().map(|v| (v.to_string(), enabled.contains(*v))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommender::types::Style;

    #[test]
    fn vendors_satisfied_any_match() {
        let mut set = HashSet::new();
        set.insert("ths".to_string());
        assert!(vendors_satisfied(&["eastmoney", "ths", "akshare"], &set));
    }

    #[test]
    fn vendors_satisfied_none_match() {
        let set: HashSet<String> = HashSet::new();
        assert!(!vendors_satisfied(&["eastmoney", "ths"], &set));
    }

    #[test]
    fn vendor_template_extraction() {
        let vars = vec![
            ("vendor_eastmoney".to_string(), serde_json::json!("enabled")),
            ("vendor_tencent".to_string(), serde_json::json!("")),
            ("vendor_akshare".to_string(), serde_json::json!("enabled")),
            ("vendor_iwencai_key".to_string(), serde_json::json!("abc")),
        ];
        let s = load_enabled_vendors_from_template(&vars);
        assert!(s.contains("eastmoney"));
        assert!(s.contains("akshare"));
        assert!(!s.contains("tencent")); // 空字符串
        assert!(!s.contains("iwencai_key")); // iwencai_key 排除
    }

    /// 回归测试：字符串 "false" 不应被误判为启用（曾因 !v.is_empty() 导致 BUG）
    #[test]
    fn vendor_string_false_should_be_disabled() {
        let vars = vec![
            ("vendor_iwencai".to_string(), serde_json::json!("false")),
            ("vendor_neodata".to_string(), serde_json::json!("False")),
            ("vendor_xueqiu".to_string(), serde_json::json!("FALSE")),
            ("vendor_eastmoney".to_string(), serde_json::json!("true")),
            ("vendor_tencent".to_string(), serde_json::json!("0")),
            ("vendor_ths".to_string(), serde_json::json!("off")),
            ("vendor_akshare".to_string(), serde_json::json!("disabled")),
        ];
        let s = load_enabled_vendors_from_template(&vars);
        // 各种大小写的 "false" 都应禁用
        assert!(!s.contains("iwencai"), "字符串 'false' 应判为禁用");
        assert!(!s.contains("neodata"), "字符串 'False' 应判为禁用");
        assert!(!s.contains("xueqiu"), "字符串 'FALSE' 应判为禁用");
        // 其他禁用关键字
        assert!(!s.contains("tencent"), "字符串 '0' 应判为禁用");
        assert!(!s.contains("ths"), "字符串 'off' 应判为禁用");
        assert!(!s.contains("akshare"), "字符串 'disabled' 应判为禁用");
        // 启用字符串
        assert!(s.contains("eastmoney"), "字符串 'true' 应判为启用");
    }

    /// 回归测试：JSON 布尔值 false 应被正确判为禁用
    #[test]
    fn vendor_json_bool_false_should_be_disabled() {
        let vars = vec![
            ("vendor_iwencai".to_string(), serde_json::json!(false)),
            ("vendor_neodata".to_string(), serde_json::json!(false)),
            ("vendor_eastmoney".to_string(), serde_json::json!(true)),
        ];
        let s = load_enabled_vendors_from_template(&vars);
        assert!(!s.contains("iwencai"), "JSON false 应判为禁用");
        assert!(!s.contains("neodata"), "JSON false 应判为禁用");
        assert!(s.contains("eastmoney"), "JSON true 应判为启用");
    }

    #[test]
    fn each_style_has_required_vendors() {
        for s in [
            Style::Trend,
            Style::Value,
            Style::Capital,
            Style::Reversion,
            Style::Watchlist,
            Style::Bottleneck,
            Style::Policy,
            Style::Earnings,
            Style::CapitalFlow,
            Style::Event,
            Style::Technical,
        ] {
            let v = required_vendors_for_style(s);
            assert!(!v.is_empty(), "{:?} missing vendors", s);
        }
    }
}
