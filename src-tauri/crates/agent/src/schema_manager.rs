use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;

use sea_orm::{DatabaseConnection, EntityTrait};

use axagent_core::entity::wikis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub version: String,
    pub created_at: i64,
    pub content_hash: String,
    pub note_count: i32,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    pub from_version: String,
    pub to_version: String,
    pub added_fields: Vec<String>,
    pub removed_fields: Vec<String>,
    pub changed_fields: Vec<FieldChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_type: String,
    pub new_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontmatterTemplate {
    pub required: Vec<FieldDef>,
    pub optional: Vec<FieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
}

pub struct SchemaManager {
    db: Arc<DatabaseConnection>,
    cache: Arc<RwLock<Option<String>>>,
}

impl SchemaManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_current_schema(&self, wiki_id: &str) -> Result<String, String> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = &*cache {
                return Ok(cached.clone());
            }
        }

        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let schema_path = PathBuf::from(&wiki.root_path).join("SCHEMA.md");
        let content = fs::read_to_string(&schema_path)
            .await
            .map_err(|e| e.to_string())?;

        {
            let mut cache = self.cache.write().await;
            *cache = Some(content.clone());
        }

        Ok(content)
    }

    pub async fn validate_frontmatter(
        &self,
        wiki_id: &str,
        frontmatter: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<String>, String> {
        let template = self.get_frontmatter_template(wiki_id).await?;
        let mut errors = Vec::new();

        for field in &template.required {
            if !frontmatter.contains_key(&field.name) {
                errors.push(format!("Missing required field: {}", field.name));
            }
        }

        for (key, value) in frontmatter {
            if !template.required.iter().any(|f| &f.name == key)
                && !template.optional.iter().any(|f| &f.name == key)
            {
                errors.push(format!("Unknown field: {}", key));
            }

            let field_def = template
                .required
                .iter()
                .chain(template.optional.iter())
                .find(|f| &f.name == key);

            if let Some(def) = field_def {
                if !self.validate_field_type(&def.field_type, value) {
                    errors.push(format!(
                        "Field '{}' has invalid type, expected {}",
                        key, def.field_type
                    ));
                }
            }
        }

        Ok(errors)
    }

    async fn get_frontmatter_template(&self, wiki_id: &str) -> Result<FrontmatterTemplate, String> {
        let schema = self.get_current_schema(wiki_id).await?;
        Ok(self.parse_template_from_schema(&schema))
    }

    fn parse_template_from_schema(&self, schema: &str) -> FrontmatterTemplate {
        let mut required = Vec::new();
        let mut optional = Vec::new();

        let lines: Vec<&str> = schema.lines().collect();
        let mut in_frontmatter = false;

        for line in lines {
            if line.trim() == "---" {
                in_frontmatter = !in_frontmatter;
                continue;
            }

            if in_frontmatter {
                if let Some((key, rest)) = line.split_once(':') {
                    let key = key.trim();
                    let field_type = rest.trim().to_string();

                    if key.starts_with('?') {
                        optional.push(FieldDef {
                            name: key[1..].to_string(),
                            field_type,
                            description: None,
                        });
                    } else {
                        required.push(FieldDef {
                            name: key.to_string(),
                            field_type,
                            description: None,
                        });
                    }
                }
            }
        }

        FrontmatterTemplate {
            required,
            optional,
        }
    }

    fn validate_field_type(&self, expected: &str, value: &serde_json::Value) -> bool {
        match (expected, value) {
            ("string", serde_json::Value::String(_)) => true,
            ("number", serde_json::Value::Number(_)) => true,
            ("boolean", serde_json::Value::Bool(_)) => true,
            ("array", serde_json::Value::Array(_)) => true,
            ("object", serde_json::Value::Object(_)) => true,
            ("date", serde_json::Value::String(s)) => {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
                    || chrono::DateTime::parse_from_rfc3339(s).is_ok()
            }
            ("tags", serde_json::Value::Array(arr)) => {
                arr.iter().all(|v| v.is_string())
            }
            _ => true,
        }
    }

    pub async fn create_schema_version(
        &self,
        wiki_id: &str,
        version: &str,
        description: Option<String>,
    ) -> Result<SchemaVersion, String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let schema_path = PathBuf::from(&wiki.root_path).join("SCHEMA.md");
        let content = fs::read_to_string(&schema_path)
            .await
            .map_err(|e| e.to_string())?;

        let content_hash = format!("{:x}", md5::compute(&content));

        let schema_version = SchemaVersion {
            version: version.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            content_hash,
            note_count: wiki.note_count,
            description,
        };

        {
            let mut cache = self.cache.write().await;
            *cache = Some(content);
        }

        Ok(schema_version)
    }

    pub async fn diff_schemas(
        &self,
        _wiki_id: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<SchemaDiff, String> {
        Ok(SchemaDiff {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            added_fields: Vec::new(),
            removed_fields: Vec::new(),
            changed_fields: Vec::new(),
        })
    }

    pub async fn migrate_pages(
        &self,
        _wiki_id: &str,
        _from_version: &str,
        _to_version: &str,
    ) -> Result<i32, String> {
        Ok(0)
    }
}