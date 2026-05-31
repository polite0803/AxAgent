# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.4.5] - 2026-05-31

### ✨ New Features
- 🚀 工作流编辑器一键布局支持子工作流递归
- 🚀 重写工作流 DebugPanel：从纯运行时改为静态调试面板


### 🎨 Styling
- 🎨 DropdownMenu 菜单零间距紧贴触发按钮


### 🐛 Bug Fixes
- 🐛 setImmediate→setTimeout 后仍需 try-catch 防御 jsdom 销毁回调
- 🐛 修复 Android 构建：String + &String/+&str 类型错误
- 🐛 修复 citation + inspector 面板 React error #185 (无限重渲染)
- 🐛 修复右侧面板显示逻辑：所有面板默认可见
- 🐛 修复右侧面板 React #185：精简 useMemo 依赖 + activeTab 验证
- 🐛 DebugPanel 新增子工作流递归分析
- 🐛 修复状态栏节点/连线计数不显示：zh-CN 缺少 {{count}}
- 🐛 修复状态栏节点/连线计数：zh-CN i18n 缺少 {{count}} 占位符
- 🐛 i18n allowlist 更新：AIPanel logIpcError + WorkflowEditor fallback + 测试断言
- 🐛 修复 base_title().map() 编译错误：&str 上无 map 方法
- 🐛 i18n allowlist: WorkflowEditor fallback + 测试 L191
- 🐛 修复编译错误：ProgressCallback 导出 + workflow_ai 未使用变量
- 🐛 修复 clippy 11 项警告：dead_code/冗余闭包/auto-deref/filter_map
- 🐛 修复 AIPanel 测试：scrollIntoView 在 jsdom 不可用
- 🐛 i18n allowlist: AIPanel 行号漂移更新 127→129, 150→151
- 🐛 修复 workflowComponents 测试：适配新 AIPanel 结构
- 🐛 修复版本号：Cargo workspace + package 统一为 2.4.5 + bump 脚本去 ^ 锚点
- Tauri IPC 参数不匹配 — 前端 invoke 调用添加 params/request 包装
- 🐛 右侧面板 tab 从垂直堆叠改为水平折行 + 纯图标紧凑模式
- 🐛 DebugPanel 添加 workflow.debug 中文翻译 (52 key)
- 🐛 修复 analyzeNodes isDeadEnd 逻辑反了：trigger 被误标为死胡同


### 📦 Miscellaneous
- bump version to v2.4.5
- i18n allowlist: AIPanel L173
- i18n allowlist: workflowComponents test 新增 48 行
- i18n allowlist: CodePropertyPanel + browserMock 行号更新


### 🔧 CI / Build
- 🔧 CI: CARGO_PROFILE_DEV_DEBUG=0 缩减 target 体积防磁盘满

