use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 循环检测告警类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleAlert {
    /// 重复调用：相同的工具+参数被反复执行
    RepeatCall {
        tool_name: String,
        count: usize,
        first_seen_at_iteration: usize,
    },
    /// 无进展：连续多次迭代没有产生新的实质性进展
    NoProgress { stagnant_iterations: usize },
}

/// ReAct 主循环的语义循环检测器
///
/// 通过两重机制防止死循环：
/// 1. 工具+参数哈希重复检测 — 完全相同调用超过阈值则告警
/// 2. 状态收敛检测 — 连续多次迭代后思考链无实质性变化则告警
pub struct CycleDetector {
    /// 哈希(tool_name + input) -> 调用次数
    repeated_calls: HashMap<u64, usize>,
    /// 哈希 -> 首次出现的迭代编号
    first_seen_at: HashMap<u64, usize>,
    /// 连续无进展迭代计数
    no_progress_count: usize,
    /// 上一次记录的 chain 步骤数
    last_chain_len: usize,
    /// 上一次记录的最后一个 observation 哈希
    last_observation_hash: Option<u64>,
    /// 最大允许的单一调用重复次数
    max_repeat_calls: usize,
    /// 最大允许的无进展迭代次数
    max_no_progress: usize,
}

impl CycleDetector {
    pub fn new(max_repeat_calls: usize, max_no_progress: usize) -> Self {
        Self {
            repeated_calls: HashMap::new(),
            first_seen_at: HashMap::new(),
            no_progress_count: 0,
            last_chain_len: 0,
            last_observation_hash: None,
            max_repeat_calls,
            max_no_progress: max_no_progress.max(1),
        }
    }

    /// 检查工具+参数的组合是否已重复调用超过阈值
    ///
    /// 返回 `Some(CycleAlert)` 表示检测到循环，`None` 表示正常。
    pub fn check_repeated_call(
        &mut self,
        tool_name: &str,
        tool_input: &str,
        iteration: usize,
    ) -> Option<CycleAlert> {
        let hash = hash_call(tool_name, tool_input);

        let count = self.repeated_calls.entry(hash).or_insert(0);
        *count += 1;

        self.first_seen_at.entry(hash).or_insert(iteration);

        if *count >= self.max_repeat_calls {
            Some(CycleAlert::RepeatCall {
                tool_name: tool_name.to_string(),
                count: *count,
                first_seen_at_iteration: self.first_seen_at[&hash],
            })
        } else {
            None
        }
    }

    /// 检查思考链是否连续多次迭代无实质性进展
    ///
    /// 通过比较 chain 长度和最后一个 observation 来判断是否有进展。
    /// 返回 `Some(CycleAlert)` 表示检测到停滞，`None` 表示正常。
    pub fn check_state_convergence(
        &mut self,
        chain_len: usize,
        latest_observation: Option<&str>,
    ) -> Option<CycleAlert> {
        let current_obs_hash = latest_observation.map(hash_str);

        let has_progress = chain_len > self.last_chain_len
            || (current_obs_hash.is_some()
                && self.last_observation_hash.is_some()
                && current_obs_hash != self.last_observation_hash);

        if has_progress {
            self.no_progress_count = 0;
        } else {
            self.no_progress_count += 1;
        }

        self.last_chain_len = chain_len;
        self.last_observation_hash = current_obs_hash;

        if self.no_progress_count >= self.max_no_progress {
            Some(CycleAlert::NoProgress {
                stagnant_iterations: self.no_progress_count,
            })
        } else {
            None
        }
    }

    /// 记录一步执行（内部调用 check_repeated_call 和 check_state_convergence 的便捷方法）
    ///
    /// 返回检测到的告警列表。
    pub fn record_step(
        &mut self,
        tool_name: &str,
        tool_input: &str,
        chain_len: usize,
        latest_observation: Option<&str>,
        iteration: usize,
    ) -> Vec<CycleAlert> {
        let mut alerts = Vec::new();

        if let Some(alert) = self.check_repeated_call(tool_name, tool_input, iteration) {
            alerts.push(alert);
        }

        if let Some(alert) = self.check_state_convergence(chain_len, latest_observation) {
            alerts.push(alert);
        }

        alerts
    }

    /// 重置所有检测状态
    pub fn reset(&mut self) {
        self.repeated_calls.clear();
        self.first_seen_at.clear();
        self.no_progress_count = 0;
        self.last_chain_len = 0;
        self.last_observation_hash = None;
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new(3, 5)
    }
}

/// 对工具名+输入做稳定哈希
fn hash_call(tool_name: &str, tool_input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    tool_name.hash(&mut hasher);
    tool_input.hash(&mut hasher);
    hasher.finish()
}

/// 对字符串做稳定哈希
fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeat_call_detection() {
        let mut detector = CycleDetector::new(3, 5);

        // 前两次不告警
        assert!(detector.check_repeated_call("bash", "ls -la", 1).is_none());
        assert!(detector.check_repeated_call("bash", "ls -la", 2).is_none());

        // 第三次触发告警
        let alert = detector.check_repeated_call("bash", "ls -la", 3).unwrap();
        assert_eq!(
            alert,
            CycleAlert::RepeatCall {
                tool_name: "bash".to_string(),
                count: 3,
                first_seen_at_iteration: 1,
            }
        );
    }

    #[test]
    fn test_different_inputs_no_alert() {
        let mut detector = CycleDetector::new(3, 5);

        assert!(detector.check_repeated_call("bash", "ls -la", 1).is_none());
        assert!(
            detector
                .check_repeated_call("bash", "cat file1", 2)
                .is_none()
        );
        assert!(
            detector
                .check_repeated_call("bash", "cat file2", 3)
                .is_none()
        );
    }

    #[test]
    fn test_state_convergence_no_progress() {
        let mut detector = CycleDetector::new(3, 3);

        // 第一步：有进展（chain 增长）
        assert!(
            detector
                .check_state_convergence(1, Some("output1"))
                .is_none()
        );
        // 第二步：无进展（chain 长度不变，observation 相同）
        assert!(
            detector
                .check_state_convergence(1, Some("output1"))
                .is_none()
        );
        // 第三步：仍无进展
        assert!(
            detector
                .check_state_convergence(1, Some("output1"))
                .is_none()
        );
        // 第四步：触发告警
        let alert = detector
            .check_state_convergence(1, Some("output1"))
            .unwrap();
        assert_eq!(
            alert,
            CycleAlert::NoProgress {
                stagnant_iterations: 3
            }
        );
    }

    #[test]
    fn test_state_convergence_chain_growth_is_progress() {
        let mut detector = CycleDetector::new(3, 3);

        // chain 持续增长 = 有进展
        assert!(detector.check_state_convergence(1, None).is_none());
        assert!(detector.check_state_convergence(2, None).is_none());
        assert!(detector.check_state_convergence(3, None).is_none());
        assert!(detector.check_state_convergence(4, None).is_none());
    }

    #[test]
    fn test_observation_change_is_progress() {
        let mut detector = CycleDetector::new(3, 3);

        assert!(
            detector
                .check_state_convergence(1, Some("output A"))
                .is_none()
        );
        assert!(
            detector
                .check_state_convergence(1, Some("output B"))
                .is_none()
        );
        assert!(
            detector
                .check_state_convergence(1, Some("output C"))
                .is_none()
        );
        // 每次 observation 都不同，不应该触发告警
    }

    #[test]
    fn test_reset() {
        let mut detector = CycleDetector::new(2, 3);

        detector.check_repeated_call("bash", "cmd", 1);
        detector.check_state_convergence(1, Some("same"));

        detector.reset();

        assert_eq!(detector.no_progress_count, 0);
        assert_eq!(detector.last_chain_len, 0);
        assert!(detector.repeated_calls.is_empty());
    }

    #[test]
    fn test_record_step_convenience() {
        let mut detector = CycleDetector::new(2, 2);

        // 正常步骤
        let alerts = detector.record_step("bash", "cmd", 1, Some("out1"), 1);
        assert!(alerts.is_empty());

        // 重复步骤
        let alerts = detector.record_step("bash", "cmd", 1, Some("out1"), 2);
        assert_eq!(alerts.len(), 2); // 同时触发 RepeatCall 和 NoProgress
    }

    #[test]
    fn test_default_constructor() {
        let detector = CycleDetector::default();
        assert_eq!(detector.max_repeat_calls, 3);
        assert_eq!(detector.max_no_progress, 5);
    }
}
