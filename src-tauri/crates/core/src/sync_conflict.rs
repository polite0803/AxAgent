/// Conflict detection and resolution for cloud workspace sync.
///
/// Architecture:
/// - Each file tracked by: key, local_etag, remote_etag, timestamps, content_hash
/// - Conflict detected when both local and remote change since last sync
/// - Tombstone records track deletions across devices
/// - Atomic operations use conditional PUT/DELETE (If-Match header)
/// - Conflicts resolved via strategies: latest_wins, local_wins, remote_wins, manual
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ─── File Entry with Full Tracking ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedFileEntry {
    /// Relative file key (path within workspace)
    pub key: String,
    /// ETag of the file as seen on the cloud at last sync
    pub last_sync_remote_etag: Option<String>,
    /// ETag of the file as seen locally at last sync (content hash for local files)
    pub last_sync_local_hash: Option<String>,
    /// Size in bytes
    pub size: i64,
    /// Local file modification timestamp (Unix epoch ms)
    pub local_modified_at: u64,
    /// Remote modification timestamp as seen at last sync (Unix epoch ms)
    pub last_sync_remote_modified_at: Option<u64>,
    /// Whether this file was locally deleted since last sync
    pub locally_deleted: bool,
    /// Whether this file was remotely deleted (tombstone)
    pub tombstoned: bool,
    /// Conflict metadata if detected
    pub conflict: Option<ConflictInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    /// Conflict type
    pub kind: ConflictKind,
    /// Detected at timestamp
    pub detected_at: u64,
    /// Local version metadata
    pub local_version: ConflictVersion,
    /// Remote version metadata
    pub remote_version: ConflictVersion,
    /// Resolution status
    pub resolved: bool,
    /// How it was resolved (if resolved)
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Both local and remote modified the file since last sync
    BothModified,
    /// Local modified but remote deleted
    ModifiedVsDeleted,
    /// Local deleted but remote modified
    DeletedVsModified,
    /// Same file created on both ends with different content
    BothCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictVersion {
    pub size: i64,
    pub hash: String,
    pub modified_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Keep the local version, overwrite remote
    KeepLocal,
    /// Keep the remote version, overwrite local
    KeepRemote,
    /// Keep the newer version based on timestamp
    KeepNewer,
    /// Keep both, rename local as conflict copy
    KeepBoth,
    /// User manually resolved
    Manual,
}

/// User-configurable conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Keep whichever version has the latest modification time
    #[default]
    LatestWins,
    /// Always prefer the local version
    LocalWins,
    /// Always prefer the remote (cloud) version
    RemoteWins,
    /// Mark as conflict and let user decide (creates .conflict file)
    Manual,
}

// ─── Tombstone (Deletion Tracking) ───────────────────────────────────

/// Records a deletion event so it can be propagated to other devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    /// The file key that was deleted
    pub key: String,
    /// Timestamp when deletion occurred (Unix epoch ms)
    pub deleted_at: u64,
    /// Device that performed the deletion
    pub deleted_by_device: String,
    /// ETag of the file before it was deleted (for verification)
    pub last_etag: Option<String>,
}

// ─── Sync State ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    /// Schema version
    pub version: u32,
    /// Device identifier
    pub device_id: String,
    /// Workspace URI this state belongs to
    pub workspace_uri: String,
    /// Last successful full sync timestamp (RFC3339)
    pub last_sync_at: Option<String>,
    /// Monotonically increasing sync version counter
    pub sync_version: u64,
    /// Tracked files (key -> entry)
    pub files: HashMap<String, TrackedFileEntry>,
    /// Deletion tombstones
    pub tombstones: Vec<Tombstone>,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Pending conflicts that need resolution
    pub pending_conflicts: usize,
}

impl SyncState {
    pub fn new(device_id: String, workspace_uri: String) -> Self {
        Self {
            version: 2,
            device_id,
            workspace_uri,
            last_sync_at: None,
            sync_version: 0,
            files: HashMap::new(),
            tombstones: Vec::new(),
            conflict_strategy: ConflictStrategy::default(),
            pending_conflicts: 0,
        }
    }

    pub fn get_entry(&self, key: &str) -> Option<&TrackedFileEntry> {
        self.files.get(key)
    }

    pub fn upsert_entry(&mut self, key: String, entry: TrackedFileEntry) {
        self.files.insert(key, entry);
    }

    pub fn remove_entry(&mut self, key: &str) -> Option<TrackedFileEntry> {
        self.files.remove(key)
    }

    pub fn add_tombstone(&mut self, key: String, last_etag: Option<String>) {
        // Remove existing tombstone for same key (keep latest)
        self.tombstones.retain(|t| t.key != key);
        self.tombstones.push(Tombstone {
            key,
            deleted_at: current_epoch_ms(),
            deleted_by_device: self.device_id.clone(),
            last_etag,
        });
    }

    pub fn is_tombstoned(&self, key: &str) -> bool {
        self.tombstones.iter().any(|t| t.key == key)
    }

    pub fn count_conflicts(&self) -> usize {
        self.files
            .values()
            .filter(|e| e.conflict.is_some() && !e.conflict.as_ref().unwrap().resolved)
            .count()
    }

    /// Clean up old tombstones (older than 30 days)
    pub fn prune_old_tombstones(&mut self) {
        let cutoff = current_epoch_ms() - (30 * 24 * 60 * 60 * 1000);
        self.tombstones.retain(|t| t.deleted_at > cutoff);
    }
}

// ─── Sync Report ─────────────────────────────────────────────────────

/// Detailed report of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    /// Files downloaded from cloud
    pub downloaded: Vec<String>,
    /// Files uploaded to cloud
    pub uploaded: Vec<String>,
    /// Files deleted locally (synced to cloud)
    pub local_deletions_synced: Vec<String>,
    /// Files deleted from cloud (synced locally)
    pub remote_deletions_synced: Vec<String>,
    /// Conflicts detected
    pub conflicts_detected: Vec<ConflictSummary>,
    /// Conflicts auto-resolved
    pub conflicts_resolved: Vec<ConflictSummary>,
    /// Conflicts pending user action
    pub pending_conflicts: Vec<ConflictSummary>,
    /// Total duration in ms
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSummary {
    pub key: String,
    pub kind: ConflictKind,
    pub resolution: Option<ConflictResolution>,
    pub local_size: i64,
    pub remote_size: i64,
    pub local_modified_at: u64,
    pub remote_modified_at: u64,
}

// ─── Utility Functions ───────────────────────────────────────────────

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute SHA256 hash of file content for local change detection.
pub fn compute_content_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Parse RFC3339 timestamp to epoch ms.
pub fn parse_rfc3339_to_ms(ts: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis() as u64)
}

/// Convert epoch ms to RFC3339.
pub fn epoch_ms_to_rfc3339(ms: u64) -> String {
    let secs = ms / 1000;
    let nsecs = (ms % 1000) * 1_000_000;
    chrono::DateTime::from_timestamp(secs as i64, nsecs as u32)
        .unwrap_or_default()
        .to_rfc3339()
}

/// Get current time as RFC3339 string.
pub fn current_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
