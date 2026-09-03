// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;

type ConnectionFactory<C> = Arc<Box<dyn Fn() -> C + Send + Sync + 'static>>;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_idle: Option<usize>,
    pub max_idle: usize,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub connection_timeout: Duration,
    pub acquire_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_idle: Some(2),
            max_idle: 5,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            connection_timeout: Duration::from_secs(30),
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

pub struct PooledConnection<C> {
    conn: Option<C>,
    pool: Arc<ConnectionPool<C>>,
    created_at: Instant,
    last_used: Instant,
    is_valid: bool,
}

impl<C: std::fmt::Debug> std::fmt::Debug for PooledConnection<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("conn", &self.conn)
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .field("is_valid", &self.is_valid)
            .finish_non_exhaustive()
    }
}

impl<C> PooledConnection<C> {
    pub fn new(conn: C, pool: Arc<ConnectionPool<C>>) -> Self {
        let now = Instant::now();
        Self { conn: Some(conn), pool, created_at: now, last_used: now, is_valid: true }
    }

    pub fn get_ref(&self) -> &C {
        self.conn.as_ref().expect("connection taken")
    }

    pub fn get_mut(&mut self) -> &mut C {
        self.conn.as_mut().expect("connection taken")
    }

    pub fn mark_invalid(&mut self) {
        self.is_valid = false;
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid
    }
}

impl<C> Drop for PooledConnection<C> {
    fn drop(&mut self) {
        if self.is_valid
            && let Some(conn) = self.conn.take()
        {
            let pool = self.pool.clone();
            // P0-3: Drop 无法约束 C: Send，也不能假设 tokio runtime 存在，
            // 用 try_write 同步短临界区归还；拿不到锁或池满则丢连接并回调计数。
            match pool.connections.try_write() {
                Ok(mut guard) if guard.len() < pool.config.max_idle => {
                    guard.push(PooledConnectionInner { conn });
                },
                _ => {
                    if let Ok(mut count) = pool.total_count.try_write() {
                        *count = count.saturating_sub(1);
                    }
                    tracing::debug!("connection_pool: 连接未归还（锁占用或池满），计数已回调");
                },
            }
        }
    }
}

struct PooledConnectionInner<C> {
    conn: C,
}

pub struct ConnectionPool<C> {
    config: PoolConfig,
    connections: Arc<RwLock<Vec<PooledConnectionInner<C>>>>,
    total_count: Arc<RwLock<usize>>,
    semaphore: Arc<Semaphore>,
    /// P0-3: factory 改 Option；默认 None，create_connection 时返回
    /// CreationFailed("no factory configured") 而不是 panic。
    factory: Option<ConnectionFactory<C>>,
}

impl<C> std::fmt::Debug for ConnectionPool<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionPool")
            .field("config", &self.config)
            .field("total_count", &self.total_count)
            .finish_non_exhaustive()
    }
}

impl<C: Send + 'static> ConnectionPool<C> {
    pub fn new(config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        Self {
            config,
            connections: Arc::new(RwLock::new(Vec::new())),
            total_count: Arc::new(RwLock::new(0)),
            semaphore,
            factory: None,
        }
    }

    pub fn with_maker<F>(self: Arc<Self>, maker: F) -> PoolBuilder<C, F>
    where
        F: Fn() -> C + Send + Sync + 'static,
    {
        PoolBuilder { pool: self, _maker: std::marker::PhantomData, maker: Some(maker) }
    }

    pub async fn acquire(self: &Arc<Self>) -> Result<PooledConnection<C>, PoolError> {
        let _permit = timeout(self.config.acquire_timeout, self.semaphore.clone().acquire_owned())
            .await
            .map_err(|_| PoolError::AcquireTimeout)?
            .map_err(|_| PoolError::PoolClosed)?;

        // 优先复用池内闲置连接（Drop 归还路径写入），避免已建连接被闲置浪费
        if let Some(inner) = self.connections.write().await.pop() {
            return Ok(PooledConnection::new(inner.conn, self.clone()));
        }

        let total = *self.total_count.read().await;
        if total >= self.config.max_connections {
            return Err(PoolError::MaxConnectionsReached);
        }

        let mut count = self.total_count.write().await;
        *count += 1;
        drop(count);

        let conn = self.create_connection().await?;

        Ok(PooledConnection::new(conn, self.clone()))
    }

    async fn create_connection(&self) -> Result<C, PoolError> {
        // P0-3: factory 为 None 时直接返回错误，不再 panic
        let factory = self
            .factory
            .as_ref()
            .ok_or_else(|| PoolError::CreationFailed("no factory configured".to_string()))?
            .clone();
        tokio::task::spawn_blocking(move || (factory)())
            .await
            .map_err(|_| PoolError::CreationFailed("Connection creation task panicked".to_string()))
    }

    pub async fn close(&self) {
        let mut connections = self.connections.write().await;
        connections.clear();
        let mut count = self.total_count.write().await;
        *count = 0;
    }

    pub async fn state(&self) -> PoolState {
        let connections = self.connections.read().await;
        let total = *self.total_count.read().await;
        PoolState {
            total_connections: total,
            idle_connections: connections.len(),
            max_connections: self.config.max_connections,
        }
    }
}

impl<C> Clone for ConnectionPool<C> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connections: self.connections.clone(),
            total_count: self.total_count.clone(),
            semaphore: self.semaphore.clone(),
            factory: self.factory.clone(),
        }
    }
}

pub struct PoolBuilder<C, F> {
    pool: Arc<ConnectionPool<C>>,
    _maker: std::marker::PhantomData<F>,
    maker: Option<F>,
}

impl<C, F> PoolBuilder<C, F>
where
    F: Fn() -> C + Send + Sync + 'static,
{
    pub fn build(mut self) -> Arc<ConnectionPool<C>> {
        // P0-3: builder 关闭时把 maker 注入到 pool.factory，不再用 panic 占位
        let mut pool = (*self.pool).clone();
        pool.factory = Some(Arc::new(Box::new(self.maker.take().expect("Builder already used"))
            as Box<dyn Fn() -> C + Send + Sync + 'static>));
        Arc::new(pool)
    }
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub total_connections: usize,
    pub idle_connections: usize,
    pub max_connections: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection pool closed")]
    PoolClosed,

    #[error("Acquire timeout")]
    AcquireTimeout,

    #[error("Max connections reached")]
    MaxConnectionsReached,

    #[error("Connection creation failed: {0}")]
    CreationFailed(String),

    #[error("Connection invalid")]
    InvalidConnection,

    #[error("Semaphore closed during release")]
    SemaphoreClosed,
}

pub struct SessionPool<C: Sessionlike> {
    /// 预留字段：会话级真实连接复用（pool.acquire 路径）需待上游会话提供方
    /// （MemoryProvider 运行时）接入后才产生消费方，当前仅固定 API 形状。
    #[allow(dead_code)]
    pool: ConnectionPool<C>,
    sessions: Arc<RwLock<HashMap<SessionKey, Instant>>>,
    session_ttl: Duration,
}

pub trait Sessionlike: Send + Sync {
    type Connection: Send;
    type SessionId: Send + Clone + std::hash::Hash + Eq;

    fn id(&self) -> Self::SessionId;
    fn is_expired(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct SessionKey {
    pub agent_id: String,
    pub endpoint: String,
}

impl std::hash::Hash for SessionKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.agent_id.hash(state);
        self.endpoint.hash(state);
    }
}

impl PartialEq for SessionKey {
    fn eq(&self, other: &Self) -> bool {
        self.agent_id == other.agent_id && self.endpoint == other.endpoint
    }
}

impl Eq for SessionKey {}

impl<C: Sessionlike + Send + 'static> SessionPool<C> {
    pub fn new(pool: ConnectionPool<C>) -> Self {
        Self {
            pool,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_ttl: Duration::from_secs(3600),
        }
    }

    pub async fn get_session(&self, key: &SessionKey) -> Result<Option<C>, PoolError>
    where
        C: Clone,
    {
        let sessions = self.sessions.read().await;
        if let Some(last_used) = sessions.get(key)
            && last_used.elapsed() < self.session_ttl
        {
            return Ok(None); // 会话仍新鲜：真实连接经 pool.acquire 获取
        }
        Ok(None)
    }

    pub async fn store_session(&self, key: SessionKey, _session: &C) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(key, Instant::now());
    }

    pub async fn remove_session(&self, key: &SessionKey) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(key);
    }

    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, last_used| last_used.elapsed() < self.session_ttl);
    }
}

#[derive(Debug)]
pub struct SessionHolder {
    pub key: SessionKey,
    created_at: Instant,
}

impl SessionHolder {
    pub fn new(key: SessionKey) -> Self {
        Self { key, created_at: Instant::now() }
    }

    pub fn is_expired(&self) -> bool {
        false
    }
}

impl Sessionlike for SessionHolder {
    type Connection = ();
    type SessionId = SessionKey;

    fn id(&self) -> Self::SessionId {
        self.key.clone()
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(3600)
    }
}
