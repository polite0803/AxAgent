// SPDX-License-Identifier: AGPL-3.0-only

//! 计算机控制（C4）授权闸门
//!
//! 设计意图：默认关闭，仅当用户在前端显式授权后才开启。所有计算机控制入口
//! （Tauri 命令、AI 工具路径、视觉分析截屏/点击命令）都必须经过本模块的校验，
//! 避免任一入口绕过主开关。
//!
//! 该状态曾被定义在 `commands/computer_control.rs` 的局部 `static` 中，导致
//! `ComputerUseTool` 等 AI 工具路径完全不经过校验（见审计报告 #1/#2）。现将闸门
//! 下沉到 `kit` 这一被命令与工具共同依赖的层次，成为唯一权威来源。

use std::sync::atomic::{AtomicBool, Ordering};

static COMPUTER_CONTROL_GRANTED: AtomicBool = AtomicBool::new(false);

/// 授予计算机控制权限（由前端授权流程调用）。
pub fn grant_computer_control() {
    COMPUTER_CONTROL_GRANTED.store(true, Ordering::SeqCst);
    tracing::info!("computer_control permission granted");
}

/// 撤销计算机控制权限。
pub fn revoke_computer_control() {
    COMPUTER_CONTROL_GRANTED.store(false, Ordering::SeqCst);
    tracing::info!("computer_control permission revoked");
}

/// 查询当前是否已授权。
pub fn is_computer_control_granted() -> bool {
    COMPUTER_CONTROL_GRANTED.load(Ordering::SeqCst)
}

/// SECURITY (C4): 权限校验闸门。
///
/// 未授权时返回拒绝错误，防止未授权截屏 / 鼠标键盘自动化。返回 `Result<(), String>`
/// 以便被命令（`Result<_, String>`）与工具（`map_err`）直接复用。
pub fn ensure_computer_control_granted() -> Result<(), String> {
    if is_computer_control_granted() {
        Ok(())
    } else {
        Err("Permission denied: computer_control capability has not been granted. \
             The user must explicitly authorize computer control access before \
             screen capture, mouse/keyboard automation commands can be used."
            .to_string())
    }
}
