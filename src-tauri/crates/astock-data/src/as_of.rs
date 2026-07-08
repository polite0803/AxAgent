//! 时间旅行（As-Of）上下文
//!
//! 通过 `tokio::task_local!` 注入当前任务的 `AsOfContext`，所有 vendor 调用
//! 都可以隐式读取截止日，从而过滤或降级数据。Live 模式下该 task-local 为 None。
//!
//! # 双层存储：task_local + 进程级全局回退
//!
//! `tokio::task_local!` 不会跨 `tokio::spawn` / `JoinSet::spawn` 边界传播，
//! 而 `run_stock_workflow_inner` 与 `WorkEngine::run_workflow` 中分别有
//! 一次 `tokio::spawn` / `JoinSet::spawn`，导致 vendor 工具调用时
//! `current_as_of()` 返回 None、`truncate_*_by_asof` 兜底失效、数据
//! 穿透到 as-of 之后。
//!
//! 为此引入 `static GLOBAL_AS_OF: Mutex<Option<AsOfContext>>` 作为
//! 进程级回退。读取时优先 task_local（更精确、嵌套感知），没有再读全局；
//! 写入时 `with_optional_asof` 同步写全局，spawn 出去的 future 即可读到。

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

/// As-Of 数据的来源标签，用于审计
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsOfSource {
    /// 用户在 UI 手动选择
    UserReplay,
    /// Sweep 工具批量跑
    BacktestSweep,
    /// 调度器周期跑
    ScheduledReplay,
}

impl std::fmt::Display for AsOfSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsOfSource::UserReplay => write!(f, "user_replay"),
            AsOfSource::BacktestSweep => write!(f, "backtest_sweep"),
            AsOfSource::ScheduledReplay => write!(f, "scheduled_replay"),
        }
    }
}

/// 时间锚点：在该任务执行期间，所有 vendor 调用应被视为"截至 as_of_date"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsOfContext {
    pub as_of_date: NaiveDate,
    pub source: AsOfSource,
    /// 数据截止范围(混合 as-of 模式)。默认 All 兼容旧行为。
    ///
    /// - `All`:所有数据源(价格/技术/财务 + 新闻/公告)统一按 as_of 截止
    /// - `Structured`:仅"结构化数据"按 as_of 截止;新闻/公告/研报
    ///   保持实时(参考 TradingAgents-CN 的"价格截止 + 社交新闻实时")
    #[serde(default)]
    pub data_scope: AsOfDataScope,
}

/// 数据截止范围(混合 as-of 模式核心枚举)
///
/// 用户在 UI 可选择:全截止(回放) / 仅结构化截止(日常分析推荐)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AsOfDataScope {
    /// 所有数据按 as_of 截止(旧行为,默认)
    #[default]
    All,
    /// 仅"结构化数据"按 as_of 截止;新闻/公告/研报/排行 保持实时
    Structured,
}

/// 数据源种类(用于 AsOfDataScope 决策)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsOfDataKind {
    /// 结构化数据:价格/K线/技术指标/财务三张表/资金流/龙虎榜/股东
    /// /融资融券/北向/解禁/分红/一致预期
    Structured,
    /// 非结构化数据:新闻/公告/研报/社媒(StockTwits/Reddit)
    Unstructured,
    /// 排行榜/分类/指数:热门股/行业排名/概念板块/搜索新闻
    Rank,
}

impl AsOfContext {
    /// 创建 AsOfContext；as_of_date 必须在今天及之前
    pub fn new(date: NaiveDate, source: AsOfSource) -> Result<Self, AsOfError> {
        let today = Local::now().date_naive();
        if date > today {
            return Err(AsOfError::FutureDate {
                date: date.to_string(),
                today: today.to_string(),
            });
        }
        Ok(Self {
            as_of_date: date,
            source,
            data_scope: AsOfDataScope::All,
        })
    }

    /// 创建带数据范围的 AsOfContext
    pub fn with_data_scope(mut self, scope: AsOfDataScope) -> Self {
        self.data_scope = scope;
        self
    }

    /// 解析 'YYYY-MM-DD' 字符串；空字符串视为非法
    pub fn parse(s: &str) -> Result<Self, AsOfError> {
        if s.is_empty() {
            return Err(AsOfError::InvalidFormat {
                reason: "empty string".into(),
            });
        }
        let date =
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| AsOfError::InvalidFormat {
                reason: e.to_string(),
            })?;
        Self::new(date, AsOfSource::UserReplay)
    }

    /// 解析可选入参（None / 空 / 全空白 → None；合法 → Some；非法 → Err）
    pub fn parse_optional(s: Option<&str>) -> Result<Option<Self>, String> {
        match s.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => Self::parse(s)
                .map(Some)
                .map_err(|e| format!("as_of_date 解析失败: {e}")),
        }
    }

    /// 转 'YYYY-MM-DD' 字符串
    pub fn as_string(&self) -> String {
        self.as_of_date.format("%Y-%m-%d").to_string()
    }
}

#[derive(Debug, Error)]
pub enum AsOfError {
    #[error("as_of_date cannot be in the future: {date} (today is {today})")]
    FutureDate { date: String, today: String },

    #[error("as_of_date format invalid: {reason}")]
    InvalidFormat { reason: String },

    #[error("as_of_date too old: {0} days ago, max is {1}")]
    TooOld(i64, i64),
}

tokio::task_local! {
    /// 当前任务内的 AsOfContext；None 表示 live 模式
    pub static AS_OF: Option<AsOfContext>;
}

// ─── 进程级全局回退 ─────────────────────────────────────────────
//
// 设计要点：
// 1. 用 `std::sync::Mutex`（同步锁），不持锁跨 await，不会破坏
//    tokio 调度器，也不会出现"未来日期 guard 跨越 await"之类问题。
// 2. `OnceLock<Mutex<...>>` 延迟初始化，避免构造期全局状态问题。
// 3. `current_as_of()` 优先 task_local，再回退全局。这样嵌套调用
//    仍然能区分内外层；只有跨 spawn 边界才"扁平化"到全局值。
// 4. `with_optional_asof` 同步写全局；`AsOfScopeGuard` 离开时恢复，
//    确保同一进程内多次回放互不污染。

/// 进程级全局 AsOfContext：task_local 不可见时的兜底
static GLOBAL_AS_OF: OnceLock<Mutex<Option<AsOfContext>>> = OnceLock::new();

#[inline]
fn global_lock() -> &'static Mutex<Option<AsOfContext>> {
    GLOBAL_AS_OF.get_or_init(|| Mutex::new(None))
}

/// 同步写入全局 AsOfContext，返回写入前的旧值。
///
/// 注意：这是同步调用，**不**会跨 await 持锁；用于在 `tokio::spawn`
/// 之前先同步写入，使 spawn 出去的 future 通过 `current_as_of()`
/// 也能读到截止日。
///
/// 容忍 `std::sync::Mutex` poison：如果另一个持有锁的线程/任务 panic，
/// 我们仍然恢复全局状态（防止测试/工作流 panic 后污染后续调用）。
pub fn set_global_asof(ctx: Option<AsOfContext>) -> Option<AsOfContext> {
    let mut g = match global_lock().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let prev = *g;
    *g = ctx;
    prev
}

/// 同步读取全局 AsOfContext（不影响 task_local 优先级）
pub fn peek_global_asof() -> Option<AsOfContext> {
    let g = match global_lock().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *g
}

/// 同步清空全局 AsOfContext，返回清空前的旧值
pub fn clear_global_asof() -> Option<AsOfContext> {
    set_global_asof(None)
}

/// RAII 守卫：构造时写入全局 AsOfContext，drop 时恢复原值。
///
/// 用法（推荐用于 Tauri command 入口）：
/// ```ignore
/// let _guard = as_of::enter_global_asof(Some(ctx));
/// // 此作用域内 current_as_of() 在任何 task（包括 spawn 出去的）都可见 ctx
/// // 作用域结束自动恢复
/// ```
pub struct AsOfScopeGuard {
    prev: Option<AsOfContext>,
}

impl Drop for AsOfScopeGuard {
    fn drop(&mut self) {
        let _ = set_global_asof(self.prev);
    }
}

pub fn enter_global_asof(ctx: Option<AsOfContext>) -> AsOfScopeGuard {
    let prev = set_global_asof(ctx);
    AsOfScopeGuard { prev }
}

/// 读取当前任务的 AsOfContext。
///
/// 优先级：**task_local > 全局**。`tokio::task_local!` 跨 spawn
/// 边界不可见，所以 spawn 出去又没有自己 `scope` 的 future 走
/// 全局回退路径。Live 模式（task_local None + 全局 None）返回 None。
pub fn current_as_of() -> Option<AsOfContext> {
    if let Ok(Some(c)) = AS_OF.try_with(|c| *c) {
        return Some(c);
    }
    // spawn 边界兜底：task_local 不可见时读进程级全局
    peek_global_asof()
}

/// 获取 as-of 日期作为 YYYY-MM-DD 字符串，无 as-of 时返回系统当前日期
pub fn current_date_or_now() -> String {
    match current_as_of() {
        Some(ctx) => ctx.as_of_date.format("%Y-%m-%d").to_string(),
        None => Local::now().format("%Y-%m-%d").to_string(),
    }
}

/// 判断当前是否处于时间旅行模式
pub fn is_asof_active() -> bool {
    current_as_of().is_some()
}

/// 判断"指定数据种类"是否受当前 as-of 影响
///
/// 决策矩阵(借鉴 TradingAgents-CN README 202 行):
/// | scope         | Structured 工具 | Unstructured 工具 | Rank 工具 |
/// |---------------|----------------|-------------------|-----------|
/// | All           | ✅ 受影响       | ✅ 受影响          | ✅ 受影响  |
/// | Structured    | ✅ 受影响       | ❌ 实时(穿透)     | ❌ 实时   |
/// | 无 as-of(live) | ❌ 实时       | ❌ 实时            | ❌ 实时   |
///
/// 用法(供 vendor 调用处判断):
/// ```ignore
/// if as_of::is_asof_active_for(AsOfDataKind::Structured) {
///     let adjusted = current_as_of().unwrap();
///     vendor.fetch_as_of(adjusted.as_of_date);
/// } else {
///     vendor.fetch_live();
/// }
/// ```
pub fn is_asof_active_for(kind: AsOfDataKind) -> bool {
    match current_as_of() {
        None => false,
        Some(ctx) => match ctx.data_scope {
            AsOfDataScope::All => true,
            AsOfDataScope::Structured => matches!(kind, AsOfDataKind::Structured),
        },
    }
}

/// 在指定 scope 内运行闭包，并提供降级日志的 task_local 容器
pub async fn with_degradation_log<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    DEGRADATION_LOG
        .scope(std::cell::RefCell::new(Vec::new()), f)
        .await
}

/// 当前数据新鲜度描述(供工作流 prompt `{{data_freshness}}` 变量注入)
///
/// 返回中文短语,如:
/// - live 模式 → "实时数据(无时间锚定)"
/// - Structured + as_of=X → "价格/技术/财务 截至 X,新闻/公告 实时"
/// - All + as_of=X       → "全数据截至 X(回放模式)"
pub fn data_freshness_description() -> String {
    match current_as_of() {
        None => "实时数据(无时间锚定)".to_string(),
        Some(ctx) => {
            let date = ctx.as_string();
            match ctx.data_scope {
                AsOfDataScope::All => format!("全数据截至 {date}(回放模式)"),
                AsOfDataScope::Structured => {
                    format!("价格/技术/财务 截至 {date},新闻/公告 实时")
                },
            }
        },
    }
}

/// 生成当前 AsOf 的 cache key 后缀（live 模式返回 "live"）
pub fn cache_suffix() -> String {
    current_as_of()
        .map(|c| format!("asof-{}", c.as_of_date.format("%Y%m%d")))
        .unwrap_or_else(|| "live".to_string())
}

/// As-Of 降级条目：vendor / method 在该时间点不可用或无历史语义
/// (spec §4.1 统一降级协议)
#[derive(Debug, Clone, serde::Serialize)]
pub struct DegradationEntry {
    pub vendor: String,
    pub method: String,
    pub reason: String,
    pub as_of: String,
}

// 任务级降级日志：每个 tokio 任务一个 Vec，scope 结束时不重置，
// 由 workflow 节点通过 take_asof_degradation_report() 一次性消费并清空。
tokio::task_local! {
    static DEGRADATION_LOG: std::cell::RefCell<Vec<DegradationEntry>>;
}

/// 全局降级环形缓冲(缺陷 E 修复):供前端 poll 实时显示降级数量/详情。
/// 不依赖 task_local 作用域(全局可见),cap 256 条,满了弹出最早。
static GLOBAL_DEGRADATION_LOG: Mutex<VecDeque<DegradationEntry>> = Mutex::new(VecDeque::new());
static GLOBAL_DEGRADATION_TOTAL: AtomicU64 = AtomicU64::new(0);

const GLOBAL_DEGRADATION_CAP: usize = 256;

/// 记录一次降级(仅在 as-of 模式下有效，live 模式直接忽略)
pub fn record_degradation(vendor: &str, method: &str, reason: &str) {
    let as_of = match current_as_of() {
        Some(c) => c.as_string(),
        None => return, // live 模式无降级概念
    };
    let entry = DegradationEntry {
        vendor: vendor.to_string(),
        method: method.to_string(),
        reason: reason.to_string(),
        as_of,
    };
    // 任务级尝试：若没在 task_local scope 中，单独初始化一个新 scope
    let _ = DEGRADATION_LOG.try_with(|cell| {
        cell.borrow_mut().push(entry.clone());
    });
    // 全局环形缓冲: 累计总数 + 保留最近 N 条详情
    if let Ok(mut g) = GLOBAL_DEGRADATION_LOG.lock() {
        if g.len() >= GLOBAL_DEGRADATION_CAP {
            g.pop_front();
        }
        g.push_back(entry);
        GLOBAL_DEGRADATION_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
}

/// 在可选 AsOfContext 包裹下运行闭包。
///
/// - `ctx = None`  → 直接执行 `f`（live 模式，零开销）
/// - `ctx = Some(c)` → `AS_OF.scope(Some(c), f)`（让 vendor 调用能读到 task_local）
///
/// **同时同步写入进程级全局**，使 `tokio::spawn` / `JoinSet::spawn`
/// 出去的 future 通过 `current_as_of()` 也能读到截止日。
///
/// 注意：本函数**故意不**在退出时恢复全局（让 spawn 的 future 仍然
/// 读得到），调用方在合适的时机显式 `set_global_asof(None)` 即可。
/// 如需 RAII 自动恢复，请使用 `enter_global_asof` + `as_of::AS_OF.scope` 组合。
pub async fn with_optional_asof<F, T>(ctx: Option<AsOfContext>, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    // 同步写全局，供 spawn 出去的子任务回退读取
    set_global_asof(ctx);
    match ctx {
        Some(c) => AS_OF.scope(Some(c), f).await,
        None => f.await,
    }
}

/// 消费并清空当前任务的降级日志。返回累积的降级条目。
/// 必须在 with_degradation_log scope 内调用，否则返回空 Vec。
pub fn take_asof_degradation_report() -> Vec<DegradationEntry> {
    DEGRADATION_LOG
        .try_with(|cell| std::mem::take(&mut *cell.borrow_mut()))
        .unwrap_or_default()
}

/// 仅快照全局降级日志(不清空,供前端 poll 显示)。
/// 返回按时间顺序排列(旧 → 新)的最近 256 条。
pub fn peek_global_degradation_report() -> Vec<DegradationEntry> {
    GLOBAL_DEGRADATION_LOG
        .lock()
        .map(|g| g.iter().cloned().collect())
        .unwrap_or_default()
}

/// 当前累计降级总数(从进程启动起算,跨 live/replay 切换)。
pub fn global_degradation_count() -> u64 {
    GLOBAL_DEGRADATION_TOTAL.load(Ordering::Relaxed)
}

/// 清空全局降级缓冲(切换到 live 模式时由前端触发,避免过期条目一直显示)。
pub fn reset_global_degradation_log() {
    if let Ok(mut g) = GLOBAL_DEGRADATION_LOG.lock() {
        g.clear();
    }
    // total 不重置,保留"曾经降级过多少项"作为历史指标
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serial_test::serial;

    #[tokio::test]
    #[serial(asof)]
    async fn current_as_of_returns_none_outside_scope() {
        // 清理全局，确保测试隔离
        let _ = clear_global_asof();
        assert!(current_as_of().is_none());
        assert!(!is_asof_active());
    }

    #[tokio::test]
    async fn current_as_of_returns_value_inside_scope() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let got = AS_OF.scope(Some(ctx), async { current_as_of() }).await;
        assert_eq!(got.unwrap().as_of_date, date);
        assert_eq!(got.unwrap().source, AsOfSource::UserReplay);
    }

    #[tokio::test]
    async fn validate_rejects_future_date() {
        let future = Local::now().date_naive() + Duration::days(7);
        let result = AsOfContext::new(future, AsOfSource::UserReplay);
        assert!(matches!(result, Err(AsOfError::FutureDate { .. })));
    }

    #[tokio::test]
    async fn validate_rejects_empty_string() {
        let result = AsOfContext::parse("");
        assert!(matches!(result, Err(AsOfError::InvalidFormat { .. })));
    }

    #[tokio::test]
    async fn validate_rejects_invalid_format() {
        let result = AsOfContext::parse("2026/06/01");
        assert!(matches!(result, Err(AsOfError::InvalidFormat { .. })));
    }

    #[tokio::test]
    #[serial(asof)]
    async fn today_is_accepted() {
        let today = Local::now().date_naive();
        let ctx = AsOfContext::new(today, AsOfSource::UserReplay).unwrap();
        assert_eq!(ctx.as_of_date, today);
    }

    #[tokio::test]
    async fn parse_roundtrip() {
        let today = Local::now().date_naive();
        let s = today.format("%Y-%m-%d").to_string();
        let ctx = AsOfContext::parse(&s).unwrap();
        assert_eq!(ctx.as_string(), s);
    }

    #[test]
    fn parse_optional_none_is_live() {
        let r = AsOfContext::parse_optional(None).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn parse_optional_empty_is_live() {
        assert!(AsOfContext::parse_optional(Some("")).unwrap().is_none());
        assert!(AsOfContext::parse_optional(Some("   ")).unwrap().is_none());
    }

    #[test]
    fn parse_optional_past_date_is_replay() {
        let today = Local::now().date_naive();
        let past = today - Duration::days(7);
        let s = past.format("%Y-%m-%d").to_string();
        let r = AsOfContext::parse_optional(Some(&s)).unwrap();
        assert!(r.is_some());
        assert_eq!(r.unwrap().as_string(), s);
    }

    #[test]
    #[serial(asof)]
    fn parse_optional_future_date_rejected() {
        let future = Local::now().date_naive() + Duration::days(7);
        let s = future.format("%Y-%m-%d").to_string();
        let r = AsOfContext::parse_optional(Some(&s));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("as_of_date 解析失败"));
    }

    #[test]
    fn parse_optional_invalid_format_rejected() {
        assert!(AsOfContext::parse_optional(Some("2026/06/01")).is_err());
        assert!(AsOfContext::parse_optional(Some("garbage")).is_err());
    }

    #[tokio::test]
    async fn cache_suffix_returns_live_outside_scope() {
        let _ = clear_global_asof();
        let s = cache_suffix();
        assert_eq!(s, "live");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn cache_suffix_includes_date_inside_scope() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let s = AS_OF.scope(Some(ctx), async { cache_suffix() }).await;
        assert_eq!(s, "asof-20260601");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn nested_scope_uses_inner_value() {
        let _ = clear_global_asof();
        let outer_date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let inner_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let outer = AsOfContext::new(outer_date, AsOfSource::UserReplay).unwrap();
        let inner = AsOfContext::new(inner_date, AsOfSource::UserReplay).unwrap();
        let result = AS_OF
            .scope(Some(outer), async {
                let outer_val = current_as_of().unwrap().as_of_date;
                let inner_val = AS_OF
                    .scope(Some(inner), async { current_as_of().unwrap().as_of_date })
                    .await;
                // 内层 scope 结束后，外层值恢复
                let after_inner = current_as_of().unwrap().as_of_date;
                (outer_val, inner_val, after_inner)
            })
            .await;
        assert_eq!(result, (outer_date, inner_date, outer_date));
    }

    // ── 降级日志(spec §4.1 统一降级协议) ────────────────────────
    // 实时性方法在 as-of 模式下跳过时，必须把降级原因写入日志供 workflow 消费

    #[tokio::test]
    async fn record_degradation_ignored_in_live_mode() {
        let _ = clear_global_asof();
        record_degradation("vendor", "method", "test");
        // live 模式下没有 task_local scope，take 返回空
        let report = take_asof_degradation_report();
        assert!(report.is_empty());
    }

    #[tokio::test]
    async fn record_degradation_captures_entries_in_asof_scope() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        // 正确用法：AS_OF.scope 包裹 with_degradation_log，workflow 节点调用入口
        AS_OF
            .scope(Some(ctx), async {
                with_degradation_log(async {
                    record_degradation("eastmoney", "get_hot_stocks", "no historical semantics");
                    record_degradation("tencent", "get_cls_flash", "future-only feed");
                    let report = take_asof_degradation_report();
                    assert_eq!(report.len(), 2);
                    assert_eq!(report[0].vendor, "eastmoney");
                    assert_eq!(report[0].as_of, "2026-06-01");
                    assert_eq!(report[1].method, "get_cls_flash");
                    // 消费后清空
                    let second = take_asof_degradation_report();
                    assert!(second.is_empty(), "take 应当清空日志");
                })
                .await
            })
            .await;
    }

    // ── 进程级全局回退(缺陷: spawn 边界穿透) ──────────────────

    /// task_local 优先级：在两层都设值时，task_local 胜出
    #[tokio::test]
    #[serial(asof)]
    async fn current_asof_prefers_task_local() {
        let _ = clear_global_asof();
        let task_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let global_date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let task_ctx = AsOfContext::new(task_date, AsOfSource::UserReplay).unwrap();
        let global_ctx = AsOfContext::new(global_date, AsOfSource::ScheduledReplay).unwrap();
        set_global_asof(Some(global_ctx));
        let got = AS_OF
            .scope(Some(task_ctx), async { current_as_of().unwrap() })
            .await;
        assert_eq!(got.as_of_date, task_date, "task_local 必须胜过全局");
        assert_eq!(got.source, AsOfSource::UserReplay);
        let _ = clear_global_asof();
    }

    /// 跨 spawn 边界全局回退：spawn 出去的 future 应当读到全局值
    #[serial(asof)]
    #[tokio::test]
    async fn global_fallback_survives_tokio_spawn() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        set_global_asof(Some(ctx));

        // 在新 spawn 的任务中（无 task_local scope），current_as_of 应当
        // 通过全局回退读到 ctx
        let got = tokio::spawn(async move { current_as_of() }).await.unwrap();
        assert!(got.is_some(), "spawn 边界后必须能读到全局 AsOfContext");
        assert_eq!(got.unwrap().as_of_date, date);
        let _ = clear_global_asof();
    }

    /// 没有 task_local 也没有全局时，current_as_of 返回 None
    #[serial(asof)]
    #[tokio::test]
    async fn current_asof_falls_back_to_global_outside_scope() {
        let _ = clear_global_asof();
        // 仅有 task_local，无全局：scope 内读到值
        let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::BacktestSweep).unwrap();
        let in_scope = AS_OF
            .scope(Some(ctx), async { current_as_of().is_some() })
            .await;
        assert!(in_scope);
        // scope 外（且全局已清空）应返回 None
        assert!(current_as_of().is_none());
    }

    /// RAII 守卫：早返回也必须恢复原值
    #[tokio::test]
    async fn raii_guard_restores_on_early_return() {
        let _ = clear_global_asof();
        let today = Local::now().date_naive();
        let original_date = today - Duration::days(90);
        let temp_date = today - Duration::days(7);
        let original = AsOfContext::new(original_date, AsOfSource::UserReplay).unwrap();
        let temp = AsOfContext::new(temp_date, AsOfSource::ScheduledReplay).unwrap();
        set_global_asof(Some(original));

        async fn inner(temp: AsOfContext) -> Option<AsOfContext> {
            let _guard = enter_global_asof(Some(temp));
            assert_eq!(current_as_of().unwrap().as_of_date, temp.as_of_date);
            // 早返回：_guard 仍会 drop，恢复原值
            current_as_of()
        }

        let in_temp = inner(temp).await;
        assert_eq!(in_temp.unwrap().as_of_date, temp_date);
        // inner 返回后，全局应当恢复为 original
        let after = current_as_of().unwrap();
        assert_eq!(after.as_of_date, original_date, "RAII 必须恢复原值");
        let _ = clear_global_asof();
    }

    /// 嵌套守卫：内层 drop 后必须 LIFO 恢复到外层值
    #[tokio::test]
    #[serial(asof)]
    async fn nested_guards_restore_lifo() {
        let _ = clear_global_asof();
        let outer_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let inner_date = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let outer = AsOfContext::new(outer_date, AsOfSource::UserReplay).unwrap();
        let inner = AsOfContext::new(inner_date, AsOfSource::ScheduledReplay).unwrap();
        set_global_asof(Some(outer));

        let _g_outer = enter_global_asof(Some(outer));
        assert_eq!(current_as_of().unwrap().as_of_date, outer_date);

        {
            let _g_inner = enter_global_asof(Some(inner));
            assert_eq!(current_as_of().unwrap().as_of_date, inner_date);
            // _g_inner 在此 block 结束时 drop
        }

        // 回到 outer
        assert_eq!(current_as_of().unwrap().as_of_date, outer_date, "内层 drop 后必须恢复 outer");
        let _ = clear_global_asof();
    }

    // ── 混合 as-of 模式(Phase 1:数据范围分离) ──────────────────
    // 设计动机：参考 TradingAgents-CN README 的"价格截止 + 社交/新闻实时"。
    // 用户回放个股时，价格/技术/财务按 as_of 截止，但想看当时的新闻/公告
    // 是否还有后效（如事件影响持续到回放日期之后）。本组测试覆盖该模式。

    /// 默认行为兼容：未设置 data_scope 时等同于 All（保持旧语义）
    #[test]
    fn data_scope_default_is_all_compatible() {
        assert_eq!(AsOfDataScope::default(), AsOfDataScope::All);
        let today = Local::now().date_naive();
        let ctx = AsOfContext::new(today, AsOfSource::UserReplay).unwrap();
        assert_eq!(ctx.data_scope, AsOfDataScope::All);
    }

    /// with_data_scope 是消费式 API，链式调用应保留日期/来源不变
    #[test]
    fn with_data_scope_chains_correctly() {
        let today = Local::now().date_naive();
        let ctx = AsOfContext::new(today, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(AsOfDataScope::Structured);
        assert_eq!(ctx.data_scope, AsOfDataScope::Structured);
        assert_eq!(ctx.source, AsOfSource::UserReplay);
        assert_eq!(ctx.as_of_date, today);
    }

    /// data_scope=All 时，所有 kind 都被 as-of 拦截
    #[tokio::test]
    #[serial(asof)]
    async fn data_scope_all_blocks_all_kinds() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let result = AS_OF
            .scope(Some(ctx), async {
                let structured = is_asof_active_for(AsOfDataKind::Structured);
                let unstructured = is_asof_active_for(AsOfDataKind::Unstructured);
                let rank = is_asof_active_for(AsOfDataKind::Rank);
                (structured, unstructured, rank)
            })
            .await;
        assert_eq!(result, (true, true, true), "All 模式必须拦截所有 kind");
    }

    /// data_scope=Structured 时，仅结构化数据被拦截，新闻/公告/排行保持实时
    #[tokio::test]
    #[serial(asof)]
    async fn data_scope_structured_blocks_only_structured() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(AsOfDataScope::Structured);
        let result = AS_OF
            .scope(Some(ctx), async {
                let structured = is_asof_active_for(AsOfDataKind::Structured);
                let unstructured = is_asof_active_for(AsOfDataKind::Unstructured);
                let rank = is_asof_active_for(AsOfDataKind::Rank);
                (structured, unstructured, rank)
            })
            .await;
        assert_eq!(
            result,
            (true, false, false),
            "Structured 模式：仅结构化数据走 as-of，新闻/排行放行"
        );
    }

    /// live 模式（无 as_of）下，is_asof_active_for 对所有 kind 都返回 false
    #[tokio::test]
    #[serial(asof)]
    async fn live_mode_blocks_nothing() {
        let _ = clear_global_asof();
        let result = AS_OF
            .scope(None, async {
                (
                    is_asof_active_for(AsOfDataKind::Structured),
                    is_asof_active_for(AsOfDataKind::Unstructured),
                    is_asof_active_for(AsOfDataKind::Rank),
                )
            })
            .await;
        assert_eq!(result, (false, false, false));
    }

    /// is_asof_active_for 必须能跨越 spawn 边界（依赖全局回退）
    #[serial(asof)]
    #[tokio::test]
    async fn is_asof_active_for_works_across_spawn_boundary() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(AsOfDataScope::Structured);
        set_global_asof(Some(ctx));

        // spawn 出去的新任务无 task_local scope，必须能通过全局回退读到 kind 决策
        let (structured, unstructured) = tokio::spawn(async move {
            (
                is_asof_active_for(AsOfDataKind::Structured),
                is_asof_active_for(AsOfDataKind::Unstructured),
            )
        })
        .await
        .unwrap();
        assert!(structured, "spawn 后 Structured 仍走 as-of");
        assert!(!unstructured, "spawn 后 Unstructured 不应被 as-of 拦截");
        let _ = clear_global_asof();
    }

    /// data_freshness_description 应当分别覆盖 live / All / Structured 三种文案
    #[tokio::test]
    #[serial(asof)]
    async fn data_freshness_description_live() {
        let _ = clear_global_asof();
        let s = data_freshness_description();
        assert!(s.contains("实时"), "live 文案必须含『实时』: {s}");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn data_freshness_description_all() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay).unwrap();
        let s = AS_OF
            .scope(Some(ctx), async { data_freshness_description() })
            .await;
        assert!(s.contains("2026-06-01"), "All 文案必须含日期: {s}");
        assert!(s.contains("全数据"), "All 文案必须含『全数据』: {s}");
    }

    #[tokio::test]
    #[serial(asof)]
    async fn data_freshness_description_structured() {
        let _ = clear_global_asof();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(AsOfDataScope::Structured);
        let s = AS_OF
            .scope(Some(ctx), async { data_freshness_description() })
            .await;
        assert!(s.contains("2026-06-01"));
        assert!(s.contains("新闻"), "Structured 文案必须说明新闻是实时的: {s}");
    }

    /// serde 向后兼容：旧的 JSON 文本没有 data_scope 字段时，解析结果为 All
    #[test]
    fn asof_ctx_backward_compatible_serde() {
        // 旧版本序列化格式（只有两个字段）
        let legacy = r#"{"as_of_date":"2026-06-01","source":"user_replay"}"#;
        let ctx: AsOfContext = serde_json::from_str(legacy).unwrap();
        assert_eq!(ctx.as_of_date.to_string(), "2026-06-01");
        assert_eq!(ctx.source, AsOfSource::UserReplay);
        // data_scope 走 serde default -> All
        assert_eq!(ctx.data_scope, AsOfDataScope::All);
    }

    /// serde 正向：新格式带 snake_case 枚举字符串
    #[test]
    fn asof_ctx_serde_with_data_scope() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ctx = AsOfContext::new(date, AsOfSource::UserReplay)
            .unwrap()
            .with_data_scope(AsOfDataScope::Structured);
        let s = serde_json::to_string(&ctx).unwrap();
        assert!(s.contains("\"data_scope\":\"structured\""), "序列化必须小写枚举名: {s}");
        let back: AsOfContext = serde_json::from_str(&s).unwrap();
        assert_eq!(back, ctx);
    }
}
