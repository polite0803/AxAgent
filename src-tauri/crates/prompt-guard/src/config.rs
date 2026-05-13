//! Configuration types and detection result model for the prompt guard pipeline.

use serde::{Deserialize, Serialize};

/// 防护模式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardMode {
    /// 仅标记，不拦截
    Audit,
    /// 高风险拦截，其他标记
    #[default]
    Standard,
    /// 严格模式，中风险也拦截
    Strict,
}

/// 检测结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionResult {
    /// 安全通过
    Clean,
    /// 已标记（含标记的文本）
    Flagged { text: String, reasons: Vec<String> },
    /// 已拒绝
    Blocked { reason: String },
}

impl DetectionResult {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    pub fn is_flagged(&self) -> bool {
        matches!(self, Self::Flagged { .. })
    }
}

/// 全局防护配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    pub mode: GuardMode,
    /// 自定义高风险模式（追加）
    pub custom_high_patterns: Vec<String>,
    /// 自定义中风险模式（追加）
    pub custom_medium_patterns: Vec<String>,
    /// 是否启用 token smuggling 检测
    pub enable_token_smuggling: bool,
    /// 是否启用 unicode 同形字检测
    pub enable_unicode_homoglyph: bool,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            mode: GuardMode::Standard,
            custom_high_patterns: Vec::new(),
            custom_medium_patterns: Vec::new(),
            enable_token_smuggling: true,
            enable_unicode_homoglyph: true,
        }
    }
}
