//! 属性系统 — Buddy 的 5 种属性，每种 1-10 级
//!
//! 属性包括：调试能力、耐心、混乱度、智慧、毒舌

use serde::{Deserialize, Serialize};

/// Buddy 的五种属性
///
/// 每种属性范围 1-10，影响 Buddy 的行为和对话风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attribute {
    /// 🔧 调试能力 — 影响代码查找和修复的能力
    Debugging,
    /// 🧘 耐心 — 影响 Buddy 的耐心程度
    Patience,
    /// 🌪️ 混乱度 — 影响随机行为和意外事件的概率
    Chaos,
    /// 📚 智慧 — 影响建议质量和洞察力
    Wisdom,
    /// 😏 毒舌 — 影响吐槽频率和犀利度
    Snark,
}

impl Attribute {
    /// 获取属性的中文名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Debugging => "调试",
            Self::Patience => "耐心",
            Self::Chaos => "混乱",
            Self::Wisdom => "智慧",
            Self::Snark => "毒舌",
        }
    }

    /// 获取属性的 emoji 图标
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Debugging => "🔧",
            Self::Patience => "🧘",
            Self::Chaos => "🌪️",
            Self::Wisdom => "📚",
            Self::Snark => "😏",
        }
    }

    /// 返回所有属性类型的数组
    pub fn all() -> &'static [Attribute] {
        &[
            Self::Debugging,
            Self::Patience,
            Self::Chaos,
            Self::Wisdom,
            Self::Snark,
        ]
    }
}

/// Buddy 的属性值集合
///
/// 每个属性值范围 1-10，从物种的初始属性开始，通过升级不断提升
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    /// 调试能力 (1-10)
    pub debugging: u8,
    /// 耐心 (1-10)
    pub patience: u8,
    /// 混乱度 (1-10)
    pub chaos: u8,
    /// 智慧 (1-10)
    pub wisdom: u8,
    /// 毒舌 (1-10)
    pub snark: u8,
}

impl Attributes {
    /// 从初始属性数组创建属性值
    ///
    /// 数组顺序：[Debugging, Patience, Chaos, Wisdom, Snark]
    pub fn new(stats: [u8; 5]) -> Self {
        Self {
            debugging: stats[0],
            patience: stats[1],
            chaos: stats[2],
            wisdom: stats[3],
            snark: stats[4],
        }
    }

    /// 获取指定属性的当前值
    pub fn get(&self, attr: Attribute) -> u8 {
        match attr {
            Attribute::Debugging => self.debugging,
            Attribute::Patience => self.patience,
            Attribute::Chaos => self.chaos,
            Attribute::Wisdom => self.wisdom,
            Attribute::Snark => self.snark,
        }
    }

    /// 升级某个属性（最高 10 级）
    pub fn level_up(&mut self, attr: Attribute) {
        let val = match attr {
            Attribute::Debugging => &mut self.debugging,
            Attribute::Patience => &mut self.patience,
            Attribute::Chaos => &mut self.chaos,
            Attribute::Wisdom => &mut self.wisdom,
            Attribute::Snark => &mut self.snark,
        };
        *val = (*val + 1).min(10);
    }

    /// 获取总经验值（所有属性之和）
    pub fn total_xp(&self) -> u32 {
        (self.debugging + self.patience + self.chaos + self.wisdom + self.snark) as u32
    }

    /// 获取等级（基于总经验值）
    ///
    /// 等级 = 总经验 / 5 + 1
    pub fn level(&self) -> u32 {
        self.total_xp() / 5 + 1
    }
}
