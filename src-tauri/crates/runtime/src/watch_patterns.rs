//! Watch Patterns - Background process monitoring for task execution
//!
//! Features:
//! - Pattern-based process monitoring
//! - Resource usage tracking
//! - Alert thresholds
//! - Process lifecycle management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchPattern {
    pub id: String,
    pub name: String,
    pub pattern: ProcessPattern,
    pub alert_threshold: AlertThreshold,
    pub enabled: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPattern {
    pub match_type: MatchType,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    CommandContains,
    CommandEquals,
    NameContains,
    NamePrefix,
    NameRegex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<f64>,
    pub max_duration_secs: Option<u64>,
    pub max_restarts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedProcess {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub started_at: u64,
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub restart_count: u32,
    pub status: ProcessStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub event_type: WatchEventType,
    pub pid: u32,
    pub process_name: String,
    pub message: String,
    pub timestamp: u64,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEventType {
    Started,
    Stopped,
    Restarted,
    MemoryThreshold,
    CpuThreshold,
    DurationThreshold,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct WatchRegistry {
    patterns: HashMap<String, WatchPattern>,
    processes: HashMap<u32, WatchedProcess>,
    events: Vec<WatchEvent>,
}

impl WatchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_pattern(&mut self, pattern: WatchPattern) {
        self.patterns.insert(pattern.id.clone(), pattern);
    }

    pub fn unregister_pattern(&mut self, pattern_id: &str) -> Option<WatchPattern> {
        self.patterns.remove(pattern_id)
    }

    pub fn get_pattern(&self, pattern_id: &str) -> Option<&WatchPattern> {
        self.patterns.get(pattern_id)
    }

    pub fn list_patterns(&self) -> Vec<&WatchPattern> {
        self.patterns.values().collect()
    }

    pub fn list_enabled_patterns(&self) -> Vec<&WatchPattern> {
        self.patterns.values().filter(|p| p.enabled).collect()
    }

    pub fn track_process(&mut self, process: WatchedProcess) {
        self.processes.insert(process.pid, process);
    }

    pub fn untrack_process(&mut self, pid: u32) -> Option<WatchedProcess> {
        self.processes.remove(&pid)
    }

    pub fn get_process(&self, pid: u32) -> Option<&WatchedProcess> {
        self.processes.get(&pid)
    }

    pub fn list_processes(&self) -> Vec<&WatchedProcess> {
        self.processes.values().collect()
    }

    pub fn record_event(&mut self, event: WatchEvent) {
        self.events.push(event);
        if self.events.len() > 1000 {
            self.events.drain(0..100);
        }
    }

    pub fn get_events(&self, event_type: Option<WatchEventType>) -> Vec<&WatchEvent> {
        match event_type {
            Some(t) => self.events.iter().filter(|e| e.event_type == t).collect(),
            None => self.events.iter().collect(),
        }
    }

    pub fn check_thresholds(&self, pid: u32) -> Vec<WatchEventType> {
        let process = match self.processes.get(&pid) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut triggered = Vec::new();

        for pattern in self.patterns.values().filter(|p| p.enabled) {
            if !self.matches_pattern(process, &pattern.pattern) {
                continue;
            }

            if let Some(max_mem) = pattern.alert_threshold.max_memory_mb {
                if process.memory_mb > max_mem {
                    triggered.push(WatchEventType::MemoryThreshold);
                }
            }

            if let Some(max_cpu) = pattern.alert_threshold.max_cpu_percent {
                if process.cpu_percent > max_cpu {
                    triggered.push(WatchEventType::CpuThreshold);
                }
            }

            if let Some(max_dur) = pattern.alert_threshold.max_duration_secs {
                let duration = current_timestamp() - process.started_at;
                if duration > max_dur {
                    triggered.push(WatchEventType::DurationThreshold);
                }
            }

            if let Some(max_restarts) = pattern.alert_threshold.max_restarts {
                if process.restart_count > max_restarts {
                    triggered.push(WatchEventType::Restarted);
                }
            }
        }

        triggered
    }

    fn matches_pattern(&self, process: &WatchedProcess, pattern: &ProcessPattern) -> bool {
        match pattern.match_type {
            MatchType::CommandContains => process.command.contains(&pattern.value),
            MatchType::CommandEquals => process.command == pattern.value,
            MatchType::NameContains => process.name.contains(&pattern.value),
            MatchType::NamePrefix => process.name.starts_with(&pattern.value),
            MatchType::NameRegex => regex::Regex::new(&pattern.value)
                .map(|re| re.is_match(&process.name))
                .unwrap_or(false),
        }
    }

    pub fn update_process_stats(&mut self, pid: u32, memory_mb: u64, cpu_percent: f64) {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.memory_mb = memory_mb;
            process.cpu_percent = cpu_percent;
        }
    }

    pub fn increment_restart(&mut self, pid: u32) {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.restart_count += 1;
        }
    }
}

pub struct ProcessWatcher {
    registry: Arc<RwLock<WatchRegistry>>,
}

impl ProcessWatcher {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(WatchRegistry::new())),
        }
    }

    pub fn create_pattern(
        &self,
        name: &str,
        match_type: MatchType,
        value: &str,
        threshold: AlertThreshold,
    ) -> WatchPattern {
        let pattern = WatchPattern {
            id: format!("pattern_{}", uuid_simple()),
            name: name.to_string(),
            pattern: ProcessPattern {
                match_type,
                value: value.to_string(),
            },
            alert_threshold: threshold,
            enabled: true,
            created_at: current_timestamp(),
        };

        let mut reg = self.registry.write().unwrap();
        reg.register_pattern(pattern.clone());
        pattern
    }

    pub fn track(&self, pid: u32, name: &str, command: &str) -> Result<(), String> {
        let mut reg = self.registry.write().unwrap();

        let process = WatchedProcess {
            pid,
            name: name.to_string(),
            command: command.to_string(),
            started_at: current_timestamp(),
            memory_mb: 0,
            cpu_percent: 0.0,
            restart_count: 0,
            status: ProcessStatus::Running,
        };

        reg.track_process(process);

        reg.record_event(WatchEvent {
            event_type: WatchEventType::Started,
            pid,
            process_name: name.to_string(),
            message: format!("Process {} started with PID {}", name, pid),
            timestamp: current_timestamp(),
            metadata: None,
        });

        Ok(())
    }

    pub fn untrack(&self, pid: u32) -> Result<(), String> {
        let mut reg = self.registry.write().unwrap();

        if let Some(process) = reg.untrack_process(pid) {
            let process_name = process.name.clone();
            let msg = format!("Process {} stopped", process_name);
            reg.record_event(WatchEvent {
                event_type: WatchEventType::Stopped,
                pid,
                process_name,
                message: msg,
                timestamp: current_timestamp(),
                metadata: None,
            });
        }

        Ok(())
    }

    pub fn check_and_alert(&self, pid: u32) -> Vec<WatchEvent> {
        let mut reg = self.registry.write().unwrap();
        let thresholds = reg.check_thresholds(pid);

        let mut events = Vec::new();
        let process = match reg.get_process(pid) {
            Some(p) => p.clone(),
            None => return events,
        };

        for threshold in thresholds {
            let event = WatchEvent {
                event_type: threshold,
                pid,
                process_name: process.name.clone(),
                message: format!("Process {} triggered {:?} alert", process.name, threshold),
                timestamp: current_timestamp(),
                metadata: Some({
                    let mut m = HashMap::new();
                    m.insert("memory_mb".to_string(), process.memory_mb.to_string());
                    m.insert("cpu_percent".to_string(), process.cpu_percent.to_string());
                    m
                }),
            };
            events.push(event.clone());
            reg.record_event(event);
        }

        events
    }

    pub fn get_processes(&self) -> Vec<WatchedProcess> {
        let reg = self.registry.read().unwrap();
        reg.list_processes().into_iter().cloned().collect()
    }

    pub fn get_events(&self, event_type: Option<WatchEventType>) -> Vec<WatchEvent> {
        let reg = self.registry.read().unwrap();
        reg.get_events(event_type).into_iter().cloned().collect()
    }

    pub fn get_registry(&self) -> Arc<RwLock<WatchRegistry>> {
        self.registry.clone()
    }
}

impl Default for ProcessWatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn uuid_simple() -> String {
    let now = current_timestamp();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_pattern_creation() {
        let watcher = ProcessWatcher::new();

        let pattern = watcher.create_pattern(
            "test_pattern",
            MatchType::CommandContains,
            "node",
            AlertThreshold {
                max_memory_mb: Some(512),
                max_cpu_percent: Some(80.0),
                max_duration_secs: Some(3600),
                max_restarts: Some(3),
            },
        );

        assert_eq!(pattern.name, "test_pattern");
        assert!(pattern.enabled);
    }

    #[test]
    fn test_process_tracking() {
        let watcher = ProcessWatcher::new();
        watcher.track(1234, "node", "node server.js").unwrap();

        let processes = watcher.get_processes();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 1234);
    }

    #[test]
    fn test_threshold_alert() {
        let watcher = ProcessWatcher::new();

        let _pattern = watcher.create_pattern(
            "high_mem",
            MatchType::NameContains,
            "node",
            AlertThreshold {
                max_memory_mb: Some(100),
                max_cpu_percent: None,
                max_duration_secs: None,
                max_restarts: None,
            },
        );

        {
            let mut reg = watcher.registry.write().unwrap();
            reg.track_process(WatchedProcess {
                pid: 1,
                name: "node".to_string(),
                command: "node test.js".to_string(),
                started_at: current_timestamp() - 100,
                memory_mb: 150,
                cpu_percent: 10.0,
                restart_count: 0,
                status: ProcessStatus::Running,
            });
        }

        let alerts = watcher.check_and_alert(1);
        assert!(!alerts.is_empty());
    }
}
