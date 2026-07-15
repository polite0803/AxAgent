// SPDX-License-Identifier: AGPL-3.0-only

//! Smart Model Router — task-aware model selection for cost-efficient LLM usage.
//!
//! This module implements a two-layer routing architecture:
//!
//! **Layer 1 — Heuristic Classifier** (fast, no data dependency):
//! - **trivial**: Format conversion, translation, summary → Budget tier
//! - **moderate**: Q&A, code explanation, data analysis → Balanced tier
//! - **complex**: Architecture design, multi-step reasoning, debugging → Premium tier
//!
//! **Layer 2 — ML Cost-Aware Optimizer** (learns from historical outcomes):
//! - Tracks routing decisions and their outcomes (success/failure, latency, cost)
//! - Computes task feature vectors (prompt length, code density, structural complexity)
//! - When historical data is sufficient, may override heuristic decisions:
//!   - Upgrade to Premium if Budget/Balanced has high failure rate for similar tasks
//!   - Downgrade to Budget if Balanced achieves same quality with lower cost
//! - Implements cost budget enforcement (downgrade tier if budget exceeded)
//!
//! ## Feedback Loop
//!
//! The frontend calls `route_feedback` after each LLM call to report:
//! - Whether the response was satisfactory
//! - Actual latency and token usage
//! - Whether the user manually switched tiers
//!
//! This data flows into the ML layer to continuously improve routing decisions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

// ─── Task Feature Vector ───

/// Lightweight feature vector extracted from a prompt for ML-based routing.
/// Computed without any LLM call — purely structural analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFeatureVector {
    /// Total prompt length in characters.
    pub prompt_len: usize,
    /// Number of lines.
    pub line_count: usize,
    /// Number of code blocks (``` pairs).
    pub code_block_count: usize,
    /// Whether the prompt contains SQL.
    pub has_sql: bool,
    /// Density of complex keywords (count / prompt_len).
    pub complex_keyword_density: f32,
    /// Density of trivial keywords (count / prompt_len).
    pub trivial_keyword_density: f32,
    /// Whether the prompt contains a file path pattern.
    pub has_file_paths: bool,
    /// Whether the prompt contains multiple distinct tasks (numbered/comma-separated).
    pub is_multi_task: bool,
}

impl TaskFeatureVector {
    /// Extract feature vector from a prompt.
    pub fn from_prompt(prompt: &str) -> Self {
        let lower = prompt.to_lowercase();
        let prompt_len = prompt.len();
        let line_count = prompt.lines().count();
        let code_block_count = lower.matches("```").count() / 2;

        let complex_count = COMPLEX_KEYWORDS.iter().filter(|kw| lower.contains(*kw)).count();
        let trivial_count = TRIVIAL_KEYWORDS.iter().filter(|kw| lower.contains(*kw)).count();

        let complex_keyword_density = if prompt_len > 0 {
            complex_count as f32 / prompt_len as f32
        } else {
            0.0
        };
        let trivial_keyword_density = if prompt_len > 0 {
            trivial_count as f32 / prompt_len as f32
        } else {
            0.0
        };

        let has_sql = lower.contains("sql");
        let has_file_paths = lower.contains(".rs")
            || lower.contains(".ts")
            || lower.contains(".py")
            || lower.contains(".js")
            || lower.contains(".toml")
            || lower.contains(".json");

        let is_multi_task = lower.contains("\n- ")
            || lower.contains("\n* ")
            || lower.contains("\n1. ")
            || lower.contains("\n2. ")
            || lower.contains(", then ")
            || lower.contains(" and then ");

        Self {
            prompt_len,
            line_count,
            code_block_count,
            has_sql,
            complex_keyword_density,
            trivial_keyword_density,
            has_file_paths,
            is_multi_task,
        }
    }

    /// Compute a similarity score between two feature vectors (0.0 = dissimilar, 1.0 = identical).
    pub fn similarity(&self, other: &TaskFeatureVector) -> f32 {
        let mut score = 0.0_f32;
        let mut weight = 0.0_f32;

        // Length similarity (log scale to handle wide range)
        let len_ratio = if self.prompt_len.max(other.prompt_len) > 0 {
            self.prompt_len.min(other.prompt_len) as f32
                / self.prompt_len.max(other.prompt_len) as f32
        } else {
            1.0
        };
        score += len_ratio * 0.15;
        weight += 0.15;

        // Line count similarity
        let line_ratio = if self.line_count.max(other.line_count) > 0 {
            self.line_count.min(other.line_count) as f32
                / self.line_count.max(other.line_count) as f32
        } else {
            1.0
        };
        score += line_ratio * 0.10;
        weight += 0.10;

        // Code block count similarity
        let code_sim = 1.0
            - (self.code_block_count as f32 - other.code_block_count as f32).abs()
                / (self.code_block_count.max(other.code_block_count).max(1) as f32);
        score += code_sim * 0.15;
        weight += 0.15;

        // SQL match
        if self.has_sql == other.has_sql {
            score += 0.10;
        }
        weight += 0.10;

        // Keyword density similarity
        let complex_sim =
            1.0 - (self.complex_keyword_density - other.complex_keyword_density).abs().min(1.0);
        score += complex_sim * 0.15;
        weight += 0.15;

        let trivial_sim =
            1.0 - (self.trivial_keyword_density - other.trivial_keyword_density).abs().min(1.0);
        score += trivial_sim * 0.10;
        weight += 0.10;

        // File path match
        if self.has_file_paths == other.has_file_paths {
            score += 0.10;
        }
        weight += 0.10;

        // Multi-task match
        if self.is_multi_task == other.is_multi_task {
            score += 0.15;
        }
        weight += 0.15;

        if weight > 0.0 { score / weight } else { 1.0 }
    }
}

// ─── Route Decision ───

/// The router's output: which model to use + fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// The recommended model tier.
    pub tier: ModelTier,
    /// Minimum token budget for this task type.
    pub min_tokens: u32,
    /// Whether this prompt is a good candidate for semantic caching.
    pub cacheable: bool,
    /// Suggested TTL for cache (seconds), if cacheable.
    pub cache_ttl_secs: Option<u64>,
    /// Brief classification explanation for debugging.
    pub reason: String,
    /// Whether the decision was overridden by ML (vs heuristic).
    pub ml_override: bool,
    /// Confidence score (0.0-1.0) for the ML override.
    pub ml_confidence: Option<f32>,
    /// Estimated cost range (USD) for this tier.
    pub estimated_cost_usd: Option<CostEstimate>,
    /// Feature vector for feedback learning.
    #[serde(skip)]
    pub features: Option<TaskFeatureVector>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelTier {
    /// Budget models (haiku, flash, gpt-5.4-mini, deepseek-v4-flash)
    Budget,
    /// Balanced models (sonnet, gpt-5.4, gemini-pro)
    Balanced,
    /// Premium models (opus, gpt-5.5, o4-mini)
    Premium,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::Budget => "budget",
            ModelTier::Balanced => "balanced",
            ModelTier::Premium => "premium",
        }
    }

    /// Estimated cost per 1K tokens (input + output average).
    pub fn cost_per_1k_tokens(&self) -> f64 {
        match self {
            ModelTier::Budget => 0.0003,
            ModelTier::Balanced => 0.003,
            ModelTier::Premium => 0.015,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub min_usd: f64,
    pub max_usd: f64,
    pub tier: String,
}

// ─── Route History & Feedback ───

/// Outcome of a routing decision, reported by the frontend after LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOutcome {
    /// Whether the response was satisfactory.
    pub success: bool,
    /// User quality rating (0.0-1.0), if available.
    pub quality_score: Option<f32>,
    /// Whether the user manually switched to a different tier.
    pub user_override: bool,
    /// The tier the user switched to, if override.
    pub user_tier: Option<ModelTier>,
    /// Actual latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Actual token usage.
    pub tokens_used: Option<u32>,
    /// Actual cost in USD.
    pub cost_usd: Option<f64>,
}

/// Historical record of a routing decision and its outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHistoryEntry {
    /// Hash of the prompt for deduplication.
    pub prompt_hash: String,
    /// First 200 chars of the prompt for debugging.
    pub prompt_preview: String,
    /// The tier recommended by the heuristic.
    pub heuristic_tier: ModelTier,
    /// The tier actually selected (may differ from heuristic if ML overrode).
    pub selected_tier: ModelTier,
    /// The outcome of the LLM call.
    pub outcome: Option<RouteOutcome>,
    /// Unix timestamp.
    pub timestamp: i64,
    /// Feature vector for similarity matching.
    pub features: Option<TaskFeatureVector>,
}

/// Aggregate statistics for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStats {
    pub total_routes: u64,
    pub total_feedback: u64,
    pub tier_distribution: HashMap<String, u64>,
    pub success_rate_by_tier: HashMap<String, f64>,
    pub avg_latency_by_tier: HashMap<String, f64>,
    pub avg_cost_by_tier: HashMap<String, f64>,
    pub ml_override_count: u64,
    pub ml_override_success_rate: f64,
    pub user_override_count: u64,
    pub cost_saved_usd: f64,
}

// ─── ML Cost-Aware Router State ───

/// Per-tier statistics used for ML-based routing decisions.
#[derive(Debug, Clone, Default)]
struct TierStats {
    success_count: u64,
    failure_count: u64,
    total_latency_ms: u64,
    total_cost_usd: f64,
    sample_count: u64,
}

impl TierStats {
    fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.5; // Neutral prior
        }
        self.success_count as f64 / total as f64
    }

    fn avg_latency_ms(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.sample_count as f64
    }

    fn avg_cost_usd(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.0;
        }
        self.total_cost_usd / self.sample_count as f64
    }

    fn confidence(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total < 10 {
            return 0.0; // Not enough data
        }
        // Wilson score interval lower bound approximation
        let n = total as f64;
        let p = self.success_rate();
        let z = 1.96; // 95% confidence
        let denominator = 1.0 + z * z / n;
        let center = (p + z * z / (2.0 * n)) / denominator;
        let margin = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / denominator;
        (center - margin).max(0.0)
    }
}

/// The ML cost-aware router. Wraps the heuristic classifier with
/// historical data-driven optimization.
pub struct CostAwareRouter {
    /// Global tier stats (aggregated across all task types).
    global_stats: Mutex<HashMap<ModelTier, TierStats>>,
    /// Per-bucket stats (bucketed by feature vector similarity).
    bucket_stats: Mutex<HashMap<String, HashMap<ModelTier, TierStats>>>,
    /// Route history for analysis.
    history: Mutex<Vec<RouteHistoryEntry>>,
    /// Total cost saved by ML overrides.
    cost_saved_usd: AtomicU64,
    /// ML override count.
    ml_override_count: AtomicU64,
    /// ML override success count.
    ml_override_success: AtomicU64,
    /// Minimum samples before ML kicks in.
    min_samples_for_ml: u64,
    /// Minimum confidence before ML overrides heuristic.
    min_confidence: f64,
    /// Cost budget limit (USD), 0 = no limit.
    cost_budget_limit_usd: AtomicU64,
    /// Total cost spent so far.
    total_cost_spent: AtomicU64,
    /// 可选的数据库连接，用于持久化路由历史与统计。
    /// 为 None 时退化为纯内存模式（向后兼容）。
    db: Option<Arc<sea_orm::DatabaseConnection>>,
}

impl CostAwareRouter {
    pub fn new() -> Self {
        Self {
            global_stats: Mutex::new(HashMap::new()),
            bucket_stats: Mutex::new(HashMap::new()),
            history: Mutex::new(Vec::new()),
            cost_saved_usd: AtomicU64::new(0),
            ml_override_count: AtomicU64::new(0),
            ml_override_success: AtomicU64::new(0),
            min_samples_for_ml: 10,
            min_confidence: 0.6,
            cost_budget_limit_usd: AtomicU64::new(0),
            total_cost_spent: AtomicU64::new(0),
            db: None,
        }
    }

    /// 创建带数据库连接的实例，用于持久化路由历史与统计。
    /// 调用方应在构造后立即调用 `load_from_db()` 恢复历史状态。
    pub fn with_db(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        let mut router = Self::new();
        router.db = Some(db);
        router
    }

    /// 从数据库加载全部路由历史，重建内存中的 history / 统计 / 原子计数器。
    ///
    /// 启动时调用一次。无 DB 连接时静默返回 Ok(())。
    /// 加载策略：按时间倒序取最近 10000 条，聚合重建全部内存状态。
    pub async fn load_from_db(&self) -> Result<(), String> {
        let Some(ref db) = self.db else {
            return Ok(());
        };

        let entries = Self::load_all_history(db).await?;

        // 重建统计用的临时累加器
        let mut global: HashMap<ModelTier, TierStats> = HashMap::new();
        let mut buckets: HashMap<String, HashMap<ModelTier, TierStats>> = HashMap::new();
        let mut ml_override_count: u64 = 0;
        let mut ml_override_success: u64 = 0;
        let mut cost_saved_micro: u64 = 0;
        let mut total_cost_micro: u64 = 0;

        for entry in &entries {
            // 只有有 outcome 的记录才贡献统计
            if let Some(outcome) = &entry.outcome {
                let cost_usd = outcome.cost_usd.unwrap_or(0.0);
                let latency_ms = outcome.latency_ms.unwrap_or(0);

                // global stats
                let stats = global.entry(entry.selected_tier).or_default();
                stats.sample_count += 1;
                stats.total_latency_ms += latency_ms;
                stats.total_cost_usd += cost_usd;
                if outcome.success {
                    stats.success_count += 1;
                } else {
                    stats.failure_count += 1;
                }

                // bucket stats
                if let Some(features) = &entry.features {
                    let bucket = self.compute_bucket(features);
                    let stats =
                        buckets.entry(bucket).or_default().entry(entry.selected_tier).or_default();
                    stats.sample_count += 1;
                    stats.total_latency_ms += latency_ms;
                    stats.total_cost_usd += cost_usd;
                    if outcome.success {
                        stats.success_count += 1;
                    } else {
                        stats.failure_count += 1;
                    }
                }

                // ML override 计数（selected != heuristic 视为 ML 覆盖）
                let was_ml_override = entry.selected_tier != entry.heuristic_tier;
                if was_ml_override {
                    ml_override_count += 1;
                    if outcome.success {
                        ml_override_success += 1;
                    }
                }

                // cost_saved：用户降级且成功时累计节省
                if outcome.user_override
                    && let Some(user_tier) = outcome.user_tier
                {
                    if user_tier.cost_per_1k_tokens() < entry.selected_tier.cost_per_1k_tokens()
                        && outcome.success
                    {
                        let saved = (entry.selected_tier.cost_per_1k_tokens()
                            - user_tier.cost_per_1k_tokens())
                            * outcome.tokens_used.unwrap_or(0) as f64
                            / 1000.0;
                        cost_saved_micro += (saved * 1_000_000.0) as u64;
                    }
                }

                // total cost
                total_cost_micro += (cost_usd * 1_000_000.0) as u64;
            }
        }

        // 写入内存状态（lock 后批量替换，不跨 await 持有锁）
        {
            let mut history = self.history.lock().unwrap();
            *history = entries;
        }
        {
            let mut global_lock = self.global_stats.lock().unwrap();
            *global_lock = global;
        }
        {
            let mut buckets_lock = self.bucket_stats.lock().unwrap();
            *buckets_lock = buckets;
        }

        // 重建原子计数器
        self.ml_override_count.store(ml_override_count, Ordering::Relaxed);
        self.ml_override_success.store(ml_override_success, Ordering::Relaxed);
        self.cost_saved_usd.store(cost_saved_micro, Ordering::Relaxed);
        self.total_cost_spent.store(total_cost_micro, Ordering::Relaxed);

        Ok(())
    }

    /// Main routing function: heuristic + ML optimization.
    pub fn route(&self, prompt: &str) -> RouteDecision {
        let features = TaskFeatureVector::from_prompt(prompt);
        let heuristic = classify_and_route(prompt);

        // Try ML override
        if let Some(ml_decision) = self.try_ml_override(&features, &heuristic) {
            return ml_decision;
        }

        // Apply cost budget enforcement
        if let Some(budget_decision) = self.try_budget_enforcement(&heuristic) {
            return budget_decision;
        }

        RouteDecision {
            features: Some(features),
            ml_override: false,
            ml_confidence: None,
            estimated_cost_usd: Some(CostEstimate {
                min_usd: heuristic.tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 1000.0
                    * 0.5,
                max_usd: heuristic.tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 1000.0
                    * 1.5,
                tier: heuristic.tier.as_str().to_string(),
            }),
            tier: heuristic.tier,
            min_tokens: heuristic.min_tokens,
            cacheable: heuristic.cacheable,
            cache_ttl_secs: heuristic.cache_ttl_secs,
            reason: heuristic.reason,
        }
    }

    /// Attempt to override the heuristic decision using ML.
    fn try_ml_override(
        &self,
        features: &TaskFeatureVector,
        heuristic: &RouteDecision,
    ) -> Option<RouteDecision> {
        let bucket = self.compute_bucket(features);
        let stats = self.bucket_stats.lock().unwrap();
        let bucket_data = stats.get(&bucket)?;

        // Check if we have enough data for any tier
        let total_samples: u64 = bucket_data.values().map(|s| s.sample_count).sum();
        if total_samples < self.min_samples_for_ml {
            return None;
        }

        // Find the best tier for this bucket
        let heuristic_tier = heuristic.tier;
        let heuristic_score = bucket_data
            .get(&heuristic_tier)
            .map(|s| (s.success_rate(), s.avg_cost_usd()))
            .unwrap_or((0.5, heuristic_tier.cost_per_1k_tokens()));

        let mut best_tier = heuristic_tier;
        let mut best_score = self.compute_tier_score(heuristic_score.0, heuristic_score.1);

        for (tier, tier_stats) in bucket_data {
            if tier_stats.confidence() < self.min_confidence {
                continue;
            }
            let score =
                self.compute_tier_score(tier_stats.success_rate(), tier_stats.avg_cost_usd());
            if score > best_score {
                best_score = score;
                best_tier = *tier;
            }
        }

        if best_tier == heuristic_tier {
            return None; // Heuristic was right
        }

        let reason = if best_tier == ModelTier::Premium {
            format!(
                "ML upgrade: heuristic={}, but similar tasks have low success rate with {} ({}%)",
                heuristic_tier.as_str(),
                heuristic_tier.as_str(),
                (heuristic_score.0 * 100.0) as u32
            )
        } else {
            format!(
                "ML downgrade: heuristic={}, but {} achieves {:.0}% success at lower cost",
                heuristic_tier.as_str(),
                best_tier.as_str(),
                (bucket_data.get(&best_tier).map(|s| s.success_rate() * 100.0).unwrap_or(0.0))
                    as u32
            )
        };

        let confidence = bucket_data.get(&best_tier).map(|s| s.confidence()).unwrap_or(0.0);

        Some(RouteDecision {
            tier: best_tier,
            min_tokens: heuristic.min_tokens,
            cacheable: heuristic.cacheable && best_tier == ModelTier::Budget,
            cache_ttl_secs: if best_tier == ModelTier::Budget {
                Some(3600)
            } else {
                None
            },
            reason,
            ml_override: true,
            ml_confidence: Some(confidence as f32),
            estimated_cost_usd: Some(CostEstimate {
                min_usd: best_tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 1000.0
                    * 0.5,
                max_usd: best_tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 1000.0
                    * 1.5,
                tier: best_tier.as_str().to_string(),
            }),
            features: Some(features.clone()),
        })
    }

    /// Enforce cost budget by downgrading tier if needed.
    fn try_budget_enforcement(&self, heuristic: &RouteDecision) -> Option<RouteDecision> {
        let limit = self.cost_budget_limit_usd.load(Ordering::Relaxed);
        if limit == 0 {
            return None;
        }

        let limit_f64 = f64::from_bits(limit);
        let spent = f64::from_bits(self.total_cost_spent.load(Ordering::Relaxed));
        let estimated = heuristic.tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 1000.0;

        if spent + estimated <= limit_f64 {
            return None;
        }

        // Try downgrading
        let downgrade = match heuristic.tier {
            ModelTier::Premium => Some(ModelTier::Balanced),
            ModelTier::Balanced => Some(ModelTier::Budget),
            ModelTier::Budget => None,
        };

        downgrade.map(|tier| RouteDecision {
            tier,
            min_tokens: heuristic.min_tokens / 2,
            cacheable: true,
            cache_ttl_secs: Some(1800),
            reason: format!(
                "budget enforcement: spent ${:.4} of ${:.4} limit, downgraded from {}",
                spent,
                limit_f64,
                heuristic.tier.as_str()
            ),
            ml_override: true,
            ml_confidence: Some(1.0),
            estimated_cost_usd: Some(CostEstimate {
                min_usd: tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 2000.0 * 0.5,
                max_usd: tier.cost_per_1k_tokens() * heuristic.min_tokens as f64 / 2000.0 * 1.5,
                tier: tier.as_str().to_string(),
            }),
            features: heuristic.features.clone(),
        })
    }

    /// Record a routing decision (before LLM call).
    ///
    /// 内存 push 后异步写入 DB（如已配置 db）。写入失败仅记录警告，
    /// 不影响路由决策主流程。
    pub fn record_decision(&self, entry: RouteHistoryEntry) {
        let entry_for_db = entry.clone();
        self.history.lock().unwrap().push(entry);

        if let Some(ref db) = self.db {
            let db = db.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::insert_route_history(&db, &entry_for_db).await {
                    tracing::warn!("Smart Router: 写入 route_history 失败: {}", e);
                }
            });
        }
    }

    /// Record feedback after LLM call. Updates ML statistics.
    pub fn record_feedback(&self, prompt_hash: &str, outcome: RouteOutcome) -> Option<RouteStats> {
        let mut history = self.history.lock().unwrap();

        // Find the entry and update it
        let entry = history.iter_mut().find(|e| e.prompt_hash == prompt_hash)?;
        let selected_tier = entry.selected_tier;
        let was_ml_override = entry.selected_tier != entry.heuristic_tier;
        let features = entry.features.clone();

        let cost_usd = outcome.cost_usd.unwrap_or(0.0);
        let latency_ms = outcome.latency_ms.unwrap_or(0);

        entry.outcome = Some(outcome.clone());

        // Update global stats
        {
            let mut global = self.global_stats.lock().unwrap();
            let stats = global.entry(selected_tier).or_default();
            stats.sample_count += 1;
            stats.total_latency_ms += latency_ms;
            stats.total_cost_usd += cost_usd;
            if outcome.success {
                stats.success_count += 1;
            } else {
                stats.failure_count += 1;
            }
        }

        // Update bucket stats
        if let Some(features) = &features {
            let bucket = self.compute_bucket(features);
            let mut buckets = self.bucket_stats.lock().unwrap();
            let stats = buckets.entry(bucket).or_default().entry(selected_tier).or_default();
            stats.sample_count += 1;
            stats.total_latency_ms += latency_ms;
            stats.total_cost_usd += cost_usd;
            if outcome.success {
                stats.success_count += 1;
            } else {
                stats.failure_count += 1;
            }
        }

        // Track ML override success
        if was_ml_override {
            self.ml_override_count.fetch_add(1, Ordering::Relaxed);
            self.ml_override_success
                .fetch_add(if outcome.success { 1 } else { 0 }, Ordering::Relaxed);
        }

        // Track user override
        if outcome.user_override {
            if let Some(user_tier) = outcome.user_tier {
                // If user downgraded and succeeded, we could have saved cost
                if user_tier.cost_per_1k_tokens() < selected_tier.cost_per_1k_tokens()
                    && outcome.success
                {
                    let saved = (selected_tier.cost_per_1k_tokens()
                        - user_tier.cost_per_1k_tokens())
                        * outcome.tokens_used.unwrap_or(0) as f64
                        / 1000.0;
                    self.cost_saved_usd.fetch_add((saved * 1_000_000.0) as u64, Ordering::Relaxed);
                }
            }
        }

        // Update total cost
        self.total_cost_spent.fetch_add((cost_usd * 1_000_000.0) as u64, Ordering::Relaxed);

        // 准备异步 DB 回写（在 drop history 锁之前取数据，之后 spawn）
        let db_for_update = self.db.clone();
        let prompt_hash_owned = prompt_hash.to_string();

        // Return updated stats
        drop(history);

        // 异步更新 DB 中对应记录的 outcome 字段
        if let Some(db) = db_for_update {
            tokio::spawn(async move {
                if let Err(e) = Self::update_route_outcome(&db, &prompt_hash_owned, &outcome).await
                {
                    tracing::warn!("Smart Router: 更新 route_history outcome 失败: {}", e);
                }
            });
        }

        Some(self.compute_stats())
    }

    /// Compute aggregate routing statistics.
    pub fn compute_stats(&self) -> RouteStats {
        let history = self.history.lock().unwrap();
        let global = self.global_stats.lock().unwrap();

        let total_routes = history.len() as u64;
        let total_feedback = history.iter().filter(|e| e.outcome.is_some()).count() as u64;

        let mut tier_distribution: HashMap<String, u64> = HashMap::new();
        let mut success_rate_by_tier: HashMap<String, f64> = HashMap::new();
        let mut avg_latency_by_tier: HashMap<String, f64> = HashMap::new();
        let mut avg_cost_by_tier: HashMap<String, f64> = HashMap::new();

        for (tier, stats) in global.iter() {
            let key = tier.as_str().to_string();
            tier_distribution.insert(key.clone(), stats.sample_count);
            if stats.sample_count > 0 {
                success_rate_by_tier.insert(key.clone(), stats.success_rate());
                avg_latency_by_tier.insert(key.clone(), stats.avg_latency_ms());
                avg_cost_by_tier.insert(key.clone(), stats.avg_cost_usd());
            }
        }

        let ml_override_count = self.ml_override_count.load(Ordering::Relaxed);
        let ml_override_success = self.ml_override_success.load(Ordering::Relaxed);
        let ml_override_success_rate = if ml_override_count > 0 {
            ml_override_success as f64 / ml_override_count as f64
        } else {
            0.0
        };

        let user_override_count = history
            .iter()
            .filter(|e| e.outcome.as_ref().map(|o| o.user_override).unwrap_or(false))
            .count() as u64;

        let cost_saved = f64::from_bits(self.cost_saved_usd.load(Ordering::Relaxed)) / 1_000_000.0;

        RouteStats {
            total_routes,
            total_feedback,
            tier_distribution,
            success_rate_by_tier,
            avg_latency_by_tier,
            avg_cost_by_tier,
            ml_override_count,
            ml_override_success_rate,
            user_override_count,
            cost_saved_usd: cost_saved,
        }
    }

    /// Set cost budget limit.
    pub fn set_cost_budget(&self, limit_usd: f64) {
        self.cost_budget_limit_usd.store(limit_usd.to_bits(), Ordering::Relaxed);
    }

    /// Get current cost budget limit.
    pub fn get_cost_budget(&self) -> f64 {
        f64::from_bits(self.cost_budget_limit_usd.load(Ordering::Relaxed))
    }

    /// Get total cost spent.
    pub fn get_total_cost(&self) -> f64 {
        f64::from_bits(self.total_cost_spent.load(Ordering::Relaxed)) / 1_000_000.0
    }

    /// Compute a bucket key from feature vector for grouping similar tasks.
    fn compute_bucket(&self, features: &TaskFeatureVector) -> String {
        // Bucket by length category + code presence + SQL presence + multi-task
        let len_cat = match features.prompt_len {
            0..=100 => "xs",
            101..=500 => "sm",
            501..=2000 => "md",
            _ => "lg",
        };
        let code = if features.code_block_count > 0 {
            "code"
        } else {
            "nocode"
        };
        let sql = if features.has_sql { "sql" } else { "nosql" };
        let multi = if features.is_multi_task {
            "multi"
        } else {
            "single"
        };
        format!("{}-{}-{}-{}", len_cat, code, sql, multi)
    }

    /// Compute a composite score for a tier (higher = better).
    /// Balances success rate (weight 0.6) vs cost efficiency (weight 0.4).
    fn compute_tier_score(&self, success_rate: f64, avg_cost: f64) -> f64 {
        let cost_score = if avg_cost > 0.0 {
            (1.0 / (avg_cost + 0.0001)).min(100.0) / 100.0
        } else {
            1.0
        };
        success_rate * 0.6 + cost_score * 0.4
    }

    // ─── DB 持久化辅助方法 ───

    /// 插入一条路由决策记录到 DB（决策时尚无 outcome）。
    async fn insert_route_history(
        db: &sea_orm::DatabaseConnection,
        entry: &RouteHistoryEntry,
    ) -> Result<(), String> {
        use axagent_entities::route_history::{ActiveModel, Column, Entity};
        use sea_orm::{
            ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
        };

        // 生成 UUID 作为主键
        let id = uuid::Uuid::new_v4().to_string();

        // 序列化 features（如有）
        let features_json =
            entry.features.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default());

        // 检查是否已存在同 prompt_hash 的记录（去重，避免重复插入）
        let existing = Entity::find()
            .filter(Column::PromptHash.eq(&entry.prompt_hash))
            .order_by_desc(Column::Timestamp)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        if existing.is_some() {
            // 已存在同 hash 记录，跳过插入（避免重复）
            return Ok(());
        }

        let active = ActiveModel {
            id: ActiveValue::set(id),
            prompt_hash: ActiveValue::set(entry.prompt_hash.clone()),
            prompt_preview: ActiveValue::set(entry.prompt_preview.clone()),
            heuristic_tier: ActiveValue::set(entry.heuristic_tier.as_str().to_string()),
            selected_tier: ActiveValue::set(entry.selected_tier.as_str().to_string()),
            outcome_success: ActiveValue::set(None),
            outcome_quality_score: ActiveValue::set(None),
            outcome_user_override: ActiveValue::set(None),
            outcome_user_tier: ActiveValue::set(None),
            outcome_latency_ms: ActiveValue::set(None),
            outcome_tokens_used: ActiveValue::set(None),
            outcome_cost_usd: ActiveValue::set(None),
            timestamp: ActiveValue::set(entry.timestamp),
            features_json: ActiveValue::set(features_json),
        };

        active.insert(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 按 prompt_hash 更新对应记录的 outcome 字段。
    /// 找不到记录时静默返回 Ok(())（可能记录尚未写入或已过期）。
    async fn update_route_outcome(
        db: &sea_orm::DatabaseConnection,
        prompt_hash: &str,
        outcome: &RouteOutcome,
    ) -> Result<(), String> {
        use axagent_entities::route_history::{ActiveModel, Column, Entity};
        use sea_orm::{
            ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
        };

        // 找到对应 prompt_hash 的最新记录
        let model = Entity::find()
            .filter(Column::PromptHash.eq(prompt_hash))
            .order_by_desc(Column::Timestamp)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        let Some(model) = model else {
            return Ok(()); // 记录不存在，静默跳过
        };

        let user_tier_str = outcome.user_tier.as_ref().map(|t| t.as_str().to_string());

        let mut active: ActiveModel = model.into();
        active.outcome_success = ActiveValue::set(Some(outcome.success));
        active.outcome_quality_score = ActiveValue::set(outcome.quality_score.map(|v| v as f64));
        active.outcome_user_override = ActiveValue::set(Some(outcome.user_override));
        active.outcome_user_tier = ActiveValue::set(user_tier_str);
        active.outcome_latency_ms = ActiveValue::set(outcome.latency_ms.map(|v| v as i64));
        active.outcome_tokens_used = ActiveValue::set(outcome.tokens_used.map(|v| v as i64));
        active.outcome_cost_usd = ActiveValue::set(outcome.cost_usd);

        active.update(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 从 DB 加载全部路由历史（按时间倒序，最多 10000 条）。
    async fn load_all_history(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Vec<RouteHistoryEntry>, String> {
        use axagent_entities::route_history::{Column, Entity};
        use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

        let models = Entity::find()
            .order_by_desc(Column::Timestamp)
            .limit(10000)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let mut entries = Vec::with_capacity(models.len());
        for m in models {
            let heuristic_tier = parse_model_tier(&m.heuristic_tier);
            let selected_tier = parse_model_tier(&m.selected_tier);
            let features = m
                .features_json
                .as_ref()
                .and_then(|s| serde_json::from_str::<TaskFeatureVector>(s).ok());

            // outcome_success 为 None 表示尚无反馈
            let outcome = m.outcome_success.map(|success| RouteOutcome {
                success,
                quality_score: m.outcome_quality_score.map(|v| v as f32),
                user_override: m.outcome_user_override.unwrap_or(false),
                user_tier: m.outcome_user_tier.as_deref().map(parse_model_tier),
                latency_ms: m.outcome_latency_ms.map(|v| v as u64),
                tokens_used: m.outcome_tokens_used.map(|v| v as u32),
                cost_usd: m.outcome_cost_usd,
            });

            entries.push(RouteHistoryEntry {
                prompt_hash: m.prompt_hash,
                prompt_preview: m.prompt_preview,
                heuristic_tier,
                selected_tier,
                outcome,
                timestamp: m.timestamp,
                features,
            });
        }
        Ok(entries)
    }
}

/// 把字符串解析回 ModelTier。未知值默认 Balanced（保守选择）。
fn parse_model_tier(s: &str) -> ModelTier {
    match s {
        "budget" => ModelTier::Budget,
        "balanced" => ModelTier::Balanced,
        "premium" => ModelTier::Premium,
        _ => ModelTier::Balanced,
    }
}

impl Default for CostAwareRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Keywords for Feature Extraction ───

const COMPLEX_KEYWORDS: &[&str] = &[
    "architect",
    "design pattern",
    "refactor",
    "refactoring",
    "system design",
    "multi-step",
    "step by step reasoning",
    "debug",
    "troubleshoot",
    "root cause",
    "performance optimization",
    "security audit",
    "migrate from",
    "implement a",
    "build a",
    "create a full",
    "scalable",
    "production-ready",
    "enterprise",
    "microservices",
    "distributed system",
    "concurrency",
    "race condition",
    "deadlock",
    "memory leak",
    "scale horizontally",
];

const TRIVIAL_KEYWORDS: &[&str] = &[
    "translate",
    "翻译",
    "summarize",
    "summarise",
    "tldr",
    "tl;dr",
    "总结",
    "摘要",
    "概括",
    "convert to json",
    "convert to yaml",
    "convert to csv",
    "format as",
    "reformat",
    "pretty print",
    "what is",
    "who is",
    "when did",
    "where is",
    "是什么",
    "什么是",
    "怎么",
    "如何",
    "列出",
];

// ─── Task Classification (Heuristic Layer) ───

/// Classify a user prompt and return a routing decision.
///
/// Uses fast, local heuristics (no LLM call required). The classifier
/// examines prompt length, keywords, structural patterns, and code presence.
pub fn classify_and_route(prompt: &str) -> RouteDecision {
    let lower = prompt.to_lowercase();
    let prompt_len = prompt.len();
    let line_count = prompt.lines().count();

    // ── Complex indicators ──
    if is_complex_task(&lower, prompt_len, line_count) {
        return RouteDecision {
            tier: ModelTier::Premium,
            min_tokens: 4096,
            cacheable: false,
            cache_ttl_secs: None,
            reason: "complex task: multi-step reasoning, architecture, or debugging".into(),
            ml_override: false,
            ml_confidence: None,
            estimated_cost_usd: None,
            features: None,
        };
    }

    // ── Trivial indicators ──
    if is_trivial_task(&lower, prompt_len) {
        return RouteDecision {
            tier: ModelTier::Budget,
            min_tokens: 512,
            cacheable: true,
            cache_ttl_secs: Some(3600),
            reason: "trivial task: translation, summary, or format conversion".into(),
            ml_override: false,
            ml_confidence: None,
            estimated_cost_usd: None,
            features: None,
        };
    }

    // ── Moderate (default) ──
    RouteDecision {
        tier: ModelTier::Balanced,
        min_tokens: 2048,
        cacheable: false,
        cache_ttl_secs: None,
        reason: "moderate task: general Q&A or code explanation".into(),
        ml_override: false,
        ml_confidence: None,
        estimated_cost_usd: None,
        features: None,
    }
}

// ─── Classification Helpers ───

fn is_complex_task(lower: &str, prompt_len: usize, line_count: usize) -> bool {
    if prompt_len > 2000 || line_count > 20 {
        return true;
    }

    let complex_keywords = [
        "architect",
        "design pattern",
        "refactor",
        "refactoring",
        "system design",
        "multi-step",
        "step by step reasoning",
        "debug",
        "troubleshoot",
        "root cause",
        "performance optimization",
        "security audit",
        "code review the entire",
        "migrate from",
        "implement a",
        "build a",
        "create a full",
        "scalable",
        "production-ready",
        "enterprise",
        "microservices",
        "distributed system",
        "concurrency",
        "race condition",
        "deadlock",
        "memory leak",
        "scale horizontally",
    ];

    for kw in &complex_keywords {
        if lower.contains(kw) {
            return true;
        }
    }

    let code_block_count = lower.matches("```").count() / 2;
    if code_block_count >= 3 {
        return true;
    }

    if lower.contains("sql") && (lower.contains("explain") || lower.contains("optimize")) {
        return true;
    }

    false
}

fn is_trivial_task(lower: &str, prompt_len: usize) -> bool {
    if prompt_len < 50 {
        return true;
    }

    let translation_patterns =
        ["translate", "翻译", "traduire", "übersetzen", "翻成", "译为", "翻訳"];
    for pat in &translation_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    let summary_patterns = [
        "summarize",
        "summarise",
        "tldr",
        "tl;dr",
        "总结",
        "摘要",
        "概括",
        "简述",
        "in a few words",
        "brief summary",
        "key points",
    ];
    for pat in &summary_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    let format_patterns = [
        "convert to json",
        "convert to yaml",
        "convert to csv",
        "format as",
        "reformat",
        "pretty print",
        "json to",
        "yaml to",
        "csv to",
    ];
    for pat in &format_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    if prompt_len < 100 && !lower.contains("explain") && !lower.contains("why") {
        let simple_patterns = [
            "what is",
            "who is",
            "when did",
            "where is",
            "how to",
            "list",
            "show me",
            "find",
            "是什么",
            "什么是",
            "怎么",
            "如何",
            "列出",
            "显示",
            "查找",
        ];
        for pat in &simple_patterns {
            if lower.starts_with(pat) || lower.contains(&format!(" {} ", pat)) {
                return true;
            }
        }
    }

    false
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivial_summary() {
        let decision = classify_and_route("Summarize the key points of this article");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_trivial_short() {
        let decision = classify_and_route("What is Rust?");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_moderate_question() {
        let decision =
            classify_and_route("Explain how async/await works in JavaScript with examples");
        assert_eq!(decision.tier, ModelTier::Balanced);
    }

    #[test]
    fn test_complex_architecture() {
        let decision = classify_and_route(
            "Design a microservices architecture for an e-commerce platform with \
             user authentication, product catalog, payment processing, and order tracking. \
             Include database schema and API design.",
        );
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_complex_refactor() {
        let decision =
            classify_and_route("Refactor this monolithic codebase into a modular architecture");
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_complex_long_prompt() {
        let mut long = String::from("I need help with a complex problem:\n");
        for i in 0..30 {
            long.push_str(&format!("Step {}: Do something complex here\n", i));
        }
        let decision = classify_and_route(&long);
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_format_conversion() {
        let decision = classify_and_route("Convert this JSON to YAML format");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_chinese_translation() {
        let decision = classify_and_route("把这段英文翻译成中文");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_chinese_what_is() {
        let decision = classify_and_route("什么是 Rust 的所有权系统？");
        assert_eq!(decision.tier, ModelTier::Budget);
    }
}
