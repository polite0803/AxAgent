//! Lightweight token estimation for context window management.
//!
//! Uses character-based heuristics rather than a full tokenizer to keep
//! dependencies minimal. Accuracy is sufficient for threshold detection
//! (within ~15% of actual token counts for mixed-language text).

/// Estimate the number of tokens in a text string.
///
/// Heuristic:
/// - ASCII / Latin characters: ~4 characters per token
/// - CJK / fullwidth characters: ~1.5 characters per token
/// - Each message carries ~4 tokens of overhead (role, delimiters)
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut ascii_chars: usize = 0;
    let mut cjk_chars: usize = 0;

    for ch in text.chars() {
        if is_cjk(ch) {
            cjk_chars += 1;
        } else {
            ascii_chars += 1;
        }
    }

    // English-ish: ~4 chars/token; CJK: ~1.5 chars/token
    let ascii_tokens = ascii_chars.div_ceil(4); // ceil division
    let cjk_tokens = (cjk_chars * 2).div_ceil(3); // ceil(cjk * 2/3)

    ascii_tokens + cjk_tokens
}

/// Estimate tokens for an entire chat message (content + role overhead).
pub fn estimate_message_tokens(role: &str, content: &str) -> usize {
    const PER_MESSAGE_OVERHEAD: usize = 4; // role, delimiters, etc.
    estimate_tokens(role) + estimate_tokens(content) + PER_MESSAGE_OVERHEAD
}

/// Check if a character is in a CJK Unicode block.
fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{F900}'..='\u{FAFF}' // CJK Compat Ideographs
        | '\u{3000}'..='\u{303F}' // CJK Symbols and Punctuation
        | '\u{FF00}'..='\u{FFEF}' // Halfwidth and Fullwidth Forms
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_english_text() {
        // "Hello, world!" = 13 chars → ~3 tokens (actual: 4 with GPT-4)
        let tokens = estimate_tokens("Hello, world!");
        assert!(tokens >= 2 && tokens <= 6);
    }

    #[test]
    fn test_chinese_text() {
        // "你好世界" = 4 CJK chars → ~3 tokens (actual: 2-4 with GPT-4)
        let tokens = estimate_tokens("你好世界");
        assert!(tokens >= 2 && tokens <= 5);
    }

    #[test]
    fn test_mixed_text() {
        let tokens = estimate_tokens("Hello 你好 world 世界");
        assert!(tokens >= 4 && tokens <= 10);
    }

    #[test]
    fn test_message_overhead() {
        let tokens = estimate_message_tokens("user", "Hi");
        // "user" + "Hi" + 4 overhead ≈ 6-7
        assert!(tokens >= 5);
    }
}
