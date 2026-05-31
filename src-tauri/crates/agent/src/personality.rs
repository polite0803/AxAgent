use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

static PERSONALITIES_DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("personalities")
});

static ACTIVE_FILE: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("personalities")
        .join(".active")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub content: String,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SoulFrontmatter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
}

impl Personality {
    fn dir_path(&self) -> PathBuf {
        PERSONALITIES_DIR.join(&self.name)
    }

    fn soul_md_path(&self) -> PathBuf {
        self.dir_path().join("SOUL.md")
    }

    pub fn to_soul_md(&self) -> String {
        let fm = SoulFrontmatter {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
        };
        let yaml = serde_yaml::to_string(&fm).unwrap_or_default();
        format!("---\n{}---\n\n{}", yaml, self.content)
    }

    pub fn from_soul_md(name: &str, raw: &str) -> Self {
        let trimmed = raw.trim_start();
        let (frontmatter, content) = if trimmed.starts_with("---") {
            if let Some(end) = trimmed[3..].find("---") {
                let yaml_str = &trimmed[3..3 + end];
                let fm: SoulFrontmatter =
                    serde_yaml::from_str(yaml_str).unwrap_or(SoulFrontmatter {
                        name: name.to_string(),
                        version: "1.0.0".to_string(),
                        description: String::new(),
                    });
                let body = trimmed[3 + end + 3..].trim_start();
                (fm, body.to_string())
            } else {
                (
                    SoulFrontmatter {
                        name: name.to_string(),
                        version: "1.0.0".to_string(),
                        description: String::new(),
                    },
                    raw.to_string(),
                )
            }
        } else {
            (
                SoulFrontmatter {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: String::new(),
                },
                raw.to_string(),
            )
        };

        let p_name = if frontmatter.name.is_empty() {
            name.to_string()
        } else {
            frontmatter.name
        };

        Self {
            name: p_name,
            version: if frontmatter.version.is_empty() {
                "1.0.0".to_string()
            } else {
                frontmatter.version
            },
            description: frontmatter.description,
            content,
            created_at: Utc::now(),
        }
    }

    pub fn system_prompt_injection(&self) -> String {
        format!(
            "## Personality: {}\n\n{}\n\n{}",
            self.name,
            if self.description.is_empty() {
                String::new()
            } else {
                format!("*{}*\n\n", self.description)
            },
            self.content,
        )
    }
}

fn validate_personality_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Personality name cannot be empty".to_string());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Personality name contains invalid characters".to_string());
    }
    if name.starts_with('.') {
        return Err("Personality name cannot start with '.'".to_string());
    }
    Ok(())
}

pub struct PersonalityManager;

impl PersonalityManager {
    pub fn personalities_dir() -> &'static PathBuf {
        &PERSONALITIES_DIR
    }

    pub fn active_file() -> &'static PathBuf {
        &ACTIVE_FILE
    }

    pub fn ensure_dir() -> Result<(), String> {
        fs::create_dir_all(&*PERSONALITIES_DIR)
            .map_err(|e| format!("Failed to create personalities directory: {}", e))
    }

    pub fn list() -> Result<Vec<String>, String> {
        Self::ensure_dir()?;
        let mut names = Vec::new();
        let entries = fs::read_dir(&*PERSONALITIES_DIR).map_err(|e| format!("read_dir: {}", e))?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.join("SOUL.md").exists() {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn load(name: &str) -> Result<Personality, String> {
        validate_personality_name(name)?;
        let soul_path = PERSONALITIES_DIR.join(name).join("SOUL.md");
        if !soul_path.exists() {
            return Err(format!("Personality '{}' not found", name));
        }
        let raw = fs::read_to_string(&soul_path)
            .map_err(|e| format!("Failed to read SOUL.md for '{}': {}", name, e))?;
        Ok(Personality::from_soul_md(name, &raw))
    }

    pub fn save(personality: &Personality) -> Result<(), String> {
        validate_personality_name(&personality.name)?;
        Self::ensure_dir()?;
        let dir = personality.dir_path();
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;
        let content = personality.to_soul_md();
        fs::write(personality.soul_md_path(), content)
            .map_err(|e| format!("Failed to write SOUL.md: {}", e))
    }

    pub fn delete(name: &str) -> Result<(), String> {
        validate_personality_name(name)?;
        let dir = PERSONALITIES_DIR.join(name);
        if !dir.exists() {
            return Err(format!("Personality '{}' not found", name));
        }
        fs::remove_dir_all(&dir)
            .map_err(|e| format!("Failed to delete personality '{}': {}", name, e))
    }

    pub fn get_active() -> Result<Option<Personality>, String> {
        if !ACTIVE_FILE.exists() {
            return Ok(None);
        }
        let name = fs::read_to_string(&*ACTIVE_FILE)
            .map_err(|e| format!("Failed to read active personality: {}", e))?;
        let name = name.trim().to_string();
        if name.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self::load(&name)?))
    }

    pub fn set_active(name: &str) -> Result<(), String> {
        validate_personality_name(name)?;
        Self::ensure_dir()?;
        let dir = PERSONALITIES_DIR.join(name);
        if !dir.exists() || !dir.join("SOUL.md").exists() {
            return Err(format!("Personality '{}' does not exist. Create it first.", name));
        }
        fs::write(&*ACTIVE_FILE, name)
            .map_err(|e| format!("Failed to write active personality: {}", e))
    }

    pub fn clear_active() -> Result<(), String> {
        if ACTIVE_FILE.exists() {
            fs::remove_file(&*ACTIVE_FILE)
                .map_err(|e| format!("Failed to clear active personality: {}", e))?;
        }
        Ok(())
    }
}
