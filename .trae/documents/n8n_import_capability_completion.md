# 修复 n8n 工作流导入缺陷 + 自动能力补全

## Context（为什么做）

当前 n8n 工作流导入（`src-tauri/src/commands/workflow_template.rs`）是"半成品"：

- 导入后的 `WorkflowTemplateData.tool_defs` 恒为空 `vec![]`（L2490），但 Agent 兜底节点已声明了 `AgentNodeConfig.tools`（如 `http_request_{node_id}`）——导致 LLM 能选中工具、运行时 `execute_tool` 却查无 handler，报"工具未注册"。

- Agent 兜底节点 `config.context_sources = Vec::new()`（L2297），丢失 n8n 节点间的数据传递。

- `check_workflow_duplicate`（L1906）名称级 Jaccard ≥0.6 过松，n8n 常见命名被误判重复改名。

- n8n 分支完全没调用 `work_engine.precompile_tool_defs`，即使填了 `tool_defs` 也不生效。

- 导入不会自动补全缺失能力，也不把新建的 Expert/Role 登记进能力索引，"导入即能力发现"未闭环。

目标：让导入闭环——精确映射能转的节点、给降级 Agent 补齐工具声明与上下文、缺的工具/专家/角色按安全红线自动补全并登记能力索引。

## 关键结论（来自查证，落地形态依据）

- **工具引用字段**：`AgentNodeConfig.tools: Vec<ToolDef>`；运行时经 `tool_defs_to_chat_tools`（crates/harness/src/agent\_turn\_runner.rs L146）转 `ChatTool`，`execute_tool` 只认 `tool_handlers`（Rhai 工具在 `precompile_tool_defs` 注册）。

- **Rhai 纯计算、无 HTTP**：`harness/src/rhai_engine.rs` 只注册 clamp/join/json\_parse，不能真发 HTTP。故 HTTP 类不强生成自包含脚本，优先**精确映射为** **`WorkflowNode::HttpRequest`**；确需补的工具按 `pending` 落 `workflow_tools` 表待人工确认。

- **"技能"无独立表**：由 `agency_experts`+`agent_roles`+`agent_profiles` 承载，复用 `ensure_agent_profile`（幂等）即可，不新造表/不新增逻辑。

- **能力索引必须单独登记**：`sync_template_passport` 只索引工作流自身；新建 Expert/Role 需另行 `index_passport` 才能被 `capability_discover` 检索。

- **不需要数据库迁移**：全部复用 `workflow_tools`、`agency_experts`、`agent_roles`、`agent_profiles`。

- **不修复 IF 降级 LLM 路由**（`JudgeByLlm`，L1192-L1210）——刻意设计。

## 改动点

主改文件：`src-tauri/src/commands/workflow_template.rs`

### 档位 1 — 低挂果实

1. **`map_n8n_node`（L1171）新增精确映射**：`n8n-nodes-base.httpRequest` → `WorkflowNode::HttpRequest(HttpRequestNode)`；`n8n-nodes-base.executeQuery/Postgres/MySql` → `WorkflowNode::DatabaseQuery`（若类型与输入字段匹配）。减少降级依赖 LLM。
2. **填充** **`tool_defs`**：在 `convert_n8n_to_axagent` 组装处（替换 L2490 的 `vec![]`），收集降级 Agent 节点 `config.tools` 声明过的工具名，为每个生成一条可经 `register_common_functions` 编译的**纯计算/占位** `RhaiToolDef`（`tool_name` 与声明同名；HTTP 等副作用给占位脚本 + 文本说明，并在返回 warning 标注 pending）。
3. **回填** **`context_sources`**：在 edges 拓扑修补（L2416-L2467）之后、构造 `WorkflowTemplateData` 前，按降级 Agent 节点的入边反推上游节点，把上游 `output_var` 写入 `config.context_sources`（替代 L2297 的空值；edges 未定型时无法回填，故必须后置）。
4. **查重阈值**：`check_workflow_duplicate`（L1947）名称级阈值 `>=0.6` → `>=0.95`（仅名称重名才改名，避免 n8n 误改名）。`find_similar_workflows`（L1888）节点类型 0.6 保留（只告警）。

### 档位 2 — 能力补全管线（核心）

新增函数（`workflow_template.rs` 模块内，`do_import_workflow` 尾端、事务提交后调用）：

```rust
async fn complete_imported_capabilities(
    state: &AppState,
    db: &DatabaseConnection,
    template: &WorkflowTemplateData,
) -> Vec<String>; // 返回新增/待确认项清单（并入 warnings）
```

- 扫描 `template.nodes` 中 `WorkflowNode::Agent`：收集 `agent_profile_id`、`tools` 名、仍无 handler 的工具。

- **工具补全（pending）**：缺 handler 的工具写 `workflow_tools` 表（`tool_type=rhai_script`、`status=pending`、幂等键 `workflow_id+tool_name`）。复用 `axagent_dao::repo::workflow_tool::upsert`（src/commands/workflow\_tool.rs L96 逻辑）。遵循项目安全红线：不自动 active。

- **Expert/Role/Profile**：不在此重复创建，复用 `ensure_agent_profile` 既有幂等（`find_by_id` 先查后写）。

- **能力索引登记**：收集本次新建的 Expert/Role/Profile 与工具，构造 `CapabilityPassportDto`，走 `state.capability_indexer.index_batch(...)`（capability.rs L96 同款）统一登记，使 `capability_discover` 可检索。

- 返回新增项清单并入 `warnings`，随导入结果回传前端。

`do_import_workflow` 同步补两处：

- 在 `sync_template_passport`（L2595）前插入 `state.work_engine.precompile_tool_defs(&template.id, &template.tool_defs).await;`（保证填了 `tool_defs` 即生效）。

- 在 `sync_template_passport` 后调用 `complete_imported_capabilities`，把返回清单 append 到 `warnings`。

### 档位 3 — 拓扑强制修补

保留 L2416-L2467（防断流兜底），但在 `do_import_workflow` 的 `warnings` 中标注"已补全 N 条边缘"，由前端在导入结果弹层展示。

### 前端（最小）

- 无需改交互代码。导入警告（含新增能力清单）由现有导入结果展示透传。

- 若需在模板详情看到 `tool_defs`，后端 `WorkflowTemplateResponse.tool_defs` 已存在（workflow\_types.rs L2583），前端类型可直接消费，本次不强做。

## 复用的既有实现

- `ensure_agent_profile` / `ensure_agent_role`：Expert+Role+Profile 幂等创建（workflow\_template.rs L1707/L1752）。

- `sync_template_passport` + `to_passport_dto`：模板护照回灌（L84）。

- `tool_defs_to_chat_tools`：`crates/harness/src/agent_turn_runner.rs` L146。

- `precompile_tool_defs`：`crates/rt-workflow/src/work_engine/engine/mod.rs` L568。

- `workflow_tools::upsert`：`axagent_dao::repo::workflow_tool`（workflow\_tool.rs L96）。

- 能力索引登记：`state.capability_indexer.index_batch`（capability.rs L96）。

## 验证

- **快速类型**：`src-tauri/` 下 `cargo check`（幂等秒级）。

- **提交前**：`cargo clippy -- -D warnings`（DTO 保持 snake\_case + `#[serde(rename_all="camelCase")]`，勿改 Rust 字段为 camelCase）。

- **单测落点**（`workflow_template.rs` tests 模块，L2961 起）：

  - `convert_n8n_to_axagent`（注入 sqlite tx）断言 `tool_defs` 非空、`httpRequest` 映射为 HttpRequest、Agent 节点 `context_sources` 已回填。

  - `complete_imported_capabilities` mock work\_engine/capability\_indexer，断言幂等（重复导入不重复建）与 passport 登记。

- **运行环境注意**（Windows）：

  - 终端的 cargo 需注入 PATH（`rg`/`cargo` 可用目录）；PowerShell 5 输出用 `2>$null` 避免 file-lock 误判 NativeCommandError。

  - Rust 单测必须 `__TAURI_WORKSPACE__=true cargo test -p axagent --lib`，否则测试 exe 缺 Common Controls v6 manifest 启动即报 `STATUS_ENTRYPOINT_NOT_FOUND`。

