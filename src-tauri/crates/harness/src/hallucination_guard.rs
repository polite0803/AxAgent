use serde::{Deserialize, Serialize};

/// 防幻觉锚定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationGuardConfig {
    pub enabled: bool,
    /// 引用匹配阈值（0-1），低于此值判定为幻觉
    pub match_threshold: f64,
}

impl Default for HallucinationGuardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            match_threshold: 0.5,
        }
    }
}

/// 锚定检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub fn check_anchor(output: &str, source_context: &str, threshold: f64) -> AnchorResult {
    let sentences = split_sentences(output);
    let mut unverified = Vec::new();
    let mut verified_count = 0usize;

    for sentence in &sentences {
        let trimmed = sentence.trim();
        if trimmed.len() < 10 {
            continue;
        }

        // 提取句子中的关键术语（长度 > 3 的词）
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let key_terms: Vec<&str> = words.iter().filter(|w| w.len() > 3).copied().collect();

        if key_terms.is_empty() {
            continue;
        }

        // 检查每个关键术语是否出现在 source_context 中
        let source_match = key_terms
            .iter()
            .filter(|w| source_context.contains(*w))
            .count();
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

fn split_sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
