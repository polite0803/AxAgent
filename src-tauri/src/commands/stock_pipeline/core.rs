//! 股票管道核心编排逻辑
//!
//! 编排流程：
//! 1. 发现：调 `recommend_stocks` 获取候选股
//! 2. 分析：对候选股并发调 `run_single_stock_analysis`（自动写 stock_reflections pending row）
//! 3. 持仓再评估：查 `portfolio_holdings`，对持仓股调 `run_single_stock_analysis`
//! 4. 汇总：写入 `stock_pipeline_runs` 表
//!
//! 反思阶段由现有 6h cron 接力（hindsight_date = analysis_date + expected_holding_days）。

#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use axagent_agent_macro::agent_command;
use std::sync::Arc;
use tokio::sync::Semaphore;

use axagent_entities::{portfolio_holdings, reco_picks, stock_analyses, stock_pipeline_runs};
use axagent_harness::workflow_types::{Variable, WorkflowEdge, WorkflowNode};
use axagent_rt_workflow::work_engine::{ProgressCallback, RunOptions, StepProgressEvent};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use crate::commands::stock_workflow::core::run_single_stock_analysis;

/// 从 reco_picks 表读取历史推荐构造种子池
///
/// 数据来源：
/// - 最近 3 天的 `style='serenity'` 且 `synthetic=0`（瓶颈掘金真实推荐）
/// - 最近 2 天的其他风格且 `synthetic=0`（智能荐股真实推荐，非兜底合成）
///
/// 返回 None 表示数据库无历史推荐，调用方应回退到默认 build_seed_pool。
async fn load_preseed_from_db(
    db: &sea_orm::DatabaseConnection,
) -> Option<Vec<(String, String, Option<String>)>> {
    // 取最近 3 天的 serenity 推荐
    let now = chrono::Utc::now();
    let serenity_cutoff =
        (now - chrono::Duration::days(3)).format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
    let other_cutoff =
        (now - chrono::Duration::days(2)).format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

    // serenity 推荐（最近 3 天，非合成）
    let serenity_picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::Style.eq("serenity"))
        .filter(reco_picks::Column::Synthetic.eq(0))
        .filter(reco_picks::Column::GeneratedAt.gt(&serenity_cutoff))
        .order_by_desc(reco_picks::Column::GeneratedAt)
        .all(db)
        .await;

    // 其他风格推荐（最近 2 天，非合成）
    let other_picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::Synthetic.eq(0))
        .filter(reco_picks::Column::Style.ne("serenity"))
        .filter(reco_picks::Column::GeneratedAt.gt(&other_cutoff))
        .order_by_desc(reco_picks::Column::GeneratedAt)
        .all(db)
        .await;

    let serenity_picks = match serenity_picks {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[stock_pipeline] 查询 serenity reco_picks 失败: {e}");
            return None;
        },
    };
    let other_picks = match other_picks {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[stock_pipeline] 查询其他 reco_picks 失败: {e}");
            return None;
        },
    };

    if serenity_picks.is_empty() && other_picks.is_empty() {
        tracing::info!("[stock_pipeline] reco_picks 表无近期历史推荐，回退到默认种子池");
        return None;
    }

    // 合并去重：serenity 优先（排在前），其他风格排后
    let mut seen = std::collections::HashSet::new();
    let mut seed: Vec<(String, String, Option<String>)> = Vec::new();

    for p in serenity_picks.iter().chain(other_picks.iter()) {
        if seen.insert(p.stock_code.clone()) {
            seed.push((p.stock_code.clone(), p.stock_name.clone(), None));
        }
    }

    tracing::info!(
        "[stock_pipeline] 从 reco_picks 构造种子池: serenity={}, other={}, 合计去重={}",
        serenity_picks.len(),
        other_picks.len(),
        seed.len()
    );
    Some(seed)
}

/// 管道执行结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub run_id: String,
    pub run_date: String,
    pub status: String,
    pub candidates: Vec<String>,
    pub new_analyses: Vec<AnalysisSummary>,
    pub reassessed: Vec<AnalysisSummary>,
    pub summary: serde_json::Value,
    pub error: Option<String>,
}

/// 单只股票分析摘要
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub stock_code: String,
    pub stock_name: String,
    pub status: String,
    pub analysis_id: Option<String>,
    pub action: Option<String>,
    pub confidence: Option<f64>,
    pub error: Option<String>,
}

/// 管道配置参数
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 候选股最大数量
    pub max_candidates: usize,
    /// 新候选股分析并发数
    pub new_analysis_concurrency: usize,
    /// 持仓再评估并发数
    pub holdings_reassess_concurrency: usize,
    /// 新候选股分析冷却期（天）— 排除该天数内已分析的股票
    pub new_analysis_cooldown_days: i64,
    /// 持仓再评估冷却期（天）— 排除该天数内已分析的持仓股
    pub holdings_reassess_cooldown_days: i64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_candidates: 5,
            new_analysis_concurrency: 2,
            holdings_reassess_concurrency: 2,
            new_analysis_cooldown_days: 7,
            holdings_reassess_cooldown_days: 3,
        }
    }
}

impl PipelineConfig {
    /// P3-D12: 根据 vendor 健康度动态调节并发数。
    ///
    /// 算法（基于 `astock-data` 的 VendorHealthTracker 状态）：
    /// - healthy_ratio = healthy_count / (healthy + degraded)（Disabled 不计入分母）
    /// - healthy_ratio < 0.3（数据源大面积降级）→ 降到 1（保命模式）
    /// - healthy_ratio < 0.6（部分降级）→ 降到 `max(1, base / 2)`
    /// - 否则 → 保持 `base`
    ///
    /// 这样在批量股票分析时：
    /// - 上游数据源健康 → 保持配置并发（默认 2）
    /// - 部分降级 → 自动降速避免雪崩（如 200 只股票 × 8 并发 × 全部 vendor 重试）
    /// - 大面积降级 → 强制串行（避免拖垮上游）
    ///
    /// 返回 `(actual_new_concurrency, actual_reassess_concurrency, healthy_ratio)`。
    /// 调用方可记录日志反映调节情况。
    pub fn resolve_concurrency_with_vendor_health(
        &self,
        health_snapshot: &[axagent_astock_data::vendor_health::VendorHealth],
    ) -> (usize, usize, f64) {
        use axagent_astock_data::vendor_health::VendorStatus;

        // 过滤 Disabled（手动禁用的 vendor 不参与健康度计算）
        let active: Vec<_> =
            health_snapshot.iter().filter(|h| h.status != VendorStatus::Disabled).collect();
        if active.is_empty() {
            // 无 vendor 健康数据（首次运行 / 未探测）→ 保持配置值，不阻断业务
            return (self.new_analysis_concurrency, self.holdings_reassess_concurrency, 1.0);
        }
        let healthy_count = active.iter().filter(|h| h.status == VendorStatus::Healthy).count();
        let healthy_ratio = healthy_count as f64 / active.len() as f64;

        let (new_c, reassess_c) = if healthy_ratio < 0.3 {
            (1usize, 1usize)
        } else if healthy_ratio < 0.6 {
            (
                (self.new_analysis_concurrency / 2).max(1),
                (self.holdings_reassess_concurrency / 2).max(1),
            )
        } else {
            (self.new_analysis_concurrency, self.holdings_reassess_concurrency)
        };

        (new_c, reassess_c, healthy_ratio)
    }
}

/// 管道执行内部函数（供 Tauri 命令和 cron 调用）
///
/// 使用 WorkEngine 加载工作流模板并执行，与股票分析工作流保持一致。
/// 进度通过可选的回调推送。
pub async fn run_stock_pipeline_inner(
    db: &sea_orm::DatabaseConnection,
    client: &Arc<axagent_astock_data::AStockClient>,
    engine: &Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    config: &PipelineConfig,
    as_of_date: Option<&str>,
    progress_callback: Option<Arc<dyn Fn(&str, &str) + Send + Sync>>,
) -> Result<PipelineResult, String> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let run_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let started_at = chrono::Utc::now().timestamp_millis();

    // 创建 pipeline_run 记录（status=running）
    let _ = stock_pipeline_runs::ActiveModel {
        id: Set(run_id.clone()),
        run_date: Set(run_date.clone()),
        as_of_date: Set(as_of_date.map(String::from)),
        status: Set("running".to_string()),
        candidates_json: Set(None),
        new_analyses_json: Set(None),
        reassessed_json: Set(None),
        summary_json: Set(None),
        error_message: Set(None),
        started_at: Set(started_at),
        completed_at: Set(None),
        created_at: Set(started_at),
    }
    .insert(db)
    .await;

    // 加载工作流模板
    let loaded = load_pipeline_template(db).await?;

    // 构建进度回调
    let progress_cb: ProgressCallback = Arc::new(move |event: StepProgressEvent| {
        if let Some(cb) = progress_callback.as_ref() {
            let step = match event.status.as_str() {
                "running" => format!("{}: 执行中", event.node_id),
                "completed" => format!("{}: 完成", event.node_id),
                s if s == "failed" || s == "timeout" => format!("{}: {}", event.node_id, s),
                _ => event.node_id.clone(),
            };
            cb("pipeline_step", &step);
        }
        Box::pin(async move {})
    });

    // 注入变量
    let variables = build_pipeline_variables(config, as_of_date, &run_date, &run_id);

    // 创建工作流
    let wf_name = format!("stock-pipeline-{run_id}");
    let workflow =
        engine.create_workflow(&wf_name, loaded.nodes, loaded.edges).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();

    // 组装运行选项
    let mut opts = RunOptions::default().with_progress_callback(progress_cb);
    opts.input = Some(json!({
        "max_candidates": config.max_candidates,
        "new_analysis_concurrency": config.new_analysis_concurrency,
        "holdings_reassess_concurrency": config.holdings_reassess_concurrency,
    }));
    opts.variables = Some(variables);
    opts.input_schema = loaded.input_schema;
    opts.output_schema = loaded.output_schema;

    // 执行工作流
    let result = engine.run_workflow(&wf_id, opts).await;

    // 更新 pipeline_run 记录（成功/失败都更新）
    let completed_at = chrono::Utc::now().timestamp_millis();
    let pipeline_result = match result {
        Ok(ref wf) if wf.status == axagent_rt_workflow::workflow_engine::WorkflowStatus::Failed => {
            let _ = stock_pipeline_runs::Entity::update_many()
                .col_expr(
                    stock_pipeline_runs::Column::Status,
                    sea_orm::sea_query::Expr::value("failed"),
                )
                .col_expr(
                    stock_pipeline_runs::Column::ErrorMessage,
                    sea_orm::sea_query::Expr::value("工作流执行失败"),
                )
                .col_expr(
                    stock_pipeline_runs::Column::CompletedAt,
                    sea_orm::sea_query::Expr::value(completed_at),
                )
                .filter(stock_pipeline_runs::Column::Id.eq(run_id.clone()))
                .exec(db)
                .await;
            Err("工作流执行失败".to_string())
        },
        Ok(_wf) => {
            // 从工作流结果构建 PipelineResult
            let pr = build_pipeline_result_from_workflow(&_wf, &run_id, &run_date);
            update_pipeline_run_success(db, &run_id, &pr, completed_at).await;
            Ok(pr)
        },
        Err(e) => {
            let _ = stock_pipeline_runs::Entity::update_many()
                .col_expr(
                    stock_pipeline_runs::Column::Status,
                    sea_orm::sea_query::Expr::value("failed"),
                )
                .col_expr(
                    stock_pipeline_runs::Column::ErrorMessage,
                    sea_orm::sea_query::Expr::value(e.to_string()),
                )
                .col_expr(
                    stock_pipeline_runs::Column::CompletedAt,
                    sea_orm::sea_query::Expr::value(completed_at),
                )
                .filter(stock_pipeline_runs::Column::Id.eq(run_id.clone()))
                .exec(db)
                .await;
            Err(e.to_string())
        },
    };

    let _ = client; // client 保留供未来扩展
    pipeline_result
}

/// 加载股票管道工作流模板
async fn load_pipeline_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<crate::commands::stock_workflow::decision::LoadedTemplate, String> {
    use axagent_entities::workflow_template;

    let template = workflow_template::Entity::find_by_id("stock-pipeline")
        .one(db)
        .await
        .map_err(|e| format!("查询工作流模板失败: {e}"))?
        .ok_or("工作流模板 stock-pipeline 未种子化，请重启应用")?;

    let nodes: Vec<WorkflowNode> =
        serde_json::from_str(&template.nodes).map_err(|e| format!("解析模板节点失败: {e}"))?;
    let edges: Vec<WorkflowEdge> =
        serde_json::from_str(&template.edges).map_err(|e| format!("解析模板边失败: {e}"))?;

    let input_schema = template.input_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let output_schema = template.output_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let variables = template.variables.as_ref().and_then(|v| serde_json::from_str(v).ok());

    Ok(crate::commands::stock_workflow::decision::LoadedTemplate {
        nodes,
        edges,
        input_schema,
        output_schema,
        variables,
    })
}

/// 构建管道工作流变量
fn build_pipeline_variables(
    config: &PipelineConfig,
    as_of_date: Option<&str>,
    run_date: &str,
    run_id: &str,
) -> Vec<Variable> {
    vec![
        Variable {
            name: "max_candidates".into(),
            var_type: "integer".into(),
            value: serde_json::json!(config.max_candidates),
            description: Some("候选股最大数量".into()),
            is_secret: false,
        },
        Variable {
            name: "new_analysis_concurrency".into(),
            var_type: "integer".into(),
            value: serde_json::json!(config.new_analysis_concurrency),
            description: Some("新候选股分析并发数".into()),
            is_secret: false,
        },
        Variable {
            name: "holdings_reassess_concurrency".into(),
            var_type: "integer".into(),
            value: serde_json::json!(config.holdings_reassess_concurrency),
            description: Some("持仓再评估并发数".into()),
            is_secret: false,
        },
        Variable {
            name: "as_of_date".into(),
            var_type: "string".into(),
            value: as_of_date
                .map(|d| serde_json::Value::String(d.to_string()))
                .unwrap_or(serde_json::Value::Null),
            description: Some("指定分析日期".into()),
            is_secret: false,
        },
        Variable {
            name: "run_date".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(run_date.to_string()),
            description: Some("管道执行日期".into()),
            is_secret: false,
        },
        Variable {
            name: "run_id".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(run_id.to_string()),
            description: Some("管道运行ID".into()),
            is_secret: false,
        },
    ]
}

/// 从工作流结果构建 PipelineResult
fn build_pipeline_result_from_workflow(
    wf: &axagent_rt_workflow::workflow_engine::Workflow,
    run_id: &str,
    run_date: &str,
) -> PipelineResult {
    // 从工作流结果中提取数据
    let candidates = extract_var_from_results(&wf.results, "candidates")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default();

    let new_analyses = extract_var_from_results(&wf.results, "new_analyses")
        .and_then(|v| serde_json::from_value::<Vec<AnalysisSummary>>(v).ok())
        .unwrap_or_default();

    let reassessed = extract_var_from_results(&wf.results, "reassessed")
        .and_then(|v| serde_json::from_value::<Vec<AnalysisSummary>>(v).ok())
        .unwrap_or_default();

    let summary = extract_var_from_results(&wf.results, "summary").unwrap_or_else(|| json!({}));

    PipelineResult {
        run_id: run_id.to_string(),
        run_date: run_date.to_string(),
        status: "completed".to_string(),
        candidates,
        new_analyses,
        reassessed,
        summary,
        error: None,
    }
}

/// 从工作流 results 中提取变量值
fn extract_var_from_results(
    results: &std::collections::HashMap<String, serde_json::Value>,
    var_name: &str,
) -> Option<serde_json::Value> {
    // 尝试从 results 中查找变量
    for (key, value) in results {
        if key.contains(var_name) {
            return Some(value.clone());
        }
    }
    None
}

/// 更新 pipeline_run 为成功状态
async fn update_pipeline_run_success(
    db: &sea_orm::DatabaseConnection,
    run_id: &str,
    pr: &PipelineResult,
    completed_at: i64,
) {
    let _ = stock_pipeline_runs::Entity::update_many()
        .col_expr(stock_pipeline_runs::Column::Status, sea_orm::sea_query::Expr::value("completed"))
        .col_expr(
            stock_pipeline_runs::Column::CandidatesJson,
            sea_orm::sea_query::Expr::value(
                serde_json::to_string(&pr.candidates).unwrap_or_default(),
            ),
        )
        .col_expr(
            stock_pipeline_runs::Column::NewAnalysesJson,
            sea_orm::sea_query::Expr::value(
                serde_json::to_string(&pr.new_analyses).unwrap_or_default(),
            ),
        )
        .col_expr(
            stock_pipeline_runs::Column::ReassessedJson,
            sea_orm::sea_query::Expr::value(
                serde_json::to_string(&pr.reassessed).unwrap_or_default(),
            ),
        )
        .col_expr(
            stock_pipeline_runs::Column::SummaryJson,
            sea_orm::sea_query::Expr::value(serde_json::to_string(&pr.summary).unwrap_or_default()),
        )
        .col_expr(
            stock_pipeline_runs::Column::CompletedAt,
            sea_orm::sea_query::Expr::value(completed_at),
        )
        .filter(stock_pipeline_runs::Column::Id.eq(run_id.to_string()))
        .exec(db)
        .await;
}

/// 步骤 1: 股票发现 — 从 reco_picks 历史推荐构造种子池 + 调 `recommend_stocks` + 排除持仓 + 冷却去重
///
/// 种子池来源（优先级）：
/// 1. reco_picks 表中最近 3 天的 serenity（瓶颈掘金）真实推荐
/// 2. reco_picks 表中最近 2 天的其他风格智能荐股真实推荐
/// 3. 若 reco_picks 表无数据（如全新安装），回退到默认 build_seed_pool
///
/// 失败时返回空 vec（不报错），让后续步骤继续执行。
async fn discover_candidates(
    db: &sea_orm::DatabaseConnection,
    client: &Arc<axagent_astock_data::AStockClient>,
    config: &PipelineConfig,
) -> Vec<String> {
    // 从 reco_picks 表读取历史推荐作为种子池
    let preseed = load_preseed_from_db(db).await;

    // 调 recommend_stocks（Mid 周期），传入 preseed
    let template_vars: Vec<(String, serde_json::Value)> = vec![];
    let reco = match axagent_analysis_engine::recommender::recommend_stocks(
        client.clone(),
        axagent_analysis_engine::recommender::Period::Mid,
        &template_vars,
        preseed,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[stock_pipeline] recommend_stocks 失败: {e}");
            return vec![];
        },
    };

    // 收集所有候选股（从所有风格中，去重）
    let mut candidates: Vec<String> = vec![];
    for picks in reco.picks.values() {
        for pick in picks {
            if !candidates.contains(&pick.stock_code) {
                candidates.push(pick.stock_code.clone());
            }
        }
    }

    // 排除已持仓股
    let holding_codes = get_all_holding_codes(db).await;
    candidates.retain(|c| !holding_codes.contains(c));

    // 排除冷却期内的股票（近期已分析的）
    let cooldown_codes = get_recently_analyzed_codes(db, config.new_analysis_cooldown_days).await;
    candidates.retain(|c| !cooldown_codes.contains(c));

    // 截断到 max_candidates
    candidates.truncate(config.max_candidates);
    candidates
}

/// 获取所有持仓股代码（shares > 0）
async fn get_all_holding_codes(db: &sea_orm::DatabaseConnection) -> Vec<String> {
    match portfolio_holdings::Entity::find()
        .filter(portfolio_holdings::Column::Shares.gt(0.0))
        .all(db)
        .await
    {
        Ok(holdings) => holdings.into_iter().map(|h| h.stock_code).collect(),
        Err(e) => {
            tracing::warn!("[stock_pipeline] 查询持仓失败: {e}");
            vec![]
        },
    }
}

/// 获取持仓股代码（带冷却期排除）
async fn get_holding_codes_with_cooldown(
    db: &sea_orm::DatabaseConnection,
    cooldown_days: i64,
) -> Vec<String> {
    let holding_codes = get_all_holding_codes(db).await;
    let cooldown_codes = get_recently_analyzed_codes(db, cooldown_days).await;
    holding_codes.into_iter().filter(|c| !cooldown_codes.contains(c)).collect()
}

/// 获取最近 N 天内已分析的股票代码（去重）
async fn get_recently_analyzed_codes(db: &sea_orm::DatabaseConnection, days: i64) -> Vec<String> {
    let cutoff_ms = chrono::Utc::now().timestamp_millis() - days * 24 * 3600 * 1000;
    match stock_analyses::Entity::find()
        .filter(stock_analyses::Column::UpdatedAt.gt(cutoff_ms))
        .all(db)
        .await
    {
        Ok(analyses) => analyses
            .into_iter()
            .map(|a| a.stock_code)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect(),
        Err(e) => {
            tracing::warn!("[stock_pipeline] 查询近期分析失败: {e}");
            vec![]
        },
    }
}

/// 批量分析股票（Semaphore 控制并发）
///
/// 每只股票 `tokio::spawn` 调 `run_single_stock_analysis`，失败记录 error 不影响其他。
async fn analyze_stocks_batch(
    db: &sea_orm::DatabaseConnection,
    client: &Arc<axagent_astock_data::AStockClient>,
    engine: &Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    stock_codes: &[String],
    max_concurrent: usize,
    as_of_date: Option<&str>,
    emit_step: &(dyn Fn(&str, &str) + Sync),
) -> Vec<AnalysisSummary> {
    if stock_codes.is_empty() {
        return vec![];
    }

    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = vec![];

    for code in stock_codes {
        let code = code.clone();
        let sem = semaphore.clone();
        let db = db.clone();
        let client = client.clone();
        let engine = engine.clone();
        let as_of = as_of_date.map(String::from);

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            tracing::info!("[stock_pipeline] 开始分析 {code}");

            // 获取 stock_name（行情失败时用 code 兜底）
            let stock_name = match client.get_quote(&code).await {
                Ok(q) => q.name,
                Err(_) => code.clone(),
            };

            // as_of_date 当前仅用于日志，run_single_stock_analysis 内部用 live 模式
            // （批量场景不传 as_of_date，保留参数为未来扩展）
            let _ = as_of;

            match run_single_stock_analysis(&db, &client, &engine, &code, &stock_name).await {
                Ok(analysis_id) => {
                    // 读取分析结果获取 action/confidence
                    let (action, confidence) =
                        match stock_analyses::Entity::find_by_id(&analysis_id).one(&db).await {
                            Ok(Some(a)) => (a.decision_action, a.decision_position_pct),
                            _ => (None, None),
                        };
                    AnalysisSummary {
                        stock_code: code,
                        stock_name,
                        status: "completed".to_string(),
                        analysis_id: Some(analysis_id),
                        action,
                        confidence,
                        error: None,
                    }
                },
                Err(e) => {
                    tracing::error!("[stock_pipeline] 分析 {code} 失败: {e}");
                    AnalysisSummary {
                        stock_code: code,
                        stock_name,
                        status: "failed".to_string(),
                        analysis_id: None,
                        action: None,
                        confidence: None,
                        error: Some(e),
                    }
                },
            }
        }));
    }

    let mut results = vec![];
    for handle in handles {
        match handle.await {
            Ok(summary) => {
                emit_step(
                    "analyze_progress",
                    &format!("{}: {}", summary.stock_code, summary.status),
                );
                results.push(summary);
            },
            Err(e) => {
                tracing::error!("[stock_pipeline] task panic: {e}");
            },
        }
    }
    results
}

/// 生成汇总报告
fn build_summary(
    candidates: &[String],
    new_analyses: &[AnalysisSummary],
    reassessed: &[AnalysisSummary],
    run_date: &str,
) -> serde_json::Value {
    let new_success = new_analyses.iter().filter(|a| a.status == "completed").count();
    let new_failed = new_analyses.iter().filter(|a| a.status == "failed").count();
    let reassess_success = reassessed.iter().filter(|a| a.status == "completed").count();
    let reassess_failed = reassessed.iter().filter(|a| a.status == "failed").count();

    let buy_count = new_analyses.iter().filter(|a| a.action.as_deref() == Some("买入")).count();
    let hold_count = new_analyses.iter().filter(|a| a.action.as_deref() == Some("增持")).count();
    let watch_count = new_analyses.iter().filter(|a| a.action.as_deref() == Some("观望")).count();
    let sell_count = reassessed.iter().filter(|a| a.action.as_deref() == Some("卖出")).count();

    json!({
        "pipeline_date": run_date,
        "discovery": {
            "candidates_found": candidates.len()
        },
        "analysis": {
            "new_analyzed": new_success,
            "new_failed": new_failed,
            "reassessed": reassess_success,
            "reassess_failed": reassess_failed
        },
        "decisions": {
            "buy": buy_count,
            "hold": hold_count,
            "watch": watch_count,
            "sell": sell_count
        },
        "reflection_scheduled": new_success + reassess_success,
        "note": "反思由现有6h cron自动接力,28天后执行"
    })
}

// ── Tauri 命令 ──

/// 手动触发股票管道
#[agent_command(domain = invest, safety = Caution, call_mode = StateInput, description = "手动触发股票管道")]
#[tauri::command]
pub async fn run_stock_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<PipelineResult, String> {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();
    let engine = state.work_engine.clone();
    let config = PipelineConfig::default();

    let app_handle = app.clone();
    let progress_callback = Arc::new(move |step: &str, detail: &str| {
        let _ = app_handle.emit(
            "pipeline-step",
            json!({
                "step": step,
                "detail": detail,
                "timestamp": chrono::Utc::now().timestamp_millis()
            }),
        );
    });

    run_stock_pipeline_inner(
        &db,
        &client,
        &engine,
        &config,
        as_of_date.as_deref(),
        Some(progress_callback),
    )
    .await
}

/// 查询管道执行历史
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "查询管道执行历史")]
#[tauri::command]
pub async fn get_pipeline_history(
    state: State<'_, AppState>,
    limit: Option<u64>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.harness.db();
    let limit = limit.unwrap_or(20);

    let runs = stock_pipeline_runs::Entity::find()
        .order_by_desc(stock_pipeline_runs::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询管道运行记录失败: {e}"))
        })?;

    Ok(runs
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "runDate": r.run_date,
                "asOfDate": r.as_of_date,
                "status": r.status,
                "startedAt": r.started_at,
                "completedAt": r.completed_at,
                "errorMessage": r.error_message,
                "summary": r.summary_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
            })
        })
        .collect())
}

/// 查询单次管道执行详情
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "查询单次管道执行详情")]
#[tauri::command]
pub async fn get_pipeline_run_detail(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();

    let run = stock_pipeline_runs::Entity::find_by_id(&run_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询管道运行记录失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("管道运行记录不存在: {run_id}"))
                .to_string()
        })?;

    Ok(json!({
        "id": run.id,
        "runDate": run.run_date,
        "asOfDate": run.as_of_date,
        "status": run.status,
        "startedAt": run.started_at,
        "completedAt": run.completed_at,
        "errorMessage": run.error_message,
        "candidates": run.candidates_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
        "newAnalyses": run.new_analyses_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
        "reassessed": run.reassessed_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
        "summary": run.summary_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_astock_data::vendor_health::{VendorHealth, VendorStatus};

    fn make_health(name: &str, status: VendorStatus) -> VendorHealth {
        let mut h = VendorHealth::new(name);
        h.status = status;
        h
    }

    fn default_config() -> PipelineConfig {
        PipelineConfig {
            max_candidates: 5,
            new_analysis_concurrency: 4,
            holdings_reassess_concurrency: 4,
            new_analysis_cooldown_days: 7,
            holdings_reassess_cooldown_days: 3,
        }
    }

    #[test]
    fn resolve_concurrency_all_healthy_keeps_config() {
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Healthy),
            make_health("sina", VendorStatus::Healthy),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 4);
        assert_eq!(reassess_c, 4);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_partial_degradation_halves_concurrency() {
        // 5 个 active vendor, 2 个 Degraded → ratio = 3/5 = 0.6
        // 0.6 不 < 0.6 → 保持配置值
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Healthy),
            make_health("sina", VendorStatus::Healthy),
            make_health("ths", VendorStatus::Degraded),
            make_health("cninfo", VendorStatus::Degraded),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 4, "ratio=0.6 应保持配置值");
        assert_eq!(reassess_c, 4);
        assert!((ratio - 0.6).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_below_60_pct_halves_concurrency() {
        // 4 个 active vendor, 2 个 Degraded → ratio = 2/4 = 0.5 < 0.6
        // → max(1, 4/2) = 2
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Healthy),
            make_health("sina", VendorStatus::Degraded),
            make_health("ths", VendorStatus::Degraded),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 2, "ratio=0.5 < 0.6 应降到 base/2");
        assert_eq!(reassess_c, 2);
        assert!((ratio - 0.5).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_severe_degradation_forces_serial() {
        // 5 个 active, 4 个 Degraded → ratio = 1/5 = 0.2 < 0.3 → 降到 1
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Degraded),
            make_health("sina", VendorStatus::Degraded),
            make_health("ths", VendorStatus::Degraded),
            make_health("cninfo", VendorStatus::Degraded),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 1, "ratio<0.3 应强制串行");
        assert_eq!(reassess_c, 1);
        assert!(ratio < 0.3);
    }

    #[test]
    fn resolve_concurrency_excludes_disabled_vendors() {
        // 5 个 vendor: 2 Healthy + 1 Degraded + 2 Disabled
        // active = 3 (Healthy+Degraded), ratio = 2/3 ≈ 0.667 → 保持配置
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Healthy),
            make_health("sina", VendorStatus::Degraded),
            make_health("ths", VendorStatus::Disabled),
            make_health("cninfo", VendorStatus::Disabled),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 4, "Disabled 不计入分母, ratio=2/3>0.6 应保持");
        assert_eq!(reassess_c, 4);
        assert!((ratio - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_empty_health_keeps_config() {
        // 无 vendor 健康数据（首次运行）→ 保持配置值
        let cfg = default_config();
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&[]);
        assert_eq!(new_c, 4);
        assert_eq!(reassess_c, 4);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_all_disabled_keeps_config() {
        // 全部 Disabled（极端场景）→ active 为空 → 保持配置值
        let cfg = default_config();
        let health = vec![
            make_health("tencent", VendorStatus::Disabled),
            make_health("eastmoney", VendorStatus::Disabled),
        ];
        let (new_c, reassess_c, ratio) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 4);
        assert_eq!(reassess_c, 4);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn resolve_concurrency_min_one_floor() {
        // base=1, 部分降级 → max(1, 1/2) = 1（不应降到 0）
        let cfg = PipelineConfig {
            max_candidates: 5,
            new_analysis_concurrency: 1,
            holdings_reassess_concurrency: 1,
            new_analysis_cooldown_days: 7,
            holdings_reassess_cooldown_days: 3,
        };
        let health = vec![
            make_health("tencent", VendorStatus::Healthy),
            make_health("eastmoney", VendorStatus::Degraded),
            make_health("sina", VendorStatus::Degraded),
            make_health("ths", VendorStatus::Degraded),
        ];
        let (new_c, reassess_c, _) = cfg.resolve_concurrency_with_vendor_health(&health);
        assert_eq!(new_c, 1, "base=1, ratio=0.25 < 0.3 应强制 1");
        assert_eq!(reassess_c, 1);
    }
}
