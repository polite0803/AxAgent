# Changelog

All notable changes to AxAgent will be documented in this file.
## [v2.9.5] - 2026-08-16

### ⚡ Performance
- 调优图视图力导向物理参数与节点初始分布，改善布局收敛与观感


### ✨ New Features
- 专家与角色能力护照索引 + 决策标签持久化，清理会话-工作流遗留逻辑
- 角色命中时 RAR 动态检索匹配专家，运行时补全角色+专家组合
- 能力护照新增 L2 sub_category 子分类，发现面板细分展示 agent 角色/专家
- 增强搜索、图形视图功能及多语言支持
- 大规模重构能力路由、认知编排与工作流进化引擎


### 🐛 Bug Fixes
- 测试建表语句补齐 messages.decision 列，修复 storage_migration 测试失败
- 图视图新增物理预热迭代与斐波那契螺旋初始分布，解决节点堆叠问题
- conversation 消息计数 SQL 按 Postgres/SQLite 后端区分占位符与 GREATEST 函数
- 认知编排器主 DAG 加载进 WorkEngine 内存，修复路由监测为空与 LLM 无输出
- 优化 Wiki 图形视图物理引擎
- 修复 wiki.rs 中裸 map_err 错误处理
- 修复 E2E 测试失败问题与错误处理规范
- 更新 i18n 白名单，添加 browserMock.ts 新违规行 4857,4865,4873
- 简化复杂类型，添加 StatsMap 类型别名解决 clippy type_complexity 警告
- 更新 schema_compliance 测试 SCHEMA，添加 trajectory_trajectories 新列
- 修复 fleet mock 处理器 snake_case 参数访问
- update turn_summary snapshot for TokenUsage camelCase serialization
- add mock implementations for 6 unhandled BrowserMock commands


### 📦 Miscellaneous
- 更新 i18n 豁免基线，合并 browserMock.ts 新增 mock 能力数据行号
- bump version to 2.9.5


### 🧪 Testing
- 修复 E2E 测试模态框遮挡和数据格式问题

