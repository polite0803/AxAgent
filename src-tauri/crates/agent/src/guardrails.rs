// SPDX-License-Identifier: AGPL-3.0-only
//! G9 ToolCallGuardrailController
//!
//! 工具调用护栏控制器，防止 Agent Loop 陷入重复失败循环。
//!
//! ## 四类阈值
//!
//! | 阈值类型                  | 默认值 | 触发动作                     |
//! |--------------------------|--------|------------------------------|
//! | `exact_failure_warn`     | 2      | 相同工具+参数失败 2 次 → 警告 |
//! | `exact_failure_block`    | 5      | 相同工具+参数失败 5 次 → 阻止 |
//! | `same_tool_failure_warn` | 3      | 相同工具失败 3 次 → 警告      |
//! | `same_tool_failure_halt` | 8      | 相同工具失败 8 次 → 停止      |
//!
//! ## 幂等检测
//!
//! 使用 SHA256 哈希工具调用参数，识别重复调用。
//! 对幂等工具（如 `get_*` / `list_*` / `search_*`）允许重复调用，
//! 对突变工具（如 `write_*` / `delete_*` / `send_*`）严格限制重复失败。
//!
//! ## 使用方式
//!
//! ```ignore
//! use axagent_agent::guardrails::ToolCallGuardrailController;
//!
//! let mut ctrl = ToolCallGuardrailController::default();
//! // 检查是否允许调用
//! let decision = ctrl.check_allowed("get_stock_quote", r#"{"stock_code":"600519"}"#);
//! // 记录调用结果
//! ctrl.record_call("get_stock_quote", r#"{"stock_code":"600519"}"#, false);
//! ```

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// 注意：Serialize 通过 derive 使用，无需显式 use

/// 护栏决策
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardrailDecision {
    /// 允许调用
    Allow,
    /// 允许调用但发出警告（附带原因）
    Warn(String),
    /// 阻止此次调用（附带原因）
    Block(String),
    /// 停止整个 Agent Loop（附带原因）
    Halt(String),
}

impl GuardrailDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow | Self::Warn(_))
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Block(_) | Self::Halt(_))
    }
}

/// 护栏阈值配置
#[derive(Debug, Clone)]
pub struct GuardrailThresholds {
    /// 相同工具+参数失败警告阈值
    pub exact_failure_warn: u32,
    /// 相同工具+参数失败阻止阈值
    pub exact_failure_block: u32,
    /// 相同工具失败警告阈值
    pub same_tool_failure_warn: u32,
    /// 相同工具失败停止阈值
    pub same_tool_failure_halt: u32,
}

impl Default for GuardrailThresholds {
    fn default() -> Self {
        Self {
            exact_failure_warn: 2,
            exact_failure_block: 5,
            same_tool_failure_warn: 3,
            same_tool_failure_halt: 8,
        }
    }
}

/// 单个工具+参数组合的调用记录
#[derive(Debug, Clone)]
struct CallRecord {
    /// 失败次数
    failure_count: u32,
    /// 总调用次数
    total_count: u32,
    /// 最后调用时间（Unix 毫秒）
    last_call_ts: i64,
}

/// 单个工具的统计
#[derive(Debug, Clone, Default)]
struct ToolStats {
    /// 总失败次数
    total_failures: u32,
    /// 总调用次数
    total_calls: u32,
    /// 按 args 哈希分组的调用记录
    per_args: HashMap<String, CallRecord>,
}

/// 工具调用护栏控制器
pub struct ToolCallGuardrailController {
    /// 阈值配置
    thresholds: GuardrailThresholds,
    /// 按工具名分组的统计（内部加锁保护）
    stats: Mutex<HashMap<String, ToolStats>>,
    /// 突变工具名前缀列表（命中即视为突变工具，限制更严格）
    mutation_prefixes: Vec<String>,
    /// 幂等工具名前缀列表（命中即视为幂等工具，允许重复调用）
    idempotent_prefixes: Vec<String>,
}

impl Default for ToolCallGuardrailController {
    fn default() -> Self {
        Self::new(GuardrailThresholds::default())
    }
}

impl ToolCallGuardrailController {
    /// 创建新的护栏控制器
    pub fn new(thresholds: GuardrailThresholds) -> Self {
        Self {
            thresholds,
            stats: Mutex::new(HashMap::new()),
            mutation_prefixes: vec![
                "write_".to_string(),
                "delete_".to_string(),
                "send_".to_string(),
                "update_".to_string(),
                "create_".to_string(),
                "execute_".to_string(),
                "submit_".to_string(),
                "cancel_".to_string(),
            ],
            idempotent_prefixes: vec![
                "get_".to_string(),
                "list_".to_string(),
                "search_".to_string(),
                "query_".to_string(),
                "fetch_".to_string(),
                "read_".to_string(),
            ],
        }
    }

    /// 自定义突变工具前缀
    pub fn with_mutation_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.mutation_prefixes = prefixes;
        self
    }

    /// 自定义幂等工具前缀
    pub fn with_idempotent_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.idempotent_prefixes = prefixes;
        self
    }

    /// 判断工具是否为突变工具
    pub fn is_mutation_tool(&self, tool_name: &str) -> bool {
        self.mutation_prefixes.iter().any(|p| tool_name.starts_with(p))
    }

    /// 判断工具是否为幂等工具
    pub fn is_idempotent_tool(&self, tool_name: &str) -> bool {
        self.idempotent_prefixes.iter().any(|p| tool_name.starts_with(p))
    }

    /// 计算参数的 SHA256 哈希
    fn hash_args(args: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(args.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// 当前 Unix 毫秒时间戳
    fn now_ms() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
    }

    /// 检查是否允许调用工具
    pub fn check_allowed(&self, tool_name: &str, args: &str) -> GuardrailDecision {
        let args_hash = Self::hash_args(args);
        let stats = self.stats.lock().unwrap();

        let tool_stat = match stats.get(tool_name) {
            Some(s) => s,
            None => return GuardrailDecision::Allow,
        };

        // 检查相同工具的总失败次数
        if tool_stat.total_failures >= self.thresholds.same_tool_failure_halt {
            return GuardrailDecision::Halt(format!(
                "工具 '{}' 已连续失败 {} 次（达到 halt 阈值 {}），停止 Agent Loop",
                tool_name, tool_stat.total_failures, self.thresholds.same_tool_failure_halt
            ));
        }

        // 检查相同工具+参数的失败次数
        if let Some(record) = tool_stat.per_args.get(&args_hash)
            && record.failure_count >= self.thresholds.exact_failure_block
        {
            return GuardrailDecision::Block(format!(
                "工具 '{}' 相同参数已失败 {} 次（达到 block 阈值 {}），阻止此次调用",
                tool_name, record.failure_count, self.thresholds.exact_failure_block
            ));
        }

        // 警告级别（允许调用但提示）
        if tool_stat.total_failures >= self.thresholds.same_tool_failure_warn {
            return GuardrailDecision::Warn(format!(
                "工具 '{}' 已失败 {} 次（达到 warn 阈值 {}）",
                tool_name, tool_stat.total_failures, self.thresholds.same_tool_failure_warn
            ));
        }

        if let Some(record) = tool_stat.per_args.get(&args_hash)
            && record.failure_count >= self.thresholds.exact_failure_warn
        {
            return GuardrailDecision::Warn(format!(
                "工具 '{}' 相同参数已失败 {} 次（达到 warn 阈值 {}）",
                tool_name, record.failure_count, self.thresholds.exact_failure_warn
            ));
        }

        GuardrailDecision::Allow
    }

    /// 记录工具调用结果
    pub fn record_call(&self, tool_name: &str, args: &str, success: bool) {
        let args_hash = Self::hash_args(args);
        let now = Self::now_ms();
        let mut stats = self.stats.lock().unwrap();

        let tool_stat = stats.entry(tool_name.to_string()).or_default();
        tool_stat.total_calls += 1;

        let record = tool_stat.per_args.entry(args_hash).or_insert_with(|| CallRecord {
            failure_count: 0,
            total_count: 0,
            last_call_ts: 0,
        });

        record.total_count += 1;
        record.last_call_ts = now;

        if !success {
            record.failure_count += 1;
            tool_stat.total_failures += 1;
        }
    }

    /// 获取工具的统计快照
    pub fn get_stats(&self, tool_name: &str) -> Option<ToolStatsSnapshot> {
        let stats = self.stats.lock().unwrap();
        stats.get(tool_name).map(|s| ToolStatsSnapshot {
            total_calls: s.total_calls,
            total_failures: s.total_failures,
            unique_args_count: s.per_args.len(),
        })
    }

    /// 清除工具的统计记录（用于重置）
    pub fn reset_tool(&self, tool_name: &str) {
        let mut stats = self.stats.lock().unwrap();
        stats.remove(tool_name);
    }

    /// 清除所有统计记录
    pub fn reset_all(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.clear();
    }
}

/// 工具统计快照（只读视图）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStatsSnapshot {
    pub total_calls: u32,
    pub total_failures: u32,
    pub unique_args_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_allow() {
        let ctrl = ToolCallGuardrailController::default();
        let decision = ctrl.check_allowed("get_stock_quote", r#"{"stock_code":"600519"}"#);
        assert_eq!(decision, GuardrailDecision::Allow);
        assert!(decision.is_allowed());
        assert!(!decision.is_terminal());
    }

    #[test]
    fn test_exact_failure_warn() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "get_stock_quote";
        let args = r#"{"stock_code":"600519"}"#;

        // 失败 2 次 → 第 3 次调用应警告
        ctrl.record_call(tool, args, false);
        ctrl.record_call(tool, args, false);

        let decision = ctrl.check_allowed(tool, args);
        match decision {
            GuardrailDecision::Warn(_) => {},
            other => panic!("期望 Warn，实际: {:?}", other),
        }
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_exact_failure_block() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "write_file";
        let args = r#"{"path":"/tmp/test","content":"hello"}"#;

        // 失败 5 次 → 第 6 次调用应阻止
        for _ in 0..5 {
            ctrl.record_call(tool, args, false);
        }

        let decision = ctrl.check_allowed(tool, args);
        match decision {
            GuardrailDecision::Block(_) => {},
            other => panic!("期望 Block，实际: {:?}", other),
        }
        assert!(!decision.is_allowed());
        assert!(decision.is_terminal());
    }

    #[test]
    fn test_same_tool_failure_halt() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "execute_command";

        // 不同参数失败 8 次 → 应停止
        for i in 0..8 {
            ctrl.record_call(tool, &format!(r#"{{"cmd":"cmd_{i}"}}"#), false);
        }

        let decision = ctrl.check_allowed(tool, r#"{"cmd":"cmd_9"}"#);
        match decision {
            GuardrailDecision::Halt(_) => {},
            other => panic!("期望 Halt，实际: {:?}", other),
        }
        assert!(!decision.is_allowed());
        assert!(decision.is_terminal());
    }

    #[test]
    fn test_mutation_tool_detection() {
        let ctrl = ToolCallGuardrailController::default();
        assert!(ctrl.is_mutation_tool("write_file"));
        assert!(ctrl.is_mutation_tool("delete_record"));
        assert!(ctrl.is_mutation_tool("send_email"));
        assert!(!ctrl.is_mutation_tool("get_stock_quote"));
        assert!(!ctrl.is_mutation_tool("search_news"));
    }

    #[test]
    fn test_idempotent_tool_detection() {
        let ctrl = ToolCallGuardrailController::default();
        assert!(ctrl.is_idempotent_tool("get_stock_quote"));
        assert!(ctrl.is_idempotent_tool("list_files"));
        assert!(ctrl.is_idempotent_tool("search_news"));
        assert!(!ctrl.is_idempotent_tool("write_file"));
    }

    #[test]
    fn test_different_args_independent() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "get_stock_quote";

        // 不同参数各自计数
        ctrl.record_call(tool, r#"{"stock_code":"600519"}"#, false);
        ctrl.record_call(tool, r#"{"stock_code":"000001"}"#, false);

        // 第三种参数应允许
        let decision = ctrl.check_allowed(tool, r#"{"stock_code":"600036"}"#);
        assert_eq!(decision, GuardrailDecision::Allow);
    }

    #[test]
    fn test_success_does_not_count_as_failure() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "get_stock_quote";
        let args = r#"{"stock_code":"600519"}"#;

        // 成功调用多次不应触发警告
        for _ in 0..10 {
            ctrl.record_call(tool, args, true);
        }

        let decision = ctrl.check_allowed(tool, args);
        assert_eq!(decision, GuardrailDecision::Allow);
    }

    #[test]
    fn test_reset_tool() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "get_stock_quote";
        let args = r#"{"stock_code":"600519"}"#;

        // 失败 5 次
        for _ in 0..5 {
            ctrl.record_call(tool, args, false);
        }
        assert!(ctrl.get_stats(tool).is_some());

        // 重置后应清除
        ctrl.reset_tool(tool);
        assert!(ctrl.get_stats(tool).is_none());

        // 重置后应允许调用
        let decision = ctrl.check_allowed(tool, args);
        assert_eq!(decision, GuardrailDecision::Allow);
    }

    #[test]
    fn test_reset_all() {
        let ctrl = ToolCallGuardrailController::default();
        ctrl.record_call("tool1", "{}", false);
        ctrl.record_call("tool2", "{}", false);

        ctrl.reset_all();
        assert!(ctrl.get_stats("tool1").is_none());
        assert!(ctrl.get_stats("tool2").is_none());
    }

    #[test]
    fn test_get_stats() {
        let ctrl = ToolCallGuardrailController::default();
        let tool = "get_stock_quote";

        ctrl.record_call(tool, r#"{"a":1}"#, true);
        ctrl.record_call(tool, r#"{"a":1}"#, false);
        ctrl.record_call(tool, r#"{"a":2}"#, false);

        let stats = ctrl.get_stats(tool).unwrap();
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.total_failures, 2);
        assert_eq!(stats.unique_args_count, 2);
    }

    #[test]
    fn test_custom_thresholds() {
        let thresholds = GuardrailThresholds {
            exact_failure_warn: 1,
            exact_failure_block: 2,
            same_tool_failure_warn: 2,
            same_tool_failure_halt: 3,
        };
        let ctrl = ToolCallGuardrailController::new(thresholds);
        let tool = "test_tool";
        let args = "{}";

        // 失败 1 次 → 警告
        ctrl.record_call(tool, args, false);
        let d1 = ctrl.check_allowed(tool, args);
        assert!(matches!(d1, GuardrailDecision::Warn(_)));

        // 失败 2 次 → 阻止
        ctrl.record_call(tool, args, false);
        let d2 = ctrl.check_allowed(tool, args);
        assert!(matches!(d2, GuardrailDecision::Block(_)));
    }

    #[test]
    fn test_custom_prefixes() {
        let ctrl = ToolCallGuardrailController::default()
            .with_mutation_prefixes(vec!["trade_".to_string()])
            .with_idempotent_prefixes(vec!["peek_".to_string()]);

        assert!(ctrl.is_mutation_tool("trade_buy"));
        assert!(!ctrl.is_mutation_tool("write_file")); // 默认前缀被覆盖
        assert!(ctrl.is_idempotent_tool("peek_data"));
        assert!(!ctrl.is_idempotent_tool("get_data")); // 默认前缀被覆盖
    }

    #[test]
    fn test_hash_args_consistency() {
        let h1 = ToolCallGuardrailController::hash_args(r#"{"a":1}"#);
        let h2 = ToolCallGuardrailController::hash_args(r#"{"a":1}"#);
        let h3 = ToolCallGuardrailController::hash_args(r#"{"a":2}"#);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 64); // SHA256 hex 长度
    }
}
