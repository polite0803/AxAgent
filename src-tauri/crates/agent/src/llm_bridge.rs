// SPDX-License-Identifier: AGPL-3.0-only

use crate::provider_fallback::{
    ProviderEntry, ProviderFallbackManager,
};
#[cfg(test)]
use axagent_harness::trajectory_types::ProcedureStep;
use axagent_harness::trajectory_types::{
    GeneratedTool, LlmEvolutionProvider, LlmJudge, LlmJudgeFuture, LlmMutationRequest,
    LlmMutationResponse, LlmTextGradProvider, LlmToolProvider, PrmLlmProvider, RewardCategory,
    StepReward, ToolCreationRequest,
};
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

static SCORE_NUMBER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(-?\d+\.?\d*)").expect("hardcoded regex is valid"));
static CODE_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?s)```(?:javascript|js)?\s*\n(.*?)```").expect("hardcoded regex is valid")
});
static JSON_OBJECT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)\{.*\}").expect("hardcoded regex is valid"));

/// Deterministic LLM response cache.
///
/// Only caches `temperature = 0` calls (deterministic). Cache key is
/// `SHA-256(model || system_prompt || user_prompt)` — temperature is omitted
/// because non-zero temp calls are never cached.
///
/// Default TTL: 300 seconds (5 minutes). Eviction runs on each cache miss.
pub struct LlmResponseCache {
    inner: Mutex<HashMap<String, (String, Instant)>>,
    ttl: std::time::Duration,
    max_entries: usize,
}

impl LlmResponseCache {
    pub fn new(ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: std::time::Duration::from_secs(ttl_secs),
            max_entries,
        }
    }

    fn cache_key(model: &str, system: &str, user: &str) -> String {
        use std::fmt::Write as FmtWrite;
        let mut h = Sha256::new();
        h.update(model.as_bytes());
        h.update(b"||");
        h.update(system.as_bytes());
        h.update(b"||");
        h.update(user.as_bytes());
        let digest = h.finalize();
        let mut s = String::with_capacity(64);
        for byte in digest {
            write!(&mut s, "{:02x}", byte).unwrap();
        }
        s
    }

    /// Try to get a cached response. Returns `None` on miss or expiry.
    fn get(&self, model: &str, system: &str, user: &str) -> Option<String> {
        let key = Self::cache_key(model, system, user);
        let mut map = self.inner.lock().unwrap();
        // Evict expired entries on every get (amortized cleanup)
        let now = Instant::now();
        map.retain(|_, (_, expiry)| now < *expiry);
        map.get(&key)
            .and_then(|(val, expiry)| (now < *expiry).then(|| val.clone()))
    }

    /// Insert a cached response.
    fn put(&self, model: &str, system: &str, user: &str, response: &str) {
        let key = Self::cache_key(model, system, user);
        let mut map = self.inner.lock().unwrap();
        // Evict oldest entry if at capacity
        if map.len() >= self.max_entries
            && let Some(oldest) = map
                .iter()
                .min_by_key(|(_, (_, exp))| *exp)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest);
            }
        map.insert(key, (response.to_string(), Instant::now() + self.ttl));
    }
}

impl Default for LlmResponseCache {
    fn default() -> Self {
        Self::new(300, 256)
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_same_key() {
        let cache = LlmResponseCache::new(600, 10);
        cache.put("gpt-4", "You are helpful.", "Hello", "Hi there!");
        let result = cache.get("gpt-4", "You are helpful.", "Hello");
        assert_eq!(result, Some("Hi there!".to_string()));
    }

    #[test]
    fn cache_miss_different_model() {
        let cache = LlmResponseCache::new(600, 10);
        cache.put("gpt-4", "sys", "usr", "resp");
        assert!(cache.get("gpt-3.5", "sys", "usr").is_none());
    }

    #[test]
    fn cache_miss_different_prompt() {
        let cache = LlmResponseCache::new(600, 10);
        cache.put("gpt-4", "sys", "usr1", "resp");
        assert!(cache.get("gpt-4", "sys", "usr2").is_none());
    }

    #[test]
    fn cache_expiry() {
        let cache = LlmResponseCache::new(0, 10); // 0s TTL — expires immediately
        cache.put("gpt-4", "sys", "usr", "resp");
        // Any small delay should make it expire
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(cache.get("gpt-4", "sys", "usr").is_none());
    }

    #[test]
    fn cache_eviction_at_capacity() {
        let cache = LlmResponseCache::new(600, 2);
        cache.put("m", "s1", "u1", "r1");
        cache.put("m", "s2", "u2", "r2");
        cache.put("m", "s3", "u3", "r3"); // should evict oldest
        // At least one of the first two should be gone
        let hits = [
            cache.get("m", "s1", "u1").is_some(),
            cache.get("m", "s2", "u2").is_some(),
            cache.get("m", "s3", "u3").is_some(),
        ];
        assert!(hits.iter().filter(|&&x| x).count() <= 2);
        assert!(cache.get("m", "s3", "u3").is_some());
    }
}

#[derive(Clone)]
pub struct ProviderLlmBridge {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
    /// Provider fallback 管理器 + 适配器池（可选）
    fallback_mgr: Option<Arc<ProviderFallbackManager>>,
    adapter_pool: Arc<Vec<Arc<dyn ProviderAdapter>>>,
    preferred_provider_id: Option<String>,
    /// Deterministic LLM response cache (temperature=0 only)
    llm_cache: Option<Arc<LlmResponseCache>>,
}

impl ProviderLlmBridge {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: impl Into<String>,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model: model.into(),
            fallback_mgr: None,
            adapter_pool: Arc::new(Vec::new()),
            preferred_provider_id: None,
            llm_cache: None,
        }
    }

    /// 创建带 fallback 支持的桥接实例。
    ///
    /// - `fallback_mgr`: ProviderFallbackManager 实例
    /// - `adapter_pool`: 所有可用适配器列表（索引与 ProviderEntry.adapter_index 对应）
    /// - `preferred_provider_id`: 首选 Provider ID（需与 manager 中注册的一致）
    pub fn new_with_fallback(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: impl Into<String>,
        fallback_mgr: Arc<ProviderFallbackManager>,
        adapter_pool: Vec<Arc<dyn ProviderAdapter>>,
        preferred_provider_id: String,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model: model.into(),
            fallback_mgr: Some(fallback_mgr),
            adapter_pool: Arc::new(adapter_pool),
            preferred_provider_id: Some(preferred_provider_id),
            llm_cache: None,
        }
    }

    /// 启用确定性 LLM 响应缓存（temperature=0 调用自动缓存）。
    ///
    /// TTL 建议 300-600s；max_entries 默认 256。
    pub fn with_cache(mut self, ttl_secs: u64, max_entries: usize) -> Self {
        self.llm_cache = Some(Arc::new(LlmResponseCache::new(ttl_secs, max_entries)));
        self
    }

    /// 注册一个 fallback Provider（在 new 之后动态添加）
    pub async fn register_fallback(&self, entry: ProviderEntry) {
        if let Some(ref mgr) = self.fallback_mgr {
            mgr.register(entry).await;
        }
    }

    /// 获取 fallback 管理器的引用（用于外部查询健康状态）
    pub fn fallback_manager(&self) -> Option<&Arc<ProviderFallbackManager>> {
        self.fallback_mgr.as_ref()
    }

    /// 核心 LLM 调用：优先用主适配器，失败时自动 fallback。
    pub async fn call_llm(&self, system: &str, user: &str) -> Result<String, String> {
        self.call_with_temp(system, user, 0.7, 2048).await
    }

    async fn call_llm_low_temp(&self, system: &str, user: &str) -> Result<String, String> {
        self.call_with_temp(system, user, 0.3, 64).await
    }

    /// 统一的 LLM 调用 + fallback 编排
    async fn call_with_temp(
        &self,
        system: &str,
        user: &str,
        temperature: f64,
        max_tokens: u32,
    ) -> Result<String, String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(system.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(user.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ],
            stream: false,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        // 确定性缓存：仅对 temperature=0 做缓存（确定性输出）
        // 缓存键 = SHA-256(model || system_prompt || user_prompt)
        if temperature == 0.0
            && let Some(ref cache) = self.llm_cache
                && let Some(cached) = cache.get(&self.model, system, user) {
                    tracing::debug!(
                        model = %self.model,
                        prompt_len = system.len() + user.len(),
                        "LLM response cache HIT (temp=0, deterministic)"
                    );
                    return Ok(cached);
                }

        // 第一步：尝试主适配器
        let start = Instant::now();
        match self.adapter.chat(&self.ctx, request.clone()).await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                // 记录成功
                if let Some(ref mgr) = self.fallback_mgr
                    && let Some(ref pref_id) = self.preferred_provider_id {
                        mgr.record_success(pref_id, latency).await;
                    }
                // 将温度 0 的结果写入缓存
                if temperature == 0.0
                    && let Some(ref cache) = self.llm_cache {
                        cache.put(&self.model, system, user, &resp.content);
                        tracing::debug!(
                            model = %self.model,
                            latency_ms = latency,
                            "LLM response cached (temp=0)"
                        );
                    }
                Ok(resp.content)
            },
            Err(primary_err) => {
                // 记录失败
                if let Some(ref mgr) = self.fallback_mgr
                    && let Some(ref pref_id) = self.preferred_provider_id {
                        mgr.record_failure(pref_id).await;
                    }
                // 第二步：检查是否有 fallback 管理器
                let mgr = match self.fallback_mgr.as_ref() {
                    Some(m) => m,
                    None => return Err(primary_err.to_string()),
                };
                // 第三步：选择备选 Provider
                let (fallback_entry, is_fallback) = match mgr
                    .select_provider(self.preferred_provider_id.as_deref())
                    .await
                {
                    Some((entry, is_fb)) => (entry, is_fb),
                    None => return Err(primary_err.to_string()),
                };
                if !is_fallback && fallback_entry.adapter_index == 0 {
                    // 选回主 Provider 但没有实际备选，返回原始错误
                    return Err(primary_err.to_string());
                }
                // 第四步：用备选适配器重试
                let fb_adapter = self.adapter_pool.get(fallback_entry.adapter_index).cloned();
                let fb_adapter = match fb_adapter {
                    Some(a) => a,
                    None => return Err(primary_err.to_string()),
                };
                let mut fb_request = request;
                fb_request.model = fallback_entry.model_id.clone();
                let fb_start = Instant::now();
                match fb_adapter.chat(&self.ctx, fb_request).await {
                    Ok(resp) => {
                        let latency = fb_start.elapsed().as_millis() as u64;
                        mgr.record_success(&fallback_entry.provider_id, latency)
                            .await;
                        tracing::warn!(
                            "Provider fallback: {} → {} (primary error: {})",
                            self.preferred_provider_id.as_deref().unwrap_or("unknown"),
                            fallback_entry.provider_id,
                            primary_err
                        );
                        Ok(resp.content)
                    },
                    Err(fb_err) => {
                        mgr.record_failure(&fallback_entry.provider_id).await;
                        Err(format!(
                            "Primary provider error: {}. Fallback provider ({}) also failed: {}",
                            primary_err, fallback_entry.provider_id, fb_err
                        ))
                    },
                }
            },
        }
    }
}

fn extract_score_from_text(text: &str) -> f64 {
    SCORE_NUMBER_RE
        .captures(text)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0)
}

fn heuristic_mutation(request: &LlmMutationRequest) -> LlmMutationResponse {
    let mut revised = request.current_steps.clone();
    if !request.failure_evidence.is_empty() {
        for step in &mut revised {
            if step.error_handling.is_none() {
                step.error_handling =
                    Some("If this step fails, retry with alternative approach".to_string());
            }
            step.condition = Some("Verify prerequisites before execution".to_string());
        }
    }
    LlmMutationResponse {
        revised_steps: revised,
        reasoning: "Heuristic fallback: added error handling and condition checks".to_string(),
        confidence: 0.5,
    }
}

fn extract_code_from_response(text: &str) -> String {
    if let Some(cap) = CODE_BLOCK_RE.captures(text)
        && let Some(m) = cap.get(1)
    {
        return m.as_str().trim().to_string();
    }
    text.trim().to_string()
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

impl LlmEvolutionProvider for ProviderLlmBridge {
    fn generate_mutation(
        &self,
        request: &LlmMutationRequest,
    ) -> axagent_harness::trajectory_types::LlmMutationFuture<'_> {
        let steps_json = serde_json::to_string(&request.current_steps).unwrap_or_default();
        let failures = request.failure_evidence.join("\n");
        let successes = request.success_evidence.join("\n");
        let user_msg = format!(
            "Skill: {}\n\nCurrent steps:\n{}\n\nFailure evidence:\n{}\n\nSuccess evidence:\n{}\n\n\
             Respond with a JSON object: {{\"revised_steps\": [...], \"reasoning\": \"...\", \"confidence\": 0.0-1.0}}",
            request.skill_name, steps_json, failures, successes
        );
        let fallback = heuristic_mutation(request);

        Box::pin(async move {
            match self.call_llm(
                "You are a skill evolution expert. Analyze the current skill steps and failure evidence to suggest improved steps.",
                &user_msg,
            ).await {
                Ok(text) => {
                    match serde_json::from_str::<LlmMutationResponse>(&text) {
                        Ok(resp) => Ok(resp),
                        Err(_) => {
                            let json_re = &*JSON_OBJECT_RE;
                            if let Some(cap) = json_re.captures(&text)
                                && let Some(m) = cap.get(0)
                                && let Ok(resp) = serde_json::from_str::<LlmMutationResponse>(m.as_str()) {
                                    return Ok(resp);
                                }
                            Ok(fallback)
                        }
                    }
                }
                Err(_) => Ok(fallback),
            }
        })
    }

    fn evaluate_quality(
        &self,
        content: &str,
        context: &str,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>> {
        let user_msg = format!(
            "Evaluate the quality of the following content on a scale from 0.0 to 1.0.\n\nContent:\n{}\n\nContext:\n{}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            content, context
        );

        Box::pin(async move {
            match self
                .call_llm_low_temp(
                    "You are a content quality evaluator. Score the quality from 0.0 to 1.0.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }
}

impl LlmJudge for ProviderLlmBridge {
    fn evaluate_reasoning(&self, reasoning: &str, context: &str) -> LlmJudgeFuture<'_> {
        let user_msg = format!(
            "Evaluate the reasoning quality on a scale from 0.0 to 1.0.\n\nReasoning:\n{}\n\nContext:\n{}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            reasoning, context
        );

        Box::pin(async move {
            match self.call_llm_low_temp(
                "You are a reasoning quality evaluator. Score the reasoning quality from 0.0 to 1.0.",
                &user_msg,
            ).await {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }

    fn evaluate_tool_efficiency(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
    ) -> LlmJudgeFuture<'_> {
        let user_msg = format!(
            "Evaluate the tool usage efficiency on a scale from 0.0 to 1.0.\n\nTool: {}\nArguments: {}\nResult: {}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            tool_name, args, result
        );

        Box::pin(async move {
            match self
                .call_llm_low_temp(
                    "You are a tool usage evaluator. Score the tool efficiency from 0.0 to 1.0.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }
}

impl LlmTextGradProvider for ProviderLlmBridge {
    fn compute_gradient(
        &self,
        node_content: &str,
        output_feedback: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
        let user_msg = format!(
            "Node content:\n{}\n\nOutput feedback:\n{}\n\nSuggest specific improvements to the node content based on the feedback.",
            node_content, output_feedback
        );

        Box::pin(async move {
            self.call_llm(
                "You are a text gradient optimizer. Given a node's content and output feedback, suggest specific improvements.",
                &user_msg,
            ).await
        })
    }
}

impl LlmToolProvider for ProviderLlmBridge {
    fn generate_tool_code(
        &self,
        request: &ToolCreationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let tool_list = request.available_tools.join(", ");
        let user_msg = format!(
            "Pattern: {}\nContext: {}\nAvailable tools: {}\n\nGenerate a JavaScript function that implements this pattern. Wrap the code in ```javascript``` code blocks.",
            request.pattern_description, request.context, tool_list
        );
        let name = slugify(&request.pattern_description);
        let description = request.pattern_description.clone();

        Box::pin(async move {
            match self.call_llm(
                "You are a tool code generator. Generate a JavaScript function that implements the described pattern.",
                &user_msg,
            ).await {
                Ok(text) => {
                    let code = extract_code_from_response(&text);
                    Ok(GeneratedTool::new(&name, &code, &description))
                }
                Err(e) => Err(e),
            }
        })
    }

    fn improve_tool_code(
        &self,
        tool: &GeneratedTool,
        error: &str,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let user_msg = format!(
            "Current code:\n```javascript\n{}\n```\n\nError:\n{}\n\nFix the errors and return the improved code wrapped in ```javascript``` code blocks.",
            tool.code, error
        );
        let name = tool.name.clone();
        let description = tool.description.clone();

        Box::pin(async move {
            match self
                .call_llm(
                    "You are a tool code improver. Fix the errors in the provided tool code.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => {
                    let code = extract_code_from_response(&text);
                    Ok(GeneratedTool::new(&name, &code, &description))
                },
                Err(e) => Err(e),
            }
        })
    }
}

impl PrmLlmProvider for ProviderLlmBridge {
    fn evaluate_step(
        &self,
        step_content: &str,
        context: &str,
        previous_steps: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<StepReward, String>> + Send + '_>> {
        let prev_summary = previous_steps
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        let user_msg = format!(
            "Step content:\n{}\n\nTask context:\n{}\n\nPrevious steps summary:\n{}\n\n\
             Evaluate this step on each dimension. Respond with JSON:\n\
             {{\"correctness\": 0.0-1.0, \"coherence\": 0.0-1.0, \"completeness\": 0.0-1.0, \"efficiency\": 0.0-1.0, \"safety\": 0.0-1.0, \"reasoning\": \"...\"}}",
            step_content, context, prev_summary
        );

        Box::pin(async move {
            match self.call_llm(
                "You are a process reward model evaluator. Score each dimension from 0.0 to 1.0.",
                &user_msg,
            ).await {
                Ok(text) => {
                    let json_re = &*JSON_OBJECT_RE;
                    if let Some(cap) = json_re.captures(&text)
                        && let Some(m) = cap.get(0)
                        && let Ok(v) = serde_json::from_str::<serde_json::Value>(m.as_str()) {
                            let correctness = v.get("correctness").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let coherence = v.get("coherence").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let completeness = v.get("completeness").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let efficiency = v.get("efficiency").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let safety = v.get("safety").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let reasoning = v.get("reasoning").and_then(|v| v.as_str()).unwrap_or("LLM evaluation").to_string();

                            let categories = vec![
                                (RewardCategory::Correctness, correctness.clamp(0.0, 1.0)),
                                (RewardCategory::Coherence, coherence.clamp(0.0, 1.0)),
                                (RewardCategory::Completeness, completeness.clamp(0.0, 1.0)),
                                (RewardCategory::Efficiency, efficiency.clamp(0.0, 1.0)),
                                (RewardCategory::Safety, safety.clamp(0.0, 1.0)),
                            ];
                            let reward: f64 = categories.iter().map(|(c, s)| c.weight() * s).sum();

                            return Ok(StepReward {
                                step_index: 0,
                                reward,
                                reasoning,
                                categories,
                            });
                        }
                    let score = extract_score_from_text(&text);
                    Ok(StepReward {
                        step_index: 0,
                        reward: score,
                        reasoning: format!("LLM PRM fallback score: {:.2}", score),
                        categories: vec![
                            (RewardCategory::Correctness, score),
                            (RewardCategory::Coherence, score),
                            (RewardCategory::Completeness, score),
                            (RewardCategory::Efficiency, score),
                            (RewardCategory::Safety, score),
                        ],
                    })
                }
                Err(e) => Err(e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_score_from_text_number_only() {
        assert!((extract_score_from_text("0.75") - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_with_context() {
        let score = extract_score_from_text("I would rate this 0.85 out of 1.0");
        assert!((score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_integer() {
        let score = extract_score_from_text("Score: 1");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_zero() {
        let score = extract_score_from_text("0.0");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_no_number() {
        let score = extract_score_from_text("no numbers here");
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_clamped_high() {
        let score = extract_score_from_text("9.5");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_clamped_negative() {
        let score = extract_score_from_text("-0.3");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_code_from_response_with_block() {
        let text = "Here is the code:\n```javascript\nfunction hello() { return 42; }\n```\nDone.";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function hello() { return 42; }");
    }

    #[test]
    fn test_extract_code_from_response_with_js_tag() {
        let text = "```\nfunction foo() {}\n```";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function foo() {}");
    }

    #[test]
    fn test_extract_code_from_response_no_block() {
        let text = "function bar() { return 1; }";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function bar() { return 1; }");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Search Files"), "search_files");
        assert_eq!(slugify("hello-world"), "hello_world");
        assert_eq!(slugify("  multiple   spaces  "), "multiple_spaces");
    }

    #[test]
    fn test_heuristic_mutation_with_failures() {
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec!["error occurred".to_string()],
            success_evidence: vec![],
        };
        let response = heuristic_mutation(&request);
        assert!(response.revised_steps[0].error_handling.is_some());
        assert!(response.revised_steps[0].condition.is_some());
        assert!(response.confidence < 0.6);
    }

    #[test]
    fn test_heuristic_mutation_no_failures() {
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec![],
            success_evidence: vec!["worked".to_string()],
        };
        let response = heuristic_mutation(&request);
        assert!(response.revised_steps[0].error_handling.is_none());
    }
}
