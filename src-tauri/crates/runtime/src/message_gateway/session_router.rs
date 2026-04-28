use std::collections::HashMap;
use chrono::Utc;

pub struct SessionRouter {
    sessions: HashMap<String, RoutedSession>,
}

#[derive(Debug, Clone)]
pub struct RoutedSession {
    pub session_id: String,
    pub platform: String,
    pub user_id: String,
    pub username: Option<String>,
    pub agent_session_id: Option<String>,
    pub created_at: i64,
    pub last_activity: i64,
}

impl SessionRouter {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    fn session_key(platform: &str, user_id: &str) -> String {
        format!("{}_{}", platform, user_id)
    }

    pub fn resolve_or_create(
        &mut self,
        platform: &str,
        user_id: &str,
        username: Option<String>,
    ) -> &RoutedSession {
        let key = Self::session_key(platform, user_id);
        let now = Utc::now().timestamp_millis();

        self.sessions
            .entry(key)
            .and_modify(|s| {
                s.last_activity = now;
                if username.is_some() {
                    s.username = username.clone();
                }
            })
            .or_insert_with(|| RoutedSession {
                session_id: uuid::Uuid::new_v4().to_string(),
                platform: platform.to_string(),
                user_id: user_id.to_string(),
                username,
                agent_session_id: None,
                created_at: now,
                last_activity: now,
            });
        self.sessions.get(&Self::session_key(platform, user_id)).unwrap()
    }

    pub fn get_session(&self, platform: &str, user_id: &str) -> Option<&RoutedSession> {
        let key = Self::session_key(platform, user_id);
        self.sessions.get(&key)
    }

    pub fn link_agent_session(
        &mut self,
        platform: &str,
        user_id: &str,
        agent_session_id: &str,
    ) -> Option<()> {
        let key = Self::session_key(platform, user_id);
        if let Some(session) = self.sessions.get_mut(&key) {
            session.agent_session_id = Some(agent_session_id.to_string());
            Some(())
        } else {
            None
        }
    }

    pub fn list_active_sessions(&self) -> Vec<&RoutedSession> {
        let now = Utc::now().timestamp_millis();
        let five_min = 5 * 60 * 1000;
        self.sessions
            .values()
            .filter(|s| now - s.last_activity < five_min)
            .collect()
    }

    pub fn deactivate_session(&mut self, platform: &str, user_id: &str) {
        let key = Self::session_key(platform, user_id);
        if let Some(session) = self.sessions.get_mut(&key) {
            session.last_activity = 0;
        }
    }

    pub fn remove_session(&mut self, platform: &str, user_id: &str) {
        let key = Self::session_key(platform, user_id);
        self.sessions.remove(&key);
    }

    pub fn clear_expired_sessions(&mut self, timeout_ms: i64) -> usize {
        let now = Utc::now().timestamp_millis();
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| now - s.last_activity > timeout_ms)
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for key in expired {
            self.sessions.remove(&key);
        }
        count
    }
}
