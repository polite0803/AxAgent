use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopWarningLevel {
    None,
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopWarning {
    pub level: LoopWarningLevel,
    pub message: String,
    pub consecutive_failures: usize,
    pub repeated_pattern: Option<String>,
}

impl LoopWarning {
    pub fn new(level: LoopWarningLevel, message: String, consecutive_failures: usize) -> Self {
        Self {
            level,
            message,
            consecutive_failures,
            repeated_pattern: None,
        }
    }

    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.repeated_pattern = Some(pattern);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallStats {
    pub tool_name: String,
    pub call_count: usize,
    pub error_count: usize,
    pub total_execution_time_ms: u64,
    pub unique_inputs: usize,
    pub unique_outputs: usize,
}

impl ToolCallStats {
    pub fn new(tool_name: String) -> Self {
        Self {
            tool_name,
            call_count: 0,
            error_count: 0,
            total_execution_time_ms: 0,
            unique_inputs: 0,
            unique_outputs: 0,
        }
    }

    pub fn average_execution_time_ms(&self) -> u64 {
        if self.call_count == 0 {
            0
        } else {
            self.total_execution_time_ms / self.call_count as u64
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.error_count as f64 / self.call_count as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoopDetectorConfig {
    pub max_history: usize,
    pub max_consecutive_failures: usize,
    pub max_tool_calls_per_tool: usize,
    pub pattern_check_window: usize,
    pub slow_call_threshold_ms: u64,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            max_consecutive_failures: 5,
            max_tool_calls_per_tool: 10,
            pattern_check_window: 10,
            slow_call_threshold_ms: 5000,
        }
    }
}

pub struct LoopDetector {
    config: LoopDetectorConfig,
    recent_call_count: usize,
    consecutive_failures: usize,
    tool_stats: std::collections::HashMap<String, ToolCallStats>,
    state_sequence: VecDeque<u64>,
    input_hashes: std::collections::HashMap<String, std::collections::HashSet<u64>>,
    output_hashes: std::collections::HashMap<String, std::collections::HashSet<u64>>,
}

impl LoopDetector {
    pub fn new(config: LoopDetectorConfig) -> Self {
        let pattern_check_window = config.pattern_check_window;
        Self {
            config: config.clone(),
            recent_call_count: 0,
            consecutive_failures: 0,
            tool_stats: std::collections::HashMap::new(),
            state_sequence: VecDeque::with_capacity(pattern_check_window),
            input_hashes: std::collections::HashMap::new(),
            output_hashes: std::collections::HashMap::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(LoopDetectorConfig::default())
    }

    pub fn record_call(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
        is_error: bool,
        execution_time_ms: u64,
    ) {
        let input_hash = self.hash_content(input);
        let output_hash = self.hash_content(output);

        self.recent_call_count += 1;
        if self.recent_call_count > self.config.max_history {
            self.recent_call_count = self.config.max_history;
        }

        let stats = self
            .tool_stats
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolCallStats::new(tool_name.to_string()));
        stats.call_count += 1;
        stats.total_execution_time_ms += execution_time_ms;
        if is_error {
            stats.error_count += 1;
        }

        let inputs = self.input_hashes.entry(tool_name.to_string()).or_default();
        if inputs.insert(input_hash) {
            stats.unique_inputs = inputs.len();
        }

        let outputs = self.output_hashes.entry(tool_name.to_string()).or_default();
        if outputs.insert(output_hash) {
            stats.unique_outputs = outputs.len();
        }

        if is_error {
            self.consecutive_failures += 1;
        } else {
            self.consecutive_failures = 0;
        }

        self.state_sequence.push_back(output_hash);
        if self.state_sequence.len() > self.config.pattern_check_window {
            self.state_sequence.pop_front();
        }
    }

    pub fn detect_loop(&self) -> Option<LoopWarning> {
        if self.consecutive_failures >= self.config.max_consecutive_failures {
            return Some(LoopWarning::new(
                if self.consecutive_failures >= self.config.max_consecutive_failures * 2 {
                    LoopWarningLevel::Critical
                } else {
                    LoopWarningLevel::Warning
                },
                format!(
                    "Detected {} consecutive tool failures",
                    self.consecutive_failures
                ),
                self.consecutive_failures,
            ));
        }

        if let Some(pattern) = self.detect_output_pattern() {
            return Some(LoopWarning::new(
                LoopWarningLevel::Warning,
                "Detected repeating output pattern".to_string(),
                self.consecutive_failures,
            )
            .with_pattern(pattern));
        }

        for (tool_name, stats) in &self.tool_stats {
            if stats.call_count > self.config.max_tool_calls_per_tool {
                let error_rate = stats.error_rate();
                if error_rate > 0.5 {
                    return Some(LoopWarning::new(
                        LoopWarningLevel::Critical,
                        format!(
                            "Tool '{}' called {} times with {:.1}% error rate",
                            tool_name,
                            stats.call_count,
                            error_rate * 100.0
                        ),
                        self.consecutive_failures,
                    ));
                }
            }

            if stats.average_execution_time_ms() > self.config.slow_call_threshold_ms {
                return Some(LoopWarning::new(
                    LoopWarningLevel::Info,
                    format!(
                        "Tool '{}' has slow average execution time: {}ms",
                        tool_name,
                        stats.average_execution_time_ms()
                    ),
                    self.consecutive_failures,
                ));
            }
        }

        None
    }

    fn detect_output_pattern(&self) -> Option<String> {
        if self.state_sequence.len() < 4 {
            return None;
        }

        let seq: Vec<u64> = self.state_sequence.iter().copied().collect();

        for pattern_len in 2..=(seq.len() / 2) {
            let mut has_repeating = true;
            for window_start in 0..pattern_len {
                let base = seq[window_start];
                for i in (window_start..seq.len()).step_by(pattern_len) {
                    if seq.get(i) != Some(&base) {
                        has_repeating = false;
                        break;
                    }
                }
                if has_repeating {
                    let pattern_str = format!("{:?}", &seq[..pattern_len]);
                    return Some(pattern_str);
                }
            }
        }

        None
    }

    pub fn get_tool_stats(&self, tool_name: &str) -> Option<&ToolCallStats> {
        self.tool_stats.get(tool_name)
    }

    pub fn get_all_stats(&self) -> Vec<&ToolCallStats> {
        self.tool_stats.values().collect()
    }

    pub fn get_consecutive_failures(&self) -> usize {
        self.consecutive_failures
    }

    pub fn reset(&mut self) {
        self.recent_call_count = 0;
        self.consecutive_failures = 0;
        self.tool_stats.clear();
        self.state_sequence.clear();
        self.input_hashes.clear();
        self.output_hashes.clear();
    }

    fn hash_content(&self, content: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new(LoopDetectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consecutive_failure_detection() {
        let mut detector = LoopDetector::with_default_config();
        detector.config.max_consecutive_failures = 3;

        for i in 0..3 {
            detector.record_call(
                "test_tool",
                &format!("input_{}", i),
                "error_output",
                true,
                100,
            );
        }

        let warning = detector.detect_loop();
        assert!(warning.is_some());
        let warning = warning.unwrap();
        assert!(matches!(warning.level, LoopWarningLevel::Warning));
        assert_eq!(warning.consecutive_failures, 3);
    }

    #[test]
    fn test_successful_call_resets_failures() {
        let mut detector = LoopDetector::with_default_config();

        detector.record_call("tool", "input1", "error", true, 100);
        detector.record_call("tool", "input2", "error", true, 100);
        assert_eq!(detector.get_consecutive_failures(), 2);

        detector.record_call("tool", "input3", "success", false, 100);
        assert_eq!(detector.get_consecutive_failures(), 0);
    }

    #[test]
    fn test_tool_stats() {
        let mut detector = LoopDetector::with_default_config();

        detector.record_call("slow_tool", "input1", "output1", false, 6000);
        detector.record_call("slow_tool", "input2", "output2", false, 6000);

        let stats = detector.get_tool_stats("slow_tool").unwrap();
        assert_eq!(stats.call_count, 2);
        assert_eq!(stats.average_execution_time_ms(), 6000);
    }

    #[test]
    fn test_error_rate_detection() {
        let mut detector = LoopDetector::with_default_config();
        detector.config.max_tool_calls_per_tool = 5;

        for i in 0..6 {
            let is_error = i < 5;
            detector.record_call("faulty_tool", &format!("input_{}", i), "output", is_error, 100);
        }

        let warning = detector.detect_loop();
        assert!(warning.is_some());
        let warning = warning.unwrap();
        assert!(matches!(warning.level, LoopWarningLevel::Critical));
    }

    #[test]
    fn test_reset() {
        let mut detector = LoopDetector::with_default_config();

        detector.record_call("tool", "input", "output", true, 100);
        detector.record_call("tool", "input", "output", true, 100);
        assert_eq!(detector.get_consecutive_failures(), 2);

        detector.reset();
        assert_eq!(detector.get_consecutive_failures(), 0);
        assert_eq!(detector.recent_call_count, 0);
    }
}
