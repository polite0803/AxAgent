use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

use crate::message_gateway::AgentMessage;

#[derive(Debug, Clone)]
pub struct BatchingConfig {
    pub max_batch_size: usize,
    pub max_batch_delay_ms: u64,
    pub max_queue_size: usize,
    pub enable_compression: bool,
    pub compression_threshold_bytes: usize,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_delay_ms: 50,
            max_queue_size: 10000,
            enable_compression: true,
            compression_threshold_bytes: 1024,
        }
    }
}

#[derive(Debug)]
pub struct Batch<M> {
    pub id: String,
    pub messages: Vec<M>,
    pub created_at: Instant,
    pub size_bytes: usize,
}

impl<M> Batch<M> {
    pub fn new(id: String) -> Self {
        Self {
            id,
            messages: Vec::new(),
            created_at: Instant::now(),
            size_bytes: 0,
        }
    }

    pub fn add(&mut self, message: M, size: usize) {
        self.messages.push(message);
        self.size_bytes += size;
    }

    pub fn is_ready(&self, config: &BatchingConfig) -> bool {
        self.messages.len() >= config.max_batch_size || self.size_bytes >= config.compression_threshold_bytes
    }
}

pub struct MessageBatcher {
    config: BatchingConfig,
    pending: Arc<RwLock<VecDeque<AgentMessage>>>,
    batch_tx: mpsc::Sender<Batch<AgentMessage>>,
}

impl MessageBatcher {
    pub fn new(config: BatchingConfig, batch_tx: mpsc::Sender<Batch<AgentMessage>>) -> Self {
        Self {
            config,
            pending: Arc::new(RwLock::new(VecDeque::new())),
            batch_tx,
        }
    }

    pub async fn enqueue(&self, message: AgentMessage) -> Result<(), BatcherError> {
        let mut pending = self.pending.write().await;
        if pending.len() >= self.config.max_queue_size {
            return Err(BatcherError::QueueFull);
        }
        pending.push_back(message);
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), BatcherError> {
        let mut pending = self.pending.write().await;
        if pending.is_empty() {
            return Ok(());
        }

        let mut batch = Batch::new(uuid_v4());
        let messages: Vec<AgentMessage> = pending.drain(..).collect();

        for msg in messages {
            let size = estimated_size(&msg);
            batch.add(msg, size);
        }

        self.batch_tx.send(batch).await.map_err(|_| BatcherError::ChannelClosed)?;
        Ok(())
    }

    pub async fn should_flush(&self) -> bool {
        let pending = self.pending.read().await;
        if pending.is_empty() {
            return false;
        }

        if pending.len() >= self.config.max_batch_size {
            return true;
        }

        let oldest = pending.front().map(|m| m.timestamp).unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u128;

        if now - oldest > self.config.max_batch_delay_ms as u128 {
            return true;
        }

        false
    }
}

fn estimated_size(msg: &AgentMessage) -> usize {
    msg.id.len() + msg.from.len() + msg.to.len() + 100
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let random: u128 = (timestamp as u128) << 64 | (rand_u64() as u128);
    format!("{:032x}", random)
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}

#[derive(Debug, thiserror::Error)]
pub enum BatcherError {
    #[error("Queue is full")]
    QueueFull,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Gzip,
    Deflate,
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Zstd
    }
}

pub struct MessageCompressor {
    compression_type: CompressionType,
    level: CompressionLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    Fast,
    Default,
    Best,
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
}

impl CompressionLevel {
    fn to_i32(&self) -> i32 {
        match self {
            Self::Fast => 1,
            Self::Default => 6,
            Self::Best => 19,
        }
    }
}

impl MessageCompressor {
    pub fn new(compression_type: CompressionType) -> Self {
        Self {
            compression_type,
            level: CompressionLevel::Default,
        }
    }

    pub fn with_level(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    pub async fn compress(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.compress_gzip(data),
            CompressionType::Deflate => self.compress_deflate(data),
            CompressionType::Zstd => self.compress_zstd(data),
        }
    }

    pub async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.decompress_gzip(data),
            CompressionType::Deflate => self.decompress_deflate(data),
            CompressionType::Zstd => self.decompress_zstd(data),
        }
    }

    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.level.to_i32() as u32),
        );
        encoder.write_all(data).map_err(|e| BatcherError::CompressionFailed(e.to_string()))?;
        encoder.finish().map_err(|e| BatcherError::CompressionFailed(e.to_string()))
    }

    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).map_err(|e| BatcherError::DecompressionFailed(e.to_string()))?;
        Ok(output)
    }

    fn compress_deflate(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        let mut encoder = flate2::write::DeflateEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.level.to_i32() as u32),
        );
        encoder.write_all(data).map_err(|e| BatcherError::CompressionFailed(e.to_string()))?;
        encoder.finish().map_err(|e| BatcherError::CompressionFailed(e.to_string()))
    }

    fn decompress_deflate(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        let mut decoder = flate2::read::ZlibDecoder::new(data);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).map_err(|e| BatcherError::DecompressionFailed(e.to_string()))?;
        Ok(output)
    }

    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        let level = match self.level {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Best => 19,
        };
        zstd::encode_all(data, level).map_err(|e| BatcherError::CompressionFailed(e.to_string()))
    }

    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>, BatcherError> {
        zstd::decode_all(data).map_err(|e| BatcherError::DecompressionFailed(e.to_string()))
    }
}

pub struct BatchProcessor {
    batcher: MessageBatcher,
    compressor: MessageCompressor,
    config: BatchingConfig,
}

impl BatchProcessor {
    pub fn new(config: BatchingConfig) -> Self {
        let (tx, _rx) = mpsc::channel(100);
        Self {
            batcher: MessageBatcher::new(config.clone(), tx),
            compressor: MessageCompressor::new(CompressionType::Zstd),
            config,
        }
    }

    pub async fn add_message(&self, message: AgentMessage) -> Result<(), BatcherError> {
        self.batcher.enqueue(message).await
    }

    pub async fn process_batch(&self) -> Result<ProcessedBatch, BatcherError> {
        self.batcher.flush().await?;

        Ok(ProcessedBatch {
            data: Vec::new(),
            original_size: 0,
            compressed_size: 0,
            message_count: 0,
            compression_type: self.compressor.compression_type,
        })
    }

    pub async fn compress_batch(&self, messages: Vec<AgentMessage>) -> Result<CompressedBatch, BatcherError> {
        let json = serde_json::to_vec(&messages).map_err(|e| BatcherError::CompressionFailed(e.to_string()))?;
        let original_size = json.len();

        let compressed = if original_size >= self.config.compression_threshold_bytes && self.config.enable_compression {
            self.compressor.compress(&json).await?
        } else {
            json
        };

        Ok(CompressedBatch {
            data: compressed,
            original_size,
            compressed_size: 0,
            compression_type: self.compressor.compression_type,
        })
    }
}

pub struct ProcessedBatch {
    pub data: Vec<u8>,
    pub original_size: usize,
    pub compressed_size: usize,
    pub message_count: usize,
    pub compression_type: CompressionType,
}

pub struct CompressedBatch {
    pub data: Vec<u8>,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_type: CompressionType,
}

impl CompressedBatch {
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.data.len() as f64 / self.original_size as f64
    }
}

pub struct StreamingBatcher {
    config: BatchingConfig,
    buffer: Arc<RwLock<Vec<AgentMessage>>>,
    last_flush: Arc<RwLock<Instant>>,
}

impl StreamingBatcher {
    pub fn new(config: BatchingConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(Vec::with_capacity(config.max_batch_size))),
            last_flush: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn push(&self, message: AgentMessage) -> Result<Option<Vec<AgentMessage>>, BatcherError> {
        let mut buffer = self.buffer.write().await;
        buffer.push(message);

        let elapsed = self.last_flush.read().await.elapsed();

        if buffer.len() >= self.config.max_batch_size || elapsed.as_millis() >= self.config.max_batch_delay_ms as u128 {
            let batch = std::mem::replace(&mut *buffer, Vec::with_capacity(self.config.max_batch_size));
            *self.last_flush.write().await = Instant::now();
            return Ok(Some(batch));
        }

        Ok(None)
    }

    pub async fn flush(&self) -> Result<Vec<AgentMessage>, BatcherError> {
        let mut buffer = self.buffer.write().await;
        if buffer.is_empty() {
            return Ok(Vec::new());
        }
        let batch = std::mem::take(&mut *buffer);
        *self.last_flush.write().await = Instant::now();
        Ok(batch)
    }

    pub async fn pending_count(&self) -> usize {
        self.buffer.read().await.len()
    }
}