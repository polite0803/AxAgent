//! VLM 截图导入持仓 (P1-2)
//!
//! 借鉴 TradingAgents-AShare 的 VLM 截图导入功能:
//! 通过视觉语言模型解析持仓截图，自动提取股票代码、数量、价格等信息。
//!
//! ## 设计
//!
//! 纯解析层：接收 VLM 返回的原始文本，解析为结构化持仓数据。
//! 实际的 VLM API 调用在 commands 层完成（需要访问 AppState 和 provider 配置）。
//!
//! 支持两种截图类型：
//! - 标准券商持仓截图（东方财富/华泰/同花顺等）
//! - 手动标注截图（用户可以圈出需要导入的区域）

use serde::{Deserialize, Serialize};

/// 单条解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedHolding {
    /// 股票代码（如 "000001"）
    pub stock_code: String,
    /// 股票名称
    pub stock_name: String,
    /// 持有数量（股）
    pub shares: f64,
    /// 成本价
    pub avg_cost: f64,
    /// 当前价（可选，截图可能有）
    pub current_price: Option<f64>,
    /// 持仓市值（可选）
    pub market_value: Option<f64>,
    /// 盈亏比例%（可选）
    pub pnl_pct: Option<f64>,
}

/// VLM 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VlmParseResult {
    /// 是否成功
    pub success: bool,
    /// 解析出的持仓列表
    pub holdings: Vec<ParsedHolding>,
    /// 错误信息（解析失败时）
    pub error: Option<String>,
    /// 识别来源类型
    pub source_type: String,
    /// VLM 原始输出（用于调试）
    pub raw_output: String,
}

/// 用于 VLM 的 system prompt
pub const VLM_SYSTEM_PROMPT: &str = r#"你是一个专业的A股券商持仓截图解析助手。
请从截图中识别所有持仓信息，返回严格JSON格式。

输出格式（JSON数组）：
```json
[
  {
    "stock_code": "股票代码",
    "stock_name": "股票名称",
    "shares": 持有数量(数字),
    "avg_cost": 成本价(数字),
    "current_price": 当前价(数字或null),
    "market_value": 持仓市值(数字或null),
    "pnl_pct": 盈亏比例(数字或null)
  }
]
```

规则：
1. 股票代码格式：6位数字，如"000001"、"600519"
2. 如果代码为纯数字，前面不要加字母前缀
3. 数量单位为"股"，如有"万"则乘以10000
4. 价格单位为"元"
5. 如果截图中有多个表格/区域，只解析持仓部分
6. 如果无法识别任何持仓，返回空数组
7. 不要添加注释，只返回JSON
8. 严格只输出JSON数组，不要Markdown包裹（除非需要转义）"#;

/// 解析 VLM 返回的文本为结构化的持仓数据
pub fn parse_vlm_output(raw_text: &str) -> VlmParseResult {
    // 1. 尝试提取 JSON 数组
    let json_str = extract_json_array(raw_text);
    let text_for_error = raw_text.to_string();

    match json_str {
        Some(json_text) => {
            // 尝试解析 JSON
            match serde_json::from_str::<Vec<ParsedHolding>>(&json_text) {
                Ok(holdings) => {
                    let cleaned: Vec<ParsedHolding> = holdings
                        .into_iter()
                        .filter(|h| !h.stock_code.is_empty() && h.shares > 0.0)
                        .collect();

                    if cleaned.is_empty() {
                        VlmParseResult {
                            success: false,
                            holdings: vec![],
                            error: Some("VLM 返回了空持仓列表".into()),
                            source_type: "vlm".into(),
                            raw_output: text_for_error,
                        }
                    } else {
                        VlmParseResult {
                            success: true,
                            holdings: cleaned,
                            error: None,
                            source_type: "vlm".into(),
                            raw_output: text_for_error,
                        }
                    }
                },
                Err(e) => VlmParseResult {
                    success: false,
                    holdings: vec![],
                    error: Some(format!("JSON 解析失败: {e}")),
                    source_type: "vlm".into(),
                    raw_output: text_for_error,
                },
            }
        },
        None => {
            // 没有 JSON 数组，尝试从纯文本中提取
            VlmParseResult {
                success: false,
                holdings: vec![],
                error: Some("未在 VLM 输出中找到有效的 JSON 数组".into()),
                source_type: "vlm".into(),
                raw_output: text_for_error,
            }
        },
    }
}

/// 从 VLM 输出中提取 JSON 数组
fn extract_json_array(text: &str) -> Option<String> {
    let trimmed = text.trim();

    // 尝试直接解析整个文本
    if trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }

    // 尝试从 Markdown code block 中提取
    if let Some(start) = trimmed.find("```") {
        let after_start = &trimmed[start + 3..].trim();
        let after_lang = if let Some(stripped) = after_start.strip_prefix("json") {
            stripped
        } else {
            after_start
        };
        if let Some(end) = after_lang.find("```") {
            let json = after_lang[..end].trim();
            if json.starts_with('[') {
                return Some(json.to_string());
            }
        }
    }

    // 尝试找第一个 [ 和最后一个 ]
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return Some(trimmed[start..=end].to_string());
            }
        }
    }

    None
}

/// 生成批量导入命令的参数列表
pub fn holdings_to_import_params(holdings: &[ParsedHolding]) -> Vec<ImportHoldingParam> {
    holdings
        .iter()
        .map(|h| ImportHoldingParam {
            stock_code: h.stock_code.clone(),
            stock_name: h.stock_name.clone(),
            shares: h.shares,
            avg_cost: h.avg_cost,
        })
        .collect()
}

/// 批量导入参数（用于前端循环调用 add_portfolio_holding）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportHoldingParam {
    pub stock_code: String,
    pub stock_name: String,
    pub shares: f64,
    pub avg_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json_array() {
        let input = r#"[
            {"stockCode": "600519", "stockName": "贵州茅台", "shares": 100, "avgCost": 1800.0, "currentPrice": null, "marketValue": null, "pnlPct": null}
        ]"#;
        let result = parse_vlm_output(input);
        assert!(result.success);
        assert_eq!(result.holdings.len(), 1);
        assert_eq!(result.holdings[0].stock_code, "600519");
        assert_eq!(result.holdings[0].shares, 100.0);
    }

    #[test]
    fn parses_markdown_code_block() {
        let input = "根据截图识别到以下持仓：\n```json\n[\n{\"stockCode\":\"000001\",\"stockName\":\"平安银行\",\"shares\":500,\"avgCost\":12.5}\n]\n```\n";
        let result = parse_vlm_output(input);
        assert!(result.success);
        assert_eq!(result.holdings[0].stock_code, "000001");
    }

    #[test]
    fn filters_empty_stock_codes() {
        let input = r#"[
            {"stockCode": "", "stockName": "测试", "shares": 100, "avgCost": 10.0},
            {"stockCode": "000002", "stockName": "万科A", "shares": 200, "avgCost": 15.0}
        ]"#;
        let result = parse_vlm_output(input);
        assert!(result.success);
        assert_eq!(result.holdings.len(), 1);
        assert_eq!(result.holdings[0].stock_code, "000002");
    }

    #[test]
    fn returns_error_for_invalid_json() {
        let input = "这不是JSON";
        let result = parse_vlm_output(input);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn handles_empty_json_array() {
        let input = "[]";
        let result = parse_vlm_output(input);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("空持仓列表"));
    }

    #[test]
    fn extracts_json_from_mixed_text() {
        let input = "我发现了一个持仓\n[{\"stockCode\":\"600036\",\"stockName\":\"招商银行\",\"shares\":1000,\"avgCost\":35.0}]\n请确认";
        let result = parse_vlm_output(input);
        assert!(result.success);
        assert_eq!(result.holdings[0].stock_code, "600036");
    }

    #[test]
    fn generates_import_params() {
        let holdings = vec![ParsedHolding {
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            shares: 100.0,
            avg_cost: 1800.0,
            current_price: None,
            market_value: None,
            pnl_pct: None,
        }];
        let params = holdings_to_import_params(&holdings);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].stock_code, "600519");
        assert_eq!(params[0].shares, 100.0);
    }
}
