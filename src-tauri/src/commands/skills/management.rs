use super::install::{
    MARKETPLACE_SEARCH_CACHE, MarketplaceSearchCache, home_dir, load_plugin_version, skills_dir,
    validate_skill_name,
};
use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::skill as skill_err;
use agent_macro::agent_command;
use axagent_harness::types::*;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;

#[agent_command(domain = skills, safety = Caution, call_mode = StateOnly, description = "创建并打开技能目录")]
#[tauri::command]
pub async fn open_skills_dir() -> Result<(), String> {
    let dir = skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    open::that(&dir).map_err(|e| format!("Failed to open directory: {}", e))
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateInput, description = "打开指定技能所在目录")]
#[tauri::command]
pub async fn open_skill_dir(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let dir = if p.is_dir() {
        p.to_path_buf()
    } else {
        p.parent().map(|d| d.to_path_buf()).unwrap_or_else(|| p.to_path_buf())
    };
    if dir.exists() {
        open::that(&dir).map_err(|e| format!("Failed to open directory: {}", e))
    } else {
        Err(format!("Directory does not exist: {}", dir.display()))
    }
}

/// Collect `source_ref` values from `skill-manifest.json` files across all
/// three global skill directories so marketplace results can be marked as
/// installed regardless of the directory name.
fn installed_source_refs() -> std::collections::HashSet<String> {
    let home = home_dir();
    let dirs = [
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];

    let mut refs = std::collections::HashSet::new();
    for dir in &dirs {
        collect_source_refs(dir, &mut refs, /* depth */ 0);
    }
    refs
}

fn collect_source_refs(dir: &Path, refs: &mut std::collections::HashSet<String>, depth: u32) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest = path.join("skill-manifest.json");
        if manifest.exists() {
            if let Some(sr) = read_source_ref(&manifest) {
                refs.insert(sr);
            }
        }
        // Recurse one level for group containers (dirs without SKILL.md but
        // with subdirs that have skill-manifest.json).
        if depth == 0 {
            collect_source_refs(&path, refs, depth + 1);
        }
    }
}

fn read_source_ref(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let val: serde_json::Value = serde_json::from_str(&text).ok()?;
    let sr = val["source_ref"].as_str()?;
    let normalized = sr.trim().trim_end_matches('/').to_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

struct InstalledSkillInfo {
    pub commit: String,
    pub version: String,
    pub source_ref: String,
}

fn get_installed_skill_info(repo: &str) -> Option<InstalledSkillInfo> {
    let skills_path = skills_dir();
    let skill_target = skills_path.join(repo);
    let manifest_path = skill_target.join("skill-manifest.json");

    if !manifest_path.exists() {
        return None;
    }

    let manifest_str = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).ok()?;

    let source_kind = manifest["source_kind"].as_str().unwrap_or("");
    if source_kind != "github" {
        return None;
    }

    let commit = manifest["commit"].as_str().unwrap_or("").to_string();
    let source_ref = manifest["source_ref"].as_str().unwrap_or("").to_string();

    if source_ref.is_empty() || commit.is_empty() {
        return None;
    }

    let version = load_plugin_version(&skill_target);

    Some(InstalledSkillInfo { commit, version, source_ref })
}

async fn check_github_update(
    owner: &str,
    repo: &str,
    current_commit: &str,
) -> Option<(String, String)> {
    let url = format!("https://api.github.com/repos/{}/{}/commits?per_page=1", owner, repo);

    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().ok()?;
    let response = client
        .get(&url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    let commits = body.as_array()?;
    let latest = commits.first()?;
    let latest_sha = latest["sha"].as_str()?;

    if latest_sha.starts_with(current_commit)
        || current_commit == &latest_sha[..7.min(latest_sha.len())]
    {
        return None;
    }

    Some((latest_sha[..7.min(latest_sha.len())].to_string(), latest_sha.to_string()))
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateInput, description = "搜索技能市场")]
#[tauri::command]
pub async fn search_marketplace(
    query: String,
    source: Option<String>,
    sort: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let installed_refs = installed_source_refs();
    let sort_order = sort.as_deref().unwrap_or("popular");
    let source_str = source.as_deref().unwrap_or("skillhub");
    let page_num = page.unwrap_or(1).max(1);
    let per_page_num = per_page.unwrap_or(20).min(100);

    let cache_key = MarketplaceSearchCache::make_key(&query, source_str, sort_order, page_num);
    let cache_result = {
        let cache = MARKETPLACE_SEARCH_CACHE.lock().await;
        cache.get(&cache_key)
    };
    if let Some(cached_results) = cache_result {
        return Ok(cached_results);
    }

    let results = match source_str {
        "github" => {
            search_github_marketplace(&query, sort_order, page_num, per_page_num, &installed_refs)
                .await?
        },
        _ => {
            search_skillhub_marketplace(&query, sort_order, page_num, per_page_num, &installed_refs)
                .await?
        },
    };

    {
        let mut cache = MARKETPLACE_SEARCH_CACHE.lock().await;
        cache.set(cache_key, results.clone());
    }

    Ok(results)
}

async fn search_github_marketplace(
    query: &str,
    sort_order: &str,
    page: u32,
    per_page: u32,
    installed_refs: &std::collections::HashSet<String>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let gh_sort = match sort_order {
        "latest" => "updated",
        "stars" => "stars",
        _ => "stars",
    };
    let url = format!(
        "https://api.github.com/search/repositories?q={}+topic:agent-skill&sort={}&per_page={}&page={}",
        urlencoding::encode(query),
        gh_sort,
        per_page,
        page
    );

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let response = client
        .get(&url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let items = body["items"].as_array().cloned().unwrap_or_default();

    let mut results: Vec<MarketplaceSkill> = Vec::new();
    for item in items {
        let skill_name = item["name"].as_str().unwrap_or("").to_string();
        let repo = item["full_name"].as_str().unwrap_or("").to_string();
        let repo_lower = repo.trim().trim_end_matches('/').to_lowercase();
        let installed = installed_refs.contains(&repo_lower);

        let mut skill = MarketplaceSkill {
            name: skill_name,
            description: item["description"].as_str().unwrap_or("").to_string(),
            repo: repo.clone(),
            stars: item["stargazers_count"].as_i64().unwrap_or(0),
            installs: 0,
            installed,
            ..Default::default()
        };

        if installed {
            if let Some(info) = get_installed_skill_info(&repo) {
                skill.current_version = Some(info.version);
                let parts: Vec<&str> = info.source_ref.split('/').collect();
                if parts.len() == 2 {
                    if let Some((latest_short, _)) =
                        check_github_update(parts[0], parts[1], &info.commit).await
                    {
                        skill.has_update = Some(true);
                        skill.latest_version = Some(latest_short);
                    }
                }
            }
        }

        results.push(skill);
    }

    Ok(results)
}

async fn search_skillhub_marketplace(
    query: &str,
    sort_order: &str,
    page: u32,
    per_page: u32,
    installed_refs: &std::collections::HashSet<String>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let (sort_param, _) = match sort_order {
        "latest" => ("recent", 20),
        "stars" => ("stars", 20),
        _ => ("downloads", 20),
    };
    let search_query = if query.is_empty() {
        "claude".to_string()
    } else {
        query.to_string()
    };
    let offset = (page - 1) * per_page;
    let url = format!(
        "https://skillshub.wtf/api/v1/skills/search?q={}&sort={}&limit={}&offset={}",
        urlencoding::encode(&search_query),
        sort_param,
        per_page,
        offset
    );

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let response = client
        .get(&url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("skillhub API error: {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let items = body["data"].as_array().cloned().unwrap_or_default();

    let mut results: Vec<MarketplaceSkill> = Vec::new();
    for item in items {
        let name = item["name"].as_str().unwrap_or("").to_string();
        let slug = item["slug"].as_str().unwrap_or("").to_string();
        let description = item["description"].as_str().unwrap_or("").to_string();
        let repo_obj = item.get("repo").ok_or("missing repo object")?;
        let github_owner =
            repo_obj.get("githubOwner").and_then(|v| v.as_str()).ok_or("missing githubOwner")?;
        let github_repo_name = repo_obj
            .get("githubRepoName")
            .and_then(|v| v.as_str())
            .ok_or("missing githubRepoName")?;
        let repo = format!("{}/{}", github_owner, github_repo_name);
        let installed = installed_refs.contains(&repo.to_lowercase());
        let stars = item["stars"].as_i64().unwrap_or(0);
        let installs = item["downloads"].as_i64().unwrap_or(0);

        let categories = item
            .get("categories")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        let tags = item
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        let mut skill = MarketplaceSkill {
            name: if !name.is_empty() { name } else { slug },
            description: description.to_string(),
            repo: repo.clone(),
            stars,
            installs,
            installed,
            categories,
            tags,
            ..Default::default()
        };

        if installed {
            if let Some(info) = get_installed_skill_info(&repo) {
                skill.current_version = Some(info.version);
                let parts: Vec<&str> = info.source_ref.split('/').collect();
                if parts.len() == 2 {
                    if let Some((latest_short, _)) =
                        check_github_update(parts[0], parts[1], &info.commit).await
                    {
                        skill.has_update = Some(true);
                        skill.latest_version = Some(latest_short);
                    }
                }
            }
        }

        results.push(skill);
    }

    Ok(results)
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "获取技能市场分类列表")]
#[tauri::command]
pub async fn get_marketplace_categories() -> Result<Vec<MarketplaceCategory>, String> {
    let url = "https://skillshub.wtf/api/v1/categories";

    let client =
        reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let response = client
        .get(url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to get categories: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("skillhub API error: {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let items = body["data"].as_array().cloned().unwrap_or_default();

    let categories: Vec<MarketplaceCategory> = items
        .iter()
        .filter_map(|item| {
            Some(MarketplaceCategory {
                id: item["slug"].as_str()?.to_string(),
                name: item["name"].as_str()?.to_string(),
                description: item["description"].as_str().unwrap_or("").to_string(),
                skill_count: item["skillCount"].as_i64().unwrap_or(0),
            })
        })
        .collect();

    Ok(categories)
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "检查已安装技能更新")]
#[tauri::command]
pub async fn check_skill_updates() -> Result<Vec<SkillUpdateInfo>, String> {
    let skills_path = skills_dir();
    let mut updates = Vec::new();

    let entries = match std::fs::read_dir(&skills_path) {
        Ok(e) => e,
        Err(_) => return Ok(updates),
    };

    for entry in entries.flatten() {
        let manifest_path = entry.path().join("skill-manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        let manifest_str = match std::fs::read_to_string(&manifest_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let manifest: serde_json::Value = match serde_json::from_str(&manifest_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if manifest["source_kind"].as_str() != Some("github") {
            continue;
        }

        let source_ref = manifest["source_ref"].as_str().unwrap_or("").to_string();
        let current_commit = manifest["commit"].as_str().unwrap_or("").to_string();

        if source_ref.is_empty() || current_commit.is_empty() {
            continue;
        }

        let parts: Vec<&str> = source_ref.split('/').collect();
        if parts.len() != 2 {
            continue;
        }

        let url =
            format!("https://api.github.com/repos/{}/{}/commits?per_page=1", parts[0], parts[1]);

        let client =
            reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        let response = client
            .get(&url)
            .header("User-Agent", "AxAgent")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(commits) = body.as_array() {
                        if let Some(latest) = commits.first() {
                            let latest_sha = latest["sha"].as_str().unwrap_or("").to_string();
                            let short_latest = &latest_sha[..7.min(latest_sha.len())];
                            if !current_commit.is_empty()
                                && !latest_sha.starts_with(&current_commit)
                                && current_commit != short_latest
                            {
                                updates.push(SkillUpdateInfo {
                                    name: entry.file_name().to_string_lossy().to_string(),
                                    current_commit: current_commit.clone(),
                                    latest_commit: short_latest.to_string(),
                                    source_ref: source_ref.clone(),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(updates)
}

// ---------------------------------------------------------------------------
// P1: Self-evolution skill create/patch/edit commands
// ---------------------------------------------------------------------------

/// P3 #12: 提取公共前置逻辑 — 验证 skill_name、定位 SKILL.md、安全检查、读取内容。
/// Returns (canonical_path, content) 供调用方继续执行特有操作。
pub(crate) fn validate_and_read_skill_md(name: &str) -> Result<(PathBuf, String), String> {
    validate_skill_name(name)?;
    let path = skills_dir().join(name).join("SKILL.md");
    if !path.exists() {
        return Err(format!("Skill '{}' not found", name));
    }
    let canonical_dir = skills_dir().join(name).canonicalize().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let canonical_path = path.canonicalize().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err("Path traversal detected".to_string());
    }
    let content = std::fs::read_to_string(&canonical_path).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok((canonical_path, content))
}

/// Patch an existing skill by appending a note
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "为技能追加补丁内容")]
#[tauri::command]
pub async fn skill_patch(name: String, content: String) -> Result<String, ErrorResponse> {
    let (canonical_path, existing) = validate_and_read_skill_md(&name)?;
    let patched = format!(
        "{}\n\n## Patch ({})\n\n{}",
        existing,
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        content
    );

    std::fs::write(&canonical_path, &patched).map_err(|e| {
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        )
    })?;
    Ok(format!("Skill '{}' patched", name))
}

/// Edit an existing skill by replacing the body (preserving frontmatter)
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "编辑技能内容")]
#[tauri::command]
pub async fn skill_edit(name: String, content: String) -> Result<String, ErrorResponse> {
    let (canonical_path, existing) = validate_and_read_skill_md(&name)?;

    // Preserve YAML frontmatter
    let edited = if let Some(fm_end) = find_frontmatter_end(&existing) {
        format!("{}\n\n{}", &existing[..fm_end], content)
    } else {
        content
    };

    std::fs::write(&canonical_path, &edited).map_err(|e| {
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        )
    })?;
    Ok(format!("Skill '{}' edited", name))
}

/// Find the end position of YAML frontmatter (after the second `---` marker).
/// Uses byte-level search to correctly handle \r\n line endings on Windows.
pub(crate) fn find_frontmatter_end(content: &str) -> Option<usize> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    // 找到第二个 `---` 标记（跳过开头的 `---`）
    let after_first = &trimmed[3..];
    let second = after_first.find("\n---")?;
    // 返回相对于原始 content 的偏移量
    let offset = content.len() - trimmed.len();
    Some(offset + 3 + second + 4) // +3(first ---) + second(pos) + 4(len of "\n---")
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "获取技能提案列表")]
#[tauri::command]
pub async fn get_skill_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::SkillProposal>, String> {
    let service = state.skill_proposal_service.read().await;
    Ok(service.get_proposals())
}

#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "根据提案创建技能")]
#[tauri::command]
pub async fn create_skill_from_proposal(
    state: State<'_, AppState>,
    name: String,
    description: String,
    content: String,
) -> Result<String, String> {
    let result =
        skill_create(state.clone(), name.clone(), description.clone(), content, Some(false))
            .await?;
    if result.can_create {
        let mut service = state.skill_proposal_service.write().await;
        service.clear_proposal(&name);
        Ok(result.message)
    } else {
        Err(result.message)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimilarSkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub scenarios: Vec<String>,
    pub success_rate: f64,
    pub similarity_score: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillCreateCheckResult {
    pub has_similar: bool,
    pub similar_skills: Vec<SimilarSkillInfo>,
    pub can_create: bool,
    pub message: String,
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateInput, description = "检查相似技能")]
#[tauri::command]
pub async fn skill_check_similar(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<SkillCreateCheckResult, String> {
    let closed_loop = state.closed_loop_service.clone();

    let check_topic = if let Some(ref desc) = description {
        if !desc.is_empty() {
            desc.clone()
        } else {
            name.clone()
        }
    } else {
        name.clone()
    };

    let similar = closed_loop.find_similar_skills(&check_topic).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if similar.is_empty() {
        return Ok(SkillCreateCheckResult {
            has_similar: false,
            similar_skills: vec![],
            can_create: true,
            message: format!("No similar skills found. You can create '{}'.", name),
        });
    }

    let similar_infos: Vec<SimilarSkillInfo> = similar
        .into_iter()
        .map(|s| SimilarSkillInfo {
            id: s.id,
            name: s.name,
            description: s.description,
            version: s.version,
            scenarios: s.scenarios,
            success_rate: s.success_rate,
            similarity_score: 0.7,
        })
        .collect();

    Ok(SkillCreateCheckResult {
        has_similar: true,
        similar_skills: similar_infos.clone(),
        can_create: false,
        message: format!(
            "Found {} similar skill(s). Consider upgrading an existing skill instead of creating a new one.",
            similar_infos.len()
        ),
    })
}

#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "创建新技能")]
#[tauri::command]
pub async fn skill_create(
    state: State<'_, AppState>,
    name: String,
    description: String,
    content: String,
    check_similar: Option<bool>,
) -> Result<SkillCreateCheckResult, String> {
    let check = check_similar.unwrap_or(true);

    validate_skill_name(&name)?;

    if check {
        let check_result =
            skill_check_similar(state.clone(), name.clone(), Some(description.clone())).await?;
        if check_result.has_similar {
            return Ok(check_result);
        }
    }

    let dir = skills_dir().join(&name);
    if dir.exists() {
        return Ok(SkillCreateCheckResult {
            has_similar: false,
            similar_skills: vec![],
            can_create: false,
            message: format!("Skill '{}' already exists at {}", name, dir.display()),
        });
    }

    std::fs::create_dir_all(&dir).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let desc = if description.is_empty() {
        name.clone()
    } else {
        description
    };
    let escaped_name = escape_yaml_value(&name);
    let escaped_desc = escape_yaml_value(&desc);
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\nversion: 1.0.0\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
        escaped_name, escaped_desc, content
    );

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(SkillCreateCheckResult {
        has_similar: false,
        similar_skills: vec![],
        can_create: true,
        message: format!("Skill '{}' created at {}", name, dir.display()),
    })
}

fn escape_yaml_value(value: &str) -> String {
    if value.contains(':')
        || value.contains('#')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('\n')
        || value.contains('{')
        || value.contains('}')
        || value.contains('[')
        || value.contains(']')
        || value.contains('&')
        || value.contains('*')
        || value.contains('!')
        || value.contains('|')
        || value.contains('>')
        || value.contains('%')
        || value.contains('@')
        || value.contains('`')
        || value.contains(',')
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "升级或创建技能")]
#[tauri::command]
pub async fn skill_upgrade_or_create(
    state: State<'_, AppState>,
    name: String,
    description: String,
    content: String,
    target_skill_id: Option<String>,
    improvements: Option<String>,
    additional_scenarios: Option<Vec<String>>,
) -> Result<String, String> {
    validate_skill_name(&name)?;
    if let Some(skill_id) = target_skill_id {
        let closed_loop = state.closed_loop_service.clone();
        let upgrade_proposal = axagent_trajectory::SkillUpgradeProposal {
            target_skill_id: skill_id,
            suggested_improvements: improvements.unwrap_or(content),
            additional_scenarios: additional_scenarios.unwrap_or_default(),
            confidence: 1.0,
            trigger_event: "manual_upgrade_or_create".to_string(),
        };

        let auto_action = axagent_trajectory::AutoAction {
            action_type: "upgrade_skill".to_string(),
            target: serde_json::to_string(&upgrade_proposal).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?,
        };

        closed_loop.execute_upgrade_action(&auto_action).await;
        return Ok(format!("Skill '{}' upgraded successfully", name));
    }

    let dir = skills_dir().join(&name);
    if dir.exists() {
        return Err(format!("Skill '{}' already exists", name));
    }

    std::fs::create_dir_all(&dir).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let desc = if description.is_empty() {
        name.clone()
    } else {
        description
    };
    let escaped_name = escape_yaml_value(&name);
    let escaped_desc = escape_yaml_value(&desc);
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\nversion: 1.0.0\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
        escaped_name, escaped_desc, content
    );

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(format!("Skill '{}' created at {}", name, dir.display()))
}

/// 设置技能的 manifest 配置。写入或替换 skill-manifest.json。
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "设置技能清单配置")]
#[tauri::command]
pub async fn skill_set_manifest(
    name: String,
    manifest: serde_json::Value,
) -> Result<String, String> {
    validate_skill_name(&name)?;
    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name));
    }

    let manifest_path = skill_dir.join("skill-manifest.json");
    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| {
        ErrorResponse::new(skill_err::SERIALIZE_FAILED).with_detail(format!("JSON 序列化失败: {e}"))
    })?;
    std::fs::write(&manifest_path, manifest_str).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(format!("清单已保存: '{}'", name))
}

// ── 技能学习闭环:审批门命令 ──────────────────────────────────────

/// 获取所有待审批的技能操作
#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "获取待审批技能操作列表")]
#[tauri::command]
pub async fn get_pending_skill_operations(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::PendingSkillOperation>, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    Ok(manager.get_pending_operations().await)
}

/// 获取所有技能操作记录（包括已批准/已拒绝）
#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "获取所有技能操作记录")]
#[tauri::command]
pub async fn get_all_skill_operations(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::PendingSkillOperation>, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    Ok(manager.get_all_operations().await)
}

/// 批准技能操作
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "批准技能操作")]
#[tauri::command]
pub async fn approve_skill_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<String, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    manager.approve_operation(&operation_id).await?;
    Ok(format!("Operation '{}' approved", operation_id))
}

/// 拒绝技能操作
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "拒绝技能操作")]
#[tauri::command]
pub async fn reject_skill_operation(
    state: State<'_, AppState>,
    operation_id: String,
    reason: String,
) -> Result<String, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    manager.reject_operation(&operation_id, &reason).await?;
    Ok(format!("Operation '{}' rejected: {}", operation_id, reason))
}

/// 提交技能操作审批（供 AI 调用）
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "提交技能操作审批")]
#[tauri::command]
pub async fn submit_skill_operation(
    state: State<'_, AppState>,
    operation_type: String,
    skill_id: Option<String>,
    skill_name: Option<String>,
    content: String,
    reason: String,
    file_path: Option<String>,
) -> Result<axagent_trajectory::PendingSkillOperation, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    let op_type = match operation_type.as_str() {
        "create_skill" => axagent_trajectory::PendingOperationType::CreateSkill,
        "patch_skill" => axagent_trajectory::PendingOperationType::PatchSkill,
        "edit_skill" => axagent_trajectory::PendingOperationType::EditSkill,
        "delete_skill" => axagent_trajectory::PendingOperationType::DeleteSkill,
        "write_file" => axagent_trajectory::PendingOperationType::WriteFile,
        "remove_file" => axagent_trajectory::PendingOperationType::RemoveFile,
        _ => return Err(format!("Unknown operation type: {}", operation_type)),
    };

    manager
        .submit_for_approval(op_type, skill_id, skill_name, None, content, reason, file_path)
        .await
}

/// 获取技能学习配置
#[agent_command(domain = skills, safety = Safe, call_mode = StateOnly, description = "获取技能学习配置")]
#[tauri::command]
pub async fn get_skill_learning_config(
    state: State<'_, AppState>,
) -> Result<axagent_trajectory::SkillLearningConfig, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    Ok(manager.get_config().await)
}

/// 更新技能学习配置
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "更新技能学习配置")]
#[tauri::command]
pub async fn update_skill_learning_config(
    state: State<'_, AppState>,
    config: axagent_trajectory::SkillLearningConfig,
) -> Result<String, String> {
    let manager = state.skill.skill_learning_manager.read().await;
    manager.update_config(config).await;
    Ok("Skill learning config updated".to_string())
}

// ── /learn 技能生成命令 ──────────────────────────────────────────

/// /learn 命令的输入参数
#[derive(Debug, serde::Deserialize)]
pub struct LearnSkillInput {
    /// 技能名称（可选，自动生成如果未提供）
    pub name: Option<String>,
    /// 技能描述（可选）
    pub description: Option<String>,
    /// 学习来源类型
    pub source_type: String,
    /// 学习内容（文档文本、对话历史或代码片段）
    pub content: String,
    /// 额外的上下文信息
    pub context: Option<String>,
    /// 是否自动提交审批（默认 true）
    pub auto_approve: Option<bool>,
}

/// /learn 命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct LearnSkillResult {
    /// 生成的技能名称
    pub skill_name: String,
    /// 技能文件路径
    pub skill_path: String,
    /// 生成的 SKILL.md 内容
    pub skill_content: String,
    /// 提取的参考资料
    pub references: Vec<String>,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 生成的步骤/过程
    pub steps_taken: Vec<String>,
    /// 是否需要审批
    pub requires_approval: bool,
    /// 操作ID（如果提交审批）
    pub operation_id: Option<String>,
}

/// 从内容中提取关键信息并生成 SKILL.md
fn generate_skill_from_content(
    name: &str,
    description: Option<&str>,
    content: &str,
    source_type: &str,
    context: Option<&str>,
) -> (String, Vec<String>, f64) {
    let mut skill_content = String::new();
    let mut references = Vec::new();
    let mut confidence: f64 = 0.5;

    // 提取标题作为技能名或使用提供的名称
    let skill_name = if name.is_empty() {
        extract_title(content).unwrap_or_else(|| "auto-generated-skill".to_string())
    } else {
        name.to_string()
    };

    // 生成 YAML frontmatter
    let escaped_name = escape_yaml_value(&skill_name);
    let escaped_desc = escape_yaml_value(description.unwrap_or("Auto-generated skill from /learn"));

    skill_content.push_str("---\n");
    skill_content.push_str(&format!("name: {}\n", escaped_name));
    skill_content.push_str(&format!("description: {}\n", escaped_desc));
    skill_content.push_str("version: 1.0.0\n");
    skill_content.push_str(&format!("source_type: {}\n", escape_yaml_value(source_type)));
    skill_content.push_str("metadata:\n");
    skill_content.push_str("  hermes:\n");
    skill_content.push_str("    tags: [auto-learned]\n");
    skill_content.push_str("    related_skills: []\n");
    skill_content.push_str("    auto_generated: true\n");
    skill_content.push_str("    learn_source: true\n");
    skill_content.push_str("---\n\n");

    // 生成技能正文
    skill_content.push_str(&format!("# {}\n\n", skill_name));

    if let Some(desc) = description {
        skill_content.push_str(&format!("{}\n\n", desc));
    }

    // 根据来源类型生成不同的结构
    match source_type {
        "document" => {
            // 文档来源：提取章节结构
            skill_content.push_str("## Overview\n\n");
            skill_content.push_str("This skill was auto-generated from document content.\n\n");

            // 提取关键段落
            let key_sections = extract_key_sections(content);
            if !key_sections.is_empty() {
                skill_content.push_str("## Key Concepts\n\n");
                for section in &key_sections {
                    skill_content.push_str(&format!("- {}\n", section));
                }
                skill_content.push('\n');
                confidence = 0.7;
            }

            // 提取步骤或过程
            let steps = extract_numbered_steps(content);
            if !steps.is_empty() {
                skill_content.push_str("## Procedure\n\n");
                for step in &steps {
                    skill_content.push_str(&format!("{}\n", step));
                }
                skill_content.push('\n');
                confidence = (confidence + 0.2).min(1.0);
            }
        },
        "conversation" => {
            // 对话来源：提取交互模式
            skill_content.push_str("## Overview\n\n");
            skill_content.push_str("This skill was auto-generated from conversation history.\n\n");

            // 提取对话模式
            let patterns = extract_conversation_patterns(content);
            if !patterns.is_empty() {
                skill_content.push_str("## Detected Patterns\n\n");
                for pattern in &patterns {
                    skill_content.push_str(&format!("- {}\n", pattern));
                }
                skill_content.push('\n');
                confidence = 0.6;
            }

            // 提取工具调用序列
            let tool_sequence = extract_tool_sequence(content);
            if !tool_sequence.is_empty() {
                skill_content.push_str("## Tool Sequence\n\n");
                for (i, tool) in tool_sequence.iter().enumerate() {
                    skill_content.push_str(&format!("{}. {}\n", i + 1, tool));
                }
                skill_content.push('\n');
                confidence = (confidence + 0.2).min(1.0);
            }
        },
        "codebase" => {
            // 代码库来源：提取代码模式
            skill_content.push_str("## Overview\n\n");
            skill_content.push_str("This skill was auto-generated from codebase analysis.\n\n");

            // 提取函数/方法模式
            let patterns = extract_code_patterns(content);
            if !patterns.is_empty() {
                skill_content.push_str("## Code Patterns\n\n");
                for pattern in &patterns {
                    skill_content.push_str(&format!("```\n{}\n```\n\n", pattern));
                }
                confidence = 0.65;
            }

            // 提取使用说明
            let usage = extract_usage_patterns(content);
            if !usage.is_empty() {
                skill_content.push_str("## Usage\n\n");
                for line in &usage {
                    skill_content.push_str(&format!("{}\n", line));
                }
                skill_content.push('\n');
                confidence = (confidence + 0.15).min(1.0);
            }
        },
        _ => {
            // 混合或未知来源
            skill_content.push_str("## Overview\n\n");
            skill_content.push_str("This skill was auto-generated using the /learn command.\n\n");

            // 提取通用结构
            let structure = extract_generic_structure(content);
            if !structure.is_empty() {
                skill_content.push_str("## Content Summary\n\n");
                for line in &structure {
                    skill_content.push_str(&format!("{}\n", line));
                }
                skill_content.push('\n');
                confidence = 0.55;
            }
        },
    }

    // 添加上下文信息
    if let Some(ctx) = context {
        if !ctx.is_empty() {
            skill_content.push_str("## Context\n\n");
            skill_content.push_str(&format!("{}\n\n", ctx));
        }
    }

    // 添加参考资料
    skill_content.push_str("## References\n\n");
    skill_content.push_str("- Auto-generated by /learn command\n");
    skill_content.push_str(&format!("- Source type: {}\n", source_type));
    if let Some(ctx) = context {
        if !ctx.is_empty() {
            references.push(ctx.to_string());
        }
    }

    // 添加常见陷阱
    skill_content.push_str("\n## Pitfalls\n");
    skill_content.push_str("- This skill is auto-generated and may need manual review\n");
    skill_content.push_str("- Verify the content before using in production\n");

    (skill_content, references, confidence)
}

/// 从内容中提取标题
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}

/// 提取关键章节
fn extract_key_sections(content: &str) -> Vec<String> {
    let mut sections = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            sections.push(trimmed.replace('#', "").trim().to_string());
        }
    }
    sections
}

/// 提取编号步骤
fn extract_numbered_steps(content: &str) -> Vec<String> {
    let mut steps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("1.")
            || trimmed.starts_with("2.")
            || trimmed.starts_with("3.")
            || trimmed.starts_with("4.")
            || trimmed.starts_with("5."))
            && trimmed.len() > 3
        {
            steps.push(trimmed.to_string());
        } else if trimmed.starts_with("- ") && trimmed.len() > 2 {
            steps.push(trimmed.to_string());
        }
    }
    steps
}

/// 提取对话模式
fn extract_conversation_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // 查找工具调用模式
    for window in lines.windows(3) {
        let combined = format!("{} {} {}", window[0], window[1], window[2]);
        if combined.contains("tool") || combined.contains("function") {
            patterns.push(format!("Sequence: {} → {} → {}", window[0], window[1], window[2]));
        }
    }

    // 如果没有找到工具模式，提取通用交互
    if patterns.is_empty() {
        for line in lines.iter().take(5) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.len() > 10 {
                patterns.push(trimmed.chars().take(100).collect());
            }
        }
    }

    patterns
}

/// 提取工具调用序列
fn extract_tool_sequence(content: &str) -> Vec<String> {
    let mut tools = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("tool_call")
            || trimmed.contains("tool(")
            || trimmed.contains("use_tool")
        {
            tools.push(trimmed.to_string());
        }
    }
    tools
}

/// 提取代码模式
fn extract_code_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut in_code_block = false;
    let mut current_block = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block && !current_block.is_empty() {
                patterns.push(current_block.clone());
                current_block.clear();
            }
            in_code_block = !in_code_block;
        } else if in_code_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    // 如果没有代码块，提取函数定义
    if patterns.is_empty() {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("fn ")
                || trimmed.contains("function ")
                || trimmed.contains("class ")
            {
                patterns.push(trimmed.to_string());
            }
        }
    }

    patterns
}

/// 提取使用模式
fn extract_usage_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("usage")
            || trimmed.contains("Usage")
            || trimmed.contains("example")
            || trimmed.contains("Example")
            || trimmed.contains("参数")
            || trimmed.contains("使用")
        {
            patterns.push(trimmed.to_string());
        }
    }
    patterns
}

/// 提取通用结构
fn extract_generic_structure(content: &str) -> Vec<String> {
    let mut structure = Vec::new();

    // 提取前几个有意义的段落
    let paragraphs: Vec<&str> = content.split("\n\n").collect();
    for para in paragraphs.iter().take(5) {
        let trimmed = para.trim();
        if !trimmed.is_empty() && trimmed.len() > 20 {
            let summary: String = trimmed.chars().take(200).collect();
            structure.push(summary);
        }
    }

    structure
}

/// /learn 命令 — 从各种来源学习并生成技能
#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "从文档/对话/代码库学习并生成技能")]
#[tauri::command]
pub async fn learn_skill(
    state: State<'_, AppState>,
    input: LearnSkillInput,
) -> Result<LearnSkillResult, String> {
    let mut steps_taken = Vec::new();

    steps_taken.push("Analyzing input source".to_string());

    // 验证来源类型
    let source_type = match input.source_type.as_str() {
        "document" | "conversation" | "codebase" | "mixed" => input.source_type.clone(),
        _ => {
            return Err(format!(
                "Unknown source type: {}. Expected: document, conversation, codebase, mixed",
                input.source_type
            ));
        },
    };

    // 验证内容不为空
    if input.content.trim().is_empty() {
        return Err("Content cannot be empty".to_string());
    }

    steps_taken.push("Content validated".to_string());

    // 生成技能名称
    let skill_name = if let Some(ref name) = input.name {
        name.clone()
    } else {
        // 从内容中自动生成名称
        let generated_name =
            extract_title(&input.content).map(|t| slugify(&t)).unwrap_or_else(|| {
                format!("learned-{}-{}", source_type, Utc::now().format("%Y%m%d%H%M%S"))
            });
        generated_name
    };

    validate_skill_name(&skill_name)?;

    steps_taken.push(format!("Generated skill name: {}", skill_name));

    // 检查是否已存在
    let dir = skills_dir().join(&skill_name);
    if dir.exists() {
        return Err(format!("Skill '{}' already exists at {}", skill_name, dir.display()));
    }

    // 生成技能内容
    steps_taken.push("Extracting knowledge from content".to_string());

    let (skill_content, references, confidence) = generate_skill_from_content(
        &skill_name,
        input.description.as_deref(),
        &input.content,
        &source_type,
        input.context.as_deref(),
    );

    steps_taken.push(format!("Generated skill content (confidence: {:.2})", confidence));

    // 审批语义：auto_approve=true 直接落盘；否则经审批门
    // （gate 关闭时 submit_for_approval 内部直接落盘并标记 Approved）
    let auto_approve = input.auto_approve.unwrap_or(false);
    let mut requires_approval = false;

    let mut operation_id = None;

    if auto_approve {
        // 显式绕过审批门，直接创建
        steps_taken.push("Creating skill directly (auto_approve)".to_string());

        std::fs::create_dir_all(&dir).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

        std::fs::write(dir.join("SKILL.md"), &skill_content).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    } else {
        // 走审批门（gate 关闭时 submit 内部直接落盘并 Approved）
        steps_taken.push("Submitting for approval".to_string());

        let manager = state.skill.skill_learning_manager.read().await;
        let op_type = axagent_trajectory::PendingOperationType::CreateSkill;

        match manager
            .submit_for_approval(
                op_type,
                None,
                Some(skill_name.clone()),
                None,
                skill_content.clone(),
                format!("Auto-generated skill from /learn ({})", source_type),
                None,
            )
            .await
        {
            Ok(operation) => {
                operation_id = Some(operation.id.clone());
                if operation.status == axagent_trajectory::ApprovalStatus::Pending {
                    requires_approval = true;
                    steps_taken.push(format!("Approval submitted: {}", operation.id));
                } else {
                    // 审批门未启用：submit 已直接落盘
                    steps_taken.push("Approval gate disabled, skill created directly".to_string());
                }
            },
            Err(e) => {
                // 安全守卫拦截等硬错误，直接返回
                return Err(e);
            },
        }
    }

    let skill_path = dir.to_string_lossy().to_string();
    steps_taken.push(format!("Skill saved to: {}", skill_path));

    Ok(LearnSkillResult {
        skill_name,
        skill_path,
        skill_content,
        references,
        confidence,
        steps_taken,
        requires_approval,
        operation_id,
    })
}

/// 简单的字符串转 slug 函数
fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
