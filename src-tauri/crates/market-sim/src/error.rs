//! 订单簿错误类型

use thiserror::Error;

use crate::types::{OrderId, Price};

#[derive(Error, Debug)]
pub enum SimError {
    #[error("订单不存在: id={0}")]
    OrderNotFound(OrderId),

    #[error("价格必须为正数: price={0}")]
    InvalidPrice(Price),

    #[error("数量必须为正数")]
    InvalidQuantity,

    #[error("撤单失败：订单已完全成交: id={0}")]
    CancelFilledOrder(OrderId),

    #[error("订单簿为空，无法执行操作")]
    EmptyBook,

    #[error("订单簿深度不足，无法完全成交")]
    InsufficientDepth,
}
