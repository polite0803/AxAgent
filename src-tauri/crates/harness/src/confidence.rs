//! 置信度输出类型 — 用于 LLM 分类/判断结果的置信度检查。

use serde::{Deserialize, Serialize};

/// LLM 分类/判断输出带置信度
///
/// LLM 在执行分类或路由判断时，可以通过结构化输出同时返回结果和置信度。
/// 当配置了 `confidence_threshold` 时，执行器会尝试从 LLM 响应中提取此结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceOutput {
    /// 分类/判断结果（如分类名 "positive"、分支名 "true"）
    pub result: serde_json::Value,
    /// 置信度 0.0 - 1.0
    pub confidence: f64,
    /// LLM 推理过程（可选）
    pub reasoning: Option<String>,
}

impl ConfidenceOutput {
    /// 基于原始文本结果创建一个默认置信度为 1.0 的 ConfidenceOutput
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            result: serde_json::Value::String(text.into()),
            confidence: 1.0,
            reasoning: None,
        }
    }

    /// 尝试从 LLM 响应文本中解析 ConfidenceOutput。
    ///
    /// 支持两种格式：
    /// 1. 纯 JSON: `{"result": "category_a", "confidence": 0.95, "reasoning": "..."}`
    /// 2. 含 JSON 块的文本: 从文本中提取第一个 JSON 对象
    pub fn try_parse(response: &str) -> Option<Self> {
        // 先尝试直接解析为 JSON
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(response.trim()) {
            if let Some(result) = value.get("result") {
                let confidence = value.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0);
                let reasoning = value.get("reasoning").and_then(|r| r.as_str()).map(|s| s.to_string());
                return Some(Self {
                    result: result.clone(),
                    confidence,
                    reasoning,
                });
            }
        }

        // 尝试从文本中提取 JSON 块
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let json_str = &response[start..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(result) = value.get("result") {
                        let confidence = value.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0);
                        let reasoning = value.get("reasoning").and_then(|r| r.as_str()).map(|s| s.to_string());
                        return Some(Self {
                            result: result.clone(),
                            confidence,
                            reasoning,
                        });
                    }
                }
            }
        }

        None
    }

    /// 检查置信度是否达到阈值
    pub fn is_confident_enough(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_json() {
        let response = r#"{"result": "positive", "confidence": 0.95, "reasoning": "匹配规则A"}"#;
        let output = ConfidenceOutput::try_parse(response).unwrap();
        assert_eq!(output.result, serde_json::json!("positive"));
        assert!((output.confidence - 0.95).abs() < 1e-6);
        assert_eq!(output.reasoning, Some("匹配规则A".to_string()));
    }

    #[test]
    fn test_parse_json_without_reasoning() {
        let response = r#"{"result": "negative", "confidence": 0.8}"#;
        let output = ConfidenceOutput::try_parse(response).unwrap();
        assert_eq!(output.result, serde_json::json!("negative"));
        assert!((output.confidence - 0.8).abs() < 1e-6);
        assert!(output.reasoning.is_none());
    }

    #[test]
    fn test_parse_embedded_in_text() {
        let response = "分类结果：{\"result\": \"category_b\", \"confidence\": 0.6}";
        let output = ConfidenceOutput::try_parse(response).unwrap();
        assert_eq!(output.result, serde_json::json!("category_b"));
        assert!((output.confidence - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_confidence_threshold_check() {
        let output = ConfidenceOutput {
            result: serde_json::json!("ok"),
            confidence: 0.7,
            reasoning: None,
        };
        assert!(output.is_confident_enough(0.5));
        assert!(!output.is_confident_enough(0.8));
    }

    #[test]
    fn test_from_text_creates_full_confidence() {
        let output = ConfidenceOutput::from_text("hello");
        assert_eq!(output.result, serde_json::json!("hello"));
        assert!((output.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_response_returns_none() {
        assert!(ConfidenceOutput::try_parse("not json at all").is_none());
        assert!(ConfidenceOutput::try_parse("").is_none());
    }
}

/// 低置信度时的行为
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidenceAction {
    /// 拦截 LLM 输出，返回错误
    Block,
    /// 仅记录警告，继续执行
    WarnAndContinue,
    /// 使用默认输出替换 LLM 输出
    FallbackToDefault,
}

/// 置信度配置 — 定义低置信度时的行为和默认输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    /// 低置信度时的处理动作
    pub on_low_confidence: ConfidenceAction,
    /// 默认输出（仅在 FallbackToDefault 时使用）
    pub default_output: Option<String>,
}
