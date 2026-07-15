use super::install::{
    MARKETPLACE_SEARCH_CACHE, MarketplaceSearchCache, home_dir, load_plugin_version, skills_dir,
    validate_skill_name,
};
use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::skill as skill_err;
use axagent_harness::types::*;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;

#[tauri::command]
pub async fn open_skills_dir() -> Result<(), String> {
    let dir = skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open::that(&dir).map_err(|e| format!("Failed to open directory: {}", e))
}

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

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
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

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
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

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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

#[tauri::command]
pub async fn get_marketplace_categories() -> Result<Vec<MarketplaceCategory>, String> {
    let url = "https://skillshub.wtf/api/v1/categories";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
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

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
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
    let canonical_dir = skills_dir().join(name).canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err("Path traversal detected".to_string());
    }
    let content = std::fs::read_to_string(&canonical_path).map_err(|e| e.to_string())?;
    Ok((canonical_path, content))
}

/// Patch an existing skill by appending a note
#[tauri::command]
pub async fn skill_patch(name: String, content: String) -> Result<String, ErrorResponse> {
    let (canonical_path, existing) = validate_and_read_skill_md(&name)?;
    let patched = format!(
        "{}\n\n## Patch ({})\n\n{}",
        existing,
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        content
    );

    std::fs::write(&canonical_path, &patched).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' patched", name))
}

/// Edit an existing skill by replacing the body (preserving frontmatter)
#[tauri::command]
pub async fn skill_edit(name: String, content: String) -> Result<String, ErrorResponse> {
    let (canonical_path, existing) = validate_and_read_skill_md(&name)?;

    // Preserve YAML frontmatter
    let edited = if let Some(fm_end) = find_frontmatter_end(&existing) {
        format!("{}\n\n{}", &existing[..fm_end], content)
    } else {
        content
    };

    std::fs::write(&canonical_path, &edited).map_err(|e| e.to_string())?;
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

#[tauri::command]
pub async fn get_skill_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::SkillProposal>, String> {
    let service = state.skill_proposal_service.read().await;
    Ok(service.get_proposals())
}

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

    let similar = closed_loop.find_similar_skills(&check_topic).await.map_err(|e| e.to_string())?;

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

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

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

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| e.to_string())?;

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
            target: serde_json::to_string(&upgrade_proposal).map_err(|e| e.to_string())?,
        };

        closed_loop.execute_upgrade_action(&auto_action).await;
        return Ok(format!("Skill '{}' upgraded successfully", name));
    }

    let dir = skills_dir().join(&name);
    if dir.exists() {
        return Err(format!("Skill '{}' already exists", name));
    }

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

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

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' created at {}", name, dir.display()))
}

/// 设置技能的 manifest 配置。写入或替换 skill-manifest.json。
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
    std::fs::write(&manifest_path, manifest_str).map_err(|e| e.to_string())?;

    Ok(format!("清单已保存: '{}'", name))
}
