# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.4.0] - 2026-05-27

### ✨ New Features
- 🚀 工作流 Agent 节点新增单工具参数编辑功能
- 🚀 Agent 节点新增 exposed_tools 字段，区分固定工具与暴露工具
- 🚀 n8n 导入新增配置转换映射逻辑


### 🎨 Styling
- 🎨 修复对话页标题区域窄屏换行问题
- 🎨 cargo fmt 全仓格式化
- 🎨 cargo fmt 全仓格式化
- 🎨 cargo fmt
- 🎨 dprint 格式化
- 🎨 dprint 格式化


### 🐛 Bug Fixes
- 🐛 fix: recentErrors 填充 — _recentErrors 队列记录失败调用
- 🐛 fix: 模型选择器标签换行 → 单行截断
- 🐛 fix: 设置页服务商管理模型列表多行换行 → 单行截断
- 🐛 fix: CSS 全局规则 — overflow-hidden 容器内文本单行截断
- 🐛 fix: 截断文本 hover 展开显示完整内容
- 🐛 fix: 模型分组头部容器添加 overflow-hidden
- 🐛 fix: hover 展开保持单行 — position:absolute 浮层显示
- 🐛 修复模型编辑对话框中类型/能力标签 hover 跳动
- 🐛 全局 CSS 选择器 .overflow-hidden > span 太宽泛，改用专用 .ax-truncate
- 🐛 修复批量能力编辑标签容器遗漏的 overflow-hidden → flex-wrap
- 🐛 修复 AgentNodeConfig exposed_tools 遗漏的三处构造点
- 🐛 修复 QuickBarPage runCode 缺少返回值
- 🐛 修复主 crate 二处编译错误
- 🐛 移除 QuickBarPage 中过时的中文注释（i18n 检查）
- 🐛 修复三处 clippy 警告
- 🐛 修复 session_manager 二处 collapsible_if clippy 警告
- 🐛 修复 shared_memory 五处 unnecessary_lazy_evaluations
- 🐛 webhook: map_or(false) → is_some_and / is_ok_and


### 📦 Miscellaneous
- bump version to v2.4.0

