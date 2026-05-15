# i18n 硬编码字符串清理 — 设计文档

## 元信息

- **创建日期**: 2026-05-15
- **状态**: 待审批
- **范围**: 全项目（`src/` + `src-tauri/`）
- **策略**: 渐进式四阶段，不阻塞功能开发

---

## 问题陈述

项目存在系统性的 i18n 合规问题，总计约 1500 处违规，分三类：

| 类别 | 数量 | 影响 |
|------|------|------|
| 完全硬编码（未使用 t()） | ~750 处 | 非中文用户看到中文，非英文用户看到英文 |
| `t()` fallback 滥用 | 640 处 | key 可能不存在于 locale 文件，fallback 语言与用户语言不匹配 |
| locale 文件缺失 key | 50 个缺失 + 18 个空白 | 用户看到错误语言或空白 |

**根因**: CI 缺乏自动化检测，开发者依赖人工记忆遵守 CLAUDE.md 禁区第 2 条。

---

## 设计目标

1. **补洞优先**: 先修复会导致用户可见错误的问题（缺失 key、空白 key）
2. **自动化门禁**: 搭建 CI 检测脚本，阻断新增违规
3. **渐进清理**: 分批迁移存量违规，每批范围可控
4. **零回退**: 清理完成的部分不会被后续开发重新污染

---

## 总体架构：四阶段 + 双防线

```
阶段 1（紧急补洞）       阶段 2（消除 fallback）    阶段 3（迁移硬编码）    阶段 4（重构类型层）

补全缺失 locale key  →  移除 640 处 t() fallback  →  迁移 ~750 处硬编码  →  重构 types/data
修复空白 key              确保 key 存在于 11 种语言    中/英文 → t() 调用     标签映射到 i18n
搭建 CI 检测脚本

├── 防线 1: scripts/check-hardcoded-i18n.sh（增量门禁，阻断新增违规）
└── 防线 2: scripts/.i18n-allowlist.json（存量豁免清单，逐步收窄）
```

---

## 防线设计

### 防线 1: CI 检测脚本

**文件**: `scripts/check-hardcoded-i18n.sh`

检测规则：

1. **中文硬编码**: 扫描 `src/` 中 `.ts/.tsx` 文件，匹配 CJK 字符（`一-鿿`），排除：
   - `src/i18n/locales/` 目录
   - 注释行（`//`、`/*`）
   - `console.log/error/warn` 中的调试输出（可配置）
   - 已在 `.i18n-allowlist.json` 中的行

2. **英文 UI 硬编码**: 扫描特定模式——
   - `message.success/error/warning/info("...")`
   - `placeholder="..."`
   - `title="..."`
   - `aria-label="..."`
   - `notification.*({ message: "...", description: "..." })`
   - 排除已在 allowlist 中的行

3. **`t()` fallback 检测**: 扫描 `t("key", "非空字符串")` 模式，标记为 warning（不阻断 CI，但输出报告）

运行模式：
- `--strict`: 阻断模式（CI 使用），新增违规 → exit 1
- `--report`: 报告模式（本地使用），仅输出统计
- `--diff-only`: 仅检查 git diff 中的新增行（增量模式）

### 防线 2: 存量豁免清单

**文件**: `scripts/.i18n-allowlist.json`

```json
{
  "version": "1",
  "generated": "2026-05-15",
  "entries": [
    {
      "file": "src/data/expertPresets.ts",
      "lines": "6-198",
      "reason": "专家预设数据，阶段4处理",
      "phase": 4
    }
  ]
}
```

清理流程：每完成一个文件 → 从 allowlist 移除对应条目 → CI 开始对该文件生效。

---

## 阶段 1: 紧急补洞（预计 1-2 天）

### 1.1 补全 50 个缺失的 locale key

将仅存在于 `t()` fallback 中的 key 补充到所有 11 种语言文件中。

**缺失 key 清单**（按命名空间）：

| 命名空间 | 数量 | 示例 |
|----------|------|------|
| `advancedSettings.*` | 9 | `bashSecurity`, `defaultPermission`, `networkCmdDetect` |
| `benchmark.*` | 5 | `configTitle`, `empty`, `selectFirst` |
| `chat.contextGraph.*` | 2 | `hideType`, `showType` |
| `expertSelector.*` | 7 | `builtinAlreadyImported`, `importBuiltin` |
| `fineTune.*` | 7 | `stats.completed`, `tab.dataset` |
| `gateway.tab.monitor` | 1 | |
| `wiki.*` | 20 | `browse`, `dailyNote`, `exportHtml` |

**处理方式**:
- 从代码中提取 fallback 值作为 zh-CN 的正式翻译
- 为 en-US 编写英文翻译
- 其余 9 种语言标记为待翻译（保留英文 fallback 或使用脚本批量机翻+人工审核）

### 1.2 修复 18 个无 fallback 的空白 key

这些 key 在 `t()` 调用中无 fallback，且不存在于任何 locale 文件，用户会看到空白。

**方式**: 同 1.1，补充到所有语言文件。

### 1.3 搭建 CI 检测脚本

实现 `scripts/check-hardcoded-i18n.sh` 和 `.i18n-allowlist.json`。

初始化 allowlist 时，将当前所有已知违规全部录入。

### 1.4 在 CI 配置中集成

在 `.github/workflows/` 或等效 CI 配置中添加 `check-hardcoded-i18n` job，设为 **warning 模式**（不阻断构建，但输出报告）。

阶段 1 完成标准：
- [ ] 50 个缺失 key 已补充到所有 11 种语言文件
- [ ] 18 个空白 key 已修复
- [ ] CI 检测脚本可运行，allowlist 已初始化
- [ ] CI pipeline 中可见 i18n 检查报告

---

## 阶段 2: 消除 t() fallback（预计 3-5 天）

### 目标

移除所有 640 处 `t("key", "兜底文本")` 调用，改为纯 `t("key")`。

### 方法

1. **验证 key 存在性**: 对每个 fallback 调用，确认 key 在所有 11 种语言文件中存在
2. **补充缺失翻译**: 对不存在的语言条目，用 fallback 值填充 + 标记待翻译
3. **移除 fallback 参数**: 将 `t("key", "兜底")` 改为 `t("key")`
4. **从 allowlist 移除**: 完成后从豁免清单移除对应文件

### 分批策略

按文件违规数量从高到低分批：

| 批次 | 文件 | 数量 | 优先级 |
|------|------|------|--------|
| 1 | `ExpertSelector.tsx`, `HelpPanel.tsx` | ~100 | 高（用户高频使用） |
| 2 | `KnowledgePage.tsx`, `WelcomeWizard.tsx`, `AcpSettings.tsx` | ~82 | 高 |
| 3 | `WikiDetailPanel.tsx`, `ContextGraphPanel.tsx`, `AgentGeneratorModal.tsx` | ~61 | 中 |
| 4 | 其余 60+ 文件 | ~397 | 中/低 |

阶段 2 完成标准：
- [ ] 所有 `t()` 调用不再使用 fallback 参数
- [ ] 所有被引用的 key 在 11 种语言文件中均存在
- [ ] CI 检测脚本中 `t()` fallback 检测从 warning 升级为 error

---

## 阶段 3: 迁移硬编码（预计 5-8 天）

### 目标

将 ~750 处完全未使用 `t()` 的硬编码中/英文字符串迁移到 i18n 体系。

### 分层策略

#### 第 1 层: 用户可见 UI（最高优先，~200 处）

| 文件 | 典型违规 | 处理方式 |
|------|----------|----------|
| `chat/CitationManager.tsx` | 标题、按钮、状态文本 | 添加 locale key，替换为 `t()` |
| `chat/CredibilityBadge.tsx` | 可信度标签 | 同上 |
| `chat/BuddyMessage.tsx` | 情绪标签映射 | 同上 |
| `settings/ToolManager.tsx` | 工具提示文本 | 同上 |
| `shared/BaseModal.tsx` | 默认按钮文本 | 同上 |
| `decomposition/*.tsx` | 表格列标题、安装说明 | 同上 |
| `onboarding/WelcomeWizard.tsx` | 功能描述 | 同上 |
| `devtools/*.tsx` | 追踪记录文本 | 同上 |
| `benchmark/BenchmarkConfig.tsx` | 难度标签、UI 文本 | 同上 |
| `skill/*.tsx` | 编辑器标签、错误消息 | 同上 |

#### 第 2 层: Store 消息（中优先，~45 处）

| 文件 | 典型违规 | 处理方式 |
|------|----------|----------|
| `agentStore.ts` | Agent 执行状态消息 | 添加 locale key |
| `conversationStore.ts` | 错误/超时消息 | 同上 |
| `expertStore.ts` | 错误消息、分类名 | 同上 |
| `planStore.ts` | 成功/失败消息 | 同上 |
| `topicGroupStore.ts` | 默认名称 | 同上 |
| `proactiveStore.ts` | 意图关键词（**特殊处理**） | 关键词不翻译，添加注释说明 |
| `buddyStore.ts` | 物种/属性名 | 添加 locale key |
| `skillExtensionStore.ts` | 警告消息 | 添加 locale key |

#### 第 3 层: lib 工具库（中优先，~160 处）

| 文件 | 典型违规 | 处理方式 |
|------|----------|----------|
| `actionRouter.ts` | 错误消息 | 改为错误码 + 前端 i18n |
| `browserMock.ts` | 模拟数据 | 标记为 mock 数据，添加注释 |
| `memoryUtils.ts` | 标签映射 | 改为接收 `t()` 函数作为参数 |
| `searchUtils.ts` | 搜索提示词 | 标记为 LLM 提示，不翻译 |
| `skillPermissions.ts` | 权限错误 | 改为错误码 + 前端 i18n |
| `constants.ts` | 语言名称 | 已有 `LanguageNames` 映射，复用 |
| `exportChat.ts` | HTML 导出文本 | 添加 locale key |
| `chartGenerator.ts` | 关键词正则 | 标记为 NLP 逻辑，不翻译 |

#### 第 4 层: Rust 后端（低优先，~70 处）

| 文件 | 典型违规 | 处理方式 |
|------|----------|----------|
| `context_manager.rs` | 中文停用词、提示词 | 标记为 NLP 数据，不翻译 |
| `commands/agency_expert.rs` | 错误消息 | 改为错误码 |
| `crates/agent/evaluator/*` | 报告模板、标签 | 改为从前端传入语言参数 |

### 分类处理原则

并非所有硬编码都必须 i18n。以下类型**保留不译**：

| 类型 | 示例 | 理由 |
|------|------|------|
| LLM 系统提示词 | `agent.rs` 中的 `"You are AxAgent..."` | 属于模型交互数据，非 UI |
| NLP/关键词匹配 | `proactiveStore.ts` 中的意图关键词 | 算法逻辑，非用户可见 |
| Mock/测试数据 | `browserMock.ts` 中的模拟回复 | 仅开发环境使用 |
| 专有名词 | `"智谱"`, `"博查"` | 品牌名，无需翻译 |
| 快捷键标识 | `"Ctrl+C"`, `"Enter"` | 国际通用 |

阶段 3 完成标准：
- [ ] 所有用户可见 UI 文本已迁移到 i18n
- [ ] Store 消息已迁移
- [ ] lib 层按分类处理（翻译 / 标记豁免 / 重构为错误码）
- [ ] allowlist 中阶段 3 相关的条目已清除

---

## 阶段 4: 重构类型层（预计 2-3 天）

### 目标

消除 `src/types/` 和 `src/data/` 中的硬编码标签映射，重构为 i18n 原生方案。

### 4.1 `src/types/localTool.ts`

当前问题：
```typescript
// 硬编码中文映射，且项目中未被引用（死代码）
export const ToolCategoryLabels: Record<ToolCategory, string> = { ... };
export const PermissionModeLabels: Record<PermissionMode, string> = { ... };
```

方案：
1. 删除 `ToolCategoryLabels` 和 `PermissionModeLabels`（确认零引用后）
2. 在使用侧（`ToolManager.tsx`、`LocalToolSettings.tsx`）直接用 `t()` + locale key
3. locale key 命名：`toolCategory.fileRead`、`permissionMode.readOnly` 等

### 4.2 `src/types/expert.ts`

当前问题：
```typescript
export const EXPERT_CATEGORIES = { general: "通用", development: "开发", ... };
```

方案：
1. 将 `EXPERT_CATEGORIES` 改为仅存枚举 key 的数组
2. 所有显示文本通过 `t("expertCategory.${key}")` 获取
3. 同时清理 `AgentProfileManager.tsx` 中重复定义的分类名

### 4.3 `src/types/evaluator.ts`

当前问题：
```typescript
export function getDifficultyLabel(d: Difficulty): string { return "简单" / "中等" / ...; }
```

方案：
1. 函数改为接收 `t` 函数参数：`getDifficultyLabel(d, t)`
2. 或将返回值改为 locale key，由调用方 `t()` 包裹

### 4.4 `src/data/expertPresets.ts`

当前问题：50 个专家预设全部硬编码中文名称、描述、系统提示。

方案：
1. 名称和描述迁移到 locale 文件（`expertPreset.${id}.name/description`）
2. 系统提示保留不译（属于 LLM 交互数据）
3. 预设文件改为仅存结构化数据（id、category、icon 等），文本从 i18n 获取

阶段 4 完成标准：
- [ ] `types/` 中无硬编码 UI 标签
- [ ] `data/` 中预设文本已迁移到 locale 文件
- [ ] allowlist 清空（所有文件均已合规）
- [ ] CI 检测脚本从 warning 模式切换为 error 模式

---

## 目录结构变更

```
新增:
  scripts/check-hardcoded-i18n.sh     # CI 检测脚本
  scripts/.i18n-allowlist.json        # 存量豁免清单

修改:
  src/i18n/locales/*.json             # 补充 68+ 个缺失 key
  src/types/localTool.ts              # 删除硬编码标签映射
  src/types/expert.ts                 # 重构为 i18n key 引用
  src/types/evaluator.ts              # 重构函数签名
  src/data/expertPresets.ts           # 文本迁移到 locale
  src/components/**/*.tsx             # 逐步替换硬编码为 t()
  src/stores/**/*.ts                  # 逐步替换硬编码为 t()
  src/lib/**/*.ts                     # 按分类处理
  src-tauri/src/**/*.rs               # 错误消息国际化
  .github/workflows/ci.yml            # 集成 i18n 检查
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 大量 locale key 变动导致合并冲突 | 高 | 每阶段独立 PR，及时合并 |
| 翻译质量不足（非中文语言） | 中 | 标记待翻译，后期可接入翻译 API |
| `t()` 调用影响性能 | 低 | i18next 有缓存，性能影响可忽略 |
| 清理过程中引入 bug | 中 | 每批清理后运行 typecheck + test |
| Rust 后端国际化方案不成熟 | 低 | 阶段 3 第 4 层延后处理，优先用错误码 |

---

## 成功标准

1. CI pipeline 中包含硬编码字符串检测，**阻断**新增违规
2. `.i18n-allowlist.json` 条目归零
3. 所有用户可见 UI 文本通过 `t()` 获取
4. 所有 locale key 在 11 种语言文件中均存在（至少含英文 fallback）
5. `t()` 调用不再使用 fallback 参数
