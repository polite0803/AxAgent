# OPC 工作流重构方案

## 1. 背景与目标

### 1.1 当前问题

OPC（One-Person Company）工作流存在两种实现形式，与股票分析工作流不一致：

| 类型                               | 当前实现                                    | 问题                   |
| ---------------------------------- | ------------------------------------------- | ---------------------- |
| **行业工作流**（14行业）           | 通过 `OpcIndustryAdapter` 动态生成 DAG      | 间接层，难以调试和扩展 |
| **领域工作流**（17领域，75工作流） | 通过 `DomainWorkflowGenerator` 动态生成 DAG | 间接层，难以调试和扩展 |
| **股票分析工作流**                 | 手动定义 `WorkflowNode/Edge` 后种子化       | ✅ 正确实现            |

### 1.2 目标

将所有 OPC 工作流重构为与股票分析工作流**完全一致**的实现形式：

```
手动定义 WorkflowNode/Edge → 种子化写入 workflow_template 表 → 运行时 DB 加载执行
```

---

## 2. 统计数据

### 2.1 需要重构的工作流

#### 行业工作流（14 个行业，50 个工作流步骤）

| 行业                | 步骤数 | 专家配置 |
| ------------------- | ------ | -------- |
| accounting          | 4      | ✅       |
| ai_research         | 4      | ✅       |
| content_media       | 5      | ✅       |
| finance_invest      | 5      | ❌       |
| game_dev            | 5      | ❌       |
| design              | 3      | ❌       |
| ecommerce           | 3      | ❌       |
| education           | 3      | ❌       |
| geospatial          | 3      | ❌       |
| industry_consulting | 3      | ❌       |
| project_management  | 3      | ❌       |
| sales_growth        | 3      | ❌       |
| security            | 3      | ❌       |
| software_dev        | 3      | ❌       |

#### 领域工作流（17 个领域，75 个工作流）

| 领域        | 工作流数 |
| ----------- | -------- |
| academic    | 2        |
| design      | 4        |
| engineering | 13       |
| finance     | 3        |
| gamedev     | 3        |
| gis         | 4        |
| marketing   | 10       |
| paidmedia   | 2        |
| pm          | 3        |
| product     | 3        |
| sales       | 5        |
| security    | 4        |
| spatial     | 2        |
| specialized | 10       |
| strategy    | 2        |
| support     | 3        |
| testing     | 3        |
| **合计**    | **75**   |

### 2.2 需要创建的资源

| 资源类型             | 数量       | 说明                           |
| -------------------- | ---------- | ------------------------------ |
| 业务负责人角色       | 按实际需要 | 仅需统一身份入口的行业/领域    |
| 行业专家 .md 文件    | 43 个      | 每个工作流步骤对应一个专家     |
| 领域专家 .md 文件    | ~200 个    | 每个领域工作流步骤对应一个专家 |
| 行业工作流 seed 文件 | 14 个      | 每个行业一个 seed 文件         |
| 领域工作流 seed 文件 | 17 个      | 每个领域一个 seed 文件         |

#### 业务负责人角色创建原则

**创建场景**（参考股票分析 `stock-investment-lead`）：

- ✅ 需要统一身份入口（如：财务行业 → CFO 角色）
- ✅ 需要合规边界注入（如：金融投资、安全合规）
- ✅ 多专家协作需要一个负责人角色

**不创建场景**：

- ❌ 纯执行类行业（如：设计、软件开发）
- ❌ 工作流步骤之间无需统一身份
- ❌ 仅需专家方法论，不需要岗位身份

---

## 3. 目标架构

### 3.1 文件结构

```
src-tauri/
├── src/
│   └── commands/
│       ├── opc_setup/
│       │   ├── mod.rs                    # 主入口（种子化调度）
│       │   ├── roles.rs                  # 角色定义（公司核心角色 + 业务岗位 + 审批岗位）
│       │   ├── industry_agents.rs        # 行业专家/Profile 种子化
│       │   ├── industry_experts.rs       # 行业专家定义（新增）
│       │   └── domain_agents.rs          # 领域专家/Profile 种子化（新增）
│       ├── opc_workflows/
│       │   ├── mod.rs                    # 工作流种子化调度
│       │   ├── seed_industry_accounting.rs
│       │   ├── seed_industry_ai_research.rs
│       │   ├── ... (14 个行业 seed 文件)
│       │   ├── seed_domain_academic.rs
│       │   ├── seed_domain_design.rs
│       │   ├── ... (17 个领域 seed 文件)
│       │   └── seed_stock_pipeline.rs     # 已有
│       └── ...
│
└── agency_experts/
    ├── stock-analysis/                   # 已有
    └── opc/
        ├── industries/                   # 行业专家（新增）
        │   ├── accounting/
        │   ├── finance_invest/
        │   └── ... (12 个行业目录)
        └── domains/                      # 领域专家（新增）
            ├── academic/
            ├── design/
            └── ... (17 个领域目录)
```

### 3.2 三层结构（与股票分析一致）

```
角色（agent_roles）→ 专家（agency_experts）→ Profile（agent_profiles）
```

#### Profile 的三种模式

| 模式         | agent_role | expert_id | 适用场景                | 示例                           |
| ------------ | ---------- | --------- | ----------------------- | ------------------------------ |
| **专家驱动** | ❌ 无      | ✅ 有     | 纯执行类工作流          | `academic-literature-searcher` |
| **岗位驱动** | ✅ 有      | ❌ 无     | 审批/决策类工作流       | `opc_approver`                 |
| **混合模式** | ✅ 有      | ✅ 有     | 需要统一身份 + 专业方法 | `cfo-financial-clerk`          |

#### 专家驱动模式（最常用）

| 层次    | 示例                             | 说明                 |
| ------- | -------------------------------- | -------------------- |
| 专家    | `accounting-financial-clerk`     | 财务专员，方法论定义 |
| Profile | `opc-accounting-financial-clerk` | 仅绑定专家，无角色   |

#### 混合模式（需要统一身份入口时）

| 层次    | 示例                                 | 说明                 |
| ------- | ------------------------------------ | -------------------- |
| 角色    | `cfo`                                | CFO，统一身份入口    |
| 专家    | `accounting-financial-clerk`         | 财务专员，方法论定义 |
| Profile | `opc-cfo-accounting-financial-clerk` | 角色 × 专家的组合    |

### 3.3 工作流模板结构

每个工作流 seed 文件结构（参考 `seed_stock_analysis.rs`）：

#### 专家驱动型工作流（最常见）

```rust
pub(crate) async fn seed_industry_accounting_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    // 1. 定义节点（agent_profile_id 指向仅专家的 Profile）
    let nodes = vec![
        make_agent("clerk", "财务专员", "opc-accounting-financial-clerk", "..."),
        make_agent("approver", "财务审批人", "opc-accounting-financial-approver", "..."),
        // ...
    ];

    // 2. 定义边
    let edges = vec![
        WorkflowEdge { from: "start", to: "clerk", edge_type: EdgeType::Data, .. },
        WorkflowEdge { from: "clerk", to: "approver", edge_type: EdgeType::Data, .. },
        // ...
    ];

    // 3. 种子化到数据库
    // ...
}
```

#### 岗位驱动型工作流（审批场景）

```rust
// 示例：总经理审批工作流
let nodes = vec![
    make_agent("submitter", "提交人", "opc_financial_clerk", "..."),  // 岗位驱动
    make_agent("approver", "审批人", "opc_approver", "..."),          // 岗位驱动
    // ...
];
```

---

## 4. 实施步骤

### 阶段一：基础资源创建（预计 2 小时）

#### 步骤 1.1：创建业务负责人角色（按实际需要）

仅为需要统一身份入口的行业/领域创建业务负责人角色。

**创建原则**：

- ✅ 财务行业 → `cfo`（CFO，已有）
- ✅ 金融投资 → `cfo`（复用）
- ✅ 安全合规 → `opc_approver`（审批岗位，已有）
- ✅ 项目管理 → `opc_project_manager`（项目经理，已有）
- ❌ 设计行业 → 不创建（纯执行类）
- ❌ 软件开发 → 不创建（纯执行类）
- ❌ 教育培训 → 不创建（纯执行类）

**状态**：🔄 进行中（已添加审批类岗位 `opc_approver`、`opc_reviewer`、`opc_executor`）

#### 步骤 1.2：创建行业专家 .md 文件

为 12 个缺少专家的行业创建 .md 文件，每个工作流步骤对应一个专家。

**状态**：✅ 已完成（43 个文件）

#### 步骤 1.3：创建领域专家 .md 文件

为 17 个领域的 75 个工作流创建专家 .md 文件。

**状态**：⏳ 待实施

#### 步骤 1.4：更新种子化逻辑

修改 `opc_setup/industry_agents.rs` 和新增 `domain_agents.rs`。

**状态**：部分完成

---

### 阶段二：工作流 Seed 文件创建（预计 8 小时）

#### 步骤 2.1：创建 14 个行业工作流 seed 文件

为每个行业创建独立的 seed 文件，手动定义 `WorkflowNode/Edge`。

| 文件                                   | 工作流数 | 节点数 |
| -------------------------------------- | -------- | ------ |
| `seed_industry_accounting.rs`          | 1        | 4      |
| `seed_industry_ai_research.rs`         | 1        | 4      |
| `seed_industry_content_media.rs`       | 1        | 5      |
| `seed_industry_finance_invest.rs`      | 1        | 5      |
| `seed_industry_game_dev.rs`            | 1        | 5      |
| `seed_industry_design.rs`              | 1        | 3      |
| `seed_industry_ecommerce.rs`           | 1        | 3      |
| `seed_industry_education.rs`           | 1        | 3      |
| `seed_industry_geospatial.rs`          | 1        | 3      |
| `seed_industry_industry_consulting.rs` | 1        | 3      |
| `seed_industry_project_management.rs`  | 1        | 3      |
| `seed_industry_sales_growth.rs`        | 1        | 3      |
| `seed_industry_security.rs`            | 1        | 3      |
| `seed_industry_software_dev.rs`        | 1        | 3      |
| **合计**                               | **14**   | **50** |

#### 步骤 2.2：创建 17 个领域工作流 seed 文件

为每个领域创建独立的 seed 文件，包含所有子工作流。

| 文件                         | 工作流数 |
| ---------------------------- | -------- |
| `seed_domain_academic.rs`    | 2        |
| `seed_domain_design.rs`      | 4        |
| `seed_domain_engineering.rs` | 13       |
| `seed_domain_finance.rs`     | 3        |
| `seed_domain_gamedev.rs`     | 3        |
| `seed_domain_gis.rs`         | 4        |
| `seed_domain_marketing.rs`   | 10       |
| `seed_domain_paidmedia.rs`   | 2        |
| `seed_domain_pm.rs`          | 3        |
| `seed_domain_product.rs`     | 3        |
| `seed_domain_sales.rs`       | 5        |
| `seed_domain_security.rs`    | 4        |
| `seed_domain_spatial.rs`     | 2        |
| `seed_domain_specialized.rs` | 10       |
| `seed_domain_strategy.rs`    | 2        |
| `seed_domain_support.rs`     | 3        |
| `seed_domain_testing.rs`     | 3        |
| **合计**                     | **75**   |

#### 步骤 2.3：更新种子化调度

修改 `opc_workflows/mod.rs`，注册所有新的 seed 函数。

---

### 阶段三：清理旧代码（预计 2 小时）

#### 步骤 3.1：删除行业适配器

删除以下文件/模块：

- `crates/analysis-engine/src/opc/industry/adapter.rs`（如果存在）
- 移除 `OpcIndustryAdapter` trait

#### 步骤 3.2：删除领域生成器

删除以下文件/模块：

- `crates/analysis-engine/src/opc/domain/generator.rs`（如果存在）
- 移除 `DomainWorkflowGenerator`

#### 步骤 3.3：更新引用

更新所有引用旧代码的地方。

---

### 阶段四：验证与测试（预计 4 小时）

#### 步骤 4.1：编译验证

```bash
cd src-tauri && cargo build
cargo clippy -- -D warnings
```

#### 步骤 4.2：种子化测试

运行 OPC 设置命令，验证所有资源正确创建。

#### 步骤 4.3：工作流执行测试

选取典型工作流（如会计工作流、文献综述工作流），测试完整执行流程。

---

## 5. 风险与注意事项

### 5.1 向后兼容

- **数据库迁移**：旧的 `workflow_template` 数据需要清理或迁移
- **API 兼容性**：确保前端调用不受影响

### 5.2 性能影响

- **种子化时间**：70+ 工作流的种子化可能需要较长时间（预计 30-60 秒）
- **启动时间**：首次启动时种子化会增加启动时间

### 5.3 维护成本

- **新增工作流**：需要手动创建 seed 文件
- **修改工作流**：直接修改 seed 文件，无需理解适配器逻辑

---

## 6. 执行优先级

### P0 - 立即执行

1. 🔄 创建业务负责人角色（按实际需要）
2. ✅ 创建行业专家 .md 文件
3. 创建 14 个行业工作流 seed 文件
4. 更新行业工作流种子化逻辑

### P1 - 高优先级

5. 创建领域专家 .md 文件
6. 创建 17 个领域工作流 seed 文件
7. 更新领域工作流种子化逻辑

### P2 - 中优先级

8. 删除旧的适配器和生成器代码
9. 更新文档

### P3 - 低优先级

10. 性能优化
11. 代码清理

---

## 7. 验收标准

### 功能验收

- [ ] 14 个行业工作流可正常种子化和执行
- [ ] 75 个领域工作流可正常种子化和执行
- [ ] 所有工作流节点都有 `agent_profile_id`
- [ ] 所有专家都有对应的 .md 文件

### 代码验收

- [ ] 无 `OpcIndustryAdapter` 引用
- [ ] 无 `DomainWorkflowGenerator` 引用
- [ ] `cargo clippy -- -D warnings` 通过
- [ ] 所有 seed 文件手动定义 `WorkflowNode/Edge`

### 质量验收

- [ ] 工作流执行结果与重构前一致
- [ ] 专家提示词质量不下降
- [ ] 无回归 Bug

---

## 附录

### A. 股票分析工作流参考文件

- [seed_stock_analysis.rs](file:///d:/OneManager/AxInvest/src-tauri/src/commands/stock_analysis_setup/seed_stock_analysis.rs)
- [stock_analysis_setup/mod.rs](file:///d:/OneManager/AxInvest/src-tauri/src/commands/stock_analysis_setup/mod.rs)
- [agency_experts/stock-analysis/](file:///d:/OneManager/AxInvest/src-tauri/agency_experts/stock-analysis/)

### B. 旧代码位置

- [crates/analysis-engine/src/opc/industry/](file:///d:/OneManager/AxInvest/src-tauri/crates/analysis-engine/src/opc/industry/)
- [crates/analysis-engine/src/opc/domain/](file:///d:/OneManager/AxInvest/src-tauri/crates/analysis-engine/src/opc/domain/)

### C. 进度追踪

| 阶段 | 任务                                 | 状态 | 完成时间   |
| ---- | ------------------------------------ | ---- | ---------- |
| 一   | 1.1 创建业务负责人角色（按实际需要） | 🔄   | 2026-08-12 |
| 一   | 1.2 创建行业专家 .md                 | ✅   | 2026-08-12 |
| 一   | 1.3 创建领域专家 .md                 | ⏳   | -          |
| 一   | 1.4 更新种子化逻辑                   | 🔄   | -          |
| 二   | 2.1 行业 seed 文件                   | ⏳   | -          |
| 二   | 2.2 领域 seed 文件                   | ⏳   | -          |
| 二   | 2.3 更新调度                         | ⏳   | -          |
| 三   | 3.1 删除适配器                       | ⏳   | -          |
| 三   | 3.2 删除生成器                       | ⏳   | -          |
| 三   | 3.3 更新引用                         | ⏳   | -          |
| 四   | 4.1-4.3 验证                         | ⏳   | -          |
