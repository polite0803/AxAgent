use crate::profile::{self, Profile, ProfileError, ProfileInfo};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ProfileManager {
    active_profile: Arc<RwLock<String>>,
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileManager {
    pub fn new() -> Self {
        Self {
            active_profile: Arc::new(RwLock::new("default".to_string())),
        }
    }

    pub async fn active_profile(&self) -> String {
        self.active_profile.read().await.clone()
    }

    pub async fn set_active(&self, name: &str) -> Result<(), ProfileError> {
        if name != "default" && !self.list().await?.iter().any(|p| p.profile.name == name) {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        *self.active_profile.write().await = name.to_string();
        Ok(())
    }

    pub async fn create(
        &self,
        name: &str,
        display_name: &str,
    ) -> Result<ProfileInfo, ProfileError> {
        profile::validate_profile_name(name)?;
        let dir = profile::profile_dir(name);
        if dir.exists() {
            return Err(ProfileError::AlreadyExists(name.to_string()));
        }
        profile::ensure_profile_dirs(name)?;
        let profile = Profile::new(name, display_name);
        let info = self.info_for(name, &profile)?;
        let meta_path = dir.join("profile.json");
        let json = serde_json::to_string_pretty(&profile).map_err(|e| {
            ProfileError::Serialization(e.to_string())
        })?;
        std::fs::write(&meta_path, json)?;
        Ok(info)
    }

    pub async fn delete(&self, name: &str) -> Result<(), ProfileError> {
        if name == "default" {
            return Err(ProfileError::CannotDeleteDefault);
        }
        let dir = profile::profile_dir(name);
        if !dir.exists() {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        std::fs::remove_dir_all(&dir)?;
        let active = self.active_profile.read().await.clone();
        if active == name {
            *self.active_profile.write().await = "default".to_string();
        }
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<ProfileInfo>, ProfileError> {
        let root = profile::profiles_root();
        let mut result = vec![];
        result.push(self.info_for("default", &Profile::default_profile())?);
        if !root.exists() {
            return Ok(result);
        }
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta_path = entry.path().join("profile.json");
                let profile = if meta_path.exists() {
                    let content = std::fs::read_to_string(&meta_path)?;
                    serde_json::from_str(&content)
                        .unwrap_or_else(|_| Profile::new(&name, &name))
                } else {
                    Profile::new(&name, &name)
                };
                result.push(self.info_for(&name, &profile)?);
            }
        }
        Ok(result)
    }

    fn info_for(&self, name: &str, profile: &Profile) -> Result<ProfileInfo, ProfileError> {
        let dir = if name == "default" {
            dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".axagent")
        } else {
            profile::profile_dir(name)
        };
        Ok(ProfileInfo {
            profile: profile.clone(),
            config_path: dir.join("config"),
            db_path: dir.join("data").join("axagent.db"),
            sessions_path: dir.join("sessions"),
            skills_path: dir.join("skills"),
            hooks_path: dir.join("hooks"),
        })
    }

    pub async fn active_info(&self) -> Result<ProfileInfo, ProfileError> {
        let name = self.active_profile().await;
        let profiles = self.list().await?;
        profiles
            .into_iter()
            .find(|p| p.profile.name == name)
            .ok_or(ProfileError::NotFound(name))
    }
}
