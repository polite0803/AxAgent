// SPDX-License-Identifier: AGPL-3.0-only

//! 代码验收引擎契约 — 定义代码级 diff 验证和质量检查的接口
//!
//! 本模块为 OPC 行业工作流提供独立的代码验收能力：
//! - CodeVerifierPort: 代码验证端口 trait，由 wiring 层实现
//! - CodeVerificationResult: 验证结果 DTO
//! - CodeChange / DiffHunk: 代码变更描述

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 验证严重级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationSeverity {
    /// 阻断性问题（必须修复）
    Blocking,
    /// 警告（建议修复）
    Warning,
    /// 信息（可选改进）
    Info,
}

impl VerificationSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Blocking => "blocking",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// Diff hunk 描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// hunk 标题
    pub title: String,
    /// 新增行数
    pub added: u32,
    /// 删除行数
    pub removed: u32,
    /// 变更内容（简化版）
    pub content: String,
}

/// 代码变更描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// 文件路径
    pub file_path: String,
    /// 变更类型（added / modified / deleted）
    pub change_type: String,
    /// 新增行数
    pub lines_added: u32,
    /// 删除行数
    pub lines_removed: u32,
    /// Diff hunk 列表
    pub hunks: Vec<DiffHunk>,
}

impl CodeChange {
    /// 计算净变更行数
    pub fn net_lines(&self) -> i64 {
        self.lines_added as i64 - self.lines_removed as i64
    }

    /// 是否为重大变更（> 50 行）
    pub fn is_major_change(&self) -> bool {
        (self.lines_added + self.lines_removed) > 50
    }
}

/// 代码验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeVerificationResult {
    /// 是否通过验收
    pub passed: bool,
    /// 总体评分（0.0 - 1.0）
    pub score: f64,
    /// 验证摘要
    pub summary: String,
    /// 发现的问题列表
    pub issues: Vec<VerificationIssue>,
    /// 代码变更列表
    pub changes: Vec<CodeChange>,
    /// 建议的操作
    pub suggested_action: String,
    /// 验证时间戳
    pub verified_at: u64,
}

/// 验证问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    /// 严重级别
    pub severity: VerificationSeverity,
    /// 问题类别
    pub category: String,
    /// 问题描述
    pub description: String,
    /// 涉及的文件
    pub file_path: Option<String>,
    /// 行号
    pub line_number: Option<u32>,
    /// 修复建议
    pub suggestion: String,
}

/// 代码验证端口 trait
///
/// 由 wiring 层实现，用于在 OPC 工作流完成后进行代码级验收。
/// 支持多种验证策略：静态分析、单元测试覆盖率、代码规范检查等。
#[async_trait]
pub trait CodeVerifierPort: Send + Sync {
    /// 验证代码变更
    ///
    /// # 参数
    /// - `industry_id`: 行业标识
    /// - `workflow_id`: 工作流标识
    /// - `changes`: 代码变更列表
    ///
    /// # 返回
    /// - `CodeVerificationResult`: 验证结果
    async fn verify_changes(
        &self,
        industry_id: &str,
        workflow_id: &str,
        changes: &[CodeChange],
    ) -> Result<CodeVerificationResult, String>;

    /// 获取行业特定的验证规则
    ///
    /// 不同行业可能有不同的代码验收标准。
    async fn get_verification_rules(
        &self,
        industry_id: &str,
    ) -> Result<Vec<VerificationRule>, String>;
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 严重级别
    pub severity: VerificationSeverity,
    /// 检查模式（正则或关键字）
    pub pattern: String,
    /// 是否启用
    pub enabled: bool,
}

/// No-op 实现（用于测试或未配置验证器时）
pub struct NoopCodeVerifier;

impl NoopCodeVerifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopCodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CodeVerifierPort for NoopCodeVerifier {
    async fn verify_changes(
        &self,
        _industry_id: &str,
        _workflow_id: &str,
        changes: &[CodeChange],
    ) -> Result<CodeVerificationResult, String> {
        let total_changes = changes.len();
        let total_lines: u64 =
            changes.iter().map(|c| (c.lines_added + c.lines_removed) as u64).sum();

        Ok(CodeVerificationResult {
            passed: true,
            score: 0.8, // 默认给一个较高分，因为没有进行实际验证
            summary: format!(
                "代码验收通过（自动模式）：{} 个文件变更，共 {} 行",
                total_changes, total_lines
            ),
            issues: Vec::new(),
            changes: changes.to_vec(),
            suggested_action: "continue".to_string(),
            verified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    async fn get_verification_rules(
        &self,
        _industry_id: &str,
    ) -> Result<Vec<VerificationRule>, String> {
        Ok(Vec::new())
    }
}
