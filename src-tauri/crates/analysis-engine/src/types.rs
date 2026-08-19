//! 股票分析领域类型定义
//!
//! 集中管理 analysis-engine crate 中使用的枚举类型，
//! 消除魔法字符串，提升类型安全。

use serde::{Deserialize, Serialize};

/// 安全边际水平
///
/// 基于 MOS 百分比划分：
/// - `Adequate`: MOS > 30%
/// - `Moderate`: MOS 15-30%
/// - `Insufficient`: MOS 0-15%
/// - `None`: MOS ≤ 0%
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MosLevel {
    #[serde(rename = "充足")]
    Adequate,
    #[serde(rename = "适中")]
    Moderate,
    #[serde(rename = "不足")]
    Insufficient,
    #[serde(rename = "无")]
    None,
}

impl MosLevel {
    /// 从 MOS 百分比推导等级
    pub fn from_mos_pct(mos: f64) -> Self {
        if mos > 30.0 {
            Self::Adequate
        } else if mos > 15.0 {
            Self::Moderate
        } else if mos > 0.0 {
            Self::Insufficient
        } else {
            Self::None
        }
    }

    /// 中文标签（用于显示）
    pub fn label(self) -> &'static str {
        match self {
            Self::Adequate => "充足",
            Self::Moderate => "适中",
            Self::Insufficient => "不足",
            Self::None => "无",
        }
    }

    /// 安全边际描述
    pub fn description(self, mos_pct: f64) -> String {
        match self {
            Self::Adequate => format!("充足的安全边际 {:.0}%", mos_pct),
            Self::Moderate => format!("有一定安全边际 {:.0}%", mos_pct),
            Self::Insufficient => format!("安全边际不足 {:.0}%", mos_pct),
            Self::None => format!("无安全边际 {:.0}%", mos_pct),
        }
    }
}

/// Piotroski F-Score 等级
///
/// 基于 F-Score (0-9) 划分：
/// - `Excellent`: 7-9 分
/// - `Good`: 5-6 分
/// - `Fair`: 3-4 分
/// - `Weak`: 0-2 分
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FScoreLevel {
    #[serde(rename = "优秀(7-9)")]
    Excellent,
    #[serde(rename = "良好(5-6)")]
    Good,
    #[serde(rename = "一般(3-4)")]
    Fair,
    #[serde(rename = "弱(0-2)")]
    Weak,
}

impl FScoreLevel {
    /// 从 F-Score 数值推导等级
    pub fn from_score(score: u32) -> Self {
        match score {
            7..=9 => Self::Excellent,
            5..=6 => Self::Good,
            3..=4 => Self::Fair,
            _ => Self::Weak,
        }
    }

    /// 中文标签（用于显示）
    pub fn label(self) -> &'static str {
        match self {
            Self::Excellent => "优秀(7-9)",
            Self::Good => "良好(5-6)",
            Self::Fair => "一般(3-4)",
            Self::Weak => "弱(0-2)",
        }
    }

    /// 简短标签
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Excellent => "优秀",
            Self::Good => "良好",
            Self::Fair => "一般",
            Self::Weak => "差",
        }
    }
}

/// 护城河水平
///
/// 基于护城河评分 (0-100) 划分：
/// - `Wide`: ≥ 70 分
/// - `Narrow`: 40-69 分
/// - `None`: < 40 分
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoatLevel {
    #[serde(rename = "宽阔")]
    Wide,
    #[serde(rename = "狭窄")]
    Narrow,
    #[serde(rename = "无")]
    None,
}

impl MoatLevel {
    /// 从护城河评分推导等级
    pub fn from_score(score: u32) -> Self {
        if score >= 70 {
            Self::Wide
        } else if score >= 40 {
            Self::Narrow
        } else {
            Self::None
        }
    }

    /// 中文标签（用于显示）
    pub fn label(self) -> &'static str {
        match self {
            Self::Wide => "宽阔",
            Self::Narrow => "狭窄",
            Self::None => "无",
        }
    }

    /// 护城河类型描述（用于 MoatAssessment）
    pub fn type_label(self) -> &'static str {
        match self {
            Self::Wide => "宽护城河",
            Self::Narrow => "窄护城河",
            Self::None => "无护城河",
        }
    }
}

/// 综合价值信号
///
/// 基于 value_score 划分：
/// - `Undervalued`: ≥ 60
/// - `SlightlyUndervalued`: 45-59
/// - `Fair`: 30-44
/// - `SlightlyOvervalued`: 15-29
/// - `Overvalued`: < 15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueSignal {
    #[serde(rename = "低估")]
    Undervalued,
    #[serde(rename = "合理偏低")]
    SlightlyUndervalued,
    #[serde(rename = "合理")]
    Fair,
    #[serde(rename = "偏高")]
    SlightlyOvervalued,
    #[serde(rename = "高估")]
    Overvalued,
}

impl ValueSignal {
    /// 从综合评分推导信号
    pub fn from_score(score: i32) -> Self {
        if score >= 60 {
            Self::Undervalued
        } else if score >= 45 {
            Self::SlightlyUndervalued
        } else if score >= 30 {
            Self::Fair
        } else if score >= 15 {
            Self::SlightlyOvervalued
        } else {
            Self::Overvalued
        }
    }

    /// 中文标签（用于显示）
    pub fn label(self) -> &'static str {
        match self {
            Self::Undervalued => "低估",
            Self::SlightlyUndervalued => "合理偏低",
            Self::Fair => "合理",
            Self::SlightlyOvervalued => "偏高",
            Self::Overvalued => "高估",
        }
    }
}

/// 市场状态类型
///
/// 基于沪深300均线位置和斜率判断：
/// - `Bull`: 价格站上 MA60 + 多头排列 + 向上斜率
/// - `Bear`: 价格跌破 MA60 + 空头排列 + 向下斜率
/// - `Sideways`: 震荡
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketRegimeType {
    Bull,
    Bear,
    Sideways,
}

impl MarketRegimeType {
    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Bull => "牛市",
            Self::Bear => "熊市",
            Self::Sideways => "震荡",
        }
    }
}

/// 波动率水平
///
/// 基于布林带宽度（20日收盘价标准差 / MA20）：
/// - `High`: > 20%
/// - `Low`: < 10%
/// - `Normal`: 10-20%
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolatilityLevel {
    High,
    Low,
    Normal,
}

impl VolatilityLevel {
    /// 从波动率百分比推导等级
    pub fn from_bollinger_pct(pct: f64) -> Self {
        if pct > 0.20 {
            Self::High
        } else if pct < 0.10 {
            Self::Low
        } else {
            Self::Normal
        }
    }

    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::High => "高波动",
            Self::Low => "低波动",
            Self::Normal => "正常",
        }
    }
}

/// 巴菲特式投资裁决
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuffettVerdict {
    #[serde(rename = "🎯 巴菲特可能会喜欢")]
    HighlyAttractive,
    #[serde(rename = "👍 有一定吸引力")]
    Attractive,
    #[serde(rename = "🤔 需要更多安全边际")]
    NeedsMoreMargin,
    #[serde(rename = "❌ 不符合巴菲特标准")]
    DoesNotMeetStandard,
}

impl BuffettVerdict {
    /// 从指标推导裁决
    pub fn from_metrics(moat_score: u32, f_score: u32, mos_pct: f64) -> Self {
        if moat_score >= 70 && f_score >= 7 && mos_pct >= 20.0 {
            Self::HighlyAttractive
        } else if moat_score >= 50 && f_score >= 5 && mos_pct >= 10.0 {
            Self::Attractive
        } else if moat_score >= 30 {
            Self::NeedsMoreMargin
        } else {
            Self::DoesNotMeetStandard
        }
    }

    /// 中文详细描述
    pub fn detail(self) -> &'static str {
        match self {
            Self::HighlyAttractive => "宽护城河+财务健康+充足安全边际。以合理价格买入优秀公司。",
            Self::Attractive => "护城河和财务状况尚可，安全边际处于临界点。可小仓位观察。",
            Self::NeedsMoreMargin => {
                "公司质地一般，等待更好的价格。巴菲特会说：'等待那个又胖又慢的球'。"
            },
            Self::DoesNotMeetStandard => {
                "护城河不足或财务质量差。'以合理价格买入优秀公司比以便宜价格买入平庸公司好得多'。"
            },
        }
    }

    /// 完整裁决文本（含 emoji）
    pub fn full_text(self) -> String {
        format!("{} {}", self.label(), self.detail())
    }

    /// 标签（含 emoji）
    pub fn label(self) -> &'static str {
        match self {
            Self::HighlyAttractive => "🎯 巴菲特可能会喜欢",
            Self::Attractive => "👍 有一定吸引力",
            Self::NeedsMoreMargin => "🤔 需要更多安全边际",
            Self::DoesNotMeetStandard => "❌ 不符合巴菲特标准",
        }
    }
}

/// 策略趋势方向
///
/// 用于 evolution_drift 策略漂移追踪：
/// - `Up`: 策略净收益上升 > 5%
/// - `Down`: 策略净收益下降 > 5%
/// - `Stable`: 策略净收益波动在 ±5% 内
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyTrend {
    Up,
    Down,
    Stable,
}

impl StrategyTrend {
    /// 从净收益率（百分比）推导趋势
    pub fn from_net_delta(delta_pct: f64) -> Self {
        if delta_pct > 5.0 {
            Self::Up
        } else if delta_pct < -5.0 {
            Self::Down
        } else {
            Self::Stable
        }
    }

    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "上升",
            Self::Down => "下降",
            Self::Stable => "稳定",
        }
    }
}

/// 反馈趋势（分析师胜率变化）
///
/// 用于 backtest_feedback 反馈分析：
/// - `Improving`: 分析师胜率改善
/// - `Declining`: 分析师胜率下降
/// - `Stable`: 分析师胜率稳定
/// - `InsufficientData`: 数据不足
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTrend {
    Improving,
    Declining,
    Stable,
    InsufficientData,
}

impl std::fmt::Display for StrategyTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl FeedbackTrend {
    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Improving => "改善",
            Self::Declining => "下降",
            Self::Stable => "稳定",
            Self::InsufficientData => "数据不足",
        }
    }
}

impl std::fmt::Display for FeedbackTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 建议调整类型
///
/// 用于 backtest_feedback 对分析师的改进建议：
/// - `None`: 无需调整
/// - `AdjustWeight`: 仅调整权重
/// - `TweakPrompt`: 微调提示词
/// - `ReviewLogic`: 重构分析逻辑
/// - `Disable`: 暂时禁用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    None,
    AdjustWeight,
    TweakPrompt,
    ReviewLogic,
    Disable,
}

impl SuggestionType {
    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "无需调整",
            Self::AdjustWeight => "调整权重",
            Self::TweakPrompt => "微调提示词",
            Self::ReviewLogic => "重构逻辑",
            Self::Disable => "暂时禁用",
        }
    }
}

impl std::fmt::Display for SuggestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 分析师立场
///
/// 用于 backtest_feedback 分析师立场分析：
/// - `Bullish`: 看涨
/// - `Bearish`: 看跌
/// - `Neutral`: 中性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalystStance {
    Bullish,
    Bearish,
    Neutral,
}

impl AnalystStance {
    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Bullish => "看涨",
            Self::Bearish => "看跌",
            Self::Neutral => "中性",
        }
    }
}

impl std::fmt::Display for AnalystStance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mos_level_from_pct() {
        assert_eq!(MosLevel::from_mos_pct(35.0), MosLevel::Adequate);
        assert_eq!(MosLevel::from_mos_pct(20.0), MosLevel::Moderate);
        assert_eq!(MosLevel::from_mos_pct(5.0), MosLevel::Insufficient);
        assert_eq!(MosLevel::from_mos_pct(0.0), MosLevel::None);
        assert_eq!(MosLevel::from_mos_pct(-10.0), MosLevel::None);
    }

    #[test]
    fn test_f_score_level_from_score() {
        assert_eq!(FScoreLevel::from_score(9), FScoreLevel::Excellent);
        assert_eq!(FScoreLevel::from_score(7), FScoreLevel::Excellent);
        assert_eq!(FScoreLevel::from_score(6), FScoreLevel::Good);
        assert_eq!(FScoreLevel::from_score(5), FScoreLevel::Good);
        assert_eq!(FScoreLevel::from_score(4), FScoreLevel::Fair);
        assert_eq!(FScoreLevel::from_score(3), FScoreLevel::Fair);
        assert_eq!(FScoreLevel::from_score(2), FScoreLevel::Weak);
        assert_eq!(FScoreLevel::from_score(0), FScoreLevel::Weak);
    }

    #[test]
    fn test_moat_level_from_score() {
        assert_eq!(MoatLevel::from_score(80), MoatLevel::Wide);
        assert_eq!(MoatLevel::from_score(70), MoatLevel::Wide);
        assert_eq!(MoatLevel::from_score(50), MoatLevel::Narrow);
        assert_eq!(MoatLevel::from_score(40), MoatLevel::Narrow);
        assert_eq!(MoatLevel::from_score(30), MoatLevel::None);
        assert_eq!(MoatLevel::from_score(0), MoatLevel::None);
    }

    #[test]
    fn test_value_signal_from_score() {
        assert_eq!(ValueSignal::from_score(70), ValueSignal::Undervalued);
        assert_eq!(ValueSignal::from_score(60), ValueSignal::Undervalued);
        assert_eq!(ValueSignal::from_score(50), ValueSignal::SlightlyUndervalued);
        assert_eq!(ValueSignal::from_score(45), ValueSignal::SlightlyUndervalued);
        assert_eq!(ValueSignal::from_score(35), ValueSignal::Fair);
        assert_eq!(ValueSignal::from_score(30), ValueSignal::Fair);
        assert_eq!(ValueSignal::from_score(20), ValueSignal::SlightlyOvervalued);
        assert_eq!(ValueSignal::from_score(15), ValueSignal::SlightlyOvervalued);
        assert_eq!(ValueSignal::from_score(10), ValueSignal::Overvalued);
        assert_eq!(ValueSignal::from_score(0), ValueSignal::Overvalued);
    }

    #[test]
    fn test_volatility_from_pct() {
        assert_eq!(VolatilityLevel::from_bollinger_pct(0.25), VolatilityLevel::High);
        assert_eq!(VolatilityLevel::from_bollinger_pct(0.15), VolatilityLevel::Normal);
        assert_eq!(VolatilityLevel::from_bollinger_pct(0.05), VolatilityLevel::Low);
    }

    #[test]
    fn test_buffett_verdict() {
        assert_eq!(BuffettVerdict::from_metrics(80, 8, 25.0), BuffettVerdict::HighlyAttractive);
        assert_eq!(BuffettVerdict::from_metrics(55, 5, 12.0), BuffettVerdict::Attractive);
        assert_eq!(BuffettVerdict::from_metrics(35, 4, 5.0), BuffettVerdict::NeedsMoreMargin);
        assert_eq!(BuffettVerdict::from_metrics(20, 2, -5.0), BuffettVerdict::DoesNotMeetStandard);
    }

    #[test]
    fn test_serde_roundtrip() {
        let level = MosLevel::Adequate;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"充足\"");
        let back: MosLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MosLevel::Adequate);

        let signal = ValueSignal::Fair;
        let json = serde_json::to_string(&signal).unwrap();
        assert_eq!(json, "\"合理\"");
        let back: ValueSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ValueSignal::Fair);

        let regime = MarketRegimeType::Bull;
        let json = serde_json::to_string(&regime).unwrap();
        assert_eq!(json, "\"bull\"");
        let back: MarketRegimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MarketRegimeType::Bull);
    }
}
