use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub operation: String,
    pub parameters: serde_json::Value,
    pub risk_level: RiskLevel,
    pub confirmed: bool,
    pub result: Option<String>,
}

pub struct OperationAuditor {
    entries: Mutex<Vec<AuditEntry>>,
    confirm_threshold: Mutex<RiskLevel>,
}

impl OperationAuditor {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            confirm_threshold: Mutex::new(RiskLevel::Medium),
        }
    }

    pub fn record(&self, entry: AuditEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(entry);
        if entries.len() > 1000 {
            entries.drain(0..100);
        }
    }

    pub fn needs_confirmation(&self, risk: &RiskLevel) -> bool {
        let threshold = self.confirm_threshold.lock().unwrap();
        matches!(
            (risk, &*threshold),
            (RiskLevel::High, _)
                | (RiskLevel::Medium, RiskLevel::Medium)
                | (RiskLevel::Medium, RiskLevel::Low)
        )
    }

    pub fn recent(&self, n: usize) -> Vec<AuditEntry> {
        let entries = self.entries.lock().unwrap();
        entries.iter().rev().take(n).cloned().collect()
    }

    pub fn set_confirm_threshold(&self, level: RiskLevel) {
        let mut threshold = self.confirm_threshold.lock().unwrap();
        *threshold = level;
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
    }
}

impl Default for OperationAuditor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_audit_entry(
    operation: &str,
    parameters: serde_json::Value,
    risk_level: RiskLevel,
) -> AuditEntry {
    AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        operation: operation.to_string(),
        parameters,
        risk_level,
        confirmed: false,
        result: None,
    }
}
