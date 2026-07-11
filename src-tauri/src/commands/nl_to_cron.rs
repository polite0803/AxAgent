// SPDX-License-Identifier: AGPL-3.0-only

/// 将自然语言描述的定时任务转换为 cron 表达式。
/// 基于规则匹配，无需 LLM 调用。
#[tauri::command]
pub async fn nl_to_cron(natural_language: String) -> Result<String, String> {
    let text = natural_language.trim().to_lowercase();
    if text.is_empty() {
        return Err("请输入定时任务描述".to_string());
    }

    if text.contains("每天") || text.contains("每日") {
        return daily_cron(&text);
    }
    if text.contains("每周") || text.contains("每星期") || text.contains("工作日") {
        return weekly_cron(&text);
    }
    if text.contains("每月") || text.contains("每个月") {
        return monthly_cron(&text);
    }
    if text.contains("每") && (text.contains("分钟") || text.contains("小时")) {
        return interval_cron(&text);
    }

    if let Some(cron) = extract_time_cron(&text) {
        return Ok(cron);
    }

    Err(format!(
        "无法解析: '{}'\n示例：\n- 每天早上9点 → 0 9 * * *\n- 每30分钟 → */30 * * * *\n- 每周一至周五下午3点半 → 30 15 * * 1-5\n- 每月1号和15号零点 → 0 0 1,15 * *",
        natural_language
    ))
}

/// 从文本中提取小时和分钟，返回 (hour, minute)
fn extract_hour_minute(text: &str) -> Option<(u32, u32)> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    // 查找 "X点Y分" 或 "X点Y" 模式
    for i in 0..len {
        if chars[i] == '点' {
            // 提取点前面的数字
            let hour_str: String = chars[..i].iter().rev().take_while(|c| c.is_ascii_digit()).collect::<String>().chars().rev().collect();
            let hour: u32 = hour_str.parse().ok()?;

            // 检查后面是否有 "半" 或 "分"
            let after = &chars[i + 1..];
            if after.starts_with(&['半']) {
                return Some((hour, 30));
            }

            // 提取分钟数字
            let min_str: String = after.iter().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
            let min: u32 = if min_str.is_empty() { 0 } else { min_str.parse().ok()? };
            return Some((hour, min));
        }
    }

    // 查找 "H:MM" 模式
    for i in 0..len {
        if chars[i] == ':' && i > 0 && i + 1 < len {
            let hour_str: String = chars[..i].iter().collect();
            let min_str: String = chars[i + 1..i + 3].iter().collect();
            if let (Ok(h), Ok(m)) = (hour_str.parse::<u32>(), min_str.parse::<u32>()) {
                return Some((h, m));
            }
        }
    }

    None
}

/// 从文本中提取时间并生成每天 cron 表达式
fn extract_time_cron(text: &str) -> Option<String> {
    let (hour, min) = extract_hour_minute(text)?;
    let hour = adjust_hour(text, hour);
    Some(format!("{} {} * * *", min, hour))
}

/// 根据上午/下午/晚上调整小时
fn adjust_hour(text: &str, hour: u32) -> u32 {
    let adjusted = if text.contains("下午") || text.contains("晚上") {
        if hour < 12 { hour + 12 } else { hour }
    } else if text.contains("凌晨") && hour >= 12 {
        hour - 12
    } else {
        hour
    };
    adjusted.min(23)
}

fn daily_cron(text: &str) -> Result<String, String> {
    if let Some((hour, min)) = extract_hour_minute(text) {
        let hour = adjust_hour(text, hour);
        Ok(format!("{} {} * * *", min, hour))
    } else {
        Ok("0 9 * * *".to_string())
    }
}

fn weekly_cron(text: &str) -> Result<String, String> {
    let days = if text.contains("工作日") || text.contains("周一至周五") {
        "1-5"
    } else if text.contains("周末") {
        "6,0"
    } else if text.contains("周一") { "1" }
    else if text.contains("周二") { "2" }
    else if text.contains("周三") { "3" }
    else if text.contains("周四") { "4" }
    else if text.contains("周五") { "5" }
    else if text.contains("周六") { "6" }
    else if text.contains("周日") || text.contains("星期天") || text.contains("星期日") { "0" }
    else { "*" };

    if let Some((hour, min)) = extract_hour_minute(text) {
        let hour = adjust_hour(text, hour);
        Ok(format!("{} {} * * {}", min, hour, days))
    } else {
        Ok(format!("0 9 * * {}", days))
    }
}

fn monthly_cron(text: &str) -> Result<String, String> {
    // 解析 "X号" 模式
    let mut days = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i + 1] == '号' && chars[i].is_ascii_digit() {
            let mut num_str = String::new();
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                num_str.push(chars[j]);
                j += 1;
            }
            days.push(num_str);
        }
    }
    let days_str = if days.is_empty() { "1".to_string() } else { days.join(",") };

    if let Some((hour, min)) = extract_hour_minute(text) {
        let hour = adjust_hour(text, hour);
        Ok(format!("{} {} {} * *", min, hour, days_str))
    } else {
        Ok(format!("0 0 {} * *", days_str))
    }
}

fn interval_cron(text: &str) -> Result<String, String> {
    // 解析 "每N分钟"
    for (i, c) in text.char_indices() {
        if c == '分' && i >= 1 {
            let before = &text[..i];
            if let Some(pos) = before.rfind('每') {
                let num_part = &before[pos + '每'.len_utf8()..].trim();
                if let Ok(n) = num_part.parse::<u32>() {
                    if n > 0 && n <= 1440 {
                        return Ok(format!("*/{} * * * *", n));
                    }
                }
            }
        }
        // 解析 "每N小时"
        if c == '小' && text[i..].starts_with("小时") && i >= 1 {
            let before = &text[..i];
            if let Some(pos) = before.rfind('每') {
                let num_part = &before[pos + '每'.len_utf8()..].trim();
                if let Ok(n) = num_part.parse::<u32>() {
                    if n > 0 && n <= 24 {
                        return Ok(format!("0 */{} * * *", n));
                    }
                }
            }
        }
    }

    Ok("*/30 * * * *".to_string())
}
