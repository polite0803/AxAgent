use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPromptTemplate {
    pub system: String,
    pub user_template: String,
    pub expected_schema: serde_json::Value,
}

pub struct PromptTemplates;

const ATOMIC_SKILL_SCHEMA: &str = r#"{
  "name": "string (必填) - 技能唯一名称，格式: snake_case",
  "description": "string (必填) - 技能功能描述",
  "category": "string (必填) - 分类: data_processing, web_scraping, file_operation, api_integration, text_processing, automation, monitoring, integration, other",
  "input_schema": "object (可选) - JSON Schema 定义输入参数",
  "output_schema": "object (可选) - JSON Schema 定义输出",
  "entry_type": "string (必填) - 类型: builtin|mcp|local|plugin",
  "entry_ref": "string (必填) - 引用标识符",
  "code_content": "string (可选) - 代码内容",
  "dependencies": "array<string> - 依赖的其他技能名称",
  "tags": "array<string> - 标签",
  "version": "string (默认1.0.0)",
  "enabled": "boolean (默认true)",
  "source": "string (默认auto-generated)",
  "metadata": {
    "source_file": "string - 源文件路径",
    "function_name": "string - 对应函数名",
    "line_range": "string - 行号范围",
    "confidence": "number - 置信度0-1"
  }
}"#;

const WORKFLOW_NODE_TYPES: &str = r#"节点类型 (node.type):
- trigger: 触发器节点，config: { type: "manual"|"schedule"|"webhook"|"event", config: {} }
- agent: Agent节点，config: { role: "researcher"|"planner"|"developer"|"reviewer"|"synthesizer"|"executor", system_prompt, context_sources, output_var, tools, output_mode }
- llm: LLM节点，config: { model, prompt, temperature, max_tokens, tools, functions }
- condition: 条件分支，config: { conditions: [{var_path, operator, value}], logical_op: "and"|"or" }
- parallel: 并行分支，config: { branches: [{id, title, steps}], wait_for_all, timeout }
- loop: 循环，config: { loop_type: "forEach"|"while"|"doWhile"|"until", items_var, iteratee_var, max_iterations, continue_on_error, body_steps }
- merge: 合并节点，config: { merge_type, inputs }
- delay: 延迟，config: { delay_type, seconds, until }
- tool: 工具节点，config: { tool_name, input_mapping, output_var }
- code: 代码节点，config: { language, code, output_var }
- atomicSkill: 工作流步骤节点，config: { skill_id, skill_name, entry_type, input_mapping, output_var }
- end: 结束节点，config: { output_var }"#;

const WORKFLOW_EDGE_TYPES: &str = r#"边类型 (edge.edge_type):
- direct: 直接边
- conditionTrue: 条件为真
- conditionFalse: 条件为假
- loopBack: 循环回边
- parallelBranch: 并行分支
- merge: 合并边
- error: 错误边"#;

const TOOL_ENTRY_TYPES: &str = r#"工具入口类型 (entry_type):
- builtin: 内置工具
- mcp: MCP协议工具
- local: 本地工具
- plugin: 插件工具"#;

impl PromptTemplates {
    pub fn turn1_understand() -> LlmPromptTemplate {
        LlmPromptTemplate {
            system: r#"你是一个技能分解专家，负责将复合技能包拆分为工作流步骤和工作流。

输入格式：
用户将提供一个复合技能包，包含多个文件（Markdown、Python、JavaScript、TypeScript、Shell等）

输出要求：
1. 理解文件结构和依赖关系
2. 识别主要入口文件和功能模块
3. 用中文回复

重要规则：
1. 不要自行生成内容，只分析和确认
2. 如果文件结构不清晰，说明需要用户澄清
3. 返回结构化的文件结构分析"#
                .to_string(),
            user_template: r#"请分析以下文件列表，理解复合技能的结构：

文件列表：
{{file_list}}

文件内容摘要：
{{file_summaries}}

请分析：
1. 主要入口文件是哪个？
2. 支持文件有哪些，作用是什么？
3. 文件间的依赖关系是什么？
4. 技能的总体功能是什么？

用中文回复。"#
                .to_string(),
            expected_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "main_file": { "type": "string", "description": "主要入口文件路径" },
                    "supporting_files": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "支持文件路径列表"
                    },
                    "relationships": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string" },
                                "to": { "type": "string" },
                                "type": { "type": "string", "enum": ["import", "calls", "includes", "references"] }
                            }
                        }
                    },
                    "overall_purpose": { "type": "string", "description": "技能的总体功能描述" },
                    "language_detected": { "type": "string", "description": "主要编程语言" },
                    "clarifications_needed": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "需要用户澄清的问题"
                    }
                }
            }),
        }
    }

    pub fn turn2_classify() -> LlmPromptTemplate {
        LlmPromptTemplate {
            system: r#"你是一个代码分析专家，负责分析复合技能中的代码内容。

分类维度：
- metadata: 元信息（名称、描述、版本）
- instruction: 文本指令（步骤说明）
- script: 可执行代码（Python、JS、TS等）
- config: 配置数据（JSON、YAML等）
- schema: 数据结构定义
- utility: 工具函数

对于每个代码文件，需要识别：
1. 函数列表（名称、参数、返回值）
2. 导入的模块/依赖
3. 核心功能逻辑

用中文回复。"#
                .to_string(),
            user_template: r#"分析以下文件的内容类型：

已确认的文件结构：
{{file_structure}}

文件内容：
{{file_contents}}

对于每个文件：
1. 识别文件类型（metadata/instruction/script/config/schema/utility）
2. 如果是代码文件，列出所有函数
3. 说明每个函数的功能

用中文回复。"#
                .to_string(),
            expected_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_classifications": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string" },
                                "primary_type": { "type": "string", "enum": ["metadata", "instruction", "script", "config", "schema", "utility"] },
                                "secondary_types": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "functions": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "line_range": { "type": "string" },
                                            "params": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            },
                                            "return_type": { "type": "string" },
                                            "purpose": { "type": "string" },
                                            "is_entry_point": { "type": "boolean" }
                                        }
                                    }
                                },
                                "imports": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        }
                    },
                    "cross_file_references": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "source_file": { "type": "string" },
                                "source_function": { "type": "string" },
                                "target_file": { "type": "string" },
                                "target_function": { "type": "string" },
                                "reference_type": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        }
    }

    pub fn turn3_analyze() -> LlmPromptTemplate {
        LlmPromptTemplate {
            system: r#"你是一个功能分析专家，负责分析每个函数的功能边界和依赖关系。

功能类型分类：
- validator: 数据/参数验证
- processor: 业务逻辑处理
- transformer: 数据格式转换
- persister: 数据持久化
- notifier: 通知/输出
- initializer: 初始化/配置
- fetcher: 数据获取
- parser: 解析/分析

分析要求：
1. 理解每个函数实现的功能
2. 识别输入输出（参数和返回值）
3. 判断是否可以独立成工作流步骤
4. 识别函数间的依赖关系

用中文回复。"#
                .to_string(),
            user_template: r#"分析以下代码的功能：

语言：{{language}}

上下文：{{context}}

代码内容：
```{language}}
{{code_content}}
```

对于每个函数，分析：
1. 功能类型（validator/processor/transformer/persister/notifier/initializer/fetcher/parser）
2. 输入参数和类型
3. 输出/返回值
4. 依赖的外部函数或模块
5. 是否可独立成工作流步骤
6. 如果可以，建议的技能名称

用中文回复。"#
                .to_string(),
            expected_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "functions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "function_type": {
                                    "type": "string",
                                    "enum": ["validator", "processor", "transformer", "persister", "notifier", "initializer", "fetcher", "parser", "other"]
                                },
                                "description": { "type": "string" },
                                "input_schema": {
                                    "type": "object",
                                    "description": "JSON Schema 格式的输入参数定义",
                                    "properties": {
                                        "type": { "type": "string" },
                                        "properties": { "type": "object" },
                                        "required": { "type": "array", "items": { "type": "string" } }
                                    }
                                },
                                "output_schema": {
                                    "type": "object",
                                    "description": "JSON Schema 格式的输出定义"
                                },
                                "dependencies": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "依赖的函数名列表"
                                },
                                "can_be_independent": { "type": "boolean" },
                                "suggested_skill_name": { "type": "string" },
                                "suggested_category": {
                                    "type": "string",
                                    "enum": ["data_processing", "web_scraping", "file_operation", "api_integration", "text_processing", "automation", "monitoring", "integration", "other"]
                                },
                                "entry_type_suggestion": {
                                    "type": "string",
                                    "enum": ["builtin", "mcp", "local", "plugin"],
                                    "description": "建议的工具入口类型"
                                }
                            }
                        }
                    },
                    "function_dependency_graph": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string" },
                                "to": { "type": "string" },
                                "dependency_type": { "type": "string", "enum": ["calls", "imports", "returns", "raises"] }
                            }
                        }
                    }
                }
            }),
        }
    }

    pub fn turn4_design() -> LlmPromptTemplate {
        LlmPromptTemplate {
            system: format!(
                r#"你是一个工作流设计专家，基于功能分析设计工作流。

工作流设计原则：
1. 工作流必须是有向无环图（DAG）
2. 每个节点代表一个工作流步骤或控制结构
3. 边代表数据流或控制流
4. 必须有明确的开始（trigger）和结束（end）节点

节点类型：
{WORKFLOW_NODE_TYPES}

边类型：
{WORKFLOW_EDGE_TYPES}

设计步骤：
1. 确定触发方式（manual/schedule/webhook/event）
2. 按照依赖关系排序功能
3. 识别可并行的分支
4. 识别需要条件判断的地方
5. 定义节点间的数据映射

用中文回复。"#
            ),
            user_template: r#"基于以下分析结果设计工作流：

已分析的函数及其依赖关系：
{{function_analysis}}

函数依赖图：
{{dependency_graph}}

数据流信息：
{{data_flows}}

请设计工作流：
1. 确定触发类型
2. 按执行顺序列出节点
3. 确定节点间的连接和数据映射
4. 识别需要条件判断或并行处理的地方

用中文回复。"#
                .to_string(),
            expected_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "workflow_name": { "type": "string" },
                    "workflow_description": { "type": "string" },
                    "trigger_type": {
                        "type": "string",
                        "enum": ["manual", "schedule", "webhook", "event"],
                        "description": "触发类型"
                    },
                    "nodes": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "type": {
                                    "type": "string",
                                    "enum": ["trigger", "agent", "llm", "condition", "parallel", "loop", "merge", "delay", "tool", "code", "atomicSkill", "end"]
                                },
                                "title": { "type": "string" },
                                "config": { "type": "object" },
                                "skill_ref": { "type": "string", "description": "引用的工作流步骤名称" },
                                "position": {
                                    "type": "object",
                                    "properties": {
                                        "x": { "type": "number" },
                                        "y": { "type": "number" }
                                    }
                                }
                            }
                        }
                    },
                    "edges": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "source": { "type": "string" },
                                "target": { "type": "string" },
                                "sourceHandle": { "type": "string" },
                                "targetHandle": { "type": "string" },
                                "edge_type": {
                                    "type": "string",
                                    "enum": ["direct", "conditionTrue", "conditionFalse", "loopBack", "parallelBranch", "merge", "error"]
                                },
                                "label": { "type": "string" },
                                "data_mapping": {
                                    "type": "object",
                                    "additionalProperties": { "type": "string" },
                                    "description": "变量映射关系"
                                }
                            }
                        }
                    },
                    "variables": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "var_type": { "type": "string" },
                                "default_value": { "type": "string" },
                                "description": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        }
    }

    pub fn turn5_generate() -> LlmPromptTemplate {
        LlmPromptTemplate {
            system: format!(
                r#"你是一个技能生成专家，生成最终的分解结果。

输出必须严格遵循以下格式：

工作流步骤格式（Agent）：
{ATOMIC_SKILL_SCHEMA}

工作流节点类型：
{WORKFLOW_NODE_TYPES}

工作流边类型：
{WORKFLOW_EDGE_TYPES}

工具入口类型：
{TOOL_ENTRY_TYPES}

重要要求：
1. 所有字段必须符合上述格式要求
2. 工作流步骤的 name 必须是有效的标识符（snake_case）
3. 工作流必须是有效的 DAG
4. 每个工作流步骤必须有 metadata 说明来源
5. 使用 JSON 格式输出

输出结构：
{{
  "agent_steps": [/* Agent数组 */],
  "tool_dependencies": [/* 工具依赖数组 */],
  "workflow": {{ /* 工作流定义 */ }}
}}"#
            ),
            user_template: r#"基于以下分析结果，生成最终 JSON 输出：

工作流设计：
{{workflow_design}}

函数分析结果：
{{function_analysis}}

请生成完整的 JSON 输出，严格遵循格式要求：
1. workflow_steps: 工作流步骤列表，每个包含完整的字段
2. tool_dependencies: 工具依赖列表
3. workflow: 工作流定义，包含 nodes 和 edges

注意：
- 每个 agent_step 必须有唯一的 name
- metadata 必须说明来源文件、函数名、行号
- workflow.nodes 中的 atomicSkill 节点必须通过 skill_ref 或 config.skill_name 引用对应的工作流步骤"#
                .to_string(),
            expected_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "description", "category", "entry_type", "entry_ref", "metadata"],
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "技能唯一名称，snake_case格式"
                                },
                                "description": { "type": "string" },
                                "category": {
                                    "type": "string",
                                    "enum": ["data_processing", "web_scraping", "file_operation", "api_integration", "text_processing", "automation", "monitoring", "integration", "other"]
                                },
                                "input_schema": {
                                    "type": "object",
                                    "description": "JSON Schema格式，定义输入参数"
                                },
                                "output_schema": {
                                    "type": "object",
                                    "description": "JSON Schema格式，定义输出"
                                },
                                "entry_type": {
                                    "type": "string",
                                    "enum": ["builtin", "mcp", "local", "plugin"]
                                },
                                "entry_ref": {
                                    "type": "string",
                                    "description": "引用标识符"
                                },
                                "code_content": {
                                    "type": "string",
                                    "description": "代码内容（可选）"
                                },
                                "dependencies": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "依赖的技能名称"
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "version": { "type": "string", "default": "1.0.0" },
                                "enabled": { "type": "boolean", "default": true },
                                "source": { "type": "string", "default": "auto-generated" },
                                "metadata": {
                                    "type": "object",
                                    "properties": {
                                        "source_file": { "type": "string" },
                                        "function_name": { "type": "string" },
                                        "line_range": { "type": "string" },
                                        "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
                                    }
                                }
                            }
                        }
                    },
                    "tool_dependencies": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "tool_type"],
                            "properties": {
                                "name": { "type": "string" },
                                "tool_type": {
                                    "type": "string",
                                    "enum": ["builtin", "mcp", "local", "plugin"]
                                },
                                "source_info": { "type": "string" },
                                "required": { "type": "boolean" },
                                "status": {
                                    "type": "string",
                                    "enum": ["satisfied", "auto_installable", "manual_installable", "needs_generation"]
                                }
                            }
                        }
                    },
                    "workflow": {
                        "type": "object",
                        "required": ["nodes", "edges"],
                        "properties": {
                            "nodes": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["id", "type", "title", "position"],
                                    "properties": {
                                        "id": { "type": "string" },
                                        "type": {
                                            "type": "string",
                                            "enum": ["trigger", "agent", "llm", "condition", "parallel", "loop", "merge", "delay", "tool", "code", "atomicSkill", "end"]
                                        },
                                        "title": { "type": "string" },
                                        "description": { "type": "string" },
                                        "position": {
                                            "type": "object",
                                            "properties": {
                                                "x": { "type": "number" },
                                                "y": { "type": "number" }
                                            }
                                        },
                                        "config": { "type": "object" },
                                        "retry": {
                                            "type": "object",
                                            "properties": {
                                                "enabled": { "type": "boolean" },
                                                "max_retries": { "type": "number" },
                                                "backoff_type": { "type": "string" },
                                                "base_delay_ms": { "type": "number" },
                                                "max_delay_ms": { "type": "number" }
                                            }
                                        },
                                        "timeout": { "type": "number" },
                                        "enabled": { "type": "boolean" }
                                    }
                                }
                            },
                            "edges": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["id", "source", "target", "edge_type"],
                                    "properties": {
                                        "id": { "type": "string" },
                                        "source": { "type": "string" },
                                        "sourceHandle": { "type": "string" },
                                        "target": { "type": "string" },
                                        "targetHandle": { "type": "string" },
                                        "edge_type": {
                                            "type": "string",
                                            "enum": ["direct", "conditionTrue", "conditionFalse", "loopBack", "parallelBranch", "merge", "error"]
                                        },
                                        "label": { "type": "string" },
                                        "data_mapping": {
                                            "type": "object",
                                            "additionalProperties": { "type": "string" }
                                        }
                                    }
                                }
                            },
                            "variables": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "var_type": { "type": "string" },
                                        "value": { "type": "string" },
                                        "description": { "type": "string" },
                                        "is_secret": { "type": "boolean" }
                                    }
                                }
                            }
                        }
                    }
                },
                "required": ["agent_steps", "tool_dependencies", "workflow"]
            }),
        }
    }

    pub fn get_all_turns() -> Vec<LlmPromptTemplate> {
        vec![
            Self::turn1_understand(),
            Self::turn2_classify(),
            Self::turn3_analyze(),
            Self::turn4_design(),
            Self::turn5_generate(),
        ]
    }

    pub fn format_turn1_user_content(file_list: &str, file_summaries: &str) -> String {
        Self::turn1_understand()
            .user_template
            .replace("{{file_list}}", file_list)
            .replace("{{file_summaries}}", file_summaries)
    }

    pub fn format_turn2_user_content(file_structure: &str, file_contents: &str) -> String {
        Self::turn2_classify()
            .user_template
            .replace("{{file_structure}}", file_structure)
            .replace("{{file_contents}}", file_contents)
    }

    pub fn format_turn3_user_content(context: &str, language: &str, code_content: &str) -> String {
        Self::turn3_analyze()
            .user_template
            .replace("{{context}}", context)
            .replace("{{language}}", language)
            .replace("{{code_content}}", code_content)
    }

    pub fn format_turn4_user_content(
        function_analysis: &str,
        dependency_graph: &str,
        data_flows: &str,
    ) -> String {
        Self::turn4_design()
            .user_template
            .replace("{{function_analysis}}", function_analysis)
            .replace("{{dependency_graph}}", dependency_graph)
            .replace("{{data_flows}}", data_flows)
    }

    pub fn format_turn5_user_content(workflow_design: &str, function_analysis: &str) -> String {
        Self::turn5_generate()
            .user_template
            .replace("{{workflow_design}}", workflow_design)
            .replace("{{function_analysis}}", function_analysis)
    }
}
