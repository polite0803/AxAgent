// SPDX-License-Identifier: AGPL-3.0-only

//! 纯文本后处理：剔除特殊占位符、多余空行、重复标点等 LLM 输出噪音。
//!
//! 本模块从 `agent::ir_renderer::clean_output` 提取至此，
//! 使 `rt-workflow`（agent_executor）等无法依赖 `agent` crate 的模块也能共享同一套清洗逻辑。

use regex::Regex;

/// 最终文本清理：剔除多余空行、特殊占位符、重复标点。
pub fn clean_output(text: &str) -> String {
    let cleaned = text.to_string();

    // 1. 剔除重复空行（连续 3+ 换行 → 2 换行）
    let re_blank = Regex::new(r"\n{3,}").unwrap();
    let cleaned = re_blank.replace_all(&cleaned, "\n\n");

    // 2. 剔除特殊占位符
    let re_placeholder =
        Regex::new(r"<\|endoftext\|>|<\|im_end\|>|<\|im_start\|>|<s>|</s>").unwrap();
    let cleaned = re_placeholder.replace_all(&cleaned, "");

    // 3. 剔除重复标点（同时匹配 ASCII 和全角）
    let re_repeated_punct = Regex::new(r"([!?！？。，；：])[!?！？。，；：]+").unwrap();
    let cleaned = re_repeated_punct.replace_all(&cleaned, "$1");

    // 4. 行首行尾空白
    let cleaned: String = cleaned
        .lines()
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidates_repeated_blank_lines() {
        let result = clean_output("段落1\n\n\n\n\n段落2");
        assert_eq!(result, "段落1\n\n段落2");
    }

    #[test]
    fn removes_special_placeholders() {
        let result = clean_output("你好<|endoftext|>世界<|im_end|>");
        assert_eq!(result, "你好世界");
    }

    #[test]
    fn deduplicates_repeated_punctuation() {
        let result = clean_output("真的吗！！！哪里？？");
        assert_eq!(result, "真的吗！哪里？");
    }

    #[test]
    fn trims_line_whitespace() {
        let result = clean_output("  第一行  \n  第二行  ");
        assert_eq!(result, "第一行\n第二行");
    }

    #[test]
    fn handles_empty_input() {
        let result = clean_output("");
        assert_eq!(result, "");
    }

    #[test]
    fn removes_im_start_tag() {
        let result = clean_output("<|im_start|>system\n你好<|im_end|>");
        assert_eq!(result, "system\n你好");
    }
}
