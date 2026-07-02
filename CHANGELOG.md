# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.8.0] - 2026-07-01

### 🚀 Features
- Dashboard / 看板页面：总对话数、消息数、Token 消耗、Gateway 状态、Agent 活动统计
- 侧栏导航新增 chat（对话）和 dashboard（看板）入口
- 默认首页从 /knowledge 改为 /dashboard
- 多 Agent 协同：Debate 闭环（LCS 收敛检测）、Swarm↔Workflow 集成、Aggregator LLM 摘要补完
- CRDT 完善：向量时钟（VectorClock）、因果依赖检查、收敛检测
- EventBus 跨 Engine 联通：基于 tokio::broadcast 的 GlobalEventBus
- ExecutionState.variables 类型安全：Schema 校验 + set_variable_safe()
- Orchestrator LLM 驱动分解回调、master_key 注入
- Parallel 分支同步屏障修复（JoinSet 并发执行 + All/Any/Race/Majority 聚合策略）

### 🛠️ Improvements
- removeCrossorigin 中间件移至 build.rollupOptions.plugins (Vite 8 兼容)
- 导航项重新分组（Tools 分区）

### 📦 Dependencies
- @tauri-apps/plugin-http 2.5.3 → 2.5.4
- @tauri-apps/api 2.9.1 → 2.9.2

## [v2.7.0] - 2026-06-23

### 🐛 Bug Fixes
- gateway→dao 生产依赖修复：gateway→kit（dao re-export shim）
- marketplace_handlers.rs import 路径同步更新

### 🔨 Refactoring
- Volcengine DeepSeek 适配：reasoning_content 字段标准化
- 全路径引用 vs use 导入检查方法改进

### 🚀 Features
- LLM 工具链集成：工具调用链追踪增强
- 容器节点端口折叠（4+ 入边/出边自动折叠）
- SubWorkflow 子图边渲染（展开态 edge 注入）

### 🐛 Bug Fixes
- WorkEngine::new 必传 ProviderRegistry 编译期强制
- cargo fmt
- 🐛 允许 crypto.rs clippy::result_large_err（AxAgentError 来自 harness）
- 🐛 修复 axagent-kit 缺少 libc 依赖


### 📦 Miscellaneous
- 🔖 升级版本号至 2.6.0
- 合并上游更新


### 🔨 Refactoring
- core 200→0 逻辑文件, 拆出 9 个 crate, harness 架构合规

