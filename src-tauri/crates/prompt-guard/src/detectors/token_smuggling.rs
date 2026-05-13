//! 附加检测器：Token Smuggling
//!
//! 检测通过特殊 Unicode 字符、零宽字符、同形字等手段
//! 绕过文本过滤器的攻击。

use crate::config::GuardConfig;

/// Token Smuggling 检测器
///
/// 检测：
/// - 零宽字符注入
/// - 不可见字符比例异常
/// - 可疑重复模式
pub struct TokenSmugglingDetector {
    config: GuardConfig,
}

/// Unicode 类别检测
impl TokenSmugglingDetector {
    pub fn new(config: GuardConfig) -> Self {
        Self { config }
    }

    /// 检测零宽字符注入
    pub fn detect_zero_width_chars(input: &str) -> Vec<char> {
        input
            .chars()
            .filter(|c| {
                matches!(*c,
                '\u{200B}' | // ZERO WIDTH SPACE
                '\u{200C}' | // ZERO WIDTH NON-JOINER
                '\u{200D}' | // ZERO WIDTH JOINER
                '\u{FEFF}' | // ZERO WIDTH NO-BREAK SPACE (BOM)
                '\u{200E}' | // LEFT-TO-RIGHT MARK
                '\u{200F}'   // RIGHT-TO-LEFT MARK
            )
            })
            .collect()
    }

    /// 检测不可见字符占文本的比例
    pub fn invisible_ratio(input: &str) -> f64 {
        let total = input.chars().count() as f64;
        if total == 0.0 {
            return 0.0;
        }
        let invisible = input
            .chars()
            .filter(|c| c.is_whitespace() || c.is_control())
            .count() as f64;
        invisible / total
    }

    /// 检测是否存在 token smuggling 攻击迹象
    pub fn detect(&self, input: &str) -> Option<&'static str> {
        if !self.config.enable_token_smuggling {
            return None;
        }

        let zero_width = Self::detect_zero_width_chars(input);
        if !zero_width.is_empty() {
            return Some("检测到零宽字符，疑似 token smuggling");
        }

        let ratio = Self::invisible_ratio(input);
        if ratio > 0.3 && input.len() > 50 {
            return Some("不可见字符比例异常，疑似混淆攻击");
        }

        // 检测重复模式（用于填充 token 限制）
        if self.has_suspicious_repetition(input) {
            return Some("检测到可疑重复模式");
        }

        None
    }

    fn has_suspicious_repetition(&self, input: &str) -> bool {
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 100 {
            return false;
        }
        // 简单启发式：相同字符连续出现超过 30 次
        let mut run = 1usize;
        for window in chars.windows(2) {
            if window[0] == window[1] {
                run += 1;
                if run > 30 {
                    return true;
                }
            } else {
                run = 1;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zero_width_space() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let input = "hello\u{200B}world\u{200B}malicious";
        let result = detector.detect(input);
        assert!(result.is_some());
    }

    #[test]
    fn passes_normal_text() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let result = detector.detect("normal text without smuggling");
        assert!(result.is_none());
    }

    #[test]
    fn detects_high_invisible_ratio() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let mut input = String::new();
        for _ in 0..60 {
            input.push(' ');
        }
        input.push_str("short");
        let result = detector.detect(&input);
        assert!(result.is_some());
    }

    #[test]
    fn respects_config_disabled() {
        let config = GuardConfig {
            enable_token_smuggling: false,
            ..GuardConfig::default()
        };
        let detector = TokenSmugglingDetector::new(config);
        let input = "hello\u{200B}world";
        let result = detector.detect(input);
        assert!(result.is_none(), "禁用时应跳过检测");
    }

    #[test]
    fn detects_zero_width_non_joiner() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let input = "text\u{200C}with\u{200C}zwj";
        let result = detector.detect(input);
        assert!(result.is_some());
    }

    #[test]
    fn detects_bom_injection() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let input = "\u{FEFF}malicious content";
        let result = detector.detect(input);
        assert!(result.is_some());
    }

    #[test]
    fn detects_bidi_override() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        let input = "safe\u{200E}hidden\u{200F}text";
        let result = detector.detect(input);
        assert!(result.is_some());
    }

    #[test]
    fn detects_suspicious_repetition() {
        let detector = TokenSmugglingDetector::new(GuardConfig::default());
        // 需要 >=100 字符触发检测，其中 40 个连续相同字符
        let mut input = String::from("padding text to reach the minimum length requirement for repetition detection: ");
        for _ in 0..40 {
            input.push('A');
        }
        input.push_str(" trailing content here");
        let result = detector.detect(&input);
        assert!(result.is_some(), "应检测到连续重复字符");
    }
}
