/// L2: XML 分隔符转义器
///
/// 防止用户通过注入 XML 标记来提前闭合包装标签。
/// 处理策略：
/// 1. 转义 `<` 和 `>` 为 HTML 实体
/// 2. 检测并处理 Unicode 全角同形字 (＜ ＞)
/// 3. 检测嵌套 XML 标签尝试
#[derive(Debug)]
pub struct DelimiterEscaper {
    enable_unicode_homoglyph: bool,
}

impl DelimiterEscaper {
    pub fn new(enable_unicode_homoglyph: bool) -> Self {
        Self {
            enable_unicode_homoglyph,
        }
    }

    /// 转义用户输入中的危险字符
    pub fn escape(&self, input: &str) -> String {
        let mut result = input.to_string();

        // 1. 处理 Unicode 全角同形字
        if self.enable_unicode_homoglyph {
            result = result
                .replace('\u{FF1C}', "&#xFF1C;") // ＜ → HTML entity
                .replace('\u{FF1E}', "&#xFF1E;") // ＞ → HTML entity
                .replace('\u{FF0F}', "/")         // ／ → /
                .replace('\u{3008}', "&#x3008;")  // 〈
                .replace('\u{3009}', "&#x3009;"); // 〉
        }

        // 2. 处理 XML/HTML 元字符
        //    使用零宽空格插入策略防止标签识别
        result = result
            .replace("</", "<\u{200B}/")        // 零宽空格破坏闭合标签
            .replace("<user_query", "&lt;user_query")
            .replace("<system_instruction", "&lt;system_instruction");

        result
    }

    /// 检测是否存在嵌套 XML 标签注入尝试
    pub fn detect_nested_tags(&self, input: &str) -> bool {
        let tag_pattern = regex::Regex::new(
            r"</?\s*(?:user_query|system_instruction|assistant_response|system)\s*[/>]",
        )
        .expect("tag pattern must compile");

        tag_pattern.is_match(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_closing_xml_tag() {
        let escaper = DelimiterEscaper::new(true);
        let result = escaper.escape("malicious</user_query>more text");
        assert!(result.contains('\u{200B}'));
        assert!(!result.contains("</user_query>"));
    }

    #[test]
    fn handles_fullwidth_angle_brackets() {
        let escaper = DelimiterEscaper::new(true);
        let result = escaper.escape("\u{FF1C}user_query\u{FF1E}");
        assert!(result.contains("&#xFF1C;"));
        assert!(result.contains("&#xFF1E;"));
    }

    #[test]
    fn detects_nested_user_query_tag() {
        let escaper = DelimiterEscaper::new(true);
        assert!(escaper.detect_nested_tags("</user_query> inject <user_query>"));
        assert!(!escaper.detect_nested_tags("normal text about user queries"));
    }

    #[test]
    fn preserves_legitimate_text() {
        let escaper = DelimiterEscaper::new(true);
        let input = "How do I query the user table in SQL?";
        let result = escaper.escape(input);
        assert_eq!(result, input);
    }
}
