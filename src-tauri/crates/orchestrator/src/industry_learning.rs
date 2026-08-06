// SPDX-License-Identifier: AGPL-3.0-only

//! 行业学习引擎 — 实现反思、进化、自我改进、强化学习的核心逻辑
//!
//! 本模块位于 `axagent-orchestrator` (consumer crate)，为避免破坏依赖方向，
//! LLM 调用能力通过注入的 trait (`LlmInferencePort`) 提供，而非直接依赖
//! `axagent-providers` 或 `axagent-dao`。
//!
//! 强化学习部分采用行业专属的规则化 RL 实现，基于历史工作流执行数据
//! 进行奖励信号计算和策略优化，无需依赖 implementor crate。
//!
//! RL 经验持久化通过注入 `RlExperienceStore` trait 实现，支持 SQLite 存储。
//! 代码验收通过注入 `CodeVerifierPort` trait 实现，支持代码级 diff 验证。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};

use axagent_harness::code_verifier::{
    CodeChange, CodeVerificationResult, CodeVerifierPort, VerificationSeverity,
};
use axagent_harness::rl::{RlExperienceRecord, RlExperienceStore};
use axagent_harness::route_engine::{
    HardGate, HardGateStatus, RouteContext, RouteDecision, RouteEngine,
};

use crate::industry_adapters::types::{
    AcceptanceResult, EvolutionConstraints, ReflectionTemplate, ReinforcementLearningConfig,
};

// ── Trait 定义：LLM 推理端口 ─────────────────────────────────

/// LLM 推理端口 — 由 wiring 层实现（通常桥接到 `axagent-harness::execute_llm`）
#[async_trait]
pub trait LlmInferencePort: Send + Sync {
    /// 执行一次 LLM 推理
    async fn infer(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String>;
}

// ── 数据结构 ─────────────────────────────────────────────────

/// 反思请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionRequest {
    pub industry_id: String,
    pub workflow_id: String,
    pub workflow_result: serde_json::Value,
    /// 代码变更列表（可选，用于代码验证）
    #[serde(default)]
    pub code_changes: Vec<CodeChange>,
}

impl Default for ReflectionRequest {
    fn default() -> Self {
        Self {
            industry_id: String::new(),
            workflow_id: String::new(),
            workflow_result: serde_json::Value::Null,
            code_changes: Vec::new(),
        }
    }
}

/// 反思结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub success: bool,
    pub industry_id: String,
    pub workflow_id: String,
    pub quality_score: f64,
    pub dimensions: Vec<DimensionScore>,
    /// 各维度得分映射（用于结构化验收）
    pub dimension_scores: HashMap<String, f64>,
    pub suggestions: Vec<String>,
    pub summary: String,
}

/// 维度评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension: String,
    pub score: f64,
    pub weight: f64,
    pub weighted_score: f64,
    pub comment: String,
}

/// 进化请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRequest {
    pub industry_id: String,
    pub workflow_id: String,
    pub reason: String,
}

/// 进化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionResult {
    pub success: bool,
    pub industry_id: String,
    pub workflow_id: String,
    pub status: String,
    pub suggested_optimizations: Vec<String>,
    pub message: String,
}

/// 自我改进请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementRequest {
    pub industry_id: String,
    pub target: String,
}

/// 自我改进结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementResult {
    pub success: bool,
    pub industry_id: String,
    pub target: String,
    pub status: String,
    pub improvements_applied: Vec<String>,
    pub message: String,
}

// ── 强化学习数据结构 ────────────────────────────────────────

/// RL 经验记录 — 单次工作流执行的经验数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLExperience {
    pub id: String,
    pub industry_id: String,
    pub workflow_id: String,
    pub timestamp_ms: u64,
    /// 质量评分（0.0-1.0）
    pub quality_score: f64,
    /// 效率评分（0.0-1.0）
    pub efficiency_score: f64,
    /// 成本评分（0.0-1.0，越高越好）
    pub cost_score: f64,
    /// 创新性评分（0.0-1.0）
    pub innovation_score: f64,
    /// 用户满意度评分（0.0-1.0）
    pub satisfaction_score: f64,
    /// 总体奖励值
    pub total_reward: f64,
    /// 执行步骤数
    pub step_count: u32,
    /// 是否成功
    pub success: bool,
    /// 行业特定元数据
    pub metadata: serde_json::Value,
}

/// RL 策略优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLPolicyUpdate {
    pub industry_id: String,
    pub experiences_used: usize,
    pub avg_reward: f64,
    pub reward_trend: String,
    pub suggested_adjustments: Vec<String>,
    pub quality_weights_optimized: Option<Vec<(String, f64)>>,
    pub reflection_threshold: Option<f64>,
    pub evolution_trigger_adjusted: Option<bool>,
}

/// RL 经验池统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePoolStats {
    pub total_experiences: usize,
    pub industry_count: usize,
    pub oldest_timestamp_ms: Option<u64>,
    pub newest_timestamp_ms: Option<u64>,
    pub avg_reward: f64,
    pub success_rate: f64,
}

/// 综合评估结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComprehensiveEvaluation {
    /// 代码验证结果
    pub code_verification: Option<CodeVerificationResult>,
    /// 反思评估结果
    pub reflection: ReflectionResult,
    /// 结构化验收结果
    pub acceptance: AcceptanceResult,
    /// 是否整体通过
    pub overall_passed: bool,
    /// 发现的问题列表
    pub issues: Vec<String>,
    /// 改进建议列表
    pub recommendations: Vec<String>,
}

// ── 学习引擎 ─────────────────────────────────────────────────

/// 行业学习引擎
pub struct IndustryLearningEngine {
    llm_port: Option<Arc<dyn LlmInferencePort>>,
    /// RL 经验持久化存储（可选，未注入时回退到内存存储）
    rl_store: Option<Arc<dyn RlExperienceStore>>,
    /// 代码验证端口（可选，用于代码级 diff 验收）
    code_verifier: Option<Arc<dyn CodeVerifierPort>>,
    /// 路由引擎（可选，用于动态路由决策）
    route_engine: Option<Arc<dyn RouteEngine>>,
    /// Hard Gate 注册表（按行业 ID + gate ID 索引）
    hard_gates: Arc<Mutex<HashMap<String, HashMap<String, HardGate>>>>,
    /// 各行业 RL 经验池（内存回退，用于未注入 rl_store 的场景）
    experience_pools: Arc<Mutex<HashMap<String, Vec<RLExperience>>>>,
    /// 各行业累计训练次数（内存回退）
    training_counts: Arc<Mutex<HashMap<String, u64>>>,
}

impl IndustryLearningEngine {
    pub fn new() -> Self {
        Self {
            llm_port: None,
            rl_store: None,
            code_verifier: None,
            route_engine: None,
            hard_gates: Arc::new(Mutex::new(HashMap::new())),
            experience_pools: Arc::new(Mutex::new(HashMap::new())),
            training_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_llm_port(mut self, port: Arc<dyn LlmInferencePort>) -> Self {
        self.llm_port = Some(port);
        self
    }

    /// 注入 RL 经验持久化存储
    pub fn with_rl_store(mut self, store: Arc<dyn RlExperienceStore>) -> Self {
        self.rl_store = Some(store);
        self
    }

    /// 注入代码验证端口
    pub fn with_code_verifier(mut self, verifier: Arc<dyn CodeVerifierPort>) -> Self {
        self.code_verifier = Some(verifier);
        self
    }

    /// 注入路由引擎
    pub fn with_route_engine(mut self, engine: Arc<dyn RouteEngine>) -> Self {
        self.route_engine = Some(engine);
        self
    }

    /// 触发工作流反思
    pub async fn reflect_on_workflow(
        &self,
        template: &ReflectionTemplate,
        request: &ReflectionRequest,
    ) -> Result<ReflectionResult, String> {
        info!(
            industry_id = %request.industry_id,
            workflow_id = %request.workflow_id,
            "触发工作流反思"
        );

        // 构造反思 Prompt
        let prompt = self.build_reflection_prompt(template, request);

        // 尝试 LLM 推理，如果失败则使用规则评估
        let result = if let Some(ref port) = self.llm_port {
            match port.infer(&prompt, None).await {
                Ok(response) => self.parse_reflection_response(&response, template, request),
                Err(e) => {
                    warn!("LLM 反思推理失败，回退到规则评估: {}", e);
                    self.rule_based_reflection(template, request)
                },
            }
        } else {
            info!("LLM 端口未配置，使用规则评估");
            self.rule_based_reflection(template, request)
        };

        Ok(result)
    }

    /// 触发工作流进化
    pub async fn evolve_workflow(
        &self,
        constraints: &EvolutionConstraints,
        request: &EvolutionRequest,
    ) -> Result<EvolutionResult, String> {
        info!(
            industry_id = %request.industry_id,
            workflow_id = %request.workflow_id,
            reason = %request.reason,
            "触发工作流进化"
        );

        // 检查是否允许进化（当禁止列表不为空时，需要检查是否包含 workflow_evolution）
        let workflow_evolution_forbidden = constraints
            .forbidden_optimizations
            .iter()
            .any(|f| f.optimization_type == "workflow_evolution");
        if workflow_evolution_forbidden {
            return Ok(EvolutionResult {
                success: false,
                industry_id: request.industry_id.clone(),
                workflow_id: request.workflow_id.clone(),
                status: "blocked".to_string(),
                suggested_optimizations: vec![],
                message: "当前行业不允许工作流进化".to_string(),
            });
        }

        // 检查是否涉及受保护步骤
        let has_protected = constraints.protected_steps.iter().any(|step| {
            request.reason.contains(&step.step_id) || request.reason.contains(&step.reason)
        });

        if has_protected {
            return Ok(EvolutionResult {
                success: false,
                industry_id: request.industry_id.clone(),
                workflow_id: request.workflow_id.clone(),
                status: "protected".to_string(),
                suggested_optimizations: vec![],
                message: "进化请求涉及受保护步骤，已拒绝".to_string(),
            });
        }

        // 构造进化 Prompt
        let prompt = self.build_evolution_prompt(constraints, request);

        // 尝试 LLM 推理
        let optimizations = if let Some(ref port) = self.llm_port {
            match port.infer(&prompt, None).await {
                Ok(response) => self.parse_evolution_response(&response),
                Err(e) => {
                    warn!("LLM 进化推理失败: {}", e);
                    vec![format!("基于规则的优化建议: 检查失败原因({})", request.reason)]
                },
            }
        } else {
            vec![format!("基于规则的优化建议: 检查失败原因({})", request.reason)]
        };

        Ok(EvolutionResult {
            success: true,
            industry_id: request.industry_id.clone(),
            workflow_id: request.workflow_id.clone(),
            status: "completed".to_string(),
            suggested_optimizations: optimizations,
            message: "工作流进化分析完成".to_string(),
        })
    }

    /// 执行自我改进
    pub async fn run_self_improvement(
        &self,
        request: &SelfImprovementRequest,
    ) -> Result<SelfImprovementResult, String> {
        info!(
            industry_id = %request.industry_id,
            target = %request.target,
            "执行自我改进"
        );

        // 构造自我改进 Prompt
        let prompt = format!(
            r#"你是一个行业自我改进引擎。请根据以下信息生成改进建议：

行业: {industry}
改进目标: {target}

请生成具体的改进措施列表。"#,
            industry = request.industry_id,
            target = request.target,
        );

        // 尝试 LLM 推理
        let improvements = if let Some(ref port) = self.llm_port {
            match port.infer(&prompt, None).await {
                Ok(response) => {
                    // 简单解析：按行分割，取列表项
                    response
                        .lines()
                        .filter(|l| l.contains('-') || l.contains('*'))
                        .map(|l| {
                            l.trim_start_matches('-').trim_start_matches('*').trim().to_string()
                        })
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                },
                Err(e) => {
                    warn!("LLM 自我改进推理失败: {}", e);
                    vec![format!("检查目标 {} 的执行历史", request.target)]
                },
            }
        } else {
            vec![format!("检查目标 {} 的执行历史", request.target)]
        };

        Ok(SelfImprovementResult {
            success: true,
            industry_id: request.industry_id.clone(),
            target: request.target.clone(),
            status: "completed".to_string(),
            improvements_applied: improvements,
            message: "自我改进分析完成".to_string(),
        })
    }

    // ── 强化学习方法 ─────────────────────────────────────

    /// 记录工作流执行经验到 RL 经验池
    ///
    /// 每次工作流执行完成后调用，积累行业特定的训练数据。
    /// 当经验量达到阈值时，`optimize_policy` 会自动触发。
    ///
    /// 优先写入数据库（如果注入了 rl_store），同时维护内存回退池。
    pub async fn record_experience(
        &self,
        industry_id: &str,
        workflow_id: &str,
        quality_score: f64,
        workflow_result: &serde_json::Value,
        config: &ReinforcementLearningConfig,
    ) -> Result<RLExperience, String> {
        let timestamp_ms =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let (efficiency_score, cost_score, innovation_score, satisfaction_score) =
            self.compute_dimension_scores(quality_score, workflow_result);

        let total_reward = self.compute_weighted_reward(
            quality_score,
            efficiency_score,
            cost_score,
            innovation_score,
            satisfaction_score,
            config,
        );

        let step_count = workflow_result
            .get("steps")
            .and_then(|s| s.as_array())
            .map(|s| s.len() as u32)
            .unwrap_or(0);

        let success = workflow_result
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s == "success" || s == "completed")
            .unwrap_or(false);

        let metadata_str = serde_json::to_string(workflow_result).unwrap_or_default();

        let experience = RLExperience {
            id: format!("exp-{}-{}", industry_id, timestamp_ms),
            industry_id: industry_id.to_string(),
            workflow_id: workflow_id.to_string(),
            timestamp_ms,
            quality_score,
            efficiency_score,
            cost_score,
            innovation_score,
            satisfaction_score,
            total_reward,
            step_count,
            success,
            metadata: workflow_result.clone(),
        };

        // 1. 持久化到数据库（如果可用）
        let db_record = RlExperienceRecord {
            id: experience.id.clone(),
            industry_id: industry_id.to_string(),
            workflow_id: workflow_id.to_string(),
            timestamp_ms: timestamp_ms as i64,
            quality_score,
            efficiency_score,
            cost_score,
            innovation_score,
            satisfaction_score,
            total_reward,
            step_count: step_count as i32,
            success,
            metadata: metadata_str,
        };

        if let Some(ref store) = self.rl_store
            && let Err(e) = store.save_experience(&db_record).await
        {
            warn!("RL 经验持久化失败，回退到内存存储: {}", e);
        }

        // 2. 同时维护内存回退池（保持向后兼容）
        let mut pools = self.experience_pools.lock().await;
        let pool = pools.entry(industry_id.to_string()).or_default();
        pool.push(experience.clone());

        // 保持每个行业最多 200 条经验（滑动窗口）
        if pool.len() > 200 {
            let excess = pool.len() - 200;
            pool.drain(0..excess);
        }

        Ok(experience)
    }

    /// 计算各维度评分（规则化评估）
    fn compute_dimension_scores(
        &self,
        quality_score: f64,
        workflow_result: &serde_json::Value,
    ) -> (f64, f64, f64, f64) {
        // 效率评分：基于步骤数和执行时间
        let step_count = workflow_result
            .get("steps")
            .and_then(|s| s.as_array())
            .map(|s| s.len() as f64)
            .unwrap_or(10.0);
        let efficiency_score = (1.0 - (step_count / 20.0)).clamp(0.3, 1.0);

        // 成本评分：基于 token 使用量（如果有的话）
        let cost_score = workflow_result
            .get("token_usage")
            .and_then(|t| t.get("total"))
            .and_then(|t| t.as_f64())
            .map(|tokens| {
                if tokens < 10_000.0 {
                    0.9
                } else if tokens < 50_000.0 {
                    0.7
                } else if tokens < 100_000.0 {
                    0.5
                } else {
                    0.3
                }
            })
            .unwrap_or(0.6);

        // 创新性评分：基于建议数量和多样性
        let suggestion_count = workflow_result
            .get("suggestions")
            .and_then(|s| s.as_array())
            .map(|s| s.len() as f64)
            .unwrap_or(0.0);
        let innovation_score = (0.5 + suggestion_count * 0.1).clamp(0.3, 1.0);

        // 满意度评分：与质量分数正相关
        let satisfaction_score = (quality_score * 0.7 + 0.3).clamp(0.0, 1.0);

        (efficiency_score, cost_score, innovation_score, satisfaction_score)
    }

    /// 计算加权总奖励
    fn compute_weighted_reward(
        &self,
        quality: f64,
        efficiency: f64,
        cost: f64,
        innovation: f64,
        satisfaction: f64,
        config: &ReinforcementLearningConfig,
    ) -> f64 {
        let weights = &config.reward_weights;
        quality * weights.quality
            + efficiency * weights.efficiency
            + cost * weights.cost
            + innovation * weights.innovation
            + satisfaction * weights.satisfaction
    }

    /// 从数据库或内存获取行业经验列表
    async fn get_industry_experiences(&self, industry_id: &str) -> Vec<RLExperience> {
        // 优先从数据库查询
        if let Some(ref store) = self.rl_store
            && let Ok(records) = store.get_experiences(industry_id, Some(200)).await
        {
            return records
                .into_iter()
                .map(|r| {
                    let metadata: serde_json::Value =
                        serde_json::from_str(&r.metadata).unwrap_or_default();
                    RLExperience {
                        id: r.id,
                        industry_id: r.industry_id,
                        workflow_id: r.workflow_id,
                        timestamp_ms: r.timestamp_ms as u64,
                        quality_score: r.quality_score,
                        efficiency_score: r.efficiency_score,
                        cost_score: r.cost_score,
                        innovation_score: r.innovation_score,
                        satisfaction_score: r.satisfaction_score,
                        total_reward: r.total_reward,
                        step_count: r.step_count as u32,
                        success: r.success,
                        metadata,
                    }
                })
                .collect();
        }

        // 回退到内存存储
        let pools = self.experience_pools.lock().await;
        pools.get(industry_id).cloned().unwrap_or_default()
    }

    /// 获取行业经验数量
    async fn get_industry_experience_count(&self, industry_id: &str) -> u64 {
        if let Some(ref store) = self.rl_store
            && let Ok(count) = store.count_experiences(industry_id).await
        {
            return count;
        }

        let pools = self.experience_pools.lock().await;
        pools.get(industry_id).map(|p| p.len() as u64).unwrap_or(0)
    }

    /// 执行策略优化
    ///
    /// 当经验池达到阈值时触发，分析历史数据并生成优化建议。
    /// 优先从数据库读取经验数据。
    pub async fn optimize_policy(
        &self,
        industry_id: &str,
        config: &ReinforcementLearningConfig,
    ) -> Result<RLPolicyUpdate, String> {
        let experiences = self.get_industry_experiences(industry_id).await;

        if experiences.len() < config.auto_train_threshold {
            return Err(format!(
                "经验不足 ({} / {})，无法执行策略优化",
                experiences.len(),
                config.auto_train_threshold
            ));
        }

        // 计算统计量
        let avg_reward =
            experiences.iter().map(|e| e.total_reward).sum::<f64>() / experiences.len() as f64;
        let success_count = experiences.iter().filter(|e| e.success).count();
        let success_rate = success_count as f64 / experiences.len() as f64;

        // 分析奖励趋势（最近 20 条 vs 之前）
        let recent_count = 20.min(experiences.len());
        let recent: Vec<&RLExperience> = experiences.iter().rev().take(recent_count).collect();
        let older: Vec<&RLExperience> =
            experiences.iter().rev().skip(recent_count).take(recent_count).collect();

        let recent_avg = recent.iter().map(|e| e.total_reward).sum::<f64>() / recent.len() as f64;
        let older_avg = if older.is_empty() {
            recent_avg
        } else {
            older.iter().map(|e| e.total_reward).sum::<f64>() / older.len() as f64
        };

        let reward_trend = if recent_avg > older_avg + 0.05 {
            "improving".to_string()
        } else if recent_avg < older_avg - 0.05 {
            "declining".to_string()
        } else {
            "stable".to_string()
        };

        // 生成优化建议
        let mut suggestions = Vec::new();

        if success_rate < 0.5 {
            suggestions.push("成功率低于 50%，建议加强工作流分解策略".to_string());
        }

        if avg_reward < 0.5 {
            suggestions.push("平均奖励偏低，建议优化反思模板权重".to_string());
        }

        if matches!(reward_trend.as_str(), "declining") {
            suggestions.push("奖励呈下降趋势，建议触发进化优化".to_string());
        }

        // 分析各维度瓶颈
        let avg_quality =
            experiences.iter().map(|e| e.quality_score).sum::<f64>() / experiences.len() as f64;
        let avg_efficiency =
            experiences.iter().map(|e| e.efficiency_score).sum::<f64>() / experiences.len() as f64;
        let avg_cost =
            experiences.iter().map(|e| e.cost_score).sum::<f64>() / experiences.len() as f64;

        if avg_quality < 0.6 {
            suggestions.push("质量评分偏低，建议增加质量相关检查点权重".to_string());
        }
        if avg_efficiency < 0.6 {
            suggestions.push("执行效率偏低，建议优化步骤编排".to_string());
        }
        if avg_cost < 0.5 {
            suggestions.push("成本控制不佳，建议引入缓存和批量处理策略".to_string());
        }

        // 计算优化后的质量权重
        let quality_weights_optimized = if !suggestions.is_empty() {
            let mut weights = vec![
                ("task_completion".to_string(), (avg_quality * 0.4 + 0.3).clamp(0.1, 0.6)),
                ("output_quality".to_string(), (avg_quality * 0.35 + 0.25).clamp(0.1, 0.5)),
                ("efficiency".to_string(), (avg_efficiency * 0.3 + 0.2).clamp(0.1, 0.4)),
                ("cost_efficiency".to_string(), (avg_cost * 0.3 + 0.15).clamp(0.05, 0.3)),
            ];
            let total: f64 = weights.iter().map(|(_, w)| *w).sum();
            for (_, w) in weights.iter_mut() {
                *w /= total;
            }
            Some(weights)
        } else {
            None
        };

        // 反思阈值调整
        let reflection_threshold = if avg_quality < 0.7 {
            Some(0.6) // 降低触发进化的阈值，让更多工作流获得进化机会
        } else if avg_quality > 0.85 {
            Some(0.8) // 提高阈值，减少不必要的进化
        } else {
            None
        };

        // 进化触发调整
        let evolution_trigger_adjusted = Some(success_rate < 0.6);

        let experiences_used = experiences.len();

        // 更新训练计数
        let mut counts = self.training_counts.lock().await;
        *counts.entry(industry_id.to_string()).or_insert(0) += 1;
        drop(counts);

        // 如果有 rl_store，更新训练统计
        if let Some(ref store) = self.rl_store {
            let stats = axagent_harness::rl::RlIndustryStats {
                industry_id: industry_id.to_string(),
                total_experiences: experiences_used as i32,
                total_reward: avg_reward * experiences_used as f64,
                avg_reward,
                success_rate,
                last_trained_at: Some(
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                        as i64,
                ),
                policy_updated_at: Some(
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                        as i64,
                ),
                optimization_goals: suggestions.clone(),
            };
            let _ = store.upsert_stats(&stats).await;
        }

        info!(
            industry_id = %industry_id,
            avg_reward = avg_reward,
            success_rate = success_rate,
            trend = %reward_trend,
            "RL 策略优化完成"
        );

        Ok(RLPolicyUpdate {
            industry_id: industry_id.to_string(),
            experiences_used,
            avg_reward,
            reward_trend,
            suggested_adjustments: suggestions,
            quality_weights_optimized,
            reflection_threshold,
            evolution_trigger_adjusted,
        })
    }

    /// 触发行业强化学习闭环
    ///
    /// 记录经验 → 检查阈值 → 触发策略优化
    pub async fn run_reinforcement_learning(
        &self,
        industry_id: &str,
        workflow_id: &str,
        quality_score: f64,
        workflow_result: &serde_json::Value,
        config: &ReinforcementLearningConfig,
    ) -> Result<serde_json::Value, String> {
        if !config.enabled {
            return Ok(serde_json::json!({
                "status": "skipped",
                "reason": "reinforcement learning not enabled",
            }));
        }

        // 1. 记录经验（同时写入数据库和内存回退池）
        let experience = self
            .record_experience(industry_id, workflow_id, quality_score, workflow_result, config)
            .await?;

        // 2. 检查是否达到训练阈值（优先从数据库查询）
        let pool_len = self.get_industry_experience_count(industry_id).await as usize;

        let mut policy_update: Option<RLPolicyUpdate> = None;

        if pool_len >= config.auto_train_threshold {
            // 3. 触发策略优化
            match self.optimize_policy(industry_id, config).await {
                Ok(update) => {
                    policy_update = Some(update);
                },
                Err(e) => {
                    warn!("RL 策略优化失败: {}", e);
                },
            }
        }

        let training_count = {
            let counts = self.training_counts.lock().await;
            counts.get(industry_id).copied().unwrap_or(0)
        };

        Ok(serde_json::json!({
            "status": "completed",
            "industryId": industry_id,
            "experienceRecorded": {
                "id": experience.id,
                "totalReward": experience.total_reward,
                "qualityScore": experience.quality_score,
                "success": experience.success,
            },
            "poolSize": pool_len,
            "threshold": config.auto_train_threshold,
            "policyOptimized": policy_update.is_some(),
            "policyUpdate": policy_update.map(|u| {
                serde_json::json!({
                    "experiencesUsed": u.experiences_used,
                    "avgReward": u.avg_reward,
                    "rewardTrend": u.reward_trend,
                    "suggestedAdjustments": u.suggested_adjustments,
                })
            }),
            "totalTrainingRounds": training_count,
        }))
    }

    /// 获取经验池统计信息
    ///
    /// 优先从数据库查询，如果不可用则回退到内存存储。
    pub async fn get_experience_pool_stats(&self) -> ExperiencePoolStats {
        // 优先从数据库查询全局统计
        if let Some(ref store) = self.rl_store
            && let Ok(stats_list) = store.get_global_stats().await
            && !stats_list.is_empty()
        {
            let total: usize = stats_list.iter().map(|s| s.total_experiences as usize).sum();
            let industry_count = stats_list.len();
            let avg_reward = if total > 0 {
                stats_list.iter().map(|s| s.total_reward).sum::<f64>() / total as f64
            } else {
                0.0
            };
            let success_rate = if total > 0 {
                stats_list.iter().map(|s| s.success_rate).sum::<f64>() / industry_count as f64
            } else {
                0.0
            };

            // 获取时间戳范围
            let mut min_ts: Option<u64> = None;
            let mut max_ts: Option<u64> = None;
            for stats in &stats_list {
                if let Some(last) = stats.last_trained_at {
                    let ts = last as u64;
                    match (min_ts, max_ts) {
                        (Some(min), Some(max)) => {
                            min_ts = Some(min.min(ts));
                            max_ts = Some(max.max(ts));
                        },
                        _ => {
                            min_ts = Some(ts);
                            max_ts = Some(ts);
                        },
                    }
                }
            }

            return ExperiencePoolStats {
                total_experiences: total,
                industry_count,
                oldest_timestamp_ms: min_ts,
                newest_timestamp_ms: max_ts,
                avg_reward,
                success_rate,
            };
        }

        // 回退到内存存储
        let pools = self.experience_pools.lock().await;
        let mut total = 0usize;
        let mut industries = std::collections::HashSet::new();
        let mut rewards = Vec::new();
        let mut success_count = 0usize;
        let mut min_ts: Option<u64> = None;
        let mut max_ts: Option<u64> = None;

        for (industry_id, pool) in pools.iter() {
            industries.insert(industry_id.clone());
            for exp in pool {
                total += 1;
                rewards.push(exp.total_reward);
                if exp.success {
                    success_count += 1;
                }
                match (min_ts, max_ts) {
                    (Some(min), Some(max)) => {
                        min_ts = Some(min.min(exp.timestamp_ms));
                        max_ts = Some(max.max(exp.timestamp_ms));
                    },
                    _ => {
                        min_ts = Some(exp.timestamp_ms);
                        max_ts = Some(exp.timestamp_ms);
                    },
                }
            }
        }

        let avg_reward = if rewards.is_empty() {
            0.0
        } else {
            rewards.iter().sum::<f64>() / rewards.len() as f64
        };
        let success_rate = if total > 0 {
            success_count as f64 / total as f64
        } else {
            0.0
        };

        ExperiencePoolStats {
            total_experiences: total,
            industry_count: industries.len(),
            oldest_timestamp_ms: min_ts,
            newest_timestamp_ms: max_ts,
            avg_reward,
            success_rate,
        }
    }

    /// P2-7：按行业返回 RL 经验统计（stats_list 或内存池过滤）。
    pub async fn get_industry_experience_stats(
        &self,
        industry_id: &str,
    ) -> Option<ExperiencePoolStats> {
        if let Some(ref store) = self.rl_store
            && let Ok(stats_list) = store.get_global_stats().await
            && let Some(s) = stats_list.into_iter().find(|s| s.industry_id == industry_id)
        {
            return Some(ExperiencePoolStats {
                total_experiences: s.total_experiences as usize,
                industry_count: 1,
                oldest_timestamp_ms: s.last_trained_at.map(|t| t as u64),
                newest_timestamp_ms: s.policy_updated_at.map(|t| t as u64),
                avg_reward: s.avg_reward,
                success_rate: s.success_rate,
            });
        }

        // 内存池回退
        let pools = self.experience_pools.lock().await;
        let pool = pools.get(industry_id)?;
        let mut total = 0usize;
        let mut rewards = Vec::new();
        let mut success_count = 0usize;
        let mut min_ts: Option<u64> = None;
        let mut max_ts: Option<u64> = None;
        for exp in pool {
            total += 1;
            rewards.push(exp.total_reward);
            if exp.success {
                success_count += 1;
            }
            match (min_ts, max_ts) {
                (Some(min), Some(max)) => {
                    min_ts = Some(min.min(exp.timestamp_ms));
                    max_ts = Some(max.max(exp.timestamp_ms));
                },
                _ => {
                    min_ts = Some(exp.timestamp_ms);
                    max_ts = Some(exp.timestamp_ms);
                },
            }
        }
        let avg_reward = if rewards.is_empty() {
            0.0
        } else {
            rewards.iter().sum::<f64>() / rewards.len() as f64
        };
        let success_rate = if total > 0 {
            success_count as f64 / total as f64
        } else {
            0.0
        };
        Some(ExperiencePoolStats {
            total_experiences: total,
            industry_count: 1,
            oldest_timestamp_ms: min_ts,
            newest_timestamp_ms: max_ts,
            avg_reward,
            success_rate,
        })
    }

    // ── 代码验收方法 ─────────────────────────────────────

    /// 验证代码变更
    ///
    /// 在工作流完成后进行代码级验收，检查变更质量和规范性。
    pub async fn verify_code_changes(
        &self,
        industry_id: &str,
        workflow_id: &str,
        changes: &[CodeChange],
    ) -> Result<axagent_harness::code_verifier::CodeVerificationResult, String> {
        if let Some(ref verifier) = self.code_verifier {
            verifier.verify_changes(industry_id, workflow_id, changes).await
        } else {
            // 回退：使用内置的规则化验证
            Ok(self.rule_based_verification(industry_id, changes))
        }
    }

    /// 获取行业特定的验证规则
    pub async fn get_verification_rules(
        &self,
        industry_id: &str,
    ) -> Result<Vec<axagent_harness::code_verifier::VerificationRule>, String> {
        if let Some(ref verifier) = self.code_verifier {
            verifier.get_verification_rules(industry_id).await
        } else {
            Ok(self.default_verification_rules(industry_id))
        }
    }

    /// 规则化代码验证（无外部验证器时的回退逻辑）
    fn rule_based_verification(
        &self,
        industry_id: &str,
        changes: &[CodeChange],
    ) -> axagent_harness::code_verifier::CodeVerificationResult {
        use axagent_harness::code_verifier::{
            CodeVerificationResult, VerificationIssue, VerificationSeverity,
        };

        let total_changes = changes.len();
        let total_added: u32 = changes.iter().map(|c| c.lines_added).sum();
        let total_removed: u32 = changes.iter().map(|c| c.lines_removed).sum();
        let mut issues = Vec::new();

        // 检查 1: 大量变更警告
        let total_lines = total_added + total_removed;
        if total_lines > 200 {
            issues.push(VerificationIssue {
                severity: VerificationSeverity::Warning,
                category: "large_change".to_string(),
                description: format!("代码变更较大（{}行），建议分批次提交", total_lines),
                file_path: None,
                line_number: None,
                suggestion: "考虑将大变更拆分为多个小的、独立的提交".to_string(),
            });
        }

        // 检查 2: 删除代码过多警告
        if total_removed > total_added * 2 && total_removed > 50 {
            issues.push(VerificationIssue {
                severity: VerificationSeverity::Info,
                category: "high_removal".to_string(),
                description: format!(
                    "删除行数（{}）远超新增行数（{}），请确认是否为预期行为",
                    total_removed, total_added
                ),
                file_path: None,
                line_number: None,
                suggestion: "确认删除的代码确实不再需要，或添加迁移兼容层".to_string(),
            });
        }

        // 检查 3: 文件数过多警告
        if total_changes > 15 {
            issues.push(VerificationIssue {
                severity: VerificationSeverity::Warning,
                category: "many_files".to_string(),
                description: format!("涉及 {} 个文件的变更，影响范围较大", total_changes),
                file_path: None,
                line_number: None,
                suggestion: "建议进行更全面的测试和代码审查".to_string(),
            });
        }

        // 计算分数
        let score = if issues.is_empty() {
            0.9 // 无问题，高分
        } else {
            let blocker_count =
                issues.iter().filter(|i| i.severity == VerificationSeverity::Blocking).count();
            let warning_count =
                issues.iter().filter(|i| i.severity == VerificationSeverity::Warning).count();
            let info_count =
                issues.iter().filter(|i| i.severity == VerificationSeverity::Info).count();

            (0.9 - blocker_count as f64 * 0.3
                - warning_count as f64 * 0.1
                - info_count as f64 * 0.05)
                .clamp(0.0, 1.0)
        };

        let passed = !issues.iter().any(|i| i.severity == VerificationSeverity::Blocking);

        let suggested_action = if passed {
            if score >= 0.8 {
                "approve".to_string()
            } else {
                "review".to_string()
            }
        } else {
            "request_changes".to_string()
        };

        CodeVerificationResult {
            passed,
            score,
            summary: format!(
                "行业 {} 代码验收：{} 个文件变更，+{} / -{} 行，{} 个问题",
                industry_id,
                total_changes,
                total_added,
                total_removed,
                issues.len()
            ),
            issues,
            changes: changes.to_vec(),
            suggested_action,
            verified_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// 默认验证规则
    fn default_verification_rules(
        &self,
        _industry_id: &str,
    ) -> Vec<axagent_harness::code_verifier::VerificationRule> {
        use axagent_harness::code_verifier::{VerificationRule, VerificationSeverity};

        vec![
            VerificationRule {
                id: "no_hardcoded_secrets".to_string(),
                name: "禁止硬编码密钥".to_string(),
                description: "检查代码中是否包含硬编码的密钥、密码或 token".to_string(),
                severity: VerificationSeverity::Blocking,
                pattern: r#"(?i)(password|secret|api_key|token)\s*[:=]\s*["'][^"']+["']"#
                    .to_string(),
                enabled: true,
            },
            VerificationRule {
                id: "no_todo_leftover".to_string(),
                name: "清理 TODO 注释".to_string(),
                description: "确保没有遗留的 TODO/FIXME 注释".to_string(),
                severity: VerificationSeverity::Info,
                pattern: r#"(?i)\b(TODO|FIXME|HACK|XXX)\b"#.to_string(),
                enabled: true,
            },
            VerificationRule {
                id: "max_line_length".to_string(),
                name: "单行长度限制".to_string(),
                description: "代码行不超过 120 个字符".to_string(),
                severity: VerificationSeverity::Warning,
                pattern: ".{121,}".to_string(),
                enabled: true,
            },
        ]
    }

    // ── 路由引擎方法 ─────────────────────────────────────

    /// 做出路由决策
    pub async fn decide_route(&self, context: &RouteContext) -> Result<RouteDecision, String> {
        if let Some(ref engine) = self.route_engine {
            engine.decide_route(context).await
        } else {
            // 回退：使用内置的规则化路由
            Ok(self.rule_based_route_decision(context))
        }
    }

    /// 评估 Hard Gate
    pub async fn evaluate_gate(
        &self,
        industry_id: &str,
        gate_id: &str,
        context: &RouteContext,
    ) -> Result<HardGateStatus, String> {
        // 先检查 Gate 是否已注册
        let gates = self.hard_gates.lock().await;
        let industry_gates = gates
            .get(industry_id)
            .ok_or_else(|| format!("行业 {} 没有注册任何 Gate", industry_id))?;
        let gate = industry_gates.get(gate_id).ok_or_else(|| format!("Gate {} 不存在", gate_id))?;

        if let Some(ref engine) = self.route_engine {
            engine.evaluate_gate(gate, context).await
        } else {
            // 回退：使用内置的规则化评估
            Ok(self.rule_based_gate_evaluation(gate, context))
        }
    }

    /// 注册 Hard Gate
    pub async fn register_hard_gate(
        &self,
        industry_id: &str,
        gate: HardGate,
    ) -> Result<(), String> {
        let mut gates = self.hard_gates.lock().await;
        let industry_gates = gates.entry(industry_id.to_string()).or_default();
        industry_gates.insert(gate.id.clone(), gate);
        Ok(())
    }

    /// 获取行业的所有 Hard Gate
    pub async fn get_hard_gates(&self, industry_id: &str) -> Vec<HardGate> {
        let gates = self.hard_gates.lock().await;
        gates
            .get(industry_id)
            .map(|industry_gates| industry_gates.values().cloned().collect())
            .unwrap_or_default()
    }

    // ── 结构化验收方法 ─────────────────────────────────────

    /// 使用结构化验收标准评估工作流结果
    pub async fn evaluate_with_criteria(
        &self,
        template: &ReflectionTemplate,
        dimension_scores: &HashMap<String, f64>,
    ) -> AcceptanceResult {
        if !template.structured_verification_enabled {
            return AcceptanceResult {
                passed: true,
                total_criteria: 0,
                passed_criteria: 0,
                failed_criteria: 0,
                details: Vec::new(),
                overall_score: 1.0,
            };
        }

        template.evaluate_acceptance(dimension_scores)
    }

    /// 综合评估：结合代码验证 + 结构化验收
    pub async fn comprehensive_evaluation(
        &self,
        template: &ReflectionTemplate,
        request: &ReflectionRequest,
        code_changes: Option<&[CodeChange]>,
    ) -> Result<ComprehensiveEvaluation, String> {
        // 1. 代码验证
        let code_verification = if let Some(changes) = code_changes {
            Some(
                self.verify_code_changes(&request.industry_id, &request.workflow_id, changes)
                    .await?,
            )
        } else {
            None
        };

        // 2. 反思评估
        let reflection = self.reflect_on_workflow(template, request).await?;

        // 3. 结构化验收
        let acceptance = self.evaluate_with_criteria(template, &reflection.dimension_scores).await;

        // 4. 综合判断
        let mut issues: Vec<String> = Vec::new();
        let mut recommendations: Vec<String> = Vec::new();

        // 代码验证问题
        if let Some(ref cv) = code_verification {
            for issue in &cv.issues {
                if matches!(issue.severity, VerificationSeverity::Blocking) {
                    issues.push(format!("[代码] {}", issue.description));
                }
            }
            if !cv.passed {
                recommendations.push("修复代码规范问题后重新提交".to_string());
            }
        }

        // 验收标准问题
        for detail in &acceptance.details {
            if !detail.passed {
                issues.push(format!(
                    "[验收] {}: 得分 {:.2} < 阈值 {:.2}",
                    detail.criterion_name, detail.score, detail.threshold
                ));
                recommendations.push(format!(
                    "提升 {} 得分至 {:.2} 以上",
                    detail.criterion_name, detail.threshold
                ));
            }
        }

        Ok(ComprehensiveEvaluation {
            code_verification,
            reflection,
            acceptance,
            overall_passed: issues.is_empty(),
            issues,
            recommendations,
        })
    }

    /// 规则化路由决策（无外部路由引擎时的回退逻辑）
    fn rule_based_route_decision(&self, context: &RouteContext) -> RouteDecision {
        use axagent_harness::route_engine::RouteDecisionType;

        // 检查最近节点的执行结果
        let recent_failure = context.execution_history.iter().rev().take(3).all(|r| !r.success);

        if recent_failure {
            // 连续失败：建议回退
            RouteDecision {
                id: format!("route-decision-{}", uuid::Uuid::new_v4()),
                decision_type: RouteDecisionType::Fallback {
                    fallback_path_id: "default_fallback".to_string(),
                },
                reason: "最近 3 个节点连续失败，建议回退到备用路径".to_string(),
                confidence: 0.7,
                context: serde_json::Value::Null,
                decided_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            }
        } else {
            // 默认继续
            RouteDecision {
                id: format!("route-decision-{}", uuid::Uuid::new_v4()),
                decision_type: RouteDecisionType::Continue,
                reason: "默认路由决策：继续执行".to_string(),
                confidence: 1.0,
                context: serde_json::Value::Null,
                decided_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            }
        }
    }

    /// 规则化 Gate 评估（无外部路由引擎时的回退逻辑）
    fn rule_based_gate_evaluation(
        &self,
        gate: &HardGate,
        context: &RouteContext,
    ) -> HardGateStatus {
        // 基于最近节点的执行质量进行评估
        let recent_quality = context
            .execution_history
            .iter()
            .rev()
            .take(gate.criteria.len().max(3))
            .map(|r| r.quality_score)
            .collect::<Vec<_>>();

        if recent_quality.is_empty() {
            return HardGateStatus::Active;
        }

        let avg_quality = recent_quality.iter().sum::<f64>() / recent_quality.len() as f64;

        // 计算 Gate 得分
        let mut scores = std::collections::HashMap::new();
        for criterion in &gate.criteria {
            let base_score = avg_quality;
            let adjusted = (base_score * criterion.weight).clamp(0.0, 1.0);
            scores.insert(criterion.id.clone(), adjusted);
        }

        if gate.is_passed(&scores) {
            HardGateStatus::Passed
        } else {
            HardGateStatus::Rejected {
                reason: format!("综合得分 {:.2} 低于阈值，需要改进", gate.calculate_score(&scores)),
            }
        }
    }

    // ── 私有方法：Prompt 构造 ─────────────────────────────

    fn build_reflection_prompt(
        &self,
        template: &ReflectionTemplate,
        request: &ReflectionRequest,
    ) -> String {
        let checkpoint_descriptions: Vec<String> = template
            .checkpoints
            .iter()
            .map(|c| {
                format!(
                    "- {} (维度: {}, 权重: {:.2}): {}",
                    c.name, c.dimension, c.weight, c.description
                )
            })
            .collect();

        let prompts_str = if template.prompts.is_empty() {
            "请评估本次执行的质量和效率".to_string()
        } else {
            template.prompts.join("\n")
        };

        format!(
            r#"你是一个行业反思评估引擎。请根据以下反思模板对工作流结果进行评估。

## 行业: {industry_id}
## 反思模板: {template_name}

### 质量评估权重:
- 任务完成度: {weight_task_completion:.2}
- 输出质量: {weight_output_quality:.2}
- 执行效率: {weight_efficiency:.2}
- 成本效率: {weight_cost_efficiency:.2}

### 质量检查点:
{checkpoints}

### 反思提示:
{prompts}

### 工作流结果:
```json
{workflow_result}
```

请生成评估结果，包括：
1. 各维度评分（0-100）
2. 综合质量评分
3. 具体的改进建议
4. 总体评价摘要"#,
            industry_id = request.industry_id,
            template_name = template.name,
            weight_task_completion = template.quality_weights.task_completion,
            weight_output_quality = template.quality_weights.output_quality,
            weight_efficiency = template.quality_weights.efficiency,
            weight_cost_efficiency = template.quality_weights.cost_efficiency,
            checkpoints = checkpoint_descriptions.join("\n"),
            prompts = prompts_str,
            workflow_result = serde_json::to_string_pretty(&request.workflow_result)
                .unwrap_or_else(|_| "{}".to_string()),
        )
    }

    fn build_evolution_prompt(
        &self,
        constraints: &EvolutionConstraints,
        request: &EvolutionRequest,
    ) -> String {
        let protected: Vec<String> = constraints
            .protected_steps
            .iter()
            .map(|s| format!("- {} ({})", s.step_id, s.reason))
            .collect();

        let depends: Vec<String> = constraints
            .step_dependencies
            .iter()
            .map(|d| format!("- {} -> {}", d.from, d.to))
            .collect();

        let protected_str = if protected.is_empty() {
            "无".to_string()
        } else {
            protected.join("\n")
        };
        let depends_str = if depends.is_empty() {
            "无".to_string()
        } else {
            depends.join("\n")
        };

        format!(
            r#"你是一个工作流进化引擎。请根据进化约束分析工作流优化方案。

## 行业: {industry_id}
## 进化原因: {reason}

### 受保护步骤（不可修改或删除）:
{protected}

### 关键依赖关系（必须保持）:
{depends}

请生成具体的优化建议，包括可以改进的地方和预期效果。"#,
            industry_id = request.industry_id,
            reason = request.reason,
            protected = protected_str,
            depends = depends_str,
        )
    }

    // ── 私有方法：响应解析 ────────────────────────────────

    fn parse_reflection_response(
        &self,
        response: &str,
        template: &ReflectionTemplate,
        request: &ReflectionRequest,
    ) -> ReflectionResult {
        let mut dimensions = Vec::new();
        let mut dimension_scores = HashMap::new();
        let mut total_score = 0.0;

        // 尝试从响应中提取分数
        for checkpoint in &template.checkpoints {
            let score = self.extract_score_for_dimension(response, &checkpoint.dimension);
            let weighted = score * checkpoint.weight;
            total_score += weighted;

            dimension_scores.insert(checkpoint.dimension.clone(), score);

            dimensions.push(DimensionScore {
                dimension: checkpoint.dimension.clone(),
                score,
                weight: checkpoint.weight,
                weighted_score: weighted,
                comment: self.extract_comment_for_dimension(response, &checkpoint.dimension),
            });
        }

        // 提取建议
        let suggestions = self.extract_list_items(response);

        // 提取摘要
        let summary = self.extract_summary(response);

        ReflectionResult {
            success: true,
            industry_id: request.industry_id.clone(),
            workflow_id: request.workflow_id.clone(),
            quality_score: (total_score * 100.0).min(100.0),
            dimensions,
            dimension_scores,
            suggestions,
            summary,
        }
    }

    fn parse_evolution_response(&self, response: &str) -> Vec<String> {
        self.extract_list_items(response)
    }

    // ── 私有方法：规则评估（无 LLM 时回退） ──────────────

    fn rule_based_reflection(
        &self,
        template: &ReflectionTemplate,
        request: &ReflectionRequest,
    ) -> ReflectionResult {
        let mut dimensions = Vec::new();
        let mut dimension_scores = HashMap::new();
        let mut total_score = 0.0;

        // 基于规则的简单评分：结果中有 "success" 字段且为 true 则高分
        let is_success = request
            .workflow_result
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s == "success" || s == "completed")
            .unwrap_or(false);

        let base_score = if is_success { 0.85 } else { 0.5 };

        for checkpoint in &template.checkpoints {
            let score = base_score + (checkpoint.weight * 0.1);
            let weighted = score * checkpoint.weight;
            total_score += weighted;

            dimension_scores.insert(checkpoint.dimension.clone(), score);

            dimensions.push(DimensionScore {
                dimension: checkpoint.dimension.clone(),
                score,
                weight: checkpoint.weight,
                weighted_score: weighted,
                comment: if is_success {
                    "工作流执行成功，质量良好".to_string()
                } else {
                    "工作流执行未完全成功，需要改进".to_string()
                },
            });
        }

        let suggestions = if is_success {
            vec!["继续保持当前执行策略".to_string()]
        } else {
            vec!["检查工作流执行过程中的失败原因".to_string(), "优化任务分解和分配策略".to_string()]
        };

        ReflectionResult {
            success: true,
            industry_id: request.industry_id.clone(),
            workflow_id: request.workflow_id.clone(),
            quality_score: (total_score * 100.0).min(100.0),
            dimensions,
            dimension_scores,
            suggestions,
            summary: if is_success {
                "工作流整体执行良好".to_string()
            } else {
                "工作流执行存在改进空间".to_string()
            },
        }
    }

    // ── 私有方法：文本解析辅助 ────────────────────────────

    fn extract_score_for_dimension(&self, response: &str, dimension: &str) -> f64 {
        // 尝试匹配 "维度: 分数" 或 "dimension: score" 模式
        let pattern = format!(r"(?i){}[^\d]*(\d+\.?\d*)", regex::escape(dimension));
        let Ok(re) = regex::Regex::new(&pattern) else {
            return 0.7;
        };
        let Some(caps) = re.captures(response) else {
            return 0.7;
        };
        let Some(score_str) = caps.get(1) else {
            return 0.7;
        };
        let Ok(score) = score_str.as_str().parse::<f64>() else {
            return 0.7;
        };
        (score / 100.0).min(1.0)
    }

    fn extract_comment_for_dimension(&self, response: &str, dimension: &str) -> String {
        let pattern = format!(r"(?i){}[^\n]*", regex::escape(dimension));
        let Ok(re) = regex::Regex::new(&pattern) else {
            return "无评论".to_string();
        };
        let Some(m) = re.find(response) else {
            return "无评论".to_string();
        };
        let text = &response[m.start()..m.end()];
        // 清理掉开头的维度名
        let colon_pos = text.find(':').or_else(|| text.find('：'));
        if let Some(pos) = colon_pos {
            text[pos + 1..].trim().to_string()
        } else {
            text.trim().to_string()
        }
    }

    fn extract_list_items(&self, response: &str) -> Vec<String> {
        response
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with("•")
            })
            .map(|l| {
                l.trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim_start_matches("•")
                    .trim()
                    .to_string()
            })
            .filter(|l| !l.is_empty())
            .collect()
    }

    fn extract_summary(&self, response: &str) -> String {
        // 尝试找到摘要或总结部分
        let summary_patterns = ["摘要", "总结", "总体评价", "Summary", "Conclusion"];
        for keyword in summary_patterns {
            if let Some(pos) = response.find(keyword) {
                let rest = &response[pos + keyword.len()..];
                let first_line = rest.lines().find(|l| !l.trim().is_empty()).unwrap_or("无摘要");
                return first_line.trim().to_string();
            }
        }
        // 默认取第一行非空文本
        response.lines().find(|l| !l.trim().is_empty()).unwrap_or("无法生成摘要").trim().to_string()
    }
}

impl Default for IndustryLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}
