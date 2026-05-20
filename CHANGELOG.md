# Changelog

All notable changes to this project will be documented in this file.

---

## [v1.4.1] - 2026-05-08

### 🚀 新功能

- **智能体执行可视化面板**: 右侧 AgentExecutionPanel，含 Pool/Timeline/Replay 三标签页，可拖拽调整宽度
- **执行轨迹回放**: TrajectoryReplay 回放动画，支持倍速播放和逐步查看
- **ExecutionPhase 状态机**: 7 阶段统一管理 (idle→planning→executing→completed/failed/cancelled)
- **Zustand DevTools**: executionStore 集成 devtools 中间件
- **新用户欢迎向导**: 5 步引导 (欢迎→检测→预设→概览→就绪)
- **智能配置检测**: Ollama 端口探测 + 环境变量 API Key 自动检测
- **一键快速预设**: Ollama 本地 / OpenAI 云 / 最小配置
- **交互式教程**: Portal 覆盖层，4 步定位核心功能
- **上下文帮助系统**: HelpPanel 全局面板 (? 键打开) + ContextHelp 组件
- **消息续写增强**: continue_message 命令，支持分支/追加续写
- **消息主题分组**: 自动识别主题 + TopicGroupDivider 折叠/重命名/合并/删除

### 🔨 重构

- **状态管理简化**: 新建 executionStore 统一执行态，合并 trajectoryStore
- **agentStore 精简**: 事件监听委托给 executionStore，移除重复代码
- **ScrollToMessageContext**: 提取滚动上下文，支持面板内点击跳转消息

### 🐛 修复

- executionStore 内存泄漏修复 (_latestMessageIdByConv + clearConversation)
- AgentExecutionPanel destroyInactiveTabPane 懒加载
- HelpPanel 打开时锁定 body 滚动
- TrajectoryReplay 卸载安全 + 快速切换竞态保护
- WelcomeWizard 预设防重复点击
- 修复 6 个预存类型错误 (ImportExportModal, WorkflowEditor 等)

---

## [v2.0.1] - 2026-05-18

### 🚀 新功能

- **安全加固第三轮**: SSRF 防护增强、25 项安全审计问题修复、CREATE_NO_WINDOW 防弹窗
- **移动端深度优化**: 底部导航栏 + Drawer 滑出导航 + 闪现式浮动按钮
- **图标系统升级**: 全面引入 Iconify Fluent 图标，替换侧边栏/设置/右面板图标
- **用户画像合并**: 个人信息弹窗并入侧边栏
- **桌面分辨率自适应**: 启动时自动检测三档布局适配
- **密码粘贴按钮**: 所有密码/密钥输入框添加粘贴按钮
- **侧边栏新增**: Terminal + Files 导航入口
- **页面切换动画**: 恢复被全局禁用的动画系统
- **i18n 硬编码字符串清理**: 全量迁移，CI 强制检查

### 🔨 重构

- **错误类型统一**: 权限检查 trait 抽象
- **API 边界文档化**: TaskManager 迁移后台服务
- **retry_policy**: 退避算法改用 backoff crate

### 🐛 修复

- 消除 7 处 provider/task_system expect panic
- 修复 CI 失败 — i18n 硬编码字符串 + 测试断言错误
- 修复 Android 安全区适配、粘贴按钮权限、CSP 白屏
- 修复 Mobile 导航卡滞 — 消除 ContentArea 完全重建
- 修复启动白屏 — expertStore i18n.t 过早调用 + mcp_stdio 测试
- 消除前端 store 碎片，删除 loop_detector 死代码
- react-doctor 评分修复 61→63
- CI 使用 cargo-nextest 替代 cargo test（快 2-3x + swap 8GB）

### 🎨 样式

- dprint 单行 if 强制大括号
- cargo fmt 补充修复

---

## [v2.0.0] - 2026-05-11

### 🚀 新功能

- **Rust 2024 edition 升级**: 全项目迁移，兼容修复
- **本地 Candle 推理引擎**: inference.rs 集成真实 Candle LLaMA 推理
- **i18n 全面清理**: 全项目硬编码字符串迁移、CI 严格模式、豁免机制

### 🔨 重构

- **Runtime 重构**: 拆分 runtime 为 runtime-core + 5 个子系统 crate（rt-workflow / rt-messaging / rt-webhook / rt-dashboard / rt-theme）
- **旧工具体系迁移**: 完整迁移至 Tool trait，删除 builtin_tools/builtin_handlers
- **全项目功能去重**: 架构清理 (v1.5.1)

### 🐛 修复

- 安全审计全面修复 — XSS/命令注入/数据竞争/错误吞没等 25 项问题
- 消除 43% react-doctor 问题（2359→1352）
- conversationStore 拆分、优雅关闭
- E2E 测试加强
- 零 TypeScript 错误、Rust 2024 edition 兼容
- 修复 Release 产物路径偏移导致 macOS/Linux/Windows MSI 缺失

### 🔧 CI/CD

- 修复 PR CI 三大缺陷（split runner、OOM、timeout）
- mobile-build → release.yml 统一
- 使用 cargo-nextest 替代 cargo test
- 新增 i18n 硬编码字符串检查

---

## [v1.6.0] - 2026-05-11

### 🐛 修复

- **多模型接入全面修复**: 全链路连通性与参数传递修正

---

## [v1.5.9] - 2026-05-10

### 🚀 新功能

- **OpenClaw 插件生态兼容**: 完整 npm 包安装/管理/发布流程
- **插件市场**: 支持 npm 搜索安装栏与确认弹窗
- **插件自动注册**: Agent 提供者 — 插件 agents 自动注册/注销；SkillInstaller 插件技能自动部署
- **npm crate**: 新建 axagent-npm crate，定义 types 和依赖，实现 NpmRegistry 解析/获取/API
- **tarball 流式解压**: npm 包根目录检测

### 🔨 重构

- 统一工作流角色系统，优化提示词组装节约 token

### 🐛 修复

- 插件系统合并后的遗漏和缺陷修复
- 删除设置-关于中的官网和 GitHub 链接
- CI rustfmt 格式修复

---

## [v1.5.8] - 2026-05-10

### 🔨 重构

- **i18n 架构迁移**: types/data 标签映射全量迁移至 i18n-native 架构
- Skills 从导航移至设置页面

### 🐛 修复

- 删除所有 t() fallback 参数（Phase 2 Batch 4）
- 修复 E2E 测试适配工作流页面重构
- 同步 61 个缺失 i18n key 到全部 11 种语言

### 🔧 CI/CD

- 新增 i18n 硬编码字符串 CI 检查

---

## [v1.5.7] - 2026-05-09

### 🐛 修复

- 会话删除后 agent_sessions 和 summaries 孤行残留清理

---

## [v1.5.6] - 2026-05-09

### 🐛 修复

- Agent 权限确认弹窗完全不工作 — 两个 Bug 导致链路断裂
- Agent 模式无消息输出卡死 — 添加状态事件 + 超时保护
- 全部表单元素缺少 id/name 属性导致浏览器 autofill 不可用
- 上下文图谱图例点击误触发面板折叠
- AI 生成标题基于完整对话历史而非仅首条消息

### 🔧 CI/CD

- 修复 Linux CI libc RLIMIT 类型不匹配

---

## [v1.5.5] - 2026-05-09

### 🚀 新功能

- 文档工具补齐

---

## [v1.5.3] - 2026-05-09

### 🚀 新功能

- **P0 高级 RAG 端到端集成**: 查询增强（HyDE / MultiQuery / Decomposition）
- **模型下载管理器**: GGUF 按需下载 + 前端下载入口
- **Candle 本地推理引擎**: InferenceEngine 提供 rerank/judge 接口
- **重排序重构**: Reranker 改为 trait-based 后端

---

## [v1.5.2] - 2026-05-08

### 🚀 新功能

- **高级 RAG 管线**: 查询增强/重排序/自检全流程打通
- **MCP 系统全面重构**: 修复 12 项缺陷

---

## [v1.5.1] - 2026-05-08

### 🔨 重构

- **全项目功能去重与架构清理**
- **Runtime 拆分**: runtime-core + 5 个子系统 crate（rt-workflow / rt-messaging / rt-webhook / rt-dashboard / rt-theme）
- **旧工具体系迁移**: 完整迁移至 Tool trait，删除 builtin_tools/builtin_handlers

### 🐛 修复

- 修复 webhook dispatch 签名 + 补依赖
- 修复 rt-messaging 编译错误
- Android 启动崩溃修复、图标一致性增强

---

## [v1.4.9] - 2026-05-08

### 🔧 CI/CD

- CI runner OOM 修复
- 测试失败修复

---

## [v1.4.8] - 2026-05-08

### 🚀 新功能

- **知识源统一管理**: RAG/Wiki/Memory 融合为 KnowledgeHubPage
- **SourceManager 重构**: 丰富卡片式布局，统一视图导航
- **Wiki 知识图谱增强**: 高级交互、图谱洞察

---

## [v1.4.7] - 2026-05-08

### 🧪 测试

- 修复 clippy 警告、测试 AppState 缺失字段

---

## [v1.4.6] - 2026-05-08

### 🐛 修复

- DeepSeek thinking chain 修复、状态同步、会话清理
- 全面的状态一致性修复（conversationStore 等）

---

## [v1.4.5] - 2026-05-08

### 🐛 修复

- Android 运行时稳定性改进 — panic 处理和 TLS provider fallback
- APK 签名流程修复

---

## [v1.4.4] - 2026-05-08

### 🚀 新功能

- **移动端构建支持**: Android APK/AAB + iOS IPA 构建流程
- **移动端平台守卫**: 插件注册/命令注册/服务条件编译
- **全模块增强**: agent / core / runtime / telemetry / tools / frontend

### 🐛 修复

- 修复 26 个 Rust 编译/clippy 错误
- 修复 iOS 和 Android OpenSSL 交叉编译问题
- 修复 GitHub Release 桌面端与移动端构建失败
- 修复 PR CI 构建错误
- cargo-deny 许可证检查修复
- 修复 8 个 runtime 测试 + 9 个 trajectory 测试

### 🔧 CI/CD

- 安全审计: cargo-audit + cargo-deny + cargo-vet
- 覆盖率阈值调整至 45%

---

### 🚀 新功能

- **MCP 服务器别名/描述 + 三模式 (自动/手动/禁用) 完善** ([#2ad744a](https://github.com/polite0803/AxAgent/commit/2ad744a))
- **完整 n8n 工作流导入 — 批量+单条，AgentRole/Expert/Profile 自动装配** ([#3b7a48f](https://github.com/polite0803/AxAgent/commit/3b7a48f))
- **AgentRole 数据库驱动 + Open Agent Spec 导入 + AgentProfile 可视化管理** ([#0061dba](https://github.com/polite0803/AxAgent/commit/0061dba))
- **会话即工作流 — 统一岗位系统 + 交互设计强化** ([#57435a1](https://github.com/polite0803/AxAgent/commit/57435a1))
- **Paperclip 风格 UI 改造 — 深色配色 + Linear 侧栏 + 极简圆角** ([#50f5a7f](https://github.com/polite0803/AxAgent/commit/50f5a7f))
- **对话页侧栏支持拖拽调整宽度** ([#207a456](https://github.com/polite0803/AxAgent/commit/207a456))

### 🐛 Bug 修复

- **CI 环境 document_dir() 为 None 时优雅降级** ([#2ad744a](https://github.com/polite0803/AxAgent/commit/2ad744a))
- **test_rank_nodes 放宽断言，兼容 HashMap 非确定性排序** ([#0fa3c7e](https://github.com/polite0803/AxAgent/commit/0fa3c7e))
- **移除 InputArea 中重复的 Expert 选择器** ([#ce60b2b](https://github.com/polite0803/AxAgent/commit/ce60b2b))
- **Rust 1.95.0 clippy 新 lint + extract_goal type 修正** ([#09f1b24](https://github.com/polite0803/AxAgent/commit/09f1b24))
- **修复 6 个预存测试失败 — extract_goal + infer_email + tokio 嵌套 runtime** ([#02238dc](https://github.com/polite0803/AxAgent/commit/02238dc))
- **修复启动闪退 — 在 Tauri runtime 外初始化数据库** ([#6753658](https://github.com/polite0803/AxAgent/commit/6753658))
- **BuddyWidget 拖动重写 + CI 前端检查修复** ([#487fe01](https://github.com/polite0803/AxAgent/commit/487fe01))
- **继续完善 — UI 反馈和流程缺陷修复** ([#fd9281c](https://github.com/polite0803/AxAgent/commit/fd9281c))
- **会话全流程关键缺陷修复** ([#c9c6455](https://github.com/polite0803/AxAgent/commit/c9c6455))
- **P1 SSRF 防护 + P3.2 WebFetch prompt 生效** ([#a443936](https://github.com/polite0803/AxAgent/commit/a443936))
- **工具调用全面修复 — regenerate/regenerate_with_model 注入 web_search + Agent 模式 WebSearch 描述强化** ([#6be8978](https://github.com/polite0803/AxAgent/commit/6be8978))
- **web_search 优先使用配置的搜索提供商 API，DuckDuckGo 仅作 fallback** ([#91d2fa6](https://github.com/polite0803/AxAgent/commit/91d2fa6))
- **强化 web_search 系统提示 — 明确要求 LLM 必须使用搜索而非拒绝** ([#9690b37](https://github.com/polite0803/AxAgent/commit/9690b37))
- **添加 web_search 诊断日志，追踪搜索提供商状态** ([#74d3893](https://github.com/polite0803/AxAgent/commit/74d3893))
- **修复非中文语言资源文件中的中文内容** ([#9d79e75](https://github.com/polite0803/AxAgent/commit/9d79e75))
- **web_search 改用 DuckDuckGo Instant Answer API + 修复 URL 编码** ([#99f5d04](https://github.com/polite0803/AxAgent/commit/99f5d04))
- **全局 UI/UX 缺陷修复 — 消息渲染、搜索、工作流导入、设置导航等** ([#72eac44](https://github.com/polite0803/AxAgent/commit/72eac44))
- **全局缺陷修复 — dream_consolidator 实现 + 错误处理 + 类型安全** ([#99cea3f](https://github.com/polite0803/AxAgent/commit/99cea3f))
- **删除所有 as any 引用，改为正确删除 AtomicSkill 死代码** ([#1527bf8](https://github.com/polite0803/AxAgent/commit/1527bf8))
- **全部 Rust 0 warning + TypeScript 0 error** ([#06e6c1d](https://github.com/polite0803/AxAgent/commit/06e6c1d))
- **trajectory unused variable** ([#5811e28](https://github.com/polite0803/AxAgent/commit/5811e28))
- **清理 Rust/TS 警告和错误** ([#8ab18af](https://github.com/polite0803/AxAgent/commit/8ab18af))

### ⚡ 性能提升

- **持续优化 — Agent WebSearchTool 统一 + builtin fetch SSRF** ([#ddef41b](https://github.com/polite0803/AxAgent/commit/ddef41b))
- **P3.1 RAG 查询缓存 — 跳过重复向量搜索** ([#058b46b](https://github.com/polite0803/AxAgent/commit/058b46b))
- **P0 优化 — 统一搜索路径 + API key 加密存储** ([#eb78493](https://github.com/polite0803/AxAgent/commit/eb78493))

### 🔨 重构

- **拆分 conversationStore.ts 为多职责 store + dprint 格式化修复** ([#20f9aba](https://github.com/polite0803/AxAgent/commit/20f9aba))
- **原子技能体系删除 + 工作流节点统一为 AgentProfile 驱动** ([#689fb26](https://github.com/polite0803/AxAgent/commit/689fb26))

### 🎨 样式

- **dprint + rustfmt + clippy 零警告** ([#b16089b](https://github.com/polite0803/AxAgent/commit/b16089b))

### 🔧 CI/CD

- **全面修复 CI 流程 — 统一 rustfmt + dprint 检查 + 本地 CI 模拟 + pre-push 钩子** ([#afc1cf9](https://github.com/polite0803/AxAgent/commit/afc1cf9))
- **限制 dependabot PR 数量并跳过重型 CI** ([#dd83d2b](https://github.com/polite0803/AxAgent/commit/dd83d2b))

### 📝 文档

- **完整同步 11 种语言文件 — zh-CN 补全 + 各语言按源填充** ([#5269f7f](https://github.com/polite0803/AxAgent/commit/5269f7f))

---

## [v1.3.9] - 2026-05-04

### 🚀 新功能

- **完整后台任务系统 — 数据库持久化 + 真实执行 + 前端面板** ([#1d30759](https://github.com/polite0803/AxAgent/commit/1d30759))

### 🐛 Bug 修复

- **全面清理前后端参数不匹配 — agent_update_session + Tool 去重** ([#da7ace5](https://github.com/polite0803/AxAgent/commit/da7ace5))
- **修复 DeepSeek API "Tool names must be unique" 错误** ([#be346f3](https://github.com/polite0803/AxAgent/commit/be346f3))
- **SkillErrorFallback 硬编码字符串国际化 + storage.rs block_on 修复** ([#53d3669](https://github.com/polite0803/AxAgent/commit/53d3669))

### 🔨 重构

- **SkillErrorFallback 硬编码字符串国际化 + storage.rs block_on 修复** ([#53d3669](https://github.com/polite0803/AxAgent/commit/53d3669))

---

## [v1.3.8] - 2026-05-04

### 🚀 新功能

- **Skill 热拔插自定义功能体系完整实现** ([#7917637](https://github.com/polite0803/AxAgent/commit/7917637))

### 🐛 Bug 修复

- **修复 storage.rs block_in_place 在 current_thread runtime 下 panic** ([#32615f5](https://github.com/polite0803/AxAgent/commit/32615f5))
- **修复 CI npm install 失败 — 强制使用官方 npm 源** ([#e197b44](https://github.com/polite0803/AxAgent/commit/e197b44))
- **修复 repo_integration 测试缺少 expert_role_id 字段** ([#6b0a2d0](https://github.com/polite0803/AxAgent/commit/6b0a2d0))

---

## [v1.3.7] - 2026-05-03

### 🚀 新功能

- **Wiki 知识图谱 + 设置面板优化 + Migration 整理** ([#70853c7](https://github.com/polite0803/AxAgent/commit/70853c7))

### 🐛 Bug 修复

- **release workflow setup-protoc 缺少 repo-token 导致 API rate limit** ([#d72b8a3](https://github.com/polite0803/AxAgent/commit/d72b8a3))
- **修复 storage.rs 嵌套 tokio runtime 导致的 CI 测试失败** ([#e0382e0](https://github.com/polite0803/AxAgent/commit/e0382e0))

---

## [v1.3.6] - 2026-05-03

### 🚀 新功能

- **智能体能力全面升级** ([#179ecc7](https://github.com/polite0803/AxAgent/commit/179ecc7))

### 📦 杂项

- **cargo clippy + fmt 修复** ([#adf42c0](https://github.com/polite0803/AxAgent/commit/adf42c0))

---

## [v1.3.5] - 2026-05-03

### 🚀 新功能

- **统一工具体系到 axagent-tools + 前端驱动 i18n** ([#781e2b0](https://github.com/polite0803/AxAgent/commit/781e2b0))

### 🐛 Bug 修复

- **simplify get_all_memories to always use new runtime** ([#11c00b8](https://github.com/polite0803/AxAgent/commit/11c00b8))
- **add missing approval_status column to tool_executions table** ([#3bda8cb](https://github.com/polite0803/AxAgent/commit/3bda8cb))
- **handle nested tokio runtime in get_all_memories** ([#968fc98](https://github.com/polite0803/AxAgent/commit/968fc98))

### 🎨 样式

- **fix fmt ordering for prompt_templates module** ([#bf8f569](https://github.com/polite0803/AxAgent/commit/bf8f569))

---

## [v1.3.4] - 2026-05-02

### 🚀 新功能

- **添加提示词模板功能，支持在聊天、LLM节点和Skill节点中使用** ([#63b8186](https://github.com/polite0803/AxAgent/commit/63b8186))
- **技能前端扩展机制 — 动态导航/命令/面板/设置段** ([#1efbc55](https://github.com/polite0803/AxAgent/commit/1efbc55))

### 🐛 Bug 修复

- **add missing prompt_templates module and fix clippy warnings** ([#bb544f6](https://github.com/polite0803/AxAgent/commit/bb544f6))
- **unwrap Option from checked_div in metrics.rs** ([#071a250](https://github.com/polite0803/AxAgent/commit/071a250))
- **resolve all clippy warnings** ([#9eb46b3](https://github.com/polite0803/AxAgent/commit/9eb46b3))
- **export setupPlanEventListeners from stores** ([#79149c7](https://github.com/polite0803/AxAgent/commit/79149c7))
- **set documents_root to temp_dir in persist_attachments test** ([#cfe322f](https://github.com/polite0803/AxAgent/commit/cfe322f))

### 🎨 样式

- **apply dprint formatting** ([#cad434a](https://github.com/polite0803/AxAgent/commit/cad434a))
- **cargo fmt formatting for skills.rs** ([#265874b](https://github.com/polite0803/AxAgent/commit/265874b))

---

## [v1.3.3] - 2026-05-02

### 🐛 Bug 修复

- **resolve clippy warnings and format issues** ([#1a2e120](https://github.com/polite0803/AxAgent/commit/1a2e120))
- **Add missing profile.options translations** ([#2832fc4](https://github.com/polite0803/AxAgent/commit/2832fc4))

---

## [v1.3.2] - 2026-05-02

### 🚀 新功能

- **创建 12 个轨迹表 SeaORM entity + 迁移** ([#1a1641e](https://github.com/polite0803/AxAgent/commit/1a1641e))

### 🐛 Bug 修复

- **Wiki 集成修复 + Gateway 参数对齐 + 工作流编辑器缺陷修复** ([#2b75058](https://github.com/polite0803/AxAgent/commit/2b75058))
- **补齐 WorkflowNode::Validation 分支消除 non-exhaustive patterns 错误** ([#1a7ddac](https://github.com/polite0803/AxAgent/commit/1a7ddac))
- **创建 notes + knowledge 六张表迁移** ([#2de0279](https://github.com/polite0803/AxAgent/commit/2de0279))
- **创建遗漏的7张数据库表迁移** ([#1359fa1](https://github.com/polite0803/AxAgent/commit/1359fa1))

### 🔨 重构

- **storage.rs 完整 rusqlite→SeaORM 重构** ([#2fd9085](https://github.com/polite0803/AxAgent/commit/2fd9085))
- **scheduled_task.rs rusqlite→SeaORM** ([#de6584c](https://github.com/polite0803/AxAgent/commit/de6584c))
- **builtin_tools 部分重构 rusqlite→SeaORM** ([#43c90ab](https://github.com/polite0803/AxAgent/commit/43c90ab))

---

## [v1.3.1] - 2026-05-01

### 🚀 新功能

- **后端功能扩展 + 前端孤岛接线全面修复** ([#8a49b23](https://github.com/polite0803/AxAgent/commit/8a49b23))

### 🐛 Bug 修复

- **创建工作流系统数据库表迁移 + 恢复种子数据** ([#0224566](https://github.com/polite0803/AxAgent/commit/0224566))
- **smart_router 复杂任务判定修复 (&&→||)** ([#edbc6eb](https://github.com/polite0803/AxAgent/commit/edbc6eb))
- **清理 rustfmt 配置—移除无效选项消除 93 个 Unknown 警告** ([#df6e504](https://github.com/polite0803/AxAgent/commit/df6e504))
- **clippy manual_clamp + RwLock 类型不匹配修复** ([#046dba5](https://github.com/polite0803/AxAgent/commit/046dba5))
- **Release CI 添加 actions:write 权限修复跨 workflow 触发** ([#ad35ff8](https://github.com/polite0803/AxAgent/commit/ad35ff8))
- **修复 Nightly Build + Deploy Website CI (pnpm→npm)** ([#739fdbd](https://github.com/polite0803/AxAgent/commit/739fdbd))
- **数据库迁移+CI环境修复** ([#20b483e](https://github.com/polite0803/AxAgent/commit/20b483e))
- **cargo fmt --all 修复 CI 格式检查** ([#fa0c754](https://github.com/polite0803/AxAgent/commit/fa0c754))
- **修复测试函数中 AppState 初始化的类型不匹配** ([#87b5dab](https://github.com/polite0803/AxAgent/commit/87b5dab))

### ⚡ 性能提升

- **全面优化：性能/体验/Agent能力/数据处理四维度18项改进** ([#3037499](https://github.com/polite0803/AxAgent/commit/3037499))

### 🔧 CI/CD

- **修复 CI workflow 三大缺陷** ([#9fa8179](https://github.com/polite0803/AxAgent/commit/9fa8179))
- **Fix rust-check workflow: setup rustup default stable** ([#35686e2](https://github.com/polite0803/AxAgent/commit/35686e2))
- **Fix GitHub Actions checkout to use master branch** ([#ee9f41e](https://github.com/polite0803/AxAgent/commit/ee9f41e))

---

## [v1.3.0] - 2026-05-01

### 📦 杂项

- **remove test-results directory from git** ([#2d8be3a](https://github.com/polite0803/AxAgent/commit/2d8be3a))
- **add artifacts directory to gitignore** ([#1c404ef](https://github.com/polite0803/AxAgent/commit/1c404ef))
- **remove .workbuddy directory from git** ([#23ddb36](https://github.com/polite0803/AxAgent/commit/23ddb36))
- **remove playwright-report directory from git** ([#e63b24c](https://github.com/polite0803/AxAgent/commit/e63b24c))
- **remove docs directory from git** ([#e38e3db](https://github.com/polite0803/AxAgent/commit/e38e3db))

### 🐛 Bug 修复

- **fix YAML boolean type error in nightly.yml** ([#e38e3db](https://github.com/polite0803/AxAgent/commit/e38e3db))

---

## [v1.2.8] - 2026-04-30

### 🚀 新功能

- **internationalize app title and fix workflow step missing field** ([#9bccc13](https://github.com/polite0803/AxAgent/commit/9bccc13))

### 🐛 Bug 修复

- **fix protoc installation in GitHub Actions workflows** ([#53b4f90](https://github.com/polite0803/AxAgent/commit/53b4f90))
- **replace arduino/setup-protoc with native package managers to avoid network timeout** ([#241f976](https://github.com/polite0803/AxAgent/commit/241f976))
- **add missing work_strategy field to UpdateConversationInput test** ([#df06fe1](https://github.com/polite0803/AxAgent/commit/df06fe1))

---

## [v1.2.7] - 2026-04-30

版本号升级。

---

## [v1.2.6] - 2026-04-30

### 🚀 新功能

- **opencode借鉴计划全面实施 + Part-based消息模型长期方案** ([#eea0571](https://github.com/polite0803/AxAgent/commit/eea0571))

### 🐛 Bug 修复

- **Fix build issues - wayland, libpipewire, TypeScript, Rust modules** ([#d5dd235](https://github.com/polite0803/AxAgent/commit/d5dd235))
- **Fix missing Rust modules, Message struct fields, scheduler templates i18n, migration merge** ([#b2454f2](https://github.com/polite0803/AxAgent/commit/b2454f2))
- **Fix TypeScript errors: ChatView, QualityScore, SchemaEditor, conversationStore, BreadcrumbBar** ([#0780a8e](https://github.com/polite0803/AxAgent/commit/0780a8e))
- **Fix Message struct: add missing parts and blocks fields** ([#b80e669](https://github.com/polite0803/AxAgent/commit/b80e669))
- **Add missing Rust modules: note_graph, louvain, deep_research, graph_insights, ingest_queue, purpose_manager, relevance** ([#73ba381](https://github.com/polite0803/AxAgent/commit/73ba381))
- **convert reqwest::Url to str before trim_end_matches** ([#33a7269](https://github.com/polite0803/AxAgent/commit/33a7269))
- **add missing semantic_cache to test AppState** ([#b29787f](https://github.com/polite0803/AxAgent/commit/b29787f))
- **add missing parts/blocks to Message, Arc::new for SessionManager, cleanup unused imports** ([#35221ef](https://github.com/polite0803/AxAgent/commit/35221ef))
- **remove simple_chat_completion reference and add missing parts field** ([#1407fdb](https://github.com/polite0803/AxAgent/commit/1407fdb))
- **make xcap Windows-only dependency to avoid libspa build errors on Linux CI** ([#5a1df13](https://github.com/polite0803/AxAgent/commit/5a1df13))
- **update xcap to 0.9.4 and remove stale cargo update steps** ([#af9cacc](https://github.com/polite0803/AxAgent/commit/af9cacc))
- **Remove invalid workspace.metadata rustfmt/clippy config** ([#e5858e7](https://github.com/polite0803/AxAgent/commit/e5858e7))

---

## [v1.2.5] - 2026-04-29

### 🚀 新功能

- **Wiki 功能前后端集成** ([#a5d2387](https://github.com/polite0803/AxAgent/commit/a5d2387))
- **批量更新 — Phase 3 实现、LLM Wiki 设计、主题引擎、Shell 钩子等功能** ([#8df4039](https://github.com/polite0803/AxAgent/commit/8df4039))
- **Phase 1 implementation - message gateway, prompt cache, test infrastructure** ([#f763459](https://github.com/polite0803/AxAgent/commit/f763459))

### 🐛 Bug 修复

- **improve wiki validation and LLM sync features** ([#6b1e728](https://github.com/polite0803/AxAgent/commit/6b1e728))
- **update xcap from 0.0.13 to 0.8.2 and fix API breaking changes** ([#bdefce5](https://github.com/polite0803/AxAgent/commit/bdefce5))
- **add loading/error to ConversationListState and remove unused import** ([#f674052](https://github.com/polite0803/AxAgent/commit/f674052))

### 📝 文档

- **expand Hermes gap analysis with detailed 5-phase implementation plan** ([#ee1ce96](https://github.com/polite0803/AxAgent/commit/ee1ce96))
- **add Hermes Agent gap analysis and catch-up plan** ([#e25125e](https://github.com/polite0803/AxAgent/commit/e25125e))

---

## [v1.2.3] - 2026-04-28

### 🐛 Bug 修复

- **Fix warnings, improve architecture and functionality** ([#b24adbb](https://github.com/polite0803/AxAgent/commit/b24adbb))

---

## [v1.2.2] - 2026-04-27

版本号升级。

---

## [v1.2.1] - 2026-04-27

版本号升级。

---

## [v1.2.0] - 2026-04-27

版本号升级。

---

## [v1.1.0] - 2026-04-27

### 📝 文档

- **Update README documentation for all languages to include new features** ([#8c887ec](https://github.com/polite0803/AxAgent/commit/8c887ec))
- **update README with new features** ([#4b9e1d8](https://github.com/polite0803/AxAgent/commit/4b9e1d8))

---

## [v1.0.2] - 2026-04-25

### 🐛 Bug 修复

- **resolve additional issues** ([#5e2b304](https://github.com/polite0803/AxAgent/commit/5e2b304))
- **resolve various errors and add new features** ([#1f879b1](https://github.com/polite0803/AxAgent/commit/1f879b1))
- **add missing migration files for 1.0.2** ([#0d202e6](https://github.com/polite0803/AxAgent/commit/0d202e6))

---

## [v1.0.1] - 2026-04-24

### 🐛 Bug 修复

- **add missing migration files** ([#eeebf02](https://github.com/polite0803/AxAgent/commit/eeebf02))

---

## [v1.0.0] - 2026-04-24

初始发布版本。

---

[v2.0.1]: https://github.com/polite0803/AxAgent/compare/v2.0.0...v2.0.1
[v2.0.0]: https://github.com/polite0803/AxAgent/compare/v1.6.0...v2.0.0
[v1.6.0]: https://github.com/polite0803/AxAgent/compare/v1.5.9...v1.6.0
[v1.5.9]: https://github.com/polite0803/AxAgent/compare/v1.5.8...v1.5.9
[v1.5.8]: https://github.com/polite0803/AxAgent/compare/v1.5.7...v1.5.8
[v1.5.7]: https://github.com/polite0803/AxAgent/compare/v1.5.6...v1.5.7
[v1.5.6]: https://github.com/polite0803/AxAgent/compare/v1.5.5...v1.5.6
[v1.5.5]: https://github.com/polite0803/AxAgent/compare/v1.5.3...v1.5.5
[v1.5.3]: https://github.com/polite0803/AxAgent/compare/v1.5.2...v1.5.3
[v1.5.2]: https://github.com/polite0803/AxAgent/compare/v1.5.1...v1.5.2
[v1.5.1]: https://github.com/polite0803/AxAgent/compare/v1.4.9...v1.5.1
[v1.4.9]: https://github.com/polite0803/AxAgent/compare/v1.4.8...v1.4.9
[v1.4.8]: https://github.com/polite0803/AxAgent/compare/v1.4.7...v1.4.8
[v1.4.7]: https://github.com/polite0803/AxAgent/compare/v1.4.6...v1.4.7
[v1.4.6]: https://github.com/polite0803/AxAgent/compare/v1.4.5...v1.4.6
[v1.4.5]: https://github.com/polite0803/AxAgent/compare/v1.4.4...v1.4.5
[v1.4.4]: https://github.com/polite0803/AxAgent/compare/v1.4.3...v1.4.4
[v1.4.3]: https://github.com/polite0803/AxAgent/compare/v1.4.2...v1.4.3
[v1.4.2]: https://github.com/polite0803/AxAgent/compare/v1.4.1...v1.4.2
[v1.4.1]: https://github.com/polite0803/AxAgent/compare/v1.4.0...v1.4.1
[v1.4.0]: https://github.com/polite0803/AxAgent/compare/v1.3.9...v1.4.0
[v1.3.9]: https://github.com/polite0803/AxAgent/compare/v1.3.8...v1.3.9
[v1.3.8]: https://github.com/polite0803/AxAgent/compare/v1.3.7...v1.3.8
[v1.3.7]: https://github.com/polite0803/AxAgent/compare/v1.3.6...v1.3.7
[v1.3.6]: https://github.com/polite0803/AxAgent/compare/v1.3.5...v1.3.6
[v1.3.5]: https://github.com/polite0803/AxAgent/compare/v1.3.4...v1.3.5
[v1.3.4]: https://github.com/polite0803/AxAgent/compare/v1.3.3...v1.3.4
[v1.3.3]: https://github.com/polite0803/AxAgent/compare/v1.3.2...v1.3.3
[v1.3.2]: https://github.com/polite0803/AxAgent/compare/v1.3.1...v1.3.2
[v1.3.1]: https://github.com/polite0803/AxAgent/compare/v1.3.0...v1.3.1
[v1.3.0]: https://github.com/polite0803/AxAgent/compare/v1.2.8...v1.3.0
[v1.2.8]: https://github.com/polite0803/AxAgent/compare/v1.2.7...v1.2.8
[v1.2.7]: https://github.com/polite0803/AxAgent/compare/v1.2.6...v1.2.7
[v1.2.6]: https://github.com/polite0803/AxAgent/compare/v1.2.5...v1.2.6
[v1.2.5]: https://github.com/polite0803/AxAgent/compare/v1.2.3...v1.2.5
[v1.2.3]: https://github.com/polite0803/AxAgent/compare/v1.2.2...v1.2.3
[v1.2.2]: https://github.com/polite0803/AxAgent/compare/v1.2.1...v1.2.2
[v1.2.1]: https://github.com/polite0803/AxAgent/compare/v1.2.0...v1.2.1
[v1.2.0]: https://github.com/polite0803/AxAgent/compare/v1.1.0...v1.2.0
[v1.1.0]: https://github.com/polite0803/AxAgent/compare/v1.0.2...v1.1.0
[v1.0.2]: https://github.com/polite0803/AxAgent/compare/v1.0.1...v1.0.2
[v1.0.1]: https://github.com/polite0803/AxAgent/compare/v1.0.0...v1.0.1
[v1.0.0]: https://github.com/polite0803/AxAgent/releases/tag/v1.0.0
