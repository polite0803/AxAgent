use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use base64::Engine;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub require_auth: bool,
    pub token_expiry: Duration,
    pub refresh_token_expiry: Duration,
    pub max_login_attempts: usize,
    pub lockout_duration: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_auth: true,
            token_expiry: Duration::from_secs(3600),
            refresh_token_expiry: Duration::from_secs(86400),
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(900),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
}

impl AuthToken {
    pub fn new(access_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }

    pub fn with_refresh(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: i64,
    pub last_login: Option<i64>,
}

impl AuthenticatedUser {
    pub fn new(user_id: String, username: String) -> Self {
        Self {
            user_id,
            username,
            roles: Vec::new(),
            permissions: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
            last_login: None,
        }
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}

#[derive(Debug, Clone)]
pub enum AuthMethod {
    None,
    ApiKey,
    Bearer,
    Basic,
    Jwt,
}

pub struct AuthContext {
    pub user: Option<AuthenticatedUser>,
    pub auth_method: AuthMethod,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

impl AuthContext {
    pub fn new() -> Self {
        Self {
            user: None,
            auth_method: AuthMethod::None,
            client_ip: None,
            user_agent: None,
        }
    }

    pub fn with_user(mut self, user: AuthenticatedUser) -> Self {
        self.user = Some(user);
        self
    }

    pub fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }
}

impl Default for AuthContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Authenticator {
    config: AuthConfig,
    api_keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    login_attempts: Arc<RwLock<HashMap<String, LoginAttempt>>>,
}

struct ApiKeyInfo {
    key: String,
    user_id: String,
    permissions: Vec<String>,
    created_at: i64,
    expires_at: Option<i64>,
}

struct SessionInfo {
    user_id: String,
    username: String,
    created_at: i64,
    expires_at: i64,
    last_activity: i64,
}

struct LoginAttempt {
    count: usize,
    first_attempt: i64,
    locked_until: Option<i64>,
}

impl Authenticator {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_api_key(&self, key: String, user_id: String, permissions: Vec<String>) {
        let mut keys = self.api_keys.write().await;
        keys.insert(key.clone(), ApiKeyInfo {
            key,
            user_id,
            permissions,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        });
    }

    pub async fn validate_api_key(&self, key: &str) -> Result<AuthenticatedUser, AuthError> {
        let keys = self.api_keys.read().await;
        if let Some(info) = keys.get(key) {
            if let Some(expires) = info.expires_at {
                if chrono::Utc::now().timestamp() > expires {
                    return Err(AuthError::TokenExpired);
                }
            }
            return Ok(AuthenticatedUser::new(info.user_id.clone(), info.user_id.clone())
                .with_permissions(info.permissions.clone()));
        }
        Err(AuthError::InvalidCredentials)
    }

    pub async fn authenticate_basic(&self, username: &str, password: &str) -> Result<AuthToken, AuthError> {
        let key = format!("{}:{}", username, password);
        if let Ok(info) = self.validate_api_key(&key).await {
            let token = self.generate_token(&info).await?;
            return Ok(token);
        }

        let mut attempts = self.login_attempts.write().await;
        let attempt = attempts.entry(username.to_string()).or_insert_with(|| LoginAttempt {
            count: 0,
            first_attempt: 0,
            locked_until: None,
        });

        if let Some(locked_until) = attempt.locked_until {
            if chrono::Utc::now().timestamp() < locked_until {
                return Err(AuthError::AccountLocked);
            }
        }

        attempt.count += 1;
        if attempt.count >= self.config.max_login_attempts {
            attempt.locked_until = Some(chrono::Utc::now().timestamp() + self.config.lockout_duration.as_secs() as i64);
            return Err(AuthError::AccountLocked);
        }

        Err(AuthError::InvalidCredentials)
    }

    pub async fn authenticate_bearer(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(token) {
            if chrono::Utc::now().timestamp() > session.expires_at {
                return Err(AuthError::TokenExpired);
            }
            return Ok(AuthenticatedUser::new(session.user_id.clone(), session.username.clone()));
        }
        Err(AuthError::InvalidToken)
    }

    pub async fn create_session(&self, user: &AuthenticatedUser) -> Result<AuthToken, AuthError> {
        let session_id = uuid_v4();
        let now = chrono::Utc::now().timestamp();

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), SessionInfo {
                user_id: user.user_id.clone(),
                username: user.username.clone(),
                created_at: now,
                expires_at: now + self.config.token_expiry.as_secs() as i64,
                last_activity: now,
            });
        }

        let refresh_id = uuid_v4();
        let refresh_expires = now + self.config.refresh_token_expiry.as_secs() as i64;

        Ok(AuthToken::new(session_id, self.config.token_expiry.as_secs())
            .with_refresh(refresh_id))
    }

    async fn generate_token(&self, user: &AuthenticatedUser) -> Result<AuthToken, AuthError> {
        Ok(AuthToken::new(uuid_v4(), self.config.token_expiry.as_secs()))
    }

    pub async fn invalidate_session(&self, token: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token);
    }

    pub async fn cleanup_expired_sessions(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| session.expires_at > now);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Account locked")]
    AccountLocked,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Authentication required")]
    AuthenticationRequired,
}

pub struct AuthorizationService {
    policies: Arc<RwLock<HashMap<String, Policy>>>,
}

struct Policy {
    name: String,
    resources: Vec<String>,
    actions: Vec<String>,
    effect: PolicyEffect,
}

enum PolicyEffect {
    Allow,
    Deny,
}

impl AuthorizationService {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_policy(&self, policy: Policy) {
        let mut policies = self.policies.write().await;
        policies.insert(policy.name.clone(), policy);
    }

    pub async fn check_permission(&self, user: &AuthenticatedUser, resource: &str, action: &str) -> bool {
        let policies = self.policies.read().await;

        for policy in policies.values() {
            if policy.resources.iter().any(|r| r == resource || r == "*") {
                if policy.actions.iter().any(|a| a == action || a == "*") {
                    match policy.effect {
                        PolicyEffect::Allow => return true,
                        PolicyEffect::Deny => return false,
                    }
                }
            }
        }

        for role in &user.roles {
            let role_policy = format!("role:{}", role);
            if let Some(policy) = policies.get(&role_policy) {
                if policy.resources.iter().any(|r| r == resource || r == "*") {
                    if policy.actions.iter().any(|a| a == action || a == "*") {
                        match policy.effect {
                            PolicyEffect::Allow => return true,
                            PolicyEffect::Deny => return false,
                        }
                    }
                }
            }
        }

        user.permissions.iter().any(|p| p == action || p == "*")
    }
}

impl Default for AuthorizationService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JwtValidator {
    secret: String,
    issuer: Option<String>,
    audience: Option<String>,
}

impl JwtValidator {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            issuer: None,
            audience: None,
        }
    }

    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.issuer = Some(issuer);
        self
    }

    pub fn with_audience(mut self, audience: String) -> Self {
        self.audience = Some(audience);
        self
    }

    pub fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }

        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| AuthError::InvalidToken)?;

        let header_json: HashMap<String, serde_json::Value> = serde_json::from_slice(&header)
            .map_err(|_| AuthError::InvalidToken)?;

        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| AuthError::InvalidToken)?;

        #[derive(Deserialize)]
        struct JwtPayload {
            sub: Option<String>,
            exp: Option<i64>,
            iat: Option<i64>,
            roles: Option<Vec<String>>,
            permissions: Option<Vec<String>>,
        }

        let payload: JwtPayload = serde_json::from_slice(&payload)
            .map_err(|_| AuthError::InvalidToken)?;

        let now = chrono::Utc::now().timestamp();
        if let Some(exp) = payload.exp {
            if now > exp {
                return Err(AuthError::TokenExpired);
            }
        }

        Ok(Claims {
            sub: payload.sub.unwrap_or_default(),
            exp: payload.exp.unwrap_or(now + 3600),
            iat: payload.iat.unwrap_or(now),
            roles: payload.roles.unwrap_or_default(),
            permissions: payload.permissions.unwrap_or_default(),
        })
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let random: u128 = (timestamp as u128) << 64 | (rand_u64() as u128);
    format!("{:032x}", random)
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}