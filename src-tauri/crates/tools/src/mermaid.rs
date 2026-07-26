// SPDX-License-Identifier-Identifier: AGPL-3.0-only

//! Mermaid 流程图子集解析与渲染
//!
//! 完整 Mermaid 语法超出范围。本模块实现 flowchart 子集：
//!
//! - 方向声明：`graph TD` / `graph LR` / `flowchart TD` / `flowchart LR`
//! - 节点形状：
//!   - `A[文本]` — 矩形
//!   - `A(文本)` — 圆角矩形
//!   - `A{文本}` — 菱形（决策）
//!   - `A((文本))` — 圆形
//!   - `A` — 纯 ID（文本=ID）
//! - 边类型：
//!   - `A --> B` — 实线箭头
//!   - `A --- B` — 实线无箭头
//!   - `A -.-> B` — 虚线箭头
//!   - `A -->|标签| B` — 带标签箭头
//!   - `A -- 标签 --> B` — 带标签箭头（另一种语法）
//!
//! 渲染输出为 Unicode 框线字符文本，供 PDF/DOCX/PPTX 导出端使用。

/// Mermaid 节点形状
#[derive(Debug, Clone, PartialEq)]
pub enum NodeShape {
    /// 矩形 `[文本]`
    Rectangle,
    /// 圆角矩形 `(文本)`
    Rounded,
    /// 菱形 `{文本}`（决策）
    Diamond,
    /// 圆形 `((文本))`
    Circle,
    /// 无形状（纯 ID）
    Plain,
}

/// Mermaid 节点
#[derive(Debug, Clone)]
pub struct MermaidNode {
    pub id: String,
    pub text: String,
    pub shape: NodeShape,
}

/// Mermaid 边类型
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeStyle {
    /// 实线箭头 `-->`
    Arrow,
    /// 实线无箭头 `---`
    Line,
    /// 虚线箭头 `-.->`
    Dashed,
    /// 粗线箭头 `==>`
    Thick,
}

/// Mermaid 边
#[derive(Debug, Clone)]
pub struct MermaidEdge {
    pub from: String,
    pub to: String,
    pub style: EdgeStyle,
    pub label: Option<String>,
}

/// 解析后的 Mermaid 流程图
#[derive(Debug, Clone)]
pub struct MermaidGraph {
    /// 方向：true=TD（自上而下），false=LR（从左到右）
    pub top_down: bool,
    pub nodes: Vec<MermaidNode>,
    pub edges: Vec<MermaidEdge>,
}

impl MermaidGraph {
    pub fn empty() -> Self {
        Self { top_down: true, nodes: Vec::new(), edges: Vec::new() }
    }
}

/// 解析 Mermaid flowchart 文本
///
/// 示例：
/// ```text
/// graph TD
///     A[开始] --> B{是否成功?}
///     B -->|是| C[处理结果]
///     B -->|否| D[记录错误]
/// ```
pub fn parse_mermaid(text: &str) -> MermaidGraph {
    let mut graph = MermaidGraph::empty();
    let mut node_map: std::collections::HashMap<String, MermaidNode> =
        std::collections::HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // 方向声明
        if line.starts_with("graph ") || line.starts_with("flowchart ") {
            let dir = line.split_whitespace().nth(1).unwrap_or("TD");
            graph.top_down = matches!(dir, "TD" | "TB");
            continue;
        }

        // 解析边或节点定义
        if let Some((edge, nodes)) = parse_edge_with_nodes(line) {
            // 注册边中声明的节点形状
            for node in nodes {
                node_map.insert(node.id.clone(), node);
            }
            // 确保端点节点存在
            ensure_node(&mut node_map, &edge.from);
            ensure_node(&mut node_map, &edge.to);
            graph.edges.push(edge);
        } else if let Some(node) = parse_node(line) {
            node_map.insert(node.id.clone(), node);
        }
    }

    graph.nodes = node_map.into_values().collect();
    graph
}

/// 解析边定义，同时返回边中声明的节点形状信息
fn parse_edge_with_nodes(line: &str) -> Option<(MermaidEdge, Vec<MermaidNode>)> {
    // 按优先级尝试各种边类型
    // 虚线箭头 -.->  或  -. 标签 .->
    if let Some(result) = try_parse_edge_variant_labeled(line, "-.->", EdgeStyle::Dashed) {
        return Some(result);
    }
    if line.contains("-.")
        && line.contains(".->")
        && let Some(result) = parse_labeled_variant_with_nodes(line, "-.", ".->", EdgeStyle::Dashed)
    {
        return Some(result);
    }
    // 粗线箭头 ==>  或  == 标签 ==>
    if let Some(result) = try_parse_edge_variant_labeled(line, "==>", EdgeStyle::Thick) {
        return Some(result);
    }
    if line.contains("==")
        && line.contains("==>")
        && let Some(result) = parse_labeled_variant_with_nodes(line, "==", "==>", EdgeStyle::Thick)
    {
        return Some(result);
    }
    // 实线箭头 -->  或  -->|标签|  或  -- 标签 -->
    if let Some(result) = try_parse_edge_variant_labeled(line, "-->", EdgeStyle::Arrow) {
        return Some(result);
    }
    if line.contains("--")
        && line.contains("-->")
        && let Some(result) = parse_arrow_labeled_with_nodes(line)
    {
        return Some(result);
    }
    // 实线无箭头 ---
    if let Some(result) = try_parse_edge_variant_labeled(line, "---", EdgeStyle::Line) {
        return Some(result);
    }
    None
}

/// 尝试匹配指定操作符的边（支持 `-->|标签|` 语法）
fn try_parse_edge_variant_labeled(
    line: &str,
    op: &str,
    style: EdgeStyle,
) -> Option<(MermaidEdge, Vec<MermaidNode>)> {
    let pos = line.find(op)?;
    let from_part = &line[..pos];
    let to_part = &line[pos + op.len()..];

    // 检查 to_part 是否含 |标签| 语法
    let (to_str, label) = if let Some(label_start) = to_part.find('|') {
        if let Some(label_end) = to_part[label_start + 1..].find('|') {
            let label = to_part[label_start + 1..label_start + 1 + label_end].trim();
            let to = to_part[label_start + 1 + label_end + 1..].trim();
            (
                to,
                if label.is_empty() {
                    None
                } else {
                    Some(label.to_string())
                },
            )
        } else {
            (to_part.trim(), None)
        }
    } else {
        (to_part.trim(), None)
    };

    if from_part.trim().is_empty() || to_str.is_empty() {
        return None;
    }

    // 解析端点的形状定义
    let (from_id, from_node) = parse_endpoint(from_part.trim());
    let (to_id, to_node) = parse_endpoint(to_str);

    if from_id.is_empty() || to_id.is_empty() {
        return None;
    }

    let mut nodes = Vec::new();
    if let Some(n) = from_node {
        nodes.push(n);
    }
    if let Some(n) = to_node {
        nodes.push(n);
    }

    Some((MermaidEdge { from: from_id, to: to_id, style, label }, nodes))
}

/// 解析端点（可能含形状定义），返回 (ID, Option<节点定义>)
fn parse_endpoint(s: &str) -> (String, Option<MermaidNode>) {
    let s = s.trim();
    // 尝试各种形状
    if let Some((id, text)) = extract_bracketed(s, '[', ']') {
        return (id.clone(), Some(MermaidNode { id, text, shape: NodeShape::Rectangle }));
    }
    if let Some((id, text)) = extract_bracketed(s, '{', '}') {
        return (id.clone(), Some(MermaidNode { id, text, shape: NodeShape::Diamond }));
    }
    if s.contains("((")
        && s.contains("))")
        && let Some(start) = s.find("((")
        && let Some(end) = s[start + 2..].find("))")
    {
        let text = s[start + 2..start + 2 + end].trim().to_string();
        let id = s[..start].trim().to_string();
        if !id.is_empty() {
            return (id.clone(), Some(MermaidNode { id, text, shape: NodeShape::Circle }));
        }
    }
    if let Some((id, text)) = extract_bracketed(s, '(', ')') {
        return (id.clone(), Some(MermaidNode { id, text, shape: NodeShape::Rounded }));
    }
    // 纯 ID
    (extract_id(s), None)
}

/// 解析 `A -- 标签 --> B` 形式
fn parse_arrow_labeled_with_nodes(line: &str) -> Option<(MermaidEdge, Vec<MermaidNode>)> {
    let arrow_pos = line.find("-->")?;
    let before_arrow = &line[..arrow_pos];
    let after_arrow = &line[arrow_pos + 3..];

    let dash_pos = before_arrow.find("--")?;
    let from_part = before_arrow[..dash_pos].trim();
    let label = before_arrow[dash_pos + 2..].trim();
    let to_part = after_arrow.trim();

    if from_part.is_empty() || to_part.is_empty() {
        return None;
    }

    let (from_id, from_node) = parse_endpoint(from_part);
    let (to_id, to_node) = parse_endpoint(to_part);

    let mut nodes = Vec::new();
    if let Some(n) = from_node {
        nodes.push(n);
    }
    if let Some(n) = to_node {
        nodes.push(n);
    }

    Some((
        MermaidEdge {
            from: from_id,
            to: to_id,
            style: EdgeStyle::Arrow,
            label: if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
        },
        nodes,
    ))
}

fn parse_labeled_variant_with_nodes(
    line: &str,
    prefix: &str,
    suffix: &str,
    style: EdgeStyle,
) -> Option<(MermaidEdge, Vec<MermaidNode>)> {
    let suffix_pos = line.find(suffix)?;
    let before = &line[..suffix_pos];
    let after = &line[suffix_pos + suffix.len()..];

    let sep_pos = before.find(prefix)?;
    let from_part = before[..sep_pos].trim();
    let label = before[sep_pos + prefix.len()..].trim();
    let to_part = after.trim();

    if from_part.is_empty() || to_part.is_empty() {
        return None;
    }

    let (from_id, from_node) = parse_endpoint(from_part);
    let (to_id, to_node) = parse_endpoint(to_part);

    let mut nodes = Vec::new();
    if let Some(n) = from_node {
        nodes.push(n);
    }
    if let Some(n) = to_node {
        nodes.push(n);
    }

    Some((
        MermaidEdge {
            from: from_id,
            to: to_id,
            style,
            label: if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
        },
        nodes,
    ))
}

/// 确保节点存在（若不存在则创建 Plain 节点）
fn ensure_node(map: &mut std::collections::HashMap<String, MermaidNode>, id: &str) {
    if !map.contains_key(id) {
        map.insert(
            id.to_string(),
            MermaidNode { id: id.to_string(), text: id.to_string(), shape: NodeShape::Plain },
        );
    }
}

/// 解析单行为节点定义（如 `A[开始]`）
fn parse_node(line: &str) -> Option<MermaidNode> {
    // 尝试匹配各种形状
    // 矩形：A[文本]
    if let Some((id, text)) = extract_bracketed(line, '[', ']') {
        return Some(MermaidNode {
            id: id.trim().to_string(),
            text: text.trim().to_string(),
            shape: NodeShape::Rectangle,
        });
    }
    // 菱形：A{文本}
    if let Some((id, text)) = extract_bracketed(line, '{', '}') {
        return Some(MermaidNode {
            id: id.trim().to_string(),
            text: text.trim().to_string(),
            shape: NodeShape::Diamond,
        });
    }
    // 圆形：A((文本))
    if line.contains("((")
        && line.contains("))")
        && let Some(start) = line.find("((")
        && let Some(end) = line[start + 2..].find("))")
    {
        let text = &line[start + 2..start + 2 + end];
        let id = line[..start].trim();
        if !id.is_empty() {
            return Some(MermaidNode {
                id: id.to_string(),
                text: text.trim().to_string(),
                shape: NodeShape::Circle,
            });
        }
    }
    // 圆角矩形：A(文本)
    if let Some((id, text)) = extract_bracketed(line, '(', ')') {
        return Some(MermaidNode {
            id: id.trim().to_string(),
            text: text.trim().to_string(),
            shape: NodeShape::Rounded,
        });
    }
    // 纯 ID
    let id = line.trim();
    if !id.is_empty() && !id.contains(['-', '>', '=', '~']) {
        return Some(MermaidNode {
            id: id.to_string(),
            text: id.to_string(),
            shape: NodeShape::Plain,
        });
    }
    None
}

/// 提取 `A[文本]` 形式的 ID 和文本
fn extract_bracketed(line: &str, open: char, close: char) -> Option<(String, String)> {
    let open_str = open.to_string();
    let close_str = close.to_string();
    if let Some(open_pos) = line.find(&open_str)
        && let Some(close_pos) = line[open_pos + 1..].find(&close_str)
    {
        let id = line[..open_pos].trim();
        let text = &line[open_pos + 1..open_pos + 1 + close_pos];
        if !id.is_empty() {
            return Some((id.to_string(), text.to_string()));
        }
    }
    None
}

/// 从可能的 `A[文本]` 或 `A` 中提取 ID
fn extract_id(s: &str) -> String {
    let s = s.trim();
    // 找第一个非 ID 字符（[、(、{、空格等）
    let mut end = s.len();
    for (i, c) in s.char_indices() {
        if matches!(c, '[' | '(' | '{' | ' ' | '\t') {
            end = i;
            break;
        }
    }
    s[..end].trim().to_string()
}

/// 将 Mermaid 图渲染为 Unicode 框线字符文本
///
/// 输出格式：
/// ```text
/// ┌─────────┐
/// │  开始   │
/// └────┬────┘
///      │
///      ▼
/// ┌─────────┐     ┌─────────┐
/// │  处理   │────▶│  结束   │
/// └─────────┘     └─────────┘
/// ```
///
/// 简化版：按节点定义顺序列出，边用箭头表示。
pub fn render_to_text(graph: &MermaidGraph) -> String {
    if graph.nodes.is_empty() && graph.edges.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    // 标题
    out.push_str("流程图");
    if graph.top_down {
        out.push_str(" (自上而下)\n");
    } else {
        out.push_str(" (从左到右)\n");
    }

    // 节点列表
    out.push_str("\n节点:\n");
    for node in &graph.nodes {
        let shape_mark = match node.shape {
            NodeShape::Rectangle => "[]",
            NodeShape::Rounded => "()",
            NodeShape::Diamond => "{}",
            NodeShape::Circle => "(())",
            NodeShape::Plain => "",
        };
        out.push_str(&format!("  {} {} {}\n", node.id, shape_mark, node.text));
    }

    // 边列表
    if !graph.edges.is_empty() {
        out.push_str("\n连接:\n");
        for edge in &graph.edges {
            let arrow = match edge.style {
                EdgeStyle::Arrow => "──>",
                EdgeStyle::Line => "───",
                EdgeStyle::Dashed => "┄┄>",
                EdgeStyle::Thick => "══>",
            };
            if let Some(label) = &edge.label {
                out.push_str(&format!(
                    "  {} {} {} {} {}\n",
                    edge.from, arrow, label, arrow, edge.to
                ));
            } else {
                out.push_str(&format!("  {} {} {}\n", edge.from, arrow, edge.to));
            }
        }
    }

    // 简化 ASCII 流程图（线性布局）
    out.push_str("\n可视化:\n");
    out.push_str(&render_ascii_graph(graph));

    out
}

/// 渲染简化 ASCII 流程图（按拓扑顺序线性排列）
fn render_ascii_graph(graph: &MermaidGraph) -> String {
    let mut out = String::new();

    // 简化：按节点在 nodes 中的顺序渲染
    // 找出起始节点（没有入边的节点）
    let has_incoming: std::collections::HashSet<&String> =
        graph.edges.iter().map(|e| &e.to).collect();

    let start_nodes: Vec<&MermaidNode> =
        graph.nodes.iter().filter(|n| !has_incoming.contains(&n.id)).collect();

    if start_nodes.is_empty() && !graph.nodes.is_empty() {
        // 有环：从第一个节点开始
        if let Some(first) = graph.nodes.first() {
            return render_node_chain(graph, first.id.clone());
        }
        return String::new();
    }

    for start in &start_nodes {
        out.push_str(&render_node_chain(graph, start.id.clone()));
    }

    out
}

/// 从指定节点开始渲染链式流程图
fn render_node_chain(graph: &MermaidGraph, start_id: String) -> String {
    let mut out = String::new();
    let mut current = Some(start_id);
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

    while let Some(id) = current {
        if visited.contains(&id) {
            break; // 防止环
        }
        visited.insert(id.clone());

        // 找当前节点
        let node = graph.nodes.iter().find(|n| n.id == id);
        if let Some(node) = node {
            out.push_str(&render_node_box(node));
            out.push('\n');

            // 找出边
            let out_edges: Vec<&MermaidEdge> =
                graph.edges.iter().filter(|e| e.from == id).collect();

            if out_edges.len() == 1 {
                // 单一后继：画箭头
                let edge = out_edges[0];
                if let Some(label) = &edge.label {
                    out.push_str(&format!("      │\n      │ {}\n      ▼\n", label));
                } else {
                    out.push_str("      │\n      ▼\n");
                }
                current = Some(edge.to.clone());
            } else if out_edges.len() > 1 {
                // 多个后继：分支
                out.push_str("      │\n");
                for edge in &out_edges {
                    let label = edge.label.as_deref().unwrap_or("");
                    out.push_str(&format!("      ├──{}──> {}\n", label, edge.to));
                }
                current = None; // 分支处停止线性渲染
            } else {
                current = None; // 无后继
            }
        } else {
            break;
        }
    }

    out
}

/// 渲染单个节点为 ASCII 框
fn render_node_box(node: &MermaidNode) -> String {
    let text = &node.text;
    let padding = 2;
    let inner_width = text.chars().count() + padding * 2;

    match node.shape {
        NodeShape::Rectangle | NodeShape::Plain => {
            let top = format!("┌{}┐", "─".repeat(inner_width));
            let mid = format!("│{}{}{}│", " ".repeat(padding), text, " ".repeat(padding));
            let bot = format!("└{}┘", "─".repeat(inner_width));
            format!("  {}\n  {}\n  {}", top, mid, bot)
        },
        NodeShape::Rounded => {
            let top = format!("╭{}╮", "─".repeat(inner_width));
            let mid = format!("│{}{}{}│", " ".repeat(padding), text, " ".repeat(padding));
            let bot = format!("╰{}╯", "─".repeat(inner_width));
            format!("  {}\n  {}\n  {}", top, mid, bot)
        },
        NodeShape::Diamond => {
            // 菱形：简化为 <>
            let pad = inner_width / 2;
            let top = format!("{}◆{}", " ".repeat(pad + 2), " ".repeat(pad + 2));
            let mid = format!("  ◇{}{}{}◇", " ".repeat(padding), text, " ".repeat(padding));
            let bot = format!("{}◆{}", " ".repeat(pad + 2), " ".repeat(pad + 2));
            format!("{}\n{}\n{}", top, mid, bot)
        },
        NodeShape::Circle => {
            let top = format!("  (( {} ))", text);
            format!("  {}\n  {}", top, top)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_flowchart() {
        let mermaid = "graph TD\n    A[开始] --> B[结束]\n";
        let graph = parse_mermaid(mermaid);
        assert!(graph.top_down);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
        assert_eq!(graph.edges[0].style, EdgeStyle::Arrow);
    }

    #[test]
    fn parse_lr_direction() {
        let mermaid = "graph LR\n    A --> B\n";
        let graph = parse_mermaid(mermaid);
        assert!(!graph.top_down);
    }

    #[test]
    fn parse_node_shapes() {
        let mermaid = "graph TD\n    A[矩形]\n    B(圆角)\n    C{菱形}\n    D((圆形))\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.nodes.len(), 4);
        let shapes: Vec<&NodeShape> = graph.nodes.iter().map(|n| &n.shape).collect();
        assert!(shapes.contains(&&NodeShape::Rectangle));
        assert!(shapes.contains(&&NodeShape::Rounded));
        assert!(shapes.contains(&&NodeShape::Diamond));
        assert!(shapes.contains(&&NodeShape::Circle));
    }

    #[test]
    fn parse_labeled_edge() {
        let mermaid = "graph TD\n    A -->|是| B\n    B -->|否| C\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].label.as_deref(), Some("是"));
        assert_eq!(graph.edges[1].label.as_deref(), Some("否"));
    }

    #[test]
    fn parse_dashed_edge() {
        let mermaid = "graph TD\n    A -.-> B\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::Dashed);
    }

    #[test]
    fn parse_decision_branch() {
        let mermaid =
            "graph TD\n    A[开始] --> B{成功?}\n    B -->|是| C[处理]\n    B -->|否| D[错误]\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 3);
        // B 应为菱形
        let b = graph.nodes.iter().find(|n| n.id == "B").unwrap();
        assert_eq!(b.shape, NodeShape::Diamond);
    }

    #[test]
    fn render_text_output() {
        let mermaid = "graph TD\n    A[开始] --> B[结束]\n";
        let graph = parse_mermaid(mermaid);
        let text = render_to_text(&graph);
        assert!(text.contains("流程图"));
        assert!(text.contains("开始"));
        assert!(text.contains("结束"));
        assert!(text.contains("──>"));
    }

    #[test]
    fn render_decision_branch() {
        let mermaid = "graph TD\n    A --> B{成功?}\n    B -->|是| C\n    B -->|否| D\n";
        let graph = parse_mermaid(mermaid);
        let text = render_to_text(&graph);
        assert!(text.contains("是"));
        assert!(text.contains("否"));
    }

    #[test]
    fn empty_graph_renders_empty() {
        let graph = MermaidGraph::empty();
        let text = render_to_text(&graph);
        assert!(text.is_empty());
    }

    #[test]
    fn parse_comments_skipped() {
        let mermaid = "graph TD\n    %% 这是注释\n    A --> B\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn parse_implicit_nodes() {
        // 边定义中隐式创建节点
        let mermaid = "graph TD\n    A --> B\n";
        let graph = parse_mermaid(mermaid);
        assert_eq!(graph.nodes.len(), 2);
        let a = graph.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a.shape, NodeShape::Plain);
        assert_eq!(a.text, "A");
    }
}
