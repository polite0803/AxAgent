// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 防幻觉锚定配置
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct HallucinationGuardConfig {
    pub enabled: bool,
    /// 引用匹配阈值（0-1），低于此值判定为幻觉
    pub match_threshold: f64,
}

impl Default for HallucinationGuardConfig {
    fn default() -> Self {
        Self { enabled: false, match_threshold: 0.5 }
    }
}

/// 锚定检查结果
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AnchorResult {
    pub passed: bool,
    pub score: f64,
    pub unverified_claims: Vec<String>,
    pub details: String,
}

/// 检查 LLM 输出中的关键信息是否在源文档中出现
///
/// # 参数
/// - `output`: LLM 的输出文本
/// - `source_context`: 源文档/上下文文本（RAG 检索结果、文档内容等）
/// - `threshold`: 锚定分数阈值（0-1），低于此值判定为幻觉
///
/// # 中文适配
/// 原算法对中文文本失效（split_whitespace 无法分词中文），改进为：
/// 1. 分句识别中文标点（。！？；）
/// 2. 关键术语提取：中文用 2-gram 字符切分，英文用 split_whitespace
/// 3. 匹配时使用子串包含
pub fn check_anchor(output: &str, source_context: &str, threshold: f64) -> AnchorResult {
    let sentences = split_sentences(output);
    let mut unverified = Vec::new();
    let mut verified_count = 0usize;

    for sentence in &sentences {
        let trimmed = sentence.trim();
        if trimmed.chars().count() < 5 {
            continue;
        }

        let key_terms = extract_key_terms(trimmed);
        if key_terms.is_empty() {
            continue;
        }

        // 检查每个关键术语是否出现在 source_context 中
        let source_match = key_terms.iter().filter(|w| source_context.contains(*w)).count();
        let match_rate = source_match as f64 / key_terms.len() as f64;

        if match_rate >= 0.5 {
            verified_count += 1;
        } else {
            unverified.push(trimmed.to_string());
        }
    }

    let total_checked = verified_count + unverified.len();
    let score = if total_checked > 0 {
        verified_count as f64 / total_checked as f64
    } else {
        1.0
    };

    let unverified_count = unverified.len();
    AnchorResult {
        passed: score >= threshold,
        score,
        unverified_claims: unverified,
        details: format!(
            "锚定分数: {:.2} (阈值: {}), 未验证句子: {}",
            score, threshold, unverified_count
        ),
    }
}

/// 分句：识别中英文标点
fn split_sentences(text: &str) -> Vec<String> {
    // 识别 ASCII 标点 + 中文全角标点（。！？；）
    text.split(['.', '!', '?', '\n', '。', '！', '？', '；', '；'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 从句子中提取关键术语
///
/// 中文：提取 2-gram 字符序列（如"紫金矿业"→"紫金","金矿","矿业"）
/// 英文/数字：split_whitespace 后保留 len > 3 的词
fn extract_key_terms(text: &str) -> Vec<String> {
    let mut terms = Vec::new();

    // 1. 英文/数字术语：按空白切分，保留长度 > 3 的词
    for word in text.split_whitespace() {
        if word.len() > 3 && word.chars().any(|c| c.is_ascii_alphanumeric()) {
            terms.push(word.to_string());
        }
    }

    // 2. 中文 2-gram：连续中文字符的相邻 2 字组合
    let chars: Vec<char> = text.chars().filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c)).collect();
    for window in chars.windows(2) {
        let bigram: String = window.iter().collect();
        terms.push(bigram);
    }

    // 去重（保持顺序）
    let mut seen = std::collections::HashSet::new();
    terms.retain(|t| seen.insert(t.clone()));

    terms
}
