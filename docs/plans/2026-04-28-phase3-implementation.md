# Phase 3：智能体深度能力 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 提升代理智能度，实现多实例 Profiles、Shell 生命周期 Hooks、/steer 中途干预、AGENTS.md 上下文兼容、RL 训练环境集成、代理中断精细控制六大能力。

**Architecture:** 在现有 `runtime`/`agent`/`trajectory`/`core` crate 体系上扩展，遵循已有的 trait + Arc<RwLock<>> + async_trait 模式。前端遵循 React + Tauri invoke 模式。

**Tech Stack:** Rust (tokio, serde, sea-orm), TypeScript/React (Tauri invoke)

---

## Task 1: 3.1 多实例 Profiles — 后端

**Files:**
- Create: `src-tauri/crates/runtime/src/profile.rs`
- Create: `src-tauri/crates/runtime/src/profile_manager.rs`
- Modify: `src-tauri/crates/runtime/src/lib.rs`
- Modify: `src-tauri/crates/core/src/unified_config.rs`
- Modify: `src-tauri/crates/core/src/db.rs`

### Step 1: 创建 `profile.rs` — Profile 数据结构与核心操作

```rust
// src-tauri/crates/runtime/src/profile.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub display_name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub profile: Profile,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub sessions_path: PathBuf,
    pub skills_path: PathBuf,
    pub hooks_path: PathBuf,
}

impl Profile {
    pub fn new(name: &str, display_name: &str) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            created_at: now,
            updated_at: now,
            is_default: false,
        }
    }

    pub fn default_profile() -> Self {
        Self {
            name: "default".to_string(),
            display_name: "Default".to_string(),
            created_at: 0,
            updated_at: 0,
            is_default: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Profile not found: {0}")]
    NotFound(String),
    #[error("Profile already exists: {0}")]
    AlreadyExists(String),
    #[error("Cannot delete default profile")]
    CannotDeleteDefault,
    #[error("Invalid profile name: {0}")]
    InvalidName(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn validate_profile_name(name: &str) -> Result<(), ProfileError> {
    if name.is_empty() {
        return Err(ProfileError::InvalidName("Name cannot be empty".to_string()));
    }
    if name.len() > 64 {
        return Err(ProfileError::InvalidName("Name too long (max 64 chars)".to_string()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ProfileError::InvalidName(
            "Name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }
    if name == "default" {
        return Err(ProfileError::AlreadyExists("default".to_string()));
    }
    Ok(())
}

pub fn profiles_root() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".axagent")
        .join("profiles")
}

pub fn profile_dir(name: &str) -> PathBuf {
    profiles_root().join(name)
}

pub fn ensure_profile_dirs(name: &str) -> Result<PathBuf, ProfileError> {
    let dir = profile_dir(name);
    fs::create_dir_all(dir.join("config"))?;
    fs::create_dir_all(dir.join("data"))?;
    fs::create_dir_all(dir.join("sessions"))?;
    fs::create_dir_all(dir.join("skills"))?;
    fs::create_dir_all(dir.join("hooks"))?;
    Ok(dir)
}
```

### Step 2: 创建 `profile_manager.rs` — Profile 管理器

```rust
// src-tauri/crates/runtime/src/profile_manager.rs
use crate::profile::{self, Profile, ProfileError, ProfileInfo};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ProfileManager {
    active_profile: Arc<RwLock<String>>,
}

impl ProfileManager {
    pub fn new() -> Self {
        Self {
            active_profile: Arc::new(RwLock::new("default".to_string())),
        }
    }

    pub async fn active_profile(&self) -> String {
        self.active_profile.read().await.clone()
    }

    pub async fn set_active(&self, name: &str) -> Result<(), ProfileError> {
        if name != "default" && !self.list().await?.iter().any(|p| p.profile.name == name) {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        *self.active_profile.write().await = name.to_string();
        Ok(())
    }

    pub async fn create(&self, name: &str, display_name: &str) -> Result<ProfileInfo, ProfileError> {
        profile::validate_profile_name(name)?;
        let dir = profile_dir(name);
        if dir.exists() {
            return Err(ProfileError::AlreadyExists(name.to_string()));
        }
        profile::ensure_profile_dirs(name)?;
        let profile = Profile::new(name, display_name);
        let info = self.info_for(name, &profile)?;
        let meta_path = dir.join("profile.json");
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|e| ProfileError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        std::fs::write(&meta_path, json)?;
        Ok(info)
    }

    pub async fn delete(&self, name: &str) -> Result<(), ProfileError> {
        if name == "default" {
            return Err(ProfileError::CannotDeleteDefault);
        }
        let dir = profile_dir(name);
        if !dir.exists() {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        std::fs::remove_dir_all(&dir)?;
        let active = self.active_profile.read().await.clone();
        if active == name {
            *self.active_profile.write().await = "default".to_string();
        }
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<ProfileInfo>, ProfileError> {
        let root = profile::profiles_root();
        let mut result = vec![];
        if !root.exists() {
            result.push(self.info_for("default", &Profile::default_profile())?);
            return Ok(result);
        }
        result.push(self.info_for("default", &Profile::default_profile())?);
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta_path = entry.path().join("profile.json");
                let profile = if meta_path.exists() {
                    let content = std::fs::read_to_string(&meta_path)?;
                    serde_json::from_str(&content)
                        .unwrap_or_else(|_| Profile::new(&name, &name))
                } else {
                    Profile::new(&name, &name)
                };
                result.push(self.info_for(&name, &profile)?);
            }
        }
        Ok(result)
    }

    fn info_for(&self, name: &str, profile: &Profile) -> Result<ProfileInfo, ProfileError> {
        let dir = if name == "default" {
            dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".axagent")
        } else {
            profile::profile_dir(name)
        };
        Ok(ProfileInfo {
            profile: profile.clone(),
            config_path: dir.join("config"),
            db_path: dir.join("data").join("axagent.db"),
            sessions_path: dir.join("sessions"),
            skills_path: dir.join("skills"),
            hooks_path: dir.join("hooks"),
        })
    }

    pub async fn active_info(&self) -> Result<ProfileInfo, ProfileError> {
        let name = self.active_profile().await;
        let profiles = self.list().await?;
        profiles
            .into_iter()
            .find(|p| p.profile.name == name)
            .ok_or_else(|| ProfileError::NotFound(name))
    }
}

fn profile_dir(name: &str) -> PathBuf {
    profile::profile_dir(name)
}
```

### Step 3: 修改 `lib.rs` 注册模块

在 `runtime/src/lib.rs` 添加:
```rust
pub mod profile;
pub mod profile_manager;
```

### Step 4: 修改 `unified_config.rs` 支持 profile 路径

在 `UnifiedConfig` 中添加 `profile` 字段:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileSettings {
    pub active_profile: String,
}

// 在 UnifiedConfig 中添加:
pub profile: ProfileSettings,
```

### Step 5: 修改 `db.rs` 支持 profile 独立数据库

在 `create_pool` 函数中添加 profile 参数支持:
```rust
pub async fn create_pool_for_profile(profile_name: &str) -> Result<DbHandle> {
    let db_path = if profile_name == "default" {
        default_db_path()
    } else {
        profile_db_path(profile_name)
    };
    create_pool(&db_path).await
}

fn default_db_path() -> String {
    let home = dirs::home_dir().expect("Could not determine home directory");
    let path = home.join(".axagent").join("data").join("axagent.db");
    path.to_string_lossy().to_string()
}

fn profile_db_path(profile_name: &str) -> String {
    let home = dirs::home_dir().expect("Could not determine home directory");
    let path = home
        .join(".axagent")
        .join("profiles")
        .join(profile_name)
        .join("data")
        .join("axagent.db");
    path.to_string_lossy().to_string()
}
```

### Step 6: 添加 Tauri 命令

在 `src-tauri/src/commands/` 中添加 `profile.rs`:
```rust
use axagent_runtime::profile_manager::ProfileManager;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ProfileManagerState = Arc<Mutex<ProfileManager>>;

#[tauri::command]
pub async fn profile_list(
    manager: State<'_, ProfileManagerState>,
) -> Result<Vec<axagent_runtime::profile::ProfileInfo>, String> {
    let mgr = manager.lock().await;
    mgr.list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_create(
    manager: State<'_, ProfileManagerState>,
    name: String,
    display_name: String,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.create(&name, &display_name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_delete(
    manager: State<'_, ProfileManagerState>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.delete(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_switch(
    manager: State<'_, ProfileManagerState>,
    name: String,
) -> Result<(), String> {
    let mgr = manager.lock().await;
    mgr.set_active(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_active(
    manager: State<'_, ProfileManagerState>,
) -> Result<axagent_runtime::profile::ProfileInfo, String> {
    let mgr = manager.lock().await;
    mgr.active_info().await.map_err(|e| e.to_string())
}
```

在 `commands/mod.rs` 中注册:
```rust
pub mod profile;
```

---

## Task 2: 3.1 多实例 Profiles — 前端

**Files:**
- Create: `src/components/settings/ProfileSelector.tsx`
- Create: `src/components/settings/ProfileManager.tsx`

### Step 1: 创建 `ProfileSelector.tsx`

```tsx
import { Select, SelectItem } from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

interface ProfileInfo {
  profile: { name: string; display_name: string; is_default: boolean };
}

export default function ProfileSelector() {
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [active, setActive] = useState("default");

  useEffect(() => {
    invoke<ProfileInfo[]>("profile_list").then(setProfiles);
    invoke<{ profile: { name: string } }>("profile_active").then((p) =>
      setActive(p.profile.name)
    );
  }, []);

  const handleSwitch = async (name: string) => {
    await invoke("profile_switch", { name });
    setActive(name);
  };

  return (
    <Select
      label="Profile"
      selectedKeys={[active]}
      onSelectionChange={(keys) => {
        const name = Array.from(keys)[0] as string;
        if (name) handleSwitch(name);
      }}
    >
      {profiles.map((p) => (
        <SelectItem key={p.profile.name}>
          {p.profile.display_name}
        </SelectItem>
      ))}
    </Select>
  );
}
```

### Step 2: 创建 `ProfileManager.tsx`

```tsx
import { Button, Card, CardBody, Input, Modal, ModalBody, ModalContent, ModalFooter, ModalHeader, useDisclosure } from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

interface ProfileInfo {
  profile: { name: string; display_name: string; is_default: boolean; created_at: number };
}

export default function ProfileManager() {
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [newName, setNewName] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const { isOpen, onOpen, onClose } = useDisclosure();

  const load = () => invoke<ProfileInfo[]>("profile_list").then(setProfiles);

  useEffect(() => { load(); }, []);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    await invoke("profile_create", { name: newName, displayName: newDisplayName || newName });
    setNewName("");
    setNewDisplayName("");
    onClose();
    load();
  };

  const handleDelete = async (name: string) => {
    await invoke("profile_delete", { name });
    load();
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-semibold">Profiles</h3>
        <Button size="sm" color="primary" onPress={onOpen}>New Profile</Button>
      </div>
      {profiles.map((p) => (
        <Card key={p.profile.name}>
          <CardBody className="flex flex-row justify-between items-center">
            <div>
              <p className="font-medium">{p.profile.display_name}</p>
              <p className="text-sm text-default-500">{p.profile.name}</p>
            </div>
            {!p.profile.is_default && (
              <Button size="sm" color="danger" variant="light" onPress={() => handleDelete(p.profile.name)}>
                Delete
              </Button>
            )}
          </CardBody>
        </Card>
      ))}
      <Modal isOpen={isOpen} onClose={onClose}>
        <ModalContent>
          <ModalHeader>Create Profile</ModalHeader>
          <ModalBody>
            <Input label="Name" value={newName} onValueChange={setNewName} />
            <Input label="Display Name" value={newDisplayName} onValueChange={setNewDisplayName} />
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onClose}>Cancel</Button>
            <Button color="primary" onPress={handleCreate}>Create</Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
}
```

---

## Task 3: 3.2 Shell 生命周期 Hooks — 后端

**Files:**
- Create: `src-tauri/crates/runtime/src/shell_hooks.rs`
- Create: `src-tauri/crates/runtime/src/hook_config.rs`
- Modify: `src-tauri/crates/runtime/src/lib.rs`

### Step 1: 创建 `hook_config.rs` — Hook 配置加载

```rust
// src-tauri/crates/runtime/src/hook_config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookConfig {
    pub event: String,
    pub command: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShellHooksConfig {
    pub hooks: Vec<ShellHookConfig>,
}

impl ShellHooksConfig {
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut hooks = Vec::new();
        if !dir.exists() {
            return Self { hooks };
        }
        let config_path = dir.join("hooks.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    return config;
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.starts_with("pre_tool_call") || name.starts_with("post_tool_call")
                    || name.starts_with("pre_llm_call") || name.starts_with("post_llm_call")
                {
                    let event = name.split('.').next().unwrap_or("").to_string();
                    let command = path.to_string_lossy().to_string();
                    hooks.push(ShellHookConfig {
                        event,
                        command,
                        enabled: true,
                    });
                }
            }
        }
        Self { hooks }
    }

    pub fn default_hooks_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".axagent")
            .join("hooks")
    }

    pub fn enabled_hooks_for(&self, event: &str) -> Vec<&ShellHookConfig> {
        self.hooks
            .iter()
            .filter(|h| h.enabled && h.event == event)
            .collect()
    }
}
```

### Step 2: 创建 `shell_hooks.rs` — Shell hook 执行器

```rust
// src-tauri/crates/runtime/src/shell_hooks.rs
use crate::hook_config::ShellHooksConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookInput {
    pub event: String,
    pub tool_name: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub veto: bool,
    pub reason: Option<String>,
    pub modified_input: Option<serde_json::Value>,
}

impl ShellHookOutput {
    pub fn from_output(exit_code: i32, stdout: String, stderr: String) -> Self {
        let mut result = Self {
            exit_code,
            stdout,
            stderr,
            veto: false,
            reason: None,
            modified_input: None,
        };
        if exit_code != 0 {
            result.veto = true;
            result.reason = Some(format!("Hook exited with code {}", exit_code));
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
            if let Some(veto) = parsed.get("veto").and_then(|v| v.as_bool()) {
                result.veto = veto;
            }
            if let Some(reason) = parsed.get("reason").and_then(|v| v.as_str()) {
                result.reason = Some(reason.to_string());
            }
            if let Some(modified) = parsed.get("modified_input") {
                result.modified_input = Some(modified.clone());
            }
        }
        result
    }
}

pub struct ShellHookExecutor {
    config: ShellHooksConfig,
}

impl ShellHookExecutor {
    pub fn from_dir(dir: &Path) -> Self {
        Self {
            config: ShellHooksConfig::load_from_dir(dir),
        }
    }

    pub fn default() -> Self {
        Self::from_dir(&ShellHooksConfig::default_hooks_dir())
    }

    pub async fn execute(&self, input: ShellHookInput) -> Vec<ShellHookOutput> {
        let hooks = self.config.enabled_hooks_for(&input.event);
        let mut results = Vec::new();
        for hook in hooks {
            let json_input = serde_json::to_string(&input).unwrap_or_default();
            let result = Command::new(&hook.command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .and_then(|mut child| async {
                    if let Some(mut stdin) = child.stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        stdin.write_all(json_input.as_bytes()).await?;
                        drop(stdin);
                    }
                    let output = child.wait_with_output().await?;
                    Ok(ShellHookOutput::from_output(
                        output.status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&output.stdout).to_string(),
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ))
                })
                .await
                .unwrap_or_else(|e| ShellHookOutput {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    veto: false,
                    reason: None,
                    modified_input: None,
                });
            if result.veto {
                results.push(result);
                return results;
            }
            results.push(result);
        }
        results
    }

    pub async fn should_veto(&self, input: ShellHookInput) -> Option<String> {
        let results = self.execute(input).await;
        results.into_iter().find_map(|r| {
            if r.veto {
                r.reason
            } else {
                None
            }
        })
    }
}
```

### Step 3: 修改 `lib.rs` 注册模块

```rust
pub mod hook_config;
pub mod shell_hooks;
```

---

## Task 4: 3.3 /steer 中途干预机制 — 后端

**Files:**
- Create: `src-tauri/crates/agent/src/steer_manager.rs`
- Modify: `src-tauri/crates/agent/src/coordinator.rs`
- Modify: `src-tauri/crates/agent/src/lib.rs`

### Step 1: 创建 `steer_manager.rs`

```rust
// src-tauri/crates/agent/src/steer_manager.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerMessage {
    pub id: String,
    pub instruction: String,
    pub injected_at: chrono::DateTime<chrono::Utc>,
    pub consumed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SteerInjectionPoint {
    AfterToolCall,
    BeforeNextLlmCall,
    Immediate,
}

pub struct SteerManager {
    queue: Arc<RwLock<Vec<SteerMessage>>>,
    injection_point: Arc<RwLock<SteerInjectionPoint>>,
}

impl SteerManager {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            injection_point: Arc::new(RwLock::new(SteerInjectionPoint::AfterToolCall)),
        }
    }

    pub async fn push(&self, instruction: String) -> SteerMessage {
        let msg = SteerMessage {
            id: uuid::Uuid::new_v4().to_string(),
            instruction,
            injected_at: chrono::Utc::now(),
            consumed: false,
        };
        self.queue.write().await.push(msg.clone());
        tracing::info!("Steer message queued: {}", msg.id);
        msg
    }

    pub async fn drain_pending(&self) -> Vec<SteerMessage> {
        let mut queue = self.queue.write().await;
        let pending: Vec<SteerMessage> = queue
            .iter()
            .filter(|m| !m.consumed)
            .cloned()
            .collect();
        for msg in queue.iter_mut() {
            msg.consumed = true;
        }
        queue.retain(|m| !m.consumed);
        pending
    }

    pub async fn format_steer_block(&self) -> Option<String> {
        let pending = self.drain_pending().await;
        if pending.is_empty() {
            return None;
        }
        let instructions: Vec<String> = pending
            .iter()
            .map(|m| format!("- [{}] {}", m.id, m.instruction))
            .collect();
        Some(format!(
            "<steer-instructions type=\"temporary\">\n{}\n</steer-instructions>",
            instructions.join("\n")
        ))
    }

    pub async fn has_pending(&self) -> bool {
        self.queue.read().await.iter().any(|m| !m.consumed)
    }

    pub async fn set_injection_point(&self, point: SteerInjectionPoint) {
        *self.injection_point.write().await = point;
    }

    pub async fn clear(&self) {
        self.queue.write().await.clear();
    }
}
```

### Step 2: 修改 `coordinator.rs` — 在 execute loop 中检查 steer queue

在 `AgentCoordinator` 中添加 `steer_manager` 字段，并在 execute 方法中注入 steer 指令：

```rust
// 在 AgentCoordinator 结构体中添加:
pub steer_manager: Arc<SteerManager>,

// 在 new() 中初始化:
steer_manager: Arc::new(SteerManager::new()),

// 在 execute 方法中，在调用 impl_guard.execute() 前添加:
if self.steer_manager.has_pending().await {
    if let Some(steer_block) = self.steer_manager.format_steer_block().await {
        let mut input = input.clone();
        input.context = Some(serde_json::json!({
            "steer": steer_block,
        }));
        tracing::info!("Injecting steer instructions into agent turn");
    }
}
```

### Step 3: 修改 `lib.rs` 注册模块

```rust
pub mod steer_manager;
```

### Step 4: 添加 Tauri 命令

在 `src-tauri/src/commands/agent.rs` 中添加:
```rust
#[tauri::command]
pub async fn agent_steer(
    instruction: String,
) -> Result<(), String> {
    // steer_manager 通过 app state 获取
    Ok(())
}
```

---

## Task 5: 3.3 /steer 中途干预机制 — 前端

**Files:**
- Create: `src/components/chat/SteerInput.tsx`

### Step 1: 创建 `SteerInput.tsx`

```tsx
import { Button, Input } from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export default function SteerInput() {
  const [instruction, setInstruction] = useState("");
  const [sending, setSending] = useState(false);

  const handleSteer = async () => {
    if (!instruction.trim()) return;
    setSending(true);
    try {
      await invoke("agent_steer", { instruction });
      setInstruction("");
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="flex gap-2 items-center p-2 border-t border-default-200 bg-warning-50">
      <Input
        size="sm"
        placeholder="Steer agent direction..."
        value={instruction}
        onValueChange={setInstruction}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSteer();
          }
        }}
        className="flex-1"
      />
      <Button
        size="sm"
        color="warning"
        isLoading={sending}
        isDisabled={!instruction.trim()}
        onPress={handleSteer}
      >
        Steer
      </Button>
    </div>
  );
}
```

---

## Task 6: 3.4 AGENTS.md 上下文文件兼容 — 后端

**Files:**
- Create: `src-tauri/crates/agent/src/context_files.rs`
- Modify: `src-tauri/crates/agent/src/coordinator.rs`
- Modify: `src-tauri/crates/agent/src/lib.rs`
- Modify: `src-tauri/crates/runtime/src/git_context.rs`

### Step 1: 创建 `context_files.rs` — 上下文文件解析器

```rust
// src-tauri/crates/agent/src/context_files.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

const CONTEXT_FILE_NAMES: &[&str] = &["AGENTS.md", "CLAUDE.md", ".axagent/memory.md"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: PathBuf,
    pub name: String,
    pub content: String,
    pub format: ContextFileFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextFileFormat {
    AgentsMd,
    ClaudeMd,
    AxAgentMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFileResult {
    pub files: Vec<ContextFile>,
    pub combined_content: String,
}

pub struct ContextFileResolver {
    cache: Arc<RwLock<Option<ContextFileResult>>>,
}

impl ContextFileResolver {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn discover(&self, project_root: &Path) -> ContextFileResult {
        let mut files = Vec::new();

        self.discover_in_dir(project_root, &mut files, true);

        self.discover_subdirs(project_root, &mut files);

        let combined_content = files
            .iter()
            .map(|f| {
                format!(
                    "## Context: {} ({})\n\n{}\n",
                    f.name,
                    f.path.display(),
                    f.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        let result = ContextFileResult {
            files,
            combined_content,
        };
        *self.cache.write().await = Some(result.clone());
        result
    }

    fn discover_in_dir(&self, dir: &Path, files: &mut Vec<ContextFile>, root: bool) {
        for &name in CONTEXT_FILE_NAMES {
            let path = if name == ".axagent/memory.md" {
                dir.join(".axagent").join("memory.md")
            } else {
                dir.join(name)
            };
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let format = match name {
                        "AGENTS.md" => ContextFileFormat::AgentsMd,
                        "CLAUDE.md" => ContextFileFormat::ClaudeMd,
                        _ => ContextFileFormat::AxAgentMemory,
                    };
                    files.push(ContextFile {
                        path: path.clone(),
                        name: name.to_string(),
                        content,
                        format,
                    });
                }
            }
        }
    }

    fn discover_subdirs(&self, root: &Path, files: &mut Vec<ContextFile>) {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                    self.discover_in_dir(&entry.path(), files, false);
                }
            }
        }
    }

    pub async fn reload(&self, project_root: &Path) -> ContextFileResult {
        *self.cache.write().await = None;
        self.discover(project_root).await
    }

    pub async fn cached(&self) -> Option<ContextFileResult> {
        self.cache.read().await.clone()
    }
}
```

### Step 2: 修改 `git_context.rs` — 扩展 git 上下文包含项目级文件发现

在 `GitContext` 中添加方法:
```rust
impl GitContext {
    pub fn detect_with_context_files(cwd: &Path) -> Option<(Self, Vec<PathBuf>)> {
        let ctx = Self::detect(cwd)?;
        let mut context_files = Vec::new();
        for name in &["AGENTS.md", "CLAUDE.md"] {
            let path = cwd.join(name);
            if path.exists() {
                context_files.push(path);
            }
        }
        Some((ctx, context_files))
    }
}
```

### Step 3: 修改 `coordinator.rs` — 构建 system prompt 时加载上下文文件

在 `AgentCoordinator` 中添加 `context_resolver` 字段:
```rust
pub context_resolver: Arc<ContextFileResolver>,
```

### Step 4: 修改 `lib.rs` 注册模块

```rust
pub mod context_files;
```

---

## Task 7: 3.5 RL 训练环境集成 — 后端

**Files:**
- Create: `src-tauri/crates/trajectory/src/training_env.rs`
- Create: `src-tauri/crates/trajectory/src/trajectory_compressor.rs`
- Create: `src-tauri/crates/trajectory/src/rl_trainer.rs`
- Modify: `src-tauri/crates/trajectory/src/lib.rs`
- Modify: `src-tauri/crates/agent/src/rl_optimizer/mod.rs`

### Step 1: 创建 `training_env.rs` — 训练环境抽象

```rust
// src-tauri/crates/trajectory/src/training_env.rs
use crate::trajectory::{Trajectory, TrajectoryOutcome, TrajectoryStep};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub prompt: String,
    pub expected_outcome: Option<String>,
    pub difficulty: f64,
    pub category: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardComputation {
    pub task_completion: f64,
    pub tool_efficiency: f64,
    pub reasoning_quality: f64,
    pub error_recovery: f64,
    pub total: f64,
}

impl RewardComputation {
    pub fn from_trajectory(trajectory: &Trajectory) -> Self {
        let task_completion = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => 0.0,
        };
        let tool_count = trajectory.steps.iter()
            .filter(|s| !s.tool_calls.as_ref().map(|t| t.is_empty()).unwrap_or(true))
            .count();
        let tool_efficiency = if tool_count > 0 {
            (1.0 / (1.0 + tool_count as f64 * 0.1)).min(1.0)
        } else {
            0.5
        };
        let reasoning_steps = trajectory.steps.iter()
            .filter(|s| s.reasoning.is_some())
            .count();
        let reasoning_quality = (reasoning_steps as f64 * 0.2).min(1.0);
        let error_steps = trajectory.steps.iter()
            .filter(|s| s.tool_results.as_ref().map(|r| r.iter().any(|t| t.is_error)).unwrap_or(false))
            .count();
        let error_recovery = if error_steps > 0 { 0.3 } else { 1.0 };
        let total = task_completion * 0.4 + tool_efficiency * 0.2
            + reasoning_quality * 0.15 + error_recovery * 0.15 + 0.1;
        Self {
            task_completion,
            tool_efficiency,
            reasoning_quality,
            error_recovery,
            total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub task_id: String,
    pub trajectory_id: String,
    pub reward: RewardComputation,
    pub passed: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct TrainingEnv {
    tasks: Vec<TaskDefinition>,
}

impl TrainingEnv {
    pub fn new(tasks: Vec<TaskDefinition>) -> Self {
        Self { tasks }
    }

    pub fn tasks(&self) -> &[TaskDefinition] {
        &self.tasks
    }

    pub fn evaluate(&self, trajectory: &Trajectory) -> EvaluationResult {
        let reward = RewardComputation::from_trajectory(trajectory);
        EvaluationResult {
            task_id: trajectory.topic.clone(),
            trajectory_id: trajectory.id.clone(),
            reward: reward.clone(),
            passed: reward.total >= 0.6,
            metadata: HashMap::new(),
        }
    }
}
```

### Step 2: 创建 `trajectory_compressor.rs` — 轨迹压缩器

```rust
// src-tauri/crates/trajectory/src/trajectory_compressor.rs
use crate::trajectory::{MessageRole, Trajectory, TrajectoryStep};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedStep {
    pub role: String,
    pub content_summary: String,
    pub tool_calls: Vec<CompressedToolCall>,
    pub is_decision_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedToolCall {
    pub name: String,
    pub arguments_summary: String,
    pub result_summary: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTrajectory {
    pub id: String,
    pub session_id: String,
    pub topic: String,
    pub outcome: String,
    pub steps: Vec<CompressedStep>,
    pub decision_points: usize,
    pub compression_ratio: f64,
}

pub struct TrajectoryCompressor {
    max_content_length: usize,
}

impl TrajectoryCompressor {
    pub fn new(max_content_length: usize) -> Self {
        Self { max_content_length }
    }

    pub fn compress(&self, trajectory: &Trajectory) -> CompressedTrajectory {
        let original_steps = trajectory.steps.len();
        let mut compressed_steps = Vec::new();
        let mut decision_points = 0;

        for step in &trajectory.steps {
            let is_decision = step.tool_calls.is_some()
                || step.reasoning.is_some();
            if is_decision {
                decision_points += 1;
            }
            let tool_calls = step.tool_calls.as_ref().map(|calls| {
                calls.iter().map(|tc| CompressedToolCall {
                    name: tc.name.clone(),
                    arguments_summary: summarize(&tc.arguments, self.max_content_length),
                    result_summary: String::new(),
                    is_error: false,
                }).collect()
            }).unwrap_or_default();

            let mut compressed_tool_calls = tool_calls;
            if let Some(results) = &step.tool_results {
                for (i, result) in results.iter().enumerate() {
                    if i < compressed_tool_calls.len() {
                        compressed_tool_calls[i].result_summary = summarize(&result.output, self.max_content_length);
                        compressed_tool_calls[i].is_error = result.is_error;
                    }
                }
            }

            compressed_steps.push(CompressedStep {
                role: match step.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }.to_string(),
                content_summary: summarize(&step.content, self.max_content_length),
                tool_calls: compressed_tool_calls,
                is_decision_point: is_decision,
            });
        }

        let compression_ratio = if original_steps > 0 {
            decision_points as f64 / original_steps as f64
        } else {
            1.0
        };

        CompressedTrajectory {
            id: trajectory.id.clone(),
            session_id: trajectory.session_id.clone(),
            topic: trajectory.topic.clone(),
            outcome: format!("{:?}", trajectory.outcome).to_lowercase(),
            steps: compressed_steps,
            decision_points,
            compression_ratio,
        }
    }

    pub fn to_jsonl(&self, trajectories: &[CompressedTrajectory]) -> Result<String, serde_json::Error> {
        let lines: Result<Vec<String>, _> = trajectories
            .iter()
            .map(|t| serde_json::to_string(t))
            .collect();
        Ok(lines?.join("\n"))
    }
}

fn summarize(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let half = max_len / 2;
        format!("{}...{}", &content[..half], &content[content.len() - half..])
    }
}
```

### Step 3: 创建 `rl_trainer.rs` — RL 训练协调器

```rust
// src-tauri/crates/trajectory/src/rl_trainer.rs
use crate::training_env::{EvaluationResult, RewardComputation, TaskDefinition, TrainingEnv};
use crate::trajectory::Trajectory;
use crate::trajectory_compressor::{CompressedTrajectory, TrajectoryCompressor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub max_episodes: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub reward_threshold: f64,
    pub export_format: ExportFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Jsonl,
    Parquet,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            max_episodes: 100,
            batch_size: 32,
            learning_rate: 0.001,
            reward_threshold: 0.6,
            export_format: ExportFormat::Jsonl,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEpisode {
    pub episode_id: String,
    pub task: TaskDefinition,
    pub trajectory: Option<CompressedTrajectory>,
    pub reward: Option<RewardComputation>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub total_episodes: u32,
    pub passed: u32,
    pub failed: u32,
    pub avg_reward: f64,
    pub episodes: Vec<TrainingEpisode>,
}

pub struct RLTrainer {
    config: TrainingConfig,
    env: TrainingEnv,
    compressor: TrajectoryCompressor,
    episodes: Vec<TrainingEpisode>,
}

impl RLTrainer {
    pub fn new(config: TrainingConfig, tasks: Vec<TaskDefinition>) -> Self {
        let env = TrainingEnv::new(tasks);
        let compressor = TrajectoryCompressor::new(500);
        Self {
            config,
            env,
            compressor,
            episodes: Vec::new(),
        }
    }

    pub fn record_trajectory(&mut self, trajectory: &Trajectory) -> EvaluationResult {
        let result = self.env.evaluate(trajectory);
        let compressed = self.compressor.compress(trajectory);
        let episode = TrainingEpisode {
            episode_id: uuid::Uuid::new_v4().to_string(),
            task: TaskDefinition {
                id: trajectory.topic.clone(),
                prompt: String::new(),
                expected_outcome: None,
                difficulty: 0.5,
                category: "general".to_string(),
                metadata: HashMap::new(),
            },
            trajectory: Some(compressed),
            reward: Some(result.reward.clone()),
            passed: result.passed,
        };
        self.episodes.push(episode);
        result
    }

    pub fn export_jsonl(&self) -> Result<String, serde_json::Error> {
        let compressed: Vec<&CompressedTrajectory> = self.episodes
            .iter()
            .filter_map(|e| e.trajectory.as_ref())
            .collect();
        let lines: Result<Vec<String>, _> = compressed
            .iter()
            .map(|t| serde_json::to_string(*t))
            .collect();
        Ok(lines?.join("\n"))
    }

    pub fn report(&self) -> TrainingReport {
        let passed = self.episodes.iter().filter(|e| e.passed).count() as u32;
        let total = self.episodes.len() as u32;
        let avg_reward = if total > 0 {
            self.episodes
                .iter()
                .filter_map(|e| e.reward.as_ref().map(|r| r.total))
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };
        TrainingReport {
            total_episodes: total,
            passed,
            failed: total - passed,
            avg_reward,
            episodes: self.episodes.clone(),
        }
    }
}
```

### Step 4: 修改 `trajectory/src/lib.rs` 注册模块

```rust
mod training_env;
mod trajectory_compressor;
mod rl_trainer;

pub use training_env::*;
pub use trajectory_compressor::*;
pub use rl_trainer::*;
```

---

## Task 8: 3.6 代理中断精细控制 — 后端

**Files:**
- Create: `src-tauri/crates/agent/src/interrupt.rs`
- Modify: `src-tauri/crates/agent/src/coordinator.rs`
- Modify: `src-tauri/crates/agent/src/recovery_strategies.rs`
- Modify: `src-tauri/crates/agent/src/lib.rs`

### Step 1: 创建 `interrupt.rs` — 中断管理器

```rust
// src-tauri/crates/agent/src/interrupt.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptLevel {
    Soft,
    Hard,
    Graceful,
}

impl InterruptLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Hard => "hard",
            Self::Graceful => "graceful",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptRequest {
    pub level: InterruptLevel,
    pub reason: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptState {
    None,
    Pending(InterruptLevel),
    Processing,
    Completed,
    Recovering,
}

pub struct InterruptManager {
    state: Arc<RwLock<InterruptState>>,
    pending: Arc<RwLock<Option<InterruptRequest>>>,
    notify: Arc<Notify>,
    auto_recovery: bool,
}

impl InterruptManager {
    pub fn new(auto_recovery: bool) -> Self {
        Self {
            state: Arc::new(RwLock::new(InterruptState::None)),
            pending: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
            auto_recovery,
        }
    }

    pub async fn request(&self, level: InterruptLevel, reason: Option<String>) {
        let request = InterruptRequest {
            level,
            reason,
            timestamp: chrono::Utc::now(),
        };
        *self.pending.write().await = Some(request);
        *self.state.write().await = InterruptState::Pending(level);
        self.notify.notify_one();
        tracing::info!("Interrupt requested: level={}", level.as_str());
    }

    pub async fn check(&self) -> Option<InterruptRequest> {
        self.pending.read().await.clone()
    }

    pub async fn should_stop_current_turn(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, InterruptState::Pending(InterruptLevel::Soft)
            | InterruptState::Pending(InterruptLevel::Hard)
            | InterruptState::Pending(InterruptLevel::Graceful))
    }

    pub async fn should_preserve_session(&self) -> bool {
        let pending = self.pending.read().await;
        matches!(pending.as_ref().map(|p| p.level),
            Some(InterruptLevel::Soft) | Some(InterruptLevel::Graceful))
    }

    pub async fn begin_processing(&self) {
        *self.state.write().await = InterruptState::Processing;
    }

    pub async fn complete(&self) {
        if self.auto_recovery {
            *self.state.write().await = InterruptState::Recovering;
            tracing::info!("Interrupt completed, auto-recovery enabled");
        } else {
            *self.state.write().await = InterruptState::Completed;
        }
        *self.pending.write().await = None;
    }

    pub async fn recover(&self) {
        *self.state.write().await = InterruptState::None;
        *self.pending.write().await = None;
        tracing::info!("Interrupt recovery completed");
    }

    pub async fn state(&self) -> InterruptState {
        *self.state.read().await
    }

    pub fn notified(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub async fn soft_stop(&self) {
        self.request(InterruptLevel::Soft, Some("User requested soft stop".to_string())).await;
    }

    pub async fn hard_stop(&self) {
        self.request(InterruptLevel::Hard, Some("User requested hard stop".to_string())).await;
    }

    pub async fn graceful_stop(&self) {
        self.request(InterruptLevel::Graceful, Some("User requested graceful stop".to_string())).await;
    }
}
```

### Step 2: 修改 `coordinator.rs` — 中断状态管理 + 自动恢复逻辑

在 `AgentCoordinator` 中添加 `interrupt_manager` 字段:
```rust
pub interrupt_manager: Arc<InterruptManager>,
```

在 `cancel` 方法中修改为使用中断管理器:
```rust
pub async fn cancel(&self) -> Result<(), AgentError> {
    self.interrupt_manager.soft_stop().await;
    {
        let mut impl_guard = self.implementation.lock().await;
        impl_guard.cancel().await?;
    }
    let mut status = self.status.write().await;
    *status = AgentStatus::Idle;
    self.interrupt_manager.complete().await;
    Ok(())
}
```

### Step 3: 修改 `recovery_strategies.rs` — 添加连接中断后自动恢复策略

```rust
// 添加新的恢复策略变体:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    // ... existing variants ...
    AutoRecover {
        max_attempts: usize,
        checkpoint_interval_secs: u64,
    },
}

impl RecoveryStrategy {
    pub fn for_interrupt() -> Self {
        RecoveryStrategy::AutoRecover {
            max_attempts: 3,
            checkpoint_interval_secs: 30,
        }
    }
}
```

### Step 4: 修改 `lib.rs` 注册模块

```rust
pub mod interrupt;
```

---

## 验收标准汇总

- [ ] 创建新 profile 完全隔离配置、会话历史、技能
- [ ] 启动时通过命令行 `--profile <name>` 或 UI 切换
- [ ] 不同 profile 的 API 密钥完全独立
- [ ] `~/.axagent/hooks/pre_tool_call.sh` 在每次工具调用前执行
- [ ] hook 脚本通过 stdin 接收 JSON 上下文，stdout 返回结果
- [ ] hook 脚本返回 `{"veto": true}` 可阻止工具执行
- [ ] 代理运行中输入 `/steer <instruction>`，代理在下个工具调用后看到该指令
- [ ] steer 不中断当前 turn，不破坏 prompt cache
- [ ] steer 消息在 system prompt 中标注为临时注入
- [ ] 项目根目录的 AGENTS.md 自动注入到 system prompt
- [ ] CLAUDE.md 格式兼容读取
- [ ] 多层目录（root + subdir）上下文文件叠加
- [ ] `/context reload` 命令重新加载
- [ ] 轨迹数据可导出为标准训练格式（JSONL）
- [ ] 压缩后的轨迹保留关键决策点
- [ ] 奖励信号计算管线可用
- [ ] `/stop` 不重置 session（只停止当前 turn）
- [ ] 网关重启后自动恢复未完成的代理任务
- [ ] 中断响应延迟 < 1s
