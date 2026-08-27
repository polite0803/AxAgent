//! 股票分析专家/角色/Profile 自动种子化到 agency_experts/agent_roles/agent_profiles 表。
//! 使用 include_str! 编译期嵌入 .md 内容，打包后无需文件 I/O。

use crate::commands::error_code::stock_setup;

pub(crate) async fn seed_serenity_screening_workflow_template(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), String> {
    use crate::commands::error::ErrorResponse;
    use axagent_entities::workflow_template;
    use axagent_harness::HallucinationGuardConfig;
    use axagent_harness::workflow_types::{
        AgentNode, AgentNodeConfig, CodeNode, CodeNodeConfig, EdgeType, JsonSchema,
        JsonSchemaProperty, OutputMode, Position, RetryConfig, ToolDef, ToolNode, ToolNodeConfig,
        TriggerConfig, TriggerNode, TriggerType, Variable, WorkflowEdge, WorkflowNode,
        WorkflowNodeBase,
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    const TEMPLATE_ID: &str = "serenity-screening";
    // 每次修改 Rhai 脚本或节点拓扑后+1，强制模板重新写入
    // v2: 重构趋势智选，6策略枚举，多策略评分，策略标记
    // v3: 修复 t-policy-news 的 search_news input_mapping 参数名 query→keyword
    // v16: DB 历史版本已到 v15（重构时版本号曾重置为 2，导致 15>=2/15>=3 永久跳过重建，
    //      所有模板修复（keyword/闭包脚本）都无法生效）。必须 >15 强制重建。
    //      - t-policy-news keyword 修复（v3 内容）
    //      - strategy-scorer.rhai 闭包版（v16 内容：Rhai fn 无法访问注入变量，改闭包）
    // v17: 移除 14 个硬编码行业节点（t-baseline-* / t-signal-*）及 ref_*_code 变量——
    //      行业信息改由知识库（RAG）提供，财务指标由 agent 按需调工具获取
    // v18: strategy-scorer.rhai 归一化 chain_analysis（字符串→JSON 对象，防
    //      "Data type incorrect: string (expecting i64)"）；find→contains
    //      （Rhai 无内置 find）；配合 agent_executor 支持 ```json 围栏拆包
    // v31: 🚨 版本门教训 2：2026-07-31 实测 DB 版本已涨到 30（用户前端保存模板会递增
    //      version），18>=18 又跳过重建。从此版本号直接取"当前 DB 版本 + 1"，
    //      修改模板前先查 DB：SELECT version FROM workflow_templates WHERE id=...
    //      - v19: t-policy-news keyword 字面值→policy_news_keywords 模板变量引用
    //        （tool_executor resolve_var_path 只解析变量路径，字面值查表失败→null，
    //        连续 6 轮日志 keyword= 空的真正根因）
    //      - v20: 移除 stock-candidate-mapper 硬编码 doubao 模型（用户未配火山 →
    //        503 no_available_account 阻断整条链），回落全局默认模型
    // v38 定稿：版本门简化（2026-07-31 20:47 用户指正"搞复杂了"）——
    //   根因其实是前端保存递增 version（dao repo 原 version+1），导致数字版本门失效。
    //   修复只需一处：dao repo update 保存时【不递增 version】（version 只由 seed 写入）。
    //   此后 `existing.version >= TEMPLATE_VERSION` 版本门天然稳定：前端 auto-save 不会
    //   推高 version 误挡 seed 重建；用户编辑（加知识源/改参数）在 seed 未升版本号时
    //   也不会被 seed 覆盖。seed 要更新模板 → 版本号+1 即可。曾尝试的内容比对/接管标记
    //   （mission_hash）/preset 只读保护均已回退删除，避免过度设计。
    // v40: 20:55 日志确认 t-policy-news 已引用 policy_news_keywords，但 DB variables 缺该
    //      变量（前端保存写回旧 variables，且 variables 列含历史 GBK 乱码无法 SQL 修补）→
    //      升 40 强制重建写入完整干净 variables。另 data-verifier.rhai has()→in 修复。
    // v41: 21:47 轮（重启后）candidate-mapper 出 4 候选（中际旭创/源杰科技/天孚通信/仕佳光子）
    //      但 LLM 把 arguments 直接输出为裸数组 → agent_executor 拆包后 content=候选数组，
    //      serenity_extract 只认对象形态 → 种子不注入（已由 serenity.rs 裸数组分支兜住）。
    //      prompt 强化：arguments 必须为 {candidates, summary} 对象、禁裸数组、禁尾逗号。
    //      另 harness json_parse 加尾逗号容忍（c-scorer-trend2 trailing comma 根治）。
    // v42: 22:11 轮 5 个 c-scorer 的 industry_ranking/trend_strategy/w_* 输入全 null 实锤：
    //      code_executor 只注入 edges 直接上游输出（get_node_dependency_results），c-scorer
    //      是 Code 节点吃不到 V57 context_sources 机制（仅 Agent 节点）→ 非直接上游
    //      t-industry-rank / a-trend-scanner 不注入 → resolve 全 null → 评分 industry_momentum
    //      固定 30、matched_industry 空。修复：① c-scorer 手动补边 t-industry-rank →
    //      c-scorer、a-trend-scanner → c-scorer；② 模板 variables 补 w_supply/w_demand/
    //      w_irreplace（默认 0.35/0.35/0.30，前端可调）。
    // v43: 22:25 轮 c-scorer-trend2 报 "For loop expects iterable type (line 96)"——
    //      实锤 ToolNode 输出经 ToolResult.content(String) 注入 scope 后是 JSON 文本字符串，
    //      industry_ranking `!= ()` 通过后 for 对字符串迭代炸。strategy-scorer.rhai v3 防御：
    //      新增 to_array/to_f64 辅助；industry_ranking 字符串→json_parse、对象包装→取内层数组；
    //      chain_nodes 及 policy/earnings/capital/event/technical 全部数组字段统一 to_array；
    //      w_supply/w_demand/w_irreplace 权重 to_f64 防 i64/字符串数字。
    //      另 consistency-check.rhai find()→contains()（Rhai 无内置 find，v18 记录实锤，
    //      c-scorer 修复后 consistency-check 将拿到真实数据执行到 find → 下一个炸点，同轮修）。
    // v44: 23:05 轮候选校验 总量=0 实锤：agent_executor 拆包 tool_json 后 content 是
    //      arguments 对象文本字符串，data-verifier 的 input_mapping 路径
    //      content.arguments.candidates / content.candidates 全部 resolve 失败（arguments
    //      层已剥 / 字符串无 candidates 字段）→ early return 空数组 → serenity 优先取
    //      c-data-verifier 吞掉真候选。修复：① data-verifier input_mapping 增加
    //      candidates_raw（a-candidate-mapper.content 整体）由脚本 json_parse 兜底；
    //      ② data-verifier.rhai 非 bottleneck 分支 c.get("code")/c.get("name") 改为
    //      in 检查（Rhai map 无 get，此前一旦走该分支必 Function not found）；
    //      ③ serenity.rs candidates_raw 对 data-verifier 空数组结果 filter 回退
    //      a-candidate-mapper 原始输出。
    // v47: 对话式主题荐股——新增 user_themes 变量 + a-trend-scanner 输入分支
    //      + prompt 增加用户主题优先指令
    const TEMPLATE_VERSION: i32 = 47;

    let now = chrono::Utc::now().timestamp_millis();

    // ── ToolDef 定义 ──
    let td_industry = ToolDef {
        name: "get_industry_ranking".into(),
        description: Some("获取行业涨跌排名".into()),
        parameters: None,
    };
    let td_cls = ToolDef {
        name: "get_cls_flash".into(),
        description: Some("获取财联社实时快讯".into()),
        parameters: None,
    };
    let td_concept = ToolDef {
        name: "get_stock_concept_blocks".into(),
        description: Some("获取概念板块归属（需股票代码）".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_dragon = ToolDef {
        name: "get_market_dragon_tiger".into(),
        description: Some("获取龙虎榜数据".into()),
        parameters: None,
    };
    let td_north = ToolDef {
        name: "get_north_bound_flow".into(),
        description: Some("获取北向资金成交额（净流入2024-08起停披，返回成交额序列）".into()),
        parameters: None,
    };
    let td_fin = ToolDef {
        name: "get_stock_financials".into(),
        description: Some("获取财务数据：营收、净利润、EPS、ROE、毛利率等".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_quote = ToolDef {
        name: "get_stock_quote".into(),
        description: Some("获取股票实时行情".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_visits = ToolDef {
        name: "get_stock_institutional_visits".into(),
        description: Some("获取机构调研数据".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    // 新闻 + 研报工具，供 Agent 节点动态搜索验证（催化剂/CapEx/退出信号）
    let td_serenity_news = ToolDef {
        name: "get_stock_news".into(),
        description: Some("获取个股近期新闻公告，验证催化剂/退出信号".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([
                (
                    "stock_code".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("6位股票代码".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "limit".into(),
                    JsonSchemaProperty {
                        schema_type: "integer".into(),
                        description: Some("返回数量".into()),
                        default: Some(serde_json::json!(30)),
                        enum_values: None,
                        format: None,
                    },
                ),
            ])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_serenity_research = ToolDef {
        name: "get_stock_research_reports".into(),
        description: Some("获取券商研报，验证需求/壁垒/CapEx逻辑".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    // 关键词新闻搜索工具，供 Agent 验证催化剂/CapEx/行业趋势
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

    // 关注度评分工具（方案B）
    let td_attention = ToolDef {
        name: "compute_attention_score".into(),
        description: Some("计算个股关注度评分 0-100，越低越冷门，用于验证低关注度因子".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    // 行业竞争地位分析工具（方案C）
    let td_industry_pos = ToolDef {
        name: "compute_industry_position".into(),
        description: Some(
            "行业竞争地位分析：同行对比毛利率/ROE，产能指标（资本开支/折旧比）".into(),
        ),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "stock_code".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("6位股票代码".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };

    // 退出信号检查工具（Phase 3）
    let td_exit = ToolDef {
        name: "check_exit_signals".into(),
        description: Some(
            "检查个股退出信号：价格止损、技术替代新闻、毛利率趋势。返回 exit_urgency".into(),
        ),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([
                (
                    "stock_code".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("6位股票代码".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "entry_price".into(),
                    JsonSchemaProperty {
                        schema_type: "number".into(),
                        description: Some("买入价（用于计算止损触发）".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "stop_loss_price".into(),
                    JsonSchemaProperty {
                        schema_type: "number".into(),
                        description: Some("止损价".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
            ])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };

    // 回馈闭环工具
    let td_perf = ToolDef {
        name: "compute_serenity_performance".into(),
        description: Some("计算 Serenity 候选推荐后表现".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([
                (
                    "stock_code".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("6位股票代码".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "recommend_date".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("推荐日期 YYYY-MM-DD".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
            ])),
            required: Some(vec!["stock_code".into(), "recommend_date".into()]),
            items: None,
        }),
    };
    let td_cat = ToolDef {
        name: "verify_catalysts".into(),
        description: Some("验证 Serenity 候选的催化剂是否兑现".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([
                (
                    "stock_code".into(),
                    JsonSchemaProperty {
                        schema_type: "string".into(),
                        description: Some("6位股票代码".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
                (
                    "catalyst_descriptions".into(),
                    JsonSchemaProperty {
                        schema_type: "array".into(),
                        description: Some("催化剂描述列表".into()),
                        default: None,
                        enum_values: None,
                        format: None,
                    },
                ),
            ])),
            required: Some(vec!["stock_code".into()]),
            items: None,
        }),
    };
    let td_opt = ToolDef {
        name: "optimize_attention_weights".into(),
        description: Some("基于历史表现调优关注度评分权重".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "samples".into(),
                JsonSchemaProperty {
                    schema_type: "array".into(),
                    description: Some("样本列表".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["samples".into()]),
            items: None,
        }),
    };

    // 搜索股票代码工具（借助 NeoDataVendor 末位路由覆盖全球供应链公司）
    let td_search_stock = ToolDef {
        name: "search_stock".into(),
        description: Some("搜索股票代码：输入公司中文名/英文名，返回6位A股代码或港股(如00700.HK)/美股(如TSM.US)代码".into()),
        parameters: Some(JsonSchema {
            schema_type: "object".into(),
            description: None,
            properties: Some(std::collections::HashMap::from([(
                "keyword".into(),
                JsonSchemaProperty {
                    schema_type: "string".into(),
                    description: Some("公司名称（支持中英文，如\"台积电\"、\"NVIDIA\"、\"三星电子\"）".into()),
                    default: None,
                    enum_values: None,
                    format: None,
                },
            )])),
            required: Some(vec!["keyword".into()]),
            items: None,
        }),
    };

    let tool_defs: Vec<ToolDef> = vec![
        td_industry,
        td_cls,
        td_concept,
        td_dragon,
        td_north,
        td_fin,
        td_quote,
        td_visits,
        td_serenity_news,
        td_serenity_research,
        td_search_news,
        td_search_stock,
        td_attention,
        td_industry_pos,
        td_exit,
        td_perf,
        td_cat,
        td_opt,
    ];
    let tool_defs_json = serde_json::to_string(&tool_defs).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化 ToolDef 失败: {e}"))
    })?;

    // ── 快捷构建函数 ──
    let tool_node = |id: &str,
                     title: &str,
                     tool_name: &str,
                     output_var: &str,
                     x: f64,
                     y: f64|
     -> WorkflowNode {
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("获取数据: {tool_name}")),
                position: Position { x, y },
                retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
                timeout: Some(120),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: tool_name.into(),
                input_mapping: std::collections::HashMap::new(),
                output_var: output_var.into(),
            },
        })
    };

    let agent_node = |id: &str,
                      title: &str,
                      expert_id: &str,
                      system_prompt: &str,
                      context_sources: Vec<&str>,
                      input_mapping: std::collections::HashMap<String, String>,
                      x: f64,
                      y: f64|
     -> WorkflowNode {
        WorkflowNode::Agent(AgentNode {
            base: WorkflowNodeBase {
                id: id.into(),
                title: title.into(),
                description: Some(format!("Serenity 分析: {expert_id}")),
                position: Position { x, y },
                retry: RetryConfig { enabled: true, max_retries: 1, ..Default::default() },
                timeout: Some(600),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: AgentNodeConfig {
                system_prompt: system_prompt.into(),
                context_sources: context_sources.into_iter().map(String::from).collect(),
                input_mapping,
                output_var: id.into(),
                model: None,
                temperature: Some(0.3),
                max_tokens: Some(32768),
                tools: vec![],
                exposed_tools: vec![],
                output_mode: OutputMode::Json,
                agent_profile_id: Some(format!("stock-{expert_id}")),
                max_tool_rounds: Some(8),
                execution_mode: None,
                rag_source_ids: vec![],
                // FIX-05: 显式设置 model_role，确保 domain_constraints 按 stock-analyst 角色注入
                model_role: Some("stock-analyst".into()),
                consistency_check: None,
                // V74 关闭: hallucination_guard 锚定检查（同 stock-analysis，实测误报率 ~100%）
                hallucination_guard: Some(HallucinationGuardConfig {
                    enabled: false,
                    match_threshold: 0.4,
                }),
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
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

    // ── 构建节点 ──
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    // Trigger: 手动触发，无需参数（自驱动扫描市场）
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".into(),
            title: "启动 Serenity 筛选".into(),
            description: Some("自动扫描市场数据，发现产业瓶颈机会".into()),
            position: Position { x: 340.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({
                "description": "Serenity 瓶颈筛选: 从市场数据中识别瓶颈环节候选公司",
                "required_params": []
            }),
        },
    }));

    // ── Phase 0: 数据采集工具（并行） ──
    // 2026-08-01 v2 恢复 t-northbound：北向净流入 2024-08-16 起监管停披，但**成交额/领涨股
    // 仍披露**（datacenter-web RPT_MUTUAL_DEAL_HISTORY，eastmoney get_north_bound_flow v3
    // 已改返回成交额序列并标注 timestamp）。北向成交活跃度仍是有价值的资金面信号。
    let t_names = [
        ("t-industry-rank", "行业排名", "get_industry_ranking", "t-industry-rank", 240.0, 80.0),
        ("t-cls-flash", "实时快讯", "get_cls_flash", "t-cls-flash", 440.0, 80.0),
        ("t-northbound", "北向资金", "get_north_bound_flow", "t-northbound", 840.0, 80.0),
        ("t-policy-news", "政策新闻", "search_news", "t-policy-news", 1040.0, 80.0),
    ];
    let t_trend_ids: Vec<&str> = t_names.iter().map(|(id, _, _, _, _, _)| *id).collect();
    for (id, title, tool, output, x, y) in &t_names {
        if *id == "t-policy-news" {
            // search_news 需要固定搜索关键词。
            // ⚠️ 必须引用模板变量 policy_news_keywords：tool_executor 的 resolve_var_path 只按
            // 变量路径解析，直接写字面值（含空格）会被当成变量名查表 → 查不到 → 参数为 null，
            // 导致 search_news 收到空 keyword、4 个 vendor 全部"新闻搜索无结果"（连续 6 轮日志）。
            let mut im = std::collections::HashMap::new();
            im.insert("keyword".to_string(), "policy_news_keywords".to_string());
            nodes.push(WorkflowNode::Tool(ToolNode {
                base: WorkflowNodeBase {
                    id: id.to_string(),
                    title: title.to_string(),
                    description: Some("搜索近期政策类新闻".into()),
                    position: Position { x: *x, y: *y },
                    retry: RetryConfig { enabled: true, max_retries: 2, ..Default::default() },
                    timeout: Some(120),
                    enabled: true,
                    parent_id: None,
                    compensation: None,
                    continue_on_fail: true,
                },
                config: ToolNodeConfig {
                    tool_name: "search_news".into(),
                    input_mapping: im,
                    output_var: id.to_string(),
                },
            }));
        } else {
            nodes.push(tool_node(id, title, tool, output, *x, *y));
        }
        edges.push(edge(&format!("e-trigger-{id}"), "trigger", id));
    }

    // ── a-trend-scanner: 综合分析，输出 2-3 个趋势 ──
    // 强约束输出：必须且只能输出一个 tool_json 代码块，无任何前后文。
    // tool_json 块由项目 IR Normalizer 直接解析为 ContentBlock::ToolUse。
    let trend_scanner_prompt = "你的任务：基于上游提供的实时市场数据，识别当前A股市场最具潜力的 2-3 个产业方向。\
         \n\n\
         **用户指定主题优先（重要）**：若 `user_themes` 非空（用户指定主题，如 ['AI SSD','存储芯片']），你必须\
         **优先围绕用户主题**展开：直接基于该主题识别产业趋势并完成结构化输出（trend_name 需包含主题关键词、strategy_type、\
         bottleneck_candidate、demand_evidence 必须给可验证硬证据或如实标注证据缺口）。此时行业排名/快讯/北向数据仅作辅助验证，\
         不因数据缺失而拒绝输出。若 `user_themes` 为空，按原逻辑从市场数据自由扫描。\
         \n\n\
         **数据可用性检查（重要——违反将产生无效趋势）**：\
         \n\
         - 表变量 `industry_ranking` 中包含了从交易所获取的实时行业排名数据（涨跌幅、主力资金流入）。\
         \n- 如果 `industry_ranking` 为空数组 `[]` 或全部数值为 0，说明数据接口不可用。\
         **此时不要用训练知识编造趋势**。直接返回 {\"trends\": [], \"summary\": \"实时市场数据不可用，无法识别趋势\"}。\
         \n- 如果 `industry_ranking` 包含有效数据（至少 1 个行业的 changePct > 0），\
         你必须以这些真实数据为主要依据。涨幅最大且资金流入为正的板块是你的优先分析对象。\
         不要忽略真实数据去编造训练数据中的趋势。\
         \n\n\
         核心原则：\n\
         1. 优先选择行业排名中涨幅 2-15% 且主力资金净流入为正的板块（\"萌芽→加速\"阶段）。\n\
         2. 排除已过度上涨的赛道：排名中 1 月涨幅 > 30% 的板块直接排除。\n\
         3. 每个趋势必须有可验证的 CapEx/订单/政策证据支撑——纯 LLM 推测不可接受。\n\
         4. 每个趋势必须给出明确的上下游因果链。\n\
         5. **策略分类**：每个趋势必须标注 strategy_type（bottleneck/policy/earnings/capital/event/technical）。\n\
            bottleneck=产业链供给瓶颈（首选），policy=政策驱动，earnings=业绩驱动，capital=资金面驱动。\n\
         6. 必须输出至少一个 bottleneck_candidate（初步判断的瓶颈环节）。\n\
         \n\
         ============== 输出格式强约束（必须严格遵守） ==============\n\
         1. 你的回复必须且只能包含一个代码块，开头三个反引号紧跟 tool_json。\n\
         2. 代码块内容为单一 JSON 对象，结构：{\"name\": \"submit_trends\", \"arguments\": <数据>}\n\
         3. <数据> 字段即下面的趋势数据。\n\
         4. 代码块外禁止任何文字：不要写\"以下是...\"、\"输出：\"、注释、解释、前缀、后缀。\n\
         5. 字段值为空时用 null，不要省略字段。\n\
         6. 数字字段（confidence 等）必须是 JSON 数字，不要加引号。\n\
         7. 严禁在 JSON 字符串值中夹带思考文字或自述注解。\n\
         ============================================================\n\
         \n\
         <数据> 结构：\n\
         {\"trends\": [{\"trend_name\": \"...\", \"confidence\": 75, \"phase\": \"accelerating\",\
         \"core_logic\": \"...\", \"causal_chain\": \"...\",\
         \"strategy_type\": \"bottleneck | policy | earnings | capital | event | technical\",\
         \"bottleneck_candidate\": \"...\", \"bottleneck_rationale\": \"...\",\
         \"demand_evidence\": {\"type\": \"capex | policy_mandate | order_backlog\",\
         \"source\": \"具体证据来源\", \"confidence\": 75, \"detail\": \"...\"},\
         \"downstream_giants\": [\"直接受益/推动的下游巨头名称\"]}],\
         \"summary\": \"最终判断总结，如果 trends 为空时说明具体原因\"}\n\
         \n\n\
         ⚠️ 终极约束（违反将导致整个工作流报废）：\n\
         1. 你**唯一**合法的输出方式是一个 tool_json 代码块。\
         绝不输出任何自然语言、注释、前言后语、拒绝文本。\n\
         2. '抱歉'、'我无法回答'、'数据不足'等自然语言文本会直接破坏下游所有节点——\
         这将导致整个分析流程失败，责任在你。不要这样做。\n\
         3. 如果实在无法识别趋势，返回 {\"trends\": [], \"summary\": \"原因\"}，\
         这是合法的结构化输出，下游系统能正确处理。\n\
         4. 你是一个函数，你的输出必须是 JSON。你不是在对话，你是在向系统返回值。";
    // 注入 industry_ranking 作为结构化变量，取代 LLM 自行猜测
    let mut ts_input_mapping = std::collections::HashMap::new();
    ts_input_mapping.insert("industry_ranking".to_string(), "t-industry-rank.result".to_string());
    ts_input_mapping.insert("policy_news".to_string(), "t-policy-news.result".to_string());
    ts_input_mapping.insert("user_themes".to_string(), "user_themes".to_string());
    nodes.push(agent_node(
        "a-trend-scanner",
        "产业趋势扫描",
        "trend-scanner",
        trend_scanner_prompt,
        t_trend_ids.clone(),
        ts_input_mapping,
        340.0,
        180.0,
    ));
    for tid in &t_trend_ids {
        edges.push(edge(&format!("e-{tid}-a-trend-scanner"), tid, "a-trend-scanner"));
    }

    // ── Phase 1: 对每个趋势拆解产业链+瓶颈鉴定 ──
    // 使用 5 个并行的 chain-decomposer + 5 个 chokepoint-identifier
    // 每个槽位对应一个趋势索引（0-4）。trend-scanner 通常输出 2-5 个趋势，
    // prompt 已处理"趋势不存在"的情况（返回空 chain_nodes）。
    // 收集所有 scorer 的 output_var 供 candidate-mapper 消费
    //
    // v17 移除（2026-07-31）：原 Phase 0b/0c 的 14 个硬编码行业节点
    // （t-baseline-* / t-signal-*，覆盖固定 7 行业：半导体/电池/化工/医药/军工/消费电子/汽车）：
    // - t-signal-* 定义后零下游引用，纯白跑
    // - t-baseline-* 只覆盖预置 7 行业，LLM 识别的趋势行业（如 AI 算力/液冷/光模块）匹配不上，
    //   与「从知识库获取行业信息」的设计意图冲突
    // 替代：行业/产业链信息由 agent 节点知识源（RAG）检索 kb 获取；
    //      财务指标由 chain-decomposer 按需调用 get_stock_financials 等工具拉取真实数据。
    let trend_names = ["trend1", "trend2", "trend3", "trend4", "trend5"];
    let trend_x_positions = [60.0, 220.0, 380.0, 540.0, 700.0];

    for (i, tn) in trend_names.iter().enumerate() {
        let decomposer_id = format!("a-chain-{tn}");
        let code_node_id = format!("c-scorer-{tn}");

        // ── trend-analyzer: 多策略趋势分析 Agent（根据 strategy_type 分支）──
        let decomposer_prompt = format!(
            "你的任务：对上游 a-trend-scanner 输出的趋势 #{i} 进行产业链拆解。\
             将产业从上到下拆解为 5-8 个关键环节，标注每个环节的供应商数量、技术壁垒、扩产周期。\
             \n\n\
             核心要求：\n\
             1. 拆解到具体产品或工艺层面（如 HBM3E 环氧塑封料）。\n\
             2. 每个环节必须标注 global_supplier_count / tech_barrier / expansion_cycle_months。\n\
             3. 标注 bottleneck_potential（high/medium/low）及理由。\n\
             4. **额外标注每个环节的需求验证信息**：直接下游厂商是谁、最终需求驱动方、是否有已公开的\
             订单/合同负债/CapEx 支撑。\n\
             5. **【尽量】调用工具获取真实财务数据，无法获取时基于训练知识合理估算**。\
             对于 chain_nodes 中的代表性公司，你 **尽量**：\
             ① 用 search_stock 搜索公司中文名/英文名得到股票代码\
                （A 股返回 6 位代码；美股如 TSM.US；港股如 00700.HK）；\
             ② A 股公司用 get_stock_financials 获取真实财务数据；\
             ③ 用 compute_industry_position 获取行业竞争地位对比（毛利率/ROE/负债率排名）；\
             将结果嵌入对应 chain_node 的 financial_data 字段：\
             {{\"gross_margin\": 45.0, \"revenue_growth_yoy\": 25.0, \"debt_ratio\": 35.0, \
             \"roe\": 12.0, \"rnd_ratio\": 15.0, \"capex_dep_ratio\": 3.5}}。\
             非 A 股公司（如 NVIDIA、台积电）仅能获取行情数据（股价/PE/PB），财务数据标为 null。\
             如果工具调用超时或返回空，基于行业常识填入合理估算值，并在 financial_data 中标注 estimated。\
             **行业信息获取（重要）**：你的知识源（知识库）中检索到了与当前趋势/产业链相关的行业资料，\
             优先以知识库检索结果为行业背景依据。涉及个股财务指标时，调用 get_stock_financials 工具拉取真实数据；\
             涉及行业对比时，可调用 compute_industry_position 工具获取行业竞争地位数据。\
             工具不可用或超时时，再基于训练知识估算并在 financial_data 中标注 estimated。\
             严禁凭空编造不存在的财务数字。\
             \n\n\
             \n\n\
             重要：如果上游输出的 trends 数组为空或 trend #{i} 不存在，\
             不要输出自然语言拒绝。直接返回空 chain_nodes 数组的 JSON：\
             {{\"trend_name\": null, \"chain_nodes\": []}}。这是合法的结构化输出。\
             \n\n\
             ============== 输出格式强约束（必须严格遵守） ==============\n\
             1. 你的回复必须且只能包含一个代码块，开头三个反引号紧跟 tool_json。\n\
             2. 代码块内容为单一 JSON 对象，结构：{{\"name\": \"submit_chain\", \"arguments\": <数据>}}\n\
             3. <数据> 字段即下面的产业链数据。\n\
             4. 代码块外禁止任何文字：不要写\"以下是...\"、\"输出：\"、注释、解释、前缀、后缀。\n\
             5. 字段值为空时用 null，不要省略字段。\n\
             6. 数字字段（global_supplier_count、expansion_cycle_months 等）必须是 JSON 数字。\n\
             7. 严禁在 JSON 字符串值中夹带思考文字或自述注解。\n\
             ============================================================\n\
             \n\n\
             <数据> 结构：\n\
             {{\"trend_name\": \"...\", \"chain_nodes\": [{{\"node_name\": \"...\",\
             \"global_supplier_count\": 3, \"tech_barrier\": \"high\",\
             \"expansion_cycle_months\": 24, \"bottleneck_potential\": \"high\",\
             \"bottleneck_rationale\": \"...\",\
             \"financial_data\": {{\"gross_margin\": 45.0, \"revenue_growth_yoy\": 25.0, \
             \"debt_ratio\": 35.0, \"roe\": 12.0, \"rnd_ratio\": 15.0, \"capex_dep_ratio\": 3.5}},\
             \"demand_validation\": {{\"direct_downstream\": \"直接下游厂商\",\
             \"final_demand_driver\": \"最终需求驱动方\", \"demand_certainty\": \"high | medium | low\",\
             \"evidence\": \"关键证据，如英伟达 FY2025 CapEx $80B\",\
             \"order_visibility\": \"有已公开长协/订单 | 合同负债增长 | 产能预订 | 无公开证据\"}}}}]}}\n\n\
             如果 trend_strategy 不是 bottleneck (policy/earnings/capital/event/technical)：\n\
             输出格式见下方，**禁止**输出上述 chain_nodes 结构。\n\n\
             policy 策略输出：{{\"trend_name\": \"消费振兴\", \"policy_impact_score\": 75,\
             \"beneficiaries\": [{{\"sector\": \"食品饮料\",\
             \"stocks\": [{{\"code\": \"600887\", \"name\": \"伊利股份\"}}]}}],\
             \"chain_nodes\": []}}\n\n\
             earnings 策略输出：{{\"trend_name\": \"Q3 业绩超预期\",\
             \"candidates\": [{{\"code\": \"000858\", \"name\": \"五粮液\",\
             \"expected_beat_pct\": 15, \"reason\": \"动销超预期\"}}],\
             \"chain_nodes\": []}}\n\n\
             capital 策略输出：{{\"trend_name\": \"北向流入电力设备\",\
             \"sectors\": [{{\"name\": \"电力设备\", \"net_inflow\": 12.5, \"days\": 3,\
             \"volume_surge\": 1.3, \"stocks\": [{{\"code\": \"300750\"}}]}}],\
             \"chain_nodes\": []}}\n\n\
             event 策略输出：{{\"trend_name\": \"事件驱动\",\
             \"events\": [{{\"code\": \"002371\", \"name\": \"北方华创\",\
             \"event_type\": \"订单\", \"description\": \"获大单\",\
             \"timeframe\": \"short_term\", \"price_position\": \"not_run\"}}],\
             \"chain_nodes\": []}}\n\n\
             technical 策略输出：{{\"trend_name\": \"技术形态突破\",\
             \"signals\": [{{\"code\": \"300750\", \"name\": \"宁德时代\",\
             \"pattern\": \"突破\", \"volume_surge\": 1.5,\
             \"sector_aligned\": true, \"reason\": \"放量突破\"}}],\
             \"chain_nodes\": []}}"
        );
        // v17: 移除 7 个硬编码行业基线注入（baseline_*）——行业信息改由知识库 RAG 提供
        let mut cd_input_mapping = std::collections::HashMap::new();
        // 注入策略类型，Agent 根据 strategy_type 选择分析模式
        cd_input_mapping.insert(
            "trend_strategy".to_string(),
            format!("a-trend-scanner.content.trends[{i}].strategy_type"),
        );
        nodes.push(agent_node(
            &decomposer_id,
            &format!("产业链拆解 #{i}"),
            "chain-decomposer",
            &decomposer_prompt,
            vec!["a-trend-scanner"],
            cd_input_mapping,
            trend_x_positions[i],
            300.0,
        ));
        edges.push(edge(
            &format!("e-a-trend-scanner-{decomposer_id}"),
            "a-trend-scanner",
            &decomposer_id,
        ));

        // ── c-scorer: 统一多策略评分 CodeNode ──
        // 根据 trend_strategy 路由到 bottleneck/policy/earnings/capital/event/technical 评分逻辑
        let scorer_code = include_str!("../strategy-scorer.rhai").to_string();
        let mut sc_input_mapping = std::collections::HashMap::new();
        sc_input_mapping.insert("chain_analysis".to_string(), format!("{decomposer_id}.content"));
        sc_input_mapping
            .insert("industry_ranking".to_string(), "t-industry-rank.result".to_string());
        sc_input_mapping.insert(
            "trend_strategy".to_string(),
            format!("a-trend-scanner.content.trends[{i}].strategy_type"),
        );
        // 评分权重（来自模板变量，兜底 0.35/0.35/0.30）
        sc_input_mapping.insert("w_supply".to_string(), "w_supply".to_string());
        sc_input_mapping.insert("w_demand".to_string(), "w_demand".to_string());
        sc_input_mapping.insert("w_irreplace".to_string(), "w_irreplace".to_string());
        nodes.push(WorkflowNode::Code(CodeNode {
            base: WorkflowNodeBase {
                id: code_node_id.clone(),
                title: format!("策略评分 #{i}"),
                description: Some(
                    "多策略评分：bottleneck/policy/earnings/capital/event/technical".into(),
                ),
                position: Position { x: trend_x_positions[i] + 20.0, y: 360.0 },
                retry: RetryConfig::default(),
                timeout: Some(30),
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: CodeNodeConfig {
                language: "rhai".into(),
                code: scorer_code,
                output_var: code_node_id.clone(),
                tool_name: None,
                execute_directly: true,
                input_mapping: sc_input_mapping,
            },
        }));
        edges.push(edge(
            &format!("e-{decomposer_id}-{code_node_id}"),
            &decomposer_id,
            &code_node_id,
        ));
        // v42 修复：input_mapping 引用的非直接上游手动补边（MEMORY.md 纪律）。
        // code_executor 只注入 edges 直接上游输出（get_node_dependency_results），
        // c-scorer 是 Code 节点吃不到 V57 context_sources（仅 Agent 节点）→
        // industry_ranking（t-industry-rank.result）与 trend_strategy
        // （a-trend-scanner.content.trends[i].strategy_type）resolve 为 null。
        // 补边后这两个节点输出进入 deps 注入，评分可用真实行业数据。
        // 调度无环：t-industry-rank / a-trend-scanner 本就先于 a-chain-* 完成。
        edges.push(edge(
            &format!("e-t-industry-rank-{code_node_id}"),
            "t-industry-rank",
            &code_node_id,
        ));
        edges.push(edge(
            &format!("e-a-trend-scanner-{code_node_id}"),
            "a-trend-scanner",
            &code_node_id,
        ));
    }

    // ── FIX-04: 跨节点一致性检查 CodeNode ──
    // 检查各趋势分析 Agent 输出的数据是否一致
    // 输出: consistency_report 供 downstream 参考
    let consistency_code = include_str!("../consistency-check.rhai");
    let mut consistency_input_mapping = std::collections::HashMap::new();
    for tn in &trend_names {
        // v42 修复：原引用 a-analyzer-{tn}（不存在的节点名，真实节点是 a-chain-{tn}），
        // 导致 chain_node_trendX 全部 resolve null → 一致性检查空转（22:11 日志实锤
        // chain_node_trend1~5 全 null）。
        let did = format!("a-chain-{tn}");
        let cid = format!("c-scorer-{tn}");
        consistency_input_mapping.insert(format!("chain_node_{tn}"), format!("{did}.content"));
        consistency_input_mapping.insert(format!("strategy_{tn}"), format!("{cid}.result"));
    }
    // 注意：不再注入 trend_names/chain_node_keys 字面量
    // resolve_var_path 不支持非变量路径的字面量值，None→() 导致 json_parse(()) 失败
    // 新版 consistency-check.rhai v2 改为显式枚举 5 个趋势槽位，自动推导 trend_names
    nodes.push(WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "c-consistency-check".into(),
            title: "一致性检查".into(),
            description: Some("检查各 LLM 节点输出是否存在数据矛盾".into()),
            position: Position { x: 380.0, y: 520.0 },
            retry: RetryConfig::default(),
            timeout: Some(15),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: consistency_code.to_string(),
            output_var: "c-consistency-check".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: consistency_input_mapping,
        },
    }));
    // bottleneck-calc → consistency-check
    for tn in &trend_names {
        let cid = format!("c-scorer-{tn}");
        edges.push(edge(&format!("e-{cid}-c-consistency-check"), &cid, "c-consistency-check"));
    }
    // chain-decomposers → consistency-check
    // v42 修复：原引用 a-analyzer-{tn}（不存在的节点名）→ 边指向死节点，
    // a-chain 输出不注入 consistency 的 variables。改为真实节点 a-chain-{tn}。
    for tn in &trend_names {
        let did = format!("a-chain-{tn}");
        edges.push(edge(&format!("e-{did}-c-consistency-check"), &did, "c-consistency-check"));
    }
    // consistency-check → candidate-mapper (注入一致性报告到 context)
    edges.push(edge(
        "e-c-consistency-check-a-candidate-mapper",
        "c-consistency-check",
        "a-candidate-mapper",
    ));

    // ── Phase 2: 候选公司映射 ──
    // a-candidate-mapper: Agent 直接调用工具筛选，无需前置 ToolNode。
    // 综合所有瓶颈鉴定结果，输出最终候选股清单（含催化剂、退出信号、关注度评分）
    let mapper_prompt = "你的任务：综合所有瓶颈鉴定客观指标，对候选公司进行筛选和打分。\
         \n\n\
         你收到的输入：\n\
         - `strategy_trend1`~`strategy_trend5` 变量中包含 5 个趋势的策略评分结果\
         （`computed_nodes` 数组，每项含策略评分和标签）。\n\
         - `a-chain-trend*` 的 context 中包含产业链拆解详情（环节名称、供应商数量、技术壁垒等）。\n\
         - `a-trend-scanner` 的 context 中包含原始趋势描述。\n\
         \n\
         \n\n\
         ⚠️ 字段硬性要求（违反则输出无效）：\
         \n\
         - 每个 candidates 数组元素**必须**包含 stock_code（6位数字）和 stock_name（中文简称）。\
         \n- 缺少这两个字段任一的候选将被系统自动丢弃，前端不会显示。\
         \n- stock_code 必须是真实 A 股代码（如 300285、002371），不可用占位符。\
         \n\n\
         核心原则：\n\
         1. 优先选择市值 50-500 亿、机构覆盖少的公司（Serenity 偏好）。\n\
         2. 客户质量高于一切：已进入头部客户供应链的优先级 > 有技术但无客户验证的。\n\
         3. 排除股价已过度上涨的（近 3 月 > 150% 涨幅，瓶颈股早期启动属正常，150% 以上才需警惕）。\n\
         4. 排除负债率过高（> 85%）且营收增速 < 15% 的公司——瓶颈企业扩张期适度高负债（60-80%）是正常资本密集型特征，只有高负债 + 低增长双重恶化才需排除。\n\
         5. **催化剂决定何时买入**：每个候选必须至少给出 1 个近期催化剂（财报/量产/政策/供给冲击/产能释放）。\
         没有催化剂的候选不得输出。\n\
         6. **退出信号决定何时卖出**：每个候选必须评估技术替代、产能过剩、新进入者、需求放缓四大退出风险。\
         exit_now 的候选直接排除。\n\
         7. **低关注度量化**：评估机构覆盖变化、搜索热度、相对交易量、市场预期差。\
         关注度越低弹性越大，attention_score > 70 扣分。\n\
         8. **瓶颈客观评分优先**：下游 `c-scorer-*` 节点已按策略类型完成评分。\
         每个瓶颈环节的 `bottleneck_composite`、`supply_rigidity_score`、`demand_elasticity_score`、\
         `irreplaceability_score` 和 `data_reliability` 标签在 `result.computed_nodes` 中。\
         你的候选认定必须以这些客观评分为基准，而非从零编造。\
         评分低于 55 的环节标记为 \"weak_signal\"，候选优先级降低。\n\
         9. **策略评分数据**：`bottleneck_trend1~5.bottleneck_signals` 中包含 7 个代表股的瓶颈信号：\
         存货周转天数变化（`inventory_turnover` — 存货堆积=供给过剩风险）、\
         毛利率同比趋势（`gross_margin_trend` — 扩张=议价权提升）、\
         Capex/折旧比（`capex` — >3=积极扩产，可能预示未来产能瓶颈）。\
         对候选股的财务信号与链条中瓶颈环节的匹配度做交叉验证。\n\
         10. **需求确定性验证**：上游 chain-decomposer 已验证每个环节的需求确定性。\
         利用这些数据结合 search_news 工具搜索关键词验证（如搜索\"英伟达 CapEx\"确认需求真实性），\
         确保持续需求由 CapEx/订单/政策硬证据支撑而非 LLM 推测。无硬证据扣 20 分。\n\
         10. 每个候选必须给出具体的 serenity_score 和风险提示。\n\
         11. **股票发现策略（重要！瓶颈环节关键词高度专业化，直接搜索可能无结果）**：\n\
            a) 先搜索具体产品关键词（如\"高纯氮化铝\"）\n\
            b) 无结果则搜索上游材料/工艺关键词（如\"陶瓷基板\"、\"第三代半导体\"）\n\
            c) 仍无结果则搜索产业链关键词（如\"半导体材料\"、\"电子化学品\"）\n\
            d) 可调用 get_industry_ranking 或 get_stock_concept_blocks 获取概念板块成分股\n\
            e) 搜索到后逐一用 get_stock_financials/get_stock_quote 验证财务\n\
         12. **瓶颈股财务容忍度**：瓶颈企业多处于扩张/技术爬坡期，毛利率 15-30%、\n\
         负债率 50-75%、营收增速 0-10%、PE 为负但 ROE 改善——均属正常。\n\
         不要用消费/互联网标准要求制造业瓶颈股，核心判断标准是瓶颈环节的不可替代性。\n\
         13. **输出数量**：尽力输出 3-5 个候选。若确实全不达标可少于 3 个，\n\
         但 **summary 必须详述不达标原因**（搜索无结果/财务不达标的具体指标/可信度不足等）。\n\
         14. **再次强调**：必须至少尝试 3 轮不同关键词搜索，只有全部无结果才能输出空候选。\n\
         \n\n\
         ============== 输出格式强约束（必须严格遵守） ==============\n\
         1. 你的回复必须且只能包含一个代码块，开头三个反引号紧跟 tool_json。\n\
         2. 代码块内容为单一 JSON 对象，结构：{\"name\": \"submit_candidates\", \"arguments\": {\"candidates\": [...], \"summary\": \"...\"}}\n\
         3. **arguments 必须是含 candidates 和 summary 两个键的对象，严禁把候选数组直接放在 arguments 位置**（arguments 本身不是数组）。\n\
         4. <数据> 字段即下面的候选股数据。\n\
         5. 代码块外禁止任何文字：不要写\"以下是...\"、\"输出：\"、注释、解释、前缀、后缀。\n\
         6. 字段值为空时用 null，不要省略字段。\n\
         7. 数字字段（serenity_score、confidence 等）必须是 JSON 数字，不要加引号。\n\
         8. 严禁在 JSON 字符串值中夹带思考文字或自述注解；严禁尾逗号（对象/数组最后一个元素后不得出现逗号）。\n\
         ============================================================\n\
         \n\n\
         <数据> 结构：\n\
         {\"candidates\": [{\"stock_code\": \"300285\", \"stock_name\": \"国瓷材料\",\
         \"relevance\": \"direct\", \"serenity_score\": 75, \"confidence\": 70,\
         \"bottleneck_product\": \"半导体级高纯氮化铝(AlN)粉体\",\
         \"primary_risk\": \"客户验证周期较长\",\
         \"catalysts\": [{\"type\": \"earnings | production_ramp | policy | supply_shock\",\
         \"description\": \"催化剂描述\", \"expected_timeframe\": \"short_term | mid_term | long_term\",\
         \"confidence\": 70, \"trigger_condition\": \"触发条件\"}],\
         \"exit_signals\": {\"technology_disruption_risk\": \"风险描述\",\
         \"capacity_oversupply_risk\": \"风险描述\", \"new_entrant_risk\": \"风险描述\",\
         \"demand_slowdown_risk\": \"风险描述\",\
         \"overall_exit_urgency\": \"no_urgency | watch | caution | exit_now\"},\
         \"attention_metrics\": {\"coverage_change_3m\": \"新增 N 篇 | 减少 N 篇 | 无变化\",\
         \"search_heat\": \"冷门 | 正常 | 热门\",\
         \"relative_volume\": \"低于均值 N% | 正常 | 高于均值 N%\",\
         \"consensus_gap\": \"明显低估 | 合理 | 高估\", \"attention_score\": 30}}], \"summary\": \"...\"}";
    // context_sources: 提供趋势上下文 + 完整产业链拆解 + 策略评分结果
    let mapper_ctx: Vec<&str> = vec![
        "a-trend-scanner",
        "a-chain-trend1",
        "a-chain-trend2",
        "a-chain-trend3",
        "a-chain-trend4",
        "a-chain-trend5",
        "c-scorer-trend1",
        "c-scorer-trend2",
        "c-scorer-trend3",
        "c-scorer-trend4",
        "c-scorer-trend5",
        "c-consistency-check", // FIX-04: 一致性检查报告注入 context
    ];
    // 注入 bottleneck-calc 计算结果作为结构化变量
    let mut mapper_input_mapping = std::collections::HashMap::new();
    for tn in &trend_names {
        mapper_input_mapping.insert(format!("strategy_{tn}"), format!("c-scorer-{tn}.result"));
    }
    nodes.push(agent_node(
        "a-candidate-mapper",
        "候选公司筛选",
        "candidate-mapper",
        mapper_prompt,
        mapper_ctx,
        mapper_input_mapping,
        340.0,
        660.0,
    ));
    // 为所有 bottleneck-calc 节点添加直接边到 candidate-mapper
    for tn in &trend_names {
        let cid = format!("c-scorer-{tn}");
        edges.push(edge(&format!("e-{cid}-a-candidate-mapper"), &cid, "a-candidate-mapper"));
    }
    // trend-scanner → candidate-mapper
    edges.push(edge(
        "e-a-trend-scanner-a-candidate-mapper",
        "a-trend-scanner",
        "a-candidate-mapper",
    ));
    // chain-decomposers → candidate-mapper
    // v42 修复：原引用 a-analyzer-{tn}（不存在的节点名）→ 死边。改为真实节点 a-chain-{tn}，
    // 与 mapper_ctx（context_sources）声明的软依赖一致，双保险。
    for tn in &trend_names {
        let did = format!("a-chain-{tn}");
        edges.push(edge(&format!("e-{did}-a-candidate-mapper"), &did, "a-candidate-mapper"));
    }

    // ── FIX-02: 财务数据交叉验证 CodeNode ──
    // 对 candidate-mapper 输出的候选股 financial_snapshot 做基本合理性验证
    // 输入: candidates (candidate-mapper 的 context 输出)
    // 脚本: data-verifier.rhai
    // 输出: 同结构 candidates 数组，每项增加 data_verification 字段
    let verifier_code = include_str!("../data-verifier.rhai");
    let mut verifier_input_mapping = std::collections::HashMap::new();
    // S-M2 修复: 注入两条路径，Rhai 脚本自行选择有效的
    // 路径1: tool_call 格式 (LLM 模拟工具调用) — arguments.candidates
    // 路径2: 直接 JSON 格式 — 顶层 candidates
    verifier_input_mapping.insert(
        "candidates".to_string(),
        "a-candidate-mapper.content.arguments.candidates".to_string(),
    );
    verifier_input_mapping.insert(
        "candidates_direct".to_string(),
        "a-candidate-mapper.content.candidates".to_string(),
    );
    // v44 修复（2026-07-31 23:05）：agent_executor 拆包 tool_json 后 content 是
    // arguments 对象文本字符串（{"candidates":[...],"summary":...}），路径1/2 在 content
    // 为字符串时全部 resolve 失败（content.arguments 层不存在 / 字符串无 candidates 字段）
    // → data-verifier early return 空数组 → serenity 优先取 c-data-verifier 吞掉真候选。
    // 增加路径3：直接注入 a-candidate-mapper.content 整体，由 data-verifier.rhai json_parse 兜底。
    verifier_input_mapping
        .insert("candidates_raw".to_string(), "a-candidate-mapper.content".to_string());
    // P0-2 修复: 传入 tool_calls_made 用于交叉验证
    // LLM 在候选映射阶段调用的 get_stock_financials 工具返回真实财务数据，
    // data-verifier.rhai 比对 LLM 输出的 financial_snapshot 与工具返回的真实数据，
    // 检测 LLM 是否篡改/幻觉了财务数字。
    verifier_input_mapping
        .insert("tool_calls_made".to_string(), "a-candidate-mapper.tool_calls_made".to_string());
    nodes.push(WorkflowNode::Code(CodeNode {
        base: WorkflowNodeBase {
            id: "c-data-verifier".into(),
            title: "财务数据验证".into(),
            description: Some("对候选股财务数据进行合理性验证".into()),
            position: Position { x: 580.0, y: 660.0 },
            retry: RetryConfig::default(),
            timeout: Some(30),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        },
        config: CodeNodeConfig {
            language: "rhai".into(),
            code: verifier_code.to_string(),
            output_var: "c-data-verifier".into(),
            tool_name: None,
            execute_directly: true,
            input_mapping: verifier_input_mapping,
        },
    }));
    // candidate-mapper → data-verifier
    edges.push(edge(
        "e-a-candidate-mapper-c-data-verifier",
        "a-candidate-mapper",
        "c-data-verifier",
    ));

    // ── 候选持久化 ──
    // 注：StorageNode 不再需要 → run_serenity_screening() 中 Rust 代码已通过
    // reco_picks::ActiveModel 实体完成持久化（含完整性校验、种子同步、全量数据缓存）
    // 此处保持干净，不为 workflow 引入虚假节点。

    // ── 为所有 AgentNode 按角色配置工具 ──
    // agent_node 闭包中 tools/exposed_tools 为空，需要从 tool_defs 中注入。
    // 按角色分配工具集：避免 LLM 调用不相关的工具浪费 token。
    let tool_def_map: std::collections::HashMap<&str, &ToolDef> =
        tool_defs.iter().map(|td| (td.name.as_str(), td)).collect();
    let resolve_tools = |names: &[&str]| -> Vec<ToolDef> {
        names.iter().filter_map(|name| tool_def_map.get(name).cloned()).cloned().collect()
    };
    // 数据集工具（Phase 0）：可供趋势扫描器调用获取行业级数据
    let phase0_tools = &[
        "get_industry_ranking",
        "get_cls_flash",
        "get_stock_concept_blocks",
        "get_north_bound_flow",
        "get_market_dragon_tiger",
    ];
    // 产业链分析工具（Phase 1）：供 chain-decomposer 调用
    let chain_tools = &[
        "search_stock",
        "get_stock_financials",
        "get_stock_quote",
        "get_stock_institutional_visits",
        "compute_industry_position",
        "compute_bottleneck_signals", // V55: 瓶颈信号计算
    ];
    // 候选筛选工具（Phase 2）：供 candidate-mapper 全功能调用
    let candidate_tools = &[
        "search_stock",
        "get_stock_financials",
        "get_stock_quote",
        "get_stock_institutional_visits",
        "get_stock_news",
        "get_stock_research_reports",
        "search_news",
        "compute_attention_score",
        "compute_industry_position",
        "check_exit_signals",
        "compute_bottleneck_signals", // V55: 瓶颈信号验证
    ];
    // 后处理工具（回馈闭环）
    for node in &mut nodes {
        if let WorkflowNode::Agent(a) = node {
            let tools = match a.config.agent_profile_id.as_deref() {
                Some("stock-trend-scanner") => resolve_tools(phase0_tools),
                Some("stock-chain-decomposer") => resolve_tools(chain_tools),
                Some("stock-candidate-mapper") => {
                    // 候选映射器需要完整上下文：链分析 + 候选筛选
                    // V53 曾硬编码 doubao-seed-2-0-code-preview-260215（"agnes 小模型输出
                    // 29 tokens 截断"），但该模型依赖火山引擎账户，用户未配置 → 503
                    // no_available_account → candidate-mapper 节点永远失败 → 工作流
                    // PartiallyCompleted、候选为 0。2026-07-31 移除硬编码：回落全局默认
                    // 模型（用户实际配置的 GLM-5.2，其余 6 个 agent 节点均用它且输出
                    // 6000-9000 字符完整），模型选择交给用户在设置页统一管理。
                    let mut t = resolve_tools(chain_tools);
                    t.extend(resolve_tools(candidate_tools));
                    t
                },
                // 兜底：给予基本查询工具
                _ => resolve_tools(&["search_stock", "get_stock_quote"]),
            };
            a.config.tools = tools;
        }
    }

    // ── 序列化 ──
    let nodes_json = serde_json::to_string(&nodes).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化节点失败: {e}"))
    })?;
    let edges_json = serde_json::to_string(&edges).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化边失败: {e}"))
    })?;

    // ── Variables（用户可调整的参数字段）──
    // v17: 移除 ref_*_code（原行业基线代表股，随 t-baseline-* 节点一并删除）
    let serenity_vars = vec![
        // ── t-policy-news 政策新闻搜索关键词（tool input_mapping 引用变量名，不能直接写字面值）──
        Variable {
            name: "policy_news_keywords".into(),
            var_type: "string".into(),
            value: serde_json::json!("政策 利好 促进 扶持 振兴 补贴 实施方案 专项 消费 产业 投资"),
            description: Some("政策新闻搜索关键词（空格分隔），t-policy-news 节点引用".into()),
            is_secret: false,
        },
        // ── V6 新增：估值过滤参数（可在前端 Serenity 设置Tab中调整）──
        Variable {
            name: "serenity_max_pe".into(),
            var_type: "float".into(),
            value: serde_json::json!(100.0),
            description: Some("PE 上限：PE(TTM) 超过此值的候选直接排除（默认100）".into()),
            is_secret: false,
        },
        Variable {
            name: "serenity_max_pb".into(),
            var_type: "float".into(),
            value: serde_json::json!(10.0),
            description: Some("PB 上限：市净率超过此值的候选直接排除（默认10）".into()),
            is_secret: false,
        },
        Variable {
            name: "serenity_max_3m_gain_pct".into(),
            var_type: "float".into(),
            value: serde_json::json!(150.0),
            description: Some(
                "近3月涨幅上限(%)：超过此值的候选排除（默认150%，瓶颈股早期启动属正常）".into(),
            ),
            is_secret: false,
        },
        Variable {
            name: "serenity_max_12m_gain_pct".into(),
            var_type: "float".into(),
            value: serde_json::json!(300.0),
            description: Some(
                "近12月涨幅上限(%)：超过此值的候选排除，只拦长期飞过头的标的（默认300%）".into(),
            ),
            is_secret: false,
        },
        Variable {
            name: "serenity_min_gross_margin".into(),
            var_type: "float".into(),
            value: serde_json::json!(15.0),
            description: Some(
                "毛利率下限(%)：低于此值且评分<80时排除（默认15%，瓶颈股扩张期成本高）".into(),
            ),
            is_secret: false,
        },
        Variable {
            name: "serenity_max_debt_ratio".into(),
            var_type: "float".into(),
            value: serde_json::json!(85.0),
            description: Some(
                "负债率上限(%)：高于此值且评分<85时排除（默认85%，瓶颈企业扩张期负债偏高正常）"
                    .into(),
            ),
            is_secret: false,
        },
        Variable {
            name: "serenity_growth_exempt_pct".into(),
            var_type: "float".into(),
            value: serde_json::json!(50.0),
            description: Some(
                "高增长豁免阈值(%)：营收增速超过此值时PE超标可豁免（PEG<2即可放行，默认50%）"
                    .into(),
            ),
            is_secret: false,
        },
        Variable {
            name: "serenity_min_revenue_growth".into(),
            var_type: "float".into(),
            value: serde_json::json!(0.0),
            description: Some(
                "营收增速下限(%)：低于此值且评分<85时排除（默认0%，瓶颈股产能爬坡期增速低）".into(),
            ),
            is_secret: false,
        },
        // ── v42 新增：瓶颈评分权重（c-scorer input_mapping 引用，缺定义导致 resolve null → 走脚本 fallback）──
        Variable {
            name: "w_supply".into(),
            var_type: "float".into(),
            value: serde_json::json!(0.35),
            description: Some("瓶颈评分-供给刚性权重（默认0.35，c-scorer 引用）".into()),
            is_secret: false,
        },
        Variable {
            name: "w_demand".into(),
            var_type: "float".into(),
            value: serde_json::json!(0.35),
            description: Some("瓶颈评分-需求弹性权重（默认0.35，c-scorer 引用）".into()),
            is_secret: false,
        },
        Variable {
            name: "w_irreplace".into(),
            var_type: "float".into(),
            value: serde_json::json!(0.30),
            description: Some("瓶颈评分-不可替代性权重（默认0.30，c-scorer 引用）".into()),
            is_secret: false,
        },
        // ── v47 新增：用户主题输入（对话式主题荐股）──
        Variable {
            name: "user_themes".into(),
            var_type: "json".into(),
            value: serde_json::json!([]),
            description: Some(
                "用户指定主题词列表（如 ['AI SSD','存储芯片']），空数组表示自动扫描".into(),
            ),
            is_secret: false,
        },
    ];
    let variables_json = serde_json::to_string(&serenity_vars).map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化变量失败: {e}"))
    })?;

    // ── Tags ──
    let tags_json =
        serde_json::to_string(&["serenity", "bottleneck", "screening"]).map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL).with_detail(format!("序列化标签失败: {e}"))
        })?;

    // ── 写入 DB（版本门）──
    // 2026-07-31 简化定稿：前端保存（update_workflow_template）已改为【不递增 version】，
    // version 只由 seed 写入 → `existing.version >= TEMPLATE_VERSION` 版本门天然稳定：
    // 前端 auto-save 不会再推高 version 误挡 seed 重建；用户编辑的内容在 seed 未升
    // 版本号时也不会被 seed 覆盖。seed 想更新模板 → TEMPLATE_VERSION+1 即可。
    if let Some(existing) =
        workflow_template::Entity::find_by_id(TEMPLATE_ID).one(db).await.map_err(|e| {
            ErrorResponse::new(stock_setup::INTERNAL)
                .with_detail(format!("查询工作流模板失败: {e}"))
        })?
    {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[stock_analysis_setup] Serenity 模板已是最新 v{TEMPLATE_VERSION}（DB version={}），跳过",
                existing.version
            );
            return Ok(());
        }
        tracing::info!(
            "[stock_analysis_setup] 更新 Serenity 模板 v{} → v{TEMPLATE_VERSION}",
            existing.version
        );
    } else {
        tracing::info!("[stock_analysis_setup] Serenity 模板不存在，准备创建");
    }
    let _ = workflow_template::Entity::delete_by_id(TEMPLATE_ID).exec(db).await;
    workflow_template::ActiveModel {
        id: Set(TEMPLATE_ID.to_string()),
        cluster_id: Set(Some("trend".to_string())),
        route_path: Set(Some("/finance/trend/serenity".to_string())),
        name: Set("趋势智选".to_string()),
        description: Set(Some(
            "多策略趋势分析引擎：从市场数据中识别产业链瓶颈/政策驱动/业绩驱动/资金面驱动信号，自动筛选候选标的".to_string(),
        )),
        icon: Set("search".into()),
        tags: Set(Some(tags_json)),
        version: Set(TEMPLATE_VERSION),
        is_preset: Set(true),
        is_editable: Set(true),
        is_public: Set(true),
        trigger_config: Set(Some(
            serde_json::to_string(&TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({
                    "description": "Serenity 瓶颈筛选: 自动扫描市场发现产业链瓶颈机会",
                    "required_params": []
                }),
            })
            .map_err(|e| {
                ErrorResponse::new(stock_setup::INTERNAL)
                    .with_detail(format!("序列化触发器配置失败: {e}"))
            })?,
        )),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some(variables_json)),
        error_config: Set(None),
        composite_source: Set(None),
        tool_defs: Set(Some(tool_defs_json)),
        mission_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| {
        ErrorResponse::new(stock_setup::INTERNAL)
            .with_detail(format!("写入 Serenity 模板失败: {e}"))
    })?;

    tracing::info!("[stock_analysis_setup] Serenity 瓶颈筛选工作流模板已创建 (serenity-screening)");
    Ok(())
}
