# 认知编排分支缺陷 — 修复实施方案

> 依据 `src-tauri/src/commands/cognitive.rs` / `crates/harness/src/cognitive_router.rs` /
> `crates/harness/src/capability.rs` / `crates/tools/src/tools/*.rs` 逐行核对后的结论。
> 原文 11 条指控：3 条不属实、2 条部分属实、5 条属实，另补 2 条原文漏掉的真问题。
> 本文件只写**需要动手**的部分。

---

## 🔍 实施后 review（2026-09-01 完成 F1-F6 全部六项）

按 implement → review 循环逐项复查，**发现 3 个新缺陷**（1 个自引入、1 个既有、1 个口径问题）：

| #  | 发现                                                                                               | 位置                               | 处理                                     |
| -- | -------------------------------------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------- |
| R1 | F3 的 match 守卫改了分派路径却没改 `mode` 变量 → `execution_mode="workflow"` 却返回 Agent 执行视图 | `cognitive.rs` `defer_to_agent` 块 | ✅ 已修：defer 时统一改 `Delegate`       |
| R2 | F6 只覆盖 4 个命令层入口，实际有 13 处运行时写入 `workflow_templates`                              | 见 F6 章节                         | ✅ 已补 9 处                             |
| R3 | **profile 禁用工具被认知编排注入绕过**（F4 review 发现，既有缺陷，F4 扩大了影响面）                | `agent/mod.rs` 工具微调块          | ✅ 已修：blocked 过滤移到 extra 注入之后 |

### R3 详解（安全策略绕过，共 2 处）

**通病**：「黑名单过滤 + 白名单/推荐追加」同时存在时，**黑名单必须最后执行**，否则追加项能
把刚被移除的禁用项复活。同一代码库里 `agent_def_types.rs:156` `is_tool_allowed` 的顺序是对的
（先判 disallowed 再判白名单），反证下面两处是疏漏而非设计。

| 位置                                     | 原顺序                                          | 绕过后果                                                            | 附加问题                                                                                                          |
| ---------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `agent/mod.rs` 工具微调块                | 先 retain 移除 disallowed，再注入 `extra_tools` | 认知编排按能力护照 `tool_ref` 注入的工具能把 profile 禁用的工具注回 | —                                                                                                                 |
| `local_tool.rs:145-156` `get_tool_count` | 先 remove disallowed，再 insert recommended     | 同时出现在 `recommended_tools` 的禁用工具被注回                     | 追加源用 `registry.tools.list_all()`，**连 registry 层 `disable()` 都不过滤**，会把用户在设置里关掉的工具算进计数 |

关键事实：`get_chat_tools_by_names`（`crates/tools/src/registry.rs:815`）只过滤 registry 层的
`disable()`，不看 profile 的 `disallowed_tools`；而 `list_all()` 连 `disable()` 都不过滤。

**修法**：两处统一改为「先追加（走 `get_chat_tools_by_names` 复用 registry 统一过滤），
再应用 disallowed 作最终兜底」。这也恢复了 `get_tool_count` 与 `agent_query` 的筛选语义一致
（禁区 12：两者共享 `resolve_profile_tool_context`，顺序也必须一致，否则展示数量与实际注入数量漂移）。

F4 把 `extra_tools` 首次带进 Clarify 二次执行后，第一处的绕过面从通配分支扩大到两条路径。

> **后记（2026-09-01）**：该取舍已被用户指示推翻——已抽出 `fn apply_tool_policy(..) -> Vec<ChatTool>`
> 并补 7 个单测（见下方 R4）。**正是这次抽取暴露了 R4 这个 P0**，证明「因为不好测所以不测」
> 这个理由本身就是风险来源：不可测的代码通常也意味着没人真正验证过它生效。

### 🔴 R4 · 工具策略块是死代码，整块从未生效（P0，抽取 `apply_tool_policy` 时暴露）

**现象**：把 `push`/`retain` 的原地改写换成 `chat_tools = apply_tool_policy(...)` 返回新 Vec 后，
编译器立刻报 `unused_assignments: value assigned to chat_tools is never read`。

**根因**：`build_streaming_api_client` 在 `:1426` 按值接收 `chat_tools.clone()` 作为**快照**，
而工具策略块位于 `:1556` —— 在快照之后。此后对 `chat_tools` 的任何增删都不会进入下发 LLM
的工具列表。原写法用 `push`/`retain`（可变借用，非赋值）绕开了 `unused_assignments` 的检测，
所以这个缺陷一直潜在。

**影响面**（`request.extra_tools` 全文件仅此 1 个消费点）：

| 受影响项                                        | 后果                                                                                                 |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `profile.disallowed_tools`                      | **完全失效** —— 被 profile 禁用的工具照常出现在 LLM 可见列表                                         |
| `profile.recommended_tools`                     | **完全失效** —— 推荐工具从未被追加                                                                   |
| F4 的 `extra_tools` 注入（能力护照 `tool_ref`） | **完全失效** —— 主动模式下 LLM 只拿到 7 个 `DISCLOSURE_TOOLS` 元工具，命中能力的真实工具定义从未下发 |

对照：`extra_skills` 在 `:1219` 消费、`:1230` 进 `chat_tools`，**早于** 1426 快照 → F4 的
**技能半边是生效的，工具半边一直没生效**。这解释了为什么「主动模式发现的能力执行不了」这个
症状被反复记录却始终没被修好。

**修法**：把策略块整体前移到 `build_streaming_api_client` **之前**，并在该处留下「位置强约束」
注释（写明快照机制与踩坑史，防止后人再挪回去）。前移后必须仍在所有基础工具装配之后
（MCP `:1087` / 统一工具 `:1185` / 技能 `:1230` / 去重 `:1236` / Tauri 命令工具 `:1385`），
当前落点满足。

**R3 与 R4 的关系**：R3 修的是策略块**内部**的顺序（先追加后剔除），R4 修的是策略块**整体**
的位置。两者都必要——R4 之前，R3 修的顺序对了一个从不执行的代码块，属于「修了个寂寞」。

> **方法论**：改造为纯函数不只是为了可测，更是为了**让编译器能看见**。原地 `push`/`retain`
> 的副作用式写法对 lint 天然免疫；改成返回新值后，数据流断点立刻变成编译警告。

**残余观察（不修，记录备查）**：`request.options.disabled_tools`（`disabled_set`）只在两处生效：
`:1155`/`:1161` 过滤统一工具，以及 `:1166` 写入 `tool_registry` 的 `blocked_tools`（后者在
`registry.rs:543` `check_tool_enabled` **执行期**拦截）。因此：

- MCP / 技能 / Tauri 命令工具的 schema 本就不受 `disabled_set` 过滤，被禁工具的 schema 仍会出现在
  LLM 可见列表里，只是调用会被 `permission_denied` 拒掉。
- 策略块走 `get_chat_tools_by_names`（只查 `groups.is_tool_enabled`，不看 `blocked_tools`），
  同样不拦 `disabled_set`。

→ **不是安全绕过**（执行期有闸），只是「看得见但调不动」的既有 UX。前移策略块没有改变这一点。
若要收紧，把 `disabled_set` 并入 `blocked_names` 一起传给 `apply_tool_policy` 即可一行解决，
但会改变非统一工具的既有可见性行为，超出本轮范围，留给后续决定。

**端到端链路核查（前移后补做）** —— 生产 → 传输 → 消费三段连起来查：

| 段   | 位置                  | 内容                                                                                  | 判定                |
| ---- | --------------------- | ------------------------------------------------------------------------------------- | ------------------- |
| 生产 | `cognitive.rs`        | `resolve_exposure_injection` 产出 `forced_tools` / `orchestration_tools`              | ✅                  |
| 传输 | `cognitive.rs:1029`   | forced 路径（Clarify 二次执行）：`extra_tools: forced_tools` → 直调 `agent_query`     | ✅                  |
| 传输 | `cognitive.rs:1882`   | 编排路径（Ask/Act/Delegate）：`extra_tools: orchestration_tools` → 直调 `agent_query` | ✅                  |
| 传输 | `cognitive.rs:2063`   | 通用问答降级（Ask、无能力命中）：`extra_tools: None`                                  | ✅ 预期，注释已说明 |
| 消费 | `agent/mod.rs` 策略块 | 前移到快照之前（即本修复）                                                            | ✅                  |

**回归风险排查**（前移让策略块首次真正生效，需确认不误伤）：主动模式下基础工具列表只有
7 个 `DISCLOSURE_TOOLS`（`registry.rs:416`：SkillsList / SkillView / SkillReference /
DiscoverSkills / CapabilityView / CapabilityLoad / CapabilityBrowse），若某 profile 的
`disallowed_tools` 命中它们，会打断能力发现闭环。

- `config/` 与 `data/` 全仓 grep：**无任何** YAML/JSON 预置 `disallowedTools`；取值只来自 DB
  `agent_profiles` 表或 `agent_def_loader.rs:205` 的 YAML 键。
- `agent/src/verification_agent.rs:42` 的 `disallowed_tools() = ["FileWrite","FileEdit"]` 是另一个
  agent 的自有常量，不属 profile 的 `disallowed_tools`。
- → **无预置 profile 禁用披露工具，无回归。** 用户显式禁用属预期语义。

> **建议（未做，待用户拍板）**：可考虑让 `DISCLOSURE_TOOLS` 对 profile 黑名单免疫。它们是能力
> 发现闭环的元工具，被静默禁用会让编排器「发现不了任何能力」且极难归因。属防御性改进，
> 前提是确认是否允许管理员限制这类元工具。

### F5 复查的边界判断（先保留，后按用户指示改删）

`clamp_mode_for_kind`（`cognitive_router.rs:842`）的 `ParameterExtract` 分支**不可达**——
两个调用点传入的 mode 都取自 `execution_mode_from_confidence`，值域恒为 6 项，不含该变体
（`ParameterExtract` 仅由 JSON 快速通道产出后直接 return）。

**初判：保留**（理由是它属 P5 安全降级机制的语义穷举，漏掉会绕过保护）。

**用户指示改删（2026-09-01）** → 已删。最终形态：

```rust
match mode {
    ExecutionMode::Workflow | ExecutionMode::Direct => ExecutionMode::Delegate,
    other => other,
}
```

删除后，原先靠「多写一个永假分支」换取的安全感改由**文档契约**承担，两处注释已同步：

| 位置                             | 契约内容                                                                                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `execution_mode_from_confidence` | 标注「**值域契约**」：产出恒为 6 项、不含 `ParameterExtract`；下游 `clamp_mode_for_kind` **强依赖**此契约，此处新增档位必须同步确认下游覆盖 |
| `clamp_mode_for_kind`            | 写明入参值域来源与不含项，并加 ⚠️：若将来新增调用点且值域可能含 `ParameterExtract`，**必须同步补上该档位**，否则静默绕过委派保护             |

**注**：枚举变体 `ExecutionMode::ParameterExtract` 本身保留（JSON 快速通道响应、前端模式列表、
trajectory 进化证据判定 3 处真实使用），删的只是 `clamp_mode_for_kind` 里这一个永假的匹配分支。

> **事后复盘**：保留派与删除派的分歧点在于「永假分支算不算安全网」。保留派的收益是未来改值域时
> 自动生效；代价是这段分支**永远不会被任何测试覆盖**，且读者会误以为 `ParameterExtract` 真会走到
> 这里。删除派把保证从「代码」移到「注释」——更弱，但更诚实。用户选了后者，属于**显式指令优先于
> AI 判断**（P0 冲突规则），已执行。

---

## ✅ 实施状态（2026-08-31 批次 1-3 全部完成）

| #  | 修复项                                     | 状态 | 落点                                                                                                                                                                    |
| -- | ------------------------------------------ | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1 | 模板固化不回灌能力索引                     | ✅   | harness `workflow_types.rs:2231` 抽 `workflow_template_passport` 权威入口 + `repo_dtos.rs:88` `to_capability_passport`；`save_as_workflow.rs`（move 前派生护照 + 回灌） |
| F6 | 模板 update/delete/duplicate 索引同步      | ✅   | `workflow_template.rs` 新增 `sync_template_passport` / `remove_template_passport`，接入 insert/update/delete/duplicate 四入口                                           |
| F2 | 短路缓存 `execution_mode` 与实际分派不一致 | ✅   | `cognitive.rs` 短路覆盖块抽为 `apply_shortcut_override`（纯函数）+ 反推抽为 `derive_mode_from_execution_view`；补 2 个单测（四视图映射 + 缓存 mode=plan 撒谎场景）      |
| F3 | Workflow/Direct 无视会话已加载能力         | ✅   | `cognitive.rs` 新增 `resolve_exposure_injection` + `merge_injection` + 前置 2.5 读取 `loaded_capability_ids` + domain 交集软档闸（match 守卫）                          |
| F4 | Clarify 二次执行不注入 `tool_ref`          | ✅   | `cognitive.rs` forced 路径复用 `resolve_exposure_injection`                                                                                                             |
| F5 | `ParameterExtract` match arm 死代码        | ✅   | 删 36 行 arm；`cognitive_router.rs` 枚举注释补说明                                                                                                                      |

验证：`cargo check`（3 crate）✅ · `cargo fmt --check` ✅ · `cargo clippy --all-targets -D warnings`（3 crate）✅ · `cargo test -p axagent-harness --lib` 519 passed ✅ · `cargo test -p axagent --lib` 见下方结果。

---

## 一、结论总览

| #  | 修复项                                                   | 优先级 | 改动量                              | 批次   |
| -- | -------------------------------------------------------- | ------ | ----------------------------------- | ------ |
| F1 | 模板增删改不回灌能力索引（会话内孤儿）                   | **P0** | ~40 行（含 harness 新增 1 函数）    | 批次 1 |
| F2 | 短路缓存 `execution_mode` 与实际执行分派不一致           | **P0** | ~10 行                              | 批次 1 |
| F3 | Workflow/Direct 分支无视会话已加载能力                   | P1     | ~60 行（抽 1 函数 + 加 1 道前置闸） | 批次 2 |
| F4 | Clarify 二次执行不注入 `tool_ref`                        | P1     | ~5 行（依赖 F3 抽的函数）           | 批次 2 |
| F5 | `ExecutionMode::ParameterExtract` match arm 是死代码     | P2     | 删 36 行                            | 批次 3 |
| F6 | 模板 update/delete 不更新索引（F1 的延伸，同批次一起做） | P1     | ~10 行                              | 批次 1 |

**原文指控但不修**（详见第五节）：OnDemand 死锁、两套复制粘贴决策逻辑、`candidate_details` 为空、Plan 只拆解不执行、Ask/Act/Delegate 执行路径相同。

---

## 二、P0 修复项

### F1 · 模板增删改不回灌能力索引（会话内孤儿）

#### 现状

| 环节                       | 代码锚点                                                                                                   | 是否动索引                       |
| -------------------------- | ---------------------------------------------------------------------------------------------------------- | -------------------------------- |
| 启动期全量重建             | `src/init/state.rs:1384` `register_all_capabilities` → `:1413-1430` 从 `workflow_templates` 表全量收集护照 | ✅                               |
| 启动时机                   | `src/init/state.rs:2170`（`run_deferred_init` 内，只在启动时跑一次）                                       | —                                |
| SaveAsWorkflow 工具        | `crates/tools/src/tools/save_as_workflow.rs:240-246`                                                       | ❌ 只 `create_workflow_template` |
| save_dynamic_workflow 命令 | `src/commands/workflow_template.rs:2851-2855`                                                              | ❌ 只 `insert_workflow_template` |
| 模板 update / delete       | `src/commands/workflow_template.rs`（全文无 `capability_indexer` 引用）                                    | ❌                               |

**结论**：不是"永久孤儿"（重启后 `register_all_capabilities` 会捡起来），但**会话内不可路由**，且模板改名/删除后索引残留脏护照直到重启。

#### 根因（比原文描述更具体）

`save_as_workflow.rs:215` 构造的是 `axagent_harness::repo_dtos::WorkflowTemplateData`（`crates/harness/src/repo_dtos.rs:51`），
这个 DTO **没有** `to_passport_dto()`。权威护照派生口径在
`crates/harness/src/workflow_types.rs:2079` `impl CapabilityPassport for WorkflowTemplateData`（另一个同名 DTO）。

两个 DTO 字段不对齐：

| 字段                                               | `workflow_types` 版 | `repo_dtos` 版   |
| -------------------------------------------------- | ------------------- | ---------------- |
| `tags`                                             | `Vec<String>`       | `Option<String>` |
| `nodes`                                            | `Vec<Node>`         | `String`（JSON） |
| `visibility`                                       | 有                  | **无**           |
| `tool_defs` / `error_workflow_id` / `mission_hash` | 有                  | 无               |

护照关键派生依赖这些字段：`capability_id = "workflow:{id}"`、`kind = Workflow`、
`domain`/`sub_category` 从 `route_path` 拆解、`visibility` 由 `is_system_template()` 兜底、
`planning_complexity` 由 `nodes.len()` 分档。

#### 修复方案

**步骤 1 — harness 抽权威派生函数（口径唯一，防止索引漂移）**

在 `crates/harness/src/workflow_types.rs` 新增自由函数，供三处调用：

```rust
/// 工作流模板 → 能力护照的唯一权威口径。
///
/// 调用方（必须全部走这里，否则运行时增量索引与启动期全量重建会漂移）：
/// - `impl CapabilityPassport for WorkflowTemplateData`（本文件，启动期全量）
/// - `repo_dtos::WorkflowTemplateData::to_capability_passport`（运行时增量）
pub fn workflow_template_passport(
    id: &str,
    name: &str,
    description: &str,
    tags: Vec<String>,
    cluster_id: Option<&str>,
    route_path: Option<&str>,
    node_count: usize,
    visibility: crate::capability::Visibility,
) -> crate::capability::CapabilityPassportDto {
    // capability_id 统一 `workflow:{id}`；domain/sub_category 从 route_path 拆 L1/L2 段；
    // planning_complexity 按 node_count 分 Simple(<=3)/Moderate(4-10)/Complex(>10)。
    // 实现直接搬迁 workflow_types.rs:2080-2155 现有逻辑。
}
```

并让 `workflow_types.rs:2079` 的 trait impl 内部改为调用它（保持对外行为不变）。

**步骤 2 — `repo_dtos::WorkflowTemplateData` 补派生方法**

```rust
// crates/harness/src/repo_dtos.rs，impl WorkflowTemplateData 内
pub fn to_capability_passport(&self) -> crate::capability::CapabilityPassportDto {
    crate::workflow_types::workflow_template_passport(
        &self.id,
        &self.name,
        self.description.as_deref().unwrap_or(""),
        self.tags.as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default(),
        self.cluster_id.as_deref(),
        self.route_path.as_deref(),
        serde_json::from_str::<Vec<serde_json::Value>>(&self.nodes)
            .map(|v| v.len()).unwrap_or(0),
        // 无 visibility 列：按 route_path L1 段兜底，与 workflow_types 版同口径
        if self.route_path.as_deref().is_some_and(|p| p.starts_with("/system/")) {
            crate::capability::Visibility::SystemOnly
        } else {
            crate::capability::Visibility::Public
        },
    )
}
```

**步骤 3 — 两个写入口补索引（只 warn，不阻断主流程）**

```rust
// crates/tools/src/tools/save_as_workflow.rs:246 之后
let passport = template.to_capability_passport();
let passport_id = passport.capability_id.clone();
if let Err(e) = indexer.index_passport(&passport).await {
    // 索引失败不影响已落库的模板，重启后 register_all_capabilities 会重建
    tracing::warn!(capability_id = %passport_id, error = %e,
        "SaveAsWorkflow 模板已落库但能力索引失败（重启后自动重建）");
}
```

```rust
// src/commands/workflow_template.rs:2855 之后
let passport = template.to_passport_dto();   // 此处 template 已是 workflow_types 版
let passport_id = passport.capability_id.clone();
if let Err(e) = state.capability_indexer.index_passport(&passport).await {
    tracing::warn!(capability_id = %passport_id, error = %e,
        "save_dynamic_workflow 模板已落库但能力索引失败（重启后自动重建）");
}
```

**步骤 4（F6）— update / delete 同步索引**

| 命令                                  | 位置                                | 补什么                                                                        |
| ------------------------------------- | ----------------------------------- | ----------------------------------------------------------------------------- |
| `update_workflow_template`            | `src/commands/workflow_template.rs` | 落库成功后 `index_passport`（同一 capability_id 覆盖写）                      |
| `delete_workflow_template`            | 同上                                | 删除成功后 `state.capability_indexer.remove_index(&format!("workflow:{id}"))` |
| 模板重新种子（TEMPLATE_VERSION 递增） | 同上                                | 种子循环内对每个模板 `index_passport`                                         |

> `remove_index` 已在 `crates/harness/src/capability_indexer.rs:41` 定义，`src/commands/plugin.rs:222` 有现成调用范例。

**实际落地后的全入口复查（2026-08-31）**

原计划只列了 4 个入口，复查时按「谁往 `workflow_templates` 表写」重新穷举（grep `insert_workflow_template` / `update_workflow_template` / `delete_workflow_template` / `seed_preset_templates` 全项目调用点），发现 **另有 9 处运行时写入路径漏同步**，已全部补齐：

| # | 入口                                             | 位置                                 | 为什么漏了                                                                                               | 补法                                                                                       |
| - | ------------------------------------------------ | ------------------------------------ | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| ① | `save_dynamic_workflow`                          | `workflow_template.rs` insert 后     | 能力组装→固化闭环的落点，拼完模板直接调 dao 层 insert                                                    | 直接 `sync_template_passport`                                                              |
| ② | `do_import_workflow`（n8n 单文件导入）           | `workflow_template.rs` insert 后     | 函数只接 `db`，拿不到 `state`                                                                            | 加 `state: &AppState` 参数，两个分支（事务提交/直接插入）汇合后统一 sync；2 个调用点同步改 |
| ③ | `import_n8n_directory`（批量 n8n）               | `workflow_template.rs` insert 成功后 | 自管循环，不走 `do_import_workflow`                                                                      | 在 `imported.push` **之前** sync（`push` 会移动 `template.name`）                          |
| ④ | `seed_preset_templates`                          | `workflow_template.rs`               | 运行期手动触发（启动期全量重建已过）；`items` 被 db 层消费                                               | 消费前 `items.clone()` 留副本，落库后逐个 sync                                             |
| ⑤ | `persist_template` + `apply_rollback_to_version` | `workflow_ai_apply.rs`               | **直接调 dao 层 update，绕过 commands 层**，是 AI 应用类修改（插/删/改节点、改变量、快照回滚）的统一落点 | 抽 `sync_persisted_template()`；`persist_template` 加 `state` 参数，4 个调用点同步改       |
| ⑥ | `generate_gap_workflow_template`                 | `capability_gap_workflow.rs`         | **能力补齐闭环最后一环**                                                                                 | 直接 `sync_template_passport`                                                              |
| ⑦ | `compile_mission`                                | `workflow_ai/compile.rs`             | mission 编译产物                                                                                         | 直接 `sync_template_passport`                                                              |
| ⑧ | 技能分解落库                                     | `skill_decomposition.rs`             | 手上是 entity `ActiveModel`，无完整 DTO                                                                  | 走 `sync_template_index_by_id` 读回再派生                                                  |
| ⑨ | `save_skill_workflow_from_llm`                   | `skill_workflow.rs`                  | 同 ⑧                                                                                                     | 走 `sync_template_index_by_id` 读回再派生                                                  |

**统一收口（新增 2 个辅助函数，`workflow_template.rs`）：**

```rust
pub(crate) async fn sync_template_passport(state: &AppState, template: &WorkflowTemplateData)   // 手上有完整 DTO
pub(crate) async fn sync_template_index_by_id(state: &AppState, template_id: &str)              // 只有 ID，读回再派生
```

**三个辅助结论：**

- **护照派生口径已统一**：`repo_dtos::WorkflowTemplateData::to_capability_passport()` 与 `workflow_types::WorkflowTemplateData::to_passport_dto()` 最终都收敛到同一个 `workflow_template_passport()` 权威入口（F1 目标达成），两条链路不会产生不同 `capability_id`。
- **删除路径唯一**：`db_repo::delete_workflow_template` 全项目仅 `commands/workflow_template.rs` 一处调用，无批量删除等旁路，索引清理不会漏。
- **`save_as_workflow`（tools crate）此前已自带索引回灌**，且同样走 `workflow_template_passport()`，口径一致，无需改动。

> **教训（值得写进项目规范）**：以「命令层入口」为单位做同步是不可靠的，必须以「谁写这张表」为单位穷举。两类高危漏点：
> ① 绕过 commands 层直接调 dao 层（`workflow_ai_apply.rs`、能力补齐/mission 编译/技能分解各自单干）；
> ② 手上是 entity `ActiveModel` 拿不到完整 DTO 的调用方（默认就"没法同步"，于是干脆没同步）。
> 建议后续给 `db_repo` 的写入函数加 `#[must_use]` 或直接把索引同步下沉到 repository 层，从机制上杜绝此类遗漏（本轮不扩大改动范围，仅登记为技术债）。

#### 验证

```bash
# 类型检查（日常）
cargo check
# 提交前
cargo clippy -- -D warnings && cargo fmt -- --check
```

新增单测（放 `crates/tools/src/tools/save_as_workflow.rs` 的 `#[cfg(test)]`，或用现有 test fixture）：
固化模板后 `indexer.get_passport(&format!("workflow:{}", template_id))` 返回 `Some`，
且 `passport.kind == CapabilityKind::Workflow`、`passport.capability_id` 以 `workflow:` 开头。
另加一条对照断言：该护照与 `register_all_capabilities` 重建出的护照 `domain`/`visibility`/`planning_complexity` 三个字段相等（防漂移回归）。

> Windows 跑测试必须带环境变量，否则测试 exe 未嵌入 Common Controls v6 manifest，启动即 `STATUS_ENTRYPOINT_NOT_FOUND`：
> `__TAURI_WORKSPACE__=true cargo test -p axagent-tools save_as_workflow`

#### 风险

- `index_passport` 会生成 embedding（依赖本地 embedding 服务 `localhost:8091`）。服务不可用时索引失败 → 已用 `tracing::warn` 降级，不阻断。
- 若 `index_passport` 内部有同步锁/`await` 长耗时，在工具调用路径上会拖慢 SaveAsWorkflow 返回。建议评估后决定是否 `tokio::spawn` 异步化 —— 但异步化会让"固化后立即可路由"的时序不确定，**先同步实现**，测出耗时再优化。

---

### F2 · 短路缓存 `execution_mode` 与实际执行分派不一致

#### 现状

三条代码路径各说各话：

| 位置                   | 行为                                                                                                                                |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `cognitive.rs:566`     | 短路命中 → `request.forced_capability_id = Some(cached.capability_id)`                                                              |
| `cognitive.rs:742-745` | forced 路径按 `forced_kind` 定性：`Agent → "delegate"`，其余 → `"workflow"`                                                         |
| `cognitive.rs:758-777` | forced 路径实际分派：`Workflow` kind → `workflow_execute`；其余 → `agent_query`                                                     |
| `cognitive.rs:590-595` | **用缓存的 `cached.execution_mode` 覆盖响应字段**（缓存值是上一轮按 confidence 分档的产物，可能是 `plan`/`act`/`clarify`/`direct`） |

冲突场景：缓存 mode = `plan`（上轮 confidence 0.40~0.75）、kind = `workflow` 时：

- 响应字段 `execution_mode = "plan"`
- `response.execution = Workflow { workflow_id, execution_id }`（真的跑了工作流）
- 落库 `decision` 用 `"workflow"`（`:750` 构造）
- 若不走 agent，`<cognitive-mode>` 提示词也不会注入，LLM 侧无感知

**三方不自洽**：前端若按 `execution_mode` 渲染（如 plan 模式去等 `plan-generated` 事件）会挂错。

#### 修复方案

`cognitive.rs:590-595` 的覆盖块里**删掉 execution_mode 的覆盖**，改为按 `response.execution` 视图反推，保证与实际分派一致：

```rust
if let Some(ovr) = &shortcut_override {
    response.route_path = ovr.route_path.clone();
    response.domain = ovr.domain.clone();
    response.cluster = ovr.cluster.clone();
    // execution_mode 不再取缓存值：短路后实际走 forced 路径（按 kind 分派），
    // 缓存的 mode 是上一轮按 confidence 分档的结果，两者口径不同。
    // 统一以实际执行视图反推，保证响应字段 / decision 标签 / execution 视图三者一致。
    response.execution_mode = match &response.execution {
        Some(CognitiveExecutionView::Workflow { .. }) => ExecutionMode::Workflow.as_str().to_string(),
        Some(CognitiveExecutionView::Plan { .. }) => ExecutionMode::Plan.as_str().to_string(),
        Some(CognitiveExecutionView::Clarify { .. }) => ExecutionMode::Clarify.as_str().to_string(),
        Some(CognitiveExecutionView::Agent { .. }) | None => ExecutionMode::Delegate.as_str().to_string(),
    };
}
```

`route_path` / `domain` / `cluster` 三条保留覆盖（forced 路径不产出这三项，缓存值是正确的）。

#### 验证

手工复现：同一会话连续两条消息（触发 10 分钟内短路缓存），第一条让路由命中 workflow 能力但 confidence 落在 0.40~0.75（缓存 mode = plan），第二条检查：
`response.execution_mode == "workflow"` 且 `response.execution` 为 `Workflow` 视图、落库 decision 标签为 `workflow`。

建议补单测：构造 `shortcut_override { execution_mode: "plan", .. }` + forced kind = Workflow，断言响应 `execution_mode == "workflow"`。

✅ **已补（2026-08-31）**：覆盖块抽为 `apply_shortcut_override` 纯函数（`cognitive.rs`），反推逻辑抽为 `derive_mode_from_execution_view`；新增 2 个单测 `derive_mode_from_execution_view_maps_all_views`（四视图 + None 确定性映射）与 `apply_shortcut_override_overwrites_stale_cached_mode`（缓存 mode=plan + Workflow 视图 → 断言最终 `execution_mode == "workflow"` 且 route_path/domain/cluster 取缓存、confidence 保持 1.0）。

#### 风险

低。唯一副作用：短路命中时响应字段不再反映"上一轮真实路由的 confidence 档位"。
若前端有依赖该字段区分"plan 展示"的逻辑，需确认前端以 `response.execution` 为准（这是更可靠的判据，本来就该如此）。

---

## 三、P1 修复项

### F3 · Workflow/Direct 分支无视会话已加载能力

#### 现状

- `cognitive.rs:1361` `ExecutionMode::Workflow | ExecutionMode::Direct` → 直接 `workflow_execute()`，不进 LLM 循环。
- 路由全链路**零 SessionState 读取**：`NS_SKILL_LOADED`（`crates/harness/src/session_state.rs:60`）全项目仅 4 处使用，全在 agent 侧：

| 使用点         | 文件                                                            |
| -------------- | --------------------------------------------------------------- |
| 能力加载写入   | `crates/tools/src/tools/capability_load.rs:141`                 |
| 上下文注入读取 | `crates/agent/src/context_contributors/loaded_capability.rs:56` |
| 固化读取       | `crates/tools/src/tools/save_as_workflow.rs:127`                |
| 命令侧读取     | `src/commands/workflow_template.rs:2770`                        |

**后果**：用户已通过 CapabilityLoad 叠加了 N 个能力，若路由恰好命中一个现成模板（confidence > 0.75 且 kind = workflow），系统直接跑模板，已加载的能力组合被完全无视。

#### 修复方案（在主流程加一道"已加载能力优先"闸）

**步骤 1 — 抽出暴露闭环注入函数**（`:1614-1670` 现有逻辑，F4 也要复用）

```rust
/// 按能力护照的 exposure / kind 解析要注入 chat_tools 的工具与技能。
///
/// 原为 cognitive.rs 通配分支内联逻辑，抽出后供两处复用：
/// - 通配分支（Ask/Act/Delegate）路由命中能力
/// - Clarify 二次执行（forced_capability_id）
/// - F3 会话已加载能力集合
async fn resolve_exposure_injection(
    state: &AppState,
    capability_ids: &[String],
) -> (Option<Vec<String>>, Option<Vec<String>>) {
    let mut tools: Vec<String> = Vec::new();
    let mut skills: Vec<String> = Vec::new();
    for cid in capability_ids {
        let Some(p) = state.capability_indexer.get_passport(cid).await else { continue };
        match p.exposure {
            CapabilityExposure::OnDemand | CapabilityExposure::Managed => continue,
            CapabilityExposure::Auto => match p.kind {
                CapabilityKind::Skill => skills.push(
                    p.capability_id.strip_prefix("skill:").unwrap_or(&p.capability_id).to_string()),
                CapabilityKind::Toolchain => {
                    for step in &p.steps {
                        if let Some(sp) = state.capability_indexer.get_passport(step).await
                            && let Some(tr) = sp.tool_ref { tools.push(tr.tool_name); }
                    }
                },
                _ => { if let Some(tr) = p.tool_ref { tools.push(tr.tool_name); } },
            },
        }
    }
    (
        (!tools.is_empty()).then_some(tools),
        (!skills.is_empty()).then_some(skills),
    )
}
```

**步骤 2 — 路由前读取会话已加载能力**

在 `cognitive_query_inner` 前置阶段（`cognitive.rs:659` JSON 检测之前）新增：

```rust
// 前置 2.5：会话已加载能力感知
// 本会话已通过 CapabilityLoad 加载过能力时，用户意图大概率是"用这些已加载能力组合做事"，
// 不应被路由命中的现成模板抢走执行权 —— 转到 agent 路径，让 LLM 在已加载能力上下文中编排。
let loaded_capability_ids: Vec<String> = match request.conversation_id.as_deref() {
    Some(cid) if !cid.trim().is_empty() => {
        let prefix = namespace_prefix(StateScope::Temp, NS_SKILL_LOADED, cid, None);
        state.session_state_store.list_by_prefix(&prefix).await
            .unwrap_or_default().iter()
            .filter_map(|e| serde_json::from_str::<serde_json::Value>(&e.value).ok()
                .and_then(|v| v["capabilityId"].as_str().map(str::to_string)))
            .collect()
    },
    _ => Vec::new(),
};
let has_loaded_capabilities = !loaded_capability_ids.is_empty();
```

**步骤 3 — 分支判定加闸**

```rust
response.execution = Some(match mode {
    // 会话已加载能力时不走模板直发，转 agent 路径（已加载能力由 LLM 自行编排）
    ExecutionMode::Workflow | ExecutionMode::Direct if !has_loaded_capabilities => { /* 原逻辑 */ },
    _ => { /* agent_query 路径，extra_tools 合并 loaded 能力的注入结果 */ },
});
```

在 agent 路径里把 `resolve_exposure_injection(&state, &loaded_capability_ids)` 的结果
**合并**进 `orchestration_tools` / `orchestration_skills`（去重，路由命中能力优先）。

**软硬两档（建议先上软档）**

| 档位         | 判定                                                                     | 适用场景                                        |
| ------------ | ------------------------------------------------------------------------ | ----------------------------------------------- |
| 软档（推荐） | 仅当「已加载能力的 domain」与「路由命中能力的 domain」有交集时才转 agent | 避免"加载了股票能力后问天气"也被拖进 agent 路径 |
| 硬档         | 只要 `has_loaded_capabilities` 就转 agent                                | 实现最简，但无关问题也多花一轮 LLM              |

domain 交集判断：`loaded_capability_ids` 逐个取护照的 `domain`，与 `response.capability_id` 护照的 `domain` 比对，相等或任一方为 `General` 视为命中。

#### 验证

1. 单测：`has_loaded_capabilities = true` + mode = `Workflow` 时，断言 `response.execution` 为 `Agent` 视图而非 `Workflow` 视图。
2. 手工：会话内先 CapabilityLoad 两个 skill，再发一条能高置信命中现成工作流模板的消息 → 应走 agent 路径且 `extra_skills` 含这两个 skill。

#### 风险

- 会话内一旦加载过能力，后续所有请求都可能多走一轮 LLM（软档可缓解）。
- `NS_SKILL_LOADED` 的 value 结构是 `{"capabilityId": "..."}`（`workflow_template.rs:2785` 的解析口径），需确认 `capability_load.rs:141` 写入的是同一结构，否则读出来是空。`save_dynamic_workflow` 用的是 `v["capabilityId"]`，两者必须一致 —— **实施前先核对 `capability_load.rs` 的写入结构**。

---

### F4 · Clarify 二次执行不注入 `tool_ref`

#### 现状

`cognitive.rs:823-826`：

```rust
// Clarify 二次执行：按命中能力注入工具由调用方决定，此处不自动注入
extra_tools: None,
// Clarify 二次执行：技能按需加载由调用方决定，此处不自动注入
extra_skills: None,
```

**影响被原文夸大**：`DISCLOSURE_TOOLS` 白名单（`crates/tools/src/registry.rs:416-424`）在
`execution_mode.is_some()` 时无条件注入 CapabilityView / CapabilityLoad / CapabilityBrowse / SkillsList /
SkillView / SkillReference / DiscoverSkills（`src/commands/agent/mod.rs:1116-1128`），
所以 LLM **能**自己展开加载，代价是多一轮工具往返。**不是"无法执行"**，是浪费一轮 + 多耗 token。

#### 修复方案

复用 F3 抽出的 `resolve_exposure_injection`：

```rust
// cognitive.rs:799 构造 AgentQueryRequest 之前
let (forced_tools, forced_skills) = resolve_exposure_injection(&state, &[forced_id.clone()]).await;
// ...
extra_tools: forced_tools,
extra_skills: forced_skills,
```

#### 验证

Clarify 选一个 `kind = Tool` 且 `exposure = Auto` 的候选 → 断言 `AgentQueryRequest.extra_tools` 含该能力的 `tool_ref.tool_name`。

#### 风险

低。仅省一轮往返；若 passport 无 `tool_ref`（如知识库），行为与现在完全一致。

---

## 四、P2 修复项

### F5 · `ExecutionMode::ParameterExtract` match arm 是死代码

#### 现状

`cognitive.rs:1409-1444` 的 `ExecutionMode::ParameterExtract => {...}` 分支**永远进不去**：

- `execution_mode_from_confidence`（`cognitive_router.rs:817-831`）从不产出 `ParameterExtract`
- 唯一产出该值的 JSON 快速通道在 `cognitive.rs:693` 就 `return Ok(...)` 了，不经过 `match mode`

**但枚举变体必须保留**，它还有 4 处真实使用：

| 位置                                    | 用途                                                                                         |
| --------------------------------------- | -------------------------------------------------------------------------------------------- |
| `cognitive.rs:670` / `:706`             | JSON 快速通道响应字段                                                                        |
| `cognitive.rs:1822`                     | `cognitive_list_execution_modes` 供前端展示                                                  |
| `crates/trajectory/src/evidence.rs:95`  | 进化证据判定：`"workflow" \| "direct" \| "parameter_extract" \| "agent" \| "plan"` → Success |
| `crates/trajectory/src/evidence.rs:784` | 同上（测试）                                                                                 |

> 原文说"直接删枚举变体"是错的 —— 删了会让历史 decision 记录和 trajectory 进化证据判定全部失配。

#### 修复方案

只删 `cognitive.rs:1407-1444` 这个不可达的 match arm（36 行）。删后 `_ =>` 通配 arm 会接住它，编译期穷尽性没问题（match 已含 `_`）。

在 `cognitive_router.rs:230` 的枚举变体文档注释上补一句：

```rust
/// 精准命中（置信度 > 0.90）：跳过澄清，直接参数抽取后执行目标能力
///
/// 注意：本变体不会由 `execution_mode_from_confidence` 产出，
/// 仅用于 JSON 快速通道（`cognitive_query` 前置 2 直接 return 的路径）
/// 与 trajectory 进化证据判定。认知编排主 match 无对应分支。
```

#### 验证

`cargo clippy -- -D warnings` + `grep -rn "ParameterExtract" src-tauri/` 确认剩余引用均为上述 4 处合法用途。

---

## 五、原文指控但不修的项（含不修理由）

| 原文说法                                                                                            | 不修理由（代码事实）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 🔴 OnDemand 死锁：不注 extra_tools → LLM 拿不到 CapabilityView/CapabilityLoad schema → 无法展开加载 | **不成立**。元工具走独立白名单 `DISCLOSURE_TOOLS`（`crates/tools/src/registry.rs:416-424`），点名放行 CapabilityView/CapabilityLoad/CapabilityBrowse/SkillsList/SkillView/SkillReference/DiscoverSkills；`src/commands/agent/mod.rs:1116-1128` 在 `execution_mode.is_some()` 时按名注入，**与 exposure 状态无关**。OnDemand 护照也照常进 `<capability-index>` 目录（`src/commands/agent/capability_index.rs:119` 只判 `is_user_visible()`）。加载后经 `DynamicToolSet` → `agent/mod.rs:446` 下发，下轮即可调用。闭环是通的，省的是 token 不是能力。 |
| 🟡 Clarify 二次执行会命中短路缓存，用的是缓存能力而非用户所选                                       | **不成立**。短路缓存前置检查要求 `request.forced_capability_id.is_none()`（`cognitive.rs:537`），且 Clarify 模式不写缓存（`:1304-1307`）。前端二次调用带 `forcedCapabilityId`（`src/stores/domain/conversationStoreSend.ts:772`），走 `:724` 强制路径，完全绕开路由与缓存。                                                                                                                                                                                                                                                                         |
| 🟡 L3 图谱路由与 `route_with_hint` 两套复制粘贴实现                                                 | **不成立**。两者调用同一个 `execution_mode_from_confidence`（`cognitive_router.rs:817`）和 `clamp_mode_for_kind`（`:838`），注释明写"供两处共用"。调用点 `:953-954` 与 `:1263-1264`。且 `route_with_hint` 不在生产链路（commands 层零引用）。                                                                                                                                                                                                                                                                                                       |
| 🟡 Plan 路径 `candidate_details` 为空                                                               | **不成立**。`L3_NORMALIZE_EXPRESSION`（`src/init/cognitive_router_init.rs:945-950`）在 RAR 无候选时兜底构造单候选。唯一空候选是 `workflow.output = None` 的合成 general_qa（`cognitive.rs:1075-1088`），且 Clarify 空候选已有显式兜底（`:1345-1356`）。                                                                                                                                                                                                                                                                                             |
| 🟡 Plan 只拆解不执行                                                                                | **不算缺陷，是设计**。`plan_generate`（`cognitive.rs:1453`）落库 + 发 `plan-generated` 事件，执行靠前端 `src/stores/feature/planStore.ts:161` 用户点批准后调 `plan_execute`。这是**人在环审批闸**，改掉等于去掉人工确认。若要改，正确方向是"审批通过后自动回填执行结果"，而非后端自转。                                                                                                                                                                                                                                                             |
| 🟡 Ask/Act/Delegate 执行路径相同                                                                    | **不建议改**。`cognitive.rs:1489` 一个 `_ =>` arm，差异只在注入 `<cognitive-mode>` 提示词（`src/commands/agent/mod.rs:1894-1899`），靠 LLM 自觉。硬拆成三条执行路径会造成行为割裂且收益不明。真要落差异，应落到**请求参数**层面（如 `max_tool_iterations`、是否允许写操作），而不是拆执行分支 —— 属新需求，不在本次缺陷修复范围。                                                                                                                                                                                                                   |

---

## 六、实施批次

### 批次 1（P0，可独立发布）

1. F1 步骤 1-3：harness 抽 `workflow_template_passport` + `repo_dtos` 补 `to_capability_passport` + 两个写入口补索引
2. F6：模板 update/delete/重种子补索引同步
3. F2：短路缓存 execution_mode 改为按执行视图反推

门禁：`cargo check` → `cargo clippy -- -D warnings` → `cargo fmt` → 新增单测（Windows 带 `__TAURI_WORKSPACE__=true`）

### 批次 2（P1，依赖批次 1）

4. F3 步骤 1：抽 `resolve_exposure_injection`（先抽函数、改通配分支调用，单独验证行为不变）
5. F4：Clarify 二次执行接入该函数
6. F3 步骤 2-3：会话已加载能力前置闸（**先实现软档 domain 交集判定**）

> 注意 F3 实施前必须先核对 `capability_load.rs:141` 写入 SessionState 的 value 结构
> 与 `workflow_template.rs:2785` 的读取口径 `v["capabilityId"]` 是否一致，不一致则读取恒空。

### 批次 3（P2，清理）

7. F5：删 `cognitive.rs:1407-1444` 死 arm + 补枚举注释

---

## 七、验证命令清单

```bash
cd d:/OneManager/AxAgent/src-tauri

# 日常快速检查
cargo check

# 提交前门禁（CI 强制）
cargo clippy -- -D warnings
cargo fmt -- --check

# 单元测试（Windows 必须带 __TAURI_WORKSPACE__，否则测试 exe 启动即 0xc0000139）
__TAURI_WORKSPACE__=true cargo test -p axagent-tools save_as_workflow
__TAURI_WORKSPACE__=true cargo test -p axagent --lib commands::cognitive
__TAURI_WORKSPACE__=true cargo test -p axagent-trajectory evidence

# 死代码复查（F5 后）
grep -rn "ParameterExtract" src/ crates/ | grep -v target
```

---

## 八、待确认事项（全部闭环）

| # | 事项                                                                                    | 结论                                                                                                                                                                                                                                                       |
| - | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | `capability_load.rs:141` 写入 SessionState 的 value 结构是否与 `v["capabilityId"]` 一致 | ✅ **一致**。写入 `capability_load.rs:147-153` 为 `{ capabilityId, kind, name, agentId, loadedAtMs }`（camelCase），读取 `workflow_template.rs:2830-2832` / `save_as_workflow.rs:146-148` 均用 `v["capabilityId"]`。                                       |
| 2 | `index_passport` 在 embedding 服务不可用时的失败模式                                    | ✅ **内置降级，无需异步化**。`capability_indexer_impl.rs:260-270`：embedding 失败仍 `store_metadata` 并返回 `Ok(success:false)`，不传播 Err、不阻塞。同步调用 + warn 降级已足够，**不需要 `tokio::spawn`**。                                               |
| 3 | 前端是否消费 `response.execution_mode` 做分支渲染                                       | ✅ **仅展示，无分支渲染**。前端分发以 `conversationStoreSend.ts:795` 的 `cognitiveResult.execution.kind` 为准；`executionMode` 只用于 `CognitiveRoutePanel.tsx:78-80` / `CognitiveDecisionCard.tsx:48` 展示标签。F2 修复后标签与实际执行一致，无兼容风险。 |
| 4 | 模板 update/delete 是否已有其它索引同步路径                                             | ✅ **无其它路径**。`workflow_template.rs` 全文无 `capability_indexer` 引用（F6 前），无触发器/事件监听触碰能力索引；唯一重建路径是启动期 `register_all_capabilities`。F6 不重复。                                                                          |
