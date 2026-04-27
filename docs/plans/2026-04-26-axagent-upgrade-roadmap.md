# AxAgent 智能体能力升级路线图

> 规划日期: 2026-04-26
> 规划依据: 与 Claude/GPT-4/AutoGPT/SWE-agent 等先进智能体的差距分析
> 目标: 在 12 个月内将 AxAgent 打造为最具竞争力的本地桌面智能体

---

## 一、升级愿景

### 1.1 总体目标

将 AxAgent 从「功能完整的本地智能体」升级为「具有云端先进智能体能力的下一代桌面 AI 工作站」。

### 1.2 核心升级方向

| 方向 | 当前状态 | 目标状态 | 优先级 |
|------|---------|---------|--------|
| 多模态输出 | 基础 Mermaid/D2 渲染 | 实时可交互 UI Preview + 图像生成 | 🔴 P0 |
| 主动 Agent | 无 | 计算机控制 + 浏览器自动化 | 🔴 P0 |
| 深度推理 | 基础 thinking 块 | ReAct + 自我验证 + 智能恢复 | 🟠 P1 |
| 研究能力 | 基础搜索 | 多轮深度研究 + 自动报告生成 | 🟠 P1 |
| 个性化学习 | 基础记忆提取 | 用户画像 + 风格适应 | 🟡 P2 |
| 协作能力 | 单用户 | 团队协作 + 共享知识库 | 🟢 P3 |

---

## 二、升级阶段规划

### Phase 0: 基础增强（1-2 月）- 立即可执行

**目标**: 修复已知问题，增强现有功能稳定性

#### 0.1 安全修复（P0）

| 问题 | 位置 | 修复方案 | 工作量 |
|------|------|---------|--------|
| SQL 注入风险 | builtin_tools.rs#L1044 | 参数化查询 + 输入验证 | 0.5d |
| 路径遍历风险 | builtin_tools.rs#L496 | 添加路径验证逻辑 | 0.5d |
| Base64 无大小限制 | builtin_tools.rs#L1283 | 添加 100MB 上限 | 0.5d |

#### 0.2 现有 Artifact 系统增强

**现状**: 已有 `Artifact` 类型和 `artifactStore`，但功能有限

**增强内容**:
1. 支持更多格式（HTML/CSS/React 组件预览）
2. 添加实时预览面板（类似 Claude Artifacts）
3. 支持代码片段执行预览

**文件变更**:
```
src/types/artifact.ts          # 扩展 ArtifactFormat
src/stores/shared/artifactStore.ts  # 增强 CRUD 能力
src/components/chat/ArtifactPanel.tsx  # 实时预览组件
src/lib/artifactPreview.ts     # 新增：HTML/React 渲染引擎
```

**实现要点**:
- 使用 iframe 沙箱隔离执行用户生成的 HTML/JS
- 支持 React 组件的虚拟 DOM 渲染
- 添加代码编辑器集成（Monaco）

#### 0.3 代码解释器增强

**目标**: 为数据分析、自动化任务提供可靠的代码执行环境

**技术方案**:
- 前端: 使用 WebAssembly 沙箱（如 Pyodide for Python）
- 后端: Tauri 命令调用隔离的 Node.js 进程

**新增文件**:
```
src-tauri/crates/core/src/sandbox_runner.rs  # 沙箱进程管理
src/lib/codeExecutor.ts    # 前端执行器接口
src/components/chat/CodeExecutorPanel.tsx  # 执行结果展示
```

#### 0.4 自我验证机制（基础版）

**目标**: 执行结果自动验证，减少错误传播

**实现方案**:
1. 在 Workflow 节点执行后添加验证步骤
2. 支持断言式验证（assert conditions）
3. 失败时自动回退或重试

**文件变更**:
```
src-tauri/crates/runtime/src/workflow_engine.rs  # 添加验证节点
src/components/workflow/Nodes/ValidationNode.tsx  # 新增验证节点类型
```

---

### Phase 1: 多模态输出能力（2-3 月）

**目标**: 实现与 Claude Artifacts 类似的多模态输出体验

#### 1.1 图像生成集成

**集成方案**:
1. 集成 Flux API / Stable Diffusion API
2. 支持用户通过自然语言描述生成图片
3. 支持图片编辑（局部重绘、风格迁移）

**新增文件**:
```
src-tauri/crates/providers/image_gen.rs  # 图像生成 provider
src/components/chat/ImageGenerator.tsx   # 生成界面
src/stores/feature/imageGenStore.ts      # 生成状态管理
```

**API 集成**:
- 优先支持 Flux API（质量好、延迟低）
- 配置路径: `src-tauri/crates/providers/image_gen.rs`

#### 1.2 Artifact 实时预览增强

**目标**: 支持生成可交互的 HTML/CSS/React 组件并实时预览

**功能清单**:
| 功能 | 描述 | 技术方案 |
|------|------|---------|
| HTML 预览 | 渲染生成的 HTML 代码 | iframe 沙箱 |
| CSS 预览 | 实时样式编辑 | Style Tag 注入 |
| React 组件预览 | 渲染 TSX 代码 | Babel + React DOM |
| 图表生成 | 基于数据的可视化 | 集成 Chart.js / D3 |
| 交互式图表 | 可点击、hover 交互 | ECharts / AntV |

**新增文件**:
```
src/components/chat/ArtifactPreview/
├── index.tsx
├── HtmlPreview.tsx
├── ReactPreview.tsx
├── ChartPreview.tsx
└── InteractivePreview.tsx
src/lib/artifactRenderer.ts  # 渲染引擎
```

**UI 交互设计**:
```
┌─────────────────────────────────────────────────────┐
│  Artifact Panel                                      │
├────────────────────────┬────────────────────────────┤
│  代码编辑器             │  实时预览                   │
│  (Monaco)              │  (iframe 沙箱)              │
│                        │                            │
│  支持: HTML/CSS/TSX    │  支持: 交互式图表           │
│                        │  支持: 表单交互             │
├────────────────────────┴────────────────────────────┤
│  [复制代码] [全屏预览] [下载] [分享]                  │
└─────────────────────────────────────────────────────┘
```

#### 1.3 图表生成系统

**目标**: 根据自然语言描述或数据自动生成图表

**支持类型**:
- 折线图、柱状图、饼图
- 散点图、热力图
- 地图可视化
- 桑基图、关系图

**新增文件**:
```
src/lib/chartGenerator.ts   # 自然语言 → 图表配置
src/components/shared/ChartRenderer.tsx  # 图表渲染
src/hooks/useChartData.ts   # 数据获取 hook
```

---

### Phase 2: 主动 Agent 能力（3-4 月）

**目标**: 赋予 AxAgent 计算机控制能力，实现真正的自主操作

#### 2.1 屏幕感知与 UI 元素定位

**技术方案**:
1. 使用截图 API 获取屏幕内容
2. 使用视觉模型（GPT-4V/Claude Vision）分析 UI 元素
3. 通过坐标映射实现点击、输入操作

**新增 Rust 模块**:
```
src-tauri/crates/core/src/screen_capture.rs   # 屏幕截图
src-tauri/crates/core/src/ui_element_locator.rs  # UI 元素定位
src-tauri/crates/core/src/computer_control.rs    # 计算机控制主模块
```

**前端集成**:
```
src/components/chat/ComputerControlPanel.tsx  # 控制面板
src/hooks/useScreenCapture.ts   # 截图 hook
src/stores/feature/computerControlStore.ts
```

#### 2.2 浏览器自动化

**目标**: 集成 Playwright/Puppeteer，实现网页自动化

**集成方案**:
1. 通过 Tauri 命令调用本地 Playwright
2. 支持自然语言描述的操作（"点击登录按钮"）
3. 支持表单填写、数据抓取

**新增文件**:
```
src-tauri/crates/plugins/browser-automation/  # 浏览器自动化插件
├── src/lib.rs
├── src/playwright_client.rs
└── src/element_finder.rs
src/components/chat/BrowserAutomationPanel.tsx
```

**功能清单**:
| 功能 | 描述 |
|------|------|
| 页面导航 | 打开 URL、前进后退 |
| 元素操作 | 点击、输入、选择 |
| 数据提取 | 抓取页面内容 |
| 表单填写 | 自动填充表单 |
| 截图 | 页面或元素截图 |

#### 2.3 文件系统深度操作

**目标**: 突破沙箱限制，实现更强大的文件操作

**增强内容**:
1. 临时授权机制（类似 macOS 权限提示）
2. 批量文件操作（重命名、移动、删除）
3. 智能文件推荐（基于用户工作习惯）

**文件变更**:
```
src-tauri/crates/core/src/builtin_tools.rs  # 增强文件工具
src/components/shared/FilePermissionDialog.tsx  # 权限对话框
```

---

### Phase 3: 深度推理与规划（2-3 月）

**目标**: 实现类似 o1/Claude Thinking 的深度推理能力

#### 3.1 ReAct（Reasoning + Acting）模式

**目标**: 将推理和执行紧密结合，实现自主问题解决

**架构设计**:
```
用户输入 → 推理引擎 → 动作规划 → 工具执行 → 结果验证 → 循环
                ↑                                        │
                └────────────────────────────────────────┘
```

**新增文件**:
```
src-tauri/crates/agent/src/react_engine.rs   # ReAct 推理引擎
src-tauri/crates/agent/src/planner.rs        # 任务规划器
src-tauri/crates/agent/src/self_verifier.rs  # 自我验证器
```

**核心特性**:
- 思维链可视化（类似 Claude thinking 块）
- 中间结果记录与回溯
- 多路径探索与剪枝

#### 3.2 智能任务分解

**目标**: 自动将复杂任务拆分为可执行的子任务

**实现方案**:
1. 使用 LLM 分析任务意图
2. 生成任务分解树
3. 动态调整执行顺序

**新增文件**:
```
src-tauri/crates/agent/src/task_decomposer.rs  # 任务分解
src/components/chat/TaskDecompositionView.tsx   # 分解可视化
```

#### 3.3 智能错误恢复

**目标**: 基于历史失败模式学习的自动恢复

**功能清单**:
| 功能 | 描述 |
|------|------|
| 错误分类 | 识别错误类型（网络/权限/语法等） |
| 恢复策略 | 根据错误类型选择恢复方法 |
| 失败学习 | 记录失败模式避免重复 |
| 重试策略 | 智能指数退避重试 |

**新增文件**:
```
src-tauri/crates/agent/src/error_recovery.rs   # 错误恢复引擎
src-tauri/crates/agent/src/recovery_recipes.rs # 恢复配方库
```

---

### Phase 4: 研究型 Agent（2-3 月）

**目标**: 实现 Perplexity/GPT Research 级别的深度研究能力

#### 4.1 多轮深度研究模式

**目标**: 支持长时间运行的深入研究任务

**架构设计**:
```
研究任务 → 搜索计划 → 并行搜索 → 信息提取 → 综合分析 → 报告生成
              ↑                                           │
              └──────────────── 反馈循环 ─────────────────┘
```

**新增文件**:
```
src-tauri/crates/agent/src/research_agent.rs   # 研究智能体
src-tauri/crates/agent/src/search_orchestrator.rs  # 搜索编排
src-tauri/crates/agent/src/report_generator.rs # 报告生成器
```

**功能清单**:
| 功能 | 描述 |
|------|------|
| 研究规划 | 自动规划搜索策略 |
| 并行搜索 | 多源同时搜索 |
| 信息提取 | 智能提取关键信息 |
| 引用追踪 | 精确到句的引用 |
| 报告生成 | Markdown/HTML/PDF |

#### 4.2 信息可信度评估

**目标**: 自动评估信息来源的可靠性

**评估维度**:
- 来源权威性（官方媒体 vs 匿名博客）
- 信息一致性（多源交叉验证）
- 时效性（最新 vs 过时）
- 主观性（事实 vs 观点）

**新增文件**:
```
src-tauri/crates/agent/src/credibility评估.rs  # 可信度评估
```

#### 4.3 持续追踪能力

**目标**: 支持长时间运行的监控和研究

**功能**:
- 定时检查更新
- 变化通知
- 增量信息整合

---

### Phase 5: 个性化与持续学习（2-3 月）

**目标**: 建立深层次的用户偏好模型

#### 5.1 用户画像系统

**目标**: 全面理解用户的编码风格、沟通偏好、工作习惯

**画像维度**:
| 维度 | 内容 |
|------|------|
| 编码风格 | 命名习惯、代码模式、框架偏好 |
| 沟通偏好 | 详细程度、语气、格式 |
| 工作习惯 | 高效时段、常用工具、工作流程 |
| 领域知识 | 专业领域、兴趣方向 |

**新增文件**:
```
src-tauri/crates/trajectory/src/user_profile.rs   # 用户画像
src-tauri/crates/trajectory/src/preference_learner.rs  # 偏好学习
src/stores/feature/userProfileStore.ts
```

#### 5.2 风格迁移

**目标**: 自动适应用户的写作和编码风格

**实现方案**:
1. 分析用户历史输出
2. 提取风格特征
3. 应用到新生成内容

**新增文件**:
```
src-tauri/crates/agent/src/style_migrator.rs  # 风格迁移
src/lib/styleAnalyzer.ts   # 前端风格分析
```

#### 5.3 主动助手能力

**目标**: 预测用户需求并提前准备

**功能清单**:
| 功能 | 描述 |
|------|------|
| 上下文预测 | 基于当前上下文预测下一步 |
| 主动建议 | 在恰当时机提供建议 |
| 任务准备 | 提前准备可能需要的资源 |
| 例行提醒 | 定期提醒重要事项 |

---

### Phase 6: 开发者生态（2-3 月）

**目标**: 打造开放的开发者生态系统

#### 6.1 可视化执行追踪

**目标**: 类似 LangSmith 的调试和追踪工具

**功能清单**:
| 功能 | 描述 |
|------|------|
| 调用链追踪 | 完整的工具调用链路 |
| 性能分析 | 各步骤耗时分析 |
| 成本追踪 | Token 和 API 费用 |
| 调试日志 | 详细的执行日志 |

**新增文件**:
```
src-tauri/crates/telemetry/src/tracer.rs   # 追踪器
src/pages/DevTools/TraceExplorer.tsx   # 追踪浏览器
src/components/devtools/
```

#### 6.2 评估框架

**目标**: 内置基准测试和性能评估

**功能**:
- 标准 benchmark 数据集
- 自动化评估流程
- 性能报告生成

**新增文件**:
```
src-tauri/crates/agent/src/evaluator.rs   # 评估器
src/benchmark/

---

### Phase 7: 智能化提升（长期）

**目标**: 探索前沿 AI 技术

#### 7.1 强化学习优化

**目标**: 基于 RL 的技能优化

**探索方向**:
- 工具选择策略优化
- 任务分解策略学习
- 错误恢复策略进化

#### 7.2 轻量级微调

**目标**: 支持 LoRA 等轻量级微调

**功能**:
- 本地微调数据管理
- LoRA 训练流程
- 模型切换管理

#### 7.3 智能工具推荐

**目标**: 基于上下文推荐最佳工具组合

**实现方案**:
- 分析当前任务上下文
- 推荐相关工具
- 学习用户工具使用习惯

---

## 三、技术架构调整

### 3.1 模块化解耦

**目标**: 将核心 Agent 能力与 Tauri 解耦

**方案**:
```
┌─────────────────────────────────────────────────────────┐
│                    AxAgent Core                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ Agent Core  │  │  Workflow   │  │  Memory     │     │
│  │             │  │  Engine     │  │  System     │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
├─────────────────────────────────────────────────────────┤
│                    Interface Layer                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ Tauri IPC   │  │   REST API  │  │   gRPC      │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
├─────────────────────────────────────────────────────────┤
│                    Platform Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   Desktop   │  │    Web      │  │   Mobile    │     │
│  │   (Tauri)   │  │  (Browser)  │  │   (React)   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
```

### 3.2 插件沙箱隔离

**目标**: 更安全的插件隔离机制

**技术方案**:
- WebAssembly 沙箱
- 进程级隔离
- 资源限制（CPU/内存/网络）

### 3.3 性能优化

**目标**: 大量工具注册时的冷启动优化

**优化措施**:
- 懒加载工具注册
- 工具缓存
- 并行初始化

---

## 四、实施时间线

### 4.1 总览

```
Q2 2026                    Q3 2026                    Q4 2026                    Q1 2027
├─────────────────────────┼─────────────────────────┼─────────────────────────┤
Phase 0: 基础增强
Phase 1: 多模态输出 ◄─────┼─────────────────────────┤
Phase 2: 主动 Agent ────────────────────────────────┤
Phase 3: 深度推理 ───────────────────────────────┤
Phase 4: 研究能力 ──────────────────────────────┤
Phase 5: 个性化学习 ───────────────────────────┤
Phase 6: 协作能力 ──────────────────────────┤
Phase 7: 开发者生态 ───────────────────────┤
Phase 8: 智能化提升 (持续)
```

### 4.2 详细里程碑

| 阶段 | 开始 | 结束 | 关键交付 |
|------|------|------|---------|
| Phase 0 | 2026-04-26 | 2026-05-15 | 安全修复、Artifact 增强、代码解释器 |
| Phase 1 | 2026-05-15 | 2026-07-15 | 图像生成、实时预览、图表生成 |
| Phase 2 | 2026-07-01 | 2026-10-01 | 屏幕感知、浏览器自动化、文件系统增强 |
| Phase 3 | 2026-08-01 | 2026-10-15 | ReAct 引擎、智能任务分解、错误恢复 |
| Phase 4 | 2026-09-01 | 2026-11-15 | 研究 Agent、可信度评估、报告生成 |
| Phase 5 | 2026-10-15 | 2027-01-01 | 用户画像、风格迁移、主动助手 |
| Phase 6 | 2027-01-01 | 2027-04-01 | 团队协作、权限管理、实时协作 |
| Phase 7 | 2027-04-01 | 2027-07-01 | 执行追踪、评估框架 |

---

## 五、资源估算

### 5.1 开发工作量

| 阶段 | 前端 | 后端(Rust) | 设计 | 测试 | 总人月 |
|------|------|-----------|------|------|--------|
| Phase 0 | 1 | 1 | 0.5 | 0.5 | 3 |
| Phase 1 | 4 | 2 | 1 | 1 | 8 |
| Phase 2 | 3 | 4 | 1 | 1 | 9 |
| Phase 3 | 2 | 3 | 0.5 | 0.5 | 6 |
| Phase 4 | 2 | 3 | 0.5 | 0.5 | 6 |
| Phase 5 | 2 | 2 | 0.5 | 0.5 | 5 |
| Phase 6 | 4 | 3 | 1 | 1 | 9 |
| Phase 7 | 2 | 2 | 0.5 | 0.5 | 5 |
| **总计** | **20** | **20** | **5.5** | **5.5** | **51** |

### 5.2 外部依赖

| 依赖 | 用途 | 成本 |
|------|------|------|
| Flux API | 图像生成 | 按量计费 |
| Tavily API | 深度搜索 | 按量计费 |
| Playwright | 浏览器自动化 | 开源免费 |
| Pyodide | Python 沙箱 | 开源免费 |

---

## 六、风险与缓解

### 6.1 技术风险

| 风险 | 影响 | 缓解方案 |
|------|------|---------|
| 屏幕感知精度不足 | 高 | 混合方案：视觉模型 + 辅助定位 |
| 沙箱安全漏洞 | 高 | 定期安全审计、权限最小化 |
| 性能瓶颈 | 中 | 性能测试、渐进优化 |
| LLM 推理延迟 | 中 | 缓存、异步处理、进度反馈 |

### 6.2 项目风险

| 风险 | 影响 | 缓解方案 |
|------|------|---------|
| 开发资源不足 | 高 | 优先级排序、阶段交付 |
| 依赖外部 API | 中 | 多 provider 备份、本地模型选项 |
| 用户接受度 | 中 | 渐进发布、用户反馈 |

---

## 七、立即行动项

### 7.1 本周可执行任务

1. **安全修复**（P0）
   - [ ] 修复 SQL 注入风险
   - [ ] 修复路径遍历风险
   - [ ] 添加 Base64 大小限制

2. **Artifact 增强**（P0）
   - [ ] 扩展 ArtifactFormat 支持 HTML/CSS/React
   - [ ] 开发 ArtifactPreview 组件
   - [ ] 集成 Monaco 编辑器

3. **代码解释器**（P1）
   - [ ] 设计沙箱架构
   - [ ] 实现 Node.js 隔离执行
   - [ ] 开发执行结果展示面板

### 7.2 下周计划

1. 完成 Phase 0 的剩余任务
2. 开始 Phase 1 的技术预研
3. 制定详细的技术设计方案

---

## 八、附录

### 8.1 相关文档

- [项目审计报告](./2026-04-25-project-audit-and-fix-plan.md)
- [工作流编辑器设计](./2026-04-24-workflow-template-editor-design.md)

### 8.2 参考项目

- [Claude Artifacts](https://docs.anthropic.com/en/docs/build-claude-code/artifacts)
- [SWE-agent](https://github.com/princeton-nlp/SWE-agent)
- [LangChain/LangGraph](https://github.com/langchain-ai/langgraph)
- [AutoGPT](https://github.com/Significant-Gravitas/AutoGPT)

### 8.3 术语表

| 术语 | 定义 |
|------|------|
| ReAct | Reasoning + Acting，推理与执行结合的智能体框架 |
| Artifact | AxAgent 中生成的可保存内容块 |
| 沙箱 | 隔离的执行环境，防止恶意代码影响系统 |