# OpenClaw 插件生态全面兼容设计

## 概述

以 `@clawd/ths`（OpenClaw 连接同花顺 A 股数据插件）为驱动示例，实现 AxAgent 对 OpenClaw 插件生态的全面兼容。改造包括：新增 npm registry 客户端、放通插件清单全部字段、接通 MCP/技能/Agent 生命周期、打通前后端安装链路。

## 目标

用户能像 OpenClaw 一样安装和使用插件：
```bash
# 目标体验（前端搜索框输入）
@clawd/ths          # npm 包，latest
@clawd/stock@1.2.0  # npm 包，指定版本
https://github.com/...  # Git URL（已有能力）
/path/to/plugin     # 本地路径（已有能力）
```

## 架构

### 新增 crate：`axagent-npm`

```
src-tauri/crates/npm/
├── Cargo.toml
└── src/
    ├── lib.rs         # crate 入口，re-export
    ├── registry.rs    # NpmRegistry: resolve / download
    ├── tarball.rs     # flate2 + tar 解压
    └── types.rs       # PackageInfo, VersionInfo, DistInfo, NpmError
```

依赖：`reqwest`（HTTP 请求 registry API）、`flate2`（gzip 解压）、`tar`（tar 解包）、`tempfile`、`thiserror`。

### 改造 crate：`axagent-plugins`

```
src-tauri/crates/plugins/
├── Cargo.toml                  # 新增依赖 axagent-npm
└── src/
    ├── lib.rs                  # PluginInstallSource 加 NpmPackage；清单兼容；materialize_source npm 分支
    ├── hooks.rs                # 不变
    ├── agent_provider.rs       # 从插件 agents 字段加载
    ├── mcp_launcher.rs         # 🆕 MCP 子进程管理
    └── skill_installer.rs      # 🆕 技能文件部署
```

### 改造其他模块

| 模块 | 改动 |
|------|------|
| `src-tauri/src/app_state.rs` | 注入 `PluginManager` |
| `src-tauri/src/commands/plugin.rs` | 空桩 → 真实调用 |
| `src/components/chat/PluginMarketplace.tsx` | 增加搜索安装栏 + 确认弹窗 + 详情展示 |
| `src/types/` | 新增 `PluginSummaryDto`, `InstallOutcomeDto`, `PluginManifestDto` |

### 数据流

```
用户输入 @clawd/ths
     │
     ▼
npm crate  ──GET /@clawd%2Fths──▶ registry.npmjs.org
     │                              ◄── PackageInfo { dist.tarball }
     ▼
npm crate  ──download──▶ tarball → 解压到 install_root/.tmp/
     │
     ▼
plugins/lib.rs
  解析 plugin.json → PluginManifest { mcpServers, skills, agents, hooks, tools }
     │
     ├──▶ mcp_launcher.rs  ──std::process::Command──▶ 启动 MCP 子进程
     ├──▶ skill_installer.rs ──复制──▶ ~/.claw/skills/<plugin-id>/
     ├──▶ agent_provider.rs ──注册──▶ PluginAgentRegistry
     ├──▶ hooks.rs          ──不变──▶ HookRunner
     └──▶ 已有逻辑          ──不变──▶ 工具注册
     │
     ▼
Tauri 命令  ──invoke──▶ 前端 PluginMarketplace.tsx
```

## 详细设计

### 1. npm crate — `axagent-npm`

#### 1.1 types.rs

```rust
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

#### 1.2 registry.rs

```rust
const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

pub struct NpmRegistry {
    registry_url: String,
    client: reqwest::Client,
}

impl NpmRegistry {
    pub fn new() -> Self;

    /// 解析包名: "@scope/name@version" → ("@scope/name", Option<"version">)
    pub fn parse_package_spec(spec: &str) -> (&str, Option<&str>);

    /// GET /<package> → PackageInfo
    /// scoped package: @scope/name → @scope%2Fname
    pub async fn fetch_package_info(&self, name: &str) -> Result<PackageInfo, NpmError>;

    /// 解析版本 latest 或 semver
    pub fn resolve_version<'a>(
        info: &'a PackageInfo,
        version: Option<&str>,
    ) -> Result<&'a VersionInfo, NpmError>;

    /// 下载 tarball 流式解压到 dest，返回插件根目录
    pub async fn download_and_extract(
        &self,
        dist: &DistInfo,
        dest: &Path,
    ) -> Result<PathBuf, NpmError>;
}
```

- `fetch_package_info`: `reqwest::get("{registry}/{escaped_name}")`, 404 返回 `NpmError::NotFound`
- `download_and_extract`: `bytes_stream` → `GzDecoder` → `tar::Archive::unpack`，解压后检测外层 package 目录

### 2. PluginInstallSource 扩展

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginInstallSource {
    LocalPath { path: PathBuf },
    GitUrl { url: String },
    NpmPackage {                     // 🆕
        name: String,
        version: Option<String>,
    },
}
```

`parse_install_source()` 增加 npm 识别：以 `@` 开头或符合 `scope/name` 模式时解析为 `NpmPackage`。

`materialize_source()` 增加 npm 分支：调用 `NpmRegistry::fetch_package_info` → `resolve_version` → `download_and_extract`。

### 3. 清单完全兼容

#### 3.1 删除拒绝逻辑

`detect_claude_code_manifest_contract_gaps()` 中删除对 `skills`、`mcpServers`、`agents` 的拒绝。保留对 `commands` 字符串数组形式和未知 Hook 的拒绝（这两项是后端能力限制，非有意兼容）。

#### 3.2 PluginManifest 新增字段

```rust
pub struct PluginManifest {
    // 现有不变
    pub name: String;
    pub version: String;
    pub description: String;
    pub permissions: Vec<PluginPermission>;
    pub default_enabled: bool;
    pub hooks: PluginHooks;
    pub lifecycle: PluginLifecycle;
    pub tools: Vec<PluginToolManifest>;
    pub commands: Vec<PluginCommandManifest>;
    pub scenarios: Vec<String>;

    // 🆕
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    pub agents: Vec<PluginAgentDefInternal>,
}
```

`RawPluginManifest` 对应增加 `#[serde(default)]` 字段，`build_plugin_manifest()` 中做校验转换。

### 4. MCP 启动器 (`mcp_launcher.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

pub struct McpLauncher {
    running: HashMap<String, Vec<RunningMcpProcess>>,
}

impl McpLauncher {
    pub fn start_plugin_mcps(&mut self, plugin_id: &str, servers: &[PluginMcpServer], plugin_root: &Path) -> Result<(), McpLaunchError>;
    pub fn stop_plugin_mcps(&mut self, plugin_id: &str);
}
```

- **启用时**：`enable()` 调用 `start_plugin_mcps`，spawn 子进程，设置 `CLAWD_PLUGIN_ID` / `CLAWD_PLUGIN_ROOT` 环境变量
- **禁用时**：`disable()` 调用 `stop_plugin_mcps`，kill 子进程 + wait 回收
- **Drop 保护**：`McpLauncher::drop` 遍历 kill 所有残留进程
- **健康检查**：spawn 后等待 2s 检查存活；stdout/stderr 通过 tracing 记录

### 5. 技能安装器 (`skill_installer.rs`)

```rust
pub struct PluginSkillEntry {
    pub name: String,
    pub path: String,  // 插件内相对路径
}

pub struct SkillInstaller {
    skills_root: PathBuf,  // ~/.claw/skills/
}

impl SkillInstaller {
    pub fn install_plugin_skills(&self, plugin_id: &str, skills: &[PluginSkillEntry], plugin_root: &Path) -> Result<Vec<PathBuf>, std::io::Error>;
    pub fn remove_plugin_skills(&self, plugin_id: &str) -> Result<(), std::io::Error>;
}
```

路径映射：`<plugin_root>/skills/stock-analyzer/SKILL.md` → `~/.claw/skills/<plugin-id>/skills/stock-analyzer/SKILL.md`

### 6. Agent 提供者增强

`agent_provider.rs` 新增 `register_plugin_agents()`，插件加载时自动将 `agents` 字段中的定义注册到 `PluginAgentRegistry`。

### 7. Tauri 命令接通

#### 7.1 AppState 注入

```rust
pub struct AppState {
    pub plugin_manager: std::sync::Mutex<PluginManager>,
    // ... 现有字段
}
```

#### 7.2 命令重写

| 命令 | 功能 |
|------|------|
| `plugin_install` | 调用 `PluginManager::install()` |
| `plugin_enable` | 调用 `PluginManager::enable()` — 含 MCP 启动 |
| `plugin_disable` | 调用 `PluginManager::disable()` — 含 MCP 停止 |
| `plugin_uninstall` | 调用 `PluginManager::uninstall()` — 含 skills 清理 |
| `plugin_update` | 调用 `PluginManager::update()` |
| `plugin_list` | 调用 `PluginManager::list_plugins()` |
| `plugin_validate_source` | 调用 `PluginManager::validate_plugin_source()` |

全部返回 `Result<T, String>`，通过 `PluginManager` 方法链式调用 `.map_err(|e| e.to_string())`。

### 8. 前端 UI

#### 8.1 PluginMarketplace.tsx 改造

- 顶部增加 `Input.Search` 搜索/安装栏，placeholder 提示三种安装格式
- 安装前弹出确认 Modal，展示插件名称、版本、描述、权限、MCP 服务、技能、工具
- 插件列表卡片增加 MCP 服务、技能数量、Agent 数量的展示
- 增加安装中 loading 状态和错误 toast

#### 8.2 前端类型 (`src/types/`)

```typescript
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

export interface InstallOutcomeDto {
  plugin_id: string;
  version: string;
  install_path: string;
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
```

### 9. 错误处理

| 场景 | 错误类型 | 用户提示 |
|------|---------|---------|
| npm 包不存在 | `NpmError::NotFound` | 找不到 npm 包 `xxx` |
| 版本不存在 | `NpmError::VersionNotFound` | 版本 `x.x.x` 不存在 |
| registry 网络不通 | `NpmError::RequestFailed` | 无法连接 npm registry，请检查网络 |
| tarball 解压失败 | `NpmError::ExtractFailed` | 插件包解压失败，可能已损坏 |
| 无有效清单 | `PluginError::NotFound` | 该包不包含有效的插件清单 |
| 同名已安装 | `PluginError::CommandFailed` | 插件已安装，请先卸载 |
| MCP 启动失败 | `McpLaunchError` | MCP 服务 `xxx` 启动失败 |

### 10. 测试策略

| 层级 | 内容 | 工具 |
|------|------|------|
| npm crate | parse_package_spec 各种格式；mock HTTP server 验证 API | Rust `#[cfg(test)]` |
| plugins crate | install source 识别；manifest 新字段解析；MCP 生命周期 | 已有 20+ tests 增量加 |
| 集成测试 | 端到端安装 mock npm registry 中的插件 | Rust test |
| 前端 E2E | 搜索 → 验证 → 安装 → 启用 → 卸载 | Playwright |
| CI 门禁 | `cargo clippy -- -D warnings` + `npm run typecheck` + `npm run format` | GitHub Actions |

## 实施顺序

| 步骤 | 内容 | 预估工日 |
|------|------|---------|
| 1 | 新建 `axagent-npm` crate，实现 registry + tarball | 1 天 |
| 2 | `PluginInstallSource` 扩展 + `materialize_source` npm 分支 | 0.5 天 |
| 3 | 清单兼容：删除拒绝逻辑 + 新增 mcpServers/skills/agents | 0.5 天 |
| 4 | `mcp_launcher.rs` + `skill_installer.rs` + agent 增强 | 1 天 |
| 5 | Tauri 命令接通 + AppState 注入 | 0.5 天 |
| 6 | 前端 UI 改造 | 0.5 天 |
| 7 | 测试 + 文档 | 0.5 天 |
| **合计** | | **4.5 天** |

## 关键决策记录

| 决策 | 结论 |
|------|------|
| npm 安装方式 | 纯 Rust 实现，不依赖 Node.js |
| 清单兼容范围 | mcpServers + skills + agents 全部放通 |
| 安装入口 | 前端市场 + 命令接通 |
| MCP 生命周期 | 自动管理（启用即启动，禁用即停止） |
| 架构风格 | 方案 B（npm 独立 crate，分层解耦） |
| npm registry 地址 | 默认 `registry.npmjs.org`，预留可配置 |

## 不变项

- Hook 系统：仅支持 PreToolUse / PostToolUse / PostToolUseFailure，不支持额外 Hook 类型
- slash 命令：仍走内置分发，不加载插件 command markdown 目录
- 清单路径：仅支持 `plugin.json` / `.claude-plugin/plugin.json` / `SKILL.md` 三层
- 权限模型：维持 read / write / execute 三级
- 工具权限：维持 read-only / workspace-write / danger-full-access 三级
