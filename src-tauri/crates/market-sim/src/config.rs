//! 模拟配置 — DES Kernel 运行参数、延迟矩阵、Agent 布局。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{Price, SimTimestamp};

/// 模拟器全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    /// 最大模拟时间（纳秒），到达后停止。0 = 无限。
    ///
    /// 默认 600_000_000_000（10 分钟），防止多个 Agent WakeupAfter 自续约导致死循环。
    /// 修复 H3.8: 原默认值 0（无限）会导致 SimConfig::default() 永不停止。
    pub max_time_ns: SimTimestamp,
    /// 随机种子（用于可复现仿真）
    pub seed: u64,
    /// 股票代码（模拟的目标股票）
    pub stock_code: String,
    /// 参考价格（用于 Agent 初始校准，单位：分）
    pub reference_price: Price,
    /// 最小价格变动单位（A 股 = 1 分）
    pub tick_size: Price,
    /// Agent 间默认延迟（纳秒），未在 latency_matrix 中配置的 Agent 对使用此值
    pub default_latency_ns: SimTimestamp,
    /// 启用详细追踪日志
    pub trace: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            // 修复 H3.8: 默认 10 分钟，防止 Agent 自续约导致死循环
            max_time_ns: 600_000_000_000,
            seed: 42,
            stock_code: "000001".to_string(),
            reference_price: 1000, // 10.00 元
            tick_size: 1,
            default_latency_ns: 1_000_000, // 1ms
            trace: false,
        }
    }
}

/// Agent 间延迟矩阵
///
/// 配置不同 Agent 之间的消息传递延迟。
/// 未配置的 Agent 对使用 `SimConfig::default_latency_ns`。
#[derive(Debug, Clone, Default)]
pub struct LatencyMatrix {
    /// (source_agent_id, target_agent_id) → 延迟（纳秒）
    entries: HashMap<(String, String), SimTimestamp>,
    /// 按 Agent 类型配置的延迟基准（未找到精确匹配时使用类型匹配）
    type_entries: HashMap<(String, String), SimTimestamp>,
}

impl LatencyMatrix {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), type_entries: HashMap::new() }
    }

    /// 设置两个特定 Agent 之间的延迟
    pub fn set(&mut self, source: &str, target: &str, latency_ns: SimTimestamp) {
        self.entries.insert((source.to_string(), target.to_string()), latency_ns);
    }

    /// 设置两种 Agent 类型之间的默认延迟
    pub fn set_type(&mut self, source_type: &str, target_type: &str, latency_ns: SimTimestamp) {
        self.type_entries.insert((source_type.to_string(), target_type.to_string()), latency_ns);
    }

    /// 查询延迟：精确匹配 > 类型匹配 > default_latency_ns
    pub fn get(
        &self,
        source_id: &str,
        target_id: &str,
        source_type: &str,
        target_type: &str,
        default_ns: SimTimestamp,
    ) -> SimTimestamp {
        // 1. 精确 Agent ID 匹配
        let key = (source_id.to_string(), target_id.to_string());
        if let Some(&latency) = self.entries.get(&key) {
            return latency;
        }

        // 2. Agent 类型匹配
        let type_key = (source_type.to_string(), target_type.to_string());
        if let Some(&latency) = self.type_entries.get(&type_key) {
            return latency;
        }

        // 3. 默认
        default_ns
    }
}
