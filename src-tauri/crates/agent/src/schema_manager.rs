use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

use axagent_core::entity::{notes, wikis};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Compatibility {
    Compatible,
    Incompatible {
        message: String,
        migration_steps: Vec<String>,
    },
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

    pub async fn check_schema_compatibility(
        &self,
        wiki_id: &str,
        required_version: &str,
    ) -> Result<Compatibility, String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let current = parse_version(&wiki.schema_version);
        let required = parse_version(required_version);

        if compare_versions(&current, &required) >= 0 {
            return Ok(Compatibility::Compatible);
        }

        let schema_content = self.get_current_schema(wiki_id).await?;
        let migration_steps =
            self.generate_migration_steps(&wiki.schema_version, required_version, &schema_content);

        Ok(Compatibility::Incompatible {
            message: format!(
                "Schema version {} is below required version {}. Please upgrade.",
                wiki.schema_version, required_version
            ),
            migration_steps,
        })
    }

    fn generate_migration_steps(
        &self,
        _current_version: &str,
        _target_version: &str,
        _schema_content: &str,
    ) -> Vec<String> {
        vec![
            "1. Backup your current SCHEMA.md".to_string(),
            "2. Update SCHEMA.md with new required fields".to_string(),
            "3. Run wiki lint to validate all pages".to_string(),
            "4. Run auto-fix to update existing pages".to_string(),
        ]
    }

    pub async fn diff_schemas(
        &self,
        wiki_id: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<SchemaDiff, String> {
        let from_template = self.parse_version_template(wiki_id, from_version).await?;
        let to_template = self.parse_version_template(wiki_id, to_version).await?;

        let from_names: std::collections::HashSet<String> = from_template
            .required
            .iter()
            .chain(from_template.optional.iter())
            .map(|f| f.name.clone())
            .collect();

        let to_names: std::collections::HashSet<String> = to_template
            .required
            .iter()
            .chain(to_template.optional.iter())
            .map(|f| f.name.clone())
            .collect();

        let mut to_by_name: std::collections::HashMap<String, &FieldDef> =
            std::collections::HashMap::new();
        for f in to_template
            .required
            .iter()
            .chain(to_template.optional.iter())
        {
            to_by_name.insert(f.name.clone(), f);
        }

        let mut from_by_name: std::collections::HashMap<String, &FieldDef> =
            std::collections::HashMap::new();
        for f in from_template
            .required
            .iter()
            .chain(from_template.optional.iter())
        {
            from_by_name.insert(f.name.clone(), f);
        }

        let added_fields: Vec<String> = to_names.difference(&from_names).cloned().collect();

        let removed_fields: Vec<String> = from_names.difference(&to_names).cloned().collect();

        let mut changed_fields = Vec::new();
        for name in from_names.intersection(&to_names) {
            if let (Some(from), Some(to)) = (from_by_name.get(name), to_by_name.get(name)) {
                if from.field_type != to.field_type {
                    changed_fields.push(FieldChange {
                        field: name.clone(),
                        old_type: from.field_type.clone(),
                        new_type: to.field_type.clone(),
                    });
                }
            }
        }

        Ok(SchemaDiff {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            added_fields,
            removed_fields,
            changed_fields,
        })
    }

    async fn parse_version_template(
        &self,
        wiki_id: &str,
        _version: &str,
    ) -> Result<FrontmatterTemplate, String> {
        self.get_frontmatter_template(wiki_id).await
    }

    pub async fn migrate_pages(
        &self,
        wiki_id: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<i32, String> {
        let diff = self.diff_schemas(wiki_id, from_version, to_version).await?;
        let mut migrated = 0;

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        for note_model in db_notes {
            let mut note = axagent_core::repo::note::model_to_note(note_model.clone());
            let mut content_updated = false;

            for added_field in &diff.added_fields {
                if !note.content.contains(&format!("{}:", added_field)) {
                    let insertion = format!("\n{}: ", added_field);
                    if let Some(fm_end) = note.content.find("\n---") {
                        note.content.insert_str(fm_end, &insertion);
                        content_updated = true;
                    }
                }
            }

            if content_updated {
                let input = axagent_core::repo::note::UpdateNoteInput {
                    title: None,
                    content: Some(note.content.clone()),
                    page_type: None,
                    related_pages: None,
                };
                axagent_core::repo::note::update_note(self.db.as_ref(), &note.id, input)
                    .await
                    .map_err(|e| format!("Failed to migrate page {}: {}", note.id, e))?;
                migrated += 1;
            }
        }

        if migrated > 0 {
            let wiki_model = wikis::Entity::find_by_id(wiki_id)
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

            let mut am = wiki_model.into_active_model();
            am.schema_version = Set(to_version.to_string());
            am.updated_at = Set(chrono::Utc::now().timestamp());
            am.update(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(migrated)
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

                    if let Some(stripped) = key.strip_prefix('?') {
                        optional.push(FieldDef {
                            name: stripped.to_string(),
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

        FrontmatterTemplate { required, optional }
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
            },
            ("tags", serde_json::Value::Array(arr)) => arr.iter().all(|v| v.is_string()),
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
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse::<u32>().ok()).collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

fn compare_versions(a: &(u32, u32, u32), b: &(u32, u32, u32)) -> i32 {
    if a.0 != b.0 {
        return a.0 as i32 - b.0 as i32;
    }
    if a.1 != b.1 {
        return a.1 as i32 - b.1 as i32;
    }
    a.2 as i32 - b.2 as i32
}
