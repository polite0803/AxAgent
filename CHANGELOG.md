# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.4.1] - 2026-05-29

### ⚡ Performance
- ⚡ Rhai tool() 共享 Runtime: Arc 复用, O(1) 创建


### ✨ New Features
- 🚀 CronJobStore 增加 DB 持久化能力
- 🚀 Cron 调度打通工作流引擎
- 🚀 P0-2: 条件分支 LLM 动态路由
- 🚀 P1-3: 工作流注册为工具 + P1-5: 干跑模式 + P1-6: 并行聚合策略
- 🚀 P2-9: 执行断点调试
- 🚀 P1-4: 执行历史可视化回放面板 + P2-8: 节点复制粘贴（已有）
- 🚀 Rhai 脚本引擎 —— Code 节点注册为动态工具
- 🚀 Rhai 脚本支持 tool() 函数调用已注册工具
- 🚀 工作流引擎完善：dry_run + on_fail + 熔断/超时UI + 代码编辑器 + 断点UI
- 🚀 统一规划架构：HierarchicalPlanner 为唯一规划引擎
- 🚀 AgentExecutor Plan 模式：新增审批回调 + 步骤事件推送 + Android 构建修复
- 🚀 工作流引擎 Condition 节点边路由支持
- 🚀 实现 Skill 工具型 + 流程型双支持


### 🎨 Styling
- 🎨 全仓格式化
- 🎨 cargo fmt
- 🎨 cargo fmt
- 🎨 rustfmt 格式化修复
- 🎨 修复 DropdownMenu 弹出菜单位置：左对齐 + 减小间距
- 🎨 移除 Agent 模式切换菜单中的 Beta 标签


### 🐛 Bug Fixes
- 🐛 修复测试中 jsdom 清理阶段的 window/document 未定义错误
- 🐛 修复测试 jsdom 销毁后 window/document undefined——改用 getter 兜底
- 🐛 agentStore 测试移除 renderHook，改用 Zustand getState() 避免 react-dom 引入
- 🐛 知识源功能缺陷修正
- 🐛 移除 mcp_stdio 测试重复的 #[ignore] 属性
- 🐛 修复 chatRightPanel.title 显示为资源键——添加缺失的 title 翻译
- 🐛 修复标题hover闪烁 + 右侧栏tab垂直布局
- 🐛 右侧栏每个 tab 增加错误边界，单 tab 崩溃不影响整体
- 🐛 移除 main.tsx 未使用的 logIpcError 导入
- 🐛 修复会话 tab 与会话列表不一致问题
- 🐛 修复 TS6133 未使用变量 + 会话 tab 同步修复
- 🐛 会话页顶部工具栏整行禁止换行
- 🐛 恢复 TeammatePanel useCallback 导入
- 🐛 Rhai 工具执行改为 spawn_blocking，避免嵌套 runtime 死锁
- 🐛 Plan 模式收尾：Phase依赖精确解析 + Planner运行时管理 + PlanStep增强
- 🐛 Plan + Skill 系统全面修复：DAG映射、类型一致、i18n、代码质量
- 🐛 修复 clippy::collapsible_if 警告 + SQLite ALTER TABLE 语法错误
- 🐛 修复 compile_plan_to_dag 测试：适配新增的 trigger/end 节点
- 🐛 修复 agentStore 测试：mock 中缺少 isTauri 导出
- 🐛 问答模式 6 项缺陷修复：工具错误、panic保护、并发保护、完成事件
- 🐛 Agent 三条链路 5 项缺陷修复：结果键、断路器、审批、Profile注入
- 🐛 修复 Skill 工具在问答模式中不可用：自动注入本地工具列表
- 🐛 修复内置工具名称不匹配：Read→FileRead, Write→FileWrite 等
- 🐛 修复 database.rs 缺少 PermissionsExt 导入
- 🐛 修复 agentStore 测试 teardown + P1/P2 版本策略
- 🐛 修复 agentStore 测试 setImmediate → ReferenceError: window is not defined


### 📝 Documentation
- 📝 i18n allowlist: ChatView/StructuredThinking 内容分析关键词


### 📦 Miscellaneous
- bump version to v2.4.0
- 📦 版本更新策略：CHANGELOG 自动生成 + Release Notes
- bump version to v2.4.1


### 🔧 CI / Build
- 🔧 修复 E2E CI 卡死：npx playwright install 添加 --yes + 浏览器缓存
- 🔧 CI 增加磁盘空间释放步骤，修复 "No space left on device"

