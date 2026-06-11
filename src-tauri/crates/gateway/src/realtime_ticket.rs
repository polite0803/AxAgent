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

use parking_lot::Mutex;
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
/// Backed by a `parking_lot::Mutex<HashMap>` because ticket operations are
/// short-lived and low-contention (per WS upgrade), and the critical section
/// is purely synchronous hashmap work — `tokio::sync::Mutex` would be wrong
/// here. A single global store per gateway instance is the intended
/// deployment shape.
///
/// A background tokio task is spawned in [`TicketStore::new`] that sweeps
/// expired tickets out of the map every [`SWEEP_INTERVAL`] seconds so the
/// store cannot grow unbounded under attack (a valid Bearer spamming
/// `POST /v1/realtime-ticket` without ever consuming the tickets).
#[derive(Clone)]
pub struct TicketStore {
    ttl: Duration,
    inner: Arc<Mutex<HashMap<String, Ticket>>>,
}

/// How often the background sweeper wakes up. 10s is plenty given the 30s
/// default TTL — a ticket can be at most ~10s past its expiry before it is
/// reaped, and `consume` rejects expired tickets independently.
const SWEEP_INTERVAL: Duration = Duration::from_secs(10);

impl TicketStore {
    pub fn new(ttl: Duration) -> Self {
        let inner = Arc::new(Mutex::new(HashMap::new()));
        Self::spawn_sweeper(inner.clone(), SWEEP_INTERVAL);
        Self { ttl, inner }
    }

    /// Issue a new ticket bound to `key_id`. The returned `Ticket.ticket_id`
    /// is what the client uses on the WS upgrade URL.
    pub async fn issue(&self, key_id: impl Into<String>) -> Ticket {
        let ticket = Ticket {
            ticket_id: Uuid::new_v4().to_string(),
            key_id: key_id.into(),
            expires_at: Instant::now() + self.ttl,
        };
        let mut map = self.inner.lock();
        map.insert(ticket.ticket_id.clone(), ticket.clone());
        ticket
    }

    /// Consume a ticket by id. Returns `None` if the ticket is unknown,
    /// already consumed, or expired. Successful consumption removes the
    /// ticket from the store (single-use).
    ///
    /// Kept `async` for public-API stability, even though the internal lock
    /// is synchronous.
    pub async fn consume(&self, ticket_id: &str) -> Option<Ticket> {
        let mut map = self.inner.lock();
        let ticket = map.remove(ticket_id)?;
        if ticket.expires_at < Instant::now() {
            return None;
        }
        Some(ticket)
    }

    /// Drop every ticket whose `expires_at` is in the past. Public so tests
    /// can trigger a sweep without waiting for the background interval.
    pub async fn sweep_now(&self) -> usize {
        Self::sweep(&self.inner)
    }

    fn sweep(inner: &Arc<Mutex<HashMap<String, Ticket>>>) -> usize {
        let now = Instant::now();
        let mut map = inner.lock();
        let before = map.len();
        map.retain(|_, t| t.expires_at >= now);
        let removed = before - map.len();
        if removed > 0 {
            tracing::debug!(removed, "swept expired realtime tickets");
        }
        removed
    }

    fn spawn_sweeper(inner: Arc<Mutex<HashMap<String, Ticket>>>, interval: Duration) {
        tokio::spawn(async move {
            // First tick fires immediately; skip it so we don't sweep an
            // empty store on startup.
            let mut tick = tokio::time::interval(interval);
            tick.tick().await;
            loop {
                tick.tick().await;
                Self::sweep(&inner);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ticket_single_use() {
        let store = TicketStore::new(Duration::from_secs(30));
        let ticket = store.issue("key-1").await;
        assert!(store.consume(&ticket.ticket_id).await.is_some());
        // 第二次消费必须失败
        assert!(store.consume(&ticket.ticket_id).await.is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ticket_expires() {
        // 200ms TTL + 300ms sleep — generous for CI jitter, while still
        // keeping the test well under a second.
        let store = TicketStore::new(Duration::from_millis(200));
        let ticket = store.issue("key-1").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(store.consume(&ticket.ticket_id).await.is_none());
    }

    #[tokio::test]
    async fn ticket_preserves_key_id_and_isolates_consumption() {
        let store = TicketStore::new(Duration::from_secs(30));
        let t1 = store.issue("key-1").await;
        let t2 = store.issue("key-2").await;

        // Consuming t1 must NOT take t2 with it.
        let consumed1 = store.consume(&t1.ticket_id).await.unwrap();
        assert_eq!(consumed1.key_id, "key-1");
        assert!(store.consume(&t1.ticket_id).await.is_none(), "t1 must be single-use");
        assert!(
            store.consume(&t2.ticket_id).await.is_some(),
            "t2 must remain consumable after t1 is gone"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ticket_sweeper_removes_expired() {
        let store = TicketStore::new(Duration::from_millis(50));
        let ticket = store.issue("key-1").await;
        // Wait past TTL.
        tokio::time::sleep(Duration::from_millis(80)).await;
        // Trigger sweep manually so the test doesn't have to wait the full
        // 10s background interval.
        let removed = store.sweep_now().await;
        assert!(removed >= 1, "sweep should have removed at least one ticket");
        assert!(
            store.consume(&ticket.ticket_id).await.is_none(),
            "expired ticket should be swept"
        );
    }
}
