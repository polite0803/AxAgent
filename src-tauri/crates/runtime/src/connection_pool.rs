// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use tokio::time::timeout;

type ConnectionFactory<C> = Arc<Box<dyn Fn() -> C + Send + 'static>>;

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

#[derive(Debug)]
pub struct PooledConnection<C> {
    conn: Option<C>,
    pool: Arc<ConnectionPool<C>>,
    created_at: Instant,
    last_used: Instant,
    is_valid: bool,
}

impl<C> PooledConnection<C> {
    pub fn new(conn: C, pool: Arc<ConnectionPool<C>>) -> Self {
        let now = Instant::now();
        Self {
            conn: Some(conn),
            pool,
            created_at: now,
            last_used: now,
            is_valid: true,
        }
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
            // P0-3: 不要 panic！Drop 阶段如果连接已经被取走，说明逻辑 bug，
            // 但绝不能 panic（panic 会污染其他正在进行的 tokio 任务）。
            // 若无连接可归还（Option 为 None），仅记录日志后 return。
            let conn = PooledConnection {
                conn: Some(conn),
                pool: self.pool.clone(),
                created_at: self.created_at,
                last_used: self.last_used,
                is_valid: true,
            };

            let pool = self.pool.clone();
            // P0-3: spawn 失败（runtime 关闭等）时记录日志而非静默吞掉
            if let Err(e) = tokio::spawn(async move {
                pool.release(conn).await;
            }) {
                tracing::error!(error = %e, "connection_pool: spawn release 任务失败");
            }
        }
    }
}

struct PooledConnectionInner<C> {
    conn: C,
    created_at: Instant,
    last_used: Instant,
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
        F: Fn() -> C + Send + 'static,
    {
        PoolBuilder {
            pool: self,
            _maker: std::marker::PhantomData,
            maker: Some(maker),
        }
    }

    pub async fn acquire(&self) -> Result<PooledConnection<C>, PoolError> {
        let _permit = timeout(self.config.acquire_timeout, self.semaphore.clone().acquire_owned())
            .await
            .map_err(|_| PoolError::AcquireTimeout)?
            .map_err(|_| PoolError::PoolClosed)?;

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

    /// P0-3: release 路径简化 —— 之前 acquire_owned + drop permit 是反模式（无意义消耗一次信号量）。
    /// 归还连接只需更新 total_count（如果池满）并放回 idle 列表。
    async fn release(&self, conn: PooledConnection<C>) {
        if !conn.is_valid {
            let mut count = self.total_count.write().await;
            *count = count.saturating_sub(1);
            return;
        }

        // 从 PooledConnection 中取出真实连接（Drop 时已 take，这里 unwrap 安全）
        let Some(real_conn) = conn.conn else {
            // 已没有可归还的连接，total_count 仍要恢复
            let mut count = self.total_count.write().await;
            *count = count.saturating_sub(1);
            return;
        };

        let mut connections = self.connections.write().await;
        if connections.len() < self.config.max_idle {
            connections.push(PooledConnectionInner {
                conn: real_conn,
                created_at: conn.created_at,
                last_used: conn.last_used,
            });
        } else {
            let mut count = self.total_count.write().await;
            *count = count.saturating_sub(1);
        }
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

impl Clone for PoolConfig {
    fn clone(&self) -> Self {
        Self {
            max_connections: self.max_connections,
            min_idle: self.min_idle,
            max_idle: self.max_idle,
            idle_timeout: self.idle_timeout,
            max_lifetime: self.max_lifetime,
            connection_timeout: self.connection_timeout,
            acquire_timeout: self.acquire_timeout,
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
    F: Fn() -> C + Send + 'static,
{
    pub fn build(self) -> Arc<ConnectionPool<C>> {
        // P0-3: builder 关闭时把 maker 注入到 pool.factory，不再用 panic 占位
        let mut pool = (*self.pool).clone();
        pool.factory = Some(Arc::new(Box::new(
            self.maker.take().expect("Builder already used"),
        )
            as Box<dyn Fn() -> C + Send + 'static>));
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
    pool: ConnectionPool<C::Connection>,
    sessions: Arc<RwLock<HashMap<String, Instant>>>,
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

impl SessionPool<SessionHolder> {
    pub fn new(pool: ConnectionPool<SessionHolder>) -> Self {
        Self {
            pool,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_ttl: Duration::from_secs(3600),
        }
    }

    pub async fn get_session(&self, key: &SessionKey) -> Result<Option<SessionHolder>, PoolError> {
        let sessions = self.sessions.read().await;
        if let Some(last_used) = sessions.get(key) {
            if last_used.elapsed() < self.session_ttl {
                return Ok(Some(SessionHolder::new(key.clone())));
            }
        }
        Ok(None)
    }

    pub async fn store_session(&self, key: SessionKey, _session: SessionHolder) {
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
        Self {
            key,
            created_at: Instant::now(),
        }
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
