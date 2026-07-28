// SPDX-License-Identifier: AGPL-3.0-only

// 反思系统共享 DTO 已上移到 axagent-harness,本 crate 通过 pub use 复用。
pub use axagent_harness::reflection_types::{
    QualityMetrics, Reflection, ReflectionConfig, TaskExecutionRecord,
};

use crate::insight_generator::InsightGenerator;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Reflector {
    config: ReflectionConfig,
    insight_generator: Arc<InsightGenerator>,
    history: Arc<RwLock<Vec<Reflection>>>,
    /// 可选 JSONL 持久化路径(P0-3 修复:启用后每次 reflect 都 append 一行)。
    persist_path: Option<PathBuf>,
}

impl Reflector {
    pub fn new() -> Self {
        Self {
            config: ReflectionConfig::default(),
            insight_generator: Arc::new(InsightGenerator::new()),
            history: Arc::new(RwLock::new(Vec::new())),
            persist_path: None,
        }
    }

    pub fn with_config(mut self, config: ReflectionConfig) -> Self {
        self.config = config;
        self
    }

    /// 启用 JSONL 文件持久化(P0-3 修复)。
    ///
    /// 启用后:
    /// - `reflect()` 完成后异步 append 一行 JSON 到 `path`
    /// - `load_persistence()` 在启动时从 `path` 加载历史到内存
    ///
    /// 路径应为 `app_dir.join("reflections.jsonl")`,由 wiring 层提供。
    /// 文件不存在时首次 reflect 会自动创建。
    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persist_path = Some(path);
        self
    }

    /// 启动时从持久化文件加载历史反思到内存。
    ///
    /// 仅加载最后 `max_history` 条(避免内存膨胀)。文件不存在或损坏时返回空。
    pub async fn load_persistence(&self) -> std::io::Result<usize> {
        let path = match self.persist_path.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(0),
        };
        if !path.exists() {
            return Ok(0);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut loaded = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Reflection>(line) {
                Ok(r) => loaded.push(r),
                Err(e) => {
                    tracing::warn!(
                        "[reflector] skip malformed reflection line in {}: {}",
                        path.display(),
                        e
                    );
                },
            }
        }
        // 仅保留最后 max_history 条
        let max = self.config.max_history;
        if loaded.len() > max {
            loaded = loaded.split_off(loaded.len() - max);
        }
        let count = loaded.len();
        let mut history = self.history.write().await;
        // 合并到现有内存历史(去重 by task_id)
        for r in loaded {
            if !history.iter().any(|h| h.task_id == r.task_id) {
                history.push(r);
            }
            if history.len() >= max {
                history.remove(0);
            }
        }
        tracing::info!("[reflector] loaded {} reflections from {}", count, path.display());
        Ok(count)
    }

    /// 将单条反思 append 到 JSONL 文件。
    ///
    /// 失败仅记录日志,不阻塞 reflect 主流程(反思持久化是辅助数据,失败不应影响业务)。
    async fn persist_reflection(&self, reflection: &Reflection) {
        let path = match self.persist_path.as_ref() {
            Some(p) => p.clone(),
            None => return,
        };
        let line = match serde_json::to_string(reflection) {
            Ok(s) => s + "\n",
            Err(e) => {
                tracing::warn!("[reflector] serialize reflection failed: {}", e);
                return;
            },
        };
        // 文件 IO 走 spawn_blocking,避免污染 async runtime(AGENTS.md 工程惯例)。
        let result = tokio::task::spawn_blocking({
            let path = path.clone();
            let line = line.clone();
            move || {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                // 追加模式,文件不存在自动创建。
                use std::io::Write;
                let mut file =
                    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                        Ok(f) => f,
                        Err(e) => return Err(e),
                    };
                file.write_all(line.as_bytes())
            }
        })
        .await;
        match result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => {
                tracing::warn!("[reflector] append reflection to {} failed: {}", path.display(), e);
            },
            Err(e) => {
                tracing::warn!("[reflector] persist task join error: {}", e);
            },
        }
    }

    pub async fn reflect(&self, record: &TaskExecutionRecord) -> Reflection {
        let mut reflection = Reflection::new(record.task_id.clone());

        let metrics = self.calculate_quality_metrics(record);
        reflection.quality_score = (metrics.overall_weighted_score.round() as u8).clamp(1, 10);
        reflection.quality_analysis = self.analyze_quality(record, &metrics);
        reflection.quality_metrics = Some(metrics);

        reflection.efficiency_analysis = self.analyze_efficiency(record);

        let (errors, reusable) = self.analyze_patterns(record);
        reflection.error_patterns = errors;
        reflection.reusable_patterns = reusable;

        let metrics_ref =
            reflection.quality_metrics.as_ref().expect("quality_metrics was set above");
        reflection.knowledge_suggestions = self.generate_knowledge_suggestions(record, metrics_ref);
        reflection.improvement_suggestions =
            self.generate_improvement_suggestions(record, &reflection);

        reflection.overall_summary = self.generate_summary(record, &reflection);

        if self.config.store_insights {
            let mut history = self.history.write().await;
            if history.len() >= self.config.max_history {
                history.remove(0);
            }
            history.push(reflection.clone());
            drop(history);

            // P0-3 修复:落盘到 JSONL 文件(异步,失败不阻塞主流程)。
            self.persist_reflection(&reflection).await;

            if let Some(insights) = self.insight_generator.generate_from_reflection(&reflection) {
                self.insight_generator.store_insight(insights).await;
            }

            // 自进化闭环:把高价值经验沉淀到 DB memory_items。
            //
            // 阈值:
            // - 成功经验: quality_score >= 7 且有 reusable_patterns
            // - 失败教训: 有 error_patterns(无论分数,失败都值得记忆以避免重蹈覆辙)
            //
            // 写入到名为 `REFLECTOR_INSIGHTS_NS_NAME` 的 global namespace,
            // 由 wiring 层(init/state.rs)启动时确保存在。
            // 默认 confirmed=0(未确认),需用户审核后才能晋升到 core 层(v108 确认门)。
            //
            // 失败仅日志,不阻塞 reflect 主流程(沉淀是辅助,失败不影响业务)。
            let should_persist = (reflection.quality_score >= 7
                && !reflection.reusable_patterns.is_empty())
                || !reflection.error_patterns.is_empty();
            if should_persist {
                self.persist_insight_to_memory_repository(&reflection).await;
            }
        }

        reflection
    }

    /// 将高价值经验沉淀到 DB memory_items。
    ///
    /// 通过 harness 全局 `memory_repository()` trait 获取实现(wiring 层注入),
    /// 不依赖任何 implementor crate,符合分层铁律。
    ///
    /// namespace 选择:按 name 查找 `REFLECTOR_INSIGHTS_NS_NAME`,
    /// 找不到则跳过(wiring 层应确保存在;跳过仅日志,不强制创建,
    /// 因为 trait 未提供 create_namespace 能力)。
    async fn persist_insight_to_memory_repository(&self, reflection: &Reflection) {
        const REFLECTOR_INSIGHTS_NS_NAME: &str = "Reflector Insights";
        const REFLECTOR_INSIGHTS_MIN_QUALITY_FOR_PROMOTE: u8 = 7;

        let Some(repo) = axagent_harness::repositories::try_memory_repository() else {
            return;
        };

        // v109: 经验溯源 — task_id 格式为 `{conversation_id}-{timestamp_millis}`
        // (session_manager.rs:855),解析出 conversation_id 设入 source_conversation_id,
        // 便于前端从经验跳转到原始会话。
        let source_conversation_id = parse_conversation_id_from_task_id(&reflection.task_id);

        // 1. 查找 Reflector Insights namespace
        let ns_id = match repo.list_namespaces().await {
            Ok(list) => {
                list.iter().find(|ns| ns.name == REFLECTOR_INSIGHTS_NS_NAME).map(|ns| ns.id.clone())
            },
            Err(e) => {
                tracing::warn!("[reflector] persist_insight: list_namespaces failed: {} (skip)", e);
                return;
            },
        };
        let Some(namespace_id) = ns_id else {
            // namespace 不存在,wiring 层未配置,跳过
            return;
        };

        // 2. 构造经验内容:总结 + 可复用模式 + 错误模式 + 改进建议
        let mut content = String::new();
        content.push_str("# 任务反思\n\n");
        content.push_str(&reflection.overall_summary);
        content.push_str("\n\n");

        if !reflection.reusable_patterns.is_empty() {
            content.push_str("## 可复用经验\n");
            for p in &reflection.reusable_patterns {
                content.push_str("- ");
                content.push_str(p);
                content.push('\n');
            }
            content.push('\n');
        }

        if !reflection.error_patterns.is_empty() {
            content.push_str("## 错误模式(避免重蹈覆辙)\n");
            for p in &reflection.error_patterns {
                content.push_str("- ");
                content.push_str(p);
                content.push('\n');
            }
            content.push('\n');
        }

        if !reflection.improvement_suggestions.is_empty() {
            content.push_str("## 改进建议\n");
            for s in &reflection.improvement_suggestions {
                content.push_str("- ");
                content.push_str(s);
                content.push('\n');
            }
        }

        let title = format!(
            "[{}] {}",
            reflection.quality_score,
            reflection
                .overall_summary
                .split('\n')
                .next()
                .unwrap_or(&reflection.task_id)
                .chars()
                .take(80)
                .collect::<String>()
        );

        // 3. 重要度:quality_score / 10.0;成功任务略加权,失败任务降权
        let importance = (reflection.quality_score as f64 / 10.0).clamp(0.0, 1.0);

        // 4. tags: 标记来源 + 是否高质量(供前端筛选)
        let tags: Vec<String> = vec![
            "auto_reflect".to_string(),
            if reflection.quality_score >= REFLECTOR_INSIGHTS_MIN_QUALITY_FOR_PROMOTE {
                "high_quality".to_string()
            } else {
                "needs_review".to_string()
            },
        ];

        let input = axagent_harness::types::CreateMemoryItemInput {
            namespace_id: namespace_id.clone(),
            title,
            content,
            source: Some("reflector".to_string()),
            tier: Some("long_term".to_string()),
            importance: Some(importance),
            memory_nature: Some("semantic".to_string()),
            tags: Some(tags),
            decay_rate: None,
            expires_at: None,
            // 适用范围不限制(默认空数组)
            applicability_tags: None,
            // 默认未确认,需人工审核才能晋升 core 层(v108 确认门)
            confirmed: None,
            // v109: 经验溯源
            source_conversation_id,
            source_message_id: None,
        };

        match repo.add_item(input).await {
            Ok(item) => {
                tracing::info!(
                    "[reflector] persist_insight: saved item {} to namespace {} (quality={})",
                    item.id,
                    namespace_id,
                    reflection.quality_score
                );
            },
            Err(e) => {
                tracing::warn!("[reflector] persist_insight: add_item failed: {} (skip)", e);
            },
        }
    }

    fn calculate_quality_metrics(&self, record: &TaskExecutionRecord) -> QualityMetrics {
        let task_success_score = if record.success { 10.0 } else { 0.0 };

        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);
        let unique_ratio = unique_tools as f32 / total_tools as f32;
        let iteration_ratio = (unique_tools as f32 / record.iterations.max(1) as f32).min(1.0);
        let tool_efficiency_score = unique_ratio * 5.0 + iteration_ratio * 5.0;

        let expected_iterations = (unique_tools * 2).max(1);
        let iteration_efficiency_score =
            (expected_iterations as f32 / record.iterations.max(1) as f32).min(1.0) * 10.0;

        let expected_duration = record.iterations.max(1) as u64 * 2000;
        let time_efficiency_score =
            (expected_duration as f32 / record.duration_ms.max(1) as f32).min(1.0) * 10.0;

        let error_recovery_score = if record.success {
            if record.iterations > expected_iterations {
                7.0
            } else {
                10.0
            }
        } else if record.error.is_some() {
            0.0
        } else {
            2.0
        };

        let goal_completion_score = if record.success {
            8.0 + (unique_tools as f32 * 0.4).min(2.0)
        } else {
            2.0 + (unique_tools as f32 * 0.3).min(3.0)
        };

        let overall_weighted_score = task_success_score * 0.30
            + tool_efficiency_score * 0.20
            + iteration_efficiency_score * 0.15
            + time_efficiency_score * 0.15
            + error_recovery_score * 0.10
            + goal_completion_score * 0.10;

        QualityMetrics {
            task_success_score,
            tool_efficiency_score,
            iteration_efficiency_score,
            time_efficiency_score,
            error_recovery_score,
            goal_completion_score,
            overall_weighted_score,
        }
    }

    fn analyze_quality(&self, record: &TaskExecutionRecord, metrics: &QualityMetrics) -> String {
        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);
        let unique_ratio = (unique_tools as f32 / total_tools as f32) * 100.0;
        let expected_iterations = (unique_tools * 2).max(1);
        let expected_duration = record.iterations.max(1) as u64 * 2000;

        let task_status = if record.success {
            "completed successfully"
        } else {
            "task failed"
        };

        let error_status = if record.success && record.iterations > expected_iterations {
            "recovered from intermediate errors"
        } else if record.success {
            "no errors encountered"
        } else if record.error.is_some() {
            "unresolved error"
        } else {
            "no explicit error"
        };

        let goal_status = if record.success {
            "all sub-goals addressed"
        } else {
            "partial goal completion"
        };

        format!(
            "Task Success: {:.1}/10 ({})\nTool Efficiency: {:.1}/10 ({} unique tools, {} total calls, {:.0}% unique ratio)\nIteration Efficiency: {:.1}/10 ({} iterations for complexity level {})\nTime Efficiency: {:.1}/10 ({}ms vs {}ms expected)\nError Recovery: {:.1}/10 ({})\nGoal Completion: {:.1}/10 ({})\nOverall Weighted Score: {:.1}/10",
            metrics.task_success_score,
            task_status,
            metrics.tool_efficiency_score,
            unique_tools,
            total_tools,
            unique_ratio,
            metrics.iteration_efficiency_score,
            record.iterations,
            expected_iterations,
            metrics.time_efficiency_score,
            record.duration_ms,
            expected_duration,
            metrics.error_recovery_score,
            error_status,
            metrics.goal_completion_score,
            goal_status,
            metrics.overall_weighted_score,
        )
    }

    fn analyze_efficiency(&self, record: &TaskExecutionRecord) -> String {
        let mut analysis = String::new();

        let duration_per_iteration = if record.iterations > 0 {
            record.duration_ms / record.iterations as u64
        } else {
            record.duration_ms
        };

        analysis.push_str(&format!("Total duration: {}ms. ", record.duration_ms));
        analysis.push_str(&format!("Duration per iteration: {}ms. ", duration_per_iteration));

        if record.duration_ms > 60000 {
            analysis.push_str("Execution time exceeds 1 minute. Consider optimization. ");
        } else if record.duration_ms < 5000 {
            analysis.push_str("Quick execution. ");
        }

        if record.iterations > 20 {
            analysis.push_str("High iteration count may indicate inefficient reasoning. ");
        }

        analysis
    }

    fn analyze_patterns(&self, record: &TaskExecutionRecord) -> (Vec<String>, Vec<String>) {
        let mut error_patterns = Vec::new();
        let mut reusable_patterns = Vec::new();

        if let Some(ref error) = record.error {
            let error_lower = error.to_lowercase();

            if error_lower.contains("timeout") {
                error_patterns.push(
                    "Timeout issues - consider increasing timeout or optimizing query".to_string(),
                );
            }
            if error_lower.contains("permission") || error_lower.contains("denied") {
                error_patterns.push("Permission issues - verify access rights".to_string());
            }
            if error_lower.contains("not found") || error_lower.contains("404") {
                error_patterns.push("Resource not found - verify target existence".to_string());
            }
            if error_lower.contains("network") || error_lower.contains("connection") {
                error_patterns.push("Network instability - add retry logic".to_string());
            }
        }

        let sequence_patterns = Self::detect_tool_sequence_patterns(&record.tools_used);
        reusable_patterns.extend(sequence_patterns);

        let retry_patterns = Self::detect_retry_patterns(&record.tools_used);
        error_patterns.extend(retry_patterns);

        let redundant = Self::detect_redundant_tool_calls(&record.tools_used);
        error_patterns.extend(redundant);

        let unique_tools = Self::count_unique_tools(&record.tools_used);
        if record.success && record.iterations > unique_tools * 2 {
            reusable_patterns.push("Error recovery pattern: task succeeded despite high iteration count suggesting intermediate failures".to_string());
        }
        if !record.success && record.iterations > 10 {
            error_patterns.push(format!(
                "Extended retry without success: {} iterations exhausted without recovery",
                record.iterations
            ));
        }

        if record.success {
            reusable_patterns.push(format!("Successfully completed: {}", record.task_description));
        }

        if !record.tools_used.is_empty() {
            reusable_patterns.push(format!("Tool combination: {}", record.tools_used.join(" -> ")));
        }

        (error_patterns, reusable_patterns)
    }

    fn detect_tool_sequence_patterns(tools: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();

        let has_read = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("read") || l.contains("get") || l.contains("fetch")
        });
        let has_edit = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("edit")
                || l.contains("write")
                || l.contains("update")
                || l.contains("modify")
                || l.contains("patch")
        });
        let has_verify = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("test")
                || l.contains("verify")
                || l.contains("check")
                || l.contains("validate")
        });
        let has_search = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("search")
                || l.contains("find")
                || l.contains("query")
                || l.contains("lookup")
        });

        if has_read && has_edit && has_verify {
            patterns.push("read->edit->verify pattern detected".to_string());
        }
        if has_search && has_read {
            patterns.push("search->read pattern detected".to_string());
        }
        if has_edit && has_verify {
            patterns.push("edit->verify pattern detected".to_string());
        }

        patterns
    }

    fn detect_retry_patterns(tools: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut tool_counts: Vec<(String, usize)> = Vec::new();

        for tool in tools {
            if let Some(entry) = tool_counts.iter_mut().find(|(name, _)| name == tool) {
                entry.1 += 1;
            } else {
                tool_counts.push((tool.clone(), 1));
            }
        }

        for (tool, count) in &tool_counts {
            if *count > 1 {
                patterns.push(format!("Retry with same approach: {} used {} times", tool, count));
            }
        }

        for i in 0..tools.len().saturating_sub(2) {
            if tools[i] == tools[i + 2] && tools[i] != tools[i + 1] {
                patterns.push(format!(
                    "Approach variation: {} -> {} -> {}",
                    tools[i],
                    tools[i + 1],
                    tools[i + 2]
                ));
            }
        }

        patterns
    }

    fn detect_redundant_tool_calls(tools: &[String]) -> Vec<String> {
        let mut redundant = Vec::new();

        for i in 0..tools.len().saturating_sub(1) {
            if tools[i] == tools[i + 1] {
                redundant.push(format!("Consecutive redundant call: {}", tools[i]));
            }
        }

        redundant
    }

    fn count_unique_tools(tools: &[String]) -> usize {
        tools.iter().collect::<HashSet<_>>().len()
    }

    fn generate_knowledge_suggestions(
        &self,
        record: &TaskExecutionRecord,
        metrics: &QualityMetrics,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);

        if metrics.tool_efficiency_score < 5.0 {
            let ratio = (unique_tools as f32 / total_tools as f32) * 100.0;
            suggestions.push(format!(
                "Tool efficiency ({:.1}/10) below threshold - reduce redundant calls (unique: {}/{}, ratio: {:.0}%)",
                metrics.tool_efficiency_score, unique_tools, total_tools, ratio
            ));
        }

        if metrics.iteration_efficiency_score < 5.0 {
            suggestions.push(format!(
                "Iteration efficiency ({:.1}/10) indicates excessive iterations ({}) for task complexity - consider more direct approaches",
                metrics.iteration_efficiency_score, record.iterations
            ));
        }

        if metrics.time_efficiency_score < 5.0 {
            suggestions.push(format!(
                "Time efficiency ({:.1}/10) suggests slow execution ({}ms) - consider caching or parallel execution",
                metrics.time_efficiency_score, record.duration_ms
            ));
        }

        if metrics.error_recovery_score > 0.0 && metrics.error_recovery_score < 8.0 {
            suggestions
                .push("Document error recovery patterns for similar future tasks".to_string());
        }

        if record.success && metrics.overall_weighted_score >= 7.0 {
            suggestions.push(format!(
                "High-quality execution pattern (score {:.1}) - consider templating this workflow for reuse",
                metrics.overall_weighted_score
            ));
        }

        suggestions
    }

    fn generate_improvement_suggestions(
        &self,
        record: &TaskExecutionRecord,
        reflection: &Reflection,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if let Some(metrics) = &reflection.quality_metrics {
            if metrics.task_success_score < 5.0 {
                suggestions.push(format!(
                    "Task success score ({:.1}/10) indicates failure - review error: {}",
                    metrics.task_success_score,
                    record.error.as_deref().unwrap_or("unknown")
                ));
            }

            if metrics.tool_efficiency_score < 5.0 {
                let redundant = Self::count_redundant_calls(&record.tools_used);
                suggestions.push(format!(
                    "Tool efficiency ({:.1}/10) below 5.0 threshold - {} redundant tool call(s) detected",
                    metrics.tool_efficiency_score, redundant
                ));
            }

            if metrics.iteration_efficiency_score < 5.0 {
                suggestions.push(format!(
                    "Iteration efficiency ({:.1}/10) - reduce iterations from {} by planning tool usage upfront",
                    metrics.iteration_efficiency_score, record.iterations
                ));
            }

            if metrics.time_efficiency_score < 5.0 {
                let expected = record.iterations.max(1) as u64 * 2000;
                suggestions.push(format!(
                    "Time efficiency ({:.1}/10) - execution took {}ms vs {}ms expected, enable parallel execution",
                    metrics.time_efficiency_score, record.duration_ms, expected
                ));
            }
        }

        if reflection.quality_score < self.config.min_quality_threshold {
            suggestions.push(format!(
                "Quality score ({}) below threshold ({}) - review overall execution strategy",
                reflection.quality_score, self.config.min_quality_threshold
            ));
        }

        if !reflection.error_patterns.is_empty() {
            suggestions.push(format!(
                "Address {} identified error pattern(s) before next iteration",
                reflection.error_patterns.len()
            ));
        }

        suggestions
    }

    fn count_redundant_calls(tools: &[String]) -> usize {
        let mut count = 0;
        for i in 0..tools.len().saturating_sub(1) {
            if tools[i] == tools[i + 1] {
                count += 1;
            }
        }
        count
    }

    fn generate_summary(&self, record: &TaskExecutionRecord, reflection: &Reflection) -> String {
        let metrics_detail = match &reflection.quality_metrics {
            Some(m) => format!(
                " Breakdown: success={:.1}, tool_eff={:.1}, iter_eff={:.1}, time_eff={:.1}, err_recov={:.1}, goal_comp={:.1}.",
                m.task_success_score,
                m.tool_efficiency_score,
                m.iteration_efficiency_score,
                m.time_efficiency_score,
                m.error_recovery_score,
                m.goal_completion_score
            ),
            None => String::new(),
        };
        format!(
            "Task '{}' {} in {}ms with quality score {}/10.{}{} iterations, {} tools used. {} error patterns identified. {} reusable patterns found.",
            record.task_description,
            if record.success {
                "succeeded"
            } else {
                "failed"
            },
            record.duration_ms,
            reflection.quality_score,
            metrics_detail,
            record.iterations,
            record.tools_used.len(),
            reflection.error_patterns.len(),
            reflection.reusable_patterns.len()
        )
    }

    pub async fn get_history(&self) -> Vec<Reflection> {
        self.history.read().await.clone()
    }

    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    pub fn get_insight_generator(&self) -> Arc<InsightGenerator> {
        Arc::clone(&self.insight_generator)
    }
}

/// 从 task_id 解析出 conversation_id。
///
/// task_id 格式由 session_manager.rs:855 定义为 `{conversation_id}-{timestamp_millis}`,
/// 其中 conversation_id 本身是 UUID(不包含连字符以外的特殊字符),
/// timestamp 是纯数字。因此最后一个连字符之前的部分即为 conversation_id。
///
/// 解析失败时返回 None(不影响沉淀主流程,只是缺少溯源信息)。
fn parse_conversation_id_from_task_id(task_id: &str) -> Option<String> {
    let last_dash = task_id.rfind('-')?;
    let prefix = &task_id[..last_dash];
    let suffix = &task_id[last_dash + 1..];
    if !suffix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if prefix.is_empty() {
        return None;
    }
    Some(prefix.to_string())
}

/// 从 ReAct 引擎的 ThoughtChain 派生一个最小化的 [`TaskExecutionRecord`]，
/// 供 `Reflector::reflect()` 在 Synthesizing/Reflecting 阶段做质量门检查使用。
///
/// 派生规则：
/// - `task_id`：使用 `original_input` 的哈希或固定前缀（ReAct 引擎内嵌调用时无会话级 task_id）
/// - `task_description`：取 `context.original_input`（用户原始输入）
/// - `success`：根据 chain 中是否有 result 文本判定（非空视为成功）
/// - `iterations`：取 `chain.iteration`
/// - `tools_used`：从 chain.steps 中提取所有 ToolCall 类型的 tool_name
/// - 时间戳与 duration：以"现在"为终点，duration 按 iterations × 2000ms 估算
///
/// 该函数只用于 ReAct 引擎内部的质量门检查；session_manager 在任务完成时
/// 会用真实的 task_id / 时间戳 / 工具调用记录调用 `Reflector::reflect()`，
/// 那条路径产出的反思才是会被持久化到 DB 的权威记录。
pub fn task_record_from_chain(
    chain: &crate::thought_chain::ThoughtChain,
    context: &crate::reasoning_state::ReasoningContext,
) -> TaskExecutionRecord {
    use crate::reasoning_state::ActionType;

    let now = chrono::Utc::now();
    // 估算开始时间：以当前为 end，按 iterations × 2s 倒推
    let duration_ms = (context.iteration.max(1) as u64) * 2000;
    let start_time = now - chrono::Duration::milliseconds(duration_ms as i64);

    let tools_used: Vec<String> = chain
        .steps
        .iter()
        .filter_map(|s| {
            s.action.as_ref().and_then(|a| {
                if matches!(a.action_type, ActionType::ToolCall) {
                    a.tool_name.clone()
                } else {
                    None
                }
            })
        })
        .collect();

    let has_result = chain
        .latest_step()
        .and_then(|s| s.result.as_ref())
        .map(|r| !r.trim().is_empty())
        .unwrap_or(false);

    let task_id = format!("react-inline-{}", context.iteration);

    TaskExecutionRecord::new(task_id, context.original_input.clone(), start_time, now)
        .with_success(has_result)
        .with_tools(tools_used)
        .with_iterations(context.iteration)
}

#[cfg(test)]
mod tests_parse_task_id {
    use super::*;

    #[test]
    fn test_parse_conversation_id_from_task_id_normal() {
        let task_id = "conv-abc-123-1700000000000";
        let parsed = parse_conversation_id_from_task_id(task_id);
        assert_eq!(parsed.as_deref(), Some("conv-abc-123"));
    }

    #[test]
    fn test_parse_conversation_id_from_task_id_uuid_format() {
        // UUID 形式的 conversation_id + timestamp
        let task_id = "550e8400-e29b-41d4-a716-446655440000-1700000000000";
        let parsed = parse_conversation_id_from_task_id(task_id);
        assert_eq!(parsed.as_deref(), Some("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_parse_conversation_id_from_task_id_no_dash() {
        let parsed = parse_conversation_id_from_task_id("nodash");
        assert_eq!(parsed, None);
    }

    #[test]
    fn test_parse_conversation_id_from_task_id_suffix_not_numeric() {
        let parsed = parse_conversation_id_from_task_id("conv-abc");
        assert_eq!(parsed, None);
    }

    #[test]
    fn test_parse_conversation_id_from_task_id_empty_prefix() {
        let parsed = parse_conversation_id_from_task_id("-1700000000000");
        assert_eq!(parsed, None);
    }
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_reflection_creation() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(5);

        let mut record =
            TaskExecutionRecord::new("test-1".to_string(), "Test task".to_string(), start, end);
        record.compute_duration();
        record =
            record.with_success(true).with_tools(vec!["tool1".to_string(), "tool2".to_string()]);

        let reflection = reflector.reflect(&record).await;

        assert_eq!(reflection.task_id, "test-1");
        assert!(reflection.quality_score >= 1 && reflection.quality_score <= 10);
        assert!(!reflection.overall_summary.is_empty());
        assert!(reflection.quality_metrics.is_some());
        let metrics = reflection.quality_metrics.unwrap();
        assert!(metrics.overall_weighted_score >= 0.0 && metrics.overall_weighted_score <= 10.0);
        assert_eq!(metrics.task_success_score, 10.0);
    }

    #[tokio::test]
    async fn test_quality_metrics_failed_task() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(30);

        let mut record =
            TaskExecutionRecord::new("test-2".to_string(), "Failed task".to_string(), start, end);
        record.compute_duration();
        record = record
            .with_error("timeout: connection refused".to_string())
            .with_tools(vec!["search".to_string(), "search".to_string(), "read".to_string()])
            .with_iterations(15);

        let reflection = reflector.reflect(&record).await;

        assert!(reflection.quality_score < 5);
        let metrics = reflection.quality_metrics.unwrap();
        assert_eq!(metrics.task_success_score, 0.0);
        assert!(metrics.tool_efficiency_score < 7.0);
        assert!(metrics.error_recovery_score < 1.0);
    }

    #[tokio::test]
    async fn test_tool_sequence_detection() {
        let patterns = Reflector::detect_tool_sequence_patterns(&[
            "read_file".to_string(),
            "edit_file".to_string(),
            "test_runner".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("read->edit->verify")));

        let patterns = Reflector::detect_tool_sequence_patterns(&[
            "search_code".to_string(),
            "read_file".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("search->read")));
    }

    #[tokio::test]
    async fn test_retry_pattern_detection() {
        let patterns = Reflector::detect_retry_patterns(&[
            "search".to_string(),
            "search".to_string(),
            "read".to_string(),
        ]);
        assert!(
            patterns.iter().any(|p| p.contains("Retry with same approach") && p.contains("search"))
        );

        let patterns = Reflector::detect_retry_patterns(&[
            "search".to_string(),
            "read".to_string(),
            "search".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("Approach variation")));
    }

    #[tokio::test]
    async fn test_redundant_call_detection() {
        let redundant = Reflector::detect_redundant_tool_calls(&[
            "read".to_string(),
            "read".to_string(),
            "edit".to_string(),
        ]);
        assert_eq!(redundant.len(), 1);
        assert!(redundant[0].contains("read"));
    }

    #[tokio::test]
    async fn test_error_recovery_scoring() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(10);

        let mut record =
            TaskExecutionRecord::new("test-3".to_string(), "Recovery task".to_string(), start, end);
        record.compute_duration();
        record = record
            .with_success(true)
            .with_tools(vec!["read".to_string(), "edit".to_string()])
            .with_iterations(8);

        let reflection = reflector.reflect(&record).await;
        let metrics = reflection.quality_metrics.unwrap();
        assert_eq!(metrics.error_recovery_score, 7.0);
    }
}
