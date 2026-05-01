use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionLevel {
    Read,
    Write,
    ReadWrite,
    Temp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAuthorization {
    pub id: String,
    pub path: PathBuf,
    pub level: PermissionLevel,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
    pub auto_renew: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub path: String,
    pub level: PermissionLevel,
    pub reason: String,
    pub duration_minutes: Option<i64>,
    pub auto_renew: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResponse {
    pub authorized: bool,
    pub auth_id: Option<String>,
    pub path: String,
    pub level: PermissionLevel,
    pub expires_at: Option<String>,
    pub message: String,
}

pub struct FileAuthorizer {
    authorizations: Mutex<HashMap<String, FileAuthorization>>,
    pending_requests: Mutex<Vec<AuthorizationRequest>>,
    max_temp_duration: Duration,
    default_duration: Duration,
}

impl FileAuthorizer {
    pub fn new() -> Self {
        Self {
            authorizations: Mutex::new(HashMap::new()),
            pending_requests: Mutex::new(Vec::new()),
            max_temp_duration: Duration::hours(24),
            default_duration: Duration::minutes(30),
        }
    }

    pub fn request_authorization(&self, request: AuthorizationRequest) -> AuthorizationResponse {
        let path = PathBuf::from(&request.path);

        if !self.is_path_safe(&path) {
            return AuthorizationResponse {
                authorized: false,
                auth_id: None,
                path: request.path,
                level: request.level,
                expires_at: None,
                message: "Path traversal or unsafe path detected".to_string(),
            };
        }

        let duration = request
            .duration_minutes
            .map(|m| Duration::minutes(m).min(self.max_temp_duration))
            .unwrap_or(self.default_duration);

        let expires_at = Utc::now() + duration;

        let auth = FileAuthorization {
            id: Uuid::new_v4().to_string(),
            path: path.clone(),
            level: request.level.clone(),
            created_at: Utc::now(),
            expires_at: Some(expires_at),
            reason: request.reason,
            auto_renew: request.auto_renew,
        };

        let auth_id = auth.id.clone();
        {
            let mut authorizations = self.authorizations.lock().unwrap();
            authorizations.insert(auth_id.clone(), auth);
        }

        AuthorizationResponse {
            authorized: true,
            auth_id: Some(auth_id),
            path: request.path,
            level: request.level,
            expires_at: Some(expires_at.to_rfc3339()),
            message: "Authorization granted".to_string(),
        }
    }

    pub fn check_authorization(&self, path: &str, required_level: &PermissionLevel) -> bool {
        let path = PathBuf::from(path);
        let authorizations = self.authorizations.lock().unwrap();

        for auth in authorizations.values() {
            if auth.path == path && !self.is_expired(auth) {
                return self.has_required_level(&auth.level, required_level);
            }
        }
        false
    }

    pub fn revoke_authorization(&self, auth_id: &str) -> bool {
        let mut authorizations = self.authorizations.lock().unwrap();
        authorizations.remove(auth_id).is_some()
    }

    pub fn revoke_all_for_path(&self, path: &str) -> usize {
        let path = PathBuf::from(path);
        let mut authorizations = self.authorizations.lock().unwrap();
        let before = authorizations.len();
        authorizations.retain(|_, auth| auth.path != path);
        before - authorizations.len()
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut authorizations = self.authorizations.lock().unwrap();
        let before = authorizations.len();
        authorizations.retain(|_, auth| !self.is_expired(auth));
        before - authorizations.len()
    }

    pub fn list_authorizations(&self) -> Vec<FileAuthorization> {
        let authorizations = self.authorizations.lock().unwrap();
        authorizations.values().cloned().collect()
    }

    pub fn get_authorization(&self, auth_id: &str) -> Option<FileAuthorization> {
        let authorizations = self.authorizations.lock().unwrap();
        authorizations.get(auth_id).cloned()
    }

    pub fn renew_authorization(&self, auth_id: &str, additional_minutes: i64) -> bool {
        let mut authorizations = self.authorizations.lock().unwrap();
        if let Some(auth) = authorizations.get_mut(auth_id) {
            if !auth.auto_renew {
                return false;
            }
            let additional = Duration::minutes(additional_minutes).min(self.max_temp_duration);
            auth.expires_at = Some(Utc::now() + additional);
            true
        } else {
            false
        }
    }

    fn is_expired(&self, auth: &FileAuthorization) -> bool {
        if let Some(expires_at) = auth.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    fn has_required_level(&self, granted: &PermissionLevel, required: &PermissionLevel) -> bool {
        matches!(
            (granted, required),
            (PermissionLevel::ReadWrite, _)
                | (PermissionLevel::Write, PermissionLevel::Write)
                | (PermissionLevel::Write, PermissionLevel::Read)
                | (PermissionLevel::Read, PermissionLevel::Read)
                | (PermissionLevel::Temp, PermissionLevel::Read)
        )
    }

    fn is_path_safe(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        if path_str.contains("..") || path_str.starts_with("~") {
            return false;
        }

        // 防止符号链接遍历攻击：检查路径本身不是符号链接
        // canonicalize() 会跟随符号链接，攻击者可创建 legit_dir -> /etc/passwd 绕过
        if path.is_symlink() {
            return false;
        }

        let normalized = std::fs::canonicalize(path).ok();
        if let Some(normalized) = normalized {
            let normalized_str = normalized.to_string_lossy();
            if normalized_str.contains("..") {
                return false;
            }
        }

        true
    }

    pub fn add_pending_request(&self, request: AuthorizationRequest) {
        let mut pending = self.pending_requests.lock().unwrap();
        pending.push(request);
    }

    pub fn get_pending_requests(&self) -> Vec<AuthorizationRequest> {
        let pending = self.pending_requests.lock().unwrap();
        pending.clone()
    }

    pub fn clear_pending_requests(&self) {
        let mut pending = self.pending_requests.lock().unwrap();
        pending.clear();
    }
}

impl Default for FileAuthorizer {
    fn default() -> Self {
        Self::new()
    }
}
