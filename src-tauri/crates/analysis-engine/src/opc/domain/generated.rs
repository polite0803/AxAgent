// Auto-generated from YAML via convert_yaml_to_rust.py
// DO NOT EDIT MANUALLY — edit the YAML source and re-run the converter
//
// 注意：本文件通过 include! 引入到 mod.rs，
// 所有类型导入已在 mod.rs 中完成，请勿在此添加 use 语句。

impl DomainAdapterFactory {

    /// 学术研究 (academic) — 2 个工作流
    pub fn academic() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-acd-literature", "文献综述")
                .with_description("系统性地综述学术文献")
                .with_icon("📚")
                .with_tags(vec!["opc".to_string(), "academic".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-lit-search", "文献搜索")
                        .with_prompt("搜索目标领域的关键文献")
                    ,
                    DomainStepDef::agent("a-lit-review", "文献阅读")
                        .with_prompt("阅读文献并提取关键信息")
                    ,
                    DomainStepDef::agent("a-lit-synthesize", "综述撰写")
                        .with_prompt("撰写文献综述和发现")
                    ,
                ]),
            DomainWorkflowDef::new("wf-acd-research", "研究方案")
                .with_description("设计学术研究方案和方法论")
                .with_icon("🔬")
                .with_tags(vec!["opc".to_string(), "academic".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-research-question", "研究问题")
                        .with_prompt("定义研究问题和假设")
                    ,
                    DomainStepDef::agent("a-research-method", "方法论")
                        .with_prompt("设计研究方法和数据采集方案")
                    ,
                    DomainStepDef::agent("a-research-plan", "研究计划")
                        .with_prompt("制定时间表和资源计划")
                    ,
                ])
        ]
    }

    /// 设计与创意 (design) — 4 个工作流
    pub fn design() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-des-accessibility", "无障碍审计")
                .with_description("审计和修复产品无障碍问题")
                .with_icon("♿")
                .with_tags(vec!["opc".to_string(), "design".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-a11y-scan", "扫描")
                        .with_prompt("使用工具扫描无障碍问题")
                    ,
                    DomainStepDef::agent("a-a11y-report", "报告")
                        .with_prompt("分类报告问题严重程度")
                    ,
                    DomainStepDef::agent("a-a11y-fix", "修复")
                        .with_prompt("优先级修复关键无障碍问题")
                    ,
                ]),
            DomainWorkflowDef::new("wf-des-design-system", "设计系统")
                .with_description("搭建和维护统一的设计系统")
                .with_icon("📐")
                .with_tags(vec!["opc".to_string(), "design".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-ds-audit", "审计")
                        .with_prompt("审计现有设计元件和模式")
                    ,
                    DomainStepDef::agent("a-ds-components", "组件库")
                        .with_prompt("构建核心组件库和规范文档")
                    ,
                    DomainStepDef::agent("a-ds-doc", "文档")
                        .with_prompt("输出设计系统使用文档")
                    ,
                ]),
            DomainWorkflowDef::new("wf-des-prototype", "原型设计")
                .with_description("从线框图到交互原型")
                .with_icon("🎨")
                .with_tags(vec!["opc".to_string(), "design".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-proto-wireframe", "线框图")
                        .with_prompt("绘制页面结构和布局线框图")
                    ,
                    DomainStepDef::agent("a-proto-mockup", "高保真")
                        .with_prompt("设计高保真模型和设计稿")
                    ,
                    DomainStepDef::agent("a-proto-interact", "交互原型")
                        .with_prompt("制作可点击交互原型")
                    ,
                ]),
            DomainWorkflowDef::new("wf-des-ux-research", "用户研究")
                .with_description("用户访谈、可用性测试和洞察")
                .with_icon("👥")
                .with_tags(vec!["opc".to_string(), "design".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-ux-plan", "研究计划")
                        .with_prompt("确定研究目标和用户招募标准")
                    ,
                    DomainStepDef::agent("a-ux-conduct", "执行")
                        .with_prompt("执行用户访谈或可用性测试")
                    ,
                    DomainStepDef::agent("a-ux-report", "研究报告")
                        .with_prompt("输出研究洞察和设计建议")
                    ,
                ])
        ]
    }

    /// 工程与开发 (engineering) — 13 个工作流
    pub fn engineering() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-eng-api-design", "API设计")
                .with_description("设计REST/GraphQL API并生成文档")
                .with_icon("🔌")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-spec", "定义规约")
                        .with_prompt("定义API端点、请求/响应格式、认证方式")
                    ,
                    DomainStepDef::agent("a-validate", "验证设计")
                        .with_prompt("验证: RESTful规范、命名一致性、错误处理")
                    ,
                    DomainStepDef::agent("a-doc", "生成文档")
                        .with_prompt("生成API文档和客户端SDK")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-arch-review", "架构评审")
                .with_description("后端架构师评审系统设计方案的可行性")
                .with_icon("🏗️")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-design", "设计方案")
                        .with_prompt("提交系统架构设计方案")
                    ,
                    DomainStepDef::agent("a-review-arch", "架构评审")
                        .with_prompt("评审: 技术选型、扩展性、性能、成本、安全")
                    ,
                    DomainStepDef::agent("a-finalize", "方案定稿")
                        .with_prompt("根据评审意见修改方案并定稿")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-ci-setup", "CI/CD配置")
                .with_description("搭建持续集成/持续部署流水线")
                .with_icon("🔄")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-ci-plan", "方案设计")
                        .with_prompt("设计CI/CD架构: 构建、测试、部署阶段")
                    ,
                    DomainStepDef::agent("a-ci-config", "配置")
                        .with_prompt("编写CI/CD配置文件并测试")
                    ,
                    DomainStepDef::agent("a-ci-verify", "验证")
                        .with_prompt("运行流水线确认各阶段正常")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-code-review", "代码审查流水线")
                .with_description("AI工程师审查代码质量、安全、性能")
                .with_icon("👀")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-submit", "提交代码")
                        .with_prompt("评审者提交代码变更供审查")
                    ,
                    DomainStepDef::agent("a-review", "AI审查")
                        .with_prompt("审查代码: 逻辑错误、安全漏洞、性能问题、最佳实践")
                    ,
                    DomainStepDef::agent("a-report", "审查报告")
                        .with_prompt("生成审查报告: 严重程度排序、修改建议、自动修复")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-db-migrate", "数据库迁移")
                .with_description("设计并安全执行数据库模型变更")
                .with_icon("🗄️")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-plan-migrate", "迁移计划")
                        .with_prompt("分析变更影响、编写迁移脚本")
                    ,
                    DomainStepDef::agent("a-review-migrate", "变更审查")
                        .with_prompt("审查: 兼容性、性能影响、回滚方案")
                    ,
                    DomainStepDef::agent("a-execute-migrate", "执行迁移")
                        .with_prompt("执行迁移并验证数据完整性")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-deploy", "DevOps部署流水线")
                .with_description("自动化构建、测试、部署到生产环境")
                .with_icon("🚀")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-build", "构建")
                        .with_prompt("拉取代码、安装依赖、编译构建")
                    ,
                    DomainStepDef::agent("a-test", "自动化测试")
                        .with_prompt("运行单元测试、集成测试、性能测试")
                    ,
                    DomainStepDef::agent("a-deploy", "部署")
                        .with_prompt("部署到目标环境、执行数据库迁移")
                    ,
                    DomainStepDef::agent("a-verify", "验证")
                        .with_prompt("检查部署状态、监控告警、健康检查")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-monitor-setup", "监控告警配置")
                .with_description("搭建应用监控、日志和告警系统")
                .with_icon("📊")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-monitor-plan", "监控规划")
                        .with_prompt("设计监控指标、日志采集策略")
                    ,
                    DomainStepDef::agent("a-monitor-setup", "配置")
                        .with_prompt("配置监控工具、告警规则、仪表盘")
                    ,
                    DomainStepDef::agent("a-monitor-test", "测试")
                        .with_prompt("验证告警触发和通知链路")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-onboarding", "开发入职")
                .with_description("新项目环境搭建和开发指南")
                .with_icon("📖")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-env-setup", "环境配置")
                        .with_prompt("配置开发环境、安装依赖、初始化项目")
                    ,
                    DomainStepDef::agent("a-doc-read", "文档阅读")
                        .with_prompt("阅读项目文档、架构图、API文档")
                    ,
                    DomainStepDef::agent("a-first-task", "首个任务")
                        .with_prompt("完成首个开发任务验证环境")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-perf-opt", "性能优化")
                .with_description("分析和优化系统性能瓶颈")
                .with_icon("⚡")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-profile", "性能分析")
                        .with_prompt("profile代码、数据库查询、网络延迟")
                    ,
                    DomainStepDef::agent("a-identify", "瓶颈识别")
                        .with_prompt("识别性能瓶颈和根因分析")
                    ,
                    DomainStepDef::agent("a-optimize", "优化实施")
                        .with_prompt("实施优化并验证效果")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-refactor-lite", "快速追加重构")
                .with_description("重构完成后的追加变更快速通道，支持架构调整、功能新增、技术栈升级等变更的轻量级注入。")
                .with_icon("⚡")
                .with_tags(vec!["opc".to_string(), "refactor".to_string(), "incremental".to_string(), "fast-track".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("l-assess", "变更影响评估")
                        .with_prompt(
                            concat!("你作为架构分析师，快速评估追加变更的影响：\n", "1. 识别变更涉及的模块和文件\n", "2. 评估与已有重构成果的兼容性\n", "3. 分析变更的依赖关系和连锁影响\n", "4. 评估测试影响范围\n", "5. 给出变更复杂度等级和预估工期\n", "输出 JSON：{change_assessment{change{type{architecture|feature|tech_stack|dependency|bug}, description, source}, impacted_modules[{module, change_type, risk}], compatibility_check{conflicts[], breaking_changes, required_adaptions}, test_impact{affected_tests[], new_tests_needed[], regression_scope}, complexity{level{low|medium|high|critical}, estimated_effort, estimated_duration}, recommendation{proceed|defer|escalate, rationale, prerequisites[]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                    ,
                    DomainStepDef::agent("l-plan", "最小化变更计划")
                        .with_prompt(
                            concat!("你作为技术项目经理，制定最小化变更计划：\n", "1. 确定变更实施的最小影响路径\n", "2. 规划变更与现有代码的集成点\n", "3. 制定增量测试策略\n", "4. 规划特性开关和灰度策略\n", "5. 设定验收标准\n", "输出 JSON：{change_plan{implementation_path{steps[{step, action, files[], dependencies}], integration_points[], minimal_change_set}, test_strategy{unit_tests{additions[], modifications[]}, integration_tests{new_scenarios[], regression_scope}, performance_validation{baseline_comparison, key_paths}}, feature_flags{needed[{name, scope, default, cleanup_plan}], rollback_triggers[]}, acceptance_criteria{functional{checks[]}, quality{coverage_min, lint_pass}, performance{max_regression}}, estimated_timeline{phases[{phase, duration, deliverable}], total_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("assessment".to_string(), "l-assess.result".to_string());
                            m
                        })
                    ,
                    DomainStepDef::agent("l-execute", "变更执行")
                        .with_prompt(
                            concat!("你作为高级工程师，执行追加变更：\n", "1. 按最小化变更计划逐步实施\n", "2. 保持与已有重构成果的一致性\n", "3. 每完成一个变更点运行局部测试\n", "4. 记录变更细节（文件、函数、行号、影响范围）\n", "5. 更新相关文档\n", "输出 JSON：{execution{changes_completed[{change_id, type, files_changed[], tests_run, status}], integration_checks[{point, status, issues_found}], test_results{unit{passed, failed, new}, integration{passed, failed, new}, coverage_delta{before, after}}, issues[{description, impact, resolution}]}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("plan".to_string(), "l-plan.result".to_string());
                            m
                        })
                    ,
                    DomainStepDef::agent("l-verify", "验证与交付")
                        .with_prompt(
                            concat!("你作为质量工程师，执行最终验证：\n", "1. 运行增量测试和回归测试\n", "2. 验证变更与已有重构成果的兼容性\n", "3. 检查性能是否满足基线要求\n", "4. 确认代码质量未退化\n", "5. 完成交付检查\n", "输出 JSON：{verification{test_summary{total, passed, failed, skipped, coverage_delta{before, after, delta}}, compatibility_check{refactored_modules_impacted, breaking_changes, adapters_sufficient}, performance_check{key_paths[{path, before, after, regression}], overall_regression}, quality_gate{lint_errors, lint_warnings, complexity_change, smells_introduced}, delivery{verdict{pass|fail, blocking_issues[], recommendations[]}, artifacts_updated[], handoff_notes}}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "l-execute.result".to_string());
                            m
                        })
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-refactor", "大型代码项目重构")
                .with_description("系统性重构百万行级遗留代码，支持同语言重构和跨语言迁移（C++→Rust、C#→Rust、Java→TypeScript 等）。内置 UI 组件映射和国际化迁移分析，支持从现有 i18n 导入或从零搭建 i18next。从资产普查到验收交付的完整闭环。")
                .with_icon("🔧")
                .with_tags(vec!["opc".to_string(), "refactor".to_string(), "large-scale".to_string(), "code-quality".to_string(), "cross-language".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-asset-scan", "代码资产盘点")
                        .with_prompt(
                            concat!("你作为代码审计专家，对项目进行全面的资产盘点：\n", "1. 统计代码行数（分语言、分目录、分文件）\n", "2. 识别所有源文件、配置文件、测试文件、第三方库\n", "3. 标注模块边界和目录结构\n", "4. 统计第三方依赖和框架版本\n", "5. 识别技术栈（语言、框架、数据库、中间件）\n", "6. 生成代码资产清单（文件数、行数、模块数、依赖数）\n", "输出 JSON：{inventory{total_files, total_lines, by_language[{lang, files, lines}], by_module[{module, files, lines}], dependencies[{name, version, type}], tech_stack}, structure{directories, entry_points, public_api}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_agent(DomainAgentDef::new("code_auditor", "代码审计专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("optional_input");
                            ui = ui.with_prompt("代码资产盘点开始。请指定扫描范围（可选）");
                            ui = ui.with_fields(vec![DomainUserInputField::new("scan_scope", "multi_choice", "扫描范围")    .with_options(vec!["全部代码".to_string(), "指定模块".to_string(), "排除测试代码".to_string(), "排除第三方库".to_string()]), DomainUserInputField::new("focus_modules", "text", "重点关注模块（逗号分隔）")    .with_placeholder("例如: src/core, src/api"), DomainUserInputField::new("exclude_dirs", "text", "排除目录（逗号分隔）")    .with_placeholder("例如: node_modules, vendor")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-dep-graph", "依赖关系分析")
                        .with_prompt(
                            concat!("你作为架构分析师，构建项目的依赖关系图：\n", "1. 分析模块间的 import/include 依赖关系\n", "2. 识别循环依赖和不稳定依赖\n", "3. 构建包级/模块级依赖拓扑图\n", "4. 标注公共 API 和内部 API 边界\n", "5. 识别依赖方向违反（高层依赖低层、跨层调用）\n", "输出 JSON：{dependency_graph{nodes[{id, name, type, layer}], edges[{from, to, type}], circular_dependencies[], unstable_dependencies[], layer_violations[]}, impact_analysis{high_impact_modules[{module, impacted_modules, risk_level}], blast_radius_map}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_agent(DomainAgentDef::new("architect_analyst", "架构分析师"))
                    ,
                    DomainStepDef::agent("a-complexity", "复杂度扫描")
                        .with_prompt(
                            concat!("你作为代码质量专家，评估代码复杂度：\n", "1. 计算每个函数的圈复杂度（Cyclomatic Complexity）\n", "2. 计算认知复杂度（Cognitive Complexity）\n", "3. 识别超过阈值（CC>20）的高风险函数\n", "4. 统计嵌套深度、分支数量、参数数量\n", "5. 评估代码重复率（Copy-Paste Detection）\n", "输出 JSON：{complexity{high_risk_functions[{file, function, cyclomatic, cognitive, lines}], average_cyclomatic, max_cyclomatic, by_module[{module, avg_cc, max_cc, high_risk_count}]}, duplication{rate, hotspots[{file, lines_count, duplicated_with}]}, nesting{deep_functions[{file, function, depth}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_agent(DomainAgentDef::new("quality_expert", "代码质量专家"))
                    ,
                    DomainStepDef::agent("a-behavior-snapshot", "运行时行为快照")
                        .with_prompt(
                            concat!("你作为行为测试专家，为待重构模块生成运行时行为快照：\n", "1. 识别模块的公开 API（函数/方法/入口点）\n", "2. 为每个公开 API 生成覆盖全路径的测试用例（正常路径、边界条件、异常路径）\n", "3. 运行原有代码，记录每个用例的实际输出（返回值、副作用、状态变更）\n", "4. 捕获运行时副作用：数据库写入、文件 IO、网络请求、缓存操作、事件发射\n", "5. 记录状态机转换和时序依赖\n", "6. 将所有输入-输出对保存为\"黄金测试\"（Golden Test）基线\n", "输出 JSON：{behavioral_snapshot{api_snapshots[{api, inputs[{input_args, description}], outputs[{return_value, side_effects[{type, target, data}], state_changes[{entity, before, after}], timing}], coverage{paths_covered, paths_total, edge_cases_covered, error_paths_covered}, golden_tests[{test_id, api, input_fixture, expected_output, expected_side_effects, priority}], runtime_effects{external_calls[{service, call_signature, return_value}], db_operations[{table, operation, data}], event_streams[{event_type, payload}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("dep_graph".to_string(), "a-dep-graph.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("behavior_tester", "行为测试专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("optional_input");
                            ui = ui.with_prompt("行为快照生成中。请指定重点 API 和测试范围（可选）");
                            ui = ui.with_fields(vec![DomainUserInputField::new("key_apis", "text", "重点 API（逗号分隔）")    .with_placeholder("例如: UserService.create, OrderService.calculate"), DomainUserInputField::new("skip_tests", "multi_choice", "跳过的测试类型")    .with_options(vec!["已存在充分测试".to_string(), "性能敏感路径".to_string(), "外部依赖未就绪".to_string()]), DomainUserInputField::new("test_depth", "choice", "测试深度")    .with_options(vec!["仅公开 API".to_string(), "公开+内部方法".to_string(), "全量函数".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-tacit-knowledge", "隐式知识提取")
                        .with_prompt(
                            concat!("你作为知识提炼专家，从原有代码中提取隐式知识：\n", "1. 提取代码注释中的设计意图（\"为什么这样写\"而非\"写了什么\"）\n", "2. 分析提交历史，提取 bug 修复模式和设计决策上下文\n", "3. 识别隐式契约（调用方和被调用方的非文档化约定）\n", "4. 提取魔法数字/字符串的实际含义和来源\n", "5. 识别边界条件和特殊处理（防御性代码、hack、workaround）\n", "6. 记录并发假设和时序约束\n", "输出 JSON：{tacit_knowledge{design_intent[{file, code_region, comment, inferred_purpose, confidence}], commit_insights[{file, commit_hash, message, change_type, lesson_learned, related_bug}], implicit_contracts[{api, caller_expectations, callee_assumptions, violation_examples, confidence}], magic_values[{file, value, context, inferred_meaning, source}], edge_cases[{file, function, condition, special_handling, rationale}], concurrency_constraints[{module, assumption_type, description, violation_scenario}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("knowledge_engineer", "知识提炼专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("optional_input");
                            ui = ui.with_prompt("隐式知识提取中。请补充您知道的隐式知识（可选）");
                            ui = ui.with_fields(vec![DomainUserInputField::new("known_contracts", "text", "已知隐式契约")    .with_placeholder("例如: 用户删除必须先清理关联订单缓存"), DomainUserInputField::new("known_edge_cases", "text", "已知边界条件")    .with_placeholder("例如: 金额为零时跳过手续费计算"), DomainUserInputField::new("known_risks", "text", "已知技术风险")    .with_placeholder("例如: 并发场景下可能出现的竞态条件")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-smell-detect", "坏味道检测")
                        .with_prompt(
                            concat!("你作为代码审计专家，检测代码坏味道：\n", "1. 识别长方法/长函数（>50 行）\n", "2. 识别上帝类/上帝对象（>500 行或 >20 方法）\n", "3. 识别魔法数字和魔法字符串\n", "4. 识别深层嵌套（>4 层）\n", "5. 识别重复代码和 Copy-Paste\n", "6. 识别缺失抽象（if-else 链、switch 驱动）\n", "输出 JSON：{smells[{type, file, line, description, severity, suggestion}], smell_summary{total, critical, high, medium, low, by_type[{type, count}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_agent(DomainAgentDef::new("code_auditor", "代码审计专家"))
                    ,
                    DomainStepDef::agent("a-coupling-analyze", "耦合度分析")
                        .with_prompt(
                            concat!("你作为架构分析师，评估模块耦合度：\n", "1. 计算每个模块的扇入（fan-in）和扇出（fan-out）\n", "2. 识别高耦合模块（fan-in>10 或 fan-out>10）\n", "3. 识别紧密耦合的模块组\n", "4. 标记双向依赖和网状依赖\n", "5. 评估内聚性（Cohesion）\n", "输出 JSON：{coupling{high_coupling_modules[{module, fan_in, fan_out, coupling_score}], tight_coupling_groups[{modules[], coupling_type}], bidirectional_dependencies[], cohesion_assessment[{module, cohesion_score, issues[]}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_agent(DomainAgentDef::new("architect_analyst", "架构分析师"))
                    ,
                    DomainStepDef::agent("a-risk-assess", "风险评估")
                        .with_prompt(
                            concat!("你作为重构顾问，评估重构风险：\n", "1. 评估变更影响范围（Blast Radius）\n", "2. 评估回归风险（测试覆盖盲区、无测试模块）\n", "3. 评估技术风险（ unsafe 代码、反射、动态绑定）\n", "4. 评估数据迁移风险（数据库 Schema 变更、数据完整性）\n", "5. 评估性能风险（重构后可能的性能退化）\n", "输出 JSON：{risk_assessment{impact_scope{affected_modules[], critical_paths[]}, regression_risk{uncovered_modules[], risk_level}, technical_risk{unsafe_areas[], dynamic_dependencies[]}, data_migration_risk{schema_changes, data_loss_risk}, performance_risk{hot_paths[], expected_regression}}, risk_score{overall, confidence, factors[]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("complexity".to_string(), "a-complexity.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("refactor_consultant", "重构顾问"))
                    ,
                    DomainStepDef::agent("a-strategy", "重构策略制定")
                        .with_prompt(
                            concat!("你作为架构师，制定整体重构策略：\n", "1. 根据风险评估选择重构模式（渐进式/大爆炸/绞杀者/旁路）\n", "2. 确定重构范围和边界\n", "3. 制定技术选型和架构改进方案\n", "4. 定义重构成功标准（质量指标、性能指标、交付指标）\n", "5. 识别关键路径和里程碑\n", "输出 JSON：{strategy{mode{type, rationale, pros, cons}, scope{in_scope[], out_of_scope[]}, architecture_changes[{from, to, rationale}], success_criteria{quality[{metric, target}], performance[{metric, baseline, target}], delivery[{metric, target}]}, critical_path{steps[], estimated_duration}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("risk_assessment".to_string(), "a-risk-assess.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("solution_architect", "架构师"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("重构策略已生成。请审批确认");
                            ui = ui.with_fields(vec![DomainUserInputField::new("strategy_approval", "confirm", "是否批准此重构策略？")    .with_options(vec!["批准执行".to_string(), "需要修改".to_string(), "驳回重制定".to_string()])    .with_required(true), DomainUserInputField::new("preferred_mode", "choice", "偏好的重构模式")    .with_options(vec!["渐进式（推荐）".to_string(), "大爆炸".to_string(), "绞杀者模式".to_string(), "旁路模式".to_string()]), DomainUserInputField::new("scope_notes", "text", "范围调整意见")    .with_placeholder("例如: 排除支付模块，后续单独重构"), DomainUserInputField::new("priority_modules", "text", "优先重构的模块")    .with_placeholder("例如: 用户模块, 订单模块")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-batch-plan", "分批计划")
                        .with_prompt(
                            concat!("你作为技术项目经理，制定分批执行计划：\n", "1. 按耦合度排序，确定重构批次（每批 5-10 个模块）\n", "2. 识别每批的前置依赖和后置影响\n", "3. 分配每批的工期和资源\n", "4. 设定每批的验收标准和退出条件\n", "5. 规划批次间的集成验证点\n", "输出 JSON：{batches[{batch_id, modules[], order, dependencies, estimated_effort, exit_criteria{test_coverage, complexity_reduction, performance_delta}, verification_steps[]}], milestones[{name, batch_range, deliverable, approval_required}]}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("strategy".to_string(), "a-strategy.result".to_string());
                            m.insert("dep_graph".to_string(), "a-dep-graph.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("tech_project_manager", "技术项目经理"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("分批执行计划已生成。请审批确认");
                            ui = ui.with_fields(vec![DomainUserInputField::new("batch_approval", "confirm", "是否批准此分批计划？")    .with_options(vec!["批准执行".to_string(), "需要调整分批顺序".to_string(), "驳回重制定".to_string()])    .with_required(true), DomainUserInputField::new("batch_size", "choice", "每批模块数量")    .with_options(vec!["3-5 模块（保守）".to_string(), "5-10 模块（推荐）".to_string(), "10-15 模块（激进）".to_string()]), DomainUserInputField::new("first_batch_priority", "text", "第一批优先处理的模块")    .with_placeholder("例如: 核心工具库, 基础框架"), DomainUserInputField::new("timeline_constraints", "text", "时间约束")    .with_placeholder("例如: 第一批需在两周内完成")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-change-merge", "变更融合规划")
                        .with_prompt(
                            concat!("你作为变更管理专家，将新需求/架构变更融入重构计划：\n", "1. 接收外部变更请求列表（新功能/架构调整/技术栈升级/依赖更新）\n", "2. 分类变更类型和优先级\n", "3. 评估每个变更与现有重构批次的依赖关系和冲突\n", "4. 将变更预分配到最合适的批次\n", "5. 评估增量测试需求和资源影响\n", "6. 标记不可合并的变更并给出建议\n", "输出 JSON：{changes[{id, type{architecture|feature|tech_stack|dependency|bug}, description, priority{P0|P1|P2|P3}, target_modules[], injection_batch, conflicts[{existing_step, conflict_type, resolution}], additional_tests[], resource_impact}], merged_batches[{batch_id, original_modules[], new_changes[], risk_level, additional_effort, additional_tests[]}], unmergeable_changes[{id, reason, recommendation}]}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("batch_plan".to_string(), "a-batch-plan.result".to_string());
                            m.insert("new_requests".to_string(), "external_input".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("change_manager", "变更管理专家"))
                        .with_continue_on_fail(true)
                    ,
                    DomainStepDef::agent("a-quality-baseline", "质量基线建立")
                        .with_prompt(
                            concat!("你作为质量工程师，建立重构前的质量基线：\n", "1. 运行全量测试，记录当前测试覆盖率\n", "2. 执行性能基准测试，记录关键路径性能指标\n", "3. 运行代码静态分析，记录当前代码质量指标\n", "4. 建立代码规范和 lint 规则\n", "5. 生成质量基线报告\n", "输出 JSON：{baseline{test_coverage{overall, by_module[], uncovered_modules[]}, performance{key_paths[{path, latency_ms, throughput}], resource_usage{cpu, memory, disk_io}}, code_quality{lint_errors, lint_warnings, complexity_distribution[]}, quality_gates{coverage_min, complexity_max, lint_zero_errors, performance_regression_max}}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("batch_plan".to_string(), "a-batch-plan.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("quality_engineer", "质量工程师"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("质量基线已建立。请确认门禁标准");
                            ui = ui.with_fields(vec![DomainUserInputField::new("quality_approval", "confirm", "是否确认此质量基线标准？")    .with_options(vec!["确认执行".to_string(), "需要调整标准".to_string(), "驳回重设定".to_string()])    .with_required(true), DomainUserInputField::new("coverage_target", "number", "测试覆盖率目标 (%)"), DomainUserInputField::new("complexity_max", "number", "圈复杂度上限"), DomainUserInputField::new("performance_regression_max", "number", "最大性能退化 (%)"), DomainUserInputField::new("custom_gates", "text", "自定义门禁要求")    .with_placeholder("例如: API 兼容性 100%, 零安全漏洞")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-rollback", "回滚方案")
                        .with_prompt(
                            concat!("你作为 DevOps 工程师，制定完整的回滚方案：\n", "1. 设计分支策略（主干/特性分支/发布分支）\n", "2. 规划特性开关（Feature Flag）方案\n", "3. 制定数据库迁移回滚脚本\n", "4. 设计灰度发布和 A/B 测试方案\n", "5. 建立应急预案和回滚触发条件\n", "输出 JSON：{rollback{branch_strategy{model, naming_convention, protection_rules}, feature_flags[{name, scope, default_value, rollback_strategy}], db_migration{rollback_scripts[], data_preservation_plan}, canary{percentage_based, health_checks[], auto_rollback_triggers[]}, emergency_plan{trigger_conditions[], rollback_steps[], communication_plan}, parallel_strategy{enabled, refactor_branch_pattern, feature_branch_pattern, integration_point_frequency, conflict_resolution{auto_resolve[], manual_review[]}}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("batch_plan".to_string(), "a-batch-plan.result".to_string());
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("devops_engineer", "DevOps 工程师"))
                    ,
                    DomainStepDef::agent("a-integration-plan", "目标框架集成规划")
                        .with_prompt(
                            concat!("你作为架构师，规划代码迁移到目标框架的集成方案：\n", "\n", "## 目标框架配置（从配置 config/languages/frameworks/{{target_framework}}.yaml 加载）\n", "{{framework_profile}}\n", "\n", "## 类型映射结果\n", "{{type_mapping.result}}\n", "\n", "## 内存安全审计结果\n", "{{memory_audit.result}}\n", "\n", "## UI 组件映射结果（如果涉及 UI 框架迁移）\n", "{{ui_mapping.result 或 \"无 UI 组件映射需求\"}}\n", "\n", "## 国际化迁移分析结果（如果涉及 i18n）\n", "{{i18n_analysis.result 或 \"无国际化迁移需求\"}}\n", "\n", "请完成：\n", "1. 设计目录结构和模块组织方案\n", "2. 规划后端命令注册和 State 管理\n", "3. 规划前端组件结构和 Store 分类\n", "4. 设计跨语言通信接口（Tauri IPC）\n", "5. 规划测试策略（单元测试、集成测试、E2E）\n", "6. **UI 迁移专用**：组件拆分方案、路由设计、状态管理架构\n", "7. **国际化专用**：\n", "   - 如果源有 i18n：i18next 配置方案、翻译文件转换、key 命名规范\n", "   - 如果源无 i18n（build_from_scratch 模式）：\n", "     a. 从零搭建 i18next 基础设施\n", "     b. 规划硬编码字符串提取与分类\n", "     c. 分阶段实施计划（MVP → 核心功能 → 边缘功能）\n", "     d. 配置与 Ant Design 的集成\n", "8. **后端消息国际化**：\n", "   - 根据选择的 backend_i18n_pattern 规划实现：\n", "     a. 前端翻译模式：设计错误码枚举 + 前端错误码映射\n", "     b. 后端翻译模式：配置后端 i18n 库 + 语言参数传递\n", "     c. 混合模式：设计结构化错误响应（错误码 + 参数）\n", "   - 规划错误码规范（namespace.module.action.result）\n", "   - 设计前后端 i18n 同步机制\n", "\n", "输出 JSON：{integration_plan{directory_structure{backend_tree[], frontend_tree[]}, backend_integration{command_registration_plan[], state_management_design[], error_handling_strategy, error_code_framework{enum_definition, error_response_struct, serialization_format}}, frontend_integration{component_hierarchy[], store_classification[{type, modules[], responsibilities}], i18n_strategy, error_code_mapping{constant_definition, translation_loading, error_handler_hook}}, ui_integration{component_mapping_plan[], layout_strategy, state_architecture, router_design}, i18n_integration{mode[import_existing | build_from_scratch | skip], config_plan{languages[], namespaces[], key_convention}, file_structure{locales_dir, naming_pattern}, migration_order{phase1_infrastructure, phase2_extraction, phase3_files, phase4_code, phase5_verification}, incremental_approach[{phase, scope, modules[], estimated_effort}]}, backend_i18n_integration{pattern[frontend_translation | backend_translation | hybrid | skip], error_code_convention{format, prefixes[], namespaces[]}, sync_strategy{code_generation_script, ci_coverage_check, translation_workflow}, implementation_steps[{step, description, files[]}]}, communication{ipc_patterns[], data_flow_diagram}, testing_strategy{unit_test_plan[], integration_test_plan[], e2e_test_plan[], i18n_test_plan{backend_error_test, language_switch_test}}, implementation_order{phase1_modules[], phase2_modules[], phase3_modules[]}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("batch_plan".to_string(), "a-batch-plan.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("memory_audit".to_string(), "a-memory-audit.result".to_string());
                            m.insert("ui_mapping".to_string(), "a-ui-mapping.result".to_string());
                            m.insert("i18n_analysis".to_string(), "a-i18n-analysis.result".to_string());
                            m.insert("i18n_from_scratch_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#i18n_from_scratch".to_string());
                            m.insert("backend_i18n_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#backend_message_i18n".to_string());
                            m.insert("framework_config".to_string(), "config/languages/frameworks/{{target_framework}}.yaml".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("solution_architect", "架构师"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("框架集成规划完成。请审批确认");
                            ui = ui.with_fields(vec![DomainUserInputField::new("integration_approval", "confirm", "是否批准此集成方案？")    .with_options(vec!["批准执行".to_string(), "需要调整".to_string(), "驳回重制定".to_string()])    .with_required(true), DomainUserInputField::new("structure_preferences", "text", "结构偏好备注")    .with_placeholder("例如: 模块分组方式、命名约定等")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-pre-review", "预审查")
                        .with_prompt(
                            concat!("你作为代码审查员，进行重构前的预审查：\n", "1. 审查当前批次涉及模块的现有代码\n", "2. 确认重构方案与代码实际情况一致\n", "3. 标记需要特殊处理的代码段\n", "4. 确认测试用例覆盖待重构代码\n", "5. 输出审查通过/驳回\n", "输出 JSON：{pre_review{batch_id, modules_reviewed[], findings[{file, line, severity, recommendation}], coverage_gaps[{module, uncovered_paths}], verdict{approved|rejected, blockers[], suggestions[]}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("batch_plan".to_string(), "a-batch-plan.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("code_reviewer", "代码审查员"))
                    ,
                    DomainStepDef::agent("a-execute", "分批执行")
                        .with_prompt(
                            concat!("你作为高级工程师，按计划执行重构：\n", "1. 逐模块执行重构，每完成一个模块运行测试\n", "2. 应用重构模式（Extract Method、Extract Class、Introduce Interface 等）\n", "3. 保持行为不变，确保测试持续通过\n", "4. 记录每步变更（文件、函数、行号）\n", "5. 更新依赖关系图\n", "输出 JSON：{execution{batch_id, modules_completed[{module, refactorings_applied[], tests_passed, lines_changed}], issues_encountered[{module, issue, resolution}], current_progress, remaining_modules[]}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("pre_review".to_string(), "a-pre-review.result".to_string());
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("senior_engineer", "高级工程师"))
                    ,
                    DomainStepDef::agent("a-idiomatic-convert", "语言惯用模式转换")
                        .with_prompt(
                            concat!("你作为跨语言迁移专家，将代码转换为目标语言的惯用模式：\n", "\n", "## 映射规则（从配置加载）\n", "{{mapping_profile.pattern_mapping}}\n", "\n", "## 源代码分析结果\n", "{{asset_scan.result}}\n", "\n", "## 类型映射结果\n", "{{type_mapping.result}}\n", "\n", "请完成：\n", "1. 识别源代码中的设计模式和惯用写法\n", "2. 根据映射规则转换为目标语言的惯用模式\n", "3. 应用代码规范（命名、格式、结构）\n", "4. 添加必要的文档注释\n", "5. 确保代码符合目标语言社区最佳实践\n", "\n", "输出 JSON：{conversion{patterns_converted[{source_pattern, target_pattern, file, function, lines_changed}], idiomatic_score{before, after, improvement, by_module[]}}, code_quality{naming_compliance, formatting_compliance, documentation_coverage, community_compliance}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "a-execute.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("mapping_rules".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "跨语言迁移专家"))
                    ,
                    DomainStepDef::agent("a-framework-validate", "框架集成验证")
                        .with_prompt(
                            concat!("你作为框架专家，验证生成的代码符合目标框架规范：\n", "\n", "## 目标框架规范（从配置 config/languages/frameworks/{{target_framework}}.yaml 加载）\n", "{{framework_profile}}\n", "\n", "## 集成规划结果\n", "{{integration_plan.result}}\n", "\n", "请完成：\n", "1. 检查后端命令是否符合 #[tauri::command] 注册要求\n", "2. 检查 #[agent_command] 宏标签是否正确\n", "3. 检查前端 Store 是否符合四层分类规则\n", "4. 检查组件是否使用 i18n（禁止硬编码字符串）\n", "5. 检查类型是否从 @/types 导入\n", "6. 验证前后端通信接口是否正确\n", "7. **i18n 专项检查**：\n", "   - 检查翻译文件完整性（所有语言版本 key 一致）\n", "   - 检查 key 命名规范（点分命名法）\n", "   - 检查插值变量是否使用 {{}} 语法\n", "   - 检查是否有硬编码中文字符串\n", "   - 检查 useTranslation() 导入规范\n", "8. **后端消息 i18n 检查**：\n", "   - 检查后端错误码是否已定义（Rust 枚举）\n", "   - 检查错误码格式是否符合规范（namespace.module.type）\n", "   - 检查前端错误码映射是否完整\n", "   - 检查后端返回的错误是否包含 error_code 字段\n", "   - 检查前后端错误码同步是否一致\n", "   - 检查动态参数传递是否正确\n", "\n", "输出 JSON：{validation{backend{command_registration{compliant, violations[]}, agent_command_macro{compliant, violations[]}, state_management{compliant, violations[]}, error_code_definition{compliant, missing_codes[], format_violations[]}}, frontend{store_classification{compliant, violations[]}, i18n_usage{compliant, violations[], hardcoded_strings[], missing_translations[]}, type_imports{compliant, violations[]}, error_code_mapping{compliant, unmapped_codes[], sync_status}}, i18n_validation{file_completeness{compliant, missing_keys[]}, key_naming{compliant, violations[]}, interpolation{compliant, violations[]}, import_usage{compliant, violations[]}, backend_message_i18n{compliant, violations[], coverage_score}}, communication{ipc_pattern{compliant, violations[]}, data_contract{compliant, violations[]}, error_response_format{compliant, violations[]}}, overall_compliance{score, critical_violations[], warnings[], recommendations[]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "a-execute.result".to_string());
                            m.insert("idiomatic_convert".to_string(), "a-idiomatic-convert.result".to_string());
                            m.insert("integration_plan".to_string(), "a-integration-plan.result".to_string());
                            m.insert("framework_config".to_string(), "config/languages/frameworks/{{target_framework}}.yaml".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "框架集成专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("框架验证完成。存在规范违规项，请处理");
                            ui = ui.with_fields(vec![DomainUserInputField::new("validation_verdict", "confirm", "框架集成验证结果")    .with_options(vec!["全部通过".to_string(), "部分通过（需修复警告）".to_string(), "不通过（需修复严重违规）".to_string()])    .with_required(true), DomainUserInputField::new("violation_handling", "choice", "违规处理方式")    .with_options(vec!["自动修复可修复的违规".to_string(), "手动处理所有违规".to_string(), "忽略非关键违规".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-equivalence-check", "语义等价验证")
                        .with_prompt(
                            concat!("你作为行为验证专家，对比重构前后代码的语义等价性：\n", "1. 使用黄金测试（Golden Test）用例分别运行新旧代码，对比输出结果\n", "2. 逐维度对比：返回值结构、数据内容、副作用序列、状态变更\n", "3. 识别等价差异：\n", "   - 完全等价 ✓（所有维度一致）\n", "   - 语义等价 ≈（数据等价但结构不同，如字段顺序变化）\n", "   - 行为差异 ✗（逻辑不同，需人工确认）\n", "   - 静默失败 ✗（旧代码有输出，新代码无）\n", "4. 生成差异报告，标记需要人工裁决的差异\n", "5. 验证隐式知识中的契约是否被保留\n", "输出 JSON：{equivalence_check{batch_id, comparison_results[{golden_test_id, old_output{return_value, side_effects[], state_changes[]}, new_output{return_value, side_effects[], state_changes[]}, equivalence{identical|semantic|different|silent_failure}, diff_details[{dimension, old, new, diff_type}], verdict}], fidelity_score{percentage, by_module[{module, score}], failing_tests[{test_id, module, reason}]}, tacit_knowledge_validation[{tacit_item, preserved|violated, details}], overall_verdict{pass|fail|review_required, auto_pass_count, manual_review_count, blocked_diffs[]}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "a-execute.result".to_string());
                            m.insert("behavioral_snapshot".to_string(), "a-behavior-snapshot.result".to_string());
                            m.insert("tacit_knowledge".to_string(), "a-tacit-knowledge.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("behavior_verifier", "行为验证专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("等价验证完成。存在需裁决的行为差异，请处理");
                            ui = ui.with_fields(vec![DomainUserInputField::new("equivalence_verdict", "confirm", "是否接受当前等价性验证结果？")    .with_options(vec!["全部通过".to_string(), "部分通过（需处理差异）".to_string(), "不通过（需回滚）".to_string()])    .with_required(true), DomainUserInputField::new("difference_handling", "choice", "对语义差异的处理方式")    .with_options(vec!["接受差异（非关键路径）".to_string(), "修复差异后继续".to_string(), "标记为已知变化".to_string()]), DomainUserInputField::new("manual_review_notes", "text", "人工裁决备注")    .with_placeholder("说明接受或拒绝差异的原因")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-change-gate", "中途变更闸门")
                        .with_prompt(
                            concat!("你作为变更控制专家，评估重构执行过程中的中途变更：\n", "1. 接收外部变更请求（新功能/bug修复/架构调整/技术栈升级）\n", "2. 评估与当前重构批次的冲突（模块重叠、依赖冲突、时序冲突）\n", "3. 判断变更优先级并分类：\n", "   - P0 紧急：立即处理，暂停当前批次\n", "   - P1 重要：合并到当前批次的剩余模块\n", "   - P2 常规：排入后续批次\n", "   - P3 延后：记录待重构完成后处理\n", "4. 触发增量风险评估和测试需求评估\n", "5. 给出具体的变更处理方案\n", "输出 JSON：{change_request{id, type, description, source}, assessment{conflicts[{module, conflict_type, severity}], impacted_batches[], test_impact, resource_impact}, decision{priority{P0|P1|P2|P3}, action{merge_into_current|defer_to_next|reject|pause_and_handle}, target_batch, resolution_steps[], additional_tests[]}, rollback_plan{trigger_conditions[], steps[]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "a-execute.result".to_string());
                            m.insert("change_request".to_string(), "external_input".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("change_manager", "变更控制专家"))
                        .with_continue_on_fail(true)
                    ,
                    DomainStepDef::agent("a-regression", "回归验证")
                        .with_prompt(
                            concat!("你作为测试工程师，执行回归验证：\n", "1. 运行全量单元测试和集成测试\n", "2. 对比重构前后的测试覆盖率\n", "3. 执行性能回归测试，对比关键路径性能\n", "4. 检查是否引入新的代码坏味道\n", "5. 评估重构对模块耦合度的改善\n", "输出 JSON：{regression{test_results{total, passed, failed, skipped, coverage_delta{before, after, delta}}, performance_comparison{before{path, latency}, after{path, latency}, regression_detected}, quality_improvement{complexity_reduction, coupling_reduction, smells_removed}, regressions_found[{file, test, expected, actual, severity}]}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("execution".to_string(), "a-execute.result".to_string());
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("test_engineer", "测试工程师"))
                    ,
                    DomainStepDef::agent("a-integration", "集成验证")
                        .with_prompt(
                            concat!("你作为集成测试工程师，执行跨模块集成验证：\n", "1. 验证重构模块与未重构模块的接口兼容性\n", "2. 执行端到端测试场景\n", "3. 验证数据流和状态管理的正确性\n", "4. 执行跨模块性能测试\n", "5. 确认无循环依赖和层间违规\n", "输出 JSON：{integration{interface_compatibility{verified[], breaking_changes[], adapters_needed[]}, e2e_scenarios[{scenario, steps, status, duration}], data_flow_validation{paths_verified[], state_transitions_verified[]}, cross_module_performance{before, after, delta}, dependency_check{circular_deps[], layer_violations[]}}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("regression".to_string(), "a-regression.result".to_string());
                            m.insert("dep_graph".to_string(), "a-dep-graph.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("integration_engineer", "集成测试工程师"))
                    ,
                    DomainStepDef::agent("a-quality-gate", "质量门禁")
                        .with_prompt(
                            concat!("你作为质量总监，执行最终质量门禁检查：\n", "1. 检查测试覆盖率是否达到基线要求\n", "2. 检查代码复杂度是否降低\n", "3. 检查性能回归是否在可接受范围\n", "4. 检查代码坏味道是否显著减少\n", "5. 检查模块耦合度是否改善\n", "6. **检查行为保真度**：黄金测试通过率、隐式知识保留率\n", "7. **检查副作用完整性**：外部调用、数据库操作、事件发射是否一致\n", "8. **检查边界条件覆盖**：原有代码的特殊处理是否全部保留\n", "输出 JSON：{quality_gate{coverage{target, actual, pass}, complexity{avg_before, avg_after, reduction, pass}, performance{max_regression_allowed, actual_regression, pass}, smells{before_count, after_count, reduction, pass}, coupling{improvement, pass}, behavioral_fidelity{golden_test_pass_rate{target, actual, pass}, tacit_knowledge_retention{total_items, preserved, violated, pass}, side_effect_equivalence{db_ops_match, external_calls_match, event_order_match, pass}, edge_case_retention{total_edge_cases, preserved, missing, pass}}, overall_verdict{pass|fail, blocking_issues[], recommendations[], manual_reviews_required[]}}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("integration".to_string(), "a-integration.result".to_string());
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m.insert("equivalence_check".to_string(), "a-equivalence-check.result".to_string());
                            m.insert("behavioral_snapshot".to_string(), "a-behavior-snapshot.result".to_string());
                            m.insert("tacit_knowledge".to_string(), "a-tacit-knowledge.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("quality_director", "质量总监"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("最终质量门禁已完成。请审批交付");
                            ui = ui.with_fields(vec![DomainUserInputField::new("delivery_approval", "confirm", "是否批准本次重构交付？")    .with_options(vec!["批准交付".to_string(), "有条件批准（需处理待办）".to_string(), "驳回（需返工）".to_string()])    .with_required(true), DomainUserInputField::new("manual_reviews", "text", "需人工处理的问题")    .with_placeholder("列出需要在后续批次中处理的遗留问题"), DomainUserInputField::new("sign_off", "text", "审批人签字")    .with_required(true)    .with_placeholder("请输入审批人姓名和日期")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-doc-update", "文档更新")
                        .with_prompt(
                            concat!("你作为技术文档工程师，更新项目文档：\n", "1. 更新架构文档（架构图、模块关系、数据流）\n", "2. 更新 API 文档（接口变更、新增接口、废弃接口）\n", "3. 更新开发指南（编码规范、目录结构、构建流程）\n", "4. 更新迁移指南（从旧架构到新架构的迁移步骤）\n", "5. 生成重构总结报告\n", "输出 JSON：{docs_updated{architecture_doc{updated, changes[], new_diagrams[]}, api_doc{endpoints_updated[], deprecated[], new_endpoints[]}, migration_guide{steps[], breaking_changes[], compatibility_notes}, refactor_summary{total_changes, modules_affected, complexity_reduction, coupling_improvement, lessons_learned}}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("quality_gate".to_string(), "a-quality-gate.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("tech_writer", "技术文档工程师"))
                    ,
                    DomainStepDef::agent("a-handoff", "运维交接")
                        .with_prompt(
                            concat!("你作为运维工程师，完成运维交接：\n", "1. 生成运行手册（启动、停止、配置、故障排查）\n", "2. 设置监控告警（关键指标、阈值、告警渠道）\n", "3. 准备数据备份和恢复方案\n", "4. 整理应急预案和回滚流程\n", "5. 完成交接检查清单\n", "输出 JSON：{handoff{runbook{operations[{operation, steps, estimated_duration}], troubleshooting[{issue, cause, solution}]}, monitoring{metrics[{name, threshold, alert_channel}], dashboards[]}, backup_recovery{backup_schedule, restore_steps[], rto, rpo}, handoff_checklist{items[{item, status, assignee}], sign_off_required}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("docs_updated".to_string(), "a-doc-update.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("ops_engineer", "运维工程师"))
                    ,
                    DomainStepDef::agent("a-post-review", "事后复盘")
                        .with_prompt(
                            concat!("你作为项目经理，进行重构事后复盘：\n", "1. 回顾重构过程，总结经验教训\n", "2. 评估重构效果（质量提升、性能改善、可维护性提升）\n", "3. 分析计划与实际的偏差\n", "4. 提出后续改进建议\n", "5. 生成最终复盘报告\n", "输出 JSON：{post_review{what_went_well[{item, impact}], what_could_improve[{item, root_cause, action_needed}], metrics_before_after{quality[{metric, before, after, improvement}], performance[{metric, before, after, improvement}], maintainability[{metric, before, after, improvement}]}, follow_up_actions[{action, priority, owner, timeline}], final_assessment{success_level, key_achievements, remaining_risks[]}}}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "Grep".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("handoff".to_string(), "a-handoff.result".to_string());
                            m.insert("quality_gate".to_string(), "a-quality-gate.result".to_string());
                            m
                        })
                        .with_agent(DomainAgentDef::new("project_manager", "项目经理"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("feedback_collect");
                            ui = ui.with_prompt("重构已完成。请提供您的反馈");
                            ui = ui.with_fields(vec![DomainUserInputField::new("overall_satisfaction", "choice", "整体满意度")    .with_options(vec!["非常满意".to_string(), "基本满意".to_string(), "部分满意".to_string(), "不满意".to_string()])    .with_required(true), DomainUserInputField::new("quality_improvement", "number", "质量改善评分 (1-10)")    .with_required(true), DomainUserInputField::new("process_feedback", "text", "流程改进建议")    .with_placeholder("哪些环节做得好？哪些需要改进？"), DomainUserInputField::new("follow_up_items", "text", "后续跟进事项")    .with_placeholder("需要后续处理的问题或改进点"), DomainUserInputField::new("knowledge_share", "choice", "是否愿意分享经验？")    .with_options(vec!["愿意".to_string(), "视情况".to_string(), "不愿意".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-performance-compare", "性能特性对比")
                        .with_prompt(
                            concat!("你作为性能对比专家，对比源语言和目标语言的性能特性：\n", "\n", "## 源语言性能基线（从质量基线步骤获取）\n", "{{quality_baseline.result}}\n", "\n", "## 目标语言性能测试结果\n", "{{regression.result}}\n", "\n", "## 性能分析框架配置（从配置加载）\n", "{{performance_analyst_profile}}\n", "\n", "请完成：\n", "1. 对比关键路径的延迟（P50, P95, P99）\n", "2. 对比吞吐量（QPS, 并发处理数）\n", "3. 对比内存占用（稳态内存、峰值内存）\n", "4. 对比 CPU 使用率\n", "5. 识别性能退化路径并分析原因\n", "6. 提供性能优化建议\n", "\n", "输出 JSON：{performance_comparison{latency_comparison{metric, source_before, target_after, regression_percentage, verdict}, throughput_comparison{metric, source_before, target_after, regression_percentage, verdict}, memory_comparison{metric, source_before, target_after, regression_percentage, verdict}, cpu_comparison{metric, source_before, target_after, regression_percentage, verdict}, regressions_identified[{path, description, severity, root_cause, optimization_suggestion}], performance_score{before_score, after_score, delta, pass_threshold, overall_verdict}}}")
                        )
                        .with_tools(vec!["Bash".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m.insert("regression".to_string(), "a-regression.result".to_string());
                            m.insert("integration".to_string(), "a-integration.result".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("performance_analyst", "性能对比专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("性能对比完成。请确认性能可接受性");
                            ui = ui.with_fields(vec![DomainUserInputField::new("performance_verdict", "confirm", "性能是否可接受？")    .with_options(vec!["性能达标，可交付".to_string(), "可接受，但需优化".to_string(), "性能不达标，需返工".to_string()])    .with_required(true), DomainUserInputField::new("optimization_priority", "text", "需优化的关键路径")    .with_placeholder("例如: 用户登录接口延迟从 50ms 增加到 200ms")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-type-mapping", "类型系统映射分析")
                        .with_prompt(
                            concat!("你作为代码审计专家，分析从 {{source_lang}} 到 {{target_lang}} 的类型映射：\n", "\n", "## 源语言特性（从配置 config/languages/source_languages/{{source_lang}}.yaml 加载）\n", "{{source_lang_profile.type_system}}\n", "\n", "## 目标语言特性（从配置 config/languages/target_languages/{{target_lang}}.yaml 加载）\n", "{{target_lang_profile.type_system}}\n", "\n", "## 已知映射规则（从配置 config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml 加载）\n", "{{mapping_profile.type_mapping}}\n", "\n", "请完成：\n", "1. 识别源代码中的所有类型（基本类型、集合类型、自定义类型、泛型）\n", "2. 根据映射规则自动生成目标类型映射\n", "3. 标注需要人工决策的转换点（manual_review 部分）\n", "4. 生成类型映射表和决策清单\n", "\n", "输出 JSON：{type_mapping{auto_mapped[{source_type, target_type, confidence}], manual_review_required[{source_type, target_options[], decision_guide, risk_level}], unmapped_types[{type, reason, suggestion}], mapping_coverage{total_types, auto_mapped, manual_review, unmapped, percentage}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("code_auditor", "代码审计专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("类型映射分析完成。请确认需要人工决策的类型转换");
                            ui = ui.with_fields(vec![DomainUserInputField::new("mapping_approval", "confirm", "是否批准此类型映射方案？")    .with_options(vec!["批准执行".to_string(), "需要调整映射".to_string(), "驳回重制定".to_string()])    .with_required(true), DomainUserInputField::new("manual_decisions", "text", "类型转换决策备注")    .with_placeholder("说明需要特殊处理的类型转换")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-memory-audit", "内存安全审计")
                        .with_prompt(
                            concat!("你作为内存安全专家（{{source_lang}} → {{target_lang}}），分析源代码的内存模型和迁移策略：\n", "\n", "## 源语言内存模型（从配置加载）\n", "{{source_lang_profile.memory_model}}\n", "\n", "## 目标语言内存模型（从配置加载）\n", "{{target_lang_profile.memory_model}}\n", "\n", "## 内存映射规则（从配置加载）\n", "{{mapping_profile.memory_mapping}}\n", "\n", "请完成：\n", "1. 识别源代码中的内存管理模式（栈/堆分配、生命周期、所有权）\n", "2. 标记潜在的内存安全问题（悬垂指针、双重释放、缓冲区溢出等）\n", "3. 设计目标语言的内存管理策略（所有权系统、智能指针选择、生命周期标注）\n", "4. 生成内存安全检查清单\n", "\n", "输出 JSON：{memory_audit{patterns_found[{pattern, count, risk_level}], safety_issues[{issue, severity, file, line}], migration_strategy{ownership_model, pointer_choice_rules, lifetime_annotation_guide}, safety_checklist[{checkpoint, status, action_required}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "跨语言迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("optional_input");
                            ui = ui.with_prompt("内存安全审计进行中。请补充已知的内存问题（可选）");
                            ui = ui.with_fields(vec![DomainUserInputField::new("known_issues", "text", "已知内存问题")    .with_placeholder("例如: 模块 X 存在缓冲区溢出风险"), DomainUserInputField::new("performance_requirements", "text", "内存性能要求")    .with_placeholder("例如: 稳态内存 < 100MB，无内存泄漏")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-ui-mapping", "UI 组件映射分析")
                        .with_prompt(
                            concat!("你作为 UI 框架迁移专家，分析源框架 UI 组件到目标框架的映射关系：\n", "\n", "## 源框架（{{source_framework}}）组件映射规则（从配置加载）\n", "{{mapping_profile.widget_mapping}}\n", "\n", "## 核心模块映射规则\n", "{{mapping_profile.core_modules}}\n", "\n", "## 模型/视图映射规则\n", "{{mapping_profile.model_view}}\n", "\n", "请完成：\n", "1. 识别源代码中的所有 UI 组件（Widget、Layout、Dialog 等）\n", "2. 根据映射规则确定目标框架对应组件\n", "3. 标记需要人工设计决策的组件（如复杂自定义控件）\n", "4. 规划 UI 状态管理方案（Zustand/Redux）\n", "5. 规划路由方案（React Router）\n", "\n", "输出 JSON：{ui_mapping{components[{source_widget, target_component, confidence, mapping_type}], layouts[{source_layout, target_layout_strategy, css_framework}], custom_widgets[{widget, complexity, implementation_strategy, estimated_effort}], state_management{state_model, store_design, data_flow}, routing{routes_mapped, navigation_strategy}}, migration_complexity{total_components, auto_mappable, custom_implementation, complexity_score, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("mapping_rules".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true AND source_framework != null AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "UI 框架迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("UI 组件映射分析完成。请确认映射方案");
                            ui = ui.with_fields(vec![DomainUserInputField::new("ui_mapping_approval", "confirm", "是否批准此 UI 映射方案？")    .with_options(vec!["批准执行".to_string(), "需要调整映射".to_string(), "需要重新设计 UI".to_string()])    .with_required(true), DomainUserInputField::new("custom_widget_strategy", "choice", "自定义组件实现策略")    .with_options(vec!["优先使用 Ant Design 组件".to_string(), "使用原生 React + Tailwind".to_string(), "混合使用".to_string(), "其他".to_string()]), DomainUserInputField::new("css_framework", "choice", "CSS 框架选择")    .with_options(vec!["Tailwind CSS 4 (推荐)".to_string(), "Ant Design 主题".to_string(), "原生 CSS".to_string(), "其他".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-i18n-analysis", "国际化迁移分析")
                        .with_prompt(
                            concat!("你作为国际化迁移专家，分析源框架的国际化实现并规划迁移方案：\n", "\n", "## 源框架 i18n 配置（从配置加载）\n", "{{mapping_profile.i18n_mapping}}\n", "\n", "## i18n 检查清单\n", "{{mapping_profile.i18n_checklist}}\n", "\n", "## 无 i18n 源实现时的从零搭建策略\n", "{{mapping_profile.i18n_from_scratch}}\n", "\n", "## 后端消息国际化配置\n", "{{mapping_profile.backend_message_i18n}}\n", "\n", "请完成：\n", "1. 扫描源代码中所有国际化相关代码（QTranslator、tr()、translate() 等）\n", "2. **判断源 i18n 状态**：\n", "   - 有现有 i18n：分析 .ts/.qm 文件结构和内容\n", "   - 无现有 i18n：统计硬编码字符串数量和分布\n", "3. 识别所有硬编码字符串（需全部替换为 t() 调用）\n", "4. 规划 i18next 配置方案（语言支持、命名空间结构）\n", "5. 设计翻译 key 的命名规范（module.submodule.key）\n", "6. 如果有现有 i18n：规划 .ts → JSON 文件的转换流程\n", "7. 如果无现有 i18n：规划\"提取硬编码 → 建立翻译文件 → 替换为 t() 调用\"的流程\n", "8. 检查插值变量、复数形式、日期/数字格式化的特殊处理\n", "9. **后端消息分析（新增）**：\n", "   - 识别所有后端推送给用户的消息（QMessageBox、statusBar、emit 信号等）\n", "   - 分析消息类型（错误、警告、信息、成功）\n", "   - 规划错误码体系（namespace.module.action.result）\n", "   - 选择前后端消息传递模式（前端翻译/后端翻译/混合模式）\n", "   - 设计前后端 i18n 同步策略\n", "\n", "输出 JSON：{i18n_analysis{current_state{has_i18n[boolean], translator_usage[count, files], hardcoded_strings[count, files, samples[], languages[]], translation_files[{path, format, entry_count}], backend_messages[{source, type, count, samples[]}], locale_coverage[supported_languages[]]}, migration_plan{mode[import_existing | build_from_scratch], target_languages[], key_naming_convention, namespace_structure, file_organization, backend_i18n_pattern[frontend_translation | backend_translation | hybrid]}, conversion_strategy{mode_specific_config[ts_to_json_method | extraction_strategy], key_mapping_table[{source_pattern, i18next_key}], special_cases[{type, source, target, complexity}]}, backend_strategy{error_code_convention, frontend_backend_sync, error_code_mapping_table[{qt_source, rust_error_code, ts_translation_key}]}, complexity_assessment{total_entries, auto_mappable, manual_review_required, estimated_effort, risk_level}}, migration_workflow{step1_scan, step2_extract_or_convert, step3_key_mapping, step4_code_update, step5_verify, step6_backend_integration}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("ui_mapping".to_string(), "a-ui-mapping.result".to_string());
                            m.insert("i18n_mapping_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#i18n_mapping".to_string());
                            m.insert("i18n_checklist".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#i18n_checklist".to_string());
                            m.insert("backend_i18n_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#backend_message_i18n".to_string());
                            m
                        })
                        .with_condition("cross_language_migration.enabled == true AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "国际化迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("国际化迁移分析完成。请确认方案");
                            ui = ui.with_fields(vec![DomainUserInputField::new("enable_i18n", "choice", "i18n 迁移模式")    .with_options(vec!["启用 i18n（推荐）".to_string(), "暂不启用，后续添加".to_string(), "仅提取字符串，不替换代码".to_string()])    .with_required(true), DomainUserInputField::new("backend_i18n_pattern", "choice", "后端消息国际化模式")    .with_options(vec!["前端翻译模式（推荐）".to_string(), "后端翻译模式".to_string(), "混合模式（错误码+参数）".to_string(), "暂不处理后端消息".to_string()])    .with_required(true), DomainUserInputField::new("i18n_approval", "confirm", "是否批准此国际化迁移方案？")    .with_options(vec!["批准执行".to_string(), "需要调整".to_string(), "需要重新分析".to_string()])    .with_required(true), DomainUserInputField::new("target_languages", "text", "目标支持语言")    .with_placeholder("例如: zh-CN, en, ja, ko"), DomainUserInputField::new("key_naming", "choice", "Key 命名规范")    .with_options(vec!["点分命名 (module.sub.key)".to_string(), "下划线 (module_sub_key)".to_string(), "短横线 (module-sub-key)".to_string(), "保持原样".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-afsim-analysis", "Afsim 仿真框架特征分析")
                        .with_prompt(
                            concat!("你作为 Afsim 仿真框架迁移专家，分析源代码的 Afsim 特征：\n", "\n", "## Afsim 特定映射配置\n", "{{mapping_profile.afsim_math_mapping}}\n", "\n", "## Afsim 架构映射配置\n", "{{mapping_profile.afsim_architecture_mapping}}\n", "\n", "## Afsim Qt Widget 增强映射\n", "{{mapping_profile.afsim_qt_widget_enhanced_mapping}}\n", "\n", "请完成：\n", "1. 识别 Afsim 核心类（WsfObject、WsfPlatform、WsfTrack、WsfPlugin 等）\n", "2. 识别 Afsim 数学类型（UtVec3、UtMatrix、UtQuaternion、UtEarth 等）\n", "3. 识别 Afsim 特定 Qt Widget（QDockWidget、QGLWidget、QTreeWidget 等）\n", "4. 识别 Afsim 脚本系统和自定义 DSL\n", "5. 识别 Afsim 特定的协议（DIS/HLA 等分布式仿真接口）\n", "6. 评估迁移复杂度并给出优先级排序\n", "\n", "输出 JSON：{afsim_analysis{core_classes[{name, usage_count, complexity, priority}], math_types[{name, usage_count, target_crate, target_typescript}], custom_widgets[{name, count, complexity, target_impl}], script_system{detected[boolean], language, complexity}, protocols[{name, type, complexity}], complexity_assessment{overall_score[1-10], estimated_effort_weeks, high_risk_areas[]}, migration_priority{phase1_core[], phase2_engine[], phase3_ui[], phase4_advanced[]}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("afsim_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_math_mapping".to_string());
                            m.insert("afsim_architecture".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_architecture_mapping".to_string());
                            m.insert("afsim_widgets".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_qt_widget_enhanced_mapping".to_string());
                            m.insert("rust_mapping".to_string(), "config/languages/mappings/cpp-to-rust.yaml#afsim_rust_mapping".to_string());
                            m
                        })
                        .with_condition("source_framework_contains == 'Qt' AND file_pattern_contains == 'afsim'")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "Afsim 仿真框架迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Afsim 框架特征分析完成。请确认迁移优先级");
                            ui = ui.with_fields(vec![DomainUserInputField::new("afsim_migration_approval", "confirm", "是否批准此 Afsim 迁移方案？")    .with_options(vec!["批准执行".to_string(), "需要调整优先级".to_string(), "需要重新分析".to_string()])    .with_required(true), DomainUserInputField::new("afsim_phase_selection", "text", "优先迁移阶段")    .with_placeholder("例如: phase1_core, phase2_engine"), DomainUserInputField::new("afsim_3d_strategy", "choice", "3D 可视化迁移策略")    .with_options(vec!["使用 Three.js 完整迁移".to_string(), "简化为 2D 视图".to_string(), "延后处理".to_string(), "使用 D3.js 数据可视化".to_string()]), DomainUserInputField::new("afsim_script_strategy", "choice", "脚本系统迁移策略")    .with_options(vec!["完全重写为 TypeScript".to_string(), "创建兼容层".to_string(), "移植为 Python".to_string(), "延后处理".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-afsim-math-conversion", "Afsim 数学类型转换")
                        .with_prompt(
                            concat!("你作为数学计算库迁移专家，执行 Afsim 数学类型的转换：\n", "\n", "## 数学类型映射配置\n", "{{mapping_profile.afsim_math_mapping}}\n", "\n", "## Rust crate 推荐\n", "{{mapping_profile.recommended_crates}}\n", "\n", "请完成：\n", "1. 将 UtVec2/UtVec3 转换为 nalgebra::Vector2/Vector3\n", "2. 将 UtMatrix3x3/UtMatrix4x4 转换为 nalgebra::Matrix3/Matrix4\n", "3. 将 UtQuaternion 转换为 nalgebra::UnitQuaternion\n", "4. 实现 UtEarth 坐标转换（WGS84/ECEF/NED）\n", "5. 迁移所有向量运算（Add、Subtract、Dot、Cross、Normalize）\n", "6. 迁移所有矩阵运算（Multiply、Transpose、Determinant）\n", "7. 生成对应的 TypeScript 数学工具类\n", "\n", "输出 JSON：{math_conversion{converted_types[{source, target_rust, target_typescript, file_paths[]}], operations_converted[{source_op, rust_op, typescript_op}], test_coverage{unit_tests, edge_cases}}, issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("afsim_analysis".to_string(), "a-afsim-analysis.result".to_string());
                            m.insert("math_mapping".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_math_mapping".to_string());
                            m.insert("rust_math_mapping".to_string(), "config/languages/mappings/cpp-to-rust.yaml#afsim_rust_mapping#math_crate_mapping".to_string());
                            m
                        })
                        .with_condition("a-afsim-analysis.result.math_types.count > 0")
                        .with_agent(DomainAgentDef::new("code_converter", "数学库迁移专家"))
                    ,
                    DomainStepDef::agent("a-afsim-architecture-mapping", "Afsim 核心架构映射")
                        .with_prompt(
                            concat!("你作为架构映射专家，执行 Afsim 核心类到 AxAgent 架构的映射：\n", "\n", "## 架构映射配置\n", "{{mapping_profile.afsim_architecture_mapping}}\n", "\n", "请完成：\n", "1. 将 WsfObject 基类迁移为 Rust trait + TypeScript interface\n", "2. 将 WsfPlatform 迁移为 Agent struct + Zustand Store\n", "3. 将 WsfTrack 迁移为 Conversation 模型\n", "4. 将 WsfPlugin 迁移为 Tool trait\n", "5. 将 WsfScenario 迁移为 Orchestrator\n", "6. 迁移事件系统（信号槽 → EventEmitter/Zustand 订阅）\n", "7. 迁移状态管理（内部状态 → Zustand Store）\n", "8. 创建 Cargo workspace 结构\n", "\n", "输出 JSON：{architecture_mapping{classes_mapped[{source, target_rust, target_typescript, file_paths[]}], traits_defined[{name, methods[], implementations[]}], stores_created[{name, state_shape, actions[]}], workspace_structure{crate_members[], dependencies[]}}, issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("afsim_analysis".to_string(), "a-afsim-analysis.result".to_string());
                            m.insert("architecture_mapping".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_architecture_mapping".to_string());
                            m.insert("rust_workspace".to_string(), "config/languages/mappings/cpp-to-rust.yaml#afsim_rust_mapping#workspace_structure".to_string());
                            m
                        })
                        .with_condition("a-afsim-analysis.result.core_classes.count > 0")
                        .with_agent(DomainAgentDef::new("architect_analyst", "架构迁移专家"))
                    ,
                    DomainStepDef::agent("a-afsim-ui-enhancement", "Afsim UI 组件增强迁移")
                        .with_prompt(
                            concat!("你作为 UI 组件增强迁移专家，执行 Afsim 特定 Qt Widget 的迁移：\n", "\n", "## Qt Widget 增强映射\n", "{{mapping_profile.afsim_qt_widget_enhanced_mapping}}\n", "\n", "## Qt Designer UI 解析配置\n", "{{mapping_profile.qt_designer_ui_parsing}}\n", "\n", "请完成：\n", "1. 解析所有 .ui 文件（Qt Designer XML）\n", "2. 将 QDockWidget 系统迁移为 Segmented + 可拖拽面板\n", "3. 将 QGLWidget/QOpenGLWidget 迁移为 Three.js Canvas\n", "4. 将 QStyledItemDelegate 迁移为 Ant Design Column render\n", "5. 将 QAbstractItemModel 迁移为 Zustand Store\n", "6. 将复杂 QTreeWidget（带拖拽）迁移为 Tree + dnd-kit\n", "7. 将 QSplitter 迁移为 Resizable 面板组\n", "\n", "输出 JSON：{ui_enhancement{widgets_converted[{source, target, complexity, file_paths[]}], custom_components_created[{name, framework, props[], events[]}], threejs_scene_setup{canvas_component, controls, lighting}}, issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("afsim_analysis".to_string(), "a-afsim-analysis.result".to_string());
                            m.insert("ui_enhanced_mapping".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_qt_widget_enhanced_mapping".to_string());
                            m.insert("ui_parser_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#qt_designer_ui_parsing".to_string());
                            m
                        })
                        .with_condition("a-afsim-analysis.result.custom_widgets.count > 0")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "UI 组件增强迁移专家"))
                    ,
                    DomainStepDef::agent("a-3d-visualization-migration", "3D 可视化迁移")
                        .with_prompt(
                            concat!("你作为 3D 可视化迁移专家，执行 Qt OpenGL 到 Three.js 的迁移：\n", "\n", "## 3D 可视化迁移指南\n", "{{mapping_profile.visualization_3d_migration}}\n", "\n", "## Afsim 3D 特定场景\n", "{{mapping_profile.afsim_3d_scenarios}}\n", "\n", "请完成：\n", "1. 识别所有 Qt OpenGL 相关代码（QGLWidget、QOpenGLWidget、QOpenGLFunctions 等）\n", "2. 将 OpenGL 调用映射到 Three.js API\n", "3. 迁移几何图元（glutSolidCube → boxGeometry 等）\n", "4. 迁移光照系统（glLight → directionalLight 等）\n", "5. 迁移相机控制（gluPerspective → PerspectiveCamera 等）\n", "6. 实现 Afsim 3D 特定场景（战场态势、地球坐标、飞行器模型）\n", "7. 应用性能优化建议\n", "\n", "输出 JSON：{migration_3d{opengl_components_found[{name, file, complexity}], geometries_mapped[{source, target, file_paths[]}], lighting_mapped[{source, target}], camera_mapped[{source, target}], afsim_scenarios_implemented[{name, file_paths[]}], performance_optimizations_applied[{tip, impact}], issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("visualization_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#visualization_3d_migration".to_string());
                            m.insert("afsim_3d_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#visualization_3d_migration#afsim_3d_scenarios".to_string());
                            m
                        })
                        .with_condition("file_pattern_contains == 'opengl' OR file_pattern_contains == 'glwidget' OR file_pattern_contains == 'glut'")
                        .with_agent(DomainAgentDef::new("frontend_framework_specialist", "3D 可视化迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("3D 可视化迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("strategy_3d_approval", "confirm", "是否批准此 3D 可视化迁移方案？")    .with_options(vec!["批准执行".to_string(), "简化方案（仅核心场景）".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("scenario_selection", "text", "优先实现的 3D 场景")    .with_placeholder("例如: 战场态势显示, 飞行器模型"), DomainUserInputField::new("performance_target", "choice", "性能目标")    .with_options(vec!["流畅 (60 FPS)".to_string(), "标准 (30 FPS)".to_string(), "按需优化".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-script-compatibility-migration", "脚本语言兼容迁移")
                        .with_prompt(
                            concat!("你作为脚本语言迁移专家，执行 Afsim 自定义脚本到现代方案的迁移：\n", "\n", "## 脚本兼容方案配置\n", "{{mapping_profile.afsim_script_compatibility}}\n", "\n", "请完成：\n", "1. 统计现有 Afsim 脚本文件数量和行数\n", "2. 识别脚本特性（对象创建、事件触发、循环、函数调用等）\n", "3. 选择迁移方案（推荐：完全重写为 TypeScript）\n", "4. 建立 TypeScript 核心 API\n", "5. 迁移试点脚本\n", "6. 规划批量迁移路径\n", "\n", "输出 JSON：{script_migration{scripts_analyzed{total_files, total_lines, features_detected[]}, approach_selected{name, rationale}, core_api_created{modules[], methods[]}, pilot_scripts_migrated[{original, migrated, correctness}], batch_migration_plan{phases[{phase, tasks[], estimated_effort}]}}, issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("script_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#afsim_script_compatibility".to_string());
                            m
                        })
                        .with_condition("file_pattern_contains == 'script' OR file_pattern_contains == '.afsim' OR file_pattern_contains == '.wsf'")
                        .with_agent(DomainAgentDef::new("code_converter", "脚本语言迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("脚本语言迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("script_strategy_approval", "confirm", "是否批准此脚本迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("migration_approach", "choice", "选择迁移方案")    .with_options(vec!["完全重写为 TypeScript (推荐)".to_string(), "创建兼容层（脚本解析器）".to_string(), "嵌入式脚本引擎 (JS sandbox)".to_string(), "可视化 DSL (React Flow)".to_string()])    .with_required(true), DomainUserInputField::new("api_scope", "text", "核心 API 覆盖范围")    .with_placeholder("例如: Platform, Track, Event, Weapon")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-csharp-to-rust-analysis", "C# 到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 C# 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## C# → Rust 映射配置\n", "{{mapping_profile.csharp_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 C# 特有模式（async/await、泛型、LINQ、属性等）\n", "2. 规划 Rust async/await 转换\n", "3. 规划泛型到 Rust trait/const 泛型映射\n", "4. 规划 LINQ 查询到 Rust 迭代器适配器转换\n", "5. 规划 .NET 依赖注入到 Rust 状态模式转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{csharp_rust_analysis{patterns_identified[{pattern, count, complexity}], async_conversion{methods_count, complexity, challenges[]}, generic_mapping{constraints, lifetime_annotations}, linq_mapping{query_types[], implementation_strategy}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("csharp_rust_config".to_string(), "config/languages/mappings/csharp-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'csharp' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("code_auditor", "C# 到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-java-to-typescript-analysis", "Java 到 TypeScript 迁移分析")
                        .with_prompt(
                            concat!("你作为 Java 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Java → TypeScript 映射配置\n", "{{mapping_profile.java_to_typescript_mapping}}\n", "\n", "请完成：\n", "1. 识别 Java 特有模式（Stream API、注解、接口继承等）\n", "2. 规划 Spring Bean 到 NestJS Provider 转换\n", "3. 规划 JPA 到 Prisma/TypeORM 转换\n", "4. 规划 Stream API 到 RxJS/Array 方法转换\n", "5. 规划注解到 TypeScript 装饰器转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{java_typescript_analysis{patterns_identified[{pattern, count, complexity}], spring_mapping{controllers, services, repositories}, jpa_mapping{entities, relationships, queries}, stream_mapping{operations[], rxjs_equivalents[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("java_ts_config".to_string(), "config/languages/mappings/java-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'java' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("code_auditor", "Java 到 TypeScript 迁移专家"))
                    ,
                    DomainStepDef::agent("a-dis-hla-migration", "DIS/HLA 分布式仿真协议迁移")
                        .with_prompt(
                            concat!("你作为分布式仿真协议迁移专家，执行 DIS/HLA 协议的迁移：\n", "\n", "## 分布式仿真协议配置\n", "{{mapping_profile.distributed_simulation_protocol}}\n", "\n", "请完成：\n", "1. 分析现有 DIS/HLA 实现（识别 PDU 类型、网络传输方式）\n", "2. 实现 DIS PDU 数据结构（Entity State、Fire、Explosion 等）\n", "3. 实现 DIS 二进制编解码\n", "4. 实现 UDP/TCP 网络传输\n", "5. 实现 Tauri 事件推送（DIS → React 前端）\n", "6. 选择 HLA 实现方案（自定义轻量 RTI 推荐）\n", "7. 实现 RtiAmbassador 核心接口\n", "8. 实现时间管理服务\n", "9. 实现发布/订阅机制\n", "\n", "输出 JSON：{dis_hla_migration{dis_analysis{pdu_types_used[], network_transport[], realtime_requirements}, dis_implementation{pdu_structures[{type, file_path}], codec_implementation[{encoding, file_path}], transport_implementation[{type, file_path}]}, hla_implementation{approach_selected, rti_interface{file_path}, time_manager{file_path}, pub_sub{file_path}}, test_plan{unit_tests[], integration_tests[], performance_tests[]}, issues[{type, description, severity, workaround}]}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("dis_hla_config".to_string(), "config/languages/mappings/{{source_lang}}-to-{{target_lang}}.yaml#distributed_simulation_protocol".to_string());
                            m
                        })
                        .with_condition("file_pattern_contains == 'dis' OR file_pattern_contains == 'hla' OR file_pattern_contains == 'pdu'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "分布式仿真协议迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("DIS/HLA 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("dis_hla_approval", "confirm", "是否批准此 DIS/HLA 迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("hla_approach", "choice", "HLA 实现方案")    .with_options(vec!["自定义轻量 RTI (推荐)".to_string(), "开源 RTI (OpenRTI/CERTI)".to_string(), "消息总线 (Redis Streams)".to_string()])    .with_required(true), DomainUserInputField::new("priority_protocols", "text", "优先实现的协议")    .with_placeholder("例如: DIS Entity State PDU, HLA 时间管理")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-python-to-rust-analysis", "Python 到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 Python 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Python → Rust 映射配置\n", "{{mapping_profile.python_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 Python 特有模式（动态类型、GIL、反射、装饰器等）\n", "2. 规划 Python 动态类型到 Rust 静态类型转换\n", "3. 规划 Python 异步（asyncio）到 Rust async/await 转换\n", "4. 规划 Python 装饰器到 Rust 宏/trait 转换\n", "5. 规划 Python 迭代器到 Rust 迭代器适配器转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{python_rust_analysis{patterns_identified[{pattern, count, complexity}], type_mapping{dynamic_to_static{modules_to_convert[], type_inference_strategy}}, async_conversion{asyncio_patterns[], tokio_equivalents[]}, decorator_mapping{decorators[], macro_or_trait_equivalents[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("python_rust_config".to_string(), "config/languages/mappings/python-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'python' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("code_auditor", "Python 到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-python-to-typescript-analysis", "Python 到 TypeScript 迁移分析")
                        .with_prompt(
                            concat!("你作为 Python 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Python → TypeScript 映射配置\n", "{{mapping_profile.python_to_typescript_mapping}}\n", "\n", "请完成：\n", "1. 识别 Python 特有模式（动态类型、装饰器、列表推导式等）\n", "2. 规划 Python 动态类型到 TypeScript 静态类型转换\n", "3. 规划 Python 装饰器到 TypeScript 装饰器转换\n", "4. 规划 Python 数据类到 TypeScript interface/class 转换\n", "5. 规划 Python 异步到 TypeScript async/await 转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{python_ts_analysis{patterns_identified[{pattern, count, complexity}], type_mapping{dynamic_to_static{modules[], type_inference_strategy}}, decorator_mapping{decorators[], equivalents[]}}, data_class_mapping{pydantic_dataclasses, ts_interface_equivalents[]}, async_conversion{patterns[], equivalents[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("python_ts_config".to_string(), "config/languages/mappings/python-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'python' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("code_auditor", "Python 到 TypeScript 迁移专家"))
                    ,
                    DomainStepDef::agent("a-go-to-typescript-analysis", "Go 到 TypeScript 迁移分析")
                        .with_prompt(
                            concat!("你作为 Go 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Go → TypeScript 映射配置\n", "{{mapping_profile.go_to_typescript_mapping}}\n", "\n", "请完成：\n", "1. 识别 Go 特有模式（goroutine、channel、struct、interface{} 等）\n", "2. 规划 Go goroutine/channel 到 TypeScript async/await/Promise 转换\n", "3. 规划 Go struct 到 TypeScript interface/class 转换\n", "4. 规划 Go interface{} 到 TypeScript union type/any 转换\n", "5. 规划 Go 错误处理到 TypeScript 错误处理转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{go_ts_analysis{patterns_identified[{pattern, count, complexity}], concurrency_mapping{goroutines_to_async{count, complexity}, channels_to_async_iterators{count, complexity}}, type_mapping{struct_to_interface[], interface_removal_strategy}, error_handling_mapping{patterns[], equivalents[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("go_ts_config".to_string(), "config/languages/mappings/go-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'go' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("code_auditor", "Go 到 TypeScript 迁移专家"))
                    ,
                    DomainStepDef::agent("a-go-to-rust-analysis", "Go 到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 Go 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Go → Rust 映射配置\n", "{{mapping_profile.go_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 Go 特有模式（goroutine、channel、defer、panic/recover 等）\n", "2. 规划 Go goroutine/channel 到 Rust tokio/t_channel 转换\n", "3. 规划 Go interface{} 到 Rust enum/trait object 转换\n", "4. 规划 Go defer 到 Rust Drop/RAII 转换\n", "5. 规划 Go 错误处理到 Rust Result 转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{go_rust_analysis{patterns_identified[{pattern, count, complexity}], concurrency_mapping{goroutines_to_tokio{count, complexity}, channels_to_mpsc{count, complexity}}, type_mapping{interface_to_trait[], any_to_enum_or_trait}, error_handling_mapping{panic_to_result[], recover_to_match}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("go_rust_config".to_string(), "config/languages/mappings/go-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'go' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("code_auditor", "Go 到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-typescript-to-rust-analysis", "TypeScript 后端到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 TypeScript 后端到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## TypeScript → Rust 映射配置\n", "{{mapping_profile.typescript_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 TypeScript 特有模式（async/await、interface、class、泛型、装饰器等）\n", "2. 规划 TypeScript class 到 Rust struct + impl 转换\n", "3. 规划 TypeScript interface 到 Rust trait 转换\n", "4. 规划 TypeScript async/await 到 Rust async/await + tokio 转换\n", "5. 规划 TypeScript Web 框架（Express/NestJS）到 Axum 转换\n", "6. 规划 TypeScript 数据库访问（Prisma/TypeORM）到 SeaORM/SQLx 转换\n", "7. 规划依赖注入模式到 Rust 手动组装转换\n", "8. 评估迁移复杂度\n", "\n", "输出 JSON：{ts_rust_analysis{patterns_identified[{pattern, count, complexity}], web_framework_mapping{framework_type, routes_count, middleware_count}, db_mapping{orm_type, queries_count, migrations_count}, di_mapping{decorator_count, injection_points[]}, async_mapping{async_functions_count, promise_patterns[]}, estimated_effort, risks[{type, description, severity, mitigation}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("ts_rust_config".to_string(), "config/languages/mappings/typescript-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'typescript' AND target_language == 'rust' AND file_pattern_contains != '.tsx'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "TypeScript 后端到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-typescript-to-rust-implementation", "TypeScript 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 TypeScript 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.typescript_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. 基础类型迁移\n", "   - number → f64 / i64 / u64（根据用途选择）\n", "   - string → String\n", "   - boolean → bool\n", "   - undefined/null → Option<T>\n", "   - T[] → Vec<T>\n", "\n", "2. 数据结构迁移\n", "   - interface → trait + struct\n", "   - type alias → type / newtype\n", "   - enum → enum (代数数据类型)\n", "   - class → struct + impl\n", "\n", "3. 异步迁移\n", "   - async/await → async/await (Rust)\n", "   - Promise → impl Future / BoxFuture\n", "   - EventEmitter → broadcast/mpsc channel\n", "\n", "4. Web 框架迁移（如果有）\n", "   - Express/NestJS → Axum\n", "   - 装饰器 → 手动路由注册\n", "   - 中间件 → tower middleware\n", "\n", "5. 数据库迁移（如果有）\n", "   - Prisma/TypeORM → SeaORM\n", "   - 查询构建器 → SeaORM 查询\n", "   - 数据验证 → validator crate\n", "\n", "6. 错误处理迁移\n", "   - try-catch → match + Result<T, E>\n", "   - throw → Err(MyError::Variant)\n", "   - 自定义错误类 → thiserror enum\n", "\n", "7. 序列化迁移\n", "   - JSON → serde + serde_json\n", "   - class-transformer → 自定义 From 实现\n", "\n", "8. 依赖注入迁移\n", "   - DI 装饰器 → 构造函数注入 + Arc<dyn>\n", "   - 服务定位器 → 启动时手动组装\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], patterns_used[{pattern, rust_implementation}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-typescript-to-rust-analysis.result".to_string());
                            m.insert("ts_rust_config".to_string(), "config/languages/mappings/typescript-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'typescript' AND target_language == 'rust' AND file_pattern_contains != '.tsx'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "TypeScript 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("TypeScript → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("ts_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("framework_migration", "choice", "Web 框架迁移方式")    .with_options(vec!["完整迁移到 Axum".to_string(), "保留 API 接口，内部重构".to_string(), "混合模式（新功能用 Axum，旧代码保留）".to_string()])    .with_required(true), DomainUserInputField::new("priority_migration", "text", "优先迁移的模块")    .with_placeholder("例如: 用户认证, 数据访问层, API 网关"), DomainUserInputField::new("testing_strategy", "choice", "测试策略")    .with_options(vec!["先迁移核心逻辑 + 单元测试".to_string(), "先迁移 API 层 + 集成测试".to_string(), "并行迁移 + 对比测试".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-csharp-to-typescript-analysis", "C# 到 TypeScript 迁移分析")
                        .with_prompt(
                            concat!("你作为 C# 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## C# → TypeScript 映射配置\n", "{{mapping_profile.csharp_to_typescript_mapping}}\n", "\n", "请完成：\n", "1. 识别 C# 特有模式（LINQ、异步、泛型、属性等）\n", "2. 规划 C# UI 组件（WPF/WinForms）到 React 组件转换\n", "3. 规划布局系统（Grid/StackPanel/DockPanel）到 Flexbox/CSS Grid 转换\n", "4. 规划事件处理模式转换\n", "5. 规划 LINQ 查询到 Array 方法转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{csharp_ts_analysis{patterns_identified[{pattern, count, complexity}], ui_mapping{components_count, layouts_count, complexity}, linq_mapping{query_types[], array_equivalents[]}, async_mapping{async_methods_count, task_patterns[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("csharp_ts_config".to_string(), "config/languages/mappings/csharp-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'csharp' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("frontend_framework_specialist", "C# 到 TypeScript 迁移专家"))
                    ,
                    DomainStepDef::agent("a-java-to-rust-analysis", "Java 到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 Java 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## Java → Rust 映射配置\n", "{{mapping_profile.java_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 Java/Spring 特有模式（注解、依赖注入、JPA、Stream API 等）\n", "2. 规划 Spring Boot 控制器到 Axum 路由转换\n", "3. 规划 Spring 注解到显式配置转换\n", "4. 规划 JPA/Hibernate 到 SeaORM/SQLx 转换\n", "5. 规划 Spring 依赖注入到 Rust 构造函数注入转换\n", "6. 评估迁移复杂度\n", "\n", "输出 JSON：{java_rust_analysis{patterns_identified[{pattern, count, complexity}], spring_mapping{controllers_count, services_count, repositories_count}, orm_mapping{entities_count, queries_complexity}, di_mapping{beans_count, injection_points[]}, estimated_effort}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("java_rust_config".to_string(), "config/languages/mappings/java-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'java' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "Java 到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-python-to-rust-implementation", "Python 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 Python 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.python_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. 动态类型 → 静态类型转换\n", "2. asyncio → tokio 转换\n", "3. 装饰器 → 宏/trait 转换\n", "4. 生成器 → Rust 迭代器转换\n", "5. 错误处理 → Result<T, E> 转换\n", "6. 常用库迁移\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], patterns_used[{pattern, rust_implementation}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-python-to-rust-analysis.result".to_string());
                            m.insert("py_rust_config".to_string(), "config/languages/mappings/python-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'python' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "Python 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Python → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("py_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("priority_migration", "text", "优先迁移的模块")    .with_placeholder("例如: 数据处理, API 服务, 计算密集型模块")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-cross-language-verification", "跨语言等价性验证")
                        .with_prompt(
                            concat!("你作为跨语言等价性验证专家，验证迁移后的代码与原代码的行为等价性：\n", "\n", "## 验证范围\n", "1. 功能等价性\n", "   - 输入输出一致性\n", "   - 边界条件处理\n", "   - 错误处理行为\n", "\n", "2. 性能等价性\n", "   - 基准测试对比\n", "   - 资源使用对比\n", "   - 响应时间对比\n", "\n", "3. 数据等价性\n", "   - 数据格式兼容性\n", "   - 序列化/反序列化一致性\n", "   - 数据库操作等价性\n", "\n", "4. API 等价性\n", "   - 接口契约一致性\n", "   - 请求/响应格式\n", "   - 状态码/错误码\n", "\n", "请完成：\n", "1. 执行功能等价性测试\n", "2. 执行性能基准测试\n", "3. 执行 API 契约测试\n", "4. 生成等价性验证报告\n", "\n", "输出 JSON：{verification_result{functionality{passed, tests_total, discrepancies[]}, performance{source_baseline, target_metrics, regression_detected}, api_contract{endpoints_verified[], breaking_changes[]}, data_integrity{migrated_records, validation_errors[]}, overall_status: \"PASS\" | \"WARN\" | \"FAIL\"}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("migration_result".to_string(), "a-execute.result".to_string());
                            m.insert("equivalence_check".to_string(), "a-equivalence-check.result".to_string());
                            m
                        })
                        .with_condition("source_language != target_language")
                        .with_agent(DomainAgentDef::new("cross_language_verifier", "跨语言等价性验证专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("跨语言等价性验证已完成。请查看验证结果");
                            ui = ui.with_fields(vec![DomainUserInputField::new("verification_status", "choice", "验证结果")    .with_options(vec!["通过 (PASS)".to_string(), "有警告 (WARN)".to_string(), "未通过 (FAIL)".to_string()])    .with_required(true), DomainUserInputField::new("action", "choice", "下一步操作")    .with_options(vec!["继续验收流程".to_string(), "修复差异后重新验证".to_string(), "接受差异并记录文档".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-csharp-to-typescript-implementation", "C# 到 TypeScript 实现迁移")
                        .with_prompt(
                            concat!("你作为 C# 到 TypeScript 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.csharp_to_typescript_mapping}}\n", "\n", "## 实现顺序\n", "1. UI 组件迁移（WPF/WinForms → React）\n", "2. 布局系统迁移（Grid/StackPanel → Flexbox/Grid）\n", "3. 数据访问层迁移（EF Core → Prisma/TypeORM）\n", "4. 业务逻辑迁移\n", "5. 异步模式迁移\n", "6. 国际化集成\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], components_migrated[{original, replacement, complexity}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-csharp-to-typescript-analysis.result".to_string());
                            m.insert("csharp_ts_config".to_string(), "config/languages/mappings/csharp-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'csharp' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("frontend_framework_specialist", "C# 到 TypeScript 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("C# → TypeScript 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("csharp_ts_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("ui_migration", "choice", "UI 组件迁移方式")    .with_options(vec!["完整迁移到 Ant Design 组件".to_string(), "保留样式，使用原生 HTML 元素".to_string(), "自定义组件封装".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-go-to-typescript-implementation", "Go 到 TypeScript 实现迁移")
                        .with_prompt(
                            concat!("你作为 Go 到 TypeScript 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.go_to_typescript_mapping}}\n", "\n", "## 实现顺序\n", "1. 基础类型迁移\n", "   - int → number (大整数用 bigint)\n", "   - string → string\n", "   - bool → boolean\n", "   - []T → T[]\n", "   - map[K]V → Map<K, V>\n", "\n", "2. 数据结构迁移\n", "   - struct → interface / class\n", "   - interface{} → any / 具体类型\n", "   - interface (Go) → interface (TypeScript)\n", "   - 嵌入 → extends / 组合\n", "\n", "3. 并发模型迁移\n", "   - goroutine → async function\n", "   - channel → Promise / EventEmitter\n", "   - sync.WaitGroup → Promise.all\n", "   - sync.Mutex → 串行执行队列\n", "\n", "4. 错误处理迁移\n", "   - if err != nil → try-catch / Result 模式\n", "   - 多返回值 → 元组 / 对象\n", "   - panic/recover → throw / try-catch\n", "\n", "5. 依赖注入迁移\n", "   - 手动注入 → 构造函数注入\n", "   - 全局变量 → 依赖注入容器\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], concurrency_patterns[{pattern, typescript_implementation}], error_handling{migrated_patterns[], remaining_issues[]}, tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-go-to-typescript-analysis.result".to_string());
                            m.insert("go_ts_config".to_string(), "config/languages/mappings/go-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'go' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "Go 到 TypeScript 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Go → TypeScript 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("go_ts_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("concurrency_migration", "choice", "并发模型迁移方式")    .with_options(vec!["保持 async/await，移除 channel 模式".to_string(), "使用 EventEmitter 替代 channel".to_string(), "使用 Worker Threads 处理 CPU 密集任务".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-go-to-rust-implementation", "Go 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 Go 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.go_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. 基础类型迁移\n", "   - int → i64\n", "   - string → String\n", "   - bool → bool\n", "   - []T → Vec<T>\n", "   - map[K]V → HashMap<K, V>\n", "\n", "2. 数据结构迁移\n", "   - struct → struct (值语义)\n", "   - interface → trait\n", "   - interface{} → enum / trait object\n", "   - 嵌入 → 组合\n", "\n", "3. 并发模型迁移\n", "   - goroutine → tokio::spawn\n", "   - channel → mpsc / broadcast\n", "   - sync.WaitGroup → JoinHandle.await\n", "   - sync.Mutex → tokio::sync::Mutex\n", "   - context → tokio::time::timeout\n", "\n", "4. 内存安全迁移\n", "   - 指针 → 引用 / Arc<RwLock<T>>\n", "   - nil 检查 → Option<T>\n", "   - defer → Drop trait (RAII)\n", "\n", "5. 错误处理迁移\n", "   - if err != nil → Result<T, E> + ?\n", "   - panic/recover → match / 错误处理\n", "   - 多返回值 → Result<T, E>\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], concurrency_patterns[{pattern, rust_implementation, complexity}], memory_safety{pointers_migrated[], nil_checks_converted[], defer_to_drop[]}, error_handling{errors_converted[], panics_handled[]}, ownership_issues[{file, issue, severity, fix}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-go-to-rust-analysis.result".to_string());
                            m.insert("go_rust_config".to_string(), "config/languages/mappings/go-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'go' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "Go 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Go → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("go_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("ownership_strategy", "choice", "所有权处理策略")    .with_options(vec!["优先使用引用（&T），仅在必要时使用所有权".to_string(), "使用 Arc<RwLock<T>> 处理共享状态".to_string(), "使用 Message Passing 模型（channel）".to_string()])    .with_required(true), DomainUserInputField::new("unsafe_assessment", "choice", "unsafe 代码处理方式")    .with_options(vec!["严格审计，尽量消除 unsafe".to_string(), "保留必要的 unsafe，添加详细文档".to_string(), "使用 safe wrapper 封装 unsafe".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-python-to-typescript-implementation", "Python 到 TypeScript 实现迁移")
                        .with_prompt(
                            concat!("你作为 Python 到 TypeScript 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.python_to_typescript_mapping}}\n", "\n", "## 实现顺序\n", "1. 类型系统迁移\n", "   - 动态类型 → 静态类型（添加类型注解）\n", "   - number → number\n", "   - str → string\n", "   - bool → boolean\n", "   - list → T[]\n", "   - dict → Record<K, V>\n", "   - Optional[T] → T | undefined\n", "\n", "2. 数据结构迁移\n", "   - @dataclass → interface / type\n", "   - Pydantic model → interface + zod validation\n", "   - class → class (TypeScript)\n", "   - 继承 → extends\n", "\n", "3. 异步模型迁移\n", "   - asyncio.gather → Promise.all\n", "   - asyncio.sleep → setTimeout / Promise\n", "   - async with → try-finally\n", "   - async for → for-await-of\n", "\n", "4. 装饰器迁移\n", "   - 函数装饰器 → 高阶函数 / 装饰器\n", "   - 类装饰器 → 装饰器 / mixin\n", "\n", "5. 常用库迁移\n", "   - numpy → number[] / 自定义实现\n", "   - pandas → 数据操作函数\n", "   - requests → fetch / axios\n", "   - SQLAlchemy → ORM / 原生查询\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], type_annotations_added[], dataclass_migrations[{original, target_pattern}], async_patterns[{pattern, typescript_implementation}], decorators_migrated[{decorator, equivalent}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-python-to-typescript-analysis.result".to_string());
                            m.insert("py_ts_config".to_string(), "config/languages/mappings/python-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'python' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "Python 到 TypeScript 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Python → TypeScript 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("py_ts_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("type_inference", "choice", "类型推断策略")    .with_options(vec!["严格模式：所有变量必须有类型注解".to_string(), "渐进式：优先推断，关键路径添加注解".to_string(), "宽松模式：先迁移再逐步完善类型".to_string()])    .with_required(true), DomainUserInputField::new("validation_strategy", "choice", "数据验证方案")    .with_options(vec!["使用 zod（推荐，类型安全）".to_string(), "使用 class-validator".to_string(), "手动验证函数".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-java-to-rust-implementation", "Java 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 Java 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.java_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. Web 框架迁移\n", "   - @RestController → Axum Router\n", "   - @RequestMapping → 路由注册\n", "   - @GetMapping/PostMapping → get()/post()\n", "   - @PathVariable → Path<T>\n", "   - @RequestBody → Json<T>\n", "\n", "2. 依赖注入迁移\n", "   - @Autowired → 构造函数注入\n", "   - @Service/@Component → struct + impl\n", "   - @Repository → trait + impl\n", "   - 应用启动时手动组装\n", "\n", "3. 数据访问迁移\n", "   - JPA Entity → SeaORM Entity\n", "   - Repository 接口 → trait + impl\n", "   - Query 构建器 → SeaORM 查询\n", "   - DTO → struct + From 实现\n", "\n", "4. 异步模型迁移\n", "   - CompletableFuture → impl Future / async\n", "   - Stream API → 迭代器适配器\n", "   - @Async → tokio::spawn\n", "\n", "5. 错误处理迁移\n", "   - try-catch → match + Result<T, E>\n", "   - 自定义异常 → thiserror enum\n", "   - @ControllerAdvice → 统一错误响应\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], controllers_migrated[{original, routes_count, complexity}], di_beans[{bean_type, injection_points_count}], entities_migrated[{original, table_name}], async_patterns[{pattern, rust_implementation}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-java-to-rust-analysis.result".to_string());
                            m.insert("java_rust_config".to_string(), "config/languages/mappings/java-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'java' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("backend_integration_specialist", "Java 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Java → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("java_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("web_framework", "choice", "Web 框架选择")    .with_options(vec!["Axum (推荐，社区活跃)".to_string(), "Actix Web (成熟稳定)".to_string(), "hyper (底层框架)".to_string()])    .with_required(true), DomainUserInputField::new("orm_choice", "choice", "ORM 选择")    .with_options(vec!["SeaORM (推荐，异步友好)".to_string(), "SQLx (原生 SQL)".to_string(), "Diesel (类型安全)".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-csharp-to-rust-implementation", "C# 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 C# 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.csharp_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. 类型系统迁移\n", "   - int → i32 / i64\n", "   - string → String\n", "   - bool → bool\n", "   - List<T> → Vec<T>\n", "   - Dictionary<K,V> → HashMap<K, V>\n", "   - Optional<T> → Option<T>\n", "\n", "2. 数据结构迁移\n", "   - class → struct + impl\n", "   - interface → trait\n", "   - 泛型 → 泛型 (generics)\n", "   - LINQ → 迭代器适配器\n", "\n", "3. 异步模型迁移\n", "   - async/await → async/await + tokio\n", "   - Task<T> → impl Future<Output = T>\n", "   - CancellationToken → tokio::time::timeout\n", "\n", "4. 内存安全迁移\n", "   - 指针 → 引用 / Arc<RwLock<T>>\n", "   - Nullable → Option<T>\n", "   - IDisposable → Drop trait\n", "   - unsafe code → safe wrapper\n", "\n", "5. Web 框架迁移（如果有）\n", "   - ASP.NET Core → Axum\n", "   - Controllers → Router handlers\n", "   - Middleware → tower middleware\n", "   - Filters → 统一错误处理\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], types_migrated[{csharp_type, rust_type, count}], async_patterns[{pattern, rust_implementation, complexity}], linq_queries[{query_type, rust_equivalent}], web_framework_migration{controllers_migrated, middleware_migrated}, tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-csharp-to-rust-analysis.result".to_string());
                            m.insert("csharp_rust_config".to_string(), "config/languages/mappings/csharp-to-rust.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'csharp' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "C# 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("C# → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("csharp_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("aspnet_migration", "choice", "ASP.NET Core 迁移方式")    .with_options(vec!["完整迁移到 Axum (推荐)".to_string(), "保留 REST API，内部重构".to_string(), "混合模式（微服务渐进迁移）".to_string()])    .with_required(true), DomainUserInputField::new("dotnet_features", "multi_select", "需要迁移的 .NET 特性")    .with_options(vec!["LINQ 查询".to_string(), "异步 (async/await)".to_string(), "依赖注入".to_string(), "中间件管道".to_string(), "过滤器 (Filter)".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-test-generator", "自动化测试生成")
                        .with_prompt(
                            concat!("你作为自动化测试生成专家，为迁移后的代码生成完整的测试套件：\n", "\n", "## 目标语言测试框架配置\n", "{{target_lang_test_config}}\n", "\n", "## 源语言黄金测试基线\n", "{{behavioral_snapshot.result}}\n", "\n", "## 迁移结果摘要\n", "{{migration_result_summary}}\n", "\n", "## 测试生成策略\n", "1. 单元测试生成\n", "   - 为每个迁移的函数/方法生成单元测试\n", "   - 覆盖正例、反例、边界条件\n", "   - 基于源语言黄金测试用例生成等价测试\n", "   - 目标覆盖率：≥ 80%\n", "\n", "2. 集成测试生成\n", "   - API 接口测试\n", "     · Rust 目标：axum::test / reqwest\n", "     · TypeScript 目标：supertest / vitest\n", "   - 数据库集成测试\n", "   - 服务间调用测试\n", "   - IPC 通信测试（Tauri 应用）\n", "\n", "3. 契约测试生成\n", "   - 源系统 API 契约文档化\n", "   - 目标系统 API 兼容性验证\n", "   - 自动生成 API 对比测试\n", "   - OpenAPI/Swagger 规范一致性检查\n", "\n", "4. 性能测试生成\n", "   - 基准测试\n", "     · Rust 目标：criterion.rs\n", "     · TypeScript 目标：vitest benchmark / bench.js\n", "   - 负载测试脚本（k6 / autocannon）\n", "   - 性能回归检测\n", "   - 与源系统性能基线对比\n", "\n", "5. 等价性测试生成（跨语言迁移专用）\n", "   - 源系统 vs 目标系统输出对比\n", "   - 黄金测试用例执行对比\n", "   - 随机输入下的行为一致性验证\n", "   - 边界条件等价性测试\n", "   - 副作用序列一致性验证\n", "\n", "6. UI 测试生成（前端迁移专用）\n", "   - 组件渲染测试（@testing-library/react）\n", "   - 用户交互测试\n", "   - 视觉回归测试（Storybook / Percy）\n", "   - 响应式布局测试\n", "\n", "7. 国际化测试生成（i18n 迁移专用）\n", "   - 多语言切换测试\n", "   - 翻译完整性检查\n", "   - 插值变量测试\n", "   - 日期/数字格式化测试\n", "\n", "## 测试框架选择（根据目标语言自动配置）\n", "- Rust 目标：cargo test + criterion.rs + axum::test\n", "- TypeScript 目标：Vitest + Playwright + @testing-library/react\n", "- API 测试：postman/newman / axum::test / supertest\n", "- UI 测试：Storybook + Percy\n", "- 性能测试：k6 / autocannon / criterion.rs\n", "\n", "请完成：\n", "1. 分析需要测试的模块和函数\n", "2. 基于黄金测试基线生成等价单元测试\n", "3. 生成集成测试代码\n", "4. 生成 API 契约测试\n", "5. 生成性能测试脚本\n", "6. 生成跨语言等价性测试\n", "7. 生成 UI 测试（如果涉及 UI 迁移）\n", "8. 生成国际化测试（如果涉及 i18n 迁移）\n", "9. 执行测试并生成报告\n", "\n", "输出 JSON：{test_suite{unit_tests{files_generated, total_cases, coverage_target, golden_test_baselines_used}, integration_tests{files_generated, api_tests, db_tests, ipc_tests}, contract_tests{api_endpoints_verified, breaking_changes_found, schema_validation}, performance_tests{benchmarks_count, load_test_scripts, baseline_comparison}, equivalence_tests{golden_cases_verified, random_cases, edge_cases, pass_rate, discrepancies[]}, ui_tests{components_tested, visual_regression_tests}# 可选, i18n_tests{languages_covered, translation_completeness, formatting_tests}# 可选}, execution_result{all_tests_passed, failures[{test_name, reason, fix_suggestion}], coverage_achieved{unit, integration, overall}, performance_regressions_detected, equivalence_verdict{pass|fail|review_required, discrepancies_found[]}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("migration_result".to_string(), "a-execute.result".to_string());
                            m.insert("code_structure".to_string(), "a-complexity.result".to_string());
                            m.insert("behavioral_snapshot".to_string(), "a-behavior-snapshot.result".to_string());
                            m.insert("quality_baseline".to_string(), "a-quality-baseline.result".to_string());
                            m.insert("equivalence_check".to_string(), "a-equivalence-check.result".to_string());
                            m.insert("target_lang_config".to_string(), "config/languages/target_languages/{{target_lang}}.yaml".to_string());
                            m
                        })
                        .with_condition("migration_completed == true")
                        .with_agent(DomainAgentDef::new("test_engineer", "自动化测试生成专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("测试套件已生成。请确认是否执行测试");
                            ui = ui.with_fields(vec![DomainUserInputField::new("test_approval", "confirm", "是否执行自动生成的测试？")    .with_options(vec!["立即执行".to_string(), "先审查后执行".to_string(), "跳过测试".to_string()])    .with_required(true), DomainUserInputField::new("test_scope", "choice", "测试范围")    .with_options(vec!["全部测试（单元+集成+契约+性能+等价性）".to_string(), "仅单元测试 + 集成测试".to_string(), "仅契约测试（API 兼容性）".to_string(), "仅等价性测试（跨语言对比）".to_string()])    .with_required(true), DomainUserInputField::new("coverage_target", "choice", "覆盖率目标")    .with_options(vec!["80% (标准)".to_string(), "90% (严格)".to_string(), "70% (基础)".to_string()])    .with_required(true), DomainUserInputField::new("equivalence_depth", "choice", "等价性测试深度")    .with_options(vec!["完整黄金测试 + 随机测试 + 边界测试（推荐）".to_string(), "仅黄金测试用例".to_string(), "仅 API 接口对比".to_string()])    .with_required(true), DomainUserInputField::new("performance_baseline", "text", "性能基准（可选，基准测试的目标值）")    .with_placeholder("例如: API 响应时间 < 100ms, 吞吐量 > 1000 RPS"), DomainUserInputField::new("ui_testing", "choice", "是否生成 UI 测试（如适用）")    .with_options(vec!["是，生成组件测试 + 视觉回归测试".to_string(), "仅组件功能测试".to_string(), "不需要 UI 测试".to_string()]), DomainUserInputField::new("i18n_testing", "choice", "是否生成国际化测试（如适用）")    .with_options(vec!["是，生成完整 i18n 测试".to_string(), "仅翻译完整性检查".to_string(), "不需要 i18n 测试".to_string()])]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-java-to-typescript-implementation", "Java 到 TypeScript 实现迁移")
                        .with_prompt(
                            concat!("你作为 Java 到 TypeScript 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.java_to_typescript_mapping}}\n", "\n", "## 实现顺序\n", "1. Web 框架迁移\n", "   - Spring Boot → Express/NestJS\n", "   - @RestController → Controller 类\n", "   - @Service → Provider/Service\n", "   - @Repository → 数据访问层\n", "\n", "2. 数据结构迁移\n", "   - POJO → interface / class\n", "   - 枚举 → enum\n", "   - 集合类 → Array / Map\n", "   - 泛型 → 泛型 (generics)\n", "\n", "3. UI 框架迁移（如果有）\n", "   - Swing/FX 组件 → React 组件\n", "   - 布局管理器 → Flexbox/CSS Grid\n", "   - 事件处理 → React 事件处理\n", "\n", "4. 数据库迁移\n", "   - JPA/Hibernate → Prisma/TypeORM\n", "   - JDBC → 数据库驱动\n", "   - 连接池 → 连接池库\n", "\n", "5. 测试框架迁移\n", "   - JUnit → Vitest/Jest\n", "   - Mockito → Jest Mock\n", "   - Spring Test → Supertest\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], spring_components[{type, count, target_equivalent}], ui_components[{original, target_component, complexity}], db_entities[{original, orm_model}], tests_migrated[{original_framework, target_framework}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-java-to-typescript-analysis.result".to_string());
                            m.insert("java_ts_config".to_string(), "config/languages/mappings/java-to-typescript.yaml".to_string());
                            m
                        })
                        .with_condition("source_language == 'java' AND target_language == 'typescript'")
                        .with_agent(DomainAgentDef::new("ts_framework_specialist", "Java 到 TypeScript 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("Java → TypeScript 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("java_ts_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("backend_framework", "choice", "后端框架选择")    .with_options(vec!["NestJS (推荐，类似 Spring 结构)".to_string(), "Express (简单灵活)".to_string(), "Fastify (高性能)".to_string()])    .with_required(true), DomainUserInputField::new("frontend_framework", "choice", "前端框架选择")    .with_options(vec!["React (推荐)".to_string(), "Vue".to_string(), "无前端（纯 API）".to_string()])    .with_required(true)]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-cpp-to-rust-analysis", "C++ 到 Rust 迁移分析")
                        .with_prompt(
                            concat!("你作为 C++ 到 Rust 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## C++ → Rust 映射配置\n", "{{mapping_profile.cpp_to_rust_mapping}}\n", "\n", "请完成：\n", "1. 识别 C++ 特有模式（RAII、智能指针、模板、虚函数等）\n", "2. 规划 C++ 类型到 Rust 类型转换（int→i32、std::string→String、std::vector→Vec 等）\n", "3. 规划内存模型转换（new/delete→所有权系统、shared_ptr→Arc/Rc、unique_ptr→Box）\n", "4. 规划类/继承到 struct+trait 转换\n", "5. 规划模板到泛型转换\n", "6. 规划并发原语（std::thread→tokio::task、std::mutex→tokio::sync::Mutex）\n", "7. 评估迁移复杂度和 unsafe 代码比例\n", "\n", "输出 JSON：{cpp_rust_analysis{patterns_identified[{pattern, count, complexity}], memory_patterns[{pattern, count, risk_level}], class_hierarchy{classes_count, inheritance_depth, virtual_functions}}, template_usage{template_functions, template_classes, complexity}, concurrency_patterns[{pattern, count, migration_complexity}], unsafe_code{lines_total, distribution_by_module[]}, estimated_effort, risk_assessment{high_risk_modules[], unsafe_percentage}}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("cpp_rust_config".to_string(), "config/languages/mappings/cpp-to-rust.yaml".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("memory_audit".to_string(), "a-memory-audit.result".to_string());
                            m
                        })
                        .with_condition("source_language == 'cpp' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "C++ 到 Rust 迁移专家"))
                    ,
                    DomainStepDef::agent("a-cpp-to-rust-implementation", "C++ 到 Rust 实现迁移")
                        .with_prompt(
                            concat!("你作为 C++ 到 Rust 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.cpp_to_rust_mapping}}\n", "\n", "## 实现顺序\n", "1. 基础类型迁移\n", "   - int/long → i32/i64\n", "   - unsigned → u32/u64\n", "   - float/double → f32/f64\n", "   - std::string → String\n", "   - std::vector<T> → Vec<T>\n", "   - std::map<K,V> → HashMap/BTreeMap\n", "\n", "2. 智能指针与所有权迁移\n", "   - unique_ptr → Box<T>\n", "   - shared_ptr → Arc<T>/Rc<T>\n", "   - weak_ptr → Weak<T>\n", "   - 裸指针 → 引用(&T)或Box<T>\n", "\n", "3. 类与继承迁移\n", "   - class → struct + impl\n", "   - 虚函数 → trait 方法\n", "   - 继承 → 组合 + trait\n", "   - 多重继承 → 多 trait 实现\n", "\n", "4. 模板到泛型迁移\n", "   - template<typename T> → 泛型 <T: Trait>\n", "   - 模板特化 → 枚举匹配\n", "\n", "5. 并发模型迁移\n", "   - std::thread → tokio::spawn\n", "   - std::mutex → tokio::sync::Mutex\n", "   - std::atomic → std::sync::atomic\n", "   - condition_variable → tokio::sync::Notify\n", "\n", "6. 内存安全迁移\n", "   - RAII → Drop trait\n", "   - new/delete → 所有权系统\n", "   - 内存泄漏 → 编译期安全保证\n", "\n", "7. 错误处理迁移\n", "   - try-catch → match + Result<T, E>\n", "   - 自定义异常 → thiserror 枚举\n", "   - 错误码 → Result 类型\n", "\n", "8. STL 到 Rust 标准库迁移\n", "   - std::sort → slice::sort\n", "   - std::find → Iterator::find\n", "   - std::transform → Iterator::map\n", "   - std::accumulate → Iterator::sum\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], types_migrated[{cpp_type, rust_type, count}], pointer_conversion{unique_ptr_to_box[], shared_ptr_to_arc[], raw_pointer_resolved[]}, class_conversion{classes_to_struct[], virtual_to_trait[], inheritance_to_composition[]}, template_to_generic{functions[], classes[]}, concurrency_migration{threads_to_tokio[], mutex_to_async_mutex[]}, stl_to_std{algorithms_migrated[], containers_migrated[]}, unsafe_code_generated{files, blocks_count, rationale}, tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string(), "Bash".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-cpp-to-rust-analysis.result".to_string());
                            m.insert("cpp_rust_config".to_string(), "config/languages/mappings/cpp-to-rust.yaml".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("memory_audit".to_string(), "a-memory-audit.result".to_string());
                            m.insert("behavioral_snapshot".to_string(), "a-behavior-snapshot.result".to_string());
                            m
                        })
                        .with_condition("source_language == 'cpp' AND target_language == 'rust'")
                        .with_agent(DomainAgentDef::new("cpp_rust_migrator", "C++ 到 Rust 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("C++ → Rust 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("cpp_rust_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("migration_strategy", "choice", "迁移执行策略")    .with_options(vec!["按模块逐步迁移（推荐）".to_string(), "按功能分层迁移".to_string(), "核心库先行，业务后移".to_string()])    .with_required(true), DomainUserInputField::new("ownership_policy", "choice", "所有权处理策略")    .with_options(vec!["严格所有权，尽量使用引用（推荐）".to_string(), "适当使用 Arc<RwLock> 简化并发".to_string(), "消息传递模型（channel）".to_string()])    .with_required(true), DomainUserInputField::new("unsafe_tolerance", "choice", "unsafe 代码容忍度")    .with_options(vec!["零 unsafe，完全安全（推荐）".to_string(), "允许必要的 unsafe（封装在小范围）".to_string(), "保留部分 unsafe，后续优化".to_string()])    .with_required(true), DomainUserInputField::new("priority_modules", "text", "优先迁移的模块")    .with_placeholder("例如: 核心算法, 数据结构, 工具库")]);
                            ui
                        })
                    ,
                    DomainStepDef::agent("a-cpp-to-typescript-analysis", "C++ 到 TypeScript 迁移分析")
                        .with_prompt(
                            concat!("你作为 C++ (Qt) 到 TypeScript 迁移专家，分析源代码并规划迁移方案：\n", "\n", "## C++ → TypeScript 映射配置\n", "{{mapping_profile.cpp_to_typescript_mapping}}\n", "\n", "请完成：\n", "1. 识别 Qt 特有模式（QObject 元对象系统、信号槽、事件循环等）\n", "2. 规划 Qt Widgets 到 React 组件转换\n", "3. 规划布局系统（QVBoxLayout/QHBoxLayout/QGridLayout）到 Flexbox/Grid 转换\n", "4. 规划 Qt 信号槽到 React 事件处理/Hooks 转换\n", "5. 规划 Qt 数据类型到 TypeScript 类型转换（QString→string、QList→T[]等）\n", "6. 规划 QVariant 到 union type/any 转换\n", "7. 评估 UI 迁移复杂度\n", "\n", "输出 JSON：{cpp_ts_analysis{qt_patterns[{pattern, count, complexity}], widget_analysis{widgets_count, layouts_count, custom_widgets[]}, signal_slot_analysis{signals_count, slots_count, connections_count}, data_type_analysis{qstring_usage, qlist_usage, qvariant_usage}, state_management_analysis{qobject_subclasses, state_holders[], signal_triggers[]}, estimated_effort, ui_complexity_score}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("asset_scan".to_string(), "a-asset-scan.result".to_string());
                            m.insert("cpp_ts_config".to_string(), "config/languages/mappings/cpp-to-typescript.yaml".to_string());
                            m.insert("ui_mapping".to_string(), "a-ui-mapping.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m
                        })
                        .with_condition("source_language == 'cpp' AND target_language == 'typescript' AND source_framework_contains == 'Qt'")
                        .with_agent(DomainAgentDef::new("frontend_framework_specialist", "C++ 到 TypeScript 迁移专家"))
                    ,
                    DomainStepDef::agent("a-cpp-to-typescript-implementation", "C++ 到 TypeScript 实现迁移")
                        .with_prompt(
                            concat!("你作为 C++ (Qt) 到 TypeScript 迁移实现专家，执行实际的代码迁移：\n", "\n", "## 迁移映射配置\n", "{{mapping_profile.cpp_to_typescript_mapping}}\n", "\n", "## 实现顺序\n", "1. 基础类型迁移\n", "   - int/long → number/bigint\n", "   - std::string/QString → string\n", "   - std::vector/QList/QVector → T[]\n", "   - std::map/QMap → Map<K,V>/Record<K,V>\n", "   - std::optional → T | undefined\n", "\n", "2. Qt Widgets 到 React 组件迁移\n", "   - QWidget → React 函数组件\n", "   - QPushButton → Ant Design Button\n", "   - QLineEdit → Ant Design Input\n", "   - QComboBox → Ant Design Select\n", "   - QTableWidget/QTableView → Ant Design Table\n", "   - QTreeWidget/QTreeView → Ant Design Tree\n", "   - QListWidget/QListView → Ant Design List\n", "   - QDialog → Ant Design Modal\n", "   - QMainWindow → 页面布局组件\n", "\n", "3. 布局系统迁移\n", "   - QVBoxLayout → flex flex-col\n", "   - QHBoxLayout → flex flex-row\n", "   - QGridLayout → grid grid-cols-N\n", "   - QFormLayout → Ant Design Form\n", "   - QStackedLayout → 条件渲染 / Tabs\n", "   - QSplitter → Resizable 面板组\n", "\n", "4. 信号槽到事件处理迁移\n", "   - connect(sender, signal, receiver, slot) → <Component onSignal={handler} />\n", "   - emit signal(args) → props.callback(args) / EventEmitter\n", "   - Qt::DirectConnection → 同步调用\n", "   - Qt::QueuedConnection → 异步调用(setTimeout/Promise)\n", "\n", "5. 状态管理迁移\n", "   - QObject 属性 → useState/useReducer\n", "   - Q_PROPERTY → Zustand Store state\n", "   - 信号触发状态更新 → Zustand action / setState\n", "\n", "6. 样式系统迁移\n", "   - QSS → CSS Modules / Tailwind\n", "   - Qt 样式表语法 → CSS 语法\n", "   - QColor → CSS 颜色值\n", "   - QFont → CSS font\n", "\n", "7. Qt 工具类迁移\n", "   - QString 方法 → String 方法\n", "   - QList 方法 → Array 方法\n", "   - QMap 方法 → Map/Object 方法\n", "   - QDateTime → Date/Intl.DateTimeFormat\n", "   - QByteArray → Uint8Array\n", "\n", "输出 JSON：{migration_result{files_migrated[{source_file, target_file, complexity}], widgets_migrated[{qt_widget, react_component, complexity}], layouts_converted[{qt_layout, css_strategy, components}], signal_slot_conversion{connections_migrated[], async_patterns_added[]}, state_management{stores_created[], state_variables_migrated[], actions_mapped[]}, styles_converted{qss_files, css_files, migration_rate}, qt_utils_migrated[{qt_class, typescript_equivalent, methods_converted[]}], tests_added[{test_file, coverage}], issues[{type, description, severity, workaround}]}}")
                        )
                        .with_tools(vec!["Grep".to_string(), "FileRead".to_string(), "FileWrite".to_string()])
                        .with_inputs({
                            let mut m = HashMap::new();
                            m.insert("analysis".to_string(), "a-cpp-to-typescript-analysis.result".to_string());
                            m.insert("cpp_ts_config".to_string(), "config/languages/mappings/cpp-to-typescript.yaml".to_string());
                            m.insert("ui_mapping".to_string(), "a-ui-mapping.result".to_string());
                            m.insert("type_mapping".to_string(), "a-type-mapping.result".to_string());
                            m.insert("behavioral_snapshot".to_string(), "a-behavior-snapshot.result".to_string());
                            m.insert("i18n_analysis".to_string(), "a-i18n-analysis.result".to_string());
                            m
                        })
                        .with_condition("source_language == 'cpp' AND target_language == 'typescript' AND source_framework_contains == 'Qt'")
                        .with_agent(DomainAgentDef::new("frontend_framework_specialist", "C++ 到 TypeScript 实现迁移专家"))
                        .with_user_input({
                            let mut ui = DomainUserInput::new();
                            ui = ui.with_mode("approval_gate");
                            ui = ui.with_prompt("C++ (Qt) → TypeScript 迁移方案已完成。请确认迁移策略");
                            ui = ui.with_fields(vec![DomainUserInputField::new("cpp_ts_approval", "confirm", "是否批准此迁移方案？")    .with_options(vec!["批准执行".to_string(), "调整方案".to_string(), "延后处理".to_string()])    .with_required(true), DomainUserInputField::new("ui_migration_scope", "choice", "UI 迁移范围")    .with_options(vec!["完整 UI 迁移（推荐）".to_string(), "核心 UI 优先，边缘延后".to_string(), "仅迁移特定页面/组件".to_string()])    .with_required(true), DomainUserInputField::new("component_strategy", "choice", "组件实现策略")    .with_options(vec!["优先使用 Ant Design 组件（推荐）".to_string(), "使用原生 HTML + Tailwind".to_string(), "混合使用 + 自定义组件".to_string()])    .with_required(true), DomainUserInputField::new("state_management", "choice", "状态管理方案")    .with_options(vec!["Zustand（推荐，轻量）".to_string(), "Redux Toolkit（复杂应用）".to_string(), "React Context（简单应用）".to_string()])    .with_required(true), DomainUserInputField::new("priority_widgets", "text", "优先迁移的 Widget/页面")    .with_placeholder("例如: 主窗口, 配置对话框, 数据表格")]);
                            ui
                        })
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-security-review", "安全审查")
                .with_description("代码安全审计: 漏洞扫描、依赖检查")
                .with_icon("🛡️")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-scan", "扫描")
                        .with_prompt("代码扫描: SAST、依赖漏洞、密钥泄露")
                    ,
                    DomainStepDef::agent("a-analyze-s", "分析")
                        .with_prompt("分析扫描结果、优先级排序")
                    ,
                    DomainStepDef::agent("a-fix", "修复")
                        .with_prompt("实施修复方案、验证修复效果")
                    ,
                ]),
            DomainWorkflowDef::new("wf-eng-tech-debt", "技术债管理")
                .with_description("识别、评估和消除代码库中的技术债务")
                .with_icon("📉")
                .with_tags(vec!["opc".to_string(), "engineering".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-debt-scan", "扫描")
                        .with_prompt("扫描代码库识别技术债项")
                    ,
                    DomainStepDef::agent("a-debt-prioritize", "排序")
                        .with_prompt("按影响和修复成本排序")
                    ,
                    DomainStepDef::agent("a-debt-repay", "偿还")
                        .with_prompt("制定还款计划并执行")
                    ,
                ])
        ]
    }

    /// 财务与会计 (finance) — 3 个工作流
    pub fn finance() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-fin-budget", "预算编制")
                .with_description("编制年度预算和滚动预测")
                .with_icon("💰")
                .with_tags(vec!["opc".to_string(), "finance".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-budget-review", "回顾")
                        .with_prompt("回顾上期预算执行和差异")
                    ,
                    DomainStepDef::agent("a-budget-plan", "编制")
                        .with_prompt("编制各部门预算方案")
                    ,
                    DomainStepDef::agent("a-budget-approve", "审批")
                        .with_prompt("审批预算并确定最终版本")
                    ,
                ]),
            DomainWorkflowDef::new("wf-fin-cost-analysis", "成本分析")
                .with_description("全面分析运营成本和优化空间")
                .with_icon("📉")
                .with_tags(vec!["opc".to_string(), "finance".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-cost-collect", "采集")
                        .with_prompt("采集各类成本数据")
                    ,
                    DomainStepDef::agent("a-cost-analyze", "分析")
                        .with_prompt("按类别、项目、客户分析成本")
                    ,
                    DomainStepDef::agent("a-cost-optimize", "优化")
                        .with_prompt("制定降本方案并评估影响")
                    ,
                ]),
            DomainWorkflowDef::new("wf-fin-tax", "税务申报")
                .with_description("准备和提交税务申报材料")
                .with_icon("🧾")
                .with_tags(vec!["opc".to_string(), "finance".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-tax-collect", "收集")
                        .with_prompt("收集收入、支出、抵扣凭证")
                    ,
                    DomainStepDef::agent("a-tax-calc", "计算")
                        .with_prompt("计算应纳税额和抵扣项")
                    ,
                    DomainStepDef::agent("a-tax-submit", "申报")
                        .with_prompt("生成报表并提交申报")
                    ,
                ])
        ]
    }

    /// 游戏开发 (gamedev) — 3 个工作流
    pub fn gamedev() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-gd-concept", "游戏概念设计")
                .with_description("从想法到完整的游戏设计文档")
                .with_icon("🎮")
                .with_tags(vec!["opc".to_string(), "gamedev".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-gd-idea", "概念生成")
                        .with_prompt("生成游戏核心玩法和概念")
                    ,
                    DomainStepDef::agent("a-gd-design", "游戏设计")
                        .with_prompt("设计游戏机制、关卡、角色")
                    ,
                    DomainStepDef::agent("a-gd-doc", "文档")
                        .with_prompt("编写游戏设计文档")
                    ,
                ]),
            DomainWorkflowDef::new("wf-gd-prototype", "游戏原型")
                .with_description("快速搭建可玩原型验证核心机制")
                .with_icon("🎮")
                .with_tags(vec!["opc".to_string(), "gamedev".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-gd-proto-core", "核心机制")
                        .with_prompt("实现核心玩法和控制")
                    ,
                    DomainStepDef::agent("a-gd-proto-test", "玩法测试")
                        .with_prompt("测试核心机制可玩性")
                    ,
                    DomainStepDef::agent("a-gd-proto-iterate", "迭代")
                        .with_prompt("根据测试反馈优化")
                    ,
                ]),
            DomainWorkflowDef::new("wf-gd-qa", "游戏测试")
                .with_description("全面测试游戏功能和体验")
                .with_icon("🎮")
                .with_tags(vec!["opc".to_string(), "gamedev".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-gd-qa-functional", "功能测试")
                        .with_prompt("测试游戏功能和系统")
                    ,
                    DomainStepDef::agent("a-gd-qa-balance", "平衡测试")
                        .with_prompt("测试数值平衡和难度曲线")
                    ,
                    DomainStepDef::agent("a-gd-qa-ux", "体验测试")
                        .with_prompt("测试用户体验和引导")
                    ,
                ])
        ]
    }

    /// 地理信息系统 (gis) — 4 个工作流
    pub fn gis() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-gis-3d-scene", "三维场景")
                .with_description("构建三维地理场景和可视化")
                .with_icon("🏔️")
                .with_tags(vec!["opc".to_string(), "gis".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-3d-data", "数据采集")
                        .with_prompt("采集地形、影像和模型数据")
                    ,
                    DomainStepDef::agent("a-3d-scene", "场景搭建")
                        .with_prompt("构建三维场景和光照")
                    ,
                    DomainStepDef::agent("a-3d-publish", "发布")
                        .with_prompt("发布交互式三维场景")
                    ,
                ]),
            DomainWorkflowDef::new("wf-gis-analysis", "空间分析")
                .with_description("地理空间数据分析和可视化")
                .with_icon("🗺️")
                .with_tags(vec!["opc".to_string(), "gis".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-gis-data", "数据准备")
                        .with_prompt("收集和预处理空间数据")
                    ,
                    DomainStepDef::agent("a-gis-analyze", "分析")
                        .with_prompt("执行空间分析: 缓冲、叠加、网络")
                    ,
                    DomainStepDef::agent("a-gis-viz", "可视化")
                        .with_prompt("生成地图和分析报告")
                    ,
                ]),
            DomainWorkflowDef::new("wf-gis-drone", "无人机测绘")
                .with_description("无人机航拍数据处理和分析")
                .with_icon("🛸")
                .with_tags(vec!["opc".to_string(), "gis".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-drone-plan", "飞行规划")
                        .with_prompt("规划飞行路线和采集参数")
                    ,
                    DomainStepDef::agent("a-drone-process", "数据处理")
                        .with_prompt("处理航拍影像生成正射影像和DSM")
                    ,
                    DomainStepDef::agent("a-drone-analyze", "分析")
                        .with_prompt("从测绘数据提取信息")
                    ,
                ]),
            DomainWorkflowDef::new("wf-gis-mapping", "地图制作")
                .with_description("专业地图制图和符号设计")
                .with_icon("🗺️")
                .with_tags(vec!["opc".to_string(), "gis".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-map-data", "数据准备")
                        .with_prompt("准备基础地理数据和要素")
                    ,
                    DomainStepDef::agent("a-map-design", "地图设计")
                        .with_prompt("设计地图样式、符号和标注")
                    ,
                    DomainStepDef::agent("a-map-export", "输出")
                        .with_prompt("导出地图成品")
                    ,
                ])
        ]
    }

    /// 市场营销 (marketing) — 10 个工作流
    pub fn marketing() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-mkt-ab-test", "A/B测试")
                .with_description("设计、执行和分析A/B测试")
                .with_icon("🧪")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-ab-design", "实验设计")
                        .with_prompt("确定假设、变量、样本量")
                    ,
                    DomainStepDef::agent("a-ab-execute", "执行")
                        .with_prompt("配置实验并启动流量分配")
                    ,
                    DomainStepDef::agent("a-ab-analyze", "分析")
                        .with_prompt("统计分析结果、得出结论")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-analytics", "营销数据分析")
                .with_description("整合多渠道数据生成营销洞察")
                .with_icon("📈")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-mkt-data", "数据采集")
                        .with_prompt("采集各渠道营销数据")
                    ,
                    DomainStepDef::agent("a-mkt-dashboard", "仪表盘")
                        .with_prompt("构建营销数据仪表盘")
                    ,
                    DomainStepDef::agent("a-mkt-insight", "洞察")
                        .with_prompt("提取关键洞察和改进建议")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-brand-guide", "品牌指南")
                .with_description("制定品牌视觉和文案规范")
                .with_icon("🎨")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-brand-audit", "品牌审计")
                        .with_prompt("审计现有品牌资产和一致性")
                    ,
                    DomainStepDef::agent("a-brand-voice", "品牌调性")
                        .with_prompt("定义品牌声音、语调、关键词")
                    ,
                    DomainStepDef::agent("a-brand-guide", "规范文档")
                        .with_prompt("输出品牌指南文档")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-competitive-intel", "竞争情报")
                .with_description("持续监控竞争对手动态")
                .with_icon("🕵️")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-comp-map", "竞品地图")
                        .with_prompt("识别核心竞争对手和跟踪维度")
                    ,
                    DomainStepDef::agent("a-comp-monitor", "持续监控")
                        .with_prompt("收集竞品产品更新、定价变化")
                    ,
                    DomainStepDef::agent("a-comp-report", "情报报告")
                        .with_prompt("生成竞争情报周报")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-email-campaign", "邮件营销活动")
                .with_description("策划、设计、发送邮件营销活动并分析效果")
                .with_icon("📧")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-email-plan", "活动策划")
                        .with_prompt("确定目标受众、主题、内容策略")
                    ,
                    DomainStepDef::agent("a-email-create", "内容创作")
                        .with_prompt("撰写邮件文案、设计排版、CTA")
                    ,
                    DomainStepDef::agent("a-email-analyze", "效果分析")
                        .with_prompt("分析打开率、点击率、转化率")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-influencer", "红人营销")
                .with_description("寻找和对接行业KOL合作")
                .with_icon("🤳")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-inf-search", "红人搜索")
                        .with_prompt("搜索行业相关KOL和内容创作者")
                    ,
                    DomainStepDef::agent("a-inf-evaluate", "评估")
                        .with_prompt("评估粉丝质量、互动率、匹配度")
                    ,
                    DomainStepDef::agent("a-inf-outreach", "触达")
                        .with_prompt("制定触达方案并发送合作邀请")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-pr-plan", "公关传播计划")
                .with_description("策划新闻稿和媒体传播方案")
                .with_icon("📰")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-pr-story", "故事挖掘")
                        .with_prompt("挖掘有新闻价值的故事角度")
                    ,
                    DomainStepDef::agent("a-pr-write", "撰稿")
                        .with_prompt("撰写新闻稿和媒体资料包")
                    ,
                    DomainStepDef::agent("a-pr-distribute", "分发")
                        .with_prompt("确定媒体名单并分发稿件")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-seo-audit", "SEO审计")
                .with_description("网站SEO全面审计并优化")
                .with_icon("🔍")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-seo-scan", "扫描")
                        .with_prompt("技术SEO: 爬虫、索引、页面速度")
                    ,
                    DomainStepDef::agent("a-seo-content", "内容审查")
                        .with_prompt("关键词策略、内容质量、Meta标签")
                    ,
                    DomainStepDef::agent("a-seo-optimize", "优化")
                        .with_prompt("实施优化建议并监控排名变化")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-social-plan", "社交媒体策略")
                .with_description("制定社交媒体系运营和内容日历")
                .with_icon("📱")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-soc-audit", "账号审计")
                        .with_prompt("审计现有社交账号和内容表现")
                    ,
                    DomainStepDef::agent("a-soc-strategy", "策略制定")
                        .with_prompt("确定平台、内容类型、发布频率")
                    ,
                    DomainStepDef::agent("a-soc-calendar", "内容日历")
                        .with_prompt("创建月度内容日历和排期")
                    ,
                ]),
            DomainWorkflowDef::new("wf-mkt-webinar", "线上研讨会")
                .with_description("策划和执行线上研讨会活动")
                .with_icon("🎥")
                .with_tags(vec!["opc".to_string(), "marketing".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-webinar-plan", "活动策划")
                        .with_prompt("确定主题、嘉宾、时间、渠道")
                    ,
                    DomainStepDef::agent("a-webinar-prep", "准备")
                        .with_prompt("准备PPT、推广素材、测试环境")
                    ,
                    DomainStepDef::agent("a-webinar-follow", "跟进")
                        .with_prompt("发送回放、收集反馈、线索评分")
                    ,
                ])
        ]
    }

    /// 付费媒体 (paidmedia) — 2 个工作流
    pub fn paidmedia() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-pm-campaign", "广告活动管理")
                .with_description("规划、执行和优化付费广告活动")
                .with_icon("📺")
                .with_tags(vec!["opc".to_string(), "paidmedia".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-pm-plan", "广告规划")
                        .with_prompt("确定目标受众、预算、渠道")
                    ,
                    DomainStepDef::agent("a-pm-create", "广告制作")
                        .with_prompt("制作广告创意和落地页")
                    ,
                    DomainStepDef::agent("a-pm-optimize", "优化")
                        .with_prompt("分析表现数据并优化")
                    ,
                ]),
            DomainWorkflowDef::new("wf-pm-roi", "广告ROI分析")
                .with_description("分析各渠道广告投入产出比")
                .with_icon("📊")
                .with_tags(vec!["opc".to_string(), "paidmedia".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-roi-collect", "数据采集")
                        .with_prompt("采集各渠道成本和收入")
                    ,
                    DomainStepDef::agent("a-roi-calc", "计算")
                        .with_prompt("计算ROI和客户获取成本")
                    ,
                    DomainStepDef::agent("a-roi-report", "报告")
                        .with_prompt("输出ROI报告和预算建议")
                    ,
                ])
        ]
    }

    /// 项目管理 (pm) — 3 个工作流
    pub fn pm() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-pm-risk", "风险管理")
                .with_description("识别、评估和应对项目风险")
                .with_icon("⚠️")
                .with_tags(vec!["opc".to_string(), "pm".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-risk-identify", "风险识别")
                        .with_prompt("识别技术和业务风险")
                    ,
                    DomainStepDef::agent("a-risk-assess", "评估")
                        .with_prompt("评估影响和概率")
                    ,
                    DomainStepDef::agent("a-risk-respond", "应对")
                        .with_prompt("制定风险应对策略")
                    ,
                ]),
            DomainWorkflowDef::new("wf-pm-sprint", "Sprint规划")
                .with_description("迭代冲刺规划和任务分配")
                .with_icon("📋")
                .with_tags(vec!["opc".to_string(), "pm".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-sprint-backlog", "Backlog梳理")
                        .with_prompt("梳理和估算待办项")
                    ,
                    DomainStepDef::agent("a-sprint-plan", "冲刺规划")
                        .with_prompt("确定冲刺目标和任务分配")
                    ,
                    DomainStepDef::agent("a-sprint-review", "冲刺回顾")
                        .with_prompt("回顾冲刺成果和改进点")
                    ,
                ]),
            DomainWorkflowDef::new("wf-pm-status", "项目状态报告")
                .with_description("生成项目周报和状态更新")
                .with_icon("📊")
                .with_tags(vec!["opc".to_string(), "pm".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-status-collect", "数据收集")
                        .with_prompt("收集团队进展和指标")
                    ,
                    DomainStepDef::agent("a-status-write", "报告撰写")
                        .with_prompt("撰写项目状态报告")
                    ,
                    DomainStepDef::agent("a-status-distribute", "分发")
                        .with_prompt("发送报告并安排跟进")
                    ,
                ])
        ]
    }

    /// 产品管理 (product) — 3 个工作流
    pub fn product() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-prod-launch", "产品发布")
                .with_description("新产品/功能发布流程")
                .with_icon("🚀")
                .with_tags(vec!["opc".to_string(), "product".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-launch-plan", "发布计划")
                        .with_prompt("制定发布计划和时间线")
                    ,
                    DomainStepDef::agent("a-launch-prep", "发布准备")
                        .with_prompt("准备发布说明、营销材料")
                    ,
                    DomainStepDef::agent("a-launch-exec", "执行发布")
                        .with_prompt("执行发布并监控指标")
                    ,
                ]),
            DomainWorkflowDef::new("wf-prod-roadmap", "产品路线图")
                .with_description("制定季度产品路线图")
                .with_icon("🗺️")
                .with_tags(vec!["opc".to_string(), "product".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-road-collect", "需求收集")
                        .with_prompt("收集用户反馈、数据分析、市场趋势")
                    ,
                    DomainStepDef::agent("a-road-prioritize", "优先级排序")
                        .with_prompt("按影响和资源排序功能")
                    ,
                    DomainStepDef::agent("a-road-publish", "发布")
                        .with_prompt("输出产品路线图并同步团队")
                    ,
                ]),
            DomainWorkflowDef::new("wf-prod-spec", "产品规格书")
                .with_description("编写功能规格和验收标准")
                .with_icon("📄")
                .with_tags(vec!["opc".to_string(), "product".to_string()])
                .with_profile_id("opc-cpo-cpo-product-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-spec-req", "需求分析")
                        .with_prompt("分析用户故事和功能需求")
                    ,
                    DomainStepDef::agent("a-spec-write", "编写")
                        .with_prompt("编写功能规格和验收标准")
                    ,
                    DomainStepDef::agent("a-spec-review", "评审")
                        .with_prompt("与技术团队评审可行性")
                    ,
                ])
        ]
    }

    /// 销售与商务 (sales) — 5 个工作流
    pub fn sales() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-sal-account-plan", "客户规划")
                .with_description("制定关键客户年度合作计划")
                .with_icon("🤝")
                .with_tags(vec!["opc".to_string(), "sales".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-account-review", "客户回顾")
                        .with_prompt("回顾合作历史、满意度、收入")
                    ,
                    DomainStepDef::agent("a-account-plan", "年度计划")
                        .with_prompt("制定年度目标、策略、里程碑")
                    ,
                    DomainStepDef::agent("a-account-review-plan", "审核")
                        .with_prompt("内部审核计划可行性")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sal-deal-strategy", "交易策略")
                .with_description("制定大客户交易赢单策略")
                .with_icon("🏆")
                .with_tags(vec!["opc".to_string(), "sales".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-deal-analyze", "分析")
                        .with_prompt("分析客户需求、决策链、预算")
                    ,
                    DomainStepDef::agent("a-deal-strategy", "策略")
                        .with_prompt("制定赢单策略和行动计划")
                    ,
                    DomainStepDef::agent("a-deal-execute", "执行")
                        .with_prompt("执行策略并跟踪进展")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sal-outbound", "外呼获客")
                .with_description("制定和执行主动外呼获客策略")
                .with_icon("📞")
                .with_tags(vec!["opc".to_string(), "sales".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-outbound-target", "目标画像")
                        .with_prompt("定义理想客户画像和名单")
                    ,
                    DomainStepDef::agent("a-outbound-script", "话术准备")
                        .with_prompt("准备外呼话术和常见问题")
                    ,
                    DomainStepDef::agent("a-outbound-execute", "执行")
                        .with_prompt("执行外呼并记录反馈")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sal-pipeline-review", "商机复盘")
                .with_description("销售商机管道的全面复盘")
                .with_icon("📊")
                .with_tags(vec!["opc".to_string(), "sales".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-pipe-list", "商机列表")
                        .with_prompt("列出所有活跃商机和阶段")
                    ,
                    DomainStepDef::agent("a-pipe-analyze", "分析")
                        .with_prompt("分析瓶颈、预计收入、风险")
                    ,
                    DomainStepDef::agent("a-pipe-plan", "行动计划")
                        .with_prompt("制定下周跟进计划")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sal-proposal", "方案建议书")
                .with_description("为客户撰写定制化方案建议书")
                .with_icon("📄")
                .with_tags(vec!["opc".to_string(), "sales".to_string()])
                .with_profile_id("opc-cmo-cmo-content-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-prop-needs", "需求确认")
                        .with_prompt("确认客户需求和决策标准")
                    ,
                    DomainStepDef::agent("a-prop-write", "方案撰写")
                        .with_prompt("撰写方案建议书: 方案、价值、报价")
                    ,
                    DomainStepDef::agent("a-prop-review", "内部审查")
                        .with_prompt("审查方案质量和竞品定位")
                    ,
                ])
        ]
    }

    /// 安全与合规 (security) — 4 个工作流
    pub fn security() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-sec-compliance", "合规审计")
                .with_description("检查安全合规标准和差距")
                .with_icon("✅")
                .with_tags(vec!["opc".to_string(), "security".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-comp-standard", "标准对照")
                        .with_prompt("确定适用的安全标准和框架")
                    ,
                    DomainStepDef::agent("a-comp-audit", "审计")
                        .with_prompt("逐项检查合规性")
                    ,
                    DomainStepDef::agent("a-comp-report", "报告")
                        .with_prompt("输出合规报告和整改计划")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sec-incident", "安全事件响应")
                .with_description("检测、分析和响应安全事件")
                .with_icon("🚨")
                .with_tags(vec!["opc".to_string(), "security".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-incident-detect", "检测")
                        .with_prompt("确认安全事件类型和范围")
                    ,
                    DomainStepDef::agent("a-incident-respond", "响应")
                        .with_prompt("执行应急响应和止损措施")
                    ,
                    DomainStepDef::agent("a-incident-review", "复盘")
                        .with_prompt("事故复盘和改进计划")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sec-pentest", "渗透测试")
                .with_description("对应用和基础设施进行渗透测试")
                .with_icon("🔓")
                .with_tags(vec!["opc".to_string(), "security".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-pentest-scope", "范围确定")
                        .with_prompt("确定测试范围和目标")
                    ,
                    DomainStepDef::agent("a-pentest-exec", "执行")
                        .with_prompt("执行渗透测试并记录发现")
                    ,
                    DomainStepDef::agent("a-pentest-report", "报告")
                        .with_prompt("输出漏洞报告和修复建议")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sec-threat-intel", "威胁情报")
                .with_description("收集和分析最新安全威胁情报")
                .with_icon("🕵️")
                .with_tags(vec!["opc".to_string(), "security".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-threat-collect", "情报收集")
                        .with_prompt("收集行业威胁情报和安全公告")
                    ,
                    DomainStepDef::agent("a-threat-analyze", "分析")
                        .with_prompt("评估威胁影响和风险级别")
                    ,
                    DomainStepDef::agent("a-threat-act", "行动")
                        .with_prompt("制定防护措施和更新策略")
                    ,
                ])
        ]
    }

    /// 空间计算 (spatial) — 2 个工作流
    pub fn spatial() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-spatial-ar", "AR应用设计")
                .with_description("增强现实应用概念和交互设计")
                .with_icon("🥽")
                .with_tags(vec!["opc".to_string(), "spatial".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-ar-concept", "概念设计")
                        .with_prompt("设计AR应用核心交互模式")
                    ,
                    DomainStepDef::agent("a-ar-ux", "空间UI设计")
                        .with_prompt("设计3D空间用户界面和手势")
                    ,
                    DomainStepDef::agent("a-ar-prototype", "原型验证")
                        .with_prompt("搭建AR原型验证可行性")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spatial-scene", "空间场景")
                .with_description("构建沉浸式3D空间场景")
                .with_icon("🏠")
                .with_tags(vec!["opc".to_string(), "spatial".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-scene-layout", "场景规划")
                        .with_prompt("规划空间布局和交互区域")
                    ,
                    DomainStepDef::agent("a-scene-build", "场景构建")
                        .with_prompt("构建3D场景和光照")
                    ,
                    DomainStepDef::agent("a-scene-optimize", "优化")
                        .with_prompt("优化性能和用户体验")
                    ,
                ])
        ]
    }

    /// 专业服务 (specialized) — 10 个工作流
    pub fn specialized() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-spc-change-mgmt", "变更管理")
                .with_description("企业变革管理: 评估影响、制定沟通、执行")
                .with_icon("🔄")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-change-impact", "影响评估")
                        .with_prompt("评估变革对组织、流程、人员的影响")
                    ,
                    DomainStepDef::agent("a-change-plan", "实施计划")
                        .with_prompt("制定分阶段变革实施和沟通计划")
                    ,
                    DomainStepDef::agent("a-change-exec", "执行")
                        .with_prompt("监督执行并收集反馈调整")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-data-privacy", "数据隐私合规")
                .with_description("GDPR/个保法合规审计和整改")
                .with_icon("🔒")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-privacy-audit", "合规审计")
                        .with_prompt("审计数据采集、存储、处理流程")
                    ,
                    DomainStepDef::agent("a-privacy-gap", "差距分析")
                        .with_prompt("识别合规差距和风险等级")
                    ,
                    DomainStepDef::agent("a-privacy-fix", "整改实施")
                        .with_prompt("实施整改措施并验证")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-esg", "ESG报告")
                .with_description("环境、社会和治理报告编制")
                .with_icon("🌱")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-esg-collect", "数据收集")
                        .with_prompt("收集环境、社会、治理数据")
                    ,
                    DomainStepDef::agent("a-esg-measure", "指标计算")
                        .with_prompt("计算ESG关键指标和评分")
                    ,
                    DomainStepDef::agent("a-esg-report", "报告生成")
                        .with_prompt("生成ESG报告和改善路线图")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-grant", "项目申请")
                .with_description("撰写和提交项目申请")
                .with_icon("📝")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-grant-research", "资金研究")
                        .with_prompt("研究适合的项目和资助机构")
                    ,
                    DomainStepDef::agent("a-grant-write", "申请撰写")
                        .with_prompt("撰写项目申请书和预算")
                    ,
                    DomainStepDef::agent("a-grant-submit", "提交")
                        .with_prompt("最终审核并提交申请")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-hire", "招聘流程")
                .with_description("从职位描述到Offer的完整招聘")
                .with_icon("🎯")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-hire-jd", "职位描述")
                        .with_prompt("撰写职位描述和要求")
                    ,
                    DomainStepDef::agent("a-hire-screen", "简历筛选")
                        .with_prompt("筛选简历、安排面试")
                    ,
                    DomainStepDef::agent("a-hire-evaluate", "面试评估")
                        .with_prompt("综合评估候选人、产出报告")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-legal-review", "合同审查")
                .with_description("审查法律合同条款和风险")
                .with_icon("⚖️")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-cfo-cfo-financial-analyst")
                .with_steps(vec![
                    DomainStepDef::agent("a-legal-upload", "提交合同")
                        .with_prompt("提交合同文档和背景说明")
                    ,
                    DomainStepDef::agent("a-legal-review", "条款审查")
                        .with_prompt("审查关键条款、风险点、合规性")
                    ,
                    DomainStepDef::agent("a-legal-report", "审查报告")
                        .with_prompt("输出审查意见和修改建议")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-localization", "本地化")
                .with_description("产品和服务本地化适配")
                .with_icon("🌍")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-locale-audit", "本地化审计")
                        .with_prompt("审计需要本地化的内容和功能")
                    ,
                    DomainStepDef::agent("a-locale-translate", "翻译适配")
                        .with_prompt("翻译内容、适配格式和规范")
                    ,
                    DomainStepDef::agent("a-locale-verify", "验证")
                        .with_prompt("验证本地化质量和一致性")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-m-a", "并购整合")
                .with_description("并购后业务、团队、系统整合")
                .with_icon("🤝")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-ma-audit", "尽调审计")
                        .with_prompt("审计目标公司业务、技术、团队")
                    ,
                    DomainStepDef::agent("a-ma-plan", "整合计划")
                        .with_prompt("制定100天整合计划")
                    ,
                    DomainStepDef::agent("a-ma-exec", "执行")
                        .with_prompt("执行整合并监控关键指标")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-onboard", "员工入职")
                .with_description("新员工入职流程: 账号、文档、培训")
                .with_icon("📋")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-onboard-plan", "入职计划")
                        .with_prompt("制定入职计划和任务清单")
                    ,
                    DomainStepDef::agent("a-onboard-setup", "环境搭建")
                        .with_prompt("开通账号、配置设备、访问权限")
                    ,
                    DomainStepDef::agent("a-onboard-orient", "入职引导")
                        .with_prompt("公司介绍、团队介绍、首周任务")
                    ,
                ]),
            DomainWorkflowDef::new("wf-spc-supply-chain", "供应链优化")
                .with_description("分析和优化供应链效率")
                .with_icon("📦")
                .with_tags(vec!["opc".to_string(), "specialized".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-sc-audit", "供应链审计")
                        .with_prompt("审计采购、库存、物流各环节")
                    ,
                    DomainStepDef::agent("a-sc-optimize", "优化方案")
                        .with_prompt("制定降本增效方案")
                    ,
                    DomainStepDef::agent("a-sc-implement", "实施")
                        .with_prompt("实施优化并跟踪KPI")
                    ,
                ])
        ]
    }

    /// 战略规划 (strategy) — 2 个工作流
    pub fn strategy() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-strat-biz-plan", "商业计划书")
                .with_description("编写完整商业计划书")
                .with_icon("📄")
                .with_tags(vec!["opc".to_string(), "strategy".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-bp-summary", "执行摘要")
                        .with_prompt("撰写执行摘要和公司概述")
                    ,
                    DomainStepDef::agent("a-bp-market", "市场分析")
                        .with_prompt("市场分析、竞争分析、SWOT")
                    ,
                    DomainStepDef::agent("a-bp-financial", "财务预测")
                        .with_prompt("收入模型、成本、现金流预测")
                    ,
                ]),
            DomainWorkflowDef::new("wf-strat-market-entry", "市场进入策略")
                .with_description("制定新市场进入策略和计划")
                .with_icon("🎯")
                .with_tags(vec!["opc".to_string(), "strategy".to_string()])
                .with_profile_id("opc-ceo-ceo-business-strategist")
                .with_steps(vec![
                    DomainStepDef::agent("a-market-size", "市场分析")
                        .with_prompt("分析市场规模、竞争、进入壁垒")
                    ,
                    DomainStepDef::agent("a-market-strategy", "策略制定")
                        .with_prompt("制定进入策略: 渠道、定价、定位")
                    ,
                    DomainStepDef::agent("a-market-plan", "行动计划")
                        .with_prompt("制定执行计划和预算")
                    ,
                ])
        ]
    }

    /// 客户支持 (support) — 3 个工作流
    pub fn support() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-sup-faq", "FAQ知识库")
                .with_description("从客户问题提取和更新知识库")
                .with_icon("📚")
                .with_tags(vec!["opc".to_string(), "support".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-faq-collect", "收集")
                        .with_prompt("采集高频客户问题和解决方案")
                    ,
                    DomainStepDef::agent("a-faq-write", "编写")
                        .with_prompt("编写清晰的FAQ文档")
                    ,
                    DomainStepDef::agent("a-faq-publish", "发布")
                        .with_prompt("审核并发布到知识库")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sup-satisfaction", "客户满意度调查")
                .with_description("设计、发���和分析满意度调查")
                .with_icon("📊")
                .with_tags(vec!["opc".to_string(), "support".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-sat-design", "设计")
                        .with_prompt("设计调查问卷和评分体系")
                    ,
                    DomainStepDef::agent("a-sat-send", "发送")
                        .with_prompt("选择样本并发送调查")
                    ,
                    DomainStepDef::agent("a-sat-analyze", "分析")
                        .with_prompt("分析结果并制定改进计划")
                    ,
                ]),
            DomainWorkflowDef::new("wf-sup-ticket", "客户工单处理")
                .with_description("接收、分类、处理和关闭客户工单")
                .with_icon("🎫")
                .with_tags(vec!["opc".to_string(), "support".to_string()])
                .with_profile_id("opc-coo-coo-operations-manager")
                .with_steps(vec![
                    DomainStepDef::agent("a-ticket-categorize", "分类")
                        .with_prompt("分类工单类型和紧急程度")
                    ,
                    DomainStepDef::agent("a-ticket-solve", "解决")
                        .with_prompt("排查问题并给出解决方案")
                    ,
                    DomainStepDef::agent("a-ticket-follow", "跟进")
                        .with_prompt("确认客户满意并关闭工单")
                    ,
                ])
        ]
    }

    /// 测试与质量 (testing) — 3 个工作流
    pub fn testing() -> Vec<DomainWorkflowDef> {
        vec![
            DomainWorkflowDef::new("wf-tst-automation", "自动化测试")
                .with_description("编写和维护自动化测试脚本")
                .with_icon("🤖")
                .with_tags(vec!["opc".to_string(), "testing".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-tauto-pick", "选型")
                        .with_prompt("选择自动化框架和工具")
                    ,
                    DomainStepDef::agent("a-tauto-write", "编写")
                        .with_prompt("编写测试脚本并集成本地CI")
                    ,
                    DomainStepDef::agent("a-tauto-run", "运行")
                        .with_prompt("运行测试并分析结果")
                    ,
                ]),
            DomainWorkflowDef::new("wf-tst-perf", "性能测试")
                .with_description("负载测试和性能基准")
                .with_icon("⚡")
                .with_tags(vec!["opc".to_string(), "testing".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-tperf-script", "测试脚本")
                        .with_prompt("编写性能测试脚本和场景")
                    ,
                    DomainStepDef::agent("a-tperf-run", "执行")
                        .with_prompt("执行负载测试并监控")
                    ,
                    DomainStepDef::agent("a-tperf-report", "报告")
                        .with_prompt("输出性能报告和优化建议")
                    ,
                ]),
            DomainWorkflowDef::new("wf-tst-plan", "测试计划")
                .with_description("制定完整测试策略和计划")
                .with_icon("📋")
                .with_tags(vec!["opc".to_string(), "testing".to_string()])
                .with_profile_id("opc-cto-cto-ai-engineer")
                .with_steps(vec![
                    DomainStepDef::agent("a-tplan-analyze", "需求分析")
                        .with_prompt("分析功能需求和技术规格")
                    ,
                    DomainStepDef::agent("a-tplan-design", "测试设计")
                        .with_prompt("设计测试用例和测试场景")
                    ,
                    DomainStepDef::agent("a-tplan-review", "评审")
                        .with_prompt("评审测试覆盖率和优先级")
                    ,
                ])
        ]
    }

    pub fn create(id: &str) -> Option<DomainWorkflowDef> {
        match id.trim() {
            "academic" => Self::academic().into_iter().next(),
            "design" => Self::design().into_iter().next(),
            "engineering" => Self::engineering().into_iter().next(),
            "finance" => Self::finance().into_iter().next(),
            "gamedev" => Self::gamedev().into_iter().next(),
            "gis" => Self::gis().into_iter().next(),
            "marketing" => Self::marketing().into_iter().next(),
            "paidmedia" => Self::paidmedia().into_iter().next(),
            "pm" => Self::pm().into_iter().next(),
            "product" => Self::product().into_iter().next(),
            "sales" => Self::sales().into_iter().next(),
            "security" => Self::security().into_iter().next(),
            "spatial" => Self::spatial().into_iter().next(),
            "specialized" => Self::specialized().into_iter().next(),
            "strategy" => Self::strategy().into_iter().next(),
            "support" => Self::support().into_iter().next(),
            "testing" => Self::testing().into_iter().next(),
            _ => None,
        }
    }

    pub fn list_all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("academic", "学术研究"),
            ("design", "设计与创意"),
            ("engineering", "工程与开发"),
            ("finance", "财务与会计"),
            ("gamedev", "游戏开发"),
            ("gis", "地理信息系统"),
            ("marketing", "市场营销"),
            ("paidmedia", "付费媒体"),
            ("pm", "项目管理"),
            ("product", "产品管理"),
            ("sales", "销售与商务"),
            ("security", "安全与合规"),
            ("spatial", "空间计算"),
            ("specialized", "专业服务"),
            ("strategy", "战略规划"),
            ("support", "客户支持"),
            ("testing", "测试与质量"),
        ]
    }

    pub fn create_all() -> Vec<DomainWorkflowDef> {
        let mut all = Vec::new();
        all.extend(Self::academic());
        all.extend(Self::design());
        all.extend(Self::engineering());
        all.extend(Self::finance());
        all.extend(Self::gamedev());
        all.extend(Self::gis());
        all.extend(Self::marketing());
        all.extend(Self::paidmedia());
        all.extend(Self::pm());
        all.extend(Self::product());
        all.extend(Self::sales());
        all.extend(Self::security());
        all.extend(Self::spatial());
        all.extend(Self::specialized());
        all.extend(Self::strategy());
        all.extend(Self::support());
        all.extend(Self::testing());
        all
    }
}