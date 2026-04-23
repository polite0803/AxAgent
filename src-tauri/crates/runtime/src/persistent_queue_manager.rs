use std::sync::Arc;
use std::time::Duration;

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::persistent_queue::{Entity as QueueEntity, Model as QueueModel, Column as QueueColumn};
use crate::message_gateway::{AgentMessage, MessagePayload};

pub struct PersistentMessageQueue {
    db: Arc<DatabaseConnection>,
    memory_queue: Arc<RwLock<Vec<AgentMessage>>>,
    max_batch_size: usize,
    persist_interval: Duration,
}

impl PersistentMessageQueue {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db: Arc::new(db),
            memory_queue: Arc::new(RwLock::new(Vec::new())),
            max_batch_size: 100,
            persist_interval: Duration::from_secs(5),
        }
    }

    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    pub fn with_persist_interval(mut self, interval: Duration) -> Self {
        self.persist_interval = interval;
        self
    }

    pub async fn enqueue(&self, message: AgentMessage) -> Result<(), QueueError> {
        let mut queue = self.memory_queue.write().await;
        queue.push(message.clone());

        if queue.len() >= self.max_batch_size {
            drop(queue);
            self.flush_to_disk().await?;
        }

        Ok(())
    }

    pub async fn enqueue_to_persistent(&self, message: &AgentMessage) -> Result<(), QueueError> {
        let payload_json = serde_json::to_string(&message.payload)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;

        let model = QueueModel {
            id: message.id.clone(),
            from_agent: message.from.clone(),
            to_agent: message.to.clone(),
            payload_type: match &message.payload {
                MessagePayload::Text { .. } => "text",
                MessagePayload::Json { .. } => "json",
                MessagePayload::Binary { .. } => "binary",
                MessagePayload::Command { .. } => "command",
                MessagePayload::Event { .. } => "event",
                MessagePayload::Response { .. } => "response",
                MessagePayload::Error { .. } => "error",
            }.to_string(),
            payload: payload_json,
            status: "pending".to_string(),
            retry_count: 0,
            max_retries: 3,
            created_at: message.timestamp as i64,
            updated_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            correlation_id: message.correlation_id.clone(),
            reply_to: message.reply_to.clone(),
        };

        model.insert(self.db.as_ref()).await.map_err(|e| QueueError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn dequeue(&self, agent_id: &str) -> Result<Option<AgentMessage>, QueueError> {
        let queue = self.memory_queue.read().await;

        if let Some(pos) = queue.iter().position(|m| m.to == agent_id) {
            let message = queue[pos].clone();
            drop(queue);

            let mut queue = self.memory_queue.write().await;
            queue.remove(pos);

            return Ok(Some(message));
        }

        Ok(None)
    }

    pub async fn dequeue_from_persistent(&self, agent_id: &str) -> Result<Option<AgentMessage>, QueueError> {
        let pending: Vec<QueueModel> = QueueEntity::find()
            .filter(QueueColumn::ToAgent.eq(agent_id))
            .filter(QueueColumn::Status.eq("pending"))
            .order_by_asc(QueueColumn::CreatedAt)
            .limit(1)
            .all(self.db.as_ref())
            .await
            .map_err(|e| QueueError::Database(e.to_string()))?;

        if let Some(item) = pending.into_iter().next() {
            let payload: MessagePayload = serde_json::from_str(&item.payload)
                .map_err(|e| QueueError::Serialization(e.to_string()))?;

            let message = AgentMessage {
                id: item.id,
                from: item.from_agent,
                to: item.to_agent,
                payload,
                timestamp: item.created_at as u128,
                correlation_id: item.correlation_id,
                reply_to: item.reply_to,
            };

            let _ = QueueEntity::delete_by_id(&message.id)
                .exec(self.db.as_ref())
                .await;

            return Ok(Some(message));
        }

        Ok(None)
    }

    pub async fn flush_to_disk(&self) -> Result<(), QueueError> {
        let queue = self.memory_queue.read().await;

        for msg in queue.iter() {
            if let Err(e) = self.enqueue_to_persistent(msg).await {
                tracing::warn!("Failed to persist message {}: {}", msg.id, e);
            }
        }

        Ok(())
    }

    pub async fn start_background_persistence(&self) {
        let db = self.db.clone();
        let queue = self.memory_queue.clone();
        let interval = self.persist_interval;

        tokio::spawn(async move {
            let mut ticker = interval(interval);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let messages: Vec<AgentMessage> = {
                            let q = queue.read().await;
                            q.clone()
                        };

                        if messages.is_empty() {
                            continue;
                        }

                        for msg in messages {
                            let payload_json = match serde_json::to_string(&msg.payload) {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::warn!("Failed to serialize message payload: {}", e);
                                    continue;
                                }
                            };

                            let model = QueueModel {
                                id: msg.id.clone(),
                                from_agent: msg.from.clone(),
                                to_agent: msg.to.clone(),
                                payload_type: match &msg.payload {
                                    MessagePayload::Text { .. } => "text",
                                    MessagePayload::Json { .. } => "json",
                                    MessagePayload::Binary { .. } => "binary",
                                    MessagePayload::Command { .. } => "command",
                                    MessagePayload::Event { .. } => "event",
                                    MessagePayload::Response { .. } => "response",
                                    MessagePayload::Error { .. } => "error",
                                }.to_string(),
                                payload: payload_json,
                                status: "pending".to_string(),
                                retry_count: 0,
                                max_retries: 3,
                                created_at: msg.timestamp as i64,
                                updated_at: chrono::Utc::now().timestamp(),
                                expires_at: None,
                                correlation_id: msg.correlation_id.clone(),
                                reply_to: msg.reply_to.clone(),
                            };

                            if let Err(e) = model.insert(db.as_ref()).await {
                                tracing::warn!("Failed to persist message: {}", e);
                            }
                        }

                        let mut q = queue.write().await;
                        q.retain(|m| {
                            m.timestamp > chrono::Utc::now().timestamp() as u128 - 60
                        });
                    }
                }
            }
        });
    }

    pub async fn get_queue_depth(&self, agent_id: &str) -> Result<usize, QueueError> {
        let memory_count = {
            let queue = self.memory_queue.read().await;
            queue.iter().filter(|m| m.to == agent_id).count()
        };

        let db_count: i64 = QueueEntity::find()
            .filter(QueueColumn::ToAgent.eq(agent_id))
            .filter(QueueColumn::Status.eq("pending"))
            .count(self.db.as_ref())
            .await
            .map_err(|e| QueueError::Database(e.to_string()))?;

        Ok(memory_count + db_count as usize)
    }

    pub async fn mark_failed(&self, message_id: &str) -> Result<(), QueueError> {
        let pending: Option<QueueModel> = QueueEntity::find_by_id(message_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| QueueError::Database(e.to_string()))?;

        if let Some(mut item) = pending {
            item.retry_count += 1;
            item.updated_at = chrono::Utc::now().timestamp();

            if item.retry_count >= item.max_retries {
                item.status = "failed".to_string();
            } else {
                item.status = "retry".to_string();
            }

            item.update(self.db.as_ref()).await.map_err(|e| QueueError::Database(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn retry_pending(&self) -> Result<Vec<AgentMessage>, QueueError> {
        let to_retry: Vec<QueueModel> = QueueEntity::find()
            .filter(QueueColumn::Status.eq("retry"))
            .filter(QueueColumn::RetryCount.lt(QueueColumn::MaxRetries))
            .order_by_asc(QueueColumn::CreatedAt)
            .limit(100)
            .all(self.db.as_ref())
            .await
            .map_err(|e| QueueError::Database(e.to_string()))?;

        let mut messages = Vec::new();

        for item in to_retry {
            let payload: MessagePayload = serde_json::from_str(&item.payload)
                .map_err(|e| QueueError::Serialization(e.to_string()))?;

            let message = AgentMessage {
                id: item.id.clone(),
                from: item.from_agent,
                to: item.to_agent,
                payload,
                timestamp: item.created_at as u128,
                correlation_id: item.correlation_id,
                reply_to: item.reply_to,
            };

            let mut model: QueueModel = item.clone();
            model.status = "pending".to_string();
            model.update(self.db.as_ref()).await.map_err(|e| QueueError::Database(e.to_string()))?;

            messages.push(message);
        }

        Ok(messages)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Queue empty")]
    Empty,
}