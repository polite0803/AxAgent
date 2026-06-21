# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.6.0] - 2026-06-04

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

