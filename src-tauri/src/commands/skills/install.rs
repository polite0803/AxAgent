// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::skill as skill_err;
use crate::commands::error_code::skill_op_err;
use crate::paths::axagent_home;
use axagent_harness::types::*;
use axagent_trajectory::{HermesMetadata, Skill, SkillMetadata};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{Emitter, State};

const SEARCH_CACHE_TTL_SECS: u64 = 300;

/// 简易语义版本比较。返回 Ordering。
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u32> {
        v.split(|c: char| !c.is_ascii_digit()).filter_map(|s| s.parse::<u32>().ok()).collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let na = va.get(i).copied().unwrap_or(0);
        let nb = vb.get(i).copied().unwrap_or(0);
        match na.cmp(&nb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

pub(super) fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| {
        tracing::warn!("无法确定用户主目录，使用当前目录作为后备");
        PathBuf::from(".")
    })
}

pub(super) fn skills_dir() -> PathBuf {
    axagent_home().join("skills")
}

#[derive(Debug, Clone)]
struct CachedSearchResult {
    results: Vec<MarketplaceSkill>,
    created_at: Instant,
}

pub struct MarketplaceSearchCache {
    cache: HashMap<String, CachedSearchResult>,
    ttl: Duration,
    max_capacity: usize,
}

impl MarketplaceSearchCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self { cache: HashMap::new(), ttl: Duration::from_secs(ttl_seconds), max_capacity: 256 }
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
        self.cleanup_expired();
        // 超出容量时移除最旧的条目
        if self.cache.len() >= self.max_capacity {
            let mut entries: Vec<_> = self.cache.iter().collect();
            entries.sort_by_key(|(_, v)| v.created_at);
            let remove_count = entries.len() - self.max_capacity + 1;
            // P2 #6: 使用 into_iter() 消除多余 clone
            let keys_to_remove: Vec<String> =
                entries.into_iter().take(remove_count).map(|(k, _)| k.clone()).collect();
            for k in keys_to_remove {
                self.cache.remove(&k);
            }
        }
        self.cache.insert(key, CachedSearchResult { results, created_at: Instant::now() });
    }

    pub fn cleanup_expired(&mut self) {
        self.cache.retain(|_, v| v.created_at.elapsed() < self.ttl);
    }

    pub fn make_key(query: &str, source: &str, sort: &str, page: u32) -> String {
        format!("{}:{}:{}:{}", query, source, sort, page)
    }
}

lazy_static::lazy_static! {
    pub(super) static ref MARKETPLACE_SEARCH_CACHE: tokio::sync::Mutex<MarketplaceSearchCache> =
        tokio::sync::Mutex::new(MarketplaceSearchCache::new(SEARCH_CACHE_TTL_SECS));
}

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillInfo>, String> {
    // P2 #7: 使用 SkillState 中缓存的 PluginManager，避免每次完整重建
    let plugin_manager = state.skill.plugin_manager.read().await;
    // Use plugin_registry_report() directly instead of list_plugins().
    // list_plugins() -> plugin_registry() -> plugin_registry_report()?.into_registry()
    // into_registry() returns Err(LoadFailures) if ANY plugin fails to load,
    // which makes a single broken SKILL.md kill the entire skills page.
    // By using the report directly, we can show successfully loaded plugins
    // while logging failures.
    let report = plugin_manager.plugin_registry_report().map_err(|e| e.to_string())?;
    let failures = report.failures();
    for f in failures {
        tracing::warn!("Skill load failure: {f}");
    }
    let plugins = report.into_registry_allowing_failures();

    let disabled = axagent_dao::repo::skill::get_disabled_skills(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;

    let result: Vec<SkillInfo> = {
        let mut seen: std::collections::HashMap<String, SkillInfo> =
            std::collections::HashMap::new();
        for p in plugins.summaries().into_iter() {
            let enabled = !disabled.contains(&p.metadata.name);
            let manifest = p
                .metadata
                .root
                .as_ref()
                .map(|root| root.join("skill-manifest.json"))
                .and_then(|path| std::fs::read_to_string(&path).ok())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            let info = SkillInfo {
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
                manifest,
            };
            let existing = seen.get(&info.name);
            let should_replace = match existing {
                None => true,
                Some(old) => {
                    // axagent source always takes priority
                    if info.source == "axagent" {
                        true
                    } else if old.source == "axagent" {
                        false
                    } else {
                        // Compare versions: keep the higher version
                        let old_ver = old.version.as_deref().unwrap_or("0.0.0");
                        let new_ver = info.version.as_deref().unwrap_or("0.0.0");
                        compare_versions(new_ver, old_ver).is_gt()
                    }
                },
            };
            if should_replace {
                seen.insert(info.name.clone(), info);
            }
        }
        seen.into_values().collect()
    };

    Ok(result)
}

#[tauri::command]
pub async fn get_skill(
    state: State<'_, AppState>,
    name: String,
) -> Result<SkillDetail, ErrorResponse> {
    // P2 #7: 使用 SkillState 中缓存的 PluginManager，避免每次完整重建
    let plugin_manager = state.skill.plugin_manager.read().await;
    // Use plugin_registry_report() + into_registry_allowing_failures()
    // to tolerate individual plugin load failures (e.g. Claude Code format, missing version).
    let report = plugin_manager.plugin_registry_report().map_err(|e| e.to_string())?;
    let failures = report.failures();
    for f in failures {
        tracing::warn!("Skill load failure: {f}");
    }
    let plugins = report.into_registry_allowing_failures();

    let plugin =
        plugins.summaries().into_iter().find(|p| p.metadata.name == name).ok_or_else(|| {
            ErrorResponse::new(skill_err::NOT_FOUND).with_param("name".to_string(), name.clone())
        })?;

    let disabled = axagent_dao::repo::skill::get_disabled_skills(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;

    let source_path =
        plugin.metadata.root.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
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

    // Read install metadata manifest (skill-manifest.json)
    let manifest_path = skill_dir.join("skill-manifest.json");
    let raw_manifest_json = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    let install_meta = raw_manifest_json
        .as_ref()
        .and_then(|v| serde_json::from_value::<SkillManifest>(v.clone()).ok());

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
        manifest: raw_manifest_json,
    };

    Ok(SkillDetail { info, content, files, manifest: install_meta })
}

// P2 #8: 文件大小和深度限制
const MAX_SINGLE_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
const MAX_TOTAL_CONTENT_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_RECURSION_DEPTH: u32 = 5;

/// Recursively read all .md files in a skill directory and concatenate them.
pub(super) fn collect_skill_content(dir: &Path) -> String {
    let mut content = String::new();
    let Ok(entries) = collect_markdown_files(dir, 0) else {
        return content;
    };
    let mut total_bytes: u64 = 0;
    for path in entries {
        // 检查文件大小
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > MAX_SINGLE_FILE_SIZE {
                content.push_str(&format!(
                    "\n\n<!-- [SKIPPED] {} exceeds size limit ({} bytes) -->\n",
                    path.display(),
                    meta.len()
                ));
                continue;
            }
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            total_bytes += text.len() as u64;
            if total_bytes > MAX_TOTAL_CONTENT_SIZE {
                content.push_str("\n\n<!-- [TRUNCATED] Total content exceeds 10MB limit -->\n");
                break;
            }
            if !content.is_empty() {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&text);
        }
    }
    content
}

/// Recursively collect all .md files under a directory, sorted by name.
pub(crate) fn collect_markdown_files(dir: &Path, depth: u32) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() || depth > MAX_RECURSION_DEPTH {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_markdown_files(&path, depth + 1)?);
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[tauri::command]
pub async fn toggle_skill(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<(), ErrorResponse> {
    axagent_dao::repo::skill::set_skill_enabled(state.harness.db(), &name, enabled)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "skill-state-changed",
        serde_json::json!({
            "skillName": name,
            "enabled": enabled,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn install_skill(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source: String,
    target: Option<String>,
    scenarios: Option<Vec<String>>,
) -> Result<String, String> {
    let target_dir = match target.as_deref() {
        Some("claude") => home_dir().join(".claude").join("skills"),
        Some("agents") => home_dir().join(".agents").join("skills"),
        Some("trae") => home_dir().join(".trae").join("skills"),
        Some("codebuddy") => home_dir().join(".codebuddy").join("skills"),
        Some("workbuddy") => home_dir().join(".workbuddy").join("skills"),
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

    // 检查依赖是否满足
    check_skill_dependencies(&skill_target, &target_dir)?;

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

    state.trajectory_storage.save_skill(&skill).await.map_err(|e| e.to_string())?;

    let _ = app.emit(
        "skill-state-changed",
        serde_json::json!({
            "skillName": &skill_name,
            "action": "installed",
        }),
    );

    Ok(skill_name)
}

/// 检查 skill-manifest.json 中的 dependencies 是否已安装
fn check_skill_dependencies(skill_dir: &Path, target_dir: &Path) -> Result<(), String> {
    let manifest_path = skill_dir.join("skill-manifest.json");
    if !manifest_path.exists() {
        return Ok(()); // 无清单文件，跳过检查
    }
    let contents = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
        ErrorResponse::new(skill_err::MANIFEST_PARSE_FAILED)
            .with_detail(format!("解析 skill-manifest.json 失败: {}", e))
    })?;

    let deps = match manifest.get("dependencies") {
        Some(serde_json::Value::Object(deps)) => deps,
        _ => return Ok(()), // 无依赖声明
    };

    for dep_name in deps.keys() {
        let dep_dir = target_dir.join(dep_name);
        if !dep_dir.exists() || !dep_dir.is_dir() {
            Err(ErrorResponse::new(skill_err::DEPENDENCY_NOT_FOUND)
                .with_detail(format!(
                    "依赖未满足: Skill '{}' 需要 '{}'，但未在目标目录中找到",
                    skill_dir.file_name().unwrap_or_default().to_string_lossy(),
                    dep_name
                ))
                .with_param(
                    "skill",
                    skill_dir.file_name().unwrap_or_default().to_string_lossy().to_string(),
                )
                .with_param("dependency", dep_name.to_string()))?;
        }
    }
    Ok(())
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
                return scenarios.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
        }
    }
    vec![]
}

pub(super) fn load_plugin_version(skill_dir: &Path) -> String {
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
    if repo.contains('/') || repo.contains('\\') || repo.contains("..") {
        return Err(
            "Invalid repository name: must not contain path separators or traversal".to_string()
        );
    }
    let git_url = format!("https://github.com/{}/{}.git", owner, repo);
    let skill_target = target_dir.join(repo);

    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    let mut git_cmd = axagent_kit::utils::cmd("git");
    let git_available =
        git_cmd.arg("--version").output().map(|o| o.status.success()).unwrap_or(false);

    if git_available {
        let output = axagent_kit::utils::cmd("git")
            .args(["clone", "--depth", "1", "--", &git_url, skill_target.to_str().unwrap_or("")])
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if output.status.success() {
            let commit = get_git_commit(&skill_target).unwrap_or_else(|| "unknown".to_string());
            // 清理 .git 目录，避免嵌套 git 仓库问题
            let git_dir = skill_target.join(".git");
            if git_dir.exists() {
                let _ = std::fs::remove_dir_all(&git_dir);
            }
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
    if repo.contains('/') || repo.contains('\\') || repo.contains("..") {
        return Err(
            "Invalid repository name: must not contain path separators or traversal".to_string()
        );
    }
    let url = format!("https://api.github.com/repos/{}/{}/zipball", owner, repo);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
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

    let commit = top_dir.split('-').next_back().unwrap_or("unknown").to_string();

    let dest_canonical = temp_dir
        .path()
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize temp dir: {}", e))?;
    // 阶段一：使用 enclosed_name() 验证所有 entry
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Failed to read zip entry: {}", e))?;

        // enclosed_name(): 非 UTF-8 或路径遍历路径时返回 None
        let entry_path = entry
            .enclosed_name()
            .ok_or_else(|| {
                format!("Invalid zip entry name (non-UTF-8 or path traversal): entry {}", i)
            })?
            .to_path_buf();

        let resolved = temp_dir.path().join(&entry_path);
        let canonical = resolved
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize zip entry path: {}", e))?;
        if !canonical.starts_with(&dest_canonical) {
            return Err("Path traversal detected in zip".into());
        }
    }

    // 阶段二：解压
    archive.extract(temp_dir.path()).map_err(|e| format!("Failed to extract: {}", e))?;

    // 阶段三：解压后二次验证（防止 TOCTOU）
    for i in 0..archive.len() {
        let entry =
            archive.by_index(i).map_err(|e| format!("Failed to re-read zip entry: {}", e))?;
        let entry_path = entry.enclosed_name().ok_or_else(|| {
            format!("Invalid zip entry name during post-extract check: entry {}", i)
        })?;
        let resolved = temp_dir.path().join(&entry_path);
        if resolved.exists() {
            let canonical = resolved
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize extracted file: {}", e))?;
            if !canonical.starts_with(&dest_canonical) {
                // 回滚已解压文件
                let _ = std::fs::remove_dir_all(temp_dir.path());
                return Err("Post-extract path traversal violation detected".into());
            }
        }
    }

    let extracted = temp_dir.path().join(&top_dir);
    let skill_target = target_dir.join(repo);

    if skill_target.exists() {
        std::fs::remove_dir_all(&skill_target).map_err(|e| e.to_string())?;
    }

    copy_dir_recursive(&extracted, &skill_target)?;
    save_skill_manifest(&skill_target, "github", &format!("{}/{}", owner, repo), "main", &commit)?;

    Ok((repo.to_string(), commit))
}

fn get_git_commit(repo_path: &Path) -> Option<String> {
    let output = axagent_kit::utils::cmd("git")
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

    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| {
        ErrorResponse::new(skill_err::SERIALIZE_FAILED).with_detail(format!("JSON 序列化失败: {e}"))
    })?;
    std::fs::write(&manifest_path, manifest_str).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub installed_at: String,
    pub commit: String,
}

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
        return Err(ErrorResponse::err(skill_op_err::ROLLBACK_NOT_SUPPORTED));
    }

    let parts: Vec<&str> = source_ref.split('/').collect();
    if parts.len() != 2 {
        return Err(ErrorResponse::err(skill_op_err::INVALID_FORMAT));
    }

    let (owner, repo) = (parts[0], parts[1]);
    let git_url = format!("https://github.com/{}/{}.git", owner, repo);

    std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;

    let output = axagent_kit::utils::cmd("git")
        .args(["clone", "--depth", "50", &git_url, skill_dir.to_str().unwrap_or("")])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err(format!("Git clone failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let checkout_output = axagent_kit::utils::cmd("git")
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

    Ok(format!("Rolled back {} to version {}", skill_name, target_version))
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
    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| {
        ErrorResponse::new(skill_err::SERIALIZE_FAILED).with_detail(format!("JSON 序列化失败: {e}"))
    })?;
    std::fs::write(&manifest_path, manifest_str).map_err(|e| e.to_string())?;

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

pub(super) fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Skill name must not be empty".to_string());
    }
    // P2 #10: 长度限制
    if name.len() > 64 {
        return Err("Skill name must not exceed 64 characters".to_string());
    }
    // 禁止路径分隔符和遍历字符
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Skill name must not contain path separators or traversal".to_string());
    }
    // 禁止空字节
    if name.contains('\0') {
        return Err("Skill name must not contain null bytes".to_string());
    }
    // 禁止 Windows 盘符
    if name.len() >= 2 {
        let b = name.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err("Skill name must not contain Windows drive letter".to_string());
        }
    }
    // P2 #10: Windows 保留名称黑名单（不区分大小写）
    const WINDOWS_RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let upper = name.to_ascii_uppercase();
    if WINDOWS_RESERVED
        .iter()
        .any(|r| upper.as_str() == *r || upper.starts_with(&format!("{}.", r)))
    {
        return Err(format!("Skill name '{}' is a Windows reserved name", name));
    }
    // P2 #10: 仅允许字母、数字、连字符、下划线
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(
            "Skill name must only contain alphanumeric characters, hyphens, and underscores"
                .to_string(),
        );
    }
    Ok(())
}

fn ensure_path_under_base(path: &Path, base: &Path) -> Result<(), String> {
    let canonical_path =
        path.canonicalize().map_err(|e| format!("Failed to canonicalize path: {}", e))?;
    let canonical_base =
        base.canonicalize().map_err(|e| format!("Failed to canonicalize base: {}", e))?;
    if !canonical_path.starts_with(&canonical_base) {
        return Err("Path traversal detected".to_string());
    }
    Ok(())
}

/// 卸载结果：记录每个目录的删除状况
#[derive(Debug, Clone, serde::Serialize)]
pub struct UninstallResult {
    pub dir: String,
    pub status: String, // "deleted" | "not_found" | "error"
    pub detail: Option<String>,
}

#[tauri::command]
pub async fn uninstall_skill(
    app: tauri::AppHandle,
    name: String,
) -> Result<Vec<UninstallResult>, ErrorResponse> {
    validate_skill_name(&name)?;
    let home = home_dir();
    let search_dirs = [
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
        home.join(".trae").join("skills"),
        home.join(".codebuddy").join("skills"),
        home.join(".workbuddy").join("skills"),
    ];

    let mut results: Vec<UninstallResult> = Vec::new();
    let mut any_deleted = false;

    for parent in &search_dirs {
        let skill_dir = parent.join(&name);
        let dir_label = parent.to_string_lossy().to_string();
        if skill_dir.exists() && skill_dir.is_dir() {
            match ensure_path_under_base(&skill_dir, parent)
                .and_then(|_| std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string()))
            {
                Ok(()) => {
                    results.push(UninstallResult {
                        dir: dir_label,
                        status: "deleted".to_string(),
                        detail: None,
                    });
                    any_deleted = true;
                },
                Err(e) => {
                    results.push(UninstallResult {
                        dir: dir_label,
                        status: "error".to_string(),
                        detail: Some(e),
                    });
                },
            }
        } else {
            results.push(UninstallResult {
                dir: dir_label,
                status: "not_found".to_string(),
                detail: None,
            });
        }
    }

    if any_deleted {
        let _ = app.emit(
            "skill-state-changed",
            serde_json::json!({
                "skillName": &name,
                "action": "uninstalled",
            }),
        );
    }

    if !any_deleted {
        return Err(ErrorResponse::new(skill_err::NOT_FOUND).with_param("name".to_string(), name));
    }

    Ok(results)
}

#[tauri::command]
pub async fn uninstall_skill_group(group: String) -> Result<(), String> {
    validate_skill_name(&group)?;
    let home = home_dir();
    let search_dirs = [
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];

    for parent in &search_dirs {
        let group_dir = parent.join(&group);
        if group_dir.exists() && group_dir.is_dir() {
            ensure_path_under_base(&group_dir, parent)?;
            std::fs::remove_dir_all(&group_dir).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err(format!("Skill group '{}' not found", group))
}
