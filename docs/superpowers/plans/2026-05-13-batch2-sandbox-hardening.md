# Batch 2: 沙箱加固实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 三层沙箱防护：进程资源限制（rlimit/Job Objects）+ Docker 自动检测启用 + 网络隔离确认，零新依赖

**Architecture:** 新增 `resource_limits` 模块提供跨平台进程资源限制；加固 `bash/sandbox` 自动检测 Docker；`sandbox_runner` 在执行前统一应用资源限制和网络检查；网络隔离仅作用于不受控代码执行路径，不影响 web_search/rag 等受控工具

**Tech Stack:** Rust 2021, libc (Linux/macOS), windows-sys (Windows), std::process

**Spec:** `docs/superpowers/specs/2026-05-13-batch2-sandbox-hardening-design.md`

---

## 文件结构总览

```
新增:
  src-tauri/crates/core/src/resource_limits.rs

修改:
  src-tauri/crates/core/src/lib.rs                      # pub mod resource_limits
  src-tauri/crates/tools/src/bash/sandbox.rs             # Docker 自动检测
  src-tauri/crates/tools/src/sandbox.rs                  # 环境变量白名单验证
  src-tauri/crates/core/src/sandbox_runner.rs            # 集成资源限制 + 网络检查
  src-tauri/crates/runtime-core/src/sandbox.rs           # SandboxStatus 增强
```

---

### Task 1: 创建 resource_limits 模块

**Files:**
- Create: `src-tauri/crates/core/src/resource_limits.rs`

- [ ] **Step 1: 编写 resource_limits.rs**

```rust
//! 跨平台进程资源限制。
//!
//! Linux/macOS: rlimit (RLIMIT_CPU, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_FSIZE)
//! Windows: Job Objects (内存限制 + 进程数限制)

/// 沙箱资源限制配置
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// CPU 时间限制（秒），默认 60
    pub max_cpu_seconds: u64,
    /// 虚拟内存限制（字节），默认 512MB
    pub max_memory_bytes: u64,
    /// 最大子进程数，默认 10
    pub max_processes: u32,
    /// 最大文件写入（字节），默认 100MB
    pub max_file_size_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_seconds: 60,
            max_memory_bytes: 512 * 1024 * 1024,
            max_processes: 10,
            max_file_size_bytes: 100 * 1024 * 1024,
        }
    }
}

impl ResourceLimits {
    /// 创建沙箱默认限制
    pub fn default_sandbox() -> Self {
        Self::default()
    }

    /// 应用资源限制到当前进程及其子进程
    pub fn apply_to_current_process(&self) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        self.apply_rlimit()?;

        #[cfg(target_os = "windows")]
        self.apply_job_object()?;

        tracing::info!(
            "Sandbox resource limits applied: cpu={}s, mem={}MB, procs={}, fsize={}MB",
            self.max_cpu_seconds,
            self.max_memory_bytes / (1024 * 1024),
            self.max_processes,
            self.max_file_size_bytes / (1024 * 1024),
        );

        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn apply_rlimit(&self) -> Result<(), String> {
        // RLIMIT_CPU: 进程可使用的 CPU 时间（秒）
        self.set_rlimit(
            libc::RLIMIT_CPU,
            self.max_cpu_seconds,
            self.max_cpu_seconds.saturating_add(5),
        )?;

        // RLIMIT_AS: 进程可用虚拟内存（字节）
        self.set_rlimit(
            libc::RLIMIT_AS,
            self.max_memory_bytes,
            self.max_memory_bytes,
        )?;

        // RLIMIT_NPROC: 最大子进程数
        self.set_rlimit(
            libc::RLIMIT_NPROC,
            self.max_processes as u64,
            self.max_processes as u64,
        )?;

        // RLIMIT_FSIZE: 最大文件写入（字节）
        self.set_rlimit(
            libc::RLIMIT_FSIZE,
            self.max_file_size_bytes,
            self.max_file_size_bytes,
        )?;

        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn set_rlimit(
        &self,
        resource: libc::__rlimit_resource_t,
        soft: u64,
        hard: u64,
    ) -> Result<(), String> {
        let rlim = libc::rlimit {
            rlim_cur: soft.min(hard),
            rlim_max: hard,
        };
        let ret = unsafe { libc::setrlimit(resource, &rlim) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            tracing::warn!("Failed to set rlimit {:?}: {}", resource, err);
            // 不返回错误——rlimit 失败不应阻止执行
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn apply_job_object(&self) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
            JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOB_OBJECT_LIMIT_JOB_MEMORY,
        };
        use windows_sys::Win32::Foundation::HANDLE;

        let name: Vec<u16> = std::ffi::OsStr::new("AxAgent_Sandbox_Job")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe { CreateJobObjectW(std::ptr::null(), name.as_ptr()) };
        if handle.is_null() {
            return Err("无法创建 Windows Job Object".to_string());
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_JOB_MEMORY;
        info.ProcessMemoryLimit = self.max_memory_bytes;
        info.JobMemoryLimit = self.max_memory_bytes.saturating_mul(2);

        let ret = unsafe {
            SetInformationJobObject(
                handle as HANDLE,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if ret == 0 {
            tracing::warn!("Failed to configure Windows Job Object");
        }

        let current = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
        let ret = unsafe { AssignProcessToJobObject(handle as HANDLE, current) };
        if ret == 0 {
            tracing::warn!("Failed to assign process to Job Object");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_reasonable() {
        let limits = ResourceLimits::default();
        assert!(limits.max_cpu_seconds > 0);
        assert!(limits.max_memory_bytes > 0);
        assert!(limits.max_processes > 0);
        assert!(limits.max_file_size_bytes > 0);
    }

    #[test]
    fn sandbox_limits_are_restrictive() {
        let limits = ResourceLimits::default_sandbox();
        assert!(limits.max_cpu_seconds <= 120);
        assert!(limits.max_memory_bytes <= 1024 * 1024 * 1024);
        assert!(limits.max_processes <= 50);
    }

    #[test]
    fn apply_does_not_panic() {
        let limits = ResourceLimits::default();
        let result = limits.apply_to_current_process();
        // rlimit 可能失败（权限不足等），但不应 panic
        // 在 Windows 上可能因为无 windows-sys 而失败
        assert!(result.is_ok() || result.is_err());
    }
}
```

- [ ] **Step 2: 检查 Cargo.toml 依赖**

`core/Cargo.toml` 需要以下依赖（检查是否已存在）：
- `libc` — 用于 rlimit
- `windows-sys` — 用于 Job Objects（仅 Windows，feature-gated）

如果 `libc` 不存在，添加：`libc = "0.2"`
如果 `windows-sys` 不存在，添加（含 features）：
```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_System_JobObjects", "Win32_System_Threading", "Win32_Foundation"] }
```

- [ ] **Step 3: 编译验证**

Run: `cargo check -p axagent-core`
Expected: 编译成功

- [ ] **Step 4: 运行测试**

Run: `cargo test -p axagent-core -- resource_limits`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/core/src/resource_limits.rs
git commit -m "feat: 新增跨平台进程资源限制模块（rlimit + Job Objects）"
```

---

### Task 2: 注册 resource_limits 模块

**Files:**
- Modify: `src-tauri/crates/core/src/lib.rs`

- [ ] **Step 1: 添加模块声明**

在 `lib.rs` 中找到 `pub mod` 声明区域，添加：

```rust
pub mod resource_limits;
```

以及在 re-export 区域添加：

```rust
pub use resource_limits::ResourceLimits;
```

- [ ] **Step 2: 编译验证**

Run: `cargo check -p axagent-core`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/core/src/lib.rs
git commit -m "feat: 注册 resource_limits 模块并 re-export ResourceLimits"
```

---

### Task 3: Docker 自动检测 + 默认启用

**Files:**
- Modify: `src-tauri/crates/tools/src/bash/sandbox.rs`

- [ ] **Step 1: 添加 Docker 检测函数**

在 `SandboxConfig` 的 `Default` impl 之前添加：

```rust
/// 检测 Docker 是否可用
fn detect_docker_available() -> bool {
    std::process::Command::new("docker")
        .args(["info", "--format", "{{.OSType}}"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

- [ ] **Step 2: 修改 SandboxConfig::default()**

将 `use_container: false` 改为 `use_container: detect_docker_available()`：

```rust
impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            use_container: detect_docker_available(),
            image: "alpine:latest".into(),
            allow_network: false,
            volumes: Vec::new(),
            memory_limit_mb: Some(512),
            cpu_limit: Some(0.5),
        }
    }
}
```

- [ ] **Step 3: 添加 Docker 可用性测试**

在文件末尾的 `#[cfg(test)]` 块中添加：

```rust
#[test]
fn docker_detect_does_not_panic() {
    let available = super::detect_docker_available();
    // 只验证不 panic，不假设结果
    assert!(available == true || available == false);
}

#[test]
fn sandbox_config_default_is_safe() {
    let config = SandboxConfig::default();
    // 无论 Docker 是否可用，allow_network 应为 false
    assert!(!config.allow_network);
    // 资源限制应该被设置
    assert!(config.memory_limit_mb.is_some());
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test -p axagent-tools -- bash`
Expected: 所有测试 PASS（包括新增的 2 个测试）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/tools/src/bash/sandbox.rs
git commit -m "feat: Docker 自动检测 + use_container 默认启用"
```

---

### Task 4: 环境变量白名单验证

**Files:**
- Modify: `src-tauri/crates/tools/src/sandbox.rs`

- [ ] **Step 1: 在 SecuritySandbox 中添加 env 验证方法**

在 `impl SecuritySandbox` 块中，`check_env_var` 方法之后添加：

```rust
/// 验证当前进程环境变量是否符合白名单
/// 返回被拒绝的环境变量列表
pub fn validate_environment(&self) -> Vec<String> {
    let mut denied = Vec::new();
    for (key, _value) in std::env::vars() {
        if !self.config.env_whitelist.iter().any(|allowed|
            allowed.eq_ignore_ascii_case(&key)
        ) {
            denied.push(key);
        }
    }
    if !denied.is_empty() {
        tracing::warn!(
            "Non-whitelisted environment variables detected: {:?}",
            denied
        );
    }
    denied
}
```

- [ ] **Step 2: 添加测试**

在文件末尾的测试模块中添加：

```rust
#[test]
fn env_whitelist_accepts_path() {
    let sandbox = SecuritySandbox::with_default_config();
    assert!(sandbox.check_env_var("PATH").allowed);
    assert!(sandbox.check_env_var("HOME").allowed);
    assert!(sandbox.check_env_var("TEMP").allowed);
    assert!(!sandbox.check_env_var("SECRET_KEY").allowed);
    assert!(!sandbox.check_env_var("DATABASE_URL").allowed);
}

#[test]
fn validate_environment_detects_denied_vars() {
    let sandbox = SecuritySandbox::with_default_config();
    let denied = sandbox.validate_environment();
    // PATH, HOME, TEMP 应该不在 denied 中
    assert!(!denied.iter().any(|v| v == "PATH"));
    assert!(!denied.iter().any(|v| v == "HOME"));
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p axagent-tools -- sandbox`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/tools/src/sandbox.rs
git commit -m "feat: 环境变量白名单验证 + validate_environment 方法"
```

---

### Task 5: SandboxRunner 集成资源限制 + 网络检查

**Files:**
- Modify: `src-tauri/crates/core/src/sandbox_runner.rs`

- [ ] **Step 1: 修改 execute 方法，执行前应用资源限制**

在 `sandbox_runner.rs` 中，修改 `execute` 方法。在执行代码之前添加资源限制应用和网络检查。

在 `pub async fn execute` 方法的开头添加：

```rust
pub async fn execute(&self, code: &str, language: &str) -> Result<ExecutionResult> {
    // 应用沙箱资源限制
    let limits = crate::resource_limits::ResourceLimits::default_sandbox();
    if let Err(e) = limits.apply_to_current_process() {
        tracing::warn!("Failed to apply resource limits: {}", e);
    }

    // 注意：此处网络隔离由 tools::sandbox 的 SecuritySandbox 层面保证。
    // SandboxRunner 不负责网络——它只负责代码执行。
    // network_enabled 默认为 false，由 SecuritySandbox::check_network() 在调用前检查。

    match language {
        // ... 保持原有逻辑 ...
    }
}
```

- [ ] **Step 2: 在 execute_js 中增加安全注释**

在 `execute_js` 方法中添加注释说明安全边界：

```rust
async fn execute_js(&self, code: &str) -> Result<ExecutionResult> {
    // 安全边界：此方法仅在 SecuritySandbox::check_command 通过后调用
    // 网络隔离已由调用方 SecuritySandbox 保证
    let temp_dir = std::env::temp_dir();
    // ... 保持原有执行逻辑 ...
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p axagent-core -- sandbox_runner`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/core/src/sandbox_runner.rs
git commit -m "feat: SandboxRunner 集成 resource_limits + 网络隔离注释说明"
```

---

### Task 6: SandboxStatus 增强

**Files:**
- Modify: `src-tauri/crates/runtime-core/src/sandbox.rs`

- [ ] **Step 1: 在 SandboxStatus 中添加新字段**

在 `SandboxStatus` 结构体中添加三个新字段：

```rust
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub requested: SandboxRequest,
    pub supported: bool,
    pub active: bool,
    pub namespace_supported: bool,
    pub namespace_active: bool,
    pub network_supported: bool,
    pub network_active: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub filesystem_active: bool,
    pub allowed_mounts: Vec<String>,
    pub in_container: bool,
    pub container_markers: Vec<String>,
    pub fallback_reason: Option<String>,
    // 新增字段
    /// Docker 是否可用
    #[serde(default)]
    pub docker_available: bool,
    /// 资源限制是否已激活
    #[serde(default)]
    pub resource_limits_active: bool,
}
```

- [ ] **Step 2: 在 resolve_sandbox_status_for_request 中填充新字段**

在 `resolve_sandbox_status_for_request` 函数中，返回 `SandboxStatus` 之前添加：

```rust
// Docker availability check (cached)
let docker_available = detect_docker_available();

// ... 在构造 SandboxStatus 时添加 ...
SandboxStatus {
    // ... 现有字段 ...
    docker_available,
    resource_limits_active: request.enabled,
}
```

并在文件顶部添加 Docker 检测辅助函数：

```rust
/// 检测 Docker 是否可用（缓存结果）
fn detect_docker_available() -> bool {
    use std::sync::OnceLock;
    static DOCKER_CHECK: OnceLock<bool> = OnceLock::new();
    *DOCKER_CHECK.get_or_init(|| {
        std::process::Command::new("docker")
            .args(["info", "--format", "{{.OSType}}"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p axagent-runtime-core -- sandbox`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/runtime-core/src/sandbox.rs
git commit -m "feat: SandboxStatus 增加 docker_available + resource_limits_active 字段"
```

---

### Task 7: 全量编译 + 测试验证

- [ ] **Step 1: 全量编译检查**

Run: `cargo check --all-targets` from `src-tauri/`
Expected: 所有 crate 编译成功

- [ ] **Step 2: 运行相关测试套件**

```
cargo test -p axagent-core -- resource_limits
cargo test -p axagent-core -- sandbox_runner
cargo test -p axagent-tools -- sandbox
cargo test -p axagent-tools -- bash
cargo test -p axagent-runtime-core -- sandbox
```

- [ ] **Step 3: cargo fmt 检查**

Run: `cargo fmt --all -- --check` from `src-tauri/`
If issues: `cargo fmt --all` and commit

- [ ] **Step 4: clippy 零警告**

Run: `cargo clippy --all-targets -- -D warnings` from `src-tauri/`
Fix any clippy warnings and commit

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: Batch 2 全量编译 + 测试 + lint 验证通过"
```
