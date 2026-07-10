# AxAgent Harness 架构违规审计报告

> 生成时间：2026-07-10 · 检查范围：`src-tauri/` 全部 32 个 `axagent-*` crate（含 `schema-gen` 与根包 `axagent`）
> 校验基准：AGENTS.md「Rust 后端：Harness 架构准则」六条铁律

## 一、结论摘要

| 类别                                            | 数量 | 严重度 |
| ----------------------------------------------- | ---- | ------ |
| 依赖方向硬违规（铁律 2：consumer 越过 harness） | 5    | 🔴 高  |
| dev-dependencies 测试分层违规（铁律 5）         | 5    | 🟠 中  |
| 重复类型体系（铁律 4）                          | 6 组 | 🔴 高  |
| 循环依赖（铁律 1）                              | 0    | ✅     |
| 角色声明缺失（铁律 6，软性）                    | 见注 | 🟡 低  |

**核心问题**：`runtime-core` 越权充当了"共享类型源头"——harness 中的 `ConversationMessage`/`TokenUsage`/`ContentBlock`/`TurnSummary` 注释明确写着 _"Mirrors `axagent_runtime_core::...`"_，依赖方向**完全反了**；`consumer`（`agent`）直接挂了 `dao`/`entities`/`kit`/`runtime-core`，绕过 harness；`TokenUsage` 在 harness 内部就有两套互不兼容的定义。

---

## 二、检查方法

1. 读取全部 `Cargo.toml`，提取 `axagent-*` 的 normal / dev / target dev 依赖。
2. 按 AGENTS.md「crate 角色对照表」标注每个 crate 角色（foundation / consumer / implementor / hybrid / wiring）。
3. 逐条比对铁律允许的依赖组合；对每条命中做人工源码核验（确认非残留声明、非 re-export）。
4. 重复类型扫描：对 AGENTS.md 点名的 5 个共享 DTO（`ConversationMessage`/`TokenUsage`/`Session`/`PermissionMode`/`HookEvent`）做跨 crate 定义定位。
5. Tarjan SCC 检测循环依赖。

> 可复跑脚本：`./src-tauri/harness_audit.py`（managed Python 3.13，`tomllib` 无第三方依赖）。

---

## 三、🔴 依赖方向硬违规（铁律 2 / 铁律角色表）

### 3.1 consumer 越过 harness 直接依赖实现层

| # | 依赖方(crate / 角色) | 被依赖                 | 被依赖角色           | 位置                         | 铁律                           |
| - | -------------------- | ---------------------- | -------------------- | ---------------------------- | ------------------------------ |
| 1 | `agent` / consumer   | `axagent-dao`          | implementor          | `crates/agent/Cargo.toml:9`  | 2                              |
| 2 | `agent` / consumer   | `axagent-entities`     | foundation(entities) | `crates/agent/Cargo.toml:10` | 2                              |
| 3 | `agent` / consumer   | `axagent-kit`          | implementor          | `crates/agent/Cargo.toml:12` | 2                              |
| 4 | `agent` / consumer   | `axagent-runtime-core` | consumer             | `crates/agent/Cargo.toml:14` | 2（consumer 间依赖，分层偏差） |

> 已核实：`agent` 在 `action_executor.rs`/`session_manager.rs`/`trajectory_recorder.rs` 等 **20+ 源文件**真实调用 `axagent_dao`/`axagent_entities`/`axagent_kit`/`axagent_runtime_core`，非残留声明。
> `agent → runtime-core` 属 consumer 间依赖，严格按角色表（consumer 仅可依赖 harness）亦为违规，且是 #5 的根因之一。

### 3.2 hybrid 禁止依赖 consumer

| # | 依赖方(crate / 角色) | 被依赖                 | 被依赖角色 | 位置                         | 铁律          |
| - | -------------------- | ---------------------- | ---------- | ---------------------------- | ------------- |
| 5 | `tools` / hybrid     | `axagent-runtime-core` | consumer   | `crates/tools/Cargo.toml:13` | hybrid 角色表 |

---

## 四、🟠 dev-dependencies 测试分层违规（铁律 5）

铁律 5：consumer 测试仅可用 `harness::test_support` mock，禁止 dev 依赖实现层；implementor/hybrid/wiring 测试仅允许 dev 依赖 `axagent-dao`（`create_test_pool`），不得引入其他实现层。

| #  | 依赖方 / 角色             | dev 依赖               | 被依赖角色            | 位置                                 | 说明                           |
| -- | ------------------------- | ---------------------- | --------------------- | ------------------------------------ | ------------------------------ |
| 6  | `agent` / consumer        | `axagent-migration`    | implementor           | `crates/agent/Cargo.toml:42`         | consumer dev 依赖实现层        |
| 7  | `agent` / consumer        | `axagent-tools`        | hybrid                | `crates/agent/Cargo.toml:43`         | consumer dev 依赖 hybrid       |
| 8  | `runtime-core` / consumer | `axagent-prompt-guard` | implementor           | `crates/runtime-core/Cargo.toml:24`  | consumer dev 依赖实现层        |
| 9  | `runtime-core` / consumer | `axagent-telemetry`    | implementor           | `crates/runtime-core/Cargo.toml:25`  | consumer dev 依赖实现层        |
| 10 | `runtime` / wiring        | `axagent-search`       | implementor（非 dao） | `crates/runtime/Cargo.toml` dev-deps | wiring 测试仅允许 dev 依赖 dao |

---

## 五、🔴 重复类型体系（铁律 4）

铁律 4：共享类型（`ConversationMessage`/`TokenUsage`/`Session`/`PermissionMode`/`HookEvent` 等）权威定义在 `axagent-harness`，其余 crate 必须 `pub use axagent_harness::X`，不得重复定义。

| #  | 类型                  | 定义位置（重复处）                                                                                       | 问题                                                                                                                               |
| -- | --------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| 11 | `ConversationMessage` | `harness/conversation_model.rs:36` **+** `runtime-core/session.rs:40`                                    | harness 注释明确 _"Mirrors `axagent_runtime_core::session::ConversationMessage`"_ —— **依赖方向反了**，runtime-core 不应是源头     |
| 12 | `TokenUsage`          | `runtime-core/usage.rs:44` **+** `harness/conversation_model.rs:47` **+** `harness/settings_chat.rs:415` | **三套定义**；harness 内部就有两套互不兼容形状（DeepSeek 式 vs OpenAI 式）；`harness/conversation_model.rs:47` 又镜像 runtime-core |
| 13 | `ContentBlock`        | `harness/conversation_model.rs` **+** `runtime-core/session.rs:32`                                       | 与 #11 同源镜像问题                                                                                                                |
| 14 | `TurnSummary`         | `harness` **+** `runtime-core`                                                                           | 同上镜像问题                                                                                                                       |
| 15 | `HookEvent`           | `plugins/hooks.rs:17` **+** `runtime-core/hooks.rs:22`                                                   | **harness 中完全缺失**，共享类型未集中；两处定义发散                                                                               |
| 16 | `Session`             | `runtime-core/session.rs:82` **+** `trajectory/storage.rs:1550`                                          | **harness 中完全缺失**，共享类型未集中；两处定义发散                                                                               |

> 特别严重项：**#12 `TokenUsage`** —— `harness/settings_chat.rs:415`（OpenAI 式：`prompt_tokens`/`completion_tokens`/`total_tokens`）与 `harness/conversation_model.rs:47`（DeepSeek 式：`input_tokens`/`output_tokens`/`cache_*_input_tokens`）字段结构完全不同，却同名；再加上 `runtime-core/usage.rs:44` 的第三套。这意味着同一语义在不同模块用不兼容结构体传递，是类型体系分裂的典型症状。

---

## 六、✅ 循环依赖

Tarjan SCC 检测：未发现任何 size>1 的强连通分量（无双向/环依赖）。

---

## 七、🟡 角色声明（铁律 6，软性）

AGENTS.md 中央「crate 角色对照表」已覆盖全部 32 个 crate 的角色，满足"在 AGENTS.md 中标注"的要求。但绝大多数 crate 各自的 `crates/<name>/README.md` 未单独声明角色——属文档完善度问题，非架构违规，建议后续补声明即可。

---

## 八、修复优先级建议

**P0（必须修，阻断分层）**

1. 反转类型源头：`runtime-core` 删除本地 `ConversationMessage`/`TokenUsage`/`ContentBlock`/`TurnSummary`，改为 `pub use axagent_harness::*`；harness 删除 `conversation_model.rs` 中"镜像"注释与镜像定义，改为权威定义（建议统一为一套带 `cache_*` 字段的 TokenUsage）。
2. `agent` 移除对 `dao`/`entities`/`kit` 的 normal 依赖，所需能力改经 harness trait 注入（harness 新增对应 trait + DTO）。

**P1（应尽快修）**
3. `tools` 移除对 `runtime-core` 的 normal 依赖；`runtime-core` 需要的工具能力经 harness trait 暴露。
4. 补齐 harness 缺失的 `Session` / `HookEvent` 权威定义，让 `plugins`/`runtime-core`/`trajectory` 改为 `pub use`。

**P2（测试分层合规）**
5. consumer（`agent`/`runtime-core`）测试 dev 依赖改为 `harness::test_support` mock；`runtime` 的 dev 依赖 `search` 改为仅 `dao`（或抽离为独立集成测试 crate）。

**P3（文档）**
6. 各 crate README 补角色声明；保留 `harness_audit.py` 纳入 CI 做门禁。

---

## 附录 A：各 crate 角色与依赖快照

> N = normal 依赖（axagent-_）；D = dev 依赖（axagent-_）

| crate                                                                                                                                                                                     | 角色        | N                                             | D                           |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------------------------- | --------------------------- |
| harness                                                                                                                                                                                   | foundation  | —                                             | —                           |
| entities                                                                                                                                                                                  | foundation  | harness                                       | —                           |
| disk-cache / rt-dashboard / rt-theme                                                                                                                                                      | foundation  | —                                             | —                           |
| schema-gen                                                                                                                                                                                | foundation  | harness                                       | —                           |
| agent                                                                                                                                                                                     | consumer    | **dao, entities, kit, runtime-core**, harness | **migration, tools**        |
| orchestrator                                                                                                                                                                              | consumer    | harness                                       | —                           |
| runtime-core                                                                                                                                                                              | consumer    | harness                                       | **prompt-guard, telemetry** |
| gateway                                                                                                                                                                                   | consumer    | harness                                       | —                           |
| dao / storage / migration / kit / cache / crypto / credential / mcp / search / providers / prompt-guard / telemetry / trajectory / plugins / npm / document-parser / rt-webhook / scanner | implementor | harness(+entities/兄弟implementor)            | —                           |
| tools / rt-messaging / rt-workflow                                                                                                                                                        | hybrid      | harness + implementor                         | tools 无；rt-workflow: dao  |
| runtime                                                                                                                                                                                   | wiring      | 全部                                          | search                      |

> 加粗项即本报告命中的违规依赖。
