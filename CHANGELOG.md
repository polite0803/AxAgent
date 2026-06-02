# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.4.6] - 2026-05-31

### 🐛 Bug Fixes
- 🐛 修复 DebugPanel 节点类型误判 + 终端节点误标死胡同
- 🐛 修复 DebugPanel "开始调试"无反应：添加错误捕获和错误显示
- 🐛 修复 workflowEditorStore 测试：jsonData → json_data
- 🐛 修复 collapsible_if clippy 警告 (engine.rs L264,L283)
- 🐛 修复 debug_run_workflow: template_id → templateId
- 🐛 WorkflowNode 反序列化排查：dispatcher trace + roundtrip 测试
- 🐛 修复 Vec<WorkflowNode> 整体反序列化 Tool→Agent 变体混淆
- 🐛 Tool→Agent 反序列化修复 + dispatcher trace + roundtrip 测试
- 🐛 禁用 WorkflowMigrator：不再将 Tool/Code 转换为 Agent
- 🐛 彻底禁用 WorkflowMigrator：移除 Tauri 命令注册 + 清理遗留迁移代码
- 🐛 修复编译/测试错误：Mutex→RwLock + collapsible_if + 未使用 import
- 🐛 修复 plugins/lib.rs collapsible_if x3
- 🐛 跳过 CI 超时的 plugin 测试
- 🐛 plugins/lib.rs: map_or → is_none_or
- 🐛 修复 registry.rs UTF-8 字节切片 panic


### 📦 Miscellaneous
- bump version to v2.4.6

