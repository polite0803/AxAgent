# AxAgent — CLAUDE.md

## 核心指令
全程中文：思考、注释、输出一律简体中文。代码注释优先中文。无论用户用什么语言提问，都必须用中文思考 + 中文回答。

## 项目概述
AxAgent 是 Tauri v2 + React 19 + TypeScript 跨平台 AI 桌面客户端。支持多 LLM 提供商、智能体引擎、工作流编辑、RAG 知识库、浏览器扩展等。

## 技术栈
前端：React 19 · TypeScript (strict) · Vite 8 · Zustand 5 · Ant Design 6 + Ant Design X · Tailwind CSS 4 · React Router v7 · react-i18next (11 种语言, 默认/回退均为 zh-CN) · Vitest + Playwright
后端：Rust 2021 · Tauri 2 · Tokio (full) · Sea-ORM (SQLite) · reqwest · tracing · thiserror
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
│   ├── lib.rs               # run() + generate_handler![] 注册所有命令
│   ├── commands/            # 67 个命令模块，mod.rs 统一声明
│   ├── init/                # 初始化（database, plugins, services, state）
│   └── app_state.rs         # 全局 AppState
├── crates/                  # 14 个 workspace crate
│   ├── core/                # 数据库实体、向量存储、RAG、加密
│   ├── agent/               # 智能体引擎（SessionManager 等）
│   ├── providers/           # LLM 提供商抽象层
│   ├── runtime/             # WebSocket、工作流引擎、消息网关
│   ├── gateway/             # API 网关（Axum, OpenAI 兼容接口）
│   ├── tools/               # 工具系统（注册/验证/执行）
│   ├── trajectory/          # 轨迹记录、RL 引擎、技能进化
│   └── ... (code_engine, migration, plugins, telemetry, acp)

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
- **clippy 零警告**：`cargo clippy -- -D warnings` 必须通过，CI 强制检查，禁止提交含 clippy 警告的代码

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
cargo clippy          # Rust lint（src-tauri/ 下执行）
npm run bump          # 版本号升级
```

## 禁区（必须遵守）

### 前端
1. **IPC 调用**：必须通过 `@/lib/invoke` 的 `invoke<T>()`，禁止直接 `import { invoke } from "@tauri-apps/api/core"`
2. **国际化**：新增 UI 文本必须在 locales/ 下全部 11 种语言文件中添加 key，禁止仅添加 zh-CN
3. **类型导入**：从 `@/types` 导入，不从子路径（如 `@/types/agent`）导入
4. **组件导出**：命名导出，不用默认导出（`export function X` ✓，`export default function X` ✗）
5. **Tailwind**：不要创建 tailwind.config.js 或 postcss.config.js（Tailwind 4 用 vite 插件方式）
6. **Monaco Editor**：新增语言高亮必须在 vite.config.ts 的 `SHIKI_ALLOWED_LANGS` 中添加，不加入白名单不会打包

### 后端
7. **命令注册（两步）**：新增 Tauri 命令必须同时改 `commands/mod.rs`（声明模块）+ `lib.rs`（generate_handler![] 注册），缺一不可
8. **异步锁**：必须 `tokio::sync::RwLock`，禁止 `std::sync::RwLock`（std guard 跨 await 是 UB，panic 会毒化）
9. **异步运行时**：不要在已有 tokio runtime 上下文中再创建嵌套 runtime
10. **数据库迁移**：新增/修改表结构必须写 migration，不要直接改 entity 了事

### 构建
11. **removeCrossorigin()**：vite.config.ts 中此插件不可删除（Tauri 自定义协议不支持 CORS 预检，删除会导致生产白屏）

## Store 分类规则
新增 Zustand store 按以下规则放置：
- 核心业务（消息、会话、流式）→ `stores/domain/`
- 功能特性（网关、技能、终端、知识库）→ `stores/feature/`
- 跨组件 UI 状态（标签页、侧栏、工作区布局）→ `stores/shared/`
- 开发调试图表（追踪、评估、RL）→ `stores/devtools/`

## Git 规范
Conventional Commits + 中文描述。类型映射：
`feat` → 🚀 新功能 | `fix` → 🐛 Bug 修复 | `refactor` → 🔨 重构 | `style` → 🎨 样式
`docs` → 📝 文档 | `test` → 🧪 测试 | `chore` → 📦 杂项 | `ci` → 🔧 CI/CD
`build` → 🏗️ 构建 | `perf` → ⚡ 性能提升
