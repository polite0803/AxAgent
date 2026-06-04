use serde::{Deserialize, Serialize};

/// DeepSeek 余额查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekBalance {
    /// 是否可用
    pub is_available: bool,
    /// 余额（单位：元）
    pub balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    /// 货币类型: CNY, USD
    pub currency: String,
    /// 总余额
    pub total_balance: String,
    /// 已用额度
    pub granted_balance: String,
    /// 充值余额
    pub topped_up_balance: String,
}

/// 查询 DeepSeek 账户余额
pub async fn fetch_deepseek_balance(api_key: &str) -> Result<DeepSeekBalance, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Balance request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Balance API returned {}: {}", status.as_u16(), body));
    }

    let balance: DeepSeekBalance = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse balance response: {e}"))?;

    Ok(balance)
}
