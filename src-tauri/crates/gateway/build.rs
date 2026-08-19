// SPDX-License-Identifier: AGPL-3.0-only

//! 构建时 select! 宏分支顺序检查
//!
//! 本脚本在构建时静态检查 `tokio::select!` 宏的分支顺序，
//! 确保关键并发逻辑的优先级不被意外修改。
//!
//! ## 业务规则
//!
//! 1. **realtime.rs 主循环**: 第一分支必须是 idle timeout (`sleep_until`)
//!    - 确保客户端断开或超时能被优先检测和处理
//!
//! 2. **realtime_ticket.rs sweeper**: 第一分支必须是 `tick.tick()`
//!    - 确保定时清理过期票据的逻辑按预期执行
//!    - 必须包含 shutdown 分支以支持优雅关闭

use std::process;

fn main() {
    let is_test =
        std::env::var("CARGO_CFG_TARGET_FEATURE").map(|v| v.contains("test")).unwrap_or(false);

    if !is_test {
        if let Err(e) = check_select_macros() {
            eprintln!("构建错误: select! 宏分支检查失败\n{}", e);
            eprintln!("");
            eprintln!("业务规则要求分支顺序保持不变，请检查上述违规项。");
            eprintln!("如确需修改分支顺序，请与架构师评审后更新 build.rs 中的规则。");
            process::exit(1);
        }
    }

    println!("cargo:rerun-if-changed=src/");
}

fn check_select_macros() -> Result<(), String> {
    let source_files = vec!["src/realtime.rs", "src/realtime_ticket.rs"];

    let mut violations = Vec::new();

    for file_path in &source_files {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("读取文件失败 {}: {}", file_path, e))?;

        if file_path.contains("realtime.rs") {
            check_realtime_selects(&content, file_path, &mut violations);
        }

        if file_path.contains("realtime_ticket.rs") {
            check_ticket_sweeper_selects(&content, file_path, &mut violations);
        }
    }

    if !violations.is_empty() {
        return Err(format!("违规数量: {}\n\n{}", violations.len(), violations.join("\n\n")));
    }

    println!("✅ select! 宏分支检查通过");
    Ok(())
}

/// 检查 realtime.rs 中的 select! 宏
///
/// 业务规则:
/// - 会话主循环的 select! 第一分支必须是 idle timeout (sleep_until)
/// - 这确保客户端断开或超时能被优先处理
fn check_realtime_selects(content: &str, file_path: &str, violations: &mut Vec<String>) {
    let lines: Vec<&str> = content.lines().collect();

    // 找到每个 select! 块，并检查其第一个 arm
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("select!") {
            // 提取 select! 块的第一行有效 arm
            if let Some(first_arm) = extract_first_arm(lines.as_slice(), i) {
                let is_valid = first_arm.contains("sleep_until")
                    || first_arm.contains("tick.tick()")
                    || first_arm.contains("shutdown")
                    || first_arm.contains("Close");

                if !is_valid {
                    violations.push(format!(
                        "❌ {}:{} - select! 第一分支不符合业务规则\n\
                         位置: 第 {} 行附近\n\
                         第一分支: {}\n\
                         要求: 第一分支必须是 idle timeout (sleep_until) 或心跳 (tick.tick())\n\
                         原因: 确保超时/心跳/关闭信号能被优先检测和处理",
                        file_path,
                        i + 1,
                        i + 1,
                        first_arm.trim()
                    ));
                }
            }
        }
        i += 1;
    }
}

/// 检查 realtime_ticket.rs 中的 select! 宏
///
/// 业务规则:
/// - ticket sweeper 的 select! 第一分支必须是 tick.tick()
/// - 必须包含 shutdown 分支以支持优雅关闭
fn check_ticket_sweeper_selects(content: &str, file_path: &str, violations: &mut Vec<String>) {
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("select!") {
            // 检查第一分支
            if let Some(first_arm) = extract_first_arm(lines.as_slice(), i) {
                if !first_arm.contains("tick.tick()") {
                    violations.push(format!(
                        "❌ {}:{} - ticket_sweeper select! 第一分支不符合业务规则\n\
                         位置: 第 {} 行附近\n\
                         第一分支: {}\n\
                         要求: 第一分支必须是 tick.tick()\n\
                         原因: 确保定时清理过期票据的逻辑按预期执行",
                        file_path,
                        i + 1,
                        i + 1,
                        first_arm.trim()
                    ));
                }
            }

            // 检查是否包含 shutdown 分支
            let has_shutdown = has_arm_containing(lines.as_slice(), i, "shutdown");
            let has_changed = has_arm_containing(lines.as_slice(), i, "changed()");

            if !has_shutdown && !has_changed {
                violations.push(format!(
                    "❌ {}:{} - ticket_sweeper select! 缺少 shutdown 分支\n\
                     位置: 第 {} 行附近\n\
                     要求: select! 必须包含 shutdown_rx.changed() 分支\n\
                     原因: 确保后台任务能优雅关闭",
                    file_path,
                    i + 1,
                    i + 1
                ));
            }
        }
        i += 1;
    }
}

/// 从 select! 开始位置提取第一个 arm 的条件部分
/// 返回 None 如果无法解析
fn extract_first_arm(lines: &[&str], select_line_idx: usize) -> Option<String> {
    let mut brace_depth = 0i32;
    let mut arm_content = String::new();

    for j in select_line_idx..lines.len() {
        let line = lines[j];
        let trimmed = line.trim();

        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with("//") {
            if brace_depth > 0 {
                arm_content.push('\n');
            }
            continue;
        }

        // 检查大括号
        let open_count = trimmed.chars().filter(|&c| c == '{').count();
        let close_count = trimmed.chars().filter(|&c| c == '}').count();
        brace_depth += open_count as i32 - close_count as i32;

        // 如果还有 '{' 需要进入，跳过 select! 行本身
        if j == select_line_idx && open_count > 0 {
            // 提取 { 后面的内容作为 arm 开始
            if let Some(brace_pos) = trimmed.find('{') {
                let after = trimmed[brace_pos + 1..].trim();
                if !after.is_empty() && !after.starts_with('}') {
                    arm_content.push_str(after);
                    arm_content.push(' ');
                }
            }
            if brace_depth <= 0 {
                break;
            }
            continue;
        }

        // 进入 select! 块后收集 arm 条件
        if brace_depth > 0 {
            // 检测 => 标记 arm 条件结束
            if trimmed.contains("=>") {
                if !arm_content.trim().is_empty() {
                    return Some(arm_content.trim().to_string());
                }
                arm_content.clear();
                break;
            } else {
                arm_content.push_str(trimmed);
                arm_content.push(' ');
            }
        }

        // 检查是否退出 select! 块
        if brace_depth <= 0 && j > select_line_idx {
            break;
        }
    }

    if arm_content.trim().is_empty() {
        None
    } else {
        Some(arm_content.trim().to_string())
    }
}

/// 检查 select! 块中是否有包含特定字符串的 arm
fn has_arm_containing(lines: &[&str], select_line_idx: usize, pattern: &str) -> bool {
    let mut brace_depth = 0i32;
    let mut in_select = false;

    for j in select_line_idx..lines.len() {
        let line = lines[j];
        let trimmed = line.trim();

        if !in_select {
            if trimmed.contains('{') {
                in_select = true;
                let open_count = trimmed.chars().filter(|&c| c == '{').count();
                let close_count = trimmed.chars().filter(|&c| c == '}').count();
                brace_depth = open_count as i32 - close_count as i32;

                // 检查同一行是否包含目标模式
                if trimmed.contains(pattern) {
                    return true;
                }
            }
            continue;
        }

        if trimmed.contains(pattern) && !trimmed.starts_with("//") {
            return true;
        }

        let open_count = trimmed.chars().filter(|&c| c == '{').count();
        let close_count = trimmed.chars().filter(|&c| c == '}').count();
        brace_depth += open_count as i32 - close_count as i32;

        if brace_depth <= 0 {
            break;
        }
    }

    false
}
