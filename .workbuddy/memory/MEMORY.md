# 长期记忆

## 2026-04-30 Plan 模式架构决策
- Plan 定位为 **agent 模式的工作策略**（`work_strategy: "direct" | "plan"`），不是独立 mode
- 原因：Plan 的二阶段特性（生成→审批→执行）不适合 conversation-level mode 切换模式
- Plan 状态独立持久化（plans 表），切 chat 不丢失，切回 agent 可恢复
- 执行阶段复用 agent 工具循环（SessionManager.run_turn_with_tools）
- PlanCard 渲染条件：仅 `conversation.mode === "agent"` + `activePlans` 非空（不检查 work_strategy）
- handlePlanGenerated 需将旧 plan 归档到 history 再替换
- rejectPlan/cancelPlan 需移入 history(status=cancelled) 而非直接丢弃
- loadActivePlan: 启动/会话切换时调用 plan_list(includeCompleted=false) 恢复活跃 plan
- PlanHistoryPanel: 右侧 Drawer 面板，agent 模式下的 📋 入口，支持 Resume
- handleWorkStrategyChange 需 isSwitchingStrategyRef 互斥防抖

## 2026-04-25 添加：复合技能分解全流程缺陷审查
对"复合技能→原子技能→工作流→工具"全流程进行了审查，发现 14 个缺陷（8 高/4 中/2 低）。关键缺陷：
- WorkEngine 与 WorkflowEngine 未桥接（高）
- NodeExecutor 未完成实际执行（高）
- 工具依赖硬编码为 Satisfied（高）
- confirm_decomposition 跳过重复检查（高）
- generate_missing_tool 跳过 LLM 调用（中）
- 前端参数映射错误（中）

建议优先修复执行链路打通（#14, #6, #5, #7）和数据正确性（#4, #10, #11）。

## 2026-04-30 添加：opencode 借鉴计划实施审查
对 docs/plans/2026-04-29-opencode-borrowing-plan.md 进行了系统审查，覆盖 9 个方向/3 个优先级。
核心发现：P0 中 SubAgentCard 和前后端链路已实现，但面包屑导航和面板废弃未开始；
P1 中权限系统、上下文压缩、插件体系和指令加载均已超预期实现，Tree-sitter 未实现；
P2 中 Agent 自然语言生成和 Part-based 消息模型均未实现。

## 2026-04-30 添加：用户 UI 偏好
- 以深色主题为主要使用场景，默认 dark-elegance 预设
- 标题栏、导航栏需保持紧凑不占空间（28px 标题栏、44px 侧栏 hover 展开至 180px）
- 适量动画提升使用舒适性（页面切换、消息出现、hover 反馈）
- 需要多主题可选但保持布局结构一致性

## 2026-04-30 添加：UI 主题系统架构
- 布局层（共享）：紧凑标题栏 28px、可折叠侧栏 44px↔180px、动画系统、文本选择
- 主题层（可切换）：4 套预设 — dark-elegance / dark-neon / light-professional / light-minimal
- 主题预设定义在 src/theme/shadcnTheme.ts 的 PRESETS 表中
- 用户可在 DisplaySettings 中的主题预设选择器切换

## 2026-04-30 添加：布局关键修复
- App.tsx 中 `<Sider>` 不能直接用 `<div>` 替换（antd Layout 依赖 Sider 检测），必须加 `hasSider` prop
- `ax-page-transition` 动画需 `key={location.key}` 才能随路由切换重新触发

## 2026-04-30 添加：QuickBar 图形化设计
- 双模式: 卡片网格为主（16 个可点击命令卡片），命令输入为备选（按 / 进入）
- 16 个命令覆盖: chat/agent/new/continue/search/wiki/memory/files/url/summarize/translate/calc/code/model/settings/gateway
- 5 大分类: AI对话、知识管理、网页工具、实用工具、系统快捷
- 存储: localStorage recent items 扩至 5 项
