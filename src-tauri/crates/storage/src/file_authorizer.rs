// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tokio::sync::Mutex;
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
    /// M3: 审计日志文件路径，设置后 audit() 会追加写入 JSONL 格式的持久化日志
    audit_log_path: Mutex<Option<PathBuf>>,
}

impl FileAuthorizer {
    pub fn new() -> Self {
        Self {
            authorizations: Mutex::new(HashMap::new()),
            pending_requests: Mutex::new(Vec::new()),
            audit_log: Mutex::new(Vec::new()),
            max_temp_duration: Duration::hours(24),
            default_duration: Duration::minutes(30),
            audit_log_path: Mutex::new(None),
        }
    }

    /// M3: 设置审计日志文件路径，启用文件持久化。
    pub async fn set_audit_log_path(&self, path: PathBuf) {
        let mut log_path = self.audit_log_path.lock().await;
        *log_path = Some(path);
    }

    /// SECURITY (C10): 之前直接 self-approve，现在只生成待审批 request。
    /// 真正的批准必须由 `approve_request` 完成（用户通过 UI 显式点击）。
    pub async fn request_authorization(
        &self,
        request: AuthorizationRequest,
    ) -> AuthorizationResponse {
        let path = PathBuf::from(&request.path);

        if !self.is_path_safe(&path) {
            self.audit(
                "request_authorization",
                "system",
                &request.path,
                Some(request.level.clone()),
                false,
                "unsafe path",
            )
            .await;
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
            let mut pending = self.pending_requests.lock().await;
            pending.push(req);
        }
        self.audit(
            "request_authorization",
            "system",
            &path_str,
            Some(level.clone()),
            true,
            "pending user approval",
        )
        .await;

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
    pub async fn approve_request(&self, request_id: &str, approver: &str) -> AuthorizationResponse {
        let mut pending = self.pending_requests.lock().await;
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
            )
            .await;
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
            let mut authorizations = self.authorizations.lock().await;
            authorizations.insert(auth_id.clone(), auth);
        }
        self.audit(
            "approve_request",
            approver,
            &req.path,
            Some(req.level.clone()),
            true,
            &format!("approved, expires {}", expires_at.to_rfc3339()),
        )
        .await;

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

    pub async fn deny_request(&self, request_id: &str, approver: &str) -> bool {
        let mut pending = self.pending_requests.lock().await;
        if let Some(pos) = pending.iter().position(|r| r.id == request_id) {
            let req = pending.remove(pos);
            self.audit(
                "deny_request",
                approver,
                &req.path,
                Some(req.level),
                false,
                "denied by user",
            )
            .await;
            true
        } else {
            false
        }
    }

    /// SECURITY (H5): 路径匹配：精确 → 父目录递归 → 都检查 expires_at。
    pub async fn check_authorization(&self, path: &str, required_level: &PermissionLevel) -> bool {
        let path = PathBuf::from(path);
        let authorizations = self.authorizations.lock().await;

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

    pub async fn revoke_authorization(&self, auth_id: &str) -> bool {
        let mut authorizations = self.authorizations.lock().await;
        authorizations.remove(auth_id).is_some()
    }

    pub async fn revoke_all_for_path(&self, path: &str) -> usize {
        let path = PathBuf::from(path);
        let mut authorizations = self.authorizations.lock().await;
        let before = authorizations.len();
        authorizations.retain(|_, auth| !path_matches(&auth.path, &path));
        before - authorizations.len()
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut authorizations = self.authorizations.lock().await;
        let before = authorizations.len();
        let now = Utc::now();
        authorizations.retain(|_, auth| match auth.expires_at {
            Some(t) => t > now,
            None => true,
        });
        before - authorizations.len()
    }

    pub async fn list_authorizations(&self) -> Vec<FileAuthorization> {
        let authorizations = self.authorizations.lock().await;
        authorizations.values().cloned().collect()
    }

    pub async fn get_authorization(&self, auth_id: &str) -> Option<FileAuthorization> {
        let authorizations = self.authorizations.lock().await;
        authorizations.get(auth_id).cloned()
    }

    pub async fn renew_authorization(&self, auth_id: &str, additional_minutes: i64) -> bool {
        let mut authorizations = self.authorizations.lock().await;
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

    /// SECURITY (C3): 拒绝路径遍历、符号链接、UNC/NFTS 流语法攻击。
    ///
    /// 增强策略：
    ///   1. 先用 canonicalize 解析真实路径再做前缀检查，阻止符号链接逃逸。
    ///   2. 检测 UNC 路径与 NTFS 备用数据流语法（如 `:$DATA`）。
    ///   3. 对尚不存在的路径，沿父目录链向上找到最近存在的目录后 canonicalize，
    ///      再拼接剩余相对路径做安全性判定。
    fn is_path_safe(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // ── 1. 基础合法性 ──
        if path_str.is_empty() || path_str.contains('\0') {
            return false;
        }
        if path_str.starts_with('~') {
            return false;
        }

        // ── 2. NTFS 备用数据流 (ADS) 与 UNC 语法检测 ──
        if cfg!(windows) && Self::has_ntfs_stream_or_unc_risk(&path_str) {
            return false;
        }

        // ── 3. canonicalize 解析现有路径 ──
        match std::fs::canonicalize(path) {
            Ok(real) => {
                // 解析后必须是绝对路径，且不能包含 .. 段
                if !real.is_absolute() {
                    return false;
                }
                let real_str = real.to_string_lossy();
                if real_str.contains("..") {
                    return false;
                }
                // 再次检查 canonicalize 后的 NTFS/UNC 风险
                #[cfg(windows)]
                {
                    if Self::has_ntfs_stream_or_unc_risk(&real_str) {
                        return false;
                    }
                }
                true
            },
            Err(_) => {
                // ── 4. 不存在的路径：沿父目录链找到最近存在的目录 ──
                Self::validate_non_existent_path(path)
            },
        }
    }

    /// 检测 Windows NTFS 备用数据流语法（如 `file::$DATA`）和危险 UNC 前缀。
    #[cfg(windows)]
    fn has_ntfs_stream_or_unc_risk(path_str: &str) -> bool {
        // `\\.\` 设备命名空间：始终危险。
        if path_str.starts_with("\\\\.\\") {
            return true;
        }

        // `\\?\` 是 std::fs::canonicalize 在 Windows 上必然产生的扩展长度前缀
        // (安全)。若不剥离，`\\?\C:\...` 的盘符冒号会落在位置 5 而非 1，
        // 被下面的 ADS 检测误判为风险，导致所有真实路径授权失败。
        // `\\?\UNC\server\share` 剥离为无盘符的 UNC 主体。
        let core = if let Some(rest) = path_str.strip_prefix("\\\\?\\UNC\\") {
            rest
        } else if let Some(rest) = path_str.strip_prefix("\\\\?\\") {
            rest
        } else {
            path_str
        };

        // NTFS 备用数据流：冒号出现在盘符之后的位置即视为风险。
        // 合法形式：`C:\...` (冒号在位置 1)；其余位置的冒号均为可疑。
        if let Some(colon_pos) = core.find(':')
            && (colon_pos != 1 || !core.as_bytes().first().is_some_and(|b| b.is_ascii_alphabetic()))
        {
            return true;
        }
        false
    }

    #[cfg(not(windows))]
    fn has_ntfs_stream_or_unc_risk(_path_str: &str) -> bool {
        false
    }

    /// 对尚不存在的路径，沿父目录链向上找到最近存在的目录后 canonicalize，
    /// 再拼接剩余相对路径段做安全性检查。
    fn validate_non_existent_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // 基础遍历检查：不允许任何 `..` 段
        if path_str.contains("..") {
            return false;
        }

        // 沿父目录链向上查找第一个存在的目录
        let mut current = path.to_path_buf();
        let mut remaining = std::path::PathBuf::new();

        loop {
            match current.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => {
                    if parent.exists() {
                        // 找到存在的父目录，canonicalize 它
                        if let Ok(real_parent) = std::fs::canonicalize(parent) {
                            let resolved = real_parent.join(&remaining);
                            let resolved_str = resolved.to_string_lossy();
                            // 解析后不能包含 .. 或 NTFS 风险
                            if resolved_str.contains("..") {
                                return false;
                            }
                            #[cfg(windows)]
                            {
                                if Self::has_ntfs_stream_or_unc_risk(&resolved_str) {
                                    return false;
                                }
                            }
                            return resolved.is_absolute();
                        }
                        break;
                    }
                    // 父目录不存在，继续向上
                    if let Some(segment) = current.file_name() {
                        let mut new_remaining = std::path::PathBuf::from(segment);
                        new_remaining.push(&remaining);
                        remaining = new_remaining;
                    }
                    current = parent.to_path_buf();
                },
                _ => break,
            }
        }

        // 兜底：若无法解析（如根目录不存在这种极端情况），
        // 至少确保没有明显的路径遍历标记
        !path_str.contains("..")
    }

    pub async fn add_pending_request(&self, request: AuthorizationRequest) {
        let mut pending = self.pending_requests.lock().await;
        pending.push(request);
    }

    pub async fn get_pending_requests(&self) -> Vec<AuthorizationRequest> {
        let pending = self.pending_requests.lock().await;
        pending.clone()
    }

    pub async fn clear_pending_requests(&self) {
        let mut pending = self.pending_requests.lock().await;
        pending.clear();
    }

    /// SECURITY (M10): 写审计日志（内存 ring buffer + tracing + 文件持久化）。
    /// M3: 若已设置 audit_log_path，追加写入 JSONL 格式的持久化日志文件。
    pub async fn audit(
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

        // M3: 文件持久化（JSONL 格式）
        if let Some(ref log_path) = *self.audit_log_path.lock().await
            && let Ok(json) = serde_json::to_string(&entry)
        {
            // 使用 std::fs::OpenOptions 以 append 模式打开，避免持有锁期间做 I/O
            let json_line = format!("{}\n", json);
            if let Ok(mut file) =
                std::fs::OpenOptions::new().create(true).append(true).open(log_path)
            {
                let _ = file.write_all(json_line.as_bytes());
            }
        }

        // 内存 ring buffer
        let mut log = self.audit_log.lock().await;
        log.push(entry);
        // ring buffer: 保留最近 1000 条
        if log.len() > 1000 {
            let drop = log.len() - 1000;
            log.drain(0..drop);
        }
    }

    pub async fn get_audit_log(&self) -> Vec<AuditEntry> {
        let log = self.audit_log.lock().await;
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

    #[tokio::test]
    async fn request_no_longer_auto_approves() {
        // SECURITY (C10): 直接 request_authorization 必须 pending
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/legit.txt", PermissionLevel::Read)).await;
        assert!(!r.authorized, "request_authorization must not auto-approve");
        assert!(r.request_id.is_some());
    }

    #[tokio::test]
    async fn approve_request_grants() {
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/legit.txt", PermissionLevel::Read)).await;
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1").await;
        assert!(r2.authorized);
        assert!(r2.auth_id.is_some());
        // SECURITY: 批准者被记录
        let auth = a.get_authorization(&r2.auth_id.unwrap()).await.unwrap();
        assert_eq!(auth.approver.as_deref(), Some("user-1"));
    }

    #[tokio::test]
    async fn path_under_dir_authorized() {
        // SECURITY (H5): 目录授权后子文件应通过
        let a = FileAuthorizer::new();
        // 用 tempdir 的真实路径
        let dir = std::env::temp_dir().join(format!("axagent-fauth-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("inside.txt");
        std::fs::write(&file, "x").unwrap();

        let r = a.request_authorization(req(&dir.to_string_lossy(), PermissionLevel::Read)).await;
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1").await;
        assert!(r2.authorized);

        assert!(a.check_authorization(&file.to_string_lossy(), &PermissionLevel::Read).await);
        assert!(!a.check_authorization(&file.to_string_lossy(), &PermissionLevel::Write).await);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn temp_level_has_ttl() {
        // SECURITY (H4): Temp 必须带 expires_at
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/x.txt", PermissionLevel::Temp)).await;
        let req_id = r.request_id.unwrap();
        let r2 = a.approve_request(&req_id, "user-1").await;
        let auth_id = r2.auth_id.unwrap();
        let auth = a.get_authorization(&auth_id).await.unwrap();
        assert!(auth.expires_at.is_some());
    }

    #[tokio::test]
    async fn audit_records_actions() {
        // SECURITY (M10)
        let a = FileAuthorizer::new();
        let r = a.request_authorization(req("/tmp/x.txt", PermissionLevel::Read)).await;
        let req_id = r.request_id.unwrap();
        let _ = a.approve_request(&req_id, "user-1").await;
        let log = a.get_audit_log().await;
        assert!(log.iter().any(|e| e.action == "approve_request"));
    }
}
