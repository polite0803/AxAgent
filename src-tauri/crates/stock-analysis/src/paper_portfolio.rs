// SPDX-License-Identifier: AGPL-3.0-only
//! G2 模拟观察组合（Paper Trading Portfolio）服务
//!
//! ## 用途
//!
//! 承接 DojoAgents 宣传场景 1/2/3 的研究观察列表 / 模拟建仓实体：
//! - 场景 1：把市场异动摘要沉淀成研究观察列表（持续跟踪后续表现）
//! - 场景 2：按消息发布日价格虚拟建仓（观察后市表现）
//! - 场景 3：从持仓诊断结果生成新观察列表
//!
//! ## 与 PortfolioMonitor 的区别
//!
//! - [`crate::portfolio_monitor`]：聚合**真实**持仓（trades 表）的实时盈亏/相关性/压测
//! - 本模块：管理**虚拟**组合（paper_portfolios/paper_positions 表），用于
//!   研究观察，不涉及真实资金
//!
//! ## 数据流
//!
//! ```text
//! 触发源（新闻 / 诊断 / 手动）
//!   → create_portfolio / create_portfolio_from_news / create_portfolio_from_screenshot
//!   → add_position（按事件日价格建仓）
//!   → close_position / close_portfolio
//!   → compute_portfolio_performance（拉最新 K 线计算盈亏）
//! ```
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
use sea_orm::Set;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axagent_entities::paper_portfolios as pp_entity;
use axagent_entities::paper_positions as pos_entity;
use axagent_harness::market_data::MarketDataProvider;

// ── DTO ───────────────────────────────────────────────────────────────────

/// 创建模拟组合的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePortfolioInput {
    pub name: String,
    /// 来源事件描述（如 "英伟达隔夜大跌"）
    pub source_event: String,
    /// 关联 news_archive.id（可空）
    pub source_news_id: Option<String>,
    /// 关联 screenshot_diagnoses.id（G6 用，可空）
    pub source_screenshot_diagnosis_id: Option<String>,
}

/// 添加虚拟持仓的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPositionInput {
    pub portfolio_id: String,
    pub symbol: String,
    /// 市场：A / US / HK / ETF（默认 A）
    #[serde(default = "default_market")]
    pub market: String,
    pub entry_price: f64,
    /// YYYY-MM-DD
    pub entry_date: String,
    pub quantity: f64,
    /// 备注（如 "AI 算力链" / "光模块龙头"）
    pub note: Option<String>,
}

fn default_market() -> String {
    "A".to_string()
}

/// 平仓输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosePositionInput {
    pub position_id: String,
    pub exit_price: f64,
    /// YYYY-MM-DD
    pub exit_date: String,
}

/// 组合详情（含持仓 + 实时盈亏）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioDetail {
    #[serde(flatten)]
    pub portfolio: pp_entity::Model,
    pub positions: Vec<PositionWithPnl>,
    /// 组合汇总指标
    pub summary: PortfolioSummary,
}

/// 单个持仓 + 实时盈亏
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionWithPnl {
    #[serde(flatten)]
    pub position: pos_entity::Model,
    /// 最新价（实时拉取，可空表示拉取失败）
    pub current_price: Option<f64>,
    /// 浮动盈亏（元）—— 仅 open 持仓计算
    pub unrealized_pnl: Option<f64>,
    /// 浮动盈亏（百分比）
    pub unrealized_pnl_pct: Option<f64>,
    /// 已实现盈亏（元）—— 仅 closed 持仓计算
    pub realized_pnl: Option<f64>,
    /// 已实现盈亏（百分比）
    pub realized_pnl_pct: Option<f64>,
}

/// 组合汇总
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSummary {
    /// 持仓总数
    pub position_count: usize,
    /// 仍持仓数量
    pub open_count: usize,
    /// 已平仓数量
    pub closed_count: usize,
    /// 总成本（按 entry_price × quantity 累加）
    pub total_cost: f64,
    /// 当前总市值（open 持仓按最新价，closed 按 exit_price）
    pub total_market_value: f64,
    /// 总浮动盈亏（元）
    pub total_unrealized_pnl: f64,
    /// 总已实现盈亏（元）
    pub total_realized_pnl: f64,
    /// 总收益率（百分比，基于 total_cost）
    pub total_return_pct: f64,
}

// ── 组合 CRUD ─────────────────────────────────────────────────────────────

/// 创建模拟组合
pub async fn create_portfolio(
    db: &DatabaseConnection,
    input: CreatePortfolioInput,
) -> Result<pp_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let model = pp_entity::ActiveModel {
        id: Set(id),
        name: Set(input.name),
        source_event: Set(input.source_event),
        source_news_id: Set(input.source_news_id),
        source_screenshot_diagnosis_id: Set(input.source_screenshot_diagnosis_id),
        status: Set("active".to_string()),
        created_at: Set(now),
        closed_at: Set(None),
    };
    let inserted = model.insert(db).await?;
    Ok(inserted)
}

/// 列出所有组合（按状态过滤，默认全部）
pub async fn list_portfolios(
    db: &DatabaseConnection,
    status: Option<&str>,
) -> Result<Vec<pp_entity::Model>, DbErr> {
    let mut query = pp_entity::Entity::find();
    if let Some(s) = status {
        query = query.filter(pp_entity::Column::Status.eq(s));
    }
    query.order_by_desc(pp_entity::Column::CreatedAt).all(db).await
}

/// 获取单个组合（不含持仓）
pub async fn get_portfolio(
    db: &DatabaseConnection,
    portfolio_id: &str,
) -> Result<Option<pp_entity::Model>, DbErr> {
    pp_entity::Entity::find_by_id(portfolio_id.to_string()).one(db).await
}

/// 关闭组合（status=closed，写 closed_at）—— 不影响持仓状态
pub async fn close_portfolio(
    db: &DatabaseConnection,
    portfolio_id: &str,
) -> Result<pp_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let model = pp_entity::ActiveModel {
        id: Set(portfolio_id.to_string()),
        status: Set("closed".to_string()),
        closed_at: Set(Some(now)),
        ..Default::default()
    };
    model.update(db).await
}

/// 归档组合（status=archived，不写 closed_at）—— 用于长期不跟踪
pub async fn archive_portfolio(
    db: &DatabaseConnection,
    portfolio_id: &str,
) -> Result<pp_entity::Model, DbErr> {
    let model = pp_entity::ActiveModel {
        id: Set(portfolio_id.to_string()),
        status: Set("archived".to_string()),
        ..Default::default()
    };
    model.update(db).await
}

// ── 持仓 CRUD ─────────────────────────────────────────────────────────────

/// 添加虚拟持仓
pub async fn add_position(
    db: &DatabaseConnection,
    input: AddPositionInput,
) -> Result<pos_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let model = pos_entity::ActiveModel {
        id: Set(id),
        portfolio_id: Set(input.portfolio_id),
        symbol: Set(input.symbol),
        market: Set(input.market),
        entry_price: Set(input.entry_price),
        entry_date: Set(input.entry_date),
        quantity: Set(input.quantity),
        exit_price: Set(None),
        exit_date: Set(None),
        status: Set("open".to_string()),
        note: Set(input.note),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = model.insert(db).await?;
    Ok(inserted)
}

/// 列出组合内全部持仓
pub async fn list_positions(
    db: &DatabaseConnection,
    portfolio_id: &str,
) -> Result<Vec<pos_entity::Model>, DbErr> {
    pos_entity::Entity::find()
        .filter(pos_entity::Column::PortfolioId.eq(portfolio_id))
        .order_by_asc(pos_entity::Column::CreatedAt)
        .all(db)
        .await
}

/// 平仓单个持仓
pub async fn close_position(
    db: &DatabaseConnection,
    input: ClosePositionInput,
) -> Result<pos_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let model = pos_entity::ActiveModel {
        id: Set(input.position_id),
        exit_price: Set(Some(input.exit_price)),
        exit_date: Set(Some(input.exit_date)),
        status: Set("closed".to_string()),
        updated_at: Set(now),
        ..Default::default()
    };
    model.update(db).await
}

/// 批量平仓（按 portfolio_id 平仓所有 open 持仓，统一使用同一 exit_price/exit_date）
pub async fn close_all_positions(
    db: &DatabaseConnection,
    portfolio_id: &str,
    exit_price: f64,
    exit_date: &str,
) -> Result<u64, DbErr> {
    // R1-修复: 校验 exit_price 合法性，避免 0 或负数导致 realized_pnl 计算异常
    if exit_price <= 0.0 {
        return Err(DbErr::Custom("exit_price must be positive".into()));
    }
    let now = chrono::Utc::now().timestamp_millis();
    let res = pos_entity::Entity::update_many()
        .col_expr(pos_entity::Column::ExitPrice, Expr::value(exit_price))
        .col_expr(pos_entity::Column::ExitDate, Expr::value(exit_date))
        .col_expr(pos_entity::Column::Status, Expr::value("closed"))
        .col_expr(pos_entity::Column::UpdatedAt, Expr::value(now))
        .filter(pos_entity::Column::PortfolioId.eq(portfolio_id))
        .filter(pos_entity::Column::Status.eq("open"))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}

// ── 性能计算 ───────────────────────────────────────────────────────────────

/// 获取组合详情（含持仓 + 实时盈亏）
///
/// - open 持仓：拉最新价 → 计算 unrealized_pnl
/// - closed 持仓：用 exit_price → 计算 realized_pnl
///
/// 单只标的拉取失败不影响整体，对应字段返回 None。
pub async fn get_portfolio_detail(
    db: &DatabaseConnection,
    market_data: &dyn MarketDataProvider,
    portfolio_id: &str,
) -> Result<Option<PortfolioDetail>, DbErr> {
    let portfolio = match pp_entity::Entity::find_by_id(portfolio_id.to_string()).one(db).await? {
        Some(p) => p,
        None => return Ok(None),
    };
    let positions = list_positions(db, portfolio_id).await?;
    let mut details = Vec::with_capacity(positions.len());
    for pos in positions {
        let detail = enrich_position(&pos, market_data).await;
        details.push(detail);
    }
    let summary = summarize(&details);
    Ok(Some(PortfolioDetail { portfolio, positions: details, summary }))
}

/// 列出所有 active 组合的详情（用于前端 Dashboard）
pub async fn list_active_portfolios_detail(
    db: &DatabaseConnection,
    market_data: &dyn MarketDataProvider,
) -> Result<Vec<PortfolioDetail>, DbErr> {
    let portfolios = pp_entity::Entity::find()
        .filter(pp_entity::Column::Status.eq("active"))
        .order_by_desc(pp_entity::Column::CreatedAt)
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(portfolios.len());
    for p in portfolios {
        let positions = list_positions(db, &p.id).await?;
        let mut details = Vec::with_capacity(positions.len());
        for pos in positions {
            details.push(enrich_position(&pos, market_data).await);
        }
        let summary = summarize(&details);
        out.push(PortfolioDetail { portfolio: p, positions: details, summary });
    }
    Ok(out)
}

/// 给单条持仓附加盈亏信息（拉最新价）
async fn enrich_position(
    pos: &pos_entity::Model,
    market_data: &dyn MarketDataProvider,
) -> PositionWithPnl {
    let (current_price, unrealized_pnl, unrealized_pnl_pct, realized_pnl, realized_pnl_pct) =
        if pos.status == "open" {
            // 拉最新价
            match market_data.get_quote(&pos.symbol).await {
                Ok(q) => {
                    let cur = q.price;
                    let cost = pos.entry_price * pos.quantity;
                    let mv = cur * pos.quantity;
                    let pnl = mv - cost;
                    let pnl_pct = if cost > 0.0 {
                        (pnl / cost) * 100.0
                    } else {
                        0.0
                    };
                    (Some(cur), Some(pnl), Some(pnl_pct), None, None)
                },
                Err(_) => (None, None, None, None, None),
            }
        } else {
            // closed：用 exit_price 计算 realized_pnl
            let exit_price = pos.exit_price.unwrap_or(0.0);
            let cost = pos.entry_price * pos.quantity;
            let mv = exit_price * pos.quantity;
            let pnl = mv - cost;
            let pnl_pct = if cost > 0.0 {
                (pnl / cost) * 100.0
            } else {
                0.0
            };
            (None, None, None, Some(pnl), Some(pnl_pct))
        };

    PositionWithPnl {
        position: pos.clone(),
        current_price,
        unrealized_pnl,
        unrealized_pnl_pct,
        realized_pnl,
        realized_pnl_pct,
    }
}

/// 聚合组合指标
fn summarize(positions: &[PositionWithPnl]) -> PortfolioSummary {
    let mut s = PortfolioSummary { position_count: positions.len(), ..PortfolioSummary::default() };
    for p in positions {
        let cost = p.position.entry_price * p.position.quantity;
        s.total_cost += cost;
        if p.position.status == "open" {
            s.open_count += 1;
            // 用最新价（拉取失败退化为 entry_price）
            let price = p.current_price.unwrap_or(p.position.entry_price);
            s.total_market_value += price * p.position.quantity;
            s.total_unrealized_pnl += p.unrealized_pnl.unwrap_or(0.0);
        } else {
            s.closed_count += 1;
            let exit_price = p.position.exit_price.unwrap_or(p.position.entry_price);
            s.total_market_value += exit_price * p.position.quantity;
            s.total_realized_pnl += p.realized_pnl.unwrap_or(0.0);
        }
    }
    s.total_return_pct = if s.total_cost > 0.0 {
        ((s.total_unrealized_pnl + s.total_realized_pnl) / s.total_cost) * 100.0
    } else {
        0.0
    };
    s
}

// ── 场景化封装 ─────────────────────────────────────────────────────────────

/// 场景 2：从新闻创建模拟组合（自动填 source_news_id）
pub async fn create_portfolio_from_news(
    db: &DatabaseConnection,
    news_id: &str,
    name: &str,
    source_event: &str,
) -> Result<pp_entity::Model, DbErr> {
    create_portfolio(
        db,
        CreatePortfolioInput {
            name: name.to_string(),
            source_event: source_event.to_string(),
            source_news_id: Some(news_id.to_string()),
            source_screenshot_diagnosis_id: None,
        },
    )
    .await
}

/// 场景 3：从截图诊断结果创建观察列表（自动填 source_screenshot_diagnosis_id）
pub async fn create_portfolio_from_screenshot_diagnosis(
    db: &DatabaseConnection,
    diagnosis_id: &str,
    name: &str,
    source_event: &str,
) -> Result<pp_entity::Model, DbErr> {
    create_portfolio(
        db,
        CreatePortfolioInput {
            name: name.to_string(),
            source_event: source_event.to_string(),
            source_news_id: None,
            source_screenshot_diagnosis_id: Some(diagnosis_id.to_string()),
        },
    )
    .await
}

// ── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_empty() {
        let s = summarize(&[]);
        assert_eq!(s.position_count, 0);
        assert_eq!(s.total_cost, 0.0);
        assert_eq!(s.total_return_pct, 0.0);
    }

    #[test]
    fn summarize_with_open_and_closed() {
        // 模拟两条持仓：一条 open，一条 closed
        let open_pos = pos_entity::Model {
            id: "p1".into(),
            portfolio_id: "pf1".into(),
            symbol: "600519".into(),
            market: "A".into(),
            entry_price: 100.0,
            entry_date: "2026-01-01".into(),
            quantity: 100.0,
            exit_price: None,
            exit_date: None,
            status: "open".into(),
            note: None,
            created_at: 0,
            updated_at: 0,
        };
        let closed_pos = pos_entity::Model {
            id: "p2".into(),
            portfolio_id: "pf1".into(),
            symbol: "000858".into(),
            market: "A".into(),
            entry_price: 50.0,
            entry_date: "2026-01-01".into(),
            quantity: 200.0,
            exit_price: Some(60.0),
            exit_date: Some("2026-02-01".into()),
            status: "closed".into(),
            note: None,
            created_at: 0,
            updated_at: 0,
        };
        let details = vec![
            PositionWithPnl {
                position: open_pos,
                current_price: Some(120.0),
                unrealized_pnl: Some(2000.0), // (120-100)*100
                unrealized_pnl_pct: Some(20.0),
                realized_pnl: None,
                realized_pnl_pct: None,
            },
            PositionWithPnl {
                position: closed_pos,
                current_price: None,
                unrealized_pnl: None,
                unrealized_pnl_pct: None,
                realized_pnl: Some(2000.0), // (60-50)*200
                realized_pnl_pct: Some(20.0),
            },
        ];
        let s = summarize(&details);
        assert_eq!(s.position_count, 2);
        assert_eq!(s.open_count, 1);
        assert_eq!(s.closed_count, 1);
        // 总成本 = 100*100 + 50*200 = 10000 + 10000 = 20000
        assert_eq!(s.total_cost, 20000.0);
        // 总市值 = 120*100 + 60*200 = 12000 + 12000 = 24000
        assert_eq!(s.total_market_value, 24000.0);
        // 浮动 + 已实现 = 2000 + 2000 = 4000
        assert_eq!(s.total_unrealized_pnl, 2000.0);
        assert_eq!(s.total_realized_pnl, 2000.0);
        // 总收益率 = 4000 / 20000 * 100 = 20%
        assert!((s.total_return_pct - 20.0).abs() < 1e-6);
    }
}
