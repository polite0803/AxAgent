//! Single-use, short-lived tickets for `/v1/realtime` WebSocket auth.
//!
//! Background (SECURITY P0-2.2):
//! The WebSocket upgrade URL must not contain a long-lived API key, because the URL
//! is logged by proxies, may appear in `Referer` headers, and lives in browser
//! history. Clients exchange a Bearer token for a short-lived ticket via
//! `POST /v1/realtime-ticket`, then pass the ticket as a query parameter on
//! `GET /v1/realtime?ticket=...`. The ticket is consumed on first use and
//! expires after [`TicketStore::ttl`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use uuid::Uuid;

/// A single-use ticket. Clone-able so we can hand the id back to the caller
/// while keeping the full record in the store.
#[derive(Clone, Debug)]
pub struct Ticket {
    pub ticket_id: String,
    pub key_id: String,
    pub expires_at: Instant,
}

/// In-memory store for realtime WebSocket auth tickets.
///
/// Backed by an async `Mutex<HashMap>` because ticket operations are
/// short-lived and low-contention (per WS upgrade). A single global store
/// per gateway instance is the intended deployment shape.
#[derive(Clone)]
pub struct TicketStore {
    ttl: Duration,
    inner: Arc<Mutex<HashMap<String, Ticket>>>,
}

impl TicketStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Issue a new ticket bound to `key_id`. The returned `Ticket.ticket_id`
    /// is what the client uses on the WS upgrade URL.
    pub async fn issue(&self, key_id: impl Into<String>) -> Ticket {
        let ticket = Ticket {
            ticket_id: Uuid::new_v4().to_string(),
            key_id: key_id.into(),
            expires_at: Instant::now() + self.ttl,
        };
        let mut map = self.inner.lock().await;
        map.insert(ticket.ticket_id.clone(), ticket.clone());
        ticket
    }

    /// Consume a ticket by id. Returns `None` if the ticket is unknown,
    /// already consumed, or expired. Successful consumption removes the
    /// ticket from the store (single-use).
    pub async fn consume(&self, ticket_id: &str) -> Option<Ticket> {
        let mut map = self.inner.lock().await;
        let ticket = map.remove(ticket_id)?;
        if ticket.expires_at < Instant::now() {
            return None;
        }
        Some(ticket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn ticket_single_use() {
        let store = TicketStore::new(Duration::from_secs(30));
        let ticket = store.issue("key-1").await;
        assert!(store.consume(&ticket.ticket_id).await.is_some());
        // 第二次消费必须失败
        assert!(store.consume(&ticket.ticket_id).await.is_none());
    }

    #[tokio::test]
    async fn ticket_expires() {
        let store = TicketStore::new(Duration::from_millis(50));
        let ticket = store.issue("key-1").await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(store.consume(&ticket.ticket_id).await.is_none());
    }

    #[tokio::test]
    async fn ticket_different_keys_isolated() {
        let store = TicketStore::new(Duration::from_secs(30));
        let t1 = store.issue("key-1").await;
        let t2 = store.issue("key-2").await;
        let consumed1 = store.consume(&t1.ticket_id).await.unwrap();
        let consumed2 = store.consume(&t2.ticket_id).await.unwrap();
        assert_eq!(consumed1.key_id, "key-1");
        assert_eq!(consumed2.key_id, "key-2");
    }
}
