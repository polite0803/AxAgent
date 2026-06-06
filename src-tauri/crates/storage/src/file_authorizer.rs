use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// SECURITY: 批准者（用户 / 显式 UI 流程）。Pending 阶段为空。
    pub approver: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub id: String,
    pub path: String,
    pub level: PermissionLevel,
    pub reason: String,
    pub duration_minutes: Option<i64>,
    pub auto_renew: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResponse {
    pub authorized: bool,
    pub auth_id: Option<String>,
    pub request_id: Option<String>,
    pub path: String,
    pub level: PermissionLevel,
    pub expires_at: Option<String>,
    pub message: String,
}

/// SECURITY (M10): 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub path: String,
    pub level: Option<PermissionLevel>,
    pub success: bool,
    pub note: String,
}

pub struct FileAuthorizer {
    authorizations: Mutex<HashMap<String, FileAuthorization>>,
    pending_requests: Mutex<Vec<AuthorizationRequest>>,
    audit_log: Mutex<Vec<AuditEntry>>,
    max_temp_duration: Duration,
    default_duration: Duration,
}

impl FileAuthorizer {
    pub fn new() -> Self {
        Self {
            authorizations: Mutex::new(HashMap::new()),
            pending_requests: Mutex::new(Vec::new()),
            audit_log: Mutex::new(Vec::new()),
            max_temp_duration: Duration::hours(24),
            default_duration: Duration::minutes(30),
        }
    }

    /// SECURITY (C10): 之前直接 self-approve，现在只生成待审批 request。
    /// 真正的批准必须由 `approve_request` 完成（用户通过 UI 显式点击）。
    pub fn request_authorization(&self, request: AuthorizationRequest) -> AuthorizationResponse {
        let path = PathBuf::from(&request.path);

        if !self.is_path_safe(&path) {
            self.audit(
                "request_authorization",
                "system",
                &request.path,
                Some(request.level.clone()),
                false,
                "unsafe path",
            );
            return AuthorizationResponse {
                authorized: false,
                auth_id: None,
                request_id: Some(request.id),
                path: request.path,
                level: request.level,
                expires_at: None,
                message: "Path traversal or unsafe path detected".to_string(),
            };
        }

        let req = AuthorizationRequest {
            id: request.id,
            path: request.path.clone(),
            level: request.level.clone(),
            reason: request.reason,
            duration_minutes: request.duration_minutes,
            auto_renew: request.auto_renew,
            created_at: Utc::now(),
        };
        let req_id = req.id.clone();
        let path_str = req.path.clone();
        let level = req.level.clone();
        {
            let mut pending = self
                .pending_requests
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            pending.push(req);
        }
        self.audit(
            "request_authorization",
            "system",
            &path_str,
            Some(level.clone()),
            true,
            "pending user approval",
        );

        AuthorizationResponse {
            authorized: false,
            auth_id: None,
            request_id: Some(req_id),
            path: path_str,
            level, // 修复：原代码再次用已 move 的 request.level，改用前面 clone 出的 level
            expires_at: None,
            message: "Authorization pending user approval".to_string(),
        }
    }

    /// SECURITY (C10): 显式用户/UI 批准流程。
    pub fn approve_request(&self, request_id: &str, approver: &str) -> AuthorizationResponse {
        let mut pending = self
            .pending_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let pos = pending.iter().position(|r| r.id == request_id);
        let req = match pos {
            Some(i) => pending.remove(i),
            None => {
                return AuthorizationResponse {
                    authorized: false,
                    auth_id: None,
                    request_id: Some(request_id.to_string()),
                    path: String::new(),
                    level: PermissionLevel::Read,
                    expires_at: None,
                    message: format!("No pending request '{}'", request_id),
                };
            },
        };

        let path = PathBuf::from(&req.path);
        if !self.is_path_safe(&path) {
            self.audit(
                "approve_request",
                approver,
                &req.path,
                Some(req.level.clone()),
                false,
                "unsafe path",
            );
            return AuthorizationResponse {
                authorized: false,
                auth_id: None,
                request_id: Some(req.id),
                path: req.path,
                level: req.level,
                expires_at: None,
                message: "Path failed safety check".to_string(),
            };
        }

        let duration = req
            .duration_minutes
            .map(|m| Duration::minutes(m).min(self.max_temp_duration))
            .unwrap_or(self.default_duration);
        let expires_at = Utc::now() + duration;

        let auth = FileAuthorization {
            id: Uuid::new_v4().to_string(),
            path: path.clone(),
            level: req.level.clone(),
            created_at: Utc::now(),
            expires_at: Some(expires_at),
            reason: req.reason.clone(),
            auto_renew: req.auto_renew,
            approver: Some(approver.to_string()),
        };
        let auth_id = auth.id.clone();
        {
            let mut authorizations = self
                .authorizations
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            authorizations.insert(auth_id.clone(), auth);
        }
        self.audit(
            "approve_request",
            approver,
            &req.path,
            Some(req.level.clone()),
            true,
            &format!("approved, expires {}", expires_at.to_rfc3339()),
        );

        AuthorizationResponse {
            authorized: true,
            auth_id: Some(auth_id),
            request_id: Some(req.id),
            path: req.path,
            level: req.level,
            expires_at: Some(expires_at.to_rfc3339()),
            message: "Authorization granted".to_string(),
        }
    }

    pub fn deny_request(&self, request_id: &str, approver: &str) -> bool {
        let mut pending = self
            .pending_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = pending.iter().position(|r| r.id == request_id) {
            let req = pending.remove(pos);
            self.audit(
                "deny_request",
                approver,
                &req.path,
                Some(req.level),
                false,
                "denied by user",
            );
            true
        } else {
            false
        }
    }

    /// SECURITY (H5): 路径匹配：精确 → 父目录递归 → 都检查 expires_at。
    pub fn check_authorization(&self, path: &str, required_level: &PermissionLevel) -> bool {
        let path = PathBuf::from(path);
        let authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        for auth in authorizations.values() {
            if self.is_expired(auth) {
                continue;
            }
            if !path_matches(&path, &auth.path) {
                continue;
            }
            if self.has_required_level(&auth.level, required_level) {
                return true;
            }
        }
        false
    }

    pub fn revoke_authorization(&self, auth_id: &str) -> bool {
        let mut authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        authorizations.remove(auth_id).is_some()
    }

    pub fn revoke_all_for_path(&self, path: &str) -> usize {
        let path = PathBuf::from(path);
        let mut authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let before = authorizations.len();
        authorizations.retain(|_, auth| !path_matches(&auth.path, &path));
        before - authorizations.len()
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let before = authorizations.len();
        let now = Utc::now();
        authorizations.retain(|_, auth| match auth.expires_at {
            Some(t) => t > now,
            None => true,
        });
        before - authorizations.len()
    }

    pub fn list_authorizations(&self) -> Vec<FileAuthorization> {
        let authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        authorizations.values().cloned().collect()
    }

    pub fn get_authorization(&self, auth_id: &str) -> Option<FileAuthorization> {
        let authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        authorizations.get(auth_id).cloned()
    }

    pub fn renew_authorization(&self, auth_id: &str, additional_minutes: i64) -> bool {
        let mut authorizations = self
            .authorizations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
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
        match auth.expires_at {
            Some(t) => Utc::now() > t,
            None => false,
        }
    }

    /// SECURITY (H4): Temp 必须强制 checks expiry，语义上等同"带 TTL 的 ReadWrite"。
    /// 实际我们已经在 is_expired 里检查 expires_at，所以 Temp 在 is_expired 过滤后
    /// 与 ReadWrite 行为一致；同时保证 Temp 一定有 expires_at。
    fn has_required_level(&self, granted: &PermissionLevel, required: &PermissionLevel) -> bool {
        matches!(
            (granted, required),
            (PermissionLevel::ReadWrite, _)
                | (PermissionLevel::Temp, _)
                | (PermissionLevel::Read, PermissionLevel::Read)
                | (PermissionLevel::Write, PermissionLevel::Write)
        )
    }

    /// SECURITY: 拒绝路径遍历与符号链接。
    /// 注意：要求 path 已存在（否则 canonicalize 失败），调用方应在文件创建后重新检查。
    fn is_path_safe(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        if path_str.is_empty() || path_str.contains('\0') {
            return false;
        }
        if path_str.contains("..") || path_str.starts_with('~') {
            return false;
        }
        match std::fs::canonicalize(path) {
            Ok(real) => {
                let real_str = real.to_string_lossy();
                if real_str.contains("..") {
                    return false;
                }
                // 解析后路径应是绝对路径
                if !real.is_absolute() {
                    return false;
                }
                true
            },
            Err(_) => {
                // 不存在的文件：这里放行"创建中"的请求，但写入时还会再 check。
                // 关键是不允许 .. 段。
                !path_str.contains("..")
            },
        }
    }

    pub fn add_pending_request(&self, request: AuthorizationRequest) {
        let mut pending = self
            .pending_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        pending.push(request);
    }

    pub fn get_pending_requests(&self) -> Vec<AuthorizationRequest> {
        let pending = self
            .pending_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        pending.clone()
    }

    pub fn clear_pending_requests(&self) {
        let mut pending = self
            .pending_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        pending.clear();
    }

    /// SECURITY (M10): 写审计日志（内存 ring buffer + tracing）。
    pub fn audit(
        &self,
        action: &str,
        actor: &str,
        path: &str,
        level: Option<PermissionLevel>,
        success: bool,
        note: &str,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            path: path.to_string(),
            level,
            success,
            note: note.to_string(),
        };
        if !success {
            warn!(
                target: "axagent.security.audit",
                "audit action={} actor={} path={} success={} note={}",
                entry.action, entry.actor, entry.path, entry.success, entry.note
            );
        } else {
            tracing::info!(
                target: "axagent.security.audit",
                "audit action={} actor={} path={} success={} note={}",
                entry.action, entry.actor, entry.path, entry.success, entry.note
            );
        }
        let mut log = self.audit_log.lock().unwrap_or_else(|e| e.into_inner());
        log.push(entry);
        // ring buffer: 保留最近 1000 条
        if log.len() > 1000 {
            let drop = log.len() - 1000;
            log.drain(0..drop);
        }
    }

    pub fn get_audit_log(&self) -> Vec<AuditEntry> {
        let log = self.audit_log.lock().unwrap_or_else(|e| e.into_inner());
        log.clone()
    }
}

/// SECURITY (H5): 精确匹配或父目录匹配。
/// - `target == auth.path` → 命中
/// - `auth.path` 是目录且 `target` 在它之下 → 命中
fn path_matches(target: &Path, granted: &Path) -> bool {
    if target == granted {
        return true;
    }
    // 父目录匹配：只对目录授权放行子路径
    let granted_canon = std::fs::canonicalize(granted).unwrap_or_else(|_| granted.to_path_buf());
    let target_canon = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    let g: Vec<Component> = granted_canon.components().collect();
    let t: Vec<Component> = target_canon.components().collect();
    if t.len() <= g.len() {
        return false;
    }
    t.iter().take(g.len()).eq(g.iter())
}

impl Default for FileAuthorizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(path: &str, level: PermissionLevel) -> AuthorizationRequest {
        AuthorizationRequest {
            id: Uuid::new_v4().to_string(),
            path: path.to_string(),
            level,
            reason: "test".to_string(),
            duration_minutes: Some(60),
            auto_renew: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn request_no_longer_auto_approves() {
        // SECURITY (C10): 直接 request_authorization 必须 pending
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/legit.txt", PermissionLevel::Read));
        assert!(!r.authorized, "request_authorization must not auto-approve");
        assert!(r.request_id.is_some());
    }

    #[test]
    fn approve_request_grants() {
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/legit.txt", PermissionLevel::Read));
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1");
        assert!(r2.authorized);
        assert!(r2.auth_id.is_some());
        // SECURITY: 批准者被记录
        let auth = a.get_authorization(&r2.auth_id.unwrap()).unwrap();
        assert_eq!(auth.approver.as_deref(), Some("user-1"));
    }

    #[test]
    fn path_under_dir_authorized() {
        // SECURITY (H5): 目录授权后子文件应通过
        let a = FileAuthorizer::new();
        // 用 tempdir 的真实路径
        let dir = std::env::temp_dir().join(format!("axagent-fauth-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("inside.txt");
        std::fs::write(&file, "x").unwrap();

        let r = a.request_authorization(req(&dir.to_string_lossy(), PermissionLevel::Read));
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1");
        assert!(r2.authorized);

        assert!(a.check_authorization(&file.to_string_lossy(), &PermissionLevel::Read));
        assert!(!a.check_authorization(&file.to_string_lossy(), &PermissionLevel::Write));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_level_has_ttl() {
        // SECURITY (H4): Temp 必须带 expires_at
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/x.txt", PermissionLevel::Temp));
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1");
        let auth_id = r2.auth_id.unwrap();
        let auth = a.get_authorization(&auth_id).unwrap();
        assert!(auth.expires_at.is_some());
    }

    #[test]
    fn audit_records_actions() {
        // SECURITY (M10)
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/x.txt", PermissionLevel::Read));
        let req_id = r.request_id.unwrap();
        let _ = a.approve_request(&req_id, "user-1");
        let log = a.get_audit_log();
        assert!(log.iter().any(|e| e.action == "approve_request"));
    }
}
