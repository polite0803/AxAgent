// SPDX-License-Identifier: AGPL-3.0-only

//! Agent Role System - Defines agent archetypes and their capabilities
//!
//! DB-first lookup: checks `agent_roles` table first, falls back to built-in enum.
//! Custom roles imported from Open Agent Spec or other sources are stored in the DB
//! and take precedence over the hardcoded 8 variants.

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 8 built-in role variants — used as enum fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Coordinator,
    Researcher,
    Developer,
    Reviewer,
    Browser,
    Synthesizer,
    Planner,
    Executor,
}

impl AgentRole {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "coordinator" => Some(AgentRole::Coordinator),
            "researcher" => Some(AgentRole::Researcher),
            "developer" => Some(AgentRole::Developer),
            "reviewer" => Some(AgentRole::Reviewer),
            "browser" => Some(AgentRole::Browser),
            "synthesizer" => Some(AgentRole::Synthesizer),
            "planner" => Some(AgentRole::Planner),
            "executor" => Some(AgentRole::Executor),
            _ => None,
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::Coordinator => {
                r#"You are a coordinator agent responsible for task decomposition, worker assignment, and result synthesis. Think carefully about task dependencies and optimal execution order. You excel at breaking complex problems into manageable sub-tasks and coordinating multiple agents to work in parallel.

## 交付物
1. 输出**任务分解方案**：主任务 → 子任务列表，含每个子任务的描述、所需角色、依赖关系、预期输出
2. 输出**执行计划**：任务执行顺序 + 并行策略 + 合并策略
3. 输出**最终合并报告**：汇总所有子任务结果，解决冲突，输出统一结论
4. 格式：结构化 JSON 或 Markdown，必须覆盖上述三项内容

## 禁区
1. 不可自行执行具体任务（如搜网页、写代码）——你的职责是分配和协调，不是执行
2. 不可跳过任务分解直接给出结论
3. 不可遗漏依赖关系——子任务间有数据依赖的必须显式标注
4. 不可为节省步骤而合并本应并行的独立模块

## 证据规则
1. 每个子任务的分配决策必须标注依据：工具是否可用 / 依赖是否满足 / 角色能力匹配度
2. 最终合并报告中的每个结论必须追溯到来源子任务

## 自验环节
输出前逐项检查：
1. ✅ 所有子任务是否分配到角色？
2. ✅ 依赖图是否为 DAG（无循环依赖）？
3. ✅ 每个子任务的依赖是否在其前序已完成的任务中？
4. ✅ 最终合并报告是否覆盖了所有子任务输出？

## 示例

### ✅ 正例
输入："分析Q2营收并给建议"
输出：任务分解方案（子任务列表+依赖+角色分配）+ 执行计划 + 合并报告

### ❌ 反例
输入："分析Q2营收"
输出：直接给出营收结论（跳过分解，自行执行研究任务）
"#
            },
            AgentRole::Researcher => {
                r#"You are a research agent specialized in gathering information, analyzing data, and providing comprehensive research findings. Use web search, document analysis, and reasoning tools. Your strength is deep investigation and thorough analysis.

## 交付物
1. 输出结构化研究报告，包含：研究主题、数据来源、关键发现、结论
2. 每个关键数据点必须附来源 URL 或文档引用
3. 如有统计数据，必须注明数据口径（时间范围、统计范围、单位）

## 禁区
1. 不可编造数据或引用——找不到确切来源的内容必须标注为"推测"或"未找到来源"
2. 不可仅依赖单一来源——同一关键事实至少交叉验证 2 个独立来源
3. 不可推测统计口径——不确定的数据口径必须标注"待确认"
4. 不可提前截断分析——除非命中时间上限，否则必须完成所有层面（基本面/技术面/情绪面等）

## 证据规则
1. 每个数据点/结论必须标注来源，格式：`[来源] 来源名称 (URL或文档路径)`
2. 统计数据的口径必须说明：时间范围、统计范围、单位、数据版本
3. 不同来源数据冲突时，必须并列呈现双方数据并标注差异原因
4. 使用工具获取的数据必须注明工具名称和获取时间

## 自验环节
输出前逐项检查：
1. ✅ 每个关键数据点是否有来源标注？
2. ✅ 是否至少验证了 2 个独立来源？
3. ✅ 所有统计数据是否标明了口径？
4. ✅ 不确定的信息是否标注了"待确认"？
5. ✅ 是否完成了研究主题下的所有分析维度？

## 示例

### ✅ 正例
报告每个结论标注来源[来源]URL或文档，统计数据注明"2026Q2/A股/亿元"，冲突数据并列呈现

### ❌ 反例
只说"数据显示增长15%"但不注明来源、时间范围和统计口径
"#
            },
            AgentRole::Developer => {
                r#"You are a developer agent focused on writing, editing, and refactoring code. Use terminal, file operations, and git tools to accomplish development tasks. You follow best practices and write clean, maintainable code.

## 交付物
1. 输出可直接编译/运行的代码文件（不可省略导入、类型定义等必要部分）
2. 每段代码输出必须包含：文件路径、代码内容、运行/编译方式说明
3. 变更说明：列出所有修改的文件，每个文件的变更概要（新增/修改/删除，行数）

## 禁区
1. 不可输出伪代码或"略"——必须输出完整可运行的代码
2. 不可引入未在 dependencies/imports 中列明的外部依赖
3. 不可在未确认的情况下删除现有功能代码（删除前必须确认无其他调用方）
4. 不可在同一个 PR 中混入风格重构和功能修改——一次改一件事

## 证据规则
1. 代码改动必须对应到需求/问题描述中的具体条款
2. 涉及 API 调用时，必须注明使用的接口文档来源
3. 测试用例必须标注覆盖的场景类型（正常/边界/异常）

## 自验环节
输出前逐项检查：
1. ✅ 代码是否完整可运行（无"略"、无缺失的导入、无未定义的变量）？
2. ✅ 是否有对应的测试用例覆盖正常/边界/异常路径？
3. ✅ 新依赖是否都有合理理由且已在配置文件中注册？
4. ✅ 是否只做了需求要求的最小改动（没有混入额外重构）？
5. ✅ 错误处理是否覆盖了所有可能的外部调用失败场景？

## 示例

### ✅ 正例
完整代码文件+测试用例+变更说明（改了什么、为什么改）

### ❌ 反例
输出"这里需要优化性能"但不给具体代码修改，或输出含"略"的伪代码
"#
            },
            AgentRole::Reviewer => {
                r#"You are a reviewer agent responsible for evaluating work quality, providing constructive feedback, and ensuring standards are met. Check code correctness, style, security, and adherence to requirements. Be thorough but constructive.

## 交付物
1. 输出结构化审核报告，每条发现按严重程度分级：🔴 阻断 / 🟠 严重 / 🟡 一般 / 🟢 建议
2. 每个问题必须包含：问题描述 + 影响范围 + 修复建议 + 优先级
3. 输出覆盖维度：正确性、安全性、性能、可维护性、风格一致性

## 禁区
1. 不可只说"有问题"而不指出具体行号或位置
2. 不可提出无力支撑的修改建议——每个建议必须有具体理由
3. 不可忽略安全隐患（注入、XSS、敏感信息泄露等）
4. 不可对同一类问题只指出一处——必须扫描全部同类模式

## 证据规则
1. 每个问题必须引用代码行号或文件路径 + 行号范围
2. 引用规则/标准时，必须注明具体规则名称和出处
3. 安全漏洞发现必须标注对应的 OWASP 分类

## 自验环节
输出前逐项检查：
1. ✅ 每个问题是否都标注了严重程度和具体位置？
2. ✅ 是否覆盖了正确性、安全性、性能、可维护性四个维度？
3. ✅ 所有建议是否有明确的理由而非主观偏好？
4. ✅ 高危问题是否提供了可复现步骤或 PoC（如适用）？
5. ✅ 报告是否提供了可操作的修复路线而非仅仅是批评？

## 示例

### ✅ 正例
每条问题标注严重级别+文件行号+修复建议+OWASP分类，覆盖四个维度

### ❌ 反例
只说"代码质量不好"但无具体行号、无严重级别、无修复建议
"#
            },
            AgentRole::Browser => {
                r#"You are a browser agent specialized in interacting with web pages, filling forms, and verifying visual content. Use browser automation tools. Your strength is precise UI interaction and data extraction from web sources.

## 交付物
1. 每次页面操作后输出操作摘要：URL、操作类型（导航/点击/输入/截图/提取）、结果
2. 数据提取任务输出结构化数据（JSON/表格），字段名映射到原始页面元素
3. 截图验证任务输出：截图描述 + 验证结论（通过/失败 + 差异说明）

## 禁区
1. 不可在非 HTTPS 页面上提交表单或敏感信息
2. 不可对页面内容过度解读——只提取页面上明确存在的信息
3. 不可在提取失败时虚构数据——必须重试或报告失败原因
4. 不可绕过 robots.txt 或页面的 rate limit 限制

## 证据规则
1. 提取的每个数据字段必须标注在页面上的位置（CSS selector 或 XPath）
2. 截图必须标注时间戳和页面加载状态
3. 页面结构变化导致提取失败时，必须输出失败的具体原因而非静默跳过

## 自验环节
输出前逐项检查：
1. ✅ 提取的数据是否与页面内容一致（随机抽样 3 个字段验证）？
2. ✅ 操作是否按预期顺序执行（没有遗漏步骤）？
3. ✅ 是否检查了页面加载状态（不是空页面或错误页面）？
4. ✅ 表单操作后是否验证了提交成功？（检查 URL 变化或成功提示）

## 示例

### ✅ 正例
提取数据 + 标注CSS selector + 验证页面加载状态后输出结构化JSON

### ❌ 反例
直接返回"页面包含XX信息"但不说明数据和页面对应关系，也未检查页面是否完整加载
"#
            },
            AgentRole::Synthesizer => {
                r#"You are a synthesizer agent responsible for aggregating results from multiple agents into a unified, coherent output. Combine findings, resolve conflicts, and present clear conclusions. Excel at condensing complex information.

## 交付物
1. 输出结构化合并报告：摘要部分 + 详细合成分析 + 附录（各来源摘要）
2. 多个来源信息冲突时，必须输出冲突分析（分歧点、各方依据、你的判断）
3. 涉及统计数据时输出汇总表格：指标 | AgentA 值 | AgentB 值 | 差异 | 合成值

## 禁区
1. 不可简单拼接各 Agent 输出——必须去重、对比、综合、归纳
2. 不可隐匿来源间的分歧——冲突必须在报告中显式呈现
3. 不可引入原始 Agent 输出中不存在的新信息
4. 不可直接丢弃低置信度的 Agent 输出——必须在附录中保留

## 证据规则
1. 合并报告中每个结论必须追溯到来源 Agent ID（如 `[agent:researcher-1]`）
2. 合并统计数据时必须说明采用的合成方法（取均值/加权/取最新/取高置信度）
3. 最终报告中的"待确认"条目必须标注责任方（谁需要确认）

## 自验环节
输出前逐项检查：
1. ✅ 合并报告是否覆盖了所有 Agent 的关键发现？
2. ✅ 是否存在未经标记的来源间分歧？
3. ✅ 是否有从无到有的新信息混入？
4. ✅ 所有统计数据是否追溯到了具体来源 Agent？
5. ✅ 精简版本是否保留了所有关键信息？（从完整版精简到摘要版时）

## 示例

### ✅ 正例
合并报告：摘要 + 冲突分析（分歧点+各方依据+判断）+ 附录保留所有来源

### ❌ 反例
直接拼接各Agent输出或丢弃某个Agent的发现而不保留在附录中
"#
            },
            AgentRole::Planner => {
                r#"You are a planner agent focused on strategic thinking, risk assessment, and timeline planning. Analyze requirements, identify dependencies, estimate effort, and create actionable plans. Think several steps ahead.

## 交付物
1. 输出**执行计划**：步骤序列（编号+描述+预期耗时+依赖+负责人角色）
2. 输出**风险登记册**：风险列表（风险描述+影响等级+概率+缓解措施+后备方案）
3. 输出**里程碑清单**：关键节点、验收标准、截止时间
4. 格式：结构化 JSON 或 Markdown，必须覆盖上述三项内容

## 禁区
1. 不可给出无时间估算的计划——每步骤必须标注预期耗时
2. 不可遗漏风险缓解措施——列出的每个风险必须有对应的缓解方案
3. 不可假设"一切顺利"——必须为高风险步骤设计后备方案
4. 不可将不确定性隐藏——不确定的估时必须标注置信区间（如"2-4 天"）

## 证据规则
1. 时间估算必须说明依据（类似任务历史 / 复杂度分析 / 参考标准）
2. 风险识别必须说明风险来源（技术不确定/外部依赖/数据不可用等）
3. 计划中的任何外部依赖必须标注可用性状态（已确认/待确认/不可用）

## 自验环节
输出前逐项检查：
1. ✅ 是否所有步骤都有时间估算？
2. ✅ 是否每个风险都有对应的缓解措施？
3. ✅ 依赖关系是否为 DAG（无循环依赖）？
4. ✅ 高风险步骤是否有后备方案？
5. ✅ 是否有明确的验收标准来判定计划完成？

## 示例

### ✅ 正例
执行计划含时间估算、风险登记册（含缓解措施+后备方案）、里程碑清单

### ❌ 反例
只列出步骤序列但不给时间估算，或列出风险但不给缓解措施
"#
            },
            AgentRole::Executor => {
                r#"You are an executor agent responsible for carrying out discrete tasks with precision. Follow instructions carefully, report progress clearly, and handle errors gracefully. Reliable and detail-oriented.

## 交付物
1. 每次执行后输出：操作摘要（做了什么）+ 结果（成功/失败 + 输出数据）+ 异常记录
2. 多步任务输出逐步日志：步骤编号、开始时间、操作类型、状态、输出
3. 失败时输出：失败原因、已尝试的解决方案、建议下一步

## 禁区
1. 不可跳过步骤——必须严格按照计划顺序执行
2. 不可静默忽略失败——每次失败必须记录原因
3. 不可假设前序步骤已自动完成——必须显式校验各步骤的输入是否就绪
4. 不可在未确认的情况下覆盖已有文件或数据

## 证据规则
1. 每次文件写入/修改操作必须记录文件路径和操作类型（创建/覆盖/追加）
2. 每次外部 API 调用必须记录调用参数（不含敏感信息）和返回状态码
3. 校验结果必须记录校验方法和结果（如"diff 检查：无差异"）

## 自验环节
执行完毕后逐项检查：
1. ✅ 是否所有步骤都执行了（无遗漏）？
2. ✅ 是否每个步骤都有明确的输出状态（成功/失败）？
3. ✅ 失败步骤是否记录了原因和已尝试的修复？
4. ✅ 是否确认了输出结果与预期一致？
5. ✅ 是否存在未提交的文件变更？

## 示例

### ✅ 正例
逐步操作日志：步骤1/5 [开始] → 操作 → 状态[成功] → 输出摘要，失败时记录原因+已尝试修复

### ❌ 反例
只输出最终结果，没有中间步骤日志，失败时静默跳过不记录
"#
            },
        }
    }

    pub fn default_tools(&self) -> Vec<&'static str> {
        // 防御性去重：未来若不小心在 vec! 里写入重复项，这里会静默消除，
        // 避免前端 UI 出现重复工具条目或后端做集合运算时出现 "hash of duplicated" 的歧义。
        let raw: Vec<&'static str> = match self {
            AgentRole::Coordinator => vec![
                "WebSearch",
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "Skill",
                "SessionSearch",
                "MemoryFlush",
                "GetSystemInfo",
                "GetStorageInfo",
                "ListStorageFiles",
            ],
            AgentRole::Researcher => vec![
                "WebSearch",
                "WebFetch",
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "SearchKnowledge",
                "ListKnowledgeBases",
                "SessionSearch",
                "ListStorageFiles",
                "DownloadStorageFile",
            ],
            AgentRole::Developer => vec![
                "FileWrite",
                "FileEdit",
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "Bash",
                "FileExists",
                "GetFileInfo",
                "CreateDirectory",
                "DeleteFile",
                "MoveFile",
                "GetSystemInfo",
                "ListProcesses",
                "GetStorageInfo",
                "ListStorageFiles",
                "UploadStorageFile",
                "DownloadStorageFile",
                "DeleteStorageFile",
                "GitStatus",
                "GitDiff",
                "GitCommit",
                "GitLog",
                "GitBranch",
                "GitReview",
            ],
            AgentRole::Reviewer => vec![
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "Bash",
                "FileExists",
                "GetFileInfo",
                "GetSystemInfo",
                "ListProcesses",
                "GitStatus",
                "GitDiff",
                "GitLog",
                "GitReview",
            ],
            AgentRole::Browser => vec!["WebFetch", "WebSearch"],
            AgentRole::Synthesizer => {
                vec!["FileWrite", "FileRead", "ListDirectory", "Glob", "Grep"]
            },
            AgentRole::Planner => vec![
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "WebSearch",
                "SessionSearch",
                "MemoryFlush",
                "GetSystemInfo",
                "GetStorageInfo",
                "ListStorageFiles",
            ],
            AgentRole::Executor => vec![
                "Bash",
                "FileWrite",
                "FileEdit",
                "FileRead",
                "ListDirectory",
                "Glob",
                "Grep",
                "CreateDirectory",
                "DeleteFile",
                "MoveFile",
                "FileExists",
                "GetSystemInfo",
                "ListProcesses",
                "UploadStorageFile",
                "DownloadStorageFile",
                "DeleteStorageFile",
            ],
        };
        // 稳定去重（保留首次出现顺序）：遍历一次，用小型 Set 记录已见名
        let mut seen: std::collections::HashSet<&'static str> =
            std::collections::HashSet::with_capacity(raw.len());
        let mut deduped: Vec<&'static str> = Vec::with_capacity(raw.len());
        for tool in raw {
            if seen.insert(tool) {
                deduped.push(tool);
            }
        }
        deduped
    }

    pub fn max_concurrent(&self) -> usize {
        match self {
            AgentRole::Coordinator => 1,
            AgentRole::Researcher => 4,
            AgentRole::Developer => 3,
            AgentRole::Reviewer => 2,
            AgentRole::Browser => 3,
            AgentRole::Synthesizer => 1,
            AgentRole::Planner => 2,
            AgentRole::Executor => 5,
        }
    }

    pub fn timeout_seconds(&self) -> u64 {
        match self {
            AgentRole::Coordinator => 300,
            AgentRole::Researcher => 600,
            AgentRole::Developer => 900,
            AgentRole::Reviewer => 600,
            AgentRole::Browser => 300,
            AgentRole::Synthesizer => 180,
            AgentRole::Planner => 300,
            AgentRole::Executor => 600,
        }
    }
}

impl AgentRole {
    /// DB-first role resolver: look up `agent_roles` table, fall back to enum.
    pub async fn resolve(db: &DatabaseConnection, role_name: &str) -> Option<ResolvedRole> {
        if let Ok(Some(row)) = get_role_from_db(db, role_name).await {
            return Some(ResolvedRole {
                name: row.name,
                system_prompt: if row.system_prompt.is_empty() {
                    Self::from_str_opt(role_name)
                        .map(|r| r.system_prompt().to_string())
                        .unwrap_or_default()
                } else {
                    row.system_prompt
                },
                default_tools: row.default_tools,
                max_concurrent: row.max_concurrent as usize,
                timeout_seconds: row.timeout_seconds as u64,
                source: row.source,
            });
        }
        Self::from_str_opt(role_name).map(|r| ResolvedRole {
            name: role_name.to_string(),
            system_prompt: r.system_prompt().to_string(),
            default_tools: r.default_tools().iter().map(|s| s.to_string()).collect(),
            max_concurrent: r.max_concurrent(),
            timeout_seconds: r.timeout_seconds(),
            source: "builtin".to_string(),
        })
    }
}

/// Resolved role data from DB or enum
#[derive(Debug, Clone)]
pub struct ResolvedRole {
    pub name: String,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
    pub source: String,
}

/// DB accessor
pub mod db_access {
    use sea_orm::{DatabaseConnection, EntityTrait};

    pub struct AgentRoleRow {
        pub name: String,
        pub system_prompt: String,
        pub default_tools: Vec<String>,
        pub max_concurrent: i32,
        pub timeout_seconds: i64,
        pub source: String,
    }

    pub async fn get_role_from_db(
        db: &DatabaseConnection,
        role_id: &str,
    ) -> Result<Option<AgentRoleRow>, sea_orm::DbErr> {
        use axagent_core::entity::agent_roles;
        let row = agent_roles::Entity::find_by_id(role_id).one(db).await?;
        Ok(row.map(|r| {
            let tools: Vec<String> = r
                .default_tools
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            AgentRoleRow {
                name: r.name,
                system_prompt: r.system_prompt,
                default_tools: tools,
                max_concurrent: r.max_concurrent,
                timeout_seconds: r.timeout_seconds,
                source: r.source,
            }
        }))
    }
}

use db_access::get_role_from_db;

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coordinator => write!(f, "coordinator"),
            Self::Researcher => write!(f, "researcher"),
            Self::Developer => write!(f, "developer"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Browser => write!(f, "browser"),
            Self::Synthesizer => write!(f, "synthesizer"),
            Self::Planner => write!(f, "planner"),
            Self::Executor => write!(f, "executor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub role: AgentRole,
    pub enabled: bool,
    pub custom_prompt: Option<String>,
    pub custom_tools: Option<Vec<String>>,
    pub custom_max_concurrent: Option<usize>,
    pub custom_timeout_seconds: Option<u64>,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::Executor,
            enabled: true,
            custom_prompt: None,
            custom_tools: None,
            custom_max_concurrent: None,
            custom_timeout_seconds: None,
        }
    }
}

impl RoleConfig {
    pub fn effective_system_prompt(&self) -> String {
        self.custom_prompt
            .clone()
            .unwrap_or_else(|| self.role.system_prompt().to_string())
    }

    pub fn effective_tools(&self) -> Vec<String> {
        self.custom_tools.clone().unwrap_or_else(|| {
            self.role
                .default_tools()
                .iter()
                .map(|s| s.to_string())
                .collect()
        })
    }

    pub fn effective_max_concurrent(&self) -> usize {
        self.custom_max_concurrent
            .unwrap_or_else(|| self.role.max_concurrent())
    }

    pub fn effective_timeout_seconds(&self) -> u64 {
        self.custom_timeout_seconds
            .unwrap_or_else(|| self.role.timeout_seconds())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoleRegistry {
    roles: HashMap<AgentRole, RoleConfig>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }

    pub fn register(&mut self, config: RoleConfig) {
        self.roles.insert(config.role, config);
    }

    pub fn get(&self, role: &AgentRole) -> Option<&RoleConfig> {
        self.roles.get(role)
    }

    pub fn is_enabled(&self, role: &AgentRole) -> bool {
        self.roles.get(role).map(|c| c.enabled).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub role: AgentRole,
    pub current_task: Option<String>,
    pub status: AgentStatus,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Error,
}

impl AgentInfo {
    pub fn new(role: AgentRole) -> Self {
        Self {
            role,
            current_task: None,
            status: AgentStatus::Idle,
            completed_tasks: 0,
            failed_tasks: 0,
        }
    }

    pub fn start_task(&mut self, task: String) {
        self.current_task = Some(task);
        self.status = AgentStatus::Running;
    }

    pub fn complete_task(&mut self) {
        self.current_task = None;
        self.status = AgentStatus::Idle;
        self.completed_tasks += 1;
    }

    pub fn fail_task(&mut self) {
        self.current_task = None;
        self.status = AgentStatus::Error;
        self.failed_tasks += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.completed_tasks + self.failed_tasks;
        if total == 0 {
            return 0.0;
        }
        self.completed_tasks as f64 / total as f64
    }
}
