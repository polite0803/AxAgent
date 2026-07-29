//! 5 个内置技术分析策略
//!
//! - `MaCrossStrategy`: 双均线交叉
//! - `MacdStrategy`: MACD 金叉/死叉
//! - `RsiStrategy`: RSI 超买超卖反转
//! - `BollStrategy`: 布林带均值回归
//! - `TurtleStrategy`: 海龟交易法（通道突破 + ATR 止损）
//!
//! 所有策略采用每 K 线收盘频率（D2 决策），
//! Walk-Forward 兼容（D3 决策），参数可通过 `set_param` 注入。
//!
//! ## 重要约定
//!
//! - Engine 在调用 `on_bar` 之前，必须把当前 bar push 到 `ctx.bar_history[code]`
//! - `on_bar` 内部不再 push bar，避免重复
//! - Engine 负责维护 `ctx.bar_history`，策略只读访问
//!
//! ## 关于指标 helper
//!
//! P2-C7: 技术指标（SMA/EMA/RSI/stddev）已统一收口到 `axagent_harness::indicators`。
//! 本 crate 通过 `use` 引用 harness 版本，消除历史重复实现。
//! 仅保留 `rsi_wilder` 作为 pub(crate) wrapper，将 `Option<f64>` 转为 `f64`
//! （数据不足返回 50.0 中性值），维持 script.rs 的调用签名兼容。

use async_trait::async_trait;
use serde_json::{Value, json};

// 引入 harness 的 Result 别名（即 Result<_, AxAgentError>）—— 用 HarnessResult 避免遮蔽 std Result
// Strategy trait 已下沉到 harness，方法返回 axagent_harness::core_error::Result
use axagent_harness::core_error::Result as HarnessResult;
// P2-C7: 技术指标统一来源（harness foundation 层）
use axagent_harness::indicators::{build_ema_series, sma, stddev_sample};
// 保留 QuantError：作为策略实现的内部错误类型，通过 From<QuantError> for AxAgentError
// 在 `?` 处自动转换为 AxAgentError
use crate::error::QuantError;
use crate::types::{Bar, CloseReason, Signal, SignalAction};

// 类型仅用于 trait 方法签名（Strategy trait 在 harness 中定义）
use crate::ctx::StrategyCtx;
use crate::strategy::Strategy;

// ===================== RSI wrapper =====================

/// RSI (Wilder 平滑) 指标计算 — pub(crate) wrapper
///
/// 修复 M-RES-9: 改为 pub(crate) 以便 script.rs 的 rsi_rhai 复用，
/// 消除重复实现（DRY 原则）。
/// P2-C7: 内部委托 `axagent_harness::indicators::rsi_wilder`，
/// 数据不足时返回 50.0 中性值（保持原 f64 返回签名兼容）。
pub(crate) fn rsi_wilder(closes: &[f64], period: usize) -> f64 {
    axagent_harness::indicators::rsi_wilder(closes, period).unwrap_or(50.0)
}

fn closes(history: &[Bar]) -> Vec<f64> {
    history.iter().map(|b| b.close).collect()
}

// ===================== 1. 双均线交叉 (MA Cross) =====================

pub struct MaCrossStrategy {
    pub short_period: usize,
    pub long_period: usize,
}

impl MaCrossStrategy {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        Self { short_period, long_period }
    }
}

impl Default for MaCrossStrategy {
    fn default() -> Self {
        Self::new(5, 20)
    }
}

#[async_trait]
impl Strategy for MaCrossStrategy {
    fn name(&self) -> &str {
        "ma_cross"
    }
    fn description(&self) -> &str {
        "双均线交叉：MA short 上穿 MA long 买入；下穿卖出"
    }
    fn params(&self) -> Value {
        json!({
            "short_period": self.short_period,
            "long_period": self.long_period,
        })
    }
    fn set_param(&mut self, key: &str, value: Value) -> HarnessResult<()> {
        let (new_short, new_long) = match key {
            "short_period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (v, self.long_period)
            },
            "long_period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (self.short_period, v)
            },
            _ => return Err(QuantError::Param(key.to_string()).into()),
        };
        if new_short == 0 || new_long == 0 {
            return Err(QuantError::Param(format!(
                "period 必须 > 0: short={}, long={}",
                new_short, new_long
            ))
            .into());
        }
        if new_short >= new_long {
            return Err(QuantError::Param(format!(
                "short({}) 必须 < long({})",
                new_short, new_long
            ))
            .into());
        }
        self.short_period = new_short;
        self.long_period = new_long;
        Ok(())
    }

    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> HarnessResult<Vec<Signal>> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.long_period => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let cur_short = match sma(&cs, self.short_period) {
            Some(v) => v,
            None => return Ok(vec![]),
        };
        let cur_long = match sma(&cs, self.long_period) {
            Some(v) => v,
            None => return Ok(vec![]),
        };
        let prev_short = match sma(&cs[..cs.len() - 1], self.short_period) {
            Some(v) => v,
            None => return Ok(vec![]),
        };
        let prev_long = match sma(&cs[..cs.len() - 1], self.long_period) {
            Some(v) => v,
            None => return Ok(vec![]),
        };

        // 金叉：prev_short <= prev_long && cur_short > cur_long
        if prev_short <= prev_long && cur_short > cur_long {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Buy,
                strength: 0.7,
                reason: format!(
                    "MA{} 上穿 MA{} ({:.2} > {:.2})",
                    self.short_period, self.long_period, cur_short, cur_long
                ),
                target_weight: None,
                close_reason: None,
            }]);
        }
        // 死叉
        if prev_short >= prev_long && cur_short < cur_long {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Sell,
                strength: 0.7,
                reason: format!(
                    "MA{} 下穿 MA{} ({:.2} < {:.2})",
                    self.short_period, self.long_period, cur_short, cur_long
                ),
                target_weight: None,
                close_reason: Some(CloseReason::SignalReverse),
            }]);
        }
        Ok(vec![])
    }
}

// ===================== 2. MACD =====================

pub struct MacdStrategy {
    pub fast: usize,   // 默认 12
    pub slow: usize,   // 默认 26
    pub signal: usize, // 默认 9
}

impl MacdStrategy {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self { fast, slow, signal }
    }
}

impl Default for MacdStrategy {
    fn default() -> Self {
        Self::new(12, 26, 9)
    }
}

#[async_trait]
impl Strategy for MacdStrategy {
    fn name(&self) -> &str {
        "macd"
    }
    fn description(&self) -> &str {
        "MACD 金叉死叉：DIF 上穿 DEA 买入；DIF 下穿 DEA 卖出"
    }
    fn params(&self) -> Value {
        json!({
            "fast": self.fast,
            "slow": self.slow,
            "signal": self.signal,
        })
    }
    fn set_param(&mut self, key: &str, value: Value) -> HarnessResult<()> {
        let (new_fast, new_slow, new_signal) = match key {
            "fast" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (v, self.slow, self.signal)
            },
            "slow" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (self.fast, v, self.signal)
            },
            "signal" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (self.fast, self.slow, v)
            },
            _ => return Err(QuantError::Param(key.to_string()).into()),
        };
        if new_fast == 0 || new_slow == 0 || new_signal == 0 {
            return Err(QuantError::Param(format!(
                "MACD 参数必须 > 0: fast={}, slow={}, signal={}",
                new_fast, new_slow, new_signal
            ))
            .into());
        }
        if new_fast >= new_slow {
            return Err(
                QuantError::Param(format!("fast({}) 必须 < slow({})", new_fast, new_slow)).into()
            );
        }
        self.fast = new_fast;
        self.slow = new_slow;
        self.signal = new_signal;
        Ok(())
    }

    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> HarnessResult<Vec<Signal>> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.slow + self.signal => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let ema_fast = build_ema_series(&cs, self.fast);
        let ema_slow = build_ema_series(&cs, self.slow);
        let dif_series: Vec<f64> =
            ema_fast.iter().zip(ema_slow.iter()).map(|(a, b)| a - b).collect();
        let dea_series = build_ema_series(&dif_series, self.signal);
        if dif_series.len() < 2 || dea_series.len() < 2 {
            return Ok(vec![]);
        }
        let (cur_dif, cur_dea, prev_dif, prev_dea) =
            match (dif_series.as_slice(), dea_series.as_slice()) {
                (d, e) if d.len() >= 2 && e.len() >= 2 => {
                    (d[d.len() - 1], e[e.len() - 1], d[d.len() - 2], e[e.len() - 2])
                },
                _ => return Ok(vec![]),
            };

        // 金叉
        if prev_dif <= prev_dea && cur_dif > cur_dea {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Buy,
                strength: 0.7,
                reason: format!("MACD 金叉 DIF={:.4} > DEA={:.4}", cur_dif, cur_dea),
                target_weight: None,
                close_reason: None,
            }]);
        }
        // 死叉
        if prev_dif >= prev_dea && cur_dif < cur_dea {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Sell,
                strength: 0.7,
                reason: format!("MACD 死叉 DIF={:.4} < DEA={:.4}", cur_dif, cur_dea),
                target_weight: None,
                close_reason: Some(CloseReason::SignalReverse),
            }]);
        }
        Ok(vec![])
    }
}

// ===================== 3. RSI 反转 =====================

pub struct RsiStrategy {
    pub period: usize,
    pub overbought: f64,
    pub oversold: f64,
}

impl RsiStrategy {
    /// 构造 RSI 反转策略。
    ///
    /// # 参数校验
    /// `overbought` 必须严格大于 `oversold`，否则阈值语义颠倒（超买低于超卖），
    /// 会生成相反的买卖信号。构造期即拦截，避免静默产生错误信号。
    pub fn new(period: usize, overbought: f64, oversold: f64) -> Result<Self, QuantError> {
        if overbought <= oversold {
            return Err(QuantError::Param(format!(
                "RsiStrategy: overbought({}) 必须严格大于 oversold({})，阈值颠倒",
                overbought, oversold
            )));
        }
        Ok(Self { period, overbought, oversold })
    }
}

impl Default for RsiStrategy {
    fn default() -> Self {
        Self::new(6, 70.0, 30.0).expect("RsiStrategy 默认阈值非法")
    }
}

#[async_trait]
impl Strategy for RsiStrategy {
    fn name(&self) -> &str {
        "rsi"
    }
    fn description(&self) -> &str {
        "RSI 超买超卖反转：RSI 下穿 oversold 买入；RSI 上穿 overbought 卖出"
    }
    fn params(&self) -> Value {
        json!({
            "period": self.period,
            "overbought": self.overbought,
            "oversold": self.oversold,
        })
    }
    fn set_param(&mut self, key: &str, value: Value) -> HarnessResult<()> {
        let (new_period, new_overbought, new_oversold) = match key {
            "period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (v, self.overbought, self.oversold)
            },
            "overbought" => {
                let v = value.as_f64().ok_or_else(|| QuantError::Param(key.to_string()))?;
                (self.period, v, self.oversold)
            },
            "oversold" => {
                let v = value.as_f64().ok_or_else(|| QuantError::Param(key.to_string()))?;
                (self.period, self.overbought, v)
            },
            _ => return Err(QuantError::Param(key.to_string()).into()),
        };
        if new_period == 0 {
            return Err(QuantError::Param("period 必须 > 0".into()).into());
        }
        if new_overbought <= new_oversold {
            return Err(QuantError::Param(format!(
                "overbought({}) 必须 > oversold({})",
                new_overbought, new_oversold
            ))
            .into());
        }
        self.period = new_period;
        self.overbought = new_overbought;
        self.oversold = new_oversold;
        Ok(())
    }

    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> HarnessResult<Vec<Signal>> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() >= self.period + 2 => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let cur_rsi = rsi_wilder(&cs, self.period);
        let prev_rsi = rsi_wilder(&cs[..cs.len() - 1], self.period);

        if prev_rsi >= self.oversold && cur_rsi < self.oversold {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Buy,
                strength: 0.6,
                reason: format!("RSI({}) 跌破 {} (现 {:.2})", self.period, self.oversold, cur_rsi),
                target_weight: None,
                close_reason: None,
            }]);
        }
        if prev_rsi <= self.overbought && cur_rsi > self.overbought {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Sell,
                strength: 0.6,
                reason: format!(
                    "RSI({}) 突破 {} (现 {:.2})",
                    self.period, self.overbought, cur_rsi
                ),
                target_weight: None,
                close_reason: Some(CloseReason::TakeProfit),
            }]);
        }
        Ok(vec![])
    }
}

// ===================== 4. 布林带均值回归 (BOLL) =====================

pub struct BollStrategy {
    pub period: usize,
    pub stddev: f64,
}

impl BollStrategy {
    pub fn new(period: usize, stddev: f64) -> Self {
        Self { period, stddev }
    }
}

impl Default for BollStrategy {
    fn default() -> Self {
        Self::new(20, 2.0)
    }
}

#[async_trait]
impl Strategy for BollStrategy {
    fn name(&self) -> &str {
        "boll"
    }
    fn description(&self) -> &str {
        "布林带均值回归：触及下轨买入；触及上轨卖出"
    }
    fn params(&self) -> Value {
        json!({
            "period": self.period,
            "stddev": self.stddev,
        })
    }
    fn set_param(&mut self, key: &str, value: Value) -> HarnessResult<()> {
        let (new_period, new_stddev) = match key {
            "period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (v, self.stddev)
            },
            "stddev" => {
                let v = value.as_f64().ok_or_else(|| QuantError::Param(key.to_string()))?;
                (self.period, v)
            },
            _ => return Err(QuantError::Param(key.to_string()).into()),
        };
        if new_period == 0 {
            return Err(QuantError::Param("period 必须 > 0".into()).into());
        }
        if new_stddev <= 0.0 {
            return Err(QuantError::Param(format!("stddev({}) 必须 > 0", new_stddev)).into());
        }
        self.period = new_period;
        self.stddev = new_stddev;
        Ok(())
    }

    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> HarnessResult<Vec<Signal>> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.period => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let mid = match sma(&cs, self.period) {
            Some(v) => v,
            None => return Ok(vec![]),
        };
        let sd = stddev_sample(&cs[cs.len() - self.period..], mid);
        let upper = mid + self.stddev * sd;
        let lower = mid - self.stddev * sd;
        let close = bar.close;

        if close <= lower {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Buy,
                strength: 0.65,
                reason: format!("触及布林下轨 close={:.2} lower={:.2}", close, lower),
                target_weight: None,
                close_reason: None,
            }]);
        }
        if close >= upper {
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Sell,
                strength: 0.65,
                reason: format!("触及布林上轨 close={:.2} upper={:.2}", close, upper),
                target_weight: None,
                close_reason: Some(CloseReason::TakeProfit),
            }]);
        }
        Ok(vec![])
    }
}

// ===================== 5. 海龟交易法 (Turtle) =====================

pub struct TurtleStrategy {
    pub entry_period: usize, // 默认 20 (上轨突破周期)
    pub exit_period: usize,  // 默认 10 (下轨退出周期)
    pub atr_period: usize,   // 默认 20 (ATR 计算周期)
    pub atr_multiplier: f64, // 默认 2.0 (止损 = entry - 2*ATR)
    pub entry_price: Option<f64>,
}

impl TurtleStrategy {
    pub fn new(
        entry_period: usize,
        exit_period: usize,
        atr_period: usize,
        atr_multiplier: f64,
    ) -> Self {
        Self { entry_period, exit_period, atr_period, atr_multiplier, entry_price: None }
    }
}

impl Default for TurtleStrategy {
    fn default() -> Self {
        Self::new(20, 10, 20, 2.0)
    }
}

#[async_trait]
impl Strategy for TurtleStrategy {
    fn name(&self) -> &str {
        "turtle"
    }
    fn description(&self) -> &str {
        "海龟交易法：N 日最高价突破买入；M 日最低价跌破卖出；ATR 倍数止损"
    }
    fn params(&self) -> Value {
        json!({
            "entry_period": self.entry_period,
            "exit_period": self.exit_period,
            "atr_period": self.atr_period,
            "atr_multiplier": self.atr_multiplier,
        })
    }
    fn set_param(&mut self, key: &str, value: Value) -> HarnessResult<()> {
        let (new_entry, new_exit, new_atr) = match key {
            "entry_period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (v, self.exit_period, self.atr_period)
            },
            "exit_period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (self.entry_period, v, self.atr_period)
            },
            "atr_period" => {
                let v = value.as_u64().ok_or_else(|| QuantError::Param(key.to_string()))? as usize;
                (self.entry_period, self.exit_period, v)
            },
            "atr_multiplier" => {
                self.atr_multiplier =
                    value.as_f64().ok_or_else(|| QuantError::Param(key.to_string()))?;
                return Ok(());
            },
            _ => return Err(QuantError::Param(key.to_string()).into()),
        };
        if new_entry == 0 || new_exit == 0 || new_atr == 0 {
            return Err(QuantError::Param(format!(
                "Turtle 参数必须 > 0: entry={}, exit={}, atr={}",
                new_entry, new_exit, new_atr
            ))
            .into());
        }
        self.entry_period = new_entry;
        self.exit_period = new_exit;
        self.atr_period = new_atr;
        Ok(())
    }

    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> HarnessResult<Vec<Signal>> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.entry_period => h,
            _ => return Ok(vec![]),
        };
        // 不含当前 bar 的前 N 日最高
        let entry_high = history[..history.len() - 1]
            .iter()
            .rev()
            .take(self.entry_period)
            .map(|b| b.high)
            .fold(f64::MIN, f64::max);
        // 不含当前 bar 的前 M 日最低
        let exit_low = if history.len() > self.exit_period {
            history[..history.len() - 1]
                .iter()
                .rev()
                .take(self.exit_period)
                .map(|b| b.low)
                .fold(f64::MAX, f64::min)
        } else {
            f64::MAX
        };
        let cur_close = bar.close;

        // 已有持仓：先检查止损 / 出场
        if let Some(entry_p) = self.entry_price {
            // ATR 计算
            let atr = if history.len() > 1 {
                let trs: Vec<f64> = (1..history.len())
                    .map(|i| {
                        let h = history[i].high;
                        let l = history[i].low;
                        let pc = history[i - 1].close;
                        (h - l).max((h - pc).abs()).max((l - pc).abs())
                    })
                    .collect();
                // 修复(2026-07-29): 防御 atr_period=0 导致 NaN。
                //   若用户通过 new() 直接传入 atr_period=0（set_param 已拦截但构造函数未拦截），
                //   n=0 会导致 `trs[len-0..]`=空切片, sum=0, 0/0=NaN 污染下游止损价计算。
                //   n=0 时返回 0.0 让 stop_price = entry_p,由后续逻辑处理。
                let n = self.atr_period.min(trs.len());
                if n == 0 {
                    0.0
                } else {
                    let recent = &trs[trs.len() - n..];
                    recent.iter().sum::<f64>() / n as f64
                }
            } else {
                0.0
            };
            let stop_price = entry_p - self.atr_multiplier * atr;
            if cur_close <= stop_price {
                self.entry_price = None;
                return Ok(vec![Signal {
                    code: bar.code.clone(),
                    action: SignalAction::Sell,
                    strength: 0.9,
                    reason: format!(
                        "ATR 止损 close={:.2} stop={:.2} ATR={:.2}",
                        cur_close, stop_price, atr
                    ),
                    target_weight: None,
                    close_reason: Some(CloseReason::StopLoss),
                }]);
            }
            if cur_close < exit_low {
                self.entry_price = None;
                return Ok(vec![Signal {
                    code: bar.code.clone(),
                    action: SignalAction::Sell,
                    strength: 0.8,
                    reason: format!(
                        "跌破 {} 日最低 close={:.2} exit_low={:.2}",
                        self.exit_period, cur_close, exit_low
                    ),
                    target_weight: None,
                    close_reason: Some(CloseReason::SignalReverse),
                }]);
            }
            return Ok(vec![]);
        }

        // 无持仓：检查入场
        if cur_close > entry_high {
            self.entry_price = Some(cur_close);
            return Ok(vec![Signal {
                code: bar.code.clone(),
                action: SignalAction::Buy,
                strength: 0.75,
                reason: format!(
                    "突破 {} 日最高 entry_high={:.2} close={:.2}",
                    self.entry_period, entry_high, cur_close
                ),
                target_weight: None,
                close_reason: None,
            }]);
        }
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_new_valid_thresholds() {
        // overbought > oversold 合法
        let s = RsiStrategy::new(14, 70.0, 30.0);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s.period, 14);
        assert!((s.overbought - 70.0).abs() < 1e-9);
        assert!((s.oversold - 30.0).abs() < 1e-9);
    }

    #[test]
    fn rsi_new_rejects_inverted_thresholds() {
        // 审计项「RSI 参数无交叉校验」：overbought <= oversold 必须构造期拦截
        let inverted = RsiStrategy::new(14, 30.0, 70.0);
        assert!(matches!(inverted, Err(QuantError::Param(_))));
        let equal = RsiStrategy::new(14, 50.0, 50.0);
        assert!(matches!(equal, Err(QuantError::Param(_))));
    }

    #[test]
    fn rsi_default_is_valid() {
        let s = RsiStrategy::default();
        assert!(s.overbought > s.oversold);
    }
}
