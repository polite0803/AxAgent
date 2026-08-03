# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.8.7] - 2026-07-24

### ✨ New Features
- 翻译 10 个目标语言未翻译及中文泄漏的 key
- 文档导出工具能力扩展（数学公式/图表/Mermaid 流程图）
- 成本展示由 USD 改为 CNY，支持自定义汇率
- 大规模功能增强——Gateway OpenAI API 扩展、新 LLM 提供商、插件沙箱、Event Bus、后台任务
- P3 改进——路径验证统一与无障碍增强


### 🐛 Bug Fixes
- 移除破坏 antd v6 Tabs 隐藏机制的内联 CSS hack
- workflow_executions.total_time_ms 类型对齐 BIGINT (i32→i64)
- 补齐 settings.markerPrefixPlaceholder / markerPrefixDesc 到全部 10 个目标语言
- mermaid PDF test CI failure + box-drawing CID font fallback
- 修复 CI 两项失败——CJK PDF 字体乱码 + i18n 硬编码
- pdf_math_test 添加 CJK 字体 guard，消除并行测试的 OnceLock 干扰
- 去掉 pdf_math_test 的 #[cfg(windows)]，仅保留运行时 guard 避免死代码警告
- pdf_template_test 含 CJK 文本的测试添加 msyh.ttc 运行时 guard
- storage_migration messages::ActiveModel 缺 quoted_message_id 字段
- map(..).flatten() → and_then(..) on Option
- 修复 cargo audit 漏洞并补全 quoted_message_id 字段
- ignore quick-xml 0.30.0 RUSTSEC-2026-0194/0195 (xcb transitive, 桌面场景不可利用)
- 为 react_engine_extended_tests 添加 TestLlmProvider mock，注入 with_reasoning_provider()


### 📦 Miscellaneous
- 新增 db 类型一致性审计脚本
- bump version to v2.8.7


### 🔧 CI / Build
- 合并 clippy 两阶段——axagent-disk-cache / axagent-rt-theme 在 rust 1.97 下已不再触发 ICE

