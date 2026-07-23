// SPDX-License-Identifier: AGPL-3.0-only

//! 分歧日志审计表（V55 P0-4 新增）
//!
//! 记录系统内部不同 Agent/因子对同一决策的分歧情况。
//! 一旦写入即为不可变审计记录，Dao 层实施只读保护：CRUD 仅允许 CREATE/READ，禁止 UPDATE/DELETE。
//!
//! ## 数据结构（divergence-log-schema.json）
//! - entity: 决策涉及的实体（stock_code）
//! - dimension: 分歧维度（risk/valuation/sentiment/strategy/technical）
//! - divergence_triple: 分歧三元组 {source_a, source_b, magnitude, direction}
//! - resolution: 解决方式 {resolved_by, resolution_type}
//! - sha256 链: prev_hash + current_hash 实现只读防篡改
//!
//! ## 写入时机
//! portfolio-mgr.rhai 每次产生 R-200~R-204 / R-401~R-405 否决/降级裁决时，
//! Rust 端（core.rs）调用 write_divergence_log 写入一条记录。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "divergence_logs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 股票代码（如 000001）
    pub stock_code: String,
    /// 会话 ID（stock_analysis 的 conversation_id）
    pub session_id: String,
    /// 分歧维度: risk/valuation/sentiment/strategy/technical
    pub dimension: String,
    /// 分歧来源方 A（如 "portfolio-mgr f4_signal=-0.50"）
    pub source_a: String,
    /// 分歧来源方 B（如 "t-scoring f1_signal=+0.40"）
    pub source_b: String,
    /// 分歧幅度（0-1）
    pub magnitude: f64,
    /// 分歧方向: a_higher/b_higher/opposite_sign
    pub direction: String,
    /// 触发的规则 ID（R-200~R-405）
    pub rule_id: Option<String>,
    /// LLM 原始提案文本（被否决前）
    pub llm_proposal: Option<String>,
    /// 否决原因摘要
    pub rejection_reason: Option<String>,
    /// 解决机制: risk_downgrade/technical_veto/bearish_veto/human_override/graceful_degradation
    pub resolved_by: String,
    /// 解决类型: auto/manual/pending
    pub resolution_type: String,
    /// 上一条日志的 sha256 hash（链式防篡改）
    pub prev_hash: Option<String>,
    /// 本条日志的 sha256 hash（= sha256(id + prev_hash + stock_code + timestamp + rule_id + resolved_by)）
    pub current_hash: String,
    /// 决策时间戳（ISO 8601）
    pub decision_ts: String,
    /// 写入时间戳（ISO 8601）
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 计算 sha256 防篡改链。
///
/// hash = sha256(id || prev_hash || stock_code || decision_ts || rule_id || resolved_by)
///
/// 如果链上任何一条记录被篡改，验证 `current_hash == sha256(prev_hash || ...)` 就会失败。
pub fn compute_hash(
    id: &str,
    prev_hash: Option<&str>,
    stock_code: &str,
    decision_ts: &str,
    rule_id: Option<&str>,
    resolved_by: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"|");
    hasher.update(prev_hash.unwrap_or("genesis").as_bytes());
    hasher.update(b"|");
    hasher.update(stock_code.as_bytes());
    hasher.update(b"|");
    hasher.update(decision_ts.as_bytes());
    hasher.update(b"|");
    hasher.update(rule_id.unwrap_or("R-000").as_bytes());
    hasher.update(b"|");
    hasher.update(resolved_by.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// 校验链上全部记录的完整性。
///
/// 返回 (通过数, 失败数, 失败记录 ID 列表)。
pub fn verify_chain(logs: &[Model]) -> (usize, usize, Vec<String>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_ids = Vec::new();

    let mut expected_prev: Option<&str> = None;
    for log in logs {
        // 校验 prev_hash 指针：None 匹配 None（创世块），否则比较值
        if log.prev_hash.as_deref() != expected_prev {
            failed += 1;
            failed_ids.push(log.id.clone());
            continue;
        }
        // 校验 current_hash 自身
        let computed = compute_hash(
            &log.id,
            log.prev_hash.as_deref(),
            &log.stock_code,
            &log.decision_ts,
            log.rule_id.as_deref(),
            &log.resolved_by,
        );
        if computed == log.current_hash {
            passed += 1;
        } else {
            failed += 1;
            failed_ids.push(log.id.clone());
        }
        expected_prev = Some(&log.current_hash);
    }
    (passed, failed, failed_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 =
            compute_hash("test-1", None, "000001", "2026-07-02", Some("R-200"), "risk_downgrade");
        let h2 =
            compute_hash("test-1", None, "000001", "2026-07-02", Some("R-200"), "risk_downgrade");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_differs_on_input() {
        let h1 =
            compute_hash("test-1", None, "000001", "2026-07-02", Some("R-200"), "risk_downgrade");
        let h2 =
            compute_hash("test-2", None, "000001", "2026-07-02", Some("R-200"), "risk_downgrade");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_empty_chain() {
        let (passed, failed, _) = verify_chain(&[]);
        assert_eq!(passed, 0);
        assert_eq!(failed, 0);
    }

    #[test]
    fn test_verify_valid_chain() {
        let h1 = compute_hash(
            "log-1",
            None,
            "000001",
            "2026-07-02T00:00:00Z",
            Some("R-200"),
            "risk_downgrade",
        );
        let h2 = compute_hash(
            "log-2",
            Some(&h1),
            "000001",
            "2026-07-02T00:00:00Z",
            Some("R-201"),
            "bearish_veto",
        );

        let logs = vec![
            Model {
                id: "log-1".into(),
                stock_code: "000001".into(),
                session_id: "s1".into(),
                dimension: "risk".into(),
                source_a: "f4".into(),
                source_b: "f1".into(),
                magnitude: 0.8,
                direction: "opposite_sign".into(),
                rule_id: Some("R-200".into()),
                llm_proposal: None,
                rejection_reason: Some("高风险否决".into()),
                resolved_by: "risk_downgrade".into(),
                resolution_type: "auto".into(),
                prev_hash: None,
                current_hash: h1.clone(),
                decision_ts: "2026-07-02T00:00:00Z".into(),
                created_at: "2026-07-02T00:00:00Z".into(),
            },
            Model {
                id: "log-2".into(),
                stock_code: "000001".into(),
                session_id: "s1".into(),
                dimension: "technical".into(),
                source_a: "rsi".into(),
                source_b: "llm".into(),
                magnitude: 0.5,
                direction: "a_higher".into(),
                rule_id: Some("R-201".into()),
                llm_proposal: None,
                rejection_reason: Some("空头预测".into()),
                resolved_by: "bearish_veto".into(),
                resolution_type: "auto".into(),
                prev_hash: Some(h1),
                current_hash: h2.clone(),
                decision_ts: "2026-07-02T00:00:00Z".into(),
                created_at: "2026-07-02T00:00:00Z".into(),
            },
        ];

        let (passed, failed, ids) = verify_chain(&logs);
        assert_eq!(passed, 2);
        assert_eq!(failed, 0);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_verify_tampered_chain() {
        let h1 = compute_hash(
            "log-1",
            None,
            "000001",
            "2026-07-02T00:00:00Z",
            Some("R-200"),
            "risk_downgrade",
        );
        // h2 应该是 log-2 的正确 hash，但这里我们用 log-1 的 hash 替代（篡改）
        let h2_wrong = h1.clone();

        let logs = vec![
            Model {
                id: "log-1".into(),
                stock_code: "000001".into(),
                session_id: "s1".into(),
                dimension: "risk".into(),
                source_a: "f4".into(),
                source_b: "f1".into(),
                magnitude: 0.8,
                direction: "opposite_sign".into(),
                rule_id: Some("R-200".into()),
                llm_proposal: None,
                rejection_reason: Some("高风险否决".into()),
                resolved_by: "risk_downgrade".into(),
                resolution_type: "auto".into(),
                prev_hash: None,
                current_hash: h1,
                decision_ts: "2026-07-02T00:00:00Z".into(),
                created_at: "2026-07-02T00:00:00Z".into(),
            },
            Model {
                id: "log-2".into(),
                stock_code: "000001".into(),
                session_id: "s1".into(),
                dimension: "technical".into(),
                source_a: "rsi".into(),
                source_b: "llm".into(),
                magnitude: 0.5,
                direction: "a_higher".into(),
                rule_id: Some("R-201".into()),
                llm_proposal: None,
                rejection_reason: Some("空头预测".into()),
                resolved_by: "bearish_veto".into(),
                resolution_type: "auto".into(),
                prev_hash: None, // 篡改：应该是 Some(h1) 但写成 None
                current_hash: h2_wrong,
                decision_ts: "2026-07-02T00:00:00Z".into(),
                created_at: "2026-07-02T00:00:00Z".into(),
            },
        ];

        let (_passed, failed, _) = verify_chain(&logs);
        assert!(failed > 0);
    }
}
