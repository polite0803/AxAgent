// SPDX-License-Identifier: AGPL-3.0-only

//! 工程与开发（engineering）领域工作流种子化 — 13 个工作流（v4 丰富拓扑）
//!
//! 手动定义 WorkflowNode/Edge，与行业 seed 文件模式一致。
//!
//! 生成的工作流：
//! - wf-eng-api-design:     API设计（规格 → 设计校验分支 → 文档）
//! - wf-eng-arch-review:    架构评审（设计 → 评审分支 → 定稿/修订）
//! - wf-eng-ci-setup:       CI/CD配置（规划 → 配置 → 验证分支 → 修复/发布）
//! - wf-eng-code-review:    代码审查（提交 → 审查 → 严重问题分支 → 报告）
//! - wf-eng-db-migrate:     数据库迁移（规划 → 评审分支 → 执行/修订）
//! - wf-eng-deploy:         DevOps部署（构建 → 测试分支 → 部署/重建 → 验证）
//! - wf-eng-monitor-setup:  监控告警（规划 → 配置 → 告警验证分支）
//! - wf-eng-onboarding:     开发入职（环境 → 文档 → 就绪分支 → 任务/培训）
//! - wf-eng-perf-opt:       性能优化（剖析 → 瓶颈分支 → 优化/深度剖析）
//! - wf-eng-refactor-lite:  快速追加重构（评估 → 计划 → 执行 → 验证）
//! - wf-eng-refactor:       大型代码项目重构（42 节点全链路，保持精修版）
//! - wf-eng-security-review:安全审查（扫描 → 风险分支 → 修复）
//! - wf-eng-tech-debt:      技术债管理（扫描 → 优先级分支 → 偿还）

use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

use super::seed_domain_helpers::*;

/// engineering 领域模板版本（v4 丰富拓扑；wf-eng-refactor/refactor-lite 保持各自版本）
const ENG_TEMPLATE_VERSION: i32 = 4;

/// 种子化工与开发领域的全部工作流
pub(crate) async fn seed_domain_engineering_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── 1. wf-eng-api-design: API设计 ──────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-api-design",
            "API设计",
            "API设计：定义接口规格，设计校验不通过自动补充，输出完整 API 文档",
            "🔌",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：接口规格
                make_agent_node(
                    "a-spec",
                    "接口规格",
                    "定义 API 规格：资源模型、端点、方法、请求/响应结构、错误码。\
                     输出 JSON：{\"resources\":[{\"name\":\"\", \"endpoints\":[{\"method\":\"\", \"path\":\"\", \"params\":[], \"responses\":{}}]}], \"errors\":[], \"valid\":true, \"gaps\":[]}",
                    vec![td_desc("Grep", "检索现有代码中的接口定义"), td_desc("FileRead", "读取相关文件")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-spec",
                    0.0,
                    180.0,
                ),
                // 条件：规格有效性
                make_condition_node(
                    "c-api-valid",
                    "规格校验",
                    vec![Condition {
                        var_path: "a-spec.valid".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 补充分支：补齐规格
                make_agent_node_full(
                    "a-spec-fill",
                    "规格补充",
                    "规格存在缺口，补齐缺失的端点、参数与错误定义。\
                     输出 JSON：{\"filled\":[], \"valid\":true}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-spec-fill",
                    vec![("spec", "a-spec")],
                    vec!["a-spec"],
                    -250.0,
                    540.0,
                ),
                make_merge_node("m-api", "汇合", 0.0, 720.0),
                // Agent：API 文档
                make_agent_node_full(
                    "a-doc",
                    "API文档",
                    "生成完整 API 文档：示例、调用说明、版本策略、变更日志。\
                     输出 JSON：{\"docs\":\"\", \"examples\":[], \"versioning\":\"\", \"changelog\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-doc",
                    vec![("spec", "a-spec"), ("fill", "a-spec-fill")],
                    vec!["a-spec", "a-spec-fill"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-spec", "trigger", "a-spec"),
                edge("e-spec-valid", "a-spec", "c-api-valid"),
                edge_cond("e-invalid-fill", "c-api-valid", "false", "a-spec-fill", EdgeType::ConditionFalse),
                edge_cond("e-valid-merge", "c-api-valid", "true", "m-api", EdgeType::ConditionTrue),
                edge("e-fill-merge", "a-spec-fill", "m-api"),
                edge("e-merge-doc", "m-api", "a-doc"),
                edge("e-doc-end", "a-doc", "end"),
            ],
            vec![DomainInputField { key: "api_name", label: "API名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 2. wf-eng-arch-review: 架构评审 ────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-arch-review",
            "架构评审",
            "架构评审：设计方案，评审不通过自动修订，通过后定稿",
            "🏗️",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：方案设计
                make_agent_node(
                    "a-design",
                    "方案设计",
                    "设计方案：架构风格、模块划分、技术选型、数据流、扩展点。\
                     输出 JSON：{\"architecture\":\"\", \"modules\":[], \"tech_choices\":[{\"component\":\"\", \"choice\":\"\", \"rationale\":\"\"}], \"tradeoffs\":[]}",
                    vec![td_desc("Grep", "检索现有架构"), td_desc("FileRead", "读取关键文件")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-design",
                    0.0,
                    180.0,
                ),
                // Agent：架构评审
                make_agent_node_full(
                    "a-review-arch",
                    "架构评审",
                    "评审架构：一致性、可扩展性、性能、安全、可维护性、风险。\
                     输出 JSON：{\"passed\":false, \"issues\":[{\"area\":\"\", \"severity\":\"\", \"description\":\"\", \"suggestion\":\"\"}], \"score\":0}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-review-arch",
                    vec![("design", "a-design")],
                    vec!["a-design"],
                    0.0,
                    360.0,
                ),
                // 条件：评审是否通过
                make_condition_node(
                    "c-arch-passed",
                    "评审判定",
                    vec![Condition {
                        var_path: "a-review-arch.passed".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 修订分支
                make_agent_node_full(
                    "a-revise",
                    "方案修订",
                    "按评审意见修订方案，消除阻塞性问题。\
                     输出 JSON：{\"revisions\":[], \"passed\":true}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-revise",
                    vec![("review", "a-review-arch"), ("design", "a-design")],
                    vec!["a-review-arch", "a-design"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-arch", "汇合", 0.0, 900.0),
                // Agent：定稿
                make_agent_node_full(
                    "a-finalize",
                    "定稿",
                    "架构方案定稿：版本、决策记录、实施路线图。\
                     输出 JSON：{\"final\":\"\", \"adr\":[], \"roadmap\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-finalize",
                    vec![("review", "a-review-arch"), ("revise", "a-revise")],
                    vec!["a-review-arch", "a-revise"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-design", "trigger", "a-design"),
                edge("e-design-review", "a-design", "a-review-arch"),
                edge("e-review-passed", "a-review-arch", "c-arch-passed"),
                edge_cond("e-fail-revise", "c-arch-passed", "false", "a-revise", EdgeType::ConditionFalse),
                edge_cond("e-pass-merge", "c-arch-passed", "true", "m-arch", EdgeType::ConditionTrue),
                edge("e-revise-merge", "a-revise", "m-arch"),
                edge("e-merge-finalize", "m-arch", "a-finalize"),
                edge("e-finalize-end", "a-finalize", "end"),
            ],
            vec![DomainInputField { key: "arch_scope", label: "架构范围", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 3. wf-eng-ci-setup: CI/CD配置 ──────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-ci-setup",
            "CI/CD配置",
            "CI/CD配置：规划流水线，配置验证不通过自动修复，发布验证结果",
            "🔧",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：CI 规划
                make_agent_node(
                    "a-ci-plan",
                    "CI规划",
                    "规划 CI/CD 流水线：阶段划分、触发条件、环境、缓存策略、通知。\
                     输出 JSON：{\"stages\":[{\"name\":\"\", \"steps\":[], \"trigger\":\"\", \"env\":{}}], \"cache\":\"\", \"notifications\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ci-plan",
                    0.0,
                    180.0,
                ),
                // Agent：流水线配置
                make_agent_node_full(
                    "a-ci-config",
                    "流水线配置",
                    "编写流水线配置：lint、测试、构建、制品、部署步骤。\
                     输出 JSON：{\"config\":\"\", \"steps\":[], \"valid\":true, \"issues\":[]}",
                    vec![td_desc("FileRead", "读取现有配置文件"), td_desc("Grep", "检索相关配置")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ci-config",
                    vec![("plan", "a-ci-plan")],
                    vec!["a-ci-plan"],
                    0.0,
                    360.0,
                ),
                // 条件：配置验证
                make_condition_node(
                    "c-ci-valid",
                    "验证判定",
                    vec![Condition {
                        var_path: "a-ci-config.valid".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 修复分支
                make_agent_node_full(
                    "a-ci-fix",
                    "配置修复",
                    "修复流水线配置问题：语法、步骤依赖、环境变量。\
                     输出 JSON：{\"fixes\":[], \"valid\":true}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ci-fix",
                    vec![("config", "a-ci-config")],
                    vec!["a-ci-config"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-ci", "汇合", 0.0, 900.0),
                // Agent：验证发布
                make_agent_node_full(
                    "a-ci-verify",
                    "验证发布",
                    "验证流水线端到端运行并发布配置。\
                     输出 JSON：{\"verified\":true, \"run_report\":\"\", \"published\":true}",
                    vec![td_desc("Bash", "执行流水线验证命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ci-verify",
                    vec![("config", "a-ci-config"), ("fix", "a-ci-fix")],
                    vec!["a-ci-config", "a-ci-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-ci-plan"),
                edge("e-plan-config", "a-ci-plan", "a-ci-config"),
                edge("e-config-valid", "a-ci-config", "c-ci-valid"),
                edge_cond("e-invalid-fix", "c-ci-valid", "false", "a-ci-fix", EdgeType::ConditionFalse),
                edge_cond("e-valid-merge", "c-ci-valid", "true", "m-ci", EdgeType::ConditionTrue),
                edge("e-fix-merge", "a-ci-fix", "m-ci"),
                edge("e-merge-verify", "m-ci", "a-ci-verify"),
                edge("e-verify-end", "a-ci-verify", "end"),
            ],
            vec![DomainInputField { key: "repo_url", label: "仓库地址", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 4. wf-eng-code-review: 代码审查 ────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-code-review",
            "代码审查",
            "代码审查：提交变更，审查发现严重问题自动生成阻塞报告",
            "🔍",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：提交
                make_agent_node(
                    "a-submit",
                    "提交",
                    "整理代码变更提交：变更文件、diff 摘要、关联需求。\
                     输出 JSON：{\"files\":[], \"summary\":\"\", \"related\":\"\", \"diff_size\":0}",
                    vec![td_desc("Grep", "检索变更内容"), td_desc("FileRead", "读取变更文件")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-submit",
                    0.0,
                    180.0,
                ),
                // Agent：审查
                make_agent_node_full(
                    "a-review",
                    "审查",
                    "审查代码：逻辑正确性、边界处理、安全性、性能、规范。\
                     输出 JSON：{\"comments\":[{\"file\":\"\", \"line\":0, \"severity\":\"critical|major|minor\", \"issue\":\"\", \"suggestion\":\"\"}], \"critical_count\":0}",
                    vec![td_desc("Grep", "检索相关代码"), td_desc("FileRead", "读取上下文")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-review",
                    vec![("submit", "a-submit")],
                    vec!["a-submit"],
                    0.0,
                    360.0,
                ),
                // 条件：严重问题
                make_condition_node(
                    "c-review-critical",
                    "严重度判定",
                    vec![Condition {
                        var_path: "a-review.critical_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 严重分支：阻塞报告
                make_agent_node_full(
                    "a-block",
                    "阻塞报告",
                    "存在严重问题，生成阻塞报告：阻塞项、修复要求、复核流程。\
                     输出 JSON：{\"blockers\":[], \"fix_required\":[], \"review_again\":true}",
                    vec![td_desc("OpcSendNotification", "通知开发者阻塞问题")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-block",
                    vec![("review", "a-review")],
                    vec!["a-review"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-review", "汇合", 0.0, 900.0),
                // Agent：审查报告
                make_agent_node_full(
                    "a-report",
                    "审查报告",
                    "汇总审查结论：总体评价、通过状态、遗留建议。\
                     输出 JSON：{\"verdict\":\"approve|changes_requested|blocked\", \"summary\":\"\", \"recommendations\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-report",
                    vec![("review", "a-review"), ("block", "a-block")],
                    vec!["a-review", "a-block"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-submit", "trigger", "a-submit"),
                edge("e-submit-review", "a-submit", "a-review"),
                edge("e-review-critical", "a-review", "c-review-critical"),
                edge_cond("e-critical-block", "c-review-critical", "true", "a-block", EdgeType::ConditionTrue),
                edge_cond("e-clean-merge", "c-review-critical", "false", "m-review", EdgeType::ConditionFalse),
                edge("e-block-merge", "a-block", "m-review"),
                edge("e-merge-report", "m-review", "a-report"),
                edge("e-report-end", "a-report", "end"),
            ],
            vec![DomainInputField { key: "pr_id", label: "PR编号", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 5. wf-eng-db-migrate: 数据库迁移 ───────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-db-migrate",
            "数据库迁移",
            "数据库迁移：规划迁移方案，评审不通过自动修订，通过后执行并回滚预案",
            "🗄️",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：迁移规划
                make_agent_node(
                    "a-plan-migrate",
                    "迁移规划",
                    "规划数据库迁移：变更表结构、数据回填、索引、兼容性、回滚方案。\
                     输出 JSON：{\"changes\":[{\"table\":\"\", \"operation\":\"\", \"ddl\":\"\", \"data_backfill\":\"\"}], \"rollback\":\"\", \"risk\":\"\"}",
                    vec![td_desc("Grep", "检索现有模型定义"), td_desc("FileRead", "读取迁移文件")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-plan-migrate",
                    0.0,
                    180.0,
                ),
                // Agent：迁移评审
                make_agent_node_full(
                    "a-review-migrate",
                    "迁移评审",
                    "评审迁移方案：破坏性变更、数据安全、性能影响、回滚可行性。\
                     输出 JSON：{\"passed\":false, \"issues\":[], \"suggestions\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-review-migrate",
                    vec![("plan", "a-plan-migrate")],
                    vec!["a-plan-migrate"],
                    0.0,
                    360.0,
                ),
                // 条件：评审通过
                make_condition_node(
                    "c-migrate-passed",
                    "评审判定",
                    vec![Condition {
                        var_path: "a-review-migrate.passed".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 修订分支
                make_agent_node_full(
                    "a-revise-migrate",
                    "方案修订",
                    "按评审意见修订迁移方案。\
                     输出 JSON：{\"revisions\":[], \"passed\":true}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-revise-migrate",
                    vec![("review", "a-review-migrate"), ("plan", "a-plan-migrate")],
                    vec!["a-review-migrate", "a-plan-migrate"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-migrate", "汇合", 0.0, 900.0),
                // Agent：执行迁移
                make_agent_node_full(
                    "a-execute-migrate",
                    "执行迁移",
                    "执行迁移：应用 DDL、回填数据、验证结构、准备回滚。\
                     输出 JSON：{\"status\":\"success|rolled_back\", \"applied\":[], \"verify\":\"\", \"rollback_ready\":true}",
                    vec![td_desc("Bash", "执行迁移命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-execute-migrate",
                    vec![("plan", "a-plan-migrate"), ("revise", "a-revise-migrate")],
                    vec!["a-plan-migrate", "a-revise-migrate"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-plan-migrate"),
                edge("e-plan-review", "a-plan-migrate", "a-review-migrate"),
                edge("e-review-passed", "a-review-migrate", "c-migrate-passed"),
                edge_cond("e-fail-revise", "c-migrate-passed", "false", "a-revise-migrate", EdgeType::ConditionFalse),
                edge_cond("e-pass-merge", "c-migrate-passed", "true", "m-migrate", EdgeType::ConditionTrue),
                edge("e-revise-merge", "a-revise-migrate", "m-migrate"),
                edge("e-merge-execute", "m-migrate", "a-execute-migrate"),
                edge("e-execute-end", "a-execute-migrate", "end"),
            ],
            vec![DomainInputField { key: "migration_desc", label: "迁移描述", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 6. wf-eng-deploy: DevOps部署 ───────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-deploy",
            "DevOps部署",
            "DevOps部署：构建制品，测试失败自动修复重建，部署后验证",
            "🚀",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：构建
                make_agent_node(
                    "a-build",
                    "构建",
                    "构建制品：编译、打包、版本标记、制品校验。\
                     输出 JSON：{\"artifact\":\"\", \"version\":\"\", \"build_ok\":true, \"warnings\":[]}",
                    vec![td_desc("Bash", "执行构建命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-build",
                    0.0,
                    180.0,
                ),
                // Agent：测试
                make_agent_node_full(
                    "a-test",
                    "测试",
                    "运行测试套件：单元、集成、冒烟测试，评估通过状态。\
                     输出 JSON：{\"passed\":0, \"failed\":0, \"skipped\":0, \"verdict\":\"pass|fail\", \"failures\":[]}",
                    vec![td_desc("Bash", "执行测试命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-test",
                    vec![("build", "a-build")],
                    vec!["a-build"],
                    0.0,
                    360.0,
                ),
                // 条件：测试是否通过
                make_condition_node(
                    "c-deploy-test",
                    "测试判定",
                    vec![Condition {
                        var_path: "a-test.verdict".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!("pass"),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 通过：部署
                make_agent_node_full(
                    "a-deploy",
                    "部署",
                    "执行部署：环境切换、配置下发、灰度/全量、监控联动。\
                     输出 JSON：{\"environment\":\"\", \"strategy\":\"\", \"status\":\"success|failed\", }",
                    vec![td_desc("Bash", "执行部署命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-deploy",
                    vec![("build", "a-build"), ("test", "a-test")],
                    vec!["a-build", "a-test"],
                    -250.0,
                    720.0,
                ),
                // 失败：修复重建
                make_agent_node_full(
                    "a-rebuild",
                    "修复重建",
                    "测试失败，诊断问题并修复后重新构建。\
                     输出 JSON：{\"fixes\":[], \"rebuild_ok\":true, \"verdict\":\"pass\"}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-rebuild",
                    vec![("test", "a-test"), ("build", "a-build")],
                    vec!["a-test", "a-build"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-deploy", "汇合", 0.0, 900.0),
                // Agent：部署验证
                make_agent_node_full(
                    "a-verify",
                    "验证",
                    "验证部署：健康检查、关键路径冒烟、指标对比。\
                     输出 JSON：{\"health\":\"ok|degraded\", \"smoke\":[], }",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-verify",
                    vec![("deploy", "a-deploy"), ("rebuild", "a-rebuild")],
                    vec!["a-deploy", "a-rebuild"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-build", "trigger", "a-build"),
                edge("e-build-test", "a-build", "a-test"),
                edge("e-test-verdict", "a-test", "c-deploy-test"),
                edge_cond("e-pass-deploy", "c-deploy-test", "true", "a-deploy", EdgeType::ConditionTrue),
                edge_cond("e-fail-rebuild", "c-deploy-test", "false", "a-rebuild", EdgeType::ConditionFalse),
                edge("e-deploy-merge", "a-deploy", "m-deploy"),
                edge("e-rebuild-merge", "a-rebuild", "m-deploy"),
                edge("e-merge-verify", "m-deploy", "a-verify"),
                edge("e-verify-end", "a-verify", "end"),
            ],
            vec![DomainInputField { key: "environment", label: "目标环境", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 7. wf-eng-monitor-setup: 监控告警 ──────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-monitor-setup",
            "监控告警配置",
            "监控告警：规划监控指标，配置告警规则，验证失败自动调整",
            "📡",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：监控规划
                make_agent_node(
                    "a-monitor-plan",
                    "监控规划",
                    "规划监控指标：服务健康、性能、业务指标、日志采集范围。\
                     输出 JSON：{\"metrics\":[{\"name\":\"\", }",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-monitor-plan",
                    0.0,
                    180.0,
                ),
                // Agent：告警配置
                make_agent_node_full(
                    "a-monitor-setup",
                    "告警配置",
                    "配置告警规则：阈值、聚合、静默、通知渠道、升级策略。\
                     输出 JSON：{\"rules\":[{\"metric\":\"\", \"condition\":\"\", \"severity\":\"\", \"notify\":[]}], \"valid\":true, \"issues\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-monitor-setup",
                    vec![("plan", "a-monitor-plan")],
                    vec!["a-monitor-plan"],
                    0.0,
                    360.0,
                ),
                // 条件：告警验证
                make_condition_node(
                    "c-monitor-valid",
                    "验证判定",
                    vec![Condition {
                        var_path: "a-monitor-setup.valid".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 调整分支
                make_agent_node_full(
                    "a-monitor-fix",
                    "告警调整",
                    "修复告警配置问题：阈值不合理、渠道失效、规则冲突。\
                     输出 JSON：{\"adjustments\":[], \"valid\":true}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-monitor-fix",
                    vec![("setup", "a-monitor-setup")],
                    vec!["a-monitor-setup"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-monitor", "汇合", 0.0, 900.0),
                // Agent：验证测试
                make_agent_node_full(
                    "a-monitor-test",
                    "验证测试",
                    "验证监控与告警端到端：指标上报、告警触发、通知送达。\
                     输出 JSON：{\"verified\":true, \"test_results\":[], \"report\":\"\"}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-monitor-test",
                    vec![("setup", "a-monitor-setup"), ("fix", "a-monitor-fix")],
                    vec!["a-monitor-setup", "a-monitor-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-monitor-plan"),
                edge("e-plan-setup", "a-monitor-plan", "a-monitor-setup"),
                edge("e-setup-valid", "a-monitor-setup", "c-monitor-valid"),
                edge_cond("e-invalid-fix", "c-monitor-valid", "false", "a-monitor-fix", EdgeType::ConditionFalse),
                edge_cond("e-valid-merge", "c-monitor-valid", "true", "m-monitor", EdgeType::ConditionTrue),
                edge("e-fix-merge", "a-monitor-fix", "m-monitor"),
                edge("e-merge-test", "m-monitor", "a-monitor-test"),
                edge("e-test-end", "a-monitor-test", "end"),
            ],
            vec![DomainInputField { key: "service_name", label: "服务名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 8. wf-eng-onboarding: 开发入职 ─────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-onboarding",
            "开发入职",
            "开发入职：准备开发环境，学习项目文档，未就绪自动补充培训，执行首个任务",
            "🎓",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：环境准备
                make_agent_node(
                    "a-env-setup",
                    "环境准备",
                    "准备开发环境：工具链、依赖、IDE 配置、访问权限。\
                     输出 JSON：{\"prepared\":[], \"pending\":[], \"ready\":true}",
                    vec![td_desc("Bash", "检查环境工具链")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-env-setup",
                    0.0,
                    180.0,
                ),
                // Agent：文档学习
                make_agent_node_full(
                    "a-doc-read",
                    "文档学习",
                    "学习项目文档：架构说明、代码规范、开发流程、部署说明。\
                     输出 JSON：{\"learned\":[], \"questions\":[], }",
                    vec![td_desc("FileRead", "读取项目文档"), td_desc("Grep", "检索文档")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-doc-read",
                    vec![("env", "a-env-setup")],
                    vec!["a-env-setup"],
                    0.0,
                    360.0,
                ),
                // 条件：就绪判定
                make_condition_node(
                    "c-onboard-ready",
                    "就绪判定",
                    vec![Condition {
                        var_path: "a-doc-read.learned".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 就绪：首个任务
                make_agent_node_full(
                    "a-first-task",
                    "首个任务",
                    "分配并执行首个开发任务：小步交付、导师确认。\
                     输出 JSON：{\"task\":\"\", }",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-first-task",
                    vec![("env", "a-env-setup"), ("doc", "a-doc-read")],
                    vec!["a-env-setup", "a-doc-read"],
                    -250.0,
                    720.0,
                ),
                // 未就绪：补充培训
                make_agent_node_full(
                    "a-train",
                    "补充培训",
                    "生成补充培训计划：针对性教程、练习任务、答疑安排。\
                     输出 JSON：{\"plan\":[], \"materials\":[], \"mentor\":\"\"}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-train",
                    vec![("doc", "a-doc-read")],
                    vec!["a-doc-read"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-onboard-eng", "汇合", 0.0, 900.0),
                // Agent：入职确认
                make_agent_node_full(
                    "a-onboard-done",
                    "入职确认",
                    "确认入职完成：环境就绪、任务完成、反馈收集。\
                     输出 JSON：{\"confirmed\":true, }",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-onboard-done",
                    vec![("task", "a-first-task"), ("train", "a-train")],
                    vec!["a-first-task", "a-train"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-env", "trigger", "a-env-setup"),
                edge("e-env-doc", "a-env-setup", "a-doc-read"),
                edge("e-doc-ready", "a-doc-read", "c-onboard-ready"),
                edge_cond(
                    "e-ok-task",
                    "c-onboard-ready",
                    "true",
                    "a-first-task",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-no-train",
                    "c-onboard-ready",
                    "false",
                    "a-train",
                    EdgeType::ConditionFalse,
                ),
                edge("e-task-merge", "a-first-task", "m-onboard-eng"),
                edge("e-train-merge", "a-train", "m-onboard-eng"),
                edge("e-merge-done", "m-onboard-eng", "a-onboard-done"),
                edge("e-done-end", "a-onboard-done", "end"),
            ],
            vec![DomainInputField {
                key: "developer",
                label: "开发者",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 9. wf-eng-perf-opt: 性能优化 ───────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-perf-opt",
            "性能优化",
            "性能优化：剖析性能瓶颈，瓶颈不明确自动深度剖析，实施优化",
            "⚡",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：性能剖析
                make_agent_node(
                    "a-profile",
                    "性能剖析",
                    "剖析系统性能：热点函数、耗时分布、资源占用、吞吐瓶颈。\
                     输出 JSON：{\"hotspots\":[{\"func\":\"\", }",
                    vec![td_desc("Bash", "执行性能剖析命令")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-profile",
                    0.0,
                    180.0,
                ),
                // Agent：瓶颈定位
                make_agent_node_full(
                    "a-identify",
                    "瓶颈定位",
                    "定位性能瓶颈根因：算法、IO、锁、缓存、网络。\
                     输出 JSON：{\"bottleneck\":\"\", \"root_cause\":\"\", \"identified\":true, \"candidates\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-identify",
                    vec![("profile", "a-profile")],
                    vec!["a-profile"],
                    0.0,
                    360.0,
                ),
                // 条件：瓶颈明确
                make_condition_node(
                    "c-perf-identified",
                    "瓶颈判定",
                    vec![Condition {
                        var_path: "a-identify.identified".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 明确：优化实施
                make_agent_node_full(
                    "a-optimize",
                    "优化实施",
                    "实施优化方案：算法改进、缓存引入、并发调整、IO 优化。\
                     输出 JSON：{\"changes\":[], \"expected_gain\":0, \"measure_plan\":\"\"}",
                    vec![td_desc("FileWrite", "写入优化代码")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-optimize",
                    vec![("identify", "a-identify")],
                    vec!["a-identify"],
                    -250.0,
                    720.0,
                ),
                // 不明确：深度剖析
                make_agent_node_full(
                    "a-deep",
                    "深度剖析",
                    "瓶颈不明确，进行深度剖析：采样、火焰图、指标细分。\
                     输出 JSON：{\"deep_analysis\":\"\", \"refined_candidates\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-deep",
                    vec![("profile", "a-profile")],
                    vec!["a-profile"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-perf", "汇合", 0.0, 900.0),
                // Agent：效果验证
                make_agent_node_full(
                    "a-perf-verify",
                    "效果验证",
                    "验证优化效果：复测对比、回归确认、结论报告。\
                     输出 JSON：{\"before\":0, \"after\":0, \"improvement\":0, \"regression\":false}",
                    vec![td_desc("Bash", "运行基准测试")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-perf-verify",
                    vec![("optimize", "a-optimize"), ("deep", "a-deep")],
                    vec!["a-optimize", "a-deep"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-profile", "trigger", "a-profile"),
                edge("e-profile-identify", "a-profile", "a-identify"),
                edge("e-identify-bottleneck", "a-identify", "c-perf-identified"),
                edge_cond("e-ok-optimize", "c-perf-identified", "true", "a-optimize", EdgeType::ConditionTrue),
                edge_cond("e-no-deep", "c-perf-identified", "false", "a-deep", EdgeType::ConditionFalse),
                edge("e-optimize-merge", "a-optimize", "m-perf"),
                edge("e-deep-merge", "a-deep", "m-perf"),
                edge("e-merge-verify", "m-perf", "a-perf-verify"),
                edge("e-verify-end", "a-perf-verify", "end"),
            ],
            vec![DomainInputField { key: "perf_target", label: "优化目标", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 10. wf-eng-security-review: 安全审查 ───────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-security-review",
            "安全审查",
            "安全审查：扫描安全漏洞，高危漏洞紧急修复，常规漏洞按计划修复",
            "🛡️",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：安全扫描
                make_agent_node(
                    "a-scan",
                    "安全扫描",
                    "扫描安全漏洞：注入、XSS、越权、敏感信息、依赖漏洞。\
                     输出 JSON：{\"vulns\":[{\"type\":\"\", \"severity\":\"critical|high|medium|low\", \"location\":\"\", \"description\":\"\"}], \"high_count\":0}",
                    vec![td_desc("Grep", "检索敏感模式"), td_desc("Bash", "运行安全扫描工具")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-scan",
                    0.0,
                    180.0,
                ),
                // Agent：风险分析
                make_agent_node_full(
                    "a-analyze-s",
                    "风险分析",
                    "分析漏洞风险：可利用性、影响面、修复优先级。\
                     输出 JSON：{\"analysis\":[{\"vuln\":\"\", \"exploitability\":0, \"impact\":\"\", \"priority\":\"P0|P1|P2\"}], \"recommended_order\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-analyze-s",
                    vec![("scan", "a-scan")],
                    vec!["a-scan"],
                    0.0,
                    360.0,
                ),
                // 条件：高危漏洞
                make_condition_node(
                    "c-sec-high",
                    "高危判定",
                    vec![Condition {
                        var_path: "a-scan.high_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 高危：紧急修复
                make_agent_node_full(
                    "a-fix",
                    "紧急修复",
                    "紧急修复高危漏洞：立即整改、复测验证、上线加固。\
                     输出 JSON：{\"fixes\":[{\"vuln\":\"\", \"patch\":\"\", \"verified\":true}], \"advisory\":\"\"}",
                    vec![td_desc("FileWrite", "写入修复代码"), td_desc("OpcSendNotification", "通知安全负责人")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-fix",
                    vec![("scan", "a-scan")],
                    vec!["a-scan"],
                    -250.0,
                    720.0,
                ),
                // 常规：修复计划
                make_agent_node_full(
                    "a-fix-plan",
                    "修复计划",
                    "为常规漏洞制定修复计划与排期。\
                     输出 JSON：{\"plan\":[{\"vuln\":\"\", \"action\":\"\", \"due\":\"\"}]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-fix-plan",
                    vec![("scan", "a-scan")],
                    vec!["a-scan"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-sec", "汇合", 0.0, 900.0),
                // Agent：安全报告
                make_agent_node_full(
                    "a-sec-report",
                    "安全报告",
                    "输出安全审查报告：漏洞汇总、风险等级、修复状态。\
                     输出 JSON：{\"summary\":\"\", \"risk_level\":\"\", \"remaining\":[], \"recommendations\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-sec-report",
                    vec![("fix", "a-fix"), ("fix-plan", "a-fix-plan")],
                    vec!["a-fix", "a-fix-plan"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-scan", "trigger", "a-scan"),
                edge("e-scan-analyze", "a-scan", "a-analyze-s"),
                edge("e-analyze-high", "a-analyze-s", "c-sec-high"),
                edge_cond("e-high-fix", "c-sec-high", "true", "a-fix", EdgeType::ConditionTrue),
                edge_cond("e-low-plan", "c-sec-high", "false", "a-fix-plan", EdgeType::ConditionFalse),
                edge("e-fix-merge", "a-fix", "m-sec"),
                edge("e-plan-merge", "a-fix-plan", "m-sec"),
                edge("e-merge-report", "m-sec", "a-sec-report"),
                edge("e-report-end", "a-sec-report", "end"),
            ],
            vec![DomainInputField { key: "scan_target", label: "扫描目标", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 11. wf-eng-tech-debt: 技术债管理 ───────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-eng-tech-debt",
            "技术债管理",
            "技术债管理：扫描技术债项，高优先级债务优先偿还，输出偿还计划",
            "💳",
            vec!["opc".to_string(), "engineering".to_string()],
            ENG_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：债项扫描
                make_agent_node(
                    "a-debt-scan",
                    "债项扫描",
                    "扫描技术债：遗留代码、废弃依赖、重复实现、临时方案。\
                     输出 JSON：{\"debts\":[{\"id\":\"\", \"type\":\"\", \"location\":\"\", \"severity\":\"high|medium|low\", \"impact\":\"\"}], \"high_count\":0}",
                    vec![td_desc("Grep", "检索遗留代码模式"), td_desc("FileRead", "读取相关文件")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-debt-scan",
                    0.0,
                    180.0,
                ),
                // Agent：优先级排序
                make_agent_node_full(
                    "a-debt-prioritize",
                    "优先级排序",
                    "按影响/成本/风险排序技术债项，标记高优先级。\
                     输出 JSON：{\"ranked\":[{\"debt\":\"\", \"priority\":\"P0|P1|P2\", \"rationale\":\"\"}], \"p0_count\":0}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-debt-prioritize",
                    vec![("scan", "a-debt-scan")],
                    vec!["a-debt-scan"],
                    0.0,
                    360.0,
                ),
                // 条件：高优先级
                make_condition_node(
                    "c-debt-high",
                    "优先级判定",
                    vec![Condition {
                        var_path: "a-debt-prioritize.p0_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 高优先级：优先偿还
                make_agent_node_full(
                    "a-debt-repay",
                    "优先偿还",
                    "制定 P0 债务偿还计划：立即行动、验证标准。\
                     输出 JSON：{\"repay_plan\":[{\"debt\":\"\", \"action\":\"\", \"owner\":\"\", \"due\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "通知技术债责任人")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-debt-repay",
                    vec![("ranked", "a-debt-prioritize")],
                    vec!["a-debt-prioritize"],
                    -250.0,
                    720.0,
                ),
                // 常规：偿还路线图
                make_agent_node_full(
                    "a-debt-roadmap",
                    "偿还路线图",
                    "制定常规债务偿还路线图与节奏。\
                     输出 JSON：{\"roadmap\":[], \"quarterly_quota\":0}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-debt-roadmap",
                    vec![("ranked", "a-debt-prioritize")],
                    vec!["a-debt-prioritize"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-debt", "汇合", 0.0, 900.0),
                // Agent：总结
                make_agent_node_full(
                    "a-debt-summary",
                    "总结",
                    "汇总技术债治理方案：债务总量、偿还节奏、预防机制。\
                     输出 JSON：{\"total_debt\":0, \"strategy\":\"\", \"prevention\":[]}",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-debt-summary",
                    vec![("repay", "a-debt-repay"), ("roadmap", "a-debt-roadmap")],
                    vec!["a-debt-repay", "a-debt-roadmap"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-scan", "trigger", "a-debt-scan"),
                edge("e-scan-prioritize", "a-debt-scan", "a-debt-prioritize"),
                edge("e-prioritize-high", "a-debt-prioritize", "c-debt-high"),
                edge_cond("e-high-repay", "c-debt-high", "true", "a-debt-repay", EdgeType::ConditionTrue),
                edge_cond("e-low-roadmap", "c-debt-high", "false", "a-debt-roadmap", EdgeType::ConditionFalse),
                edge("e-repay-merge", "a-debt-repay", "m-debt"),
                edge("e-roadmap-merge", "a-debt-roadmap", "m-debt"),
                edge("e-merge-summary", "m-debt", "a-debt-summary"),
                edge("e-summary-end", "a-debt-summary", "end"),
            ],
            vec![DomainInputField { key: "repo_name", label: "仓库名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }
    // ── 12. wf-eng-refactor-lite: 快速追加重构 ────────────────────
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-eng-refactor-lite",
            "快速追加重构",
            "重构完成后的追加变更快速通道，支持架构调整、功能新增、技术栈升级等变更的轻量级注入",
            "⚡",
            vec!["opc".to_string(), "refactor".to_string(), "incremental".to_string(), "fast-track".to_string(), "engineering".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(100.0, 50.0),
                make_agent_node_with_inputs(
                    "l-assess", "变更影响评估",
                    concat!(
                        "你作为架构分析师，快速评估追加变更的影响：\n",
                        "1. 识别变更涉及的模块和文件\n",
                        "2. 评估与已有重构成果的兼容性\n",
                        "3. 分析变更的依赖关系和连锁影响\n",
                        "4. 评估测试影响范围\n",
                        "5. 给出变更复杂度等级和预估工期\n",
                        "输出 JSON：{change_assessment{change{type{architecture|feature|tech_stack|dependency|bug}, description, source}, impacted_modules[{module, change_type, risk}], compatibility_check{conflicts[], breaking_changes, required_adaptions}, test_impact{affected_tests[], new_tests_needed[], regression_scope}, complexity{level{low|medium|high|critical}, estimated_effort, estimated_duration}, recommendation{proceed|defer|escalate, rationale, prerequisites[]}}"
                    ),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "l-assess_result",
                    HashMap::new(),
                    100.0, 200.0,
                ),
                make_agent_node_with_inputs(
                    "l-plan", "最小化变更计划",
                    concat!(
                        "你作为技术项目经理，制定最小化变更计划：\n",
                        "1. 确定变更实施的最小影响路径\n",
                        "2. 规划变更与现有代码的集成点\n",
                        "3. 制定增量测试策略\n",
                        "4. 规划特性开关和灰度策略\n",
                        "5. 设定验收标准\n",
                        "输出 JSON：{change_plan{implementation_path{steps[{step, action, files[], dependencies}], integration_points[], minimal_change_set}, test_strategy{unit_tests{additions[], modifications[]}, integration_tests{new_scenarios[], regression_scope}, performance_validation{baseline_comparison, key_paths}}, feature_flags{needed[{name, scope, default, cleanup_plan}], rollback_triggers[]}, acceptance_criteria{functional{checks[]}, quality{coverage_min, lint_pass}, performance{max_regression}}, estimated_timeline{phases[{phase, duration, deliverable}], total_effort}}"
                    ),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "l-plan_result",
                    HashMap::from([("assessment".to_string(), "l-assess.content".to_string())]),
                    100.0, 350.0,
                ),
                make_agent_node_with_inputs(
                    "l-execute", "变更执行",
                    concat!(
                        "你作为高级工程师，执行追加变更：\n",
                        "1. 按最小化变更计划逐步实施\n",
                        "2. 保持与已有重构成果的一致性\n",
                        "3. 每完成一个变更点运行局部测试\n",
                        "4. 记录变更细节（文件、函数、行号、影响范围）\n",
                        "5. 更新相关文档\n",
                        "输出 JSON：{execution{changes_completed[{change_id, type, files_changed[], tests_run, status}], integration_checks[{point, status, issues_found}], test_results{unit{passed, failed, new}, integration{passed, failed, new}, coverage_delta{before, after}}, issues[{description, impact, resolution}]}}"
                    ),
                    vec![td("FileRead"), td("FileWrite"), td("Bash"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "l-execute_result",
                    HashMap::from([("plan".to_string(), "l-plan.content".to_string())]),
                    100.0, 500.0,
                ),
                make_agent_node_with_inputs(
                    "l-verify", "验证与交付",
                    concat!(
                        "你作为质量工程师，执行最终验证：\n",
                        "1. 运行增量测试和回归测试\n",
                        "2. 验证变更与已有重构成果的兼容性\n",
                        "3. 检查性能是否满足基线要求\n",
                        "4. 确认代码质量未退化\n",
                        "5. 完成交付检查\n",
                        "输出 JSON：{verification{test_summary{total, passed, failed, skipped, coverage_delta{before, after, delta}}, compatibility_check{refactored_modules_impacted, breaking_changes, adapters_sufficient}, performance_check{key_paths[{path, before, after, regression}], overall_regression}, quality_gate{lint_errors, lint_warnings, complexity_change, smells_introduced}, delivery{verdict{pass|fail, blocking_issues[], recommendations[]}, artifacts_updated[], handoff_notes}}}"
                    ),
                    vec![td("Bash"), td("FileRead"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "l-verify_result",
                    HashMap::from([("execution".to_string(), "l-execute.content".to_string())]),
                    100.0, 650.0,
                ),
                make_end(100.0, 800.0),
            ],
            vec![
                edge("e-trigger-l-assess", "trigger", "l-assess"),
                edge("e-l-assess-l-plan", "l-assess", "l-plan"),
                edge("e-l-plan-l-execute", "l-plan", "l-execute"),
                edge("e-l-execute-l-verify", "l-execute", "l-verify"),
                edge("e-l-verify-end", "l-verify", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── 13. wf-eng-refactor: 大型代码项目重构 ─────────────────────
    // 系统性重构百万行级遗留代码，支持同语言重构和跨语言迁移（C++→Rust、C#→Rust、Java→TypeScript 等）。
    // 内置 UI 组件映射和国际化迁移分析，支持从现有 i18n 导入或从零搭建 i18next。
    // 从资产普查到验收交付的完整闭环。
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-eng-refactor",
            "大型代码项目重构",
            "系统性重构百万行级遗留代码，支持同语言重构和跨语言迁移（C++→Rust、C#→Rust、Java→TypeScript 等）。内置 UI 组件映射和国际化迁移分析，支持从现有 i18n 导入或从零搭建 i18next。从资产普查到验收交付的完整闭环。",
            "🔧",
            vec!["opc".to_string(), "refactor".to_string(), "large-scale".to_string(), "code-quality".to_string(), "cross-language".to_string(), "engineering".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(100.0, 50.0),
                // 1. 代码资产盘点
                make_agent_node_with_inputs(
                    "a-asset-scan", "代码资产盘点",
                    concat!("你作为代码审计专家，对项目进行全面的资产盘点：\n", "1. 统计代码行数（分语言、分目录、分文件）\n", "2. 识别所有源文件、配置文件、测试文件、第三方库\n", "3. 标注模块边界和目录结构\n", "4. 统计第三方依赖和框架版本\n", "5. 识别技术栈（语言、框架、数据库、中间件）\n", "6. 生成代码资产清单（文件数、行数、模块数、依赖数）\n", "输出 JSON：{inventory{total_files, total_lines, by_language[{lang, files, lines}], by_module[{module, files, lines}], dependencies[{name, version, type}], tech_stack}, structure{directories, entry_points, public_api}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-asset-scan_result",
                    HashMap::new(),
                    100.0, 200.0,
                ),
                // 2. 依赖关系分析
                make_agent_node_with_inputs(
                    "a-dep-graph", "依赖关系分析",
                    concat!("你作为架构分析师，构建项目的依赖关系图：\n", "1. 分析模块间的 import/include 依赖关系\n", "2. 识别循环依赖和不稳定依赖\n", "3. 构建包级/模块级依赖拓扑图\n", "4. 标注公共 API 和内部 API 边界\n", "5. 识别依赖方向违反（高层依赖低层、跨层调用）\n", "输出 JSON：{dependency_graph{nodes[{id, name, type, layer}], edges[{from, to, type}], circular_dependencies[], unstable_dependencies[], layer_violations[]}, impact_analysis{high_impact_modules[{module, impacted_modules, risk_level}], blast_radius_map}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-dep-graph_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 350.0,
                ),
                // 3. 复杂度扫描
                make_agent_node_with_inputs(
                    "a-complexity", "复杂度扫描",
                    concat!("你作为代码质量专家，评估代码复杂度：\n", "1. 计算每个函数的圈复杂度（Cyclomatic Complexity）\n", "2. 计算认知复杂度（Cognitive Complexity）\n", "3. 识别超过阈值（CC>20）的高风险函数\n", "4. 统计嵌套深度、分支数量、参数数量\n", "5. 评估代码重复率（Copy-Paste Detection）\n", "输出 JSON：{complexity{high_risk_functions[{file, function, cyclomatic, cognitive, lines}], average_cyclomatic, max_cyclomatic, by_module[{module, avg_cc, max_cc, high_risk_count}]}, duplication{rate, hotspots[{file, lines_count, duplicated_with}]}, nesting{deep_functions[{file, function, depth}]}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-complexity_result",
                    HashMap::new(),
                    100.0, 500.0,
                ),
                // 4. 运行时行为快照
                make_agent_node_with_inputs(
                    "a-behavior-snapshot", "运行时行为快照",
                    concat!("你作为行为测试专家，为待重构模块生成运行时行为快照：\n", "1. 识别模块的公开 API（函数/方法/入口点）\n", "2. 为每个公开 API 生成覆盖全路径的测试用例（正常路径、边界条件、异常路径）\n", "3. 运行原有代码，记录每个用例的实际输出（返回值、副作用、状态变更）\n", "4. 捕获运行时副作用：数据库写入、文件 IO、网络请求、缓存操作、事件发射\n", "5. 记录状态机转换和时序依赖\n", "6. 将所有输入-输出对保存为\"黄金测试\"（Golden Test）基线\n", "输出 JSON：{behavioral_snapshot{api_snapshots[{api, inputs[{input_args, description}], outputs[{return_value, side_effects[{type, target, data}], state_changes[{entity, before, after}], timing}], coverage{paths_covered, paths_total, edge_cases_covered, error_paths_covered}, golden_tests[{test_id, api, input_fixture, expected_output, expected_side_effects, priority}], runtime_effects{external_calls[{service, call_signature, return_value}], db_operations[{table, operation, data}], event_streams[{event_type, payload}]}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-behavior-snapshot_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string()), ("dep_graph".to_string(), "a-dep-graph.content".to_string())]),
                    100.0, 650.0,
                ),
                // 5. 隐式知识提取
                make_agent_node_with_inputs(
                    "a-tacit-knowledge", "隐式知识提取",
                    concat!("你作为知识提炼专家，从原有代码中提取隐式知识：\n", "1. 提取代码注释中的设计意图（\"为什么这样写\"而非\"写了什么\"）\n", "2. 分析提交历史，提取 bug 修复模式和设计决策上下文\n", "3. 识别隐式契约（调用方和被调用方的非文档化约定）\n", "4. 提取魔法数字/字符串的实际含义和来源\n", "5. 识别边界条件和特殊处理（防御性代码、hack、workaround）\n", "6. 记录并发假设和时序约束\n", "输出 JSON：{tacit_knowledge{design_intent[{file, code_region, comment, inferred_purpose, confidence}], commit_insights[{file, commit_hash, message, change_type, lesson_learned, related_bug}], implicit_contracts[{api, caller_expectations, callee_assumptions, violation_examples, confidence}], magic_values[{file, value, context, inferred_meaning, source}], edge_cases[{file, function, condition, special_handling, rationale}], concurrency_constraints[{module, assumption_type, description, violation_scenario}]}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-tacit-knowledge_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 800.0,
                ),
                // 6. 坏味道检测
                make_agent_node_with_inputs(
                    "a-smell-detect", "坏味道检测",
                    concat!("你作为代码审计专家，检测代码坏味道：\n", "1. 识别长方法/长函数（>50 行）\n", "2. 识别上帝类/上帝对象（>500 行或 >20 方法）\n", "3. 识别魔法数字和魔法字符串\n", "4. 识别深层嵌套（>4 层）\n", "5. 识别重复代码和 Copy-Paste\n", "6. 识别缺失抽象（if-else 链、switch 驱动）\n", "输出 JSON：{smells[{type, file, line, description, severity, suggestion}], smell_summary{total, critical, high, medium, low, by_type[{type, count}]}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-smell-detect_result",
                    HashMap::new(),
                    100.0, 950.0,
                ),
                // 7. 耦合度分析
                make_agent_node_with_inputs(
                    "a-coupling-analyze", "耦合度分析",
                    concat!("你作为架构分析师，评估模块耦合度：\n", "1. 计算每个模块的扇入（fan-in）和扇出（fan-out）\n", "2. 识别高耦合模块（fan-in>10 或 fan-out>10）\n", "3. 识别紧密耦合的模块组\n", "4. 标记双向依赖和网状依赖\n", "5. 评估内聚性（Cohesion）\n", "输出 JSON：{coupling{high_coupling_modules[{module, fan_in, fan_out, coupling_score}], tight_coupling_groups[{modules[], coupling_type}], bidirectional_dependencies[], cohesion_assessment[{module, cohesion_score, issues[]}]}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-coupling-analyze_result",
                    HashMap::new(),
                    100.0, 1100.0,
                ),
                // 8. 风险评估
                make_agent_node_with_inputs(
                    "a-risk-assess", "风险评估",
                    concat!("你作为重构顾问，评估重构风险：\n", "1. 评估变更影响范围（Blast Radius）\n", "2. 评估回归风险（测试覆盖盲区、无测试模块）\n", "3. 评估技术风险（ unsafe 代码、反射、动态绑定）\n", "4. 评估数据迁移风险（数据库 Schema 变更、数据完整性）\n", "5. 评估性能风险（重构后可能的性能退化）\n", "输出 JSON：{risk_assessment{impact_scope{affected_modules[], critical_paths[]}, regression_risk{uncovered_modules[], risk_level}, technical_risk{unsafe_areas[], dynamic_dependencies[]}, data_migration_risk{schema_changes, data_loss_risk}, performance_risk{hot_paths[], expected_regression}}, risk_score{overall, confidence, factors[]}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-risk-assess_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string()), ("complexity".to_string(), "a-complexity.content".to_string())]),
                    100.0, 1250.0,
                ),
                // 9. 重构策略制定
                make_agent_node_with_inputs(
                    "a-strategy", "重构策略制定",
                    concat!("你作为架构师，制定整体重构策略：\n", "1. 根据风险评估选择重构模式（渐进式/大爆炸/绞杀者/旁路）\n", "2. 确定重构范围和边界\n", "3. 制定技术选型和架构改进方案\n", "4. 定义重构成功标准（质量指标、性能指标、交付指标）\n", "5. 识别关键路径和里程碑\n", "输出 JSON：{strategy{mode{type, rationale, pros, cons}, scope{in_scope[], out_of_scope[]}, architecture_changes[{from, to, rationale}], success_criteria{quality[{metric, target}], performance[{metric, baseline, target}], delivery[{metric, target}]}, critical_path{steps[], estimated_duration}}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-strategy_result",
                    HashMap::from([("risk_assessment".to_string(), "a-risk-assess.content".to_string())]),
                    100.0, 1400.0,
                ),
                // 10. 分批计划
                make_agent_node_with_inputs(
                    "a-batch-plan", "分批计划",
                    concat!("你作为技术项目经理，制定分批执行计划：\n", "1. 按耦合度排序，确定重构批次（每批 5-10 个模块）\n", "2. 识别每批的前置依赖和后置影响\n", "3. 分配每批的工期和资源\n", "4. 设定每批的验收标准和退出条件\n", "5. 规划批次间的集成验证点\n", "输出 JSON：{batches[{batch_id, modules[], order, dependencies, estimated_effort, exit_criteria{test_coverage, complexity_reduction, performance_delta}, verification_steps[]}], milestones[{name, batch_range, deliverable, approval_required}]}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-batch-plan_result",
                    HashMap::from([("strategy".to_string(), "a-strategy.content".to_string()), ("dep_graph".to_string(), "a-dep-graph.content".to_string())]),
                    100.0, 1550.0,
                ),
                // 11. 变更融合规划
                make_agent_node_with_inputs(
                    "a-change-merge", "变更融合规划",
                    concat!("你作为变更管理专家，将新需求/架构变更融入重构计划：\n", "1. 接收外部变更请求列表（新功能/架构调整/技术栈升级/依赖更新）\n", "2. 分类变更类型和优先级\n", "3. 评估每个变更与现有重构批次的依赖关系和冲突\n", "4. 将变更预分配到最合适的批次\n", "5. 评估增量测试需求和资源影响\n", "6. 标记不可合并的变更并给出建议\n", "输出 JSON：{changes[{id, type, description, priority, target_modules[], injection_batch, conflicts[{existing_step, conflict_type, resolution}], additional_tests[], resource_impact}], merged_batches[{batch_id, original_modules[], new_changes[], risk_level, additional_effort, additional_tests[]}], unmergeable_changes[{id, reason, recommendation}]}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-change-merge_result",
                    HashMap::from([("batch_plan".to_string(), "a-batch-plan.content".to_string())]),
                    100.0, 1700.0,
                ),
                // 12. 质量基线建立
                make_agent_node_with_inputs(
                    "a-quality-baseline", "质量基线建立",
                    concat!("你作为质量工程师，建立重构前的质量基线：\n", "1. 运行全量测试，记录当前测试覆盖率\n", "2. 执行性能基准测试，记录关键路径性能指标\n", "3. 运行代码静态分析，记录当前代码质量指标\n", "4. 建立代码规范和 lint 规则\n", "5. 生成质量基线报告\n", "输出 JSON：{baseline{test_coverage{overall, by_module[], uncovered_modules[]}, performance{key_paths[{path, latency_ms, throughput}], resource_usage{cpu, memory, disk_io}}, code_quality{lint_errors, lint_warnings, complexity_distribution[]}, quality_gates{coverage_min, complexity_max, lint_zero_errors, performance_regression_max}}}"),
                    vec![td("Bash"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-quality-baseline_result",
                    HashMap::from([("batch_plan".to_string(), "a-batch-plan.content".to_string())]),
                    100.0, 1850.0,
                ),
                // 13. 回滚方案
                make_agent_node_with_inputs(
                    "a-rollback", "回滚方案",
                    concat!("你作为 DevOps 工程师，制定完整的回滚方案：\n", "1. 设计分支策略（主干/特性分支/发布分支）\n", "2. 规划特性开关（Feature Flag）方案\n", "3. 制定数据库迁移回滚脚本\n", "4. 设计灰度发布和 A/B 测试方案\n", "5. 建立应急预案和回滚触发条件\n", "输出 JSON：{rollback{branch_strategy{model, naming_convention, protection_rules}, feature_flags[{name, scope, default_value, rollback_strategy}], db_migration{rollback_scripts[], data_preservation_plan}, canary{percentage_based, health_checks[], auto_rollback_triggers[]}, emergency_plan{trigger_conditions[], rollback_steps[], communication_plan}}}"),
                    vec![td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-rollback_result",
                    HashMap::from([("batch_plan".to_string(), "a-batch-plan.content".to_string()), ("quality_baseline".to_string(), "a-quality-baseline.content".to_string())]),
                    100.0, 2000.0,
                ),
                // 14. 目标框架集成规划（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-integration-plan", "目标框架集成规划",
                    concat!("你作为架构师，规划代码迁移到目标框架的集成方案：\n", "1. 设计目录结构和模块组织方案\n", "2. 规划后端命令注册和 State 管理\n", "3. 规划前端组件结构和 Store 分类\n", "4. 设计跨语言通信接口（Tauri IPC）\n", "5. 规划测试策略（单元测试、集成测试、E2E）\n", "6. UI 迁移专用：组件拆分方案、路由设计、状态管理架构\n", "7. 国际化专用：i18next 配置方案、翻译文件转换、key 命名规范\n", "8. 后端消息国际化：错误码设计、前后端 i18n 同步机制\n", "输出 JSON：{integration_plan{directory_structure, backend_integration{command_registration_plan, state_management_design, error_handling_strategy, error_code_framework}, frontend_integration{component_hierarchy, store_classification, i18n_strategy, error_code_mapping}, ui_integration{component_mapping_plan, layout_strategy, state_architecture, router_design}, i18n_integration{mode, config_plan{languages, namespaces, key_convention}}, backend_i18n_integration{pattern, error_code_convention}, testing_strategy, implementation_order}}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-integration-plan_result",
                    HashMap::from([("batch_plan".to_string(), "a-batch-plan.content".to_string())]),
                    100.0, 2150.0,
                ),
                // 15. 预审查
                make_agent_node_with_inputs(
                    "a-pre-review", "预审查",
                    concat!("你作为代码审查员，进行重构前的预审查：\n", "1. 审查当前批次涉及模块的现有代码\n", "2. 确认重构方案与代码实际情况一致\n", "3. 标记需要特殊处理的代码段\n", "4. 确认测试用例覆盖待重构代码\n", "5. 输出审查通过/驳回\n", "输出 JSON：{pre_review{batch_id, modules_reviewed[], findings[{file, line, severity, recommendation}], coverage_gaps[{module, uncovered_paths}], verdict{approved|rejected, blockers[], suggestions[]}}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-pre-review_result",
                    HashMap::from([("batch_plan".to_string(), "a-batch-plan.content".to_string())]),
                    100.0, 2300.0,
                ),
                // 16. 分批执行
                make_agent_node_with_inputs(
                    "a-execute", "分批执行",
                    concat!("你作为高级工程师，按计划执行重构：\n", "1. 逐模块执行重构，每完成一个模块运行测试\n", "2. 应用重构模式（Extract Method、Extract Class、Introduce Interface 等）\n", "3. 保持行为不变，确保测试持续通过\n", "4. 记录每步变更（文件、函数、行号）\n", "5. 更新依赖关系图\n", "输出 JSON：{execution{batch_id, modules_completed[{module, refactorings_applied[], tests_passed, lines_changed}], issues_encountered[{module, issue, resolution}], current_progress, remaining_modules[]}}"),
                    vec![td("FileRead"), td("FileWrite"), td("Bash"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-execute_result",
                    HashMap::from([("pre_review".to_string(), "a-pre-review.content".to_string()), ("quality_baseline".to_string(), "a-quality-baseline.content".to_string())]),
                    100.0, 2450.0,
                ),
                // 17. 语言惯用模式转换（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-idiomatic-convert", "语言惯用模式转换",
                    concat!("你作为跨语言迁移专家，将代码转换为目标语言的惯用模式：\n", "1. 识别源代码中的设计模式和惯用写法\n", "2. 根据映射规则转换为目标语言的惯用模式\n", "3. 应用代码规范（命名、格式、结构）\n", "4. 添加必要的文档注释\n", "5. 确保代码符合目标语言社区最佳实践\n", "输出 JSON：{conversion{patterns_converted[{source_pattern, target_pattern, file, function, lines_changed}], idiomatic_score{before, after, improvement, by_module[]}}, code_quality{naming_compliance, formatting_compliance, documentation_coverage, community_compliance}}"),
                    vec![td("FileRead"), td("FileWrite"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-idiomatic-convert_result",
                    HashMap::from([("execution".to_string(), "a-execute.content".to_string())]),
                    100.0, 2600.0,
                ),
                // 18. 框架集成验证（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-framework-validate", "框架集成验证",
                    concat!("你作为框架专家，验证生成的代码符合目标框架规范：\n", "1. 检查后端命令是否符合 Tauri 命令注册要求\n", "2. 检查 #[agent_command] 宏标签是否正确\n", "3. 检查前端 Store 是否符合四层分类规则\n", "4. 检查组件是否使用 i18n（禁止硬编码字符串）\n", "5. 检查类型是否从 @/types 导入\n", "6. 验证前后端通信接口是否正确\n", "7. i18n 专项检查：翻译文件完整性、key 命名规范、插值变量\n", "8. 后端消息 i18n 检查：错误码定义、前后端同步\n", "输出 JSON：{validation{backend{command_registration, agent_command_macro, state_management, error_code_definition}, frontend{store_classification, i18n_usage, type_imports, error_code_mapping}, i18n_validation{file_completeness, key_naming, interpolation, import_usage}, communication{ipc_pattern, data_contract, error_response_format}, overall_compliance{score, critical_violations, warnings, recommendations}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-framework-validate_result",
                    HashMap::from([("execution".to_string(), "a-execute.content".to_string()), ("idiomatic_convert".to_string(), "a-idiomatic-convert.content".to_string())]),
                    100.0, 2750.0,
                ),
                // 19. 语义等价验证
                make_agent_node_with_inputs(
                    "a-equivalence-check", "语义等价验证",
                    concat!("你作为行为验证专家，对比重构前后代码的语义等价性：\n", "1. 使用黄金测试（Golden Test）用例分别运行新旧代码，对比输出结果\n", "2. 逐维度对比：返回值结构、数据内容、副作用序列、状态变更\n", "3. 识别等价差异：完全等价、语义等价、行为差异、静默失败\n", "4. 生成差异报告，标记需要人工裁决的差异\n", "5. 验证隐式知识中的契约是否被保留\n", "输出 JSON：{equivalence_check{batch_id, comparison_results[{golden_test_id, old_output, new_output, equivalence{identical|semantic|different|silent_failure}, diff_details, verdict}], fidelity_score{percentage, by_module}, overall_verdict{pass|fail|review_required, auto_pass_count, manual_review_count}}"),
                    vec![td("Bash"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-equivalence-check_result",
                    HashMap::from([("execution".to_string(), "a-execute.content".to_string()), ("behavioral_snapshot".to_string(), "a-behavior-snapshot.content".to_string()), ("tacit_knowledge".to_string(), "a-tacit-knowledge.content".to_string())]),
                    100.0, 2900.0,
                ),
                // 20. 中途变更闸门
                make_agent_node_with_inputs(
                    "a-change-gate", "中途变更闸门",
                    concat!("你作为变更控制专家，评估重构执行过程中的中途变更：\n", "1. 接收外部变更请求（新功能/bug修复/架构调整/技术栈升级）\n", "2. 评估与当前重构批次的冲突（模块重叠、依赖冲突、时序冲突）\n", "3. 判断变更优先级并分类：P0 紧急/P1 重要/P2 常规/P3 延后\n", "4. 触发增量风险评估和测试需求评估\n", "5. 给出具体的变更处理方案\n", "输出 JSON：{change_request{id, type, description, source}, assessment{conflicts, impacted_batches, test_impact, resource_impact}, decision{priority, action{merge_into_current|defer_to_next|reject|pause_and_handle}, target_batch, resolution_steps, additional_tests}}"),
                    vec![td("Grep"), td("FileRead"), td("Bash")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-change-gate_result",
                    HashMap::from([("execution".to_string(), "a-execute.content".to_string())]),
                    100.0, 3050.0,
                ),
                // 21. 回归验证
                make_agent_node_with_inputs(
                    "a-regression", "回归验证",
                    concat!("你作为测试工程师，执行回归验证：\n", "1. 运行全量单元测试和集成测试\n", "2. 对比重构前后的测试覆盖率\n", "3. 执行性能回归测试，对比关键路径性能\n", "4. 检查是否引入新的代码坏味道\n", "5. 评估重构对模块耦合度的改善\n", "输出 JSON：{regression{test_results{total, passed, failed, skipped, coverage_delta{before, after, delta}}, performance_comparison{before{path, latency}, after{path, latency}, regression_detected}, quality_improvement{complexity_reduction, coupling_reduction, smells_removed}, regressions_found[{file, test, expected, actual, severity}]}}"),
                    vec![td("Bash"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-regression_result",
                    HashMap::from([("execution".to_string(), "a-execute.content".to_string()), ("quality_baseline".to_string(), "a-quality-baseline.content".to_string())]),
                    100.0, 3200.0,
                ),
                // 22. 集成验证
                make_agent_node_with_inputs(
                    "a-integration", "集成验证",
                    concat!("你作为集成测试工程师，执行跨模块集成验证：\n", "1. 验证重构模块与未重构模块的接口兼容性\n", "2. 执行端到端测试场景\n", "3. 验证数据流和状态管理的正确性\n", "4. 执行跨模块性能测试\n", "5. 确认无循环依赖和层间违规\n", "输出 JSON：{integration{interface_compatibility{verified[], breaking_changes[], adapters_needed[]}, e2e_scenarios, data_flow_validation, cross_module_performance{before, after, delta}, dependency_check{circular_deps[], layer_violations[]}}}"),
                    vec![td("Bash"), td("FileRead"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-integration_result",
                    HashMap::from([("regression".to_string(), "a-regression.content".to_string()), ("dep_graph".to_string(), "a-dep-graph.content".to_string())]),
                    100.0, 3350.0,
                ),
                // 23. 质量门禁
                make_agent_node_with_inputs(
                    "a-quality-gate", "质量门禁",
                    concat!("你作为质量总监，执行最终质量门禁检查：\n", "1. 检查测试覆盖率是否达到基线要求\n", "2. 检查代码复杂度是否降低\n", "3. 检查性能回归是否在可接受范围\n", "4. 检查代码坏味道是否显著减少\n", "5. 检查模块耦合度是否改善\n", "6. 检查行为保真度：黄金测试通过率、隐式知识保留率\n", "7. 检查副作用完整性：外部调用、数据库操作、事件发射是否一致\n", "8. 检查边界条件覆盖：原有代码的特殊处理是否全部保留\n", "输出 JSON：{quality_gate{coverage, complexity, performance, smells, coupling, behavioral_fidelity{golden_test_pass_rate, tacit_knowledge_retention, side_effect_equivalence, edge_case_retention}, overall_verdict{pass|fail, blocking_issues, recommendations}}}"),
                    vec![td("Bash"), td("FileRead"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-quality-gate_result",
                    HashMap::from([("integration".to_string(), "a-integration.content".to_string()), ("quality_baseline".to_string(), "a-quality-baseline.content".to_string()), ("equivalence_check".to_string(), "a-equivalence-check.content".to_string()), ("behavioral_snapshot".to_string(), "a-behavior-snapshot.content".to_string()), ("tacit_knowledge".to_string(), "a-tacit-knowledge.content".to_string())]),
                    100.0, 3500.0,
                ),
                // 24. 文档更新
                make_agent_node_with_inputs(
                    "a-doc-update", "文档更新",
                    concat!("你作为技术文档工程师，更新项目文档：\n", "1. 更新架构文档（架构图、模块关系、数据流）\n", "2. 更新 API 文档（接口变更、新增接口、废弃接口）\n", "3. 更新开发指南（编码规范、目录结构、构建流程）\n", "4. 更新迁移指南（从旧架构到新架构的迁移步骤）\n", "5. 生成重构总结报告\n", "输出 JSON：{docs_updated{architecture_doc, api_doc, migration_guide, refactor_summary{total_changes, modules_affected, complexity_reduction, coupling_improvement, lessons_learned}}}"),
                    vec![td("FileRead"), td("FileWrite"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-doc-update_result",
                    HashMap::from([("quality_gate".to_string(), "a-quality-gate.content".to_string())]),
                    100.0, 3650.0,
                ),
                // 25. 运维交接
                make_agent_node_with_inputs(
                    "a-handoff", "运维交接",
                    concat!("你作为运维工程师，完成运维交接：\n", "1. 生成运行手册（启动、停止、配置、故障排查）\n", "2. 设置监控告警（关键指标、阈值、告警渠道）\n", "3. 准备数据备份和恢复方案\n", "4. 整理应急预案和回滚流程\n", "5. 完成交接检查清单\n", "输出 JSON：{handoff{runbook{operations, troubleshooting}, monitoring{metrics, dashboards}, backup_recovery{backup_schedule, restore_steps, rto, rpo}, handoff_checklist{items, sign_off_required}}"),
                    vec![td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-handoff_result",
                    HashMap::from([("docs_updated".to_string(), "a-doc-update.content".to_string())]),
                    100.0, 3800.0,
                ),
                // 26. 事后复盘
                make_agent_node_with_inputs(
                    "a-post-review", "事后复盘",
                    concat!("你作为项目经理，进行重构事后复盘：\n", "1. 回顾重构过程，总结经验教训\n", "2. 评估重构效果（质量提升、性能改善、可维护性提升）\n", "3. 分析计划与实际的偏差\n", "4. 提出后续改进建议\n", "5. 生成最终复盘报告\n", "输出 JSON：{post_review{what_went_well, what_could_improve, metrics_before_after{quality, performance, maintainability}, follow_up_actions, final_assessment{success_level, key_achievements, remaining_risks}}}"),
                    vec![td("FileRead"), td("Grep")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-post-review_result",
                    HashMap::from([("handoff".to_string(), "a-handoff.content".to_string()), ("quality_gate".to_string(), "a-quality-gate.content".to_string())]),
                    100.0, 3950.0,
                ),
                // 27. 性能特性对比（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-performance-compare", "性能特性对比",
                    concat!("你作为性能对比专家，对比源语言和目标语言的性能特性：\n", "1. 对比关键路径的延迟（P50, P95, P99）\n", "2. 对比吞吐量（QPS, 并发处理数）\n", "3. 对比内存占用（稳态内存、峰值内存）\n", "4. 对比 CPU 使用率\n", "5. 识别性能退化路径并分析原因\n", "6. 提供性能优化建议\n", "输出 JSON：{performance_comparison{latency_comparison, throughput_comparison, memory_comparison, cpu_comparison, regressions_identified, performance_score{before_score, after_score, delta, overall_verdict}}}"),
                    vec![td("Bash"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-performance-compare_result",
                    HashMap::from([("quality_baseline".to_string(), "a-quality-baseline.content".to_string()), ("regression".to_string(), "a-regression.content".to_string()), ("integration".to_string(), "a-integration.content".to_string())]),
                    100.0, 4100.0,
                ),
                // 28. 类型系统映射分析（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-type-mapping", "类型系统映射分析",
                    concat!("你作为代码审计专家，分析源语言到目标语言的类型映射：\n", "1. 识别源代码中的所有类型（基本类型、集合类型、自定义类型、泛型）\n", "2. 根据映射规则自动生成目标类型映射\n", "3. 标注需要人工决策的转换点\n", "4. 生成类型映射表和决策清单\n", "输出 JSON：{type_mapping{auto_mapped[{source_type, target_type, confidence}], manual_review_required[{source_type, target_options, decision_guide, risk_level}], unmapped_types, mapping_coverage}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-type-mapping_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 4250.0,
                ),
                // 29. 内存安全审计（跨语言迁移专用）
                make_agent_node_with_inputs(
                    "a-memory-audit", "内存安全审计",
                    concat!("你作为内存安全专家，分析源代码的内存模型和迁移策略：\n", "1. 识别源代码中的内存管理模式（栈/堆分配、生命周期、所有权）\n", "2. 标记潜在的内存安全问题（悬垂指针、双重释放、缓冲区溢出等）\n", "3. 设计目标语言的内存管理策略\n", "4. 生成内存安全检查清单\n", "输出 JSON：{memory_audit{patterns_found, safety_issues, migration_strategy{ownership_model, pointer_choice_rules, lifetime_annotation_guide}, safety_checklist}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-memory-audit_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string()), ("type_mapping".to_string(), "a-type-mapping.content".to_string())]),
                    100.0, 4400.0,
                ),
                // 30. UI 组件映射分析（跨语言迁移+TypeScript专用）
                make_agent_node_with_inputs(
                    "a-ui-mapping", "UI 组件映射分析",
                    concat!("你作为 UI 框架迁移专家，分析源框架 UI 组件到目标框架的映射关系：\n", "1. 识别源代码中的所有 UI 组件（Widget、Layout、Dialog 等）\n", "2. 根据映射规则确定目标框架对应组件\n", "3. 标记需要人工设计决策的组件\n", "4. 规划 UI 状态管理方案\n", "5. 规划路由方案\n", "输出 JSON：{ui_mapping{components, layouts, custom_widgets, state_management, routing}, migration_complexity{total_components, auto_mappable, custom_implementation, complexity_score, estimated_effort}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ui-mapping_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string()), ("type_mapping".to_string(), "a-type-mapping.content".to_string())]),
                    100.0, 4550.0,
                ),
                // 31. 国际化迁移分析（跨语言迁移+TypeScript专用）
                make_agent_node_with_inputs(
                    "a-i18n-analysis", "国际化迁移分析",
                    concat!("你作为国际化迁移专家，分析源框架的国际化实现并规划迁移方案：\n", "1. 扫描源代码中所有国际化相关代码\n", "2. 判断源 i18n 状态（有现有 i18n/无现有 i18n）\n", "3. 识别所有硬编码字符串\n", "4. 规划 i18next 配置方案\n", "5. 设计翻译 key 的命名规范\n", "6. 规划翻译文件转换或从零搭建方案\n", "7. 后端消息分析：错误码体系、前后端消息传递模式\n", "输出 JSON：{i18n_analysis{current_state{has_i18n, hardcoded_strings, translation_files, backend_messages}, migration_plan{mode, target_languages, key_naming_convention, backend_i18n_pattern}, conversion_strategy, backend_strategy, complexity_assessment}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-i18n-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string()), ("ui_mapping".to_string(), "a-ui-mapping.content".to_string())]),
                    100.0, 4700.0,
                ),
                // 32. Afsim 仿真框架特征分析（Afsim/Qt 专用）
                make_agent_node_with_inputs(
                    "a-afsim-analysis", "Afsim 仿真框架特征分析",
                    concat!("你作为 Afsim 仿真框架迁移专家，分析源代码的 Afsim 特征：\n", "1. 识别 Afsim 核心类（WsfObject、WsfPlatform、WsfTrack 等）\n", "2. 识别 Afsim 数学类型（UtVec3、UtMatrix、UtQuaternion 等）\n", "3. 识别 Afsim 特定 Qt Widget\n", "4. 识别 Afsim 脚本系统和自定义 DSL\n", "5. 识别 DIS/HLA 等分布式仿真接口\n", "6. 评估迁移复杂度并给出优先级排序\n", "输出 JSON：{afsim_analysis{core_classes, math_types, custom_widgets, script_system, protocols, complexity_assessment, migration_priority}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-afsim-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 4850.0,
                ),
                // 33. Afsim 数学类型转换（Afsim 专用）
                make_agent_node_with_inputs(
                    "a-afsim-math-conversion", "Afsim 数学类型转换",
                    concat!("你作为数学计算库迁移专家，执行 Afsim 数学类型的转换：\n", "1. 将 UtVec2/UtVec3 转换为 nalgebra::Vector2/Vector3\n", "2. 将 UtMatrix3x3/UtMatrix4x4 转换为 nalgebra::Matrix3/Matrix4\n", "3. 将 UtQuaternion 转换为 nalgebra::UnitQuaternion\n", "4. 实现 UtEarth 坐标转换（WGS84/ECEF/NED）\n", "5. 迁移所有向量运算和矩阵运算\n", "6. 生成对应的 TypeScript 数学工具类\n", "输出 JSON：{math_conversion{converted_types, operations_converted, test_coverage, issues}}"),
                    vec![td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-afsim-math-conversion_result",
                    HashMap::from([("afsim_analysis".to_string(), "a-afsim-analysis.content".to_string())]),
                    100.0, 5000.0,
                ),
                // 34. Afsim 核心架构映射（Afsim 专用）
                make_agent_node_with_inputs(
                    "a-afsim-architecture-mapping", "Afsim 核心架构映射",
                    concat!("你作为架构映射专家，执行 Afsim 核心类到 AxAgent 架构的映射：\n", "1. 将 WsfObject 基类迁移为 Rust trait + TypeScript interface\n", "2. 将 WsfPlatform 迁移为 Agent struct + Zustand Store\n", "3. 将 WsfTrack 迁移为 Conversation 模型\n", "4. 将 WsfPlugin 迁移为 Tool trait\n", "5. 将 WsfScenario 迁移为 Orchestrator\n", "6. 迁移事件系统（信号槽 → EventEmitter/Zustand 订阅）\n", "7. 创建 Cargo workspace 结构\n", "输出 JSON：{architecture_mapping{classes_mapped, traits_defined, stores_created, workspace_structure{crate_members, dependencies}}, issues}"),
                    vec![td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-afsim-architecture-mapping_result",
                    HashMap::from([("afsim_analysis".to_string(), "a-afsim-analysis.content".to_string())]),
                    100.0, 5150.0,
                ),
                // 35. Afsim UI 组件增强迁移（Afsim 专用）
                make_agent_node_with_inputs(
                    "a-afsim-ui-enhancement", "Afsim UI 组件增强迁移",
                    concat!("你作为 UI 组件增强迁移专家，执行 Afsim 特定 Qt Widget 的迁移：\n", "1. 解析所有 .ui 文件（Qt Designer XML）\n", "2. 将 QDockWidget 系统迁移为 Segmented + 可拖拽面板\n", "3. 将 QGLWidget/QOpenGLWidget 迁移为 Three.js Canvas\n", "4. 将 QStyledItemDelegate 迁移为 Ant Design Column render\n", "5. 将 QAbstractItemModel 迁移为 Zustand Store\n", "6. 将复杂 QTreeWidget 迁移为 Tree + dnd-kit\n", "7. 将 QSplitter 迁移为 Resizable 面板组\n", "输出 JSON：{ui_enhancement{widgets_converted, custom_components_created, threejs_scene_setup, issues}}"),
                    vec![td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-afsim-ui-enhancement_result",
                    HashMap::from([("afsim_analysis".to_string(), "a-afsim-analysis.content".to_string())]),
                    100.0, 5300.0,
                ),
                // 36. 3D 可视化迁移（Qt OpenGL 专用）
                make_agent_node_with_inputs(
                    "a-3d-visualization-migration", "3D 可视化迁移",
                    concat!("你作为 3D 可视化迁移专家，执行 Qt OpenGL 到 Three.js 的迁移：\n", "1. 识别所有 Qt OpenGL 相关代码\n", "2. 将 OpenGL 调用映射到 Three.js API\n", "3. 迁移几何图元、光照系统、相机控制\n", "4. 实现 Afsim 3D 特定场景（战场态势、地球坐标、飞行器模型）\n", "5. 应用性能优化建议\n", "输出 JSON：{migration_3d{opengl_components_found, geometries_mapped, lighting_mapped, camera_mapped, afsim_scenarios_implemented, performance_optimizations_applied, issues}}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-3d-visualization-migration_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 5450.0,
                ),
                // 37. 脚本语言兼容迁移（Afsim 脚本专用）
                make_agent_node_with_inputs(
                    "a-script-compatibility-migration", "脚本语言兼容迁移",
                    concat!("你作为脚本语言迁移专家，执行 Afsim 自定义脚本到现代方案的迁移：\n", "1. 统计现有 Afsim 脚本文件数量和行数\n", "2. 识别脚本特性（对象创建、事件触发、循环、函数调用等）\n", "3. 选择迁移方案（推荐：完全重写为 TypeScript）\n", "4. 建立 TypeScript 核心 API\n", "5. 迁移试点脚本\n", "6. 规划批量迁移路径\n", "输出 JSON：{script_migration{scripts_analyzed, approach_selected, core_api_created, pilot_scripts_migrated, batch_migration_plan, issues}}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-script-compatibility-migration_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 5600.0,
                ),
                // 38. C# 到 Rust 迁移分析（C#→Rust 专用）
                make_agent_node_with_inputs(
                    "a-csharp-to-rust-analysis", "C# 到 Rust 迁移分析",
                    concat!("你作为 C# 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "1. 识别 C# 特有模式（async/await、泛型、LINQ、属性等）\n", "2. 规划 Rust async/await 转换\n", "3. 规划泛型到 Rust trait/const 泛型映射\n", "4. 规划 LINQ 查询到 Rust 迭代器适配器转换\n", "5. 规划 .NET 依赖注入到 Rust 状态模式转换\n", "6. 评估迁移复杂度\n", "输出 JSON：{csharp_rust_analysis{patterns_identified, async_conversion, generic_mapping, linq_mapping, estimated_effort}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-csharp-to-rust-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 5750.0,
                ),
                // 39. Java 到 TypeScript 迁移分析（Java→TypeScript 专用）
                make_agent_node_with_inputs(
                    "a-java-to-typescript-analysis", "Java 到 TypeScript 迁移分析",
                    concat!("你作为 Java 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "1. 识别 Java 特有模式（Stream API、注解、接口继承等）\n", "2. 规划 Spring Bean 到 NestJS Provider 转换\n", "3. 规划 JPA 到 Prisma/TypeORM 转换\n", "4. 规划 Stream API 到 RxJS/Array 方法转换\n", "5. 规划注解到 TypeScript 装饰器转换\n", "6. 评估迁移复杂度\n", "输出 JSON：{java_typescript_analysis{patterns_identified, spring_mapping, jpa_mapping, stream_mapping, estimated_effort}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-java-to-typescript-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 5900.0,
                ),
                // 40. DIS/HLA 分布式仿真协议迁移（DIS/HLA 专用）
                make_agent_node_with_inputs(
                    "a-dis-hla-migration", "DIS/HLA 分布式仿真协议迁移",
                    concat!("你作为分布式仿真协议迁移专家，执行 DIS/HLA 协议的迁移：\n", "1. 分析现有 DIS/HLA 实现\n", "2. 实现 DIS PDU 数据结构（Entity State、Fire、Explosion 等）\n", "3. 实现 DIS 二进制编解码\n", "4. 实现 UDP/TCP 网络传输\n", "5. 实现 Tauri 事件推送（DIS → React 前端）\n", "6. 实现 HLA 核心接口（RtiAmbassador、时间管理、发布/订阅）\n", "输出 JSON：{dis_hla_migration{dis_analysis, dis_implementation{pdu_structures, codec_implementation, transport_implementation}, hla_implementation{approach_selected, rti_interface, time_manager, pub_sub}, test_plan, issues}}"),
                    vec![td("Grep"), td("FileRead"), td("FileWrite")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-dis-hla-migration_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 6050.0,
                ),
                // 41. Python 到 Rust 迁移分析（Python→Rust 专用）
                make_agent_node_with_inputs(
                    "a-python-to-rust-analysis", "Python 到 Rust 迁移分析",
                    concat!("你作为 Python 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "1. 识别 Python 特有模式（动态类型、GIL、反射、装饰器等）\n", "2. 规划 Python 动态类型到 Rust 静态类型转换\n", "3. 规划 Python 异步（asyncio）到 Rust async/await 转换\n", "4. 规划 Python 装饰器到 Rust 宏/trait 转换\n", "5. 评估迁移复杂度\n", "输出 JSON：{python_rust_analysis{patterns_identified, type_mapping, async_conversion, decorator_mapping, estimated_effort}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-python-to-rust-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 6200.0,
                ),
                // 42. Python 到 TypeScript 迁移分析（Python→TypeScript 专用）
                make_agent_node_with_inputs(
                    "a-python-to-typescript-analysis", "Python 到 TypeScript 迁移分析",
                    concat!("你作为 Python 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "1. 识别 Python 特有模式（动态类型、装饰器、列表推导式等）\n", "2. 规划 Python 动态类型到 TypeScript 静态类型转换\n", "3. 规划 Python 装饰器到 TypeScript 装饰器转换\n", "4. 规划 Python 数据类到 TypeScript interface/class 转换\n", "5. 规划 Python 异步到 TypeScript async/await 转换\n", "6. 评估迁移复杂度\n", "输出 JSON：{python_ts_analysis{patterns_identified, type_mapping, decorator_mapping, data_class_mapping, async_conversion, estimated_effort}}"),
                    vec![td("Grep"), td("FileRead")],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-python-to-typescript-analysis_result",
                    HashMap::from([("asset_scan".to_string(), "a-asset-scan.content".to_string())]),
                    100.0, 6350.0,
                ),
                make_end(100.0, 6500.0),
            ],
            vec![
                edge("e-trigger-asset-scan", "trigger", "a-asset-scan"),
                edge("e-asset-scan-dep-graph", "a-asset-scan", "a-dep-graph"),
                edge("e-dep-graph-complexity", "a-dep-graph", "a-complexity"),
                edge("e-complexity-behavior-snapshot", "a-complexity", "a-behavior-snapshot"),
                edge("e-behavior-snapshot-tacit-knowledge", "a-behavior-snapshot", "a-tacit-knowledge"),
                edge("e-tacit-knowledge-smell-detect", "a-tacit-knowledge", "a-smell-detect"),
                edge("e-smell-detect-coupling-analyze", "a-smell-detect", "a-coupling-analyze"),
                edge("e-coupling-analyze-risk-assess", "a-coupling-analyze", "a-risk-assess"),
                edge("e-risk-assess-strategy", "a-risk-assess", "a-strategy"),
                edge("e-strategy-batch-plan", "a-strategy", "a-batch-plan"),
                edge("e-batch-plan-change-merge", "a-batch-plan", "a-change-merge"),
                edge("e-change-merge-quality-baseline", "a-change-merge", "a-quality-baseline"),
                edge("e-quality-baseline-rollback", "a-quality-baseline", "a-rollback"),
                edge("e-rollback-integration-plan", "a-rollback", "a-integration-plan"),
                edge("e-integration-plan-pre-review", "a-integration-plan", "a-pre-review"),
                edge("e-pre-review-execute", "a-pre-review", "a-execute"),
                edge("e-execute-idiomatic-convert", "a-execute", "a-idiomatic-convert"),
                edge("e-idiomatic-convert-framework-validate", "a-idiomatic-convert", "a-framework-validate"),
                edge("e-framework-validate-equivalence-check", "a-framework-validate", "a-equivalence-check"),
                edge("e-equivalence-check-change-gate", "a-equivalence-check", "a-change-gate"),
                edge("e-change-gate-regression", "a-change-gate", "a-regression"),
                edge("e-regression-integration", "a-regression", "a-integration"),
                edge("e-integration-quality-gate", "a-integration", "a-quality-gate"),
                edge("e-quality-gate-doc-update", "a-quality-gate", "a-doc-update"),
                edge("e-doc-update-handoff", "a-doc-update", "a-handoff"),
                edge("e-handoff-post-review", "a-handoff", "a-post-review"),
                edge("e-post-review-performance-compare", "a-post-review", "a-performance-compare"),
                edge("e-performance-compare-type-mapping", "a-performance-compare", "a-type-mapping"),
                edge("e-type-mapping-memory-audit", "a-type-mapping", "a-memory-audit"),
                edge("e-memory-audit-ui-mapping", "a-memory-audit", "a-ui-mapping"),
                edge("e-ui-mapping-i18n-analysis", "a-ui-mapping", "a-i18n-analysis"),
                edge("e-i18n-analysis-afsim-analysis", "a-i18n-analysis", "a-afsim-analysis"),
                edge("e-afsim-analysis-math-conversion", "a-afsim-analysis", "a-afsim-math-conversion"),
                edge("e-afsim-math-conversion-architecture-mapping", "a-afsim-math-conversion", "a-afsim-architecture-mapping"),
                edge("e-afsim-architecture-mapping-ui-enhancement", "a-afsim-architecture-mapping", "a-afsim-ui-enhancement"),
                edge("e-afsim-ui-enhancement-3d-visualization", "a-afsim-ui-enhancement", "a-3d-visualization-migration"),
                edge("e-3d-visualization-script-compatibility", "a-3d-visualization-migration", "a-script-compatibility-migration"),
                edge("e-script-compatibility-csharp-rust", "a-script-compatibility-migration", "a-csharp-to-rust-analysis"),
                edge("e-csharp-rust-java-ts", "a-csharp-to-rust-analysis", "a-java-to-typescript-analysis"),
                edge("e-java-ts-dis-hla", "a-java-to-typescript-analysis", "a-dis-hla-migration"),
                edge("e-dis-hla-python-rust", "a-dis-hla-migration", "a-python-to-rust-analysis"),
                edge("e-python-rust-python-ts", "a-python-to-rust-analysis", "a-python-to-typescript-analysis"),
                edge("e-python-ts-end", "a-python-to-typescript-analysis", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    tracing::info!("[opc-workflows] 领域 engineering: {seeded}/13 个工作流已种子化");

    Ok(seeded)
}
