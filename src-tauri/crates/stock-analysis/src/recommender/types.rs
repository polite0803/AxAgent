//! 智能荐股 — 公共类型

use serde::{Deserialize, Serialize};

/// 风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// 趋势跟踪
    Trend,
    /// 价值低估
    Value,
    /// 资金驱动
    Capital,
    /// 超跌反弹
    Reversion,
    /// 候选池兜底：仅依赖 quote 数据的"系统初筛"列表，
    /// 当 4 个主风格都拿不到数据时充当 fallback，确保面板始终有内容
    Watchlist,
    /// Serenity 瓶颈分析：基于产业链供应链拆解的选股策略，
    /// 种子池由 serenity-screening workflow 生成，
    /// 确定性扫描验证财务/估值/扩产周期信号
    Serenity,
}

impl Style {
    pub fn as_str(&self) -> &'static str {
        match self {
            Style::Trend => "trend",
            Style::Value => "value",
            Style::Capital => "capital",
            Style::Reversion => "reversion",
            Style::Watchlist => "watchlist",
            Style::Serenity => "serenity",
        }
    }
}

/// 持有周期（4 种）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    /// 超短线 1-3 天（T+1 隔夜/事件驱动/情绪博弈）
    #[serde(alias = "ultra_short")]
    UltraShort,
    /// 短线 1-2 周
    Short,
    /// 中线 3-8 周
    Mid,
    /// 长线 3 个月+
    Long,
}

impl Period {
    pub fn as_str(&self) -> &'static str {
        match self {
            Period::UltraShort => "ultra_short",
            Period::Short => "short",
            Period::Mid => "mid",
            Period::Long => "long",
        }
    }

    /// 周期因子（用于动态仓位）
    pub fn factor(&self) -> f64 {
        match self {
            Period::UltraShort => 0.4,
            Period::Short => 0.6,
            Period::Mid => 0.8,
            Period::Long => 1.0,
        }
    }

    /// 建议持有天数
    pub fn default_holding_days(&self) -> u32 {
        match self {
            Period::UltraShort => 2,
            Period::Short => 5,
            Period::Mid => 28,
            Period::Long => 90,
        }
    }
}

/// 单条推荐
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoPick {
    pub stock_code: String,
    pub stock_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    pub style: Style,
    pub period: Period,
    /// 当前价
    pub price: f64,
    /// 入场下沿
    pub entry_low: f64,
    /// 入场上沿
    pub entry_high: f64,
    /// 止损
    pub stop_loss: f64,
    /// 目标位
    pub target_price: f64,
    /// 建议仓位（%）
    pub position_pct: f64,
    /// 持有天数
    pub holding_days: u32,
    /// 置信度 0-100
    pub confidence: u8,
    /// 命中理由
    pub reasons: Vec<String>,
    /// 风险提示
    pub risk_notes: Vec<String>,
    /// 次选风格（同票被多策略命中时记录）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_styles: Vec<Style>,
    /// 是否为兜底合成 pick（true = 系统初筛 / 数据稀疏兜底，无技术信号支撑；
    /// false = 主策略真实命中）。前端用此字段显示"真实/兜底"标识。
    #[serde(default)]
    pub synthetic: bool,
}

/// 荐股响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoResponse {
    pub period: Period,
    /// 按风格分组的 picks，每组 ≤ 10
    /// 使用 BTreeMap 确保序列化到 JSON 时 key 保持 Style 判别式顺序
    pub picks: std::collections::BTreeMap<Style, Vec<RecoPick>>,
    /// 被 vendor 缺失禁用的风格（live 模式下由 vendor 状态决定）
    pub disabled_styles: Vec<Style>,
    /// 被时间锚定 / as-of 截断降级的风格（spec §8）
    /// 与 `disabled_styles` 区别：disabled 是 vendor 完全不可用；
    /// degraded 是该风格对当前 as_of_date 没有历史语义（如 PE-TTM 仅有快照、
    /// 资金流无 N 日前对比等）。前端展示时用不同颜色(灰 / 橙)。
    /// 仅在 as-of 模式下非空；live 模式恒为 `vec![]`。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degraded_styles: Vec<Style>,
    /// `degraded_styles` 中各风格的降级原因（key=style, value=降级原因文本）
    /// 用于前端"⛔ 已降级：{reason}"提示。serde 序列化为 camelCase。
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub degraded_reasons: std::collections::HashMap<Style, String>,
    /// 生成时间戳（毫秒）
    pub generated_at: i64,
    /// **过滤前**的 seed pool 大小（hot + industry 龙头去重后）
    /// 实际参与扫描的池大小更小（流动性过滤会进一步剔除）
    pub raw_seed_pool_size: usize,
    /// 时间旅行模式截止日 (YYYY-MM-DD)；live 模式为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<String>,
    /// 模式标签：live / replay / backtest_sweep
    pub mode: String,
    /// 数据获取错误详情。当 recommed_stocks 前置健康探测失败或全部 vendor 不可用时
    /// 填充此字段，前端据此显示具体错误文本而非泛化的"连接失败"。
    /// None = 正常执行（即使 picks 为空）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
}
