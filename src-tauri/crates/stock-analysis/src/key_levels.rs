use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::sync::Arc;

use axagent_astock_data::AStockClient;

/// 关键价位快照
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyLevelSnapshot {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub analysis_id: String,
    /// 分析日期
    pub snapshot_date: String,
    /// 支撑位
    pub support_level: f64,
    /// 压力位
    pub resistance_level: f64,
    /// 建议止损
    pub stop_loss: Option<f64>,
    /// 建议止盈
    pub take_profit: Option<f64>,
    /// 建议入场
    pub entry_price: Option<f64>,
    /// 快照时价格
    pub price_at_snapshot: f64,
    /// 回测命中统计
    pub hit_support_1d: Option<bool>,
    pub hit_resistance_1d: Option<bool>,
    pub hit_stop_loss_1d: Option<bool>,
    pub hit_take_profit_1d: Option<bool>,
    pub hit_support_3d: Option<bool>,
    pub hit_resistance_3d: Option<bool>,
    pub hit_support_5d: Option<bool>,
    pub hit_resistance_5d: Option<bool>,
    pub created_at: i64,
}

/// 关键价位追踪器
pub struct KeyLevelTracker {
    db: Arc<DatabaseConnection>,
    client: Arc<AStockClient>,
}

impl KeyLevelTracker {
    pub fn new(db: Arc<DatabaseConnection>, client: Arc<AStockClient>) -> Self {
        Self { db, client }
    }

    /// 从分析结果中提取关键价位并保存快照
    #[allow(clippy::too_many_arguments)]
    pub async fn capture_snapshot(
        &self,
        analysis_id: &str,
        _stock_code: &str,
        _stock_name: &str,
        support_level: f64,
        resistance_level: f64,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
        entry_price: Option<f64>,
        current_price: f64,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        use axagent_core::entity::stock_analyses;
        let analysis = stock_analyses::Entity::find_by_id(analysis_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or("分析记录不存在".to_string())?;

        let mut existing_bb: serde_json::Value = analysis
            .blackboard_snapshot
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));
        existing_bb["key_levels"] = serde_json::json!({
            "support": support_level,
            "resistance": resistance_level,
            "stop_loss": stop_loss,
            "take_profit": take_profit,
            "entry_price": entry_price,
            "price_at_snapshot": current_price,
            "snapshot_date": today,
        });

        stock_analyses::Entity::update_many()
            .col_expr(
                stock_analyses::Column::BlackboardSnapshot,
                sea_orm::sea_query::Expr::value(existing_bb.to_string()),
            )
            .filter(stock_analyses::Column::Id.eq(analysis_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(id)
    }

    /// 回测历史快照命中率（可配置参数）
    pub async fn backtest_key_levels_with_config(
        &self,
        _lookback_days: u32,
        cfg: &BacktestConfig,
    ) -> Result<KeyLevelBacktestStats, String> {
        use axagent_core::entity::stock_analyses;
        let analyses = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::Status.eq("completed"))
            .filter(stock_analyses::Column::BlackboardSnapshot.is_not_null())
            .order_by_desc(stock_analyses::Column::CreatedAt)
            .limit(Some(cfg.query_limit))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut stats = KeyLevelBacktestStats {
            total_snapshots: analyses.len() as u32,
            ..Default::default()
        };

        for analysis in analyses {
            if let Some(snapshot_str) = analysis.blackboard_snapshot.as_deref() {
                if let Ok(snapshot) = serde_json::from_str::<serde_json::Value>(snapshot_str) {
                    let kl = &snapshot["key_levels"];
                    let support = kl["support"].as_f64().unwrap_or(0.0);
                    let resistance = kl["resistance"].as_f64().unwrap_or(0.0);
                    let stop_loss = kl["stop_loss"].as_f64();
                    let take_profit = kl["take_profit"].as_f64();
                    let snapshot_date = kl["snapshot_date"].as_str().unwrap_or("");

                    // 获取快照日期之后的K线
                    if let Ok(klines) = self
                        .client
                        .get_klines(&analysis.stock_code, "daily", cfg.klines_count)
                        .await
                    {
                        let future: Vec<_> = klines
                            .iter()
                            .filter(|k| k.date.as_str() > snapshot_date)
                            .collect();

                        let day1 = future.get(cfg.day1_offset);
                        let day3 = future.get(cfg.day3_offset);
                        let day5 = future.get(cfg.day5_offset);

                        if let Some(k) = day1 {
                            if k.low <= support {
                                stats.support_hit_1d += 1;
                            }
                            if k.high >= resistance {
                                stats.resistance_hit_1d += 1;
                            }
                            if let Some(sl) = stop_loss {
                                if k.low <= sl {
                                    stats.stop_loss_hit_1d += 1;
                                }
                            }
                            if let Some(tp) = take_profit {
                                if k.high >= tp {
                                    stats.take_profit_hit_1d += 1;
                                }
                            }
                        }
                        if let Some(k) = day3 {
                            if k.low <= support {
                                stats.support_hit_3d += 1;
                            }
                            if k.high >= resistance {
                                stats.resistance_hit_3d += 1;
                            }
                        }
                        if let Some(k) = day5 {
                            if k.low <= support {
                                stats.support_hit_5d += 1;
                            }
                            if k.high >= resistance {
                                stats.resistance_hit_5d += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// 回测历史快照命中率（默认配置：limit 100，klines 500，1d/3d/5d）。
    pub async fn backtest_key_levels(
        &self,
        lookback_days: u32,
    ) -> Result<KeyLevelBacktestStats, String> {
        self.backtest_key_levels_with_config(lookback_days, &BacktestConfig::default())
            .await
    }
}

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub query_limit: u64,
    pub klines_count: u32,
    pub day1_offset: usize,
    pub day3_offset: usize,
    pub day5_offset: usize,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            query_limit: 100,
            klines_count: 500,
            day1_offset: 0,
            day3_offset: 2,
            day5_offset: 4,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyLevelBacktestStats {
    pub total_snapshots: u32,
    pub support_hit_1d: u32,
    pub resistance_hit_1d: u32,
    pub stop_loss_hit_1d: u32,
    pub take_profit_hit_1d: u32,
    pub support_hit_3d: u32,
    pub resistance_hit_3d: u32,
    pub support_hit_5d: u32,
    pub resistance_hit_5d: u32,
}
