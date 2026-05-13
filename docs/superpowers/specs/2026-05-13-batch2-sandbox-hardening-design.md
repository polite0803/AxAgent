# Batch 2: 沙箱加固设计文档

> 进程资源限制 + Docker 自动检测 + 网络隔离
> 日期：2026-05-13 | 状态：待实现 | 批次：2/3

## 1. 背景与目标

AxAgent 安全审计发现沙箱为纯逻辑沙箱，无系统级隔离：
- 命令通过 `bash -c` / `cmd /C` 在宿主机直接执行
- `use_container` 默认为 `false`（已有 Docker 代码路径但未启用）
- 无资源限制（CPU/内存/进程数），易受 fork 炸弹和内存耗尽攻击
- JS/Python 代码通过 `node` / `python` 直接在宿主机执行

目标：三层防护，不引入新依赖，把已有代码用到位。

## 2. 设计哲学

| 原则 | 说明 |
|------|------|
| 零新依赖 | 使用内核 API（rlimit/Job Objects），不强制要求 Docker |
| 利用已有代码 | `build_docker_command()` 已存在，仅改默认值 + 自动检测 |
| 安全降级 | Docker 可用 → 容器模式；不可用 → 逻辑沙箱 + 前端警告 |

## 3. 三层防护架构

```
┌──────────────────────────────────────────────┐
│              第 1 层：进程资源限制              │
│  rlimit (RLIMIT_CPU/AS/NPROC/FSIZE)          │
│  Job Objects (Windows)                       │
│  作用：防 fork 炸弹、内存耗尽、CPU 占用          │
├──────────────────────────────────────────────┤
│              第 2 层：沙箱配置硬化              │
│  Docker 自动检测 → 可用则启用容器模式            │
│  不可用 → 逻辑沙箱 + 前端警告横幅               │
│  环境变量白名单（仅 PATH/HOME/TEMP）            │
├──────────────────────────────────────────────┤
│              第 3 层：网络隔离                  │
│  所有代码执行路径 network_enabled = false       │
│  Bash / JS / Python 统一禁网                   │
└──────────────────────────────────────────────┘
```

### 3.1 第 1 层：进程资源限制

新增 `crates/core/src/resource_limits.rs`：

```rust
pub struct ResourceLimits {
    pub max_cpu_seconds: u64,     // CPU 时间限制，默认 60s
    pub max_memory_bytes: u64,    // 虚拟内存限制，默认 512MB
    pub max_processes: u32,       // 最大子进程数，默认 10
    pub max_file_size_bytes: u64, // 最大文件写入，默认 100MB
}

impl ResourceLimits {
    pub fn default_sandbox() -> Self { ... }
    pub fn apply_to_current_process(&self) -> Result<(), String> { ... }
}
```

**Linux/macOS**: 使用 `libc::setrlimit` 设置 RLIMIT_CPU, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_FSIZE。
**Windows**: 使用 `Job Objects` API（`CreateJobObjectW`, `SetInformationJobObject`）限制进程内存和工作集。

### 3.2 第 2 层：Docker 自动检测

修改 `tools/bash/sandbox.rs` 的 `SandboxConfig::default()`：

```rust
// 旧：use_container: false
// 新：use_container: detect_docker_available()

fn detect_docker_available() -> bool {
    std::process::Command::new("docker")
        .args(["info", "--format", "{{.OSType}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

同时在 `tools/sandbox.rs` 的 `SecuritySandbox` 中增加环境变量白名单验证：仅透传 `PATH`, `HOME`, `TEMP`。

### 3.3 第 3 层：网络默认禁用

- `SecuritySandbox` 的 `network_enabled` 当前默认 `false`，保持不变
- 确保 `core/sandbox_runner.rs` 的 JS/Python 执行路径也继承此设置
- 新增 `check_network()` 调用点在 `SandboxRunner::execute()` 入口

### 3.4 SandboxStatus 增强

在 `runtime-core/src/sandbox.rs` 的 `SandboxStatus` 中增加字段：

```rust
pub struct SandboxStatus {
    // ... 现有字段 ...
    pub docker_available: bool,
    pub resource_limits_active: bool,
    pub resource_limits_config: Option<ResourceLimitsConfig>,
}
```

## 4. 修改点

| # | 文件 | 变更说明 |
|---|------|---------|
| 1 | `crates/core/src/resource_limits.rs` (新增) | 进程资源限制模块 |
| 2 | `crates/core/src/lib.rs` | `pub mod resource_limits` + re-export |
| 3 | `crates/tools/src/bash/sandbox.rs` | Docker 自动检测 + `use_container` 默认开启 |
| 4 | `crates/tools/src/sandbox.rs` | 环境变量白名单 + 资源限制配置集成 |
| 5 | `crates/core/src/sandbox_runner.rs` | 执行前应用资源限制 + 强制网络禁用 |
| 6 | `crates/runtime-core/src/sandbox.rs` | SandboxStatus 增加 docker_available / resource_limits_active |

## 5. 测试计划

| 测试 | 内容 | 预期 |
|------|------|------|
| resource_limits_apply | Linux: rlimit 设置后读取验证 | 限制值生效 |
| resource_limits_default | 默认沙箱限制值合理性 | 符合预设 |
| docker_detect | 模拟 Docker 可用/不可用 | 正确检测 |
| sandbox_fallback | Docker 不可用时回退逻辑 | 回退 + 警告 |
| env_whitelist | 环境变量白名单验证 | 非白名单变量被拒绝 |
| network_disabled | 所有执行路径网络禁用 | check_network() 返回 false |
| sandbox_runner_resource | JS 执行前资源限制应用 | 限制生效 |

## 6. 依赖关系

```
resource_limits (新增 core 模块)
    ├── 依赖: libc (Linux/macOS), windows-sys (Windows)
    └── 被依赖: sandbox_runner, bash/sandbox

bash/sandbox (修改)
    ├── Docker 检测: std::process::Command
    └── 被依赖: tools/sandbox, runtime-core/sandbox

sandbox_runner (修改)
    ├── 集成 resource_limits
    └── 集成 network check
```

## 7. 风险与回滚

| 风险 | 缓解 |
|------|------|
| rlimit 设置过严导致正常工具被 kill | 使用宽松默认值（60s CPU, 512MB 内存），可通过 SandboxConfig 调整 |
| Docker 检测每次执行都调 `docker info` 太慢 | 启动时检测一次，缓存结果到 SandboxStatus |
| Windows Job Objects API 兼容性 | feature-gated，仅在 Windows 平台编译 |
| 网络禁用影响需要网络的合法工具 | 白名单机制：`allowed_domains` 配置 |

如需回滚，此次改动全部在已有沙箱框架内，可以逐个还原。
