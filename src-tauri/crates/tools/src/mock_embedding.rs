// SPDX-License-Identifier: AGPL-3.0-only
//! Mock 嵌入提供者 — 用于开发/测试阶段
//!
//! 生成确定性的伪随机嵌入向量（基于文本哈希），
//! 确保相同文本始终得到相同向量，便于调试和测试。

use async_trait::async_trait;
use axagent_harness::rag_provider::EmbeddingProvider;

/// Mock 嵌入提供者
///
/// 使用 FNV-1a 哈希生成确定性伪随机向量。
/// 注意：这些向量不具备真正的语义含义，仅用于功能测试。
#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider {
    dimensions: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    /// 使用 FNV-1a 哈希生成伪随机 f32 向量
    fn hash_to_vector(&self, text: &str) -> Vec<f32> {
        let mut vector = Vec::with_capacity(self.dimensions);
        let mut hash: u64 = 0xcbf29ce484222325;

        for byte in text.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        // 从哈希值衍生多个伪随机数
        let mut state = hash;
        for _ in 0..self.dimensions {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let val = (state >> 33) as f32 / (1u64 << 31) as f32;
            // 归一化到 [-1, 1]
            vector.push(val * 2.0 - 1.0);
        }

        // L2 归一化
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt().max(1.0);
        vector.iter_mut().for_each(|x| *x /= norm);

        vector
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new(1536)
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        Ok(self.hash_to_vector(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|t| self.hash_to_vector(t)).collect())
    }

    fn dimension(&self) -> usize {
        self.dimensions
    }
}
