//! Vendor As-Of 能力抽象(vendor trait 大重构 §2.1)
//!
//! 每个 (vendor, method) 组合在 vendor 实现中申报自己的 as-of 处理能力,
//! lib.rs 路由层查表决策走哪条路(live / native / synthesize / cache / fallthrough)。
//!
//! 设计目标:
//! - vendor 内部完全掌控 as-of 路径(传 begin/end 参数、调本地缓存、合成 K 线等)
//! - lib.rs 决策简单(查表 + 分发)
//! - 老 vendor 零改动可继续工作(默认 Fallthrough → lib.rs 截断兜底)

use serde::{Deserialize, Serialize};

/// Vendor 声明自己 (method) 的 as-of 处理能力
///
/// 4 变体语义:
/// - `NativeDateParam`    — vendor 原生支持日期参数,as-of 模式调 `*_with_asof`
/// - `SynthesizeFromKline` — 用 K 线最后一行合成(典型:实时报价类)
/// - `NoHistoricalSemantic` — 概念性数据(热门股/概念板块/行业排名),无历史
///   as-of 模式查本地 SQLite 缓存;cache miss → 显式 record_degradation
/// - `Fallthrough`        — vendor 不支持,接受 "vendor 返回全量 + lib.rs 截断" 兜底
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AsOfCapability {
    /// vendor 原生支持日期参数
    /// as-of 模式调 `vendor.method_with_asof(...)`,vendor 内部自动加 begin/end
    NativeDateParam,

    /// 用 K 线最后一行合成(quote_from_klines 模式)
    /// as-of 模式拉 K 线,取 <= effective_cutoff 的最后一行作为 quote
    SynthesizeFromKline,

    /// 无历史语义(当下榜单/分类/排名/对比)
    /// as-of 模式查本地 SQLite 缓存;cache miss 时返回空 + record_degradation
    NoHistoricalSemantic,

    /// vendor 不支持,走 "vendor 返回全量 + lib.rs 截断" 兜底
    /// 仅作过渡,新 vendor 适配完成后应转为其他 3 个变体之一
    #[default]
    Fallthrough,
}

impl AsOfCapability {
    /// 简短标签,用于日志和降级报告
    pub fn label(&self) -> &'static str {
        match self {
            AsOfCapability::NativeDateParam => "native",
            AsOfCapability::SynthesizeFromKline => "synth_kline",
            AsOfCapability::NoHistoricalSemantic => "no_history",
            AsOfCapability::Fallthrough => "fallthrough",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_variants_distinct() {
        let all = [
            AsOfCapability::NativeDateParam,
            AsOfCapability::SynthesizeFromKline,
            AsOfCapability::NoHistoricalSemantic,
            AsOfCapability::Fallthrough,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn label_is_stable_string() {
        // 标签用于持久化降级报告,字符串必须稳定
        assert_eq!(AsOfCapability::NativeDateParam.label(), "native");
        assert_eq!(AsOfCapability::SynthesizeFromKline.label(), "synth_kline");
        assert_eq!(AsOfCapability::NoHistoricalSemantic.label(), "no_history");
        assert_eq!(AsOfCapability::Fallthrough.label(), "fallthrough");
    }

    #[test]
    fn serialize_roundtrip() {
        // capability 序列化用于降级日志,保证可往返
        for cap in [
            AsOfCapability::NativeDateParam,
            AsOfCapability::SynthesizeFromKline,
            AsOfCapability::NoHistoricalSemantic,
            AsOfCapability::Fallthrough,
        ] {
            let s = serde_json::to_string(&cap).unwrap();
            let back: AsOfCapability = serde_json::from_str(&s).unwrap();
            assert_eq!(cap, back);
        }
    }

    #[test]
    fn default_is_fallthrough() {
        // 现有 vendor 不申报时默认 Fallthrough(向后兼容)
        // 在 trait 默认实现中保证
        let cap: AsOfCapability = Default::default();
        assert_eq!(cap, AsOfCapability::Fallthrough);
    }
}
