use serde::{Deserialize, Serialize};

/// 一致性检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheckConfig {
    /// 是否启用
    pub enabled: bool,
    /// 检查模式：相同模型重复执行 / 不同模型交叉验证
    pub mode: ConsistencyMode,
    /// 可选：用于交叉验证的备用模型名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_model: Option<String>,
    /// 结果偏差阈值（0.0-1.0），超过则告警
    pub deviation_threshold: f64,
}

impl Default for ConsistencyCheckConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ConsistencyMode::SameModelRepeated,
            secondary_model: None,
            deviation_threshold: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyMode {
    /// 相同模型执行 2 次对比
    #[serde(rename = "sameModelRepeated")]
    SameModelRepeated,
    /// 用不同模型交叉验证
    #[serde(rename = "crossModelCompare")]
    CrossModelCompare,
}

/// 一致性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyResult {
    pub passed: bool,
    pub deviation: f64,
    pub primary_result: serde_json::Value,
    pub secondary_result: Option<serde_json::Value>,
    pub details: String,
}

/// 执行一致性检查
pub fn check_consistency(
    primary: &serde_json::Value,
    secondary: &serde_json::Value,
    threshold: f64,
) -> ConsistencyResult {
    let deviation = compute_deviation(primary, secondary);
    ConsistencyResult {
        passed: deviation <= threshold,
        deviation,
        primary_result: primary.clone(),
        secondary_result: Some(secondary.clone()),
        details: format!(
            "偏差: {:.4} (阈值: {}), passed: {}",
            deviation,
            threshold,
            deviation <= threshold
        ),
    }
}

fn compute_deviation(a: &serde_json::Value, b: &serde_json::Value) -> f64 {
    match (a, b) {
        // 分类结果：精确匹配
        (serde_json::Value::String(sa), serde_json::Value::String(sb)) => {
            if sa == sb {
                0.0
            } else {
                1.0
            }
        },
        // 数值结果：归一化偏差
        (serde_json::Value::Number(_), serde_json::Value::Number(_)) => {
            let fa = a.as_f64().unwrap_or(0.0);
            let fb = b.as_f64().unwrap_or(0.0);
            let diff = (fa - fb).abs();
            // 归一化到 [0, 1]
            let max_abs = fa.abs().max(fb.abs()).max(1.0);
            (diff / max_abs).min(1.0)
        },
        // 其他类型：JSON 字符串编辑距离归一化
        _ => {
            let sa = a.to_string();
            let sb = b.to_string();
            let max_len = sa.len().max(sb.len()) as f64;
            if max_len == 0.0 {
                return 0.0;
            }
            let edits = text_distance(&sa, &sb) as f64;
            edits / max_len
        },
    }
}

/// 简化的文本差异距离计算
fn text_distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    let (sa, sb) = if a.len() < b.len() { (a, b) } else { (b, a) };
    if sa.is_empty() {
        return sb.len();
    }
    let common_prefix = sa
        .chars()
        .zip(sb.chars())
        .take_while(|(x, y)| x == y)
        .count();
    let common_suffix = sa
        .chars()
        .rev()
        .zip(sb.chars().rev())
        .take_while(|(x, y)| x == y)
        .count();
    sa.len().max(sb.len()) - common_prefix.min(sa.len()) - common_suffix.min(sa.len())
}
