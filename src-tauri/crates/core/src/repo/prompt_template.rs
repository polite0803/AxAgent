use sea_orm::*;

use crate::entity::prompt_template;
use crate::entity::prompt_template_version;
use crate::error::{AxAgentError, Result};
use crate::types::{
    CreatePromptTemplateInput, ExportPromptFormat, ExportedPrompt, ImportFromUrlInput,
    ImportPromptResult, ImportPromptTemplateInput, PromptTemplate, PromptTemplateVersion,
    UpdatePromptTemplateInput,
};
use crate::utils::gen_id;

pub async fn list_prompt_templates(db: &DatabaseConnection) -> Result<Vec<PromptTemplate>> {
    let templates = prompt_template::Entity::find()
        .order_by(prompt_template::Column::UpdatedAt, Order::Desc)
        .all(db)
        .await?;

    Ok(templates.into_iter().map(model_to_template).collect())
}

pub async fn get_prompt_template(db: &DatabaseConnection, id: &str) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    Ok(model_to_template(template))
}

pub async fn create_prompt_template(
    db: &DatabaseConnection,
    input: CreatePromptTemplateInput,
) -> Result<PromptTemplate> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = gen_id();

    let tags_json = input.tags.as_ref().map(serde_json::to_string).transpose()?;

    let active_model = prompt_template::ActiveModel {
        id: Set(id),
        name: Set(input.name),
        description: Set(input.description),
        content: Set(input.content),
        variables_schema: Set(input.variables_schema),
        version: Set(1),
        is_active: Set(true),
        ab_test_enabled: Set(false),
        ab_test_variant: Set(None),
        category: Set(input.category),
        tags: Set(tags_json),
        author: Set(input.author),
        source: Set(input.source),
        source_type: Set(input.source_type),
        format: Set(input.format),
        metadata_json: Set(input.metadata_json),
        usage_count: Set(0),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let model = active_model.insert(db).await?;

    Ok(model_to_template(model))
}

pub async fn update_prompt_template(
    db: &DatabaseConnection,
    id: &str,
    input: UpdatePromptTemplateInput,
) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    let old_version = template.version;
    let new_version = if input.content.is_some() || input.variables_schema.is_some() {
        old_version + 1
    } else {
        old_version
    };

    if input.content.is_some() || input.variables_schema.is_some() {
        let version_snapshot = prompt_template_version::ActiveModel {
            id: Set(format!("{}_v{}", id, old_version)),
            template_id: Set(id.to_string()),
            version: Set(old_version),
            name: Set(template.name.clone()),
            description: Set(template.description.clone()),
            content: Set(template.content.clone()),
            variables_schema: Set(template.variables_schema.clone()),
            category: Set(template.category.clone()),
            tags: Set(template.tags.clone()),
            author: Set(template.author.clone()),
            source: Set(template.source.clone()),
            changelog: Set(Some(format!("更新到版本 {}", new_version))),
            created_at: Set(template.updated_at),
        };
        version_snapshot.insert(db).await?;
    }

    let mut active_model: prompt_template::ActiveModel = template.into();
    if let Some(name) = input.name {
        active_model.name = Set(name);
    }
    if let Some(description) = input.description {
        active_model.description = Set(Some(description));
    }
    if let Some(content) = input.content {
        active_model.content = Set(content);
    }
    if let Some(variables_schema) = input.variables_schema {
        active_model.variables_schema = Set(Some(variables_schema));
    }
    if let Some(is_active) = input.is_active {
        active_model.is_active = Set(is_active);
    }
    if let Some(ab_test_enabled) = input.ab_test_enabled {
        active_model.ab_test_enabled = Set(ab_test_enabled);
    }
    if let Some(category) = input.category {
        active_model.category = Set(Some(category));
    }
    if let Some(tags) = input.tags {
        let tags_json = serde_json::to_string(&tags).ok();
        active_model.tags = Set(tags_json);
    }
    if let Some(author) = input.author {
        active_model.author = Set(Some(author));
    }
    if let Some(source) = input.source {
        active_model.source = Set(Some(source));
    }
    if let Some(source_type) = input.source_type {
        active_model.source_type = Set(Some(source_type));
    }
    if let Some(format) = input.format {
        active_model.format = Set(Some(format));
    }
    if let Some(metadata_json) = input.metadata_json {
        active_model.metadata_json = Set(Some(metadata_json));
    }
    if let Some(is_favorite) = input.is_favorite {
        active_model.is_favorite = Set(is_favorite);
    }
    active_model.version = Set(new_version);
    active_model.updated_at = Set(chrono::Utc::now().timestamp_millis());

    let model = active_model.update(db).await?;

    Ok(model_to_template(model))
}

pub async fn delete_prompt_template(db: &DatabaseConnection, id: &str) -> Result<()> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    prompt_template_version::Entity::delete_many()
        .filter(prompt_template_version::Column::TemplateId.eq(id))
        .exec(db)
        .await?;

    template.delete(db).await?;

    Ok(())
}

pub async fn get_prompt_template_versions(
    db: &DatabaseConnection,
    template_id: &str,
) -> Result<Vec<PromptTemplateVersion>> {
    let versions = prompt_template_version::Entity::find()
        .filter(prompt_template_version::Column::TemplateId.eq(template_id))
        .order_by(prompt_template_version::Column::Version, Order::Desc)
        .all(db)
        .await?;

    Ok(versions.into_iter().map(model_to_version).collect())
}

fn model_to_template(m: prompt_template::Model) -> PromptTemplate {
    let tags: Option<Vec<String>> = m.tags.as_deref().and_then(|s| serde_json::from_str(s).ok());

    PromptTemplate {
        id: m.id,
        name: m.name,
        description: m.description,
        content: m.content,
        variables_schema: m.variables_schema,
        version: m.version,
        is_active: m.is_active,
        ab_test_enabled: m.ab_test_enabled,
        ab_test_variant: m.ab_test_variant,
        category: m.category,
        tags,
        author: m.author,
        source: m.source,
        source_type: m.source_type,
        format: m.format,
        metadata_json: m.metadata_json,
        usage_count: m.usage_count,
        is_favorite: m.is_favorite,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn model_to_version(m: prompt_template_version::Model) -> PromptTemplateVersion {
    let tags: Option<Vec<String>> = m.tags.as_deref().and_then(|s| serde_json::from_str(s).ok());

    PromptTemplateVersion {
        id: m.id,
        template_id: m.template_id,
        version: m.version,
        content: m.content,
        variables_schema: m.variables_schema,
        category: m.category,
        tags,
        author: m.author,
        source: m.source,
        changelog: m.changelog,
        created_at: m.created_at,
    }
}

// ========== 版本回滚 ==========

pub async fn rollback_prompt_template(
    db: &DatabaseConnection,
    id: &str,
    target_version: i32,
) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    let version_entry = prompt_template_version::Entity::find()
        .filter(prompt_template_version::Column::TemplateId.eq(id))
        .filter(prompt_template_version::Column::Version.eq(target_version))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("版本 {} 不存在", target_version)))?;

    // 先将当前版本存入历史
    let current_version = template.version;
    let current_snapshot = prompt_template_version::ActiveModel {
        id: Set(format!("{}_v{}", id, current_version)),
        template_id: Set(id.to_string()),
        version: Set(current_version),
        name: Set(template.name.clone()),
        description: Set(template.description.clone()),
        content: Set(template.content.clone()),
        variables_schema: Set(template.variables_schema.clone()),
        category: Set(template.category.clone()),
        tags: Set(template.tags.clone()),
        author: Set(template.author.clone()),
        source: Set(template.source.clone()),
        changelog: Set(Some(format!("回滚前自动保存（回滚至版本 {}）", target_version))),
        created_at: Set(template.updated_at),
    };
    current_snapshot.insert(db).await?;

    let new_version = current_version + 1;
    let mut active_model: prompt_template::ActiveModel = template.into();
    active_model.content = Set(version_entry.content);
    active_model.variables_schema = Set(version_entry.variables_schema);
    active_model.version = Set(new_version);
    active_model.updated_at = Set(chrono::Utc::now().timestamp_millis());

    let model = active_model.update(db).await?;
    Ok(model_to_template(model))
}

// ========== 批量导入 ==========

pub async fn import_prompt_templates(
    db: &DatabaseConnection,
    inputs: Vec<ImportPromptTemplateInput>,
) -> Result<ImportPromptResult> {
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    for input in inputs {
        let existing = prompt_template::Entity::find()
            .filter(prompt_template::Column::Name.eq(&input.name))
            .one(db)
            .await?;

        if existing.is_some() {
            skipped.push(input.name.clone());
            continue;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let id = gen_id();
        let tags_json = input.tags.as_ref().map(serde_json::to_string).transpose()?;

        let name_for_error = input.name.clone();
        let active_model = prompt_template::ActiveModel {
            id: Set(id),
            name: Set(input.name),
            description: Set(input.description),
            content: Set(input.content),
            variables_schema: Set(input.variables_schema),
            version: Set(1),
            is_active: Set(true),
            ab_test_enabled: Set(false),
            ab_test_variant: Set(None),
            category: Set(input.category),
            tags: Set(tags_json),
            author: Set(input.author),
            source: Set(input.source),
            source_type: Set(input.source_type),
            format: Set(input.format),
            metadata_json: Set(input.metadata_json),
            usage_count: Set(0),
            is_favorite: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match active_model.insert(db).await {
            Ok(model) => imported.push(model_to_template(model)),
            Err(e) => errors.push(format!("导入失败 {}: {}", name_for_error, e)),
        }
    }

    Ok(ImportPromptResult {
        imported,
        skipped,
        errors,
    })
}

// ========== 导出 ==========

pub async fn export_prompt_templates(
    db: &DatabaseConnection,
    ids: Vec<String>,
    format: ExportPromptFormat,
) -> Result<String> {
    let templates = if ids.is_empty() {
        prompt_template::Entity::find().all(db).await?
    } else {
        prompt_template::Entity::find()
            .filter(prompt_template::Column::Id.is_in(ids))
            .all(db)
            .await?
    };

    let exported: Vec<ExportedPrompt> = templates
        .into_iter()
        .map(|t| {
            let tags: Option<Vec<String>> =
                t.tags.as_deref().and_then(|s| serde_json::from_str(s).ok());

            ExportedPrompt {
                name: t.name,
                description: t.description,
                content: t.content,
                variables_schema: t.variables_schema,
                category: t.category,
                tags,
                author: t.author,
                source: t.source,
            }
        })
        .collect();

    match format {
        ExportPromptFormat::Json => serde_json::to_string_pretty(&exported)
            .map_err(|e| AxAgentError::Internal(format!("JSON 序列化失败: {}", e))),
        ExportPromptFormat::Yaml => serde_yaml::to_string(&exported)
            .map_err(|e| AxAgentError::Internal(format!("YAML 序列化失败: {}", e))),
        ExportPromptFormat::Markdown => Ok(export_to_markdown(&exported)),
    }
}

fn export_to_markdown(prompts: &[ExportedPrompt]) -> String {
    let mut md = String::from("# 提示词模板导出\n\n");
    for p in prompts {
        md.push_str(&format!("## {}\n\n", p.name));
        if let Some(desc) = &p.description {
            md.push_str(&format!("_{}_\n\n", desc));
        }
        if let Some(cat) = &p.category {
            md.push_str(&format!("**分类**: {}\n\n", cat));
        }
        if let Some(tags) = &p.tags
            && !tags.is_empty()
        {
            md.push_str(&format!("**标签**: {}\n\n", tags.join(", ")));
        }
        if let Some(author) = &p.author {
            md.push_str(&format!("**作者**: {}\n\n", author));
        }
        md.push_str("### Prompt\n\n");
        md.push_str(&p.content);
        md.push_str("\n\n---\n\n");
    }
    md
}

// ========== 从 URL 导入 ==========

pub async fn import_from_url(
    db: &DatabaseConnection,
    input: ImportFromUrlInput,
) -> Result<ImportPromptResult> {
    let client = reqwest::Client::builder()
        .user_agent("AxAgent/1.5")
        .build()
        .map_err(|e| AxAgentError::Internal(format!("HTTP 客户端创建失败: {}", e)))?;

    let api_url = parse_github_url(&input.url)?;

    let resp = client
        .get(&api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| AxAgentError::Internal(format!("请求失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AxAgentError::Internal(format!(
            "GitHub API 返回 {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let files: Vec<GitHubContent> = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Internal(format!("解析 GitHub 响应失败: {}", e)))?;

    let mut inputs = Vec::new();
    for file in files {
        if file.name.ends_with(".md")
            && let Some(content_url) = file.download_url
        {
            match fetch_and_parse_prompt(&client, &content_url).await {
                Ok(Some(parsed_input)) => {
                    if let Some(ref filter) = input.category_filter
                        && let Some(ref cat) = parsed_input.category
                        && !cat.contains(filter)
                    {
                        continue;
                    }
                    inputs.push(parsed_input);
                },
                Ok(None) => {},
                Err(e) => {
                    tracing::warn!("跳过文件 {}: {}", file.name, e);
                },
            }
        }
    }

    import_prompt_templates(db, inputs).await
}

fn parse_github_url(url: &str) -> std::result::Result<String, AxAgentError> {
    let url = url.trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);

    let parts: Vec<&str> = url.split("github.com/").collect();
    if parts.len() != 2 {
        return Err(AxAgentError::Internal("无法解析 GitHub URL".into()));
    }

    let path_parts: Vec<&str> = parts[1].split('/').collect();
    if path_parts.len() < 2 {
        return Err(AxAgentError::Internal("URL 格式不正确，需要 owner/repo".into()));
    }

    let owner = path_parts[0];
    let repo = path_parts[1];

    let sub_path = if path_parts.len() > 4 && path_parts[2] == "tree" {
        path_parts[4..].join("/")
    } else {
        String::from("prompts")
    };

    Ok(format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, sub_path))
}

// ========== 从本地文件夹导入 ==========

pub async fn import_from_folder(
    db: &DatabaseConnection,
    folder_path: &str,
    category_filter: Option<String>,
) -> Result<ImportPromptResult> {
    let mut inputs = Vec::new();

    for entry in walkdir::WalkDir::new(folder_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension() else { continue };
        if ext != "md" && ext != "MD" {
            continue;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                match parse_yao_prompt(&content) {
                    Ok(Some(parsed_input)) => {
                        if let Some(ref filter) = category_filter
                            && let Some(ref cat) = parsed_input.category
                            && !cat.contains(filter)
                        {
                            continue;
                        }
                        inputs.push(parsed_input);
                    },
                    Ok(None) => {
                        // 没有 YAML frontmatter 的文件，将整个文件作为 prompt 内容
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("未命名")
                            .to_string();
                        inputs.push(ImportPromptTemplateInput {
                            name,
                            description: None,
                            content,
                            variables_schema: None,
                            category: None,
                            tags: None,
                            author: None,
                            source: Some(format!("file://{}", folder_path)),
                            source_type: Some("folder_import".into()),
                            format: Some("markdown".into()),
                            metadata_json: None,
                        });
                    },
                    Err(e) => {
                        tracing::warn!("跳过文件 {}: {}", path.display(), e);
                    },
                }
            },
            Err(e) => {
                tracing::warn!("读取文件失败 {}: {}", path.display(), e);
            },
        }
    }

    import_prompt_templates(db, inputs).await
}

#[derive(Debug, serde::Deserialize)]
struct GitHubContent {
    name: String,
    #[serde(rename = "download_url")]
    download_url: Option<String>,
}

async fn fetch_and_parse_prompt(
    client: &reqwest::Client,
    url: &str,
) -> std::result::Result<Option<ImportPromptTemplateInput>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;

    let content = resp
        .text()
        .await
        .map_err(|e| format!("读取内容失败: {}", e))?;

    parse_yao_prompt(&content)
}

/// 解析 YAML frontmatter + Markdown 格式的提示词文件
pub fn parse_yao_prompt(
    content: &str,
) -> std::result::Result<Option<ImportPromptTemplateInput>, String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Ok(None);
    }

    let rest = &content[3..];
    let end_idx = rest
        .find("---")
        .ok_or_else(|| "缺少 YAML frontmatter 结束标记".to_string())?;

    let yaml_str = &rest[..end_idx].trim();
    let body = rest[end_idx + 3..].trim();

    let fm: serde_yaml::Value =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML 解析失败: {}", e))?;

    let title = fm
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();

    let description = fm
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let category = fm
        .get("category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tags = fm.get("tags").and_then(|v| {
        if let Some(arr) = v.as_sequence() {
            Some(
                arr.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect(),
            )
        } else {
            v.as_str().map(|s| vec![s.to_string()])
        }
    });

    let author = fm
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source = fm
        .get("source")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let prompt_content = if body.is_empty() {
        content.to_string()
    } else {
        body.to_string()
    };

    Ok(Some(ImportPromptTemplateInput {
        name: title,
        description,
        content: prompt_content,
        variables_schema: None,
        category,
        tags,
        author,
        source,
        source_type: Some("imported".into()),
        format: Some("markdown".into()),
        metadata_json: Some(
            serde_json::to_string(&serde_json::json!({
                "original_yaml": yaml_str,
            }))
            .unwrap_or_default(),
        ),
    }))
}

// ========== 使用计数 ==========

pub async fn increment_usage_count(db: &DatabaseConnection, id: &str) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    let usage = template.usage_count;
    let mut active_model: prompt_template::ActiveModel = template.into();
    active_model.usage_count = Set(usage + 1);

    let model = active_model.update(db).await?;
    Ok(model_to_template(model))
}
