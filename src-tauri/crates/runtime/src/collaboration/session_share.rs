use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSession {
    pub session_id: String,
    pub owner_id: String,
    pub invite_code: String,
    pub participants: Vec<Participant>,
    pub shared_resources: Vec<SharedResource>,
    pub permissions: SessionPermissions,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: String,
    pub display_name: String,
    pub role: ParticipantRole,
    pub joined_at: i64,
    pub last_active: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub access_level: AccessLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Document,
    Terminal,
    Workflow,
    KnowledgeBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    View,
    Edit,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermissions {
    pub allow_terminal_access: bool,
    pub allow_file_access: bool,
    pub allow_model_access: bool,
    pub require_approval_for_actions: bool,
    pub max_participants: usize,
}

impl Default for SessionPermissions {
    fn default() -> Self {
        Self {
            allow_terminal_access: false,
            allow_file_access: false,
            allow_model_access: true,
            require_approval_for_actions: true,
            max_participants: 10,
        }
    }
}

pub struct SessionShareManager {
    sessions: HashMap<String, SharedSession>,
}

impl Default for SessionShareManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionShareManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        session_id: &str,
        owner_id: &str,
        permissions: SessionPermissions,
    ) -> &SharedSession {
        let invite_code = Self::generate_invite_code();
        let session = SharedSession {
            session_id: session_id.to_string(),
            owner_id: owner_id.to_string(),
            invite_code: invite_code.clone(),
            participants: vec![Participant {
                user_id: owner_id.to_string(),
                display_name: "Owner".to_string(),
                role: ParticipantRole::Owner,
                joined_at: now_ms(),
                last_active: now_ms(),
            }],
            shared_resources: Vec::new(),
            permissions,
            created_at: now_ms(),
            expires_at: None,
            is_active: true,
        };
        self.sessions.insert(session_id.to_string(), session);
        self.sessions.get(session_id).unwrap()
    }

    pub fn join_session(
        &mut self,
        session_id: &str,
        invite_code: &str,
        user_id: &str,
        display_name: &str,
    ) -> Result<ParticipantRole, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        if !session.is_active {
            return Err("Session is no longer active".to_string());
        }

        if session.invite_code != invite_code {
            return Err("Invalid invite code".to_string());
        }

        if session.participants.len() >= session.permissions.max_participants {
            return Err("Session is full".to_string());
        }

        if session.participants.iter().any(|p| p.user_id == user_id) {
            return Err("Already a participant".to_string());
        }

        let role = if user_id == session.owner_id {
            ParticipantRole::Owner
        } else {
            ParticipantRole::Viewer
        };

        session.participants.push(Participant {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
            role: role.clone(),
            joined_at: now_ms(),
            last_active: now_ms(),
        });

        Ok(role)
    }

    pub fn leave_session(
        &mut self,
        session_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.participants.retain(|p| p.user_id != user_id);
        Ok(())
    }

    pub fn close_session(&mut self, session_id: &str, owner_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        if session.owner_id != owner_id {
            return Err("Only the owner can close the session".to_string());
        }

        session.is_active = false;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Option<&SharedSession> {
        self.sessions.get(session_id)
    }

    pub fn list_active_sessions(&self) -> Vec<&SharedSession> {
        self.sessions.values().filter(|s| s.is_active).collect()
    }

    pub fn share_resource(
        &mut self,
        session_id: &str,
        resource: SharedResource,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.shared_resources.push(resource);
        Ok(())
    }

    pub fn unshare_resource(
        &mut self,
        session_id: &str,
        resource_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session
            .shared_resources
            .retain(|r| r.resource_id != resource_id);
        Ok(())
    }

    pub fn update_participant_role(
        &mut self,
        session_id: &str,
        user_id: &str,
        new_role: ParticipantRole,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        if let Some(participant) = session.participants.iter_mut().find(|p| p.user_id == user_id)
        {
            participant.role = new_role;
            Ok(())
        } else {
            Err("Participant not found".to_string())
        }
    }

    fn generate_invite_code() -> String {
        use std::fmt::Write;
        let mut code = String::with_capacity(8);
        let t = now_ms() as u64;
        for i in 0..8 {
            let c = ((t >> (i * 4)) & 0xF) as u8;
            let ch = match c {
                0..=9 => (b'0' + c) as char,
                _ => (b'A' + (c - 10)) as char,
            };
            write!(code, "{}", ch).unwrap();
        }
        code
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
