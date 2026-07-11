// SPDX-License-Identifier: AGPL-3.0-only

//! 时间旅行（As-Of）上下文 — 纯 DTO + 契约层
//!
//! 提供 AsOfContext、AsOfSource 等共享类型定义，
//! 让消费者（stock-analysis、quant）不直接依赖 axagent-astock-data。
//!
//! **不含运行时状态**（task_local、全局 Mutex 等实现在 astock-data 中）。

use serde::{Deserialize, Serialize};

/// As-Of 数据的来源标签，用于审计
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsOfSource {
    /// 用户在 UI 手动选择
    UserReplay,
    /// Sweep 工具批量跑
    BacktestSweep,
    /// 调度器周期跑
    ScheduledReplay,
}

impl std::fmt::Display for AsOfSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsOfSource::UserReplay => write!(f, "user_replay"),
            AsOfSource::BacktestSweep => write!(f, "backtest_sweep"),
            AsOfSource::ScheduledReplay => write!(f, "scheduled_replay"),
        }
    }
}

/// 时间锚点：在该任务执行期间，所有 vendor 调用应被视为"截至 as_of_date"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsOfContext {
    pub as_of_date: chrono::NaiveDate,
    pub source: AsOfSource,
    /// 数据截止范围(混合 as-of 模式)。默认 All 兼容旧行为。
    #[serde(default)]
    pub data_scope: AsOfDataScope,
}

impl AsOfContext {
    /// 解析 'YYYY-MM-DD' 字符串（不含运行时验证，纯解析）
    pub fn parse(s: &str) -> Result<Self, String> {
        let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| format!("无效日期格式 '{s}': {e}"))?;
        Ok(Self {
            as_of_date: date,
            source: AsOfSource::UserReplay,
            data_scope: AsOfDataScope::All,
        })
    }

    /// 解析可选入参（None / 空 → None；合法 → Some）
    pub fn parse_optional(s: Option<&str>) -> Result<Option<Self>, String> {
        match s.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => Self::parse(s).map(Some),
        }
    }

    /// 转 'YYYY-MM-DD' 字符串
    pub fn as_string(&self) -> String {
        self.as_of_date.format("%Y-%m-%d").to_string()
    }
}

/// 数据截止范围(混合 as-of 模式核心枚举)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AsOfDataScope {
    /// 所有数据按 as_of 截止(旧行为,默认)
    #[default]
    All,
    /// 仅"结构化数据"按 as_of 截止;新闻/公告/研报/排行 保持实时
    Structured,
}

/// 数据源种类(用于 AsOfDataScope 决策)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsOfDataKind {
    /// 结构化数据
    Structured,
    /// 非结构化数据:新闻/公告/研报/社媒
    Unstructured,
    /// 排行榜/分类/指数
    Rank,
}

/// As-Of 降级条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct DegradationEntry {
    pub vendor: String,
    pub method: String,
    pub reason: String,
    pub as_of: String,
}
