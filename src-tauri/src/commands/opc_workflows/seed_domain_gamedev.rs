// SPDX-License-Identifier: AGPL-3.0-only

//! 游戏开发（gamedev）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-gd-concept: 游戏概念设计
//! - wf-gd-prototype: 游戏原型
//! - wf-gd-qa: 游戏测试

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化游戏开发领域的全部工作流
pub(crate) async fn seed_domain_gamedev_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 游戏概念设计
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gd-concept",
            "游戏概念设计",
            "从想法到完整的游戏设计文档",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-gd-idea",
                    "概念生成",
                    "生成游戏核心玩法和概念",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-idea_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-gd-design",
                    "游戏设计",
                    "设计游戏机制、关卡、角色",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-design_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-gd-doc",
                    "文档",
                    "编写游戏设计文档",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-doc_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-gd-idea", "trigger", "a-gd-idea"),
                edge("e-a-gd-idea-a-gd-design", "a-gd-idea", "a-gd-design"),
                edge("e-a-gd-design-a-gd-doc", "a-gd-design", "a-gd-doc"),
                edge("e-a-gd-doc-end", "a-gd-doc", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 游戏原型
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gd-prototype",
            "游戏原型",
            "快速搭建可玩原型验证核心机制",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-gd-proto-core",
                    "核心机制",
                    "实现核心玩法和控制",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-proto-core_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-gd-proto-test",
                    "玩法测试",
                    "测试核心机制可玩性",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-proto-test_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-gd-proto-iterate",
                    "迭代",
                    "根据测试反馈优化",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-proto-iterate_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-gd-proto-core", "trigger", "a-gd-proto-core"),
                edge("e-a-gd-proto-core-a-gd-proto-test", "a-gd-proto-core", "a-gd-proto-test"),
                edge(
                    "e-a-gd-proto-test-a-gd-proto-iterate",
                    "a-gd-proto-test",
                    "a-gd-proto-iterate",
                ),
                edge("e-a-gd-proto-iterate-end", "a-gd-proto-iterate", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 游戏测试
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gd-qa",
            "游戏测试",
            "全面测试游戏功能和体验",
            "🎮",
            vec!["opc".to_string(), "gamedev".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-gd-qa-functional",
                    "功能测试",
                    "测试游戏功能和系统",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-qa-functional_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-gd-qa-balance",
                    "平衡测试",
                    "测试数值平衡和难度曲线",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-qa-balance_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-gd-qa-ux",
                    "体验测试",
                    "测试用户体验和引导",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-gd-qa-ux_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-gd-qa-functional", "trigger", "a-gd-qa-functional"),
                edge(
                    "e-a-gd-qa-functional-a-gd-qa-balance",
                    "a-gd-qa-functional",
                    "a-gd-qa-balance",
                ),
                edge("e-a-gd-qa-balance-a-gd-qa-ux", "a-gd-qa-balance", "a-gd-qa-ux"),
                edge("e-a-gd-qa-ux-end", "a-gd-qa-ux", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
