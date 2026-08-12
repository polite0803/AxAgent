//! 行业验证服务 — 迁移自 OpcIndustryAdapter::validate
//!
//! 替代行业适配器中的 validate() 方法，保留动态业务逻辑。

use super::error::OpcResult;
use super::invoice::InvoiceStatus;
use super::rules::ValidationError;

/// 会计与财务验证
pub async fn validate_accounting(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    match entity_type {
        "invoice" => {
            if let Some(amount) = entity_data.get("amount") {
                if amount.as_f64().is_none_or(|a| a <= 0.0) {
                    errors.push(ValidationError::new("amount", "发票金额必须大于零"));
                }
            }
            if let Some(status) = entity_data.get("status") {
                let valid_statuses = [
                    InvoiceStatus::Draft.as_str(),
                    InvoiceStatus::Sent.as_str(),
                    InvoiceStatus::Paid.as_str(),
                    InvoiceStatus::Overdue.as_str(),
                    InvoiceStatus::Cancelled.as_str(),
                    InvoiceStatus::Refunded.as_str(),
                ];
                if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                    errors.push(ValidationError::new("status", "无效的发票状态"));
                }
            }
        },
        "finance_record" => {
            if let Some(amount) = entity_data.get("amount") {
                if amount.as_f64().is_none_or(|a| a == 0.0) {
                    errors.push(ValidationError::new("amount", "财务记录金额不能为零"));
                }
            }
        },
        _ => {},
    }

    Ok(errors)
}

/// 金融投资验证
pub async fn validate_finance_invest(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    if entity_type == "stock" {
        if let Some(code) = entity_data.get("code") {
            let code_str = code.as_str().unwrap_or("");
            if code_str.is_empty() || code_str.len() != 6 {
                errors.push(ValidationError::new("code", "股票代码必须是6位数字"));
            }
        }
        if let Some(amount) = entity_data.get("amount") {
            if amount.as_f64().is_none_or(|a| a <= 0.0) {
                errors.push(ValidationError::new("amount", "投资金额必须为正数"));
            }
        }
    }

    Ok(errors)
}

/// 软件研发验证
pub async fn validate_software_dev(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    match entity_type {
        "requirement" => {
            if let Some(title) = entity_data.get("title") {
                if title.as_str().is_none_or(|s| s.is_empty()) {
                    errors.push(ValidationError::new("title", "需求标题不能为空"));
                }
            }
            if let Some(priority) = entity_data.get("priority") {
                let valid = ["P0", "P1", "P2", "P3"];
                let p = priority.as_str().unwrap_or("");
                if !valid.contains(&p) {
                    errors.push(ValidationError::new("priority", "优先级必须是 P0/P1/P2/P3"));
                }
            }
        },
        "task" => {
            if let Some(assignee) = entity_data.get("assignee") {
                if assignee.as_str().is_none_or(|s| s.is_empty()) {
                    errors.push(ValidationError::new("assignee", "任务必须指派人"));
                }
            }
        },
        _ => {},
    }

    Ok(errors)
}

/// 设计验证
pub async fn validate_design(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    if entity_type == "design" {
        if let Some(name) = entity_data.get("name") {
            if name.as_str().is_none_or(|s| s.is_empty()) {
                errors.push(ValidationError::new("name", "设计名称不能为空"));
            }
        }
    }

    Ok(errors)
}

/// 内容媒体验证
pub async fn validate_content_media(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    match entity_type {
        "article" | "post" => {
            if let Some(title) = entity_data.get("title") {
                if title.as_str().is_none_or(|s| s.is_empty()) {
                    errors.push(ValidationError::new("title", "内容标题不能为空"));
                }
            }
        },
        _ => {},
    }

    Ok(errors)
}

/// 项目管理验证
pub async fn validate_project_management(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    if entity_type == "project" {
        if let Some(name) = entity_data.get("name") {
            if name.as_str().is_none_or(|s| s.is_empty()) {
                errors.push(ValidationError::new("name", "项目名称不能为空"));
            }
        }
    }

    Ok(errors)
}

/// 安全合规验证
pub async fn validate_security(
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    let mut errors = Vec::new();

    if entity_type == "incident" {
        if let Some(incident_type) = entity_data.get("type") {
            if incident_type.as_str().is_none_or(|s| s.is_empty()) {
                errors.push(ValidationError::new("type", "安全事件类型不能为空"));
            }
        }
    }

    Ok(errors)
}

/// 验证行业实体（统一入口）
pub async fn validate_entity(
    industry_id: &str,
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> OpcResult<Vec<ValidationError>> {
    match industry_id.replace('-', "_").as_str() {
        "accounting" => validate_accounting(entity_type, entity_data).await,
        "finance_invest" => validate_finance_invest(entity_type, entity_data).await,
        "software_dev" => validate_software_dev(entity_type, entity_data).await,
        "design" => validate_design(entity_type, entity_data).await,
        "content_media" => validate_content_media(entity_type, entity_data).await,
        "project_management" => validate_project_management(entity_type, entity_data).await,
        "security" => validate_security(entity_type, entity_data).await,
        _ => Ok(Vec::new()), // 其他行业使用默认验证
    }
}

/// 批量验证行业实体
pub async fn validate_batch(
    industry_id: &str,
    entities: &[(String, serde_json::Value)],
) -> OpcResult<Vec<(String, Vec<ValidationError>)>> {
    let mut results = Vec::new();
    for (entity_type, entity_data) in entities {
        let errors = validate_entity(industry_id, entity_type, entity_data).await?;
        results.push((entity_type.clone(), errors));
    }
    Ok(results)
}
