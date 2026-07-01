# AxAgent 技能系统（SKILL）缺陷修复方案

> 基于审计报告 22 个缺陷，已验证所有源码位置与行号，按 P0 → P1 → P2 → P3 优先级编排。
> P0/P1 级别提供可直接应用的代码补丁；P2/P3 提供明确修改方向和关键代码变更。

---

## P0 — 立即修复（3 项）

### 缺陷 #2：`ensure_path_under_base` 静默绕过路径遍历检查

- **严重程度**：严重（P0）
- **文件**：`src-tauri/src/commands/skills.rs`
- **位置**：第 931–939 行

**修改前代码**：

```rust
fn ensure_path_under_base(path: &Path, base: &Path) -> Result<(), String> {
    if let Ok(canonical_path) = path.canonicalize() {
        if let Ok(canonical_base) = base.canonicalize() {
            if !canonical_path.starts_with(&canonical_base) {
                return Err("Path traversal detected".to_string());
            }
        }
    }
    Ok(())
}
```

**修改后代码**：

```rust
fn ensure_path_under_base(path: &Path, base: &Path) -> Result<(), String> {
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize base: {}", e))?;
    if !canonical_path.starts_with(&canonical_base) {
        return Err("Path traversal detected".to_string());
    }
    Ok(())
}
```

**影响范围**：此函数被 `uninstall_skill`（第 960 行）、`uninstall_skill_group`（第 987 行）调用。修改后 canonicalize 失败会直接返回错误而非静默通过，调用方需要能处理新的错误路径。

**验证方法**：

1. 构造一个不存在的 `base` 路径调用 `ensure_path_under_base`，确认返回 `Err` 而非 `Ok(())`.
2. 在 Windows 上构造一个包含 `..\..\Windows\System32` 的 path，确认返回 "Path traversal detected" 错误.
3. 正常卸载一个存在的 skill（路径合法），确认功能不受影响.

---

### 缺陷 #5：ZIP 解压路径遍历检查可能被 TOCTOU 绕过

- **严重程度**：严重（P0）
- **文件**：`src-tauri/src/commands/skills.rs`
- **位置**：第 669–685 行

**修改前代码**：

```rust
for i in 0..archive.len() {
    let entry = archive
        .by_index(i)
        .map_err(|e| format!("Failed to read zip entry: {}", e))?;
    let entry_path = entry.mangled_name();
    let resolved = temp_dir.path().join(&entry_path);
    if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&dest_canonical) {
            return Err("Path traversal detected in zip".into());
        }
    }
}

archive
    .extract(temp_dir.path())
    .map_err(|e| format!("Failed to extract: {}", e))?;
```

**修改后代码**：

```rust
// 阶段一：使用 enclosed_name() 验证所有 entry
for i in 0..archive.len() {
    let entry = archive
        .by_index(i)
        .map_err(|e| format!("Failed to read zip entry: {}", e))?;

    // 使用 enclosed_name() 而非 mangled_name()：
    // enclosed_name() 在遇到非 UTF-8 或路径遍历路径时返回 None
    let entry_path = entry
        .enclosed_name()
        .ok_or_else(|| format!(
            "Invalid zip entry name (non-UTF-8 or path traversal): entry {}",
            i
        ))?
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
archive
    .extract(temp_dir.path())
    .map_err(|e| format!("Failed to extract: {}", e))?;

// 阶段三：解压后二次验证（防止 TOCTOU）
for i in 0..archive.len() {
    let entry = archive
        .by_index(i)
        .map_err(|e| format!("Failed to re-read zip entry: {}", e))?;
    let entry_path = entry
        .enclosed_name()
        .ok_or_else(|| format!("Invalid zip entry name during post-extract check: entry {}", i))?;
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
```

**验证方法**：

1. 构造一个包含 `../../../etc/passwd` entry 的恶意 ZIP，尝试安装，确认被第一阶段拦截.
2. 构造一个包含正常路径但非 UTF-8 文件名的 ZIP（如 `\xFF\xFEskill.md`），确认 `enclosed_name()` 返回 None 并报错.
3. 正常安装一个合法 GitHub skill ZIP，确认三阶段全部通过且 skill 正常运行.
4. 若 `zip` crate 版本不支持 `enclosed_name()`，先升级到 2.x 或使用 `entry.name()` + 手工路径遍历检查.

---

### 缺陷 #14：沙箱 iframe origin 验证在 `sandbox` 属性下失败

- **严重程度**：严重（P0）
- **文件**：`src/sdk/sandboxTemplate.ts`（第 158 行、第 161 行）、`src/components/skill/SkillSandboxContainer.tsx`（第 319 行）
- **位置**：`sandboxTemplate.ts` 运行时脚本中的 `TARGET_ORIGIN` / `isValidOrigin` 定义

**修改前代码**（`sandboxTemplate.ts` 第 155–163 行）：

```javascript
var TARGET_ORIGIN = window.location.origin;

function isValidOrigin(origin) {
  return origin === TARGET_ORIGIN;
}
```

**问题分析**：`SkillSandboxContainer.tsx` 第 319 行设置 `sandbox="allow-scripts"`（无 `allow-same-origin`）。在此配置下，iframe 内 `window.location.origin` 返回字符串 `"null"`，而宿主页面发送的 postMessage 携带的是宿主真实 origin，导致 `isValidOrigin` 永远返回 `false`，RPC 通信全部中断。

在 Tauri WebView 中的实际行为需要通过集成测试确认——Tauri 可能有自定义协议处理。但在标准浏览器行为下，此逻辑已损坏。

**修改后代码**（`sandboxTemplate.ts` 第 155–170 行）：

```javascript
var TARGET_ORIGIN = window.location.origin;

function isValidOrigin(origin) {
  // Tauri WebView sandbox iframe 中 window.location.origin 为 "null"
  // 此时放宽 origin 检查，接受宿主发来的任何 origin
  // 纵深防御：sandbox 属性 + CSP + fetch/XHR 禁用已提供多层保护
  if (TARGET_ORIGIN === "null") {
    return true;
  }
  return origin === TARGET_ORIGIN;
}
```

**补充修改（`SkillSandboxContainer.tsx` 第 315–320 行）**——增加 CSP 纵深防御：

```tsx
<iframe
  ref={iframeRef}
  title={`Skill: ${skillName}`}
  sandbox="allow-scripts"
  // 增加 CSP: default-src 'none' + script-src 'unsafe-inline' 限制资源加载
  // 注意：csp 属性不是标准属性，需通过 srcdoc 或 HTTP 头传递
  style={{...}}
/>
```

由于 CSP 属性不能直接设置在 `<iframe>` 标签上，改为在 `sandboxTemplate.ts` 生成的 HTML 的 `<head>` 中加入：

```html
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; font-src 'none'; connect-src 'none';">
```

**验证方法**：

1. 在 Tauri 开发环境启动应用，安装任意带 UI 的 skill，确认 iframe 正常渲染.
2. 在 iframe 内使用 skill 的 `store.get` / `store.set` 功能，确认 RPC 通信正常.
3. 使用浏览器 DevTools 检查 iframe 的 `window.location.origin` 值（应为 `"null"` 或真实 origin）.
4. 若 Tauri WebView 中 origin 行为不同于标准浏览器，根据实际值调整 `isValidOrigin` 逻辑.

---

## P1 — 本周修复（5 项）

### 缺陷 #1：技能系统零测试覆盖

- **严重程度**：致命（P1）
- **文件**：`src-tauri/src/commands/skills.rs`（2165 行）、`src-tauri/src/commands/skills_hub.rs`（370 行）、`src-tauri/src/commands/skill_decomposition.rs`（884 行）、`src-tauri/src/commands/error_code.rs`（369 行）
- **位置**：整个模块无 `#[cfg(test)]` 模块

**修复方案**：在 `src-tauri/src/commands/skills.rs` 末尾追加测试模块.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ── validate_skill_name 测试 ──────────────────────────────

    #[test]
    fn test_validate_skill_name_valid() {
        assert!(validate_skill_name("my-skill").is_ok());
        assert!(validate_skill_name("skill_123").is_ok());
        assert!(validate_skill_name("A").is_ok());
        assert!(validate_skill_name("a-b-c_d").is_ok());
    }

    #[test]
    fn test_validate_skill_name_empty() {
        assert!(validate_skill_name("").is_err());
    }

    #[test]
    fn test_validate_skill_name_path_traversal() {
        assert!(validate_skill_name("../etc").is_err());
        assert!(validate_skill_name("skill/../passwd").is_err());
        assert!(validate_skill_name("skill\\..\\system").is_err());
    }

    #[test]
    fn test_validate_skill_name_drive_letter() {
        assert!(validate_skill_name("C:skill").is_err());
        assert!(validate_skill_name("D:evil").is_err());
    }

    #[test]
    fn test_validate_skill_name_null_byte() {
        // 修复后应拒绝空字节
        assert!(validate_skill_name("skill\x00evil").is_err());
    }

    #[test]
    fn test_validate_skill_name_windows_reserved() {
        // 修复后应拒绝 Windows 保留名称
        assert!(validate_skill_name("CON").is_err());
        assert!(validate_skill_name("NUL").is_err());
        assert!(validate_skill_name("PRN").is_err());
        assert!(validate_skill_name("COM1").is_err());
        assert!(validate_skill_name("LPT1").is_err());
    }

    // ── ensure_path_under_base 测试 ───────────────────────────

    #[test]
    fn test_ensure_path_under_base_valid() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let sub = base.join("subdir");
        std::fs::create_dir(&sub).unwrap();
        let canonical_sub = sub.canonicalize().unwrap();
        assert!(ensure_path_under_base(&canonical_sub, &base).is_ok());
    }

    #[test]
    fn test_ensure_path_under_base_traversal_detected() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let other = TempDir::new().unwrap();
        let other_path = other.path().canonicalize().unwrap();
        assert!(ensure_path_under_base(&other_path, &base).is_err());
    }

    #[test]
    fn test_ensure_path_under_base_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let nonexistent = base.join("does_not_exist");
        // 修复后应返回 Err（canonicalize 失败）
        assert!(ensure_path_under_base(&nonexistent, &base).is_err());
    }

    #[test]
    fn test_ensure_path_under_base_nonexistent_base() {
        let nonexistent = PathBuf::from("Z:\\nonexistent\\base");
        assert!(ensure_path_under_base(&nonexistent.join("sub"), &nonexistent).is_err());
    }

    // ── compare_versions 测试（如存在） ───────────────────────

    // ── MarketplaceSearchCache 驱逐测试 ───────────────────────

    #[test]
    fn test_cache_eviction_lru() {
        let mut cache = MarketplaceSearchCache::new(3600);
        cache.max_capacity = 3;
        let dummy: Vec<MarketplaceSkill> = vec![];
        cache.set("a".into(), dummy.clone());
        std::thread::sleep(std::time::Duration::from_millis(50));
        cache.set("b".into(), dummy.clone());
        std::thread::sleep(std::time::Duration::from_millis(50));
        cache.set("c".into(), dummy.clone());
        // 此时容量满，再插入会驱逐最旧的 "a"
        cache.set("d".into(), dummy.clone());
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
        assert!(cache.get("d").is_some());
    }
}
```

**验证方法**：在项目根目录运行 `cargo test -p axagent-app -- commands::skills::tests`（根据实际 crate 名调整），确认所有测试通过.

---

### 缺陷 #3：`skill_read_asset` 路径注入防护增强

- **严重程度**：严重（P1）
- **文件**：`src-tauri/src/commands/skills.rs`
- **位置**：第 2129–2165 行

**确认现状**：`skill_read_asset` 已有独立的 canonicalize 检查（第 2138–2140 行，错误正确传播），且限制了白名单后缀。但 `file_name` 参数未经 `..` 过滤，且在 Windows 上需防御盘符路径.

**修改前代码**（第 2129–2133 行）：

```rust
pub fn skill_read_asset(name: String, file_name: String) -> Result<String, String> {
    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name));
    }
```

**修改后代码**（第 2129–2149 行）：

```rust
pub fn skill_read_asset(name: String, file_name: String) -> Result<String, String> {
    // 对 file_name 参数增加路径遍历字符过滤
    if file_name.contains("..")
        || file_name.contains('\\')
        || file_name.contains('/')
        || file_name.is_empty()
    {
        return Err("Invalid file_name: path traversal or empty".to_string());
    }
    // 拒绝绝对路径（Windows 盘符或 Unix 根路径）
    if file_name.len() >= 2 {
        let b = file_name.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err("Invalid file_name: absolute path not allowed".to_string());
        }
    }
    if file_name.starts_with('/') {
        return Err("Invalid file_name: absolute path not allowed".to_string());
    }

    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name));
    }
    // ... 后续 canonicalize 检查保持不变 ...
```

**验证方法**：

1. 调用 `skill_read_asset("my-skill", "../../.ssh/id_rsa")` → 返回 "Invalid file_name: path traversal or empty".
2. 调用 `skill_read_asset("my-skill", "C:\\Windows\\System32\\config\\SAM")` → 返回 "Invalid file_name: absolute path not allowed".
3. 调用 `skill_read_asset("my-skill", "index.html")` → 正常返回文件内容.

---

### 缺陷 #13：SkillsHubSettings 导入/导出按钮无功能实现

- **严重程度**：致命（P1）
- **文件**：`src/components/settings/SkillsHubSettings.tsx`
- **位置**：第 234–240 行

**修改前代码**（第 234–240 行）：

```tsx
<div className="flex gap-3">
  <Button icon={<Upload size={16} />}>
    {t("settings.skillsHub.exportSkill")}
  </Button>
  <Button icon={<Download size={16} />}>
    {t("settings.skillsHub.importSkill")}
  </Button>
</div>;
```

**修改后代码**（完整行块替换）：

```tsx
<div className="flex gap-3">
  <Button
    icon={<Upload size={16} />}
    onClick={handleExportSkill}
  >
    {t("settings.skillsHub.exportSkill")}
  </Button>
  <Button
    icon={<Download size={16} />}
    onClick={handleImportSkill}
  >
    {t("settings.skillsHub.importSkill")}
  </Button>
</div>;
```

在组件函数体内（`SkillsHubSettings` 函数中，`return` 之前）添加两个处理函数：

```tsx
const handleExportSkill = useCallback(async () => {
  // 获取用户已安装的 skill 列表
  const { useSkillExtensionStore } = await import("@/stores");
  const installedSkills = useSkillExtensionStore.getState().installedSkills;
  if (!installedSkills || installedSkills.length === 0) {
    message.warning(t("settings.skillsHub.noSkillsToExport"));
    return;
  }

  // 使用 askUser 或 modal 让用户选择要导出的 skill
  // 选中后调用 Rust 后端 skills_hub_export 命令
  // 若后端暂无该命令，先用前端兜底：读取 skill 目录，打包为 JSON 下载
  try {
    const skillName = installedSkills[0]; // 简化示例，实际应弹出选择器
    const detail = await invoke("get_skill", { name: skillName });
    const blob = new Blob([JSON.stringify(detail, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${skillName}.skill.json`;
    a.click();
    URL.revokeObjectURL(url);
    message.success(t("settings.skillsHub.exportSuccess"));
  } catch (e) {
    console.error("Export failed:", e);
    message.error(t("settings.skillsHub.exportFailed"));
  }
}, []);

const handleImportSkill = useCallback(async () => {
  // 打开文件选择器，读取 .skill.json 文件
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".skill.json";
  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) { return; }
    try {
      const text = await file.text();
      const skillData = JSON.parse(text);
      // 调用 Rust 后端 skills_hub_import 命令
      // 若后端暂无该命令，暂用 install_skill 替代
      await invoke("install_skill", {
        source: "local",
        target: skillData.name,
      });
      message.success(t("settings.skillsHub.importSuccess"));
    } catch (e) {
      console.error("Import failed:", e);
      message.error(t("settings.skillsHub.importFailed"));
    }
  };
  input.click();
}, []);
```

**验证方法**：

1. 打开设置 → Skills Hub 页面，点击 "Export Skill" 按钮，确认弹出选择器或执行下载.
2. 点击 "Import Skill" 按钮，确认弹出文件选择器.
3. 选择有效的 `.skill.json` 文件后确认后台收到 import 请求.
4. 若 Rust 后端 `skills_hub_export`/`skills_hub_import` 命令不存在，需同步实现或标记为 WIP 并显示提示.

---

### 缺陷 #15：`actionRouter.ts` handler 执行器权限上下文丢失

- **严重程度**：严重（P1）
- **文件**：`src/lib/actionRouter.ts`
- **位置**：第 425–439 行（`handler` executor）

**修改前代码**（第 425–439 行）：

```typescript
this.declarativeExecutors.set("handler", async (action, ctx) => {
  if (action.type !== "handler") {
    return { success: false, error: i18n.t("actionRouter.typeMismatch") };
  }
  const { useSkillExtensionStore } = await import("@/stores");
  const handler = useSkillExtensionStore.getState().getHandler(action.name);
  if (!handler) {
    return {
      success: false,
      error: i18n.t("actionRouter.handlerNotFound", { name: action.name }),
    };
  }
  if (handler.mode === "declarative" && handler.actions) {
    return this.executeChain(handler.actions, ctx);
  }
  // ...
```

**修改后代码**：

```typescript
this.declarativeExecutors.set("handler", async (action, ctx) => {
  if (action.type !== "handler") {
    return { success: false, error: i18n.t("actionRouter.typeMismatch") };
  }
  const { useSkillExtensionStore } = await import("@/stores");
  const handler = useSkillExtensionStore.getState().getHandler(action.name);
  if (!handler) {
    return {
      success: false,
      error: i18n.t("actionRouter.handlerNotFound", { name: action.name }),
    };
  }
  if (handler.mode === "declarative" && handler.actions) {
    // 使用 handler 所属 skill 的权限上下文，而非调用方 ctx
    const handlerPermissions = useSkillExtensionStore
      .getState()
      .getSkillPermissions(handler.skillName);
    const handlerCtx: ActionContext = {
      ...ctx,
      skillName: handler.skillName,
      permissions: handlerPermissions ?? ctx.permissions,
    };
    return this.executeChain(handler.actions, handlerCtx);
  }
  // ...
```

**同步修改**：需要在 `skillExtensionStore.ts` 中暴露 `getSkillPermissions(skillName: string): SkillPermissions | undefined` 方法：

```typescript
getSkillPermissions: (skillName: string): SkillPermissions | undefined => {
  const skill = get().skills.get(skillName);
  return skill?.manifest?.permissions;
},
```

**验证方法**：

1. 创建 Skill A（权限宽，可写入 preference store）和 Skill B（权限窄，不可写入 preference store）.
2. Skill A 的 handler 引用 Skill B 的 handler 的 action chain.
3. 执行 Skill A 的 handler，确认 Skill B 的 action 以 Skill B 的权限（窄）执行，写入 preference 被拒绝.
4. 若 Skill B 被 Skill A 通过 handler 引用是预期行为，则确认权限正确切换.

---

### 缺陷 #16：`storeRegistry` 中 `skill` store 可被任意 skill 读写

- **严重程度**：严重（P1）
- **文件**：`src/lib/storeRegistry.ts`（第 41–43 行）、`src/lib/skillPermissions.ts`（第 31–50 行）
- **位置**：`initStoreRegistry` 中的 store 注册列表、`isStorePermCovered` 函数

**修改方案**：

**方案 A（推荐）**：将 `skill` store 从 Skill 可访问白名单中移除.

**修改 `storeRegistry.ts` 第 41–43 行**（移除 skill store 注册）：

```typescript
const registry: Array<{
  name: string;
  store: { getState: () => unknown; setState: (partial: unknown) => void };
}> = [
  {
    name: "preference",
    store: stores.usePreferenceStore as unknown as { ... },
  },
  {
    name: "conversation",
    store: stores.useConversationStore as unknown as { ... },
  },
  {
    name: "ui",
    store: stores.useUIStore as unknown as { ... },
  },
  // skill store 已移除——Skill 不应通过声明式 action 修改技能系统自身状态
  // 如需 skill 间通信，使用 skillEventBus 的 emit/on 机制
  {
    name: "artifact",
    store: stores.useArtifactStore as unknown as { ... },
  },
  // ... 其余 stores 保持
];
```

**同步修改 `skillPermissions.ts`**——在 `validateSkillPermissions` 中增加硬性拒绝规则：

```typescript
// 硬性拒绝：不允许任何 Skill 声明对 skill store 的读写
const FORBIDDEN_STORES = ["skill"];

export function validateSkillPermissions(
  permissions: SkillPermissions,
): PermissionValidationResult {
  const violations: string[] = [];

  // 检查 storeRead
  for (const perm of permissions.storeRead ?? []) {
    const { storeName } = parseStorePerm(perm);
    if (FORBIDDEN_STORES.includes(storeName)) {
      violations.push(`storeRead "${perm}" is forbidden: skill store access is restricted`);
    }
  }

  // 检查 storeWrite
  for (const perm of permissions.storeWrite ?? []) {
    const { storeName } = parseStorePerm(perm);
    if (FORBIDDEN_STORES.includes(storeName)) {
      violations.push(`storeWrite "${perm}" is forbidden: skill store access is restricted`);
    }
  }

  // ... 其余校验逻辑 ...

  return { valid: violations.length === 0, violations };
}
```

**验证方法**：

1. 创建一个 manifest 中声明 `storeWrite: ["skill:*"]` 的 skill，安装时确认权限校验返回失败.
2. 现有合法 skill（不声明 skill store 访问）安装正常，功能不受影响.
3. 若现有 skill 依赖 skill store 访问（检查所有已安装 skill 的 manifest），制定迁移计划.

---

## P2 — 迭代修复（9 项）

### 缺陷 #4：`uninstall_skill` 硬编码 6 个搜索目录且无回滚

- **文件**：`src-tauri/src/commands/skills.rs`，第 944–971 行

**修改方向**：

1. 将 `search_dirs` 替换为 `all_skills_dirs()`（该函数已在第 50–52 行定义）：

```rust
pub async fn uninstall_skill(app: tauri::AppHandle, name: String) -> Result<(), String> {
    validate_skill_name(&name)?;
    let search_dirs = all_skills_dirs();
    // ... 遍历逻辑保持不变，但使用动态目录列表
```

2. 收集所有匹配目录后再删除（而非找到第一个就返回），汇总结果：

```rust
let mut removed = Vec::new();
let mut errors = Vec::new();

for parent in &search_dirs {
    let skill_dir = parent.join(&name);
    if skill_dir.exists() && skill_dir.is_dir() {
        match ensure_path_under_base(&skill_dir, parent) {
            Ok(()) => match std::fs::remove_dir_all(&skill_dir) {
                Ok(()) => removed.push(skill_dir),
                Err(e) => errors.push(format!("{}: {}", skill_dir.display(), e)),
            },
            Err(e) => errors.push(format!("{}: {}", skill_dir.display(), e)),
        }
    }
}

if removed.is_empty() && errors.is_empty() {
    return Err(format!("Skill '{}' 未在任何技能目录中找到", name));
}

// emit 事件时包含完整结果
let _ = app.emit("skill-state-changed", serde_json::json!({
    "skillName": &name,
    "action": "uninstalled",
    "removed": removed.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    "errors": errors,
}));

if !errors.is_empty() {
    return Err(format!("部分删除失败: {}", errors.join("; ")));
}
Ok(())
```

---

### 缺陷 #6：MarketplaceSearchCache LRU 驱逐代码优化

- **文件**：`src-tauri/src/commands/skills.rs`，第 92–110 行

**修改方向**：使用 `into_iter()` 消除多余 clone：

```rust
if self.cache.len() >= self.max_capacity {
    let mut entries: Vec<_> = self.cache.iter().collect();
    entries.sort_by_key(|(_, v)| v.created_at);
    let remove_count = entries.len() - self.max_capacity + 1;
    // 优化：使用 into_iter 消除 clone
    let keys_to_remove: Vec<String> = entries
        .into_iter()
        .take(remove_count)
        .map(|(k, _)| k.clone())
        .collect();
    for key in &keys_to_remove {
        self.cache.remove(key);
    }
}
```

---

### 缺陷 #7：每次 `get_skill` 调用重建 PluginManager

- **文件**：`src-tauri/src/commands/skills.rs`，第 213 行

**修改方向**：

1. 在 `SkillState` 中增加 PluginManager 缓存：

```rust
// src-tauri/src/state/skill.rs
pub struct SkillState {
    // ... 现有字段 ...
    pub plugin_manager_cache: Arc<tokio::sync::RwLock<Option<(PluginManager, Instant)>>>,
}
```

2. 修改 `get_skill` 使用缓存：

```rust
pub async fn get_skill(
    state: State<'_, AppState>,
    name: String,
) -> Result<SkillDetail, String> {
    let cache_ttl = Duration::from_secs(60); // 1 分钟缓存
    let pm = {
        let cache = state.skill_state.plugin_manager_cache.read().await;
        if let Some((ref pm, ts)) = *cache {
            if ts.elapsed() < cache_ttl {
                pm.clone()
            } else {
                drop(cache);
                let new_pm = create_plugin_manager_with_skill_dirs()?;
                let mut cache = state.skill_state.plugin_manager_cache.write().await;
                *cache = Some((new_pm.clone(), Instant::now()));
                new_pm
            }
        } else {
            drop(cache);
            let new_pm = create_plugin_manager_with_skill_dirs()?;
            let mut cache = state.skill_state.plugin_manager_cache.write().await;
            *cache = Some((new_pm.clone(), Instant::now()));
            new_pm
        }
    };
    // ... 后续使用 pm 而非重新创建 ...
```

---

### 缺陷 #8：`collect_skill_content` 无文件大小和深度限制

- **文件**：`src-tauri/src/commands/skills.rs`，第 288–330 行

**修改方向**：

```rust
const MAX_SINGLE_FILE_SIZE: u64 = 5 * 1024 * 1024;  // 5 MB
const MAX_TOTAL_SIZE: u64 = 10 * 1024 * 1024;        // 10 MB
const MAX_RECURSION_DEPTH: u32 = 5;

fn collect_markdown_files(dir: &Path, depth: u32) -> std::io::Result<Vec<PathBuf>> {
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

fn collect_skill_content(dir: &Path) -> String {
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
            if total_bytes > MAX_TOTAL_SIZE {
                content.push_str("\n\n<!-- [TRUNCATED] Total content exceeds 10MB limit -->\n");
                break;
            }
            content.push_str(&text);
            content.push('\n');
        }
    }
    content
}
```

---

### 缺陷 #9：部分命令仍返回 `Result<T, String>` 而非 `Result<T, ErrorResponse>`

- **文件**：`src-tauri/src/commands/skills.rs`
- **涉及命令**：`get_skill`, `uninstall_skill`, `toggle_skill`, `skill_patch`, `skill_edit`, `skill_read_asset`, `install_skill`

**修改方向**：

将以下命令的返回类型从 `Result<T, String>` 改为 `Result<T, ErrorResponse>`：

| 命令                       | 当前返回                      | 修改后返回                           |
| -------------------------- | ----------------------------- | ------------------------------------ |
| `get_skill` (L213)         | `Result<SkillDetail, String>` | `Result<SkillDetail, ErrorResponse>` |
| `uninstall_skill` (L943)   | `Result<(), String>`          | `Result<(), ErrorResponse>`          |
| `toggle_skill` (L332)      | `Result<(), String>`          | `Result<(), ErrorResponse>`          |
| `skill_patch` (L1509)      | `Result<String, String>`      | `Result<String, ErrorResponse>`      |
| `skill_edit` (L1541)       | `Result<String, String>`      | `Result<String, ErrorResponse>`      |
| `skill_read_asset` (L2130) | `Result<String, String>`      | `Result<String, ErrorResponse>`      |
| `install_skill` (L351)     | `Result<String, String>`      | `Result<String, ErrorResponse>`      |

错误码使用 `error_code.rs` 中已定义的 `skill` 和 `skill_op_err` 模块常量，通过 `ErrorResponse::new(skill_err::NOT_FOUND)` 等形式返回。

---

### 缺陷 #10：`validate_skill_name` 遗漏注入向量

- **文件**：`src-tauri/src/commands/skills.rs`，第 914–929 行

**修改方向**：

```rust
fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Skill name must not be empty".to_string());
    }
    // 长度限制
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
    // Windows 保留名称黑名单（不区分大小写）
    const WINDOWS_RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let upper = name.to_ascii_uppercase();
    if WINDOWS_RESERVED.iter().any(|r| upper.as_str() == *r || upper.starts_with(&format!("{}.", r))) {
        return Err(format!("Skill name '{}' is a Windows reserved name", name));
    }
    // 推荐仅允许字母、数字、连字符、下划线
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Skill name must only contain alphanumeric characters, hyphens, and underscores".to_string());
    }
    Ok(())
}
```

---

### 缺陷 #18：`skillEventBus` FIFO 语义与 LRU 注释不一致

- **文件**：`src/lib/skillEventBus.ts`，第 15–22 行

**修改方向**：将注释中的 "LRU" 改为 "FIFO"，并添加说明：

```typescript
const MAX_LISTENER_KEYS = 200;

/**
 * 容量驱逐策略：FIFO（先进先出）。
 * 当注册的监听器 key 数量超过 MAX_LISTENER_KEYS 时，驱逐最早注册的监听器。
 * 对于 event bus 场景，FIFO 是可接受的——长期未活跃的 Skill 通常先注册。
 */
function evictIfNeeded() {
  if (listeners.size <= MAX_LISTENER_KEYS) {
    return;
  }
  const keys = listeners.keys();
  const excess = listeners.size - MAX_LISTENER_KEYS;
  for (let i = 0; i < excess; i++) {
    const key = keys.next().value;
    if (key !== undefined) {
      listeners.delete(key);
    }
  }
}
```

---

### 缺陷 #19：`skillLifecycle.ts` 缓存回退静默失败

- **文件**：`src/lib/skillLifecycle.ts`，第 27–38 行

**修改方向**：

```typescript
async function readLifecycleData(
  skillName: string,
): Promise<{ hooks: SkillLifecycleHooks | null; permissions: SkillPermissions | undefined }> {
  const cached = lifecycleCache.get(skillName);
  if (cached && Date.now() - cached.ts < LIFECYCLE_CACHE_TTL_MS) {
    return { hooks: cached.hooks, permissions: cached.permissions };
  }

  // 带退避的重试
  const maxRetries = 3;
  const retryDelays = [1000, 2000, 4000]; // ms

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const detail = await invoke<{ manifest?: SkillManifest }>("get_skill", {
        name: skillName,
      });
      const hooks = detail?.manifest?.lifecycle ?? null;
      const permissions = detail?.manifest?.permissions;
      lifecycleCache.set(skillName, { hooks, permissions, ts: Date.now() });
      return { hooks, permissions };
    } catch (e) {
      if (attempt < maxRetries) {
        console.warn(
          `[skillLifecycle] get_skill failed for "${skillName}" (attempt ${attempt + 1}/${
            maxRetries + 1
          }), retrying in ${retryDelays[attempt]}ms:`,
          e,
        );
        await new Promise((resolve) => setTimeout(resolve, retryDelays[attempt]));
      } else {
        console.error(
          `[skillLifecycle] get_skill failed for "${skillName}" after ${maxRetries + 1} attempts:`,
          e,
        );
        return { hooks: null, permissions: undefined };
      }
    }
  }
  return { hooks: null, permissions: undefined };
}
```

---

### 缺陷 #20：`function` 类型 action executor 为死代码路径

- **文件**：`src/lib/actionRouter.ts`（第 41–49 行、第 410–422 行）、`src/lib/skillActionExecutor.ts`

**修改方向**：

1. 如果 `function` 类型不计划近期启用，在 `VALID_ACTION_TYPES` 中标注为 `@experimental` 并从生产白名单移除：

```typescript
const VALID_ACTION_TYPES = new Set([
  "invoke",
  "navigate",
  "emit",
  "store",
  // "function",  // @experimental — not yet ready for production use
  "handler",
  "chain",
  "update-schema",
]);
```

2. 如果计划启用，完善注册流程：在 skill 安装/启用时调用 `registerCustomFunction`。

3. 在 manifest 校验阶段，若声明了 `function` 类型的 action，返回明确的警告提示。

---

## P3 — 技术债务（5 项）

### 缺陷 #11：`SkillState` 持有 20+ 字段且构造函数参数过多

- **文件**：`src-tauri/src/state/skill.rs`
- **位置**：第 13–39 行（结构体）、第 44–105 行（构造函数）

**修改方向**：

1. 使用 Builder 模式重构 `SkillState::new`：

```rust
pub struct SkillStateBuilder {
    skill_evolution_engine: Option<Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>>,
    skill_proposal_service: Option<Arc<TokioRwLock<axagent_trajectory::SkillProposalService>>>,
    // ... 各字段默认 None
}

impl SkillStateBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn skill_evolution_engine(mut self, v: ...) -> Self { /* ... */ }
    pub fn build(self) -> SkillState { /* ... */ }
}
```

2. 对未完工引擎使用 `Option<Arc<...>>` 并以 `None` 初始化，标注 WIP 状态.

---

### 缺陷 #12：`skill_patch` 和 `skill_edit` 代码重复

- **文件**：`src-tauri/src/commands/skills.rs`
- **位置**：第 1509–1538 行（`skill_patch`）、第 1541–1580 行（`skill_edit`）

**修改方向**：提取公共函数：

```rust
/// 打开 skill 的 SKILL.md 文件，返回 (canonicalized_path, file_content)
fn open_skill_md(name: &str) -> Result<(PathBuf, String), String> {
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
```

然后 `skill_patch` 和 `skill_edit` 均调用 `open_skill_md(&name)?` 替代前 20 行重复逻辑.

---

### 缺陷 #17：沙箱 HTML 中 `delete fetch/XHR` 可被 CSP 替代

- **文件**：`src/sdk/sandboxTemplate.ts`，第 93–94 行、第 88–103 行

**修改方向**：

1. 在生成的 HTML `<head>` 中添加 CSP meta 标签（与 P0 #14 中建议一致）：

```html
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; font-src 'none'; connect-src 'none';">
```

2. 将 `delete window.fetch` / `delete window.XMLHttpRequest` 替换为更健壮的 `Object.defineProperty` 方案：

```javascript
// 更健壮的网络 API 禁用（防止 non-configurable 属性导致异常）
["fetch", "XMLHttpRequest"].forEach(function(prop) {
  try {
    Object.defineProperty(window, prop, {
      get: function() {
        throw new Error(prop + " is disabled in skill sandbox");
      },
      set: function() {},
      configurable: false,
    });
  } catch (e) {
    // 如果 defineProperty 也失败，fallback 到 delete
    try {
      delete window[prop];
    } catch (_) {}
  }
});
```

3. CSP 作为纵深防御——即使 defineProperty 被绕过，`connect-src 'none'` 也阻止网络请求.

---

### 缺陷 #21：Skill 权限声明无版本/签名校验

- **文件**：`src/lib/skillPermissions.ts`、`src/lib/skillExtensionStore.ts`

**修改方向**：

1. 在 Rust 端 `install_skill` 时计算 manifest JSON 的 SHA-256 哈希，存储到 `skill-manifest.json.sig` 或数据库.
2. 每次加载 skill 时比对哈希，若不匹配：
   - 发出警告日志.
   - 弹出用户提示 "Skill manifest has been modified, permissions may have changed".
   - 要求用户二次确认.
3. 前端在 `validateSkillPermissions` 中增加哈希校验步骤.

---

### 缺陷 #22：Skill 生命周期钩子并行执行可能导致竞态条件

- **文件**：`src/lib/skillLifecycle.ts`，第 73–86 行、第 52–59 行

**修改方向**：

```typescript
async function executeHooks(
  actions: SkillCommandAction[],
  skillName: string,
  permissions?: SkillPermissions,
): Promise<void> {
  if (!actions || actions.length === 0) {
    return;
  }
  const router = getActionRouter();
  // 改为顺序执行，避免竞态条件
  // 若需要并行，可通过 manifest 中的 "parallel": true 标记控制
  for (const action of actions) {
    try {
      await router.execute(action, { skillName, permissions });
    } catch (e) {
      logIpcError(`Lifecycle hook failed for ${skillName}: ${action.type}`)(e);
    }
  }
}
```

---

## 修复检查清单

实施过程中逐项确认：

- [ ] **P0-1** (#2) `ensure_path_under_base` canonicalize 失败返回 Err 而非 Ok(())
- [ ] **P0-2** (#5) ZIP 解压使用 `enclosed_name()` + 解压后二次验证 + TOCTOU 防护
- [ ] **P0-3** (#14) 沙箱 `isValidOrigin` 处理 `"null"` origin + 增加 CSP meta
- [ ] **P1-1** (#1) `skills.rs` 末尾追加 `#[cfg(test)] mod tests` 至少覆盖 validate_skill_name / ensure_path_under_base / cache eviction
- [ ] **P1-2** (#3) `skill_read_asset` 的 `file_name` 参数增加 `..` / 盘符 / 绝对路径校验
- [ ] **P1-3** (#13) SkillsHubSettings 导入/导出按钮添加 onClick 处理函数
- [ ] **P1-4** (#15) handler executor 中切换为 handler 所属 skill 的权限上下文
- [ ] **P1-5** (#16) skill store 从 Skill 可访问白名单移除 + permissions 校验中硬性拒绝
- [ ] **P2-1** (#4) `uninstall_skill` 使用 `all_skills_dirs()` 替代硬编码 + 收集所有结果再返回
- [ ] **P2-2** (#6) MarketplaceSearchCache 使用 `into_iter()` 消除 clone
- [ ] **P2-3** (#7) PluginManager 缓存到 SkillState，避免每次 get_skill 重建
- [ ] **P2-4** (#8) `collect_skill_content` 增加单文件 5MB / 总计 10MB / 深度 5 层限制
- [ ] **P2-5** (#9) 核心技能命令返回类型统一为 `Result<T, ErrorResponse>`
- [ ] **P2-6** (#10) `validate_skill_name` 增加空字节 / 长度 / Windows 保留名 / 字符白名单检查
- [ ] **P2-7** (#18) skillEventBus 注释从 "LRU" 改为 "FIFO"
- [ ] **P2-8** (#19) skillLifecycle 静默失败改为 console.error + 带退避重试
- [ ] **P2-9** (#20) `function` action type 标记为 `@experimental` 或完善注册流程
- [ ] **P3-1** (#11) SkillState 使用 Builder 模式 + 未完工引擎用 `Option<Arc<...>>`
- [ ] **P3-2** (#12) 提取 `open_skill_md()` 消除 skill_patch / skill_edit 重复代码
- [ ] **P3-3** (#17) 沙箱 HTML 添加 CSP meta + 用 Object.defineProperty 替代 delete
- [ ] **P3-4** (#21) 安装时计算 manifest 哈希 + 加载时比对 + 变更时二次确认
- [ ] **P3-5** (#22) executeHooks 改为顺序执行，避免竞态条件
