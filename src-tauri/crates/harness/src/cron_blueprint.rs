// SPDX-License-Identifier: AGPL-3.0-only

//! Cron 调度闭环增强 (P1-8)
//!
//! 借鉴 Hermes Agent 的调度蓝图：
//! - CronBlueprint: 预定义的常用定时任务模板
//! - UsagePattern: 使用模式建议
//! - LifecycleGuard: 生命周期守卫，防止任务在错误条件下运行

use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// CronBlueprint: 预定义的常用定时任务模板
// ---------------------------------------------------------------------------

/// Cron 任务蓝图类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CronBlueprintType {
    /// 每日站会摘要
    DailyStandup,
    /// 每周报告生成
    WeeklyReport,
    /// 代码审查提醒
    CodeReviewReminder,
    /// 邮件摘要
    EmailDigest,
    /// 系统健康检查
    HealthCheck,
    /// 数据备份
    DataBackup,
    /// 知识库更新
    KnowledgeSync,
    /// 记忆巩固
    MemoryConsolidation,
    /// 自定义蓝图
    Custom,
}

/// Cron 任务蓝图
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronBlueprint {
    /// 蓝图 ID
    pub id: String,
    /// 蓝图类型
    pub blueprint_type: CronBlueprintType,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 默认 cron 表达式
    pub default_schedule: String,
    /// 默认 prompt
    pub default_prompt: String,
    /// 可自定义的参数
    pub customizable_params: Vec<BlueprintParam>,
    /// 使用说明
    pub usage_guide: String,
    /// 适用场景
    pub use_cases: Vec<String>,
    /// 风险等级
    pub risk_level: BlueprintRiskLevel,
}

/// 蓝图可自定义参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintParam {
    pub name: String,
    pub description: String,
    pub default_value: String,
    pub required: bool,
    pub param_type: BlueprintParamType,
}

/// 参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintParamType {
    String,
    Number,
    Boolean,
    Enum,
}

/// 蓝图风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintRiskLevel {
    Low,
    Medium,
    High,
}

/// 蓝图生成的 CronJob 数据（harness 层 DTO，由实现层转换为实际 CronJob）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintCronJobData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: String,
    pub prompt: String,
    pub task_type: Option<String>,
    pub enabled: bool,
}

impl CronBlueprint {
    /// 创建蓝图 CronJob 数据
    pub fn to_job_data(
        &self,
        custom_params: Option<HashMap<String, String>>,
    ) -> BlueprintCronJobData {
        let prompt = if let Some(params) = custom_params {
            let mut result = self.default_prompt.clone();
            for (key, value) in &params {
                result = result.replace(&format!("{{{{{}}}}}", key), value);
            }
            result
        } else {
            self.default_prompt.clone()
        };

        BlueprintCronJobData {
            id: format!("bp-{}-{}", self.id, chrono::Utc::now().timestamp()),
            name: self.name.clone(),
            description: self.description.clone(),
            schedule: self.default_schedule.clone(),
            prompt,
            task_type: Some(format!("{:?}", self.blueprint_type).to_lowercase()),
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 预定义蓝图集合
// ---------------------------------------------------------------------------

impl CronBlueprint {
    /// 获取所有预定义蓝图
    pub fn presets() -> Vec<CronBlueprint> {
        vec![
            Self::daily_standup(),
            Self::weekly_report(),
            Self::code_review_reminder(),
            Self::email_digest(),
            Self::health_check(),
            Self::data_backup(),
            Self::knowledge_sync(),
            Self::memory_consolidation(),
        ]
    }

    fn daily_standup() -> Self {
        Self {
            id: "bp-daily-standup".to_string(),
            blueprint_type: CronBlueprintType::DailyStandup,
            name: "每日站会摘要".to_string(),
            description: "每天早上自动汇总昨日工作进展，生成站会摘要".to_string(),
            default_schedule: "0 9 * * 1-5".to_string(),
            default_prompt: "请汇总昨日的工作进展，包括：\n1. 完成的任务\n2. 遇到的问题\n3. 今天的计划\n\n输出格式：Markdown 列表".to_string(),
            customizable_params: vec![
                BlueprintParam {
                    name: "timezone".to_string(),
                    description: "时区".to_string(),
                    default_value: "Asia/Shanghai".to_string(),
                    required: false,
                    param_type: BlueprintParamType::String,
                },
            ],
            usage_guide: "适用于远程团队每日同步。建议在团队站会前 15 分钟执行。".to_string(),
            use_cases: vec!["远程团队协作".to_string(), "个人工作复盘".to_string()],
            risk_level: BlueprintRiskLevel::Low,
        }
    }

    fn weekly_report() -> Self {
        Self {
            id: "bp-weekly-report".to_string(),
            blueprint_type: CronBlueprintType::WeeklyReport,
            name: "每周报告生成".to_string(),
            description: "每周五自动生成周报，包含本周工作成果和下周计划".to_string(),
            default_schedule: "0 17 * * 5".to_string(),
            default_prompt: "请生成本周工作周报，包含：\n1. 本周完成的主要任务\n2. 进行中的项目\n3. 下周工作计划\n4. 需要协助的事项".to_string(),
            customizable_params: vec![
                BlueprintParam {
                    name: "report_depth".to_string(),
                    description: "报告详细程度".to_string(),
                    default_value: "detailed".to_string(),
                    required: false,
                    param_type: BlueprintParamType::Enum,
                },
            ],
            usage_guide: "适合管理层汇总团队进展。建议在每周五下班前执行。".to_string(),
            use_cases: vec!["团队管理".to_string(), "项目汇总".to_string()],
            risk_level: BlueprintRiskLevel::Low,
        }
    }

    fn code_review_reminder() -> Self {
        Self {
            id: "bp-code-review".to_string(),
            blueprint_type: CronBlueprintType::CodeReviewReminder,
            name: "代码审查提醒".to_string(),
            description: "定期检查未完成的代码审查请求，发送提醒".to_string(),
            default_schedule: "0 14 * * 1-5".to_string(),
            default_prompt: "检查是否有待处理的代码审查请求，如果有，生成提醒列表并按优先级排序。"
                .to_string(),
            customizable_params: vec![BlueprintParam {
                name: "max_age_days".to_string(),
                description: "最大等待天数".to_string(),
                default_value: "3".to_string(),
                required: false,
                param_type: BlueprintParamType::Number,
            }],
            usage_guide: "适用于代码密集型项目，防止 PR 积压。".to_string(),
            use_cases: vec!["代码质量保障".to_string(), "协作效率提升".to_string()],
            risk_level: BlueprintRiskLevel::Low,
        }
    }

    fn email_digest() -> Self {
        Self {
            id: "bp-email-digest".to_string(),
            blueprint_type: CronBlueprintType::EmailDigest,
            name: "邮件摘要".to_string(),
            description: "每天定时汇总重要邮件，生成摘要".to_string(),
            default_schedule: "0 8,14 * * *".to_string(),
            default_prompt: "汇总今天收到的重要邮件，按发件人和主题分类，生成简洁摘要。"
                .to_string(),
            customizable_params: vec![BlueprintParam {
                name: "max_emails".to_string(),
                description: "最大摘要邮件数".to_string(),
                default_value: "20".to_string(),
                required: false,
                param_type: BlueprintParamType::Number,
            }],
            usage_guide: "适合收件箱零（Inbox Zero）工作方式的辅助工具。".to_string(),
            use_cases: vec!["邮件管理".to_string(), "信息摘要".to_string()],
            risk_level: BlueprintRiskLevel::Medium,
        }
    }

    fn health_check() -> Self {
        Self {
            id: "bp-health-check".to_string(),
            blueprint_type: CronBlueprintType::HealthCheck,
            name: "系统健康检查".to_string(),
            description: "定期检查系统状态，包括磁盘空间、内存、服务健康等".to_string(),
            default_schedule: "0 */4 * * *".to_string(),
            default_prompt: "执行系统健康检查：\n1. 检查磁盘空间（使用率 > 85% 报警）\n2. 检查内存使用\n3. 检查核心服务状态\n4. 生成健康报告".to_string(),
            customizable_params: vec![
                BlueprintParam {
                    name: "disk_threshold".to_string(),
                    description: "磁盘报警阈值".to_string(),
                    default_value: "85".to_string(),
                    required: false,
                    param_type: BlueprintParamType::Number,
                },
            ],
            usage_guide: "适用于生产环境的预防性维护。".to_string(),
            use_cases: vec!["运维监控".to_string(), "预防维护".to_string()],
            risk_level: BlueprintRiskLevel::Medium,
        }
    }

    fn data_backup() -> Self {
        Self {
            id: "bp-data-backup".to_string(),
            blueprint_type: CronBlueprintType::DataBackup,
            name: "数据备份".to_string(),
            description: "定时执行数据备份任务".to_string(),
            default_schedule: "0 2 * * 0".to_string(),
            default_prompt: "执行数据备份：\n1. 备份所有重要数据\n2. 验证备份完整性\n3. 清理过期备份（保留最近 7 天）\n4. 生成备份报告".to_string(),
            customizable_params: vec![
                BlueprintParam {
                    name: "retention_days".to_string(),
                    description: "备份保留天数".to_string(),
                    default_value: "7".to_string(),
                    required: false,
                    param_type: BlueprintParamType::Number,
                },
            ],
            usage_guide: "建议在业务低峰期（凌晨）执行。".to_string(),
            use_cases: vec!["数据保护".to_string(), "灾难恢复".to_string()],
            risk_level: BlueprintRiskLevel::High,
        }
    }

    fn knowledge_sync() -> Self {
        Self {
            id: "bp-knowledge-sync".to_string(),
            blueprint_type: CronBlueprintType::KnowledgeSync,
            name: "知识库更新".to_string(),
            description: "定期同步和索引知识库内容".to_string(),
            default_schedule: "0 3 * * *".to_string(),
            default_prompt:
                "执行知识库同步：\n1. 扫描新文件\n2. 更新索引\n3. 提取新的知识条目\n4. 生成更新报告"
                    .to_string(),
            customizable_params: vec![BlueprintParam {
                name: "scan_paths".to_string(),
                description: "扫描路径".to_string(),
                default_value: "/documents,/notes".to_string(),
                required: false,
                param_type: BlueprintParamType::String,
            }],
            usage_guide: "保持知识库时效性，建议每天凌晨执行。".to_string(),
            use_cases: vec!["知识管理".to_string(), "RAG 增强".to_string()],
            risk_level: BlueprintRiskLevel::Low,
        }
    }

    fn memory_consolidation() -> Self {
        Self {
            id: "bp-memory-consolidation".to_string(),
            blueprint_type: CronBlueprintType::MemoryConsolidation,
            name: "记忆巩固".to_string(),
            description: "在系统空闲时巩固记忆，优化检索效率".to_string(),
            default_schedule: "0 4 * * *".to_string(),
            default_prompt: "执行记忆巩固：\n1. 分析最近的会话轨迹\n2. 提取高价值记忆\n3. 合并重复记忆\n4. 优化索引结构".to_string(),
            customizable_params: vec![
                BlueprintParam {
                    name: "min_session_count".to_string(),
                    description: "最小会话数触发".to_string(),
                    default_value: "3".to_string(),
                    required: false,
                    param_type: BlueprintParamType::Number,
                },
            ],
            usage_guide: "系统空闲时执行，不影响日常使用。".to_string(),
            use_cases: vec!["记忆优化".to_string(), "学习增强".to_string()],
            risk_level: BlueprintRiskLevel::Low,
        }
    }
}

// ---------------------------------------------------------------------------
// LifecycleGuard: 生命周期守卫
// ---------------------------------------------------------------------------

/// 生命周期守卫 - 防止任务在错误条件下运行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleGuard {
    /// 最小运行间隔（秒）
    pub min_interval_secs: u64,
    /// 最大并发执行数
    pub max_concurrent: u32,
    /// 是否只在工作时间运行
    pub work_hours_only: bool,
    /// 工作时间开始（小时，0-23）
    pub work_start_hour: u8,
    /// 工作时间结束（小时，0-23）
    pub work_end_hour: u8,
    /// 跳过周末
    pub skip_weekends: bool,
    /// 系统负载阈值（超过则跳过）
    pub max_system_load: Option<f64>,
    /// 健康检查前置条件
    pub require_healthy_system: bool,
}

impl Default for LifecycleGuard {
    fn default() -> Self {
        Self {
            min_interval_secs: 60,
            max_concurrent: 3,
            work_hours_only: false,
            work_start_hour: 9,
            work_end_hour: 18,
            skip_weekends: false,
            max_system_load: None,
            require_healthy_system: false,
        }
    }
}

impl LifecycleGuard {
    /// 检查是否允许执行
    pub fn can_execute(
        &self,
        last_execution: Option<u64>,
        current_time_millis: u64,
        concurrent_count: u32,
        system_load: Option<f64>,
    ) -> GuardCheckResult {
        // 1. 检查最小间隔
        if let Some(last) = last_execution {
            let elapsed = (current_time_millis - last) / 1000;
            if elapsed < self.min_interval_secs {
                return GuardCheckResult::Blocked(format!(
                    "距上次执行仅 {} 秒，最小间隔 {} 秒",
                    elapsed, self.min_interval_secs
                ));
            }
        }

        // 2. 检查并发数
        if concurrent_count >= self.max_concurrent {
            return GuardCheckResult::Blocked(format!(
                "并发执行数 {} 已达上限 {}",
                concurrent_count, self.max_concurrent
            ));
        }

        // 3. 检查工作时间
        if self.work_hours_only {
            let time = chrono::DateTime::from_timestamp_millis(current_time_millis as i64);
            if let Some(dt) = time {
                let hour = dt.hour() as u8;
                if hour < self.work_start_hour || hour >= self.work_end_hour {
                    return GuardCheckResult::Blocked(format!(
                        "当前非工作时间 ({}-{} 小时)",
                        self.work_start_hour, self.work_end_hour
                    ));
                }
            }
        }

        // 4. 检查周末
        if self.skip_weekends {
            let time = chrono::DateTime::from_timestamp_millis(current_time_millis as i64);
            if let Some(dt) = time {
                let weekday = dt.weekday().num_days_from_monday();
                if weekday >= 5 {
                    return GuardCheckResult::Blocked("周末不执行".to_string());
                }
            }
        }

        // 5. 检查系统负载
        if let (Some(load), Some(max_load)) = (system_load, self.max_system_load)
            && load > max_load
        {
            return GuardCheckResult::Blocked(format!(
                "系统负载 {:.1}% 超过阈值 {:.1}%",
                load * 100.0,
                max_load * 100.0
            ));
        }

        GuardCheckResult::Allowed
    }

    /// 根据蓝图类型创建推荐的守卫配置
    pub fn recommended_for(blueprint_type: CronBlueprintType) -> Self {
        match blueprint_type {
            CronBlueprintType::DailyStandup => Self {
                work_hours_only: true,
                skip_weekends: true,
                min_interval_secs: 3600,
                ..Default::default()
            },
            CronBlueprintType::WeeklyReport => Self {
                work_hours_only: true,
                skip_weekends: true,
                min_interval_secs: 86400,
                ..Default::default()
            },
            CronBlueprintType::HealthCheck => Self {
                max_system_load: Some(0.9),
                require_healthy_system: false,
                ..Default::default()
            },
            CronBlueprintType::DataBackup => Self {
                work_hours_only: false,
                max_system_load: Some(0.5),
                require_healthy_system: true,
                ..Default::default()
            },
            _ => Self::default(),
        }
    }
}

/// 守卫检查结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardCheckResult {
    Allowed,
    Blocked(String),
}

// ---------------------------------------------------------------------------
// UsagePattern: 使用模式建议
// ---------------------------------------------------------------------------

/// 使用模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePattern {
    /// 模式 ID
    pub id: String,
    /// 模式名称
    pub name: String,
    /// 适用的蓝图类型
    pub applicable_blueprints: Vec<CronBlueprintType>,
    /// 模式描述
    pub description: String,
    /// 推荐的最佳实践
    pub best_practices: Vec<String>,
    /// 常见陷阱
    pub pitfalls: Vec<String>,
    /// 频率建议
    pub frequency_suggestions: FrequencySuggestion,
}

/// 频率建议
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencySuggestion {
    /// 最小频率
    pub min: String,
    /// 推荐频率
    pub recommended: String,
    /// 最大频率
    pub max: String,
    /// 注意事项
    pub note: String,
}

impl UsagePattern {
    /// 获取所有使用模式
    pub fn all() -> Vec<UsagePattern> {
        vec![
            Self::daily_cron_pattern(),
            Self::weekly_cron_pattern(),
            Self::monitoring_pattern(),
            Self::maintenance_pattern(),
            Self::learning_pattern(),
        ]
    }

    fn daily_cron_pattern() -> Self {
        Self {
            id: "pattern-daily".to_string(),
            name: "每日执行模式".to_string(),
            applicable_blueprints: vec![
                CronBlueprintType::DailyStandup,
                CronBlueprintType::EmailDigest,
                CronBlueprintType::MemoryConsolidation,
            ],
            description: "每天在固定时间执行一次的任务模式".to_string(),
            best_practices: vec![
                "选择用户工作时间的开始或结束".to_string(),
                "避免凌晨执行高负载任务".to_string(),
                "设置合理的超时时间".to_string(),
            ],
            pitfalls: vec!["不要在用户高峰时段执行".to_string(), "注意时区差异".to_string()],
            frequency_suggestions: FrequencySuggestion {
                min: "每天一次".to_string(),
                recommended: "每天 1-2 次".to_string(),
                max: "每天 4 次".to_string(),
                note: "过于频繁会增加系统负载".to_string(),
            },
        }
    }

    fn weekly_cron_pattern() -> Self {
        Self {
            id: "pattern-weekly".to_string(),
            name: "每周执行模式".to_string(),
            applicable_blueprints: vec![CronBlueprintType::WeeklyReport],
            description: "每周固定时间执行的任务模式".to_string(),
            best_practices: vec![
                "选择周五下午或周一上午".to_string(),
                "确保覆盖完整的工作周期".to_string(),
            ],
            pitfalls: vec!["避免在周末执行".to_string(), "注意节假日影响".to_string()],
            frequency_suggestions: FrequencySuggestion {
                min: "每周一次".to_string(),
                recommended: "每周一次".to_string(),
                max: "每周两次".to_string(),
                note: "周报类任务不需要更频繁".to_string(),
            },
        }
    }

    fn monitoring_pattern() -> Self {
        Self {
            id: "pattern-monitoring".to_string(),
            name: "监控模式".to_string(),
            applicable_blueprints: vec![
                CronBlueprintType::HealthCheck,
                CronBlueprintType::CodeReviewReminder,
            ],
            description: "定期检查和监控的任务模式".to_string(),
            best_practices: vec![
                "设置合理的检查间隔".to_string(),
                "避免在系统负载高时执行".to_string(),
                "配置告警阈值".to_string(),
            ],
            pitfalls: vec![
                "过于频繁的检查会影响性能".to_string(),
                "告警疲劳 - 不要设置过于敏感的阈值".to_string(),
            ],
            frequency_suggestions: FrequencySuggestion {
                min: "每小时一次".to_string(),
                recommended: "每 4-6 小时一次".to_string(),
                max: "每 12 小时一次".to_string(),
                note: "健康检查类任务建议中等频率".to_string(),
            },
        }
    }

    fn maintenance_pattern() -> Self {
        Self {
            id: "pattern-maintenance".to_string(),
            name: "维护模式".to_string(),
            applicable_blueprints: vec![
                CronBlueprintType::DataBackup,
                CronBlueprintType::KnowledgeSync,
            ],
            description: "系统维护类任务模式".to_string(),
            best_practices: vec![
                "在业务低峰期执行".to_string(),
                "确保有足够的磁盘空间".to_string(),
                "保留足够的历史数据".to_string(),
            ],
            pitfalls: vec![
                "不要在用户活跃时段执行".to_string(),
                "备份可能很大，注意存储限制".to_string(),
            ],
            frequency_suggestions: FrequencySuggestion {
                min: "每周一次".to_string(),
                recommended: "每天一次".to_string(),
                max: "每周一次".to_string(),
                note: "备份类任务建议每天一次".to_string(),
            },
        }
    }

    fn learning_pattern() -> Self {
        Self {
            id: "pattern-learning".to_string(),
            name: "学习模式".to_string(),
            applicable_blueprints: vec![CronBlueprintType::MemoryConsolidation],
            description: "系统学习和记忆巩固任务模式".to_string(),
            best_practices: vec![
                "在系统空闲时执行".to_string(),
                "确保有足够的新数据".to_string(),
                "不要过于频繁".to_string(),
            ],
            pitfalls: vec![
                "没有足够新数据时执行无效".to_string(),
                "过于频繁会干扰正常使用".to_string(),
            ],
            frequency_suggestions: FrequencySuggestion {
                min: "每周一次".to_string(),
                recommended: "每天一次".to_string(),
                max: "每 3 天一次".to_string(),
                note: "记忆巩固需要积累足够的会话数据".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blueprint_presets() {
        let presets = CronBlueprint::presets();
        assert_eq!(presets.len(), 8);

        let daily = &presets[0];
        assert_eq!(daily.name, "每日站会摘要");
        assert_eq!(daily.blueprint_type, CronBlueprintType::DailyStandup);
    }

    #[test]
    fn test_blueprint_to_job_data() {
        let bp = CronBlueprint::daily_standup();
        let data = bp.to_job_data(None);
        assert_eq!(data.name, "每日站会摘要");
        assert_eq!(data.schedule, "0 9 * * 1-5");
        assert!(!data.prompt.is_empty());
        assert!(data.enabled);
    }

    #[test]
    fn test_blueprint_custom_params() {
        let bp = CronBlueprint::daily_standup();
        let mut params = HashMap::new();
        params.insert("timezone".to_string(), "America/New_York".to_string());

        let data = bp.to_job_data(Some(params));
        assert!(data.prompt.contains("汇总"));
    }

    #[test]
    fn test_lifecycle_guard_allowed() {
        let guard = LifecycleGuard::default();
        let result = guard.can_execute(None, 1000000, 0, None);
        assert_eq!(result, GuardCheckResult::Allowed);
    }

    #[test]
    fn test_lifecycle_guard_blocked_interval() {
        let guard = LifecycleGuard::default();
        let result = guard.can_execute(Some(999000), 1000000, 0, None);
        assert!(matches!(result, GuardCheckResult::Blocked(_)));
    }

    #[test]
    fn test_lifecycle_guard_blocked_concurrent() {
        let guard = LifecycleGuard::default();
        let result = guard.can_execute(None, 1000000, 3, None);
        assert!(matches!(result, GuardCheckResult::Blocked(_)));
    }

    #[test]
    fn test_lifecycle_guard_recommended() {
        let guard = LifecycleGuard::recommended_for(CronBlueprintType::DailyStandup);
        assert!(guard.work_hours_only);
        assert!(guard.skip_weekends);
    }

    #[test]
    fn test_usage_patterns() {
        let patterns = UsagePattern::all();
        assert_eq!(patterns.len(), 5);

        let daily = &patterns[0];
        assert_eq!(daily.name, "每日执行模式");
        assert!(!daily.best_practices.is_empty());
    }
}
