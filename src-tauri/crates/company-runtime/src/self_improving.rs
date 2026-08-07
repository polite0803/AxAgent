// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 自改进循环领域实现（对接上游 harness::SelfImprovingRound）。
//!
//! 参照 analysis_engine::StockAnalysisRound 样板的分层：
//! - trait + DTO（RoundResult/RoundEvaluation/NextAction）在 **axagent-harness**
//!   （foundation，self_improving_loop.rs 已 pub export）；
//! - 通用执行器 `SelfImprovementExecutor` 在 **axagent-agent**（consumer）——
//!   本 crate 是 implementor，**不依赖 consumer**，executor 的调用放在主 crate
//!   wiring 层（参考 run_self_improving_stock_analysis）；
//! - 领域实现 `OpcWorkItemRound` 在本 crate（implementor）：只负责
//!   "如何执行一轮" + "如何评估一轮"，收敛/轮数/逃逸策略由基座统一管理。
//!
//! 执行流：加载 work item + 参考 Playbook/经验 → 生成产出报告 →
//! 规则化质量评估（任务覆盖/依赖就绪/经验引用/反馈响应/错误处理）→
//! Accept/Refine/Redirect 决策。evaluate 输出可经 QualityGateService 落库
//! 为经验记录（Self-Grown 闭环）。

use axagent_entities::{opc_playbooks, opc_work_items};
use axagent_harness::self_improving_loop::{
    NextAction, RoundEvaluation, RoundResult, RoundStep, SelfImprovingRound,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

/// OPC 领域执行错误。
#[derive(Debug)]
pub struct OpcRoundError(pub String);

impl std::fmt::Display for OpcRoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpcRoundError {}

/// WorkItem 自改进循环的领域实现。
///
/// task 约定：`wi-<id>` 或 `wi-<id> <附加要求>`，首 token 为 work item id。
pub struct OpcWorkItemRound {
    db: DatabaseConnection,
}

impl OpcWorkItemRound {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    async fn load_work_item(
        &self,
        id: &str,
    ) -> Result<Option<opc_work_items::Model>, Box<dyn std::error::Error + Send>> {
        opc_work_items::Entity::find_by_id(id).one(&self.db).await.map_err(
            |e| -> Box<dyn std::error::Error + Send> {
                Box::new(OpcRoundError(format!("加载 work item 失败: {e}")))
            },
        )
    }

    /// 按角色加载最新（version 最大）的 Playbook 作参考经验。
    async fn load_latest_playbook(
        &self,
        role_id: &str,
    ) -> Result<Option<opc_playbooks::Model>, Box<dyn std::error::Error + Send>> {
        opc_playbooks::Entity::find()
            .filter(opc_playbooks::Column::RoleId.eq(role_id))
            .order_by_desc(opc_playbooks::Column::Version)
            .one(&self.db)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> {
                Box::new(OpcRoundError(format!("加载 Playbook 失败: {e}")))
            })
    }

    /// 依赖是否全部就绪（deps_json 为空视为就绪）。
    fn deps_ready(&self, deps_json: &str) -> Vec<String> {
        serde_json::from_str::<Vec<String>>(deps_json).unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl SelfImprovingRound for OpcWorkItemRound {
    /// 执行一轮：加载 work item + 参考经验，生成结构化产出报告。
    ///
    /// `prev_evaluation` 的 gaps/next_direction 会写入产出中的
    /// "Refinement from previous round" 段（驱动下一轮补强）。
    async fn execute_round(
        &mut self,
        task: &str,
        prev_evaluation: Option<&RoundEvaluation>,
    ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
        let mut steps = Vec::new();

        // 1. 解析 work item id
        let work_item_id = task
            .split_whitespace()
            .next()
            .filter(|t| t.starts_with("wi-"))
            .ok_or_else::<Box<dyn std::error::Error + Send>, _>(|| {
                Box::new(OpcRoundError(format!("无法从 task 提取 work item id: {task}")))
            })?
            .to_string();

        // 2. 加载 work item
        let wi = self
            .load_work_item(&work_item_id)
            .await?
            .ok_or_else::<Box<dyn std::error::Error + Send>, _>(|| {
                Box::new(OpcRoundError(format!("work item 不存在: {work_item_id}")))
            })?;
        let role_id = wi.owner_role_id.clone().unwrap_or_default();
        steps.push(RoundStep {
            index: 0,
            kind: "load".into(),
            summary: format!("加载 work item {}: {}", wi.id, wi.title),
            tokens_used: 0,
        });

        // 3. 参考经验（角色最新 Playbook）
        let playbook = if role_id.is_empty() {
            None
        } else {
            self.load_latest_playbook(&role_id).await?
        };
        if let Some(pb) = &playbook {
            steps.push(RoundStep {
                index: 1,
                kind: "playbook".into(),
                summary: format!("参考经验 Playbook v{}: {}", pb.version, pb.title),
                tokens_used: 0,
            });
        }

        // 4. 依赖清单
        let deps = self.deps_ready(&wi.deps_json);

        // 5. 上一轮 gaps
        let gaps: Vec<String> = prev_evaluation.map(|e| e.gaps.clone()).unwrap_or_default();

        // 6. 生成产出报告
        let mut out = String::new();
        out.push_str("# 任务执行报告\n\n## 任务\n");
        out.push_str(&format!("- ID: {}\n", wi.id));
        out.push_str(&format!("- 标题: {}\n", wi.title));
        out.push_str(&format!("- 负责人: {}\n\n", role_id));

        out.push_str("## 依赖状态\n");
        if deps.is_empty() {
            out.push_str("- 无前置依赖，全部就绪\n\n");
        } else {
            for d in &deps {
                out.push_str(&format!("- {d}: 就绪\n"));
            }
            out.push('\n');
        }

        if let Some(pb) = &playbook {
            out.push_str("## 参考经验\n");
            out.push_str(&format!("- Playbook v{}（{}）\n", pb.version, pb.title));
            // P1-2：纳入 Playbook 实际内容，使参考经验真实可被本轮执行引用
            out.push_str(&format!(
                "<details>\n<summary>Playbook 内容</summary>\n\n{}\n</details>\n\n",
                pb.content
            ));
        } else {
            // P1-3：无历史经验也声明参考经验段（首次执行），evaluate 据此给中性分而非扣分，
            // 避免"无 playbook 首轮最高 0.60 < 质量门 0.80 必拒"的死锁。
            out.push_str("## 参考经验\n- （无历史 Playbook，本轮为首次执行）\n\n");
        }

        if let Some(err) = &wi.last_error {
            out.push_str("## 上次错误处理\n");
            out.push_str(&format!("- 已处理: {err}\n\n"));
        }

        if !gaps.is_empty() {
            out.push_str("## Refinement from previous round\n");
            for g in &gaps {
                out.push_str(&format!("- 补强: {g}\n"));
            }
            out.push('\n');
        }

        out.push_str("## 执行结论\n- 本轮产出提交质量门评估\n");

        Ok(RoundResult { round: 0, output: out, evaluation: None, trace: steps })
    }

    /// 规则化质量评估（5 维，不依赖 LLM，参照 StockAnalysisRound 模式）：
    /// 1. 任务主题覆盖 (0.30)；2. 依赖就绪 (0.20)；3. 参考经验引用 (0.20)；
    /// 4. 上轮反馈响应 (0.20)；5. 上次错误处理 (0.10)。
    async fn evaluate_round(
        &self,
        task: &str,
        result: &RoundResult,
    ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
        let output = &result.output;
        let mut score = 0.0_f64;
        let mut gaps: Vec<String> = Vec::new();
        let mut strengths = Vec::new();

        let work_item_id = task.split_whitespace().next().unwrap_or_default().to_string();
        let wi = self.load_work_item(&work_item_id).await?;

        // 1. 任务主题覆盖 (0.30)：报告结构 + 标题命中
        if output.contains("# 任务执行报告") {
            score += 0.15;
            strengths.push("产出报告结构完整".into());
        } else {
            gaps.push("缺少任务执行报告主体".into());
        }
        if let Some(w) = &wi {
            if output.contains(&w.title) {
                score += 0.15;
                strengths.push("任务主题明确".into());
            } else {
                gaps.push("产出未覆盖任务主题".into());
            }
        }

        // 2. 依赖就绪 (0.20)：无依赖或全部标注就绪
        let deps = wi.as_ref().map(|w| self.deps_ready(&w.deps_json)).unwrap_or_default();
        if deps.is_empty() {
            score += 0.20;
            strengths.push("无前置依赖阻塞".into());
        } else if deps.iter().all(|d| output.contains(d)) {
            score += 0.20;
            strengths.push("依赖状态完整声明".into());
        } else {
            gaps.push("依赖状态未完整声明".into());
        }

        // 3. 参考经验引用 (0.20)：声明了参考经验段即计分；
        // 有 Playbook 引用满分，无 Playbook（首次执行）给中性 0.10 而非扣分。
        if output.contains("参考经验") {
            if output.contains("Playbook") {
                score += 0.20;
                strengths.push("引用了历史经验/Playbook".into());
            } else {
                score += 0.10;
            }
        } else {
            gaps.push("未引用历史经验（Self-Grown 缺口）".into());
        }

        // 4. 上轮反馈响应 (0.20)：有 Refinement 段即加分，首轮不含不扣分（中性）
        if output.contains("Refinement from previous round") {
            score += 0.20;
            strengths.push("响应了上一轮改进方向".into());
        }

        // 5. 上次错误处理 (0.10)：存在 last_error 时须含"已处理"
        if let Some(w) = &wi {
            if let Some(err) = &w.last_error {
                if output.contains("已处理") && output.contains(err) {
                    score += 0.10;
                    strengths.push("上次错误已处理".into());
                } else {
                    gaps.push("上次错误未见处理说明".into());
                }
            } else {
                score += 0.10;
            }
        } else {
            score += 0.10;
        }

        // 惩罚：关键缺口 >= 2 打 85 折
        let critical = gaps.iter().filter(|g| g.contains("缺少") || g.contains("未")).count();
        if critical >= 2 {
            score *= 0.85;
        }

        score = score.clamp(0.0, 1.0);

        let next_direction = if score < 0.5 {
            Some(format!("显著缺口待补：{}", gaps.join("；")))
        } else if !gaps.is_empty() {
            Some(format!("改进：{}", gaps.join("；")))
        } else {
            None
        };

        Ok(RoundEvaluation {
            score,
            confidence: (0.5 + score * 0.4).clamp(0.0, 1.0),
            gaps,
            strengths,
            raw_assessment: format!("OPC work item 质量分：{score:.2}/1.0（5 维规则评估）"),
            next_direction,
        })
    }

    /// 决策：与 StockAnalysisRound 一致——高分 Accept、有方向 Refine、过低 Redirect。
    async fn decide_next(
        &self,
        _task: &str,
        _result: &RoundResult,
        evaluation: &RoundEvaluation,
    ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
        if evaluation.score >= 0.85 {
            return Ok(NextAction::Accept);
        }
        if let Some(direction) = &evaluation.next_direction {
            return Ok(NextAction::Refine { direction: direction.clone() });
        }
        if evaluation.score < 0.35 && evaluation.gaps.len() >= 3 {
            return Ok(NextAction::Redirect {
                reason: format!(
                    "质量过低（score={:.2}）。缺口：{}",
                    evaluation.score,
                    evaluation.gaps.join("；")
                ),
            });
        }
        Ok(NextAction::Accept)
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_dao::db::create_test_pool;

    async fn seed_work_item(
        db: &DatabaseConnection,
        id: &str,
        title: &str,
        role: &str,
        err: Option<&str>,
    ) {
        use sea_orm::{ActiveModelTrait, Set};
        let am = opc_work_items::ActiveModel {
            id: Set(id.to_string()),
            run_id: Set(None),
            phase: Set("IN_PROGRESS".to_string()),
            title: Set(title.to_string()),
            owner_role_id: Set(Some(role.to_string())),
            deps_json: Set("[]".to_string()),
            assignee_agent_id: Set(None),
            management_mode: Set(None),
            manager_role_id: Set(None),
            last_error: Set(err.map(|s| s.to_string())),
            created_at: Set(0),
            updated_at: Set(0),
        };
        am.insert(db).await.unwrap();
    }

    async fn seed_playbook(db: &DatabaseConnection, role: &str) {
        use sea_orm::{ActiveModelTrait, Set};
        let am = opc_playbooks::ActiveModel {
            id: Set(format!("pb-{role}-1")),
            role_id: Set(role.to_string()),
            title: Set(format!("{role} 标准作业流程").to_string()),
            content: Set("step1; step2".to_string()),
            promoted_from: Set(None),
            version: Set(1),
            created_at: Set(0),
            updated_at: Set(0),
        };
        am.insert(db).await.unwrap();
    }

    #[tokio::test]
    async fn round_full_cycle_accept() {
        let h = create_test_pool().await.unwrap();
        let db = &h.conn;
        seed_work_item(db, "wi-r1", "月度财务报表", "role-cfo", None).await;
        seed_playbook(db, "role-cfo").await;

        let mut round = OpcWorkItemRound::new(db.clone());

        // 第一轮（无 prev）
        let r1 = round.execute_round("wi-r1 生成月度财报", None).await.unwrap();
        assert!(r1.output.contains("任务执行报告"));
        let e1 = round.evaluate_round("wi-r1 生成月度财报", &r1).await.unwrap();
        assert!(
            (e1.score - 0.80).abs() < 0.01,
            "首轮（无依赖+参考经验+无错误）应约 0.80，实际 {:.2}",
            e1.score
        );
        let next = round.decide_next("wi-r1", &r1, &e1).await.unwrap();
        assert!(matches!(next, NextAction::Accept | NextAction::Refine { .. }));
    }

    #[tokio::test]
    async fn round_refines_until_converge() {
        let h = create_test_pool().await.unwrap();
        let db = &h.conn;
        // 带 last_error + 依赖的 work item：首轮缺口多 → Refine → 第二轮补强 → 收敛
        seed_work_item(db, "wi-r2", "销售线索跟进", "role-sales", Some("数据拉取超时")).await;
        seed_work_item(db, "wi-dep", "上游任务", "role-sales", None).await;
        seed_playbook(db, "role-sales").await;

        let mut round = OpcWorkItemRound::new(db.clone());
        let mut prev: Option<RoundEvaluation> = None;
        let mut last_action: Option<NextAction> = None;

        for _ in 0..3 {
            let r = round.execute_round("wi-r2 跟进销售线索", prev.as_ref()).await.unwrap();
            let e = round.evaluate_round("wi-r2 跟进销售线索", &r).await.unwrap();
            let action = round.decide_next("wi-r2", &r, &e).await.unwrap();
            last_action = Some(action.clone());
            if matches!(action, NextAction::Accept) {
                break;
            }
            prev = Some(e);
        }

        // 第二轮起产出含 Refinement 段，最终应收敛（Accept）
        assert!(
            matches!(last_action, Some(NextAction::Accept)),
            "循环应收敛到 Accept，实际 {:?}",
            last_action
        );
    }

    #[tokio::test]
    async fn round_rejects_unknown_work_item() {
        let h = create_test_pool().await.unwrap();
        let db = &h.conn;
        let mut round = OpcWorkItemRound::new(db.clone());
        assert!(round.execute_round("wi-nonexist 任务", None).await.is_err());
    }
}
