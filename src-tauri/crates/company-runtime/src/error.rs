// SPDX-License-Identifier: AGPL-3.0-only

//! 公司运行时统一错误类型。

use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CompanyError {
    #[error("数据库错误: {0}")]
    Db(#[from] DbErr),
    #[error("实体未找到: {0}")]
    NotFound(String),
    #[error("状态机非法转换: {0}")]
    State(String),
    #[error("参数错误: {0}")]
    Invalid(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

impl CompanyError {
    pub fn other(msg: impl Into<String>) -> Self {
        CompanyError::Other(msg.into())
    }
}

pub type CompanyResult<T> = Result<T, CompanyError>;

impl From<CompanyError> for String {
    fn from(e: CompanyError) -> Self {
        e.to_string()
    }
}
