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

use std::collections::HashSet;
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

    /// 关联扩展（Phase 3）：一跳上下游依赖能力加入候选。
    ///
    /// 遍历命中候选的 `upstream` / `downstream`，索引中存在的依赖能力以
    /// 语义分保底 0.4（综合分 0.24）追加到候选尾部，供认知编排做组合路径选择；
    /// 已在候选中的跳过；未在索引中的（外部/未注册）跳过。
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

        for candidate in final_candidates.iter() {
            for dep_id in candidate.passport.upstream.iter().chain(&candidate.passport.downstream) {
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
                        // 依赖扩展项：低语义分保底，排在实际命中之后
                        semantic_score: 0.4,
                        keyword_score: 0.0,
                        tag_score: 0.0,
                        retrieval_score: 0.24,
                        matched_tags: Vec::new(),
                        negative_hit: false,
                        passport: dep.clone(),
                    });
                }
            }
        }

        final_candidates.extend(expanded);
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
                CapabilityCandidate {
                    capability_id: c.passport.capability_id.clone(),
                    name: c.passport.name.clone(),
                    kind: c.passport.kind,
                    domain: c.passport.domain,
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

        // 8. 按综合分降序排序
        final_candidates.sort_by(|a, b| {
            b.retrieval_score.partial_cmp(&a.retrieval_score).unwrap_or(std::cmp::Ordering::Equal)
        });

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
