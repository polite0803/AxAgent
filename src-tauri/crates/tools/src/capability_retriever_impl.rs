// SPDX-License-Identifier: AGPL-3.0-only
//! 能力检索器实现 — 混合检索引擎
//!
//! 实现三层检索逻辑：
//! 1. 向量相似度检索（复用 VectorStore.search）
//! 2. 标签硬匹配（内存元数据索引）
//! 3. 负面场景排除（negative collection 搜索）
//!
//! # 检索流程
//! 1. 将用户输入嵌入向量
//! 2. 在正向 collection 中语义搜索
//! 3. 按 kind/domain 过滤元数据
//! 4. 计算标签匹配分
//! 5. 在负向 collection 中搜索排除命中
//! 6. 合成综合分并排序

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use axagent_harness::rag_provider::EmbeddingProvider;
use axagent_harness::{
    CAPABILITY_COLLECTION, CAPABILITY_NEGATIVE_COLLECTION, CapabilityCandidate,
    CapabilityIndexStats, CapabilityIndexer, CapabilityPassportDto, CapabilityQuery,
    CapabilityRetrievalResult, CapabilityRetriever,
};
use axagent_search::vector_store::{VectorSearchResult, VectorStore};

/// 向量检索结果的内部映射结构
struct ScoredCandidate {
    pub passport: CapabilityPassportDto,
    pub semantic_score: f64,
    /// 关键词匹配分（BM25/FTS 归一化到 0.0-1.0）
    pub keyword_score: f64,
    pub tag_score: f64,
    pub matched_tags: Vec<String>,
    pub negative_hit: bool,
}

/// 负向场景命中阈值（相似度 >= 阈值即判定为负向命中，剔除候选）
const NEGATIVE_HIT_THRESHOLD: f64 = 0.6;

/// 分层检索命中阈值（P0）：某层 top1 候选的 retrieval_score ≥ 此值即判定该层命中。
/// 阈值语义：语义 0.6 权重下，semantic≈0.5 + 少量关键词/标签分即达标，代表"语义明显相关"。
/// 低于阈值说明该层无强匹配，允许降级到下一层。
const LAYER_HIT_THRESHOLD: f64 = 0.45;

/// 关联扩展最大跳数（P2）：2 跳 BFS，兼顾组合路径发现与候选噪音控制。
const DEPENDENCY_MAX_DEPTH: usize = 2;
/// 关联扩展每跳衰减系数（P2）：第 2 跳语义分 = 第 1 跳 × 0.6。
const DEPENDENCY_DECAY: f64 = 0.6;

/// 能力检索器实现
#[derive(Clone)]
pub struct CapabilityRetrieverImpl {
    vector_store: Arc<VectorStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    indexer: Arc<dyn CapabilityIndexer>,
}

impl CapabilityRetrieverImpl {
    pub fn new(
        vector_store: Arc<VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        indexer: Arc<dyn CapabilityIndexer>,
    ) -> Self {
        Self { vector_store, embedding_provider, indexer }
    }

    /// 计算标签匹配分
    /// 完全匹配 = 1.0，部分匹配 = 0.5，无匹配 = 0.0
    fn compute_tag_score(required_tags: &[String], passport_tags: &[String]) -> (f64, Vec<String>) {
        if required_tags.is_empty() {
            return (0.0, Vec::new());
        }

        let passport_set: HashSet<&String> = passport_tags.iter().collect();
        let mut matched = Vec::new();
        let mut match_count = 0;

        for req_tag in required_tags {
            let req_lower = req_tag.to_lowercase();
            if passport_set.iter().any(|t| t.to_lowercase() == req_lower) {
                match_count += 1;
                matched.push(req_tag.clone());
            } else if passport_set.iter().any(|t| t.to_lowercase().contains(&req_lower)) {
                // 部分匹配
                match_count += 1;
                matched.push(req_tag.clone());
            }
        }

        let ratio = match_count as f64 / required_tags.len().max(1) as f64;
        let score = if ratio >= 1.0 {
            1.0
        } else if ratio > 0.0 {
            0.5
        } else {
            0.0
        };

        (score, matched)
    }

    /// 将 L2 distance 转换为 0-1 相似度分
    /// L2=0 → 1.0, L2≥阈值 → 0.0
    fn distance_to_score(distance: f32) -> f64 {
        // 经验阈值：1536维向量 L2 通常在 5-40 之间
        let normalized = 1.0 / (1.0 + distance as f64 / 10.0);
        normalized.clamp(0.0, 1.0)
    }

    /// 从元数据索引构建完整的候选列表
    ///
    /// 使用 `list_passports` 一次性获取所有护照（单次读锁），
    /// 避免 N 次逐个 `get_passport` 造成的加锁开销。
    async fn build_candidates_from_metadata(
        &self,
        query: &CapabilityQuery,
    ) -> Vec<ScoredCandidate> {
        let all_passports = self.indexer.list_passports().await;
        let mut candidates = Vec::with_capacity(all_passports.len());

        for passport in all_passports {
            if query.exclude_ids.contains(&passport.capability_id) {
                continue;
            }

            // 检查 kind 过滤
            if let Some(ref kinds) = query.kind_filter
                && !kinds.contains(&passport.kind)
            {
                continue;
            }

            // 检查 domain 过滤
            if let Some(ref domains) = query.domain_filter
                && !domains.contains(&passport.domain)
            {
                continue;
            }

            // 检查 enabled
            if !passport.enabled {
                continue;
            }

            // 检查可见性：SystemOnly / Hidden 能力不可被用户发现（元能力隔离）
            if !passport.visibility.is_discoverable() {
                continue;
            }

            candidates.push(ScoredCandidate {
                passport,
                semantic_score: 0.0,
                keyword_score: 0.0,
                tag_score: 0.0,
                matched_tags: Vec::new(),
                negative_hit: false,
            });
        }

        candidates
    }

    /// 执行负向场景搜索
    ///
    /// 注意：此方法每次都会重新计算查询嵌入，仅供 `filter_negative` 等单次调用场景使用。
    /// `retrieve` 主流程应使用 `negative_search_with_embedding` 复用预计算嵌入（Bug 8）。
    async fn negative_search(
        &self,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>, String> {
        let embedding = self
            .embedding_provider
            .embed(query_text)
            .await
            .map_err(|e| format!("Embedding failed: {e}"))?;

        self.vector_store
            .search(CAPABILITY_NEGATIVE_COLLECTION, embedding, top_k)
            .await
            .map_err(|e| format!("Negative search failed: {e}"))
    }

    /// 语义向量搜索（复用预计算的查询嵌入，避免重复 embed）
    async fn semantic_search_with_embedding(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>, String> {
        self.vector_store
            .search(CAPABILITY_COLLECTION, query_embedding.to_vec(), top_k)
            .await
            .map_err(|e| format!("Vector search failed: {e}"))
    }

    /// 负向场景搜索（复用预计算的查询嵌入，避免重复 embed）
    async fn negative_search_with_embedding(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>, String> {
        self.vector_store
            .search(CAPABILITY_NEGATIVE_COLLECTION, query_embedding.to_vec(), top_k)
            .await
            .map_err(|e| format!("Negative search failed: {e}"))
    }

    /// 将向量搜索结果合并到候选列表
    fn merge_vector_results(
        candidates: &mut [ScoredCandidate],
        vector_results: &[VectorSearchResult],
    ) {
        for result in vector_results {
            // result.document_id 是 capability_id
            if let Some(candidate) =
                candidates.iter_mut().find(|c| c.passport.capability_id == result.document_id)
            {
                let score = Self::distance_to_score(result.score);
                if score > candidate.semantic_score {
                    candidate.semantic_score = score;
                }
            }
        }
    }

    /// 从负向结果中标记命中
    fn mark_negative_hits(
        candidates: &mut [ScoredCandidate],
        negative_results: &[VectorSearchResult],
        threshold: f64,
    ) {
        for result in negative_results {
            let score = Self::distance_to_score(result.score);
            if score >= threshold
                && let Some(candidate) =
                    candidates.iter_mut().find(|c| c.passport.capability_id == result.document_id)
            {
                candidate.negative_hit = true;
            }
        }
    }

    /// 别名硬匹配（Phase 2）：用户输入命中护照 `aliases` 时直接提升语义分。
    ///
    /// 规则：trim + 小写后精确匹配 → 语义分保底 0.85；双向包含匹配 → 保底 0.6。
    /// 别名是"用户口语 → 能力 ID"的映射（如 "发邮件" → mail_send），
    /// 语义向量对口语/缩写不敏感，别名提供确定性兜底。
    fn apply_alias_boost(candidates: &mut [ScoredCandidate], user_input: &str) {
        let input = user_input.trim().to_lowercase();
        if input.is_empty() {
            return;
        }
        for candidate in candidates.iter_mut() {
            if candidate.passport.aliases.is_empty() {
                continue;
            }
            for alias in &candidate.passport.aliases {
                let alias_lower = alias.trim().to_lowercase();
                if alias_lower.is_empty() {
                    continue;
                }
                let boost = if alias_lower == input {
                    0.85
                } else if input.contains(&alias_lower) || alias_lower.contains(&input) {
                    0.6
                } else {
                    continue;
                };
                if boost > candidate.semantic_score {
                    candidate.semantic_score = boost;
                }
                break;
            }
        }
    }

    /// 分层检索闸门（P0）：应用层 → 任务层 → 原子层逐层判定。
    ///
    /// 输入已按 retrieval_score 排序的候选；按 `layer` 分组后逐层检查：
    /// - App / Task 层：top1 综合分 ≥ [`LAYER_HIT_THRESHOLD`] 即命中该层，返回层内前
    ///   `top_k`（不再降级到下层）；
    /// - Atomic 层：无条件兜底，返回层内前 `top_k`。
    ///
    /// 返回值保持 retrieval_score 降序（层内已排序），供下游过滤/排序继续消费。
    fn apply_layer_gating(
        mut candidates: Vec<CapabilityCandidate>,
        top_k: usize,
    ) -> Vec<CapabilityCandidate> {
        if candidates.is_empty() {
            return candidates;
        }

        // 按层分组（层内已保持原排序，无需重排——调用方已按综合分降序）
        let mut app: Vec<CapabilityCandidate> = Vec::new();
        let mut task: Vec<CapabilityCandidate> = Vec::new();
        let mut atomic: Vec<CapabilityCandidate> = Vec::new();
        for c in candidates.drain(..) {
            match c.layer {
                axagent_harness::CapabilityLayer::App => app.push(c),
                axagent_harness::CapabilityLayer::Task => task.push(c),
                axagent_harness::CapabilityLayer::Atomic => atomic.push(c),
            }
        }

        // 逐层判定：App → Task → Atomic（原子层兜底）
        for (idx, layer_candidates) in [app, task, atomic].into_iter().enumerate() {
            if layer_candidates.is_empty() {
                continue;
            }
            // 层内仍按综合分排序（分组不破坏顺序），取 top1 判定
            let hit = idx == 2 || layer_candidates[0].retrieval_score >= LAYER_HIT_THRESHOLD;
            if hit {
                let mut result = layer_candidates;
                result.truncate(top_k);
                return result;
            }
        }
        Vec::new()
    }

    /// 关联扩展（P2）：多跳 BFS 上下游依赖扩展。
    ///
    /// 从命中候选出发，沿护照 `upstream`/`downstream` 做广度优先遍历
    /// （[`DEPENDENCY_MAX_DEPTH`]=2 跳），每跳语义分按 [`DEPENDENCY_DECAY`]=0.6 衰减：
    /// 第 1 跳保底 0.4（综合分 0.24），第 2 跳 0.24（综合分 0.144）。
    /// 已在候选/已访问的跳过；未在索引中的（外部/未注册）跳过；
    /// 不可发现的系统能力不扩展。
    fn expand_dependencies(
        final_candidates: &mut Vec<CapabilityCandidate>,
        all_passports: &[axagent_harness::CapabilityPassportDto],
    ) {
        if final_candidates.is_empty() || all_passports.is_empty() {
            return;
        }
        let by_id: std::collections::HashMap<&str, &axagent_harness::CapabilityPassportDto> =
            all_passports.iter().map(|p| (p.capability_id.as_str(), p)).collect();

        let mut existing: std::collections::HashSet<String> =
            final_candidates.iter().map(|c| c.capability_id.clone()).collect();
        let mut expanded: Vec<CapabilityCandidate> = Vec::new();

        // BFS 队列：(能力ID, 深度)。起点候选深度 1（第 1 跳即原一跳行为，向后兼容）。
        let mut queue: VecDeque<(String, usize)> =
            final_candidates.iter().map(|c| (c.capability_id.clone(), 1usize)).collect();

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > DEPENDENCY_MAX_DEPTH {
                continue;
            }
            let Some(current) = by_id.get(current_id.as_str()).copied() else {
                continue;
            };
            // 本跳语义分衰减：第 1 跳 0.4，第 2 跳 0.4×0.6=0.24
            let hop_score = 0.4 * DEPENDENCY_DECAY.powi(depth as i32 - 1);
            for dep_id in current.upstream.iter().chain(&current.downstream) {
                if !existing.insert(dep_id.clone()) {
                    continue;
                }
                if let Some(dep) = by_id.get(dep_id.as_str()).copied() {
                    // 依赖能力同样受可见性约束（不可发现的系统能力不扩展）
                    if !dep.visibility.is_discoverable() || !dep.enabled {
                        continue;
                    }
                    expanded.push(CapabilityCandidate {
                        capability_id: dep.capability_id.clone(),
                        name: dep.name.clone(),
                        kind: dep.kind,
                        domain: dep.domain,
                        layer: axagent_harness::CapabilityLayer::from_kind(dep.kind),
                        // 依赖扩展项：低语义分保底（随跳数衰减），排在实际命中之后
                        semantic_score: hop_score,
                        keyword_score: 0.0,
                        tag_score: 0.0,
                        retrieval_score: hop_score * 0.6,
                        matched_tags: Vec::new(),
                        negative_hit: false,
                        passport: dep.clone(),
                    });
                    // 任务①：按护照声明解析执行分派模式（Sync/Async/Streaming），
                    // 记录于发现边界；dispatcher 据此消费 `passport.execution_mode`
                    // （当前默认 Sync，故不改变既有行为）。
                    let _resolved_mode = axagent_harness::capability::resolve_execution_mode(
                        dep.execution_mode,
                        dep.kind,
                    );
                    tracing::debug!(
                        capability_id = %dep.capability_id,
                        declared = ?dep.execution_mode,
                        resolved = ?_resolved_mode,
                        "🧩 护照执行模式解析（任务①）"
                    );
                    queue.push_back((dep.capability_id.clone(), depth + 1));
                }
            }
        }

        final_candidates.extend(expanded);

        // 任务②：上下游契约静态校验（触发条件：编排前校验「下游入参=上游输出」）。
        // 仅当 schema 实际就绪（非 None 且为对象型 JSON Schema）时才产生告警，
        // 当前绝大多数护照 schema 未填充，故此调用为 no-op，不改变既有行为。
        for m in Self::validate_dependency_chain_compatibility(all_passports) {
            tracing::warn!(
                downstream = %m.downstream_id,
                upstream = %m.upstream_id,
                missing = ?m.missing_properties,
                "🧩 上下游契约不兼容：下游必填入参未被上游输出覆盖"
            );
        }
    }

    /// 检查单条上下游边的契约兼容性（任务②核心实现）。
    ///
    /// 当下游 `input_schema` 声明了 `required` 属性，而上游 `output_schema` 的
    /// `properties` 未提供同名属性时，记为不兼容。
    /// 任一 schema 为 `None` 或非对象型 JSON Schema 时**跳过**（无操作），
    /// 故当前 schema 未填充的护照完全不受影响。
    fn check_edge_compatibility(
        upstream: &CapabilityPassportDto,
        downstream: &CapabilityPassportDto,
    ) -> Option<SchemaMismatch> {
        let (Some(up_out), Some(down_in)) = (&upstream.output_schema, &downstream.input_schema)
        else {
            return None;
        };
        let (serde_json::Value::Object(up_obj), serde_json::Value::Object(down_obj)) =
            (up_out, down_in)
        else {
            return None;
        };
        let up_props = up_obj.get("properties").and_then(|v| v.as_object());
        let Some(down_required) = down_obj.get("required").and_then(|v| v.as_array()) else {
            return None; // 下游未声明必填，无需校验
        };
        let mut missing = Vec::new();
        for req in down_required {
            if let Some(name) = req.as_str() {
                let satisfied = up_props.map(|p| p.contains_key(name)).unwrap_or(false);
                if !satisfied {
                    missing.push(name.to_string());
                }
            }
        }
        if missing.is_empty() {
            None
        } else {
            Some(SchemaMismatch {
                upstream_id: upstream.capability_id.clone(),
                downstream_id: downstream.capability_id.clone(),
                missing_properties: missing,
            })
        }
    }

    /// 校验整张护照依赖图的契约兼容性，返回所有不兼容边。
    ///
    /// 同时供 `expand_dependencies` 运行时告警与单测复用（任务②）。
    pub fn validate_dependency_chain_compatibility(
        passports: &[CapabilityPassportDto],
    ) -> Vec<SchemaMismatch> {
        let by_id: HashMap<&str, &CapabilityPassportDto> =
            passports.iter().map(|p| (p.capability_id.as_str(), p)).collect();
        let mut mismatches = Vec::new();
        for p in passports {
            // p 的 upstream 是 p 的前置提供者：检查「上游输出 → 下游入参」覆盖
            for up_id in &p.upstream {
                if let Some(up) = by_id.get(up_id.as_str()).copied()
                    && let Some(m) = Self::check_edge_compatibility(up, p)
                {
                    mismatches.push(m);
                }
            }
        }
        mismatches
    }

    /// 将 FTS 关键词检索结果合并到候选列表的 keyword_score
    ///
    /// FTS 的 score 语义为"越小越匹配"（bm25 返回负数，PG 取负 ts_rank），
    /// 归一化到 0.0-1.0：`1.0 / (1.0 + |score|)`，score 越接近 0（越匹配）归一化值越大。
    fn merge_fts_results(candidates: &mut [ScoredCandidate], fts_results: &[VectorSearchResult]) {
        for result in fts_results {
            if let Some(candidate) =
                candidates.iter_mut().find(|c| c.passport.capability_id == result.document_id)
            {
                let normalized = 1.0 / (1.0 + result.score.abs() as f64);
                if normalized > candidate.keyword_score {
                    candidate.keyword_score = normalized;
                }
            }
        }
    }
}

/// 上下游契约静态校验（任务②）的不兼容记录。
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaMismatch {
    /// 上游能力 ID（应提供输出）
    pub upstream_id: String,
    /// 下游能力 ID（消费方，声明必填入参）
    pub downstream_id: String,
    /// 下游要求、但上游 `output_schema.properties` 未提供的属性名
    pub missing_properties: Vec<String>,
}

#[async_trait]
impl CapabilityRetriever for CapabilityRetrieverImpl {
    async fn retrieve(&self, query: &CapabilityQuery) -> Result<CapabilityRetrievalResult, String> {
        let start = Instant::now();
        let top_k = query.top_k.max(1);

        // 1. 从元数据索引构建候选池
        let mut candidates = self.build_candidates_from_metadata(query).await;

        if candidates.is_empty() {
            return Ok(CapabilityRetrievalResult {
                candidates: Vec::new(),
                total_recalled: 0,
                elapsed_ms: start.elapsed().as_millis() as u64,
            });
        }

        // 2. 预先计算查询嵌入（避免语义搜索与负向搜索重复 embed —— Bug 8）
        let query_embedding = self
            .embedding_provider
            .embed(&query.user_input)
            .await
            .map_err(|e| format!("Embedding failed: {e}"))?;

        // 3. 执行语义向量搜索（复用预计算嵌入）
        let vector_results = self
            .semantic_search_with_embedding(&query_embedding, top_k.max(candidates.len()))
            .await?;
        Self::merge_vector_results(&mut candidates, &vector_results);

        // 4. 执行 FTS 关键词检索（Bug 4：接入 BM25/FTS 计算 keyword_score）
        //    FTS 索引不存在或后端不支持时返回空 Vec，keyword_score 保持 0.0（降级）
        let fts_results = match self
            .vector_store
            .fts_search(CAPABILITY_COLLECTION, &query.user_input, top_k.max(candidates.len()))
            .await
        {
            Ok(results) => results,
            Err(e) => {
                tracing::debug!("[capability] FTS 检索失败，降级 keyword_score=0.0: {}", e);
                Vec::new()
            },
        };
        Self::merge_fts_results(&mut candidates, &fts_results);

        // 5. 计算标签匹配分
        for candidate in candidates.iter_mut() {
            let (tag_score, matched_tags) =
                Self::compute_tag_score(&query.required_tags, &candidate.passport.tags);
            candidate.tag_score = tag_score;
            candidate.matched_tags = matched_tags;
        }

        // 6. 负面场景排除（复用预计算嵌入）
        let negative_results =
            self.negative_search_with_embedding(&query_embedding, candidates.len()).await?;
        Self::mark_negative_hits(&mut candidates, &negative_results, NEGATIVE_HIT_THRESHOLD);

        // 6b. 别名硬匹配（Phase 2）：用户输入命中护照别名时直接加权，等价于高置信标签命中。
        //     精确匹配（大小写不敏感）→ 语义分保底 0.85；包含匹配 → 保底 0.6。
        Self::apply_alias_boost(&mut candidates, &query.user_input);

        // 7. 计算综合分并过滤负面命中
        //    综合分公式：语义 0.6 + 关键词 0.2 + 标签 0.2（Bug 3：修正原公式中 semantic 重复计算）
        let mut final_candidates: Vec<CapabilityCandidate> = candidates
            .into_iter()
            .filter(|c| !c.negative_hit)
            .map(|c| {
                let retrieval_score =
                    c.semantic_score * 0.6 + c.keyword_score * 0.2 + c.tag_score * 0.2;
                let layer = axagent_harness::CapabilityLayer::from_kind(c.passport.kind);
                CapabilityCandidate {
                    capability_id: c.passport.capability_id.clone(),
                    name: c.passport.name.clone(),
                    kind: c.passport.kind,
                    domain: c.passport.domain,
                    layer,
                    semantic_score: c.semantic_score,
                    keyword_score: c.keyword_score,
                    tag_score: c.tag_score,
                    retrieval_score,
                    matched_tags: c.matched_tags,
                    negative_hit: false,
                    passport: c.passport,
                }
            })
            .collect();

        // 8. 分层检索闸门（P0）：应用层(Workflow) → 任务层(Skill/Template/Toolchain)
        //    → 原子层(Tool/Agent/KB) 逐层判定。高层 top1 综合分 ≥ 阈值即命中该层，
        //    返回该层前 top_k（不再降级）；原子层无条件兜底。
        //    替代原先的单层全量排序截断 —— 规范"高层命中就不降级"语义落地。
        final_candidates = Self::apply_layer_gating(final_candidates, top_k);

        // 8b. 关联扩展（Phase 3）：对命中的候选做一跳上下游扩展。
        //     候选中护照声明了 upstream/downstream 依赖时，把索引中存在的依赖能力
        //     也加入候选（语义分保底 0.4，排在实际命中之后），供认知编排组组合路径。
        Self::expand_dependencies(&mut final_candidates, &self.indexer.list_passports().await);

        // 9. 截断到 top_k
        final_candidates.truncate(top_k);

        let total_recalled = final_candidates.len();

        Ok(CapabilityRetrievalResult {
            candidates: final_candidates,
            total_recalled,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn filter_negative(
        &self,
        candidates: Vec<CapabilityCandidate>,
        user_input: &str,
    ) -> Vec<CapabilityCandidate> {
        if candidates.is_empty() {
            return candidates;
        }

        // 构建文档 ID 列表
        let doc_ids: Vec<String> = candidates.iter().map(|c| c.capability_id.clone()).collect();

        // 执行负向搜索
        let negative_results = match self.negative_search(user_input, doc_ids.len()).await {
            Ok(results) => results,
            Err(_) => return candidates,
        };

        // 找出命中的 capability_id
        let hit_ids: HashSet<String> = negative_results
            .iter()
            .filter(|r| Self::distance_to_score(r.score) >= NEGATIVE_HIT_THRESHOLD)
            .map(|r| r.document_id.clone())
            .collect();

        // 标记并过滤
        candidates
            .into_iter()
            .map(|mut c| {
                if hit_ids.contains(&c.capability_id) {
                    c.negative_hit = true;
                }
                c
            })
            .filter(|c| !c.negative_hit)
            .collect()
    }

    async fn refresh_index(&self) -> Result<CapabilityIndexStats, String> {
        self.indexer.get_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn passport(
        id: &str,
        upstream: &[&str],
        input: Option<serde_json::Value>,
        output: Option<serde_json::Value>,
    ) -> CapabilityPassportDto {
        CapabilityPassportDto {
            capability_id: id.to_string(),
            upstream: upstream.iter().map(|s| (*s).to_string()).collect(),
            input_schema: input,
            output_schema: output,
            ..Default::default()
        }
    }

    #[test]
    fn no_mismatch_when_schemas_absent() {
        let passports =
            vec![passport("down", &["up"], None, None), passport("up", &[], None, None)];
        let mismatches =
            CapabilityRetrieverImpl::validate_dependency_chain_compatibility(&passports);
        assert!(mismatches.is_empty(), "未填充 schema 时不应产生告警");
    }

    #[test]
    fn mismatch_when_upstream_lacks_required_output() {
        let passports = vec![
            passport("down", &["up"], Some(json!({"type":"object","required":["ip"]})), None),
            passport("up", &[], None, Some(json!({"type":"object","properties":{"port":{}}}))),
        ];
        let mismatches =
            CapabilityRetrieverImpl::validate_dependency_chain_compatibility(&passports);
        assert_eq!(mismatches.len(), 1, "上游缺 ip 输出应被判为不兼容");
        assert_eq!(mismatches[0].upstream_id, "up");
        assert_eq!(mismatches[0].downstream_id, "down");
        assert_eq!(mismatches[0].missing_properties, vec!["ip".to_string()]);
    }

    #[test]
    fn no_mismatch_when_upstream_covers_required() {
        let passports = vec![
            passport("down", &["up"], Some(json!({"type":"object","required":["ip"]})), None),
            passport("up", &[], None, Some(json!({"type":"object","properties":{"ip":{}}}))),
        ];
        let mismatches =
            CapabilityRetrieverImpl::validate_dependency_chain_compatibility(&passports);
        assert!(mismatches.is_empty(), "上游已覆盖下游必填入参时不应告警");
    }
}
