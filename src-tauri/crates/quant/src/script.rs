//! Rhai 策略加载器
//!
//! ## D1 决策落实
//!
//! - 策略以 Rhai 脚本形式提供（热加载 / 用户编辑）
//! - sandbox 模式：禁止文件系统 / 网络 / 系统调用
//! - 暴露给脚本的 API 极简（仅读 K 线、读 ctx、返回 Signal 数组）
//!
//! ## Rhai API（用户视角）
//!
//! ```rhai
//! // 策略函数签名
//! fn on_bar(bar, ctx) {
//!     // bar: #{date, code, open, high, low, close, volume, ...}
//!     // ctx: #{cash, current_date, history_len, closes, position_qty}
//!     
//!     if ctx.closes.len < 20 { return []; }
//!     // ... 策略逻辑
//!     if buy_signal {
//!         return [#{
//!             action: "buy",
//!             code: bar.code,
//!             strength: 0.7,
//!             reason: "MA 金叉"
//!         }];
//!     }
//!     return [];
//! }
//!
//! // 可选：参数初始化
//! fn init(params) {
//!     // params: #{...}
//!     // 返回: 全局变量定义
//! }
//! ```
//!
//! ## 实现要点
//!
//! - 每个 RhaiStrategy 实例编译一次 AST（缓存）
//! - on_bar 调用时把 Rust Bar + Ctx 转 Rhai Map
//! - 用户返回的 Rhai Array 转 Vec<Signal>
//! - 调 set_param 时 re-run init(params) 注入新参数

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rhai::{AST, Array, Engine, Map, Scope};
use serde_json::Value;

use crate::ctx::StrategyCtx;
use crate::error::QuantError;
use crate::strategy::Strategy;
use crate::types::{Bar, CloseReason, Side, Signal, SignalAction};

/// Rhai 策略（脚本驱动）
pub struct RhaiStrategy {
    name: String,
    version: String,
    description: String,
    script: String,
    /// 共享 Rhai 引擎（script 内的 init / on_bar 都跑在同一引擎）
    /// Mutex 保护：Rhai Engine 不是 Send（内部 Rc），需要包 Mutex
    engine: Arc<Mutex<Engine>>,
    /// 编译后的 AST（缓存，避免每次 on_bar 重新编译）
    ast: Arc<AST>,
    /// 参数
    params: HashMap<String, Value>,
}

impl RhaiStrategy {
    /// 从 script 创建策略
    ///
    /// - 编译 script 到 AST（一次性）
    /// - 不执行 init（懒执行，set_param 时才跑）
    pub fn from_script(
        name: impl Into<String>,
        script: impl Into<String>,
    ) -> Result<Self, QuantError> {
        Self::from_script_full(name, script, "1.0.0", "")
    }

    pub fn from_script_full(
        name: impl Into<String>,
        script: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, QuantError> {
        let name = name.into();
        let script = script.into();
        let engine = build_engine();
        let ast = engine
            .compile(&script)
            .map_err(|e| QuantError::Script(format!("编译失败: {}", e)))?;
        Ok(Self {
            name,
            version: version.into(),
            description: description.into(),
            script,
            engine: Arc::new(Mutex::new(engine)),
            ast: Arc::new(ast),
            params: HashMap::new(),
        })
    }

    /// 获取 script 源码（用于 UI 编辑 + 持久化）
    pub fn script(&self) -> &str {
        &self.script
    }
}

#[async_trait]
impl Strategy for RhaiStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn params(&self) -> Value {
        serde_json::to_value(&self.params).unwrap_or(Value::Null)
    }

    fn set_param(&mut self, key: &str, value: Value) -> Result<(), QuantError> {
        self.params.insert(key.to_string(), value.clone());
        // 重新调用 init(params) 让用户脚本感知新参数
        let engine = self
            .engine
            .lock()
            .map_err(|e| QuantError::Script(format!("engine lock: {}", e)))?;
        let ast = self.ast.clone();
        let params_map = json_value_to_rhai(&value);
        // 调用 init(params) - 用户脚本可选实现
        let mut scope = Scope::new();
        let _: Result<(), Box<rhai::EvalAltResult>> =
            engine.call_fn(&mut scope, &ast, "init", (params_map,));
        Ok(())
    }

    async fn on_bar(
        &mut self,
        bar: &Bar,
        ctx: &mut StrategyCtx,
    ) -> Result<Vec<Signal>, QuantError> {
        let ast = self.ast.clone();
        let bar_map = bar_to_rhai(bar);
        let ctx_map = ctx_to_rhai(ctx, bar);
        let result: Result<Array, Box<rhai::EvalAltResult>> = {
            let engine = self
                .engine
                .lock()
                .map_err(|e| QuantError::Script(format!("engine lock: {}", e)))?;
            let mut scope = Scope::new();
            engine.call_fn(&mut scope, &ast, "on_bar", (bar_map, ctx_map))
        };
        let arr = result.map_err(|e| QuantError::Script(format!("on_bar 执行失败: {}", e)))?;
        rhai_array_to_signals(arr)
    }

    async fn on_init(&mut self, _ctx: &mut StrategyCtx) -> Result<(), QuantError> {
        // 用当前 params 调一次 init（让用户脚本初始化全局变量）
        let engine = self
            .engine
            .lock()
            .map_err(|e| QuantError::Script(format!("engine lock: {}", e)))?;
        let ast = self.ast.clone();
        let params_value = serde_json::to_value(&self.params).unwrap_or(Value::Null);
        let params_map = json_value_to_rhai(&params_value);
        let mut scope = Scope::new();
        let _: Result<(), Box<rhai::EvalAltResult>> =
            engine.call_fn(&mut scope, &ast, "init", (params_map,));
        Ok(())
    }

    async fn on_finish(&mut self, _ctx: &mut StrategyCtx) -> Result<(), QuantError> {
        let engine = self
            .engine
            .lock()
            .map_err(|e| QuantError::Script(format!("engine lock: {}", e)))?;
        let ast = self.ast.clone();
        let mut scope = Scope::new();
        let _: Result<(), Box<rhai::EvalAltResult>> =
            engine.call_fn(&mut scope, &ast, "on_finish", ());
        Ok(())
    }
}

// ===================== 转换辅助 =====================

fn build_engine() -> Engine {
    let mut engine = Engine::new();
    // D1 sandbox：禁止文件系统/网络/系统调用
    // Rhai 1.x 默认就是 sandbox（不 register_package 时无危险 API）
    // 仅注册我们需要的 helper 函数
    // 提升复杂度上限：默认 128/64 对长字符串拼接 + 长 if 链不够
    engine.set_max_expr_depths(64, 64);
    engine.set_max_operations(20_000);
    engine.set_max_call_levels(32);
    engine.register_fn("sma", sma_rhai);
    engine.register_fn("ema", ema_rhai);
    engine.register_fn("rsi", rsi_rhai);
    engine
}

fn bar_to_rhai(bar: &Bar) -> Map {
    let mut m = Map::new();
    m.insert("date".into(), bar.date.clone().into());
    m.insert("code".into(), bar.code.clone().into());
    m.insert("open".into(), bar.open.into());
    m.insert("high".into(), bar.high.into());
    m.insert("low".into(), bar.low.into());
    m.insert("close".into(), bar.close.into());
    m.insert("volume".into(), bar.volume.into());
    m.insert("amount".into(), bar.amount.into());
    if let Some(t) = bar.turnover_rate {
        m.insert("turnover_rate".into(), t.into());
    }
    if let Some(a) = bar.adj_factor {
        m.insert("adj_factor".into(), a.into());
    }
    if let Some(lu) = bar.limit_up {
        m.insert("limit_up".into(), lu.into());
    }
    if let Some(ld) = bar.limit_down {
        m.insert("limit_down".into(), ld.into());
    }
    m.insert("is_st".into(), bar.is_st.into());
    m
}

fn ctx_to_rhai(ctx: &StrategyCtx, bar: &Bar) -> Map {
    let mut m = Map::new();
    m.insert("cash".into(), ctx.cash.into());
    m.insert("current_date".into(), ctx.current_date.clone().into());
    m.insert("current_time".into(), ctx.current_time.clone().into());
    m.insert("is_replay".into(), ctx.is_replay.into());
    if let Some(asof) = &ctx.asof_date {
        m.insert("asof_date".into(), asof.clone().into());
    }
    // 当前 bar.code 的历史 K 线
    let history = ctx.bar_history.get(&bar.code);
    let history_len = history.map(|h| h.len()).unwrap_or(0);
    m.insert("history_len".into(), (history_len as i64).into());
    let closes: Array = history
        .map(|h| h.iter().map(|b| b.close.into()).collect())
        .unwrap_or_default();
    m.insert("closes".into(), closes.into());
    // 当前 bar.code 的持仓
    let pos_qty = ctx
        .position(&bar.code)
        .map(|p| p.quantity as i64)
        .unwrap_or(0);
    m.insert("position_qty".into(), pos_qty.into());
    let pos_cost = ctx.position(&bar.code).map(|p| p.cost_basis).unwrap_or(0.0);
    m.insert("position_cost".into(), pos_cost.into());
    m
}

fn rhai_array_to_signals(arr: Array) -> Result<Vec<Signal>, QuantError> {
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let m = v
            .try_cast::<Map>()
            .ok_or_else(|| QuantError::Script("on_bar 返回值必须为 Map 数组".to_string()))?;
        let code = m
            .get("code")
            .and_then(|v| v.clone().try_cast::<String>())
            .ok_or_else(|| QuantError::Script("signal 缺 code 字段".to_string()))?;
        let action_str = m
            .get("action")
            .and_then(|v| v.clone().try_cast::<String>())
            .ok_or_else(|| QuantError::Script("signal 缺 action 字段".to_string()))?;
        let action = match action_str.as_str() {
            "buy" => SignalAction::Buy,
            "sell" => SignalAction::Sell,
            "hold" => SignalAction::Hold,
            other => return Err(QuantError::Script(format!("signal action 非法: {}", other))),
        };
        let strength = m
            .get("strength")
            .and_then(|v| v.clone().try_cast::<f64>())
            .unwrap_or(0.5);
        let reason = m
            .get("reason")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default();
        let close_reason = m
            .get("close_reason")
            .and_then(|v| v.clone().try_cast::<String>())
            .and_then(|s| match s.as_str() {
                "take_profit" => Some(CloseReason::TakeProfit),
                "stop_loss" => Some(CloseReason::StopLoss),
                "signal_reverse" => Some(CloseReason::SignalReverse),
                "risk_control" => Some(CloseReason::RiskControl),
                "end_of_backtest" => Some(CloseReason::EndOfBacktest),
                "manual" => Some(CloseReason::Manual),
                _ => None,
            });
        out.push(Signal {
            code,
            action,
            strength,
            reason,
            target_weight: None,
            close_reason,
        });
    }
    Ok(out)
}

fn json_value_to_rhai(v: &Value) -> rhai::Dynamic {
    match v {
        Value::Null => rhai::Dynamic::UNIT,
        Value::Bool(b) => (*b).into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into()
            } else if let Some(f) = n.as_f64() {
                f.into()
            } else {
                rhai::Dynamic::UNIT
            }
        },
        Value::String(s) => s.clone().into(),
        Value::Array(arr) => {
            let rhai_arr: Array = arr.iter().map(json_value_to_rhai).collect();
            // 这里会动 array，按值返回
            rhai::Dynamic::from_array(rhai_arr)
        },
        Value::Object(obj) => {
            let mut m = Map::new();
            for (k, v) in obj {
                m.insert(k.clone().into(), json_value_to_rhai(v));
            }
            m.into()
        },
    }
}

// ===================== 注册给 Rhai 调用的 helper 函数 =====================

fn sma_rhai(values: Array, period: i64) -> f64 {
    if period <= 0 || values.len() < period as usize {
        return 0.0;
    }
    let start = values.len() - period as usize;
    let sum: f64 = values[start..]
        .iter()
        .filter_map(|v| v.clone().try_cast::<f64>())
        .sum();
    if period == 0 {
        0.0
    } else {
        sum / period as f64
    }
}

fn ema_rhai(values: Array, period: i64) -> f64 {
    if values.is_empty() || period <= 0 {
        return 0.0;
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema = values[0].clone().try_cast::<f64>().unwrap_or(0.0);
    for v in &values[1..] {
        let val = v.clone().try_cast::<f64>().unwrap_or(0.0);
        ema = (val - ema) * multiplier + ema;
    }
    ema
}

fn rsi_rhai(values: Array, period: i64) -> f64 {
    if values.len() < (period as usize + 1) || period <= 0 {
        return 50.0;
    }
    let closes: Vec<f64> = values
        .iter()
        .filter_map(|v| v.clone().try_cast::<f64>())
        .collect();
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period as usize {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += -diff;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    for i in (period as usize + 1)..closes.len() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::StrategyCtx;

    const MA_CROSS_RHAI: &str = r#"
fn on_bar(bar, ctx) {
    if ctx.closes.len < 20 { return []; }
    let s5 = sma(ctx.closes, 5);
    let s20 = sma(ctx.closes, 20);
    if s5 > s20 && ctx.position_qty == 0 {
        return [#{
            action: "buy",
            code: bar.code,
            strength: 0.7,
            reason: "MA5(" + s5 + ") > MA20(" + s20 + ")"
        }];
    }
    if s5 < s20 && ctx.position_qty > 0 {
        return [#{
            action: "sell",
            code: bar.code,
            strength: 0.7,
            reason: "MA5 下穿 MA20",
            close_reason: "signal_reverse"
        }];
    }
    return [];
}
"#;

    #[test]
    fn test_rhai_strategy_compile() {
        let s = RhaiStrategy::from_script("ma_cross_rhai", MA_CROSS_RHAI).unwrap();
        assert_eq!(s.name(), "ma_cross_rhai");
        assert!(!s.script().is_empty());
    }

    #[test]
    fn test_rhai_strategy_set_param() {
        let mut s = RhaiStrategy::from_script("p", MA_CROSS_RHAI).unwrap();
        let mut v = serde_json::Map::new();
        v.insert("short_period".into(), serde_json::json!(5));
        let value = serde_json::Value::Object(v);
        s.set_param("short_period", value).unwrap();
        let p = s.params();
        assert!(p.get("short_period").is_some());
    }

    #[tokio::test]
    async fn test_rhai_strategy_on_bar_insufficient_history() {
        let mut s = RhaiStrategy::from_script("p", MA_CROSS_RHAI).unwrap();
        let mut ctx = StrategyCtx::new(1_000_000.0);
        let bar = Bar {
            date: "2025-01-15".to_string(),
            code: "600519".to_string(),
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            volume: 1_000_000.0,
            amount: 103_000_000.0,
            turnover_rate: Some(1.0),
            adj_factor: Some(1.0),
            limit_up: Some(113.3),
            limit_down: Some(92.7),
            is_st: false,
        };
        // 历史不足 20 根，应返回空信号
        s.on_init(&mut ctx).await.unwrap();
        let signals = s.on_bar(&bar, &mut ctx).await.unwrap();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_sma_rhai_helper() {
        let mut arr: Array = Vec::new();
        for i in 1..=10 {
            arr.push((i as f64).into());
        }
        assert_eq!(sma_rhai(arr.clone(), 5), 8.0); // 6+7+8+9+10 / 5
        assert_eq!(sma_rhai(arr, 10), 5.5);
    }

    #[test]
    fn test_json_to_rhai_conversion() {
        let v = serde_json::json!({
            "a": 1,
            "b": "hello",
            "c": [1, 2, 3],
            "d": true
        });
        let r = json_value_to_rhai(&v);
        // 简单 smoke test：不崩溃
        let _ = r.to_string();
    }
}

// 抑制 Side 未使用警告（保留以备 future 扩展，如 init 中处理 side 参数）
#[allow(dead_code)]
fn _ensure_side_used(_: Side) {}
