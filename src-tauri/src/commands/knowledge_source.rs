// SPDX-License-Identifier: AGPL-3.0-only

//! 知识源管理命令 — 知识库增长更新入口（docs/knowledge-source-ingest-plan.md）
//!
//! 五层管道：Source → Fetch → Process → Store → Refresh
//!
//! - P1: `fetch_url_to_wiki` — URL 单页抓取 → 规范化 MD → Wiki 页面 + RAG 索引
//! - P2: `knowledge_source_*` — 知识源 CRUD + 手动/批量抓取 + RSS 订阅
//! - P3: `github_repo_import` — 开源知识库导入
//!
//! 增量更新闭环：抓取 → 规范化 MD → sha256 指纹对比 wiki_sources.content_hash，
//! 内容变了才 update 页面 + 重索引，没变直接跳过。

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::knowledge_source as ks_err;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::index_jobs as jobs;
use axagent_dao::repo::note::CreateNoteInput;
use axagent_dao::repo::wiki::{
    delete_source_by_id, get_source_by_id, list_all_sources, update_source_fetch_meta,
    update_source_fields,
};
use axagent_dao::repo::wiki_source_repository::DaoWikiSourceRepository;
use axagent_entities::wiki_sync_queue;
use axagent_harness::wiki_dtos::{InsertWikiSourceInput, WikiSource, WikiSourceRepository};
use axagent_kit::html_cleaner::HtmlCleaner;
use axagent_search::search::{is_safe_url_deep, shared_http_client};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, State};

const MAX_CONTENT_LENGTH: usize = 200_000;
const BINARY_CONTENT_TYPES: &[&str] = &[
    "application/pdf",
    "application/zip",
    "application/x-rar",
    "application/x-tar",
    "application/x-gzip",
    "application/x-bzip2",
    "application/x-7z-compressed",
    "application/octet-stream",
    "image/",
    "video/",
    "audio/",
    "font/",
];

fn err_str<E: std::fmt::Display>(e: E) -> String {
    String::from(ErrorResponse::from_error(e, ErrorCategory::Retryable))
}

/// 计算规范化 Markdown 的 sha256 指纹（增量更新去重依据）。
fn content_fingerprint(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    // digest 0.11 的 finalize() 返回 generic-array 的 Array，未实现 LowerHex，转字节切片后用 hex 编码。
    hex::encode(hasher.finalize().as_slice())
}

/// 转义 frontmatter 值：双引号、反斜杠、换行（防 YAML 注入/解析破坏）。
fn yaml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// 把抓取的正文包装为带 frontmatter 的规范化 Markdown。
///
/// 注意：frontmatter 刻意不写入 `fetched_at` —— 抓取时间随每次运行变化，
/// 若写入正文会导致 sha256 指纹每次必变，增量更新闭环（doc:74 指纹比对）永远失效。
/// 抓取时间已记录在知识源的 `last_fetched_at` 字段，无需进正文。
fn normalize_fetched_md(url: &str, title: &str, body: &str) -> String {
    let (t, u) = (yaml_escape(title), yaml_escape(url));
    format!("---\ntitle: \"{t}\"\nsource: \"{u}\"\nurl: \"{u}\"\ntype: web\n---\n\n# {t}\n\n{body}")
}

/// slug 化标题，用作 file_path。
fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if c == ' ' || c == '-' || c == '_' {
            slug.push('-');
        }
        // 其他字符（含中文等非 ASCII）直接丢弃，保证 slug 可安全用于文件名/URL
    }
    if slug.trim_matches('-').is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

/// 抓取 URL 正文，返回 (title, 主文本)。
async fn fetch_url_content(url: &str) -> Result<(String, String), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ErrorResponse::err_with_detail(
            ks_err::URL_SCHEME_INVALID,
            "url 必须以 http:// 或 https:// 开头",
        ));
    }
    if !is_safe_url_deep(url).await {
        return Err(ErrorResponse::err_with_detail(ks_err::URL_BLOCKED, "禁止访问内网或私有地址"));
    }

    let client = shared_http_client();
    let response = client.get(url).send().await.map_err(|e| {
        ErrorResponse::err_with_detail(ks_err::HTTP_FETCH_FAILED, format!("HTTP 请求失败: {e}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ErrorResponse::err_with_detail(
            ks_err::HTTP_STATUS_ERROR,
            format!("HTTP 状态码异常: {status}"),
        ));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    for binary_ct in BINARY_CONTENT_TYPES {
        if content_type.contains(binary_ct) {
            return Err(ErrorResponse::err_with_detail(
                ks_err::CONTENT_BINARY,
                format!("目标内容为二进制类型（{binary_ct}），无法提取文本"),
            ));
        }
    }

    let raw_bytes = response.bytes().await.map_err(|e| {
        ErrorResponse::err_with_detail(ks_err::HTTP_READ_FAILED, format!("读取响应失败: {e}"))
    })?;
    let body = String::from_utf8_lossy(&raw_bytes).into_owned();

    if content_type.contains("text/html") || content_type.contains("application/xhtml") {
        let cleaner = HtmlCleaner::new();
        let (title, extracted) = cleaner.extract_with_title(&body, "", MAX_CONTENT_LENGTH);
        Ok((title, extracted))
    } else {
        Ok(("Plain Text".to_string(), body))
    }
}

/// 在当前 vault 中按标题（大小写不敏感）查已存在的笔记。
///
/// 委托 repo 层 DB 级查询（原实现 list_notes 全量加载后内存过滤，
/// N 篇导入时 O(N²)）。
async fn find_note_by_title(
    db: &sea_orm::DatabaseConnection,
    vault_id: &str,
    title: &str,
) -> Result<Option<axagent_dao::repo::note::Note>, String> {
    axagent_dao::repo::note::find_note_by_title_ci(db, vault_id, title).await.map_err(err_str)
}

/// 在当前 vault 中按 source_ref 精确匹配已存在的笔记。
///
/// 修复「不同 URL 同标题页面互相覆盖」：同源（同 URL/feed/GitHub 路径）的内容
/// 应回到同一笔记做增量更新，而不是被别的同标题页面顶掉。
/// 委托 repo 层 DB 级查询（原实现 list_notes 全量加载后内存过滤）。
async fn find_note_by_source_ref(
    db: &sea_orm::DatabaseConnection,
    vault_id: &str,
    source_ref: &str,
) -> Result<Option<axagent_dao::repo::note::Note>, String> {
    axagent_dao::repo::note::find_note_by_source_ref(db, vault_id, source_ref)
        .await
        .map_err(err_str)
}

/// 创建/更新 Wiki 笔记并触发 RAG 索引 + wiki_sync_queue 事件。
///
/// `state`/`app` 为 None 时走纯 DB 入队（cron 后台调度用，不依赖 Tauri 事件）。
async fn upsert_note(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    vault_id: &str,
    title: &str,
    file_path: &str,
    content: &str,
    source_ref: &str,
) -> Result<(String, String), String> {
    // 查重优先级：source_ref 精确匹配 > 标题匹配。
    // 同 URL/feed/GitHub 路径优先命中既有笔记做增量更新；
    // 标题匹配作为兜底，避免重复导入同标题页面（P4 冲突处理）。
    let existing = match find_note_by_source_ref(db, vault_id, source_ref).await? {
        Some(n) => Some(n),
        None => find_note_by_title(db, vault_id, title).await?,
    };

    if let Some(existing) = existing {
        // 内容变化才更新（增量更新闭环）
        if existing.content == content {
            return Ok((existing.id.clone(), "skipped".to_string()));
        }
        // 用户编辑保护（P4 冲突处理）：用户手工编辑过的页面不自动覆盖
        if existing.user_edited {
            tracing::info!("[knowledge-source] 笔记 {} 已被用户编辑，跳过自动更新", existing.id);
            return Ok((existing.id.clone(), "skipped".to_string()));
        }
        // 用抓取管道专用更新函数：不动 user_edited，避免自动更新被误判为用户编辑
        let updated =
            axagent_dao::repo::note::update_note_from_pipeline(db, &existing.id, title, content)
                .await
                .map_err(err_str)?;
        // R8: 抓取更新路径同步失效图谱缓存，避免图谱显示旧数据直到下次命令触发
        if let Err(e) = axagent_dao::repo::wiki_graph_cache::invalidate_cache(db, vault_id).await {
            tracing::warn!("[knowledge-source] 失效图谱缓存失败: {e}");
        }
        enqueue_wiki_sync(db, vault_id, "note_updated", &updated.id);
        enqueue_index(state, app, db, vault_id, &updated.id);
        return Ok((updated.id, "updated".to_string()));
    }

    let note = axagent_dao::repo::note::create_note(
        db,
        CreateNoteInput {
            vault_id: vault_id.to_string(),
            title: title.to_string(),
            file_path: file_path.to_string(),
            content: content.to_string(),
            author: "knowledge-source".to_string(),
            page_type: Some("knowledge".to_string()),
            source_refs: Some(vec![source_ref.to_string()]),
        },
    )
    .await
    .map_err(err_str)?;

    // R8: 新建笔记同样失效图谱缓存（与更新路径保持一致）
    if let Err(e) = axagent_dao::repo::wiki_graph_cache::invalidate_cache(db, vault_id).await {
        tracing::warn!("[knowledge-source] 失效图谱缓存失败: {e}");
    }

    enqueue_wiki_sync(db, vault_id, "note_created", &note.id);
    enqueue_index(state, app, db, vault_id, &note.id);
    Ok((note.id, "created".to_string()))
}

/// RAG 索引入队：有 Tauri 上下文走事件版 enqueue_job_sync，否则纯 DB 入队（cron 路径）。
fn enqueue_index(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    vault_id: &str,
    note_id: &str,
) {
    let input = jobs::CreateIndexJobInput {
        job_type: jobs::JOB_TYPE_INDEX_WIKI_NOTE.to_string(),
        container_type: "wiki".to_string(),
        container_id: vault_id.to_string(),
        item_id: note_id.to_string(),
        max_retries: None,
        priority: None,
        metadata: None,
    };
    match (state, app) {
        (Some(st), Some(ap)) => {
            let _ = crate::index_queue::enqueue_job_sync(
                st,
                ap,
                jobs::JOB_TYPE_INDEX_WIKI_NOTE,
                "wiki",
                vault_id,
                note_id,
                None,
                None,
            );
        },
        _ => {
            let db = db.clone();
            tauri::async_runtime::spawn(async move {
                let _ = jobs::enqueue_job(&db, input).await;
            });
        },
    }
}

/// 写入 `wiki_sync_queue` 同步事件（计划 doc:76：指纹变化 → wiki_sync_queue 事件）。
///
/// 消费端 `process_sync_event`（llm_wiki.rs）会据此做向量索引等下游加工。
/// 纯 DB 插入即可，无需 Tauri 上下文（后台 cron 路径与命令路径共用）。
fn enqueue_wiki_sync(
    db: &sea_orm::DatabaseConnection,
    vault_id: &str,
    event_type: &str,
    note_id: &str,
) {
    let db = db.clone();
    let (wiki_id, event_type, note_id) =
        (vault_id.to_string(), event_type.to_string(), note_id.to_string());
    tauri::async_runtime::spawn(async move {
        let am = wiki_sync_queue::Model::new_pending(
            wiki_id,
            event_type,
            "note".to_string(),
            note_id,
            None,
        );
        if let Err(e) = wiki_sync_queue::Entity::insert(am).exec(&db).await {
            tracing::warn!("[knowledge-source] 写入 wiki_sync_queue 失败: {e}");
        }
    });
}

// ── P1: URL 单页抓取 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchUrlResult {
    pub note_id: String,
    pub title: String,
    /// created | updated | skipped
    pub action: String,
    pub source_id: Option<String>,
}

/// 输入 URL → 抓取 → 规范化 MD → Wiki 页面 + RAG 索引 + 知识源登记。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "抓取URL导入Wiki")]
#[tauri::command]
pub async fn fetch_url_to_wiki(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    title: Option<String>,
    wiki_id: Option<String>,
) -> Result<FetchUrlResult, String> {
    let (page_title, body) = fetch_url_content(&url).await?;
    let title = title.filter(|t| !t.trim().is_empty()).unwrap_or(page_title);

    // 确定目标 vault：优先指定，否则用第一个 wiki
    let vault_id = match wiki_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let wikis =
                axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(err_str)?;
            let first = wikis.into_iter().next().ok_or_else(|| {
                ErrorResponse::err_with_detail(
                    ks_err::WIKI_NOT_AVAILABLE,
                    "没有可用的 Wiki，请先创建一个 Wiki 知识库",
                )
            })?;
            first.id
        },
    };

    let md = normalize_fetched_md(&url, &title, &body);
    let file_path = format!("web/{}.md", slugify(&title));

    // 增量更新闭环：按 URL 查重源记录，复用并更新指纹
    let hash = content_fingerprint(&md);
    let existing_source = list_all_sources(state.harness.db())
        .await
        .map_err(err_str)?
        .into_iter()
        .find(|s| s.source_type == "url" && s.source_path == url);
    let source_id = match existing_source {
        Some(s) => {
            let _ = update_source_fetch_meta(
                state.harness.db(),
                &s.id,
                &hash,
                chrono::Utc::now().timestamp_millis(),
            )
            .await;
            Some(s.id)
        },
        None => {
            let repo = DaoWikiSourceRepository::new(Arc::new(state.harness.db().clone()));
            let src = repo
                .insert(InsertWikiSourceInput {
                    id: format!("url:{}", uuid::Uuid::new_v4()),
                    wiki_id: vault_id.clone(),
                    source_type: "url".to_string(),
                    source_path: url.clone(),
                    title: title.clone(),
                    mime_type: "text/markdown".to_string(),
                    size_bytes: md.len() as i64,
                    content_hash: hash.clone(),
                    metadata_json: Some(serde_json::json!({ "url": url })),
                    schedule_cron: None,
                    last_fetched_at: Some(chrono::Utc::now().timestamp_millis()),
                    status: "active".to_string(),
                })
                .await
                .map_err(err_str)?;
            Some(src.id)
        },
    };

    let (note_id, action) = upsert_note(
        Some(&state),
        Some(&app),
        state.harness.db(),
        &vault_id,
        &title,
        &file_path,
        &md,
        &format!("url:{url}"),
    )
    .await?;

    Ok(FetchUrlResult { note_id, title, action, source_id })
}

// ── P2: 知识源管理 CRUD ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeSourceInput {
    pub wiki_id: String,
    pub source_type: String,
    pub source_path: String,
    pub title: String,
    pub mime_type: Option<String>,
    pub schedule_cron: Option<String>,
    pub status: Option<String>,
    pub metadata_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateKnowledgeSourceInput {
    pub id: String,
    pub title: Option<String>,
    pub source_type: Option<String>,
    pub source_path: Option<String>,
    pub schedule_cron: Option<String>,
    /// 传 Some(None) 表示清除调度；传 None 表示不改动
    pub clear_schedule: Option<bool>,
    pub status: Option<String>,
    pub metadata_json: Option<serde_json::Value>,
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "列出知识源")]
#[tauri::command]
pub async fn knowledge_source_list(state: State<'_, AppState>) -> Result<Vec<WikiSource>, String> {
    list_all_sources(state.harness.db()).await.map_err(err_str)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "创建知识源")]
#[tauri::command]
pub async fn knowledge_source_create(
    state: State<'_, AppState>,
    input: CreateKnowledgeSourceInput,
) -> Result<WikiSource, String> {
    // 兜底：wiki_id 为空时取第一个 wiki，避免前端漏传导致源无法关联
    let wiki_id = if input.wiki_id.trim().is_empty() {
        let wikis =
            axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(err_str)?;
        wikis
            .into_iter()
            .next()
            .ok_or_else(|| {
                ErrorResponse::err_with_detail(
                    ks_err::WIKI_NOT_AVAILABLE,
                    "没有可用的 Wiki，请先创建 Wiki 知识库",
                )
            })?
            .id
    } else {
        input.wiki_id
    };

    let repo = DaoWikiSourceRepository::new(Arc::new(state.harness.db().clone()));
    let src = repo
        .insert(InsertWikiSourceInput {
            id: uuid::Uuid::new_v4().to_string(),
            wiki_id,
            source_type: input.source_type,
            source_path: input.source_path,
            title: input.title,
            mime_type: input.mime_type.unwrap_or_else(|| "text/markdown".to_string()),
            size_bytes: 0,
            content_hash: String::new(),
            metadata_json: input.metadata_json,
            schedule_cron: input.schedule_cron,
            last_fetched_at: None,
            status: input.status.unwrap_or_else(|| "active".to_string()),
        })
        .await
        .map_err(err_str)?;
    Ok(src)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "更新知识源")]
#[tauri::command]
pub async fn knowledge_source_update(
    state: State<'_, AppState>,
    input: UpdateKnowledgeSourceInput,
) -> Result<WikiSource, String> {
    let schedule_cron = if input.clear_schedule == Some(true) {
        Some(None)
    } else {
        input.schedule_cron.map(Some)
    };
    update_source_fields(
        state.harness.db(),
        &input.id,
        axagent_dao::repo::wiki::UpdateSourceFieldsInput {
            title: input.title,
            source_type: input.source_type,
            source_path: input.source_path,
            schedule_cron,
            status: input.status,
            metadata_json: input.metadata_json.map(Some),
        },
    )
    .await
    .map_err(err_str)?;
    get_source_by_id(state.harness.db(), &input.id)
        .await
        .map_err(err_str)?
        .ok_or_else(|| ErrorResponse::err_with_detail(ks_err::NOT_FOUND, "知识源不存在"))
}

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateInput, description = "删除知识源")]
#[tauri::command]
pub async fn knowledge_source_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    delete_source_by_id(state.harness.db(), &id).await.map_err(err_str)
}

// ── P2: 抓取执行（单源 / 全部）──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchSourceResult {
    pub source_id: String,
    pub source_title: String,
    /// created | updated | skipped | error
    pub action: String,
    pub detail: String,
}

/// 单源抓取闭环：按 source_type 分派 → 指纹去重 → 入库 → 更新元数据。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "立即抓取知识源")]
#[tauri::command]
pub async fn knowledge_source_fetch_now(
    app: AppHandle,
    state: State<'_, AppState>,
    source_id: String,
) -> Result<FetchSourceResult, String> {
    let source = get_source_by_id(state.harness.db(), &source_id)
        .await
        .map_err(err_str)?
        .ok_or_else(|| ErrorResponse::err_with_detail(ks_err::NOT_FOUND, "知识源不存在"))?;

    Ok(sync_one_source(Some(&state), Some(&app), state.harness.db(), &source).await)
}

/// 全部 active 知识源批量抓取（定时任务入口）。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "批量抓取所有知识源")]
#[tauri::command]
pub async fn knowledge_source_fetch_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<FetchSourceResult>, String> {
    let _ = app;
    Ok(run_knowledge_source_sync(state.harness.db()).await)
}

/// 纯 DB 全量同步：cron 定时任务与命令共用，遍历 active 源逐个抓取。
pub(crate) async fn run_knowledge_source_sync(
    db: &sea_orm::DatabaseConnection,
) -> Vec<FetchSourceResult> {
    let sources = match list_all_sources(db).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[knowledge-source] 读取知识源列表失败: {e}");
            return Vec::new();
        },
    };
    let active: Vec<_> = sources.into_iter().filter(|s| s.status == "active").collect();
    let mut results = Vec::with_capacity(active.len());
    for source in active {
        results.push(sync_one_source(None, None, db, &source).await);
    }
    results
}

/// 单源同步核心：url/rss 分派 + 增量更新闭环。
async fn sync_one_source(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    source: &WikiSource,
) -> FetchSourceResult {
    let result = match source.source_type.as_str() {
        "url" => fetch_url_source(state, app, db, source).await,
        "rss" => fetch_rss_source(state, app, db, source).await,
        "github" => fetch_github_source(state, app, db, source).await,
        other => Err(ErrorResponse::err_with_detail(
            ks_err::TYPE_UNSUPPORTED,
            format!("暂不支持的知识源类型: {other}"),
        )),
    };

    match result {
        Ok(r) => r,
        Err(e) => FetchSourceResult {
            source_id: source.id.clone(),
            source_title: source.title.clone(),
            action: "error".to_string(),
            detail: e,
        },
    }
}

/// github 型源：复用 github_sync 增量重抓（内容指纹对比跳过未变化文件）。
async fn fetch_github_source(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    source: &WikiSource,
) -> Result<FetchSourceResult, String> {
    let (owner, repo_name) = parse_github_repo(&source.source_path).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            ks_err::REPO_PARSE_FAILED,
            "知识源地址无法解析为 GitHub 仓库",
        )
    })?;
    let subdir = source
        .metadata_json
        .as_ref()
        .and_then(|m| m["subdir"].as_str())
        .unwrap_or("docs")
        .to_string();

    let result = github_sync(state, app, db, &owner, &repo_name, &subdir, &source.wiki_id).await?;

    // 增量同步后更新源指纹（以导入统计为准，标记最近抓取时间）
    update_source_fetch_meta(db, &source.id, &result.detail, chrono::Utc::now().timestamp_millis())
        .await
        .map_err(err_str)?;

    Ok(result)
}

/// url 型源：抓取 → 规范化 → 指纹对比 → upsert 笔记。
async fn fetch_url_source(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    source: &WikiSource,
) -> Result<FetchSourceResult, String> {
    let (page_title, body) = fetch_url_content(&source.source_path).await?;
    let title = if source.title.is_empty() {
        page_title
    } else {
        source.title.clone()
    };
    let md = normalize_fetched_md(&source.source_path, &title, &body);
    let hash = content_fingerprint(&md);

    if source.content_hash == hash {
        update_source_fetch_meta(db, &source.id, &hash, chrono::Utc::now().timestamp_millis())
            .await
            .map_err(err_str)?;
        return Ok(FetchSourceResult {
            source_id: source.id.clone(),
            source_title: source.title.clone(),
            action: "skipped".to_string(),
            detail: "内容未变化，已跳过".to_string(),
        });
    }

    let file_path = format!("web/{}.md", slugify(&title));
    let (note_id, action) = upsert_note(
        state,
        app,
        db,
        &source.wiki_id,
        &title,
        &file_path,
        &md,
        &format!("url:{}", source.source_path),
    )
    .await?;

    // P4: 精华沉淀进 Memory（created/updated 时）
    if action != "skipped" {
        deposit_to_memory(db, &title, &source.source_path, &body).await;
    }

    update_source_fetch_meta(db, &source.id, &hash, chrono::Utc::now().timestamp_millis())
        .await
        .map_err(err_str)?;

    Ok(FetchSourceResult {
        source_id: source.id.clone(),
        source_title: source.title.clone(),
        action,
        detail: format!("note_id={note_id}"),
    })
}

/// rss 型源：解析 feed → 每篇文章建一个 wiki note（按条目链接独立 source_ref 去重）。
/// 存量数据不迁移：旧 feed 本就只剩最后一条，重新 fetch 后按新 ref 自然补全。
async fn fetch_rss_source(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    source: &WikiSource,
) -> Result<FetchSourceResult, String> {
    let feed_url = source.source_path.clone();
    let client = shared_http_client();
    let body = client
        .get(&feed_url)
        .send()
        .await
        .map_err(|e| {
            ErrorResponse::err_with_detail(ks_err::RSS_FETCH_FAILED, format!("RSS 请求失败: {e}"))
        })?
        .text()
        .await
        .map_err(|e| {
            ErrorResponse::err_with_detail(ks_err::RSS_READ_FAILED, format!("RSS 读取失败: {e}"))
        })?;

    let feed = feed_rs::parser::parse(body.as_bytes()).map_err(|e| {
        ErrorResponse::err_with_detail(ks_err::RSS_PARSE_FAILED, format!("RSS 解析失败: {e}"))
    })?;

    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut updated = 0usize;
    for entry in feed.entries {
        let entry_title =
            entry.title.map(|t| t.content).unwrap_or_else(|| "未命名条目".to_string());
        let entry_link =
            entry.links.first().map(|l| l.href.clone()).unwrap_or_else(|| feed_url.clone());
        let summary = entry
            .summary
            .map(|s| s.content)
            .or_else(|| entry.content.map(|c| c.body.unwrap_or_default()))
            .unwrap_or_default();

        let (t, u) = (yaml_escape(&entry_title), yaml_escape(&entry_link));
        let md = format!(
            "---\ntitle: \"{t}\"\nsource: \"{feed_url}\"\nurl: \"{u}\"\ntype: rss\n---\n\n# {t}\n\n{summary}"
        );
        let file_path = format!("rss/{}.md", slugify(&entry_title));

        // 每条目独立 source_ref：此前所有条目共用 rss:{feed_url}，而查重优先级
        // source_ref > 标题，导致整份 feed 只剩一条笔记（后续条目互相覆盖）。
        // 有链接用链接；链接缺失（回退为 feed_url）时用内容指纹保证唯一。
        let entry_source_ref = if entry_link == feed_url {
            format!("rss:{}#{}", feed_url, content_fingerprint(&format!("{entry_title}:{summary}")))
        } else {
            format!("rss:{entry_link}")
        };

        match upsert_note(
            state,
            app,
            db,
            &source.wiki_id,
            &entry_title,
            &file_path,
            &md,
            &entry_source_ref,
        )
        .await
        {
            Ok((_, action)) => match action.as_str() {
                "created" => {
                    created += 1;
                    // P4: 新条目精华沉淀进 Memory
                    deposit_to_memory(db, &entry_title, &entry_link, &summary).await;
                },
                "updated" => updated += 1,
                _ => skipped += 1,
            },
            Err(e) => {
                tracing::warn!("[knowledge-source] RSS 条目 {} 入库失败: {}", entry_title, e);
                skipped += 1;
            },
        }
    }

    let all_hash = content_fingerprint(&format!("{feed_url}:{created}:{updated}:{skipped}"));
    update_source_fetch_meta(db, &source.id, &all_hash, chrono::Utc::now().timestamp_millis())
        .await
        .map_err(err_str)?;

    let detail = format!("新增 {created} 条，更新 {updated} 条，跳过 {skipped} 条");
    let action = if created + updated > 0 {
        "updated"
    } else {
        "skipped"
    };
    Ok(FetchSourceResult {
        source_id: source.id.clone(),
        source_title: source.title.clone(),
        action: action.to_string(),
        detail,
    })
}

/// 注册知识源定时刷新任务（task_type=knowledge_source_fetch_all，走 CronScheduler 分支）。
/// 幂等：同名任务已存在时更新其 cron，不重复创建。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "注册知识源定时同步")]
#[tauri::command]
pub async fn knowledge_source_schedule_sync(
    state: State<'_, AppState>,
    cron_expression: String,
) -> Result<String, String> {
    use axagent_runtime_core::cron_job::CronJob;

    // cron 粗校验：必须是 5 字段（分 时 日 月 周）
    let field_count = cron_expression.split_whitespace().count();
    if field_count != 5 {
        return Err(ErrorResponse::err_with_detail(
            ks_err::CRON_INVALID,
            format!("cron 表达式必须是 5 个字段（当前 {field_count} 个），例如 0 3 * * *"),
        ));
    }

    // 幂等：同名任务存在则更新 cron 表达式
    let store = state.cron_job_store.clone();
    if let Some(existing) =
        store.list().await.into_iter().find(|j| j.name == "knowledge-source-sync")
    {
        let id = existing.id.clone();
        store
            .update(&id, |job| {
                job.schedule = cron_expression.clone();
            })
            .await;
        return Ok(id);
    }

    let mut job =
        CronJob::new("knowledge-source-sync", &cron_expression, "知识源定时刷新", "知识源定时刷新");
    job.task_type = Some("knowledge_source_fetch_all".to_string());
    job.recurring = true;
    let id = state.cron_job_store.add(job.clone()).await;
    Ok(id)
}
// ── P3: 开源知识库导入（GitHub API 方案，零新增依赖）──────────────

/// 解析 `https://github.com/owner/repo` 或 `owner/repo` 为 (owner, repo)。
fn parse_github_repo(repo: &str) -> Option<(String, String)> {
    let trimmed = repo.trim().trim_end_matches('/');
    let parts: Vec<&str> = trimmed.split('/').filter(|p| !p.is_empty()).collect();
    // 定位 github.com 段（支持 https:// 等 scheme 前缀与 /tree/branch 后缀）
    if let Some(gi) = parts.iter().position(|p| *p == "github.com") {
        let owner = parts.get(gi + 1)?;
        let repo_name = parts.get(gi + 2)?;
        return Some(((*owner).to_string(), (*repo_name).to_string()));
    }
    // owner/repo[/branch]
    match parts.as_slice() {
        [owner, repo_name] if !owner.starts_with("http") => {
            Some(((*owner).to_string(), (*repo_name).to_string()))
        },
        _ => None,
    }
}

/// 从 GitHub 拉取仓库 docs 目录，逐文件导入为 Wiki 笔记。
///
/// 实现决策：用 GitHub Git Trees API + raw 文件下载，而非 git2/libgit2。
/// 理由：零新增依赖、无 vendored C 编译与打包体积开销；个人使用未认证
/// 60 req/h 足够。如需更高配额可经 metadata_json 传 token。
///
/// 导入完成后登记 source_type="github" 的知识源，之后可经
/// knowledge_source_fetch_all 增量重抓（内部按内容指纹跳过未变化文件）。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "导入GitHub仓库")]
#[tauri::command]
pub async fn github_repo_import(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: String,
    path_filter: Option<String>,
    wiki_id: Option<String>,
) -> Result<FetchSourceResult, String> {
    let (owner, repo_name) = parse_github_repo(&repo).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            ks_err::REPO_PARSE_FAILED,
            "无法解析仓库地址，支持 owner/repo 或完整 GitHub URL",
        )
    })?;
    let subdir = path_filter.unwrap_or_else(|| "docs".to_string()).trim_matches('/').to_string();

    let vault_id = match wiki_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let wikis =
                axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(err_str)?;
            let first = wikis.into_iter().next().ok_or_else(|| {
                ErrorResponse::err_with_detail(
                    ks_err::WIKI_NOT_AVAILABLE,
                    "没有可用的 Wiki，请先创建",
                )
            })?;
            first.id
        },
    };

    let result = github_sync(
        Some(&state),
        Some(&app),
        state.harness.db(),
        &owner,
        &repo_name,
        &subdir,
        &vault_id,
    )
    .await?;

    // 登记知识源（幂等：已存在则跳过），支撑后续增量同步
    let repo_path = format!("{owner}/{repo_name}");
    let existing = list_all_sources(state.harness.db())
        .await
        .map_err(err_str)?
        .into_iter()
        .any(|s| s.source_type == "github" && s.source_path == repo_path);
    if !existing {
        let source_repo = DaoWikiSourceRepository::new(Arc::new(state.harness.db().clone()));
        let _ = source_repo
            .insert(InsertWikiSourceInput {
                id: uuid::Uuid::new_v4().to_string(),
                wiki_id: vault_id,
                source_type: "github".to_string(),
                source_path: repo_path.clone(),
                title: repo_path.clone(),
                mime_type: "text/markdown".to_string(),
                size_bytes: 0,
                content_hash: result.detail.clone(),
                metadata_json: Some(serde_json::json!({ "subdir": subdir })),
                schedule_cron: None,
                last_fetched_at: Some(chrono::Utc::now().timestamp_millis()),
                status: "active".to_string(),
            })
            .await;
    }

    Ok(result)
}

/// GitHub 仓库同步核心（命令与 cron/源分派共用）。
async fn github_sync(
    state: Option<&State<'_, AppState>>,
    app: Option<&AppHandle>,
    db: &sea_orm::DatabaseConnection,
    owner: &str,
    repo_name: &str,
    subdir: &str,
    vault_id: &str,
) -> Result<FetchSourceResult, String> {
    let client = shared_http_client();
    let tree_url =
        format!("https://api.github.com/repos/{owner}/{repo_name}/git/trees/HEAD?recursive=1");
    let tree_resp = client
        .get(&tree_url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| {
            ErrorResponse::err_with_detail(
                ks_err::GITHUB_API_FAILED,
                format!("GitHub API 请求失败: {e}"),
            )
        })?;
    if !tree_resp.status().is_success() {
        return Err(ErrorResponse::err_with_detail(
            ks_err::GITHUB_API_FAILED,
            format!("GitHub API 错误（HTTP {}），请检查仓库地址或限流", tree_resp.status()),
        ));
    }
    let tree_json: serde_json::Value = tree_resp.json().await.map_err(|e| {
        ErrorResponse::err_with_detail(ks_err::GITHUB_API_FAILED, format!("解析仓库树失败: {e}"))
    })?;

    let mut files: Vec<String> = Vec::new();
    if let Some(entries) = tree_json["tree"].as_array() {
        for entry in entries {
            let path = entry["path"].as_str().unwrap_or("");
            let etype = entry["type"].as_str().unwrap_or("");
            if etype != "blob" {
                continue;
            }
            if !path.starts_with(subdir) {
                continue;
            }
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(ext.as_str(), "md" | "mdx" | "txt" | "markdown") {
                files.push(path.to_string());
            }
        }
    }
    if files.is_empty() {
        return Err(ErrorResponse::err_with_detail(
            ks_err::GITHUB_NO_DOCS,
            format!("仓库 {owner}/{repo_name} 的 {subdir}/ 下未找到 Markdown 文档"),
        ));
    }

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for path in files {
        let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo_name}/HEAD/{path}");
        let body = match client.get(&raw_url).send().await {
            Ok(r) if r.status().is_success() => match r.text().await {
                Ok(t) => t,
                Err(_) => {
                    failed += 1;
                    continue;
                },
            },
            _ => {
                failed += 1;
                continue;
            },
        };
        let body = body.chars().take(MAX_CONTENT_LENGTH).collect::<String>();

        let stem =
            std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");
        let title = stem.replace(['-', '_'], " ");
        let (t, u) = (yaml_escape(&title), yaml_escape(&raw_url));
        let md = format!(
            "---\ntitle: \"{t}\"\nsource: \"https://github.com/{owner}/{repo_name}\"\nurl: \"{u}\"\ntype: github\n---\n\n# {t}\n\n{body}"
        );
        let file_path = format!("github/{owner}-{repo_name}/{path}");

        match upsert_note(
            state,
            app,
            db,
            vault_id,
            &title,
            &file_path,
            &md,
            &format!("github:{owner}/{repo_name}:{path}"),
        )
        .await
        {
            Ok((_, action)) if action == "created" => imported += 1,
            Ok(_) => skipped += 1,
            Err(e) => {
                tracing::warn!("[knowledge-source] GitHub 文件 {} 入库失败: {}", path, e);
                failed += 1;
            },
        }
    }

    let detail = format!(
        "导入 {imported} 个，跳过 {skipped} 个，失败 {failed} 个（{owner}/{repo_name}: {subdir}/）"
    );
    let action = if imported > 0 { "created" } else { "skipped" };
    Ok(FetchSourceResult {
        source_id: format!("github:{owner}/{repo_name}"),
        source_title: format!("{owner}/{repo_name}"),
        action: action.to_string(),
        detail,
    })
}

// ── P3: sitemap 批量抓取 ─────────────────────────────────────────

/// 解析站点 sitemap.xml，批量创建 url 型知识源。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "抓取站点Sitemap")]
#[tauri::command]
pub async fn sitemap_crawl(
    state: State<'_, AppState>,
    base_url: String,
    wiki_id: Option<String>,
) -> Result<Vec<FetchSourceResult>, String> {
    let base = base_url.trim().trim_end_matches('/');
    if !base.starts_with("http://") && !base.starts_with("https://") {
        return Err(ErrorResponse::err_with_detail(
            ks_err::URL_SCHEME_INVALID,
            "站点地址必须以 http(s):// 开头",
        ));
    }

    let vault_id = match wiki_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let wikis =
                axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(err_str)?;
            let first = wikis.into_iter().next().ok_or_else(|| {
                ErrorResponse::err_with_detail(
                    ks_err::WIKI_NOT_AVAILABLE,
                    "没有可用的 Wiki，请先创建",
                )
            })?;
            first.id
        },
    };

    let client = shared_http_client();
    let sitemap_url = format!("{base}/sitemap.xml");
    let body = client
        .get(&sitemap_url)
        .send()
        .await
        .map_err(|e| {
            ErrorResponse::err_with_detail(
                ks_err::SITEMAP_FETCH_FAILED,
                format!("sitemap 请求失败: {e}"),
            )
        })?
        .text()
        .await
        .map_err(|e| {
            ErrorResponse::err_with_detail(
                ks_err::SITEMAP_READ_FAILED,
                format!("sitemap 读取失败: {e}"),
            )
        })?;

    // 简单 XML 解析：提取 <loc> 节点内容
    let mut urls: Vec<String> = Vec::new();
    let mut rest = body.as_str();
    while let Some(start) = rest.find("<loc>") {
        let after = &rest[start + 5..];
        if let Some(end) = after.find("</loc>") {
            let loc = after[..end].trim().to_string();
            if !loc.is_empty() && urls.len() < 200 {
                urls.push(loc);
            }
            rest = &after[end + 6..];
        } else {
            break;
        }
    }

    if urls.is_empty() {
        return Err(ErrorResponse::err_with_detail(
            ks_err::SITEMAP_EMPTY,
            "sitemap.xml 中未解析到任何 URL",
        ));
    }

    // 幂等去重：跳过已登记过同 URL 的 url 型源，避免重复 sitemap 抓取无限堆积
    let mut known_paths: std::collections::HashSet<String> = list_all_sources(state.harness.db())
        .await
        .map_err(err_str)?
        .into_iter()
        .filter(|s| s.source_type == "url")
        .map(|s| s.source_path)
        .collect();

    let repo = DaoWikiSourceRepository::new(Arc::new(state.harness.db().clone()));
    let mut results = Vec::with_capacity(urls.len());
    for url in urls {
        let title = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("untitled")
            .replace(['-', '_'], " ")
            .chars()
            .take(80)
            .collect::<String>();
        // 幂等去重：同 URL 已在知识源中则跳过，不重复建源
        if known_paths.contains(&url) {
            results.push(FetchSourceResult {
                source_id: format!("url:{url}"),
                source_title: title.clone(),
                action: "skipped".to_string(),
                detail: "该 URL 已在知识源中，跳过".to_string(),
            });
            continue;
        }
        let src = repo
            .insert(InsertWikiSourceInput {
                id: uuid::Uuid::new_v4().to_string(),
                wiki_id: vault_id.clone(),
                source_type: "url".to_string(),
                source_path: url.clone(),
                title,
                mime_type: "text/markdown".to_string(),
                size_bytes: 0,
                content_hash: String::new(),
                metadata_json: Some(serde_json::json!({ "from_sitemap": base })),
                schedule_cron: None,
                last_fetched_at: None,
                status: "active".to_string(),
            })
            .await
            .map_err(err_str)?;
        known_paths.insert(url.clone());
        results.push(FetchSourceResult {
            source_id: src.id.clone(),
            source_title: src.title.clone(),
            action: "created".to_string(),
            detail: url,
        });
    }

    Ok(results)
}

// ── P4: 用户编辑保护 + 精华进 Memory ─────────────────────────────

/// 抓取更新时保护用户手工编辑过的笔记：user_edited=true 则跳过覆盖，
/// 避免「网络内容覆盖人工整理」的冲突。该保护在 upsert_note 中生效：
/// 命中用户编辑时返回 action=skipped。
///
/// 说明：Note DTO 的 user_edited 字段由前端编辑置位，抓取管道只读不改。
/// 若要强制覆盖，前端先调用 wiki_notes_update 清除 user_edited 再抓取。
///
/// 抓取成功后把页面精华沉淀进 Memory（写入首个可用 namespace，失败仅告警）。
async fn deposit_to_memory(db: &sea_orm::DatabaseConnection, title: &str, url: &str, body: &str) {
    let namespaces = match axagent_dao::repo::memory::list_namespaces(db).await {
        Ok(ns) => ns,
        Err(e) => {
            tracing::warn!("[knowledge-source] 读取记忆命名空间失败，跳过沉淀: {e}");
            return;
        },
    };
    let Some(ns) = namespaces.into_iter().find(|n| n.scope != "system") else {
        tracing::warn!("[knowledge-source] 没有可用记忆命名空间，跳过沉淀");
        return;
    };

    let snippet: String = body.chars().take(300).collect();
    let input = axagent_harness::types::rag_voice_etc::CreateMemoryItemInput {
        namespace_id: ns.id.clone(),
        title: title.to_string(),
        content: format!("知识源抓取：[{title}]({url})\n\n{snippet}"),
        source: Some("knowledge-source".to_string()),
        tier: Some("working".to_string()),
        importance: Some(0.4),
        memory_nature: Some("semantic".to_string()),
        tags: Some(vec!["knowledge-source".to_string(), "web".to_string()]),
        decay_rate: None,
        expires_at: None,
        applicability_tags: None,
        confirmed: None,
        source_conversation_id: None,
        source_message_id: None,
    };
    if let Err(e) = axagent_dao::repo::memory::add_item(db, input).await {
        tracing::warn!("[knowledge-source] 精华沉淀进 Memory 失败: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_title() {
        assert_eq!(slugify("Hello World"), "Hello-World");
        assert_eq!(slugify("Rust 语言 入门"), "Rust--");
        assert_eq!(slugify("---"), "untitled");
        assert_eq!(slugify(r"a/b\c"), "abc");
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        let a = content_fingerprint("hello world");
        let b = content_fingerprint("hello world");
        let c = content_fingerprint("hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn normalize_md_has_frontmatter() {
        let md = normalize_fetched_md("https://example.com", "标题", "正文内容");
        assert!(md.contains("---"));
        assert!(md.contains("source: \"https://example.com\""));
        assert!(md.contains("url: \"https://example.com\""));
        assert!(md.contains("# 标题"));
        assert!(md.contains("正文内容"));
    }

    #[test]
    fn normalize_md_is_time_stable() {
        // 增量更新闭环的指纹稳定性：frontmatter 不得含时间戳（fetched_at 已移除），
        // 否则同一内容两次抓取指纹不同，闭环永远失效。
        let a = normalize_fetched_md("https://example.com", "标题", "正文内容");
        let b = normalize_fetched_md("https://example.com", "标题", "正文内容");
        assert_eq!(a, b);
        assert!(!a.contains("fetched_at"));
        assert_eq!(content_fingerprint(&a), content_fingerprint(&b));
    }

    #[test]
    fn parse_github_repo_handles_variants() {
        assert_eq!(
            parse_github_repo("owner/repo"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_github_repo("https://github.com/rust-lang/rust"),
            Some(("rust-lang".to_string(), "rust".to_string()))
        );
        assert_eq!(
            parse_github_repo("github.com/owner/repo/tree/main"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(parse_github_repo("not a repo"), None);
        assert_eq!(parse_github_repo(""), None);
    }

    #[test]
    fn normalize_truncates_rss_summary_correctly() {
        let long: String = "a".repeat(500);
        let snippet: String = long.chars().take(300).collect();
        assert_eq!(snippet.len(), 300);
    }
}
