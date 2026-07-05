# AxAgent v2.8.0 缺陷完善计划

> 基于 2026-07-05 全面审计中证实的缺陷，制定分阶段可执行计划
> 当前版本：v2.8.0 | 单人开发 | 32 Rust Crates | 30 天/Phase

---

## 一、优先级框架

**排序原则**：影响面 × 修复成本 × 安全风险，单人开发优先做"投入产出比高"的事。

| 优先级 | 定义                            | 处理节奏           |
| ------ | ------------------------------- | ------------------ |
| **P0** | 影响日常开发/发布流程的阻塞问题 | 本周内             |
| **P1** | 架构性短板但短期内可接受的      | 本迭代 (1个月内)   |
| **P2** | 功能缺失、体验优化              | 下个迭代 (2-3个月) |
| **P3** | 锦上添花，对齐业界标准          | 远期规划 (3个月+)  |

---

## 二、Phase 0 ✅ 已完成（2026-07-05）

### 2.1 ✅ 建立测试基础设施

报告中"26 个测试文件"的结论虽然低估了实际测试量（实际 28 个 tests/*.rs 文件 + 301 处 mod tests 内联模块），但**缺乏覆盖率度量**是事实。

**完成情况：**

| #  | 任务 | 涉及文件 | 状态 | 说明 |
| -- | ---- | -------- | ---- | ---- |
| T1 | 引入 `cargo-llvm-cov`，在 CI 中生成覆盖率报告 | `.github/workflows/rust-ci.yml` | ✅ | 新增 `coverage` job，使用 `taiki-e/install-action@cargo-llvm-cov`，`continue-on-error` 避免阻塞 |
| T2 | 用 `cargo-tarpaulin` 生成首份全覆盖率报告 | 脚本层 | ⏳ | 本地环境缺少 LLVM 工具链，可在 CI 中首次运行时自动生成 |
| T3 | 为 agent crate 核心模块补单元测试，共 **29 个新增测试** | `crates/agent/tests/` + `react_engine.rs` 内联 | ✅ | `react_engine_extended_tests.rs`(4)、`coordinator_lifecycle_tests.rs`(4)、`react_engine.rs` 内联(10)、`registry_lifecycle_tests.rs`(11)，全部通过 |
| T4 | 为 tools crate 的注册/解析/执行路径补单元测试 | `crates/tools/tests/registry_lifecycle_tests.rs` | ✅ | 11 个测试覆盖 register/find/disable/enable/unregister/by_category/empty 全部路径 |
| T5 | CI 中添加 `cargo-audit` + `npm audit` 依赖漏洞扫描 | `.github/workflows/pr-ci.yml` | ✅ | cargo-audit 已存在；新增 `npm audit --audit-level=high`（non-blocking） |
| T6 | 建立 `CHECKS.md` 发版前检查清单 | 根目录 | ✅ | 包含 Rust 后端 5 步 + 前端 5 步 + 安全审计 2 步 + 发布前确认 |
| T7 | 统一 README 中 crates 数量描述 | `README.md` | ✅ | `18 个` → `30 个`（含括号说明 workspace 共 32 个成员） |

---

## 三、Phase 1 — 架构深耕（1-2 个月，P1）

### 3.1 🔧 替换 DefaultReasoningProvider（P1-高）

**问题**：`react_engine.rs` 的 DefaultReasoningProvider 用正则/词数规则判断任务复杂度，而非调用 LLM。

**方案**：

| #   | 任务                                                                                                             | 涉及文件                           | 预估工时 |
| --- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------- | -------- |
| T8  | 在 `providers` crate 中新增 `ReasoningModelProvider` trait，封装 LLM 调用的推理接口                              | `crates/providers/src/`            | 1天      |
| T9  | 实现 `LLMDrivenReasoningProvider`，将复杂度判断改为 LLM 调用（prompt 判断→返回 complexity + reasoning_approach） | `crates/agent/src/react_engine.rs` | 2天      |
| T10 | 保留规则推理作为 fallback（离线/低延迟场景），加 configuration flag 切换                                         | `crates/agent/src/react_engine.rs` | 1天      |
| T11 | 为 LLM 推理结果添加缓存（相同 prompt 的复杂度判断结果可缓存）                                                    | `crates/cache/src/`                | 0.5天    |

### 3.2 🔧 建立测试/评估框架（P1-高）

| #   | 任务                                                                                     | 涉及文件                                               | 预估工时 |
| --- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------ | -------- |
| T12 | 定义 Agent 评估 benchmark：5-10 个标准任务 prompt，验证工具调用序列正确性 + 最终输出质量 | `crates/agent/tests/benchmark.rs`                      | 2天      |
| T13 | 引入 `cargo-nextest` 并行测试，设定 CI 强制通过                                          | `.github/workflows/rust-ci.yml`                        | 1h       |
| T14 | 建立集成测试：Mock LLM provider 返回，验证 agent 在已知输入下的行为不变性                | `crates/agent/tests/integration_tests.rs`              | 3天      |
| T15 | 引入 property-based testing（`proptest`），覆盖 agent 状态机的不变量                     | `crates/agent/tests/proptest_tests.rs`（已有文件扩展） | 1天      |

### 3.3 🔧 图编排引擎基础（P1-高）

**问题**：不支持 LangGraph 式的有状态图执行。

**方案**：在 `rt-workflow` crate 中增量实现，不推翻现有架构。

| #   | 任务                                                                   | 涉及文件                                   | 预估工时 |
| --- | ---------------------------------------------------------------------- | ------------------------------------------ | -------- |
| T16 | 定义 `Node` + `Edge` + `StateGraph` 核心类型（节点、条件边、状态类型） | `crates/rt-workflow/src/graph/`            | 2天      |
| T17 | 实现线性执行器：按拓扑序执行节点，传递状态                             | `crates/rt-workflow/src/graph/executor.rs` | 2天      |
| T18 | 实现条件边：`Edge::Conditional { source, condition_fn, target_map }`   | `crates/rt-workflow/src/graph/edge.rs`     | 1天      |
| T19 | 实现节点级断点：`NodeConfig::interrupt_before` / `interrupt_after`     | `crates/rt-workflow/src/graph/node.rs`     | 1天      |
| T20 | 将 `coordinator.rs` 的 Worker 调度改为基于 Graph 的调度（兼容旧接口）  | `crates/agent/src/coordinator.rs`          | 3天      |

---

## 四、Phase 2 — 功能深化（2-3 个月，P2）

### 4.1 🧠 记忆系统升级（P2）

| #   | 任务                                                                                                                       | 涉及文件                               | 预估工时 |
| --- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | -------- |
| T21 | 实现记忆自编辑：重要性评分 → 自动合并 → 过期淘汰                                                                           | `crates/agent/src/memory/`             | 3天      |
| T22 | 建立 3 层记忆架构：Working Memory（会话内，in-memory）→ Episodic Memory（会话间，SQLite）→ Semantic Memory（全局，向量化） | `crates/agent/src/memory/layers/`      | 3天      |
| T23 | 实现多维记忆检索排序（重要性 + 时效性 + 相关性加权）                                                                       | `crates/agent/src/memory/retrieval.rs` | 2天      |
| T24 | 为上下文压缩（compactor.rs）建立评估脚本，量化压缩后的信息保留率                                                           | `crates/agent/src/compactor.rs` + 测试 | 1天      |

### 4.2 🛡️ 安全增强（P2）

| #   | 任务                                                                    | 涉及文件                                     | 预估工时 |
| --- | ----------------------------------------------------------------------- | -------------------------------------------- | -------- |
| T25 | 引入 OPA/Cedar 策略文件支持，策略引擎从此支持标准化规则语言             | `crates/runtime/src/policy/`                 | 3天      |
| T26 | 运行时内存加密：对敏感数据（API keys, tokens）使用 `secrecy` crate 包装 | `crates/crypto/src/` + `crates/runtime/src/` | 1天      |

### 4.3 🔗 A2A 协议 + MCP 扩展（P2）

| #   | 任务                                                | 涉及文件                       | 预估工时 |
| --- | --------------------------------------------------- | ------------------------------ | -------- |
| T27 | 实现 A2A 协议客户端：发送 task 请求、接收状态更新   | `crates/mcp/src/a2a/client.rs` | 2天      |
| T28 | 实现 A2A 协议服务端：暴露 agent 能力为 A2A endpoint | `crates/mcp/src/a2a/server.rs` | 2天      |
| T29 | 补全 MCP Resource 协议（资源 URI 注册、读取）       | `crates/mcp/src/resource.rs`   | 2天      |
| T30 | 补全 MCP Prompt 协议（模板注册、参数填充）          | `crates/mcp/src/prompt.rs`     | 1天      |

### 4.4 🎯 规划与推理增强（P2）

| #   | 任务                                                                               | 涉及文件                                                             | 预估工时 |
| --- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------- | -------- |
| T31 | 在 Tree of Thoughts 旁新增 MCTS 规划器（配置可选开关）                             | `crates/agent/src/tree_of_thoughts.rs` 或 `crates/agent/src/mcts.rs` | 3天      |
| T32 | 为 hierarchical_planner 增加语义验证：在生成子任务时检测工具是否可用、资源是否可达 | `crates/agent/src/hierarchical_planner.rs`                           | 2天      |
| T33 | 实现结构化推理输出：LLM 回复中提取 XML/JSON 约束的思维链                           | `crates/agent/src/reasoning_router.rs`                               | 2天      |
| T34 | 增加实时反思：在执行中每 N 步调用 LLM 评估当前进度，动态调整策略                   | `crates/agent/src/reflector.rs`                                      | 2天      |

### 4.5 🖥️ 人机交互优化（P2）

| #   | 任务                                                                         | 涉及文件                                                           | 预估工时 |
| --- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------ | -------- |
| T35 | 图节点级中断：在工作流引擎中实现 `interrupt_before` / `interrupt_after` 语义 | `crates/rt-workflow/src/graph/node.rs`（与 T19 联动）              | 1天      |
| T36 | 审批超时 + 自动委托机制：超时后通知/降级/委托给备选审批人                    | `src/stores/shared/approval.ts` + `crates/runtime/src/approval.rs` | 2天      |
| T37 | 文件编辑 diff 预览 + 逐块 Accept/Reject（Cursor 模式）                       | `src/components/chat/` + `crates/tools/src/builtin/file_edit.rs`   | 3天      |
| T38 | 暂停时立即中断（而非等工具执行完）：给正在执行的工具发 cancel signal         | `crates/tools/src/execution.rs`                                    | 1天      |

### 4.6 📊 执行回放与分析（P2）

| #   | 任务                                                        | 涉及文件                                     | 预估工时 |
| --- | ----------------------------------------------------------- | -------------------------------------------- | -------- |
| T39 | 基于 trajectory 记录的数据，实现执行确定性重放              | `crates/trajectory/src/replay.rs`            | 3天      |
| T40 | 构建 Agent 性能分析仪表盘：成功率、延迟分布、Token 效率趋势 | `src/pages/devtools/dashboard.tsx`           | 3天      |
| T41 | 前端增加日志级别动态调整 UI（当前仅靠 env-filter）          | `src/stores/devtools/settings.ts` + 面板组件 | 1天      |

---

## 五、Phase 3 — 体验与对标（3-6 个月，P3）

### 5.1 🎤 多模态扩展（P3）

| #   | 任务                                                             | 涉及文件                                   | 预估工时 |
| --- | ---------------------------------------------------------------- | ------------------------------------------ | -------- |
| T42 | 集成本地 Whisper.cpp（语音识别），通过 `tauri-plugin-shell` 调用 | `crates/tools/src/builtin/speech.rs`       | 3天      |
| T43 | 集成 Piper TTS（本地语音合成）                                   | 同上                                       | 2天      |
| T44 | 视频帧提取 + 分析管道：传入视频 → 帧采样 → 视觉模型分析          | `crates/agent/src/vision_pipeline.rs` 扩展 | 3天      |
| T45 | 文本-视觉-语音多模态融合策略（图文联合推理）                     | `crates/agent/src/reasoning_router.rs`     | 2天      |

### 5.2 🔧 工具系统深层优化（P3）

| #   | 任务                                                                  | 涉及文件                                              | 预估工时 |
| --- | --------------------------------------------------------------------- | ----------------------------------------------------- | -------- |
| T46 | 统一工具描述为 OpenAI JSON Schema 格式，确保跨模型兼容                | `crates/tools/src/registry.rs` + `src/types/tools.ts` | 2天      |
| T47 | 实现 Tool Pipeline DSL：声明式工具链编排（如 `fetch → parse → save`） | `crates/tools/src/pipeline.rs`                        | 3天      |
| T48 | WASM 工具沙箱（实验性）：用 wasmtime 运行第三方工具                   | `crates/tools/src/sandbox_wasm.rs`                    | 5天      |
| T49 | MCP 服务器 QoS 监控 + 自动故障切换                                    | `crates/mcp/src/health.rs`                            | 2天      |
| T50 | MCP 服务器市场/发现机制（社区注册 + 推荐）                            | `crates/mcp/src/discovery.rs`                         | 2天      |

### 5.3 🧪 持续质量工程（P3）

| #   | 任务                                                              | 涉及文件                           | 预估工时 |
| --- | ----------------------------------------------------------------- | ---------------------------------- | -------- |
| T51 | 在 CI 中集成 SWE-bench Lite 自动评分（Agent 能力回归测试）        | `.github/workflows/swe-bench.yml`  | 3天      |
| T52 | Performance benchmark：每次 CI 记录构建时间、测试耗时、二进制大小 | `.github/workflows/perf-bench.yml` | 1天      |
| T53 | 为 `playwright-cli` 技能增加 Agent 执行路径的 E2E 覆盖            | `e2e/`                             | 3天      |

---

## 六、计划总览（甘特图风格）

```
Phase 0 (本周)              ████████████
  T1-T7: 测试基础设施+CI规范

Phase 1 (1-2个月)           ██████████████████████████████
  T8-T11: DefaultReasoningProvider 替换
  T12-T15: 测试/评估框架
  T16-T20: 图编排引擎基础

Phase 2 (2-3个月)           ██████████████████████████████████████████
  T21-T24: 记忆系统升级
  T25-T26: 安全增强（策略+加密）
  T27-T30: A2A + MCP 扩展
  T31-T34: 推理规划增强
  T35-T38: 人机交互优化
  T39-T41: 执行回放+分析

Phase 3 (3-6个月)           ████████████████████████████████████████████████████
  T42-T45: 多模态扩展
  T46-T50: 工具系统深层优化
  T51-T53: 持续质量工程
```

---

## 七、资源评估（单人开发视角）

| 阶段    | 预估总工时   | 说人话                          |
| ------- | ------------ | ------------------------------- |
| Phase 0 | **~2 天**    | 这周搞完                        |
| Phase 1 | **~3 周**    | 核心架构改进，每天 2-3h 的节奏  |
| Phase 2 | **~5-7 周**  | 功能深化，可以和 Phase 1 有重叠 |
| Phase 3 | **~8-12 周** | 锦上添花，不急着做              |

**建议执行节奏：**

- 每天早上先修 bug + 看 issue（1h）→ 然后集中攻 Phase 当前任务（2-3h）
- 每周五下午做 release checklist（`CHECKS.md`）+ 更新 `CHANGELOG.md`
- 每完成一个 Phase 发布一个 minor 版本（v2.9.0 → v2.10.0 → v2.11.0 → v3.0.0）

---

## 八、与现有路线图的对齐

当前 `README.md` / `CHANGELOG.md` 中 v3.0.0 的目标应与本计划对齐：

| 当前 README 承诺 | 本计划的 Phase       |
| ---------------- | -------------------- |
| 测试覆盖提升     | Phase 0 + Phase 1    |
| 架构重构         | Phase 1（图引擎）    |
| Agent 评估框架   | Phase 1              |
| 记忆系统增强     | Phase 2              |
| 安全增强         | Phase 2（策略+加密） |
| A2A 协议         | Phase 2              |
| 多模态支持       | Phase 3              |

---

## 九、风险提醒

1. **Scope creep 风险**：Phase 3 的项目（WASM 沙箱、SWE-bench 集成）可能比预估耗时更长，建议 Phase 2 结束后重新评估
2. **技术债积累**：Phase 1 的图引擎如果设计不当会增加后续维护成本，建议先写 RFC（`docs/rfcs/graph-engine.md`）review 再动手
3. **单人瓶颈**：上述 53 个任务如果全部做完估计需要 5-6 个月全职投入，建议优先保 Phase 0-2（核心质量），Phase 3 做选择性投入
4. **测试先行**：Phase 0 的测试基础设施建立后再动 Phase 1 的重构，避免"重构完发现没测试兜底"的恶性循环

---

> 计划生成日期：2026-07-05
> 基准版本：v2.8.0
> 下一版本建议：v2.9.0（Phase 0 完成时发布）
