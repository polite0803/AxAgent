use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("Rate limit exceeded from {vendor}")]
    RateLimited { vendor: String },

    #[error("Vendor error from {vendor}: {message}")]
    VendorError { vendor: String, message: String },

    #[error("Stock code not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// As-Of 模式下数据降级：vendor / method 在该时间点不支持或无历史语义。
    /// 不视为致命错误——workflow 节点应降级到空数据并在 `dataQualitySummary`
    /// 里记录"降级原因",但允许继续推进决策(置信度由 aggregator 扣分)。
    ///
    /// spec §4.1 统一降级协议
    #[error("As-Of 模式下 {vendor}::{method} 降级: {reason} (as_of={as_of})")]
    AsOfDegraded { vendor: String, method: String, reason: String, as_of: String },

    /// 内部错误（信号量关闭、运行时异常等不应发生但需要传播的错误）
    /// 修复 M-DEF-1: 用于替代 `acquire_owned().expect("semaphore closed")`
    #[error("Internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asof_degraded_error_displays_all_fields() {
        let e = DataError::AsOfDegraded {
            vendor: "eastmoney".into(),
            method: "get_hot_stocks".into(),
            reason: "no historical semantics".into(),
            as_of: "2026-06-01".into(),
        };
        let s = e.to_string();
        assert!(s.contains("eastmoney"));
        assert!(s.contains("get_hot_stocks"));
        assert!(s.contains("2026-06-01"));
        assert!(s.contains("降级"));
    }
}
