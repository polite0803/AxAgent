//! 统一错误类型
//!
//! 所有 quant crate 内部错误均应实现 `Into<QuantError>`，
//! 边界层（Tauri command）将 `QuantError` 转为 `String` 返回给前端。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuantError {
    /// 数据层错误（K 线缺失、Quote 缺失、AsOf 截断失败等）
    #[error("数据错误: {0}")]
    Data(String),

    /// 策略内部错误（指标计算失败、参数越界、状态机异常）
    #[error("策略错误: {0}")]
    Strategy(String),

    /// 回测引擎错误（事件循环异常、撮合失败、权益曲线写入失败）
    #[error("回测错误: {0}")]
    Backtest(String),

    /// 策略参数错误（参数名不存在、参数值类型不匹配）
    #[error("参数错误: 参数 `{0}` 不存在")]
    Param(String),

    /// Rhai 脚本错误（编译失败、运行期异常、函数未定义）
    #[error("Rhai 脚本错误: {0}")]
    Script(String),

    /// Walk-Forward 验证错误（窗口长度非法、样本不足、grid search 失败）
    #[error("Walk-Forward 错误: {0}")]
    WalkForward(String),

    /// 多策略组合错误（权重非法、相关性矩阵非法）
    #[error("多策略错误: {0}")]
    Multi(String),

    /// 序列化/反序列化错误
    #[error("序列化错误: {0}")]
    Serde(String),
}

impl From<serde_json::Error> for QuantError {
    fn from(e: serde_json::Error) -> Self {
        QuantError::Serde(e.to_string())
    }
}
