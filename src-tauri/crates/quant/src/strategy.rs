//! Strategy trait — 单代码源接口
//!
//! 同一份策略代码同时跑回测与实盘：
//! - 回测时：`BacktestEngine` 按 K 线序列逐 bar 调用 `on_bar`
//! - 实盘时：`LiveRunner` 订阅行情推送，按 tick/分钟 bar 调用 `on_bar`
//!
//! 两条路径共享 trait，保证回测表现与实盘行为一致。

use async_trait::async_trait;
use serde_json::Value;

use crate::ctx::StrategyCtx;
use crate::error::QuantError;
use crate::types::{Bar, Signal};

/// 量化策略接口
#[async_trait]
pub trait Strategy: Send + Sync {
    /// 策略名（DB 主键之一，建议英数下划线）
    fn name(&self) -> &str;

    /// 策略版本（语义化版本号，默认 "1.0.0"）
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 策略描述（UI 展示用）
    fn description(&self) -> &str {
        ""
    }

    /// 暴露所有可调参数
    /// - 返回 JSON 对象，key 为参数名，value 为当前值
    /// - UI 用此渲染参数表单
    /// - Walk-Forward grid search 用此生成参数网格
    fn params(&self) -> Value;

    /// 运行时改参（UI 改参 / grid search 注入）
    ///
    /// 返回 `QuantError::Param` 表示参数名不存在或类型不匹配
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError>;

    /// 每根 K 线收盘后回调（D2 决策：每 K 线收盘 = 默认频率）
    ///
    /// - bar: 当前 K 线（已含涨跌停信息）
    /// - ctx: 策略上下文（可读持仓/权益/历史 K 线/指标，**不要**直接修改 cash/positions）
    /// - 返回 0..N 个 Signal；Engine 收集本 bar 全部 Signal 后转 Order
    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx)
    -> Result<Vec<Signal>, QuantError>;

    /// 回测/实盘启动时调用一次（用于初始化指标历史等）
    async fn on_init(&mut self, _ctx: &mut StrategyCtx) -> Result<(), QuantError> {
        Ok(())
    }

    /// 回测/实盘结束时调用一次（用于释放资源、打印统计）
    async fn on_finish(&mut self, _ctx: &mut StrategyCtx) -> Result<(), QuantError> {
        Ok(())
    }
}
