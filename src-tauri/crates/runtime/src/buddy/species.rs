//! 物种定义 — 12 种 Buddy 物种，每种有不同的特性和初始属性
//!
//! 参考 claude-code-main 的 Buddy/Companion 陪伴系统设计

use serde::{Deserialize, Serialize};

/// 稀有度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rarity {
    /// 普通 — 最常见的物种
    Common,
    /// 罕见 — 稍微少见
    Uncommon,
    /// 稀有 — 不太常见
    Rare,
    /// 史诗 — 非常稀有
    Epic,
    /// 传说 — 极难获得
    Legendary,
}

/// Buddy 物种定义
///
/// 每个物种有唯一的 `id`、emoji 图标、描述、稀有度和初始属性值。
/// 属性数组顺序：[Debugging, Patience, Chaos, Wisdom, Snark]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDef {
    /// 物种标识符
    pub id: &'static str,
    /// 中文名称
    pub name: &'static str,
    /// emoji 图标
    pub emoji: &'static str,
    /// 物种描述
    pub description: &'static str,
    /// 稀有度
    pub rarity: Rarity,
    /// 初始属性值 [Debugging, Patience, Chaos, Wisdom, Snark]
    pub base_stats: [u8; 5],
}

/// 所有可用的 Buddy 物种列表
pub const ALL_SPECIES: &[SpeciesDef] = &[
    // --- Common ---
    SpeciesDef {
        id: "duck",
        name: "鸭子",
        emoji: "🦆",
        description: "可靠的工作伙伴，善于调试",
        rarity: Rarity::Common,
        base_stats: [4, 3, 2, 3, 2],
    },
    SpeciesDef {
        id: "cat",
        name: "猫咪",
        emoji: "🐱",
        description: "优雅但有时不太合作",
        rarity: Rarity::Common,
        base_stats: [3, 2, 4, 3, 4],
    },
    SpeciesDef {
        id: "dog",
        name: "小狗",
        emoji: "🐕",
        description: "忠诚热情，但容易分心",
        rarity: Rarity::Common,
        base_stats: [2, 5, 2, 2, 1],
    },
    // --- Uncommon ---
    SpeciesDef {
        id: "owl",
        name: "猫头鹰",
        emoji: "🦉",
        description: "博学多才，擅长分析",
        rarity: Rarity::Uncommon,
        base_stats: [3, 4, 1, 5, 2],
    },
    SpeciesDef {
        id: "fox",
        name: "狐狸",
        emoji: "🦊",
        description: "聪明机智，善于找捷径",
        rarity: Rarity::Uncommon,
        base_stats: [4, 2, 3, 4, 3],
    },
    SpeciesDef {
        id: "octopus",
        name: "章鱼",
        emoji: "🐙",
        description: "多任务处理大师",
        rarity: Rarity::Uncommon,
        base_stats: [4, 3, 3, 4, 3],
    },
    SpeciesDef {
        id: "panda",
        name: "熊猫",
        emoji: "🐼",
        description: "慢条斯理但很可靠",
        rarity: Rarity::Uncommon,
        base_stats: [3, 4, 1, 3, 2],
    },
    // --- Rare ---
    SpeciesDef {
        id: "dragon",
        name: "小龙",
        emoji: "🐉",
        description: "强大的守护者",
        rarity: Rarity::Rare,
        base_stats: [5, 3, 4, 4, 3],
    },
    SpeciesDef {
        id: "robot",
        name: "机器人",
        emoji: "🤖",
        description: "精确高效，逻辑严密",
        rarity: Rarity::Rare,
        base_stats: [5, 4, 1, 3, 1],
    },
    SpeciesDef {
        id: "ghost",
        name: "幽灵",
        emoji: "👻",
        description: "神秘莫测，忽隐忽现",
        rarity: Rarity::Rare,
        base_stats: [2, 2, 5, 3, 4],
    },
    // --- Epic ---
    SpeciesDef {
        id: "unicorn",
        name: "独角兽",
        emoji: "🦄",
        description: "魔法般的存在",
        rarity: Rarity::Epic,
        base_stats: [4, 4, 3, 5, 2],
    },
    SpeciesDef {
        id: "phoenix",
        name: "凤凰",
        emoji: "🐦‍🔥",
        description: "浴火重生，永不言弃",
        rarity: Rarity::Epic,
        base_stats: [3, 5, 3, 4, 2],
    },
];

/// 根据物种 ID 查找物种定义
pub fn find_species(id: &str) -> Option<&'static SpeciesDef> {
    ALL_SPECIES.iter().find(|s| s.id == id)
}

/// 随机选择一个物种（基于时间）
pub fn random_species() -> &'static SpeciesDef {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    &ALL_SPECIES[(nanos as usize) % ALL_SPECIES.len()]
}
