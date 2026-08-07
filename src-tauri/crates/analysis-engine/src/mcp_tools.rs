// SPDX-License-Identifier: AGPL-3.0-only

//! G3 产业链传导映射 — MCP 工具集
//!
//! 本模块提供两个 MCP 工具（与 `axagent-astock-data::mcp_tools` 解耦）：
//! - `get_industry_chain_propagation`：按链 ID + 起始节点传导影响
//! - `map_news_to_cross_market_stocks`：新闻文本 → 跨市场股票映射
//!
//! ## 架构归属
//!
//! 原位于 `axagent-astock-data::mcp_tools`，于 P2-8 阶段随 `industry_chain.rs`
//! 一并迁回 `axagent-stock-analysis`。`astock-data` 仅保留数据获取类 MCP 工具。

use serde_json::{json, Value};

use crate::industry_chain::{
    get_industry_chain, map_news_to_chain, propagate_impact, ImpactDirection,
};

/// G3 产业链相关 MCP 工具名集合
pub const INDUSTRY_CHAIN_TOOL_NAMES: &[&str] =
    &["get_industry_chain_propagation", "map_news_to_cross_market_stocks"];

/// 判断工具名是否属于 G3 产业链工具集
pub fn is_industry_chain_tool(tool_name: &str) -> bool {
    INDUSTRY_CHAIN_TOOL_NAMES.contains(&tool_name)
}

/// 返回 G3 产业链相关 MCP 工具的 JSON Schema 定义列表
pub fn industry_chain_mcp_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "get_industry_chain_propagation",
            "description": "G3 产业链传导映射：根据产业链 ID（ai_compute/semiconductor/optical_module/nev/consumer_electronics）和起始节点 ID，沿产业链 BFS 传导影响，返回所有受影响节点（含 A 股/美股/港股代码、累积强度、传导时滞、路径）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chain_id": {
                        "type": "string",
                        "description": "产业链 ID：ai_compute / semiconductor / optical_module / nev / consumer_electronics"
                    },
                    "start_node_id": {
                        "type": "string",
                        "description": "起始节点 ID（如 gpu / lithium_mining / panel 等，见 chain.nodes[].id）"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["positive", "negative", "neutral"],
                        "description": "影响方向（默认 neutral）"
                    },
                    "min_strength": {
                        "type": "number",
                        "description": "最小累积强度阈值（默认 0.1，低于此值停止该分支传导）"
                    }
                },
                "required": ["chain_id", "start_node_id"]
            }
        }),
        json!({
            "name": "map_news_to_cross_market_stocks",
            "description": "G3 新闻 → 跨市场股票映射：输入新闻文本，关键词命中规则自动识别影响产业链，输出命中的链、激活节点、综合方向、强度，以及沿链传导后的所有 A 股/美股/港股代码列表",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "news_text": {
                        "type": "string",
                        "description": "新闻正文（中文，≥10 字符）"
                    }
                },
                "required": ["news_text"]
            }
        }),
    ]
}

/// 执行 G3 产业链相关 MCP 工具
///
/// 返回 JSON 字符串（与 `astock-data::mcp_tools::execute_mcp_tool` 保持一致的接口契约）
pub fn execute_industry_chain_tool(tool_name: &str, arguments: &Value) -> Result<String, String> {
    match tool_name {
        "get_industry_chain_propagation" => {
            let chain_id = arguments["chain_id"]
                .as_str()
                .ok_or_else(|| "chain_id 参数缺失".to_string())?
                .to_string();
            let start_node_id = arguments["start_node_id"]
                .as_str()
                .ok_or_else(|| "start_node_id 参数缺失".to_string())?
                .to_string();
            let direction_str = arguments["direction"].as_str().unwrap_or("neutral");
            let direction = match direction_str {
                "positive" => ImpactDirection::Positive,
                "negative" => ImpactDirection::Negative,
                _ => ImpactDirection::Neutral,
            };
            let min_strength = arguments["min_strength"].as_f64().unwrap_or(0.1);
            let chain = get_industry_chain(&chain_id)
                .ok_or_else(|| format!("未知产业链 ID: {chain_id}"))?;
            let result = propagate_impact(&chain, &start_node_id, direction, min_strength);
            serde_json::to_string(&result).map_err(|e| e.to_string())
        },
        "map_news_to_cross_market_stocks" => {
            let news_text =
                arguments["news_text"].as_str().ok_or_else(|| "news_text 参数缺失".to_string())?;
            if news_text.len() < 10 {
                return Err("news_text 长度不足（需 ≥10 字符）".to_string());
            }
            let hits = map_news_to_chain(news_text);
            serde_json::to_string(&hits).map_err(|e| e.to_string())
        },
        _ => Err(format!("Unknown industry chain tool: {tool_name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_industry_chain_tool() {
        assert!(is_industry_chain_tool("get_industry_chain_propagation"));
        assert!(is_industry_chain_tool("map_news_to_cross_market_stocks"));
        assert!(!is_industry_chain_tool("get_stock_kline"));
    }

    #[test]
    fn test_mcp_tools_count() {
        let tools = industry_chain_mcp_tools();
        assert_eq!(tools.len(), 2);
        let names: Vec<_> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"get_industry_chain_propagation"));
        assert!(names.contains(&"map_news_to_cross_market_stocks"));
    }

    #[test]
    fn test_execute_get_industry_chain_propagation() {
        let args = json!({
            "chain_id": "ai_compute",
            "start_node_id": "gpu",
            "direction": "positive"
        });
        let result = execute_industry_chain_tool("get_industry_chain_propagation", &args);
        assert!(result.is_ok());
        let json_str = result.unwrap();
        assert!(json_str.contains("gpu"));
        assert!(json_str.contains("optical_module"));
    }

    #[test]
    fn test_execute_map_news_to_cross_market_stocks() {
        let args = json!({
            "news_text": "英伟达发布新一代 GPU，光模块需求大增"
        });
        let result = execute_industry_chain_tool("map_news_to_cross_market_stocks", &args);
        assert!(result.is_ok());
        let json_str = result.unwrap();
        assert!(json_str.contains("ai_compute"));
    }

    #[test]
    fn test_execute_unknown_tool() {
        let args = json!({});
        let result = execute_industry_chain_tool("unknown_tool", &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_short_news_text() {
        let args = json!({
            "news_text": "短"
        });
        let result = execute_industry_chain_tool("map_news_to_cross_market_stocks", &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("长度不足"));
    }
}
