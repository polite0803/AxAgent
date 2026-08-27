//! 股票分析专家/角色/Profile 自动种子化到 agency_experts/agent_roles/agent_profiles 表。
//! 使用 include_str! 编译期嵌入 .md 内容，打包后无需文件 I/O。

use super::{PROFILE_TOOLS, build_analyst_input_mapping, merge_variable_values};
use crate::commands::error_code::stock_setup;

pub(crate) async fn seed_stock_analysis_workflow_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    use axagent_entities::workflow_template;
    use axagent_harness::hallucination_guard::HallucinationGuardConfig;
    use axagent_harness::workflow_types::{
        AgentNode, AgentNodeConfig, AggregatorNode, AggregatorNodeConfig, BackoffType, Branch,
        CodeNode, CodeNodeConfig, DebateNode, DebateNodeConfig, DegradeStrategy, EdgeType, EndNode,
        EndNodeConfig, ErrorConfig, JsonSchema, JsonSchemaProperty, LlmClassifierNode,
        LlmClassifierNodeConfig, MergeStrategy, NotificationNode, NotificationNodeConfig,
        OnFailureAction, OutputMode, ParallelNode, ParallelNodeConfig, Position, RetryConfig,
        StorageNode, StorageNodeConfig, SubGraph, SwitchCase, SwitchNode, SwitchNodeConfig,
        ToolDef, ToolNode, ToolNodeConfig, TriggerConfig, TriggerNode, TriggerType,
        ValidationAssertion, ValidationNode, ValidationNodeConfig, Variable, WorkflowEdge,
        WorkflowNode, WorkflowNodeBase, WorkflowRetryPolicy,
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    const TEMPLATE_ID: &str = "stock-analysis";

    // V3(2026-08-09): data-quality.rhai 全量缺陷修复——
    //   P0: count_chars replace 崩溃（Rhai 默认 Engine 无 StringPackage）+ pm_compute_factor_completeness
    //       Option 参数注册不可调用（Rhai 1.25 多 Option 参数闭包 Function not found）→ Dynamic 参数
    //   P1-1: money_flow/lockup_bundle/announcements 注入为 map 的 type_of 判断修复
    //   P2-2: trader_direction 类型防御；P2-3: n 动态推导；P3-1/2/4: count_chars/consistency_bonus/diag_for
    const TEMPLATE_VERSION: i32 = 3;

    tracing::info!(
        "[stock_analysis_setup] seed_stock_analysis_workflow_template 开始: TEMPLATE_ID={TEMPLATE_ID}, TEMPLATE_VERSION={TEMPLATE_VERSION}"
    );

    // 升级前保留旧模板的变量自定义值，在函数体外声明以延长生命周期
    let mut old_variables: Option<String> = None;

    if let Some(existing) =
        workflow_template::Entity::find_by_id(TEMPLATE_ID).one(db).await.map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL)
                .with_detail(format!("查询工作流模板失败: {e}"))
        })?
    {
        tracing::info!(
            "[stock_analysis_setup] 找到已有模板 v{}: nodes.len={}, edges.len={}",
            existing.version,
            existing.nodes.len(),
            existing.edges.len()
        );

        // 检查节点数据完整性：如果节点或边为空，即使版本号满足也需要强制重新种子化
        let nodes_empty =
            existing.nodes.is_empty() || existing.nodes == "[]" || existing.nodes == "null";
        let edges_empty =
            existing.edges.is_empty() || existing.edges == "[]" || existing.edges == "null";

        if nodes_empty || edges_empty {
            tracing::warn!(
                "[stock_analysis_setup] 模板 v{} 节点/边数据为空 (nodes_empty={}, edges_empty={})，强制重新种子化",
                existing.version,
                nodes_empty,
                edges_empty
            );
        } else if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[stock_analysis_setup] 模板已是最新版本 v{}，跳过种子化 (nodes={}, edges={})",
                existing.version,
                existing.nodes.len(),
                existing.edges.len()
            );
            return Ok(());
        }

        tracing::info!(
            "[stock_analysis_setup] 更新股票分析工作流模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );
        // 写版本快照（复用 update_workflow_template 的 snapshot 机制）
        let ver_id = format!("{}_v{}", TEMPLATE_ID, existing.version);
        if axagent_entities::workflow_template_version::Entity::find_by_id(&ver_id)
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("查重失败: {e}"))
            })?
            .is_none()
        {
            use sea_orm::ActiveModelTrait;
            let snapshot = axagent_entities::workflow_template_version::ActiveModel {
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
    } else {
        tracing::info!("[stock_analysis_setup] 模板 {TEMPLATE_ID} 不存在，将创建新模板");
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
                retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
                timeout: None, // 继承 RunOptions.tool_timeout（来自 tool_timeout_secs 设置）,用户可在面板中控制
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
    let td_social_sentiment = ToolDef {
        name: "get_social_sentiment".into(),
        description: Some("获取社交舆情：股吧帖子数/情感倾向/看多看空比例".into()),
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
    let td_search_news = ToolDef {
        name: "search_news".into(),
        description: Some("按关键词搜索财经新闻，用于验证催化剂/CapEx/行业趋势".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([
                (
                    "keyword".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("搜索关键词".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "limit".into(),
                    JsonSchemaProperty {
                        schema_type: "integer".into(),
                        description: Some("返回条数".into()),
                        default: Some(serde_json::json!(10)),
                        enum_values: None,
                        format: None,
                    },
                ),
            ])),
            required: Some(vec!["keyword".into()]),
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
    // P0 修复(2026-07-22): 移除未实现的 get_announcement_content ToolDef
    // 该工具在 mcp_tools.rs 中未注册 dispatch，调用会触发 "Unknown MCP tool" 错误
    let td_sector_info = ToolDef {
        name: "get_stock_sector_info".into(),
        description: Some("获取行业分类（申万一级/二级、概念板块标签）".into()),
        parameters: stock_code_params(),
    };
    let td_candlestick_patterns = ToolDef {
        name: "detect_candlestick_patterns".into(),
        description: Some("检测 K 线形态（吞没/锤子/晨星等 12 种）".into()),
        parameters: data_params(),
    };
    let td_divergence = ToolDef {
        name: "detect_divergence".into(),
        description: Some("检测价量背离（RSI 顶底背离 + OBV 背离）".into()),
        parameters: data_params(),
    };

    // 工具名 → ToolDef 映射（用于按名查找，给节点填充 config.tools）
    let tool_def_map: std::collections::HashMap<&str, ToolDef> = [
        ("get_stock_quote", td_quote.clone()),
        ("get_stock_kline", td_kline.clone()),
        ("get_stock_financials", td_fin.clone()),
        // Phase 2: 基本面报告(markdown)由 t-fundamentals-data 节点调用
        ("get_fundamentals_report_markdown", td_fundamentals_report.clone()),
        ("get_stock_news", td_news.clone()),
        ("get_stock_money_flow", td_mf.clone()),
        ("get_social_sentiment", td_social_sentiment.clone()),
        ("compute_scoring", td_score.clone()),
        ("compute_valuation", td_val.clone()),
        ("compute_portfolio_risk", td_risk.clone()),
        (
            "search_stock",
            ToolDef {
                name: "search_stock".into(),
                description: Some(
                    "按代码或名称模糊搜索A股。keyword 必须是完整的中文名称（如'中国卫通'）或 6 位数字代码（如'601698'），禁止传入拼音片段".into(),
                ),
                parameters: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("keyword".into(), sc_prop("完整中文名称或6位数字代码，如'中国卫通'或'601698'"));
                    Some(JsonSchema {
                        schema_type: "object".into(),
                        description: None,
                        properties: Some(props),
                        required: Some(vec!["keyword".into()]),
                        items: None,
                    })
                },
            },
        ),
        ("get_hot_stocks", td_hot.clone()),
        ("get_industry_ranking", td_industry.clone()),
        ("get_stock_announcements", td_announce.clone()),
        ("get_stock_consensus_eps", td_consensus.clone()),
        ("compute_kdj", td_kdj.clone()),
        ("compute_obv", td_obv.clone()),
        ("get_cls_flash", td_cls.clone()),
        ("search_news", td_search_news.clone()),
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
        ("get_stock_sector_info", td_sector_info.clone()),
        ("detect_candlestick_patterns", td_candlestick_patterns.clone()),
        ("detect_divergence", td_divergence.clone()),
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
                timeout: None, // 继承 RunOptions.step_timeout（来自 agent_timeout_secs 设置）,让用户在面板中可调
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
                // V74 关闭: hallucination_guard 锚定检查。
                // 原因：LLM 输出是分析结论（自然语言），source_context 是工具返回的 JSON 数据，
                // 格式天然不匹配导致 2-gram 匹配率极低（实测分数 0.0-0.39，阈值 0.4），
                // 几乎所有 AgentNode 都触发 WARN 误报，无实际拦截价值。
                // portfolio-mgr.rhai 已有 __untrusted 兜底机制（来自 a-policy 等数据缺失场景），
                // 不依赖 hallucination_guard 的输出。
                hallucination_guard: Some(HallucinationGuardConfig {
                    enabled: false,
                    match_threshold: 0.4,
                }),
                // H4.1: fallback_model 不在此硬编码。用户可在工作流编辑器中为单个 Agent 节点
                // 配置 model（主模型），agent_executor 校验失败时若 fallback_model ≠ model 则触发重试。
                // 股票分析模板默认不设 fallback，由项目默认模型一致性保证。
                fallback_model: None,
                task_scene: None,
                // stream_chunk_timeout_secs: 300s（5 分钟）
                // 默认 120s 在大上下文场景下偶发 TTFB >120s 导致超时重试浪费时间
                // （参见 debate-convergence 节点的同类修复注释）
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
        ("a-market-analyst", "技术面分析：K线形态、MACD/RSI、支撑阻力位", "market-analyst"),
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
        // 修复(2026-07-21): t-sentiment-data 改为 get_social_sentiment。
        // 原调用 get_stock_news 与 t-policy-data/t-news-data 重复,且 news API
        // 失败时 sentiment-analyst 拿不到任何前置数据。改为 get_social_sentiment:
        //   1) 真正对齐前端 "舆情" label —— 股吧帖子数/情感倾向/看多看空比例
        //   2) 与 t-hotmoney-data (get_stock_money_flow) 解耦,避免重复
        //   3) PROFILE_TOOLS 中仍保留 get_stock_news/get_stock_money_flow,
        //      LLM 可按需调用补充新闻和资金流数据
        ("t-sentiment-data", "获取股吧社交舆情", "get_social_sentiment", "stock_code"),
        // 修复(2026-07-21): t-news-data 改回 get_stock_news。
        // 2026-07-11 改为 get_stock_announcements 是因为 a-news 拿不到新闻,但
        // 导致前端 label "新闻" 与实际数据源 "公告" 错位,且与 t-catalyst-data
        // 重复调用同一工具。改回 get_stock_news 让 "新闻" label 真对应新闻,
        // a-news 仍可通过 PROFILE_TOOLS 调用 get_stock_announcements 补充公告。
        ("t-news-data", "获取近期新闻", "get_stock_news", "stock_code"),
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
        // 修复(2026-07-22): 改用新增的 get_stock_policy_news 工具。
        // 该工具基于股票所属行业做关键词搜索("政策/规划/通知/补贴"),
        // 相比原 get_stock_news(综合新闻)能更精准命中政策类内容,
        // 避免与 t-news-data 重复调用同一工具。
        //
        // 数据来源:东方财富搜索 API(基于行业关键词),无需对接政府网站。
        // 限制:返回的是新闻摘要(非政策原文),a-policy 分析师需基于摘要做推断。
        ("t-policy-data", "获取政策新闻", "get_stock_policy_news", "stock_code"),
        // F-8 重排: a-hot-money 前置改为资金流向工具
        ("t-hotmoney-data", "获取资金流向", "get_stock_money_flow", "stock_code"),
        // F-8 重排: a-lockup 前置改为解禁质押工具
        ("t-lockup-data", "获取解禁+增减持+大宗交易", "get_stock_lockup_bundle", "stock_code"),
        // F-8 重排: a-research 前置改为研报工具
        ("t-research-data", "获取研报+新闻", "get_stock_research_reports", "stock_code"),
        ("t-sector-data", "获取行情+行业排名", "get_industry_ranking", "stock_code"),
        // 修复(2026-07-21): t-catalyst-data 保留 get_stock_announcements。
        // 这是唯一调用 get_stock_announcements 的前置 ToolNode,避免与 t-news-data
        // 重复调用同一工具导致缓存击穿 + 双份失败警告。
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
            // 从工作流变量 market_regime.regime 注入（bull/bear/sideways 状态字符串）
            if *id == "a-fundamentals" {
                a.config
                    .input_mapping
                    .insert("market_regime".to_string(), "market_regime.regime".to_string());
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
            a.config.tools =
                tool_names.iter().filter_map(|&tn| tool_def_map.get(tn).cloned()).collect();
            a.config.exposed_tools = vec![];
            // a-catalyst 改用 Json 输出模式，prompt 已改为纯 JSON 格式
            if *id == "a-catalyst" {
                a.config.output_mode = OutputMode::Json;
                // P1 修复(2.2): 强制纯 JSON 输出，字段必须与 catalyst-analyst.md 的
                // 输出格式完全一致——前端 AnalystReportCard.extractSummary/extractTags
                // 与下游 portfolio-mgr 的 catalyst_level 映射都依赖这套字段，
                // 之前误写成 target_entity/event_type 等孤儿字段导致卡片恒显
                // "分析完成，但未返回结构化内容"且催化剂信号丢失。
                a.config.system_prompt = format!(
                    "{}\n{}\n{}",
                    a.config.system_prompt,
                    tool_prompt(&a.config.tools),
                    "【强制 JSON Schema 约束】\n\
                     输出必须是纯 JSON，字段名须与以下完全一致（不要增删、不要改名）:\n\
                     {\n\
                       \"verdict\": \"方向结论，必须三选一：看多 | 看空 | 中性\",\n\
                       \"bull_score\": \"看多评分 0-100 的整数，与 bear_score 之和接近 100\",\n\
                       \"bear_score\": \"看空评分 0-100 的整数，与 bull_score 之和接近 100\",\n\
                       \"report\": \"你的完整分析报告文本（自然语言，可含结构化分析，长度>50字）\",\n\
                       \"catalyst_level\": \"无 | L1普通消息 | L2业绩拐点级 | L3估值体系级 | L-1普通利空 | L-2业绩暴雷级 | L-3退市/造假级\",\n\
                       \"institutional_trace\": \"无 | 疑似建仓 | 有建仓痕迹 | 明显建仓\",\n\
                       \"narrative_completeness\": \"叙事完整度评分 0-100 的整数\",\n\
                       \"confidence\": \"置信度评分 0-100 的整数\",\n\
                       \"reasoning\": \"简短的推理过程摘要\"\n\
                     }\n\
                     只输出上述 JSON 对象，前后不要有任何其他文字（不要用 markdown 代码块包裹）。\n\
                     verdict/bull_score/bear_score 判定规则：L2/L3 利好催化剂 + 有建仓痕迹 + 叙事完整度≥60 → verdict=看多, bull_score≥60；\n\
                     L-2/L-3 利空催化剂 + 叙事破位 → verdict=看空, bear_score≥60；\n\
                     其它情况 → verdict=中性, bull_score/bear_score 均在 40-60 区间。"
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
    // V58 修复(2026-07-23): 删除 e-p-analysts-brief 伪边（p-analysts 是装饰性容器，
    // 立即完成且不携带子节点结果），改为直接从 10 个分析师节点连到 analyst-brief，
    // 确保：1) DAG 调度等所有分析师完成后才执行 analyst-brief；
    //       2) deps_results 包含 10 个分析师节点的输出，input_mapping 路径解析正确。
    // 前端验证要求容器节点有至少一条入边/出边，入边 e-trigger-p-analysts 已满足。

    // ── analyst-brief（分析师摘要）：CodeNode 聚合10份VERDICT评分+关键论据 ──
    // 替代原「辨手直接加载10份全量报告」方案，大幅降低辩论阶段上下文体积。
    // 输出 analyst-brief 字符串，经 input_mapping 接收10个分析师的 .content。
    //
    // 时序：p-analysts 全部完成后运行 → debate-bull-bear 依赖此摘要。
    {
        let ab_code = include_str!("../analyst-brief.rhai").to_string();
        let ab_input: std::collections::HashMap<String, String> = a_ids
            .iter()
            .map(|id| {
                let short = match *id {
                    "a-market-analyst" => "a_market_raw",
                    "a-sentiment" => "a_sentiment_raw",
                    "a-news" => "a_news_raw",
                    "a-fundamentals" => "a_fundamentals_raw",
                    "a-policy" => "a_policy_raw",
                    "a-hot-money" => "a_hot_money_raw",
                    "a-lockup" => "a_lockup_raw",
                    "a-research" => "a_research_raw",
                    "a-sector" => "a_sector_raw",
                    "a-catalyst" => "a_catalyst_raw",
                    _ => id,
                };
                // V58 修复(2026-07-23): 直接下钻到 .content.verdict，
                // resolve_var_path 会自动解析 content JSON 字符串并提取 verdict map。
                // 避免 Rhai 脚本中 json_parse 字符串解析的不可靠性。
                //
                // V60 修复(2026-07-23): a-catalyst 例外——它用 OutputMode::Json +
                // 扁平 JSON schema（bull_score/bear_score 在顶层，verdict 字段是字符串
                // "看多/看空/中性"），不是嵌套的 verdict map。用 .content.verdict 会
                // 拿到字符串而非 map，analyst-brief 触发"数据不可用"。
                // 改用 .content 让 Rhai 拿到整个对象，format_analyst 从顶层读
                // bull_score/bear_score（其他字段如 bull_points 缺失时 Rhai 返回 ()，
                // present() 检查会自然跳过）。
                let path = if *id == "a-catalyst" {
                    format!("{id}.content")
                } else {
                    format!("{id}.content.verdict")
                };
                (short.to_string(), path)
            })
            .collect();
        nodes.push(WorkflowNode::Code(CodeNode {
            base: WorkflowNodeBase {
                id: "analyst-brief".into(),
                title: "分析师摘要（VERDICT评分+关键论据）".into(),
                description: Some(
                    "将10位分析师的VERDICT评分和bull_points/bear_points压缩为摘要，供辩论阶段使用"
                        .into(),
                ),
                position: Position { x: 50.0, y: 1150.0 },
                retry: RetryConfig::default(),
                timeout: Some(10),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: true,
            },
            config: CodeNodeConfig {
                language: "rhai".into(),
                code: ab_code,
                output_var: "analyst-brief".into(),
                tool_name: None,
                execute_directly: true,
                input_mapping: ab_input,
            },
        }));
    }
    // V58 修复: 添加 10 条从分析师节点直连 analyst-brief 的 edges，
    // 确保 deps_results 包含所有分析师输出，且等所有分析师完成后才执行 analyst-brief。
    for aid in a_ids.iter() {
        edges.push(WorkflowEdge {
            id: format!("e-{aid}-brief"),
            source: (*aid).into(),
            source_handle: None,
            target: "analyst-brief".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });
    }
    // analyst-brief 直接喂给辩论阶段。个别维度缺数据的兜底由 brief 内的
    // 「**数据不可用**」标记处理——辩手看到标记自然知道跳过该维度。
    // 整份 brief 全空（极低概率）时辩手仍能输出「数据不足」的降级辩论，
    // 下游 pipeline 不会因缺少辩论输出而断裂。
    edges.push(WorkflowEdge {
        id: "e-brief-debate".into(),
        source: "analyst-brief".into(),
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
            position: Position { x: DEBATE_X, y: DEBATE_Y },
            retry: RetryConfig { enabled: true, max_retries: 1, ..Default::default() },
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
    // v16 历史工具集：辩手节点 v71 改为纯决策节点后不再使用，保留定义供未来恢复参考。
    // v17+ 可考虑给空方注入估值/风险类特色工具(td_var / td_maxdd / td_pledge / td_corr)。
    let _bull_tools = vec![
        td_quote.clone(),
        td_kline.clone(),
        td_fin.clone(),
        td_news.clone(),
        td_score.clone(),
        td_earnings.clone(),
        td_ma_cross.clone(),
        td_candlestick_patterns.clone(),
        td_divergence.clone(),
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
            a.base.retry = RetryConfig { enabled: true, max_retries: 1, ..Default::default() };
            // 超时继承 RunOptions.step_timeout（来自 agent_timeout_secs 设置），用户可在面板控制
            a.base.timeout = None;
            // P0 修复(2026-07-22): 移除 bull_tools，改为纯决策节点。
            // 原问题：bull_tools 含 td_quote/td_kline/td_fin 等需要 stock_code 的工具，
            // 但 input_mapping 未注入 stock_code（只有分析师评分），LLM 会传空值。
            // 辩手的 context_sources 已包含 analyst-brief 摘要（评分+关键论据），无需重新获取。
            a.config.tools = vec![];
            a.config.exposed_tools = vec![];
            a.config.system_prompt = format!(
                "{}\n\n--- 数据约束 ---\n\
                 你是辩论辩手，所有数据来自分析师摘要节点和前序辩论输出，禁止调用任何工具重新获取数据。\n\
                 基于分析师摘要中的评分数据和关键论据进行论证/质询/反驳。",
                a.config.system_prompt
            );
            a.config.max_tool_rounds = Some(0);
            a.config.model_role = Some("debater".into());
            // 注入前序轮次辩论输出 + analyst-brief 摘要作为上下文
            // （替代原先加载10份全量报告，由 analyst-brief CodeNode 聚合）
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..round_num {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            // 使用 analyst-brief CodeNode 的输出替代10份全量分析师报告
            ctx.push("analyst-brief".to_string());
            a.config.context_sources = ctx;
            // 注：评分数据已嵌入 analyst-brief 文本，不再额外通过 input_mapping 注入 30 个字段
            a.config.input_mapping = [].into_iter().collect();
        }
        nodes.push(bull_an);

        // 空方辩手：引用本轮多方输出 + 前序轮次辩论输出
        let mut bear_an =
            agent(&bear_id, &bear_title, bear_expert, Some("debate-bull-bear"), bear_x, bear_y);
        if let WorkflowNode::Agent(ref mut a) = bear_an {
            // 同 bull_an:R1/R2/R3 空方统一用 bull_tools。
            // 修复(阶段 4):同 bull_an,加 1 次重试 + 180s 超时,避免 LLM 瞬时失败
            //   导致辩论链雪崩(详见 bull_an 注释)。
            a.base.retry = RetryConfig { enabled: true, max_retries: 1, ..Default::default() };
            // 超时继承 RunOptions.step_timeout（来自 agent_timeout_secs 设置），用户可在面板控制
            a.base.timeout = None;
            // P0 修复(2026-07-22): 移除 bull_tools，改为纯决策节点。
            // 原问题：bull_tools 含 td_quote/td_kline/td_fin 等需要 stock_code 的工具，
            // 但 input_mapping 未注入 stock_code（只有分析师评分），LLM 会传空值。
            // 辩手的 context_sources 已包含 analyst-brief 摘要（评分+关键论据），无需重新获取。
            a.config.tools = vec![];
            a.config.exposed_tools = vec![];
            a.config.system_prompt = format!(
                "{}\n\n--- 数据约束 ---\n\
                 你是辩论辩手，所有数据来自分析师摘要节点和前序辩论输出，禁止调用任何工具重新获取数据。\n\
                 基于分析师摘要中的评分数据和关键论据进行论证/质询/反驳。",
                a.config.system_prompt
            );
            a.config.max_tool_rounds = Some(0);
            a.config.model_role = Some("debater".into());
            // 注入前序轮次 + 本轮多方输出 + analyst-brief 摘要作为上下文
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..round_num {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            ctx.push(bull_id.clone());
            // 使用 analyst-brief 替代全量分析师报告
            ctx.push("analyst-brief".to_string());
            a.config.context_sources = ctx;
            // 评分数据已嵌入 analyst-brief 文本，不再额外注入
            a.config.input_mapping = [].into_iter().collect();
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
            // 动态构建 context_sources：仅引用辩手输出（6 轮 R1→R3 完整辩论轨迹）。
            //
            // v69 优化(2026-07-22): 移除 10 个分析师节点,context_sources 从 16 → 6。
            // 理由:
            //   1. debate-convergence prompt 的核心职责是"收敛辩论",evidence 源是 R1-R3
            //      辩手输出(含 report 全文 + VERDICT 标签),不依赖分析师原文。
            //   2. 辩手 R1(bull/bear-researcher.md)已消化分析师报告,prompt 明确写
            //      "所有原始信号已经在上游 10 位分析师的报告中",convergence 无需重复读取。
            //   3. 分析师的 bull_score/bear_score/consensus_score 通过 input_mapping
            //      独立通道注入(agent_executor.rs 中 context_sources 和 input_mapping
            //      是两条独立路径,input_mapping 从全局 blackboard 提取,不依赖
            //      context_sources),30 个评分字段仍可用。
            // 预期效果: input tokens ~30k-40k → ~15k-20k(减少约 50%),缓解 LLM TTFB 超时。
            let mut ctx: Vec<String> = Vec::new();
            for r in 1..=debate_max_rounds {
                ctx.push(format!("bull-r{r}"));
                ctx.push(format!("bear-r{r}"));
            }
            a.config.context_sources = ctx;
            a.config.model_role = Some("debater".into());
            // 纯决策节点：tools 默认为空（agent 闭包），无需工具调用轮次
            a.config.max_tool_rounds = Some(0);
            a.config.output_mode = OutputMode::Json; // 输出结构化 JSON，确保 consensus_score / aggregate_prediction 被 input_mapping 解析
            a.config.input_mapping = build_analyst_input_mapping(&a_ids);
            // #1 修复(2026-07-22): debate-convergence 上下文极大
            // (6 轮辩手 + 30 个 input_mapping 结构化字段, ~15k-20k input tokens),
            // LLM 处理大上下文 TTFB 偶发 >120s 触发 stream chunk timeout。
            // 1) stream_chunk_timeout_secs: 300s（5 分钟）— 单 chunk 等待余量
            // 2) base.timeout: 900s（15 分钟）— 节点级总超时,避免外层 600s 兜底先触发
            a.config.stream_chunk_timeout_secs = Some(300);
        }
        if let WorkflowNode::Agent(ref mut a) = dc {
            a.base.timeout = Some(900);
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
                // V60 修复(2026-07-23): 接入 t-valuation 客观估值数据
                // 原问题：context_sources 只含 LLM 叙述(a-fundamentals/a-research/a-sector)，
                // 没有结构化财务数据。LLM 拿不到 PE/ROE/FCF/增速等原始数字，
                // 无法计算 PEG 或相对估值，只能凭叙述"猜"内在价值，出于保守本能
                // 必然给出低于现价的估值（"目标价值很低"问题的根因）。
                // t-valuation 提供 result.dcf.{low,mid,high,upsidePct}、
                // result.graham.upsidePct、result.fScore.score、result.moat.label 等
                // 客观算法估值，作为 LLM 估值的锚点。
                "t-valuation".into(),
            ];
            a.config.model_role = Some("stock-analyst".into());
            // P0 修复(2026-07-22): 移除所有工具，改为纯决策节点。
            // 原问题：tools 含 get_stock_financials/compute_valuation 等需要 stock_code
            // 的工具，但 input_mapping 未注入 stock_code，LLM 会传空值。
            // value-investor 的 context_sources 已包含 a-fundamentals/a-research/a-sector
            // + 辩论结果 + t-valuation，基本面和估值数据已通过上游注入。
            a.config.tools = vec![];
            a.config.exposed_tools = vec![];
            a.config.max_tool_rounds = Some(0);
            a.config.output_mode = OutputMode::Json;
            a.config.system_prompt = format!(
                "{}\n\n--- 数据约束 ---\n\
                 你是价值投资评估官，所有数据来自上游节点输出，禁止调用任何工具重新获取数据。\n\
                 - 基本面叙述: 来自 a-fundamentals（LLM 分析文本）\n\
                 - 研报数据: 来自 a-research\n\
                 - 行业数据: 来自 a-sector\n\
                 - 辩论共识: 来自 debate-convergence\n\
                 - **客观估值数据**: 来自 t-valuation（结构化算法结果）\n\
                   - result.dcf.{{low,mid,high}}: DCF 内在价值区间\n\
                   - result.dcf.upsidePct: DCF 上行空间百分比（正值=低估，负值=高估）\n\
                   - result.graham.upsidePct: 格雷厄姆上行空间\n\
                   - result.fScore.score: Piotrosky F-Score（0-9，越高越好）\n\
                   - result.moat.label: 护城河评级\n\
                 \n\
                 **关键**: t-valuation 是客观算法估值，作为你的估值锚点。\n\
                 你的 intrinsic_value_range 应参考 result.dcf.{{low,mid,high}} 区间，\n\
                 margin_of_safety 应参考 result.dcf.upsidePct。\n\
                 对成长股，参考 result.dcf.upsidePct 判断是否「合理偏低」，\n\
                 不要一味给出低于现价的保守估值。",
                a.config.system_prompt
            );
            // 环 A: 注入历史反思教训
            a.config.input_mapping =
                std::collections::HashMap::from([("stock_lessons".into(), "stock_lessons".into())]);
        }
        nodes.push(vi);
        edges.push(edge("e-debate-value-investor", &last_debate_node, vi_id));
        // value-investor 的 context_sources 中 debate-convergence 需要显式边，
        // 否则只在 bear-r3 完成后就调度，debate-convergence 还没跑完
        edges.push(edge("e-convergence-value-investor", "debate-convergence", vi_id));
        // V60 修复: t-valuation 加入 context_sources，需要显式边等待其完成，
        // 否则 t-valuation 的输出不会进入 value-investor 的变量池。
        // 拓扑链：bear-r3 → t-scoring → t-valuation → value-investor
        edges.push(edge("e-valuation-value-investor", "t-valuation", vi_id));
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
            position: Position { x: RISK_X, y: RISK_Y },
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
                    // P0 修复: 改用 UseDefault，超时后注入 null 到 workflow.results，
                    // 确保下游 research-mgr 的 context_sources 能找到 risk-agg 变量，
                    // 避免 "context_sources 变量未在 context.variables 中找到" ERROR。
                    degrade_strategy: DegradeStrategy::UseDefault,
                },
                Branch {
                    id: "risk-con".into(),
                    title: "保守评估".into(),
                    steps: vec!["risk-con".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::UseDefault,
                },
                Branch {
                    id: "risk-neu".into(),
                    title: "中性评估".into(),
                    steps: vec!["risk-neu".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::UseDefault,
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

    for (i, (rid, rtitle, rexpert, _rtools)) in [
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
                td_divergence.clone(),
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
                td_divergence.clone(),
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
                td_divergence.clone(),
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
            // P0 修复(2026-07-22): 移除 rtools，改为纯决策节点。
            // 原问题：rtools 含 td_score/td_risk/td_val/td_sharpe 等需要 stock_code 或
            // kline_json 的工具，但 input_mapping 未注入 stock_code，LLM 会传空值。
            // 风险评估的 context_sources 已包含所有分析师报告 + 辩论结果 + t-scoring/
            // t-valuation/t-risk，数据已完整注入，无需重新计算。
            a.config.tools = vec![];
            a.config.max_tool_rounds = Some(0);
            a.config.system_prompt = format!(
                "{}\n\n--- 数据约束 ---\n\
                 你是风险评估师，所有数据来自上游分析师报告、辩论结果和技术评分节点，禁止调用任何工具重新获取或计算。\n\
                 - 技术评分: 来自 t-scoring\n\
                 - 估值数据: 来自 t-valuation\n\
                 - 风险评分: 来自 t-risk\n\
                 基于上述数据以{}评估该股票的风险。",
                a.config.system_prompt, rtitle
            );
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
            position: Position { x: 300.0, y: 2400.0 },
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
            // 纯决策节点：tools 默认为空（agent 闭包），无需工具调用轮次
            a.config.max_tool_rounds = Some(0);
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
            position: Position { x: 840.0, y: 2700.0 },
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
            categories_var: None,
        },
    }));
    edges.push(edge("e-t-risk-cls-risk", "t-risk", "cls-risk-level"));

    // ── Validation: 结果完整性校验 ──
    nodes.push(WorkflowNode::Validation(ValidationNode {
        base: WorkflowNodeBase {
            id: "v-validate".into(),
            title: "结果完整性校验".into(),
            description: Some("确保分析报告包含必要字段，缺失时降级处理".into()),
            position: Position { x: 300.0, y: 3300.0 },
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

    // ── P1-4 修复: data-quality 确定性评分（CodeNode + Rhai，替代原 LLM Agent）──
    // 原 LLM Agent 需 5-10 秒 + token 消耗，改为 Rhai 确定性脚本 <10ms。
    // 基于 10 个分析师的 confidence + data_gaps 布尔值做聚合评分，
    // 输出 grade(A-F) + score(0-100) 与 Agent 版本格式一致。
    // 下游 quality-gate SwitchNode 和 portfolio-mgr 无需修改。
    {
        let dq_id = "data-quality";
        let dq_code = include_str!("../data-quality.rhai").to_string();
        nodes.push(WorkflowNode::Code(CodeNode {
            base: WorkflowNodeBase {
                id: dq_id.into(),
                title: "数据质量确定性评分".into(),
                description: Some("基于分析师信心+数据缺口做确定性评分，输出 grade/score".into()),
                position: Position { x: 840.0, y: 3300.0 },
                retry: RetryConfig::default(),
                timeout: Some(10),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: CodeNodeConfig {
                language: "rhai".into(),
                code: dq_code,
                output_var: dq_id.into(),
                tool_name: None,
                execute_directly: true,
                input_mapping: [
                    // P1 修复(2026-07-23): resolve_var_path("{id}.content.verdict.confidence") 全员 null，
                    // 改为 {id}.content.verdict 接收整份 verdict map，在 data-quality.rhai 内提取 confidence。
                    ("mk_verdict", "a-market-analyst.content.verdict"),
                    ("sent_verdict", "a-sentiment.content.verdict"),
                    ("news_verdict", "a-news.content.verdict"),
                    ("fund_verdict", "a-fundamentals.content.verdict"),
                    ("pol_verdict", "a-policy.content.verdict"),
                    ("hm_verdict", "a-hot-money.content.verdict"),
                    ("lk_verdict", "a-lockup.content.verdict"),
                    ("res_verdict", "a-research.content.verdict"),
                    ("sec_verdict", "a-sector.content.verdict"),
                    // P2 修复(2026-07-23): a-catalyst 的 cat_verdict 映射缺失 → Rhai 报
                    // "Variable not found: cat_verdict"。a-catalyst 走 OutputMode::Json +
                    // 扁平 JSON schema，confidence 在顶层（不在 verdict map 内），
                    // 所以用 .content 拿到完整 JSON 对象，extract_conf(v["confidence"]) 可正确提取。
                    ("cat_verdict", "a-catalyst.content"),
                    // P1-B3 新增(2026-07-24): 拿 10 个分析师的报告正文，算法化 report_quality_score。
                    // AgentExecutor OutputMode::Text 把 LLM 输出包装为 {report, verdict} JSON，
                    // 因此 .content.report 直接是字符串正文（含自然语言分析，不含 VERDICT 标签）。
                    ("mk_report", "a-market-analyst.content.report"),
                    ("sent_report", "a-sentiment.content.report"),
                    ("news_report", "a-news.content.report"),
                    ("fund_report", "a-fundamentals.content.report"),
                    ("pol_report", "a-policy.content.report"),
                    ("hm_report", "a-hot-money.content.report"),
                    ("lk_report", "a-lockup.content.report"),
                    ("res_report", "a-research.content.report"),
                    ("sec_report", "a-sector.content.report"),
                    ("cat_report", "a-catalyst.content.report"),
                    // ── V67 修复(2026-07-29): 映射分析师 __untrusted 标记 ──
                    // agent_executor 在 strict_mode 降级时于 NodeOutput 顶层注入 __untrusted=true。
                    // data-quality.rhai 需读取此标记,将不可信分析师排除出 good_count,
                    // 避免中性兜底 confidence=50 被当成有效信号推高 tool_credibility_score。
                    ("mk_untrusted", "a-market-analyst.__untrusted"),
                    ("sent_untrusted", "a-sentiment.__untrusted"),
                    ("news_untrusted", "a-news.__untrusted"),
                    ("fund_untrusted", "a-fundamentals.__untrusted"),
                    ("pol_untrusted", "a-policy.__untrusted"),
                    ("hm_untrusted", "a-hot-money.__untrusted"),
                    ("lk_untrusted", "a-lockup.__untrusted"),
                    ("res_untrusted", "a-research.__untrusted"),
                    ("sec_untrusted", "a-sector.__untrusted"),
                    ("cat_untrusted", "a-catalyst.__untrusted"),
                    // ── 因子数据完整度评估（供 pm_compute_factor_completeness 使用）──
                    // 这些因子数据来自 ToolNode，用于评估因子层数据完整度
                    ("total_score", "t-scoring.result.totalScore"),
                    ("consensus_score", "debate-convergence.content.consensus_score"),
                    ("catalyst_level", "a-catalyst.content.catalyst_level"),
                    ("risk_volatility", "t-risk.result.stockRiskProfile.annualizedVolatilityPct"),
                    ("valuation_dcf_upside", "t-valuation.result.dcf.upsidePct"),
                    ("trader_direction", "trader.content.verdict"),
                    ("money_flow", "t-hotmoney-data.result"),
                    ("lockup_bundle", "t-lockup-data.result"),
                    ("announcements", "t-catalyst-data.result"),
                    ("pace_signal", "pace-calc.result.pace_signal"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            },
        }));
        edges.push(edge("e-v-validate-data-quality", "v-validate", dq_id));
        // P1 修复(2026-07-24): data-quality 需要读到10个分析师的 verdict 输出，
        // 但它的依赖链仅含 v-validate → cls-risk-level，这两者都不依赖分析师，
        // 导致 data-quality 在分析师之前就跑完了。
        // 添加从每个分析师的边确保 data-quality 等待所有分析师完成。
        for aid in &a_ids {
            edges.push(edge(&format!("e-{aid}-data-quality"), aid, dq_id));
        }
        // 因子数据完整度评估：data-quality 需要等待 ToolNode 完成以获取因子数据
        edges.push(edge("e-t-scoring-data-quality", "t-scoring", dq_id));
        edges.push(edge("e-t-risk-data-quality", "t-risk", dq_id));
        edges.push(edge("e-t-valuation-data-quality", "t-valuation", dq_id));
        edges.push(edge("e-t-hotmoney-data-quality", "t-hotmoney-data", dq_id));
        edges.push(edge("e-t-lockup-data-quality", "t-lockup-data", dq_id));
        edges.push(edge("e-t-catalyst-data-quality", "t-catalyst-data", dq_id));
        edges.push(edge("e-pace-calc-data-quality", "pace-calc", dq_id));
        edges.push(edge("e-debate-convergence-data-quality", "debate-convergence", dq_id));
        // 修复循环依赖: 移除 trader → data-quality 边
        // data-quality.rhai 已内置 trader_direction 缺失值处理逻辑，无需此边
        // 原循环: data-quality → trader → data-quality 导致 CycleDetected 错误
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
            // 因此 research-mgr 直接消费三个原始风险辩手的输出。
            // V67 修复: 移除 "risk-aggregated"——V29 注释明确说不引用聚合数组，
            // 但该字段遗留未删，导致 research-mgr 报 "context_sources 变量未找到" ERROR。
            "risk-agg".into(),
            "risk-con".into(),
            "risk-neu".into(),
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
        // P0 修复(2026-07-22): research-mgr 改为纯决策节点，移除全部 14 个计算工具。
        // 原问题与 trader 同款：
        //   1) td_score/td_val/td_risk 与上游 t-scoring/t-valuation/t-risk 重复
        //   2) td_maxdd/td_sharpe/td_var/td_kelly 等需要 stock_code 或 kline_json 参数，
        //      但 input_mapping 未注入 stock_code，LLM 会传空值触发无效重试
        // 正确架构：综合评估基于 context_sources 注入的上游数据（t-scoring/t-valuation/
        // t-risk/risk-agg/risk-con/risk-neu/debate-convergence），无需重新计算。
        a.config.tools = vec![];
        a.config.system_prompt = format!(
            "{}\n\n--- 数据约束 ---\n\
             你是综合风险评估官，所有需要的数据已通过输入上下文注入，禁止调用任何工具重新获取或计算。\n\
             - 技术评分/指标: 来自 t-scoring（currentPrice/indicators/totalScore）\n\
             - 估值数据: 来自 t-valuation\n\
             - 风险评分: 来自 t-risk\n\
             - 三档风险评估: 来自 risk-agg/risk-con/risk-neu\n\
             - 辩论共识: 来自 debate-convergence（consensus_score）\n\
             基于上述数据综合评估总体风险评级与主要风险点清单。",
            a.config.system_prompt
        );
        a.config.max_tool_rounds = Some(0);
        a.config.exposed_tools = vec![];
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
        // P0 根因修复(2026-07-22): trader 改为纯决策节点，移除所有数据获取工具。
        // 原设计问题：
        //   1) td_quote/td_kline/td_mf 需要 stock_code，但 Agent 节点工具参数由 LLM
        //      自主生成，LLM 不知道 stock_code（input_mapping 只注入 system_prompt 文本），
        //      导致空 stock_code → 6 vendor × 2 轮无效重试（浪费 3.4 分钟）。
        //   2) compute_atr/kelly/mc 等需要 kline_json 参数，LLM 无法可靠地从
        //      system_prompt 复制 120 根 K 线 JSON 到工具参数。
        // 正确架构：数据由上游 t-scoring 获取并注入，trader 基于注入数据做决策。
        // ATR/Kelly/MC 等复杂计算应由独立 Code 节点完成（后续优化）。
        a.config.tools = vec![]; // 纯决策节点，无需工具
        a.config.system_prompt = format!(
            "{}\n\n--- 数据约束 ---\n\
             你是交易方案制定者，所有需要的数据已通过输入上下文注入，禁止调用任何工具重新获取数据。\n\
             - 当前价: 参考【reference_price】\n\
             - 技术指标: 参考【technical_indicators】(含 ma5/ma20/macd_dif/rsi14/boll_upper/boll_lower 等)\n\
             - 综合评分: 参考【total_score】\n\
             - 共识评分: 参考【consensus_score】\n\
             - 风险分歧: 参考【risk_disagreement】(>50 时保守)\n\
             - 数据质量: 参考【dqi_score】(<50 时保守)\n\
             基于上述数据直接制定交易方案，输出入场价、目标价、止损价、仓位比例。",
            a.config.system_prompt
        );
        a.config.max_tool_rounds = Some(0); // 禁用工具调用轮次
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
            // V58 修复: data-quality 是 CodeNode，score 在 .result 里
            ("dqi_score", "data-quality.result.score"),
            // P0 修复(2026-07-22): 注入 t-scoring 完整技术指标，替代 get_stock_quote/kline。
            // 包含 ma5/ma20/bias_ma5/macd_dif/macd_dea/rsi14/boll_upper/boll_lower 等，
            // trader 可直接读取指标制定交易方案，无需重新调用行情工具。
            ("technical_indicators", "t-scoring.result.indicators"),
            ("total_score", "t-scoring.result.totalScore"),
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
            position: Position { x: 240.0, y: 4200.0 },
            retry: RetryConfig::default(),
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: true,
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
                // V58 修复: data-quality 实为 CodeNode（Rhai），输出结构为
                //   {status, result: {grade, score, ...}, input_params, node_id, params}
                //   score 字段在 .result 里，旧路径 "data-quality.score" 无法穿透
                //   CodeNode 包装，导致 dqi_score 缺失 → f6_weight=0 → total_weight
                //   下降触发 weights_collapsed 误坍缩。
                ("dqi_score", "data-quality.result.score"),
                // P1/P2: 因子回测数据（compute_scoring 工具附加输出）
                ("factor_weights", "t-scoring.result.factor_backtest.factors"),
                // P1-1: 市场状态权重调节（regime-weights.rhai）替代纯回测权重
                // 牛市→趋势↑, 熊市→估值/风险↑, 高波动→全降权
                // V58 修复: regime-weights 是 CodeNode，factor_weights 在 .result 里
                ("regime_factor_weights", "regime-weights.result.factor_weights"),
                // market_regime 是 core.rs 注入的工作流变量（非 t-scoring 节点输出）
                ("market_regime_prior", "market_regime.confidence"),
                ("market_regime_state", "market_regime.regime"),
                // P1-7 修复: LLM 风险分类器作为算法分类的 fallback
                // Rhai 脚本先基于 t-risk stockRiskProfile 做确定性算法分类，
                // 仅当数据缺失时回退到此 LLM 分类结果。
                // 注意：该 LlmClassifierNode 的节点 id 是 "cls-risk-level"（output_var 才是 "risk-level"），
                // 边与 input_mapping 必须以节点 id 为准，否则 context.variables 查不到。
                ("overall_risk_llm", "cls-risk-level.category"),
                // AgentNode(Json mode) 输出包裹在 {role, content: <json_string>, ...} 中
                ("catalyst_level", "a-catalyst.content.catalyst_level"),
                ("consensusScore", "debate-convergence.content.consensus_score"),
                // V65: trader 输出完整 6 维度字段（与 portfolio-mgr 同维度对齐用于双视角对比）
                // 旧字段保留: trader_direction/trader_target_price/trader_stop_loss 供 f7 兼容路径
                // trader Json 模式输出：{action, verdict, positionPct, confidence, riskLevel, currentPrice,
                //   targetPrice, stopLoss, stopLossPct, takeProfitPct, timeHorizon, expectedHoldingDays,
                //   data_gaps, evidence_cited, risk_factors, decision_trail, lessons_applied, reasoning}
                ("trader_action", "trader.content.action"),
                ("trader_direction", "trader.content.verdict"),
                ("trader_confidence", "trader.content.confidence"),
                // currentPrice: 从 t-scoring 工具节点（get_stock_quote）获取，可靠数据源。
                // 不用 trader.content.currentPrice，因为 LLM 不一定输出该字段。
                ("current_price", "t-scoring.result.currentPrice"),
                ("trader_target_price", "trader.content.targetPrice"),
                ("trader_stop_loss", "trader.content.stopLoss"),
                ("trader_time_horizon", "trader.content.timeHorizon"),
                ("trader_holding_days", "trader.content.expectedHoldingDays"),
                // V65 新增: trader 6 维度对比字段（f7 可消费更丰富的 LLM 信号）
                ("trader_position_pct", "trader.content.positionPct"),
                ("trader_risk_level", "trader.content.riskLevel"),
                ("trader_stop_loss_pct", "trader.content.stopLossPct"),
                ("trader_take_profit_pct", "trader.content.takeProfitPct"),
                ("trader_data_gaps", "trader.content.data_gaps"),
                ("trader_evidence_count", "trader.content.evidence_cited"),
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
                // ── PACE 情绪因子 f11: pace-calc CodeNode 输出 pace_signal ──
                // V58 修复: pace-calc 是 CodeNode，pace_signal/pace_degraded 在 .result 里
                ("pace_signal", "pace-calc.result.pace_signal"),
                // P2-2: pace 降级标志（valid_event_count==0 时 pace-calc 设置）
                ("pace_degraded", "pace-calc.result.pace_degraded"),
                // ── 技术否决（technical-veto）输入：从 t-scoring 的完整指标获取 ──
                ("rsi_14", "t-scoring.result.indicators.rsi14"),
                ("macd_dif", "t-scoring.result.indicators.macdDif"),
                ("macd_dea", "t-scoring.result.indicators.macdDea"),
                // ── 市场模拟门（S-501~503）：core.rs 从个股 K 线注入的模拟指标 ──
                ("sim_stability", "sim_stability"),
                ("sim_liquidity", "sim_liquidity"),
                ("sim_impact", "sim_impact"),
                // ── D7/D8: 可配置阈值参数（来自 workflow_template.variables，通过 settings 页面持久化）──
                // action 决策阈值
                ("action_buy_threshold", "action_buy_threshold"),
                ("action_increase_threshold", "action_increase_threshold"),
                ("action_hold_threshold", "action_hold_threshold"),
                ("action_watch_threshold", "action_watch_threshold"),
                ("action_reduce_threshold", "action_reduce_threshold"),
                // 仓位阈值
                ("pos_buy_min", "pos_buy_min"),
                ("pos_increase_min", "pos_increase_min"),
                // 风险仓位上限
                ("pos_cap_extreme", "pos_cap_extreme"),
                ("pos_cap_high", "pos_cap_high"),
                ("pos_cap_mid", "pos_cap_mid"),
                // 风险分类阈值（极高/高/低）
                ("risk_debt_extreme", "risk_debt_extreme"),
                ("risk_vol_extreme", "risk_vol_extreme"),
                ("risk_sharpe_extreme", "risk_sharpe_extreme"),
                ("risk_vol_high", "risk_vol_high"),
                ("risk_dd_high", "risk_dd_high"),
                ("risk_roe_high", "risk_roe_high"),
                ("risk_debt_high", "risk_debt_high"),
                ("risk_vol_low", "risk_vol_low"),
                ("risk_sharpe_low", "risk_sharpe_low"),
                ("risk_dd_low", "risk_dd_low"),
                ("risk_roe_low", "risk_roe_low"),
                ("risk_debt_low", "risk_debt_low"),
                ("risk_growth_low", "risk_growth_low"),
                ("cost_pct", "cost_pct"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        },
    });
    nodes.push(pm);
    edges.push(edge("e-trader-portfolio-mgr", "trader", "portfolio-mgr"));
    edges.push(edge("e-research-mgr-portfolio-mgr", "research-mgr", "portfolio-mgr"));

    // ── P1-E13: portfolio-risk-gate 组合风控门（CodeNode + Rhai）──
    // 在 portfolio-mgr 之后、rule-check 之前运行。
    // 职责：单股仓位上限、行业暴露、持仓数量、风险档位否决、空头强制卖出、组合归一化
    // 输出保留 portfolio-mgr 的所有字段，仅覆盖 action/positionPct/riskLevel，追加 risk_gate 元数据
    let prg_code = include_str!("../portfolio-risk-gate.rhai").to_string();
    let prg = WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "portfolio-risk-gate".into(),
            title: "组合风控门".into(),
            description: Some(
                "组合层风控：仓位上限/行业暴露/风险档位否决/空头强制卖出/组合归一化".into(),
            ),
            position: Position { x: 480.0, y: 4200.0 },
            retry: RetryConfig::default(),
            timeout: Some(10),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: true,
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: prg_code,
            output_var: "portfolio-risk-gate".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: [
                // portfolio-mgr 的完整 result 对象（保留所有字段，覆盖调整字段）
                ("pm_result", "portfolio-mgr.result"),
                // 当前价（用于计算新增仓位市值 + 空头检测）
                ("current_price", "t-scoring.result.currentPrice"),
                // 目标价（用于空头检测：target < current × 0.85 → 强制卖出）
                ("target_price", "trader.content.targetPrice"),
                // 工作流变量（core.rs 注入）
                ("stock_code", "stock_code"),
                ("stock_sector", "stock_sector"),
                ("holdings_json", "holdings_json"),
                ("portfolio_cash", "portfolio_cash"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        },
    });
    nodes.push(prg);
    // portfolio-mgr → portfolio-risk-gate（主依赖）
    edges.push(edge("e-portfolio-mgr-risk-gate", "portfolio-mgr", "portfolio-risk-gate"));
    // t-scoring → portfolio-risk-gate（current_price 输入）
    edges.push(edge("e-t-scoring-risk-gate", "t-scoring", "portfolio-risk-gate"));
    // trader → portfolio-risk-gate（target_price 输入）
    edges.push(edge("e-trader-risk-gate", "trader", "portfolio-risk-gate"));

    // ── P1-1: regime-weights 市场状态权重调节（CodeNode + Rhai）──
    // 在 portfolio-mgr 之前运行，输出调节后的因子权重。
    // 牛市→趋势+资金面权重↑，熊市→估值+风险权重↑，高波动→所有因子降权
    {
        let rw_code = include_str!("../regime-weights.rhai").to_string();
        nodes.push(WorkflowNode::Code(CodeNode {
            base: WorkflowNodeBase {
                id: "regime-weights".into(),
                title: "市场状态权重调节".into(),
                description: Some("基于市场状态动态调节因子权重".into()),
                position: Position { x: 20.0, y: 4200.0 },
                retry: RetryConfig::default(),
                timeout: Some(5),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: true,
            },
            config: CodeNodeConfig {
                language: "rhai".into(),
                code: rw_code,
                output_var: "regime-weights".into(),
                tool_name: None,
                execute_directly: true,
                input_mapping: [
                    // market_regime 是 core.rs 注入的工作流变量（非 t-scoring 节点输出）
                    ("market_regime_state", "market_regime.regime"),
                    ("market_regime_prior", "market_regime.confidence"),
                    ("market_regime_volatility", "market_regime.volatility"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            },
        }));
        edges.push(edge("e-t-scoring-regime-weights", "t-scoring", "regime-weights"));
        edges.push(edge("e-regime-weights-portfolio-mgr", "regime-weights", "portfolio-mgr"));
    }

    // debate-convergence → portfolio-mgr: 显式边确保 consensus_score 在公式执行前就绪
    edges.push(edge("e-debate-convergence-portfolio-mgr", "debate-convergence", "portfolio-mgr"));
    // V29 修复: debate-convergence → research-mgr / trader 显式边
    // research-mgr 和 trader 的 input_mapping 引用 debate-convergence.content.consensus_score，
    // 加显式边确保共识分数在节点执行前就绪（符合显式依赖原则）
    edges.push(edge("e-debate-convergence-research-mgr", "debate-convergence", "research-mgr"));
    edges.push(edge("e-debate-convergence-trader", "debate-convergence", "trader"));
    // P0-3 修复: risk-convergence → research-mgr 显式边
    // research-mgr 的 context_sources 引用 risk-agg/risk-con/risk-neu 的原始输出，
    // 但 DAG 中仅有 debate-convergence → research-mgr 边，不保证风险辩手已完成。
    // 添加 risk-convergence → research-mgr 边，使 research-mgr 等待三个风险辩手
    // 全部完成后再调度，避免 risk-con 超时重试期间 context_sources 变量缺失。
    edges.push(edge("e-risk-convergence-research-mgr", "risk-convergence", "research-mgr"));
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
    edges.push(edge("e-t-dragon-tiger-data-portfolio-mgr", "t-dragon-tiger-data", "portfolio-mgr"));
    // ── 2026-07-25 修复: 龙虎榜数据 → a-hot-money 显式边 ──
    // t-dragon-tiger-data 输出 get_stock_dragon_tiger 结果（机构席位/游资动向/上榜原因），
    // a-hot-money 需要此数据做资金面追踪分析。LLM 仍可通过 PROFILE_TOOLS 调 get_stock_dragon_tiger
    // 补充，但有了这条边 + input_mapping 注入后可免 LLM 自行调用。
    edges.push(edge("e-t-dragon-tiger-data-hot-money", "t-dragon-tiger-data", "a-hot-money"));
    // ── P2 新增: 公告数据源 → portfolio-mgr 显式边 ──
    // t-catalyst-data 输出公司公告列表，用于公告关键词风险检测
    edges.push(edge("e-t-catalyst-data-portfolio-mgr", "t-catalyst-data", "portfolio-mgr"));

    // ── 修复 portfolio-mgr 因子输入全空（决策塌成全零空壳）: 补齐缺失的显式边 ──
    // portfolio-mgr 是 CodeNode（无 context_sources），其结构化输入仅来自 edges 直接上游节点。
    // 下方 5 个节点的输出被 portfolio-mgr.input_mapping 引用，但此前缺指向它的边，
    // 导致这些节点不进入 context.variables，因子输入（totalScore / catalyst_level /
    // 估值因子 / pace_signal / LLM 风险兜底）全部取不到 → 后验塌成先验 0.5 →
    // action=观望、positionPct=0、confidence=0 的全零空壳。
    // 与下方 p-risk-assess 补 e-scoring-p-risk-assess 边的修复同源（见行 1421 注释）。
    edges.push(edge("e-t-scoring-portfolio-mgr", "t-scoring", "portfolio-mgr"));
    edges.push(edge("e-a-catalyst-portfolio-mgr", "a-catalyst", "portfolio-mgr"));
    edges.push(edge("e-t-valuation-portfolio-mgr", "t-valuation", "portfolio-mgr"));
    edges.push(edge("e-pace-calc-portfolio-mgr", "pace-calc", "portfolio-mgr"));
    edges.push(edge("e-cls-risk-level-portfolio-mgr", "cls-risk-level", "portfolio-mgr"));

    // ── PACE 情绪因子（f11）: pace-calc.rhai — 基于公告的四维情绪向量计算 ──
    // pace-calc.rhai 已实现完整的 PACE 计算逻辑（Polarity/Actuality/Credibility/Expectation），
    // 基于 t-catalyst-data 的公告数据输出 pace_signal（[-1, 1]）。
    // 输出: {pace_vector:{P,A,C,E}, pace_signal, ...}
    // portfolio-mgr 消费 pace_signal 作为 f11_signal。
    {
        let pace_id = "pace-calc";
        let pace_code = include_str!("../pace-calc.rhai").to_string();
        nodes.push(WorkflowNode::Code(CodeNode {
            base: WorkflowNodeBase {
                id: pace_id.into(),
                title: "PACE 情绪因子计算（f11）".into(),
                description: Some(
                    "基于公告数据计算四维情绪向量，输出 pace_signal 作为 f11 因子信号".into(),
                ),
                // 放在 portfolio-mgr 左侧同一行，与 regime-weights 对称
                position: Position { x: 460.0, y: 4200.0 },
                retry: RetryConfig::default(),
                timeout: Some(10),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: true, // PACE 失败不应阻断主流程
            },
            config: CodeNodeConfig {
                language: "rhai".into(),
                code: pace_code,
                output_var: pace_id.into(),
                tool_name: None,
                execute_directly: true,
                input_mapping: [
                    // 主事件源：公告数据（fallback 路径，pace-calc 会从中提取 event_type）
                    ("announcement_events", "t-catalyst-data.result"),
                    // 资金流向数据（用于背离修正）
                    // t-hotmoney-data.result 的 JSON 结构含 main_net_inflow / history（近5日）
                    ("money_flow_net", "t-hotmoney-data.result.main_net_inflow"),
                    // 资金流向历史序列（近5日，含当日），用于趋势背离判断
                    ("money_flow_history", "t-hotmoney-data.result.history"),
                    // 板块 ETF 资金流向（用于协同增强）- 暂未接入
                    ("sector_etf_direction", ""),
                    // 历史 P 值 - 暂未接入（需要 upstream LLM 长期输出）
                    ("p_history", ""),
                    // LLM 事件源：a-catalyst 输出（含 catalyst_level/confidence/verdict）
                    // P0 修复(2026-07-22): 原为空占位导致 pace-calc 在 a-catalyst 完成前执行，
                    // 且 llm_events 恒为空。现在接入 a-catalyst.content，pace-calc.rhai 会
                    // 从 catalyst_level 中提取事件类型。
                    // V58 修复: a-catalyst 是 AgentNode（输出 {role, content, ...}），
                    // 不是 ToolNode/CodeNode，路径应为 .content 而非 .result
                    ("llm_events", "a-catalyst.content"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            },
        }));
        // pace-calc 依赖 t-catalyst-data 和 t-hotmoney-data（工具节点）
        edges.push(edge("e-t-catalyst-data-pace-calc", "t-catalyst-data", pace_id));
        edges.push(edge("e-t-hotmoney-data-pace-calc", "t-hotmoney-data", pace_id));
        // P0 修复(2026-07-22): 添加 pace-calc → a-catalyst 依赖边
        // 原缺失此边导致 pace-calc 在 a-catalyst 完成前就执行，llm_events 恒为 null
        edges.push(edge("e-a-catalyst-pace-calc", "a-catalyst", pace_id));
        // pace-calc → portfolio-mgr: pace_signal 作为 f11 输入
        edges.push(edge("e-pace-calc-portfolio-mgr", pace_id, "portfolio-mgr"));
    }

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
                "portfolio-risk-gate".into(),
                "t-scoring".into(),
                "t-valuation".into(),
                "t-risk".into(),
                "trader".into(),
            ];
            a.config.model_role = Some("risk-evaluator".into());
            // P0 修复(2026-07-22): rule-check 改为纯决策节点，移除全部工具。
            // 原问题与 trader 同款：context_sources 已包含 t-scoring/t-valuation/t-risk，
            // 这些上游 ToolNode 的输出就是 compute_scoring/compute_valuation/
            // compute_portfolio_risk 的计算结果。LLM 重新调用这些工具属于重复获取，
            // 且 compute_* 工具需要 stock_code 参数，Agent 节点工具参数由 LLM 自主
            // 生成，容易传空值触发无效重试。rule-check 的职责是对照硬性规则阈值
            // 检查交易方案是否违规，所需技术指标/估值/风控数据已通过 context_sources
            // 注入，无需重新计算。
            a.config.tools = vec![];
            a.config.exposed_tools = vec![];
            a.config.max_tool_rounds = Some(0);
            a.config.system_prompt = format!(
                "{}\n\n--- 数据约束 ---\n\
                 你是硬性规则检查员，所有需要的数据已通过输入上下文注入，禁止调用任何工具重新获取或计算。\n\
                 - 技术指标(RSI/MACD/乖离率等): 参考 t-scoring 的 indicators\n\
                 - 估值数据(DCF/格雷厄姆/F-Score): 参考 t-valuation\n\
                 - 风险指标(波动率/最大回撤/夏普比率): 参考 t-risk\n\
                 - 交易方案(入场价/目标价/止损价): 参考 trader\n\
                 - 组合决策(action/positionPct): 参考 portfolio-risk-gate\n\
                 基于上述数据直接检查交易方案是否违反硬性规则（RSI超买/乖离率追高/缺失止损/放量下跌/空头排列），\n\
                 输出 violations / corrections / force_signals。",
                a.config.system_prompt
            );
        }
        nodes.push(rc);
        // ── P1-E13: portfolio-risk-gate → rule-check（替代原 portfolio-mgr → rule-check）──
        // rule-check 现在从组合风控门获取最终决策（含风控调整），而非直接从 portfolio-mgr
        edges.push(edge("e-risk-gate-rule-check", "portfolio-risk-gate", rc_id));
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
            position: Position { x: 700.0, y: 4500.0 },
            retry: RetryConfig::default(),
            timeout: Some(10),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: SwitchNodeConfig {
            // data-quality 是 CodeNode（Rhai），输出结构为
            //   {status, result: {grade, score, ...}, input_params, node_id, params}
            // resolve_var_path 需要导航到 .result.grade 才能取到等级字段。
            // 旧值 "data-quality.grade" 无法穿透 CodeNode 的 result 包装，
            // 导致 A 级数据被误判为低质量，路由到 quality-fallback 保守路径
            // （其 prompt 不输出 confidence 字段 → 前端显示"持有 0%"）。
            input_var: "data-quality.result.grade".into(),
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
            // P0 修复(2026-07-22): 移除 td_quote/td_kline/td_score——context_sources 已包含
            // t-scoring/t-valuation/t-risk，上游数据已注入。原配置与 trader 同款问题：
            // LLM 重新获取数据会传入空 stock_code，触发无效重试。
            a.config.tools = vec![];
            a.config.system_prompt =
                "数据质量评估为 D 或 F，上游分析数据不可靠。你需要在数据不足的情况下做出最保守的投资决策。\
                 所有需要的数据已通过输入上下文注入（t-scoring 的 currentPrice/indicators/totalScore、\
                 t-valuation 的估值、t-risk 的风险评分），禁止调用任何工具重新获取数据。\
                 输出JSON格式（严格模式）：{\"action\":\"持有/减持/卖出\",\"positionPct\":0-20,\"confidence\":20-40,\"riskLevel\":\"高风险\",\"reasoning\":\"保守决策理由\"}\
                 规则：action 只能是'持有'/'减持'/'卖出'（禁止买入/增持）；positionPct 0-20（保守低仓位）；\
                 confidence 20-40（数据不足时置信度低，D级给30-40，F级给20-30）；riskLevel 固定为'高风险'。\
                 只输出上述JSON对象，前后不要有任何其他文字"
                    .to_string();
            a.config.exposed_tools = vec![];
            a.config.max_tool_rounds = Some(0);
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
    // 在 portfolio-risk-gate 组合风控门完成后，用 LLM 生成决策依据说明书 + 规则追溯码
    // 输入: portfolio-risk-gate 的 final_action / confidence / reasoning / decision_trail
    // 输出: 自然语言解释文案，带规则追溯码 R-xxx
    {
        let de_id = "decision-explainer";
        let de_title = "决策解释：将硬裁决结果翻译为自然语言说明书，附带规则追溯码";
        let mut de = agent(de_id, de_title, "explainer", None, 700.0, 4400.0);
        if let WorkflowNode::Agent(ref mut a) = de {
            a.config.context_sources = vec![
                "portfolio-risk-gate".into(),
                "rule-check".into(),
                "t-scoring".into(),
                "t-risk".into(),
            ];
            a.config.model_role = Some("explainer".into());
            a.config.output_mode = OutputMode::Json;
            // P0 修复(2026-07-22): 移除 td_quote/td_score——context_sources 已包含 t-scoring，
            // 上游 currentPrice/indicators/totalScore 已注入。explainer 职责是翻译规则引擎
            // 裁决结果为自然语言，不需要重新获取行情或计算评分。
            a.config.tools = vec![];
            a.config.exposed_tools = vec![];
            a.config.max_tool_rounds = Some(0);
            a.config.system_prompt = format!(
                "{}\n{}",
                "你是投资决策解释官。输入是符号系统（Rhai 规则引擎）的硬裁决结果，你的任务是将裁决翻译为用户可读的决策依据说明书。",
                "输出 JSON 格式（严格模式）：\n\
                 {\n\
                   \"summary\": \"一段话摘要（50-100字），包含最终行动、仓位、置信度\",\n\
                   \"explanation\": \"详细的决策依据说明（200-300字），解释为什么做出这个决策\",\n\
                   \"rule_trace\": [\n\
                     {\"rule_id\": \"R-xxx\", \"status\": \"PASS/VETOED/DOWNGRADED\", \"description\": \"规则的通俗解释\"}\n\
                   ],\n\
                   \"risk_comment\": \"风险提示（如有）\",\n\
                   \"confidence_note\": \"置信度解读\"\n\
                 }\n\
                 规则追溯码对照表：\n\
                 R-200: 高风险风控否决 | R-201: 空头预测否决 | R-202: trader数据异常\n\
                 R-203: 因子权重坍缩 | R-204: 零仓位修正 | R-205: 单点不可信部分降级\n\
                 R-401: RSI>80追高风险否决 | R-402: RSI<20恐慌否决\n\
                 R-403: MACD顶背离否决 | R-404: MACD底背离否决 | R-405: RSI+MACD双重超买\n\
                 只输出上述JSON对象，前后不要有任何其他文字"
            );
            a.config.input_mapping = [
                // P1-E13: decision-explainer 现在从 portfolio-risk-gate 读取最终决策
                // portfolio-risk-gate 是 CodeNode，输出结构为
                //   {status, result: {action, confidence, ..., risk_gate: {...}}, ...}
                // 保留了 portfolio-mgr 的所有字段，并覆盖了被风控门调整的字段
                ("pm_action", "portfolio-risk-gate.result.action"),
                ("pm_confidence", "portfolio-risk-gate.result.confidence"),
                ("pm_position_pct", "portfolio-risk-gate.result.positionPct"),
                ("pm_reasoning", "portfolio-risk-gate.result.reasoning"),
                ("pm_risk_level", "portfolio-risk-gate.result.riskLevel"),
                ("pm_stop_loss", "portfolio-risk-gate.result.stopLossPct"),
                ("pm_take_profit", "portfolio-risk-gate.result.takeProfitPct"),
                ("pm_decision_trail", "portfolio-risk-gate.result.decision_trail"),
                ("pm_target_timeframe", "portfolio-risk-gate.result.targetTimeframe"),
                ("pm_computation_logs", "portfolio-risk-gate.result.computation_logs"),
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
            position: Position { x: 300.0, y: 4500.0 },
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
    // 将完整分析结果（portfolio-risk-gate 最终决策）写入 SQLite history 表，供后续回测/复盘引用。
    nodes.push(WorkflowNode::Storage(StorageNode {
        base: WorkflowNodeBase {
            id: "store-result".into(),
            title: "分析结果持久化".into(),
            description: Some("写入分析结果到历史记录表".into()),
            position: Position { x: 300.0, y: 4800.0 },
            retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: StorageNodeConfig {
            backend: "sqlite".into(),
            operation: "insert".into(),
            input_var: "portfolio-risk-gate".into(),
            collection: "analysis_history".into(),
            key_var: None,
            output_var: "storage-result".into(),
        },
    }));
    edges.push(edge("e-notify-store-result", "notify-result", "store-result"));
    // store-result 直接从 portfolio-risk-gate 取决策变量，绕过 state.variables 查找
    edges.push(edge("e-risk-gate-store-result", "portfolio-risk-gate", "store-result"));

    // EndNode: 把 portfolio-risk-gate 输出提升为工作流顶层输出
    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end-output".into(),
            title: "最终输出".into(),
            description: Some("将 portfolio-risk-gate 决策结果提升到工作流输出".into()),
            position: Position { x: 300.0, y: 5100.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: EndNodeConfig { output_var: Some("portfolio-risk-gate".into()) },
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
    output_props.insert(
        "riskSource".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("风险分类来源: 算法/LLM回退/无数据".to_string()),
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

    // ── 模板变量定义（拆分到 seed_variables.rs） ──
    use super::seed_variables::build_template_variables;
    let variables: Vec<Variable> = build_template_variables();

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
        retry_policy: Some(WorkflowRetryPolicy {
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
                WorkflowNode::MultiAgent(n) => {
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
        let sub_nodes: Vec<WorkflowNode> =
            nodes.iter().filter(|n| child_node_ids.contains(n.base_id())).cloned().collect();
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
    let _ = workflow_template::Entity::delete_by_id(TEMPLATE_ID).exec(db).await;
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.to_string()),
        cluster_id: Set(Some("equity".to_string())),
        route_path: Set(Some("/finance/equity/multi-dim-analysis".to_string())),
        name: Set("A股多维度分析".to_string()),
        description: Set(Some(
            "10 维度分析师 → LLM 智能辩论 → 价值投资（巴菲特框架）→ 3 风险维度 → Rhai 评分 → 交易方案 → 投资决策"
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
                    // 标准单 cron 字段，供 ScheduleTriggerConfig 反序列化使用
                    "cron": "0 9 * * 1-5",
                    // 多时段 map，供独立调度器（start_stock_pipeline）使用
                    "schedules": {
                        "morning": "0 9 * * 1-5",
                        "afternoon": "0 14 * * 1-5",
                    },
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
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("写入工作流模板失败: {e}")))?;

    tracing::info!(
        "[stock_analysis_setup] 股票分析工作流模板已种子化完成: TEMPLATE_ID={TEMPLATE_ID}, VERSION={TEMPLATE_VERSION}"
    );
    Ok(())
}
