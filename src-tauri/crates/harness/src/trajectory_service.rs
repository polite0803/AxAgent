// SPDX-License-Identifier: AGPL-3.0-only

//! 轨迹学习服务契约。
//!
//! 提供会话压缩完整性校验和复杂度评估功能。
//! 实现方（`axagent-trajectory`）管理轨迹数据的处理和优化。

use std::fmt;

/// 完整性检查结果
#[derive(Debug, Clone)]
pub struct IntegrityResult {
    pub is_valid: bool,
    pub checks: Vec<IntegrityCheck>,
}

/// 单项完整性检查结果
#[derive(Debug, Clone)]
pub struct IntegrityCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// 轨迹学习服务契约
pub trait TrajectoryService: fmt::Debug + Send + Sync {
    /// 从消息中提取关键实体（用于完整性校验）
    fn extract_entities(&self, messages: &[serde_json::Value]) -> Vec<String>;

    /// 校验压缩完整性
    fn verify_compression_integrity(
        &self,
        original: &[serde_json::Value],
        compressed: &[serde_json::Value],
        key_entities: &[String],
    ) -> IntegrityResult;

    /// 评估输入复杂度
    fn estimate_complexity(&self, input: &str) -> TaskComplexity;
}

/// 任务复杂度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

impl TaskComplexity {
    pub fn default_max_iterations(&self) -> usize {
        match self {
            Self::Low => 20,
            Self::Medium => 50,
            Self::High => 100,
        }
    }
}

/// 空实现 — 提供默认降级行为
#[derive(Debug)]
pub struct NoopTrajectoryService;

impl TrajectoryService for NoopTrajectoryService {
    fn extract_entities(&self, _messages: &[serde_json::Value]) -> Vec<String> {
        Vec::new()
    }

    fn verify_compression_integrity(
        &self,
        _original: &[serde_json::Value],
        _compressed: &[serde_json::Value],
        _key_entities: &[String],
    ) -> IntegrityResult {
        IntegrityResult {
            is_valid: true,
            checks: Vec::new(),
        }
    }

    fn estimate_complexity(&self, _input: &str) -> TaskComplexity {
        TaskComplexity::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_returns_defaults() {
        let svc = NoopTrajectoryService;
        assert!(svc.extract_entities(&[]).is_empty());
        assert!(svc.verify_compression_integrity(&[], &[], &[]).is_valid);
        assert_eq!(svc.estimate_complexity("test"), TaskComplexity::Medium);
    }

    #[test]
    fn complexity_iterations() {
        assert_eq!(TaskComplexity::Low.default_max_iterations(), 20);
        assert_eq!(TaskComplexity::Medium.default_max_iterations(), 50);
        assert_eq!(TaskComplexity::High.default_max_iterations(), 100);
    }
}
