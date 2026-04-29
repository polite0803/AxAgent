use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub display_name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub profile: Profile,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub sessions_path: PathBuf,
    pub skills_path: PathBuf,
    pub hooks_path: PathBuf,
}

impl Profile {
    pub fn new(name: &str, display_name: &str) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            created_at: now,
            updated_at: now,
            is_default: false,
        }
    }

    pub fn default_profile() -> Self {
        Self {
            name: "default".to_string(),
            display_name: "Default".to_string(),
            created_at: 0,
            updated_at: 0,
            is_default: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Profile not found: {0}")]
    NotFound(String),
    #[error("Profile already exists: {0}")]
    AlreadyExists(String),
    #[error("Cannot delete default profile")]
    CannotDeleteDefault,
    #[error("Invalid profile name: {0}")]
    InvalidName(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub fn validate_profile_name(name: &str) -> Result<(), ProfileError> {
    if name.is_empty() {
        return Err(ProfileError::InvalidName(
            "Name cannot be empty".to_string(),
        ));
    }
    if name.len() > 64 {
        return Err(ProfileError::InvalidName(
            "Name too long (max 64 chars)".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ProfileError::InvalidName(
            "Name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }
    if name == "default" {
        return Err(ProfileError::AlreadyExists("default".to_string()));
    }
    Ok(())
}

pub fn profiles_root() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".axagent")
        .join("profiles")
}

pub fn profile_dir(name: &str) -> PathBuf {
    profiles_root().join(name)
}

pub fn ensure_profile_dirs(name: &str) -> Result<PathBuf, ProfileError> {
    let dir = profile_dir(name);
    std::fs::create_dir_all(dir.join("config"))?;
    std::fs::create_dir_all(dir.join("data"))?;
    std::fs::create_dir_all(dir.join("sessions"))?;
    std::fs::create_dir_all(dir.join("skills"))?;
    std::fs::create_dir_all(dir.join("hooks"))?;
    Ok(dir)
}
