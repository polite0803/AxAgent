// SPDX-License-Identifier: AGPL-3.0-only

//! 核心数据结构断言锁
//!
//! 本模块使用 static_assertions 在编译时锁定关键结构的内存布局。
//! 这些断言确保：
//! 1. Tauri IPC DTO 的序列化格式稳定
//! 2. 跨线程共享结构的大小在可控范围内
//! 3. 防止 AI 辅助编码时意外修改核心结构
//!
//! ⚠️ 重要：修改这些断言前必须经过人工评审！
//!
//! ## 跨平台说明
//!
//! 尺寸断言仅在 64 位平台（`target_pointer_width = "64"`）上启用。
//! 原因：`String` / `Vec` 等堆分配类型的大小取决于指针宽度：
//! - 64 位平台：String = 24 字节（ptr + len + capacity，各 8 字节）
//! - 32 位平台：String = 12 字节（ptr + len + capacity，各 4 字节）
//!
//! 移动端 ARM（32 位）与桌面端（64 位）的内存布局不同，但：
//! - Tauri IPC 使用 JSON 序列化，不涉及内存布局的跨平台传输
//! - 序列化后的字段（如 `session_id`、`role`、`blocks`）不受指针宽度影响
//! - 开发 / CI 主平台为 64 位，因此在 64 位上锁定尺寸已足以守护 DTO 稳定性
//!
//! ## 如何更新锁定值
//!
//! 1. 在 64 位平台上运行尺寸检测测试：
//!    ```bash
//!    cargo test -p axagent-harness -- inspect_dto_sizes -- --nocapture
//!    ```
//! 2. 复制输出的尺寸值
//! 3. 更新下方的 assert_eq_size! 宏（仅限 `#[cfg(target_pointer_width = "64")]` 块内）
//! 4. 提交 PR 并附原因说明

// ============================================================================
// 基础类型尺寸锁定（确保底层类型稳定性）
// ============================================================================

// bool 基础尺寸（平台无关）
static_assertions::assert_eq_size!(bool, [u8; 1]);

// u32 基础尺寸（平台无关）
static_assertions::assert_eq_size!(u32, [u8; 4]);

// u64 基础尺寸（平台无关）
static_assertions::assert_eq_size!(u64, [u8; 8]);

// f64 基础尺寸（平台无关）
static_assertions::assert_eq_size!(f64, [u8; 8]);

// ============================================================================
// 64 位平台专属尺寸锁定
//
// 以下断言依赖 String / Vec 等堆分配类型的 64 位布局（ptr=8B）。
// 在 32 位平台上这些值完全不同，因此仅在 target_pointer_width = "64" 时启用。
// ============================================================================
#[cfg(target_pointer_width = "64")]
mod size_locks_64 {
    // String 基础尺寸（在 64 位平台上为 24 字节）
    static_assertions::assert_eq_size!(String, [u8; 24]);

    // ========================================================================
    // Agent 相关 DTO 尺寸锁定（锁定于 2026-08-20，平台: 64-bit Windows）
    // ========================================================================

    // AgentCapability: name(String) + description(String)
    static_assertions::assert_eq_size!(crate::agent::AgentCapability, [u8; 48]);

    // AgentExecuteRequest: goal(String) + context(Option<String>) + max_steps(Option<u32>)
    static_assertions::assert_eq_size!(crate::agent::AgentExecuteRequest, [u8; 56]);

    // AgentResult: output(String) + success(bool) + steps_taken(u32)
    static_assertions::assert_eq_size!(crate::agent::AgentResult, [u8; 56]); // +Option<String> session_id

    // PlanStep: description(String) + agent(Option<String>)
    static_assertions::assert_eq_size!(crate::agent::PlanStep, [u8; 48]);

    // AgentPlan: steps(Vec<PlanStep>)
    static_assertions::assert_eq_size!(crate::agent::AgentPlan, [u8; 24]);

    // AgentInfo: name + description + capabilities(Vec<AgentCapability>)
    static_assertions::assert_eq_size!(crate::agent::AgentInfo, [u8; 72]);

    // ========================================================================
    // Conversation 相关 DTO 尺寸锁定
    // ========================================================================

    // TokenUsage: input_tokens + output_tokens + cache_creation_input_tokens +
    //             cache_read_input_tokens + cache_miss_input_tokens(Option<u32>)
    static_assertions::assert_eq_size!(crate::conversation_model::TokenUsage, [u8; 24]);

    // ContentBlock: 枚举（最大变体包含 tool_use_id + tool_name + output + is_error + 对齐）
    static_assertions::assert_eq_size!(crate::conversation_model::ContentBlock, [u8; 80]);

    // ConversationMessage: role(MessageRole) + blocks(Vec<ContentBlock>) + usage(Option<TokenUsage>)
    static_assertions::assert_eq_size!(crate::conversation_model::ConversationMessage, [u8; 56]);

    // SessionInfo: session_id + user_id + title(Option<String>) + timestamps + token_usage
    static_assertions::assert_eq_size!(crate::conversation_model::SessionInfo, [u8; 112]);

    // ========================================================================
    // Workflow 相关 DTO 尺寸锁定
    // ========================================================================

    // Position: x(f64) + y(f64)
    static_assertions::assert_eq_size!(crate::workflow_types::Position, [u8; 16]);

    // BackoffType: 枚举（无数据字段）
    static_assertions::assert_eq_size!(crate::workflow_types::BackoffType, [u8; 1]);

    // RetryConfig: enabled + max_retries + backoff_type + base_delay_ms + max_delay_ms
    static_assertions::assert_eq_size!(crate::workflow_types::RetryConfig, [u8; 24]);

    // CompensationStrategy: 枚举（无数据字段）
    static_assertions::assert_eq_size!(crate::workflow_types::CompensationStrategy, [u8; 1]);

    // CompensationConfig: strategy + compensation_nodes(Vec<String>)
    static_assertions::assert_eq_size!(crate::workflow_types::CompensationConfig, [u8; 32]);

    // NodeKind: 枚举（无数据字段）
    static_assertions::assert_eq_size!(crate::workflow_types::NodeKind, [u8; 1]);

    // Variable: name + var_type + value(JsonValue) + description(Option<String>) + is_secret
    static_assertions::assert_eq_size!(crate::workflow_types::Variable, [u8; 112]);

    // WorkflowNodeBase: id + title + description + position + retry + timeout +
    //                   enabled + parent_id + compensation + continue_on_fail
    static_assertions::assert_eq_size!(crate::workflow_types::WorkflowNodeBase, [u8; 192]);
}

// ============================================================================
// 尺寸检测工具（仅在测试时编译，始终可用）
// ============================================================================

#[cfg(test)]
mod size_inspector {
    /// 打印所有关键 DTO 的当前尺寸
    ///
    /// 使用方法:
    /// ```bash
    /// cargo test -p axagent-harness -- inspect_dto_sizes -- --nocapture
    /// ```
    #[test]
    fn inspect_dto_sizes() {
        println!("=== DTO 尺寸检测报告 ===");
        println!("指针宽度: {} 位", std::mem::size_of::<*const ()>() * 8);

        // 基础类型
        println!();
        println!("--- 基础类型 ---");
        println!("String:                    {} bytes", std::mem::size_of::<String>());
        println!("Option<String>:            {} bytes", std::mem::size_of::<Option<String>>());
        println!("Option<u32>:               {} bytes", std::mem::size_of::<Option<u32>>());
        println!("bool:                      {} bytes", std::mem::size_of::<bool>());
        println!("u32:                       {} bytes", std::mem::size_of::<u32>());
        println!("u64:                       {} bytes", std::mem::size_of::<u64>());
        println!("f64:                       {} bytes", std::mem::size_of::<f64>());

        // Agent 相关
        println!();
        println!("--- Agent DTO ---");
        println!(
            "AgentCapability:           {} bytes",
            std::mem::size_of::<crate::agent::AgentCapability>()
        );
        println!(
            "AgentExecuteRequest:       {} bytes",
            std::mem::size_of::<crate::agent::AgentExecuteRequest>()
        );
        println!(
            "AgentResult:               {} bytes",
            std::mem::size_of::<crate::agent::AgentResult>()
        );
        println!(
            "PlanStep:                  {} bytes",
            std::mem::size_of::<crate::agent::PlanStep>()
        );
        println!(
            "AgentPlan:                 {} bytes",
            std::mem::size_of::<crate::agent::AgentPlan>()
        );
        println!(
            "AgentInfo:                 {} bytes",
            std::mem::size_of::<crate::agent::AgentInfo>()
        );

        // Conversation 相关
        println!();
        println!("--- Conversation DTO ---");
        println!(
            "TokenUsage:                {} bytes",
            std::mem::size_of::<crate::conversation_model::TokenUsage>()
        );
        println!(
            "ContentBlock:              {} bytes",
            std::mem::size_of::<crate::conversation_model::ContentBlock>()
        );
        println!(
            "ConversationMessage:       {} bytes",
            std::mem::size_of::<crate::conversation_model::ConversationMessage>()
        );
        println!(
            "SessionInfo:               {} bytes",
            std::mem::size_of::<crate::conversation_model::SessionInfo>()
        );

        // Workflow 相关
        println!();
        println!("--- Workflow DTO ---");
        println!(
            "Position:                  {} bytes",
            std::mem::size_of::<crate::workflow_types::Position>()
        );
        println!(
            "BackoffType:               {} bytes",
            std::mem::size_of::<crate::workflow_types::BackoffType>()
        );
        println!(
            "RetryConfig:               {} bytes",
            std::mem::size_of::<crate::workflow_types::RetryConfig>()
        );
        println!(
            "CompensationStrategy:      {} bytes",
            std::mem::size_of::<crate::workflow_types::CompensationStrategy>()
        );
        println!(
            "CompensationConfig:        {} bytes",
            std::mem::size_of::<crate::workflow_types::CompensationConfig>()
        );
        println!(
            "NodeKind:                  {} bytes",
            std::mem::size_of::<crate::workflow_types::NodeKind>()
        );
        println!(
            "Variable:                  {} bytes",
            std::mem::size_of::<crate::workflow_types::Variable>()
        );
        println!(
            "WorkflowNodeBase:          {} bytes",
            std::mem::size_of::<crate::workflow_types::WorkflowNodeBase>()
        );

        println!();
        println!("=== 报告结束 ===");
    }
}
