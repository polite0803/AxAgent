use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, OwnedSemaphorePermit, Semaphore};
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
    conn: C,
    pool: Arc<ConnectionPool<C>>,
    created_at: Instant,
    last_used: Instant,
    is_valid: bool,
}

impl<C> PooledConnection<C> {
    pub fn new(conn: C, pool: Arc<ConnectionPool<C>>) -> Self {
        let now = Instant::now();
        Self {
            conn,
            pool,
            created_at: now,
            last_used: now,
            is_valid: true,
        }
    }

    pub fn get_ref(&self) -> &C {
        &self.conn
    }

    pub fn get_mut(&mut self) -> &mut C {
        &mut self.conn
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
        if self.is_valid {
            let mut conn = PooledConnection {
                conn: std::mem::replace(&mut self.conn, panic!("connection already taken")),
                pool: self.pool.clone(),
                created_at: self.created_at,
                last_used: self.last_used,
                is_valid: true,
            };
            conn.last_used = Instant::now();

            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.release(conn).await;
            });
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
    factory: ConnectionFactory<C>,
}

impl<C: Send + 'static> ConnectionPool<C> {
    pub fn new(config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        Self {
            config,
            connections: Arc::new(RwLock::new(Vec::new())),
            total_count: Arc::new(RwLock::new(0)),
            semaphore,
            factory: Arc::new(Box::new(|| panic!("No factory configured")) as Box<dyn Fn() -> C + Send + 'static>),
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
        let factory = self.factory.clone();
        tokio::task::spawn_blocking(move || (factory)())
            .await
            .map_err(|_| PoolError::CreationFailed("Connection creation task panicked".to_string()))
    }

    async fn release(&self, mut conn: PooledConnection<C>) {
        if !conn.is_valid {
            let mut count = self.total_count.write().await;
            *count = count.saturating_sub(1);
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            drop(permit);
            return;
        }

        let mut connections = self.connections.write().await;
        if connections.len() < self.config.max_idle {
            connections.push(PooledConnectionInner {
                conn: conn.conn,
                created_at: conn.created_at,
                last_used: conn.last_used,
            });
        } else {
            let mut count = self.total_count.write().await;
            *count = count.saturating_sub(1);
        }

        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        drop(permit);
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
        Arc::new(ConnectionPool {
            config: self.pool.config.clone(),
            connections: self.pool.connections.clone(),
            total_count: self.pool.total_count.clone(),
            semaphore: self.pool.semaphore.clone(),
            factory: Arc::new(Box::new(self.maker.take().expect("Builder already used")) as Box<dyn Fn() -> C + Send + 'static>),
        })
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