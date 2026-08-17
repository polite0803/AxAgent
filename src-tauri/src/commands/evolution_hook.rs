// SPDX-License-Identifier: AGPL-3.0-only

//! 技能侧反思钩子 wiring 层实现（自我进化通道二：能力偏弱进化改进）。
//!
//! 实现 harness `SkillEvolutionHook` 契约，由主对话 `agent_query` 经
//! `ConversationRuntimeFactoryArgs::with_skill_evolution_hook` 注入。
//! 职责（对齐 harness 契约）：
//!   1. `tool_name` → Skill 映射（技能以 `skill_{name}` 工具暴露）；
//!   2. 记录执行结果（更新 `total_usages` / `success_rate` / `consecutive_failures`）；
//!   3. `EvolutionDecider` 贝叶斯后验判定是否应进化；
//!   4. 命中后生成 `SkillEvolution` 型提议，走与认知编排器共用的用户同意通道
//!      （emit `evolution-consent-request`），同意后运行遗传算法进化并落库。
//!
//! 反思结果不阻塞工具执行主流程：耗时操作（同意等待 + 进化）在 spawn 任务内异步完成。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::runtime_types::capability_gap::{CapabilityGapProposal, CapabilityGapType};
use axagent_harness::skill_evolution_hook::SkillEvolutionHook;
use axagent_trajectory::{
    EvolutionDecision, ImmutableConstitution, Skill, SkillEvolutionEngine, Trajectory,
    TrajectoryStorage, ViolationSeverity,
};
use tauri::AppHandle;
use tokio::sync::{Mutex, oneshot};

/// wiring 层 `SkillEvolutionHook` 实现（持有运行所需字段的 Arc，分布式注入）。
#[derive(Clone)]
pub struct SkillEvolutionHookImpl {
    /// 用于 emit `evolution-consent-request` 用户同意弹窗事件
    pub app: AppHandle,
    pub trajectory_storage: Arc<TrajectoryStorage>,
    pub skill_evolution_engine: Arc<Mutex<SkillEvolutionEngine>>,
    pub evolution_consent_senders: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    pub constitution: Arc<ImmutableConstitution>,
}

#[async_trait::async_trait]
impl SkillEvolutionHook for SkillEvolutionHookImpl {
    async fn on_tool_executed(&self, tool_name: &str, success: bool, _output: &str) -> bool {
        // 1. tool → skill 映射：技能以 `skill_{name}` 工具暴露，其余工具与本通道无关
        let Some(skill_key) = tool_name.strip_prefix("skill_") else {
            return false;
        };
        let skills = match self.trajectory_storage.get_skills().await {
            Ok(s) => s,
            Err(_) => return false,
        };
        let Some(skill) = skills.into_iter().find(|s| s.name == skill_key) else {
            return false;
        };

        // 2. 记录本次执行结果（更新 total_usages / success_rate / consecutive_failures）
        let _ = self
            .trajectory_storage
            .record_skill_execution(&skill.id, None, success, 0, None, None)
            .await;

        // 3. 贝叶斯后验判定是否应进化
        let (decision, reason) = {
            let engine = self.skill_evolution_engine.lock().await;
            engine.evolution_decision(&skill)
        };
        if !matches!(decision, EvolutionDecision::Evolve) {
            return false;
        }

        // 4. 命中：spawn 异步「同意 + 进化」，不阻塞工具执行主流程
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let proposal = build_skill_evolution_proposal(&skill, &reason);
            let approved = match crate::commands::cognitive::await_capability_consent(
                &this.app,
                &this.evolution_consent_senders,
                &proposal,
            )
            .await
            {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!(%e, "🗺️ 技能进化同意等待失败，保持原技能不变");
                    false
                },
            };
            if approved {
                this.execute_evolution(&skill).await;
            }
        });
        true
    }
}

impl SkillEvolutionHookImpl {
    /// 用户同意后执行遗传算法进化：constitution 校验 → 落库。
    async fn execute_evolution(&self, skill: &Skill) {
        let test_trajectories: Vec<Trajectory> =
            match self.trajectory_storage.get_trajectories(Some(30)).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(%e, "🗺️ 技能进化：获取轨迹失败，跳过");
                    return;
                },
            };
        let test_refs: Vec<&Trajectory> = test_trajectories.iter().collect();

        let modification = {
            let mut engine = self.skill_evolution_engine.lock().await;
            let Some(mutation) = engine.run(skill, &test_refs).await else {
                return;
            };
            mutation
        };

        if let Err(violations) = self.constitution.validate_evolution(&modification) {
            let has_fatal = violations.iter().any(|v| v.severity == ViolationSeverity::Fatal);
            let has_critical = violations.iter().any(|v| v.severity == ViolationSeverity::Critical);
            if has_fatal || has_critical {
                tracing::warn!(
                    skill = %skill.name,
                    "🗺️ 技能进化被 constitution 阻断（fatal={}, critical={}）",
                    has_fatal,
                    has_critical
                );
                return;
            }
        }

        if !modification.validation_result.as_ref().is_some_and(|v| v.success) {
            return;
        }

        let mut updated = skill.clone();
        updated.content = modification.new_content.clone();
        updated.quality_score = modification.confidence;
        updated.bump_version();
        if let Err(e) = self.trajectory_storage.save_skill(&updated).await {
            tracing::warn!(%e, skill = %skill.name, "🗺️ 技能进化结果落库失败");
            return;
        }
        tracing::info!(
            skill = %skill.name,
            confidence = %modification.confidence,
            "🗺️ 技能进化已保存（用户同意后即时路径）"
        );
    }
}

/// 构造技能进化提议（`SkillEvolution` 型能力补齐提议，走既有用户同意弹窗）。
fn build_skill_evolution_proposal(skill: &Skill, reason: &str) -> CapabilityGapProposal {
    CapabilityGapProposal {
        id: format!("skill-evol:{}", chrono::Utc::now().timestamp_millis()),
        gap_type: CapabilityGapType::SkillEvolution,
        category: None,
        title: format!("技能改进提议：技能「{}」表现偏弱", skill.name),
        proposal: format!(
            "将基于贝叶斯后验对技能「{}」运行遗传算法进化，生成表现更优的新版本并替换（保留旧版本可回退）。",
            skill.name
        ),
        reason: reason.to_string(),
        impact: "改进后该技能成功率提升；未改进前保持原版本行为。".to_string(),
        rollback: "可逆：新版本通过版本号自增保存，旧版本内容可回滚。".to_string(),
        created_at: chrono::Utc::now(),
    }
}
