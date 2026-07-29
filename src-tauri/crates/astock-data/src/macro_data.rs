//! 宏观经济数据 — 数据结构和获取接口
//!
//! 覆盖：GDP、CPI、PMI、利率（LPR/MLF/Shibor）、社融、货币供应量（M0/M1/M2）、
//! 进出口、工业增加值、固定投资、消费零售、外汇储备、汇率等。
//!
//! ## Vendor 适配
//!
//! 数据实际从以下渠道获取：
//! - **国家统计局 / 央行**（通过 neodata / eastmoney / akshare 代理）
//! - 目前为数据结构层 + 占位实现，TODO: 接入真实数据源

use serde::{Deserialize, Serialize};

/// 宏观经济数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDataPoint {
    /// 指标名（如 "gdp", "cpi", "pmi"）
    pub indicator: String,
    /// 指标显示名（如 "国内生产总值(GDP)"）
    pub display_name: String,
    /// 统计期间（如 "2025Q3", "2025-09", "2025"）
    pub period: String,
    /// 数值
    pub value: f64,
    /// 同比（%），部分指标有
    pub yoy: Option<f64>,
    /// 环比（%），部分指标有
    pub mom: Option<f64>,
    /// 单位
    pub unit: String,
    /// 数据来源
    pub source: String,
    /// 发布时间
    pub release_date: Option<String>,
}

/// 宏观数据集合（一次性获取全部可用数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDataSnapshot {
    /// 数据日期（获取时的有效日期）
    pub snapshot_date: String,
    /// 最新 GDP 数据
    pub gdp: Option<MacroDataPoint>,
    /// 最新 CPI 数据
    pub cpi: Option<MacroDataPoint>,
    /// 最新 PPI 数据
    pub ppi: Option<MacroDataPoint>,
    /// 最新 PMI 数据（制造业）
    pub pmi_manufacturing: Option<MacroDataPoint>,
    /// 最新 PMI 数据（非制造业）
    pub pmi_non_manufacturing: Option<MacroDataPoint>,
    /// 最新 LPR 1年期
    pub lpr_1y: Option<MacroDataPoint>,
    /// 最新 LPR 5年期
    pub lpr_5y: Option<MacroDataPoint>,
    /// 最新 Shibor 隔夜
    pub shibor_on: Option<MacroDataPoint>,
    /// 最新 MLF 利率
    pub mlf_rate: Option<MacroDataPoint>,
    /// 社会融资规模存量同比
    pub social_financing_yoy: Option<MacroDataPoint>,
    /// M2 货币供应量同比
    pub m2_yoy: Option<MacroDataPoint>,
    /// M1 货币供应量同比
    pub m1_yoy: Option<MacroDataPoint>,
    /// 新增人民币贷款
    pub new_loan: Option<MacroDataPoint>,
    /// 出口同比
    pub export_yoy: Option<MacroDataPoint>,
    /// 进口同比
    pub import_yoy: Option<MacroDataPoint>,
    /// 工业增加值同比
    pub industrial_output: Option<MacroDataPoint>,
    /// 社会消费品零售总额同比
    pub retail_sales: Option<MacroDataPoint>,
    /// 固定资产投资累计同比
    pub fixed_asset_investment: Option<MacroDataPoint>,
    /// 外汇储备（亿美元）
    pub forex_reserve: Option<MacroDataPoint>,
    /// 美元/人民币汇率（中间价）
    pub usd_cny: Option<MacroDataPoint>,
    /// 全部数据聚合列表（方便循环遍历）
    #[serde(skip)]
    pub all: Vec<MacroDataPoint>,
}

impl MacroDataSnapshot {
    /// 创建一个空的快照
    pub fn empty(date: &str) -> Self {
        Self {
            snapshot_date: date.to_string(),
            gdp: None,
            cpi: None,
            ppi: None,
            pmi_manufacturing: None,
            pmi_non_manufacturing: None,
            lpr_1y: None,
            lpr_5y: None,
            shibor_on: None,
            mlf_rate: None,
            social_financing_yoy: None,
            m2_yoy: None,
            m1_yoy: None,
            new_loan: None,
            export_yoy: None,
            import_yoy: None,
            industrial_output: None,
            retail_sales: None,
            fixed_asset_investment: None,
            forex_reserve: None,
            usd_cny: None,
            all: Vec::new(),
        }
    }
}

/// 宏观数据客户端（占位，TODO: 接入 vendor 数据）
///
/// 当前返回模拟/空数据，真实实现需要接入 neodata / eastmoney / akshare 等 vendor
/// 的宏观数据 API。
pub struct MacroDataClient;

impl Default for MacroDataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroDataClient {
    pub fn new() -> Self {
        Self
    }

    /// 获取宏观经济数据快照
    ///
    /// ⚠️ 当前为占位实现（mock 数据），真实数据源接入前不应在生产决策中依赖此返回值。
    /// TODO: 调用 vendor API 获取真实数据
    /// - eastmoney: /api/qt/schd/data?type=HG
    /// - neodata: macro indicators
    /// - akshare: macro_china_*
    pub async fn snapshot(&self) -> MacroDataSnapshot {
        // 修复(2026-07-29):
        // 1. 原 `chrono::Local::now()` 在非中国时区部署时会偏移一天,改用 UTC+8 固定时区。
        // 2. 原 mock 数据 source 字段写成 "国家统计局"/"中国人民银行" 会误导下游
        //    把 mock 当真实数据使用,改为 "mock(占位实现)" 显式标注。
        // 3. 增加 tracing::warn! 警告日志,让运行时可见当前返回的是 mock 数据。
        tracing::warn!(
            "[macro_data] MacroDataClient.snapshot() 返回 mock 占位数据,TODO: 接入真实 vendor API"
        );
        let today = chrono::Utc::now()
            .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
            .format("%Y-%m-%d")
            .to_string();
        let mut snap = MacroDataSnapshot::empty(&today);

        // mock 数据 source 统一标注为 "mock(占位实现)",避免下游误判为真实数据
        const MOCK_SOURCE: &str = "mock(占位实现)";
        snap.all = vec![
            MacroDataPoint {
                indicator: "gdp".into(),
                display_name: "国内生产总值(GDP)".into(),
                period: "2025Q3".into(),
                value: 4.6,
                yoy: Some(4.6),
                mom: Some(1.3),
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-10-18".into()),
            },
            MacroDataPoint {
                indicator: "cpi".into(),
                display_name: "居民消费价格指数(CPI)".into(),
                period: "2025-12".into(),
                value: 0.2,
                yoy: Some(0.2),
                mom: Some(0.1),
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2026-01-09".into()),
            },
            MacroDataPoint {
                indicator: "pmi_manufacturing".into(),
                display_name: "制造业采购经理指数(PMI)".into(),
                period: "2025-12".into(),
                value: 50.1,
                yoy: None,
                mom: Some(-0.2),
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-31".into()),
            },
            MacroDataPoint {
                indicator: "lpr_1y".into(),
                display_name: "LPR 1年期".into(),
                period: "2025-12".into(),
                value: 3.10,
                yoy: None,
                mom: None,
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-20".into()),
            },
            MacroDataPoint {
                indicator: "lpr_5y".into(),
                display_name: "LPR 5年期以上".into(),
                period: "2025-12".into(),
                value: 3.60,
                yoy: None,
                mom: None,
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-20".into()),
            },
            MacroDataPoint {
                indicator: "m2_yoy".into(),
                display_name: "M2 货币供应量同比".into(),
                period: "2025-11".into(),
                value: 7.1,
                yoy: Some(7.1),
                mom: None,
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-13".into()),
            },
            MacroDataPoint {
                indicator: "social_financing_yoy".into(),
                display_name: "社融存量同比".into(),
                period: "2025-11".into(),
                value: 8.0,
                yoy: Some(8.0),
                mom: None,
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-13".into()),
            },
            MacroDataPoint {
                indicator: "industrial_output".into(),
                display_name: "工业增加值同比".into(),
                period: "2025-11".into(),
                value: 5.4,
                yoy: Some(5.4),
                mom: Some(0.3),
                unit: "%".into(),
                source: MOCK_SOURCE.into(),
                release_date: Some("2025-12-16".into()),
            },
        ];

        // 映射到命名字段
        for dp in &snap.all {
            match dp.indicator.as_str() {
                "gdp" => snap.gdp = Some(dp.clone()),
                "cpi" => snap.cpi = Some(dp.clone()),
                "pmi_manufacturing" => snap.pmi_manufacturing = Some(dp.clone()),
                "lpr_1y" => snap.lpr_1y = Some(dp.clone()),
                "lpr_5y" => snap.lpr_5y = Some(dp.clone()),
                "m2_yoy" => snap.m2_yoy = Some(dp.clone()),
                "social_financing_yoy" => snap.social_financing_yoy = Some(dp.clone()),
                "industrial_output" => snap.industrial_output = Some(dp.clone()),
                _ => {},
            }
        }

        snap
    }
}
