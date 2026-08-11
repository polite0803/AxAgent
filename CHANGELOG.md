# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.9.3] - 2026-08-11

### ✨ New Features
- Wiki 与知识库建立显式关联（v118 迁移），笔记 wikilink 同步下沉至 dao 层


### 🎨 Styling
- 全量 rustfmt 格式化并通过 pre-commit 检查


### 🐛 Bug Fixes
- 修复 PostgreSQL 后端硬编码和 llama.cpp 服务地址不同步
- 修正错误链转换与 transport 注册借用，细化网关日志并补齐 SAFETY 注释
- 修复 clippy lint 告警和 schema 合规性测试


### 📦 Miscellaneous
- bump version to 2.9.3
- 升级版本号至 2.9.3
- 升级 sea-orm 至 2.0.1 stable 并清理 oxlint 告警、删除 Trae 临时文件


### 🔨 Refactoring
- 代码质量与 CI 改进（oxlint 替代 ESLint、锁毒化防护、unwrap 消除、错误分类优化）
- 消除 unwrap 并保留错误根因链、CI cargo-audit 加固、依赖覆盖修复

