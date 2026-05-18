//! Immutable Constitution safety mechanism
//!
//! Provides an immutable set of constitutional rules that govern agent behavior,
//! preventing reward hacking, ensuring sandboxed execution, and maintaining
//! alignment with user intent.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstitutionalRule {
    NoSelfModificationOfReward,
    NoCodeExecutionWithoutSandbox,
    PreserveUserIntent,
    MaxModificationSize(f64),
    RequiredHumanApprovalFor(String),
    Custom {
        name: String,
        description: String,
        check_fn_name: String,
    },
}

impl ConstitutionalRule {
    pub fn name(&self) -> &str {
        match self {
            ConstitutionalRule::NoSelfModificationOfReward => "no_self_modification_of_reward",
            ConstitutionalRule::NoCodeExecutionWithoutSandbox => {
                "no_code_execution_without_sandbox"
            },
            ConstitutionalRule::PreserveUserIntent => "preserve_user_intent",
            ConstitutionalRule::MaxModificationSize(_) => "max_modification_size",
            ConstitutionalRule::RequiredHumanApprovalFor(_) => "required_human_approval_for",
            ConstitutionalRule::Custom { name, .. } => name,
        }
    }

    pub fn description(&self) -> String {
        match self {
            ConstitutionalRule::NoSelfModificationOfReward => {
                "Prevents the agent from modifying its own reward function to artificially inflate rewards".to_string()
            }
            ConstitutionalRule::NoCodeExecutionWithoutSandbox => {
                "Ensures all code execution occurs within a sandboxed environment".to_string()
            }
            ConstitutionalRule::PreserveUserIntent => {
                "Requires that all modifications preserve the original user intent".to_string()
            }
            ConstitutionalRule::MaxModificationSize(limit) => {
                format!("Limits modification size to a maximum of {:.0}% of original content", limit * 100.0)
            }
            ConstitutionalRule::RequiredHumanApprovalFor(action) => {
                format!("Requires human approval before: {}", action)
            }
            ConstitutionalRule::Custom { description, .. } => description.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    Warning,
    Critical,
    Fatal,
}

impl ViolationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationSeverity::Warning => "warning",
            ViolationSeverity::Critical => "critical",
            ViolationSeverity::Fatal => "fatal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionViolation {
    pub rule_name: String,
    pub description: String,
    pub severity: ViolationSeverity,
    pub timestamp: DateTime<Utc>,
    pub id: String,
}

impl ConstitutionViolation {
    pub fn new(rule_name: String, description: String, severity: ViolationSeverity) -> Self {
        Self {
            rule_name,
            description,
            severity,
            timestamp: Utc::now(),
            id: Uuid::new_v4().to_string(),
        }
    }
}

pub trait CustomRuleChecker: Send + Sync {
    fn check_skill_modification(
        &self,
        modification: &crate::skill::SkillModification,
    ) -> Option<ConstitutionViolation>;
    fn check_tool_creation(
        &self,
        name: &str,
        code: &str,
        description: &str,
    ) -> Option<ConstitutionViolation>;
    fn check_reward_hacking(&self, reward_history: &[f64]) -> Option<ConstitutionViolation>;
    fn name(&self) -> &str;
}

pub struct CustomRuleRegistry {
    checkers: HashMap<String, Arc<dyn CustomRuleChecker>>,
}

impl CustomRuleRegistry {
    pub fn new() -> Self {
        Self {
            checkers: HashMap::new(),
        }
    }

    pub fn register(&mut self, checker: Arc<dyn CustomRuleChecker>) {
        self.checkers.insert(checker.name().to_string(), checker);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn CustomRuleChecker>> {
        self.checkers.get(name)
    }
}

impl Default for CustomRuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FnCustomRuleChecker {
    name: String,
    #[allow(clippy::type_complexity)]
    check_fn: Box<dyn Fn(&str, &serde_json::Value) -> Option<ConstitutionViolation> + Send + Sync>,
}

impl FnCustomRuleChecker {
    #[allow(clippy::type_complexity)]
    pub fn new(
        name: String,
        check_fn: Box<
            dyn Fn(&str, &serde_json::Value) -> Option<ConstitutionViolation> + Send + Sync,
        >,
    ) -> Self {
        Self { name, check_fn }
    }
}

impl CustomRuleChecker for FnCustomRuleChecker {
    fn check_skill_modification(
        &self,
        modification: &crate::skill::SkillModification,
    ) -> Option<ConstitutionViolation> {
        let value = serde_json::to_value(modification).ok()?;
        (self.check_fn)("skill_modification", &value)
    }

    fn check_tool_creation(
        &self,
        name: &str,
        code: &str,
        description: &str,
    ) -> Option<ConstitutionViolation> {
        let value = serde_json::json!({
            "name": name,
            "code": code,
            "description": description,
        });
        (self.check_fn)("tool_creation", &value)
    }

    fn check_reward_hacking(&self, reward_history: &[f64]) -> Option<ConstitutionViolation> {
        let value = serde_json::json!({
            "reward_history": reward_history,
        });
        (self.check_fn)("reward_hacking", &value)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionConfig {
    pub enabled: bool,
    pub auto_revert_on_critical: bool,
    pub log_violations: bool,
}

impl Default for ConstitutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_revert_on_critical: true,
            log_violations: true,
        }
    }
}

pub struct ImmutableConstitution {
    rules: Vec<ConstitutionalRule>,
    config: ConstitutionConfig,
    violation_log: Vec<ConstitutionViolation>,
    custom_registry: Option<Arc<CustomRuleRegistry>>,
}

impl ImmutableConstitution {
    pub fn new(rules: Vec<ConstitutionalRule>, config: ConstitutionConfig) -> Self {
        Self {
            rules,
            config,
            violation_log: Vec::new(),
            custom_registry: None,
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn with_custom_registry(mut self, registry: Arc<CustomRuleRegistry>) -> Self {
        self.custom_registry = Some(registry);
        self
    }

    pub fn with_default_rules(config: ConstitutionConfig) -> Self {
        let rules = vec![
            ConstitutionalRule::NoSelfModificationOfReward,
            ConstitutionalRule::NoCodeExecutionWithoutSandbox,
            ConstitutionalRule::PreserveUserIntent,
            ConstitutionalRule::MaxModificationSize(0.5),
            ConstitutionalRule::RequiredHumanApprovalFor("skill_deletion".to_string()),
            ConstitutionalRule::RequiredHumanApprovalFor("reward_function_change".to_string()),
        ];
        Self::new(rules, config)
    }

    pub fn rules(&self) -> &[ConstitutionalRule] {
        &self.rules
    }

    pub fn config(&self) -> &ConstitutionConfig {
        &self.config
    }

    pub fn validate_evolution(
        &self,
        modification: &crate::skill::SkillModification,
    ) -> Result<(), Vec<ConstitutionViolation>> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut violations = Vec::new();

        for rule in &self.rules {
            match rule {
                ConstitutionalRule::MaxModificationSize(limit) => {
                    if let Some(ref old_content) = modification.old_content {
                        let old_len = old_content.len() as f64;
                        if old_len > 0.0 {
                            let new_len = modification.new_content.len() as f64;
                            let ratio = new_len / old_len;
                            let max_ratio = 1.0 + limit;
                            if ratio > max_ratio {
                                violations.push(ConstitutionViolation::new(
                                    rule.name().to_string(),
                                    format!(
                                        "Modification size ratio {:.2} exceeds maximum {:.2}",
                                        ratio, max_ratio
                                    ),
                                    ViolationSeverity::Critical,
                                ));
                            }
                        }
                    }
                },
                ConstitutionalRule::PreserveUserIntent => {
                    if modification.confidence < 0.3 {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            format!(
                                "Low confidence ({:.2}) modification may not preserve user intent",
                                modification.confidence
                            ),
                            ViolationSeverity::Warning,
                        ));
                    }
                },
                ConstitutionalRule::RequiredHumanApprovalFor(action) => {
                    let mod_type_str =
                        format!("{:?}", modification.modification_type).to_lowercase();
                    if mod_type_str.contains(action)
                        || modification.reason.to_lowercase().contains(action)
                    {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            format!("Modification requires human approval for: {}", action),
                            ViolationSeverity::Critical,
                        ));
                    }
                },
                ConstitutionalRule::NoSelfModificationOfReward => {
                    if modification.reason.to_lowercase().contains("reward")
                        || modification
                            .new_content
                            .to_lowercase()
                            .contains("reward_function")
                    {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            "Modification appears to alter reward-related logic".to_string(),
                            ViolationSeverity::Fatal,
                        ));
                    }
                },
                ConstitutionalRule::Custom { check_fn_name, .. } => {
                    if let Some(ref registry) = self.custom_registry {
                        if let Some(checker) = registry.get(check_fn_name) {
                            if let Some(violation) = checker.check_skill_modification(modification)
                            {
                                violations.push(violation);
                            }
                        } else {
                            violations.push(ConstitutionViolation::new(
                                rule.name().to_string(),
                                format!(
                                    "Custom rule checker '{}' not found in registry",
                                    check_fn_name
                                ),
                                ViolationSeverity::Warning,
                            ));
                        }
                    } else {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            "No custom rule registry configured; custom rule cannot be enforced"
                                .to_string(),
                            ViolationSeverity::Warning,
                        ));
                    }
                },
                ConstitutionalRule::NoCodeExecutionWithoutSandbox => {},
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn validate_tool_creation(
        &self,
        name: &str,
        code: &str,
        description: &str,
    ) -> Result<(), Vec<ConstitutionViolation>> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut violations = Vec::new();

        for rule in &self.rules {
            match rule {
                ConstitutionalRule::NoCodeExecutionWithoutSandbox => {
                    let unsandboxed_indicators = [
                        "std::process::Command",
                        "exec(",
                        "system(",
                        "subprocess",
                        "os.exec",
                        "child_process",
                    ];
                    for indicator in &unsandboxed_indicators {
                        if code.contains(indicator) {
                            violations.push(ConstitutionViolation::new(
                                rule.name().to_string(),
                                format!(
                                    "Generated tool '{}' contains unsandboxed execution indicator: {}",
                                    name, indicator
                                ),
                                ViolationSeverity::Fatal,
                            ));
                            break;
                        }
                    }
                },
                ConstitutionalRule::NoSelfModificationOfReward => {
                    let reward_indicators = [
                        "reward_function",
                        "modify_reward",
                        "set_reward",
                        "hack_reward",
                    ];
                    for indicator in &reward_indicators {
                        if code.contains(indicator) || description.contains(indicator) {
                            violations.push(ConstitutionViolation::new(
                                rule.name().to_string(),
                                format!(
                                    "Generated tool '{}' may attempt to modify reward system",
                                    name
                                ),
                                ViolationSeverity::Fatal,
                            ));
                            break;
                        }
                    }
                },
                ConstitutionalRule::PreserveUserIntent => {
                    let manipulative_patterns = [
                        "bypass",
                        "override_safety",
                        "ignore_constraint",
                        "skip_validation",
                    ];
                    for pattern in &manipulative_patterns {
                        if code.contains(pattern) || description.contains(pattern) {
                            violations.push(ConstitutionViolation::new(
                                rule.name().to_string(),
                                format!(
                                    "Generated tool '{}' contains potentially manipulative pattern: {}",
                                    name, pattern
                                ),
                                ViolationSeverity::Critical,
                            ));
                            break;
                        }
                    }
                },
                ConstitutionalRule::RequiredHumanApprovalFor(action) => {
                    if name.contains(action) || description.contains(action) {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            format!(
                                "Generated tool '{}' requires human approval for: {}",
                                name, action
                            ),
                            ViolationSeverity::Warning,
                        ));
                    }
                },
                ConstitutionalRule::Custom { check_fn_name, .. } => {
                    if let Some(ref registry) = self.custom_registry {
                        if let Some(checker) = registry.get(check_fn_name) {
                            if let Some(violation) =
                                checker.check_tool_creation(name, code, description)
                            {
                                violations.push(violation);
                            }
                        } else {
                            violations.push(ConstitutionViolation::new(
                                rule.name().to_string(),
                                format!(
                                    "Custom rule checker '{}' not found in registry",
                                    check_fn_name
                                ),
                                ViolationSeverity::Warning,
                            ));
                        }
                    } else {
                        violations.push(ConstitutionViolation::new(
                            rule.name().to_string(),
                            "No custom rule registry configured; custom rule cannot be enforced"
                                .to_string(),
                            ViolationSeverity::Warning,
                        ));
                    }
                },
                ConstitutionalRule::MaxModificationSize(_) => {},
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn check_reward_hacking(
        &self,
        reward_history: &[f64],
    ) -> Result<(), ConstitutionViolation> {
        if !self.config.enabled {
            return Ok(());
        }

        if reward_history.len() < 5 {
            return Ok(());
        }

        let has_rule = self
            .rules
            .iter()
            .any(|r| matches!(r, ConstitutionalRule::NoSelfModificationOfReward));

        if !has_rule {
            return Ok(());
        }

        let len = reward_history.len();
        let recent_window = &reward_history[len.saturating_sub(5)..];
        let recent_mean = recent_window.iter().sum::<f64>() / recent_window.len() as f64;

        let older_end = len.saturating_sub(5);
        if older_end < 5 {
            return Ok(());
        }
        let older_window = &reward_history[older_end.saturating_sub(5)..older_end];
        let older_mean = older_window.iter().sum::<f64>() / older_window.len() as f64;

        let sudden_spike = recent_mean > older_mean * 2.0 && recent_mean > 0.8;

        let all_near_max = recent_window.iter().all(|&r| r > 0.95);

        let variance: f64 = {
            let mean = recent_window.iter().sum::<f64>() / recent_window.len() as f64;
            recent_window
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / recent_window.len() as f64
        };
        let suspiciously_low_variance = variance < 1e-6 && recent_mean > 0.9;

        if sudden_spike || all_near_max || suspiciously_low_variance {
            Err(ConstitutionViolation::new(
                "no_self_modification_of_reward".to_string(),
                format!(
                    "Suspicious reward pattern detected: recent_mean={:.3}, older_mean={:.3}, variance={:.6}",
                    recent_mean, older_mean, variance
                ),
                ViolationSeverity::Fatal,
            ))
        } else {
            Ok(())
        }
    }

    pub fn log_violation(&mut self, violation: ConstitutionViolation) {
        if self.config.log_violations {
            self.violation_log.push(violation);
        }
    }

    pub fn get_violation_log(&self) -> &[ConstitutionViolation] {
        &self.violation_log
    }

    pub fn has_fatal_violations(&self) -> bool {
        self.violation_log
            .iter()
            .any(|v| v.severity == ViolationSeverity::Fatal)
    }

    pub fn has_critical_violations(&self) -> bool {
        self.violation_log
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical)
    }

    pub fn clear_violation_log(&mut self) {
        self.violation_log.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{ModificationType, SkillModification};

    fn default_constitution() -> ImmutableConstitution {
        ImmutableConstitution::with_default_rules(ConstitutionConfig::default())
    }

    #[test]
    fn test_constitutional_rule_names() {
        assert_eq!(
            ConstitutionalRule::NoSelfModificationOfReward.name(),
            "no_self_modification_of_reward"
        );
        assert_eq!(
            ConstitutionalRule::NoCodeExecutionWithoutSandbox.name(),
            "no_code_execution_without_sandbox"
        );
        assert_eq!(ConstitutionalRule::PreserveUserIntent.name(), "preserve_user_intent");
        assert_eq!(ConstitutionalRule::MaxModificationSize(0.5).name(), "max_modification_size");
        assert_eq!(
            ConstitutionalRule::RequiredHumanApprovalFor("test".to_string()).name(),
            "required_human_approval_for"
        );
        assert_eq!(
            ConstitutionalRule::Custom {
                name: "custom_rule".to_string(),
                description: "desc".to_string(),
                check_fn_name: "check".to_string(),
            }
            .name(),
            "custom_rule"
        );
    }

    #[test]
    fn test_constitutional_rule_descriptions() {
        let desc = ConstitutionalRule::NoSelfModificationOfReward.description();
        assert!(desc.contains("reward"));
    }

    #[test]
    fn test_violation_severity_as_str() {
        assert_eq!(ViolationSeverity::Warning.as_str(), "warning");
        assert_eq!(ViolationSeverity::Critical.as_str(), "critical");
        assert_eq!(ViolationSeverity::Fatal.as_str(), "fatal");
    }

    #[test]
    fn test_violation_creation() {
        let v = ConstitutionViolation::new(
            "test_rule".to_string(),
            "test description".to_string(),
            ViolationSeverity::Warning,
        );
        assert_eq!(v.rule_name, "test_rule");
        assert_eq!(v.severity, ViolationSeverity::Warning);
        assert!(!v.id.is_empty());
    }

    #[test]
    fn test_constitution_default_rules() {
        let constitution = default_constitution();
        assert_eq!(constitution.rules().len(), 6);
    }

    #[test]
    fn test_constitution_immutability() {
        let constitution = default_constitution();
        let rules = constitution.rules();
        assert_eq!(rules.len(), 6);
    }

    #[test]
    fn test_validate_evolution_within_size_limit() {
        let constitution = default_constitution();
        let modification = SkillModification {
            modification_type: ModificationType::ContentPatch,
            old_content: Some("this is some existing content that is reasonably long".to_string()),
            new_content: "this is some updated content that is reasonably long".to_string(),
            reason: "improve clarity".to_string(),
            confidence: 0.8,
            validation_result: None,
        };
        assert!(constitution.validate_evolution(&modification).is_ok());
    }

    #[test]
    fn test_validate_evolution_exceeds_size_limit() {
        let constitution = default_constitution();
        let modification = SkillModification {
            modification_type: ModificationType::ContentPatch,
            old_content: Some("short".to_string()),
            new_content: "a".repeat(100),
            reason: "expand content".to_string(),
            confidence: 0.8,
            validation_result: None,
        };
        let result = constitution.validate_evolution(&modification);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(
            violations
                .iter()
                .any(|v| v.rule_name == "max_modification_size")
        );
    }

    #[test]
    fn test_validate_evolution_reward_modification() {
        let constitution = default_constitution();
        let modification = SkillModification {
            modification_type: ModificationType::LogicRevision,
            old_content: Some("old".to_string()),
            new_content: "modify reward_function to increase scores".to_string(),
            reason: "optimize reward".to_string(),
            confidence: 0.8,
            validation_result: None,
        };
        let result = constitution.validate_evolution(&modification);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(
            violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Fatal)
        );
    }

    #[test]
    fn test_validate_evolution_low_confidence() {
        let constitution = default_constitution();
        let modification = SkillModification {
            modification_type: ModificationType::ContentPatch,
            old_content: Some("content".to_string()),
            new_content: "updated content".to_string(),
            reason: "improve".to_string(),
            confidence: 0.1,
            validation_result: None,
        };
        let result = constitution.validate_evolution(&modification);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(
            violations
                .iter()
                .any(|v| v.rule_name == "preserve_user_intent")
        );
    }

    #[test]
    fn test_validate_tool_creation_safe() {
        let constitution = default_constitution();
        let result = constitution.validate_tool_creation(
            "file_reader",
            "fn read(path: &str) -> String { ... }",
            "Reads a file from disk",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tool_creation_unsandboxed_execution() {
        let constitution = default_constitution();
        let result = constitution.validate_tool_creation(
            "shell_runner",
            "std::process::Command::new(\"sh\")",
            "Runs shell commands",
        );
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(
            violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Fatal)
        );
    }

    #[test]
    fn test_validate_tool_creation_reward_hack() {
        let constitution = default_constitution();
        let result = constitution.validate_tool_creation(
            "reward_helper",
            "fn modify_reward(val: f64) { ... }",
            "Adjusts reward values",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tool_creation_bypass_pattern() {
        let constitution = default_constitution();
        let result = constitution.validate_tool_creation(
            "quick_access",
            "fn bypass_safety() { }",
            "Bypass safety checks",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_check_reward_hacking_normal() {
        let constitution = default_constitution();
        let rewards: Vec<f64> = vec![0.3, 0.4, 0.5, 0.6, 0.5, 0.6, 0.7, 0.6, 0.7, 0.8];
        assert!(constitution.check_reward_hacking(&rewards).is_ok());
    }

    #[test]
    fn test_check_reward_hacking_sudden_spike() {
        let constitution = default_constitution();
        let rewards: Vec<f64> = vec![0.1, 0.1, 0.1, 0.1, 0.1, 0.99, 0.99, 0.99, 0.99, 0.99];
        let result = constitution.check_reward_hacking(&rewards);
        assert!(result.is_err());
        let violation = result.unwrap_err();
        assert_eq!(violation.severity, ViolationSeverity::Fatal);
    }

    #[test]
    fn test_check_reward_hacking_insufficient_data() {
        let constitution = default_constitution();
        let rewards: Vec<f64> = vec![0.5, 0.5, 0.5];
        assert!(constitution.check_reward_hacking(&rewards).is_ok());
    }

    #[test]
    fn test_check_reward_hacking_suspiciously_low_variance() {
        let constitution = default_constitution();
        let rewards: Vec<f64> = vec![
            0.1, 0.1, 0.1, 0.1, 0.1, 0.999999, 0.999999, 0.999999, 0.999999, 0.999999,
        ];
        assert!(constitution.check_reward_hacking(&rewards).is_err());
    }

    #[test]
    fn test_log_violation() {
        let mut constitution = default_constitution();
        let violation = ConstitutionViolation::new(
            "test_rule".to_string(),
            "test".to_string(),
            ViolationSeverity::Warning,
        );
        constitution.log_violation(violation);
        assert_eq!(constitution.get_violation_log().len(), 1);
    }

    #[test]
    fn test_log_violation_disabled() {
        let mut constitution = ImmutableConstitution::new(
            vec![ConstitutionalRule::NoSelfModificationOfReward],
            ConstitutionConfig {
                enabled: true,
                auto_revert_on_critical: true,
                log_violations: false,
            },
        );
        let violation = ConstitutionViolation::new(
            "test_rule".to_string(),
            "test".to_string(),
            ViolationSeverity::Warning,
        );
        constitution.log_violation(violation);
        assert!(constitution.get_violation_log().is_empty());
    }

    #[test]
    fn test_constitution_disabled() {
        let config = ConstitutionConfig {
            enabled: false,
            auto_revert_on_critical: false,
            log_violations: true,
        };
        let constitution = ImmutableConstitution::new(
            vec![ConstitutionalRule::NoSelfModificationOfReward],
            config,
        );
        let modification = SkillModification {
            modification_type: ModificationType::LogicRevision,
            old_content: Some("old".to_string()),
            new_content: "modify reward_function".to_string(),
            reason: "hack".to_string(),
            confidence: 0.1,
            validation_result: None,
        };
        assert!(constitution.validate_evolution(&modification).is_ok());
    }

    #[test]
    fn test_has_fatal_violations() {
        let mut constitution = default_constitution();
        assert!(!constitution.has_fatal_violations());
        constitution.log_violation(ConstitutionViolation::new(
            "rule".to_string(),
            "desc".to_string(),
            ViolationSeverity::Fatal,
        ));
        assert!(constitution.has_fatal_violations());
    }

    #[test]
    fn test_has_critical_violations() {
        let mut constitution = default_constitution();
        assert!(!constitution.has_critical_violations());
        constitution.log_violation(ConstitutionViolation::new(
            "rule".to_string(),
            "desc".to_string(),
            ViolationSeverity::Critical,
        ));
        assert!(constitution.has_critical_violations());
    }

    #[test]
    fn test_clear_violation_log() {
        let mut constitution = default_constitution();
        constitution.log_violation(ConstitutionViolation::new(
            "rule".to_string(),
            "desc".to_string(),
            ViolationSeverity::Warning,
        ));
        assert_eq!(constitution.get_violation_log().len(), 1);
        constitution.clear_violation_log();
        assert!(constitution.get_violation_log().is_empty());
    }
}
