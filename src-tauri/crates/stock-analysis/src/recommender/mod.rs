//! 智能荐股 — 顶层模块
//!
//! ## 架构
//! - 18 个子策略（5 风格 × 4 周期，超跌反弹仅短/中线）实现 [`strategy::RecommendStrategy`]
//! - [`recommend_stocks`] 是组合入口：seed pool → 候选过滤 → vendor 降级 → 并行扫描 → 去重 → 按风格分组
//! - 5 min 内存缓存（按 period 维度）
//!
//! ## 公开 API
//! - [`recommend_stocks`] — Tauri command 调用

pub mod indicators;
pub mod pool;
pub mod scoring;
pub mod strategies;
pub mod strategy;
pub mod types;

pub use strategy::{RecoContext, RecommendStrategy};
pub use types::{Period, RecoPick, RecoResponse, Style};

use axagent_astock_data::as_of;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant};
// `HashSet` 在 Rust 2024 edition 已加入 prelude

use crate::recommender::pool::SeedItem;
use crate::recommender::pool::{
    build_seed_pool, clear_cached_vendors, get_cached_vendors, liquidity_filter_and_truncate,
    load_enabled_vendors_from_template, set_cached_vendors,
};
use crate::recommender::scoring::{dedup_and_merge, group_by_style_and_trim};
use crate::recommender::strategies::{
    emit_synthetic_picks, CapitalStrategy, ReversionStrategy, SerenityStrategy, TrendStrategy,
    ValueStrategy, WatchlistStrategy,
};
use crate::recommender::strategy::PerCodeLocks;

/// Serenity 工作流产出的候选股，供 SerenityStrategy 读取
/// 自带 TTL（1h），避免 workflow 长时间不运行时使用过期候选。
static SERENITY_SEED: LazyLock<RwLock<Vec<SeedItem>>> = LazyLock::new(|| RwLock::new(Vec::new()));

/// Serenity 缓存写入时间戳，用于 TTL 检查
static SERENITY_CACHE_TIME: LazyLock<RwLock<Option<Instant>>> = LazyLock::new(|| RwLock::new(None));

/// Serenity 缓存 TTL：1 小时
const SERENITY_CACHE_TTL: Duration = Duration::from_secs(3600);

/// 检查 Serenity 缓存是否过期
fn is_serenity_cache_fresh() -> bool {
    if let Ok(guard) = SERENITY_CACHE_TIME.read() {
        if let Some(ts) = *guard {
            return ts.elapsed() < SERENITY_CACHE_TTL;
        }
    }
    false
}

/// 设置 serenity 候选种子（由 run_serenity_screening 命令写入）
pub fn set_serenity_seed(seed: Vec<SeedItem>) {
    tracing::info!("[serenity] 注入 {} 条候选到全局种子", seed.len());
    if let Ok(mut guard) = SERENITY_SEED.write() {
        *guard = seed;
    }
    if let Ok(mut guard) = SERENITY_CACHE_TIME.write() {
        *guard = Some(Instant::now());
    }
}

/// 读取 serenity 候选种子（缓存过期返回空）
pub fn get_serenity_seed() -> Vec<SeedItem> {
    if !is_serenity_cache_fresh() {
        return Vec::new();
    }
    SERENITY_SEED.read().map(|g| g.clone()).unwrap_or_default()
}

/// Serenity 候选全量数据缓存（serenity_score / catalysts / exit_signals / attention_metrics），
/// 从 workflow 输出传递到 SerenityStrategy::scan_one，使策略能感知上下文。
static SERENITY_CANDIDATE_CACHE: LazyLock<RwLock<HashMap<String, serde_json::Value>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 设置全量候选数据缓存（由 run_serenity_screening 写入）
pub fn set_serenity_candidate_cache(cache: HashMap<String, serde_json::Value>) {
    tracing::info!("[serenity] 注入 {} 条候选全量数据到缓存", cache.len());
    if let Ok(mut guard) = SERENITY_CANDIDATE_CACHE.write() {
        *guard = cache;
    }
    if let Ok(mut guard) = SERENITY_CACHE_TIME.write() {
        *guard = Some(Instant::now());
    }
}

/// 读取单个候选的全量数据（缓存过期返回 None）
pub fn get_serenity_candidate_detail(code: &str) -> Option<serde_json::Value> {
    if !is_serenity_cache_fresh() {
        return None;
    }
    SERENITY_CANDIDATE_CACHE
        .read()
        .ok()
        .and_then(|g| g.get(code).cloned())
}

/// 清空候选全量数据缓存（同时清时间戳）
pub fn clear_serenity_candidate_cache() {
    if let Ok(mut guard) = SERENITY_CANDIDATE_CACHE.write() {
        guard.clear();
    }
    if let Ok(mut guard) = SERENITY_CACHE_TIME.write() {
        *guard = None;
    }
    if let Ok(mut guard) = SERENITY_SEED.write() {
        guard.clear();
    }
}

/// 从 template_vars 中解析 "reco_strategy_weights" 权重表。
///
/// 这是复盘 → 进化（R1）的注入点：recommend_stocks 会按 (style, period)
/// 对每条 pick 的 confidence 乘上对应的权重，让近期胜率低的策略自然被降权。
///
/// 格式：`reco_strategy_weights` 是一个 JSON 对象：
/// ```json
/// { "trend_short": 1.2, "trend_ultra_short": 0.8, "value_mid": 0.8, ... }
/// ```
/// 缺失的 (style, period) 默认 1.0（不调整）。
pub(crate) fn parse_strategy_weights(
    template_vars: &[(String, serde_json::Value)],
) -> HashMap<(Style, Period), f64> {
    let mut out: HashMap<(Style, Period), f64> = HashMap::new();
    for (name, value) in template_vars {
        if name != "reco_strategy_weights" {
            continue;
        }
        let Some(obj) = value.as_object() else { continue };
        for (key, val) in obj {
            let Some(weight) = val.as_f64() else { continue };
            // key 形如 "trend_short" / "trend_ultra_short" / "value_mid" / "watchlist_long"
            // 用 splitn(2, '_') 处理 ultra_short 自带下划线的情况
            let mut parts = key.splitn(2, '_');
            let style = match parts.next() {
                Some("trend") => Style::Trend,
                Some("value") => Style::Value,
                Some("capital") => Style::Capital,
                Some("reversion") => Style::Reversion,
                Some("watchlist") => Style::Watchlist,
                Some("serenity") => Style::Serenity,
                _ => continue,
            };
            let period = match parts.next() {
                Some("ultra_short") => Period::UltraShort,
                Some("short") => Period::Short,
                Some("mid") => Period::Mid,
                Some("long") => Period::Long,
                _ => continue,
            };
            out.insert((style, period), weight.clamp(0.0, 2.0));
        }
    }
    out
}

// ── 缓存 ──
// 缓存 key 由 (period, as_of) 二元组组成，确保 live / replay 互相隔离。
// 之前为单条目 RwLock<Option>，同时请求不同 period 会互相驱逐。
// 改为 HashMap<(Period, String), (RecoResponse, Instant)> 后多 period 独立缓存。
// replay 模式后缀为 `asof-YYYYMMDD`，由 `as_of::cache_suffix()` 提供。

#[allow(clippy::type_complexity)]
static RESULT_CACHE: LazyLock<RwLock<HashMap<(Period, String), (RecoResponse, Instant)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 缓存 TTL：从 5 min 缩短到 60 s。
///
/// 荐股的 entry / target / stop_loss 是基于扫描时刻的现价算的，
/// 5 min 内股价可能已经走完整个目标区间（如图 莱伯泰科 5 min 内 38 → 48），
/// 这种"还显示着旧 target / entry 但现价远超"的推荐对用户毫无价值，
/// 还会误导用户按旧价位挂单。60 s 是为价格时效性与扫描性能做的折中。
const CACHE_TTL: Duration = Duration::from_secs(60);

// ── 推荐器配置（从 workflow template variables 读取） ──

/// 用户可配置的推荐器参数
struct RecoConfig {
    trend_enabled: bool,
    reversion_enabled: bool,
    value_enabled: bool,
    capital_enabled: bool,
    watchlist_enabled: bool,
    serenity_enabled: bool,
    min_confidence: u8,
}

impl Default for RecoConfig {
    fn default() -> Self {
        Self {
            trend_enabled: true,
            reversion_enabled: true,
            value_enabled: true,
            capital_enabled: true,
            watchlist_enabled: true,
            serenity_enabled: true,
            min_confidence: 0, // 0 = 不筛选
        }
    }
}

/// 从 template_vars 解析推荐器配置
///
/// `template_vars` 来自 workflow_template 表的 variables 字段，
/// 由前端 StockAnalysisConfigPanel 保存。变量名映射参见前端 getDefaultVariables()。
fn parse_reco_config(template_vars: &[(String, serde_json::Value)]) -> RecoConfig {
    let mut cfg = RecoConfig::default();
    for (name, value) in template_vars {
        let bool_val = || -> Option<bool> {
            value.as_bool().or_else(|| {
                value.as_str().and_then(|s| match s {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                })
            })
        };
        match name.as_str() {
            "reco_trend_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.trend_enabled = v;
                }
            },
            "reco_reversion_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.reversion_enabled = v;
                }
            },
            "reco_value_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.value_enabled = v;
                }
            },
            "reco_capital_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.capital_enabled = v;
                }
            },
            "reco_watchlist_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.watchlist_enabled = v;
                }
            },
            "reco_serenity_enabled" => {
                if let Some(v) = bool_val() {
                    cfg.serenity_enabled = v;
                }
            },
            "reco_min_confidence" => {
                if let Some(n) = value.as_f64() {
                    cfg.min_confidence = n.clamp(0.0, 100.0) as u8;
                }
            },
            _ => {},
        }
    }
    cfg
}

fn cache_get(period: Period) -> Option<RecoResponse> {
    let suffix = as_of::cache_suffix();
    let g = RESULT_CACHE.read().unwrap_or_else(|e| e.into_inner());
    g.get(&(period, suffix.clone()))
        .filter(|(_, ts)| ts.elapsed() < CACHE_TTL)
        .map(|(resp, _)| resp.clone())
}

fn cache_put(period: Period, resp: RecoResponse) {
    let suffix = as_of::cache_suffix();
    // 空结果也缓存（同 60s TTL），避免 vendor 故障时每次请求都全量扫描 vendor
    // 旧逻辑跳过了空结果的缓存，导致 vendor 故障期间形成
    // "请求→扫描→空→不缓存→请求"的死循环。
    let mut g = RESULT_CACHE.write().unwrap_or_else(|e| e.into_inner());
    g.insert((period, suffix), (resp, Instant::now()));
}

/// 主动失效缓存（设置页保存 vendor 后调用）
///
/// 同时清 vendor 启用集合缓存 + 推荐结果缓存 + Serenity 候选缓存，
/// 保证下次调用立即反映新 vendor 状态和干净的候选池
pub fn invalidate_cache() {
    let mut g = RESULT_CACHE.write().unwrap_or_else(|e| e.into_inner());
    g.clear();
    clear_cached_vendors();
    clear_serenity_candidate_cache();
}

// ── 公开入口 ──

/// 拉取指定周期的荐股结果
///
/// 调用方应保证传入的 `template_vars` 是当前 workflow template 的变量列表（vendor_* 启用状态）
///
/// `client` 用 `Arc` 包一层，因为后续并行任务（liquidity filter + 4 子策略）需要共享所有权
pub async fn recommend_stocks(
    client: Arc<AStockClient>,
    period: Period,
    template_vars: &[(String, serde_json::Value)],
) -> Result<RecoResponse, String> {
    if let Some(cached) = cache_get(period) {
        return Ok(cached);
    }

    // ── 前置健康探测 ──
    // 同时探测 quote 和 K 线，两者都失败时直接短路，避免 200 次无效 API 调用
    let quote_probe = client.get_quote("000001").await;
    let kline_probe = client.get_klines("000001", "daily", 5).await;

    let quote_ok = quote_probe.is_ok();
    let kline_ok = kline_probe.as_ref().is_ok_and(|k| k.len() >= 2);

    if !quote_ok && !kline_ok {
        let quote_err = quote_probe.err().map(|e| e.to_string());
        let kline_err = kline_probe.err().map(|e| e.to_string());
        let detail = format!(
            "行情与K线数据源均不可用 (quote: {}, klines: {})，跳过荐股扫描",
            quote_err.as_deref().unwrap_or("返回异常"),
            kline_err.as_deref().unwrap_or("返回不足(<2条)"),
        );
        tracing::warn!("[recommender] {detail}");
        let resp = RecoResponse {
            period,
            picks: std::collections::BTreeMap::new(),
            disabled_styles: vec![],
            degraded_styles: vec![],
            degraded_reasons: std::collections::HashMap::new(),
            generated_at: chrono::Utc::now().timestamp_millis(),
            raw_seed_pool_size: 0,
            as_of_date: None,
            mode: "live".to_string(),
            error_detail: Some(detail),
        };
        cache_put(period, resp.clone());
        return Ok(resp);
    }

    if !kline_ok {
        tracing::warn!("[recommender] K 线数据源当前不可用, 趋势/资金/超跌策略将跳过");
    }
    if !quote_ok {
        tracing::warn!("[recommender] 行情数据源当前不可用, Watchlist/Synthetic 兜底将跳过");
    }

    // 1. 预热 enabled-vendors 缓存（settings 页保存 vendor 时需要 invalid 这个缓存
    //    来刷新结果缓存；此处不依赖 enabled_vendors 做策略 gating）
    let _ = get_cached_vendors().unwrap_or_else(|| {
        let s = load_enabled_vendors_from_template(template_vars);
        set_cached_vendors(s.clone());
        s
    });

    // 2. seed pool + 流动性过滤
    let mut seed = build_seed_pool(&client).await;
    let raw_seed_pool_size = seed.len();
    tracing::info!("[recommender] period={:?}, raw_seed_pool_size={}", period, raw_seed_pool_size);
    // 保留 raw_seed 给 WatchlistStrategy（它只依赖 quote，不依赖 K 线）
    let raw_seed = seed.clone();
    seed = liquidity_filter_and_truncate(client.clone(), seed).await;
    tracing::info!(
        "[recommender] after liquidity filter, seed={}, raw_seed={}",
        seed.len(),
        raw_seed.len()
    );

    // 流动性过滤兜底：若 vendor 拿不到 60 日 K 线 / 全部不达标，过滤后池子可能
    // 全空。这种情况下 4 个主策略（trend/value/capital/reversion）会直接跳过。
    // Watchlist 用 raw_seed（不过流动性过滤），保证面板至少显示内容。
    // 注意：非 Watchlist 策略不再 fallback 到 raw_seed——raw_seed 未过滤流动性，
    // 可能导致大量无效 scan_one 调用（scan_one 内部判空后 return None）。
    if seed.is_empty() && !raw_seed.is_empty() {
        eprintln!(
            "[recommender] liquidity filter removed all {} stocks; \
             Watchlist will use raw seed, main strategies skip",
            raw_seed_pool_size
        );
    }

    // 注入 Serenity 候选（由 serenity-screening workflow 发现，写入 reco_picks，
    // 并通过 set_serenity_seed 同步到全局静态）
    let serenity_seed = get_serenity_seed();
    if !serenity_seed.is_empty() {
        for item in &serenity_seed {
            if !seed.contains(item) {
                seed.push(item.clone());
            }
        }
    }

    // 3. 选定该 period 下的所有子策略（不再做 vendor 禁用检查——
    //    原本要求 "enabled_vendors" 至少覆盖一个 required vendor，但生产环境
    //    workflow template 中往往 vendor_* 变量为空，导致 4 个 style 全 disabled。
    //    现在所有 style 都跑；data 真的取不到时该 style 自然返回空 picks，
    //    前端展示 "no data" 而非误报 "数据源未启用"）
    let all_strategies: Vec<Box<dyn RecommendStrategy>> = match period {
        Period::UltraShort => vec![
            Box::new(TrendStrategy::ultra_short()),
            Box::new(ValueStrategy::ultra_short()),
            Box::new(CapitalStrategy::ultra_short()),
            // ReversionStrategy ultra_short 不做（超跌反弹至少需要中线）
            Box::new(WatchlistStrategy::ultra_short()),
        ],
        Period::Short => vec![
            Box::new(TrendStrategy::short()),
            Box::new(ValueStrategy::short()),
            Box::new(CapitalStrategy::short()),
            Box::new(ReversionStrategy::short()),
            Box::new(WatchlistStrategy::short()),
        ],
        Period::Mid => vec![
            Box::new(TrendStrategy::mid()),
            Box::new(ValueStrategy::mid()),
            Box::new(CapitalStrategy::mid()),
            Box::new(ReversionStrategy::mid()),
            Box::new(SerenityStrategy::mid()),
            Box::new(WatchlistStrategy::mid()),
        ],
        Period::Long => vec![
            Box::new(TrendStrategy::long()),
            Box::new(ValueStrategy::long()),
            Box::new(CapitalStrategy::long()),
            // ReversionStrategy long 不做
            Box::new(SerenityStrategy::long()),
            Box::new(WatchlistStrategy::long()),
        ],
    };

    let reco_cfg = parse_reco_config(template_vars);
    // 复盘→进化：按 (style, period) 注入自适应权重
    let strategy_weights = parse_strategy_weights(template_vars);
    let mut disabled_styles_set: std::collections::HashSet<Style> =
        std::collections::HashSet::new();
    let enabled: Vec<Box<dyn RecommendStrategy>> = all_strategies
        .into_iter()
        .filter(|s| {
            let ok = match s.style() {
                Style::Trend => reco_cfg.trend_enabled,
                Style::Value => reco_cfg.value_enabled,
                Style::Capital => reco_cfg.capital_enabled,
                Style::Reversion => reco_cfg.reversion_enabled,
                Style::Watchlist => reco_cfg.watchlist_enabled,
                Style::Serenity => reco_cfg.serenity_enabled,
            };
            if !ok {
                disabled_styles_set.insert(s.style());
            }
            ok
        })
        .collect();

    // 4. 并行执行（per-code 互斥锁：不同 code 真正并行，同 code 4 策略间串行）
    let per_code_locks = PerCodeLocks::new();
    let ctx_pool = seed.clone();
    let raw_ctx_pool = raw_seed.clone();
    let vars_map: HashMap<String, Value> = template_vars.iter().cloned().collect();
    let mut futures = Vec::new();
    for s in enabled.iter() {
        // WatchlistStrategy 用 raw_seed（不过流动性过滤），
        // 这样 K 线全部拿不到时它仍能基于 raw pool 出 picks
        let use_raw = matches!(s.style(), Style::Watchlist);
        let s_ref: &dyn RecommendStrategy = s.as_ref();
        let client_ref: Arc<AStockClient> = client.clone();
        let lock_ref = per_code_locks.clone();
        let seed_ref = if use_raw {
            raw_ctx_pool.clone()
        } else {
            ctx_pool.clone()
        };
        let period_val = period;
        let vars_for_future = vars_map.clone();
        // 复盘→进化：该 (style, period) 当前的权重
        let style_weight = strategy_weights
            .get(&(s.style(), period))
            .copied()
            .unwrap_or(1.0);
        let fut = async move {
            let ctx = RecoContext {
                client: &client_ref,
                seed: &seed_ref,
                per_code_locks: lock_ref,
                period: period_val,
                vars: &vars_for_future,
            };
            let mut raw = s_ref.scan(&ctx).await?;
            // 应用自适应权重：confidence 与 position_pct 同步缩放
            for p in raw.iter_mut() {
                let new_conf = (p.confidence as f64 * style_weight).clamp(0.0, 100.0) as u8;
                p.confidence = new_conf;
                // 权重缩放后仓位重新经过 calc_position（含 period_factor），
                // 而非独立 scaling——确保缩放后的置信度与仓位参数一致
                p.position_pct = crate::recommender::scoring::calc_position(
                    p.position_pct / style_weight, // 还原 base
                    new_conf,
                    period_val,
                );
                // 信号质量校准（贝叶斯收缩 + 反身性）：
                //   三层框架 — 统计学(开仓勇气) × 反身性(持仓理性) × 贝叶斯(空仓定力)
                let quality_id = format!("{}_{}", p.style.as_str(), p.period.as_str());
                let (posterior_win_rate, sample_count, _prior) =
                    crate::backtest_strategy::bayesian_signal_quality(&quality_id, "neutral");
                let quality_factor = if sample_count >= 5 {
                    // 贝叶斯后验映射到 [0.7, 1.3]
                    ((posterior_win_rate / 0.50) - 0.20).clamp(0.7, 1.3)
                } else {
                    1.0
                };
                // 反身性：高风险策略信号噪声大 → 额外折扣
                let refl = match p.period {
                    crate::recommender::types::Period::UltraShort => 0.85, // 超短线博弈性强
                    _ => 1.0,
                };
                let combined = quality_factor * refl;
                let delta = if combined > 1.0 {
                    combined - 1.0
                } else {
                    1.0 - combined
                };
                if delta > 0.01 {
                    p.confidence = (p.confidence as f64 * combined).clamp(0.0, 100.0) as u8;
                    p.position_pct = (p.position_pct * combined).clamp(0.0, 100.0);
                }
            }
            Ok::<_, String>(raw)
        };
        futures.push(fut);
    }
    let results: Vec<Result<Vec<types::RecoPick>, String>> =
        futures::future::join_all(futures).await;

    // 5. 合并
    let mut all_picks: Vec<types::RecoPick> = Vec::new();
    for mut picks in results.into_iter().flatten() {
        all_picks.append(&mut picks);
    }
    // 诊断日志 V52：扫描完成后输出各策略的出票数
    {
        let mut counts: std::collections::BTreeMap<types::Style, usize> =
            std::collections::BTreeMap::new();
        for p in &all_picks {
            *counts.entry(p.style).or_insert(0) += 1;
        }
        tracing::info!(
            "[recommender] period={:?}, raw_picks={}, by_style={:?}",
            period,
            all_picks.len(),
            counts
        );
    }

    // 6. 去重
    dedup_and_merge(&mut all_picks);
    tracing::info!("[recommender] after dedup, picks={}", all_picks.len());

    // P3-1: drop picks whose numeric fields are NaN/inf — these would render as "NaN" in JSON.
    all_picks.retain(|p| {
        !p.price.is_nan()
            && !p.entry_low.is_nan()
            && !p.entry_high.is_nan()
            && !p.stop_loss.is_nan()
            && !p.target_price.is_nan()
            && !p.position_pct.is_nan()
    });
    tracing::info!("[recommender] after NaN filter, picks={}", all_picks.len());

    // P3-2: drop picks whose target_price is already below current price
    // (no upside left — the BUY thesis is dead). Frontend should also visually
    // flag this in case cache holds the stale pick, but the backend drop
    // makes sure new scans never emit these in the first place.
    all_picks.retain(|p| {
        // 允许 ≤0.5% 的轻微容差，避免价格微抖时被误杀
        p.target_price >= p.price * 0.995
    });
    tracing::info!("[recommender] after target<price filter, picks={}", all_picks.len());

    // P3-3: drop picks below user-configured min_confidence
    // (reco_min_confidence from StockAnalysisConfigPanel, 0 = no filter)
    if reco_cfg.min_confidence > 0 {
        tracing::info!(
            "[recommender] before confidence filter (min={}), picks={}",
            reco_cfg.min_confidence,
            all_picks.len()
        );
        all_picks.retain(|p| p.confidence >= reco_cfg.min_confidence);
        tracing::info!("[recommender] after confidence filter, picks={}", all_picks.len());
    }

    // 7. 按风格分组 + 限 10
    let mut by_style = group_by_style_and_trim(&mut all_picks, 10);
    for (style, picks) in &by_style {
        tracing::info!("[recommender] final bucket: style={:?}, picks={}", style, picks.len());
    }

    // 8. 数据稀疏兜底：5 个 style 桶里若有空，且 raw_seed 非空，用 get_quote 拉
    //    基础行情 emit 合成 picks 填入对应 style 桶。解决"只有 Watchlist 有 10 条，
    //    其他 4 个主风格全空"的问题（K 线 / 财务 / 资金数据全不可用时主策略都
    //    短路返回 None，但 watchlist 仅依赖 quote 仍能出 picks）。
    //
    //    合成 pick 的 reason 明确标注"信号缺失，按现价合成"，UI 可区分。
    //    跳过 dedup：兜底数据加在 by_style 之后，不与真实 pick 合并。
    if !raw_seed.is_empty() {
        for style in [
            Style::Trend,
            Style::Value,
            Style::Capital,
            Style::Reversion,
            Style::Watchlist,
            Style::Serenity,
        ] {
            // 用户已禁用的风格不补充合成 picks
            if disabled_styles_set.contains(&style) {
                continue;
            }
            let bucket_empty = by_style.get(&style).is_none_or(|v| v.is_empty());
            if !bucket_empty {
                continue;
            }
            let synthetic = emit_synthetic_picks(
                client.clone(),
                style,
                period,
                &raw_seed,
                per_code_locks.clone(),
                &vars_map,
            )
            .await;
            let mut tagged: Vec<types::RecoPick> = synthetic;
            tagged.truncate(10);
            tracing::info!(
                "[recommender] style={:?} bucket was empty, filled with {} synthetic picks",
                style,
                tagged.len()
            );
            by_style.insert(style, tagged);
        }
    }

    let as_of_ctx = as_of::current_as_of();
    // spec §8: as-of 模式下因数据截断被降级(≠ 缺失)的风格,前端用橙色显示
    // 简化: 若 as-of 激活 + 某风格被 disabled,等同降级;后续 B11 可细化原因
    let disabled_vec: Vec<Style> = disabled_styles_set.into_iter().collect();
    let (degraded_styles, degraded_reasons) = if as_of_ctx.is_some() {
        let reasons: std::collections::HashMap<Style, String> = disabled_vec
            .iter()
            .map(|s| (*s, "as-of 截断后该风格依赖的历史数据不可用".to_string()))
            .collect();
        (disabled_vec.clone(), reasons)
    } else {
        (Vec::new(), std::collections::HashMap::new())
    };
    let resp = RecoResponse {
        period,
        picks: by_style,
        disabled_styles: disabled_vec,
        degraded_styles,
        degraded_reasons,
        generated_at: chrono::Utc::now().timestamp_millis(),
        raw_seed_pool_size,
        as_of_date: as_of_ctx.as_ref().map(|c| c.as_string()),
        mode: as_of_ctx
            .as_ref()
            .map(|c| c.source.to_string())
            .unwrap_or_else(|| "live".to_string()),
        error_detail: None,
    };
    cache_put(period, resp.clone());
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommender::types::{Period, RecoPick, Style};

    /// 构造一个带 dummy pick 的 RecoResponse（避免 cache_put 跳过空结果）
    fn dummy_resp(period: Period, generated_at: i64, mode: &str) -> RecoResponse {
        let pick = RecoPick {
            stock_code: "000001".into(),
            stock_name: "平安银行".into(),
            sector: None,
            style: Style::Trend,
            period,
            price: 10.0,
            entry_low: 9.5,
            entry_high: 10.5,
            stop_loss: 9.0,
            target_price: 12.0,
            position_pct: 10.0,
            holding_days: 5,
            confidence: 70,
            reasons: vec!["测试".into()],
            risk_notes: vec![],
            secondary_styles: vec![],
            synthetic: false,
        };
        let mut picks = std::collections::BTreeMap::new();
        picks.insert(Style::Trend, vec![pick]);
        RecoResponse {
            period,
            picks,
            disabled_styles: vec![],
            degraded_styles: vec![],
            degraded_reasons: std::collections::HashMap::new(),
            generated_at,
            raw_seed_pool_size: 1,
            as_of_date: None,
            mode: mode.to_string(),
            error_detail: None,
        }
    }

    #[test]
    fn cache_invalidate_works() {
        invalidate_cache();
        let resp = dummy_resp(Period::Short, 0, "live");
        cache_put(Period::Short, resp);
        assert!(cache_get(Period::Short).is_some());
        invalidate_cache();
        assert!(cache_get(Period::Short).is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn cache_live_and_replay_are_isolated() {
        use axagent_astock_data::as_of::{AsOfContext, AsOfSource};
        use chrono::NaiveDate;
        invalidate_cache();

        // 1) live scope 写入
        let live_resp = dummy_resp(Period::Short, 1, "live");
        cache_put(Period::Short, live_resp);

        // 2) live 读命中 (generated_at == 1)
        assert_eq!(cache_get(Period::Short).unwrap().generated_at, 1);

        // 3) replay scope 内读应当 miss
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let got_in_replay = as_of::AS_OF
            .scope(Some(ctx), async { cache_get(Period::Short) })
            .await;
        assert!(
            got_in_replay.is_none(),
            "live cache must not leak into replay scope (got generated_at={:?})",
            got_in_replay.as_ref().map(|r| r.generated_at)
        );

        // 4) replay scope 内写入 → 退出 scope 后 live 仍然 miss，但 replay 自己命中
        let mut replay_resp = dummy_resp(Period::Short, 2, "user_replay");
        replay_resp.as_of_date = Some("2026-06-01".into());
        let replay_cached = as_of::AS_OF
            .scope(Some(ctx), async {
                cache_put(Period::Short, replay_resp);
                cache_get(Period::Short)
            })
            .await
            .expect("replay cache miss after put");
        assert_eq!(replay_cached.generated_at, 2);

        // 5) 切回 live 后再读：不会读到 replay 写入的脏数据，应返回 step 1 的 live entry
        let live_hit = cache_get(Period::Short);
        assert!(live_hit.is_some(), "live cache entry from step 1 should still exist");
        assert_eq!(
            live_hit.unwrap().generated_at,
            1,
            "replay entry (generated_at=2) must not leak into live cache"
        );

        // 6) live scope 再次写入 → 覆盖
        let live_resp2 = dummy_resp(Period::Short, 11, "live");
        cache_put(Period::Short, live_resp2);
        assert_eq!(cache_get(Period::Short).unwrap().generated_at, 11);

        invalidate_cache();
    }

    #[test]
    fn reco_response_serializes_asof_fields() {
        let resp = RecoResponse {
            period: Period::Mid,
            picks: std::collections::BTreeMap::new(),
            disabled_styles: vec![],
            degraded_styles: vec![],
            degraded_reasons: std::collections::HashMap::new(),
            generated_at: 100,
            raw_seed_pool_size: 50,
            as_of_date: Some("2026-06-01".into()),
            mode: "user_replay".into(),
            error_detail: None,
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"asOfDate\":\"2026-06-01\""));
        assert!(s.contains("\"mode\":\"user_replay\""));
    }

    #[test]
    fn reco_response_live_omits_asof_field() {
        let resp = RecoResponse {
            period: Period::Mid,
            picks: std::collections::BTreeMap::new(),
            disabled_styles: vec![],
            degraded_styles: vec![],
            degraded_reasons: std::collections::HashMap::new(),
            generated_at: 100,
            raw_seed_pool_size: 50,
            as_of_date: None,
            mode: "live".into(),
            error_detail: None,
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert!(!s.contains("asOfDate"), "as_of_date should be skipped when None");
        assert!(s.contains("\"mode\":\"live\""));
    }

    fn make_pick(price: f64, target: f64) -> types::RecoPick {
        types::RecoPick {
            stock_code: "688056".into(),
            stock_name: "莱伯泰科".into(),
            sector: None,
            style: Style::Trend,
            period: Period::Short,
            price,
            entry_low: price * 0.98,
            entry_high: price * 1.02,
            stop_loss: price * 0.95,
            target_price: target,
            position_pct: 5.0,
            holding_days: 5,
            confidence: 60,
            reasons: vec!["test".into()],
            risk_notes: vec![],
            secondary_styles: vec![],
            synthetic: false,
        }
    }

    /// Bug 修复：target ≤ current price 的 pick 必须被剔除。
    /// 旧逻辑会保留"现价 48 / 目标 41"的 BUY 推荐，逻辑上矛盾。
    ///
    /// 阈值 0.995 的实际行为：
    /// - 41.87 / 48.16 = 0.869 → DROP
    /// - 42.00 / 38.00 = 1.105 → KEEP
    /// - 38.00 / 38.00 = 1.000 → KEEP（target == price 在容差内）
    /// - 37.81 / 38.00 ≈ 0.995 → KEEP（恰好 >= 0.995 浮点边界）
    /// - 37.85 / 38.00 ≈ 0.996 → KEEP
    /// 期望保留 4 个（pick 2/3/4/5）。
    #[test]
    fn drops_picks_with_no_upside() {
        let mut picks = vec![
            make_pick(48.16, 41.87), // ✗ 无上行空间
            make_pick(38.0, 42.0),   // ✓ 正常
            make_pick(38.0, 38.0),   // ✓ target == price 在 0.995 容差内
            make_pick(38.0, 37.81),  // ✓ 浮点边界，>= 0.995
            make_pick(38.0, 37.85),  // ✓ 37.85/38 ≈ 0.996 > 0.995
        ];
        picks.retain(|p| p.target_price >= p.price * 0.995);
        let codes: Vec<&str> = picks.iter().map(|p| p.stock_code.as_str()).collect();
        assert_eq!(picks.len(), 4, "保留: {codes:?}");
    }

    #[test]
    fn parse_strategy_weights_basic() {
        let vars = vec![(
            "reco_strategy_weights".to_string(),
            serde_json::json!({
                "trend_short": 1.2,
                "value_mid": 0.5,
                "watchlist_long": 0.0
            }),
        )];
        let m = parse_strategy_weights(&vars);
        assert_eq!(m.get(&(Style::Trend, Period::Short)).copied(), Some(1.2));
        assert_eq!(m.get(&(Style::Value, Period::Mid)).copied(), Some(0.5));
        assert_eq!(m.get(&(Style::Watchlist, Period::Long)).copied(), Some(0.0));
        assert!(m.get(&(Style::Capital, Period::Short)).is_none(), "缺失 key 不应有值");
    }

    #[test]
    fn parse_strategy_weights_clamps_to_2x() {
        let vars =
            vec![("reco_strategy_weights".to_string(), serde_json::json!({ "trend_short": 10.0 }))];
        let m = parse_strategy_weights(&vars);
        assert_eq!(m.get(&(Style::Trend, Period::Short)).copied(), Some(2.0), "应 clamp 到 2.0");
    }

    #[test]
    fn parse_strategy_weights_ignores_malformed_keys() {
        let vars = vec![(
            "reco_strategy_weights".to_string(),
            serde_json::json!({
                "trend_extra_short": 1.5,
                "unknown_short": 1.0,
                "trend_week": 1.0
            }),
        )];
        let m = parse_strategy_weights(&vars);
        assert!(m.is_empty(), "所有 key 都应被忽略");
    }

    #[test]
    fn parse_strategy_weights_absent_var_returns_empty() {
        let vars = vec![("reco_trend_enabled".to_string(), serde_json::json!(true))];
        let m = parse_strategy_weights(&vars);
        assert!(m.is_empty());
    }
}
