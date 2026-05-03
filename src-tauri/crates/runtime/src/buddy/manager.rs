//! Buddy 管理器 — 选择、配置、升级陪伴角色
//!
//! 负责 Buddy 的召唤、经验管理、消息生成等功能

use serde::{Deserialize, Serialize};

use super::attributes::{Attribute, Attributes};
use super::species::{find_species, random_species, SpeciesDef};

/// Buddy 实例 — 用户的陪伴角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buddy {
    /// 物种 ID（如 "duck", "dragon"）
    pub species_id: String,
    /// Buddy 的显示名称（emoji + 名称）
    pub name: String,
    /// 当前属性值
    pub attributes: Attributes,
    /// 累计经验值
    pub xp: u32,
    /// 当前等级
    pub level: u32,
}

/// Buddy 发出的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddyMessage {
    /// Buddy 名称
    pub buddy_name: String,
    /// 物种 emoji
    pub emoji: String,
    /// 消息文本
    pub text: String,
    /// 当前心情
    pub mood: BuddyMood,
}

/// Buddy 的心情状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuddyMood {
    /// 开心
    Happy,
    /// 自豪
    Proud,
    /// 好奇
    Curious,
    /// 毒舌
    Snarky,
    /// 担忧
    Concerned,
    /// 兴奋
    Excited,
}

/// Buddy 消息触发上下文
///
/// 决定 Buddy 何时说话以及说什么
pub enum BuddyContext {
    /// 任务完成（成功/失败）
    TaskCompleted { success: bool },
    /// 发现 Bug
    BugFound,
    /// 代码已写入
    CodeWritten,
    /// 发生错误
    ErrorOccurred,
    /// 空闲状态
    Idle,
    /// 启动时
    Startup,
}

/// Buddy 管理器
///
/// 每个用户只能拥有一个 Buddy。管理器负责 Buddy 的生命周期。
pub struct BuddyManager {
    /// 当前激活的 Buddy（无则为 None）
    pub buddy: Option<Buddy>,
}

impl BuddyManager {
    /// 创建一个空的 Buddy 管理器
    pub fn new() -> Self {
        Self { buddy: None }
    }

    /// 随机召唤一个 Buddy（基于时间种子）
    pub fn summon_random(&mut self) -> &Buddy {
        let species = random_species();
        let buddy = Buddy {
            species_id: species.id.to_string(),
            name: format!("{}{}", species.emoji, species.name),
            attributes: Attributes::new(species.base_stats),
            xp: 0,
            level: 1,
        };
        self.buddy = Some(buddy);
        self.buddy.as_ref().unwrap()
    }

    /// 按物种 ID 召唤特定 Buddy
    ///
    /// 返回 None 如果物种 ID 无效
    pub fn summon_species(&mut self, species_id: &str) -> Option<&Buddy> {
        let species = find_species(species_id)?;
        let buddy = Buddy {
            species_id: species.id.to_string(),
            name: format!("{}{}", species.emoji, species.name),
            attributes: Attributes::new(species.base_stats),
            xp: 0,
            level: 1,
        };
        self.buddy = Some(buddy);
        self.buddy.as_ref()
    }

    /// 获取当前 Buddy 的引用
    pub fn current(&self) -> Option<&Buddy> {
        self.buddy.as_ref()
    }

    /// 给予 Buddy 经验值
    ///
    /// 每累积 100 XP 升一级，升级时随机提升一项属性
    pub fn grant_xp(&mut self, amount: u32) {
        if let Some(ref mut buddy) = self.buddy {
            buddy.xp += amount;
            // 每 100 XP 升一级
            let new_level = buddy.xp / 100 + 1;
            if new_level > buddy.level {
                buddy.level = new_level;
                // 随机升级一个属性（基于 XP 总量取模伪随机选择）
                let attrs = Attribute::all();
                let idx = (buddy.xp as usize) % attrs.len();
                buddy.attributes.level_up(attrs[idx]);
            }
        }
    }

    /// 根据上下文生成 Buddy 消息
    ///
    /// 返回 None 如果当前没有激活的 Buddy
    pub fn generate_message(&self, context: &BuddyContext) -> Option<BuddyMessage> {
        let buddy = self.buddy.as_ref()?;
        let species = find_species(&buddy.species_id)?;

        let mood = match context {
            BuddyContext::TaskCompleted { success } => {
                if *success {
                    BuddyMood::Proud
                } else {
                    BuddyMood::Concerned
                }
            }
            BuddyContext::BugFound => BuddyMood::Curious,
            BuddyContext::CodeWritten => BuddyMood::Happy,
            BuddyContext::ErrorOccurred => BuddyMood::Concerned,
            BuddyContext::Idle => BuddyMood::Curious,
            BuddyContext::Startup => BuddyMood::Excited,
        };

        let text = self.pick_message(species, mood);

        Some(BuddyMessage {
            buddy_name: buddy.name.clone(),
            emoji: species.emoji.to_string(),
            text,
            mood,
        })
    }

    /// 根据物种和心情选择一条消息
    fn pick_message(&self, species: &SpeciesDef, mood: BuddyMood) -> String {
        let messages = match mood {
            BuddyMood::Happy => vec![
                format!("{} 干得好！", species.emoji),
                format!("{} 一切顺利~", species.emoji),
                format!("{} 这感觉不错！", species.emoji),
            ],
            BuddyMood::Proud => vec![
                format!("{} 看，我们做到了！", species.emoji),
                format!("{} 代码质量杠杠的！", species.emoji),
                format!("{} 完美运行！", species.emoji),
            ],
            BuddyMood::Curious => vec![
                format!("{} 这里有点意思...", species.emoji),
                format!("{} 让我看看...", species.emoji),
                format!("{} 嗯？这行代码在做什么？", species.emoji),
            ],
            BuddyMood::Snarky => vec![
                format!("{} 这代码...咳咳", species.emoji),
                format!("{} 我觉得可以写得更好~", species.emoji),
                format!("{} 又是一个 TODO？", species.emoji),
            ],
            BuddyMood::Concerned => vec![
                format!("{} 等等，好像不对...", species.emoji),
                format!("{} 注意这里！", species.emoji),
                format!("{} 需要检查一下。", species.emoji),
            ],
            BuddyMood::Excited => vec![
                format!("{} 开始工作了！", species.emoji),
                format!("{} 今天也是充满活力的一天！", species.emoji),
                format!("{} 冲啊！", species.emoji),
            ],
        };

        // 使用系统时间的纳秒做伪随机选择，避免额外依赖 rand
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let idx = (nanos as usize) % messages.len();
        messages[idx].clone()
    }
}

impl Default for BuddyManager {
    fn default() -> Self {
        Self::new()
    }
}
