// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::{note_backlinks, note_links, notes};
use axagent_harness::core_error::{AxAgentError, Result};
pub use axagent_harness::note_dtos::{
    self, CreateNoteInput, Note, UpdateNoteInput, calculate_content_hash,
};
use axagent_harness::util_fns::gen_id;

// NoteLink DTO 在 harness 里定义（提升到 harness 让 search 等下游 crate 不用反向依赖 dao），
// 这里 re-export 保持向后兼容 — 单一类型来源。
pub use axagent_harness::types::NoteLink;

pub use axagent_harness::rag_config::NoteSearchResult;

pub fn model_to_note(m: notes::Model) -> Note {
    Note {
        id: m.id,
        vault_id: m.vault_id,
        title: m.title,
        file_path: m.file_path,
        content: m.content,
        content_hash: m.content_hash,
        author: m.author,
        page_type: m.page_type,
        source_refs: m.source_refs.map(|j| serde_json::from_value(j).unwrap_or_default()),
        related_pages: m.related_pages.map(|j| serde_json::from_value(j).unwrap_or_default()),
        quality_score: m.quality_score,
        last_linted_at: m.last_linted_at,
        last_compiled_at: m.last_compiled_at,
        compiled_source_hash: m.compiled_source_hash,
        user_edited: m.user_edited != 0,
        user_edited_at: m.user_edited_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
        is_deleted: m.is_deleted != 0,
    }
}

fn model_to_link(m: note_links::Model) -> NoteLink {
    NoteLink {
        id: m.id,
        vault_id: m.vault_id,
        source_note_id: m.source_note_id,
        target_note_id: m.target_note_id,
        link_text: m.link_text,
        link_type: m.link_type,
        created_at: m.created_at,
    }
}

pub async fn list_notes(db: &DatabaseConnection, vault_id: &str) -> Result<Vec<Note>> {
    let models = notes::Entity::find()
        .filter(notes::Column::VaultId.eq(vault_id))
        .filter(notes::Column::IsDeleted.eq(0))
        .order_by_asc(notes::Column::Title)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_note).collect())
}

/// 在数据库层面执行 Wiki 笔记搜索，带 WHERE 过滤和 LIMIT。
///
/// 避免 `list_notes` 全表加载后在内存中过滤，
/// 当笔记数量大时能显著降低内存占用和延迟。
/// 当 `vault_id` 为空字符串时不按 vault 过滤（搜索全部）。
pub async fn search_notes(
    db: &DatabaseConnection,
    vault_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<Note>> {
    let query_lower = format!("%{}%", query.to_lowercase());
    let limit = limit as u64;

    // 标题精确匹配优先，然后内容模糊匹配
    let mut select = notes::Entity::find().filter(notes::Column::IsDeleted.eq(0));
    if !vault_id.is_empty() {
        select = select.filter(notes::Column::VaultId.eq(vault_id));
    }
    let models = select
        .filter(
            Condition::any()
                .add(notes::Column::Title.like(query_lower.clone()))
                .add(notes::Column::Content.like(query_lower)),
        )
        .order_by_desc(notes::Column::QualityScore)
        .limit(limit)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_note).collect())
}

pub async fn get_note(db: &DatabaseConnection, id: &str) -> Result<Note> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    Ok(model_to_note(model))
}

pub async fn get_note_by_path(
    db: &DatabaseConnection,
    vault_id: &str,
    file_path: &str,
) -> Result<Note> {
    let model = notes::Entity::find()
        .filter(notes::Column::VaultId.eq(vault_id))
        .filter(notes::Column::FilePath.eq(file_path))
        .filter(notes::Column::IsDeleted.eq(0))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note at path {}", file_path)))?;

    Ok(model_to_note(model))
}

/// P1-1: 批量加载指定 IDs 的 notes（用于 Wiki 实体抽取等场景）
pub async fn get_notes_by_ids(db: &DatabaseConnection, ids: &[String]) -> Result<Vec<Note>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let models = notes::Entity::find()
        .filter(notes::Column::Id.is_in(ids.to_vec()))
        .filter(notes::Column::IsDeleted.eq(0))
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_note).collect())
}

pub async fn create_note(db: &DatabaseConnection, input: CreateNoteInput) -> Result<Note> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();
    let content_hash = calculate_content_hash(&input.content);

    let am = notes::ActiveModel {
        id: Set(id.clone()),
        vault_id: Set(input.vault_id.clone()),
        title: Set(input.title.clone()),
        file_path: Set(input.file_path.clone()),
        content: Set(input.content.clone()),
        content_hash: Set(content_hash),
        author: Set(input.author.clone()),
        page_type: Set(input.page_type.clone()),
        source_refs: Set(input.source_refs.map(|v| serde_json::to_value(v).unwrap_or_default())),
        related_pages: Set(None),
        quality_score: Set(None),
        last_linted_at: Set(None),
        last_compiled_at: Set(None),
        compiled_source_hash: Set(None),
        user_edited: Set(0),
        user_edited_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        is_deleted: Set(0),
    };

    am.insert(db).await?;

    let note = get_note(db, &id).await?;

    // 自动解析 [[wikilink]] 并同步 note_links + note_backlinks
    // 确保所有路径（包括批量导入、脚本桥接）都能正确建立双向链接
    if let Err(e) = sync_note_links_from_content(db, &note.vault_id, &note.id, &note.content).await
    {
        tracing::warn!("[dao::note] 笔记 {} 创建后链接同步失败: {}", note.id, e);
    }

    Ok(note)
}

pub async fn update_note(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateNoteInput,
) -> Result<Note> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    let mut am = model.into_active_model();

    if let Some(title) = input.title {
        am.title = Set(title);
    }

    if let Some(content) = input.content {
        am.content = Set(content.clone());
        am.content_hash = Set(calculate_content_hash(&content));
        am.user_edited = Set(1);
        am.user_edited_at = Set(Some(chrono::Utc::now().timestamp()));
    }

    if let Some(page_type) = input.page_type {
        am.page_type = Set(Some(page_type));
    }

    if let Some(related_pages) = input.related_pages {
        am.related_pages = Set(Some(serde_json::to_value(related_pages).unwrap_or_default()));
    }

    am.updated_at = Set(chrono::Utc::now().timestamp());

    am.update(db).await?;

    let note = get_note(db, id).await?;

    // 自动解析 [[wikilink]] 并同步 note_links + note_backlinks
    // 内容变更时必须重新解析链接，确保双向链接数据一致性
    if let Err(e) = sync_note_links_from_content(db, &note.vault_id, &note.id, &note.content).await
    {
        tracing::warn!("[dao::note] 笔记 {} 更新后链接同步失败: {}", note.id, e);
    }

    Ok(note)
}

/// 抓取管道专用的笔记内容更新：仅更新标题/正文/指纹/时间戳，
/// 不触碰 `user_edited` 标记（避免把自动更新误判为用户编辑，
/// 导致第三次抓取起被 P4 用户编辑保护永久跳过）。
///
/// 用户编辑保护（P4 冲突处理）由调用方在命中 `user_edited=true` 时自行跳过。
pub async fn update_note_from_pipeline(
    db: &DatabaseConnection,
    id: &str,
    title: &str,
    content: &str,
) -> Result<Note> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    let mut am = model.into_active_model();
    am.title = Set(title.to_string());
    am.content = Set(content.to_string());
    am.content_hash = Set(calculate_content_hash(content));
    am.updated_at = Set(chrono::Utc::now().timestamp());

    am.update(db).await?;

    let note = get_note(db, id).await?;

    // 抓取管道更新内容后也需同步链接
    if let Err(e) = sync_note_links_from_content(db, &note.vault_id, &note.id, &note.content).await
    {
        tracing::warn!("[dao::note] 笔记 {} 管道更新后链接同步失败: {}", note.id, e);
    }

    Ok(note)
}

pub async fn delete_note(db: &DatabaseConnection, id: &str) -> Result<()> {
    let model = notes::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Note {}", id)))?;

    let mut am = model.into_active_model();
    am.is_deleted = Set(1);
    am.updated_at = Set(chrono::Utc::now().timestamp());
    am.update(db).await?;

    Ok(())
}

pub async fn get_note_links(db: &DatabaseConnection, note_id: &str) -> Result<Vec<NoteLink>> {
    let models = note_links::Entity::find()
        .filter(note_links::Column::SourceNoteId.eq(note_id))
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_link).collect())
}

pub async fn get_note_backlinks(db: &DatabaseConnection, note_id: &str) -> Result<Vec<NoteLink>> {
    let models = note_backlinks::Entity::find()
        .filter(note_backlinks::Column::TargetNoteId.eq(note_id))
        .all(db)
        .await?;

    Ok(models
        .into_iter()
        .map(|m| NoteLink {
            id: m.id,
            vault_id: m.vault_id,
            source_note_id: m.source_note_id,
            target_note_id: m.target_note_id,
            link_text: m.link_text,
            link_type: m.link_type,
            created_at: m.created_at,
        })
        .collect())
}

pub async fn get_note_backlinks_by_vault(
    db: &DatabaseConnection,
    vault_id: &str,
) -> Result<Vec<NoteLink>> {
    let models = note_backlinks::Entity::find()
        .filter(note_backlinks::Column::VaultId.eq(vault_id))
        .all(db)
        .await?;

    Ok(models
        .into_iter()
        .map(|m| NoteLink {
            id: m.id,
            vault_id: m.vault_id,
            source_note_id: m.source_note_id,
            target_note_id: m.target_note_id,
            link_text: m.link_text,
            link_type: m.link_type,
            created_at: m.created_at,
        })
        .collect())
}

pub async fn create_note_link(
    db: &DatabaseConnection,
    vault_id: &str,
    source_note_id: &str,
    target_note_id: &str,
    link_text: &str,
    link_type: &str,
) -> Result<NoteLink> {
    let id = note_links::Entity::insert(note_links::ActiveModel {
        vault_id: Set(vault_id.to_string()),
        source_note_id: Set(source_note_id.to_string()),
        target_note_id: Set(target_note_id.to_string()),
        link_text: Set(link_text.to_string()),
        link_type: Set(link_type.to_string()),
        created_at: Set(chrono::Utc::now().timestamp()),
        ..Default::default()
    })
    .exec_with_returning(db)
    .await?;

    Ok(model_to_link(id))
}

pub async fn sync_note_links(
    db: &DatabaseConnection,
    vault_id: &str,
    source_note_id: &str,
    links: Vec<(String, String, String)>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // 1. 删除旧的正向链接
    note_links::Entity::delete_many()
        .filter(note_links::Column::SourceNoteId.eq(source_note_id))
        .exec(db)
        .await?;

    // 2. 删除旧的反向链接（source_note_id 作为 target 的记录）
    note_backlinks::Entity::delete_many()
        .filter(note_backlinks::Column::TargetNoteId.eq(source_note_id))
        .exec(db)
        .await?;

    // 3. 同步写入新的正向链接 + 反向链接
    for (target_note_id, link_text, link_type) in &links {
        // 正向链接：source_note → target_note
        note_links::Entity::insert(note_links::ActiveModel {
            vault_id: Set(vault_id.to_string()),
            source_note_id: Set(source_note_id.to_string()),
            target_note_id: Set(target_note_id.clone()),
            link_text: Set(link_text.clone()),
            link_type: Set(link_type.clone()),
            created_at: Set(now),
            ..Default::default()
        })
        .exec(db)
        .await?;

        // 反向链接：target_note ← source_note（自动维护 note_backlinks 索引）
        note_backlinks::Entity::insert(note_backlinks::ActiveModel {
            vault_id: Set(vault_id.to_string()),
            source_note_id: Set(source_note_id.to_string()),
            target_note_id: Set(target_note_id.clone()),
            link_text: Set(link_text.clone()),
            link_type: Set(link_type.clone()),
            created_at: Set(now),
            ..Default::default()
        })
        .exec(db)
        .await?;
    }

    Ok(())
}

pub use axagent_harness::graph_dtos::{GraphData, GraphEdge, GraphNode};

pub async fn get_vault_graph(db: &DatabaseConnection, vault_id: &str) -> Result<GraphData> {
    // 优化：用 list_notes_for_graph 只取图谱必要字段（id/title/file_path/page_type），
    // 避免 10 万节点 × 5KB content = 500MB 内存浪费。
    // tags 字段不在 notes 表中，extract_tags_from_content 需要 content，
    // 但 tags 仅用于节点展示，大图场景下前端会降级渲染，这里返回空 tags。
    let notes = list_notes_for_graph(db, vault_id).await?;
    let links =
        note_links::Entity::find().filter(note_links::Column::VaultId.eq(vault_id)).all(db).await?;
    let backlinks = note_backlinks::Entity::find()
        .filter(note_backlinks::Column::VaultId.eq(vault_id))
        .all(db)
        .await?;

    let note_ids: std::collections::HashSet<_> = notes.iter().map(|n| n.0.clone()).collect();

    let mut link_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut backlink_counts: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

    for link in &links {
        if note_ids.contains(&link.target_note_id) {
            *link_counts.entry(link.source_note_id.clone()).or_insert(0) += 1;
            *backlink_counts.entry(link.target_note_id.clone()).or_insert(0) += 1;
        }
    }

    for backlink in &backlinks {
        if note_ids.contains(&backlink.source_note_id) {
            *link_counts.entry(backlink.source_note_id.clone()).or_insert(0) += 1;
            *backlink_counts.entry(backlink.target_note_id.clone()).or_insert(0) += 1;
        }
    }

    let mut nodes: Vec<GraphNode> = Vec::new();
    for (id, title, file_path, page_type) in &notes {
        nodes.push(GraphNode {
            id: id.clone(),
            title: title.clone(),
            node_type: page_type.clone().unwrap_or_else(|| "note".to_string()),
            tags: Vec::new(),
            link_count: *link_counts.get(id).unwrap_or(&0),
            backlink_count: *backlink_counts.get(id).unwrap_or(&0),
            path: file_path.clone(),
        });
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    for link in &links {
        if note_ids.contains(&link.target_note_id) {
            edges.push(GraphEdge {
                source: link.source_note_id.clone(),
                target: link.target_note_id.clone(),
                edge_type: "link".to_string(),
            });
        }
    }

    for backlink in &backlinks {
        if note_ids.contains(&backlink.source_note_id) {
            edges.push(GraphEdge {
                source: backlink.source_note_id.clone(),
                target: backlink.target_note_id.clone(),
                edge_type: "backlink".to_string(),
            });
        }
    }

    Ok(GraphData { nodes, edges })
}

/// 图谱查询专用的轻量 notes 列表：只取 id/title/file_path/page_type，
/// 不加载 content（10 万节点 × 5KB content = 500MB，图谱无需 content）。
///
/// 返回元组 (id, title, file_path, page_type)。tags 不在此处返回
/// （需要 content 解析），大图场景前端降级渲染时不显示 tags。
pub async fn list_notes_for_graph(
    db: &DatabaseConnection,
    vault_id: &str,
) -> Result<Vec<(String, String, String, Option<String>)>> {
    let rows = notes::Entity::find()
        .filter(notes::Column::VaultId.eq(vault_id))
        .filter(notes::Column::IsDeleted.eq(0))
        .select_only()
        .column(notes::Column::Id)
        .column(notes::Column::Title)
        .column(notes::Column::FilePath)
        .column(notes::Column::PageType)
        .into_tuple::<(String, String, String, Option<String>)>()
        .all(db)
        .await?;
    Ok(rows)
}

/// 从 markdown 内容中提取 `[[Note]]` / `[[Note|alias]]` / `[[Note#anchor]]` 链接。
/// 返回去重后的目标笔记名称列表（保留原始大小写，匹配时再做归一化）。
fn extract_wikilink_targets(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'['
            && bytes[i + 1] == b'['
            && let Some(end) = content[i + 2..].find("]]")
        {
            let raw = &content[i + 2..i + 2 + end];
            // 取 | 之前、# 之前的部分作为 note 名
            let name = raw.split('|').next().unwrap_or("").split('#').next().unwrap_or("").trim();
            if !name.is_empty() && seen.insert(name.to_lowercase()) {
                names.push(name.to_string());
            }
            i += 2 + end + 2;
            continue;
        }
        i += 1;
    }
    names
}

/// 解析笔记内容中的 `[[wikilink]]` 并同步 note_links + note_backlinks 表。
/// 这是一个公共方法，允许其他模块（如 knowledge.rs 中的桥接逻辑）在创建笔记后
/// 立即解析链接，避免双向链接机制断裂。
///
/// 匹配规则：优先按 title 完全匹配（大小写不敏感），其次按 file_path 去扩展名匹配。
/// 未找到目标的链接静默跳过，避免污染链接表。
pub async fn sync_note_links_from_content(
    db: &DatabaseConnection,
    vault_id: &str,
    source_note_id: &str,
    content: &str,
) -> Result<()> {
    let target_names = extract_wikilink_targets(content);

    // 无链接时也要清空旧链接（用户可能删除了所有 wikilink）
    if target_names.is_empty() {
        return sync_note_links(db, vault_id, source_note_id, Vec::new()).await;
    }

    // 批量加载 vault 内所有笔记，构建 title/file_path → note_id 映射（大小写不敏感）
    let notes_in_vault = list_notes(db, vault_id).await?;
    let mut name_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::with_capacity(notes_in_vault.len() * 2);
    for n in &notes_in_vault {
        // 跳过自身（避免自环）
        if n.id == source_note_id {
            continue;
        }
        // 优先用 title 作为 key
        if !n.title.is_empty() {
            name_to_id.entry(n.title.to_lowercase()).or_insert_with(|| n.id.clone());
        }
        // file_path 去扩展名也作为 key（兼容 Obsidian 习惯）
        if let Some(stem) = std::path::Path::new(&n.file_path).file_stem().and_then(|s| s.to_str())
            && !stem.is_empty()
        {
            name_to_id.entry(stem.to_lowercase()).or_insert_with(|| n.id.clone());
        }
    }

    // 根据目标名称解析目标 ID，构建链接列表
    let mut links: Vec<(String, String, String)> = Vec::new();
    for target_name in &target_names {
        if let Some(target_id) = name_to_id.get(&target_name.to_lowercase()) {
            links.push((target_id.clone(), target_name.clone(), "wikilink".to_string()));
        }
    }

    sync_note_links(db, vault_id, source_note_id, links).await
}
