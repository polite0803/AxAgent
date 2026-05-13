# Batch 1: 安全加固设计文档

> 提示词注入防护 + 权限路径修复
> 日期：2026-05-13 | 状态：待实现 | 批次：1/3

## 1. 背景与目标

AxAgent 安全审计发现 5 类缺陷，按优先级分 3 批修复。第 1 批聚焦攻击面最广的两个领域：

| 缺陷 | 严重度 | 核心问题 |
|------|--------|---------|
| 提示词注入 | 高 | 用户输入零清理、外部数据无信任边界、无协议级隔离 |
| 权限路径越界 | 中 | 字符串前缀可被 `..` 绕过、TOCTOU 竞态、DangerFullAccess 无防护 |

目标：建立纵深防御体系，覆盖所有数据入口，消除已知路径绕过漏洞。

## 2. 提示词注入防护（方案 B：纵深防御）

### 2.1 新增 crate: `prompt-guard`

独立安全模块，不耦合业务逻辑：

```
crates/prompt-guard/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # pub mod declarations
│   ├── pipeline.rs             # 4 级过滤引擎编排
│   ├── detectors/
│   │   ├── mod.rs
│   │   ├── pattern_detect.rs   # L1: 注入模式正则匹配
│   │   ├── delimiter_escape.rs # L2: XML 分隔符转义
│   │   └── token_smuggling.rs  # 附加: token smuggling 检测
│   ├── wrappers.rs             # L3: XML 包装器
│   ├── trust_labels.rs         # L4: 外部数据信任标签
│   └── config.rs               # 防护配置
└── tests/
    ├── injection_basic.rs
    ├── injection_nested.rs
    ├── injection_unicode.rs
    ├── injection_jailbreak.rs
    ├── injection_token_smuggling.rs
    └── false_positive.rs
```

### 2.2 4 级过滤 Pipeline

```
user_input → [L1: PatternDetect] → [L2: DelimiterEscape] → [L3: XmlWrapper] → LLM context
external_data → [L4: TrustLabels] → [L2] → [L3] → LLM context
```

**L1 — 模式检测 (PatternDetect)**

检测已知注入模式，按风险分级处理：

| 风险等级 | 模式示例 | 动作 |
|---------|---------|------|
| 高 | `ignore (all )?previous (instructions\|directives)` | 拒绝 |
| 高 | `you are now` / `pretend you are` / `act as` | 拒绝 |
| 高 | `system:` / `<system>` 角色伪造 | 拒绝 |
| 高 | `DAN` / `jailbreak` 已知攻击向量 | 拒绝 |
| 中 | `---END OF SYSTEM---` 分隔符注入 | 标记 + 警告日志 |
| 低 | `as a [role]` 角色暗示 | 仅标记 |

**L2 — 分隔符转义 (DelimiterEscape)**

- 转义用户输入中的 XML 元字符（`<` → 对应实体或 CDATA）
- 检测并阻止嵌套 XML 标签
- 处理 Unicode 同形字绕过（full-width `＜` `＞`）

**L3 — XML 包装器 (XmlWrapper)**

将清理后的内容包装为结构化格式：

```xml
<user_query role="user" trusted="false" sanitized="true">
  [cleaned content]
</user_query>
```

System prompt 中包含对应的指令，明确 `<user_query>` 内外文本的优先级。

**L4 — 信任标签 (TrustLabels)**

为外部数据源注入来源标注：

| 数据源 | 标签 |
|--------|------|
| RAG 知识库 | `[UNTRUSTED-SOURCE:rag/kb-{id}]` |
| CLAUDE.md 等指令文件 | `[EXTERNAL-INSTRUCTIONS:{path}]` |
| 网页抓取 | `[UNTRUSTED-SOURCE:web/{domain}]` |
| Git status/diff/log | `[CONTEXT:git]` |

### 2.3 修改点

| # | 文件 | 变更说明 |
|---|------|---------|
| 1 | `crates/prompt-guard/` (新增) | 新建 crate，实现全部防护逻辑 |
| 2 | `crates/runtime-core/src/session.rs` | `push_user_text()` 调用 pipeline 过滤 |
| 3 | `crates/runtime/src/prompt.rs` | System prompt 加入用户/系统分隔指令 |
| 4 | `crates/runtime/src/git_context.rs` | Git 信息注入前加信任标签 |
| 5 | `src-tauri/src/context_manager.rs` | RAG 上下文注入前加信任标签 |
| 6 | `crates/agent/src/session_manager.rs` | Agent 上下文组装时集成 pipeline |
| 7 | `crates/providers/src/anthropic.rs` | Anthropic 启用 API 级 system param（增强） |
| 8 | `Cargo.toml` (workspace) | 添加 prompt-guard 成员 |

### 2.4 Anthropic Provider 增强

对于 Anthropic 模型，额外在 API 层使用 `system` 参数传递系统指令，实现协议级隔离。其他 provider 使用 XML 标记 + 信任标签作为通用方案。

### 2.5 安全测试

| 测试 | 攻击向量 | 预期 |
|------|---------|------|
| injection_basic | `ignore previous instructions` | 拒绝 |
| injection_nested | `</user_query>malicious<user_query>` | 转义后安全通过 |
| injection_unicode | `＜user_query＞` 全角标签 | 检测并处理 |
| injection_jailbreak | DAN 角色扮演劫持 | 拒绝 |
| injection_token_smuggling | token smuggling 绕过 | 检测 |
| false_positive | 合法技术讨论中的关键字符串 | 不过滤 |

## 3. 权限路径修复

### 3.1 修复 `is_within_workspace()` — 字符串前缀 → canonicalize 比较

**旧实现（有漏洞）：**

```rust
fn is_within_workspace(path: &str, workspace_root: &str) -> bool {
    let normalized = if path.starts_with('/') { path.to_owned() }
                     else { format!("{}/{}", workspace_root, path) };
    normalized.starts_with(&root) || normalized == workspace_root
}
```

**绕过方式：** `/workspace/../etc/passwd` — 字符串以 `/workspace/..` 开头，不匹配 `/workspace/` 前缀，但也不触发 canonicalize。

**新实现：**

```rust
fn is_within_workspace(path: &str, workspace_root: &str) -> bool {
    if path.is_empty() || path.contains('\0') { return false; }
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let canonical_root = match std::fs::canonicalize(workspace_root) {
        Ok(p) => p,
        Err(_) => return false,
    };
    canonical.starts_with(&canonical_root) || canonical == canonical_root
}
```

### 3.2 修复 `is_path_safe()` — TOCTOU 缓解

**旧实现：** 先 `is_symlink()` 再 `canonicalize()`，两次系统调用之间存在明显 TOCTOU 窗口。

**新实现：** 先 canonicalize 再验证，缩小攻击窗口：

```rust
fn validate_path_atomic(path: &Path) -> Result<PathBuf, PathError> {
    let real = std::fs::canonicalize(path)
        .map_err(|_| PathError::InvalidPath)?;
    if real != path && path.is_symlink() {
        return Err(PathError::SymlinkDenied);
    }
    Ok(real)
}
```

注意：canonicalize 与 is_symlink 之间仍有微小 TOCTOU 窗口，无法在此次修复中完全消除（彻底修复需要 OS 级 openat2/RESOLVE_NO_SYMLINKS）。这是已知的可接受残余风险，在后续批次中如有必要可通过 seccomp 规则进一步加固。

### 3.3 DangerFullAccess 审计增强

不再静默放行所有操作：

```rust
fn check_file_write_danger(path: &str, workspace_root: &str) -> EnforcementResult {
    let outside = !is_within_workspace(path, workspace_root);
    if outside {
        tracing::warn!("DANGER: file write outside workspace: path={}", path);
    }
    EnforcementResult::AllowedWithAudit { outside_workspace: outside }
}
```

- `EnforcementResult` 新增 `AllowedWithAudit` 变体
- 前端在 DangerFullAccess 模式下展示警告横幅
- 敏感路径（`/etc`, `/proc`, `C:\Windows`）额外 WARN 级别日志

### 3.4 修改点

| # | 文件 | 变更说明 |
|---|------|---------|
| 1 | `crates/runtime-core/src/permission_enforcer.rs` | 重写 `is_within_workspace()`，canonicalize 后比较 |
| 2 | `crates/runtime-core/src/permission_enforcer.rs` | DangerFullAccess 增加 `AllowedWithAudit` + 审计日志 |
| 3 | `crates/core/src/file_authorizer.rs` | `is_path_safe()` 原子化 |
| 4 | `crates/tools/src/bash/path_validation.rs` | canonicalize 前检查 null 字节 + Windows 路径防护 |
| 5 | `crates/runtime-core/src/lib.rs` | `EnforcementResult` 添加 `AllowedWithAudit` 变体 |
| 6 | `src-tauri/src/commands/agent.rs` | 权限变更时推送前端警告 |

### 3.5 测试验证

| 测试用例 | 预期 |
|---------|------|
| `../etc/passwd` 路径遍历 | 拒绝 |
| `/workspace\x00/etc` null 字节 | 拒绝 |
| 符号链接 `/workspace/link → /etc` | 拒绝 |
| 合法路径 `/workspace/src/main.rs` | 允许 |
| Windows `C:\Windows\..\workspace` | 拒绝 |

## 4. 依赖关系

```
prompt-guard (新 crate)
    ├── 依赖: regex, unicase (unicode 检测)
    └── 被依赖: runtime-core, agent, runtime

permission_enforcer (修改)
    └── 被依赖: runtime-core (lib.rs), tools/bash, core/file_authorizer

EnforcementResult::AllowedWithAudit (新增变体)
    ├── 定义: runtime-core/lib.rs
    ├── 生产者: permission_enforcer.rs
    └── 消费者: agent/commands, 前端 IPC 事件
```

## 5. 风险与回滚

| 风险 | 缓解措施 |
|------|---------|
| L1 误拦合法消息 | 仅高风险模式做硬拒绝，中低风险仅标记 |
| canonicalize 性能 | 仅在工作区边界检查时调用，不影响普通读写 |
| Anthropic system param 兼容性 | feature-gated，不影响其他 provider |
| 前端警告横幅过多 | DangerFullAccess 模式才展示，其余模式不展示 |

如需回滚，此次所有改动集中在 1 个新 crate（可整体删除）和有限的现有文件修改（git revert）。
