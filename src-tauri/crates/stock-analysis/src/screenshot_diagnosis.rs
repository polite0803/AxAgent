// SPDX-License-Identifier: AGPL-3.0-only
//! G6 截图持仓诊断完整闭环服务层
//!
//! ## 用途
//!
//! 承接 DojoAgents 宣传场景 3「截图持仓诊断」：
//! - 接收 OCR + LLM 结构化解析后的持仓列表
//! - 计算风险诊断 schema（7 项指标）
//! - 持久化到 screenshot_diagnoses 表
//! - 与 paper_portfolio 联动（通过 source_screenshot_diagnosis_id 外键）
//!
//! ## 数据流
//!
//! ```text
//! 用户上传截图
//!   → commands/screenshot_diagnosis::screenshot_diagnosis_create_from_image
//!     → VisionPipeline::analyze(image, VisionTask::Ocr) 提取 OCR 文本
//!     → LLM 结构化解析 OCR 文本为 Vec<ScreenshotPosition>
//!     → compute_risk_diagnosis(&positions)（纯计算 7 项风险指标）
//!     → LLM 生成 narrative（自然语言诊断说明）
//!     → 本模块 create_diagnosis 持久化
//!   → 前端展示
//!   → 用户点击「转为观察组合」
//!   → commands/screenshot_diagnosis::screenshot_diagnosis_to_paper_portfolio
//!     → paper_portfolio::create_portfolio_from_screenshot_diagnosis
//! ```
//!
//! ## 与 paper_portfolio 的区别
//!
//! - [`crate::paper_portfolio`]：模拟观察组合（含建仓 / 平仓 / 收益跟踪）
//! - 本模块：截图持仓诊断（一次性快照式诊断，可一键转 paper_portfolio）
//!
//! 全部读写均经过 SeaORM，无副作用，可幂等调用。

use sea_orm::sea_query::Expr;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseConnection;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::Set;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axagent_entities::screenshot_diagnoses as sd_entity;

// ── DTO ───────────────────────────────────────────────────────────────────

/// 单条持仓（截图 OCR + LLM 结构化解析后产物）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPosition {
    /// 股票代码（A 股 6 位 / 港股 5 位+后缀 / 美股字母）
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 持仓数量（股）
    #[serde(default)]
    pub qty: f64,
    /// 成本价
    #[serde(default)]
    pub cost_price: f64,
    /// 当前市值（元）
    #[serde(default)]
    pub market_value: f64,
    /// 权重（百分比，0-100），由 total_market_value 计算
    #[serde(default)]
    pub weight: f64,
}

/// 风险诊断 schema（7 项指标）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RiskDiagnosis {
    /// 集中度风险（top 1 持仓权重 > 30% 视为高）
    pub concentration_risk: ConcentrationRisk,
    /// 重复持仓（同一标的多账户/多基金重复）
    pub overlap_positions: Vec<OverlapPosition>,
    /// 防御仓位占比（公用事业 / 银行 / 消费必需 / 医药）
    pub defense_ratio: f64,
    /// 美股敞口占比（A 股截图通常为 0）
    pub us_exposure: f64,
    /// 弱势仓位（近期下跌幅度大的标的）
    pub weak_exposure: WeakExposure,
    /// 同一标的重复出现（前端展示用）
    pub repeated_positions: Vec<String>,
    /// 核心仓位集中度（top 3 持仓权重总和）
    pub core_concentration: CoreConcentration,
}

/// 集中度风险
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConcentrationRisk {
    /// top 1 持仓代码
    pub top1_code: String,
    /// top 1 持仓权重（0-100）
    pub top1_weight: f64,
    /// 风险等级（"high" / "medium" / "low"）
    pub level: String,
    /// 说明
    pub narrative: String,
}

/// 重复持仓
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlapPosition {
    /// 重复的标的代码
    pub code: String,
    /// 重复次数
    pub count: usize,
    /// 合并权重（百分比）
    pub merged_weight: f64,
}

/// 弱势仓位
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WeakExposure {
    /// 弱势标的代码列表
    pub codes: Vec<String>,
    /// 弱势仓位总权重
    pub total_weight: f64,
    /// 说明
    pub narrative: String,
}

/// 核心仓位集中度
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoreConcentration {
    /// top 3 持仓代码
    pub top3_codes: Vec<String>,
    /// top 3 持仓权重总和（0-100）
    pub top3_weight: f64,
    /// 风险等级（"high" / "medium" / "low"）
    pub level: String,
    /// 说明
    pub narrative: String,
}

/// 创建截图诊断的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScreenshotDiagnosisInput {
    /// 截图 SHA256（用于去重）
    pub image_hash: Option<String>,
    /// 截图本地存储路径
    pub image_path: Option<String>,
    /// 缩略图 base64
    pub image_thumbnail_base64: Option<String>,
    /// 原图宽度
    pub image_width: Option<i32>,
    /// 原图高度
    pub image_height: Option<i32>,
    /// 截图来源 App
    pub source_app: Option<String>,
    /// OCR 提取的完整文本
    pub ocr_text: Option<String>,
    /// 结构化持仓列表
    pub positions: Vec<ScreenshotPosition>,
    /// 截图时刻总市值（若为 0，由 positions 求和计算）
    #[serde(default)]
    pub total_market_value: f64,
    /// 风险诊断（若为空，由 compute_risk_diagnosis 计算）
    #[serde(default)]
    pub diagnosis: Option<RiskDiagnosis>,
    /// 自然语言诊断说明
    #[serde(default)]
    pub narrative: String,
    /// 建议动作列表
    #[serde(default)]
    pub recommended_actions: Vec<String>,
    /// 来源工作流执行 ID
    pub source_workflow_execution_id: Option<String>,
    /// LLM provider ID
    pub provider_id: Option<String>,
    /// LLM model ID
    pub model_id: Option<String>,
    /// 初始状态（默认 "active"）
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "active".to_string()
}

/// 更新诊断的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScreenshotDiagnosisInput {
    pub diagnosis_id: String,
    pub status: Option<String>,
    pub narrative: Option<String>,
    pub recommended_actions: Option<Vec<String>>,
    pub error_message: Option<String>,
}

// ── 风险诊断计算 ─────────────────────────────────────────────────────────

/// 防御行业关键词（用于代码或名称匹配）
///
/// 注：此处用名称关键词匹配，因为截图 OCR 可能只给名称不给代码。
/// 实际匹配时按名称模糊匹配。
const DEFENSE_KEYWORDS: &[&str] = &[
    // 银行
    "银行",
    "工商银行",
    "建设银行",
    "农业银行",
    "中国银行",
    "招商银行",
    "交通银行",
    // 公用事业
    "电力",
    "燃气",
    "水务",
    "公用",
    // 消费必需
    "食品",
    "饮料",
    "乳业",
    "粮油",
    "调味",
    "超市",
    "百货",
    // 医药
    "医药",
    "生物",
    "制药",
    "医疗",
    "中药",
    "疫苗",
];

/// 美股代码特征：纯字母（A 股 6 位数字 / 港股 5 位数字+后缀）
fn is_us_code(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|c| c.is_ascii_alphabetic())
}

/// 判断是否为防御行业（按名称模糊匹配）
fn is_defense_name(name: &str) -> bool {
    DEFENSE_KEYWORDS.iter().any(|kw| name.contains(kw))
}

/// 计算风险诊断 schema（7 项指标）
///
/// 输入：截图 OCR + LLM 结构化解析后的持仓列表
/// 输出：完整的 [`RiskDiagnosis`]
///
/// 算法：
/// 1. 重新归一化权重（若 weight 字段缺失或总和 ≠ 100，由 market_value 重新计算）
/// 2. concentration_risk: top 1 权重 > 30% → high, > 20% → medium, 否则 low
/// 3. overlap_positions: 同一 code 出现多次 → 合并权重
/// 4. defense_ratio: 防御行业标的权重总和
/// 5. us_exposure: 美股代码权重总和
/// 6. weak_exposure: 这里不计算（无历史数据），返回空列表（前端可二次调取）
/// 7. repeated_positions: 同 3，返回 code 列表
/// 8. core_concentration: top 3 权重总和 > 60% → high, > 40% → medium, 否则 low
pub fn compute_risk_diagnosis(positions: &[ScreenshotPosition]) -> RiskDiagnosis {
    if positions.is_empty() {
        return RiskDiagnosis::default();
    }

    // 1. 重新归一化权重
    let total_mv: f64 = positions.iter().map(|p| p.market_value.max(0.0)).sum();
    let normalized: Vec<(usize, &ScreenshotPosition, f64)> = positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let w = if total_mv > 0.0 {
                p.market_value / total_mv * 100.0
            } else {
                p.weight
            };
            (i, p, w)
        })
        .collect();

    // 2. concentration_risk: top 1 持仓
    let mut sorted_by_weight = normalized.iter().collect::<Vec<_>>();
    sorted_by_weight.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let top1 = sorted_by_weight[0];
    let top1_weight = top1.2;
    let (level, narr) = if top1_weight > 30.0 {
        (
            "high",
            format!(
                "Top 1 持仓 {}（{}）权重 {:.1}%，超过 30% 警戒线，集中度过高",
                top1.1.code, top1.1.name, top1_weight
            ),
        )
    } else if top1_weight > 20.0 {
        (
            "medium",
            format!(
                "Top 1 持仓 {}（{}）权重 {:.1}%，接近 30% 警戒线",
                top1.1.code, top1.1.name, top1_weight
            ),
        )
    } else {
        (
            "low",
            format!(
                "Top 1 持仓 {}（{}）权重 {:.1}%，集中度合理",
                top1.1.code, top1.1.name, top1_weight
            ),
        )
    };
    let concentration_risk = ConcentrationRisk {
        top1_code: top1.1.code.clone(),
        top1_weight,
        level: level.to_string(),
        narrative: narr,
    };

    // 3. overlap_positions: 同一 code 出现多次
    let mut code_groups: std::collections::HashMap<String, (usize, f64)> =
        std::collections::HashMap::new();
    for (_, p, w) in &normalized {
        let entry = code_groups.entry(p.code.clone()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += w;
    }
    let overlap_positions: Vec<OverlapPosition> = code_groups
        .iter()
        .filter(|(_, (cnt, _))| *cnt > 1)
        .map(|(code, (cnt, w))| OverlapPosition {
            code: code.clone(),
            count: *cnt,
            merged_weight: *w,
        })
        .collect();
    let repeated_positions: Vec<String> =
        overlap_positions.iter().map(|o| o.code.clone()).collect();

    // 4. defense_ratio: 防御行业标的权重总和
    let defense_ratio: f64 =
        normalized.iter().filter(|(_, p, _)| is_defense_name(&p.name)).map(|(_, _, w)| *w).sum();

    // 5. us_exposure: 美股代码权重总和
    let us_exposure: f64 =
        normalized.iter().filter(|(_, p, _)| is_us_code(&p.code)).map(|(_, _, w)| *w).sum();

    // 6. weak_exposure: 截图静态数据无法计算近期涨跌幅，返回空
    // （前端可通过 stock-analysis 二次查询补全）
    let weak_exposure = WeakExposure {
        codes: vec![],
        total_weight: 0.0,
        narrative: "截图为静态快照，无法计算弱势仓位。建议结合近期行情二次诊断。".to_string(),
    };

    // 7. core_concentration: top 3 权重总和
    let top3_slice = &sorted_by_weight[..3.min(sorted_by_weight.len())];
    let top3_codes: Vec<String> = top3_slice.iter().map(|(_, p, _)| p.code.clone()).collect();
    let top3_weight: f64 = top3_slice.iter().map(|(_, _, w)| *w).sum();
    let (core_level, core_narr) = if top3_weight > 60.0 {
        (
            "high",
            format!("Top 3 持仓权重总和 {:.1}%，超过 60% 警戒线，核心仓位过于集中", top3_weight),
        )
    } else if top3_weight > 40.0 {
        ("medium", format!("Top 3 持仓权重总和 {:.1}%，接近 60% 警戒线", top3_weight))
    } else {
        ("low", format!("Top 3 持仓权重总和 {:.1}%，核心仓位分布合理", top3_weight))
    };
    let core_concentration = CoreConcentration {
        top3_codes,
        top3_weight,
        level: core_level.to_string(),
        narrative: core_narr,
    };

    RiskDiagnosis {
        concentration_risk,
        overlap_positions,
        defense_ratio,
        us_exposure,
        weak_exposure,
        repeated_positions,
        core_concentration,
    }
}

// ── CRUD ──────────────────────────────────────────────────────────────────

/// 创建截图诊断记录
pub async fn create_diagnosis(
    db: &DatabaseConnection,
    input: CreateScreenshotDiagnosisInput,
) -> Result<sd_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();

    // 若未提供 total_market_value，由 positions 求和
    let total_mv = if input.total_market_value > 0.0 {
        input.total_market_value
    } else {
        input.positions.iter().map(|p| p.market_value.max(0.0)).sum()
    };

    // 若未提供 diagnosis，由 compute_risk_diagnosis 计算
    let diagnosis = input.diagnosis.unwrap_or_else(|| compute_risk_diagnosis(&input.positions));

    let positions_json =
        serde_json::to_string(&input.positions).unwrap_or_else(|_| "[]".to_string());
    let diagnosis_json = serde_json::to_string(&diagnosis).unwrap_or_else(|_| "{}".to_string());
    let actions_json =
        serde_json::to_string(&input.recommended_actions).unwrap_or_else(|_| "[]".to_string());

    let model = sd_entity::ActiveModel {
        id: Set(id),
        image_hash: Set(input.image_hash),
        image_path: Set(input.image_path),
        image_thumbnail_base64: Set(input.image_thumbnail_base64),
        image_width: Set(input.image_width),
        image_height: Set(input.image_height),
        source_app: Set(input.source_app),
        ocr_text: Set(input.ocr_text),
        positions_json: Set(positions_json),
        total_market_value: Set(total_mv),
        diagnosis_json: Set(diagnosis_json),
        narrative: Set(input.narrative),
        recommended_actions: Set(actions_json),
        source_workflow_execution_id: Set(input.source_workflow_execution_id),
        provider_id: Set(input.provider_id),
        model_id: Set(input.model_id),
        status: Set(input.status),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await
}

/// 按 ID 获取
pub async fn get_diagnosis(
    db: &DatabaseConnection,
    diagnosis_id: &str,
) -> Result<Option<sd_entity::Model>, DbErr> {
    sd_entity::Entity::find_by_id(diagnosis_id.to_string()).one(db).await
}

/// 列出最近 N 条诊断（按 created_at 降序）
pub async fn list_recent_diagnoses(
    db: &DatabaseConnection,
    limit: usize,
) -> Result<Vec<sd_entity::Model>, DbErr> {
    sd_entity::Entity::find()
        .order_by_desc(sd_entity::Column::CreatedAt)
        .limit(limit as u64)
        .all(db)
        .await
}

/// 按状态过滤
pub async fn list_diagnoses_by_status(
    db: &DatabaseConnection,
    status: &str,
) -> Result<Vec<sd_entity::Model>, DbErr> {
    sd_entity::Entity::find()
        .filter(sd_entity::Column::Status.eq(status))
        .order_by_desc(sd_entity::Column::CreatedAt)
        .all(db)
        .await
}

/// 按来源 App 过滤
pub async fn list_diagnoses_by_source_app(
    db: &DatabaseConnection,
    source_app: &str,
) -> Result<Vec<sd_entity::Model>, DbErr> {
    sd_entity::Entity::find()
        .filter(sd_entity::Column::SourceApp.eq(source_app))
        .order_by_desc(sd_entity::Column::CreatedAt)
        .all(db)
        .await
}

/// 按 image_hash 查询（去重判断）
pub async fn find_by_image_hash(
    db: &DatabaseConnection,
    image_hash: &str,
) -> Result<Option<sd_entity::Model>, DbErr> {
    sd_entity::Entity::find().filter(sd_entity::Column::ImageHash.eq(image_hash)).one(db).await
}

/// 更新诊断（部分字段）
pub async fn update_diagnosis(
    db: &DatabaseConnection,
    input: UpdateScreenshotDiagnosisInput,
) -> Result<sd_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut model = sd_entity::ActiveModel {
        id: Set(input.diagnosis_id),
        updated_at: Set(now),
        ..Default::default()
    };
    if let Some(s) = input.status {
        model.status = Set(s);
    }
    if let Some(n) = input.narrative {
        model.narrative = Set(n);
    }
    if let Some(actions) = input.recommended_actions {
        let actions_json = serde_json::to_string(&actions).unwrap_or_else(|_| "[]".to_string());
        model.recommended_actions = Set(actions_json);
    }
    if let Some(err) = input.error_message {
        model.error_message = Set(Some(err));
    }
    model.update(db).await
}

/// 归档诊断（status=archived）
pub async fn archive_diagnosis(
    db: &DatabaseConnection,
    diagnosis_id: &str,
) -> Result<sd_entity::Model, DbErr> {
    update_diagnosis(
        db,
        UpdateScreenshotDiagnosisInput {
            diagnosis_id: diagnosis_id.to_string(),
            status: Some("archived".to_string()),
            narrative: None,
            recommended_actions: None,
            error_message: None,
        },
    )
    .await
}

/// 标记诊断失败（status=failed + error_message）
pub async fn mark_failed(
    db: &DatabaseConnection,
    diagnosis_id: &str,
    error_message: &str,
) -> Result<sd_entity::Model, DbErr> {
    update_diagnosis(
        db,
        UpdateScreenshotDiagnosisInput {
            diagnosis_id: diagnosis_id.to_string(),
            status: Some("failed".to_string()),
            narrative: None,
            recommended_actions: None,
            error_message: Some(error_message.to_string()),
        },
    )
    .await
}

/// 显式触发 [`Expr`] 引用，避免未使用警告
#[allow(dead_code)]
fn _touch_expr() {
    let _ = Expr::value(0i64);
}

// ── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(code: &str, name: &str, mv: f64) -> ScreenshotPosition {
        ScreenshotPosition {
            code: code.to_string(),
            name: name.to_string(),
            qty: 0.0,
            cost_price: 0.0,
            market_value: mv,
            weight: 0.0,
        }
    }

    #[test]
    fn compute_risk_diagnosis_empty_returns_default() {
        let d = compute_risk_diagnosis(&[]);
        assert_eq!(d.concentration_risk.top1_weight, 0.0);
        assert_eq!(d.defense_ratio, 0.0);
    }

    #[test]
    fn compute_risk_diagnosis_single_high_concentration() {
        let positions = vec![
            make_position("600519", "贵州茅台", 100_000.0),
            make_position("000858", "五粮液", 10_000.0),
        ];
        let d = compute_risk_diagnosis(&positions);
        assert!(d.concentration_risk.top1_weight > 90.0);
        assert_eq!(d.concentration_risk.level, "high");
        assert_eq!(d.concentration_risk.top1_code, "600519");
    }

    #[test]
    fn compute_risk_diagnosis_detects_defense_ratio() {
        let positions = vec![
            make_position("600036", "招商银行", 50_000.0),
            make_position("600519", "贵州茅台", 50_000.0),
        ];
        let d = compute_risk_diagnosis(&positions);
        assert!((d.defense_ratio - 50.0).abs() < 0.1, "defense_ratio 应为 50");
    }

    #[test]
    fn compute_risk_diagnosis_detects_us_exposure() {
        let positions = vec![
            make_position("AAPL", "Apple Inc.", 30_000.0),
            make_position("600519", "贵州茅台", 70_000.0),
        ];
        let d = compute_risk_diagnosis(&positions);
        assert!((d.us_exposure - 30.0).abs() < 0.1, "us_exposure 应为 30");
    }

    #[test]
    fn compute_risk_diagnosis_detects_repeated_positions() {
        let positions = vec![
            make_position("600519", "贵州茅台", 30_000.0),
            make_position("600519", "贵州茅台", 20_000.0),
            make_position("000858", "五粮液", 50_000.0),
        ];
        let d = compute_risk_diagnosis(&positions);
        assert_eq!(d.overlap_positions.len(), 1);
        assert_eq!(d.overlap_positions[0].code, "600519");
        assert_eq!(d.overlap_positions[0].count, 2);
        assert!((d.overlap_positions[0].merged_weight - 50.0).abs() < 0.1);
        assert_eq!(d.repeated_positions, vec!["600519".to_string()]);
    }

    #[test]
    fn compute_risk_diagnosis_core_concentration() {
        let positions = vec![
            make_position("600519", "贵州茅台", 40_000.0),
            make_position("000858", "五粮液", 30_000.0),
            make_position("002230", "科大讯飞", 20_000.0),
            make_position("300750", "宁德时代", 10_000.0),
        ];
        let d = compute_risk_diagnosis(&positions);
        assert!((d.core_concentration.top3_weight - 90.0).abs() < 0.1);
        assert_eq!(d.core_concentration.level, "high");
        assert_eq!(d.core_concentration.top3_codes.len(), 3);
    }

    #[test]
    fn is_us_code_detects_alphabetic() {
        assert!(is_us_code("AAPL"));
        assert!(is_us_code("TSLA"));
        assert!(!is_us_code("600519"));
        assert!(!is_us_code("00700.HK"));
        assert!(!is_us_code(""));
    }

    #[test]
    fn is_defense_name_matches_keywords() {
        assert!(is_defense_name("招商银行"));
        assert!(is_defense_name("工商银行"));
        assert!(is_defense_name("长江电力"));
        assert!(!is_defense_name("贵州茅台"));
    }

    #[test]
    fn create_input_serialization_roundtrip() {
        let input = CreateScreenshotDiagnosisInput {
            image_hash: Some("abc123".into()),
            image_path: None,
            image_thumbnail_base64: None,
            image_width: Some(1080),
            image_height: Some(1920),
            source_app: Some("同花顺".into()),
            ocr_text: Some("贵州茅台 100股".into()),
            positions: vec![make_position("600519", "贵州茅台", 200_000.0)],
            total_market_value: 200_000.0,
            diagnosis: None,
            narrative: "集中度过高".into(),
            recommended_actions: vec!["减持贵州茅台".into()],
            source_workflow_execution_id: None,
            provider_id: Some("openai".into()),
            model_id: Some("gpt-4o".into()),
            status: "active".into(),
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: CreateScreenshotDiagnosisInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.positions.len(), 1);
        assert_eq!(parsed.source_app, Some("同花顺".into()));
    }
}
