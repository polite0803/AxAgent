use crate::paths::axagent_home;
use crate::AppState;
use axagent_core::types::*;
use axagent_plugins::PluginManager;
use axagent_trajectory::{Skill, SkillMetadata, HermesMetadata};
use std::path::{Path, PathBuf};
use tauri::State;

fn home_dir() -> PathBuf {
    dirs::home_dir().expect("Could not determine home directory")
}

fn skills_dir() -> PathBuf {
    axagent_home().join("skills")
}

fn all_skills_dirs() -> Vec<PathBuf> {
    let home = home_dir();
    vec![
        axagent_home().join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ]
}

fn create_plugin_manager_with_skill_dirs() -> Result<PluginManager, String> {
    let home = home_dir();
    let config_home = home.join(".claw");
    let mut config = axagent_plugins::PluginManagerConfig::new(config_home);
    config.external_dirs = all_skills_dirs();
    Ok(PluginManager::new(config))
}

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillInfo>, String> {
    let plugin_manager = create_plugin_manager_with_skill_dirs()?;
    let plugins = plugin_manager.list_plugins().map_err(|e| e.to_string())?;

    let disabled = axagent_core::repo::skill::get_disabled_skills(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    let result: Vec<SkillInfo> = plugins
        .into_iter()
        .map(|p| {
            let enabled = !disabled.contains(&p.metadata.name);
            SkillInfo {
                name: p.metadata.name.clone(),
                description: p.metadata.description.clone(),
                author: None,
                version: Some(p.metadata.version.clone()),
                source: p.metadata.source.clone(),
                source_path: p.metadata.root.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                enabled,
                has_update: false,
                user_invocable: true,
                argument_hint: None,
                when_to_use: None,
                group: None,
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn get_skill(state: State<'_, AppState>, name: String) -> Result<SkillDetail, String> {
    let plugin_manager = create_plugin_manager_with_skill_dirs()?;
    let plugins = plugin_manager.list_plugins().map_err(|e| e.to_string())?;

    let plugin = plugins
        .into_iter()
        .find(|p| p.metadata.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    let disabled = axagent_core::repo::skill::get_disabled_skills(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    let source_path = plugin.metadata.root.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let skill_dir = plugin.metadata.root.unwrap_or(PathBuf::new());

    // List files in skill directory
    let files = std::fs::read_dir(&skill_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Read manifest if exists
    let manifest_path = skill_dir.join("skill-manifest.json");
    let manifest = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<SkillManifest>(&s).ok());

    // Read all .md files in the skill directory as content
    let content = collect_skill_content(&skill_dir);

    let info = SkillInfo {
        name: plugin.metadata.name.clone(),
        description: plugin.metadata.description.clone(),
        author: None,
        version: Some(plugin.metadata.version.clone()),
        source: plugin.metadata.source.clone(),
        source_path,
        enabled: !disabled.contains(&plugin.metadata.name),
        has_update: false,
        user_invocable: true,
        argument_hint: None,
        when_to_use: None,
        group: None,
    };

    Ok(SkillDetail {
        info,
        content,
        files,
        manifest,
    })
}

/// Recursively read all .md files in a skill directory and concatenate them.
fn collect_skill_content(dir: &Path) -> String {
    let mut content = String::new();
    let Ok(entries) = collect_markdown_files(dir) else {
        return content;
    };
    for path in entries {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if !content.is_empty() {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&text);
        }
    }
    content
}

/// Recursively collect all .md files under a directory, sorted by name.
pub(crate) fn collect_markdown_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_markdown_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[tauri::command]
pub async fn toggle_skill(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    axagent_core::repo::skill::set_skill_enabled(&state.sea_db, &name, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_skill(
    state: State<'_, AppState>,
    source: String,
    target: Option<String>,
    scenarios: Option<Vec<String>>,
) -> Result<String, String> {
    let target_dir = match target.as_deref() {
        Some("claude") => home_dir().join(".claude").join("skills"),
        Some("agents") => home_dir().join(".agents").join("skills"),
        _ => skills_dir(),
    };
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let skill_name = if source.starts_with('/') || source.starts_with('.') {
        install_from_local(&source, &target_dir).await?
    } else {
        let (owner, repo) = parse_github_source(&source)?;
        install_from_github(&owner, &repo, &target_dir).await?
    };

    let skill_target = target_dir.join(&skill_name);
    let content = collect_skill_content(&skill_target);
    let now = chrono::Utc::now();

    let manifest_scenarios = load_plugin_scenarios(&skill_target);
    let final_scenarios = merge_scenarios(manifest_scenarios, scenarios);

    let skill = Skill {
        id: uuid::Uuid::new_v4().to_string(),
        name: skill_name.clone(),
        description: String::new(),
        version: "1.0.0".to_string(),
        content,
        category: "installed".to_string(),
        tags: vec![],
        platforms: vec![],
        scenarios: final_scenarios,
        quality_score: 0.0,
        success_rate: 0.0,
        avg_execution_time_ms: 0,
        total_usages: 0,
        successful_usages: 0,
        created_at: now,
        updated_at: now,
        last_used_at: None,
        metadata: SkillMetadata {
            hermes: HermesMetadata {
                tags: vec![],
                category: "installed".to_string(),
                fallback_for_toolsets: vec![],
                requires_toolsets: vec![],
                config: vec![],
            },
            references: vec![],
        },
    };

    state
        .trajectory_storage
        .save_skill(&skill)
        .map_err(|e| e.to_string())?;

    Ok(skill_name)
}

fn load_plugin_scenarios(skill_dir: &Path) -> Vec<String> {
    let manifest_path = skill_dir.join("plugin.json");
    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<axagent_plugins::PluginManifest>(&contents) {
            return manifest.scenarios;
        }
    }
    let skill_manifest_path = skill_dir.join("skill-manifest.json");
    if let Ok(contents) = std::fs::read_to_string(&skill_manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(scenarios) = manifest.get("scenarios").and_then(|v| v.as_array()) {
                return scenarios
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }
    }
    vec![]
}

fn merge_scenarios(manifest_scenarios: Vec<String>, user_scenarios: Option<Vec<String>>) -> Vec<String> {
    match user_scenarios {
        Some(user) if !user.is_empty() => {
            let mut merged = manifest_scenarios;
            for s in user {
                if !merged.contains(&s) {
                    merged.push(s);
                }
            }
            merged
        }
        _ => manifest_scenarios,
    }
}

fn parse_github_source(source: &str) -> Result<(String, String), String> {
    let clean = source.trim_end_matches('/').trim_end_matches(".git");

    if clean.contains("github.com") {
        let parts: Vec<&str> = clean.split('/').collect();
        let len = parts.len();
        if len >= 2 {
            return Ok((parts[len - 2].to_string(), parts[len - 1].to_string()));
        }
        return Err(format!("Invalid GitHub URL: {}", source));
    }

    let parts: Vec<&str> = source.split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        Err(format!(
            "Invalid source format '{}'. Expected 'owner/repo', GitHub URL, or local path.",
            source
        ))
    }
}

async fn install_from_github(
    owner: &str,
    repo: &str,
    target_dir: &Path,
) -> Result<String, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/zipball",
        owner, repo
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to download skill: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API returned status {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cursor = std::io::Cursor::new(&bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip: {}", e))?;

    // GitHub zipball has a top-level directory like "owner-repo-hash/"
    let top_dir = archive
        .file_names()
        .next()
        .and_then(|n| n.split('/').next())
        .map(String::from)
        .ok_or("Empty archive")?;

    archive
        .extract(temp_dir.path())
        .map_err(|e| format!("Failed to extract: {}", e))?;

    let extracted = temp_dir.path().join(&top_dir);
    let skill_target = target_dir.join(repo);

    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    copy_dir_recursive(&extracted, &skill_target)?;

    let manifest = serde_json::json!({
        "source_kind": "github",
        "source_ref": format!("{}/{}", owner, repo),
        "branch": "main",
        "commit": top_dir.split('-').last().unwrap_or("unknown"),
        "installed_at": chrono::Utc::now().to_rfc3339(),
        "installed_via": "marketplace"
    });
    let manifest_path = skill_target.join("skill-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    Ok(repo.to_string())
}

async fn install_from_local(source: &str, target_dir: &Path) -> Result<String, String> {
    let source_path = PathBuf::from(source);
    if !source_path.exists() {
        return Err(format!("Source path does not exist: {}", source));
    }
    if !source_path.is_dir() {
        return Err(format!("Source path is not a directory: {}", source));
    }

    let name = source_path
        .file_name()
        .ok_or("Invalid source directory name")?
        .to_string_lossy()
        .to_string();

    let skill_target = target_dir.join(&name);
    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    copy_dir_recursive(&source_path, &skill_target)?;

    let manifest = serde_json::json!({
        "source_kind": "local",
        "source_ref": source,
        "installed_at": chrono::Utc::now().to_rfc3339(),
        "installed_via": "local"
    });
    let manifest_path = skill_target.join("skill-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    Ok(name)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn uninstall_skill(name: String) -> Result<(), String> {
    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found in ~/.axagent/skills/", name));
    }
    std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_skill_group(group: String) -> Result<(), String> {
    // Search all skill roots for a directory matching the group name
    let home = home_dir();
    let search_dirs = [
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];

    for parent in &search_dirs {
        let group_dir = parent.join(&group);
        if group_dir.exists() && group_dir.is_dir() {
            std::fs::remove_dir_all(&group_dir).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err(format!("Skill group '{}' not found", group))
}

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

#[tauri::command]
pub async fn search_marketplace(
    query: String,
    source: Option<String>,
    sort: Option<String>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let installed_refs = installed_source_refs();
    let sort_order = sort.as_deref().unwrap_or("popular");

    match source.as_deref().unwrap_or("skillhub") {
        "github" => {
            let gh_sort = match sort_order {
                "latest" => "updated",
                "stars" => "stars",
                _ => "stars",
            };
            let url = format!(
                "https://api.github.com/search/repositories?q={}+topic:agent-skill&sort={}&per_page=20",
                urlencoding::encode(&query),
                gh_sort
            );

            let client = reqwest::Client::new();
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

            let body: serde_json::Value =
                response.json().await.map_err(|e| e.to_string())?;
            let items = body["items"].as_array().cloned().unwrap_or_default();

            let results: Vec<MarketplaceSkill> = items
                .into_iter()
                .map(|item| {
                    let skill_name = item["name"].as_str().unwrap_or("").to_string();
                    let repo = item["full_name"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let installed = installed_refs
                        .contains(&repo.trim().trim_end_matches('/').to_lowercase());
                    MarketplaceSkill {
                        name: skill_name,
                        description: item["description"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        repo,
                        stars: item["stargazers_count"].as_i64().unwrap_or(0),
                        installs: 0,
                        installed,
                    }
                })
                .collect();

            Ok(results)
        }
        _ => {
            let (sort_param, limit) = match sort_order {
                "latest" => ("recent", 20),
                "stars" => ("stars", 20),
                _ => ("downloads", 20),
            };
            let url = format!(
                "https://skillshub.wtf/api/v1/skills/search?q={}&sort={}&limit={}",
                urlencoding::encode(&query),
                sort_param,
                limit
            );

            let client = reqwest::Client::new();
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

            let body: serde_json::Value =
                response.json().await.map_err(|e| e.to_string())?;
            let items = body["data"].as_array().cloned().unwrap_or_default();

            let results: Vec<MarketplaceSkill> = items
                .into_iter()
                .filter_map(|item| {
                    let name = item["name"].as_str().unwrap_or("").to_string();
                    let slug = item["slug"].as_str().unwrap_or("").to_string();
                    let description = item["description"].as_str().unwrap_or("").to_string();
                    let repo_obj = item.get("repo")?;
                    let github_owner = repo_obj.get("githubOwner")?.as_str()?;
                    let github_repo_name = repo_obj.get("githubRepoName")?.as_str()?;
                    let repo = format!("{}/{}", github_owner, github_repo_name);
                    let installed = installed_refs.contains(&repo.to_lowercase());
                    let stars = item["stars"].as_i64().unwrap_or(0);
                    let installs = item["downloads"].as_i64().unwrap_or(0);
                    Some(MarketplaceSkill {
                        name: if !name.is_empty() { name } else { slug },
                        description: description.to_string(),
                        repo,
                        stars,
                        installs,
                        installed,
                    })
                })
                .collect();

            Ok(results)
        }
    }
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

        let url = format!(
            "https://api.github.com/repos/{}/{}/commits?per_page=1",
            parts[0], parts[1]
        );

        let client = reqwest::Client::new();
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
                            let latest_sha =
                                latest["sha"].as_str().unwrap_or("").to_string();
                            let short_latest = &latest_sha[..7.min(latest_sha.len())];
                            if !current_commit.is_empty()
                                && !latest_sha.starts_with(&current_commit)
                                && current_commit != short_latest
                            {
                                updates.push(SkillUpdateInfo {
                                    name: entry
                                        .file_name()
                                        .to_string_lossy()
                                        .to_string(),
                                    current_commit: current_commit.clone(),
                                    latest_commit: short_latest.to_string(),
                                    source_ref: source_ref.clone(),
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

/// Create a new skill with SKILL.md (YAML frontmatter + Markdown body)
#[tauri::command]
pub async fn skill_create(
    name: String,
    description: String,
    content: String,
) -> Result<String, String> {
    let dir = skills_dir().join(&name);
    if dir.exists() {
        return Err(format!("Skill '{}' already exists at {}", name, dir.display()));
    }

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let desc = if description.is_empty() { name.clone() } else { description };
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\nversion: 1.0.0\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
        name, desc, content
    );

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' created at {}", name, dir.display()))
}

/// Patch an existing skill by appending a note
#[tauri::command]
pub async fn skill_patch(
    name: String,
    content: String,
) -> Result<String, String> {
    let path = skills_dir().join(&name).join("SKILL.md");
    if !path.exists() {
        return Err(format!("Skill '{}' not found", name));
    }

    let existing = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let patched = format!(
        "{}\n\n## Patch ({})\n\n{}",
        existing,
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        content
    );

    std::fs::write(&path, &patched).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' patched", name))
}

/// Edit an existing skill by replacing the body (preserving frontmatter)
#[tauri::command]
pub async fn skill_edit(
    name: String,
    content: String,
) -> Result<String, String> {
    let path = skills_dir().join(&name).join("SKILL.md");
    if !path.exists() {
        return Err(format!("Skill '{}' not found", name));
    }

    let existing = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

    // Preserve YAML frontmatter
    let edited = if let Some(fm_end) = find_frontmatter_end(&existing) {
        format!("{}\n\n{}", &existing[..fm_end], content)
    } else {
        content
    };

    std::fs::write(&path, &edited).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' edited", name))
}

/// Find the end position of YAML frontmatter (after the second `---` marker).
fn find_frontmatter_end(content: &str) -> Option<usize> {
    let mut count = 0;
    for (i, line) in content.lines().enumerate() {
        if line.trim() == "---" {
            count += 1;
            if count == 2 {
                let pos = content.lines().take(i + 1).map(|l| l.len() + 1).sum::<usize>();
                return Some(pos);
            }
        }
    }
    None
}

#[tauri::command]
pub async fn get_skill_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::SkillProposal>, String> {
    let service = state.skill_proposal_service.read().map_err(|e| e.to_string())?;
    Ok(service.get_proposals())
}

#[tauri::command]
pub async fn create_skill_from_proposal(
    state: State<'_, AppState>,
    name: String,
    description: String,
    content: String,
) -> Result<String, String> {
    let result = skill_create(name.clone(), description, content).await?;
    let mut service = state.skill_proposal_service.write().map_err(|e| e.to_string())?;
    service.clear_proposal(&name);
    Ok(result)
}
