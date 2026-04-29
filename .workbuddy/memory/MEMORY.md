# 长期记忆

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
