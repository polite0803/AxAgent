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
//! `astock-data::indicators` 中的 sma/ema/rsi 是 private fn。
//! M1 阶段在 quant 内复制必要的实现（自给自足、不改 astock-data 公开面）；
//! 后续可考虑将 astock-data 指标函数提升为 pub 复用。

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::ctx::StrategyCtx;
use crate::error::QuantError;
use crate::strategy::Strategy;
use crate::types::{Bar, CloseReason, Signal, SignalAction};

// ===================== 共享指标 helpers =====================

fn sma_last(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period || period == 0 {
        return None;
    }
    let start = values.len() - period;
    Some(values[start..].iter().sum::<f64>() / period as f64)
}

fn ema_series(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![0.0];
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema = data[0];
    let mut out = Vec::with_capacity(data.len());
    out.push(ema);
    for &v in &data[1..] {
        ema = (v - ema) * multiplier + ema;
        out.push(ema);
    }
    out
}

fn stddev_sample(data: &[f64], mean: f64) -> f64 {
    let n = data.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let v = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    v.sqrt()
}

fn rsi_wilder(closes: &[f64], period: usize) -> f64 {
    if closes.len() < period + 1 {
        return 50.0;
    }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += -diff;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let g = if diff > 0.0 { diff } else { 0.0 };
        let l = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + g) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + l) / period as f64;
    }
    if avg_loss < 1e-10 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
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
        Self {
            short_period,
            long_period,
        }
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
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        match key {
            "short_period" => {
                self.short_period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "long_period" => {
                self.long_period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            _ => return Err(QuantError::Param(key.to_string())),
        }
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.long_period => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let cur_short = sma_last(&cs, self.short_period).unwrap();
        let cur_long = sma_last(&cs, self.long_period).unwrap();
        let prev_short = sma_last(&cs[..cs.len() - 1], self.short_period).unwrap();
        let prev_long = sma_last(&cs[..cs.len() - 1], self.long_period).unwrap();

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
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        match key {
            "fast" => {
                self.fast = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "slow" => {
                self.slow = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "signal" => {
                self.signal = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            _ => return Err(QuantError::Param(key.to_string())),
        }
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.slow + self.signal => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let ema_fast = ema_series(&cs, self.fast);
        let ema_slow = ema_series(&cs, self.slow);
        let dif_series: Vec<f64> = ema_fast
            .iter()
            .zip(ema_slow.iter())
            .map(|(a, b)| a - b)
            .collect();
        let dea_series = ema_series(&dif_series, self.signal);
        if dif_series.len() < 2 || dea_series.len() < 2 {
            return Ok(vec![]);
        }
        let cur_dif = *dif_series.last().unwrap();
        let cur_dea = *dea_series.last().unwrap();
        let prev_dif = dif_series[dif_series.len() - 2];
        let prev_dea = dea_series[dea_series.len() - 2];

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
    pub fn new(period: usize, overbought: f64, oversold: f64) -> Self {
        Self {
            period,
            overbought,
            oversold,
        }
    }
}

impl Default for RsiStrategy {
    fn default() -> Self {
        Self::new(6, 70.0, 30.0)
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
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        match key {
            "period" => {
                self.period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "overbought" => {
                self.overbought = value
                    .as_f64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
            },
            "oversold" => {
                self.oversold = value
                    .as_f64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
            },
            _ => return Err(QuantError::Param(key.to_string())),
        }
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
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
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        match key {
            "period" => {
                self.period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "stddev" => {
                self.stddev = value
                    .as_f64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
            },
            _ => return Err(QuantError::Param(key.to_string())),
        }
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.period => h,
            _ => return Ok(vec![]),
        };
        let cs = closes(history);
        let mid = sma_last(&cs, self.period).unwrap();
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
        Self {
            entry_period,
            exit_period,
            atr_period,
            atr_multiplier,
            entry_price: None,
        }
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
    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        match key {
            "entry_period" => {
                self.entry_period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "exit_period" => {
                self.exit_period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "atr_period" => {
                self.atr_period = value
                    .as_u64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
                    as usize
            },
            "atr_multiplier" => {
                self.atr_multiplier = value
                    .as_f64()
                    .ok_or_else(|| QuantError::Param(key.to_string()))?
            },
            _ => return Err(QuantError::Param(key.to_string())),
        }
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
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
                let n = self.atr_period.min(trs.len());
                let recent = &trs[trs.len() - n..];
                recent.iter().sum::<f64>() / n as f64
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
