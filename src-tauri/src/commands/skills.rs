use crate::paths::axagent_home;
use crate::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::types::*;
use axagent_plugins::PluginManager;
use axagent_trajectory::{HermesMetadata, Skill, SkillMetadata};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::State;

const SEARCH_CACHE_TTL_SECS: u64 = 300;

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
        home.join(".trae").join("skills"),
        home.join(".codebuddy").join("skills"),
        home.join(".workbuddy").join("skills"),
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

#[derive(Debug, Clone)]
struct CachedSearchResult {
    results: Vec<MarketplaceSkill>,
    created_at: Instant,
}

pub struct MarketplaceSearchCache {
    cache: HashMap<String, CachedSearchResult>,
    ttl: Duration,
}

impl MarketplaceSearchCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<MarketplaceSkill>> {
        self.cache.get(key).and_then(|cached| {
            if cached.created_at.elapsed() < self.ttl {
                Some(cached.results.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: String, results: Vec<MarketplaceSkill>) {
        self.cache.insert(
            key,
            CachedSearchResult {
                results,
                created_at: Instant::now(),
            },
        );
    }

    #[allow(dead_code)]
    pub fn cleanup_expired(&mut self) {
        self.cache.retain(|_, v| v.created_at.elapsed() < self.ttl);
    }

    pub fn make_key(query: &str, source: &str, sort: &str, page: u32) -> String {
        format!("{}:{}:{}:{}", query, source, sort, page)
    }
}

lazy_static::lazy_static! {
    static ref MARKETPLACE_SEARCH_CACHE: tokio::sync::Mutex<MarketplaceSearchCache> =
        tokio::sync::Mutex::new(MarketplaceSearchCache::new(SEARCH_CACHE_TTL_SECS));
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
            let frontend = p
                .metadata
                .root
                .as_ref()
                .map(|root| root.join("skill-manifest.json"))
                .and_then(|path| std::fs::read_to_string(&path).ok())
                .and_then(|s| {
                    let manifest: serde_json::Value = serde_json::from_str(&s).ok()?;
                    manifest.get("frontend").cloned()
                })
                .and_then(|v| serde_json::from_value(v).ok());
            SkillInfo {
                name: p.metadata.name.clone(),
                description: p.metadata.description.clone(),
                author: None,
                version: Some(p.metadata.version.clone()),
                source: p.metadata.source.clone(),
                source_path: p
                    .metadata
                    .root
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                enabled,
                has_update: false,
                user_invocable: true,
                argument_hint: None,
                when_to_use: None,
                group: None,
                frontend,
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

    let source_path = plugin
        .metadata
        .root
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
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
        frontend: manifest.as_ref().and_then(|m| m.frontend.clone()),
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
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
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

    let (skill_name, commit, source_ref, source_kind) =
        if source.starts_with('/') || source.starts_with('.') {
            let (name, commit) = install_from_local(&source, &target_dir).await?;
            (name, commit, source.clone(), "local".to_string())
        } else {
            let (owner, repo) = parse_github_source(&source)?;
            let ((name, commit), source_ref, source_kind) = (
                install_from_github(&owner, &repo, &target_dir).await?,
                format!("{}/{}", owner, repo),
                "github".to_string(),
            );
            (name, commit, source_ref, source_kind)
        };

    let skill_target = target_dir.join(&skill_name);
    let content = collect_skill_content(&skill_target);
    let now = chrono::Utc::now();

    let manifest_scenarios = load_plugin_scenarios(&skill_target);
    let final_scenarios = merge_scenarios(manifest_scenarios, scenarios);
    let version = load_plugin_version(&skill_target);

    let skill = Skill {
        id: uuid::Uuid::new_v4().to_string(),
        name: skill_name.clone(),
        description: String::new(),
        version,
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
                source_kind: Some(source_kind),
                source_ref: Some(source_ref),
                commit: Some(commit),
                skill_dependencies: None,
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

fn load_plugin_version(skill_dir: &Path) -> String {
    let manifest_path = skill_dir.join("plugin.json");
    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(version) = manifest.get("version").and_then(|v| v.as_str()) {
                return version.to_string();
            }
        }
    }
    "1.0.0".to_string()
}

fn merge_scenarios(
    manifest_scenarios: Vec<String>,
    user_scenarios: Option<Vec<String>>,
) -> Vec<String> {
    match user_scenarios {
        Some(user) if !user.is_empty() => {
            let mut merged = manifest_scenarios;
            for s in user {
                if !merged.contains(&s) {
                    merged.push(s);
                }
            }
            merged
        },
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
) -> Result<(String, String), String> {
    let git_url = format!("https://github.com/{}/{}.git", owner, repo);
    let skill_target = target_dir.join(repo);

    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    let git_available = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if git_available {
        let output = std::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                &git_url,
                skill_target.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if output.status.success() {
            let commit = get_git_commit(&skill_target).unwrap_or_else(|| "unknown".to_string());
            save_skill_manifest(
                &skill_target,
                "github",
                &format!("{}/{}", owner, repo),
                "main",
                &commit,
            )?;
            return Ok((repo.to_string(), commit));
        }
    }

    install_from_github_zipball(owner, repo, target_dir).await
}

async fn install_from_github_zipball(
    owner: &str,
    repo: &str,
    target_dir: &Path,
) -> Result<(String, String), String> {
    let url = format!("https://api.github.com/repos/{}/{}/zipball", owner, repo);

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

    let top_dir = archive
        .file_names()
        .next()
        .and_then(|n| n.split('/').next())
        .map(String::from)
        .ok_or("Empty archive")?;

    let commit = top_dir
        .split('-')
        .next_back()
        .unwrap_or("unknown")
        .to_string();

    archive
        .extract(temp_dir.path())
        .map_err(|e| format!("Failed to extract: {}", e))?;

    let extracted = temp_dir.path().join(&top_dir);
    let skill_target = target_dir.join(repo);

    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    copy_dir_recursive(&extracted, &skill_target)?;
    save_skill_manifest(
        &skill_target,
        "github",
        &format!("{}/{}", owner, repo),
        "main",
        &commit,
    )?;

    Ok((repo.to_string(), commit))
}

fn get_git_commit(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        let hash = String::from_utf8_lossy(&output.stdout);
        Some(hash.trim()[..7.min(hash.len())].to_string())
    } else {
        None
    }
}

fn save_skill_manifest(
    skill_target: &Path,
    source_kind: &str,
    source_ref: &str,
    branch: &str,
    commit: &str,
) -> Result<(), String> {
    let manifest_path = skill_target.join("skill-manifest.json");

    let mut manifest: serde_json::Value = if manifest_path.exists() {
        let existing = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&existing).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    manifest["source_kind"] = serde_json::json!(source_kind);
    manifest["source_ref"] = serde_json::json!(source_ref);
    manifest["branch"] = serde_json::json!(branch);
    manifest["commit"] = serde_json::json!(commit);
    manifest["installed_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    manifest["installed_via"] = serde_json::json!("marketplace");

    let version_entry = serde_json::json!({
        "version": commit,
        "installed_at": chrono::Utc::now().to_rfc3339(),
        "commit": commit
    });

    if let Some(versions) = manifest["versions"].as_array_mut() {
        versions.insert(0, version_entry);
        if versions.len() > 10 {
            *versions = versions.iter().take(10).cloned().collect();
        }
    } else {
        manifest["versions"] = serde_json::json!([version_entry]);
    }

    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct SkillVersion {
    pub version: String,
    pub installed_at: String,
    pub commit: String,
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_skill_versions(skill_name: String) -> Result<Vec<SkillVersion>, String> {
    let skill_dir = skills_dir().join(&skill_name);
    let manifest_path = skill_dir.join("skill-manifest.json");

    if !manifest_path.exists() {
        return Err(format!("Skill {} not found", skill_name));
    }

    let manifest_str = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;

    let versions: Vec<SkillVersion> = manifest["versions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(SkillVersion {
                        version: v["version"].as_str()?.to_string(),
                        installed_at: v["installed_at"].as_str()?.to_string(),
                        commit: v["commit"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(versions)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn rollback_skill(skill_name: String, target_version: String) -> Result<String, String> {
    let skill_dir = skills_dir().join(&skill_name);
    let manifest_path = skill_dir.join("skill-manifest.json");

    if !manifest_path.exists() {
        return Err(format!("Skill {} not found", skill_name));
    }

    let manifest_str = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;

    let source_kind = manifest["source_kind"].as_str().unwrap_or("github");
    let source_ref = manifest["source_ref"].as_str().unwrap_or("");
    let branch = manifest["branch"].as_str().unwrap_or("main");

    if source_kind != "github" {
        return Err("Rollback is only supported for GitHub-sourced skills".to_string());
    }

    let parts: Vec<&str> = source_ref.split('/').collect();
    if parts.len() != 2 {
        return Err("Invalid source_ref format".to_string());
    }

    let (owner, repo) = (parts[0], parts[1]);
    let git_url = format!("https://github.com/{}/{}.git", owner, repo);

    std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;

    let output = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "50",
            &git_url,
            skill_dir.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let checkout_output = std::process::Command::new("git")
        .args(["checkout", &target_version])
        .current_dir(&skill_dir)
        .output()
        .map_err(|e| format!("Failed to checkout version: {}", e))?;

    if !checkout_output.status.success() {
        return Err(format!(
            "Git checkout failed: {}",
            String::from_utf8_lossy(&checkout_output.stderr)
        ));
    }

    save_skill_manifest(&skill_dir, source_kind, source_ref, branch, &target_version)?;

    Ok(format!(
        "Rolled back {} to version {}",
        skill_name, target_version
    ))
}

async fn install_from_local(source: &str, target_dir: &Path) -> Result<(String, String), String> {
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

    Ok((name, "local".to_string()))
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
        p.parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| p.to_path_buf())
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

    Some(InstalledSkillInfo {
        commit,
        version,
        source_ref,
    })
}

async fn check_github_update(
    owner: &str,
    repo: &str,
    current_commit: &str,
) -> Option<(String, String)> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits?per_page=1",
        owner, repo
    );

    let client = reqwest::Client::new();
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

    Some((
        latest_sha[..7.min(latest_sha.len())].to_string(),
        latest_sha.to_string(),
    ))
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

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let items = body["data"].as_array().cloned().unwrap_or_default();

    let mut results: Vec<MarketplaceSkill> = Vec::new();
    for item in items {
        let name = item["name"].as_str().unwrap_or("").to_string();
        let slug = item["slug"].as_str().unwrap_or("").to_string();
        let description = item["description"].as_str().unwrap_or("").to_string();
        let repo_obj = item.get("repo").ok_or("missing repo object")?;
        let github_owner = repo_obj
            .get("githubOwner")
            .and_then(|v| v.as_str())
            .ok_or("missing githubOwner")?;
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
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        let tags = item.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

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

#[allow(dead_code)]
#[tauri::command]
pub async fn get_marketplace_categories() -> Result<Vec<MarketplaceCategory>, String> {
    let url = "https://skillshub.wtf/api/v1/categories";

    let client = reqwest::Client::new();
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

/// Patch an existing skill by appending a note
#[tauri::command]
pub async fn skill_patch(name: String, content: String) -> Result<String, String> {
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
pub async fn skill_edit(name: String, content: String) -> Result<String, String> {
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
                let pos = content
                    .lines()
                    .take(i + 1)
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
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
    let result = skill_create(
        state.clone(),
        name.clone(),
        description.clone(),
        content,
        Some(false),
    )
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

    let similar = closed_loop
        .find_similar_skills(&check_topic)
        .map_err(|e| e.to_string())?;

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
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\nversion: 1.0.0\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
        name, desc, content
    );

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| e.to_string())?;

    Ok(SkillCreateCheckResult {
        has_similar: false,
        similar_skills: vec![],
        can_create: true,
        message: format!("Skill '{}' created at {}", name, dir.display()),
    })
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
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\nversion: 1.0.0\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
        name, desc, content
    );

    std::fs::write(dir.join("SKILL.md"), &skill_md).map_err(|e| e.to_string())?;
    Ok(format!("Skill '{}' created at {}", name, dir.display()))
}

/// 设置技能的前端扩展配置。若 skill-manifest.json 不存在则创建，存在则更新 frontend 字段。
#[tauri::command]
pub async fn skill_set_frontend(
    name: String,
    frontend: SkillFrontendExtension,
) -> Result<String, String> {
    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name));
    }

    let manifest_path = skill_dir.join("skill-manifest.json");
    let mut manifest: serde_json::Value = if manifest_path.exists() {
        let existing =
            std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&existing).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    manifest["frontend"] =
        serde_json::to_value(&frontend).map_err(|e| e.to_string())?;

    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    Ok(format!(
        "Frontend config set for skill '{}'",
        name
    ))
}

/// 使用 LLM 分析技能内容，自动生成前端扩展配置建议
#[tauri::command]
pub async fn skill_analyze_frontend(
    state: State<'_, AppState>,
    name: String,
) -> Result<SkillFrontendExtension, String> {
    // 读取技能内容
    let plugin_manager = create_plugin_manager_with_skill_dirs()?;
    let plugins = plugin_manager.list_plugins().map_err(|e| e.to_string())?;
    let plugin = plugins
        .into_iter()
        .find(|p| p.metadata.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    let skill_dir = plugin.metadata.root.ok_or_else(|| "Skill has no root dir".to_string())?;
    let raw_content = collect_skill_content(&skill_dir);

    // 限制内容长度，避免 prompt 过大
    let max_content_len = 8000;
    let skill_content = if raw_content.len() > max_content_len {
        format!(
            "{}...(内容已截断，总长度 {} 字符)",
            &raw_content[..max_content_len],
            raw_content.len()
        )
    } else {
        raw_content
    };

    if skill_content.trim().is_empty() {
        return Err("Skill content is empty, cannot analyze".to_string());
    }

    // 获取默认 Provider 配置
    let settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    let provider_id = settings
        .default_provider_id
        .as_ref()
        .ok_or_else(|| "未配置默认模型提供商".to_string())?;
    let model_id = settings
        .default_model_id
        .as_ref()
        .ok_or_else(|| "未配置默认模型".to_string())?;

    // 获取 Provider 并解密 key
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider.id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted_key =
        decrypt_key(&key_row.key_encrypted, &state.master_key).map_err(|e| e.to_string())?;

    // 注册 Provider Adapter
    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let registry_key = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("Provider adapter not found for {}", registry_key))?;

    // 构造分析 prompt
    let prompt = format!(
        r#"你是一个 UI 扩展分析专家。请分析以下技能(Skill)的说明文档，推断出该技能可能需要哪些前端 UI 扩展。

技能名称：{name}

技能内容：
---
{skill_content}
---

## 分析要求

请为该技能设计合适的前端扩展配置，以 JSON 格式输出。支持的扩展类型：

### navigation（导航）
向侧边栏添加导航菜单项。字段：
- id: 唯一标识
- label: 显示标签
- icon: 图标标识 (如 "lucide:Puzzle", "lucide:FolderOpen", "lucide:Settings", "lucide:Search", "lucide:Code", "lucide:ChartBar", "lucide:Database", "lucide:Globe", "lucide:Play", "lucide:Wrench", "lucide:MessageSquare", "lucide:LayoutDashboard", "lucide:ExternalLink")
- path: 路由路径 (如 "/skill-name")
- position: "Top" 或 "Bottom"
- order: 排序数字

### pages（页面）
技能的自定义页面。字段：
- id: 唯一标识
- path: 路由路径
- title: 页面标题
- componentType: "Html" | "Iframe" | "Markdown"
- componentConfig: 配置对象，如 {{ "file": "index.html" }}

### commands（命令）
命令面板中的命令。字段：
- id: 唯一标识
- label: 命令标签
- category: 命令分类
- icon: 图标标识（可选）
- action: 动作对象，支持以下类型：
  - {{ "type": "Navigate", "path": "/path" }}  导航到页面
  - {{ "type": "InvokeBackend", "command": "command_name", "args": {{}} }}  调用后端命令
  - {{ "type": "EmitEvent", "event": "event_name", "payload": {{}} }}  发送事件

### panels（面板）
嵌入到界面的面板。字段：
- id: 唯一标识
- title: 面板标题
- componentType: "Html" | "Iframe" | "Markdown"
- componentConfig: 配置对象，如 {{ "file": "panel.html" }}
- position: "Main" | "Sidebar" | "Header" | "Footer"
- size: "Small" | "Medium" | "Large" | "FullWidth"
- collapsible: true/false

### settingsSections（设置段）
设置页面的自定义段。字段：
- id: 唯一标识
- label: 段标签
- icon: 图标标识（可选）
- componentType: "Html" | "Iframe" | "Markdown"
- componentConfig: 配置对象，如 {{ "file": "settings.html" }}

## 输出格式

请只输出 JSON，不要有其他文字。格式如下：
{{
  "navigation": [...],
  "pages": [...],
  "commands": [...],
  "panels": [...],
  "settingsSections": [...]
}}

请根据技能内容，合理推断并生成合适的扩展配置。如果某个类型不需要，返回空数组 []。"#,
        name = name,
        skill_content = skill_content,
    );

    let base_url =
        axagent_providers::resolve_base_url_for_type(&provider.api_host, &provider.provider_type);
    let ctx = axagent_providers::ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id.clone(),
        base_url: Some(base_url),
        api_path: provider.api_path.clone(),
        proxy_config: provider.proxy_config.clone(),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let llm_request = ChatRequest {
        model: model_id.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(prompt),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: Some(0.3),
        top_p: None,
        max_tokens: Some(4096),
        stream: false,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let response = adapter
        .chat(&ctx, llm_request)
        .await
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    let content = response.content.trim();

    // 提取 JSON（处理 markdown 代码块）
    let json_str = if let Some(start) = content.find("```json") {
        let after = &content[start + 7..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            after
        }
    } else if let Some(start) = content.find("```") {
        let after = &content[start + 3..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            after
        }
    } else {
        let json_start = content.find('{');
        let json_end = content.rfind('}').map(|i| i + 1);
        match (json_start, json_end) {
            (Some(start), Some(end)) => &content[start..end],
            _ => content,
        }
    }
    .trim();

    let frontend: SkillFrontendExtension =
        serde_json::from_str(json_str).map_err(|e| {
            format!(
                "解析 LLM 响应失败: {}。原始响应: {}",
                e,
                &content[..content.len().min(500)]
            )
        })?;

    Ok(frontend)
}

/// 读取技能目录下的资源文件内容（用于 HTML/JS/CSS 等静态资源）
#[tauri::command]
pub fn skill_read_asset(name: String, file_name: String) -> Result<String, String> {
    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name));
    }

    // 安全检查：防止路径遍历攻击
    let requested = skill_dir.join(&file_name);
    let canonical_dir = skill_dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_requested = requested.canonicalize().map_err(|e| e.to_string())?;

    if !canonical_requested.starts_with(&canonical_dir) {
        return Err("Access denied: file is outside skill directory".to_string());
    }

    if !canonical_requested.is_file() {
        return Err(format!("File '{}' not found in skill '{}'", file_name, name));
    }

    // 只允许文本类文件
    let ext = canonical_requested
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allowed = ["html", "htm", "md", "txt", "css", "js", "json", "svg", "xml"];
    if !allowed.contains(&ext.as_str()) {
        return Err(format!("File type '{}' is not allowed for direct reading", ext));
    }

    std::fs::read_to_string(&canonical_requested).map_err(|e| e.to_string())
}
