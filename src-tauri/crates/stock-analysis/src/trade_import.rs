//! 交易记录导入 — 从券商/通达信导出的 CSV 文件批量导入成交记录。
//!
//! 支持的格式：
//! - 通达信客户端导出的成交记录（标准 TDX 列名）
//! - 东方财富证券导出的成交记录
//! - 通用格式（通过列名映射）

use csv::ReaderBuilder;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::path::Path;

// ── 导入结果类型 ──

/// 单行解析结果（预览用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRow {
    pub row: usize,
    pub stock_code: String,
    pub stock_name: String,
    pub direction: String, // "buy" | "sell"
    pub price: f64,
    pub quantity: i32,
    pub trade_date: String,
    pub trade_time: String,
    pub fee: Option<f64>,
    pub notes: Option<String>,
    pub errors: Vec<String>,
}

/// 导入摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub total: usize,
    pub valid: usize,
    pub skipped: usize, // 重复跳过
    pub failed: usize,
    pub errors: Vec<(usize, String)>, // (row_index, error_msg)
    pub preview: Vec<ImportRow>,
}

// ── 列名映射 ──

/// 将 CSV 中文列名标准化为内部字段名
fn normalize_header(name: &str) -> String {
    match name.trim() {
        // 证券代码
        "证券代码" | "股票代码" | "代码" | "合约代码" | "合约" | "StockCode" | "stock_code"
        | "sec_code" | "secCode" | "code" => "stock_code".into(),
        // 证券名称
        "证券名称" | "股票名称" | "名称" | "合约名称" | "StockName" | "stock_name" | "sec_name"
        | "name" => "stock_name".into(),
        // 买卖方向
        "买卖方向" | "买卖标志" | "方向" | "成交方向" | "业务类型" | "业务名称" | "Direction"
        | "direction" | "bs_flag" | "bsFlag" | "BSFlag" => "direction".into(),
        // 成交日期
        "成交日期" | "发生日期" | "日期" | "交易日期" | "TradeDate" | "trade_date" | "date"
        | "trd_date" => "trade_date".into(),
        // 成交时间
        "成交时间" | "发生时间" | "时间" | "交易时间" | "TradeTime" | "trade_time" | "time"
        | "trd_time" => "trade_time".into(),
        // 成交价格
        "成交价格" | "成交价" | "价格" | "委托价格" | "Price" | "price" | "trd_price"
        | "trade_price" => "price".into(),
        // 成交数量
        "成交数量" | "数量" | "股数" | "成交量" | "Volume" | "volume" | "quantity" | "qty"
        | "shares" => "quantity".into(),
        // 成交金额
        "成交金额" | "金额" | "发生金额" | "成交额" | "Amount" | "amount" | "trd_amount"
        | "trade_amount" | "sum_money" => "amount".into(),
        // 佣金/手续费
        "佣金" | "手续费" | "交易费用" | "Fee" | "fee" | "commission" | "brokerage"
        | "broker_fee" => "fee".into(),
        // 备注
        "备注" | "说明" | "Notes" | "notes" | "remark" => "notes".into(),
        other => other.to_lowercase().replace(' ', "_"),
    }
}

/// 解析买卖方向
fn parse_direction(s: &str) -> Result<String, String> {
    match s.trim() {
        "买入" | "买" | "buy" | "Buy" | "B" | "1" | "0" => Ok("buy".into()),
        "卖出" | "卖" | "sell" | "Sell" | "S" | "2" | "卖出成交" => Ok("sell".into()),
        other => Err(format!("无法识别的买卖方向: '{other}'（请使用 买入/卖出）")),
    }
}

/// 尝试解析数值（支持含逗号和空值）
fn parse_float(s: &str) -> Option<f64> {
    let cleaned: String = s.trim().chars().filter(|c| *c != ',').collect();
    if cleaned.is_empty() || cleaned == "--" || cleaned == "-" {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

/// 尝试解析整数（同上）
fn parse_int(s: &str) -> Option<i32> {
    let cleaned: String = s.trim().chars().filter(|c| *c != ',').collect();
    if cleaned.is_empty() || cleaned == "--" || cleaned == "-" {
        return None;
    }
    cleaned.parse::<i32>().ok()
}

/// 标准化日期格式（支持多种输入格式 → YYYY-MM-DD）
fn normalize_date(s: &str) -> Option<String> {
    let s = s.trim();
    // YYYY-MM-DD
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // YYYYMMDD
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y%m%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // YYYY/MM/DD
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // MM/DD/YY
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%m/%d/%y") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    None
}

/// 标准化时间格式（HH:MM:SS → HH:MM）
fn normalize_time(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 5 {
        // 截取前5个字符 HH:MM
        s[..5].to_string()
    } else {
        "15:00".into()
    }
}

// ── CSV 解析器 ──

/// 解析 CSV 文件，返回解析行（带错误标记）
pub fn parse_csv(file_path: &str) -> Result<Vec<ImportRow>, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {file_path}"));
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| format!("无法打开 CSV 文件: {e}"))?;

    // 读取并规范化表头
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("CSV 表头读取失败: {e}"))?
        .iter()
        .map(normalize_header)
        .collect();

    // 检查必填列
    let col_code = find_col(&headers, "stock_code");
    let col_name = find_col(&headers, "stock_name").or_else(|| find_col(&headers, "code"));
    let col_dir = find_col(&headers, "direction");
    let col_date = find_col(&headers, "trade_date");
    let col_time = find_col(&headers, "trade_time");
    let col_price = find_col(&headers, "price");
    let col_qty = find_col(&headers, "quantity");
    let col_fee = find_col(&headers, "fee");
    let col_notes = find_col(&headers, "notes");

    // 统计找到的列
    let missing: Vec<&str> = [
        (col_code.is_some(), "证券代码(stock_code)"),
        (col_price.is_some(), "成交价格(price)"),
        (col_qty.is_some(), "成交数量(quantity)"),
        (col_dir.is_some(), "买卖方向(direction)"),
        (col_date.is_some(), "成交日期(trade_date)"),
        (col_name.is_some(), "证券名称(可选)"),
        (col_time.is_some(), "成交时间(可选)"),
    ]
    .iter()
    .filter(|(found, _)| !found)
    .map(|(_, name)| *name)
    .collect();

    if missing.len() > 2 {
        return Err(format!(
            "CSV 缺少必要列: {}。\n找到的列: {}\n请确保文件包含 证券代码、成交价格、成交数量、买卖方向、成交日期 等列。",
            missing.join(", "),
            headers.join(", ")
        ));
    }

    let mut rows = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let row_num = idx + 2; // +2 because header is row 1
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                rows.push(ImportRow {
                    row: row_num,
                    stock_code: String::new(),
                    stock_name: String::new(),
                    direction: String::new(),
                    price: 0.0,
                    quantity: 0,
                    trade_date: String::new(),
                    trade_time: "15:00".into(),
                    fee: None,
                    notes: None,
                    errors: vec![format!("CSV 解析错误: {e}")],
                });
                continue;
            },
        };

        let mut errors: Vec<String> = Vec::new();

        // 读取各字段
        let stock_code = get_col(&record, col_code).unwrap_or_default();
        let stock_name = get_col(&record, col_name).unwrap_or_default();
        let direction_raw = get_col(&record, col_dir).unwrap_or_default();
        let date_raw = get_col(&record, col_date).unwrap_or_default();
        let time_raw = get_col(&record, col_time).unwrap_or_default();
        let price_raw = get_col(&record, col_price).unwrap_or_default();
        let qty_raw = get_col(&record, col_qty).unwrap_or_default();
        let fee_raw = get_col(&record, col_fee).unwrap_or_default();
        let notes_raw = get_col(&record, col_notes).unwrap_or_default();

        // 解析买卖方向
        let direction = match parse_direction(&direction_raw) {
            Ok(d) => d,
            Err(e) => {
                errors.push(e);
                String::new()
            },
        };

        // 解析价格
        let price = parse_float(&price_raw).unwrap_or_else(|| {
            errors.push(format!("成交价格无法解析: '{price_raw}'"));
            0.0
        });

        // 解析数量
        let quantity = parse_int(&qty_raw).unwrap_or_else(|| {
            errors.push(format!("成交数量无法解析: '{qty_raw}'"));
            0
        });

        // 解析日期
        let trade_date = normalize_date(&date_raw).unwrap_or_else(|| {
            errors.push(format!("成交日期无法解析: '{date_raw}'，期望格式 YYYY-MM-DD"));
            date_raw
        });

        let trade_time = normalize_time(&time_raw);

        // 解析手续费
        let fee = parse_float(&fee_raw);

        // 验证基本合法性
        if stock_code.is_empty() {
            errors.push("证券代码为空".into());
        }
        if price <= 0.0 && !errors.iter().any(|e| e.contains("价格")) {
            errors.push("价格必须 > 0".into());
        }
        if quantity <= 0 && !errors.iter().any(|e| e.contains("数量")) {
            errors.push("数量必须 > 0".into());
        }
        if trade_date.len() != 10 && !errors.iter().any(|e| e.contains("日期")) {
            errors.push("日期格式无效".into());
        }
        if direction.is_empty() && !errors.iter().any(|e| e.contains("方向")) {
            errors.push("买卖方向无法识别".into());
        }

        rows.push(ImportRow {
            row: row_num,
            stock_code,
            stock_name,
            direction,
            price,
            quantity,
            trade_date,
            trade_time,
            fee,
            notes: if notes_raw.is_empty() {
                None
            } else {
                Some(notes_raw)
            },
            errors,
        });
    }

    Ok(rows)
}

// ── 辅助函数 ──

fn find_col(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name)
}

fn get_col(record: &csv::StringRecord, col: Option<usize>) -> Option<String> {
    col.and_then(|i| record.get(i)).map(|s| s.to_string())
}

// ── 批量导入 ──

/// 批量导入交易记录（直接写入数据库）
pub async fn batch_import_trades(
    db: &DatabaseConnection,
    rows: &[ImportRow],
) -> Result<ImportSummary, String> {
    let mut summary = ImportSummary {
        total: rows.len(),
        valid: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
        preview: Vec::new(),
    };

    for row in rows {
        if !row.errors.is_empty() {
            summary.failed += 1;
            summary.errors.push((row.row, row.errors.join("; ")));
            continue;
        }

        // 查重：同一股票+同方向+同日期+同价格+同数量 → 视为重复
        let is_dup = is_duplicate_trade(db, row).await.unwrap_or(false);
        if is_dup {
            summary.skipped += 1;
            continue;
        }

        // 使用 trading engine 的逻辑写入数据库
        // 这里直接复用 upsert_position + trades::ActiveModel
        let now = chrono::Utc::now().timestamp_millis();
        let trade_id = uuid::Uuid::new_v4().to_string();

        // 买入：合并持仓；卖出：减仓并计算盈亏
        if row.direction == "buy" {
            upsert_position_import(
                db,
                &row.stock_code,
                &row.stock_name,
                row.quantity as f64,
                row.price,
            )
            .await?;
        } else {
            // 卖出：减仓
            let holdings = axagent_entities::portfolio_holdings::Entity::find()
                .filter(axagent_entities::portfolio_holdings::Column::StockCode.eq(&row.stock_code))
                .one(db)
                .await
                .map_err(|e| e.to_string())?;

            if let Some(h) = holdings {
                let sell_qty = row.quantity as f64;
                let cost = h.avg_cost * sell_qty;
                let revenue = row.price * sell_qty;
                let _realized_pnl = revenue - cost;

                let remaining = h.shares - sell_qty;
                if remaining <= 0.0 {
                    axagent_entities::portfolio_holdings::Entity::delete_by_id(&h.id)
                        .exec(db)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    use sea_orm::Set;
                    let mut holding: axagent_entities::portfolio_holdings::ActiveModel = h.into();
                    holding.shares = Set(remaining);
                    holding.updated_at = Set(now);
                    holding.update(db).await.map_err(|e| e.to_string())?;
                }
            }
        }

        // 写入交易记录
        use axagent_entities::trades;
        use sea_orm::ActiveModelTrait;
        use sea_orm::Set;

        let trade = trades::ActiveModel {
            id: Set(trade_id),
            stock_code: Set(row.stock_code.clone()),
            stock_name: Set(row.stock_name.clone()),
            direction: Set(row.direction.clone()),
            price: Set(row.price),
            quantity: Set(row.quantity),
            trade_date: Set(row.trade_date.clone()),
            trade_time: Set(row.trade_time.clone()),
            fee: Set(row.fee),
            strategy: Set(None),
            realized_pnl: Set(None), // 会在卖出时重新计算
            notes: Set(row.notes.clone()),
            created_at: Set(now),
        };

        trade
            .insert(db)
            .await
            .map_err(|e| format!("第 {} 行写入失败: {e}", row.row))?;

        summary.valid += 1;
    }

    Ok(summary)
}

/// 检查是否为重复交易（同一股票 + 同方向 + 同日期 + 同价格 + 同数量）
async fn is_duplicate_trade(db: &DatabaseConnection, row: &ImportRow) -> Result<bool, String> {
    use axagent_entities::trades;
    use sea_orm::EntityTrait;

    let existing = trades::Entity::find()
        .filter(trades::Column::StockCode.eq(&row.stock_code))
        .filter(trades::Column::Direction.eq(&row.direction))
        .filter(trades::Column::TradeDate.eq(&row.trade_date))
        .filter(trades::Column::TradeTime.eq(&row.trade_time))
        .filter(trades::Column::Price.eq(row.price))
        .filter(trades::Column::Quantity.eq(row.quantity))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(existing.is_some())
}

/// 导入时的持仓更新（复制自 trading.rs 的逻辑，避免循环依赖）
async fn upsert_position_import(
    db: &DatabaseConnection,
    stock_code: &str,
    stock_name: &str,
    shares: f64,
    price: f64,
) -> Result<(), String> {
    use axagent_entities::portfolio_holdings;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    let existing = portfolio_holdings::Entity::find()
        .filter(portfolio_holdings::Column::StockCode.eq(stock_code))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    let now = chrono::Utc::now().timestamp_millis();

    if let Some(h) = existing {
        let old_total = h.shares * h.avg_cost;
        let new_total = shares * price;
        let new_shares = h.shares + shares;
        let new_avg_cost = if new_shares > 0.0 {
            (old_total + new_total) / new_shares
        } else {
            price
        };
        let mut holding: portfolio_holdings::ActiveModel = h.into();
        holding.shares = Set(new_shares);
        holding.avg_cost = Set(new_avg_cost);
        holding.updated_at = Set(now);
        holding.update(db).await.map_err(|e| e.to_string())?;
    } else {
        let model = portfolio_holdings::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            stock_code: Set(stock_code.to_string()),
            stock_name: Set(stock_name.to_string()),
            shares: Set(shares),
            avg_cost: Set(price),
            notes: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(db).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
