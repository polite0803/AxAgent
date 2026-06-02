//! 余额 fetcher：从 LLM 提供商拉取账户余额。
//!
//! P1.5 阶段只落地 DeepSeek（reasonix `balance.go` 移植）。
//! 设计原则：
//! - 异步、超时 12s（与 reasonix 一致）
//! - 空 API key → `Ok(None)` 优雅降级（不抛错，方便 UI 在用户没填 key 时直接隐藏余额区块）
//! - 错误转 String 而非暴露 reqwest::Error 细节给前端
//! - 内部统一 `BalanceError`（thiserror），边界处 `.map_err(|e| e.to_string())`
//!
//! 后续可扩展为 Anthropic / OpenAI / Gemini（每个 fetcher 独立 async fn，统一 Result<Option<Balance>, BalanceError>）。

use serde::{Deserialize, Serialize};

/// 账户余额聚合视图。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    /// DeepSeek `is_available`：账户是否可正常扣费
    pub available: bool,
    /// 多币种余额列表（DeepSeek 通常返回 CNY + USD 两条）
    pub infos: Vec<BalanceInfo>,
}

/// 单个币种的余额详情。
///
/// 字段全部用 `String` 是因为 DeepSeek API 把金额当 decimal string 返回（如 `"100.5000"`），
/// 不损失精度，前端可按需 `parseFloat`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BalanceInfo {
    pub currency: String,
    /// 总余额（含赠送 + 充值）
    pub total: String,
    /// 赠送余额部分
    pub granted: String,
    /// 充值余额部分
    pub topped_up: String,
}

/// balance fetcher 错误类型。
///
/// 故意不暴露 `reqwest::Error` 内部细节（避免与 reqwest 版本耦合 + 防止敏感信息泄漏到 UI）。
/// 边界处统一 `to_string()` 转给前端。
#[derive(Debug, thiserror::Error)]
pub enum BalanceError {
    #[error("HTTP 请求失败: {0}")]
    Http(String),
    #[error("响应解析失败: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for BalanceError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e.to_string())
    }
}

const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";
const BALANCE_FETCH_TIMEOUT_SECS: u64 = 12;

/// 从 DeepSeek 拉取账户余额。
///
/// 行为契约：
/// - `api_key` 为空（含纯空白）→ `Ok(None)`，调用方应在 UI 隐藏余额展示
/// - HTTP 4xx/5xx → `Err(BalanceError::Http)`
/// - 响应 JSON 结构与预期不符 → `Err(BalanceError::Parse)`
/// - 超时 12s（reasonix 一致）
pub async fn fetch_deepseek_balance(api_key: &str) -> Result<Option<Balance>, BalanceError> {
    if api_key.trim().is_empty() {
        return Ok(None);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(BALANCE_FETCH_TIMEOUT_SECS))
        .build()?;

    let resp = client
        .get(DEEPSEEK_BALANCE_URL)
        .bearer_auth(api_key)
        .send()
        .await?
        .error_for_status()?;

    let wire: DeepSeekBalanceResponse = resp
        .json()
        .await
        .map_err(|e| BalanceError::Parse(e.to_string()))?;

    Ok(Some(Balance {
        available: wire.is_available,
        infos: wire
            .balance_infos
            .into_iter()
            .map(|info| BalanceInfo {
                currency: info.currency,
                total: info.total_balance,
                granted: info.granted_balance,
                topped_up: info.topped_up_balance,
            })
            .collect(),
    }))
}

/// DeepSeek `GET /user/balance` 原始响应。
///
/// 字段顺序与官方文档保持一致；使用 `#[serde(default)]` 兼容未来字段新增。
#[derive(Debug, Deserialize)]
struct DeepSeekBalanceResponse {
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<DeepSeekBalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekBalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_api_key_returns_none() {
        let result = fetch_deepseek_balance("").await.expect("should be Ok");
        assert!(result.is_none());

        let result = fetch_deepseek_balance("   \n\t  ")
            .await
            .expect("should be Ok");
        assert!(result.is_none());
    }

    #[test]
    fn balance_serializes_to_json() {
        let balance = Balance {
            available: true,
            infos: vec![BalanceInfo {
                currency: "CNY".to_string(),
                total: "100.5000".to_string(),
                granted: "10.0000".to_string(),
                topped_up: "90.5000".to_string(),
            }],
        };
        let json = serde_json::to_string(&balance).expect("serialize");
        assert!(json.contains("\"available\":true"));
        assert!(json.contains("\"currency\":\"CNY\""));
        assert!(json.contains("\"total\":\"100.5000\""));
    }

    #[test]
    fn wire_response_parses_minimal_payload() {
        // DeepSeek 实际可能只返回 is_available 而无 balance_infos
        let wire: DeepSeekBalanceResponse =
            serde_json::from_str(r#"{"is_available": false}"#).expect("parse minimal");
        assert!(!wire.is_available);
        assert!(wire.balance_infos.is_empty());
    }

    #[test]
    fn wire_response_parses_full_payload() {
        let payload = r#"{
            "is_available": true,
            "balance_infos": [
                {
                    "currency": "CNY",
                    "total_balance": "100.50",
                    "granted_balance": "10.00",
                    "topped_up_balance": "90.50"
                }
            ]
        }"#;
        let wire: DeepSeekBalanceResponse = serde_json::from_str(payload).expect("parse full");
        assert!(wire.is_available);
        assert_eq!(wire.balance_infos.len(), 1);
        assert_eq!(wire.balance_infos[0].currency, "CNY");
        assert_eq!(wire.balance_infos[0].total_balance, "100.50");
    }
}
