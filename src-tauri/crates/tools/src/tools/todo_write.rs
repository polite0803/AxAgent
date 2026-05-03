//! TodoWriteTool / AskUserQuestionTool / NotebookEditTool
//! 组合几个小工具到一个文件

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

// ============================================================
// TodoWriteTool
// ============================================================

pub struct TodoWriteTool;

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "TodoWrite"
    }
    fn description(&self) -> &str {
        "创建和管理结构化任务列表。用于跟踪多步骤工作进度。每个任务有状态：pending, in_progress, completed。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string", "description": "任务描述" },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "任务状态"
                            },
                            "activeForm": {
                                "type": "string",
                                "description": "进行中的任务描述（用于 UI 显示）"
                            }
                        },
                        "required": ["content", "status", "activeForm"]
                    },
                    "description": "任务列表"
                }
            },
            "required": ["todos"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let todos = input["todos"]
            .as_array()
            .ok_or_else(|| ToolError::invalid_input_for("TodoWrite", "todos 必须是数组"))?;

        let mut output = String::from("## Todo 列表\n\n");
        let mut counts = [0, 0, 0]; // pending, in_progress, completed

        for todo in todos {
            let content = todo["content"].as_str().unwrap_or("");
            let status = todo["status"].as_str().unwrap_or("pending");

            let icon = match status {
                "completed" => {
                    counts[2] += 1;
                    "✅"
                },
                "in_progress" => {
                    counts[1] += 1;
                    "🔄"
                },
                _ => {
                    counts[0] += 1;
                    "⏳"
                },
            };

            output.push_str(&format!("- {} [{}] {}\n", icon, status, content));
        }

        output.push_str(&format!(
            "\n---\n总计: {} pending | {} in_progress | {} completed",
            counts[0], counts[1], counts[2]
        ));

        Ok(ToolResult::success(output))
    }
}

// ============================================================
// AskUserQuestionTool
// ============================================================

pub struct AskUserQuestionTool;

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "AskUserQuestion"
    }
    fn description(&self) -> &str {
        "向用户提问以获取决策信息。支持单选和多选。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": { "type": "string", "description": "完整问题" },
                            "header": { "type": "string", "description": "简短标签" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multiSelect": { "type": "boolean", "default": false }
                        },
                        "required": ["question", "header", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let questions = input["questions"].as_array().ok_or_else(|| {
            ToolError::invalid_input_for("AskUserQuestion", "questions 必须是数组")
        })?;

        let mut output = String::from("## 用户提问\n\n");
        for (i, q) in questions.iter().enumerate() {
            let question = q["question"].as_str().unwrap_or("");
            let _header = q["header"].as_str().unwrap_or("");
            let multi = q["multiSelect"].as_bool().unwrap_or(false);
            let options = q["options"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|o| {
                            o["label"]
                                .as_str()
                                .map(|l| (l, o["description"].as_str().unwrap_or("")))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            output.push_str(&format!(
                "**Q{}**: {} {}\n",
                i + 1,
                if multi { "(多选)" } else { "" },
                question
            ));
            for (label, desc) in &options {
                output.push_str(&format!("  - `{}` — {}\n", label, desc));
            }
            output.push('\n');
        }

        output.push_str("[等待用户回复...]");
        Ok(ToolResult::success(output))
    }
}

// ============================================================
// NotebookEditTool
// ============================================================

pub struct NotebookEditTool;

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }
    fn description(&self) -> &str {
        "编辑 Jupyter Notebook (.ipynb) 文件。支持替换、插入、删除单元格。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "notebook_path": {
                    "type": "string",
                    "description": "Notebook 文件的绝对路径"
                },
                "cell_id": {
                    "type": "string",
                    "description": "要编辑的单元格 ID（插入时为插入位置之前的单元格 ID）"
                },
                "new_source": {
                    "type": "string",
                    "description": "新的单元格源码"
                },
                "cell_type": {
                    "type": "string",
                    "enum": ["code", "markdown"],
                    "description": "单元格类型"
                },
                "edit_mode": {
                    "type": "string",
                    "enum": ["replace", "insert", "delete"],
                    "description": "编辑模式",
                    "default": "replace"
                }
            },
            "required": ["notebook_path", "new_source"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input["notebook_path"].as_str().unwrap();
        let new_source = input["new_source"].as_str().unwrap();
        let edit_mode = input["edit_mode"].as_str().unwrap_or("replace");
        let cell_type = input["cell_type"].as_str().unwrap_or("code");

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::execution_failed(format!("读取 Notebook 失败: {}", e)))?;

        let mut nb: Value = serde_json::from_str(&content)
            .map_err(|e| ToolError::execution_failed(format!("JSON 解析失败: {}", e)))?;

        let cells = nb["cells"].as_array_mut().ok_or_else(|| {
            ToolError::execution_failed_for("NotebookEdit", "无效的 Notebook 格式")
        })?;

        match edit_mode {
            "replace" => {
                let cid = input["cell_id"].as_str();
                if let Some(cell) = cells
                    .iter_mut()
                    .find(|c| cid.is_some_and(|id| c["id"].as_str() == Some(id)))
                {
                    cell["source"] = Value::Array(vec![Value::String(new_source.to_string())]);
                    cell["cell_type"] = Value::String(cell_type.to_string());
                } else {
                    // 未找到指定 cell，追加
                    cells.push(make_cell(new_source, cell_type));
                }
            },
            "insert" => {
                let new_cell = make_cell(new_source, cell_type);
                if let Some(cid) = input["cell_id"].as_str() {
                    if let Some(pos) = cells.iter().position(|c| c["id"].as_str() == Some(cid)) {
                        cells.insert(pos + 1, new_cell);
                    } else {
                        cells.push(new_cell);
                    }
                } else {
                    cells.insert(0, new_cell);
                }
            },
            "delete" => {
                if let Some(cid) = input["cell_id"].as_str() {
                    cells.retain(|c| c["id"].as_str() != Some(cid));
                }
            },
            _ => {
                return Err(ToolError::invalid_input(format!(
                    "未知编辑模式: {}",
                    edit_mode
                )))
            },
        }

        let new_content = serde_json::to_string_pretty(&nb)
            .map_err(|e| ToolError::execution_failed(format!("序列化失败: {}", e)))?;

        std::fs::write(path, &new_content)
            .map_err(|e| ToolError::execution_failed(format!("写入失败: {}", e)))?;

        Ok(ToolResult::success(format!(
            "✅ 已{} Notebook 单元格: {}",
            match edit_mode {
                "insert" => "插入",
                "delete" => "删除",
                _ => "替换",
            },
            path
        )))
    }
}

fn make_cell(source: &str, cell_type: &str) -> Value {
    use uuid::Uuid;
    serde_json::json!({
        "id": Uuid::new_v4().to_string(),
        "cell_type": cell_type,
        "metadata": {},
        "source": [source],
        "outputs": [],
        "execution_count": null
    })
}
