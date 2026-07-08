//! 股票分析专家/角色/Profile 自动种子化到 agency_experts/agent_roles/agent_profiles 表。
//! 使用 include_str! 编译期嵌入 .md 内容，打包后无需文件 I/O。

use super::{PROFILE_TOOLS, build_analyst_input_mapping, merge_variable_values};
use crate::commands::error_code::stock_setup;

pub(crate) async fn seed_stock_analysis_workflow_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    use axagent_core::entity::workflow_template;
    use axagent_harness::hallucination_guard::HallucinationGuardConfig;
    use axagent_harness::workflow_types::{
        AgentNode, AgentNodeConfig, AggregatorNode, AggregatorNodeConfig, BackoffType, Branch,
        CodeNode, CodeNodeConfig, DebateNode, DebateNodeConfig, EdgeType, EndNode, EndNodeConfig,
        ErrorConfig, JsonSchema, JsonSchemaProperty, LlmClassifierNode, LlmClassifierNodeConfig,
        MergeStrategy, NotificationNode, NotificationNodeConfig, OnFailureAction, OutputMode,
        ParallelNode, ParallelNodeConfig, Position, RetryConfig, RetryPolicy, StorageNode,
        StorageNodeConfig, SubGraph, SwitchCase, SwitchNode, SwitchNodeConfig, ToolDef, ToolNode,
        ToolNodeConfig, TriggerConfig, TriggerNode, TriggerType, ValidationAssertion,
        ValidationNode, ValidationNodeConfig, Variable, WorkflowEdge, WorkflowNode,
        WorkflowNodeBase,
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    const TEMPLATE_ID: &str = "stock-analysis";

    // v42: V42 风险重构 — posterior 不再截断，用 risk_bias 左移 action 阈值 + 仓位上限
    const TEMPLATE_VERSION: i32 = 1;

    // 升级前保留旧模板的变量自定义值，在函数体外声明以延长生命周期
    let mut old_variables: Option<String> = None;

    if let Some(existing) = workflow_template::Entity::find_by_id(TEMPLATE_ID)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL)
                .with_detail(format!("查询工作流模板失败: {e}"))
        })?
    {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[stock_analysis_setup] 模板已是最新版本 v{}，跳过种子化以保留用户修改",
                existing.version
            );
            return Ok(());
        }
        tracing::info!(
            "[stock_analysis_setup] 更新股票分析工作流模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );
        // 写版本快照（复用 update_workflow_template 的 snapshot 机制）
        let ver_id = format!("{}_v{}", TEMPLATE_ID, existing.version);
        if axagent_core::entity::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
            .is_none()
        {
            use sea_orm::ActiveModelTrait;
            let snapshot = axagent_core::entity::workflow_template_version::ActiveModel {
                id: Set(ver_id.clone()),
                template_id: Set(TEMPLATE_ID.to_string()),
                name: Set(existing.name.clone()),
                description: Set(existing.description.clone()),
                icon: Set(existing.icon.clone()),
                tags: Set(existing.tags.clone()),
                version: Set(existing.version),
                is_preset: Set(existing.is_preset),
                is_editable: Set(existing.is_editable),
                is_public: Set(existing.is_public),
                trigger_config: Set(existing.trigger_config.clone()),
                nodes: Set(existing.nodes.clone()),
                edges: Set(existing.edges.clone()),
                input_schema: Set(existing.input_schema.clone()),
                output_schema: Set(existing.output_schema.clone()),
                variables: Set(existing.variables.clone()),
                error_config: Set(existing.error_config.clone()),
                created_at: Set(chrono::Utc::now().timestamp_millis()),
            };
            snapshot.insert(db).await.map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("写入版本快照失败: {e}"))
            })?;
            tracing::info!("[stock_analysis_setup] 旧版本快照已保存: {ver_id}");
        }
        old_variables = existing.variables.clone();
        // 用 UPDATE 替代 DELETE，保留用户自定义变量
    }

    let now = chrono::Utc::now().timestamp_millis();

    let tool_node = |id: &str,
                     title: &str,
                     tool_name: &str,
                     output_var: &str,
                     arg_key: &str,
                     parent_id: Option<&str>,
                     x: f64,
                     y: f64|
     -> WorkflowNode {
        let mut input_mapping = std::collections::HashMap::new();
        input_mapping.insert(arg_key.to_string(), "stock_code".to_string());
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("获取数据: {tool_name}")),
                position: Position { x, y },
                retry: RetryConfig {
                    enabled: true,
                    max_retries: 2,
                    ..Default::default()
                },
                timeout: Some(120),
                enabled: true,
                parent_id: parent_id.map(String::from),
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: tool_name.into(),
                input_mapping,
                output_var: output_var.into(),
            },
        })
    };

    // ── ToolDef 参数 schema 辅助构建 ──
    fn sc_prop(desc: &str) -> JsonSchemaProperty {
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some(desc.into()),
            default: None,
            enum_values: None,
            format: None,
        }
    }
    fn sc_prop_default(desc: &str, default: &str) -> JsonSchemaProperty {
        JsonSchemaProperty {
            schema_type: "string".into(),
            description: Some(desc.into()),
            default: Some(serde_json::Value::String(default.into())),
            enum_values: None,
            format: None,
        }
    }
    fn int_prop(desc: &str, default: Option<i64>) -> JsonSchemaProperty {
        JsonSchemaProperty {
            schema_type: "integer".into(),
            description: Some(desc.into()),
            default: default.map(|v| serde_json::json!(v)),
            enum_values: None,
            format: None,
        }
    }
    fn stock_code_params() -> Option<JsonSchema> {
        let mut props = std::collections::HashMap::new();
        props.insert("stock_code".into(), sc_prop("6位股票代码，如 600519"));
        Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(props),
            required: Some(vec!["stock_code".into()]),
            items: None,
        })
    }
    fn no_params() -> Option<JsonSchema> {
        Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::new()),
            required: None,
            items: None,
        })
    }
    fn data_params() -> Option<JsonSchema> {
        let mut props = std::collections::HashMap::new();
        props.insert(
            "data".into(),
            JsonSchemaProperty {
                schema_type: "string".into(),
                description: Some("JSON 格式的数值数组或数据序列".into()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(props),
            required: None,
            items: None,
        })
    }

    // 常用工具定义
    let td_quote = ToolDef {
        name: "get_stock_quote".into(),
        description: Some("获取股票实时行情：现价、涨跌幅、PE、PB、市值".into()),
        parameters: stock_code_params(),
    };
    let mut kline_props = std::collections::HashMap::new();
    kline_props.insert("stock_code".into(), sc_prop("6位股票代码"));
    kline_props.insert("period".into(), sc_prop_default("周期: daily/weekly/monthly", "daily"));
    kline_props.insert("limit".into(), int_prop("K线数量", Some(120)));
    let td_kline = ToolDef {
        name: "get_stock_kline".into(),
        description: Some("获取K线数据：OHLCV，可指定周期和数量".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(kline_props),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_fin = ToolDef {
        name: "get_stock_financials".into(),
        description: Some("获取财务数据：营收、净利润、EPS、ROE、毛利率等".into()),
        parameters: stock_code_params(),
    };
    // Phase 2: 预聚合的基本面分析报告（markdown 格式）。
    // 由 a-fundamentals 节点通过 t-fundamentals-data 预拉,作为冷启动 context 输入,
    // 避免 LLM 在大量原始财报上重复计算同比/环比/健康度等基础比率。
    let td_fundamentals_report = ToolDef {
        name: "get_fundamentals_report_markdown".into(),
        description: Some(
            "获取基本面分析报告(预聚合 markdown):含 PE/PB/ROE/同比环比/估值带/0-100 健康度评分 \
             与质量等级。返回字符串,直接消费"
                .into(),
        ),
        parameters: stock_code_params(),
    };
    // Phase 3: 市场 Regime 识别(借鉴 TradingAgents-CN 自适应 prompt)
    let td_regime = ToolDef {
        name: "get_market_regime".into(),
        description: Some(
            "市场 Regime 识别:综合 20/60 日均线/布林带/波动率/连涨连跌, \
             判定 Bull/Bear/Sideways/Volatile 并返回 prompt_bias"
                .into(),
        ),
        parameters: stock_code_params(),
    };
    let mut news_props = std::collections::HashMap::new();
    news_props.insert("stock_code".into(), sc_prop("6位股票代码"));
    news_props.insert("limit".into(), int_prop("新闻数量", Some(30)));
    let td_news = ToolDef {
        name: "get_stock_news".into(),
        description: Some("获取近期新闻公告".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(news_props),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_mf = ToolDef {
        name: "get_stock_money_flow".into(),
        description: Some("获取资金流向：主力/超大单/大单/中单/小单净流入".into()),
        parameters: stock_code_params(),
    };
    let td_score = ToolDef {
        name: "compute_scoring".into(),
        description: Some("计算技术评分：基于趋势、偏离度、MACD、成交量、RSI、支撑阻力".into()),
        parameters: stock_code_params(),
    };
    let td_val = ToolDef {
        name: "compute_valuation".into(),
        description: Some("计算估值指标：DCF、F-Score、护城河量化、安全边际".into()),
        parameters: stock_code_params(),
    };
    let mut risk_props = std::collections::HashMap::new();
    risk_props.insert("stock_codes".into(), sc_prop("逗号分隔的股票代码列表"));
    risk_props.insert("weights".into(), sc_prop("逗号分隔的持仓权重(0-1)，不填则等权"));
    let td_risk = ToolDef {
        name: "compute_portfolio_risk".into(),
        description: Some("计算组合风险：总市值、集中度、风险等级".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(risk_props),
            required: Some(vec!["stock_codes".into()]),
            items: None,
        }),
    };
    // ── 新增 12 个金融模型 ToolDef ──
    let td_maxdd = ToolDef {
        name: "calc_max_drawdown".into(),
        description: Some("计算最大回撤比例".into()),
        parameters: data_params(),
    };
    let td_sharpe = ToolDef {
        name: "calc_sharpe_ratio".into(),
        description: Some("计算夏普比率".into()),
        parameters: data_params(),
    };
    let td_var = ToolDef {
        name: "calc_var".into(),
        description: Some("历史模拟法 VaR 计算".into()),
        parameters: data_params(),
    };
    let td_pe_pct = ToolDef {
        name: "calc_pe_percentile".into(),
        description: Some("PE 历史分位数".into()),
        parameters: data_params(),
    };
    let td_peg = ToolDef {
        name: "calc_peg".into(),
        description: Some("PEG 估值指标".into()),
        parameters: data_params(),
    };
    let td_ma_cross = ToolDef {
        name: "detect_ma_cross".into(),
        description: Some("MA 金叉死叉检测".into()),
        parameters: data_params(),
    };
    let td_breakout = ToolDef {
        name: "detect_breakout".into(),
        description: Some("支撑阻力突破检测".into()),
        parameters: data_params(),
    };
    let td_kelly = ToolDef {
        name: "calc_kelly".into(),
        description: Some("凯利公式仓位计算".into()),
        parameters: data_params(),
    };
    let td_rp = ToolDef {
        name: "calc_risk_parity".into(),
        description: Some("风险平价权重计算".into()),
        parameters: data_params(),
    };
    // ── 新增 9 个数据 API ToolDef ──
    let td_research = ToolDef {
        name: "get_stock_research_reports".into(),
        description: Some("获取券商研报".into()),
        parameters: stock_code_params(),
    };
    let td_consensus = ToolDef {
        name: "get_stock_consensus_eps".into(),
        description: Some("获取一致性预期EPS".into()),
        parameters: stock_code_params(),
    };
    let td_concepts = ToolDef {
        name: "get_stock_concept_blocks".into(),
        description: Some("获取概念板块归属".into()),
        parameters: stock_code_params(),
    };
    let td_announce = ToolDef {
        name: "get_stock_announcements".into(),
        description: Some("获取公司公告".into()),
        parameters: stock_code_params(),
    };
    let td_north = ToolDef {
        name: "get_north_bound_flow".into(),
        description: Some("获取北向资金流向".into()),
        parameters: no_params(),
    };
    let td_dragon = ToolDef {
        name: "get_market_dragon_tiger".into(),
        description: Some("获取龙虎榜数据".into()),
        parameters: no_params(),
    };
    let td_hot = ToolDef {
        name: "get_hot_stocks".into(),
        description: Some("获取市场热门股".into()),
        parameters: no_params(),
    };
    let td_industry = ToolDef {
        name: "get_industry_ranking".into(),
        description: Some("获取行业涨跌排名".into()),
        parameters: no_params(),
    };
    let td_cls = ToolDef {
        name: "get_cls_flash".into(),
        description: Some("获取财联社实时快讯".into()),
        parameters: no_params(),
    };
    // ── P1: 4 个技术指标 ToolDef ──
    let mut atr_props = std::collections::HashMap::new();
    atr_props.insert("klines_json".into(), sc_prop("K线JSON(含high/low/close)"));
    atr_props.insert("period".into(), int_prop("ATR周期", Some(14)));
    let td_atr = ToolDef {
        name: "compute_atr".into(),
        description: Some("计算 ATR 平均真实波幅".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(atr_props),
            required: None,
            items: None,
        }),
    };
    let mut kdj_props = std::collections::HashMap::new();
    kdj_props.insert("klines_json".into(), sc_prop("K线JSON(含high/low/close)"));
    kdj_props.insert("n".into(), int_prop("KDJ周期N", Some(9)));
    let td_kdj = ToolDef {
        name: "compute_kdj".into(),
        description: Some("计算 KDJ 随机指标".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(kdj_props),
            required: None,
            items: None,
        }),
    };
    let td_obv = ToolDef {
        name: "compute_obv".into(),
        description: Some("计算 OBV 能量潮".into()),
        parameters: {
            let mut p = std::collections::HashMap::new();
            p.insert("klines_json".into(), sc_prop("K线JSON(含close/volume)"));
            Some(JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some(p),
                required: None,
                items: None,
            })
        },
    };
    let mut beta_props = std::collections::HashMap::new();
    beta_props.insert("stock_returns_json".into(), sc_prop("个股收益率JSON数组"));
    beta_props.insert("market_returns_json".into(), sc_prop("大盘收益率JSON数组"));
    let _td_beta = ToolDef {
        name: "calc_beta".into(),
        description: Some("计算 Beta 系数".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(beta_props),
            required: None,
            items: None,
        }),
    };
    // ── P2: 事件检测 + 组合分析 ToolDef ──
    let mut earn_props = std::collections::HashMap::new();
    earn_props.insert("actual_eps".into(), sc_prop("实际EPS"));
    earn_props.insert("consensus_eps".into(), sc_prop("一致预期EPS"));
    let td_earnings = ToolDef {
        name: "detect_earnings_surprise".into(),
        description: Some("检测业绩超预期/低于预期".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(earn_props),
            required: None,
            items: None,
        }),
    };
    let mut pledge_props = std::collections::HashMap::new();
    pledge_props.insert("pledge_pct".into(), sc_prop("质押比例(%)"));
    pledge_props.insert("warning_line".into(), sc_prop("预警线(默认50)"));
    pledge_props.insert("liquidation_line".into(), sc_prop("平仓线(默认70)"));
    let td_pledge = ToolDef {
        name: "detect_pledge_risk".into(),
        description: Some("检测大股东质押风险".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(pledge_props),
            required: None,
            items: None,
        }),
    };
    let td_corr = ToolDef {
        name: "calc_correlation_matrix".into(),
        description: Some("计算收益率相关系数矩阵".into()),
        parameters: {
            let mut p = std::collections::HashMap::new();
            p.insert("returns_matrix_json".into(), sc_prop("收益率矩阵JSON(二维数组)"));
            Some(JsonSchema {
                schema_type: "object".into(),
                description: None,
                properties: Some(p),
                required: None,
                items: None,
            })
        },
    };
    // ── P3: 独立新能力 ToolDef ──
    let mut mc_props = std::collections::HashMap::new();
    mc_props.insert("current_price".into(), sc_prop("当前价格"));
    mc_props.insert("annual_return".into(), sc_prop("年化收益率(默认0.08)"));
    mc_props.insert("annual_volatility".into(), sc_prop("年化波动率(默认0.3)"));
    mc_props.insert("days".into(), int_prop("模拟天数", Some(30)));
    mc_props.insert("simulations".into(), int_prop("模拟次数", Some(1000)));
    let td_mc = ToolDef {
        name: "run_monte_carlo".into(),
        description: Some("蒙特卡洛模拟价格路径".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(mc_props),
            required: None,
            items: None,
        }),
    };
    let mut ind_props = std::collections::HashMap::new();
    ind_props.insert("stock_pe".into(), sc_prop("个股PE"));
    ind_props.insert("stock_growth".into(), sc_prop("个股增长率"));
    ind_props.insert("industry_avg_pe".into(), sc_prop("行业平均PE"));
    ind_props.insert("industry_avg_growth".into(), sc_prop("行业平均增长率"));
    let td_ind = ToolDef {
        name: "analyze_industry_position".into(),
        description: Some("行业内估值/增长对比分析".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(ind_props),
            required: None,
            items: None,
        }),
    };
    let mut lup_props = std::collections::HashMap::new();
    lup_props.insert("klines_json".into(), sc_prop("K线JSON(含close/high/volume)"));
    lup_props.insert("market_type".into(), sc_prop("板块: main/star/chinext/bj"));
    let td_lup = ToolDef {
        name: "detect_limit_up_potential".into(),
        description: Some("涨停潜力评估".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(lup_props),
            required: None,
            items: None,
        }),
    };
    let td_block = ToolDef {
        name: "get_stock_block_trades".into(),
        description: Some("获取大宗交易记录：成交价、成交量、买卖方营业部、折价率".into()),
        parameters: stock_code_params(),
    };
    let td_visit = ToolDef {
        name: "get_stock_institutional_visits".into(),
        description: Some("获取机构调研记录：调研日期、机构数量、调研内容".into()),
        parameters: stock_code_params(),
    };
    let td_idx = ToolDef {
        name: "get_index_quotes".into(),
        description: Some("获取大盘指数行情（上证指数、深证成指、创业板指）".into()),
        parameters: no_params(),
    };
    let td_peers = ToolDef {
        name: "get_stock_peers".into(),
        description: Some("获取同行业可比公司估值（PE/PB/ROE/涨跌幅/市值）".into()),
        parameters: stock_code_params(),
    };
    let td_pcr = ToolDef {
        name: "get_stock_option_pcr".into(),
        description: Some("获取期权PCR（看跌/看涨比率和持仓量比率，市场情绪前瞻指标）".into()),
        parameters: stock_code_params(),
    };
    let td_lockup = ToolDef {
        name: "get_stock_lockup".into(),
        description: Some("获取限售解禁日程（解禁日期、股数、比例、股东名称）".into()),
        parameters: stock_code_params(),
    };
    let td_lockup_bundle = ToolDef {
        name: "get_stock_lockup_bundle".into(),
        description: Some("获取筹码面分析数据（解禁+增减持+大宗交易三方聚合）".into()),
        parameters: stock_code_params(),
    };
    let td_sh_trades = ToolDef {
        name: "get_stock_shareholder_trades".into(),
        description: Some("获取大股东增减持记录（变动类型、数量、均价、原因）".into()),
        parameters: stock_code_params(),
    };
    let td_dividend = ToolDef {
        name: "get_stock_dividend_records".into(),
        description: Some("获取除权除息/分红送配记录".into()),
        parameters: stock_code_params(),
    };
    let td_nb_holding = ToolDef {
        name: "get_stock_north_bound".into(),
        description: Some("获取北向资金个股持仓（持股数量、占比）".into()),
        parameters: stock_code_params(),
    };
    let td_dt = ToolDef {
        name: "get_stock_dragon_tiger".into(),
        description: Some("获取个股龙虎榜数据（营业部买卖、上榜原因）".into()),
        parameters: stock_code_params(),
    };
    let td_margin = ToolDef {
        name: "get_stock_margin_data".into(),
        description: Some("获取融资融券数据（融资买入额、余额、融券卖出量、余量）".into()),
        parameters: stock_code_params(),
    };
    let td_announce_content = ToolDef {
        name: "get_announcement_content".into(),
        description: Some("获取公司公告PDF全文内容（下载并解析公告PDF正文）".into()),
        parameters: stock_code_params(),
    };
    let td_sector_info = ToolDef {
        name: "get_stock_sector_info".into(),
        description: Some("获取行业分类（申万一级/二级、概念板块标签）".into()),
        parameters: stock_code_params(),
    };

    // 工具名 → ToolDef 映射（用于按名查找，给节点填充 config.tools）
    let tool_def_map: std::collections::HashMap<&str, ToolDef> = [
        ("get_stock_quote", td_quote.clone()),
        ("get_stock_kline", td_kline.clone()),
        ("get_stock_financials", td_fin.clone()),
        // Phase 2: 基本面报告(markdown)由 t-fundamentals-data 节点调用
        ("get_fundamentals_report_markdown", td_fundamentals_report.clone()),
        // Phase 3: Regime 识别,由 t-regime-detect 节点调用
        ("get_market_regime", td_regime.clone()),
        ("get_stock_news", td_news.clone()),
        ("get_stock_money_flow", td_mf.clone()),
        ("compute_scoring", td_score.clone()),
        ("compute_valuation", td_val.clone()),
        ("compute_portfolio_risk", td_risk.clone()),
        (
            "search_stock",
            ToolDef {
                name: "search_stock".into(),
                description: Some("按代码或名称模糊搜索A股".into()),
                parameters: None,
            },
        ),
        ("get_hot_stocks", td_hot.clone()),
        ("get_industry_ranking", td_industry.clone()),
        ("get_stock_announcements", td_announce.clone()),
        ("get_stock_consensus_eps", td_consensus.clone()),
        ("compute_kdj", td_kdj.clone()),
        ("compute_obv", td_obv.clone()),
        ("get_cls_flash", td_cls.clone()),
        ("get_north_bound_flow", td_north.clone()),
        ("get_market_dragon_tiger", td_dragon.clone()),
        ("get_stock_research_reports", td_research.clone()),
        ("get_stock_concept_blocks", td_concepts.clone()),
        ("get_stock_block_trades", td_block.clone()),
        ("get_stock_institutional_visits", td_visit.clone()),
        ("get_index_quotes", td_idx.clone()),
        ("get_stock_peers", td_peers.clone()),
        ("get_stock_option_pcr", td_pcr.clone()),
        ("get_stock_lockup", td_lockup.clone()),
        ("get_stock_lockup_bundle", td_lockup_bundle.clone()),
        ("get_stock_shareholder_trades", td_sh_trades.clone()),
        ("get_stock_dividend_records", td_dividend.clone()),
        ("get_stock_north_bound", td_nb_holding.clone()),
        ("get_stock_dragon_tiger", td_dt.clone()),
        ("get_stock_margin_data", td_margin.clone()),
        ("get_announcement_content", td_announce_content.clone()),
        ("get_stock_sector_info", td_sector_info.clone()),
    ]
    .into_iter()
    .collect();

    // 从 ToolDef 列表生成 "可用工具" prompt 片段
    fn tool_prompt(tools: &[ToolDef]) -> String {
        if tools.is_empty() {
            return String::new();
        }
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        format!(
            "\n\n你可以调用以下工具获取最新数据或计算指标：{}。请先调用相关工具获取数据，再基于返回结果进行分析。",
            names.join("、")
        )
    }

    let agent = |id: &str,
                 title: &str,
                 expert_id: &str,
                 parent_id: Option<&str>,
                 x: f64,
                 y: f64|
     -> WorkflowNode {
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("股票分析: {expert_id}")),
                position: Position { x, y },
                retry: RetryConfig {
                    enabled: true,
                    max_retries: 3, // v24: 从 2 提升到 3，GLM-5.1 429 限流可持续 30s+
                    base_delay_ms: 3000, // v24: 从 1000 提升到 3000，避免短退避对限流无效
                    max_delay_ms: 60000, // v24: 从 30000 提升到 60000
                    backoff_type: BackoffType::Exponential,
                },
                timeout: Some(600), // V40: 与 step_timeout=600s 对齐，为多轮工具调用保留余量
                enabled: true,
                parent_id: parent_id.map(String::from),
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                // inline system_prompt 只放任务指令，专家 prompt 由 agent_profile 自动加载，
                // 行情数据通过 context_sources 由上游 Tool 节点输出自动注入
                // P0 回退(v16):inline prefix 回退到 v14 之前的形式 —— 不在
                //   inline prefix 中用 {{stock_code}}/{{stock_name}} Slot。
                //   原因:v14/v15 改动在 inline prefix 引入 Slot 后,某些
                //   context.variables 注入路径下 render_prompt 失败,导致所有
                //   Agent 节点返回 "暂无数据"。stock_code/stock_name 改为通过
                //   expert .md prompt 头部 "{{stock_code}} / {{stock_name}}"
                //   primacy 锚点注入,避开 inline prefix 的风险。
                system_prompt: format!(
                    "你的任务: {title}\n\n重要原则：\n1. 如果上游数据节点返回为空，请主动调用可用工具获取补充数据。\n2. 如果经过补充获取仍然无法获得某些数据，请在分析报告中诚实标记该维度数据获取失败的状态，并评估该缺失对分析结论的影响程度。\n3. 始终针对目标股票给出明确的观点（看多/看空/中性）和论据。\n4. 工具返回空数组或空对象有两种可能：①该数据源暂无法获取（技术问题）；②该股票在该维度无数据（如无机构覆盖）。请在报告中明确区分两种情况并评估对分析的影响。\n5. 如果你是研报分析师，目标是从券商研报、一致预期EPS、机构调研等维度给出观点。如果这些数据源返回空，说明该股票暂无机构覆盖，请标注'无机构覆盖'并基于公司基本面、行业地位、新闻公告等公开信息给出独立分析。",
                ),
                context_sources: vec![],
                // 通过 input_mapping 自动注入股票代码/名称到 system_prompt
                input_mapping: [
                    ("stock_code".to_string(), "stock_code".to_string()),
                    ("stock_name".to_string(), "stock_name".to_string()),
                ]
                .into_iter()
                .collect(),
                output_var: id.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(32768),
                tools: vec![],
                exposed_tools: vec![],
                output_mode: OutputMode::Text,
                agent_profile_id: Some(format!("stock-{expert_id}")),
                max_tool_rounds: None,
                execution_mode: None,
                rag_source_ids: vec![],
                model_role: None,
                consistency_check: None,
                // V55 启用: hallucination_guard 锚定检查。
                // 阈值 0.4（略低于默认 0.5）—— 平衡：长自然语言报告不应被严苛拦截，
                // 但当 LLM 编造关键数字/术语时（unverified_claims 占比 > 60%）会触发
                // anchor_result.passed=false → agent_executor 注入 __untrusted 标记 →
                // portfolio-mgr.rhai 触发 weights_collapsed 兜底（观望+空仓）。
                hallucination_guard: Some(HallucinationGuardConfig {
                    enabled: true,
                    match_threshold: 0.4,
                }),
                stream_chunk_timeout_secs: Some(300),
            },
        })
    };

    let edge = |id: &str, source: &str, target: &str| -> WorkflowEdge {
        WorkflowEdge {
            id: id.into(),
            source: source.into(),
            source_handle: None,
            target: target.into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        }
    };

    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    // Trigger
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".into(),
            title: "开始分析".into(),
            description: Some("输入股票代码启动分析".into()),
            // F-1 修复: 3×3 网格最右列 x=1240+200=1440, 居中 trigger x=520
            position: Position { x: 520.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({"stock_code": "{{stock_code}}"}),
        },
    }));

    // 9 个分析师 + catalyst-analyst
    let analysts = [
        (
            "a-market-analyst",
            "技术面分析：K线形态、MACD/RSI、支撑阻力位",
            "market-analyst",
        ),
        ("a-sentiment", "市场情绪分析：资金流向、散户/机构态度", "sentiment-analyst"),
        ("a-news", "新闻公告影响评估", "news-analyst"),
        ("a-fundamentals", "基本面估值分析：PE/PB/ROE等", "fundamentals-analyst"),
        ("a-policy", "宏观政策与行业政策影响分析", "policy-analyst"),
        ("a-hot-money", "游资动向与主力资金追踪", "hot-money-tracker"),
        ("a-lockup", "解禁减持与质押风险排查", "lockup-watcher"),
        ("a-research", "券商研报观点汇总", "research-analyst"),
        ("a-sector", "行业景气度与轮动分析", "sector-analyst"),
        ("a-catalyst", "催化剂与叙事完整度评估", "catalyst-analyst"),
    ];
    let a_ids: Vec<&str> = analysts.iter().map(|(id, _, _)| *id).collect();

    // 为每个分析师插入对应的数据获取 Tool 节点
    // 注：节点工具决定了下游 analyst 拿到的"前置数据"。LLM agent 自身仍可调用
    // PROFILE_TOOLS 中的工具，但首屏/冷启动数据由这些 tool 节点预拉。
    //
    // F-8 修复: 顺序必须与上面的 `analysts` 数组完全一致：
    //   [0] a-market-analyst   ↔ t-market-data
    //   [1] a-sentiment        ↔ t-sentiment-data
    //   [2] a-news             ↔ t-news-data
    //   [3] a-fundamentals     ↔ t-fundamentals-data
    //   [4] a-policy           ↔ t-policy-data
    //   [5] a-hot-money        ↔ t-hotmoney-data   (原: t-research-data 错位)
    //   [6] a-lockup           ↔ t-lockup-data     (原: t-hotmoney-data 错位)
    //   [7] a-research         ↔ t-research-data   (原: t-lockup-data 错位)
    //   [8] a-sector           ↔ t-sector-data
    // 错位会导致 hot-money analyst 拿到研报数据、research analyst 拿到解禁数据，
    // 9 个分析师产出的报告与各自的角色语义不符。
    let tool_assignments: &[(&str, &str, &str, &str)] = &[
        ("t-market-data", "获取K线+行情", "get_stock_kline", "stock_code"),
        // 修复: t-sentiment-data 原调用 get_hot_stocks（热门股票列表，非个股新闻），
        // 导致情绪面分析师拿不到个股新闻舆情数据。改为 get_stock_news。
        ("t-sentiment-data", "获取新闻+热门", "get_stock_news", "stock_code"),
        // 修复: t-news-data 原调用 get_announcements（公告），导致消息面分析师
        // 拿不到个股新闻数据。改为 get_stock_news 与 a-news 的 data_sources 匹配。
        // 公告数据已由 t-catalyst-data 负责获取。
        ("t-news-data", "获取新闻+公告", "get_stock_news", "stock_code"),
        // 修复 P1: 基本面分析师前置数据改用 get_stock_financials（财报）而非
        // get_consensus_eps（一致预期），让 a-fundamentals 启动时就能拿到
        // 营收/利润/资产负债等核心财务数据。
        //
        // Phase 2: 升级为 get_fundamentals_report_markdown —— 工作流引擎在 a-fundamentals
        // 启动前预拉"预聚合 markdown 报告"(健康度评分/估值带/同比环比/质量等级)。
        // LLM 启动时直接消费 markdown,引用 system_pre_computed 字段
        // (health_score / valuation_state / safety_margin_pct / yoy_*),
        // 避免在大量原始财报上重复计算基础比率。
        // 注意: PROFILE_TOOLS 中仍保留 get_stock_financials,LLM 需要做更细颗粒分析时可主动调用。
        (
            "t-fundamentals-data",
            "获取基本面报告(markdown)",
            "get_fundamentals_report_markdown",
            "stock_code",
        ),
        // 修复 P1: 政策分析师前置数据改用 get_stock_news（新闻）而非
        // get_announcements（公告）。新闻覆盖宏观/产业政策动态更广，
        // 与 a-news 的公告视角形成互补。
        //
        // F-4 待办: 当前 get_stock_news 与 t-sentiment-data 完全重复调用,
        //   实际差异化靠 a-policy 的 system_prompt 提示词过滤"政策类"新闻。
        //   理想方案: 在 src-tauri/src/tools/finance.rs 注册新工具
        //   `get_policy_news`,接受参数 category=policy 走单独数据源(政府/官媒/
        //   监管公告),此处把 tool_name 改为 "get_policy_news" 即可。
        //   本次仅修 title 让 a-policy 与 a-sentiment 在画布上可区分。
        ("t-policy-data", "获取政策新闻", "get_stock_news", "stock_code"),
        // F-8 重排: a-hot-money 前置改为资金流向工具
        ("t-hotmoney-data", "获取资金流向", "get_stock_money_flow", "stock_code"),
        // F-8 重排: a-lockup 前置改为解禁质押工具
        (
            "t-lockup-data",
            "获取解禁+增减持+大宗交易",
            "get_stock_lockup_bundle",
            "stock_code",
        ),
        // F-8 重排: a-research 前置改为研报工具
        ("t-research-data", "获取研报+新闻", "get_stock_research_reports", "stock_code"),
        ("t-sector-data", "获取行情+行业排名", "get_industry_ranking", "stock_code"),
        ("t-catalyst-data", "获取公司公告", "get_stock_announcements", "stock_code"),
    ];

    // ── Phase 1: ParallelNode 作为视觉分组，包裹 9 组 Tool + Agent ──
    // F-1 修复: 布局从"2 列 9 行"改为"3 列 3 行"网格。
    //   原布局 (x=20 单一列, 9 行 80px) 存在 3 类重叠：
    //     1) trigger (x=250, y=0) 与 a-market-analyst (x=240, y=40) 边界框重叠 ~7600 px²
    //     2) p-analysts 容器 (x=300, y=200) 与 a-fundamentals (x=240, y=200)
    //        等 3 行 analyst 节点重叠
    //     3) 单一纵列 9 行总高 720px 浪费大量垂直空间
    //   新布局: 3×3 网格,col_width=480 (tool 200 + gap 40 + agent 200 + 余量 40)
    //     col_x = [40, 520, 1000]
    //     tool x = col_x[col], agent x = col_x[col] + 240
    //     row_y = 100 + row*120  (节点高 80, 行距 40)
    //   trigger 居中放置 x=580 (3 列总宽 1200, 居中后左侧 580),y=0
    //   p-analysts 容器 (20, 80) 起,完整包络 3×3 网格
    let col_x = [40.0_f64, 520.0, 1000.0];
    let row_y_base = 100.0;
    // FIX: agent 节点高度 160px, 之前 row_dy=120 导致连续行重叠 40px
    let row_dy = 180.0;
    let mut analyst_branches: Vec<Branch> = Vec::with_capacity(tool_assignments.len());
    for (i, (tool_id, tool_title, tool_name, arg_key)) in tool_assignments.iter().enumerate() {
        let analyst_id = a_ids[i];
        let col = i % 3;
        let row = i / 3;
        let x_tool = col_x[col];
        let y = row_y_base + row as f64 * row_dy;
        nodes.push(tool_node(
            tool_id,
            // F-2 修复: 原本硬编码 "获取数据" 导致 9 个 tool 节点 title 完全一致、
            // 编辑器画布无法区分。改用 tool_assignments 中已经声明的中文描述。
            tool_title,
            tool_name,
            tool_id,
            arg_key,
            Some("p-analysts"),
            x_tool,
            y,
        ));
        edges.push(edge(&format!("e-trigger-{tool_id}"), "trigger", tool_id));
        edges.push(edge(&format!("e-{tool_id}-{analyst_id}"), tool_id, analyst_id));
        analyst_branches.push(Branch {
            id: format!("branch-{analyst_id}"),
            title: tool_title.to_string(),
            steps: vec![tool_id.to_string(), analyst_id.to_string()],
            branch_timeout_ms: None,
            degrade_strategy: Default::default(),
        });
    }

    // 工具由模板节点 config.tools 统一管理
    // 第 10 个 a-catalyst 放置在 3×3 网格下方（col 0, row 3），作为额外独立行
    for (i, (id, title, _expert)) in analysts.iter().enumerate() {
        let tool_id = tool_assignments[i].0;
        let _fixed_tool_name = tool_assignments[i].2;
        let col = i % 3;
        let row = i / 3;
        let x_agent = col_x[col] + 240.0;
        let row_y = row_y_base + row as f64 * row_dy;
        let mut an = agent(id, title, _expert, Some("p-analysts"), x_agent, row_y);
        if let WorkflowNode::Agent(ref mut a) = an {
            a.config.context_sources = vec![tool_id.to_string()];
            // fundamentals-analyst prompt 引用了 {{market_regime}}，
            // 从 t-scoring 注入 market_regime.state（bull/bear/neutral 状态字符串）
            if *id == "a-fundamentals" {
                a.config.input_mapping.insert(
                    "market_regime".to_string(),
                    "t-scoring.result.market_regime.state".to_string(),
                );
            }
            // catalyst-analyst 需要 3 轮：R1 读公告→确认催化剂,R2 调 K线/概念验证,R3 综合评估叙事
            a.config.max_tool_rounds = if *id == "a-catalyst" {
                Some(3)
            } else {
                Some(2)
            };
            a.config.model_role = Some("stock-analyst".into());
            let tool_names = PROFILE_TOOLS
                .iter()
                .find(|(k, _)| **k == **_expert)
                .map(|(_, v)| *v)
                .unwrap_or(&[]);
            a.config.tools = tool_names
                .iter()
                .filter_map(|&tn| tool_def_map.get(tn).cloned())
                .collect();
            a.config.exposed_tools = vec![];
            // a-catalyst 改用 Json 输出模式，prompt 已改为纯 JSON 格式
            if *id == "a-catalyst" {
                a.config.output_mode = OutputMode::Json;
                // P1 修复(2.2): 强制输出 5 个关键字段，禁止自由文本
                a.config.system_prompt = format!(
                    "{}\n{}\n{}",
                    a.config.system_prompt,
                    tool_prompt(&a.config.tools),
                    "【强制 JSON Schema 约束】\n\
                     输出必须是纯 JSON，包含以下 5 个必填字段:\n\
                     {\n\
                       \"target_entity\": \"受影响的公司/行业/产品名称\",\n\
                       \"event_type\": \"事件类型: 财报/政策/并购/技术突破/行业景气/监管/高管变动/研报/大宗交易/其他\",\n\
                       \"impact_direction\": \"影响方向: positive/negative/neutral\",\n\
                       \"impact_timeline\": \"影响时间线: immediate(1-3天)/short(1-4周)/mid(1-3月)/long(3月+)\",\n\
                       \"confidence_score\": \"置信度评分 0.0-1.0，基于信息来源可信度\"\n\
                     }\n\
                     只输出上述 JSON 对象，前后不要有任何其他文字。"
                );
            } else {
                a.config.system_prompt =
                    format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
            }
            // 环 A: 注入历史反思教训，让分析师看到该股之前的错因和改进建议
            a.config.input_mapping =
                std::collections::HashMap::from([("stock_lessons".into(), "stock_lessons".into())]);
        }
        nodes.push(an);
    }

    // 分析师节点 → c-need-debate 的出边（编辑器可视化 + 运行时依赖）
    for aid in &a_ids {
        edges.push(edge(&format!("e-{aid}-debate"), aid, "debate-bull-bear"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 【装饰节点 / Decorative Container】p-analysts
    // ═══════════════════════════════════════════════════════════════════════
    // 语义：视觉分组容器，包裹 9 组 (Tool + Agent) 子节点
    // 调度：容器本身在引擎中立即 Completed（不参与流程控制）
    //      - wait_for_all=true, aggregation=All: 等所有子节点完成后聚合
    //      - auto_input_from_parent=false: 不自动从父节点拉数据
    //      - 实际依赖通过显式 edge 表达（e-trigger-{tool_id} 和 e-{tool_id}-{aid}）
    // parent_id：仅供前端编辑器嵌套渲染用，运行时调度忽略此字段
    //
    // 为什么不直接用 Edges 表达？
    //   前端需要把 9 个 Tool 和 9 个 Agent 画在一个可折叠的分组框内，
    //   单纯靠 edge 拓扑无法表达"视觉从属关系"。ParallelNode 在此
    //   充当"虚拟容器"，是 workflow_types 中两种角色之一的产物：
    //     1) 真正的并行控制器（wait_for_all + 聚合）
    //     2) 纯装饰性容器（仅前端展示，调度无意义）  ← 属于此类
    // ═══════════════════════════════════════════════════════════════════════
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-analysts".into(),
            title: "10 维度分析师分组".into(),
            description: Some("行情/情绪/新闻/基本面/政策/游资/解禁/研报/行业/催化剂".into()),
            // F-1 修复: 原 (300, 200) 恰好压在 a-fundamentals (240, 200) 上。
            //   3×3 网格范围 x∈[40, 1400] y∈[100, 460],容器左上放 (20, 80),
            //   让前端能正确按 bbox 渲染分组框。
            position: Position { x: 20.0, y: 80.0 },
            retry: RetryConfig::default(),
            timeout: Some(120),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: ParallelNodeConfig {
            branches: analyst_branches,
            wait_for_all: true,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: false, // 不自动从父节点接收输入
            sub_graph: None,               // v23+：稍后通过 inject_container_subgraphs 注入
        },
    }));
    // 前端验证要求容器节点有至少一条入边/出边，这里添加伪边绕过"死分支"检查。
    // 运行时容器立即完成，这些边不影响调度。
    edges.push(WorkflowEdge {
        id: "e-trigger-p-analysts".into(),
        source: "trigger".into(),
        source_handle: None,
        target: "p-analysts".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });
    edges.push(WorkflowEdge {
        id: "e-p-analysts-debate".into(),
        source: "p-analysts".into(),
        source_handle: None,
        target: "debate-bull-bear".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    });

    // Phase 2: 决策检查点 — 记录分析师完成状态，辩论始终执行
    // 分析师节点已直接连接 DebateNode（无中间条件节点）

    // ── 辩论轮数（DAG 展开为 max_rounds 轮顺序执行） ──
    // 用户在「股票分析设置 → 参数 → 工作流 → 多空辩论轮数」中调整的 `debate_rounds`
    // 会在旧模板升级时被 merge_variable_values 保留到 old_variables 里；这里
    // 优先读旧值，确保重建后的 DAG 与用户当前意图一致；缺失/越界时回退到 3。
    let debate_max_rounds: usize = match old_variables.as_deref() {
        Some(s) if !s.is_empty() => serde_json::from_str::<Vec<serde_json::Value>>(s)
            .ok()
            .and_then(|arr| {
                arr.into_iter().find_map(|v| {
                    let name = v.get("name")?.as_str()?;
                    if name != "debate_rounds" {
                        return None;
                    }
                    v.get("value")?.as_u64().map(|n| n as usize)
                })
            })
            .map(|n| n.clamp(1, 10))
            .unwrap_or(3),
        _ => 3,
    };

    // ═══════════════════════════════════════════════════════════════════════
    // 【装饰节点 / Decorative Container】debate-bull-bear
    // ═══════════════════════════════════════════════════════════════════════
    // 语义：多空辩论的视觉分组容器，配置 max_rounds=3 的辩论元数据
    // 调度：容器本身在引擎中立即 Completed（返回 debater_steps 配置，不返回辩论结果）
    //      - debater_steps: 6 个真实辩手节点 (bull-r1..r3, bear-r1..r3)
    //      - max_rounds=3: 固定 3 轮，无"是否收敛"循环控制
    //      - convergence_prompt/model: 配置就绪但当前未启用（避免辩论死循环）
    //
    // ⚠️ 关键陷阱（P0 已修复）：
    //   历史 bug：曾将 value-investor 的入边连到本容器，导致 value-investor
    //   在容器 Completed 时立即启动——拿到的是"辩论配置"而非"辩论结果"。
    //   正确接法：value-investor 应等待最后一个真实辩手节点 bear-r3 完成。
    //
    // 真实调度依赖链（首轮 bull-r1 启动条件）：
    //   trigger → tool → a-* → debate-bull-bear（立即完成）→ bull-r1
    //   后续轮次：bull-r{r+1} 等 bear-r{r}，bear-r{r} 等 bull-r{r}
    // parent_id：仅供前端编辑器嵌套渲染用
    //
    // ⚠️ 坐标约定（FIX: 所有节点位置为画布绝对坐标）：
    //   容器 debate-bull-bear 放在 (DEBATE_X, DEBATE_Y)
    //   辩手节点 x = DEBATE_X + 20px（容器内偏移）
    //   辩手节点 y = DEBATE_Y + 40px + round*2*180px（按轮次纵向排列）
    //   前端 WorkflowEditor 通过 parentId 减去容器坐标得到相对坐标交给 ReactFlow。
    // ═══════════════════════════════════════════════════════════════════════
    const DEBATE_X: f64 = 300.0;
    const DEBATE_Y: f64 = 1280.0;
    nodes.push(WorkflowNode::Debate(DebateNode {
        base: WorkflowNodeBase {
            id: "debate-bull-bear".into(),
            title: "多空辩论".into(),
            description: Some(format!(
                "{debate_max_rounds} 轮多空辩论：多方构建论点 → 空方反驳 → 循环"
            )),
            position: Position {
                x: DEBATE_X,
                y: DEBATE_Y,
            },
            retry: RetryConfig {
                enabled: true,
                max_retries: 1,
                ..Default::default()
            },
            timeout: Some(900),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: DebateNodeConfig {
            debater_steps: (0..debate_max_rounds)
                .flat_map(|r| vec![format!("bull-r{}", r + 1), format!("bear-r{}", r + 1)])
                .collect(),
            max_rounds: debate_max_rounds as u32,
            convergence_prompt: None,
            convergence_model: None,
            convergence_model_role: None,
            topic_var: "trigger.output".into(),
            output_var: String::new(),
            sub_graph: None, // v23+：稍后通过 inject_container_subgraphs 注入
        },
    }));

    // DebateNode 的子节点：按轮次展开多方辩手和空方辩手
    // parentId 指向容器节点，前端将它们渲染在 DebateNode 内部
    // 位置：容器内 20px 左偏移，按轮次纵向排列（绝对坐标 = 容器坐标 + 偏移）
    // v16 修复:R1/R2/R3 多空双方统一用 bull_tools(基础数据工具集),让 R2/R3
    // LMM 能拿到 stock_quote/kline/financials/news 等基础数据,避免工具调用
    // 全部返回空导致 R2/R3 输出 "暂无数据"。R2/R3 的"质询型"角色由
    // bull-r2.md / bear-r2.md / bull-r3.md / bear-r3.md prompt 控制,与工具集无关。
    // v17+ 可考虑给空方注入估值/风险类特色工具(td_var / td_maxdd / td_pledge / td_corr),
    // 当前 v16 简化统一工具集优先修复 R2/R3 没数据问题。
    let bull_tools = vec![
        td_quote.clone(),
        td_kline.clone(),
        td_fin.clone(),
        td_news.clone(),
        td_score.clone(),
        td_earnings.clone(),
        td_ma_cross.clone(),
    ];

    for round in 0..debate_max_rounds {
        let round_num = round + 1;
        let bull_id = format!("bull-r{round_num}");
        let bear_id = format!("bear-r{round_num}");
        // R1 走 bull-researcher / bear-researcher（初始论证型），R2 走 bull-r2 / bear-r2
        // （质询型），R3 走 bull-r3 / bear-r3（最终反驳型）。R2/R3 工具集一致：
        // 都需要 compute_scoring / compute_valuation 核实对方论据中的技术/估值假设。
        let bull_expert = match round_num {
            2 => "bull-r2",
            3 => "bull-r3",
            _ => "bull-researcher",
        };
        let bear_expert = match round_num {
            2 => "bear-r2",
            3 => "bear-r3",
            _ => "bear-researcher",
        };
        let bull_title = format!("多方研究员·第{round_num}轮");
        let bear_title = format!("空方研究员·第{round_num}轮");
        // 绝对坐标 = 容器基准 + 内部偏移
        let bull_x = DEBATE_X + 20.0;
        let bull_y = DEBATE_Y + 40.0 + (round * 2) as f64 * 180.0;
        let bear_x = DEBATE_X + 20.0;
        let bear_y = DEBATE_Y + 40.0 + (round * 2 + 1) as f64 * 180.0;

        // 多方辩手：首轮无前置辩论上下文，后续轮次引用所有前序辩论输出
        let mut bull_an =
            agent(&bull_id, &bull_title, bull_expert, Some("debate-bull-bear"), bull_x, bull_y);
        if let WorkflowNode::Agent(ref mut a) = bull_an {
            // R1 用 bull_tools 工具集(含 get_stock_quote/kline/financials/news 等基础数据工具,
            //   LLM 能直接调通拿数据,产出论据)。
            // R2/R3 走 PROFILE_TOOLS 路径(质询/反驳需技术评分+估值工具)。
            // 修复(v16):R1/R2/R3 多空辩手统一用 bull_tools(基础数据工具集)。
            //   之前 R2/R3 走 PROFILE_TOOLS(只有 compute_scoring / compute_valuation
            //   计算工具)—— R2/R3 没有上游数据节点,LLM 拿不到 stock_quote / kline
            //   / financials / news 等基础数据,工具调用全部返回空,导致 R2/R3
            //   输出 "暂无数据"。
            //   R2 质询 / R3 反驳的角色由 bull-r2.md / bear-r2.md / bull-r3.md /
            //   bear-r3.md prompt 控制,与工具集无关。
            // 修复(阶段 4):辩论子节点加 1 次重试 + 180s 超时。LLM 偶发超时/429
            //   是单点失败主因,max_retries=0 导致整链雪崩(bear-r1 拿不到 bull-r1
            //   上下文则 R2/R3 全部"暂无数据")。1 次重试覆盖 ~95% 瞬时失败,不会
            //   把工作流时长翻倍(30s 退避)。
            a.base.retry = RetryConfig {
                enabled: true,
                max_retries: 1,
                ..Default::default()
            };
            a.base.timeout = Some(180);
            a.config.tools = bull_tools.clone();
            a.config.exposed_tools = bull_tools.iter().map(|t| t.name.clone()).collect();
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&bull_tools));
            a.config.max_tool_rounds = Some(2);
            a.config.model_role = Some("debater".into());
            // 注入前序轮次辩论输出 + 所有分析师报告作为上下文
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..round_num {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            // 添加所有分析师报告，让辩手有素材可辩论
            for aid in &a_ids {
                ctx.push(aid.to_string());
            }
            a.config.context_sources = ctx;
            // 注入分析师 params 作为结构化输入（resolve_var_path 支持点号路径）
            a.config.input_mapping = build_analyst_input_mapping(&a_ids);
        }
        nodes.push(bull_an);

        // 空方辩手：引用本轮多方输出 + 前序轮次辩论输出
        let mut bear_an =
            agent(&bear_id, &bear_title, bear_expert, Some("debate-bull-bear"), bear_x, bear_y);
        if let WorkflowNode::Agent(ref mut a) = bear_an {
            // 同 bull_an:R1/R2/R3 空方统一用 bull_tools。
            // 修复(阶段 4):同 bull_an,加 1 次重试 + 180s 超时,避免 LLM 瞬时失败
            //   导致辩论链雪崩(详见 bull_an 注释)。
            a.base.retry = RetryConfig {
                enabled: true,
                max_retries: 1,
                ..Default::default()
            };
            a.base.timeout = Some(180);
            a.config.tools = bull_tools.clone();
            a.config.exposed_tools = bull_tools.iter().map(|t| t.name.clone()).collect();
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&bull_tools));
            a.config.max_tool_rounds = Some(2);
            a.config.model_role = Some("debater".into());
            // 注入前序轮次 + 本轮多方输出 + 所有分析师报告作为上下文
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..round_num {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            ctx.push(bull_id.clone());
            // 添加所有分析师报告
            for aid in &a_ids {
                ctx.push(aid.to_string());
            }
            a.config.context_sources = ctx;
            // 注入分析师 params 作为结构化输入
            a.config.input_mapping = build_analyst_input_mapping(&a_ids);
        }
        nodes.push(bear_an);

        // ── 轮次依赖边 ──
        if round == 0 {
            // 首轮：从 DebateNode 容器出发
            edges.push(edge(&format!("e-debate-bull-r{round_num}"), "debate-bull-bear", &bull_id));
        } else {
            // 后续轮次：上一轮空方完成后启动本轮多方
            let prev_bear = format!("bear-r{}", round);
            edges.push(edge(&format!("e-r{round}-bull-r{round_num}"), &prev_bear, &bull_id));
        }
        // 每轮：多方 → 空方（空方看到多方论点后反驳）
        edges.push(edge(&format!("e-bull-r{round_num}-bear-r{round_num}"), &bull_id, &bear_id));
    }

    // ── debate-convergence（辩论收敛分析）──
    // 读取全部 6 轮辩手输出，输出 consensus_score 供 portfolio-mgr 公式使用。
    // 入边从 bear-r{debate_max_rounds} 出发，确保等真辩论结束后再启动收敛。
    // 出边到 value-investor 和 portfolio-mgr，确保收敛结果在决策前可用。
    {
        let last_debate_node = format!("bear-r{debate_max_rounds}");
        let mut dc = agent(
            "debate-convergence",
            "辩论结果收敛：consensus_score 聚合",
            "debate-convergence",
            None,
            500.0,
            1420.0,
        );
        if let WorkflowNode::Agent(ref mut a) = dc {
            // 动态构建 context_sources：根据实际辩论轮数引用所有辩手输出
            // 同时包含全部分析师报告，因为 input_mapping 通过 build_analyst_input_mapping
            // 注入了 10 个分析师的 bull_score/bear_score/consensus_score 结构化字段，
            // 追加分析师节点到 context_sources 确保 context_sources 覆盖 input_mapping 引用。
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..=debate_max_rounds {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            for aid in &a_ids {
                ctx.push(aid.to_string());
            }
            a.config.context_sources = ctx;
            a.config.model_role = Some("debater".into());
            a.config.max_tool_rounds = Some(1);
            a.config.output_mode = OutputMode::Json; // 输出结构化 JSON，确保 consensus_score / aggregate_prediction 被 input_mapping 解析
            a.config.input_mapping = build_analyst_input_mapping(&a_ids);
        }
        nodes.push(dc);
        edges.push(edge("e-bear-r3-debate-convergence", &last_debate_node, "debate-convergence"));
    }

    // ── value-investor（巴菲特框架）：在辩论之后、与风险评估并行运行 ──
    // 入边从 bear-r{debate_max_rounds} 出发，确保等真辩论收敛后再启动
    // （debate-bull-bear 是 DebateNode 容器，立即 Completed，返回的是配置而非辩论结果）
    {
        let vi_id = "value-investor";
        let vi_title = "以巴菲特-芒格价值投资理念评估该标的，分析护城河、财务健康度、管理层、安全边际，输出结构化估值框架";
        let vi_y = 1540.0;
        let last_debate_node = format!("bear-r{debate_max_rounds}");
        let mut vi = agent(vi_id, vi_title, "value-investor", None, 20.0, vi_y);
        if let WorkflowNode::Agent(ref mut a) = vi {
            a.config.context_sources = vec![
                "a-fundamentals".into(),
                "a-research".into(),
                "a-sector".into(),
                // 改为辩论最后一轮空方的输出（真辩论结论），而非 DebateNode 容器
                last_debate_node.clone(),
                "debate-convergence".into(),
            ];
            a.config.model_role = Some("stock-analyst".into());
            a.config.max_tool_rounds = Some(2);
            // value-investor 改用 Json 输出模式，prompt 已从 VERDICT 标签改为纯 JSON 格式
            a.config.output_mode = OutputMode::Json;
            let tool_names = PROFILE_TOOLS
                .iter()
                .find(|(k, _)| *k == "value-investor")
                .map(|(_, v)| *v)
                .unwrap_or(&[]);
            a.config.tools = tool_names
                .iter()
                .filter_map(|&tn| tool_def_map.get(tn).cloned())
                .collect();
            a.config.exposed_tools = tool_names.iter().map(|&tn| tn.to_string()).collect();
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
            // 环 A: 注入历史反思教训
            a.config.input_mapping =
                std::collections::HashMap::from([("stock_lessons".into(), "stock_lessons".into())]);
        }
        nodes.push(vi);
        edges.push(edge("e-debate-value-investor", &last_debate_node, vi_id));
        // value-investor 的 context_sources 中 debate-convergence 需要显式边，
        // 否则只在 bear-r3 完成后就调度，debate-convergence 还没跑完
        edges.push(edge("e-convergence-value-investor", "debate-convergence", vi_id));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 【装饰节点 / Decorative Container】p-risk-assess
    // ═══════════════════════════════════════════════════════════════════════
    // 语义：风险评估的视觉分组容器，包裹 3 个并行风险偏好 Agent
    // 调度：与 p-analysts 相同——容器立即 Completed，子节点独立调度
    //      - aggressive-debator / conservative-debator / neutral-debator
    //      - 3 个子节点共享同一份风险输入（来自聚合后的辩论+分析师输出）
    //      - 实际依赖通过显式 edge 表达：e-bear-r3 → p-risk-assess（容器）→ 3 个子节点
    //        容器完成是"瞬时"的，3 个子节点会同时被引擎解锁
    // parent_id：仅供前端编辑器嵌套渲染用
    //
    // 与 p-analysts 的区别：
    //   p-analysts 包裹 9 组 (Tool+Agent) 强调"数据预拉 + 分析"两阶段
    //   p-risk-assess 包裹纯 Agent，强调"同输入多视角并行评估"
    //
    // ⚠️ 坐标约定（FIX: 所有节点位置为画布绝对坐标）：
    //   容器 p-risk-assess 放在 (RISK_X, RISK_Y)
    //   子节点 x = RISK_X + 20px, y = RISK_Y + 40px + i*180px
    //   前端 WorkflowEditor 通过 parentId 减去容器坐标得到相对坐标。
    // ═══════════════════════════════════════════════════════════════════════
    const RISK_X: f64 = 300.0;
    const RISK_Y: f64 = 1800.0;
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-risk-assess".into(),
            // F-3 修复: 原本 title="风险评估" 与下面的 t-risk (compute_portfolio_risk) 同名，
            // 编辑器画布上无法区分视觉分组与单 tool。改为"三档风险评估分组"。
            title: "三档风险评估分组".into(),
            description: Some("三种风险偏好并行评估".into()),
            position: Position {
                x: RISK_X,
                y: RISK_Y,
            },
            retry: RetryConfig::default(),
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "risk-agg".into(),
                    title: "激进评估".into(),
                    steps: vec!["risk-agg".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: Default::default(),
                },
                Branch {
                    id: "risk-con".into(),
                    title: "保守评估".into(),
                    steps: vec!["risk-con".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: Default::default(),
                },
                Branch {
                    id: "risk-neu".into(),
                    title: "中性评估".into(),
                    steps: vec!["risk-neu".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: Default::default(),
                },
            ],
            wait_for_all: true,
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: false,
            timeout: Some(600),
            sub_graph: None, // v23+：稍后通过 inject_container_subgraphs 注入
        },
    }));
    edges.push(edge(
        "e-debate-p-risk-assess",
        &format!("bear-r{debate_max_rounds}"),
        "p-risk-assess",
    ));
    // risk 节点的 context_sources 中 bull-r3/t-scoring/t-valuation/debate-convergence
    // 需要显式边等待；否则 bear-r3 完成后就调度，但缺少边连接的节点输出
    // 不会进入 deps_results/exec_ctx.variables，导致 context_sources 报 ERROR。
    // 注：t-valuation 虽已可到达（链 bear-r3→t-scoring→t-valuation），但无直接边
    // 则 bull-r3/t-scoring 的输出不进入变量池。
    edges.push(edge(
        "e-bull-r3-p-risk-assess",
        &format!("bull-r{debate_max_rounds}"),
        "p-risk-assess",
    ));
    edges.push(edge("e-scoring-p-risk-assess", "t-scoring", "p-risk-assess"));
    edges.push(edge("e-valuation-p-risk-assess", "t-valuation", "p-risk-assess"));
    edges.push(edge("e-convergence-p-risk-assess", "debate-convergence", "p-risk-assess"));

    for (i, (rid, rtitle, rexpert, rtools)) in [
        (
            "risk-agg",
            "以最激进的风险偏好评估该股票",
            "aggressive-debator",
            vec![
                td_score.clone(),
                td_risk.clone(),
                td_maxdd.clone(),
                td_var.clone(),
                td_kelly.clone(),
                td_mc.clone(),
            ],
        ),
        (
            "risk-con",
            "以最保守的风险偏好评估该股票",
            "conservative-debator",
            vec![
                td_score.clone(),
                td_risk.clone(),
                td_sharpe.clone(),
                td_maxdd.clone(),
                td_pledge.clone(),
                td_corr.clone(),
            ],
        ),
        (
            "risk-neu",
            "以中性风险偏好评估该股票",
            "neutral-debator",
            vec![
                td_score.clone(),
                td_risk.clone(),
                td_val.clone(),
                td_pe_pct.clone(),
                td_peg.clone(),
                td_rp.clone(),
                td_ind.clone(),
            ],
        ),
    ]
    .iter()
    .enumerate()
    {
        let risk_y = RISK_Y + 40.0 + i as f64 * 180.0;
        let risk_x = RISK_X + 20.0;
        let mut an = agent(rid, rtitle, rexpert, Some("p-risk-assess"), risk_x, risk_y);
        if let WorkflowNode::Agent(ref mut a) = an {
            a.config.tools = rtools.clone();
            a.config.max_tool_rounds = Some(2);
            a.config.system_prompt = format!("{}{}", a.config.system_prompt, tool_prompt(rtools));
            a.config.model_role = Some("risk-evaluator".into());
            // 修复：风险评估 Agent 需要读到上游分析师报告 + 辩论结果 + 技术指标，
            // 否则 LLM 没有分析素材，不会主动调用工具。
            a.config.context_sources = vec![
                "a-market-analyst".into(),
                "a-sentiment".into(),
                "a-news".into(),
                "a-fundamentals".into(),
                "a-policy".into(),
                "a-hot-money".into(),
                "a-lockup".into(),
                "a-research".into(),
                "a-sector".into(),
                "a-catalyst".into(),
                format!("bull-r{debate_max_rounds}"),
                format!("bear-r{debate_max_rounds}"),
                "debate-convergence".into(),
                "t-scoring".into(),
                "t-valuation".into(),
            ];
            a.config.input_mapping = {
                let mut m = build_analyst_input_mapping(&a_ids);
                // 注入辩论收敛的 consensus_score 供 Kelly 公式使用
                // 路径规则（V29 修复）：AgentNode 输出包裹在 {role, content: <json_string>, ...} 中，
                // resolve_var_path 遇到 Value::String 会自动 from_str 解析后再继续下钻，
                // 因此必须用 .content.field 路径访问 AgentNode 业务字段。
                m.insert(
                    "consensus_score".to_string(),
                    "debate-convergence.content.consensus_score".to_string(),
                );
                m
            };
        }
        nodes.push(an);
        // p-risk-assess 容器 → 子节点依赖边：防止子节点被独立调度
        edges.push(edge(&format!("e-p-risk-{rid}"), "p-risk-assess", rid));
    }

    // ── AggregatorNode: 聚合三种风险偏好评估结果 ──
    nodes.push(WorkflowNode::Aggregator(AggregatorNode {
        base: WorkflowNodeBase {
            id: "agg-risk".into(),
            title: "风险偏好聚合".into(),
            description: Some("聚合激进/保守/中性三种风险偏好评估".into()),
            position: Position {
                x: 300.0,
                y: 2400.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(60),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: AggregatorNodeConfig {
            strategy: "all".into(),
            input_sources: vec!["risk-agg".into(), "risk-con".into(), "risk-neu".into()],
            output_var: "risk-aggregated".into(),
            wait_for_all: true,
            weights: vec![],
            summarize_prompt: None,
            summarize_model: None,
            sub_graph: None,
        },
    }));
    for rid in &["risk-agg", "risk-con", "risk-neu"] {
        edges.push(edge(&format!("e-{rid}-agg-risk"), rid, "agg-risk"));
    }

    // ── P1-3: 三档风险辩论收敛（agg-risk 之后、算法工具之前）──
    // 读取三方风险评估输出，分析分歧并生成收敛报告。
    // 收敛输出结构见 risk-convergence.md
    {
        let mut rc = agent(
            "risk-convergence",
            "三档风险辩论收敛：分歧分析与综合裁决",
            "risk-convergence",
            None,
            300.0,
            2550.0,
        );
        if let WorkflowNode::Agent(ref mut a) = rc {
            a.config.context_sources =
                vec!["risk-agg".into(), "risk-con".into(), "risk-neu".into()];
            a.config.model_role = Some("risk-evaluator".into());
            a.config.max_tool_rounds = Some(1);
            a.config.output_mode = OutputMode::Json; // V54 修复: 纯JSON输出,使 content.disagreement_score 可解析
        }
        nodes.push(rc);
        edges.push(edge("e-agg-risk-risk-convergence", "agg-risk", "risk-convergence"));
    }

    // ── 算法 Tool 节点：仅 3 个核心评分/估值/风控（独立画布节点，parent_id = None）──
    // 位置：risk-convergence 节点 (300, 2550) 之后横排，间距 180
    // 位置：agg-risk 节点 (300, 2400) 之后横排，间距 180
    let algo_tools: &[(&str, &str, &str, &str, f64, f64)] = &[
        ("t-scoring", "技术评分", "compute_scoring", "stock_code", 300.0, 2700.0),
        ("t-valuation", "估值计算", "compute_valuation", "stock_code", 480.0, 2700.0),
        // F-3 修复: title 由 "风险评估" 改为 "组合风险计算"，避免与
        // 上面的 p-risk-assess 容器（"三档风险评估分组"）同名混淆。
        ("t-risk", "组合风险计算", "compute_portfolio_risk", "stock_codes", 660.0, 2700.0),
    ];
    for (tool_id, title, tool_name, arg_key, x, y) in algo_tools {
        nodes.push(tool_node(tool_id, title, tool_name, tool_id, arg_key, None, *x, *y));
    }
    edges.push(edge("e-bear-r3-t-scoring", "bear-r3", "t-scoring"));
    edges.push(edge("e-t-scoring-t-valuation", "t-scoring", "t-valuation"));
    edges.push(edge("e-t-valuation-t-risk", "t-valuation", "t-risk"));

    // ── P1/P2 新增: 龙虎榜数据获取节点（独立 ToolNode，不配 Agent）──
    // 独立创建以保持 raw-data 聚合的完整性，同时直接供给 portfolio-mgr 做筹码面分析增强。
    // 龙虎榜数据包含机构席位买卖动向、游资席位上榜频率等，是 f10 筹码面因子的重要补充。
    let dragon_tiger_id = "t-dragon-tiger-data";
    nodes.push(tool_node(
        dragon_tiger_id,
        "获取个股龙虎榜明细",
        "get_stock_dragon_tiger",
        dragon_tiger_id,
        "stock_code",
        None,
        840.0,  // x: 接在 t-risk (660) 之后
        2700.0, // y: 与 algo_tools 同行
    ));
    edges.push(edge(&format!("e-bear-r3-{dragon_tiger_id}"), "bear-r3", dragon_tiger_id));

    // ── P3 (real-nodes): raw-data 聚合节点 ──
    // 把 13 个 t-* / algo 工具节点的输出聚合成单个 raw 对象，供 portfolio-mgr 决策时
    // 通过 context_sources 读取 "raw-data-aggregated" 变量。
    //
    // F-5 修复: 显式追加 e-raw-data-portfolio-mgr 边。
    //   原设计 raw-data 入度 12、出度 0，仅靠 portfolio-mgr.context_sources 消费。
    //   1) 上游 validate_workflow 会把 raw-data 标为"data_blackhole"硬错误
    //   2) 画布上 raw-data 与 portfolio-mgr 之间无连线，可视化上看像断头
    //   aggregator 节点本身是纯数据合并（不调 LLM），调度等待成本可忽略；
    //   加边后 portfolio-mgr 启动前的等待时间依然是 max(trader, raw-data)，
    //   raw-data 远快于 trader，无可观察的延迟变化。
    let raw_input_sources: Vec<String> = algo_tools
        .iter()
        .map(|(id, _, _, _, _, _)| id.to_string())
        .chain(tool_assignments.iter().map(|(id, _, _, _)| id.to_string()))
        .chain(std::iter::once(dragon_tiger_id.to_string()))
        .collect();
    nodes.push(WorkflowNode::Aggregator(AggregatorNode {
        base: WorkflowNodeBase {
            id: "raw-data".into(),
            title: "原始数据聚合".into(),
            description: Some("聚合 13 个工具节点的原始输出（10 个数据源 + 3 个算法）".into()),
            position: Position {
                x: 840.0,
                y: 2700.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: AggregatorNodeConfig {
            strategy: "all".into(),
            input_sources: raw_input_sources,
            output_var: "raw-data-aggregated".into(),
            wait_for_all: true,
            weights: vec![],
            summarize_prompt: None,
            summarize_model: None,
            sub_graph: None,
        },
    }));
    // 修复 Defect #8: 为 raw-data 显式添加 13 个 tool 节点的入边。
    // 之前只有 1 条 e-t-risk-raw-data 边，依赖关系是"隐性"的（依赖 t-risk 是
    // 13 个 tool 节点中最深的间接前置）。改成显式声明后，调度器会等待所有 13
    // 个上游 tool 节点都完成才启动 raw-data，input_sources 才有数据可读。
    // 迭代器自然包含 e-t-risk-raw-data（来自 algo_tools 末项）。
    for src in algo_tools
        .iter()
        .map(|(id, _, _, _, _, _)| *id)
        .chain(tool_assignments.iter().map(|(id, _, _, _)| *id))
        .chain(std::iter::once(dragon_tiger_id))
    {
        edges.push(edge(&format!("e-{src}-raw-data"), src, "raw-data"));
    }
    // F-5: 显式出边到 portfolio-mgr，让上游 validate_workflow 的"data_blackhole"
    //      规则不再误报，同时让画布上能看到 raw-data → portfolio-mgr 的连线。
    //      注意：portfolio-mgr 已改为 CodeNode，不设 context_sources，
    //      raw-data 通过显式边 e-raw-data-portfolio-mgr 确保调度依赖。
    edges.push(edge("e-raw-data-portfolio-mgr", "raw-data", "portfolio-mgr"));

    // ── LlmClassifierNode: 风险等级分类 ──
    nodes.push(WorkflowNode::LlmClassifier(LlmClassifierNode {
        base: WorkflowNodeBase {
            id: "cls-risk-level".into(),
            title: "风险等级分类".into(),
            description: Some("基于算法评分结果自动分类风险等级".into()),
            position: Position {
                x: 300.0,
                y: 3000.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
        continue_on_fail: false,
        },
        config: LlmClassifierNodeConfig {
            categories: vec![
                "低风险".into(),
                "中风险".into(),
                "高风险".into(),
                // V38 修复: 增加"极高风险"档位（退市/造假/流动性危机），
                // 与 portfolio-mgr.rhai 的 D1 修复（极高风险→立即卖出清仓）对齐
                "极高风险".into(),
            ],
            prompt: "你是专业风险分析师。根据以下单股风险画像数据，\
                     判断该股票的整体风险等级（低风险/中风险/高风险/极高风险）。\
                     \n\n## 数据解读（A股标准）\
                     \n### 量化指标\
                     \n- annualizedVolatilityPct: 年化波动率（%）。A股正常15-45%，<20%低波动，20-35%正常，35-50%偏高，>50%高波动\
                     \n- maxDrawdownPct: 历史最大回撤（%）。A股正常20-40%，<25%好，25-40%正常，40-55%偏大，>55%深\
                     \n- sharpeRatio: 夏普比率。>0.5好，0-0.5正常，<0偏弱\
                     \n\
                     \n### 基本面指标\
                     \n- roeTTMPct: ROE(TTM)（%）。>10%良好，5-10%一般，<5%偏弱\
                     \n- grossMarginPct: 毛利率（%）。>25%好，15-25%正常，<15%偏低\
                     \n- debtRatioPct: 资产负债率（%）。<40%低，40-60%正常，>60%偏高\
                     \n- revenueGrowthYoYPct: 营收增速YoY（%）。>15%好，5-15%正常，0-5%偏低，<0萎缩\
                     \n- peTTM: 市盈率(TTM)。<0亏损，0-20低，20-40正常，>40偏高\
                     \n\
                     \n\n## 等级判定规则（按优先级，满足即判定，不要计算综合评分）\
                     \n### 极高风险（立即回避）\
                     \n- 标的为ST/*ST/退市股\
                     \n- 资产负债率>80% 且 营收增速<0（财务困境）\
                     \n- 年化波动率>60% 且 夏普比率<-1.5\
                     \n\
                     \n### 高风险（谨慎参与，小仓位）\
                     \n- 量化维度高风险：波动率>40% 或 夏普<0 或 回撤>45%\
                     \n- 且基本面有至少一个风险点：ROE<5% 或 毛利率<10% 或 负债率>65%\
                     \n- 或：量化偏高（波动率>35% 或 夏pe<0.3）且基本面无亮点（ROE<8% 且 营收增速<5%）\
                     \n\
                     \n### 中风险（正常参与）\
                     \n- 量化维度中等：波动率20-40% 或 夏普0-0.5 或 回撤25-45%\
                     \n- 且基本面无硬伤：ROE>5% 且 负债率<65% 且 营收增速>0\
                     \n- 或：量化低风险（波动率<20% 且 夏普>0.5）但基本面中等\
                     \n\
                     \n### 低风险（优先配置）\
                     \n- 量化维度低风险：波动率<20% 且 夏普>0.5 且 回撤<30%\
                     \n- 且基本面健康：ROE>10% 且 毛利率>20% 且 负债率<50% 且 营收增速>5%\
                     \n- 且：无ST/退市风险，无重大负面公告\
                     \n\
                     \n\n## 重要\
                     \n- 不要计算综合评分，直接按规则判定\
                     \n- A股大多数股票应落在「中风险」档\
                     \n- 仅当量化+基本面均差时才判「高风险」\
                     \n\
                     \n\n## 输入数据\n{input_text}\n\n请仅输出一行：风险等级|最短理由\
                     \n例如：中风险|波动率28%正常，ROE 8.5%一般，负债率52%可控"
                .into(),
            model: None,
            // V50 修复: t-risk 现在对单股输出 stockRiskProfile（波动率/VaR/最大回撤/夏普），
            // 不再是旧版的组合级 HHI/集中度。分类器基于真实风险指标做判断。
            input_var: "t-risk".into(),
            output_var: "risk-level".into(),
            confidence_threshold: None,
            fallback_label: None,
            consistency_check: None,
        },
    }));
    edges.push(edge("e-t-risk-cls-risk", "t-risk", "cls-risk-level"));

    // ── Validation: 结果完整性校验 ──
    nodes.push(WorkflowNode::Validation(ValidationNode {
        base: WorkflowNodeBase {
            id: "v-validate".into(),
            title: "结果完整性校验".into(),
            description: Some("确保分析报告包含必要字段，缺失时降级处理".into()),
            position: Position {
                x: 300.0,
                y: 3300.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(60),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: ValidationNodeConfig {
            assertions: vec![ValidationAssertion {
                assertion_type: "exists".into(),
                expected: None,
                actual: Some("t-risk.output".into()),
                expression: None,
            }],
            on_fail: "skip".into(),
            max_retries: 1,
        },
    }));
    edges.push(edge("e-cls-risk-v-validate", "cls-risk-level", "v-validate"));

    // ── P3 (real-nodes): data-quality 数据质量检查 Agent ──
    // 等待 v-validate 完成（在 cls-risk-level + t-* algo 全跑完后），
    // 然后评估所有分析师报告的覆盖度、字数、占位检测、一致性。
    // 与 research-mgr 并行启动，输出通过 portfolio-mgr.context_sources 注入
    // 最终决策（见 portfolio-mgr 节点的 context_sources 配置）。
    //
    // F-6 修复: data-quality 是有意"仅靠 context_sources 消费"的终态。
    //   data-quality 是慢速 LLM agent（约 5-10s）,与 research-mgr → trader 链路
    //   并行执行。如果加 e-data-quality-portfolio-mgr 显式边,调度器会强制
    //   portfolio-mgr 等待 data-quality 完成,串行化整条路径,引入不必要的延迟。
    //   正确做法是保持 context_sources 消费模式,允许并行。
    //   画布上 data-quality 看似"断头"是预期设计,非真实 bug。
    //
    //   注: 如果未来上游 validate_workflow 把 data-quality 标为 data_blackhole
    //       或 orphan,可考虑给节点加 kind="context_sink" 标记让校验跳过。
    {
        let dq_id = "data-quality";
        let dq_title = "数据质量评估：覆盖度、字数、占位检测，输出 JSON 格式 grade/score";
        let dq_y = 3300.0;
        let mut dq = agent(dq_id, dq_title, "data-quality-inspector", None, 840.0, dq_y);
        if let WorkflowNode::Agent(ref mut a) = dq {
            a.config.context_sources = vec![
                "a-market-analyst".into(),
                "a-sentiment".into(),
                "a-news".into(),
                "a-fundamentals".into(),
                "a-policy".into(),
                "a-hot-money".into(),
                "a-lockup".into(),
                "a-research".into(),
                "a-sector".into(),
                "a-catalyst".into(),
                // 注入算法工具节点的 credibility 元数据，支持数据质量检查员
                // 评估工具可信度分的 4 个维度（freshness/completeness/warnings/source）
                "t-scoring".into(),
                "t-valuation".into(),
                "t-risk".into(),
            ];
            // ── 结构化参数注入（结构化参数方案 Phase 2）──
            // 注入各分析师的 confidence 结构化值，使 DQI 可直接判断
            // "信心低迷（confidence < 30）" 条件，无需从文本中重新提取。
            //
            // 路径规则（V29 修复）：AgentNode 输出包裹在 {role, content: <json_string>, ...} 中，
            // resolve_var_path 遇到 Value::String 会自动 from_str 解析后再继续下钻，
            // 因此必须用 `.content.field` 路径访问业务字段。
            a.config.input_mapping = [
                ("mk_confidence", "a-market-analyst.content.confidence"),
                ("sent_confidence", "a-sentiment.content.confidence"),
                ("news_confidence", "a-news.content.confidence"),
                ("fund_confidence", "a-fundamentals.content.confidence"),
                ("pol_confidence", "a-policy.content.confidence"),
                ("hm_confidence", "a-hot-money.content.confidence"),
                ("lk_confidence", "a-lockup.content.confidence"),
                ("res_confidence", "a-research.content.confidence"),
                ("sec_confidence", "a-sector.content.confidence"),
                ("cat_confidence", "a-catalyst.content.confidence"),
                // 注入各分析师的 if_data_gaps 布尔值，无需扫描全文检查缺失项
                ("mk_data_gaps", "a-market-analyst.content.if_data_gaps"),
                ("sent_data_gaps", "a-sentiment.content.if_data_gaps"),
                ("news_data_gaps", "a-news.content.if_data_gaps"),
                ("fund_data_gaps", "a-fundamentals.content.if_data_gaps"),
                ("pol_data_gaps", "a-policy.content.if_data_gaps"),
                ("hm_data_gaps", "a-hot-money.content.if_data_gaps"),
                ("lk_data_gaps", "a-lockup.content.if_data_gaps"),
                ("res_data_gaps", "a-research.content.if_data_gaps"),
                ("sec_data_gaps", "a-sector.content.if_data_gaps"),
                ("cat_data_gaps", "a-catalyst.content.if_data_gaps"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            a.config.output_mode = OutputMode::Json; // V36: 改为 JSON 输出模式，确保 grade/score 被 resolve_var_path 正确解析
            a.config.model_role = Some("stock-analyst".into());
            let tool_names = PROFILE_TOOLS
                .iter()
                .find(|(k, _)| *k == "data-quality-inspector")
                .map(|(_, v)| *v)
                .unwrap_or(&[]);
            a.config.tools = tool_names
                .iter()
                .filter_map(|&tn| tool_def_map.get(tn).cloned())
                .collect();
            a.config.exposed_tools = tool_names.iter().map(|&tn| tn.to_string()).collect();
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
        }
        nodes.push(dq);
        edges.push(edge("e-v-validate-data-quality", "v-validate", dq_id));
    }

    // research-mgr → trader → portfolio-mgr
    let mut rm = agent(
        "research-mgr",
        "综合风险评估：总体风险评级与主要风险点清单",
        "research-manager",
        None,
        240.0,
        3600.0,
    );
    if let WorkflowNode::Agent(ref mut a) = rm {
        a.config.context_sources = vec![
            "value-investor".into(),
            "t-scoring".into(),
            "t-valuation".into(),
            "t-risk".into(),
            // V29 修复: 改为引用三档风险评估的原始 AgentNode，而非聚合后的数组
            // AggregatorNode strategy="all" 的 result 是数组，无法用对象字段路径导航，
            // 因此 research-mgr 直接消费三个原始风险辩手的输出
            "risk-agg".into(),
            "risk-con".into(),
            "risk-neu".into(),
            "risk-aggregated".into(),
            "risk-level".into(),
            // V29 修复: input_mapping 引用 debate-convergence，需在 context_sources 中声明
            "debate-convergence".into(),
        ];
        // ── 结构化参数注入（结构化参数方案 Phase 2）──
        // 注入风险的结构化评分，使 research-mgr 可在 system_prompt 中
        // 直接使用 risk_level 等值，无需从文本中重新提取。
        //
        // 路径规则（V29 修复）：
        // - LlmClassifierNode: {category, model, ...} → 直接 .category
        // - AgentNode: {role, content: <json_string>, ...} → .content.field
        //   （resolve_var_path 遇到 Value::String 会自动 from_str 解析后再下钻）
        // - AggregatorNode strategy="all": result 是数组，不支持对象字段路径导航，
        //   改为直接引用原始 AgentNode 的 .content.position_pct
        a.config.input_mapping = [
            // P1 修复(3.2 信息隔离): 提案阶段禁止暴露风险预算参数
            // 原 overall_risk / agg_risk_pos / cons_risk_pos / neut_risk_pos 已移除
            // AgentNode(Json mode) 输出包裹在 {role, content: <json_string>, ...} 中
            ("consensus_score", "debate-convergence.content.consensus_score"),
            ("stock_lessons", "stock_lessons"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        a.config.model_role = Some("decision-maker".into());
        a.config.tools = vec![
            td_score.clone(),
            td_val.clone(),
            td_risk.clone(),
            td_maxdd.clone(),
            td_sharpe.clone(),
            td_var.clone(),
            td_pe_pct.clone(),
            td_peg.clone(),
            td_kelly.clone(),
            td_rp.clone(),
            td_corr.clone(),
            td_ind.clone(),
        ];
        a.config.system_prompt =
            format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
        a.config.max_tool_rounds = Some(3);
        // exposed_tools 排除已由 t-scoring/t-valuation/t-risk 注入的算法工具
        a.config.exposed_tools = a
            .config
            .tools
            .iter()
            .map(|td| td.name.clone())
            .filter(|n| {
                n != "compute_scoring" && n != "compute_valuation" && n != "compute_portfolio_risk"
            })
            .collect();
    }
    nodes.push(rm);
    edges.push(edge("e-value-investor-research-mgr", "value-investor", "research-mgr"));
    edges.push(edge("e-v-validate-research-mgr", "v-validate", "research-mgr"));

    // trader: 执行方案 — 实时行情 + 技术指标 + 凯利仓位
    let mut trader = agent(
        "trader",
        "制定A股交易方案：入场价、目标价、止损价、仓位比例。遵守T+1和涨跌停规则",
        "trader",
        None,
        240.0,
        3900.0,
    );
    if let WorkflowNode::Agent(ref mut a) = trader {
        // P2 修复: 扩展 context_sources 覆盖所有 input_mapping 引用的上游节点
        // （显式依赖原则：input_mapping 引用的上游节点必须有关联边或 context_sources）
        // t-scoring: factor_weights 因子权重 | risk-convergence: risk_disagreement 风险分歧度
        // data-quality: dqi_score 数据质量
        a.config.context_sources = vec![
            "research-mgr".into(),
            "debate-convergence".into(),
            "t-scoring".into(),
            "risk-convergence".into(),
            "data-quality".into(),
        ];
        a.config.model_role = Some("trader".into());
        a.config.output_mode = OutputMode::Json;
        a.config.tools = vec![
            td_quote.clone(),
            td_kline.clone(),
            td_mf.clone(),
            td_score.clone(),
            td_atr.clone(),
            td_ma_cross.clone(),
            td_breakout.clone(),
            td_kelly.clone(),
            td_mc.clone(),
            td_lup.clone(),
        ];
        a.config.system_prompt =
            format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
        a.config.max_tool_rounds = Some(3);
        a.config.input_mapping = [
            ("consensus_score", "debate-convergence.content.consensus_score"),
            ("stock_lessons", "stock_lessons"),
            // P1 修复: 注入标准参考价，确保 trader 与 portfolio-mgr 使用相同的 currentPrice
            // 避免 trader 自行调用 get_stock_quote 获取的实时价与 t-scoring 缓存的 currentPrice
            // 不一致导致的系统性分歧。
            ("reference_price", "t-scoring.result.currentPrice"),
            // P2 修复: 注入因子权重，使 trader 知道哪些因子在公式中权重更高
            // factor_weights 是 JSON 对象 {trend:{weight}, macd:{weight}, ...}
            ("factor_weights", "t-scoring.result.factor_backtest.factors"),
            // P2 修复: 注入风险分歧度，使 trader 知道三位风险评估师的分歧程度
            // 分歧高(>50)时 trader 应避免过度自信
            ("risk_disagreement", "risk-convergence.content.disagreement_score"),
            // P2 修复: 注入数据质量评分，使 trader 知道当前数据覆盖度
            // dqi_score 0-100，低分时 trader 应保守操作
            ("dqi_score", "data-quality.content.score"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    }
    nodes.push(trader);
    edges.push(edge("e-research-mgr-trader", "research-mgr", "trader"));
    // P2 修复: 为 trader 新增的 input_mapping 入口加显式边
    // t-scoring → trader: 因子权重和参考价
    edges.push(edge("e-t-scoring-trader-p2", "t-scoring", "trader"));
    // risk-convergence → trader: 风险分歧度
    edges.push(edge("e-risk-convergence-trader-p2", "risk-convergence", "trader"));
    // data-quality → trader: 数据质量评分
    edges.push(edge("e-data-quality-trader-p2", "data-quality", "trader"));

    // portfolio-mgr: 最终决策 — 确定性计算（CodeNode + Rhai）
    // ── 结构化参数方案 Phase 3 ──
    // 原为 Agent 节点（LLM 执行公式），现改为 CodeNode（Rhai 确定性执行）。
    //
    // 公式逻辑（与 portfolio-manager prompt 保持一致）：
    //   confidence = clamp(totalScore + adjustment, 0, 100)
    //   adjustment = 共识调整 + 数据质量调整 + 风险调整 + 催化剂加成 + 机构加成
    let pm_code = include_str!("../portfolio-mgr.rhai").to_string();
    let pm = WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "portfolio-mgr".into(),
            title: "投资组合经理（确定性决策）".into(),
            description: Some("基于结构化参数，用确定性公式计算最终决策".into()),
            position: Position {
                x: 240.0,
                y: 4200.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: pm_code,
            output_var: "portfolio-mgr".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: [
                // ToolNode 输出包裹在 {tool_name, result: <json_string>, ...} 中
                ("totalScore", "t-scoring.result.totalScore"),
                // AgentNode 输出包裹在 {role, content: <json_string>, ...} 中
                // V29 修复: data-quality 是 AgentNode，无 .result 字段，必须走 .content.
                ("dqi_score", "data-quality.content.score"),
                // P1/P2: 因子回测数据（compute_scoring 工具附加输出）
                ("factor_weights", "t-scoring.result.factor_backtest.factors"),
                ("market_regime_prior", "t-scoring.result.market_regime.prior"),
                ("market_regime_state", "t-scoring.result.market_regime.state"),
                // LlmClassifierNode 输出 {category, model, ...}
                ("overall_risk", "risk-level.category"),
                // AgentNode(Json mode) 输出包裹在 {role, content: <json_string>, ...} 中
                ("catalyst_level", "a-catalyst.content.catalyst_level"),
                ("consensusScore", "debate-convergence.content.consensus_score"),
                // trader Json 模式输出：{action, targetPrice, stopLoss, ...}
                ("trader_action", "trader.content.action"),
                // currentPrice: 从 t-scoring 工具节点（get_stock_quote）获取，可靠数据源。
                // 不用 trader.content.currentPrice，因为 LLM 不一定输出该字段。
                ("current_price", "t-scoring.result.currentPrice"),
                ("trader_target_price", "trader.content.targetPrice"),
                ("trader_stop_loss", "trader.content.stopLoss"),
                ("trader_time_horizon", "trader.content.timeHorizon"),
                ("trader_holding_days", "trader.content.expectedHoldingDays"),
                // V50 修复: 接入 risk-convergence 的三方风险分歧度
                // 避免该 LLM 节点（约5-10s）的输出被浪费
                ("risk_disagreement", "risk-convergence.content.disagreement_score"),
                // V51 新增: 估值因子数据源
                // t-valuation 输出 DCF/格雷厄姆上行空间，用于 f5_signal 估值因子
                ("valuation_dcf_upside", "t-valuation.result.dcf.upsidePct"),
                ("valuation_graham_upside", "t-valuation.result.graham.upsidePct"),
                ("valuation_fscore", "t-valuation.result.fScore.score"),
                ("valuation_moat", "t-valuation.result.moat.label"),
                // V52 新增: t-risk 算法风险分类数据源
                // 用确定性算法替代 cls-risk-level 的 LLM 分类器（消除 LLM 不一致性）
                // t-risk 是 ToolNode, stockRiskProfile 在 result 中
                ("risk_volatility", "t-risk.result.stockRiskProfile.annualizedVolatilityPct"),
                ("risk_drawdown", "t-risk.result.stockRiskProfile.maxDrawdownPct"),
                ("risk_sharpe", "t-risk.result.stockRiskProfile.sharpeRatio"),
                ("risk_roe", "t-risk.result.stockRiskProfile.roeTTMPct"),
                ("risk_gross_margin", "t-risk.result.stockRiskProfile.grossMarginPct"),
                ("risk_debt_ratio", "t-risk.result.stockRiskProfile.debtRatioPct"),
                ("risk_revenue_growth", "t-risk.result.stockRiskProfile.revenueGrowthYoYPct"),
                ("risk_pe", "t-risk.result.stockRiskProfile.peTTM"),
                // V53 修复: 从瓶颈掘金工作流传入的上下文标记
                // 告诉 portfolio-mgr"当前分析的股票来自 Serenity 筛选",
                // 允许风险分类器对瓶颈股特征（高波动/扩张期）做评分修正
                ("screening_source", "screening_source"),
                // X1 桥接: Serenity 瓶颈分析上下文（serenity_score / bottleneck_product 等）
                // 由 core.rs 在 screening_source=serenity 时注入为工作流变量
                ("serenity_context", "serenity_context"),
                // ── P1 新增: 资金面因子 f9 数据源 ──
                // t-hotmoney-data 输出 get_stock_money_flow 的 JSON 字符串
                // Rhai 中用 json_parse() 解析后提取主力净流入占比
                ("money_flow", "t-hotmoney-data.result"),
                // ── P1 新增: 筹码面因子 f10 数据源 ──
                // t-lockup-data 输出 get_stock_lockup_bundle 的 JSON 字符串
                // 含解禁/增减持/大宗交易三方信息
                ("lockup_bundle", "t-lockup-data.result"),
                // ── P2 新增: 龙虎榜数据源（f10 筹码面增强）──
                // t-dragon-tiger-data 输出 get_stock_dragon_tiger 的 JSON 字符串
                // 含机构席位买卖、游资动向、上榜原因等
                ("dragon_tiger", "t-dragon-tiger-data.result"),
                // ── P2 新增: 公告风险信号（f3 催化剂增强）──
                // t-catalyst-data 输出 get_stock_announcements 的 JSON 字符串
                // 含公告标题/类型/日期，用于关键词风险检测
                ("announcements", "t-catalyst-data.result"),
                // ── V55 新增: 上游 strict_mode 兜底哨兵 ──
                // 每个 AgentNode 在 strict_mode 降级时会在顶层注入 __untrusted=true。
                // portfolio-mgr.rhai 累加这些哨兵，任意一个为 true 即触发 weights_collapsed
                // 兜底（强制观望+空仓+confidence 对半），避免 LLM 失败的 50/50 兜底
                // 被当成有效信号继续融合。
                ("untrusted_trader", "trader.__untrusted"),
                ("untrusted_research_mgr", "research-mgr.__untrusted"),
                ("untrusted_catalyst", "a-catalyst.__untrusted"),
                ("untrusted_debate_conv", "debate-convergence.__untrusted"),
                ("untrusted_data_quality", "data-quality.__untrusted"),
                ("untrusted_risk_conv", "risk-convergence.__untrusted"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        },
    });
    nodes.push(pm);
    edges.push(edge("e-trader-portfolio-mgr", "trader", "portfolio-mgr"));
    edges.push(edge("e-research-mgr-portfolio-mgr", "research-mgr", "portfolio-mgr"));
    // debate-convergence → portfolio-mgr: 显式边确保 consensus_score 在公式执行前就绪
    edges.push(edge(
        "e-debate-convergence-portfolio-mgr",
        "debate-convergence",
        "portfolio-mgr",
    ));
    // V29 修复: debate-convergence → research-mgr / trader 显式边
    // research-mgr 和 trader 的 input_mapping 引用 debate-convergence.content.consensus_score，
    // 加显式边确保共识分数在节点执行前就绪（符合显式依赖原则）
    edges.push(edge("e-debate-convergence-research-mgr", "debate-convergence", "research-mgr"));
    edges.push(edge("e-debate-convergence-trader", "debate-convergence", "trader"));
    // data-quality → portfolio-mgr: 显式边确保 dqi_score 在 Rhai 公式执行前就绪
    edges.push(edge("e-data-quality-portfolio-mgr", "data-quality", "portfolio-mgr"));
    // V50 修复: risk-convergence → portfolio-mgr 显式边
    // risk-convergence 的三方分歧度(disagreement_score)已被加入 input_mapping，
    // 需要显式边确保在执行 portfolio-mgr 前就绪
    edges.push(edge("e-risk-convergence-portfolio-mgr", "risk-convergence", "portfolio-mgr"));
    // V52: t-risk → portfolio-mgr 显式边
    // portfolio-mgr 需要 t-risk.stockRiskProfile 数据做算法风险分类
    edges.push(edge("e-t-risk-portfolio-mgr", "t-risk", "portfolio-mgr"));
    // ── P1 新增: f9 资金面因子数据源 → portfolio-mgr 显式边 ──
    // t-hotmoney-data 输出资金流向数据，portfolio-mgr 的 input_mapping 引用
    // "money_flow" → "t-hotmoney-data.result"，需要显式边确保调度顺序
    edges.push(edge("e-t-hotmoney-data-portfolio-mgr", "t-hotmoney-data", "portfolio-mgr"));
    // ── P1 新增: f10 筹码面因子数据源 → portfolio-mgr 显式边 ──
    // t-lockup-data 输出解禁/增减持/大宗交易数据，portfolio-mgr 的 input_mapping 引用
    // "lockup_bundle" → "t-lockup-data.result"，需要显式边确保调度顺序
    edges.push(edge("e-t-lockup-data-portfolio-mgr", "t-lockup-data", "portfolio-mgr"));
    // ── P2 新增: 龙虎榜数据源 → portfolio-mgr 显式边 ──
    edges.push(edge(
        "e-t-dragon-tiger-data-portfolio-mgr",
        "t-dragon-tiger-data",
        "portfolio-mgr",
    ));
    // ── P2 新增: 公告数据源 → portfolio-mgr 显式边 ──
    // t-catalyst-data 输出公司公告列表，用于公告关键词风险检测
    edges.push(edge("e-t-catalyst-data-portfolio-mgr", "t-catalyst-data", "portfolio-mgr"));

    // ── P3 (real-nodes): rule-check 规则检查 Agent ──
    // 在 portfolio-mgr 完成后启动，对照硬性规则阈值（RSI/乖离率/止损/放量下跌/空头排列）
    // 检查交易方案是否违规，输出 violations / corrections / force_signals
    {
        let rc_id = "rule-check";
        let rc_title = "硬性规则检查：RSI超买/乖离率追高/缺失止损/放量下跌/空头排列";
        let rc_y = 4200.0;
        let mut rc = agent(rc_id, rc_title, "rule-checker", None, 700.0, rc_y);
        if let WorkflowNode::Agent(ref mut a) = rc {
            a.config.context_sources = vec![
                "portfolio-mgr".into(),
                "t-scoring".into(),
                "t-valuation".into(),
                "t-risk".into(),
                "trader".into(),
            ];
            a.config.model_role = Some("risk-evaluator".into());
            let tool_names = PROFILE_TOOLS
                .iter()
                .find(|(k, _)| *k == "rule-checker")
                .map(|(_, v)| *v)
                .unwrap_or(&[]);
            a.config.tools = tool_names
                .iter()
                .filter_map(|&tn| tool_def_map.get(tn).cloned())
                .collect();
            a.config.exposed_tools = tool_names.iter().map(|&tn| tn.to_string()).collect();
            a.config.system_prompt =
                format!("{}{}", a.config.system_prompt, tool_prompt(&a.config.tools));
        }
        nodes.push(rc);
        // ── NotificationNode 由 rule-check 完成后触发（不再保留 portfolio-mgr → notify
        // 直连，避免通知在规则检查改写决策之前发出）──
        edges.push(edge("e-portfolio-mgr-rule-check", "portfolio-mgr", rc_id));
        edges.push(edge("e-rule-check-quality-gate", rc_id, "quality-gate"));
        // data-quality → quality-gate: 显式边确保 data-quality 变量在 switch 判断前就绪
        edges.push(edge("e-data-quality-quality-gate", "data-quality", "quality-gate"));
    }

    // ── SwitchNode: 数据质量门禁 ──
    // 检查 data-quality Agent 的 JSON 输出中的 grade 字段（A/B/C/D/F），C 级以上继续，D/F 走降级路径。
    // data-quality 输出为 JSON 格式，resolve_var_path 导航到 content.grade 提取等级。
    nodes.push(WorkflowNode::Switch(SwitchNode {
        base: WorkflowNodeBase {
            id: "quality-gate".into(),
            title: "数据质量门禁".into(),
            description: Some("检查数据质量等级，A/B/C 级以上继续，D/F 走保守降级路径".into()),
            position: Position {
                x: 700.0,
                y: 4500.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(10),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: SwitchNodeConfig {
            // V36 修复: data-quality 已改为 JSON 模式，输出包裹在 {role, content: <json_string>} 中，
            // resolve_var_path 自动解析 JSON 字符串后导航到 grade 字段。
            // 不能用 .params.grade — params 不是 AgentNode 输出的顶层字段。
            input_var: "data-quality.content.grade".into(),
            cases: vec![SwitchCase {
                value: "_value == \"A\" || _value == \"B\" || _value == \"C\"".into(),
                label: "acceptable".into(),
            }],
            default_case: Some("low-quality".into()),
            match_mode: "expression".into(),
            use_llm: None,
            llm_prompt: None,
            llm_model: None,
            output_var: "quality-gate-result".into(),
        },
    }));

    // ── Agent: 降级处理路径（数据质量不足时生成保守决策）──
    {
        let fq_id = "quality-fallback";
        let fq_title = "数据不足→保守决策：持仓不变/减仓观望";
        let fq_y = 4500.0;
        let mut fq = agent(fq_id, fq_title, "quality-fallback", None, 20.0, fq_y);
        if let WorkflowNode::Agent(ref mut a) = fq {
            a.config.context_sources = vec![
                "rule-check".into(),
                "data-quality".into(),
                "t-scoring".into(),
                "t-valuation".into(),
                "t-risk".into(),
            ];
            a.config.output_mode = OutputMode::Json;
            a.config.model_role = Some("decision-maker".into());
            a.config.tools = vec![td_quote.clone(), td_kline.clone(), td_score.clone()];
            a.config.system_prompt =
                "数据质量评估为 D 或 F，上游分析数据不可靠。你需要在数据不足的情况下做出最保守的投资决策。\
                 输出JSON格式（严格模式）：{\"action\":\"持有/减持/卖出\",\"positionPct\":0-20,\"reasoning\":\"保守决策理由\"}}\
                 只输出上述JSON对象，前后不要有任何其他文字"
                    .to_string();
            a.config.exposed_tools = vec![
                "get_stock_quote".into(),
                "get_stock_kline".into(),
                "compute_scoring".into(),
            ];
            a.config.max_tool_rounds = Some(1);
        }
        nodes.push(fq);
        // Switch 出边：
        //   case "acceptable" → notify-result（source_handle = 匹配的 case label）
        //   default → quality-fallback（无 source_handle）
        edges.push(WorkflowEdge {
            id: "e-quality-gate-notify".into(),
            source: "quality-gate".into(),
            source_handle: Some("acceptable".into()),
            target: "notify-result".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: Some("通过 ✓".into()),
        });
        edges.push(WorkflowEdge {
            id: "e-quality-gate-quality-fallback".into(),
            source: "quality-gate".into(),
            source_handle: None,
            target: fq_id.into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: Some("降级 →".into()),
        });
    }
    // quality-fallback 降级完成后同样触发 explainer
    edges.push(edge("e-quality-fallback-explainer", "quality-fallback", "decision-explainer"));

    // ── P0 补: decision-explainer（三明治第三段）──
    // 在 portfolio-mgr 硬裁决完成后，用 LLM 生成决策依据说明书 + 规则追溯码
    // 输入: portfolio-mgr 的 final_action / confidence / reasoning / decision_trail
    // 输出: 自然语言解释文案，带规则追溯码 R-xxx
    {
        let de_id = "decision-explainer";
        let de_title = "决策解释：将硬裁决结果翻译为自然语言说明书，附带规则追溯码";
        let mut de = agent(de_id, de_title, "explainer", None, 700.0, 4400.0);
        if let WorkflowNode::Agent(ref mut a) = de {
            a.config.context_sources = vec![
                "portfolio-mgr".into(),
                "rule-check".into(),
                "t-scoring".into(),
                "t-risk".into(),
            ];
            a.config.model_role = Some("explainer".into());
            a.config.output_mode = OutputMode::Json;
            a.config.tools = vec![td_quote.clone(), td_score.clone()];
            a.config.exposed_tools = vec!["get_stock_quote".into(), "compute_scoring".into()];
            a.config.max_tool_rounds = Some(1);
            a.config.system_prompt = format!(
                "{}\n{}",
                "你是投资决策解释官。输入是符号系统（Rhai 规则引擎）的硬裁决结果，你的任务是将裁决翻译为用户可读的决策依据说明书。",
                "输出 JSON 格式（严格模式）：\n\
                 {{\n\
                   \"summary\": \"一段话摘要（50-100字），包含最终行动、仓位、置信度\",\n\
                   \"explanation\": \"详细的决策依据说明（200-300字），解释为什么做出这个决策\",\n\
                   \"rule_trace\": [\n\
                     {{\"rule_id\": \"R-xxx\", \"status\": \"PASS/VETOED/DOWNGRADED\", \"description\": \"规则的通俗解释\"}}\n\
                   ],\n\
                   \"risk_comment\": \"风险提示（如有）\",\n\
                   \"confidence_note\": \"置信度解读\"\n\
                 }}\n\
                 规则追溯码对照表：\n\
                 R-200: 高风险风控否决 | R-201: 空头预测否决 | R-202: trader数据异常\n\
                 R-203: 因子权重坍缩 | R-204: 零仓位修正 | R-205: 单点不可信部分降级\n\
                 R-401: RSI>80追高风险否决 | R-402: RSI<20恐慌否决\n\
                 R-403: MACD顶背离否决 | R-404: MACD底背离否决 | R-405: RSI+MACD双重超买\n\
                 只输出上述JSON对象，前后不要有任何其他文字"
            );
            a.config.input_mapping = [
                ("pm_action", "portfolio-mgr.action"),
                ("pm_confidence", "portfolio-mgr.confidence"),
                ("pm_position_pct", "portfolio-mgr.positionPct"),
                ("pm_reasoning", "portfolio-mgr.reasoning"),
                ("pm_risk_level", "portfolio-mgr.riskLevel"),
                ("pm_stop_loss", "portfolio-mgr.stopLossPct"),
                ("pm_take_profit", "portfolio-mgr.takeProfitPct"),
                ("pm_decision_trail", "portfolio-mgr.decision_trail"),
                ("pm_target_timeframe", "portfolio-mgr.targetTimeframe"),
                ("pm_computation_logs", "portfolio-mgr.computation_logs"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        }
        nodes.push(de);
        // quality-gate 的 acceptable 路径 → decision-explainer
        //（覆盖原来的 e-quality-gate-notify，下面重新建边到 notify-result）
    }
    // ── 重定向: quality-gate acceptable → decision-explainer ──
    // 替换步骤 1: 注册新边 (source_handle="acceptable" → decision-explainer)
    edges.push(WorkflowEdge {
        id: "e-quality-gate-explainer".into(),
        source: "quality-gate".into(),
        source_handle: Some("acceptable".into()),
        target: "decision-explainer".into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: Some("通过 ✓→解释".into()),
    });
    // 替换步骤 2: 删除旧边 e-quality-gate-notify（遍历时过滤掉）
    edges.retain(|e| e.id != "e-quality-gate-notify");
    // explainer 完成后通知 + 持久化
    edges.push(edge("e-explainer-notify", "decision-explainer", "notify-result"));
    edges.push(edge("e-explainer-store", "decision-explainer", "store-result"));

    // ── NotificationNode: 分析完成通知 ──
    nodes.push(WorkflowNode::Notification(NotificationNode {
        base: WorkflowNodeBase {
            id: "notify-result".into(),
            title: "分析完成通知".into(),
            description: Some("股票分析完成后发送通知".into()),
            position: Position {
                x: 300.0,
                y: 4500.0,
            },
            retry: RetryConfig::default(),
            timeout: Some(10),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: NotificationNodeConfig {
            channel: "system".into(),
            message: "股票分析已完成，请查看决策结果".into(),
            webhook_url: None,
            recipients: vec![],
            subject: Some("股票分析完成".into()),
            enabled: true,
            output_var: "notification".into(),
        },
    }));
    // 注：移除 e-portfolio-mgr-notify 直连，notify-result 现在仅由 rule-check 完成后触发

    // ── StorageNode: 分析结果持久化 ──
    // 将完整分析结果（portfolio-mgr 决策）写入 SQLite history 表，供后续回测/复盘引用。
    nodes.push(WorkflowNode::Storage(StorageNode {
        base: WorkflowNodeBase {
            id: "store-result".into(),
            title: "分析结果持久化".into(),
            description: Some("写入分析结果到历史记录表".into()),
            position: Position {
                x: 300.0,
                y: 4800.0,
            },
            retry: RetryConfig {
                enabled: true,
                max_retries: 2,
                ..Default::default()
            },
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: StorageNodeConfig {
            backend: "sqlite".into(),
            operation: "insert".into(),
            input_var: "portfolio-mgr".into(),
            collection: "analysis_history".into(),
            key_var: None,
            output_var: "storage-result".into(),
        },
    }));
    edges.push(edge("e-notify-store-result", "notify-result", "store-result"));
    // store-result 直接从 portfolio-mgr 取决策变量，绕过 state.variables 查找
    edges.push(edge("e-portfolio-mgr-store-result", "portfolio-mgr", "store-result"));

    // EndNode: 把 portfolio-mgr 输出提升为工作流顶层输出
    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end-output".into(),
            title: "最终输出".into(),
            description: Some("将 portfolio-mgr 决策结果提升到工作流输出".into()),
            position: Position {
                x: 300.0,
                y: 5100.0,
            },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: EndNodeConfig {
            output_var: Some("portfolio-mgr".into()),
        },
    }));
    edges.push(edge("e-store-end", "store-result", "end-output"));

    // 构建 input_schema / output_schema / variables
    let mut input_props = std::collections::HashMap::new();
    input_props.insert(
        "stock_code".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("股票代码，如 000001、600519".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    let input_schema_val = serde_json::to_string(&JsonSchema {
        schema_type: "object".to_string(),
        description: Some("股票分析运行时输入".to_string()),
        properties: Some(input_props),
        required: Some(vec!["stock_code".to_string()]),
        items: None,
    })
    .unwrap();

    let mut output_props = std::collections::HashMap::new();
    output_props.insert(
        "action".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("投资决策: 买入/增持/持有/减持/卖出".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "positionPct".to_string(),
        JsonSchemaProperty {
            schema_type: "number".to_string(),
            description: Some("建议仓位百分比 (0-100)".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "targetPrice".to_string(),
        JsonSchemaProperty {
            schema_type: "number".to_string(),
            description: Some("目标价".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "stopLoss".to_string(),
        JsonSchemaProperty {
            schema_type: "number".to_string(),
            description: Some("止损价".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "reasoning".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("决策理由 (300字以内)".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "riskLevel".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("风险等级: 低/中/高".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    output_props.insert(
        "confidence".to_string(),
        JsonSchemaProperty {
            schema_type: "number".to_string(),
            description: Some("置信度 (0-100)".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    let output_schema_val = serde_json::to_string(&JsonSchema {
        schema_type: "object".to_string(),
        description: Some("股票分析最终决策输出".to_string()),
        properties: Some(output_props),
        required: None,
        items: None,
    })
    .unwrap();

    let variables: Vec<Variable> = vec![
        // ── 分析流程参数 ──
        Variable {
            name: "analysis_depth".into(),
            var_type: "enum".into(),
            value: serde_json::json!("standard"),
            description: Some("分析深度: quick / standard / deep".into()),
            is_secret: false,
        },
        Variable {
            name: "debate_rounds".into(),
            var_type: "number".into(),
            // 与 seed 时使用的常量保持一致（seed 函数里硬编码 3）。
            // 用户在「股票分析设置 → 参数」里调成 6 后，下次模板升级会
            // 用 merge_variable_values 保留用户的 6，并据此展开 DAG。
            value: serde_json::json!(3),
            description: Some("多空辩论轮数 (1-10)".into()),
            is_secret: false,
        },
        Variable {
            name: "screening_source".into(),
            var_type: "string".into(),
            value: serde_json::json!(""),
            description: Some("筛选来源标记：serenity(瓶颈掘金) / ''(直接分析)".into()),
            is_secret: false,
        },
        Variable {
            name: "max_concurrent".into(),
            var_type: "number".into(),
            // v8.1: 从 12 降至 5，避免 10 个 Agent 同批次打满 LLM provider 并发限流。
            // 之前 12 导致 001313 等小盘股分析卡死（所有新闻源全空 → LLM 响应极慢
            // → stream.next() 无内部超时 → JoinSet 阻塞整个引擎 5 分钟）。
            value: serde_json::json!(5),
            description: Some("并行分析的 Agent 数量上限".into()),
            is_secret: false,
        },
        // ── 数据源参数 ──
        Variable {
            name: "kline_period".into(),
            var_type: "enum".into(),
            value: serde_json::json!("daily"),
            description: Some("K线周期: daily / weekly / monthly".into()),
            is_secret: false,
        },
        Variable {
            name: "kline_limit".into(),
            var_type: "number".into(),
            value: serde_json::json!(120),
            description: Some("K线获取根数 (1-500)".into()),
            is_secret: false,
        },
        Variable {
            name: "news_limit".into(),
            var_type: "number".into(),
            value: serde_json::json!(30),
            description: Some("新闻获取条数 (1-100)".into()),
            is_secret: false,
        },
        // ── Agent 节点 LLM 参数 ──
        Variable {
            name: "agent_temperature".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.3),
            description: Some("所有 Agent 节点 LLM 温度 (0-2)".into()),
            is_secret: false,
        },
        Variable {
            name: "agent_max_tokens".into(),
            var_type: "number".into(),
            value: serde_json::json!(4096),
            description: Some("所有 Agent 节点最大输出 token 数".into()),
            is_secret: false,
        },
        Variable {
            name: "agent_timeout_secs".into(),
            var_type: "number".into(),
            // v8.1: 从 300s 降至 120s，配合 max_concurrent=5，单 Agent 最多等 2 分钟。
            // 之前 300s 在 10 个 Agent 同批次场景下，任一挂起即阻塞引擎 5 分钟。
            value: serde_json::json!(120),
            description: Some("每个 Agent 节点执行超时秒数 (v8.1: 120s)".into()),
            is_secret: false,
        },
        Variable {
            name: "agent_retry_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(2),
            description: Some("每个 Agent 节点最大重试次数".into()),
            is_secret: false,
        },
        // ── Tool 节点参数 ──
        Variable {
            name: "tool_timeout_secs".into(),
            var_type: "number".into(),
            value: serde_json::json!(30),
            description: Some("每个 Tool 节点执行超时秒数".into()),
            is_secret: false,
        },
        Variable {
            name: "tool_retry_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(2),
            description: Some("每个 Tool 节点最大重试次数".into()),
            is_secret: false,
        },
        // ── 评分权重 (ScoringWeights) ──
        Variable {
            name: "scoring_trend".into(),
            var_type: "number".into(),
            value: serde_json::json!(30.0),
            description: Some("趋势评分权重 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "scoring_deviation".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("偏离度评分权重 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "scoring_macd".into(),
            var_type: "number".into(),
            value: serde_json::json!(15.0),
            description: Some("MACD 评分权重 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "scoring_volume".into(),
            var_type: "number".into(),
            value: serde_json::json!(15.0),
            description: Some("成交量评分权重 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "scoring_rsi".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("RSI 评分权重 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "scoring_support".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("支撑阻力评分权重 (0-100)".into()),
            is_secret: false,
        },
        // 补全：decision.rs:75 的 ScoringWeights 里有这个字段，但模板里漏了种子化
        Variable {
            name: "scoring_boll".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("布林带评分权重 (0-100)".into()),
            is_secret: false,
        },
        // ── 规则引擎阈值 (RuleConfig) ──
        Variable {
            name: "rule_rsi_overbought".into(),
            var_type: "number".into(),
            value: serde_json::json!(80.0),
            description: Some("RSI 超买阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "rule_rsi_oversold".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("RSI 超卖阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "rule_bias_limit_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("均线偏离极限 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "rule_volume_signal_block".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("成交量异常时是否阻塞信号".into()),
            is_secret: false,
        },
        Variable {
            name: "rule_bear_low_score".into(),
            var_type: "number".into(),
            value: serde_json::json!(30),
            description: Some("空方低分阈值 (低于此分数触发警告)".into()),
            is_secret: false,
        },
        Variable {
            name: "rule_auto_stop_loss_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("自动止损线 (%)".into()),
            is_secret: false,
        },
        // ── 仓位限制 (PositionLimitsConfig) ──
        Variable {
            name: "pos_max_single_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("单只股票最大仓位占比 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "pos_max_total".into(),
            var_type: "number".into(),
            value: serde_json::json!(10),
            description: Some("最大持仓数量".into()),
            is_secret: false,
        },
        Variable {
            name: "pos_max_sector_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(40.0),
            description: Some("最大行业暴露占比 (%)".into()),
            is_secret: false,
        },
        // ── 估值参数 (ValueConfig) ──
        Variable {
            name: "value_dcf_growth_rate".into(),
            var_type: "number".into(),
            value: serde_json::json!(8.0),
            description: Some("DCF 增长率 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "value_dcf_perpetual_rate".into(),
            var_type: "number".into(),
            value: serde_json::json!(3.0),
            description: Some("DCF 永续增长率 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "value_dcf_discount_rate".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("DCF 折现率 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "value_moat_threshold".into(),
            var_type: "number".into(),
            value: serde_json::json!(60),
            description: Some("护城河评分阈值 (0-100)".into()),
            is_secret: false,
        },
        Variable {
            name: "value_fscore_buy".into(),
            var_type: "number".into(),
            value: serde_json::json!(7),
            description: Some("F-Score 买入阈值 (0-9)".into()),
            is_secret: false,
        },
        Variable {
            name: "value_safety_margin".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("安全边际最低折扣 (%)".into()),
            is_secret: false,
        },
        // ── 监控参数 (MonitorConfig) ──
        Variable {
            name: "monitor_poll_interval_secs".into(),
            var_type: "number".into(),
            value: serde_json::json!(30),
            description: Some("监控轮询间隔秒数".into()),
            is_secret: false,
        },
        Variable {
            name: "monitor_change_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("价格异动提醒阈值 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "monitor_turnover".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("换手率异动提醒阈值 (%)".into()),
            is_secret: false,
        },
        // ── 置信度参数 ──
        Variable {
            name: "min_confidence".into(),
            var_type: "number".into(),
            value: serde_json::json!(60),
            description: Some("最低置信度阈值 (低于此值建议观望)".into()),
            is_secret: false,
        },
        // ── 数据源 (vendor_ 前缀，健康检查关联) ──
        Variable {
            name: "vendor_tencent".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("腾讯财经 — 报价数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_eastmoney".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("东方财富 — 财务/K线数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_sina".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("新浪财经 — 新闻数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_ths".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("同花顺 — 综合数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_cninfo".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("巨潮资讯 — 信息披露".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_baidu_stock".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("百度股票 — 数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_iwencai".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("问财 — 选股数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_akshare".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("AKShare — 开源数据".into()),
            is_secret: false,
        },
        Variable {
            name: "vendor_mootdx".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("Mootdx — 本地行情接口".into()),
            is_secret: false,
        },
        // ── 金融模型参数（通过 prompt 模板 {{var}} 传入 agent）──
        Variable {
            name: "risk_free_rate".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.03),
            description: Some("无风险利率".into()),
            is_secret: false,
        },
        Variable {
            name: "var_confidence".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.95),
            description: Some("VaR 置信度 (0-1)".into()),
            is_secret: false,
        },
        Variable {
            name: "outlier_method".into(),
            var_type: "enum".into(),
            value: serde_json::json!("zscore"),
            description: Some("异常值检测方法: zscore / iqr".into()),
            is_secret: false,
        },
        Variable {
            name: "outlier_threshold".into(),
            var_type: "number".into(),
            value: serde_json::json!(2.0),
            description: Some("异常值 Z-score 阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "kelly_fraction".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.5),
            description: Some("凯利仓位系数 (建议仓位 = half_kelly × 此系数)".into()),
            is_secret: false,
        },
        // ── A 类补全：凯利前置条件（risk.rs:188-198）──
        Variable {
            name: "kelly_min_win_rate".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.4),
            description: Some("凯利最低胜率要求 (0-1)，低于此值返回不适用".into()),
            is_secret: false,
        },
        Variable {
            name: "kelly_min_odds".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.0),
            description: Some("凯利最低赔率要求 (avg_win/avg_loss)，低于此值降权".into()),
            is_secret: false,
        },
        // ── A 类补全：组合风控（trading.rs:200 / risk.rs）──
        Variable {
            name: "risk_max_drawdown_limit".into(),
            var_type: "number".into(),
            value: serde_json::json!(15.0),
            description: Some("组合最大回撤熔断线 (%)，超过则暂停新开仓".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_max_daily_loss_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(3.0),
            description: Some("单日最大亏损 (%)，超过则停手".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_correlation_lookback_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(60),
            description: Some("风险平价 / 相关性矩阵的回看窗口 (交易日)".into()),
            is_secret: false,
        },
        // ── A 类补全：仓位限制扩展（position_limits.rs）──
        Variable {
            name: "pos_min_cash_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("最低现金比例 (%)，低于则禁止新开仓".into()),
            is_secret: false,
        },
        Variable {
            name: "pos_max_turnover_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(100.0),
            description: Some("单期最大换手率 (%)，超过则分批调仓".into()),
            is_secret: false,
        },
        // ── A 类补全：护城河量化阈值（value.rs:320-434）──
        Variable {
            name: "moat_roe_years_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(3),
            description: Some("ROE>15% 最少连续年数 (0-10)".into()),
            is_secret: false,
        },
        Variable {
            name: "moat_avg_gross_margin_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("平均毛利率下限 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "moat_margin_stable_std_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("毛利率稳定性标准差上限 (σ，%)".into()),
            is_secret: false,
        },
        Variable {
            name: "moat_fcf_ratio_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.5),
            description: Some("FCF/净利润 比率下限 (0-1)".into()),
            is_secret: false,
        },
        // ── A 类补全：选股筛选（screener.rs:8 ScreenCriteria）──
        Variable {
            name: "screener_min_change_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(-30.0),
            description: Some("选股最小涨跌幅下限 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_max_change_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(30.0),
            description: Some("选股最大涨跌幅上限 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_main_inflow_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.0),
            description: Some("主力净流入下限 (万元)，0=不限".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_northbound_ratio_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.0),
            description: Some("北向持仓占比下限 (%)，0=不限".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_turnover_rate_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.0),
            description: Some("换手率下限 (%)，0=不限".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_rsi_oversold".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(false),
            description: Some("选股时要求 RSI 超卖 (<30)".into()),
            is_secret: false,
        },
        Variable {
            name: "screener_rsi_overbought".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(false),
            description: Some("选股时要求 RSI 超买 (>70)".into()),
            is_secret: false,
        },
        // ── A 类补全：信号检测（signals.rs detect_ma_cross / detect_breakout）──
        Variable {
            name: "signal_ma_fast".into(),
            var_type: "number".into(),
            value: serde_json::json!(5),
            description: Some("MA 金叉检测快线周期 (3-30)".into()),
            is_secret: false,
        },
        Variable {
            name: "signal_ma_slow".into(),
            var_type: "number".into(),
            value: serde_json::json!(20),
            description: Some("MA 金叉检测慢线周期 (10-120)".into()),
            is_secret: false,
        },
        Variable {
            name: "signal_breakout_volume_mult".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.5),
            description: Some("突破/破位放量倍数阈值 (1.0-3.0)".into()),
            is_secret: false,
        },
        // ── A 类补全：关键价位（key_levels.rs KeyLevelTracker）──
        Variable {
            name: "keylevel_lookback_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(60),
            description: Some("关键价位回看窗口 (交易日，10-250)".into()),
            is_secret: false,
        },
        Variable {
            name: "keylevel_touch_tolerance_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.0),
            description: Some("关键价位触碰容差 (%，0.1-5.0)".into()),
            is_secret: false,
        },
        Variable {
            name: "keylevel_min_touches".into(),
            var_type: "number".into(),
            value: serde_json::json!(2),
            description: Some("确认支撑/阻力最少触碰次数 (1-10)".into()),
            is_secret: false,
        },
        // ── A 类补全：监控告警（monitor.rs MonitorConfig）──
        Variable {
            name: "monitor_alert_cooldown_secs".into(),
            var_type: "number".into(),
            value: serde_json::json!(300),
            description: Some("同一标的告警冷却时间 (秒，10-3600)".into()),
            is_secret: false,
        },
        Variable {
            name: "monitor_min_severity".into(),
            var_type: "enum".into(),
            value: serde_json::json!("info"),
            description: Some("最低推送告警等级: info / warn / critical".into()),
            is_secret: false,
        },
        Variable {
            name: "monitor_channels".into(),
            var_type: "string".into(),
            value: serde_json::json!("in_app"),
            description: Some("推送渠道，逗号分隔: in_app / lark / email / webhook".into()),
            is_secret: false,
        },
        // ── A 类补全：推荐器策略开关（recommender/strategies）──
        Variable {
            name: "reco_trend_enabled".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("启用趋势跟踪子策略".into()),
            is_secret: false,
        },
        Variable {
            name: "reco_reversion_enabled".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("启用超跌反弹子策略".into()),
            is_secret: false,
        },
        Variable {
            name: "reco_value_enabled".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("启用价值选股子策略".into()),
            is_secret: false,
        },
        Variable {
            name: "reco_capital_enabled".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("启用资金流向子策略".into()),
            is_secret: false,
        },
        Variable {
            name: "reco_watchlist_enabled".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(true),
            description: Some("启用自选股策略".into()),
            is_secret: false,
        },
        Variable {
            name: "reco_min_confidence".into(),
            var_type: "number".into(),
            value: serde_json::json!(60),
            description: Some("推荐器最低置信度 (0-100)，低于此值不入选".into()),
            is_secret: false,
        },
        // ── A 类补全：决策回溯（decision_tracker.rs）──
        Variable {
            name: "decision_max_history_per_stock".into(),
            var_type: "number".into(),
            value: serde_json::json!(50),
            description: Some("每只股票保留的历史决策条数 (10-200)".into()),
            is_secret: false,
        },
        // ── B 类补全：技术指标周期（indicators.rs IndicatorConfig）──
        Variable {
            name: "macd_fast".into(),
            var_type: "number".into(),
            value: serde_json::json!(12),
            description: Some("MACD 快线周期 (5-30)".into()),
            is_secret: false,
        },
        Variable {
            name: "macd_slow".into(),
            var_type: "number".into(),
            value: serde_json::json!(26),
            description: Some("MACD 慢线周期 (10-60)".into()),
            is_secret: false,
        },
        Variable {
            name: "macd_signal".into(),
            var_type: "number".into(),
            value: serde_json::json!(9),
            description: Some("MACD 信号线周期 (3-20)".into()),
            is_secret: false,
        },
        Variable {
            name: "boll_period".into(),
            var_type: "number".into(),
            value: serde_json::json!(20),
            description: Some("布林带周期 (10-50)".into()),
            is_secret: false,
        },
        Variable {
            name: "boll_stddev".into(),
            var_type: "number".into(),
            value: serde_json::json!(2.0),
            description: Some("布林带标准差倍数 (1.0-3.0)".into()),
            is_secret: false,
        },
        Variable {
            name: "volume_lookback".into(),
            var_type: "number".into(),
            value: serde_json::json!(5),
            description: Some("均量计算回看周期 (3-30，交易日)".into()),
            is_secret: false,
        },
        Variable {
            name: "volume_surge_ratio".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.5),
            description: Some("放量阈值 (量比 > 此值判为放量)".into()),
            is_secret: false,
        },
        Variable {
            name: "volume_shrink_ratio".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.7),
            description: Some("缩量阈值 (量比 < 此值判为缩量)".into()),
            is_secret: false,
        },
        // ── B 类补全：推荐器参数（recommender/strategies）──
        Variable {
            name: "trend_kline_limit".into(),
            var_type: "number".into(),
            value: serde_json::json!(250),
            description: Some("趋势策略读取 K 线上限".into()),
            is_secret: false,
        },
        Variable {
            name: "trend_amount_ratio_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.8),
            description: Some("趋势策略最低量比".into()),
            is_secret: false,
        },
        Variable {
            name: "rev_rsi_short_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(35.0),
            description: Some("超跌反弹短线 RSI 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "rev_drawdown_min_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("超跌反弹中线最低回撤 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "rev_rsi_monthly_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(50.0),
            description: Some("超跌反弹月线 RSI 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pe_short_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(50.0),
            description: Some("价值策略短线 PE 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pe_mid_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(40.0),
            description: Some("价值策略中线 PE 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pb_mid_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(8.0),
            description: Some("价值策略中线 PB 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "cap_inflow_short_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(200.0),
            description: Some("资金策略短线主力净流入下限 (万元)".into()),
            is_secret: false,
        },
        Variable {
            name: "cap_inflow_mid_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(500.0),
            description: Some("资金策略中线主力净流入下限 (万元)".into()),
            is_secret: false,
        },
        Variable {
            name: "cap_turnover_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(2.0),
            description: Some("资金策略最低换手率 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "cap_nb_ratio_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.3),
            description: Some("资金策略北向持仓占比下限 (%)".into()),
            is_secret: false,
        },
        // ── B 类补全：交易决策（trading.rs）──
        Variable {
            name: "trading_price_deviation_limit".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("交易价偏离分析目标价最大容忍度 (%)".into()),
            is_secret: false,
        },
        // ── B 类补全：风险模型（risk.rs）──
        Variable {
            name: "risk_sharpe_annualization".into(),
            var_type: "number".into(),
            value: serde_json::json!(252),
            description: Some("夏普比率年化因子（252=日频，12=月频，4=季频，1=年频）".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_kelly_heavy_threshold".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.25),
            description: Some("凯利公式重仓阈值（>此值判为重仓）".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_kelly_medium_threshold".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.1),
            description: Some("凯利公式中仓阈值（>此值判为中仓）".into()),
            is_secret: false,
        },
        // ── B 类补全：compute_scoring / compute_valuation 工具内部参数 ──
        Variable {
            name: "fscore_roe_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.10),
            description: Some("F-Score ROE 最低要求 (小数)".into()),
            is_secret: false,
        },
        Variable {
            name: "fscore_gross_margin_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.30),
            description: Some("F-Score 毛利率最低要求 (小数)".into()),
            is_secret: false,
        },
        Variable {
            name: "fscore_net_margin_min".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.10),
            description: Some("F-Score 净利率最低要求 (小数)".into()),
            is_secret: false,
        },
        Variable {
            name: "fscore_debt_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.60),
            description: Some("F-Score 负债率上限 (小数)".into()),
            is_secret: false,
        },
        Variable {
            name: "fscore_pe_max".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("F-Score PE 上限".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pe_low".into(),
            var_type: "number".into(),
            value: serde_json::json!(15.0),
            description: Some("基本面修正 PE 低估阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pe_high".into(),
            var_type: "number".into(),
            value: serde_json::json!(50.0),
            description: Some("基本面修正 PE 高估阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pb_low".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.0),
            description: Some("基本面修正 PB 低估阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "val_pb_high".into(),
            var_type: "number".into(),
            value: serde_json::json!(6.0),
            description: Some("基本面修正 PB 高估阈值".into()),
            is_secret: false,
        },
        // ── B 类补全：组合风控 compute_portfolio_risk ──
        Variable {
            name: "risk_hhi_concentrated".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.25),
            description: Some("组合 HHI 高度集中阈值 (0-1)".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_hhi_medium".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.15),
            description: Some("组合 HHI 中度集中阈值 (0-1)".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_divers_high".into(),
            var_type: "number".into(),
            value: serde_json::json!(8.0),
            description: Some("组合有效股票数充分分散阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "risk_divers_medium".into(),
            var_type: "number".into(),
            value: serde_json::json!(4.0),
            description: Some("组合有效股票数适度分散阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "analysis_dry_run".into(),
            var_type: "boolean".into(),
            value: serde_json::json!(false),
            description: Some("干跑模式：不调用 LLM，用 mock 输出验证流程".into()),
            is_secret: false,
        },
        // ── 业绩超预期分级阈值
        Variable {
            name: "earnings_th_huge_pos".into(),
            var_type: "number".into(),
            value: serde_json::json!(50.0),
            description: Some("业绩超预期: 大幅超预期下界 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "earnings_th_strong_pos".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("业绩超预期: 强超预期下界 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "earnings_th_mild_pos".into(),
            var_type: "number".into(),
            value: serde_json::json!(5.0),
            description: Some("业绩超预期: 略超预期下界 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "earnings_th_mild_neg".into(),
            var_type: "number".into(),
            value: serde_json::json!(-5.0),
            description: Some("业绩超预期: 略低于预期下界 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "earnings_th_strong_neg".into(),
            var_type: "number".into(),
            value: serde_json::json!(-20.0),
            description: Some("业绩超预期: 强低于预期下界 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "earnings_th_huge_neg".into(),
            var_type: "number".into(),
            value: serde_json::json!(-50.0),
            description: Some("业绩超预期: 大幅低于预期下界 (%)".into()),
            is_secret: false,
        },
        // 质押风险分级阈值
        Variable {
            name: "pledge_warning_line".into(),
            var_type: "number".into(),
            value: serde_json::json!(50.0),
            description: Some("大股东质押比例预警线 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "pledge_liquidation_line".into(),
            var_type: "number".into(),
            value: serde_json::json!(70.0),
            description: Some("大股东质押比例平仓线 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "pledge_medium_line".into(),
            var_type: "number".into(),
            value: serde_json::json!(30.0),
            description: Some("大股东质押中风险阈值 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "pledge_low_line".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("大股东质押低风险阈值 (%)".into()),
            is_secret: false,
        },
        // 蒙特卡洛模拟默认参数
        Variable {
            name: "mc_default_price".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("蒙特卡洛模拟默认价格".into()),
            is_secret: false,
        },
        Variable {
            name: "mc_default_return".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.08),
            description: Some("蒙特卡洛模拟默认年化收益".into()),
            is_secret: false,
        },
        Variable {
            name: "mc_default_volatility".into(),
            var_type: "number".into(),
            value: serde_json::json!(0.3),
            description: Some("蒙特卡洛模拟默认年化波动率".into()),
            is_secret: false,
        },
        Variable {
            name: "mc_default_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(30),
            description: Some("蒙特卡洛模拟默认天数".into()),
            is_secret: false,
        },
        Variable {
            name: "mc_default_simulations".into(),
            var_type: "number".into(),
            value: serde_json::json!(1000),
            description: Some("蒙特卡洛模拟默认路径数".into()),
            is_secret: false,
        },
        // 行业内估值/增长对比阈值
        Variable {
            name: "industry_pe_cheap".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.0),
            description: Some("行业内 PE 相对低估阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "industry_pe_expensive".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.5),
            description: Some("行业内 PE 相对高估阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "industry_growth_high".into(),
            var_type: "number".into(),
            value: serde_json::json!(1.2),
            description: Some("行业内高增长阈值".into()),
            is_secret: false,
        },
        // 涨停潜力评分
        Variable {
            name: "limit_pct_main".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("主板涨停幅度 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_pct_star".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("创业板/科创板涨停幅度 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_pct_bj".into(),
            var_type: "number".into(),
            value: serde_json::json!(30.0),
            description: Some("北交所涨停幅度 (%)".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_w_trend".into(),
            var_type: "number".into(),
            value: serde_json::json!(40.0),
            description: Some("涨停潜力评分 - 趋势权重".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_w_volume".into(),
            var_type: "number".into(),
            value: serde_json::json!(20.0),
            description: Some("涨停潜力评分 - 量能权重".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_w_hits".into(),
            var_type: "number".into(),
            value: serde_json::json!(15.0),
            description: Some("涨停潜力评分 - 历史涨停权重".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_th_high".into(),
            var_type: "number".into(),
            value: serde_json::json!(60.0),
            description: Some("涨停潜力 - 高潜力阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_th_med".into(),
            var_type: "number".into(),
            value: serde_json::json!(30.0),
            description: Some("涨停潜力 - 中潜力阈值".into()),
            is_secret: false,
        },
        Variable {
            name: "limit_up_th_low".into(),
            var_type: "number".into(),
            value: serde_json::json!(10.0),
            description: Some("涨停潜力 - 低潜力阈值".into()),
            is_secret: false,
        },
        // ── 反思复盘参数（quality-fallback / portfolio-mgr 复用 portfolio-manager 模板）──
        Variable {
            name: "actual_outcome".into(),
            var_type: "string".into(),
            value: serde_json::json!(""),
            description: Some("实际走势结果，如 '30天跌8% → 失败'，非空时切换反思模式".into()),
            is_secret: false,
        },
        Variable {
            name: "reflection_depth".into(),
            var_type: "string".into(),
            value: serde_json::json!("light"),
            description: Some("反思深度：light(简要) / deep(详细推理链)".into()),
            is_secret: false,
        },
        // ── v18 (A1 借鉴): 历史反思教训注入 ──
        // TradingAgents 的 past_context 机制: 决策链路(trader/research-mgr/
        // value-investor)能拿之前反思的简短教训(同 ticker 近 90 天的
        // lesson_summary),避免重蹈覆辙。
        // runtime 由 run_stock_workflow_inner / run_single_stock_analysis
        // 显式注入 fetch_stock_lessons() 的查询结果;此处仅声明 schema 占位,
        // 确保模板渲染不报 VARIABLE_NOT_FOUND。
        Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::json!("（暂无历史反思）"),
            description: Some(
                "v18: 该股最近 90 天的反思教训(lesson_summary),由 runtime 注入".into(),
            ),
            is_secret: false,
        },
    ];
    let variables_val = serde_json::to_string(&variables).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化变量失败: {e}"))
    })?;

    // ── 合并旧版本的变量值（保留用户自定义的评分权重/阈值等）──
    let variables_val = match old_variables {
        Some(ref ov) if !ov.is_empty() => {
            merge_variable_values(&variables_val, ov).unwrap_or_else(|_| variables_val.clone())
        },
        _ => variables_val,
    };

    // ── Phase 3/4: Rhai 综合评分工具 + ErrorConfig ──
    use crate::commands::error::ErrorResponse;
    use axagent_harness::workflow_types::RhaiToolDef;
    let stock_score_rhai = r##"
// 综合评分脚本：技术面(30%) + 基本面(25%) + 情绪面(20%) + 资金面(15%) + 政策面(10%)
let w_tech = ctx.variables.weight_technical ?? 30.0;
let w_fund = ctx.variables.weight_fundamental ?? 25.0;
let w_sent = ctx.variables.weight_sentiment ?? 20.0;
let w_flow = ctx.variables.weight_money_flow ?? 15.0;
let w_pol = ctx.variables.weight_policy ?? 10.0;

let tech = ctx.results["a-market-analyst"] ?? 50.0;
let fund = ctx.results["a-fundamentals"] ?? 50.0;
let sent = ctx.results["a-sentiment"] ?? 50.0;
let flow = ctx.results["a-hot-money"] ?? 50.0;
let pol = ctx.results["a-policy"] ?? 50.0;

let score = (tech * w_tech + fund * w_fund + sent * w_sent + flow * w_flow + pol * w_pol) / 100.0;
#{
    score: score,
    level: if score >= 80 { "强烈推荐" }
           else if score >= 60 { "推荐" }
           else if score >= 40 { "中性" }
           else { "回避" }
}
"##;
    let rhai_tool_defs: Vec<RhaiToolDef> = vec![RhaiToolDef {
        tool_name: "compute_stock_score".into(),
        description: Some("综合技术面/基本面/情绪面/资金面/政策面计算 0-100 评分".into()),
        code: stock_score_rhai.into(),
    }];
    let tool_defs_val = serde_json::to_string(&rhai_tool_defs).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL)
            .with_detail(format!("序列化 Rhai 工具定义失败: {e}"))
    })?;

    let error_config = ErrorConfig {
        retry_policy: Some(RetryPolicy {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        }),
        on_failure: OnFailureAction::ContinueWithDefault,
        error_branch: None,
        compensation_steps: None,
    };

    let error_config_val = serde_json::to_string(&error_config).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL)
            .with_detail(format!("序列化 ErrorConfig 失败: {e}"))
    })?;

    /// 将子图节点坐标从绝对坐标转换为相对容器的偏移。
    /// 种子数据中的节点坐标是画布绝对坐标，但编辑器 Phase 3 的 subGraph 注入
    /// 将 subGraph 节点 position 视为相对容器的偏移（editor 叠加容器 position
    /// 计算绝对坐标），因此必须在注入前转换。
    fn adjust_positions_to_relative(
        mut sub_nodes: Vec<WorkflowNode>,
        container_id: &str,
        all_nodes: &[WorkflowNode],
    ) -> Vec<WorkflowNode> {
        let container_pos = all_nodes
            .iter()
            .find(|n| n.base_id() == container_id)
            .map(|n| n.base().position.clone())
            .unwrap_or(Position { x: 0.0, y: 0.0 });
        for node in sub_nodes.iter_mut() {
            match node {
                WorkflowNode::Trigger(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Agent(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Llm(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Condition(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Parallel(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Loop(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Merge(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Delay(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Validation(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::SubWorkflow(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::DocumentParser(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::VectorRetrieve(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::End(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::HttpRequest(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Switch(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::DatabaseQuery(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Notification(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Approval(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::FileOperation(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::DataTransformer(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::WebhookSend(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Logging(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::LlmClassifier(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Aggregator(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Email(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Debate(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Swarm(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Storage(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Tool(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::Code(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
                WorkflowNode::WorkflowRef(n) => {
                    n.base.position.x -= container_pos.x;
                    n.base.position.y -= container_pos.y;
                },
            }
        }
        sub_nodes
    }

    // ── 注入容器节点子图（subGraph）用于编辑器嵌套渲染 ──
    // 子图仅在编辑器的 ReactFlow 渲染层中用于坐标转换（绝对→相对），
    // 运行时引擎仍从顶层 nodes 读取所有节点。
    // 编辑器保存时会自动去重（上游 WorkflowEditor.tsx save 路径过滤 subGraph 子节点）。
    let container_nodes: &[&str] = &["p-analysts", "debate-bull-bear", "p-risk-assess"];
    for &cid in container_nodes {
        let child_ids: Vec<String> = nodes
            .iter()
            .filter(|n| n.base().parent_id.as_deref() == Some(cid))
            .map(|n| n.base_id().to_string())
            .collect();
        if child_ids.is_empty() {
            continue;
        }
        let child_node_ids: std::collections::HashSet<&str> =
            child_ids.iter().map(|s| s.as_str()).collect();
        let sub_edges: Vec<WorkflowEdge> = edges
            .iter()
            .filter(|e| {
                child_node_ids.contains(e.source.as_str())
                    && child_node_ids.contains(e.target.as_str())
            })
            .cloned()
            .collect();
        let sub_nodes: Vec<WorkflowNode> = nodes
            .iter()
            .filter(|n| child_node_ids.contains(n.base_id()))
            .cloned()
            .collect();
        let sub_graph = SubGraph {
            // 子图节点坐标必须相对于容器（Phase 3 编辑器将 subGraph 节点视为相对偏移，
            // 计算绝对坐标时叠加 container.position）。种子数据中的坐标是绝对坐标，
            // 因此在注入前转换为相对坐标。
            nodes: adjust_positions_to_relative(sub_nodes, cid, &nodes),
            edges: sub_edges,
        };
        // 注入到容器节点 config 中
        for n in nodes.iter_mut() {
            if n.base_id() != cid {
                continue;
            }
            match n {
                WorkflowNode::Parallel(p) => {
                    p.config.sub_graph = Some(sub_graph);
                },
                WorkflowNode::Debate(d) => {
                    d.config.sub_graph = Some(sub_graph);
                },
                _ => {},
            }
            break;
        }
    }
    // 写入 DB
    let nodes_json = serde_json::to_string(&nodes).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化节点失败: {e}"))
    })?;
    // DEBUG: 验证前几个 Tool 节点的 type 字段
    for n in nodes.iter().take(5) {
        let json = serde_json::to_string(n).unwrap_or_default();
        let preview = if json.len() > 200 {
            &json[..200]
        } else {
            &json
        };
        tracing::info!(node_id = %n.base_id(), json_preview = %preview, "seed_node_type");
    }
    let edges_json = serde_json::to_string(&edges).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化边失败: {e}"))
    })?;
    let tags = serde_json::to_string(&["stock", "analysis", "A股"]).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化标签失败: {e}"))
    })?;

    // 先删再插，避免 SeaORM .save() 对已存在记录的 update 失败
    let _ = workflow_template::Entity::delete_by_id(TEMPLATE_ID)
        .exec(db)
        .await;
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.to_string()),
        name: Set("A股多维度分析".to_string()),
        description: Set(Some(
            "9 维度分析师 → LLM 智能辩论 → 价值投资（巴菲特框架）→ 3 风险维度 → Rhai 评分 → 交易方案 → 投资决策"
                .to_string(),
        )),
        icon: Set("chart-bar".into()),
        tags: Set(Some(tags)),
        version: Set(TEMPLATE_VERSION),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(
            serde_json::to_string(&TriggerConfig {
                trigger_type: TriggerType::Schedule,
                config: serde_json::json!({
                    "schedules": {
                        "morning": "0 9 * * 1-5",
                        "afternoon": "0 14 * * 1-5",
                    },
                    // F-9 修复: 原 enabled=false 导致工作流不会自动调度。
                    //   既然有 schedule 配置,就应该是自动跑。改为 true,
                    //   用户仍可在 UI 切换到 "未启用" 状态临时停止。
                    "enabled": true,
                    "timezone": "Asia/Shanghai",
                }),
            })
            .unwrap_or_default(),
        )),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(Some(input_schema_val)),
        output_schema: Set(Some(output_schema_val)),
        variables: Set(if let Some(ref ov) = old_variables {
            // 升级时保留用户自定义的变量值
            let new_vars: Vec<serde_json::Value> =
                serde_json::from_str(&variables_val).unwrap_or_default();
            let mut final_vars = new_vars.clone();
            if let Ok(old_parsed) = serde_json::from_str::<Vec<serde_json::Value>>(ov) {
                for nv in &mut final_vars {
                    let nv_name = nv.get("name").and_then(|n| n.as_str());
                    if let Some(nv_name) = nv_name {
                        if let Some(old_v) = old_parsed
                            .iter()
                            .find(|ov| ov.get("name").and_then(|n| n.as_str()) == Some(nv_name))
                        {
                            if let Some(old_val) = old_v.get("value") {
                                nv.as_object_mut()
                                    .map(|o| o.insert("value".into(), old_val.clone()));
                            }
                        }
                    }
                }
            }
            Some(serde_json::to_string(&final_vars).unwrap_or(variables_val.clone()))
        } else {
            Some(variables_val.clone())
        }),
        error_config: Set(Some(error_config_val)),
        composite_source: Set(None),
        tool_defs: Set(Some(tool_defs_val)),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("写入工作流模板失败: {e}")))?;

    tracing::info!("[stock_analysis_setup] 股票分析工作流模板已种子化 ({TEMPLATE_ID})");
    Ok(())
}
