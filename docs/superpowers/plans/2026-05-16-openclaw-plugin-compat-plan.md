# OpenClaw 插件生态全面兼容 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 AxAgent 全面兼容 OpenClaw 插件生态，支持 npm 包安装、mcpServers/skills/agents 全部清单字段，接通前后端安装链路。

**Architecture:** 新增 `axagent-npm` crate（纯 Rust npm registry 客户端），改造 `axagent-plugins` crate（扩展安装源、清单字段、MCP/技能/Agent 生命周期），接通 Tauri 命令和前端 UI。

**Tech Stack:** Rust 2021 (reqwest, flate2, tar, serde), TypeScript/React 19 (Ant Design 6, Zustand 5), Tauri 2

---

## 文件结构

| 操作 | 文件 | 职责 |
|------|------|------|
| **新建** | `src-tauri/crates/npm/Cargo.toml` | npm crate 元数据与依赖 |
| **新建** | `src-tauri/crates/npm/src/lib.rs` | crate 入口，re-export |
| **新建** | `src-tauri/crates/npm/src/types.rs` | PackageInfo, VersionInfo, DistInfo, NpmError |
| **新建** | `src-tauri/crates/npm/src/registry.rs` | NpmRegistry 结构体 + API 调用 |
| **新建** | `src-tauri/crates/npm/src/tarball.rs` | tarball 流式下载与解压 |
| **修改** | `src-tauri/Cargo.toml:1-3` | workspace members 增加 npm crate |
| **修改** | `src-tauri/crates/plugins/Cargo.toml` | 新增依赖 axagent-npm, tokio, tempfile |
| **修改** | `src-tauri/crates/plugins/src/lib.rs` | PluginInstallSource + 清单兼容 + materialize_source |
| **新建** | `src-tauri/crates/plugins/src/mcp_launcher.rs` | McpLauncher — MCP 子进程生命周期 |
| **新建** | `src-tauri/crates/plugins/src/skill_installer.rs` | SkillInstaller — 技能文件部署 |
| **修改** | `src-tauri/crates/plugins/src/agent_provider.rs` | 新增 register_plugin_agents + 内部类型 |
| **修改** | `src-tauri/src/app_state.rs` | 注入 PluginManager 字段 |
| **修改** | `src-tauri/src/commands/plugin.rs` | 空桩 → 真实调用 |
| **修改** | `src-tauri/src/lib.rs` | generate_handler![] 增加新命令 |
| **修改** | `src/components/chat/PluginMarketplace.tsx` | 搜索安装栏 + 确认弹窗 + 详情 |
| **修改** | `src/types/index.ts` | PluginSummaryDto 等前端类型 |
| **修改** | `src/i18n/locales/*.json` (11 files) | 新增 i18n key |

---

### Task 1: 创建 axagent-npm crate — 基础骨架

**Files:**
- Create: `src-tauri/crates/npm/Cargo.toml`
- Create: `src-tauri/crates/npm/src/lib.rs`
- Create: `src-tauri/crates/npm/src/types.rs`
- Modify: `src-tauri/Cargo.toml:1`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "axagent-npm"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
flate2 = "1.0"
tar = "0.4"
tempfile = "3"
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: 将新 crate 加入 workspace**

Edit `src-tauri/Cargo.toml` line 1:

```diff
- members = [".", "crates/core", "crates/providers", ..., "crates/rt-theme"]
+ members = [".", "crates/core", "crates/providers", ..., "crates/rt-theme", "crates/npm"]
```

- [ ] **Step 3: 创建 types.rs**

```rust
// src-tauri/crates/npm/src/types.rs
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: DistTags,
    pub versions: HashMap<String, VersionInfo>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DistTags {
    pub latest: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub dist: DistInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub shasum: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NpmError {
    #[error("package not found: {0}")]
    NotFound(String),
    #[error("version not found: {0}@{1}")]
    VersionNotFound(String, String),
    #[error("registry request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("tarball extraction failed: {0}")]
    ExtractFailed(#[from] std::io::Error),
}
```

- [ ] **Step 4: 创建 lib.rs**

```rust
// src-tauri/crates/npm/src/lib.rs
pub mod registry;
pub mod tarball;
pub mod types;

pub use registry::NpmRegistry;
pub use types::{DistInfo, NpmError, PackageInfo, VersionInfo};
```

- [ ] **Step 5: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-npm
```

Expected: 编译成功（registry.rs 和 tarball.rs 为空模块，会提示未使用，可接受）。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/npm/ src-tauri/Cargo.toml
git commit -m "feat: 新建 axagent-npm crate，定义 types 和依赖
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 2: 实现 NpmRegistry — parse & fetch

**Files:**
- Create: `src-tauri/crates/npm/src/registry.rs`

- [ ] **Step 1: 实现 parse_package_spec 和 NpmRegistry 结构体**

```rust
// src-tauri/crates/npm/src/registry.rs
use std::path::{Path, PathBuf};

use tracing::info;

use crate::tarball;
use crate::types::{DistInfo, NpmError, PackageInfo, VersionInfo};

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

pub struct NpmRegistry {
    registry_url: String,
    client: reqwest::Client,
}

impl NpmRegistry {
    pub fn new() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY.to_string(),
            client: reqwest::Client::builder()
                .user_agent("axagent-npm/0.1.0")
                .build()
                .expect("reqwest client build"),
        }
    }

    /// 解析包名: "@scope/name@version" → ("@scope/name", Option<"version">")
    /// 也支持无 scope: "plain-package@1.0.0"
    pub fn parse_package_spec(spec: &str) -> (&str, Option<&str>) {
        // 找最后一个 @ 后面的版本号（排除 scoped package 的 @scope 前缀）
        if let Some(at_pos) = spec.rfind('@') {
            if at_pos > 0 {
                let name = &spec[..at_pos];
                let version = &spec[at_pos + 1..];
                if !version.is_empty() && !version.contains('/') {
                    return (name, Some(version));
                }
            }
        }
        (spec, None)
    }

    /// 将 npm 包名转换为 registry URL path
    /// @scope/name → @scope%2Fname
    fn package_path(name: &str) -> String {
        name.replace('/', "%2F")
    }

    /// GET /<package> → PackageInfo
    pub async fn fetch_package_info(&self, name: &str) -> Result<PackageInfo, NpmError> {
        let path = Self::package_path(name);
        let url = format!("{}/{}", self.registry_url, path);
        info!("npm: fetching package info from {}", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(NpmError::NotFound(name.to_string()));
        }

        let info: PackageInfo = response.error_for_status()?.json().await?;
        Ok(info)
    }

    /// 解析版本 latest 或 semver
    pub fn resolve_version<'a>(
        info: &'a PackageInfo,
        version: Option<&str>,
    ) -> Result<&'a VersionInfo, NpmError> {
        let version_str = version.unwrap_or("latest");
        let semver = if version_str == "latest" {
            &info.dist_tags.latest
        } else {
            version_str
        };
        info.versions.get(semver).ok_or_else(|| {
            NpmError::VersionNotFound(info.name.clone(), semver.to_string())
        })
    }

    /// 下载 tarball 流式解压到 dest，返回插件根目录
    pub async fn download_and_extract(
        &self,
        dist: &DistInfo,
        dest: &Path,
    ) -> Result<PathBuf, NpmError> {
        info!("npm: downloading tarball from {}", dist.tarball);
        let response = self
            .client
            .get(&dist.tarball)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;
        tarball::extract_tarball(&bytes, dest)?;
        // npm 包解压后通常有一层外层的 package/ 目录，需要检测并返回实际根目录
        Ok(tarball::detect_package_root(dest)?.unwrap_or_else(|| dest.to_path_buf()))
    }
}

impl Default for NpmRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-npm
```

Expected: 编译成功（tarball 模块尚未实现，会在链接时报错）。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/npm/src/
git commit -m "feat: 实现 NpmRegistry — parse/fetch/resolve API
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 3: 实现 tarball 解压模块

**Files:**
- Create: `src-tauri/crates/npm/src/tarball.rs`

- [ ] **Step 1: 实现 extract_tarball 和 detect_package_root**

```rust
// src-tauri/crates/npm/src/tarball.rs
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

/// 将 .tgz 字节流解压到 dest 目录
pub fn extract_tarball(data: &[u8], dest: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dest)?;
    let cursor = Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

/// npm 包解压后通常有一层外层的 package/ 目录
/// 读取 dest 下的顶层目录：
/// - 如果只有 1 个目录，返回该目录路径
/// - 如果有多个或没有，返回 None (即 dest 本身就是根)
pub fn detect_package_root(dest: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    let entries: Vec<_> = fs::read_dir(dest)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    if entries.len() == 1 {
        let single_dir = entries.into_iter().next().unwrap().path();
        // 检查该目录下是否有 plugin.json 或 .claude-plugin/ 或 SKILL.md
        if single_dir.join("plugin.json").exists()
            || single_dir.join(".claude-plugin").exists()
            || single_dir.join("SKILL.md").exists()
            || single_dir.join("package.json").exists()
        {
            return Ok(Some(single_dir));
        }
    }
    Ok(None)
}
```

- [ ] **Step 2: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-npm
```

Expected: crate 编译成功，无错误。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/npm/src/tarball.rs
git commit -m "feat: 实现 tarball 流式解压与 npm 包根目录检测
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 4: npm crate 单元测试

**Files:**
- Modify: `src-tauri/crates/npm/src/registry.rs` (追加测试模块)

- [ ] **Step 1: 添加 parse_package_spec 测试**

在 `src-tauri/crates/npm/src/registry.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scoped_package_latest() {
        let (name, version) = NpmRegistry::parse_package_spec("@clawd/ths");
        assert_eq!(name, "@clawd/ths");
        assert_eq!(version, None);
    }

    #[test]
    fn parse_scoped_package_with_version() {
        let (name, version) = NpmRegistry::parse_package_spec("@clawd/stock@1.2.0");
        assert_eq!(name, "@clawd/stock");
        assert_eq!(version, Some("1.2.0"));
    }

    #[test]
    fn parse_plain_package_latest() {
        let (name, version) = NpmRegistry::parse_package_spec("my-plugin");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, None);
    }

    #[test]
    fn parse_plain_package_with_version() {
        let (name, version) = NpmRegistry::parse_package_spec("my-plugin@2.0.0");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, Some("2.0.0"));
    }

    #[test]
    fn parse_scoped_package_with_semver_tag() {
        let (name, version) = NpmRegistry::parse_package_spec("@scope/pkg@beta");
        assert_eq!(name, "@scope/pkg");
        assert_eq!(version, Some("beta"));
    }

    #[test]
    fn package_path_scoped() {
        assert_eq!(NpmRegistry::package_path("@clawd/ths"), "@clawd%2Fths");
    }

    #[test]
    fn package_path_plain() {
        assert_eq!(NpmRegistry::package_path("lodash"), "lodash");
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-npm
```

Expected: 7 tests passed.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/npm/src/registry.rs
git commit -m "test: npm crate parse_package_spec + package_path 单元测试
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 5: plugins crate — 扩展 PluginInstallSource

**Files:**
- Modify: `src-tauri/crates/plugins/Cargo.toml`
- Modify: `src-tauri/crates/plugins/src/lib.rs`

- [ ] **Step 1: 更新 plugins Cargo.toml 依赖**

```diff
  [dependencies]
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
+ axagent-npm = { path = "../npm" }
+ tokio = { workspace = true }
+ tempfile = { version = "3" }
```

- [ ] **Step 2: 在 lib.rs 顶部增加 axagent-npm 的 use**

在 `src-tauri/crates/plugins/src/lib.rs` 顶部 existing `use` 块之后追加：

```rust
use axagent_npm::{NpmError, NpmRegistry};
```

- [ ] **Step 3: 扩展 PluginInstallSource 枚举**

在 `lib.rs:358`，在 `GitUrl` 变体后追加：

```rust
    NpmPackage {
        name: String,
        version: Option<String>,
    },
```

完整枚举：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginInstallSource {
    LocalPath { path: PathBuf },
    GitUrl { url: String },
    NpmPackage {
        name: String,
        version: Option<String>,
    },
}
```

- [ ] **Step 4: 扩展 parse_install_source 函数**

在 `lib.rs:2228` `parse_install_source()` 函数开头增加 npm 检测：

```rust
fn parse_install_source(source: &str) -> Result<PluginInstallSource, PluginError> {
    // npm 包: @scope/name 或以 @version 结尾（包含 @ 符号且非 Git URL）
    if (source.starts_with('@') || source.contains("@") && !looks_like_url_or_git(source))
        && looks_like_npm_spec(source)
    {
        let (name, version) = NpmRegistry::parse_package_spec(source);
        return Ok(PluginInstallSource::NpmPackage {
            name: name.to_string(),
            version: version.map(|v| v.to_string()),
        });
    }
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || Path::new(source)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("git"))
    {
        Ok(PluginInstallSource::GitUrl {
            url: source.to_string(),
        })
    } else {
        Ok(PluginInstallSource::LocalPath {
            path: resolve_local_source(source)?,
        })
    }
}

fn looks_like_npm_spec(source: &str) -> bool {
    // 不以 http/https/git@ 开头，也不像本地路径
    !source.starts_with("http://")
        && !source.starts_with("https://")
        && !source.starts_with("git@")
        && !source.starts_with('/')
        && !source.starts_with('.')
        && !source.starts_with('\\')
        && !source.contains("://")
}

fn looks_like_url_or_git(source: &str) -> bool {
    source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || source.ends_with(".git")
}
```

- [ ] **Step 5: 扩展 materialize_source 函数**

在 `lib.rs:2251` `materialize_source()` 的 match 中增加 NpmPackage 分支：

```rust
fn materialize_source(
    source: &PluginInstallSource,
    temp_root: &Path,
) -> Result<PathBuf, PluginError> {
    fs::create_dir_all(temp_root)?;
    match source {
        PluginInstallSource::LocalPath { path } => Ok(path.clone()),
        PluginInstallSource::GitUrl { url } => {
            // ... 现有 git clone 逻辑不变 ...
        },
        PluginInstallSource::NpmPackage { name, version } => {
            let registry = NpmRegistry::new();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            let info = runtime.block_on(registry.fetch_package_info(name))?;
            let ver = registry.resolve_version(&info, version.as_deref())?;
            let dest = temp_root.join(format!("npm-{}", sanitize_plugin_id(name)));
            runtime
                .block_on(registry.download_and_extract(&ver.dist, &dest))
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            Ok(dest)
        },
    }
}
```

- [ ] **Step 6: 扩展 describe_install_source 函数**

在 `lib.rs:2311` 增加 NpmPackage 分支：

```rust
        PluginInstallSource::NpmPackage { name, version } => match version {
            Some(version) => format!("{name}@{version}"),
            None => name.clone(),
        },
```

- [ ] **Step 7: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-plugins
```

Expected: 编译成功。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/crates/plugins/Cargo.toml src-tauri/crates/plugins/src/lib.rs
git commit -m "feat: PluginInstallSource 新增 NpmPackage，支持 npm 包安装
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 6: 清单兼容 — 放通 skills / mcpServers / agents

**Files:**
- Modify: `src-tauri/crates/plugins/src/lib.rs`

- [ ] **Step 1: 新增清单内部类型定义**

在 `lib.rs` 中 `PluginCommandManifest` 定义之后、`RawPluginManifest` 之前追加：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginMcpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginSkillEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawPluginAgentDef {
    #[serde(rename = "agentType")]
    pub agent_type: String,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(rename = "disallowedTools", default)]
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub background: bool,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
}
```

- [ ] **Step 2: 更新 RawPluginManifest 结构体**

在 `RawPluginManifest` 中增加 3 个字段（在 `scenarios` 字段之后）：

```rust
    #[serde(default, alias = "mcpServers")]
    pub mcp_servers: Vec<RawPluginMcpServer>,
    #[serde(default)]
    pub skills: Vec<RawPluginSkillEntry>,
    #[serde(default)]
    pub agents: Vec<RawPluginAgentDef>,
```

- [ ] **Step 3: 删除 detect_claude_code_manifest_contract_gaps 中的拒绝逻辑**

删除对 `skills`、`mcpServers`、`agents` 三个字段的拒绝检查。保留对 commands 字符串数组形式和未知 Hook 的检查。

修改前（删除整个 for 循环中的这三项）：

```rust
fn detect_claude_code_manifest_contract_gaps(
    raw_manifest: &Value,
) -> Vec<PluginManifestValidationError> {
    let Some(root) = raw_manifest.as_object() else {
        return Vec::new();
    };

    let mut errors = Vec::new();

    // 删除以下 for 循环：skills, mcpServers, agents
    // 保留：commands 字符串数组检查
    // 保留：非白名单 Hook 检查

    if root
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| commands.iter().any(Value::is_string))
    {
        errors.push(PluginManifestValidationError::UnsupportedManifestContract {
            detail: "plugin manifest field `commands` uses Claude Code-style directory globs; `claw` slash dispatch is still built-in and does not load plugin slash command markdown files.".to_string(),
        });
    }

    if let Some(hooks) = root.get("hooks").and_then(Value::as_object) {
        for hook_name in hooks.keys() {
            if !matches!(hook_name.as_str(), "PreToolUse" | "PostToolUse" | "PostToolUseFailure") {
                errors.push(PluginManifestValidationError::UnsupportedManifestContract {
                    detail: format!(
                        "plugin hook `{hook_name}` uses the Claude Code lifecycle contract; `claw` plugins currently support only PreToolUse, PostToolUse, and PostToolUseFailure."
                    ),
                });
            }
        }
    }

    errors
}
```

- [ ] **Step 4: 更新 PluginManifest 结构体**

在 `PluginManifest` 的 `scenarios` 字段之后追加：

```rust
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    pub agents: Vec<PluginAgentDefInternal>,
```

- [ ] **Step 5: 定义 PluginMcpServer 和 PluginSkillEntry**

在 `PluginManifest` 定义之后追加：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSkillEntry {
    pub name: String,
    pub path: String,
}

/// 插件内部 Agent 定义（反序列化后转换为 agent_provider::PluginAgentDef）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAgentDefInternal {
    pub agent_type: String,
    pub description: String,
    pub tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub background: bool,
    pub system_prompt: Option<String>,
}
```

- [ ] **Step 6: 更新 SKILL.md 解析 — 保持空数组默认值**

在 `load_manifest_from_skill_md()` 的 `PluginManifest` 构造末尾追加：

```rust
        // ... existing fields ...
        scenarios: Vec::new(),
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        agents: Vec::new(),
    };
```

- [ ] **Step 7: 更新 build_plugin_manifest — 新字段传递**

在 `build_plugin_manifest()` 的 `Ok(PluginManifest { ... })` 中追加：

```rust
    Ok(PluginManifest {
        name: raw.name,
        version: raw.version,
        description: raw.description,
        permissions,
        default_enabled: raw.default_enabled,
        hooks: raw.hooks,
        lifecycle: raw.lifecycle,
        tools,
        commands,
        scenarios: raw.scenarios,
        mcp_servers: raw.mcp_servers.into_iter().map(|r| PluginMcpServer {
            name: r.name,
            command: r.command,
            args: r.args,
            env: r.env,
            cwd: r.cwd,
        }).collect(),
        skills: raw.skills.into_iter().map(|r| PluginSkillEntry {
            name: r.name,
            path: r.path,
        }).collect(),
        agents: raw.agents.into_iter().map(|r| PluginAgentDefInternal {
            agent_type: r.agent_type,
            description: r.description,
            tools: r.tools,
            disallowed_tools: r.disallowed_tools,
            model: r.model,
            background: r.background,
            system_prompt: r.system_prompt,
        }).collect(),
    })
```

- [ ] **Step 8: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-plugins
```

Expected: 编译成功。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/crates/plugins/src/lib.rs
git commit -m "feat: 放通 mcpServers/skills/agents 清单字段，完全兼容 OpenClaw 插件格式
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 7: MCP 启动器 — 子进程生命周期管理

**Files:**
- Create: `src-tauri/crates/plugins/src/mcp_launcher.rs`
- Modify: `src-tauri/crates/plugins/src/lib.rs`

- [ ] **Step 1: 创建 mcp_launcher.rs**

```rust
// src-tauri/crates/plugins/src/mcp_launcher.rs
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use tracing::{error, info, warn};

use crate::PluginMcpServer;

#[derive(Debug)]
struct RunningMcpProcess {
    child: Child,
    server_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum McpLaunchError {
    #[error("MCP server `{server}` failed to start: {source}")]
    SpawnFailed {
        server: String,
        source: std::io::Error,
    },
    #[error("MCP server `{0}` exited immediately after start")]
    ImmediateExit(String),
}

pub struct McpLauncher {
    running: HashMap<String, Vec<RunningMcpProcess>>,
}

impl McpLauncher {
    pub fn new() -> Self {
        Self {
            running: HashMap::new(),
        }
    }

    /// 启动插件声明的所有 MCP 服务
    pub fn start_plugin_mcps(
        &mut self,
        plugin_id: &str,
        servers: &[PluginMcpServer],
        plugin_root: &Path,
    ) -> Result<(), McpLaunchError> {
        let mut processes = Vec::new();
        for server in servers {
            let proc = self.spawn_server(plugin_id, server, plugin_root)?;
            processes.push(proc);
        }
        self.running.insert(plugin_id.to_string(), processes);
        Ok(())
    }

    /// 停止插件所有 MCP 服务
    pub fn stop_plugin_mcps(&mut self, plugin_id: &str) {
        if let Some(processes) = self.running.remove(plugin_id) {
            for mut proc in processes {
                info!(
                    "mcp: stopping server `{}` for plugin `{}`",
                    proc.server_name, plugin_id
                );
                let _ = proc.child.kill();
                let _ = proc.child.wait();
            }
        }
    }

    fn spawn_server(
        &self,
        plugin_id: &str,
        server: &PluginMcpServer,
        plugin_root: &Path,
    ) -> Result<RunningMcpProcess, McpLaunchError> {
        info!(
            "mcp: starting server `{}` for plugin `{}`: {} {}",
            server.name,
            plugin_id,
            server.command,
            server.args.join(" ")
        );

        let mut cmd = Command::new(&server.command);
        cmd.args(&server.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env("CLAWD_PLUGIN_ID", plugin_id);
        cmd.env("CLAWD_PLUGIN_ROOT", plugin_root);
        for (k, v) in &server.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &server.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(plugin_root);
        }

        let mut child = cmd.spawn().map_err(|source| McpLaunchError::SpawnFailed {
            server: server.name.clone(),
            source,
        })?;

        // 等待短暂时间确认进程没有立即崩溃
        std::thread::sleep(Duration::from_secs(2));
        match child.try_wait() {
            Ok(Some(status)) => {
                warn!(
                    "mcp: server `{}` for plugin `{}` exited immediately with {:?}",
                    server.name, plugin_id, status
                );
                Err(McpLaunchError::ImmediateExit(server.name.clone()))
            }
            Ok(None) => {
                info!(
                    "mcp: server `{}` for plugin `{}` running (pid {})",
                    server.name,
                    plugin_id,
                    child.id()
                );
                Ok(RunningMcpProcess {
                    child,
                    server_name: server.name.clone(),
                })
            }
            Err(e) => {
                // try_wait 失败，进程可能已崩溃
                Err(McpLaunchError::SpawnFailed {
                    server: server.name.clone(),
                    source: e,
                })
            }
        }
    }
}

impl Drop for McpLauncher {
    fn drop(&mut self) {
        let plugin_ids: Vec<String> = self.running.keys().cloned().collect();
        for plugin_id in plugin_ids {
            self.stop_plugin_mcps(&plugin_id);
        }
    }
}

impl Default for McpLauncher {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 在 lib.rs 中注册模块并公开类型**

在 `src-tauri/crates/plugins/src/lib.rs` 顶部追加模块声明：

```rust
pub mod mcp_launcher;
```

在 `lib.rs` 的 pub use 区域追加：

```rust
pub use mcp_launcher::{McpLaunchError, McpLauncher};
```

- [ ] **Step 3: PluginManager 集成 McpLauncher**

修改 `PluginManager` 结构体：

```rust
pub struct PluginManager {
    config: PluginManagerConfig,
    mcp_launcher: McpLauncher,
}
```

修改 `PluginManager::new`：

```rust
    pub fn new(config: PluginManagerConfig) -> Self {
        Self {
            config,
            mcp_launcher: McpLauncher::new(),
        }
    }
```

在 `PluginManager::enable` 末尾增加 MCP 启动：

```rust
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        self.ensure_known_plugin(plugin_id)?;
        self.write_enabled_state(plugin_id, Some(true))?;
        self.config
            .enabled_plugins
            .insert(plugin_id.to_string(), true);

        // 启动 MCP 服务
        let registry = self.load_registry()?;
        if let Some(record) = registry.plugins.get(plugin_id) {
            let manifest = load_plugin_from_directory(&record.install_path)?;
            if !manifest.mcp_servers.is_empty() {
                self.mcp_launcher
                    .start_plugin_mcps(plugin_id, &manifest.mcp_servers, &record.install_path)
                    .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
            }
        }
        Ok(())
    }
```

在 `PluginManager::disable` 开头增加 MCP 停止：

```rust
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        // 先停 MCP
        self.mcp_launcher.stop_plugin_mcps(plugin_id);

        self.ensure_known_plugin(plugin_id)?;
        // ... 其余现有逻辑 ...
    }
```

- [ ] **Step 4: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-plugins
```

Expected: 编译成功。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/plugins/src/mcp_launcher.rs src-tauri/crates/plugins/src/lib.rs
git commit -m "feat: McpLauncher — 插件 MCP 服务自动启动/停止生命周期
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 8: 技能安装器

**Files:**
- Create: `src-tauri/crates/plugins/src/skill_installer.rs`
- Modify: `src-tauri/crates/plugins/src/lib.rs`

- [ ] **Step 1: 创建 skill_installer.rs**

```rust
// src-tauri/crates/plugins/src/skill_installer.rs
use std::fs;
use std::path::{Path, PathBuf};

use tracing::info;

use crate::PluginSkillEntry;

pub struct SkillInstaller {
    skills_root: PathBuf,
}

impl SkillInstaller {
    pub fn new(skills_root: impl Into<PathBuf>) -> Self {
        Self {
            skills_root: skills_root.into(),
        }
    }

    /// 将插件 skills 部署到系统技能目录
    pub fn install_plugin_skills(
        &self,
        plugin_id: &str,
        skills: &[PluginSkillEntry],
        plugin_root: &Path,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let plugin_skill_dir = self.skills_root.join(sanitize_for_path(plugin_id));
        fs::create_dir_all(&plugin_skill_dir)?;
        let mut installed = Vec::new();
        for skill in skills {
            let src = plugin_root.join(&skill.path);
            let dest = plugin_skill_dir.join(&skill.path);
            if src.exists() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dest)?;
                info!(
                    "skill: installed `{}` from plugin `{}` to `{}`",
                    skill.name,
                    plugin_id,
                    dest.display()
                );
                installed.push(dest);
            }
        }
        Ok(installed)
    }

    /// 卸载插件 skills
    pub fn remove_plugin_skills(&self, plugin_id: &str) -> Result<(), std::io::Error> {
        let plugin_skill_dir = self.skills_root.join(sanitize_for_path(plugin_id));
        if plugin_skill_dir.exists() {
            fs::remove_dir_all(&plugin_skill_dir)?;
            info!("skill: removed skills for plugin `{}`", plugin_id);
        }
        Ok(())
    }
}

/// 将 plugin_id (如 "@clawd/ths@external") 转换为安全的文件系统名称
fn sanitize_for_path(id: &str) -> String {
    id.chars()
        .map(|ch| match ch {
            '/' | '\\' | '@' | ':' => '-',
            other => other,
        })
        .collect()
}
```

- [ ] **Step 2: 在 lib.rs 中注册模块**

在 `lib.rs` 顶部追加：

```rust
pub mod skill_installer;
```

在 pub use 区域追加：

```rust
pub use skill_installer::SkillInstaller;
```

- [ ] **Step 3: PluginManager 集成 SkillInstaller**

修改 `PluginManager` 结构体：

```rust
pub struct PluginManager {
    config: PluginManagerConfig,
    mcp_launcher: McpLauncher,
    skill_installer: SkillInstaller,
}
```

修改 `PluginManager::new`：

```rust
    pub fn new(config: PluginManagerConfig) -> Self {
        let skill_installer = SkillInstaller::new(config.config_home.join("skills"));
        Self {
            config,
            mcp_launcher: McpLauncher::new(),
            skill_installer,
        }
    }
```

在 `PluginManager::enable` 的 MCP 启动之后追加技能安装：

```rust
        // 安装 skills
        let manifest = load_plugin_from_directory(&record.install_path)?;
        if !manifest.skills.is_empty() {
            self.skill_installer
                .install_plugin_skills(plugin_id, &manifest.skills, &record.install_path)
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
        }
```

在 `PluginManager::disable` 增加技能移除：

```rust
        self.skill_installer.remove_plugin_skills(plugin_id).ok();
```

在 `PluginManager::uninstall` 增加技能清理：

```rust
        self.skill_installer.remove_plugin_skills(plugin_id).ok();
```

- [ ] **Step 4: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-plugins
```

Expected: 编译成功。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/plugins/src/skill_installer.rs src-tauri/crates/plugins/src/lib.rs
git commit -m "feat: SkillInstaller — 插件技能自动部署到 ~/.claw/skills/
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 9: Agent 提供者 — 从插件清单注册自动加载

**Files:**
- Modify: `src-tauri/crates/plugins/src/agent_provider.rs`
- Modify: `src-tauri/crates/plugins/src/lib.rs`

- [ ] **Step 1: agent_provider.rs 增加批量注册函数**

在 `agent_provider.rs` 末尾追加：

```rust
use crate::PluginAgentDefInternal;

/// 从插件清单注册 agents
pub fn register_plugin_agents(plugin_id: &str, agents: &[PluginAgentDefInternal]) {
    let registry = global_plugin_agents();
    for agent in agents {
        registry.register(PluginAgentDef {
            agent_type: format!("{}/{}", plugin_id, agent.agent_type),
            description: agent.description.clone(),
            tools: agent.tools.clone(),
            disallowed_tools: agent.disallowed_tools.clone(),
            model: agent.model.clone(),
            background: agent.background,
            system_prompt: agent.system_prompt.clone(),
        });
    }
}

/// 从插件注销所有 agents
pub fn unregister_plugin_agents(plugin_id: &str) {
    let registry = global_plugin_agents();
    let prefix = format!("{}/", plugin_id);
    let to_remove: Vec<String> = registry
        .all()
        .into_iter()
        .filter(|a| a.agent_type.starts_with(&prefix))
        .map(|a| a.agent_type)
        .collect();
    for agent_type in to_remove {
        registry.unregister(&agent_type);
    }
}
```

- [ ] **Step 2: PluginManager::enable 中调用 agent 注册**

在 `PluginManager::enable` 的 skill 安装之后追加：

```rust
        // 注册 agents
        if !manifest.agents.is_empty() {
            crate::agent_provider::register_plugin_agents(plugin_id, &manifest.agents);
        }
```

在 `PluginManager::disable` 中增加 agent 注销：

```rust
        crate::agent_provider::unregister_plugin_agents(plugin_id);
```

在 `PluginManager::uninstall` 中增加 agent 注销：

```rust
        crate::agent_provider::unregister_plugin_agents(plugin_id);
```

- [ ] **Step 3: 验证编译**

```bash
cd src-tauri && cargo check -p axagent-plugins
```

Expected: 编译成功。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/plugins/src/agent_provider.rs src-tauri/crates/plugins/src/lib.rs
git commit -m "feat: Agent 提供者 — 插件 agents 自动注册/注销
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 10: AppState 注入 PluginManager

**Files:**
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/init/state.rs`

- [ ] **Step 1: app_state.rs 增加 plugin_manager 字段**

在 `AppState` 结构体末尾追加（`sync_engine` 之后）：

```rust
    pub plugin_manager: std::sync::Mutex<PluginManager>,
```

在文件顶部增加 use：

```rust
use axagent_plugins::PluginManager;
```

- [ ] **Step 2: 在 create_app_state 中构造 PluginManager 并注入 AppState**

在 `src-tauri/src/init/state.rs` 顶部 use 区域追加：

```rust
use axagent_plugins::{PluginManager, PluginManagerConfig};
```

在 `create_app_state` 函数中，`let sync_engine = create_sync_engine(...)` 之后、`AppState {` 之前追加：

```rust
    let home = dirs::home_dir().unwrap_or_default();
    let config_home = home.join(".claw");
    let mut plugin_config = PluginManagerConfig::new(config_home.clone());
    plugin_config.external_dirs = axagent_core::skill_dirs::all_skills_dirs();
    let plugin_manager = std::sync::Mutex::new(PluginManager::new(plugin_config));
```

在 `AppState {` 构造的 `sync_engine,` 之后追加：

```rust
        plugin_manager,
```

完整的 `state.rs` 修改位置：
- 第 11 行后新增 `use axagent_plugins::{PluginManager, PluginManagerConfig};`
- 第 380 行（`sync_engine` 赋值后）新增 PluginManager 构造代码
- 第 370 行（`sync_engine,` 后）新增 `plugin_manager,`

- [ ] **Step 4: 验证编译**

```bash
cd src-tauri && cargo check
```

Expected: 整体项目编译成功。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/init/state.rs
git commit -m "feat: AppState 注入 PluginManager 实例
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 11: Tauri 命令接通

**Files:**
- Modify: `src-tauri/src/commands/plugin.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 重写 plugin.rs**

```rust
// src-tauri/src/commands/plugin.rs
use tauri::State;

use crate::app_state::AppState;

/// 列出已安装插件
#[command]
pub fn plugin_list(state: State<'_, AppState>) -> Result<Vec<PluginSummaryDto>, String> {
    let manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager
        .list_plugins()
        .map(|plugins| {
            plugins
                .into_iter()
                .map(|p| PluginSummaryDto {
                    id: p.metadata.id,
                    name: p.metadata.name,
                    version: p.metadata.version,
                    description: p.metadata.description,
                    kind: p.metadata.kind.to_string(),
                    enabled: p.enabled,
                    tools: p.tool_names,
                    mcp_servers: p.mcp_server_names,
                    skills: p.skill_names,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// 验证插件源（安装前预览清单）
#[command]
pub fn plugin_validate_source(
    state: State<'_, AppState>,
    source: String,
) -> Result<PluginManifestDto, String> {
    let manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let manifest = manager
        .validate_plugin_source(&source)
        .map_err(|e| e.to_string())?;
    Ok(PluginManifestDto {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        permissions: manifest.permissions.iter().map(|p| p.as_str().to_string()).collect(),
        default_enabled: manifest.default_enabled,
        hooks: {
            let mut hooks = serde_json::Map::new();
            hooks.insert(
                "PreToolUse".to_string(),
                serde_json::Value::Array(manifest.hooks.pre_tool_use.iter().map(|s| serde_json::Value::String(s.clone())).collect()),
            );
            hooks.insert(
                "PostToolUse".to_string(),
                serde_json::Value::Array(manifest.hooks.post_tool_use.iter().map(|s| serde_json::Value::String(s.clone())).collect()),
            );
            hooks
        },
        tools: manifest.tools.iter().map(|t| ToolDto {
            name: t.name.clone(),
            description: t.description.clone(),
        }).collect(),
        mcp_servers: manifest.mcp_servers.iter().map(|m| McpServerDto {
            name: m.name.clone(),
            command: m.command.clone(),
        }).collect(),
        skills: manifest.skills.iter().map(|s| SkillDto {
            name: s.name.clone(),
            path: s.path.clone(),
        }).collect(),
    })
}

/// 安装插件（同步命令，Tauri 在线程池上运行，避免嵌套 tokio runtime）
#[command]
pub fn plugin_install(
    state: State<'_, AppState>,
    source: String,
) -> Result<InstallOutcomeDto, String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let outcome = manager.install(&source).map_err(|e| e.to_string())?;
    Ok(InstallOutcomeDto {
        plugin_id: outcome.plugin_id,
        version: outcome.version,
        install_path: outcome.install_path.display().to_string(),
    })
}

/// 启用插件
#[command]
pub async fn plugin_enable(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.enable(&plugin_id).map_err(|e| e.to_string())
}

/// 禁用插件
#[command]
pub async fn plugin_disable(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.disable(&plugin_id).map_err(|e| e.to_string())
}

/// 卸载插件
#[command]
pub async fn plugin_uninstall(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    manager.uninstall(&plugin_id).map_err(|e| e.to_string())
}

/// 更新插件（同步命令）
#[command]
pub fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<UpdateOutcomeDto, String> {
    let mut manager = state.plugin_manager.lock().map_err(|e| e.to_string())?;
    let outcome = manager.update(&plugin_id).map_err(|e| e.to_string())?;
    Ok(UpdateOutcomeDto {
        plugin_id: outcome.plugin_id,
        old_version: outcome.old_version,
        new_version: outcome.new_version,
        install_path: outcome.install_path.display().to_string(),
    })
}

// —— DTO 类型（前端兼容） ——

#[derive(Debug, serde::Serialize)]
struct PluginSummaryDto {
    id: String,
    name: String,
    version: String,
    description: String,
    kind: String,
    enabled: bool,
    tools: Vec<String>,
    mcp_servers: Vec<String>,
    skills: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct PluginManifestDto {
    name: String,
    version: String,
    description: String,
    permissions: Vec<String>,
    default_enabled: bool,
    hooks: serde_json::Map<String, serde_json::Value>,
    tools: Vec<ToolDto>,
    mcp_servers: Vec<McpServerDto>,
    skills: Vec<SkillDto>,
}

#[derive(Debug, serde::Serialize)]
struct ToolDto {
    name: String,
    description: String,
}

#[derive(Debug, serde::Serialize)]
struct McpServerDto {
    name: String,
    command: String,
}

#[derive(Debug, serde::Serialize)]
struct SkillDto {
    name: String,
    path: String,
}

#[derive(Debug, serde::Serialize)]
struct InstallOutcomeDto {
    plugin_id: String,
    version: String,
    install_path: String,
}

#[derive(Debug, serde::Serialize)]
struct UpdateOutcomeDto {
    plugin_id: String,
    old_version: String,
    new_version: String,
    install_path: String,
}
```

- [ ] **Step 2: 更新 lib.rs generate_handler![] 注册**

在 `lib.rs` 的 `generate_handler![]` 中，替换原有的 4 个 plugin 命令：

```diff
-            commands::plugin::list_plugin_tools,
-            commands::plugin::plugin_enable,
-            commands::plugin::plugin_disable,
-            commands::plugin::plugin_install,
-            commands::plugin::plugin_uninstall,
+            commands::plugin::plugin_list,
+            commands::plugin::plugin_validate_source,
+            commands::plugin::plugin_install,
+            commands::plugin::plugin_enable,
+            commands::plugin::plugin_disable,
+            commands::plugin::plugin_uninstall,
+            commands::plugin::plugin_update,
```

- [ ] **Step 3: 验证编译**

```bash
cd src-tauri && cargo check
```

Expected: 整体项目编译成功。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/plugin.rs src-tauri/src/lib.rs
git commit -m "feat: Tauri 插件命令接通 PluginManager，替换空桩实现
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 12: 前端 UI — 搜索安装栏 + 确认弹窗

**Files:**
- Modify: `src/components/chat/PluginMarketplace.tsx`

- [ ] **Step 1: 重写 PluginMarketplace.tsx**

```tsx
import {
  Badge,
  Button,
  Card,
  Descriptions,
  Input,
  Modal,
  Space,
  Tag,
  Typography,
  message,
} from "antd";
import {
  CheckCircle,
  Code2,
  Download,
  Loader2,
  PackageSearch,
  RefreshCw,
  XCircle,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface PluginSummary {
  id: string;
  name: string;
  version: string;
  description: string;
  kind: string;
  enabled: boolean;
  tools: string[];
  mcp_servers: string[];
  skills: string[];
}

interface PluginManifest {
  name: string;
  version: string;
  description: string;
  permissions: string[];
  default_enabled: boolean;
  hooks: Record<string, string[]>;
  tools: { name: string; description: string }[];
  mcp_servers: { name: string; command: string }[];
  skills: { name: string; path: string }[];
}

interface InstallOutcome {
  plugin_id: string;
  version: string;
  install_path: string;
}

function PluginMarketplace() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installInput, setInstallInput] = useState("");
  const [searchLoading, setSearchLoading] = useState(false);
  const [confirmManifest, setConfirmManifest] = useState<PluginManifest | null>(null);
  const [confirmSource, setConfirmSource] = useState("");

  useEffect(() => {
    fetchPlugins();
  }, []);

  const fetchPlugins = async () => {
    setLoading(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const data = await invoke<PluginSummary[]>("plugin_list").catch(() => []);
      setPlugins(data);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  const handleSearchInstall = async () => {
    const source = installInput.trim();
    if (!source) return;
    setSearchLoading(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const manifest = await invoke<PluginManifest>("plugin_validate_source", {
        source,
      });
      setConfirmManifest(manifest);
      setConfirmSource(source);
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      message.error(`验证失败: ${errMsg}`);
    } finally {
      setSearchLoading(false);
    }
  };

  const handleConfirmInstall = async () => {
    if (!confirmSource) return;
    setInstalling(confirmSource);
    setConfirmManifest(null);
    try {
      const { invoke } = await import("@/lib/invoke");
      const result = await invoke<InstallOutcome>("plugin_install", {
        source: confirmSource,
      });
      message.success(
        `已安装 ${result.plugin_id} v${result.version}`,
      );
      setInstallInput("");
      setConfirmSource("");
      await fetchPlugins();
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      message.error(`安装失败: ${errMsg}`);
    } finally {
      setInstalling(null);
    }
  };

  const handleToggle = async (pluginId: string, enable: boolean) => {
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke(enable ? "plugin_enable" : "plugin_disable", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    }
  };

  const handleUninstall = async (pluginId: string) => {
    setInstalling(pluginId);
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke("plugin_uninstall", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    } finally {
      setInstalling(null);
    }
  };

  return (
    <>
      <Card size="small" className="plugin-marketplace">
        <div className="flex items-center justify-between mb-3">
          <Space>
            <PackageSearch size={16} className="text-purple-500" />
            <Title level={5} className="mb-0">
              {t("chat.plugins.marketplace.title")}
            </Title>
            <Badge count={plugins.length} size="small" />
          </Space>
          <Button size="small" onClick={fetchPlugins} loading={loading}>
            {t("chat.plugins.marketplace.refresh")}
          </Button>
        </div>

        <div className="mb-3">
          <Input.Search
            placeholder={t("chat.plugins.marketplace.installPlaceholder")}
            enterButton={t("chat.plugins.marketplace.install")}
            loading={searchLoading}
            value={installInput}
            onChange={(e) => setInstallInput(e.target.value)}
            onSearch={handleSearchInstall}
          />
        </div>

        {loading && plugins.length === 0 && (
          <div className="flex items-center gap-2 py-4 text-sm text-gray-500">
            <Loader2 size={14} className="animate-spin" />
            <span>{t("chat.plugins.marketplace.loading")}</span>
          </div>
        )}

        <div className="space-y-2 max-h-96 overflow-auto">
          {plugins.map((plugin) => (
            <Card key={plugin.id} size="small" className="plugin-card">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <Code2 size={14} className="text-purple-500" />
                    <Text strong className="text-sm">
                      {plugin.name}
                    </Text>
                    <Tag color="purple" className="text-xs">
                      {plugin.version}
                    </Tag>
                    {plugin.enabled && (
                      <CheckCircle size={12} className="text-green-500" />
                    )}
                  </div>
                  <Text type="secondary" className="text-xs block mt-1">
                    {plugin.description}
                  </Text>
                  <Space size="small" className="mt-1">
                    <Tag color="geekblue" className="text-xs">
                      {plugin.kind}
                    </Tag>
                    {(plugin.mcp_servers.length > 0 || plugin.skills.length > 0) && (
                      <Text type="secondary" className="text-xs">
                        ⚡{plugin.mcp_servers.length} 📋{plugin.skills.length}
                      </Text>
                    )}
                  </Space>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    size="small"
                    type={plugin.enabled ? "default" : "primary"}
                    onClick={() => handleToggle(plugin.id, !plugin.enabled)}
                  >
                    {plugin.enabled
                      ? t("chat.plugins.marketplace.disable")
                      : t("chat.plugins.marketplace.enable")}
                  </Button>
                  <Button
                    size="small"
                    danger
                    icon={<XCircle size={12} />}
                    loading={installing === plugin.id}
                    onClick={() => handleUninstall(plugin.id)}
                  />
                </div>
              </div>

              {plugin.tools.length > 0 && (
                <div className="flex gap-2 mt-2 flex-wrap">
                  {plugin.tools.slice(0, 5).map((tool, i) => (
                    <Tag key={i} color="cyan" className="text-xs">
                      {tool}
                    </Tag>
                  ))}
                  {plugin.tools.length > 5 && (
                    <Text type="secondary" className="text-xs">
                      +{plugin.tools.length - 5}
                    </Text>
                  )}
                </div>
              )}
            </Card>
          ))}
        </div>
      </Card>

      {/* 安装确认弹窗 */}
      <Modal
        title={`安装 ${confirmManifest?.name ?? ""}`}
        open={!!confirmManifest}
        onOk={handleConfirmInstall}
        onCancel={() => setConfirmManifest(null)}
        okText="确认安装"
        cancelText="取消"
        width={560}
      >
        {confirmManifest && (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label="版本">
              {confirmManifest.version}
            </Descriptions.Item>
            <Descriptions.Item label="描述">
              {confirmManifest.description}
            </Descriptions.Item>
            <Descriptions.Item label="权限">
              {confirmManifest.permissions.length > 0
                ? confirmManifest.permissions.join(", ")
                : "无"}
            </Descriptions.Item>
            <Descriptions.Item label="MCP 服务">
              {confirmManifest.mcp_servers.length > 0
                ? confirmManifest.mcp_servers
                    .map((s) => `${s.name} (${s.command})`)
                    .join(", ")
                : "无"}
            </Descriptions.Item>
            <Descriptions.Item label="技能">
              {confirmManifest.skills.length > 0
                ? confirmManifest.skills.map((s) => s.name).join(", ")
                : "无"}
            </Descriptions.Item>
            <Descriptions.Item label="工具">
              {confirmManifest.tools.length > 0
                ? confirmManifest.tools.map((t) => t.name).join(", ")
                : "无"}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Modal>
    </>
  );
}

export default PluginMarketplace;
```

- [ ] **Step 2: 验证前端编译**

```bash
npm run typecheck
```

Expected: 类型检查通过。

- [ ] **Step 3: Commit**

```bash
git add src/components/chat/PluginMarketplace.tsx
git commit -m "feat: PluginMarketplace 增加 npm 搜索安装栏与确认弹窗
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 13: 前端类型定义 + i18n

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/i18n/locales/zh-CN.json`
- Modify: `src/i18n/locales/en-US.json`
- Modify: `src/i18n/locales/ja-JP.json`
- Modify: `src/i18n/locales/ko-KR.json`
- Modify: `src/i18n/locales/de-DE.json`
- Modify: `src/i18n/locales/fr-FR.json`
- Modify: `src/i18n/locales/pt-BR.json`
- Modify: `src/i18n/locales/es-ES.json`
- Modify: `src/i18n/locales/ru-RU.json`
- Modify: `src/i18n/locales/zh-TW.json`
- Modify: `src/i18n/locales/ar-SA.json`

- [ ] **Step 1: 在 src/types/index.ts 末尾追加类型定义**

```typescript
// === Plugin System ===
export interface PluginSummaryDto {
  id: string;
  name: string;
  version: string;
  description: string;
  kind: "builtin" | "bundled" | "external";
  enabled: boolean;
  tools: string[];
  mcp_servers: string[];
  skills: string[];
}

export interface PluginManifestDto {
  name: string;
  version: string;
  description: string;
  permissions: string[];
  default_enabled: boolean;
  hooks: Record<string, string[]>;
  tools: { name: string; description: string }[];
  mcp_servers: { name: string; command: string }[];
  skills: { name: string; path: string }[];
}

export interface InstallOutcomeDto {
  plugin_id: string;
  version: string;
  install_path: string;
}
```

- [ ] **Step 2: 在 zh-CN.json 的 plugins.marketplace 中增加 installPlaceholder**

```json
"plugins": {
  "marketplace": {
    "disable": "禁用",
    "enable": "启用",
    "install": "安装",
    "installPlaceholder": "安装: @scope/name 或 Git URL 或本地路径",
    "loading": "加载中...",
    "refresh": "刷新",
    "title": "插件市场"
  }
}
```

- [ ] **Step 3: 更新其他 10 种语言的 plugins.marketplace.installPlaceholder**

| 语言 | installPlaceholder |
|------|-------------------|
| en-US | `"Install: @scope/name or Git URL or local path"` |
| ja-JP | `"インストール: @scope/name または GitURL またはローカルパス"` |
| ko-KR | `"설치: @scope/name 또는 Git URL 또는 로컬 경로"` |
| de-DE | `"Installieren: @scope/name oder Git-URL oder lokaler Pfad"` |
| fr-FR | `"Installer: @scope/name ou URL Git ou chemin local"` |
| pt-BR | `"Instalar: @scope/name ou URL Git ou caminho local"` |
| es-ES | `"Instalar: @scope/name o URL Git o ruta local"` |
| ru-RU | `"Установка: @scope/name или Git URL или локальный путь"` |
| zh-TW | `"安裝: @scope/name 或 Git URL 或本機路徑"` |
| ar-SA | `"تثبيت: @scope/name أو Git URL أو مسار محلي"` |

- [ ] **Step 4: 验证编译**

```bash
npm run typecheck && npm run format
```

Expected: 类型检查通过，格式化通过。

- [ ] **Step 5: Commit**

```bash
git add src/types/index.ts src/i18n/locales/
git commit -m "feat: 前端类型 PluginSummaryDto + 11 语言 i18n 安装提示
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 14: 集成测试 — 端到端安装流程

**Files:**
- Modify: `src-tauri/crates/plugins/src/lib.rs` (追加测试)

- [ ] **Step 1: 在 lib.rs 测试模块中增加 npm 源解析测试**

在现有 `#[cfg(test)] mod tests` 块末尾追加：

```rust
    #[test]
    fn parse_install_source_recognizes_npm_scoped() {
        let result = parse_install_source("@clawd/ths").expect("should parse");
        assert!(matches!(
            result,
            PluginInstallSource::NpmPackage { ref name, ref version }
            if name == "@clawd/ths" && version.is_none()
        ));
    }

    #[test]
    fn parse_install_source_recognizes_npm_with_version() {
        let result = parse_install_source("@clawd/stock@1.2.0").expect("should parse");
        assert!(matches!(
            result,
            PluginInstallSource::NpmPackage { ref name, ref version }
            if name == "@clawd/stock" && version == &Some("1.2.0".to_string())
        ));
    }

    #[test]
    fn parse_install_source_recognizes_git_url() {
        let result = parse_install_source("https://github.com/user/repo.git")
            .expect("should parse");
        assert!(matches!(result, PluginInstallSource::GitUrl { .. }));
    }

    #[test]
    fn manifest_parses_mcp_servers() {
        let json = r#"{
            "name": "test-plugin",
            "version": "1.0.0",
            "description": "test",
            "mcpServers": [
                {
                    "name": "test-mcp",
                    "command": "python",
                    "args": ["-m", "test"],
                    "env": {}
                }
            ]
        }"#;
        let raw: RawPluginManifest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(raw.mcp_servers.len(), 1);
        assert_eq!(raw.mcp_servers[0].name, "test-mcp");
    }

    #[test]
    fn manifest_parses_skills() {
        let json = r#"{
            "name": "test-plugin",
            "version": "1.0.0",
            "description": "test",
            "skills": [
                {"name": "analyzer", "path": "skills/analyzer/SKILL.md"}
            ]
        }"#;
        let raw: RawPluginManifest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(raw.skills.len(), 1);
        assert_eq!(raw.skills[0].name, "analyzer");
    }

    #[test]
    fn manifest_parses_agents() {
        let json = r#"{
            "name": "test-plugin",
            "version": "1.0.0",
            "description": "test",
            "agents": [
                {
                    "agentType": "stock-bot",
                    "description": "Stock analysis agent",
                    "tools": ["get_price"],
                    "disallowedTools": [],
                    "background": false
                }
            ]
        }"#;
        let raw: RawPluginManifest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(raw.agents.len(), 1);
        assert_eq!(raw.agents[0].agent_type, "stock-bot");
    }

    #[test]
    fn manifest_accepts_mcp_servers_without_error() {
        let json = serde_json::json!({
            "name": "test-plugin",
            "version": "1.0.0",
            "description": "test",
            "mcpServers": [{"name": "mcp", "command": "echo", "args": [], "env": {}}],
            "skills": [{"name": "skill", "path": "s.md"}],
            "agents": [{"agentType": "bot", "description": "bot", "tools": [], "disallowedTools": [], "background": false}]
        });
        let errors = detect_claude_code_manifest_contract_gaps(&json);
        assert!(errors.is_empty(), "mcpServers/skills/agents should not be rejected");
    }
```

- [ ] **Step 2: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-plugins
```

Expected: All tests pass（包括新增加的 7 个测试）。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/plugins/src/lib.rs
git commit -m "test: npm 源解析 + 清单新字段解析 + 兼容性检查单元测试
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 15: 最终验证 — CI 门禁

**Files:** 无新建，验证现有代码质量门禁。

- [ ] **Step 1: Rust 格式化**

```bash
cd src-tauri && cargo fmt -- --check
```

Expected: 无格式差异。

- [ ] **Step 2: Clippy 零警告**

```bash
cd src-tauri && cargo clippy -- -D warnings
```

Expected: 零警告通过。

- [ ] **Step 3: 前端类型检查**

```bash
npm run typecheck
```

Expected: 类型检查通过。

- [ ] **Step 4: 前端格式化**

```bash
npm run format
```

Expected: 格式化通过或显示无差异。

- [ ] **Step 5: 完整构建验证**

```bash
npm run build
```

Expected: tsc + vite build 成功。

- [ ] **Step 6: Commit (如有 CI 修复)**

```bash
# 仅当 clippy/fmt 有修复时执行
git add -u && git commit -m "chore: CI 门禁修复 — clippy + fmt + typecheck
Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```
