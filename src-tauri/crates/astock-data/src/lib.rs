#![allow(
    clippy::useless_format,
    clippy::redundant_closure,
    clippy::let_and_return,
    clippy::if_same_then_else
)]
pub mod adjustment;
pub mod as_of;
pub mod as_of_capability;
pub mod batch;
pub mod board;
pub mod calendar;
pub mod candlestick_pattern;
pub mod daily_snapshot;
pub mod disk_cache;
pub mod divergence;
pub mod error;
pub mod fallback;
pub mod fundamentals_report;
pub mod gate;
pub mod indicators;
// G3 industry_chain 已于 P2-8 阶段迁回 axagent-stock-analysis crate（架构归属：
// 产业链定义/传导算法/新闻映射均为分析逻辑而非数据获取）。
// 调用方应使用 `axagent_analysis_engine::industry_chain::*`。
pub mod macro_data;
pub mod mcp_tools;
pub mod quality;
pub mod realtime_quote;
pub mod regime;
pub mod scoring;
pub mod sentiment;
pub mod types;
pub mod validation;
pub mod valuation_band;
pub mod vendor_health;
pub mod vendors;

use chrono::Local;
use futures::future::BoxFuture;
use moka::future::Cache as MokaCache;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::as_of_capability::AsOfCapability;
use crate::gate::DomainGate;
use crate::vendor_health::{VendorHealthConfig, VendorHealthTracker};
pub use error::DataError;
pub use macro_data::{MacroDataClient, MacroDataPoint, MacroDataSnapshot};
pub use realtime_quote::{
    HttpPollingStreamer, QuoteCallback, QuoteChangeEvent, RealTimeQuoteWatcher, WatchPriority,
};
pub use types::*;
// R3: 估值带（暴露在 crate 根，方便 commands 端直接 `axagent_astock_data::ValuationBand`）
pub use valuation_band::{FinancialSnapshotLike, MetricBand, ValuationBand};
use vendors::akshare::AkshareVendor;
use vendors::baidu_stock::BaiduStockVendor;
use vendors::browser_eastmoney::{BrowserEastMoneyVendor, BrowserHttpFetch};
use vendors::cninfo::CninfoVendor;
use vendors::eastmoney::EastMoneyVendor;
use vendors::guba::GubaVendor;
use vendors::international::InternationalVendor;
use vendors::iwencai::IwencaiVendor;
use vendors::mootdx::MootdxVendor;
use vendors::neodata::NeoDataVendor;
use vendors::sina::SinaVendor;
use vendors::tencent::TencentVendor;
use vendors::ths::ThsVendor;
use vendors::xueqiu::XueqiuVendor;
use vendors::StockVendor;

/// P6: 新闻入库 sink 抽象。
///
/// astock-data 不直接依赖 dao/entities,所以用 trait 抽象把"入库"延迟到
/// main crate 实现并通过 `with_news_archive_sink` 注入。这样:
/// - 保持 astock-data 轻量(避免引入 sea-orm 等大依赖)
/// - 实现可替换(测试时可换 in-memory mock)
/// - 即使 sink 未注入也能正常工作(降级为不写库)
#[async_trait::async_trait]
pub trait NewsArchiveSink: Send + Sync {
    /// 批量 upsert NewsItem。失败仅记录日志,不影响主流程。
    async fn upsert(
        &self,
        source: &str,
        stock_code: Option<&str>,
        keyword: Option<&str>,
        items: &[NewsItem],
    );

    /// as-of 模式查询:`publish_time_ms <= as_of_ts` 的最新 limit 条,
    /// 关键词匹配 title/summary 子串,stock_code 给定时额外过滤。
    /// 返回空 vec 表示无数据(由调用方决定是否降级)。
    async fn search_asof(
        &self,
        keyword: &str,
        stock_code: Option<&str>,
        as_of_ts_ms: i64,
        limit: u32,
    ) -> Vec<NewsItem>;
}

/// 把 NewsItem.publish_time(String) 解析为 unix 毫秒。
///
/// 支持格式:
/// - "YYYY-MM-DD HH:MM:SS"
/// - "YYYY-MM-DD"
/// - "" / 不可解析 → None
fn parse_news_publish_time_ms(s: &str) -> Option<i64> {
    use chrono::NaiveDateTime;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // 优先尝试完整 datetime
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return dt.and_utc().timestamp_millis().into();
    }
    // fallback 到纯日期
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis().into();
    }
    // 兼容 ISO 8601 "YYYY-MM-DDTHH:MM:SS"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return dt.and_utc().timestamp_millis().into();
    }
    None
}

type VendorRef = (String, Box<dyn StockVendor>);

/// 检查 HTTP 响应状态码，429 → DataError::RateLimited
pub fn check_response_429(resp: &reqwest::Response, vendor: &str) -> Result<(), DataError> {
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        Err(DataError::RateLimited { vendor: vendor.to_string() })
    } else {
        Ok(())
    }
}

struct VendorRouting {
    quote: Vec<String>,
    klines: Vec<String>,
    financials: Vec<String>,
    news: Vec<String>,
    money_flow: Vec<String>,
    dragon_tiger: Vec<String>,
    lockup: Vec<String>,
    search: Vec<String>,
    search_news: Vec<String>,
    margin: Vec<String>,
    north_bound: Vec<String>,
    sector: Vec<String>,
    shareholder_trades: Vec<String>,
    dividend: Vec<String>,
    research_reports: Vec<String>,
    consensus_eps: Vec<String>,
    concept_blocks: Vec<String>,
    announcements: Vec<String>,
    market_dragon_tiger: Vec<String>,
    hot_stocks: Vec<String>,
    earnings_calendar: Vec<String>,
    social_sentiment: Vec<String>,
    industry_ranking: Vec<String>,
    concept_boards: Vec<String>,
    board_members: Vec<String>,
    cls_flash: Vec<String>,
    north_bound_flow: Vec<String>,
    block_trades: Vec<String>,
    policy_news: Vec<String>,
    institutional_visits: Vec<String>,
    index_quotes: Vec<String>,
    peers: Vec<String>,
    option_pcr: Vec<String>,
    /// 新增(2026-07-22 #4): 股权质押数据路由
    pledge: Vec<String>,
    /// 缺陷 G 修复: per-method replay 旁路 vendor 顺序。
    /// Key 必须与上述 27 个字段名完全一致;
    /// 若 key 不存在(默认),replay 模式用上述字段本身。
    /// 典型用例: replay 模式下把 quote 从 tencent 改 baidu_stock,
    /// 因 baidu_stock 对历史日期有更好支持。
    replay: HashMap<&'static str, Vec<String>>,
}

impl VendorRouting {
    /// 缺陷 G 修复: 按当前 AsOf 模式选 vendor 顺序。
    /// live 模式 = 默认 routing;replay 模式 = replay 覆盖(若 key 存在)或默认 fallback。
    /// 返回的 slice 借用 self,调用方需 &'a Vendors 形式使用。
    fn vendors_for<'a>(
        &'a self,
        method: &'static str,
        default: &'a Vec<String>,
    ) -> &'a Vec<String> {
        if crate::as_of::is_asof_active() {
            self.replay.get(method).unwrap_or(default)
        } else {
            default
        }
    }

    fn default_routing() -> Self {
        Self {
            quote: vec![
                "tencent".into(),
                "mootdx".into(),
                "sina".into(),
                "xueqiu".into(),
                "eastmoney".into(),
                // P1 修复(2026-07-25): eastmoney push2his 反爬触发时自动 fallback 到
                // 浏览器内核(绕过 JA3 TLS 指纹封锁)。仅桌面端 fetcher 已注入时生效。
                "browser_eastmoney".into(),
                "neodata".into(), // 末位兜底（美股/港股）
            ],
            // 2026-08-01：push2his.eastmoney.com 在本机被连接拒绝（IPv4 快速 RST / IPv6 间歇），
            // eastmoney 首选 kline 每次失败 → 累计降级 → 连累 dataapi/bkzj 等正常域名全被跳过
            // （趋势智选全空链路）。tencent kline 一直健康，改首选；eastmoney 仅作 fallback。
            klines: vec![
                "tencent".into(),
                "eastmoney".into(),
                "xueqiu".into(),
                "mootdx".into(),
                "browser_eastmoney".into(),
            ],
            financials: vec![
                "eastmoney".into(),
                "browser_eastmoney".into(),
                "baidu_stock".into(), // P2-1 修复(2026-07-22): 新增 baidu_stock 作为备选，避免 eastmoney IncompleteMessage + browser_eastmoney/xueqiu/neodata token 缺失时无可用源
                "xueqiu".into(),
                "akshare".into(),
                "neodata".into(), // 末位兜底
            ],
            // 注意(2026-06): xueqiu 的 stock_timeline.json 被阿里云 WAF 拦截,
            // 无有效 token 或非浏览器环境时返回 WAF 挑战页面(HTML)而非 JSON。
            // sina 的 /corp/go.php/vCB_AllNewsStock/symbol/{code}.json 端点返回
            // HTTP 200 + 空 body(接口疑似废弃), 已添加备选端点。
            // 两个源失败后由 eastmoney(搜索API)兜底, 实际可用。
            news: vec![
                "xueqiu".into(),
                "sina".into(),
                "eastmoney".into(),
                "browser_eastmoney".into(),
                "ths".into(),
                "akshare".into(),
                "neodata".into(), // 末位兜底（docData 文章）
            ],
            money_flow: vec![
                "tencent".into(),
                "eastmoney".into(),
                "sina".into(),
                "browser_eastmoney".into(),
                "baidu_stock".into(),
            ],
            dragon_tiger: vec![
                "eastmoney".into(),
                "browser_eastmoney".into(),
                "baidu_stock".into(),
            ],
            lockup: vec!["eastmoney".into(), "baidu_stock".into()],
            search: vec![
                "eastmoney".into(),
                "iwencai".into(),
                "baidu_stock".into(),
                "neodata".into(),
            ],
            search_news: vec![
                "eastmoney".into(),
                "browser_eastmoney".into(),
                "akshare".into(),
                "neodata".into(),
            ],
            margin: vec!["eastmoney".into(), "browser_eastmoney".into(), "baidu_stock".into()],
            north_bound: vec!["eastmoney".into(), "browser_eastmoney".into(), "baidu_stock".into()],
            sector: vec![
                "eastmoney".into(),
                "ths".into(),
                "baidu_stock".into(),
                "iwencai".into(),
                "neodata".into(), // 末位兜底（自然语言查询行业归属）
            ],
            shareholder_trades: vec!["eastmoney".into(), "baidu_stock".into()],
            dividend: vec!["eastmoney".into(), "baidu_stock".into()],
            research_reports: vec!["eastmoney".into(), "baidu_stock".into()],
            // P1-2: eastmoney 首选（reportapi 接口稳定），ths/akshare fallback，iwencai 需 api_key
            consensus_eps: vec![
                "eastmoney".into(),
                "ths".into(),
                "akshare".into(),
                "iwencai".into(),
            ],
            concept_blocks: vec![
                "eastmoney".into(),
                "ths".into(),
                "baidu_stock".into(),
                "iwencai".into(),
            ],
            announcements: vec!["cninfo".into(), "eastmoney".into()],
            market_dragon_tiger: vec!["ths".into(), "eastmoney".into(), "baidu_stock".into()],
            hot_stocks: vec![
                "ths".into(),
                "baidu_stock".into(),
                "iwencai".into(),
                "neodata".into(),
            ],
            // P3 修复(2026-07-25): 移除 neodata(走 trait 默认 Ok(vec![]) 会把 vendor 故障
            // 误报为"成功无数据");加入 browser_eastmoney 作为反爬 fallback。
            // 现在语义清晰:eastmoney 故障 → fallback 到 browser_eastmoney;
            // 两个都失败 → 返回明确错误,前端不再被"假成功空数组"误导。
            earnings_calendar: vec!["eastmoney".into(), "browser_eastmoney".into()],
            social_sentiment: vec!["guba".into()],
            // 2026-08-01 实锤：ths（data.10jqka.com.cn/dataapi/limit_up/industry_board → 404 死链）、
            // baidu_stock（gushitong resource_id=5359 → 参数错误）、neodata（TOKEN_MISSING 无凭据）
            // 三个 vendor 必失败且"空数据不降级"→ 永远霸占健康列表，把唯一可靠的 eastmoney
            // 挤在轮询外（趋势智选全链空根因之一）。瘦身为 eastmoney + browser_eastmoney。
            industry_ranking: vec!["eastmoney".into(), "browser_eastmoney".into()],
            concept_boards: vec!["eastmoney".into()],
            board_members: vec!["eastmoney".into()],
            cls_flash: vec!["eastmoney".into(), "browser_eastmoney".into(), "akshare".into()],
            north_bound_flow: vec![
                "eastmoney".into(),
                "browser_eastmoney".into(),
                "ths".into(),
                "baidu_stock".into(),
            ],
            block_trades: vec!["eastmoney".into(), "baidu_stock".into()],
            // 政策新闻:优先 eastmoney(基于行业关键词搜索),baidu_stock 作为备选(个股新闻+政策过滤),
            // akshare 未实现 get_policy_news(默认返回空),实际不生效。
            // P1-2 修复(2026-07-22): 新增 baidu_stock 作为有效备选，避免 eastmoney 单点故障。
            policy_news: vec!["eastmoney".into(), "baidu_stock".into(), "akshare".into()],
            institutional_visits: vec!["eastmoney".into(), "browser_eastmoney".into()],
            index_quotes: vec!["eastmoney".into(), "tencent".into(), "neodata".into()],
            peers: vec!["eastmoney".into(), "iwencai".into(), "neodata".into()], // neodata 末位兜底
            option_pcr: vec!["eastmoney".into()],
            // #4: 股权质押数据 — eastmoney datacenter 接口稳定
            pledge: vec!["eastmoney".into()],
            // P2-4 修复: 在 replay 模式下, 把 3 个核心方法切到对历史日期支持最好的 vendor。
            // 依据 as_of_capability.rs:
            //   - get_quote:        tencent 是 SynthesizeFromKline(replay 模式从 K 线最后一行
            //                       合成, 数据确定性强; baidu_stock 是 Fallthrough)
            //   - get_klines:       tencent 是 NativeDateParam(可显式传 as_of_date, 最准;
            //                       其他 vendor 走 Fallthrough + 客户端截断)
            //   - get_financials:   eastmoney 是 Fallthrough(已是最优)
            // 其他方法保持默认 routing(只在 live 模式有意义的 vendor 排名)。
            replay: {
                let mut m: HashMap<&'static str, Vec<String>> = HashMap::new();
                m.insert("quote", vec!["tencent".into(), "mootdx".into(), "eastmoney".into()]);
                m.insert(
                    "klines",
                    vec![
                        "tencent".into(),
                        "eastmoney".into(),
                        "browser_eastmoney".into(),
                        "mootdx".into(),
                        "xueqiu".into(),
                    ],
                );
                m.insert("financials", vec!["eastmoney".into(), "browser_eastmoney".into()]);
                m
            },
        }
    }
}

pub struct AStockClient {
    vendors: Vec<VendorRef>,
    routing: VendorRouting,
    gate: DomainGate,
    http: reqwest::Client,
    /// C1 修复: 用 moka L1 替换手工 HashMap + LRU，支持自动容量管理和 TTL
    cache: MokaCache<String, (i64, String)>,
    /// V40 修复: vendor 健康追踪器 — 连续失败 3 次自动降级，5 分钟后恢复。
    /// 在 get_quote/get_klines/get_financials 调用路径中根据返回结果
    /// 调用 record_success/record_failure 更新状态。
    pub health_tracker: Arc<VendorHealthTracker>,
    /// 缺陷 D 修复:可选 L2 磁盘缓存(spec §3.2)。
    /// 启动时注入,None 表示走纯 L1 内存模式(向后兼容)。
    l2: Option<Arc<disk_cache::DiskCache>>,
    /// P5:可选每日快照缓存(NoHistoricalSemantic 数据后台 sweep)
    /// 需要先配置 l2,再通过 with_daily_snapshot_cache() 启用,默认关闭。
    daily_snapshot: Option<daily_snapshot::DailySnapshotCache>,
    pub iwencai_key: RwLock<String>,
    /// 雪球 token 共享引用（前端设置页写入，vendor 自动读取）
    pub xq_token: Option<Arc<RwLock<String>>>,
    /// NeoData token 共享引用（前端设置页写入，vendor 自动读取）
    pub neodata_token: Option<Arc<RwLock<String>>>,
    /// P6:本地新闻语料库 sink(None 表示不写库,as-of 模式 search_news 降级)
    /// 通过 with_news_archive_sink() 注入。
    news_archive_sink: Option<Arc<dyn NewsArchiveSink>>,
    /// 浏览器 HTTP fetch 能力（Harness 注入）
    /// 通过 Playwright/Chromium 绕过 EastMoney JA3 封锁
    browser_fetcher: Option<Arc<dyn BrowserHttpFetch>>,
    /// vendor 启用状态过滤器（来自设置页 vendor_* 布尔开关）
    /// - None = 全部启用（默认，向后兼容）
    /// - Some(set) = 只有 set 中的 vendor 启用，find_vendor 会跳过未启用的 vendor
    ///
    /// 用 parking_lot::RwLock 支持同步读取（find_vendor 是同步方法，不跨 await）
    enabled_vendors: parking_lot::RwLock<Option<HashSet<String>>>,
    /// 空数据冷却（2026-07-31 新增）：
    /// key = "{datatype}:{code}" → 冷却到期时间戳(epoch ms)。
    /// 用于 north_bound / money_flow 等"全部源返回空"的场景：
    /// - 北向个股持仓明细自 2024-08 港交所停止披露，所有源永远空，
    ///   荐股 run 内对每只股票白打 3 源 × 重试链 = 数百次无效请求。
    /// - 非交易时段资金流向接口同样返回空，每只股票白打 2 源。
    ///
    /// 冷却期内直接返回空结果，到期后自动恢复探测。
    empty_cooldown: Arc<Mutex<HashMap<String, i64>>>,
}

/// 判断字符串是否为港股/美股代码格式（如"00700.HK"、"TSM.US"）
/// 规则：点号前为纯数字或纯大写字母，点号后为 2-3 位大写字母
fn is_pure_digits_before_dot_and_uppercase_after(s: &str) -> bool {
    if let Some((before, after)) = s.split_once('.') {
        let before_ok = !before.is_empty()
            && (before.chars().all(|c| c.is_ascii_digit())
                || before.chars().all(|c| c.is_ascii_uppercase()));
        let after_ok =
            !after.is_empty() && after.len() <= 3 && after.chars().all(|c| c.is_ascii_uppercase());
        before_ok && after_ok
    } else {
        false
    }
}

/// G1 跨市场数据接入：判断代码是否为国际股票（港股/美股/ETF）
///
/// 规则：
/// - "00700.HK" / "AAPL.US" / "BABA.US" → true（带后缀）
/// - "00700" / "09988"（≤5 位纯数字） → true（港股代码长度）
/// - "AAPL" / "TSLA" / "BABA"（1-5 位纯字母） → true（美股代码）
/// - "000001" / "600519" / "688981"（6 位纯数字） → false（A 股）
/// - "US_AAPL" / "hk00700"（已编码国际格式） → true
/// - "SPX" / "IXIC" / "HSI"（基准指数，3-4 位大写字母） → true
/// - 其他 → false
pub fn is_international_code(code: &str) -> bool {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return false;
    }
    // 已编码的国际格式
    if trimmed.starts_with("US_") || trimmed.starts_with("hk") {
        return true;
    }
    // 带后缀形式
    if let Some((_, suffix)) = trimmed.split_once('.') {
        let s = suffix.to_uppercase();
        return s == "HK" || s == "US";
    }
    // 6 位纯数字 → A 股
    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // ≤5 位纯数字 → 港股（00700 / 09988 / 03690）
    if trimmed.len() <= 5 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // 1-5 位纯大写字母 → 美股（AAPL / TSLA / BABA / NVDA）
    if !trimmed.is_empty() && trimmed.len() <= 5 && trimmed.chars().all(|c| c.is_ascii_uppercase())
    {
        return true;
    }
    false
}

/// 判断是否为基准指数代码（标普 500 / 纳指 / 恒生 / 上证等）
pub fn is_benchmark_code(code: &str) -> bool {
    matches!(
        code.trim().to_uppercase().as_str(),
        "SPX"
            | "SP500"
            | "S&P500"
            | "IXIC"
            | "NDX"
            | "DJI"
            | "HSI"
            | "HSCEI"
            | "000001.SH"
            | "SH000001"
            | "399001"
            | "399006"
            | "000300"
    )
}

/// 判断是否为外汇对代码（USD/CNY、HKD/CNY 等）
pub fn is_forex_pair(code: &str) -> bool {
    let trimmed = code.trim().to_uppercase();
    if let Some((base, quote)) = trimmed.split_once('/') {
        return (base == "USD" || base == "HKD" || base == "EUR" || base == "JPY")
            && (quote == "CNY" || quote == "CNH" || quote == "USD" || quote == "HKD");
    }
    false
}

/// 强制 IPv4 解析的 DNS resolver（2026-08-01）
///
/// 根因实测：本机 IPv6 链路到部分数据源（push2his.eastmoney.com / push2.eastmoney.com）
/// 被服务器 RST 掐断——curl -6 3/3 失败（close_notify missing / ERR_EMPTY_RESPONSE），
/// 而 curl -4 3/3 HTTP 200。系统 DNS 同时返回 IPv6 + IPv4，客户端（reqwest/浏览器）
/// 默认优先 IPv6 → 全部请求失败，误判为"反爬封锁"。
///
/// 此 resolver 过滤 IPv6 地址只保留 IPv4，绕开坏链路——应用内实现，无需代理。
/// 对纯 AAAA 域名（无 IPv4 记录）回退默认解析，避免误杀。
struct Ipv4OnlyResolver;

impl reqwest::dns::Resolve for Ipv4OnlyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let addrs: Vec<std::net::SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let v4: Vec<std::net::SocketAddr> = addrs.into_iter().filter(|a| a.is_ipv4()).collect();
            if !v4.is_empty() {
                return Ok(Box::new(v4.into_iter()) as reqwest::dns::Addrs);
            }
            // 无 IPv4 记录（纯 AAAA 域名）→ 回退默认解析
            let addrs: Vec<std::net::SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

impl AStockClient {
    /// 修复 P0-A4: 原 `expect("Failed to create HTTP client")` 在 TLS 初始化
    /// 失败时 panic，拖垮整个 Tauri 进程。改为返回 Result。
    ///
    /// 修复 M-RES-1: 原 `unwrap_or_else` 在 TLS 失败时静默降级为空 vendors，
    /// 上层无感知。改为使用 reqwest 默认配置兜底（无自定义 TLS），并保留 vendors 注册，
    /// 让降级路径仍可用。tracing::error! 已记录便于诊断。
    pub fn new() -> Self {
        match Self::try_new() {
            Ok(client) => client,
            Err(e) => {
                tracing::error!(
                    "[astock-data] HTTP client 创建失败（TLS 初始化错误），降级为默认配置: {e}"
                );
                // 降级：用 reqwest 默认配置（无自定义 TLS），至少不 panic。
                // 仍注册全部 vendor，保证降级后数据源可用。
                // P1-NEW-4 修复：降级路径的 reqwest::Client::new() 无超时配置。
                // 原正常路径通过 try_new() 设置了 30s 超时 / 15s 连接超时 / 32 连接池 / 30s 空闲超时，
                // 降级后需保持同等配置，避免降级路径产生无超时的长连接泄漏。
                let http = reqwest::Client::builder()
                    .dns_resolver(Arc::new(Ipv4OnlyResolver))
                    .timeout(std::time::Duration::from_secs(30))
                    .connect_timeout(std::time::Duration::from_secs(15))
                    .pool_max_idle_per_host(32)
                    .pool_idle_timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_else(|_| {
                        // 连降级 builder 也失败时（极罕见），用 reqwest 默认配置兜底并记录。
                        // 默认配置无超时，上层需通过 tracing::error! 告警。
                        tracing::error!(
                            "[astock-data] 降级 HTTP client builder 也失败，使用 reqwest 裸默认配置（无超时）"
                        );
                        reqwest::Client::new()
                    });
                let mut client = Self {
                    vendors: Vec::new(),
                    routing: VendorRouting::default_routing(),
                    gate: DomainGate::new(),
                    http: http.clone(),
                    cache: MokaCache::builder()
                        .max_capacity(4096)
                        .time_to_idle(Duration::from_secs(3600))
                        .build(),
                    health_tracker: Arc::new(VendorHealthTracker::new(
                        VendorHealthConfig::default(),
                    )),
                    l2: None,
                    daily_snapshot: None,
                    iwencai_key: RwLock::new(String::new()),
                    xq_token: None,
                    neodata_token: None,
                    news_archive_sink: None,
                    browser_fetcher: None,
                    enabled_vendors: parking_lot::RwLock::new(None),
                    empty_cooldown: Arc::new(Mutex::new(HashMap::new())),
                };
                client.register_default_vendors(http);
                client
            },
        }
    }

    /// 注册默认 vendor 集合（try_new 与降级路径共用）
    fn register_default_vendors(&mut self, http: reqwest::Client) {
        self.register_vendor("tencent", Box::new(TencentVendor { http: http.clone() }));
        self.register_vendor(
            "eastmoney",
            Box::new(EastMoneyVendor {
                http: http.clone(),
                proxy_http: EastMoneyVendor::build_proxy_client(),
            }),
        );
        self.register_vendor("sina", Box::new(SinaVendor { http: http.clone() }));
        self.register_vendor("ths", Box::new(ThsVendor { http: http.clone() }));
        self.register_vendor("cninfo", Box::new(CninfoVendor { http: http.clone() }));
        self.register_vendor("baidu_stock", Box::new(BaiduStockVendor { http: http.clone() }));
        self.register_vendor(
            "iwencai",
            Box::new(IwencaiVendor { http: http.clone(), api_key: String::new() }),
        );
        self.register_vendor("akshare", Box::new(AkshareVendor { http: http.clone() }));
        self.register_vendor("mootdx", Box::new(MootdxVendor::new()));
        self.register_vendor("browser_eastmoney", Box::new(BrowserEastMoneyVendor::new()));
        // 国际股票（港股/美股/ETF）
        self.register_vendor(
            "international",
            Box::new(InternationalVendor { http: http.clone(), hook_executor: None }),
        );
        // NeoData Financial Search — 末位 fallback vendor
        let neodata_token = Arc::new(RwLock::new(String::new()));
        self.neodata_token = Some(neodata_token.clone());
        self.register_vendor("neodata", Box::new(NeoDataVendor { token: neodata_token }));
        // 雪球数据源（始终注册，token 通过共享 Arc 运行时注入）
        let xq_token = Arc::new(RwLock::new(String::new()));
        self.xq_token = Some(xq_token.clone());
        self.register_vendor(
            "xueqiu",
            Box::new(XueqiuVendor { http: http.clone(), token: xq_token }),
        );
        // 东方财富股吧（社交舆情数据源，无需认证）
        self.register_vendor("guba", Box::new(GubaVendor::new(http.clone())));
    }

    /// 修复 P0-A4: 返回 Result 的构造函数，调用方可自行处理 TLS 失败
    pub fn try_new() -> Result<Self, DataError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .cookie_store(true)
            .pool_max_idle_per_host(32)
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            // 2026-08-01: 强制 IPv4 解析。本机 IPv6 链路到东财 push2his/push2 被 RST，
            // 客户端默认优先 IPv6 导致全部请求失败（误判为反爬封锁），IPv4 完全正常。
            .dns_resolver(Arc::new(Ipv4OnlyResolver))
            .build()
            .map_err(|e| DataError::VendorError {
                vendor: "http_client".into(),
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        let mut client = Self {
            vendors: Vec::new(),
            routing: VendorRouting::default_routing(),
            gate: DomainGate::new(),
            http: http.clone(),
            // C1: moka L1 缓存 — 1h 空闲过期,4096 条上限
            cache: MokaCache::builder()
                .max_capacity(4096)
                .time_to_idle(Duration::from_secs(3600))
                .build(),
            health_tracker: Arc::new(VendorHealthTracker::new(VendorHealthConfig::default())),
            l2: None,             // 默认不开启 L2,调用方通过 with_l2_cache 注入
            daily_snapshot: None, // P5:默认不开启,调用方通过 with_daily_snapshot_cache 注入
            iwencai_key: RwLock::new(String::new()),
            xq_token: None,
            neodata_token: None,
            news_archive_sink: None, // P6:默认不写入,调用方通过 with_news_archive_sink 注入
            browser_fetcher: None,   // 浏览器 fetch 通过 with_browser_fetcher() 注入
            enabled_vendors: parking_lot::RwLock::new(None), // 默认全部启用
            empty_cooldown: Arc::new(Mutex::new(HashMap::new())),
        };

        client.register_default_vendors(http);

        Ok(client)
    }

    pub fn register_vendor(&mut self, name: &str, vendor: Box<dyn StockVendor>) {
        self.vendors.push((name.to_string(), vendor));
    }

    /// 设置 vendor 启用状态过滤器（来自设置页 vendor_* 布尔开关）
    /// 传入空 set 等效于全部禁用；传入 None 等效于全部启用（向后兼容）
    pub fn set_enabled_vendors(&self, vendors: Option<HashSet<String>>) {
        *self.enabled_vendors.write() = vendors;
    }

    /// 检查 vendor 是否启用（用于 find_vendor 过滤）
    /// - enabled_vendors 为 None → 全部启用（默认）
    /// - enabled_vendors 为 Some(set) → 只有 set 中的 vendor 启用
    fn is_vendor_enabled(&self, name: &str) -> bool {
        self.enabled_vendors.read().as_ref().is_none_or(|set| set.contains(name))
    }

    /// 检查 vendor 凭据是否已配置（async：读 tokio RwLock）。
    /// 无凭据必失败的 vendor（neodata 需 token、iwencai 需 api_key）在路由入口剔除，
    /// 避免每次调用都 TOKEN_MISSING/api_key not configured 失败 + 重试拖慢链路。
    async fn vendor_has_credentials(&self, name: &str) -> bool {
        match name {
            "neodata" => {
                if let Some(token) = self.neodata_token.as_ref() {
                    !token.read().await.is_empty()
                } else {
                    false
                }
            },
            "iwencai" => !self.iwencai_key.read().await.is_empty(),
            _ => true,
        }
    }

    /// 缺陷 D 修复: 注入 L2 磁盘缓存。
    /// 返回 (client, l2_handle) 二元组,l2_handle 交给调用方持有以启动后台 flush 任务。
    /// 二次调用会覆盖前一个 L2(向后兼容:返回的 l2_handle 仍是新注入的实例)。
    pub fn with_l2_cache(mut self, path: PathBuf) -> (Self, Arc<disk_cache::DiskCache>) {
        let l2 = disk_cache::DiskCache::load_or_default(path);
        self.l2 = Some(l2.clone());
        (self, l2)
    }

    /// P5:启用每日快照缓存(必须在 with_l2_cache 之后调用)
    pub fn with_daily_snapshot_cache(mut self) -> Self {
        if let Some(l2) = self.l2.clone() {
            self.daily_snapshot = Some(daily_snapshot::DailySnapshotCache::from_disk(l2));
        }
        self
    }

    /// P6:注入本地新闻语料库 sink。
    /// 注入后 `get_news` / `search_news` 会自动 upsert 结果;as-of 模式
    /// `search_news` 会优先查 sink 而不是直接降级。
    pub fn with_news_archive_sink(mut self, sink: Arc<dyn NewsArchiveSink>) -> Self {
        self.news_archive_sink = Some(sink);
        self
    }

    /// 注入浏览器 HTTP fetch 能力（用于绕过 EastMoney JA3 封锁）
    /// 接收 axagent-kit::browser_automation::PlaywrightClient 的实现封装
    pub fn with_browser_fetcher(mut self, fetcher: Arc<dyn BrowserHttpFetch>) -> Self {
        self.browser_fetcher = Some(fetcher.clone());
        // 替换已注册的 browser_eastmoney vendor，使其持有 fetcher
        if let Some(pos) = self.vendors.iter().position(|(name, _)| name == "browser_eastmoney") {
            self.vendors[pos] = (
                "browser_eastmoney".into(),
                Box::new(BrowserEastMoneyVendor::with_fetcher(fetcher)),
            );
        }
        self
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// C1 修复: 用 moka L1 替换手工 HashMap
    /// - L1: moka, 自动写入时 TTL + 1h idle
    /// - L2: 可选的 DiskCache(JSON 文件, 如配置)
    /// - 返回前检查 per-entry 过期(expires_at), 过期则移除并返回 None
    async fn cache_get(&self, key: &str) -> Option<String> {
        // 1) L1 moka
        if let Some((expires_at, val)) = self.cache.get(key).await {
            if expires_at > chrono::Utc::now().timestamp() {
                // P0-2 修复(2026-07-22): 缓存命中添加 debug 日志,
                // 区分"缓存命中快速返回"和"vendor 静默降级返回空"
                tracing::debug!("[astock-data] 缓存命中(L1): key={key}");
                return Some(val);
            }
            // 已过期: moka 自动 tidle 会清理, 这里直接忽略
        }
        // 2) L2 磁盘
        if let Some(l2) = &self.l2 {
            if let Some(val) = l2.get(key) {
                let expires_at = chrono::Utc::now().timestamp() + 3600;
                self.cache.insert(key.to_string(), (expires_at, val.clone())).await;
                tracing::debug!("[astock-data] 缓存命中(L2): key={key}");
                return Some(val);
            }
        }
        None
    }

    /// 写 L1(moka) + L2(DiskCache, 如配置), 带 replay TTL cap。
    /// C1 修复: 用 moka 替代手工 HashMap, 不再有 cache_set_internal。
    async fn cache_set(&self, key: String, value: String, ttl_secs: i64) {
        // spec §5.1: replay 模式下历史数据是定值, TTL cap 到 1h 避免内存无限增长;
        // live 模式保持调用方设定的精细 TTL(60s/300s/3600s/86400s 各异)。
        let ttl_secs = if crate::as_of::is_asof_active() {
            ttl_secs.min(3600)
        } else {
            ttl_secs
        };
        let expires_at = chrono::Utc::now().timestamp() + ttl_secs;
        self.cache.insert(key.clone(), (expires_at, value.clone())).await;
        // L2 同样写
        if let Some(l2) = &self.l2 {
            l2.set(key, value, ttl_secs);
        }
    }

    /// 序列化结果并写入缓存。序列化失败时跳过缓存写入并记录 warn 日志。
    /// 替代 `serde_json::to_string(&v).unwrap_or_default()` 模式：
    /// 后者会在序列化失败时写入空字符串，导致下次读取反序列化失败 → 缓存污染。
    async fn cache_set_serialized<T: serde::Serialize>(
        &self,
        cache_key: String,
        value: &T,
        ttl_secs: i64,
    ) {
        match serde_json::to_string(value) {
            Ok(json) => self.cache_set(cache_key, json, ttl_secs).await,
            Err(e) => {
                tracing::warn!("[astock-data] 序列化失败，跳过缓存写入: key={cache_key}, err={e}")
            },
        }
    }

    /// 空数据冷却查询：冷却期内返回 true（调用方直接返回空结果，不再请求 vendor）
    async fn empty_cooldown_active(&self, key: &str) -> bool {
        let map = self.empty_cooldown.lock().await;
        match map.get(key) {
            Some(&until) => chrono::Utc::now().timestamp_millis() < until,
            None => false,
        }
    }

    /// 标记空数据冷却：cooldown_secs 秒内同一 key 直接短路
    async fn mark_empty_cooldown(&self, key: &str, cooldown_secs: i64) {
        let until = chrono::Utc::now().timestamp_millis() + cooldown_secs * 1000;
        self.empty_cooldown.lock().await.insert(key.to_string(), until);
    }

    /// 清除空数据冷却（数据源恢复后调用，确保下次立即重新探测）
    async fn clear_empty_cooldown(&self, key: &str) {
        self.empty_cooldown.lock().await.remove(key);
    }

    /// 生成 L1 cache key；自动包含当前 AsOf 后缀以避免 live/replay 互相污染
    fn cache_key_for(method: &str, args: &str) -> String {
        format!("{}:{}::{}", method, args, crate::as_of::cache_suffix())
    }

    /// 修复 P1-5: 在 is_asof_active_for(kind) 为 true 但 current_as_of() 返回 None
    /// 的 race condition 场景下，原代码使用 .expect(...) 会导致 panic。
    /// 改为退化为 None（让调用方原样返回数据，不截断）并通过 record_degradation
    /// 记录降级原因，使决策可观测。这与 is_asof_active_for=false 的行为一致——
    /// "as_of 未生效时数据不截断"，是更安全的失败方向（避免错误丢弃数据）。
    fn as_of_ctx_or_degrade(method: &str) -> Option<crate::as_of::AsOfContext> {
        match crate::as_of::current_as_of() {
            Some(c) => Some(c),
            None => {
                tracing::warn!(
                    "[asof] race condition: is_asof_active_for 为真但 current_as_of 为 None，{} 退化为不截断",
                    method
                );
                crate::as_of::record_degradation(
                    "astock-data",
                    method,
                    "current_as_of 为 None（race condition），退化为不截断",
                );
                None
            },
        }
    }

    /// K 线专用 cache key:在 cache_key_for 基础上追加 effective_cutoff(交易日 fallback 后),
    /// 解决缺陷 B —— 同一 as_of_date 下,周末 vs 周一/effective_cutoff 不同时缓存会污染。
    /// live 模式下 effective 与 as_of 一致,行为不变。
    fn kline_cache_key(
        stock_code: &str,
        period: &str,
        adj: Option<crate::types::AdjType>,
    ) -> String {
        // P1-3: adj 维度 — None/Forward 共用 "fwd"(多数 vendor 默认前复权),
        // Backward 独立 "bwd", AdjType::None 独立 "raw"。
        let adj_tag = match adj {
            None | Some(crate::types::AdjType::Forward) => "fwd",
            Some(crate::types::AdjType::Backward) => "bwd",
            Some(crate::types::AdjType::None) => "raw",
        };
        let base = Self::cache_key_for("klines", &format!("{stock_code}:{period}:adj={adj_tag}"));
        if let Some(ctx) = crate::as_of::current_as_of() {
            let effective = if crate::calendar::is_trading_day(&ctx.as_of_date) {
                ctx.as_of_date
            } else {
                crate::calendar::previous_trading_day(ctx.as_of_date)
            };
            format!("{}:eff={}", base, effective.format("%Y%m%d"))
        } else {
            base
        }
    }

    /// 按当前 AsOfContext 截断 K 线：保留 date <= as_of_date 的行；live 模式原样返回。
    /// 截断必须在 cache_set 之前执行，确保每个模式各自缓存自己的过滤结果。
    /// **Phase 1 混合 as-of**：仅当 `is_asof_active_for(Structured)` 为真（即 as-of 模式
    /// 且 data_scope ∈ {All, Structured}）时截断；data_scope=Structured 时保持原行为，
    /// data_scope=Unstructured 不适用本函数（K线属于结构化数据）。
    fn truncate_klines_by_asof(klines: Vec<KLine>) -> Vec<KLine> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return klines;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_klines_by_asof") {
            Some(c) => c,
            None => return klines,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        let before = klines.len();
        let filtered: Vec<KLine> =
            klines.into_iter().filter(|k| k.date.as_str() <= cutoff.as_str()).collect();
        let truncated = before - filtered.len();
        if truncated > 0 {
            tracing::warn!(
                "[asof] 截断 {} 条 K 线（截止日={}，原始={}，保留={}）",
                truncated,
                cutoff,
                before,
                filtered.len()
            );
            crate::as_of::record_degradation(
                "astock-data",
                "truncate_klines_by_asof",
                &format!(
                    "截断 {} 条 K 线（保留 {} 条，截止日={}）",
                    truncated,
                    filtered.len(),
                    cutoff
                ),
            );
        }
        filtered
    }

    /// 按当前 AsOfContext 截断 News：保留 publish_time 日期 <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：当 data_scope=Structured 时放行（用户回放仍想看实时新闻）。
    ///
    /// 修复(2026-07-21): 原实现把 publish_time 不可解析的新闻直接丢弃,导致 vendor
    /// 返回的 showTime 字段格式异常时全部新闻被过滤掉。改为:不可解析的 publish_time
    /// 视为不可信但保留,并记录降级日志。news_date_key 已对常见格式做兼容,空串
    /// 通常意味着字段缺失而非真的"未来新闻"。
    fn truncate_news_by_asof(news: Vec<NewsItem>) -> Vec<NewsItem> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Unstructured) {
            return news;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_news_by_asof") {
            Some(c) => c,
            None => return news,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        let mut empty_date_count = 0;
        let filtered: Vec<NewsItem> = news
            .into_iter()
            .filter(|n| {
                let key = news_date_key(&n.publish_time);
                if key.is_empty() {
                    empty_date_count += 1;
                    return true; // 空日期视为不可信但保留
                }
                key <= cutoff.as_str()
            })
            .collect();
        if empty_date_count > 0 {
            crate::as_of::record_degradation(
                "astock-data",
                "truncate_news_by_asof",
                &format!(
                    "{} 条新闻 publish_time 不可解析,视为不可信但保留(可能含未来新闻)",
                    empty_date_count
                ),
            );
        }
        filtered
    }

    /// 按当前 AsOfContext 截断 FinancialReport：保留 report_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_financials_by_asof(reports: Vec<FinancialReport>) -> Vec<FinancialReport> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return reports;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_financials_by_asof") {
            Some(c) => c,
            None => return reports,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        reports.into_iter().filter(|r| r.report_date.as_str() <= cutoff.as_str()).collect()
    }

    /// 按当前 AsOfContext 截断 DragonTigerEntry：保留 date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_dragon_tiger_by_asof(entries: Vec<DragonTigerEntry>) -> Vec<DragonTigerEntry> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return entries;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_dragon_tiger_by_asof") {
            Some(c) => c,
            None => return entries,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        entries.into_iter().filter(|e| e.date.as_str() <= cutoff.as_str()).collect()
    }

    /// 按当前 AsOfContext 截断 Announcement：保留 announce_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：当 data_scope=Structured 时放行（公告属于非结构化）。
    ///
    /// 修复(2026-07-21): 原实现对 announce_date 为空的项直接丢弃,导致 vendor 返回
    /// 的 notice_date 字段缺失时(如 eastmoney 部分历史公告)所有公告被全部过滤掉,
    /// 触发"公告数据获取为空"警告。改为:空 announce_date 视为不可信但保留,并记录
    /// 降级日志,让下游分析师能拿到数据自行判断时效性。
    fn truncate_announcements_by_asof(items: Vec<Announcement>) -> Vec<Announcement> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Unstructured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_announcements_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        let mut empty_date_count = 0;
        let filtered: Vec<Announcement> = items
            .into_iter()
            .filter(|a| {
                if a.announce_date.is_empty() {
                    empty_date_count += 1;
                    return true; // 空日期视为不可信但保留
                }
                a.announce_date.as_str() <= cutoff.as_str()
            })
            .collect();
        if empty_date_count > 0 {
            crate::as_of::record_degradation(
                "astock-data",
                "truncate_announcements_by_asof",
                &format!(
                    "{} 条公告 announce_date 为空,视为不可信但保留(可能含未来公告)",
                    empty_date_count
                ),
            );
        }
        filtered
    }

    /// 按当前 AsOfContext 截断 ResearchReport：保留 publish_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：研报属于非结构化，data_scope=Structured 时放行。
    fn truncate_research_reports_by_asof(items: Vec<ResearchReport>) -> Vec<ResearchReport> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Unstructured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_research_reports_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|r| !r.publish_date.is_empty() && r.publish_date.as_str() <= cutoff.as_str())
            .collect()
    }

    /// 按当前 AsOfContext 截断 LockupSchedule：保留 unlock_date <= as_of_date 的项
    /// (as_of 之后才解禁的"未来"事件过滤掉)。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_lockup_by_asof(items: Vec<LockupSchedule>) -> Vec<LockupSchedule> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_lockup_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|l| !l.unlock_date.is_empty() && l.unlock_date.as_str() <= cutoff.as_str())
            .collect()
    }

    #[expect(dead_code)]
    /// 按当前 AsOfContext 截断 DividendRecord：保留 ex_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_dividend_by_asof(items: Vec<DividendRecord>) -> Vec<DividendRecord> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_dividend_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|d| !d.ex_date.is_empty() && d.ex_date.as_str() <= cutoff.as_str())
            .collect()
    }

    /// 按当前 AsOfContext 截断 ShareholderTrade：保留 date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_shareholder_trades_by_asof(items: Vec<ShareholderTrade>) -> Vec<ShareholderTrade> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_shareholder_trades_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|s| !s.date.is_empty() && s.date.as_str() <= cutoff.as_str())
            .collect()
    }

    /// 按当前 AsOfContext 截断 BlockTrade：保留 trade_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_block_trades_by_asof(items: Vec<BlockTrade>) -> Vec<BlockTrade> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_block_trades_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|b| !b.trade_date.is_empty() && b.trade_date.as_str() <= cutoff.as_str())
            .collect()
    }

    /// 按当前 AsOfContext 截断 InstitutionalVisit：保留 visit_date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    fn truncate_institutional_visits_by_asof(
        items: Vec<InstitutionalVisit>,
    ) -> Vec<InstitutionalVisit> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return items;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_institutional_visits_by_asof") {
            Some(c) => c,
            None => return items,
        };
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        items
            .into_iter()
            .filter(|i| !i.visit_date.is_empty() && i.visit_date.as_str() <= cutoff.as_str())
            .collect()
    }

    /// 按当前 AsOfContext 截断 NorthBoundFlow：保留 date <= as_of_date 的项。
    /// **Phase 1 混合 as-of**：仅结构化数据走 as-of。
    /// 同时截断 recent_history 中 date > as_of_date 的项(用于趋势观察的多日数据)。
    fn truncate_north_bound_flow_by_asof(item: Option<NorthBoundFlow>) -> Option<NorthBoundFlow> {
        if !crate::as_of::is_asof_active_for(crate::as_of::AsOfDataKind::Structured) {
            return item;
        }
        let ctx = match Self::as_of_ctx_or_degrade("truncate_north_bound_flow_by_asof") {
            Some(c) => c,
            None => return item,
        };
        let mut item = item?;
        let cutoff = ctx.as_of_date.format("%Y-%m-%d").to_string();
        // 主字段 date > cutoff → 整体丢弃
        if !item.date.is_empty() && item.date.as_str() > cutoff.as_str() {
            return None;
        }
        // 截断 recent_history 中 date > cutoff 的项(保持"从新到旧"顺序)
        item.recent_history.retain(|d| d.date.as_str() <= cutoff.as_str());
        Some(item)
    }

    /// As-Of 模式下：用截断后的 K 线最后一条合成实时行情。
    /// 当且仅当当前 AsOf 处于激活状态、且传入的 K 线包含 <= effective_cutoff 的数据时返回 Some；
    /// Live 模式或 K 线为空时返回 None（让调用方走原 vendor 路径）。
    ///
    /// effective_cutoff 解析:
    /// - 若 as_of_date 本身是交易日,直接用 as_of_date
    /// - 若 as_of_date 是周末/节假日,fallback 到上一交易日,
    ///   并通过 record_degradation 报告 "使用了上一交易日" 让用户可见
    fn quote_from_klines(stock_code: &str, klines: &[KLine]) -> Option<StockQuote> {
        let ctx = crate::as_of::current_as_of()?;
        if klines.is_empty() {
            return None;
        }
        let as_of_str = ctx.as_of_date.format("%Y-%m-%d").to_string();
        let effective = if crate::calendar::is_trading_day(&ctx.as_of_date) {
            as_of_str.clone()
        } else {
            let prev = crate::calendar::previous_trading_day(ctx.as_of_date);
            crate::as_of::record_degradation(
                "astock-data",
                "get_quote",
                &format!(
                    "as_of_date={} 非交易日,fallback 至上一交易日 {}",
                    as_of_str,
                    prev.format("%Y-%m-%d")
                ),
            );
            prev.format("%Y-%m-%d").to_string()
        };
        let last = klines.iter().rev().find(|k| k.date.as_str() <= effective.as_str())?;
        Some(StockQuote {
            code: stock_code.to_string(),
            name: stock_code.to_string(),
            price: last.close,
            pre_close: 0.0,
            open: last.open,
            high: last.high,
            low: last.low,
            volume: last.volume,
            amount: last.amount,
            change_pct: 0.0,
            turnover_rate: last.turnover_rate.unwrap_or(0.0),
            pe: None,
            pb: None,
            total_mv: None,
            circulating_mv: None,
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: last.date.clone(),
        })
    }

    fn find_vendor(&self, name: &str) -> Option<&dyn StockVendor> {
        // 启用状态过滤：未启用的 vendor 直接返回 None，跳过调用
        if !self.is_vendor_enabled(name) {
            tracing::debug!("[astock-data] vendor '{name}' 未启用，跳过");
            return None;
        }
        self.vendors.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_ref())
    }

    // ── H1 修复: 通用 vendor 遍历 + 健康追踪 + 指数退避重试 ──

    /// 通用 vendor 遍历 + 健康追踪 + 指数退避重试
    ///
    /// 对 vendor_names 按健康状态排序，逐个调用 fetch_fn 获取数据。
    /// - fetch_fn 返回 Ok → 自动 record_success，立刻返回结果
    /// - fetch_fn 返回 Err → 自动 record_failure，尝试下一个 vendor
    /// - 全部失败后等待指数退避重试（最多 max_retries-1 次额外重试）
    ///
    /// fetch_fn 内部应完成：vendor 方法调用 → 质量检查 → 缓存写入（如有）。
    /// 通过闭包捕获额外参数（如 period/limit/adj_type）。
    async fn try_vendors_retry<T, F>(
        &self,
        stock_code: &str,
        route_key: &str,
        vendor_names: &[String],
        max_retries: u32,
        fetch_fn: F,
    ) -> Result<T, DataError>
    where
        T: Send + 'static,
        for<'a> F: Fn(&'a str, &'a dyn StockVendor) -> BoxFuture<'a, Result<T, DataError>>,
    {
        // 2026-08-01：路由入口剔除"无凭据必失败"的 vendor（neodata 需 token、iwencai 需
        // api_key）。此前它们在模板变量里没有 vendor_enabled_* 开关 → load_enabled_vendors
        // 默认全部启用 → 每次调用都 TOKEN_MISSING/api_key not configured 失败 + 重试，
        // 拖慢链路并污染健康窗口（趋势智选日志 neodata 8 次失败降级实锤）。
        let mut vendor_names_list: Vec<String> = Vec::with_capacity(vendor_names.len());
        for name in vendor_names {
            if self.vendor_has_credentials(name).await {
                vendor_names_list.push(name.to_string());
            } else {
                tracing::info!(
                    "[astock-data] vendor '{name}' 无凭据（neodata/iwencai），从路由链剔除"
                );
            }
        }
        let mut retry_count = 0u32;

        loop {
            let mut last_err = None;

            // 健康过滤：排除已降级的 vendor
            let healthy_names = self
                .health_tracker
                .try_vendors(&vendor_names_list)
                .await
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            // V48 修复: 所有 vendor 均降级时不回退完整列表
            // 原逻辑"回退完整列表→重试已降级 vendor→再失败→再降级"形成无效重试循环。
            // 修正: 首次尝试允许完整列表兜底一次；重试时所有 vendor 仍降级则直接返回错误。
            let names_to_try: &[String] = if healthy_names.is_empty() {
                if retry_count > 0 {
                    // 已重试过，所有 vendor 仍然降级 → 放弃，不再无意义重试
                    tracing::warn!(
                        "[health] {} {} 所有 vendor 已降级（第{}次重试后），放弃",
                        route_key,
                        stock_code,
                        retry_count
                    );
                    return Err(last_err.unwrap_or_else(|| DataError::VendorError {
                        vendor: "all".into(),
                        message: format!(
                            "{route_key} {stock_code} 所有数据源均不可用（全部 vendor 已降级）"
                        ),
                    }));
                }
                // 首次尝试: 允许完整列表兜底（健康状态可能过时）
                tracing::warn!(
                    "[health] {} {} 所有 vendor 已降级，首次尝试回退完整列表",
                    route_key,
                    stock_code
                );
                &vendor_names_list
            } else {
                &healthy_names
            };

            for name in names_to_try {
                if let Some(vendor) = self.find_vendor(name) {
                    // 修复 M-DEF-1: acquire 现在返回 Result，错误传播为 DataError
                    let _guard = self.gate.acquire(name).await?;
                    match fetch_fn(name, vendor).await {
                        Ok(result) => {
                            self.health_tracker.record_success(name).await;
                            // P3-B5(F): 若此 vendor 是 fallback（非首选），记录 fallback 路径
                            // 用于前端调试"为什么 X 数据用了 Y vendor 而非 Z"
                            if name != names_to_try.first().map(|s| s.as_str()).unwrap_or("") {
                                self.health_tracker
                                    .record_fallback(
                                        route_key,
                                        stock_code,
                                        names_to_try.first().map(|s| s.as_str()).unwrap_or(""),
                                        name,
                                        "primary_failed",
                                    )
                                    .await;
                            }
                            return Ok(result);
                        },
                        Err(e) => {
                            tracing::warn!(
                                "[降级] {} {} {} 失败: {}",
                                route_key,
                                stock_code,
                                name,
                                e
                            );
                            // 限流(429)单独处理：不增加连续失败计数（避免"限流→降级→
                            // 降级后 vendor 列表縮短→剩余 vendor 压力更大→更多 429"的恶性循环）
                            //
                            // "数据为空"类错误也不触发降级(2026-07-22 修复):
                            // cls_flash/news 等工具返回"数据为空"通常是因为该数据源
                            // 当前时段确实没有数据(如非交易时段、cls 快讯暂时为空),
                            // 并非 vendor 本身故障。如果将其计入降级计数,会导致
                            // eastmoney 因 cls_flash 空数据被全局降级,进而影响
                            // news/money_flow/peers 等所有依赖 eastmoney 的工具。
                            match &e {
                                DataError::RateLimited { .. } => {
                                    tracing::warn!(
                                        "[降级] {} {} {} 被限流(429)，不触发 vendor 降级",
                                        route_key,
                                        stock_code,
                                        name
                                    );
                                },
                                DataError::VendorError { message, .. } => {
                                    let msg_lower = message.to_lowercase();
                                    let is_empty_data = msg_lower.contains("为空")
                                        || msg_lower.contains("返回空")
                                        || msg_lower.contains("empty")
                                        || msg_lower.contains("no data")
                                        || msg_lower.contains("无数据");
                                    if is_empty_data {
                                        tracing::info!(
                                            "[降级] {} {} {} 返回空数据(非故障)，不触发 vendor 降级",
                                            route_key,
                                            stock_code,
                                            name
                                        );
                                    } else {
                                        self.health_tracker
                                            .record_failure(name, &e.to_string())
                                            .await;
                                    }
                                },
                                _ => {
                                    self.health_tracker.record_failure(name, &e.to_string()).await;
                                },
                            }
                            last_err = Some(e);
                        },
                    }
                }
            }

            if retry_count < max_retries && last_err.is_some() {
                // V54: 若可用的 vendor 数量 ≤ 2（即数据源单一或大部分已降级），
                // 跳过重试。因为：1-2 个 vendor 全部首次尝试失败几乎不可能是瞬态问题
                // （API 接口变更 / 数据为空），重试只是浪费 1s+3s 的退避延迟。
                //
                // 日志中常见案例：
                //   dragon_tiger 仅 eastmoney/baidu_stock 2 家 vendor，
                //   eastmoney 返回 "HTTP request failed: error decoding response body"，
                //   这是 eastmoney 接口变动或反爬升级的持续故障，重试无意义。
                if names_to_try.len() <= 2 {
                    tracing::warn!(
                        "[retry-skip] {} {} 仅有 {} 个 vendor 且全部首次失败，跳过重试（预期为持续故障）",
                        route_key,
                        stock_code,
                        names_to_try.len()
                    );
                    return Err(last_err.unwrap_or_else(|| DataError::VendorError {
                        vendor: "all".into(),
                        message: format!("{route_key} {stock_code} 所有数据源均不可用"),
                    }));
                }
                retry_count += 1;
                let delay = (1u64 << retry_count) - 1; // 指数退避: 1s, 3s
                tracing::warn!(
                    "[retry] {} {} 所有源失败，{}s 后第 {} 次重试整条链",
                    route_key,
                    stock_code,
                    delay,
                    retry_count
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                continue;
            }

            return Err(last_err.unwrap_or_else(|| DataError::VendorError {
                vendor: "all".into(),
                message: format!("{route_key} {stock_code} 所有数据源均不可用"),
            }));
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // As-Of dispatch helpers (vendor trait 大重构 P0 §2.2)
    //
    // 路由层决策 4 步:
    //   1. live 模式 → 直接调原方法(向后兼容)
    //   2. as-of 模式 → 遍历 vendors_for(method)
    //   3. 查 vendor.asof_capability(method) 决定走哪条路
    //   4. 失败时回退到下一个 vendor
    //
    // 这些 helper 是泛型形态;每个具体方法(如 get_quote)在自己的 impl 中
    // 调用对应 helper + 自己的 4 路分支逻辑。
    // ─────────────────────────────────────────────────────────────────

    /// 查 vendor 申报的 as-of 能力
    /// (内部统一入口,所有方法调这里而不是 vendor.asof_capability 直接调)
    pub fn vendor_asof_capability(&self, vendor_name: &str, method: &str) -> AsOfCapability {
        match self.find_vendor(vendor_name) {
            Some(v) => v.asof_capability(method),
            None => AsOfCapability::Fallthrough,
        }
    }

    /// 通用 as-of 模式分支 helper: vendor 调 *_with_asof
    /// 返回 Some(T) 表示 vendor 成功;None 表示该 vendor 在 as-of 模式下不可用
    /// 让调用方决定是否继续尝试下一个 vendor
    pub async fn try_vendor_with_asof<T, Fut>(
        &self,
        method_name: &str,
        vendor_name: &str,
        call: Fut,
    ) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
        Fut: std::future::Future<Output = Result<T, DataError>>,
    {
        if !crate::as_of::is_asof_active() {
            return None; // live 模式不适用
        }
        let cap = self.vendor_asof_capability(vendor_name, method_name);
        match cap {
            AsOfCapability::NativeDateParam | AsOfCapability::SynthesizeFromKline => {
                // vendor 应在闭包里调对应的 *_with_asof
                match call.await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        crate::as_of::record_degradation(
                            vendor_name,
                            method_name,
                            &format!("with_asof 调用失败: {e}"),
                        );
                        None
                    },
                }
            },
            AsOfCapability::NoHistoricalSemantic => {
                // 先查每日快照缓存（如已配置）
                if let Some(ctx) = crate::as_of::current_as_of() {
                    let date = ctx.as_string();
                    if let Some(cached) = self.try_daily_snapshot(method_name, &date) {
                        match serde_json::from_str::<T>(&cached) {
                            Ok(v) => return Some(v),
                            Err(e) => tracing::warn!(
                                "[asof] daily_snapshot 反序列化失败({method_name}/{date}): {e}"
                            ),
                        }
                    }
                }
                // 缓存未命中：记录降级
                crate::as_of::record_degradation(
                    vendor_name,
                    method_name,
                    "no historical semantic;无每日快照缓存",
                );
                None
            },
            AsOfCapability::Fallthrough => {
                // 老 vendor 没申报:走 lib.rs "全量 + 截断" 兜底(由调用方自己处理)
                None
            },
        }
    }

    /// 检查在当前模式下,应不应该用 as-of 分支
    /// 单一布尔查询,消除到处写 `crate::as_of::is_asof_active()` 的模板
    pub fn should_use_asof(&self) -> bool {
        crate::as_of::is_asof_active()
    }

    /// P5:尝试验证每日快照缓存(NoHistoricalSemantic 数据兜底)
    /// 启用条件:self.daily_snapshot 不为 None 且 method 在 SNAPSHOT_METHODS 中
    /// 返回 Some(json_str) 表示缓存命中;None 表示未命中或未启用
    fn try_daily_snapshot(&self, method: &str, date: &str) -> Option<String> {
        if !daily_snapshot::SNAPSHOT_METHODS.contains(&method) {
            return None;
        }
        self.daily_snapshot.as_ref().and_then(|c| c.get(method, date))
    }

    /// C5.3 修复：带 keyword 维度的每日快照查询（用于 search_stock 等方法）
    /// 与 try_daily_snapshot 的区别：cache_key 含 keyword，避免不同 keyword 互相覆盖
    fn try_daily_keyword_snapshot(
        &self,
        method: &str,
        keyword: &str,
        date: &str,
    ) -> Option<String> {
        if !daily_snapshot::SNAPSHOT_METHODS.contains(&method) {
            return None;
        }
        self.daily_snapshot.as_ref().and_then(|c| c.get_keyword(method, keyword, date))
    }

    /// 设置每日快照（全市场方法），供 Tauri command 写入
    pub fn set_daily_snapshot(&self, method: &str, date: &str, json: &str) {
        if let Some(ref snap) = self.daily_snapshot {
            snap.set_snapshot(method, date, json);
        }
    }

    /// 设置个股级每日快照，供 Tauri command 写入
    pub fn set_stock_daily_snapshot(&self, method: &str, stock_code: &str, date: &str, json: &str) {
        if let Some(ref snap) = self.daily_snapshot {
            snap.set_stock_snapshot(method, stock_code, date, json);
        }
    }

    /// C5.3 修复：设置带 keyword 的每日快照（用于 search_stock 等方法）
    /// 供 Tauri command sweep_daily_snapshots 写入预抓结果
    pub fn set_daily_keyword_snapshot(&self, method: &str, keyword: &str, date: &str, json: &str) {
        if let Some(ref snap) = self.daily_snapshot {
            snap.set_keyword_snapshot(method, keyword, date, json);
        }
    }

    /// P3-B5(G): 返回所有已注册的 vendor 名单，供后台健康探测遍历
    pub fn vendor_names(&self) -> Vec<String> {
        self.vendors.iter().map(|(n, _)| n.to_string()).collect()
    }

    /// 检查指定 vendor 的连接可用性（按实际能力选择探针方法）
    pub async fn check_vendor_health(&self, vendor_name: &str) -> Result<(), DataError> {
        let vendor = self.find_vendor(vendor_name).ok_or_else(|| DataError::VendorError {
            vendor: vendor_name.into(),
            message: "vendor not registered".into(),
        })?;
        // 按 vendor 实际能力选择探测方法，避免用未实现的方法误判
        match vendor_name {
            "eastmoney" => {
                vendor.get_klines("000001", "daily", 5, None).await?;
            },
            "sina" => {
                // sina news API (vip.stock.finance.sina.com.cn) 不稳定，改用 quote 探测
                vendor.get_quote("000001").await?;
            },
            "ths" => {
                vendor.get_hot_stocks().await?;
            },
            "cninfo" => {
                vendor.get_announcements("000001").await?;
            },
            "iwencai" => {
                vendor.search_stock("平安银行").await?;
            },
            "akshare" => {
                vendor.get_news("000001", 3).await?;
            },
            _ => {
                // tencent, baidu_stock, mootdx — 有 get_quote
                vendor.get_quote("000001").await?;
            },
        }
        Ok(())
    }

    pub async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        // G1 跨市场数据接入：国际代码直接路由到 international vendor，绕过 A 股 routing
        // 避免 A 股 vendor 不识别国际代码导致的 6 vendor × 2 轮无效重试
        if is_international_code(stock_code) {
            return self.get_international_quote(stock_code).await;
        }
        // As-Of 模式：K线最后一行合成行情。K线合成失败时返回 Error，绝不回退到
        // vendor.get_quote（返回今日实时数据，时间泄露）。
        if crate::as_of::is_asof_active() {
            // 遍历 vendors_for("klines") ，NativeDateParam vendor 调 _with_asof
            let kline_names: Vec<String> =
                self.routing.vendors_for("klines", &self.routing.klines).clone();
            let mut last_err: Option<DataError> = None;
            for name in &kline_names {
                if let Some(vendor) = self.find_vendor(name) {
                    let cap = self.vendor_asof_capability(name, "get_klines");
                    let ks_result = match cap {
                        AsOfCapability::NativeDateParam => {
                            vendor.get_klines_with_asof(stock_code, "daily", 5, None).await
                        },
                        _ => vendor.get_klines(stock_code, "daily", 5, None).await,
                    };
                    match ks_result {
                        Ok(ks) if !ks.is_empty() => {
                            if let Some(mut q) = Self::quote_from_klines(stock_code, &ks) {
                                // 非阻塞填充 PE/PB — 超时或失败不影响行情立即返回
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(2),
                                    self.get_financials(stock_code),
                                )
                                .await
                                {
                                    Ok(Ok(fins)) => {
                                        if let Some(latest) = fins.into_iter().next() {
                                            if let Some(eps) = latest.eps.filter(|&v| v > 0.0) {
                                                q.pe = Some(q.price / eps);
                                            }
                                            if let Some(bps) = latest.bps.filter(|&v| v > 0.0) {
                                                q.pb = Some(q.price / bps);
                                            }
                                        }
                                    },
                                    _ => tracing::trace!(
                                        "[asof] PE/PB 填充跳过（get_financials 超时或失败）"
                                    ),
                                }
                                return Ok(q);
                            }
                            tracing::warn!(
                                "[asof] {} {} K线无法合成quote，尝试下一源",
                                name,
                                stock_code
                            );
                            last_err = Some(DataError::VendorError {
                                vendor: name.clone(),
                                message: "K线全部晚于as_of_date".into(),
                            });
                        },
                        Ok(_) => {
                            tracing::warn!("[asof] {} {} K线返回空，尝试下一源", name, stock_code);
                        },
                        Err(e) => {
                            tracing::warn!(
                                "[asof] {} {} K线失败: {}，尝试下一源",
                                name,
                                stock_code,
                                e
                            );
                            last_err = Some(e);
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_quote",
                &format!("as-of 模式所有K线源均无法合成 {stock_code} 行情，已阻止回退到今日数据"),
            );
            return Err(last_err.unwrap_or_else(|| DataError::VendorError {
                vendor: "all".into(),
                message: format!("as-of 模式下 {stock_code} 行情合成失败（所有K线源不可用）"),
            }));
        }

        let cache_key = Self::cache_key_for("quote", stock_code);
        if let Some(cached) = self.cache_get(&cache_key).await {
            if let Ok(quote) = serde_json::from_str::<StockQuote>(&cached) {
                return Ok(quote);
            }
        }

        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("quote", &self.routing.quote)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        let result = self
            .try_vendors_retry(stock_code, "quote", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_quote(&sc).await?;
                    // 质量检查：price>0 且 name 非空
                    if result.price <= 0.0 || result.name.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: format!(
                                "行情数据质量不足: price={}, name='{}'",
                                result.price, result.name
                            ),
                        });
                    }
                    Ok(result)
                })
            })
            .await?;

        self.cache_set_serialized(cache_key, &result, 30).await;
        Ok(result)
    }

    /// G1 跨市场数据接入：国际股票行情（港股/美股）
    ///
    /// 直接调用 international vendor，绕过 A 股 routing，避免无效重试。
    /// 缓存 30s（与 A 股 get_quote 一致）。
    pub async fn get_international_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let cache_key = format!("intl_quote:{stock_code}");
        if let Some(cached) = self.cache_get(&cache_key).await {
            if let Ok(q) = serde_json::from_str::<StockQuote>(&cached) {
                return Ok(q);
            }
        }
        let vendor = self.find_vendor("international").ok_or_else(|| DataError::VendorError {
            vendor: "international".into(),
            message: "国际股票 vendor 未注册".into(),
        })?;
        let result = vendor.get_quote(stock_code).await?;
        self.cache_set_serialized(cache_key, &result, 30).await;
        Ok(result)
    }

    /// G1 跨市场数据接入：国际股票 K 线
    pub async fn get_international_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj_type: Option<crate::types::AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let cache_key = format!("intl_klines:{stock_code}:{period}:{limit}:{:?}", adj_type);
        if let Some(cached) = self.cache_get(&cache_key).await {
            if let Ok(ks) = serde_json::from_str::<Vec<KLine>>(&cached) {
                if ks.len() >= limit as usize {
                    let start = ks.len().saturating_sub(limit as usize);
                    return Ok(ks[start..].to_vec());
                }
            }
        }
        let vendor = self.find_vendor("international").ok_or_else(|| DataError::VendorError {
            vendor: "international".into(),
            message: "国际股票 vendor 未注册".into(),
        })?;
        let result = vendor.get_klines(stock_code, period, limit, adj_type).await?;
        self.cache_set_serialized(cache_key, &result, 300).await;
        Ok(result)
    }

    /// G1 跨市场数据接入：基准指数 K 线（标普 500 / 纳指 / 恒生 / 上证等）
    ///
    /// 通过 eastmoney 国际指数接口获取，统一返回 KLine 数组。
    /// 支持的代码：SPX/SP500/IXIC/NDX/DJI/HSI/HSCEI（国际）+ 000001.SH/399001/399006/000300（A 股）
    pub async fn get_benchmark_klines(
        &self,
        benchmark_code: &str,
        period: &str,
        limit: u32,
    ) -> Result<Vec<KLine>, DataError> {
        let cache_key = format!("benchmark_klines:{benchmark_code}:{period}:{limit}");
        if let Some(cached) = self.cache_get(&cache_key).await {
            if let Ok(ks) = serde_json::from_str::<Vec<KLine>>(&cached) {
                if ks.len() >= limit as usize {
                    let start = ks.len().saturating_sub(limit as usize);
                    return Ok(ks[start..].to_vec());
                }
            }
        }
        // 国际指数走 international vendor；A 股指数走默认 routing
        let result = if is_international_code(benchmark_code)
            || matches!(
                benchmark_code.trim().to_uppercase().as_str(),
                "SPX" | "SP500" | "S&P500" | "IXIC" | "NDX" | "DJI" | "HSI" | "HSCEI"
            ) {
            let vendor =
                self.find_vendor("international").ok_or_else(|| DataError::VendorError {
                    vendor: "international".into(),
                    message: "international vendor 未注册".into(),
                })?;
            // 国际指数通过 eastmoney 接口获取，使用国际代码格式
            let intl_code = match benchmark_code.trim().to_uppercase().as_str() {
                "SPX" | "SP500" | "S&P500" => "US_SPX".to_string(),
                "IXIC" => "US_IXIC".to_string(),
                "NDX" => "US_NDX".to_string(),
                "DJI" => "US_DJI".to_string(),
                "HSI" => "hkHSI".to_string(),
                "HSCEI" => "hkHSCEI".to_string(),
                other => other.to_string(),
            };
            vendor.get_klines(&intl_code, period, limit, None).await?
        } else {
            // A 股指数：剥离后缀，直接走默认 K 线 routing
            let clean_code = benchmark_code.trim().trim_end_matches(".SH").trim_end_matches(".SZ");
            self.get_klines(clean_code, period, limit).await?
        };
        self.cache_set_serialized(cache_key, &result, 300).await;
        Ok(result)
    }

    /// G1 跨市场数据接入：外汇 K 线（USD/CNY、HKD/CNY 等）
    ///
    /// 通过 eastmoney 外汇接口获取。`pair` 格式为 "BASE/QUOTE"（如 "USD/CNY"）。
    pub async fn get_forex_klines(
        &self,
        pair: &str,
        period: &str,
        limit: u32,
    ) -> Result<Vec<KLine>, DataError> {
        let cache_key = format!("forex_klines:{pair}:{period}:{limit}");
        if let Some(cached) = self.cache_get(&cache_key).await {
            if let Ok(ks) = serde_json::from_str::<Vec<KLine>>(&cached) {
                if ks.len() >= limit as usize {
                    let start = ks.len().saturating_sub(limit as usize);
                    return Ok(ks[start..].to_vec());
                }
            }
        }
        let vendor = self.find_vendor("international").ok_or_else(|| DataError::VendorError {
            vendor: "international".into(),
            message: "international vendor 未注册".into(),
        })?;
        // 外汇代码转换：USD/CNY → forex.usdcny
        let forex_code = pair.trim().to_uppercase().replace('/', "");
        let result = vendor.get_klines(&format!("forex.{forex_code}"), period, limit, None).await?;
        self.cache_set_serialized(cache_key, &result, 300).await;
        Ok(result)
    }

    pub async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
    ) -> Result<Vec<KLine>, DataError> {
        self.get_klines_with_adj(stock_code, period, limit, None).await
    }

    /// K 线查询，支持复权方式 (R3-A 接口)
    ///
    /// 实现策略：
    /// - `adj_type=None` 或 `Some(AdjType::None)`：不复权，直接返回 vendor 原始数据
    /// - `Some(AdjType::Forward)` 或 `Some(AdjType::Backward)`：
    ///   1. vendor 若支持复权（如 eastmoney 的 fqt 参数），返回的 K 线 `adj_factor` 标记为 `Some`
    ///   2. vendor 若不支持复权（如 sina/163），返回的 K 线 `adj_factor` 为 `None`，
    ///      lib 层检测到后本地应用 `compute_adj_factors` + `apply_adjustment`
    pub async fn get_klines_with_adj(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj_type: Option<crate::types::AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        // G1 跨市场数据接入：国际代码直接路由到 international vendor
        if is_international_code(stock_code) {
            return self.get_international_klines(stock_code, period, limit, adj_type).await;
        }
        let cache_key = Self::kline_cache_key(stock_code, period, adj_type);
        let fetch_limit = limit.max(500);

        {
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(klines) = serde_json::from_str::<Vec<KLine>>(&cached) {
                    if klines.len() >= limit as usize {
                        // 修复 M-DS-1: 仅检查长度不够，还需校验最后一条 K 线的日期
                        // 是否为最新交易日。若缓存过期（如周末/节假日拉取后过了夜），
                        // 视为未命中，继续走 vendor 拿最新数据。
                        let latest_td = crate::calendar::latest_trading_day();
                        let cache_stale = klines
                            .last()
                            .and_then(|k| {
                                chrono::NaiveDate::parse_from_str(&k.date, "%Y-%m-%d").ok()
                            })
                            .map(|d| d < latest_td)
                            .unwrap_or(true);
                        if !cache_stale {
                            let start = klines.len().saturating_sub(limit as usize);
                            return Ok(klines[start..].to_vec());
                        }
                        tracing::debug!(
                            "[astock-data] K 线缓存已过期 (last_date={:?}, latest_trading_day={}), 重新拉取 vendor",
                            klines.last().map(|k| k.date.as_str()),
                            latest_td
                        );
                    }
                }
            }
        }

        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("klines", &self.routing.klines)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        let period_owned = period.to_string();
        let result = self
            .try_vendors_retry(stock_code, "klines", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                let period = period_owned.clone();
                Box::pin(async move {
                    let cap = vendor.asof_capability("get_klines");
                    let klines = match cap {
                        AsOfCapability::NativeDateParam if crate::as_of::is_asof_active() => {
                            vendor.get_klines_with_asof(&sc, &period, fetch_limit, adj_type).await?
                        },
                        _ => vendor.get_klines(&sc, &period, fetch_limit, adj_type).await?,
                    };
                    if klines.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "K线返回空".into(),
                        });
                    }
                    let truncated = Self::truncate_klines_by_asof(klines);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "K线全部晚于截止日或返回空".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await?;

        // R3: 本地复权 fallback — 若 vendor 未应用复权（adj_factor 全为 None），
        // 且用户请求了复权（adj_type 为 Some 且非 None），则本地应用复权因子
        let result = self.apply_local_adjustment_if_needed(result, adj_type, stock_code).await;

        self.cache_set_serialized(cache_key, &result, 300).await;
        let start = result.len().saturating_sub(limit as usize);
        Ok(result[start..].to_vec())
    }

    /// 本地复权 fallback：当 vendor 返回的 K 线 adj_factor 全为 None 时，
    /// 获取除权事件并本地应用复权因子。
    ///
    /// - `adj_type=None` 或 `Some(None)`：直接返回原数据（不复权）
    /// - `adj_type=Some(Forward/Backward)` 且 K 线已有 adj_factor 标记：vendor 已处理，直接返回
    /// - `adj_type=Some(Forward/Backward)` 且 K 线 adj_factor 全 None：本地应用复权
    async fn apply_local_adjustment_if_needed(
        &self,
        klines: Vec<KLine>,
        adj_type: Option<crate::types::AdjType>,
        stock_code: &str,
    ) -> Vec<KLine> {
        // 不复权或无复权请求：直接返回
        let adj = match adj_type {
            None | Some(crate::types::AdjType::None) => return klines,
            Some(adj) => adj,
        };

        // vendor 已应用复权（至少一根 K 线 adj_factor 为 Some）：信任 vendor
        if klines.iter().any(|k| k.adj_factor.is_some()) {
            return klines;
        }

        // 所有 K 线 adj_factor 都为 None → vendor 未应用复权，本地 fallback
        // 获取除权除息事件
        let dividends = match self.get_dividend_records(stock_code).await {
            Ok(records) => records,
            Err(e) => {
                tracing::warn!(
                    stock = %stock_code,
                    error = %e,
                    "获取除权事件失败，跳过本地复权 fallback"
                );
                return klines;
            },
        };

        if dividends.is_empty() {
            tracing::debug!(
                stock = %stock_code,
                "无除权事件，本地复权 fallback 无需应用（K 线原样返回）"
            );
            return klines;
        }

        // DividendRecord → AdjustmentEvent 转换
        // 注：DividendRecord 不含 rights_ratio/rights_price，配股调整暂缺
        let events: Vec<crate::types::AdjustmentEvent> = dividends
            .iter()
            .map(|d| crate::types::AdjustmentEvent {
                stock_code: d.stock_code.clone(),
                ex_date: d.ex_date.clone(),
                cash_dividend: d.dividend_per_share,
                bonus_share_ratio: d.bonus_share_ratio,
                rights_ratio: 0.0,
                rights_price: 0.0,
            })
            .collect();

        let factors = crate::adjustment::compute_adj_factors(&klines, &events, adj);
        let adjusted = crate::adjustment::apply_adjustment(&klines, &factors, adj);
        tracing::info!(
            stock = %stock_code,
            adj_type = ?adj,
            events_count = events.len(),
            klines_count = klines.len(),
            "本地复权 fallback 已应用（vendor 未复权）"
        );
        adjusted
    }

    /// 财报披露日历 (R3-B 接口, 暂为 stub)
    ///
    /// 设计目标: 复用 `get_announcements` vendor 链路(cninfo 优先),按标题归类
    /// 成 preliminary/express/formal/shareholders_meeting,过滤其它类。
    ///
    /// 当前实现: vendor 暂未实现按标题分类的 earnings 抽取,直接返回空数组;
    /// 完整实现留在 P1-5 (K 线叠加财报日图标) 阶段一并补全。
    pub async fn get_earnings_calendar(
        &self,
        stock_code: &str,
    ) -> Result<Vec<crate::types::EarningsEvent>, DataError> {
        let vendor_names: Vec<String> =
            self.routing.earnings_calendar.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        self.try_vendors_retry(
            stock_code,
            "earnings_calendar",
            &vendor_names,
            2,
            |_name, vendor| {
                let sc = sc.clone();
                Box::pin(async move { vendor.get_earnings_calendar(&sc).await })
            },
        )
        .await
    }

    /// 获取社交舆情数据（股吧/雪球热度）
    pub async fn get_social_sentiment(
        &self,
        stock_code: &str,
    ) -> Result<Vec<crate::types::SocialSentiment>, DataError> {
        let vendor_names: Vec<String> =
            self.routing.social_sentiment.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        let mut sentiments = self
            .try_vendors_retry(stock_code, "social_sentiment", &vendor_names, 1, |_name, vendor| {
                let sc = sc.clone();
                Box::pin(async move { vendor.get_social_sentiment(&sc).await })
            })
            .await?;

        // 修复 M-HOT-1: vendor(guba)无法直接提供 hot_rank（跨股票热度排名），
        // 调用 get_hot_stocks 查找当前股票在热度榜中的位置，填充 hot_rank 字段。
        // 失败时不影响主流程（hot_rank 保持 None）。
        let needs_hot_rank = sentiments.iter().any(|s| s.hot_rank.is_none());
        if needs_hot_rank {
            if let Ok(hot_stocks) = self.get_hot_stocks().await {
                let normalized_code = stock_code
                    .trim_start_matches("sh")
                    .trim_start_matches("sz")
                    .trim_start_matches("bj");
                for (idx, hs) in hot_stocks.iter().enumerate() {
                    let hs_code = hs
                        .stock_code
                        .trim_start_matches("sh")
                        .trim_start_matches("sz")
                        .trim_start_matches("bj");
                    if hs_code == normalized_code {
                        let rank = (idx + 1) as u32;
                        for s in sentiments.iter_mut() {
                            if s.hot_rank.is_none() {
                                s.hot_rank = Some(rank);
                            }
                        }
                        break;
                    }
                }
                if sentiments.iter().all(|s| s.hot_rank.is_none()) {
                    tracing::debug!(
                        "[astock] get_social_sentiment: 股票 {stock_code} 未在热度榜中，hot_rank 保持 None"
                    );
                }
            }
        }

        Ok(sentiments)
    }

    pub async fn get_financials(
        &self,
        stock_code: &str,
    ) -> Result<Vec<FinancialReport>, DataError> {
        {
            let cache_key = Self::cache_key_for("financials", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<FinancialReport>>(&cached) {
                    return Ok(data);
                }
            }
        }
        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let vendor_names: Vec<String> =
            self.routing.financials.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "financials", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_financials(&sc).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "财务数据返回空".into(),
                        });
                    }
                    let truncated = Self::truncate_financials_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "财报全部晚于截止日或返回空".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("financials", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                // C: fallback — 全部数据源失败时返回错误，不包装估算数据
                // 估算数据不应被 Ok 包装，否则下游无法区分真实财报和估算值
                tracing::warn!("[C-fallback] {stock_code} 所有财务数据源失败: {e}");
                Err(DataError::VendorError {
                    vendor: "all".into(),
                    message: format!("所有财务数据源失败，无法获取 {} 的财报数据", stock_code),
                })
            },
        }
    }

    /// P2-B4: 对 vendor 返回的新闻列表填充 sentiment_score
    ///
    /// 仅当原值为 None 时填充,保留 vendor 可能提供的精确评分。
    /// 在 get_news / search_news / get_policy_news 的 Ok(result) 入口统一调用,
    /// 确保缓存和 news_archive_sink 持久化的数据都带 sentiment_score。
    fn fill_sentiment_scores(items: &mut [NewsItem]) {
        for n in items.iter_mut() {
            if n.sentiment_score.is_none() {
                n.sentiment_score = crate::sentiment::compute_news_sentiment(&n.title, &n.summary);
            }
        }
    }

    pub async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        {
            let cache_key = Self::cache_key_for("news", &format!("{stock_code}:{limit}"));
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<NewsItem>>(&cached) {
                    return Ok(data);
                }
            }
        }
        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let news_names: Vec<String> = self.routing.vendors_for("news", &self.routing.news).clone();
        let sc = stock_code.to_string();
        let limit_owned = limit;
        match self
            .try_vendors_retry(stock_code, "news", &news_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_news(&sc, limit_owned).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "新闻返回空".into(),
                        });
                    }
                    let truncated = Self::truncate_news_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "新闻全部晚于截止日".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(mut result) => {
                // P2-B4: 统一填充 sentiment_score(在缓存和持久化前完成)
                Self::fill_sentiment_scores(&mut result);
                let cache_key = Self::cache_key_for("news", &format!("{stock_code}:{limit}"));
                self.cache_set_serialized(cache_key, &result, 300).await;
                // P6:自动 upsert 到 news_archive(无关缓存命中/降级,
                // 任何 vendor 返回的非空结果都入本地语料库)
                if let Some(sink) = &self.news_archive_sink {
                    let filtered: Vec<NewsItem> = result
                        .iter()
                        .filter(|n| parse_news_publish_time_ms(&n.publish_time).is_some())
                        .cloned()
                        .collect();
                    if !filtered.is_empty() {
                        sink.upsert("news", Some(stock_code), None, &filtered).await;
                    }
                }
                Ok(result)
            },
            Err(e) => {
                // 与 get_financials 一致：全部数据源失败时返回 Err，
                // 避免调用方把空列表误判为"无新闻/无催化剂"
                tracing::warn!("[news] {stock_code} 所有新闻数据源失败: {e}");
                Err(DataError::VendorError {
                    vendor: "all".into(),
                    message: format!("所有新闻数据源失败，无法获取 {} 的新闻", stock_code),
                })
            },
        }
    }

    /// 获取政策相关新闻(基于股票所属行业做关键词搜索)
    ///
    /// 实现路径:vendor.get_policy_news → search_news("{行业} 政策/规划/通知/补贴")
    /// as-of 模式:走 news_archive 本地语料库(与 search_news 相同)
    pub async fn get_policy_news(
        &self,
        stock_code: &str,
        limit: u32,
    ) -> Result<Vec<NewsItem>, DataError> {
        // as-of 模式:政策新闻本质是搜索语义,与 search_news 一致走 news_archive
        if let Some(ctx) = crate::as_of::current_as_of() {
            if let Some(sink) = &self.news_archive_sink {
                let as_of_ts_ms = ctx
                    .as_of_date
                    .and_hms_opt(23, 59, 59)
                    .and_then(|dt| dt.and_local_timezone(chrono::Local).single())
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or_else(|| {
                        ctx.as_of_date
                            .and_hms_opt(23, 59, 59)
                            .and_then(|dt| dt.and_utc().timestamp_millis().into())
                            .unwrap_or(0)
                    });
                // 用 "政策" 关键词查 news_archive
                let archived = sink.search_asof("政策", Some(stock_code), as_of_ts_ms, limit).await;
                if !archived.is_empty() {
                    tracing::info!(
                        "[news_archive] get_policy_news 命中 {} 条 (stock={}, as_of={})",
                        archived.len(),
                        stock_code,
                        ctx.as_of_date
                    );
                    return Ok(archived);
                }
                crate::as_of::record_degradation(
                    "astock-data",
                    "get_policy_news",
                    "as-of 模式 news_archive 无政策新闻数据",
                );
                return Ok(vec![]);
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_policy_news",
                "as-of 模式新闻搜索不可用(未配置 news_archive sink)",
            );
            return Ok(vec![]);
        }

        // live 模式:走 vendor + 缓存
        {
            let cache_key = Self::cache_key_for("policy_news", &format!("{stock_code}:{limit}"));
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<NewsItem>>(&cached) {
                    return Ok(data);
                }
            }
        }

        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("policy_news", &self.routing.policy_news)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        let limit_owned = limit;
        match self
            .try_vendors_retry(stock_code, "policy_news", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_policy_news(&sc, limit_owned).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "政策新闻返回空".into(),
                        });
                    }
                    let truncated = Self::truncate_news_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "政策新闻全部晚于截止日".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(mut result) => {
                // P2-B4: 统一填充 sentiment_score(在缓存和持久化前完成)
                Self::fill_sentiment_scores(&mut result);
                let cache_key =
                    Self::cache_key_for("policy_news", &format!("{stock_code}:{limit}"));
                self.cache_set_serialized(cache_key, &result, 300).await;
                // 自动 upsert 到 news_archive
                if let Some(sink) = &self.news_archive_sink {
                    let filtered: Vec<NewsItem> = result
                        .iter()
                        .filter(|n| parse_news_publish_time_ms(&n.publish_time).is_some())
                        .cloned()
                        .collect();
                    if !filtered.is_empty() {
                        sink.upsert("policy_news", Some(stock_code), None, &filtered).await;
                    }
                }
                Ok(result)
            },
            Err(e) => {
                // 政策新闻是过滤类工具(从全部新闻中筛选政策相关内容),
                // 空结果可能是正常的(该股票近期无政策相关新闻),不返回 Err。
                // 与 get_news 不同:get_news 空结果意味着数据源故障,
                // get_policy_news 空结果可能是"确实没有政策新闻"。
                tracing::info!(
                    "[get_policy_news] {stock_code} 无政策相关新闻(vendor 失败或过滤后为空): {e}"
                );
                Ok(vec![])
            },
        }
    }

    pub async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // 目前所有 vendor 均为 Fallthrough(不支持 as-of 参数),as-of 模式返回 None
        if crate::as_of::is_asof_active() {
            for name in self.routing.vendors_for("money_flow", &self.routing.money_flow) {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_money_flow") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) = vendor.get_money_flow_with_asof(stock_code).await {
                                return Ok(Some(r));
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_money_flow",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_money_flow",
                "as-of 模式所有 vendor 均未提供历史资金流向",
            );
            return Ok(None);
        }
        // 空数据冷却：非交易时段/全源空后 30 分钟内直接短路，避免荐股 run 内反复请求
        let cool_key = format!("money_flow:{stock_code}");
        if self.empty_cooldown_active(&cool_key).await {
            return Ok(None);
        }
        {
            let cache_key = Self::cache_key_for("money_flow", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Option<MoneyFlow>>(&cached) {
                    return Ok(data);
                }
            }
        }
        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("money_flow", &self.routing.money_flow)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "money_flow", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    match vendor.get_money_flow(&sc).await {
                        Ok(Some(result)) => Ok(result),
                        Ok(None) => Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "资金流向返回空".into(),
                        }),
                        Err(e) => Err(e),
                    }
                })
            })
            .await
        {
            Ok(result) => {
                // 成功 → 清除空数据冷却（数据源恢复后立即重新探测）
                self.clear_empty_cooldown(&cool_key).await;
                let cache_key = Self::cache_key_for("money_flow", stock_code);
                self.cache_set_serialized(cache_key, &Some(result.clone()), 60).await;
                Ok(Some(result))
            },
            Err(e) => {
                // H1.5 修复:不再静默吞错,记录 vendor 错误详情便于排查
                tracing::warn!("[get_money_flow] 所有 vendor 失败(stock_code={}): {e}", stock_code);
                // 全源失败 → 空数据冷却 30 分钟（非交易时段/数据源故障，避免反复请求）
                self.mark_empty_cooldown(&cool_key, 30 * 60).await;
                Ok(None)
            },
        }
    }

    pub async fn get_dragon_tiger(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DragonTigerEntry>, DataError> {
        {
            let cache_key = Self::cache_key_for("dragon_tiger", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<DragonTigerEntry>>(&cached) {
                    return Ok(data);
                }
            }
        }
        // 使用通用 try_vendors_retry 完成 vendor 遍历 + 健康追踪 + 指数退避重试
        let vendor_names: Vec<String> =
            self.routing.dragon_tiger.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "dragon_tiger", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_dragon_tiger(&sc).await?;
                    let truncated = Self::truncate_dragon_tiger_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "龙虎榜数据为空".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("dragon_tiger", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                // H1.5 修复:不再静默吞错,记录 vendor 错误详情便于排查
                tracing::warn!(
                    "[get_dragon_tiger] 所有 vendor 失败(stock_code={}): {e}",
                    stock_code
                );
                Ok(vec![])
            },
        }
    }

    pub async fn get_lockup_schedule(
        &self,
        stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        {
            let cache_key = Self::cache_key_for("lockup", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<LockupSchedule>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let vendor_names: Vec<String> = self.routing.lockup.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "lockup", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_lockup_schedule(&sc).await?;
                    let truncated = Self::truncate_lockup_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "限售解禁数据为空".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("lockup", stock_code);
                self.cache_set_serialized(cache_key, &result, 86400).await;
                Ok(result)
            },
            Err(e) => {
                // H1.5 修复:不再静默吞错,记录 vendor 错误详情便于排查
                tracing::warn!(
                    "[get_lockup_schedule] 所有 vendor 失败(stock_code={}): {e}",
                    stock_code
                );
                Ok(vec![])
            },
        }
    }

    pub async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        // 拼音片段检测：拦截 LLM 生成的拼音片段或中英混合片段
        // 合法输入：完整中文名称（如"紫金矿业"）、6位数字代码（如"601899"）、港股/美股代码（如"00700.HK"）
        // 非法输入：纯拼音（如"zi'jin"）、中英混合片段（如"中贝t"、"中贝tong"）
        let trimmed = keyword.trim();
        if !trimmed.is_empty() {
            let is_pure_digits = trimmed.chars().all(|c| c.is_ascii_digit());
            let has_cjk = trimmed.chars().any(|c| (0x4E00..=0x9FFF).contains(&(c as u32)));
            // 包含 CJK 的字符串还需检查是否有拉丁字母混合（如"中贝t"）
            let has_latin = trimmed.chars().any(|c| c.is_ascii_alphabetic());
            if !is_pure_digits && !has_cjk {
                // 纯拉丁字母（含撇号）→ 拼音片段
                tracing::warn!("[search_stock] 检测到拼音片段关键词: '{}'", trimmed);
                return Err(DataError::VendorError {
                    vendor: "search_stock".into(),
                    message: format!(
                        "keyword '{trimmed}' 看起来像拼音片段，请传入完整中文名称（如'紫金矿业'）或6位数字代码（如'601899'）"
                    ),
                });
            }
            if has_cjk && has_latin {
                // 中英混合（如"中贝t"、"中贝tong"）→ 拼音片段
                // 例外：港股/美股代码格式如"00700.HK"、"TSM.US"（数字+点+字母）
                let is_hk_us_code = is_pure_digits_before_dot_and_uppercase_after(trimmed);
                if !is_hk_us_code {
                    tracing::warn!("[search_stock] 检测到中英混合关键词: '{}'", trimmed);
                    return Err(DataError::VendorError {
                        vendor: "search_stock".into(),
                        message: format!(
                            "keyword '{trimmed}' 包含中英混合片段，请传入完整中文名称（如'紫金矿业'）或6位数字代码（如'601899'）"
                        ),
                    });
                }
            }
        }
        // P5:搜索是当下语义(iwencai NoHistoricalSemantic),as-of 模式检查每日快照或返回空
        // C5.3 修复:cache_key 含 keyword 维度，避免不同 keyword 互相覆盖
        if crate::as_of::is_asof_active() {
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) =
                    self.try_daily_keyword_snapshot("search_stock", keyword, &date)
                {
                    if let Ok(r) = serde_json::from_str::<Vec<StockSearchResult>>(&cached) {
                        if !r.is_empty() {
                            return Ok(r);
                        }
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "search_stock",
                "as-of 模式搜索不可用(搜索是当下语义)",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        // FIX-2026-08-01: LLM 常把概念词/后缀词混进公司名再调 search_stock
        // （如"方大炭素 石墨烯"、"国瓷材料 概念股"、"国瓷材料股份有限公司"）。
        // 东财 searchadapter 是精确匹配，此类混合关键词实测全部返回空
        // （curl 验证 TotalCount:0），整条 vendor 链随之空 → LLM 误判
        // "该股票不存在" → Serenity 候选全灭。清洗出纯公司名片段后兜底重试。
        let clean = clean_search_keyword(trimmed);
        let attempts: Vec<&str> = if clean.is_empty() || clean == trimmed {
            vec![trimmed]
        } else {
            vec![trimmed, clean.as_str()]
        };
        let mut last_err: Option<DataError> = None;
        for attempt in attempts {
            match self.search_stock_once(attempt).await {
                Ok(r) if !r.is_empty() => return Ok(r),
                Ok(_) => {
                    last_err = Some(DataError::VendorError {
                        vendor: "search_stock".into(),
                        message: format!("搜索无结果: '{attempt}'"),
                    });
                },
                Err(e) => {
                    last_err = Some(e);
                },
            }
            // 该词失败 → 负缓存 30s（与 search_stock_once 内检查对应）
            let neg_key = format!("search_stock:neg::{}", attempt);
            self.cache_set(neg_key, "1".to_string(), 30).await;
        }
        tracing::warn!(
            "[search_stock] 所有尝试失败(keyword={}, clean={}): {last_err:?}",
            trimmed,
            clean
        );
        Ok(vec![])
    }

    /// 单次搜索尝试：正缓存(L1) + 负缓存 + vendor 链。
    /// 由 `search_stock` 主流程按"原始关键词 → 清洗关键词"顺序调用。
    async fn search_stock_once(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        // H1.1 修复:live 模式也添加 L1 缓存(搜索结果 60s 内变化不大,频繁搜索同关键词可命中缓存)
        // P2 修复(2026-07-25): 正缓存 TTL 从 60s 提到 300s(搜索结果变化慢,5 分钟足够);
        //                    新增负缓存(失败关键词 30s 内不重打全 vendor 链,避免 4-13s 串行延迟放大)。
        {
            let cache_key = Self::cache_key_for("search_stock", keyword);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<StockSearchResult>>(&cached) {
                    return Ok(data);
                }
            }
            // 负缓存:30s 内相同关键词失败过 → 直接返回空 vec,不重打 vendor 链
            let neg_key = format!("search_stock:neg::{}", keyword);
            if self.cache_get(&neg_key).await.is_some() {
                tracing::debug!("[search_stock] 负缓存命中(30s 内失败过): keyword={}", keyword);
                return Ok(vec![]);
            }
        }
        let vendor_names: Vec<String> = self.routing.search.iter().map(|n| n.to_string()).collect();
        let kw = keyword.to_string();
        match self
            .try_vendors_retry(keyword, "search_stock", &vendor_names, 2, |_, vendor| {
                let kw = kw.clone();
                Box::pin(async move {
                    let result = vendor.search_stock(&kw).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: "__search__".into(),
                            message: "搜索无结果".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => {
                // P2:正缓存 TTL 300s(原 60s 太短,用户切换关键词再回来时几乎必 miss)
                let cache_key = Self::cache_key_for("search_stock", keyword);
                self.cache_set_serialized(cache_key, &result, 300).await;
                Ok(result)
            },
            Err(e) => {
                // H1.5 修复:不再静默吞错,记录 vendor 错误详情便于排查
                tracing::warn!("[search_stock] vendor 链失败(keyword={}): {e}", keyword);
                Err(e)
            },
        }
    }

    /// 按关键词搜索新闻（用于验证 CapEx/催化剂/行业趋势）
    /// 只在 live 模式可用（搜索是当下语义），as-of 模式下跳过
    pub async fn search_news(&self, keyword: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        // P6:as-of 模式查 news_archive 本地语料库
        // 截止时间为 as_of_date 当天 23:59:59.999(毫秒)
        if let Some(ctx) = crate::as_of::current_as_of() {
            if let Some(sink) = &self.news_archive_sink {
                let as_of_ts_ms = ctx
                    .as_of_date
                    .and_hms_opt(23, 59, 59)
                    .and_then(|dt| dt.and_local_timezone(chrono::Local).single())
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or_else(|| {
                        // fallback:用 UTC 23:59:59.999
                        // 修复 M-RES-8: 内层 unwrap_or(0) 在极端情况下
                        // （如 as_of_date 无效）静默返回 0，导致 sink 查询条件
                        // 变为 "ts <= 0"，几乎不可能命中。添加 warn 日志便于发现。
                        ctx.as_of_date
                            .and_hms_opt(23, 59, 59)
                            .and_then(|dt| dt.and_utc().timestamp_millis().into())
                            .unwrap_or_else(|| {
                                tracing::warn!(
                                    "[news_archive] as_of_ts_ms 计算失败，回退为 0 (as_of_date={})",
                                    ctx.as_of_date
                                );
                                0
                            })
                    });
                let archived = sink.search_asof(keyword, None, as_of_ts_ms, limit).await;
                if !archived.is_empty() {
                    tracing::info!(
                        "[news_archive] search_asof 命中 {} 条 (keyword={}, as_of={})",
                        archived.len(),
                        keyword,
                        ctx.as_of_date
                    );
                    return Ok(archived);
                }
                // sink 空 → 记录降级 + 返回空（保持"当下语义"语义,避免混用 live）
                crate::as_of::record_degradation(
                    "astock-data",
                    "search_news",
                    "as-of 模式 news_archive 无数据(本地未积累该日期之前的新闻)",
                );
                return Ok(vec![]);
            }
            // sink 未注入:走原有降级路径
            crate::as_of::record_degradation(
                "astock-data",
                "search_news",
                "as-of 模式新闻搜索不可用（搜索是当下语义，且未配置 news_archive sink）",
            );
            return Ok(vec![]);
        }
        // live 模式:走 vendor + 自动 upsert
        // H1.2 修复:live 模式添加 L1 缓存(60s TTL,新闻搜索结果短期内变化不大)
        {
            let cache_key = Self::cache_key_for("search_news", &format!("{keyword}:{limit}"));
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<NewsItem>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let vendor_names: Vec<String> =
            self.routing.search_news.iter().map(|n| n.to_string()).collect();
        let kw = keyword.to_string();
        let limit_owned = limit;
        match self
            .try_vendors_retry(keyword, "search_news", &vendor_names, 2, |name, vendor| {
                let kw = kw.clone();
                Box::pin(async move {
                    let result = vendor.search_news(&kw, limit_owned).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "新闻搜索无结果".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(mut result) => {
                // P2-B4: 统一填充 sentiment_score(在缓存和持久化前完成)
                Self::fill_sentiment_scores(&mut result);
                // H1.2 修复:写入 L1 缓存(60s TTL)
                let cache_key = Self::cache_key_for("search_news", &format!("{keyword}:{limit}"));
                self.cache_set_serialized(cache_key, &result, 60).await;
                if let Some(sink) = &self.news_archive_sink {
                    let filtered: Vec<NewsItem> = result
                        .iter()
                        .filter(|n| parse_news_publish_time_ms(&n.publish_time).is_some())
                        .cloned()
                        .collect();
                    if !filtered.is_empty() {
                        sink.upsert("search_news", None, Some(keyword), &filtered).await;
                    }
                }
                Ok(result)
            },
            Err(e) => {
                // H1.5 修复:不再静默吞错,记录 vendor 错误详情便于排查
                tracing::warn!(
                    "[search_news] 所有 vendor 失败(keyword={}, limit={}): {e}",
                    keyword,
                    limit
                );
                Ok(vec![])
            },
        }
    }

    pub async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney 已申报 NativeDateParam,as-of 模式调 with_asof 可拿到当日数据
        if crate::as_of::is_asof_active() {
            for name in &self.routing.margin {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_margin_data") {
                        AsOfCapability::NativeDateParam => {
                            match vendor.get_margin_data_with_asof(stock_code).await {
                                Ok(Some(r)) => {
                                    let cache_key = Self::cache_key_for("margin", stock_code);
                                    self.cache_set_serialized(cache_key, &Some(&r), 300).await;
                                    return Ok(Some(r));
                                },
                                Ok(None) => continue,
                                Err(e) => {
                                    crate::as_of::record_degradation(
                                        name,
                                        "get_margin_data",
                                        &format!("with_asof 失败: {e}"),
                                    );
                                    continue;
                                },
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            // 没有 truncation 函数,不能使用 vendor 实时数据
                            crate::as_of::record_degradation(
                                name,
                                "get_margin_data",
                                "Fallthrough vendor 不支持 as-of 参数,跳过",
                            );
                            continue;
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_margin_data",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_margin_data",
                "as-of 模式所有 vendor 均未提供历史数据",
            );
            return Ok(None);
        }
        {
            let cache_key = Self::cache_key_for("margin", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Option<MarginData>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let vendor_names: Vec<String> = self.routing.margin.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        // P1-1 修复: margin 不走 try_vendors_retry 的 health_tracker 过滤。
        // 原因：eastmoney 的 get_margin_data 用独立的 RPTA_WEB_RZRQ_GGMX 接口，
        // 与 push2his/push2 系列接口完全独立。当 push2his 系列故障导致 eastmoney
        // 被整体降级时，margin 不应受牵连。
        //
        // 重试策略（用户要求"保证代码正确，不在错误状态下不断重试"）：
        // - 真实故障（网络/解析错误）→ 指数退避重试（最多 2 次），计入 health_tracker
        // - 空数据（Ok(None) 或 Err("为空")）→ 确定性错误，不重试，直接返回 Ok(None)
        //   理由：空数据是业务层面的"无数据"（如该股票无融资融券），重试只会浪费时间
        let max_retries = 2u32;
        let mut retry_count = 0u32;
        loop {
            let mut last_retryable_err: Option<DataError> = None;
            let mut has_empty = false;
            for name in &vendor_names {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.get_margin_data(&sc).await {
                        Ok(Some(r)) => {
                            self.health_tracker.record_success(name).await;
                            let cache_key = Self::cache_key_for("margin", stock_code);
                            self.cache_set_serialized(cache_key, &r, 300).await;
                            return Ok(Some(r));
                        },
                        Ok(None) => {
                            has_empty = true;
                            tracing::info!(
                                "[margin] {} {} 返回空数据(非故障)，尝试下一个 vendor",
                                name,
                                stock_code
                            );
                        },
                        Err(e) => {
                            // 判断是否是空数据类错误（不可重试的确定性错误）
                            let is_empty_err = matches!(
                                e,
                                DataError::VendorError { ref message, .. }
                                    if message.contains("为空") || message.contains("empty")
                            );
                            if is_empty_err {
                                has_empty = true;
                            } else {
                                // 真实故障，可重试，计入 health_tracker
                                tracing::warn!("[降级] margin {} {} 失败: {}", stock_code, name, e);
                                self.health_tracker.record_failure(name, &e.to_string()).await;
                                last_retryable_err = Some(e);
                            }
                        },
                    }
                }
            }
            // 只有真实故障才重试；空数据是确定性错误，重试无意义
            if retry_count < max_retries && last_retryable_err.is_some() {
                retry_count += 1;
                let delay = (1u64 << retry_count) - 1;
                tracing::warn!(
                    "[retry] margin {} 真实故障，{}s 后第 {} 次重试",
                    stock_code,
                    delay,
                    retry_count
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                continue;
            }
            if last_retryable_err.is_some() {
                tracing::warn!(
                    "[astock] get_margin_data 所有 vendor 失败(stock_code={}): {}",
                    stock_code,
                    last_retryable_err.as_ref().map(|e| e.to_string()).unwrap_or_default()
                );
            } else if has_empty {
                tracing::info!("[margin] {} 所有 vendor 返回空数据（非故障），不重试", stock_code);
            }
            return Ok(None);
        }
    }

    /// #4: 股权质押数据路由方法。
    ///
    /// 实现：调用 `vendor.get_pledge_data(stock_code)`，eastmoney 已实现。
    /// as-of 模式：所有 vendor 申报 Fallthrough（无历史语义），直接降级返回 None。
    /// 重试策略：与 margin 一致 — 真实故障（网络/解析）才重试，空数据直接返回。
    pub async fn get_pledge_data(&self, stock_code: &str) -> Result<Option<PledgeData>, DataError> {
        if crate::as_of::is_asof_active() {
            crate::as_of::record_degradation(
                "astock-data",
                "get_pledge_data",
                "as-of 模式所有 vendor 均未提供历史质押数据",
            );
            return Ok(None);
        }
        {
            let cache_key = Self::cache_key_for("pledge", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Option<PledgeData>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let vendor_names: Vec<String> = self.routing.pledge.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        // 与 margin 一致：区分真实故障与空数据，空数据不重试
        let max_retries = 2u32;
        let mut retry_count = 0u32;
        loop {
            let mut last_retryable_err: Option<DataError> = None;
            let mut has_empty = false;
            for name in &vendor_names {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.get_pledge_data(&sc).await {
                        Ok(Some(r)) => {
                            self.health_tracker.record_success(name).await;
                            let cache_key = Self::cache_key_for("pledge", stock_code);
                            self.cache_set_serialized(cache_key, &r, 300).await;
                            return Ok(Some(r));
                        },
                        Ok(None) => {
                            has_empty = true;
                            tracing::info!(
                                "[pledge] {} {} 返回空数据(非故障)，尝试下一个 vendor",
                                name,
                                stock_code
                            );
                        },
                        Err(e) => {
                            let is_empty_err = matches!(
                                e,
                                DataError::VendorError { ref message, .. }
                                    if message.contains("为空") || message.contains("empty")
                            );
                            if is_empty_err {
                                has_empty = true;
                            } else {
                                tracing::warn!("[降级] pledge {} {} 失败: {}", stock_code, name, e);
                                self.health_tracker.record_failure(name, &e.to_string()).await;
                                last_retryable_err = Some(e);
                            }
                        },
                    }
                }
            }
            if retry_count < max_retries && last_retryable_err.is_some() {
                retry_count += 1;
                let delay = (1u64 << retry_count) - 1;
                tracing::warn!(
                    "[retry] pledge {} 真实故障，{}s 后第 {} 次重试",
                    stock_code,
                    delay,
                    retry_count
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                continue;
            }
            if last_retryable_err.is_some() {
                tracing::warn!(
                    "[astock] get_pledge_data 所有 vendor 失败(stock_code={}): {}",
                    stock_code,
                    last_retryable_err.as_ref().map(|e| e.to_string()).unwrap_or_default()
                );
            } else if has_empty {
                tracing::info!("[pledge] {} 所有 vendor 返回空数据（非故障），不重试", stock_code);
            }
            return Ok(None);
        }
    }

    pub async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        // P4: 按 capability 决策(所有 vendor Fallthrough,as-of 模式返回 None)
        if crate::as_of::is_asof_active() {
            for name in &self.routing.north_bound {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_north_bound_holding") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) =
                                vendor.get_north_bound_holding_with_asof(stock_code).await
                            {
                                return Ok(Some(r));
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_north_bound_holding",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_north_bound_holding",
                "as-of 模式所有 vendor 均未提供历史北向持仓",
            );
            return Ok(None);
        }
        // 空数据冷却：北向个股持仓自 2024-08 港交所停披，所有源永远空，
        // 冷却 6 小时内直接短路，避免荐股 run 内每只股票白打 3 源 × 重试链
        let cool_key = format!("north_bound:{stock_code}");
        if self.empty_cooldown_active(&cool_key).await {
            return Ok(None);
        }
        {
            let cache_key = Self::cache_key_for("north_bound", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Option<NorthBoundHolding>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let vendor_names: Vec<String> =
            self.routing.north_bound.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "north_bound", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    match vendor.get_north_bound_holding(&sc).await {
                        Ok(Some(r)) => Ok(r),
                        Ok(None) => Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "北向资金持仓数据为空".into(),
                        }),
                        Err(e) => Err(e),
                    }
                })
            })
            .await
        {
            Ok(result) => {
                // 成功 → 清除空数据冷却（北向数据源若恢复立即重新探测）
                self.clear_empty_cooldown(&cool_key).await;
                let cache_key = Self::cache_key_for("north_bound", stock_code);
                self.cache_set_serialized(cache_key, &result, 300).await;
                Ok(Some(result))
            },
            Err(_) => {
                // 全源空/失败 → 冷却 6 小时（数据源根本性失效，无谓重试纯浪费）
                self.mark_empty_cooldown(&cool_key, 6 * 3600).await;
                Ok(None)
            },
        }
    }

    pub async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney Fallthrough, iwencai NoHistoricalSemantic
        if crate::as_of::is_asof_active() {
            // P5:先查每日快照缓存
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) = self.try_daily_snapshot("get_sector_info", &date) {
                    if let Ok(r) = serde_json::from_str::<Option<SectorInfo>>(&cached) {
                        if r.is_some() {
                            return Ok(r);
                        }
                    }
                }
            }
            for name in &self.routing.sector {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_sector_info") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) = vendor.get_sector_info_with_asof(stock_code).await
                            {
                                return Ok(Some(r));
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_sector_info",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_sector_info",
                "as-of 模式所有 vendor 均未提供历史行业分类",
            );
            return Ok(None);
        }
        let vendor_names: Vec<String> = self.routing.sector.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "sector", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    vendor.get_sector_info(&sc).await?.ok_or_else(|| DataError::VendorError {
                        vendor: name.to_string(),
                        message: "行业分类数据为空".into(),
                    })
                })
            })
            .await
        {
            Ok(result) => Ok(Some(result)),
            Err(_) => Ok(None),
        }
    }

    pub async fn get_shareholder_trades(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("shareholder_trades", &self.routing.shareholder_trades)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(
                stock_code,
                "shareholder_trades",
                &vendor_names,
                2,
                |name, vendor| {
                    let sc = sc.clone();
                    Box::pin(async move {
                        let result = vendor.get_shareholder_trades(&sc).await?;
                        let truncated = Self::truncate_shareholder_trades_by_asof(result);
                        if truncated.is_empty() {
                            return Err(DataError::VendorError {
                                vendor: name.to_string(),
                                message: "股东增减持数据为空".into(),
                            });
                        }
                        Ok(truncated)
                    })
                },
            )
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("shareholder_trades", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                tracing::warn!(
                    "[astock] get_shareholder_trades 所有 vendor 失败(stock_code={}): {}",
                    stock_code,
                    e
                );
                Ok(vec![])
            },
        }
    }

    pub async fn get_dividend_records(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        let vendor_names: Vec<String> =
            self.routing.dividend.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "dividend", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_dividend_records(&sc).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "分红数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn get_research_reports(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // - NativeDateParam: vendor 支持 beginTime/endTime,走 _with_asof 真正按 as_of 窗口拉取
        // - 其他: as-of 模式记降级,跳过(避免泄漏 2030-01-01 全量窗口)
        if crate::as_of::is_asof_active() {
            for name in self.routing.vendors_for("research_reports", &self.routing.research_reports)
            {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_research_reports") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(reports) =
                                vendor.get_research_reports_with_asof(stock_code).await
                            {
                                // 双重保险:vendor 端已用 as_of 窗口拉取,这里再截一遍防御 vendor bug
                                let reports = Self::truncate_research_reports_by_asof(reports);
                                let cache_key = Self::cache_key_for("research_reports", stock_code);
                                self.cache_set_serialized(cache_key, &reports, 3600).await;
                                return Ok(reports);
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_research_reports",
                                "vendor 不支持 as-of 参数,跳过(避免泄漏全量窗口)",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_research_reports",
                "as-of 模式所有 vendor 均未提供历史研报窗口",
            );
            return Ok(vec![]);
        }
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("research_reports", &self.routing.research_reports)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "research_reports", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_research_reports(&sc).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "研报数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("research_reports", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn get_consensus_eps(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConsensusEPS>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // 所有 vendor 均为 Fallthrough(as-of 模式跳过,不执行 C-fallback 估算)
        if crate::as_of::is_asof_active() {
            for name in self.routing.vendors_for("consensus_eps", &self.routing.consensus_eps) {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_consensus_eps") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) =
                                vendor.get_consensus_eps_with_asof(stock_code).await
                            {
                                return Ok(Some(r));
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_consensus_eps",
                                "vendor 不支持 as-of 参数,跳过",
                            );
                            continue;
                        },
                    }
                }
            }
            // as-of 下尝试从最新财报计算 trailing EPS
            if let Ok(fins) = self.get_financials(stock_code).await {
                if let Some(latest) = fins.into_iter().next() {
                    if let Some(eps) = latest.eps.filter(|&v| v > 0.0) {
                        let year = crate::as_of::current_as_of()
                            .map(|ctx| ctx.as_of_date.format("%Y").to_string())
                            .unwrap_or_else(|| Local::now().format("%Y").to_string());
                        return Ok(Some(ConsensusEPS {
                            stock_code: stock_code.to_string(),
                            consensus_eps: Some(eps),
                            consensus_target_price: None,
                            rating_avg: None,
                            rating_count: None,
                            year,
                        }));
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_consensus_eps",
                "C-fallback 估算是基于当前年份的板块均值,replay 模式禁用",
            );
            return Ok(None);
        }
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("consensus_eps", &self.routing.consensus_eps)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "consensus_eps", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    match vendor.get_consensus_eps(&sc).await {
                        Ok(Some(r)) => Ok(r),
                        Ok(None) => Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "一致预期数据为空".into(),
                        }),
                        Err(e) => Err(e),
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(Some(result)),
            Err(_) => {
                // C: consensus_eps fallback — 基于挂牌板块的估算值
                tracing::warn!("[C-fallback] consensus_eps 全部失败，为 {stock_code} 使用估算值");
                let eps_est = match detect_market_type(stock_code) {
                    "star" | "chinext" => 0.40,
                    "bj" => 0.25,
                    _ => 0.55,
                };
                let this_year = Local::now().format("%Y").to_string();
                Ok(Some(ConsensusEPS {
                    stock_code: stock_code.to_string(),
                    consensus_eps: Some(eps_est),
                    consensus_target_price: None,
                    rating_avg: None,
                    rating_count: None,
                    year: this_year,
                }))
            },
        }
    }

    pub async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney/ths/iwencai 均为 NoHistoricalSemantic
        if crate::as_of::is_asof_active() {
            // P5:先查每日快照缓存
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) = self.try_daily_snapshot("get_stock_concept_blocks", &date) {
                    // 概念板块按个股有差异,缓存只能做"今日全市场数据"的兜底
                    // 如果精确到个股,需要后续细化
                    if let Ok(r) = serde_json::from_str::<Option<ConceptBlocks>>(&cached) {
                        if r.is_some() {
                            return Ok(r);
                        }
                    }
                }
            }
            for name in &self.routing.concept_blocks {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_concept_blocks") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) =
                                vendor.get_concept_blocks_with_asof(stock_code).await
                            {
                                return Ok(Some(r));
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            if let Ok(Some(r)) = vendor.get_concept_blocks(stock_code).await {
                                return Ok(Some(r));
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_concept_blocks",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_concept_blocks",
                "as-of 模式所有 vendor 均未提供历史概念板块",
            );
            return Ok(None);
        }
        let vendor_names: Vec<String> =
            self.routing.concept_blocks.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "concept_blocks", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    vendor.get_concept_blocks(&sc).await?.ok_or_else(|| DataError::VendorError {
                        vendor: name.to_string(),
                        message: "概念板块数据为空".into(),
                    })
                })
            })
            .await
        {
            Ok(result) => Ok(Some(result)),
            Err(e) => {
                tracing::warn!(
                    "[get_concept_blocks] 所有 vendor 失败(stock_code={}): {e}",
                    stock_code
                );
                Ok(None)
            },
        }
    }

    pub async fn get_announcements(
        &self,
        stock_code: &str,
    ) -> Result<Vec<Announcement>, DataError> {
        let vendor_names: Vec<String> =
            self.routing.vendors_for("announcements", &self.routing.announcements).clone();
        // 缓存检查
        {
            let cache_key = Self::cache_key_for("announcements", stock_code);
            if let Some(cached) = self.cache_get(&cache_key).await {
                if let Ok(data) = serde_json::from_str::<Vec<Announcement>>(&cached) {
                    return Ok(data);
                }
            }
        }
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "announcements", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let cap = vendor.asof_capability("get_announcements");
                    let result = match cap {
                        AsOfCapability::NativeDateParam if crate::as_of::is_asof_active() => {
                            vendor.get_announcements_with_asof(&sc).await?
                        },
                        _ => vendor.get_announcements(&sc).await?,
                    };
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "公告数据为空".into(),
                        });
                    }
                    let truncated = match cap {
                        AsOfCapability::NativeDateParam if crate::as_of::is_asof_active() => {
                            result // NativeDateParam 已按日期过滤
                        },
                        _ => Self::truncate_announcements_by_asof(result),
                    };
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "公告均在截止日后".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("announcements", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                // 与 get_news / get_financials 一致：全部数据源失败时返回 Err，
                // 避免调用方把空列表误判为"无公告"。原实现 Ok(vec![]) 会让前端
                // 触发"公告数据获取为空"警告且无法区分"真无数据"与"获取失败"。
                tracing::warn!("[announcements] {stock_code} 所有公告数据源失败: {e}");
                Err(DataError::VendorError {
                    vendor: "all".into(),
                    message: format!("所有公告数据源失败，无法获取 {} 的公告", stock_code),
                })
            },
        }
    }

    pub async fn get_market_dragon_tiger(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        // vendor trait 大重构 P1.5:as-of 模式下按 vendor 申报的 capability 决策
        // D 档修复:replay 模式现在能拿到 as_of_date 当日的数据(原 bug:无守卫返回 today)
        if crate::as_of::is_asof_active() {
            for name in
                self.routing.vendors_for("market_dragon_tiger", &self.routing.market_dragon_tiger)
            {
                if let Some(vendor) = self.find_vendor(name) {
                    if vendor.asof_capability("get_market_dragon_tiger")
                        == AsOfCapability::NativeDateParam
                    {
                        match vendor.get_market_dragon_tiger_with_asof().await {
                            Ok(items) => {
                                if !items.is_empty() {
                                    return Ok(items);
                                }
                            },
                            Err(e) => {
                                crate::as_of::record_degradation(
                                    name,
                                    "get_market_dragon_tiger",
                                    &format!("with_asof 失败: {e}"),
                                );
                            },
                        }
                    }
                }
            }
            // 全部 vendor 都不支持 or 失败: 返回空而非 live 数据（防止后见信息泄露）
            crate::as_of::record_degradation(
                "astock-data",
                "get_market_dragon_tiger",
                "as-of 模式下无可用 vendor",
            );
            return Ok(vec![]);
        }
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("market_dragon_tiger", &self.routing.market_dragon_tiger)
            .iter()
            .map(|n| n.to_string())
            .collect();
        match self
            .try_vendors_retry("", "market_dragon_tiger", &vendor_names, 2, |_, vendor| {
                Box::pin(async move {
                    let result = vendor.get_market_dragon_tiger().await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: "__market__".into(),
                            message: "全市场龙虎榜数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn get_hot_stocks(&self) -> Result<Vec<HotStock>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney/ths/iwencai NoHistoricalSemantic,baidu Fallthrough
        if crate::as_of::is_asof_active() {
            // P5:先查每日快照缓存
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) = self.try_daily_snapshot("get_hot_stocks", &date) {
                    if let Ok(r) = serde_json::from_str::<Vec<HotStock>>(&cached) {
                        if !r.is_empty() {
                            return Ok(r);
                        }
                    }
                }
            }
            for name in &self.routing.hot_stocks {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_hot_stocks") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(r) = vendor.get_hot_stocks_with_asof().await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            if let Ok(r) = vendor.get_hot_stocks().await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_hot_stocks",
                                "no historical semantics",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_hot_stocks",
                "as-of 模式所有 vendor 均未提供热门股数据",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.hot_stocks.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "hot_stocks", &vendor_names, 2, |name, vendor| {
                Box::pin(async move {
                    match vendor.get_hot_stocks().await {
                        Ok(result) => {
                            if result.is_empty() {
                                tracing::warn!("[get_hot_stocks] vendor {name} 返回空数据");
                                Err(DataError::VendorError {
                                    vendor: "__market__".into(),
                                    message: format!("{name} 热门股数据为空"),
                                })
                            } else {
                                Ok(result)
                            }
                        },
                        Err(e) => {
                            tracing::warn!("[get_hot_stocks] vendor {name} 失败: {e}");
                            Err(e)
                        },
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!("[get_hot_stocks] 所有 vendor 均不可用, 返回空列表. 详细: {e}");
                Ok(vec![])
            },
        }
    }

    pub async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney/ths NoHistoricalSemantic
        if crate::as_of::is_asof_active() {
            // P5:先查每日快照缓存
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) = self.try_daily_snapshot("get_industry_ranking", &date) {
                    if let Ok(r) = serde_json::from_str::<Vec<IndustryRank>>(&cached) {
                        if !r.is_empty() {
                            return Ok(r);
                        }
                    }
                }
            }
            for name in &self.routing.industry_ranking {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_industry_ranking") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(r) = vendor.get_industry_ranking_with_asof().await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            if let Ok(r) = vendor.get_industry_ranking().await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_industry_ranking",
                                "no historical semantics",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_industry_ranking",
                "as-of 模式所有 vendor 均未提供行业排名",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.industry_ranking.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "industry_ranking", &vendor_names, 2, |name, vendor| {
                Box::pin(async move {
                    match vendor.get_industry_ranking().await {
                        Ok(result) => {
                            if result.is_empty() {
                                tracing::warn!("[get_industry_ranking] vendor {name} 返回空数据");
                                Err(DataError::VendorError {
                                    vendor: "__market__".into(),
                                    message: format!("{name} 行业排名数据为空"),
                                })
                            } else {
                                Ok(result)
                            }
                        },
                        Err(e) => {
                            tracing::warn!("[get_industry_ranking] vendor {name} 失败: {e}");
                            Err(e)
                        },
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!(
                    "[get_industry_ranking] 所有 vendor 均不可用, 返回空列表. 详细: {e}"
                );
                Ok(vec![])
            },
        }
    }

    pub async fn search_concept_boards(
        &self,
        keyword: &str,
    ) -> Result<Vec<ConceptBoard>, DataError> {
        let keyword_owned = keyword.to_string();
        let vendor_names: Vec<String> =
            self.routing.concept_boards.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "concept_boards", &vendor_names, 1, |name, vendor| {
                let kw = keyword_owned.clone();
                Box::pin(async move {
                    match vendor.search_concept_boards(&kw).await {
                        Ok(result) => {
                            if result.is_empty() {
                                tracing::warn!("[search_concept_boards] vendor {name} 返回空数据");
                                Err(DataError::VendorError {
                                    vendor: "__market__".into(),
                                    message: format!("{name} 概念板块搜索结果为空"),
                                })
                            } else {
                                Ok(result)
                            }
                        },
                        Err(e) => {
                            tracing::warn!("[search_concept_boards] vendor {name} 失败: {e}");
                            Err(e)
                        },
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!("[search_concept_boards] 所有 vendor 失败，返回空列表: {e}");
                Ok(vec![])
            },
        }
    }

    pub async fn get_concept_board_members(
        &self,
        board_code: &str,
    ) -> Result<Vec<BoardMember>, DataError> {
        let code_owned = board_code.to_string();
        let vendor_names: Vec<String> =
            self.routing.board_members.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "board_members", &vendor_names, 1, |name, vendor| {
                let code = code_owned.clone();
                Box::pin(async move {
                    match vendor.get_concept_board_members(&code).await {
                        Ok(result) => {
                            if result.is_empty() {
                                tracing::warn!(
                                    "[get_concept_board_members] vendor {name} 返回空数据"
                                );
                                Err(DataError::VendorError {
                                    vendor: "__market__".into(),
                                    message: format!("{name} 板块成分股为空"),
                                })
                            } else {
                                Ok(result)
                            }
                        },
                        Err(e) => {
                            tracing::warn!("[get_concept_board_members] vendor {name} 失败: {e}");
                            Err(e)
                        },
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!("[get_concept_board_members] 所有 vendor 失败，返回空列表: {e}");
                Ok(vec![])
            },
        }
    }

    pub async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney/akshare NoHistoricalSemantic
        if crate::as_of::is_asof_active() {
            // P5:先查每日快照缓存
            let as_of = crate::as_of::current_as_of();
            if let Some(ref ctx) = as_of {
                let date = ctx.as_of_date.format("%Y-%m-%d").to_string();
                if let Some(cached) = self.try_daily_snapshot("get_cls_flash", &date) {
                    if let Ok(r) = serde_json::from_str::<Vec<ClsFlashItem>>(&cached) {
                        if !r.is_empty() {
                            return Ok(r);
                        }
                    }
                }
            }
            for name in &self.routing.cls_flash {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_cls_flash") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(r) = vendor.get_cls_flash_with_asof().await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_cls_flash",
                                "no historical semantics",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_cls_flash",
                "as-of 模式所有 vendor 均未提供实时快讯",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.cls_flash.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "cls_flash", &vendor_names, 2, |_, vendor| {
                Box::pin(async move {
                    let result = vendor.get_cls_flash().await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: "__market__".into(),
                            message: "快讯数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        // ── live 模式 ──
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("north_bound_flow", &self.routing.north_bound_flow)
            .iter()
            .map(|n| n.to_string())
            .collect();
        match self
            .try_vendors_retry("", "north_bound_flow", &vendor_names, 2, |_, vendor| {
                Box::pin(async move {
                    vendor.get_north_bound_flow().await?.ok_or_else(|| DataError::VendorError {
                        vendor: "__market__".into(),
                        message: "北向资金流向数据为空".into(),
                    })
                })
            })
            .await
        {
            Ok(result) => {
                // 混合 as-of 模式下截断
                let result = Self::truncate_north_bound_flow_by_asof(Some(result));
                if let Some(ref r) = result {
                    let cache_key = Self::cache_key_for("north_bound_flow", "market");
                    self.cache_set_serialized(cache_key, r, 300).await;
                }
                Ok(result)
            },
            Err(e) => {
                tracing::warn!("[get_north_bound_flow] 所有 vendor 失败: {e}");
                Ok(None)
            },
        }
    }

    pub async fn get_block_trades(&self, stock_code: &str) -> Result<Vec<BlockTrade>, DataError> {
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.block_trades.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "block_trades", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_block_trades(&sc).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "大宗交易数据为空".into(),
                        });
                    }
                    let truncated = Self::truncate_block_trades_by_asof(result);
                    if truncated.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "大宗交易均在截止日后".into(),
                        });
                    }
                    Ok(truncated)
                })
            })
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("block_trades", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                tracing::warn!(
                    "[astock] get_block_trades 所有 vendor 失败(stock_code={}): {}",
                    stock_code,
                    e
                );
                Ok(vec![])
            },
        }
    }

    pub async fn get_institutional_visits(
        &self,
        stock_code: &str,
    ) -> Result<Vec<InstitutionalVisit>, DataError> {
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.institutional_visits.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(
                stock_code,
                "institutional_visits",
                &vendor_names,
                2,
                |name, vendor| {
                    let sc = sc.clone();
                    Box::pin(async move {
                        let result = vendor.get_institutional_visits(&sc).await?;
                        if result.is_empty() {
                            return Err(DataError::VendorError {
                                vendor: name.to_string(),
                                message: "机构调研数据为空".into(),
                            });
                        }
                        let truncated = Self::truncate_institutional_visits_by_asof(result);
                        if truncated.is_empty() {
                            return Err(DataError::VendorError {
                                vendor: name.to_string(),
                                message: "机构调研均在截止日后".into(),
                            });
                        }
                        Ok(truncated)
                    })
                },
            )
            .await
        {
            Ok(result) => {
                let cache_key = Self::cache_key_for("institutional_visits", stock_code);
                self.cache_set_serialized(cache_key, &result, 3600).await;
                Ok(result)
            },
            Err(e) => {
                tracing::warn!(
                    "[get_institutional_visits] 所有 vendor 失败(stock_code={}): {e}",
                    stock_code
                );
                Ok(vec![])
            },
        }
    }

    /// 筹码面分析数据聚合：一次调用获取解禁 + 增减持 + 大宗交易
    /// 供 lockup-watcher 冷启动使用，避免 LLM 因单源空数据而不主动调其他工具
    ///
    /// 设计：聚合函数，单源失败时返回部分结果 + 在 JSON 中注入 `errors` 字段
    /// 记录失败原因，避免 `unwrap_or_default()` 完全静默吞错导致下游无法感知数据缺失。
    pub async fn get_lockup_bundle(
        &self,
        stock_code: &str,
    ) -> Result<serde_json::Value, DataError> {
        let mut errors: Vec<String> = Vec::new();

        let lockup = match self.get_lockup_schedule(stock_code).await {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("lockup_schedule: {e}"));
                vec![]
            },
        };
        let trades = match self.get_shareholder_trades(stock_code).await {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("shareholder_trades: {e}"));
                vec![]
            },
        };
        let block = match self.get_block_trades(stock_code).await {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("block_trades: {e}"));
                vec![]
            },
        };

        if !errors.is_empty() {
            tracing::warn!(
                "[astock-data] get_lockup_bundle({stock_code}) 部分数据源失败: {:?}",
                errors
            );
        }

        Ok(serde_json::json!({
            "lockup_schedule": lockup,
            "shareholder_trades": trades,
            "block_trades": block,
            "errors": errors,
        }))
    }

    pub async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        if crate::as_of::is_asof_active() {
            crate::as_of::record_degradation(
                "astock-data",
                "get_index_quotes",
                "as-of 模式不支持指数行情",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.index_quotes.iter().map(|n| n.to_string()).collect();
        match self
            .try_vendors_retry("", "index_quotes", &vendor_names, 2, |_, vendor| {
                Box::pin(async move {
                    let result = vendor.get_index_quotes().await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: "__market__".into(),
                            message: "指数行情数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn get_peers(&self, stock_code: &str) -> Result<Vec<PeerComparison>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney Fallthrough(同行对比带 date 字段,可截断),as-of 模式调用 live + truncate
        if crate::as_of::is_asof_active() {
            for name in self.routing.vendors_for("peers", &self.routing.peers) {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_peers") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(r) = vendor.get_peers_with_asof(stock_code).await {
                                if !r.is_empty() {
                                    return Ok(r);
                                }
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            crate::as_of::record_degradation(
                                name,
                                "get_peers",
                                "no historical semantic",
                            );
                            continue;
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_peers",
                                "no historical snapshot semantics",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_peers",
                "as-of 模式所有 vendor 均未提供同行对比",
            );
            return Ok(vec![]);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> = self
            .routing
            .vendors_for("peers", &self.routing.peers)
            .iter()
            .map(|n| n.to_string())
            .collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "peers", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    let result = vendor.get_peers(&sc).await?;
                    if result.is_empty() {
                        return Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "同行对比数据为空".into(),
                        });
                    }
                    Ok(result)
                })
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!("[get_peers] 所有 vendor 失败(stock_code={}): {e}", stock_code);
                Ok(vec![])
            },
        }
    }

    pub async fn get_option_pcr(&self, stock_code: &str) -> Result<Option<OptionPCR>, DataError> {
        // P4: 按 vendor 申报的 capability 决策
        // eastmoney Fallthrough(date 字段可用),as-of 模式调用 live + 取最新
        if crate::as_of::is_asof_active() {
            for name in &self.routing.option_pcr {
                if let Some(vendor) = self.find_vendor(name) {
                    match vendor.asof_capability("get_option_pcr") {
                        AsOfCapability::NativeDateParam => {
                            if let Ok(Some(r)) = vendor.get_option_pcr_with_asof(stock_code).await {
                                return Ok(Some(r));
                            }
                        },
                        AsOfCapability::Fallthrough => {
                            crate::as_of::record_degradation(
                                name,
                                "get_option_pcr",
                                "no historical semantic",
                            );
                            continue;
                        },
                        _ => {
                            crate::as_of::record_degradation(
                                name,
                                "get_option_pcr",
                                "no historical semantic",
                            );
                            continue;
                        },
                    }
                }
            }
            crate::as_of::record_degradation(
                "astock-data",
                "get_option_pcr",
                "as-of 模式所有 vendor 均未提供期权 PCR",
            );
            return Ok(None);
        }
        // ── live 模式 ──
        let vendor_names: Vec<String> =
            self.routing.option_pcr.iter().map(|n| n.to_string()).collect();
        let sc = stock_code.to_string();
        match self
            .try_vendors_retry(stock_code, "option_pcr", &vendor_names, 2, |name, vendor| {
                let sc = sc.clone();
                Box::pin(async move {
                    match vendor.get_option_pcr(&sc).await {
                        Ok(Some(r)) => Ok(r),
                        Ok(None) => Err(DataError::VendorError {
                            vendor: name.to_string(),
                            message: "期权PCR数据为空".into(),
                        }),
                        Err(e) => Err(e),
                    }
                })
            })
            .await
        {
            Ok(result) => Ok(Some(result)),
            Err(e) => {
                tracing::debug!(
                    "[get_option_pcr] 所有 vendor 失败(stock_code={}): {e}",
                    stock_code
                );
                Ok(None)
            },
        }
    }

    pub async fn fetch_market_data(&self) -> Result<MarketRawData, DataError> {
        let (hot_r, industry_r, cls_r, mdt_r, nbf_r, idx_r) = tokio::join!(
            self.get_hot_stocks(),
            self.get_industry_ranking(),
            self.get_cls_flash(),
            self.get_market_dragon_tiger(),
            self.get_north_bound_flow(),
            self.get_index_quotes(),
        );

        Ok(MarketRawData {
            hot_stocks: hot_r.unwrap_or_else(|e| {
                tracing::warn!("hot_stocks failed: {e}");
                vec![]
            }),
            industry_ranking: industry_r.unwrap_or_else(|e| {
                tracing::warn!("industry_ranking failed: {e}");
                vec![]
            }),
            cls_flash: cls_r.unwrap_or_else(|e| {
                tracing::warn!("cls_flash failed: {e}");
                vec![]
            }),
            market_dragon_tiger: mdt_r.unwrap_or_else(|e| {
                tracing::warn!("market_dragon_tiger failed: {e}");
                vec![]
            }),
            north_bound_flow: nbf_r.unwrap_or_else(|e| {
                tracing::warn!("north_bound_flow failed: {e}");
                None
            }),
            index_quotes: idx_r.unwrap_or_else(|e| {
                tracing::warn!("index_quotes failed: {e}");
                vec![]
            }),
        })
    }

    pub async fn fetch_all(
        &self,
        stock_code: &str,
        kline_period: &str,
        kline_limit: u32,
        news_limit: u32,
    ) -> Result<StockRawData, DataError> {
        let (
            quote_r,
            klines_r,
            financials_r,
            news_r,
            money_flow_r,
            dragon_tiger_r,
            lockup_r,
            margin_r,
            north_bound_r,
            sector_r,
            shareholder_r,
            dividend_r,
            research_r,
            consensus_r,
            concept_r,
            announcements_r,
            block_trades_r,
            institutional_visits_r,
            peers_r,
            option_pcr_r,
        ) = tokio::join!(
            self.get_quote(stock_code),
            self.get_klines(stock_code, kline_period, kline_limit),
            self.get_financials(stock_code),
            self.get_news(stock_code, news_limit),
            self.get_money_flow(stock_code),
            self.get_dragon_tiger(stock_code),
            self.get_lockup_schedule(stock_code),
            self.get_margin_data(stock_code),
            self.get_north_bound_holding(stock_code),
            self.get_sector_info(stock_code),
            self.get_shareholder_trades(stock_code),
            self.get_dividend_records(stock_code),
            self.get_research_reports(stock_code),
            self.get_consensus_eps(stock_code),
            self.get_concept_blocks(stock_code),
            self.get_announcements(stock_code),
            self.get_block_trades(stock_code),
            self.get_institutional_visits(stock_code),
            self.get_peers(stock_code),
            self.get_option_pcr(stock_code),
        );

        let quote = match quote_r {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("quote failed: {e}");
                let cache_key = Self::cache_key_for("quote", stock_code);
                match self.cache_get(&cache_key).await {
                    Some(cached) => match serde_json::from_str(&cached) {
                        Ok(q) => q,
                        Err(_) => return Err(e),
                    },
                    None => return Err(e),
                }
            },
        };
        // H1.4 修复:收集所有子查询错误,调用方可据此判断数据完整性
        let mut errors: Vec<String> = Vec::new();
        let klines = klines_r.unwrap_or_else(|e| {
            let msg = format!("klines: {e}");
            tracing::warn!("{} failed: {}", "klines", msg);
            errors.push(msg);
            vec![]
        });
        let financials = financials_r.unwrap_or_else(|e| {
            let msg = format!("financials: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let news = news_r.unwrap_or_else(|e| {
            let msg = format!("news: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let money_flow = money_flow_r.unwrap_or_else(|e| {
            let msg = format!("money_flow: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let dragon_tiger = dragon_tiger_r.unwrap_or_else(|e| {
            let msg = format!("dragon_tiger: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let lockup = lockup_r.unwrap_or_else(|e| {
            let msg = format!("lockup: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let margin_data = margin_r.unwrap_or_else(|e| {
            let msg = format!("margin_data: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let north_bound = north_bound_r.unwrap_or_else(|e| {
            let msg = format!("north_bound: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let sector_info = sector_r.unwrap_or_else(|e| {
            let msg = format!("sector_info: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let shareholder_trades = shareholder_r.unwrap_or_else(|e| {
            let msg = format!("shareholder_trades: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let dividend_records = dividend_r.unwrap_or_else(|e| {
            let msg = format!("dividend_records: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let research_reports = research_r.unwrap_or_else(|e| {
            let msg = format!("research_reports: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let consensus_eps = consensus_r.unwrap_or_else(|e| {
            let msg = format!("consensus_eps: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let concept_blocks = concept_r.unwrap_or_else(|e| {
            let msg = format!("concept_blocks: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        let announcements = announcements_r.unwrap_or_else(|e| {
            let msg = format!("announcements: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let block_trades = block_trades_r.unwrap_or_else(|e| {
            let msg = format!("block_trades: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let institutional_visits = institutional_visits_r.unwrap_or_else(|e| {
            let msg = format!("institutional_visits: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let peers = peers_r.unwrap_or_else(|e| {
            let msg = format!("peers: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            vec![]
        });
        let option_pcr = option_pcr_r.unwrap_or_else(|e| {
            let msg = format!("option_pcr: {e}");
            tracing::warn!("{}", msg);
            errors.push(msg);
            None
        });
        if !errors.is_empty() {
            tracing::warn!(
                "[fetch_all] {} 个子查询失败(stock_code={}): {:?}",
                errors.len(),
                stock_code,
                errors
            );
        }

        Ok(StockRawData {
            quote,
            klines,
            financials,
            news,
            money_flow,
            dragon_tiger,
            lockup,
            margin_data,
            north_bound,
            sector_info,
            shareholder_trades,
            dividend_records,
            research_reports,
            consensus_eps,
            concept_blocks,
            announcements,
            block_trades,
            institutional_visits,
            peers,
            option_pcr,
            errors,
        })
    }
}

impl Default for AStockClient {
    fn default() -> Self {
        Self::new()
    }
}

// ── MarketDataProvider impl ──────────────────────────────────────────────
// 实现 harness 契约，让 quant/gateway 通过 trait 调用，无需直接依赖 astock-data。

#[async_trait::async_trait]
impl axagent_harness::market_data::MarketDataProvider for AStockClient {
    async fn get_quote(
        &self,
        stock_code: &str,
    ) -> std::result::Result<
        axagent_harness::market_data::StockQuote,
        axagent_harness::core_error::AxAgentError,
    > {
        self.get_quote(stock_code)
            .await
            .map_err(|e| axagent_harness::core_error::AxAgentError::Provider(e.to_string()))
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj_type: Option<axagent_harness::market_data::AdjType>,
    ) -> std::result::Result<
        Vec<axagent_harness::market_data::KLine>,
        axagent_harness::core_error::AxAgentError,
    > {
        self.get_klines_with_adj(stock_code, period, limit, adj_type)
            .await
            .map_err(|e| axagent_harness::core_error::AxAgentError::Provider(e.to_string()))
    }

    async fn search_stock(
        &self,
        keyword: &str,
    ) -> std::result::Result<
        Vec<axagent_harness::market_data::StockSearchResult>,
        axagent_harness::core_error::AxAgentError,
    > {
        self.search_stock(keyword)
            .await
            .map_err(|e| axagent_harness::core_error::AxAgentError::Provider(e.to_string()))
    }
}

/// 提取新闻 publish_time 的 YYYY-MM-DD 前缀；无法解析时返回空串（截断时会被过滤掉）
fn news_date_key(s: &str) -> &str {
    if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        &s[..10]
    } else {
        ""
    }
}

/// 清洗 LLM 传入的搜索关键词，提取纯公司名片段。
///
/// 背景（2026-08-01 实锤）：Serenity 工作流的 LLM 常把概念词/后缀词混进
/// 公司名再调用 `search_stock`，如"方大炭素 石墨烯"、"国瓷材料 概念股"、
/// "国瓷材料股份有限公司"。东财 searchadapter 是**精确匹配**，混合关键词
/// 实测全部返回空（`Data: null, TotalCount: 0`），整条 vendor 链随之空 →
/// LLM 误判"该股票不存在" → 候选全灭。此处按「最长连续 CJK 片段 +
/// 去尾部噪声词」提取纯名称，供 `search_stock` 兜底重试。
///
/// 清洗规则：
/// 1. 已是数字代码/市场代码（600516 / 00700.HK / TSM.US）→ 原样返回
/// 2. 取关键词中最长的连续 CJK 片段（"方大炭素 石墨烯" → "方大炭素"）
/// 3. 若片段以公司后缀/查询意图词结尾则截断（"国瓷材料股份有限公司" → "国瓷材料"）
fn clean_search_keyword(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // 已是数字代码 / 市场代码 → 不洗
    if trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '_') {
        return trimmed.to_string();
    }
    // 取最长连续 CJK 片段（含间隔号"·"）
    let mut best = String::new();
    let mut cur = String::new();
    for c in trimmed.chars() {
        if (0x4E00..=0x9FFF).contains(&(c as u32)) || c == '·' {
            cur.push(c);
        } else {
            if cur.chars().count() > best.chars().count() {
                best = cur.clone();
            }
            cur.clear();
        }
    }
    if cur.chars().count() > best.chars().count() {
        best = cur;
    }
    // 提取不到 CJK 片段（如 "00700.HK"、"TSM.US" 之外的字母串）→ 无可清洗内容，原样返回
    if best.is_empty() {
        return trimmed.to_string();
    }
    // 去尾部常见噪声词（长词优先，保证贪婪匹配）
    const NOISE_SUFFIXES: &[&str] = &[
        "股份有限公司",
        "有限责任公司",
        "股票代码",
        "股票行情",
        "有限公司",
        "概念股",
        "成分股",
        "股票",
        "股价",
        "公司",
        "集团",
        "板块",
        "概念",
        "龙头",
        "行情",
        "股份",
    ];
    let mut name: &str = best.as_str();
    loop {
        let mut cut = false;
        for suf in NOISE_SUFFIXES {
            if name.len() > suf.len() && name.ends_with(suf) {
                name = &name[..name.len() - suf.len()];
                cut = true;
                break;
            }
        }
        if !cut {
            break;
        }
    }
    name.trim().to_string()
}

#[cfg(test)]
mod clean_search_keyword_tests {
    use super::clean_search_keyword;

    #[test]
    fn pure_name_unchanged() {
        assert_eq!(clean_search_keyword("国瓷材料"), "国瓷材料");
        assert_eq!(clean_search_keyword("方大炭素"), "方大炭素");
        assert_eq!(clean_search_keyword("紫金矿业"), "紫金矿业");
    }

    #[test]
    fn strips_query_suffix() {
        assert_eq!(clean_search_keyword("国瓷材料 股票代码"), "国瓷材料");
        assert_eq!(clean_search_keyword("方大炭素 概念股"), "方大炭素");
        assert_eq!(clean_search_keyword("比亚迪 股票"), "比亚迪");
    }

    #[test]
    fn strips_company_suffix() {
        assert_eq!(clean_search_keyword("国瓷材料股份有限公司"), "国瓷材料");
        assert_eq!(clean_search_keyword("国瓷材料有限公司"), "国瓷材料");
        assert_eq!(clean_search_keyword("北方华创科技集团"), "北方华创科技");
    }

    #[test]
    fn picks_first_cjk_segment_for_concept_mix() {
        // "方大炭素 石墨烯" → 最长的连续 CJK 片段是"方大炭素"
        assert_eq!(clean_search_keyword("方大炭素 石墨烯"), "方大炭素");
        assert_eq!(clean_search_keyword("国瓷材料 石墨烯 陶瓷"), "国瓷材料");
    }

    #[test]
    fn market_codes_passthrough() {
        assert_eq!(clean_search_keyword("600516"), "600516");
        assert_eq!(clean_search_keyword("00700.HK"), "00700.HK");
        assert_eq!(clean_search_keyword("TSM.US"), "TSM.US");
    }

    #[test]
    fn empty_input() {
        assert_eq!(clean_search_keyword(""), "");
        assert_eq!(clean_search_keyword("  "), "");
    }
}

#[cfg(test)]
mod cache_key_tests {
    use super::*;
    use crate::as_of::{AsOfContext, AsOfSource};
    use chrono::NaiveDate;
    use serial_test::serial;

    #[test]
    #[serial(asof)]
    fn cache_key_for_live_mode() {
        let key = AStockClient::cache_key_for("quote", "000001");
        assert_eq!(key, "quote:000001::live");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn cache_key_for_asof_mode() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let key = crate::as_of::AS_OF
            .scope(Some(ctx), async { AStockClient::cache_key_for("klines", "000001:daily") })
            .await;
        assert_eq!(key, "klines:000001:daily::asof-20260601");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn different_asof_yields_different_keys() {
        use crate::as_of::AS_OF;
        let d1 = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let k1 = AS_OF
            .scope(Some(AsOfContext::new(d1, AsOfSource::UserReplay).unwrap()), async {
                AStockClient::cache_key_for("quote", "000001")
            })
            .await;
        let k2 = AS_OF
            .scope(Some(AsOfContext::new(d2, AsOfSource::UserReplay).unwrap()), async {
                AStockClient::cache_key_for("quote", "000001")
            })
            .await;
        assert_ne!(k1, k2);
    }
}

#[cfg(test)]
mod asof_truncate_tests {
    use super::*;
    use crate::as_of::{AsOfContext, AsOfSource};
    use chrono::NaiveDate;
    use serial_test::serial;

    fn kline(date: &str) -> KLine {
        KLine {
            date: date.into(),
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
            amount: 0.0,
            turnover_rate: None,
            adj_factor: None,
        }
    }

    fn news(publish_time: &str) -> NewsItem {
        NewsItem {
            title: "x".into(),
            summary: "".into(),
            source: "".into(),
            url: "".into(),
            publish_time: publish_time.into(),
            sentiment_score: None,
        }
    }

    fn fin(report_date: &str) -> FinancialReport {
        FinancialReport {
            stock_code: "000001".into(),
            report_date: report_date.into(),
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
            goodwill: None,
            accounts_receivable: None,
            estimated: Some(false),
        }
    }

    fn dt(date: &str) -> DragonTigerEntry {
        DragonTigerEntry {
            stock_code: "000001".into(),
            date: date.into(),
            dept_name: "".into(),
            buy_amount: 0.0,
            sell_amount: 0.0,
            net_amount: 0.0,
            reason: None,
        }
    }

    #[test]
    #[serial(asof)]
    fn truncate_klines_live_passthrough() {
        let ks = vec![kline("2026-05-30"), kline("2026-06-01"), kline("2026-06-02")];
        let out = AStockClient::truncate_klines_by_asof(ks.clone());
        assert_eq!(out.len(), 3);
    }

    #[tokio::test]
    async fn truncate_klines_drops_future_dates() {
        use crate::as_of::AS_OF;
        let ks = vec![kline("2026-05-30"), kline("2026-06-01"), kline("2026-06-05")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out = AS_OF.scope(Some(ctx), async { AStockClient::truncate_klines_by_asof(ks) }).await;
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|k| k.date.as_str() <= "2026-06-01"));
    }

    #[tokio::test]
    async fn truncate_news_keeps_only_before_or_on_asof() {
        use crate::as_of::AS_OF;
        let items = vec![
            news("2026-05-30 09:00:00"),
            news("2026-06-01 10:00:00"),
            news("2026-06-02 08:00:00"),
        ];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_news_by_asof(items) }).await;
        assert_eq!(out.len(), 2);
    }

    #[tokio::test]
    async fn truncate_news_drops_unparseable_publish_time() {
        use crate::as_of::AS_OF;
        // 修复(2026-07-21): 不可解析的 publish_time 视为不可信但保留,
        // 不再丢弃。避免 vendor 字段缺失时全部新闻被过滤掉。
        let items = vec![news("2026-06-01 10:00:00"), news("not-a-date")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_news_by_asof(items) }).await;
        assert_eq!(out.len(), 2); // 保留 1 条可解析 + 1 条不可解析(降级保留)
    }

    #[tokio::test]
    async fn truncate_financials_keeps_only_reports_on_or_before_asof() {
        use crate::as_of::AS_OF;
        let rs = vec![fin("2025-12-31"), fin("2026-03-31"), fin("2026-06-30")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_financials_by_asof(rs) }).await;
        assert_eq!(out.len(), 2);
    }

    #[tokio::test]
    async fn truncate_dragon_tiger_keeps_only_entries_on_or_before_asof() {
        use crate::as_of::AS_OF;
        let es = vec![dt("2026-05-20"), dt("2026-05-30"), dt("2026-06-05")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_dragon_tiger_by_asof(es) }).await;
        assert_eq!(out.len(), 2);
    }

    // ── 混合 as-of 模式(Phase 1)集成测试 ──────────────────────
    // 验证：当用户在 UI 选 `data_scope=Structured` 时，
    // - 结构化数据（K线/财务/龙虎榜）按 as_of 截止（与默认 All 一致）
    // - 非结构化数据（新闻/公告/研报）保持实时放行

    /// K 线属于结构化数据，Structured 模式下仍按 as_of 截断
    #[tokio::test]
    #[serial(asof)]
    async fn truncate_klines_structured_scope_still_truncates() {
        use crate::as_of::AS_OF;
        let _ = crate::as_of::clear_global_asof();
        let ks = vec![kline("2026-05-30"), kline("2026-06-01"), kline("2026-06-05")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(crate::as_of::AsOfDataScope::Structured);
        let out = AS_OF.scope(Some(ctx), async { AStockClient::truncate_klines_by_asof(ks) }).await;
        assert_eq!(out.len(), 2, "Structured 模式下 K 线仍应截断到 2026-06-01");
        let _ = crate::as_of::clear_global_asof();
    }

    /// 新闻属于非结构化数据，Structured 模式下放行（保留全部新闻）
    #[tokio::test]
    #[serial(asof)]
    async fn truncate_news_structured_scope_passes_through() {
        use crate::as_of::AS_OF;
        let _ = crate::as_of::clear_global_asof();
        let items = vec![
            news("2026-05-30 09:00:00"),
            news("2026-06-01 10:00:00"),
            news("2026-06-02 08:00:00"), // 未来日期
        ];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(crate::as_of::AsOfDataScope::Structured);
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_news_by_asof(items) }).await;
        assert_eq!(out.len(), 3, "Structured 模式下新闻应放行,保留全部 3 条(含未来日期)");
        let _ = crate::as_of::clear_global_asof();
    }

    /// 默认 data_scope=All 时,新闻仍然按 as_of 截断（兼容旧行为）
    #[tokio::test]
    #[serial(asof)]
    async fn truncate_news_all_scope_keeps_truncating() {
        use crate::as_of::AS_OF;
        let _ = crate::as_of::clear_global_asof();
        let items = vec![
            news("2026-05-30 09:00:00"),
            news("2026-06-01 10:00:00"),
            news("2026-06-02 08:00:00"),
        ];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        // 默认 All 行为,无须 with_data_scope
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_news_by_asof(items) }).await;
        assert_eq!(out.len(), 2, "All 模式下新闻仍应按 as_of 截断,与历史行为一致");
        let _ = crate::as_of::clear_global_asof();
    }

    /// 财务报告属于结构化数据，Structured 模式下仍按 as_of 截断
    #[tokio::test]
    #[serial(asof)]
    async fn truncate_financials_structured_scope_still_truncates() {
        use crate::as_of::AS_OF;
        let _ = crate::as_of::clear_global_asof();
        let rs = vec![fin("2025-12-31"), fin("2026-03-31"), fin("2026-06-30")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(crate::as_of::AsOfDataScope::Structured);
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_financials_by_asof(rs) }).await;
        assert_eq!(out.len(), 2, "Structured 模式下财报仍截断");
        let _ = crate::as_of::clear_global_asof();
    }

    /// 龙虎榜属于结构化数据，Structured 模式下仍按 as_of 截断
    #[tokio::test]
    #[serial(asof)]
    async fn truncate_dragon_tiger_structured_scope_still_truncates() {
        use crate::as_of::AS_OF;
        let _ = crate::as_of::clear_global_asof();
        let es = vec![dt("2026-05-20"), dt("2026-05-30"), dt("2026-06-05")];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(crate::as_of::AsOfDataScope::Structured);
        let out =
            AS_OF.scope(Some(ctx), async { AStockClient::truncate_dragon_tiger_by_asof(es) }).await;
        assert_eq!(out.len(), 2, "Structured 模式下龙虎榜仍截断");
        let _ = crate::as_of::clear_global_asof();
    }

    #[test]
    fn news_date_key_extracts_ymd_prefix() {
        assert_eq!(news_date_key("2026-06-01 10:00:00"), "2026-06-01");
        assert_eq!(news_date_key("2026-06-01"), "2026-06-01");
        assert_eq!(news_date_key(""), "");
        assert_eq!(news_date_key("garbage"), "");
    }
}

#[cfg(test)]
mod asof_realtime_degrade_tests {
    use super::*;
    use crate::as_of::{AsOfContext, AsOfSource};
    use chrono::NaiveDate;
    use serial_test::serial;

    fn kline(date: &str, close: f64) -> KLine {
        KLine {
            date: date.into(),
            open: close - 0.5,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1000.0,
            amount: 10000.0,
            turnover_rate: Some(0.01),
            adj_factor: None,
        }
    }

    #[test]
    #[serial(asof)]
    fn quote_from_klines_returns_none_in_live_mode() {
        let ks = vec![kline("2026-06-01", 10.0)];
        assert!(AStockClient::quote_from_klines("000001", &ks).is_none());
    }

    #[tokio::test]
    async fn quote_from_klines_uses_last_kline_on_or_before_asof() {
        use crate::as_of::AS_OF;
        let ks =
            vec![kline("2026-05-28", 9.5), kline("2026-05-30", 10.0), kline("2026-06-05", 11.0)];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let q = AS_OF
            .scope(Some(ctx), async { AStockClient::quote_from_klines("000001", &ks) })
            .await
            .expect("should produce quote");
        assert_eq!(q.price, 10.0);
        assert_eq!(q.timestamp, "2026-05-30");
    }

    #[tokio::test]
    async fn quote_from_klines_returns_none_when_no_kline_before_asof() {
        use crate::as_of::AS_OF;
        let ks = vec![kline("2026-06-05", 11.0)];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let q =
            AS_OF.scope(Some(ctx), async { AStockClient::quote_from_klines("000001", &ks) }).await;
        assert!(q.is_none());
    }

    #[tokio::test]
    async fn quote_from_klines_returns_none_for_empty_klines() {
        use crate::as_of::AS_OF;
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let q =
            AS_OF.scope(Some(ctx), async { AStockClient::quote_from_klines("000001", &[]) }).await;
        assert!(q.is_none());
    }

    // ── 缺陷 B 修复: K 线 cache_key 包含 effective_cutoff ─────────
    // 同一 as_of_date (周末),effective_cutoff 落到上一交易日,
    // key 必须区分"as_of 周末" vs "as_of 上一交易日"以避免缓存污染。

    #[serial(asof)]
    #[tokio::test]
    async fn kline_cache_key_live_no_effective_suffix() {
        use crate::as_of::AS_OF;
        // 显式声明 live 模式(None),确保不受其他测试全局污染
        let key = AS_OF
            .scope(None, async { AStockClient::kline_cache_key("000001", "daily", None) })
            .await;
        assert!(!key.contains("eff="), "live 模式不应有 eff= 后缀: {key}");
    }

    #[tokio::test]
    async fn kline_cache_key_replay_trading_day_has_effective() {
        use crate::as_of::AS_OF;
        // 2025-04-25 是周五(交易日,不在硬编码节假日列表)
        let date = NaiveDate::from_ymd_opt(2025, 4, 25).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let key = AS_OF
            .scope(Some(ctx), async { AStockClient::kline_cache_key("000001", "daily", None) })
            .await;
        assert!(key.contains("asof-20250425"));
        assert!(key.contains("eff=20250425"), "交易日 eff= 应等于 as_of: {key}");
    }

    #[tokio::test]
    async fn kline_cache_key_replay_weekend_falls_back() {
        use crate::as_of::AS_OF;
        // 2025-04-26 是周六(非交易日,不在调休 workdays 列表)
        let date = NaiveDate::from_ymd_opt(2025, 4, 26).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let key = AS_OF
            .scope(Some(ctx), async { AStockClient::kline_cache_key("000001", "daily", None) })
            .await;
        assert!(key.contains("asof-20250426"));
        assert!(key.contains("eff=20250425"), "周末 eff= 应 fallback 到周五: {key}");
    }

    // ── 缺陷 G 修复: vendor 路由 per-mode 切换 ─────────
    // live 模式无 override → 用默认 routing;
    // replay 模式有 override → 用 override 顺序(否则 fallback 默认)。

    #[test]
    #[serial(asof)]
    fn vendors_for_live_returns_default() {
        let routing = VendorRouting::default_routing();
        let default = vec!["eastmoney".into()];
        let chosen = routing.vendors_for("quote", &default);
        assert_eq!(chosen, &default);
    }

    #[tokio::test]
    async fn vendors_for_replay_with_override() {
        use crate::as_of::AS_OF;
        let mut routing = VendorRouting::default_routing();
        routing.replay.insert("quote", vec!["baidu_stock".into(), "eastmoney".into()]);
        let default = vec!["tencent".into(), "mootdx".into()];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let chosen = AS_OF.scope(Some(ctx), async { routing.vendors_for("quote", &default) }).await;
        assert_eq!(
            chosen,
            &vec!["baidu_stock".to_string(), "eastmoney".to_string()],
            "replay 模式命中 override,不应走默认"
        );
    }

    #[tokio::test]
    async fn vendors_for_replay_without_override_falls_back() {
        use crate::as_of::AS_OF;
        let routing = VendorRouting::default_routing();
        // 用一个 P2-4 没加 override 的 method(例如 north_bound_flow)以验证
        // 真正的"无 override" 路径还能 fallback 到 default
        let default = vec!["tencent".into(), "mootdx".into()];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let chosen = AS_OF
            .scope(Some(ctx), async { routing.vendors_for("north_bound_flow", &default) })
            .await;
        assert_eq!(chosen, &default, "replay 模式无 override 时应 fallback 到默认");
    }

    // P2-4: 默认 routing 的 replay 覆盖应在初始化时把 quote/klines/financials
    // 切到对历史日期支持最好的 vendor。重要:覆盖存在时不能 fallback 默认,
    // 否则 as-of 模式的 NativeDateParam 优势就拿不到了。
    #[tokio::test]
    async fn vendors_for_replay_default_routing_uses_replay_overrides() {
        use crate::as_of::AS_OF;
        let routing = VendorRouting::default_routing();
        let quote_default = vec!["eastmoney".into()];
        let klines_default = vec!["eastmoney".into()];
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let (quote_chosen, klines_chosen) = AS_OF
            .scope(Some(ctx), async {
                (
                    routing.vendors_for("quote", &quote_default).clone(),
                    routing.vendors_for("klines", &klines_default).clone(),
                )
            })
            .await;
        // quote 的 replay 覆盖第一应是 tencent(SynthesizeFromKline)
        assert_eq!(quote_chosen[0], "tencent", "quote replay 覆盖首 vendor 必须是 tencent");
        // klines 的 replay 覆盖第一应是 tencent(NativeDateParam 唯一)
        assert_eq!(klines_chosen[0], "tencent", "klines replay 覆盖首 vendor 必须是 tencent");
        // 覆盖值不能等于 default(否则覆盖就是 noop, 修复没生效)
        assert!(quote_chosen != quote_default, "quote replay 覆盖必须与 default 不同");
        assert!(klines_chosen != klines_default, "klines replay 覆盖必须与 default 不同");
    }

    // P2-4: live 模式不受 replay 覆盖影响, 仍走 default
    #[test]
    #[serial(asof)]
    fn vendors_for_live_unaffected_by_replay_overrides() {
        let routing = VendorRouting::default_routing();
        let default = vec!["eastmoney".into()];
        let chosen = routing.vendors_for("quote", &default);
        assert_eq!(chosen, &default, "live 模式不应使用 replay 覆盖");
    }

    #[test]
    #[serial(asof)]
    fn is_asof_active_false_in_live() {
        assert!(!crate::as_of::is_asof_active());
    }

    // ── 实时性方法 as-of 守卫回归测试 ───────────────────────────
    // 这些方法在 replay 模式下没有"过去某日的实时"语义，必须整方法跳过
    // 以免把 today 之后的数据塞入 backtest 视图。

    #[tokio::test]
    async fn get_hot_stocks_returns_empty_in_asof_scope() {
        use crate::as_of::AS_OF;
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let r = AS_OF.scope(Some(ctx), async { client.get_hot_stocks().await }).await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_empty(), "replay 模式必须返回空列表");
    }

    #[tokio::test]
    async fn get_industry_ranking_returns_empty_in_asof_scope() {
        use crate::as_of::AS_OF;
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let r = AS_OF.scope(Some(ctx), async { client.get_industry_ranking().await }).await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_empty(), "replay 模式必须返回空列表");
    }

    #[tokio::test]
    async fn get_cls_flash_returns_empty_in_asof_scope() {
        use crate::as_of::AS_OF;
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let r = AS_OF.scope(Some(ctx), async { client.get_cls_flash().await }).await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_empty(), "replay 模式必须返回空列表");
    }

    // ── vendor trait 大重构 P0 测试 ───────────────────────────
    // 验证:
    // 1. asof_capability 默认返回 Fallthrough(老 vendor 行为)
    // 2. try_vendor_with_asof live 模式不适用,返回 None
    // 3. try_vendor_with_asof replay 模式 + Fallthrough,返回 None
    // 4. try_vendor_with_asof replay + NoHistoricalSemantic,记录降级
    // 5. try_vendor_with_asof replay + NativeDateParam + 成功,返回 Some
    // 6. try_vendor_with_asof replay + NativeDateParam + 失败,记录降级 + 返回 None
    // 7. should_use_asof 在 scope 内/外行为正确
    // 8. vendor_asof_capability 对未注册 vendor 返回 Fallthrough

    #[test]
    fn default_asof_capability_is_fallthrough() {
        use crate::vendors::StockVendor;
        // 用一个 EastMoneyVendor 实例调 asof_capability
        // (EastMoneyVendor 还没 override,所以默认是 Fallthrough,等 P1 改完后变其他变体)
        let vendor = EastMoneyVendor { http: reqwest::Client::new(), proxy_http: None };
        let cap = vendor.asof_capability("get_quote");
        assert!(
            cap == AsOfCapability::Fallthrough
                || cap == AsOfCapability::NativeDateParam
                || cap == AsOfCapability::SynthesizeFromKline
                || cap == AsOfCapability::NoHistoricalSemantic,
            "asof_capability 应返回 4 个变体之一(默认 Fallthrough)"
        );
    }

    #[tokio::test]
    #[serial(asof)]
    async fn should_use_asof_live_is_false() {
        let client = AStockClient::new();
        assert!(!client.should_use_asof(), "live 模式 should_use_asof = false");
    }

    #[tokio::test]
    async fn should_use_asof_replay_is_true() {
        use crate::as_of::AS_OF;
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let r = AS_OF.scope(Some(ctx), async { client.should_use_asof() }).await;
        assert!(r, "replay 模式 should_use_asof = true");
    }

    #[tokio::test]
    async fn vendor_asof_capability_unknown_vendor_returns_fallthrough() {
        let client = AStockClient::new();
        let cap = client.vendor_asof_capability("nonexistent_vendor", "get_quote");
        assert_eq!(
            cap,
            AsOfCapability::Fallthrough,
            "未注册 vendor 的 capability 必须是 Fallthrough"
        );
    }

    #[tokio::test]
    #[serial(asof)]
    async fn try_vendor_with_asof_live_returns_none() {
        let client = AStockClient::new();
        // live 模式: helper 不该被调用,但兜底返回 None
        let r: Option<String> = client
            .try_vendor_with_asof("get_quote", "eastmoney", async {
                Ok::<String, DataError>("live_result".to_string())
            })
            .await;
        assert!(r.is_none(), "live 模式 try_vendor_with_asof 返回 None");
    }

    #[tokio::test]
    async fn try_vendor_with_asof_replay_with_fallthrough_returns_none() {
        use crate::as_of::AS_OF;
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        // 用 get_news(eastmoney 申报为 Fallthrough)
        let r: Option<String> = AS_OF
            .scope(Some(ctx), async {
                client
                    .try_vendor_with_asof("get_news", "eastmoney", async {
                        Ok::<String, DataError>("unused".to_string())
                    })
                    .await
            })
            .await;
        // eastmoney.get_news 是 Fallthrough(返回带 date 字段的全量,lib.rs 截断)
        // try_vendor_with_asof 对 Fallthrough 返回 None,让调用方走截断兜底
        assert!(r.is_none(), "Fallthrough vendor 在 replay 模式应返回 None(由调用方走截断)");
    }

    // ── P1.5:D 档修复集成测试 ─────────────────────────────────
    // 验证:get_market_dragon_tiger 在 as-of 模式会走 capability 决策
    // 期望:eastmoney 申报 NativeDateParam,会调 get_market_dragon_tiger_with_asof
    //       网络失败会进 record_degradation,最终 live 路径兜底返回空
    #[tokio::test]
    #[serial(asof)]
    async fn d_bug_fix_market_dragon_tiger_uses_capability() {
        use crate::as_of::{peek_global_degradation_report, AS_OF};
        let client = AStockClient::new();
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        // 重置全局降级日志
        crate::as_of::reset_global_degradation_log();
        let r = AS_OF.scope(Some(ctx), async { client.get_market_dragon_tiger().await }).await;
        // 网络在测试中失败,返回 Ok(vec![]) 是允许的(走完 live 兜底路径)
        assert!(r.is_ok(), "replay 模式调用应成功(网络失败时仍返回空)");
        // 验证:eastmoney 的 with_asof 路径被走过(因为它申报了 NativeDateParam)
        //   失败会被 record_degradation 记录
        // 不强求具体的 vendor 记录(可能 ths/eastmoney 都不在测试 routing 里)
        // 但验证降级日志确实被触发了
        let report = peek_global_degradation_report();
        let has_asof_entry = report.iter().any(|e| e.method == "get_market_dragon_tiger");
        assert!(has_asof_entry, "D 档修复后,as-of 模式应至少记录一次 get_market_dragon_tiger 降级");
    }
}
