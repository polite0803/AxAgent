//! 拒绝追踪器
//!
//! 追踪工具拒绝历史，当同一工具被连续拒绝次数超过阈值时触发降级。

use std::collections::HashMap;

/// 降级阈值
const DEGRADATION_THRESHOLD: u32 = 5;

/// 拒绝追踪器
#[derive(Debug, Clone)]
pub struct DenialTracker {
    /// 工具名 -> 连续拒绝次数
    denials: HashMap<String, u32>,
    /// 启动降级的工具列表
    degraded: Vec<String>,
}

impl DenialTracker {
    pub fn new() -> Self {
        Self {
            denials: HashMap::new(),
            degraded: Vec::new(),
        }
    }

    /// 记录一次拒绝
    pub fn record_denial(&mut self, tool_name: &str) {
        let count = self.denials.entry(tool_name.to_string()).or_insert(0);
        *count += 1;

        // 达到阈值触发降级
        if *count >= DEGRADATION_THRESHOLD && !self.degraded.contains(&tool_name.to_string()) {
            self.degraded.push(tool_name.to_string());
        }
    }

    /// 记录一次允许（重置计数）
    pub fn record_allow(&mut self, tool_name: &str) {
        self.denials.remove(tool_name);
        self.degraded.retain(|t| t != tool_name);
    }

    /// 是否应该降级该工具
    pub fn should_degrade(&self, tool_name: &str) -> bool {
        self.degraded.contains(&tool_name.to_string())
    }

    /// 获取连续拒绝次数
    pub fn denial_count(&self, tool_name: &str) -> u32 {
        self.denials.get(tool_name).copied().unwrap_or(0)
    }

    /// 重置指定工具的追踪
    pub fn reset(&mut self, tool_name: &str) {
        self.denials.remove(tool_name);
        self.degraded.retain(|t| t != tool_name);
    }

    /// 全量重置
    pub fn reset_all(&mut self) {
        self.denials.clear();
        self.degraded.clear();
    }

    /// 获取所有已降级的工具
    pub fn degraded_tools(&self) -> &[String] {
        &self.degraded
    }
}

impl Default for DenialTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_denial_threshold() {
        let mut tracker = DenialTracker::new();

        for _ in 0..4 {
            tracker.record_denial("rm");
            assert!(!tracker.should_degrade("rm"));
        }

        // 第 5 次触发降级
        tracker.record_denial("rm");
        assert!(tracker.should_degrade("rm"));
    }

    #[test]
    fn test_allow_resets() {
        let mut tracker = DenialTracker::new();

        tracker.record_denial("bash");
        tracker.record_denial("bash");
        assert_eq!(tracker.denial_count("bash"), 2);

        tracker.record_allow("bash");
        assert_eq!(tracker.denial_count("bash"), 0);
    }
}
