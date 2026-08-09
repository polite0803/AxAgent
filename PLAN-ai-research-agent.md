---

## 修订记录（2026-08-09 09:17-09:55，审查后遗留问题修复）

### R1（P2→已修）KPI 半配套

- `ai_research.rs`：移除无数据源的 `quality_score`（KPI 定义 + dashboard 卡），`task_completion_rate` 真实计算（已完成/非取消项目数），`compute_kpis` 返回 2 个真实 KPI，`default_kpi_definitions` 补全 2 条。

### R2（P2→已修）status 契约

- 前端 `IndustryTabContent.tsx` handleExecute 兼容 `"success"`（= 成功但有跳过/装饰节点未执行），不再误报失败。

### R3（P2→已修）DAG 内 KPI/Aggregator 节点冗余

- 根因：rt-workflow 对非 Rhai Code 节点只返回 `code_ready` 占位（code_executor.rs:565-580），KPI Code 节点（language="rust"）不真实计算；Aggregator 输入源与 KPI 节点 output_var 不匹配。KPI 数据实际由 `opc_get_industry_dashboard` → `adapter.aggregate_dashboard()` 独立提供（opc_industry_runtime.rs:154）。
- 修复：`workflow.rs` `from_adapter` 删除 KPI Code 节点块与 Aggregator 节点块，**模板版本 3→4**（否则 DB 已存 v3 不重 seed）。DAG 精简为 Trigger → Validation → 业务 AgentNode → 自动化规则 → End。
- 顺带：清理 `AggregatorNode/AggregatorNodeConfig` 未使用 import。

### 验证

- `cargo check` analysis-engine ✅ 零警告；主 crate ✅（仅剩历史遗留 `spawn_workflow_run` dead_code，非本次）；前端 `tsc --noEmit` ✅；clippy 本次改动零新增警告。
- 注意：编译期遇到 rustc 1.97 增量缓存损坏 panic（axagent-runtime），清空 `target/debug/incremental` 后恢复——非代码问题。
