# AxAgent — AGENTS.md

## 核心指令

全程中文：思考、注释、输出一律简体中文。代码注释优先中文。无论用户用什么语言提问，都必须用中文思考 + 中文回答。

## 项目概述

AxAgent 是 Tauri v2 + React 19 + TypeScript 跨平台 AI 桌面客户端。支持多 LLM 提供商、智能体引擎、工作流编辑、RAG 知识库、浏览器扩展等。

## 技术栈

前端：React 19 · TypeScript (strict) · Vite 8 · Zustand 5 · Ant Design 6 + Ant Design X · Tailwind CSS 4 · React Router v7 · react-i18next (11 种语言, 默认/回退均为 zh-CN) · Vitest + Playwright
后端：Rust 2021 · Edition 2024 · Tauri 2 · Tokio (full) · Sea-ORM (SQLite) · reqwest · tracing · thiserror
格式化：dprint (TS/JSON, 双引号+强制大括号) · rustfmt (max_width=100, tab_spaces=4)
注意：Tailwind 4 使用 `@tailwindcss/vite` 插件，不要创建 tailwind.config.js 或 postcss.config.js

## 目录架构

```
src/                         # React 前端 (npm run dev 浏览器模式走 localStorage mock)
├── components/              # chat/ workflow/ settings/ files/ skill/ terminal/ gateway/ layout/
├── pages/                   # 路由页面
├── stores/                  # Zustand 状态，四层分类：
│   ├── domain/              # 核心业务：conversation, message, stream, preference, workspace
│   ├── feature/             # 功能模块：provider, agent, skill, mcp, terminal, gateway, ...
│   ├── shared/              # 跨组件共享：ui, tab, artifact, chatWorkspace
│   └── devtools/            # 调试工具：tracer, evaluator, rl, fineTune
├── hooks/ lib/ types/ i18n/ theme/

src-tauri/                   # Rust 后端 (Cargo workspace)
├── src/                     # 主 crate
│   ├── lib.rs               # run() + register_all_commands!() 宏注册所有命令
│   ├── commands/            # 120 个命令模块，mod.rs 统一声明
│   ├── init/                # 初始化（database, plugins, services, state）
│   └── app_state.rs         # 全局 AppState
├── crates/                  # 36 个 workspace crate（另含 src-tauri/schema-gen，共 37 个成员）
│   ├── agent/               # 智能体引擎（SessionManager 等）
│   ├── providers/           # LLM 提供商抽象层
│   ├── runtime/             # WebSocket、工作流引擎、消息网关
│   ├── gateway/             # API 网关（Axum, OpenAI 兼容接口）
│   ├── tools/               # 工具系统（注册/验证/执行）
│   ├── trajectory/          # 轨迹记录、RL 引擎、技能进化
│   └── ... (其余 crate 见下方「crate 角色对照表」)

extension/ website/ e2e/ scripts/
```

## 代码规范

### 前端 (TypeScript/React)

- 组件：函数组件 + 命名导出（`export function Foo() {}`），禁止默认导出
- Store 模式：`export const useXxxStore = create<State>((set, get) => ({}))`，在 stores/index.ts re-export
- 类型：所有类型从 `@/types` 导入（barrel export），不从子文件导入
- i18n：UI 文本一律 `const { t } = useTranslation()`，禁止硬编码字符串
- 路径别名：`@/` = `src/`
- 样式：首选 Ant Design theme token + Tailwind 工具类，避免新建 CSS 文件
- **dprint 格式化**：`npm run format`（即 `dprint fmt`）必须通过，CI 强制检查，禁止提交未格式化的 TS/JSON 代码

### 后端 (Rust)

- Tauri 命令返回 `Result<T, String>`，用 `.map_err(|e| e.to_string())`
- 库 crate 错误用 `#[derive(thiserror::Error)]`，应用层（lib.rs/commands）用 `anyhow::Result`
- 模块可见性：内部用 `pub(crate)`，对外 API 用 `pub`
- 所有 `pub mod` 声明在 commands/mod.rs 中统一管理
- 数据库操作：方向在 entity 层用 sea-orm，有复杂查询逻辑用 repository 模式
- **rustfmt 格式化**：`cargo fmt` 必须通过，CI 强制检查，禁止提交未格式化的 Rust 代码
- **分层检查策略**：日常开发用 `cargo check`（秒级）快速验证类型正确；提交前必须通过 `cargo clippy -- -D warnings`（CI 强制）
- **增量编译加速**：`Cargo.toml` 已配置 `[profile.dev.build-override]` 优化第三方依赖编译，sccache 缓存 50G，避免全量重编

## 常用命令

```
npm run dev           # Vite 前端（浏览器模式，走 localStorage mock）
npm run tauri dev     # 完整 Tauri 桌面应用
npm run typecheck     # tsc --noEmit
npm run test:run      # Vitest 单元测试
npm run test:e2e      # Playwright E2E 测试
npm run format        # dprint 格式化前端
npm run build         # tsc + vite build 生产构建
cargo fmt             # rustfmt 格式化（src-tauri/ 下执行）
cargo check           # 快速类型检查（推荐日常使用，秒级完成）
cargo clippy          # Rust lint（提交前必须通过，耗时较长）
npm run bump          # 版本号升级

# Windows 跑 Rust 单元测试必须设置 __TAURI_WORKSPACE__=true
# 否则测试 exe 未嵌入 Common Controls v6 manifest，链接 tauri 的测试
# 启动即报 STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)。参考 tauri issue #11028。
__TAURI_WORKSPACE__=true cargo test -p axagent --lib commands::knowledge_source
```

## 禁区（必须遵守）

### 前端

1. **IPC 调用**：必须通过 `@/lib/invoke` 的 `invoke<T>()`，禁止直接 `import { invoke } from "@tauri-apps/api/core"`
2. **国际化**：新增 UI 文本必须在 locales/ 下全部 11 种语言文件中添加 key，禁止仅添加 zh-CN
3. **类型导入**：从 `@/types` 导入，不从子路径（如 `@/types/agent`）导入
4. **组件导出**：命名导出，不用默认导出（`export function X` ✓，`export default function X` ✗）。例外：`src/i18n/index.ts` 的 `export default i18n` 属 react-i18next 框架惯例（`initReactI18next` 返回值需默认导出），合法
5. **Tailwind**：不要创建 tailwind.config.js 或 postcss.config.js（Tailwind 4 用 vite 插件方式）
6. **Monaco Editor**：新增语言高亮必须在 vite.config.ts 的 `SHIKI_ALLOWED_LANGS` 中添加，不加入白名单不会打包

### 后端

7. **命令注册（两步）**：新增 Tauri 命令必须同时改 `commands/mod.rs`（声明模块）+ `lib.rs`（register_all_commands!() 宏注册），缺一不可
8. **异步锁**：必须 `tokio::sync::RwLock`，禁止 `std::sync::RwLock`（std guard 跨 await 是 UB，panic 会毒化）
9. **异步运行时**：不要在已有 tokio runtime 上下文中再创建嵌套 runtime
10. **数据库迁移**：新增/修改表结构必须写 migration，不要直接改 entity 了事

### 构建

11. **removeCrossorigin()**：vite.config.ts 中此插件不可删除（Tauri 自定义协议不支持 CORS 预检，删除会导致生产白屏）

### 全栈（禁止重复代码）

12. **禁止重复定义**：写新代码前，必须先检索项目是否已有相同或相似的定义（trait / struct / enum / type / interface / 工具函数 / 常量）。规则：

- **已有定义 → 必须复用**：通过 `pub use` re-export 或 `import` 引用已有定义
- **需要扩展 → 扩展而非重定义**：给已有类型加字段/方法，不要新建同义类型
- **后端权威来源层级**：所有共享数据模型（DTO、事件、配置等）的权威定义在 `axagent-harness`；`runtime-core` 作为上层抽象，通过 re-export 使用 harness 中的类型，并仅依赖 harness 提供的 trait 接口
- **前端权威来源**：类型定义在 `src/types/`，store 应 import 而非重定义 interface
- **Tauri 命令一致性**：删除后端命令时必须同步清理前端调用方；新增命令时前端类型须与后端 DTO 一致
- **删除前确认零引用**：删除任何文件/模块/函数/命令前，用 grep 确认全项目引用为零
- **浮层触发组件必须透传 ref**：任何自定义组件被 antd 浮层组件（Popover / Popconfirm / Tooltip / Dropdown / Select 等）作为 children 时，必须 `forwardRef` 并把 ref 透传到真实 DOM 元素。否则 rc-trigger 无法定位触发器，弹层会落到视口左下角并立即消失（React 19 已移除 findDOMNode，无法兜底）。模式参考 `src/components/layout/Tooltip.tsx` 的 `setTriggerRef` 透传实现。新增这类组件时，用 grep 检查其是否被浮层包裹并相应转发 ref

13. **DTO 字段命名（全站统一 camelCase）**：前后端字段命名以「Rust 侧 snake_case + serde 注解 + TS 侧 camelCase」为唯一标准（全项目 300+ 处先例，如 harness / providers / storage / runtime 各 DTO）。规则：

- **后端 Rust 结构体字段保持 snake_case（Rust / rustfmt / clippy 惯例）**，通过 `#[serde(rename_all = "camelCase")]` 注解在序列化输出 camelCase。**禁止**直接改 Rust 字段为 camelCase —— 会触发 clippy `non_camel_case_types` / `non_snake_case` 警告，违反 `cargo clippy -- -D warnings` 铁律
- **前端 TS 类型字段消费 camelCase**，与后端序列化输出对齐（如 `session_id`→`sessionId`、`duration_ms`→`durationMs`、`timestamp_ms`→`timestampMs`、`tool_calls`→`toolCalls`）。**禁止**在 TS 类型/消费处写 snake_case
- **新增/修改 DTO 两步必须同步**：① 后端结构体加 `#[serde(rename_all = "camelCase")]`；② 前端 `src/types/` 类型改 camelCase 并同步所有消费方（组件 / store）
- **命令参数（invoke 传参）同样用 camelCase**：Tauri v2 的 `#[tauri::command]` 宏默认 `rename_all = "camelCase"`，会把 Rust 参数名 `session_id` 校验为 JS 侧键名 `sessionId`，传 snake_case 会直接报 `missing required key sessionId`（IPC 层拒绝，不会进 handler）。因此前端 `invoke()` 传参键名**必须用 camelCase**（如 `sessionId`、`trajectoryId`）。注意：参数名带前导下划线时（`_url`）前导下划线被忽略（JS 侧传 `url`）。DTO 字段名与命令参数名实际是同一套 camelCase 规则。新增 invoke 调用后建议跑 `.workbuddy/tmp/scan_ipc_args.py` 做前后端参数名一致性扫描

## 后端错误码 i18n 规范（强制）

后端用户可见错误通过**错误码映射**做国际化，而非硬编码字符串。机制：后端返回 `ErrorResponse { code, category, detail, params }`（`src-tauri/src/commands/error.rs` 定义），前端按 `code` 走现有 i18n 翻译层。

### 后端（Rust）

1. **命令错误必须带错误码**：返回 `Result<T, ErrorResponse>` 或通过 `CommandError`（`commands/error.rs`）包装；禁止把裸英文/中文 `e.to_string()` 直接作为用户可见错误回传。
2. **复用既有错误码常量**：码定义在 `src-tauri/src/commands/error_code.rs`（业务域）与 `crates/harness/src/error_codes.rs`（基座域），命名 `{CATEGORY}_{SHORT_NAME}` 全大写下划线（如 `CONVERSATION_NOT_FOUND`、`TOOL_NOT_FOUND`）。新增错误优先复用/扩展这两个文件的常量，不要凭空写字符串字面量。
3. **动态参数走 `params`**：需要插值的内容（如 ID、名称）放进 `params` 而非拼进 `detail`，前端按 `t("error.${code}", params)` 插值。
4. **错误归类**：`ErrorCategory`（Unrecoverable / Recoverable / Permission / Timeout 等）用于前端智能分支（重试/授权引导），请在构造 `ErrorResponse` 时正确设置。

### 前端（TypeScript/React）

5. **消费错误码而非裸字符串**：解析后端错误 `JSON.parse(e.message)` 取 `code` + `params`，用 `t("error.${code}", params)` 翻译；不要 `message.error(String(e))` 直接显示原始串。参考 `src/components/chat/WorkflowProgressPanel.tsx` 的 `translateError` 模式。
6. **后端错误码翻译统一平铺在顶层 `error` 段**：键即后端 `code` 字符串（例如 `error: { CONVERSATION_NOT_FOUND: "..." }`），与既有 14 个 camelCase UI 错误（如 `error.network`）并存于同一 `error` 对象。**禁止**再散落到 `chat.workflow.errorDetail`、`quickbar.result.errorCode` 等子段。
7. **码表前后端对齐**：后端 `error_code.rs` / `error_codes.rs` 中 `="XXX_YYY"` 的值 ⊆ 前端 11 语言 `error` 段的翻译键。新增后端错误码时，**必须同步补齐 11 种语言**（zh-CN 为源）的 `error.${CODE}` 翻译，缺译会导致该语言下显示原始码串。

## Store 分类规则

新增 Zustand store 按以下规则放置：

- 核心业务（消息、会话、流式）→ `stores/domain/`
- 功能特性（网关、技能、终端、知识库）→ `stores/feature/`
- 跨组件 UI 状态（标签页、侧栏、工作区布局）→ `stores/shared/`
- 开发调试图表（追踪、评估、RL）→ `stores/devtools/`

## Rust 后端：Harness 架构准则（强制约束）

### 核心原则

**依赖方向铁律：** `组件 → harness ← 实现`

```
foundation (harness 零 axagent-* 依赖；entities 仅依赖 harness):
  harness        — trait 契约 + 纯 DTO
  entities       — SeaORM 数据定义（共享数据模型，仅依赖 harness）
  disk-cache, rt-dashboard, rt-theme
  schema-gen     — 代码生成辅助（SeaORM schema 生成），仅依赖 harness，位于 src-tauri/ 根目录（不在 crates/ 下）

implementor (harness + entities + 兄弟 implementor):
  dao, storage, migration, kit, crypto, mcp, search, providers,
  cache, credential, prompt-guard, telemetry, trajectory, plugins,
  npm, document-parser, rt-webhook, scanner

hybrid (harness + 按需 implementor，禁止依赖 consumer 和 entities):
  tools, rt-messaging, rt-workflow

consumer (仅 harness):
  agent, orchestrator, runtime-core, gateway

wiring (全栈胶水，通过 harness trait 传递能力):
  runtime, src/commands/, src/init/
```

### 铁律清单

1. **禁止循环依赖** — 任何两个 crate 不能互相依赖。`harness` 不得依赖任何 axagent-* crate；`entities` 仅可依赖 `harness`（共享 DTO 层），二者均为 foundation 叶子节点。

2. **消费者禁止越过 harness** — agent / gateway / orchestrator / runtime-core 等"消费者" crate，只能依赖 `axagent-harness`（获取 trait 接口和 DTO），不得直接依赖 dao / entities / mcp / crypto / storage / kit 等实现层。

3. **实现方可以依赖 entities** — storage / dao / migration 等 implementor crate 使用 `axagent-entities` 的数据定义是正确分层依赖，不是违规。

4. **不允许重复类型体系** — 所有共享类型（ConversationMessage、TokenUsage、Session、PermissionMode、HookEvent 等）的权威定义在 `axagent-harness`。其他 crate 通过 `pub use axagent_harness::X` 引用，不得重复定义。发现重复 → 改为 pub use + 删除本地定义及所有 From 转换。

5. **test 代码分层处理** — consumer crate（agent / gateway / runtime-core / orchestrator）的测试只能通过 `axagent_harness::test_support::*` mock，禁止在 dev-dependencies 中引入任何实现层 crate。implementor / hybrid / wiring 的测试优先使用 mock；仅在需要真实数据库连接等集成测试基础设施时，允许 dev-dependencies 引入 `axagent-dao`（用于 `create_test_pool`），但不得引入其他实现层 crate。

6. **新增 Rust crate 时必须声明归属** — 每个新 crate 必须在 `crates/README.md` 或 AGENTS.md 中标注是 `foundation`、`consumer`、`implementor`、`hybrid` 还是 `wiring`，并符合对应的依赖约束。

### 验证方式

新增/修改任何 `Cargo.toml` 中的 `axagent-*` 依赖前，对照上述铁律检查依赖方向是否正确。允许的依赖组合：

| crate 角色  | 可依赖的 axagent-* crate                                                                                                                                   |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| consumer    | `axagent-harness`                                                                                                                                          |
| implementor | `axagent-harness` + `axagent-entities` + 其他 implementor crate（dao/kit/mcp/search/storage/crypto 等）                                                    |
| hybrid      | `axagent-harness` + 按需依赖具体 implementor（如 kit、crypto、search、mcp 等）。禁止依赖 consumer（agent、runtime-core、gateway、orchestrator）和 entities |
| wiring      | 所有（但必须通过 harness trait 传递能力，不得直接暴露实现层给 consumer）                                                                                   |
| foundation  | `harness` 零依赖；`entities` 仅依赖 `harness`                                                                                                              |

### crate 角色对照表

| foundation                           | consumer                | implementor                     | hybrid             | wiring                 |
| ------------------------------------ | ----------------------- | ------------------------------- | ------------------ | ---------------------- |
| `harness`                            | `agent`, `orchestrator` | `dao`                           | **`tools`**        | `runtime`              |
| `disk-cache`                         | `runtime-core`          | `storage`                       | **`rt-messaging`** | 二进制 `src/commands/` |
| `rt-dashboard`                       | `gateway`               | `migration`                     | **`rt-workflow`**  | `src/init/`            |
| `rt-theme`, `entities`, `schema-gen` | `crdt`                  | `kit`                           |                    | `axagent-mobile`       |
| `axagent-agent-command-types`, `axagent-agent-macro` |                         | `cache`, `crypto`, `credential` |                    |                        |
|                                      |                         | `mcp`, `search`                 |                    |                        |
|                                      |                         | `providers`                     |                    |                        |
|                                      |                         | `prompt-guard`                  |                    |                        |
|                                      |                         | `telemetry`                     |                    |                        |
|                                      |                         | `trajectory`                    |                    |                        |
|                                      |                         | `plugins`, `npm`                |                    |                        |
|                                      |                         | `document-parser`               |                    |                        |
|                                      |                         | `rt-webhook`                    |                    |                        |
|                                      |                         | `scanner`, `device`             |                    |                        |

> **规则**：foundation（harness 零依赖，entities 仅依赖 harness）；consumer 仅依赖 harness；implementor 可依赖 harness + entities + 其他 implementor。

> **注**：`runtime`（wiring）和 `dao`（implementor）因其架构角色，对兄弟 crate 的直接依赖属于预期行为，不计入违规。

## Git 规范

Conventional Commits + 中文描述。类型映射：
`feat` → 🚀 新功能 | `fix` → 🐛 Bug 修复 | `refactor` → 🔨 重构 | `style` → 🎨 样式
`docs` → 📝 文档 | `test` → 🧪 测试 | `chore` → 📦 杂项 | `ci` → 🔧 CI/CD
`build` → 🏗️ 构建 | `perf` → ⚡ 性能提升

### 铁律

- **禁止 `git checkout` / `git switch` 丢弃未提交的变更** — 会导致代码丢失、前功尽弃。需要回退时，先 `git stash` 暂存或 `git commit` 提交当前进度，再用 `git checkout` 切分支。严禁在 `git status` 显示有未提交变更时执行 `git checkout`。
