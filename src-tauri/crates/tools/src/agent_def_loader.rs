//! Agent 定义加载器 — 从 `.axagent/agents/` 目录加载 Markdown+YAML frontmatter 定义
//!
//! 格式示例：
//!
//! ```markdown
//! ---
//! name: my-custom-agent
//! description: 自定义代码审查 agent
//! whenToUse: 当需要深度代码审查时使用
//! tools: [FileRead, Grep, Glob, Bash]
//! model: opus
//! background: false
//! ---
//!
//! # System Prompt
//! 你是一个专业的代码审查 Agent...
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent_def_types::{AgentDefSource, AgentDefinition};

/// 从 frontmatter 字符串解析出的原始数据
struct Frontmatter {
    fields: Vec<(String, String)>,
    body: String,
}

/// 简单 YAML frontmatter 解析器（不依赖完整 YAML 库）
///
/// 支持单行键值对和嵌套 YAML（缩进块），如 mcpServers。
fn parse_frontmatter(content: &str) -> Option<Frontmatter> {
    let content = content.trim();
    // 必须以 `---` 开头
    let rest = content.strip_prefix("---")?;
    // 找到结束的 `---`
    let (frontmatter_part, body) = rest.split_once("\n---")?;

    let mut fields = Vec::new();
    let mut lines = frontmatter_part.lines().peekable();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim();

            if value.is_empty() {
                // 可能是多行嵌套 YAML，收集后续缩进行
                let mut nested_lines: Vec<String> = Vec::new();
                while let Some(next_line) = lines.peek() {
                    let trimmed = next_line.trim();
                    if trimmed.is_empty() {
                        // 空行：如果是块的一部分（前面已收集了内容），继续收集
                        if !nested_lines.is_empty() {
                            nested_lines.push(lines.next().unwrap().to_string());
                        } else {
                            // 还没有收集到内容，跳过空行
                            lines.next();
                        }
                    } else if next_line.starts_with(' ') || next_line.starts_with('\t') {
                        // 缩进行属于嵌套块
                        nested_lines.push(lines.next().unwrap().to_string());
                    } else {
                        // 非缩进行表示嵌套块结束
                        break;
                    }
                }
                if !nested_lines.is_empty() {
                    // 去掉每行开头的公共缩进（2 空格或 4 空格）
                    let dedented: Vec<String> = nested_lines
                        .iter()
                        .map(|l| l.trim_start().to_string())
                        .collect();
                    fields.push((key, dedented.join("\n")));
                } else {
                    // 空值字段
                    fields.push((key, String::new()));
                }
            } else {
                let value = value
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .to_string();
                fields.push((key, value));
            }
        }
    }

    Some(Frontmatter {
        fields,
        body: body.trim().to_string(),
    })
}

/// 解析 YAML 格式的字符串数组，如 `[FileRead, Grep, Glob]` 或 `[FileRead,Grep]`
fn parse_yaml_array(value: &str) -> Vec<String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Vec::new();
    }
    let inner = &value[1..value.len() - 1];
    inner
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 解析 mcpServers 嵌套 YAML 块为 AgentMcpServerSpec 列表
///
/// 输入已去缩进的文本块，格式示例：
/// ```text
/// - name: filesystem
///   command: npx
///   args: [-y, @anthropic/mcp-filesystem]
/// - name: github
///   command: npx
///   args: [-y, @anthropic/mcp-github]
/// ```
fn parse_mcp_servers(block: &str) -> Vec<crate::agent_def_types::AgentMcpServerSpec> {
    let mut servers = Vec::new();
    let mut current_name = String::new();
    let mut current_command = String::new();
    let mut current_args = Vec::new();
    let mut in_entry = false;

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 检测新条目开始 "- name:"
        if trimmed.starts_with("- name:") {
            // 保存上一个条目
            if in_entry && !current_name.is_empty() {
                servers.push(crate::agent_def_types::AgentMcpServerSpec {
                    name: std::mem::take(&mut current_name),
                    command: std::mem::take(&mut current_command),
                    args: std::mem::take(&mut current_args),
                });
            }
            current_name = trimmed
                .strip_prefix("- name:")
                .unwrap()
                .trim()
                .trim_matches('"')
                .to_string();
            current_command = String::new();
            current_args = Vec::new();
            in_entry = true;
        } else if trimmed.starts_with("command:") {
            current_command = trimmed
                .strip_prefix("command:")
                .unwrap()
                .trim()
                .trim_matches('"')
                .to_string();
        } else if trimmed.starts_with("args:") {
            let args_str = trimmed.strip_prefix("args:").unwrap().trim();
            current_args = parse_yaml_array(args_str);
        }
    }

    // 保存最后一个条目
    if in_entry && !current_name.is_empty() {
        servers.push(crate::agent_def_types::AgentMcpServerSpec {
            name: current_name,
            command: current_command,
            args: current_args,
        });
    }

    servers
}

/// 从 frontmatter 字段构建 AgentDefinition
fn build_definition(
    frontmatter: &Frontmatter,
    source: AgentDefSource,
    path: &Path,
) -> AgentDefinition {
    let mut def = AgentDefinition::builtin("", "");
    def.source = source;
    def.source_path = Some(path.display().to_string());

    for (key, value) in &frontmatter.fields {
        match key.as_str() {
            "name" | "agent_type" => def.agent_type = value.clone(),
            "description" => def.description = value.clone(),
            "whenToUse" | "when_to_use" => def.when_to_use = value.clone(),
            "tools" => def.tools = parse_yaml_array(value),
            "disallowedTools" | "disallowed_tools" => {
                def.disallowed_tools = parse_yaml_array(value)
            },
            "skills" => def.skills = parse_yaml_array(value),
            "model" => def.model = Some(value.clone()),
            "background" => def.background = value == "true",
            "isolation" => {
                def.isolation = match value.as_str() {
                    "worktree" => Some(crate::agent_def_types::IsolationMode::Worktree),
                    "remote" => Some(crate::agent_def_types::IsolationMode::Remote),
                    _ => None,
                };
            },
            "permissionMode" | "permission_mode" => def.permission_mode = Some(value.clone()),
            "maxTurns" | "max_turns" => {
                if let Ok(n) = value.parse::<u32>() {
                    def.max_turns = Some(n);
                }
            },
            "memoryScope" | "memory_scope" => {
                def.memory_scope = match value.as_str() {
                    "user" => Some(crate::agent_def_types::MemoryScope::User),
                    "project" => Some(crate::agent_def_types::MemoryScope::Project),
                    "local" => Some(crate::agent_def_types::MemoryScope::Local),
                    _ => None,
                };
            },
            "color" => def.color = Some(value.clone()),
            "hooks" => def.hooks = parse_yaml_array(value),
            "mcpServers" | "mcp_servers" => {
                def.mcp_servers = parse_mcp_servers(value);
            },
            "omitClaudeMd" | "omit_claude_md" => def.omit_claude_md = value == "true",
            "initialPrompt" | "initial_prompt" => def.initial_prompt = Some(value.clone()),
            _ => {},
        }
    }

    // 从 Markdown 正文提取 system prompt
    if !frontmatter.body.is_empty() {
        def.system_prompt = Some(frontmatter.body.clone());
    }

    // 如果 frontmatter 中没有 name，从文件名提取
    if def.agent_type.is_empty() {
        def.agent_type = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();
    }

    def
}

/// 加载指定目录下所有 `.md` agent 定义文件
pub fn load_agents_from_dir(dir: &Path) -> Vec<AgentDefinition> {
    let mut definitions = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return definitions;
    }

    let source = if dir.starts_with(dirs::home_dir().unwrap_or_default()) {
        AgentDefSource::User
    } else {
        AgentDefSource::Project
    };

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return definitions,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "md" && ext != "markdown" {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(frontmatter) = parse_frontmatter(&content) {
            let def = build_definition(&frontmatter, source, &path);
            definitions.push(def);
        }
    }

    definitions
}

/// 获取用户级 agent 目录
pub fn user_agents_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".axagent")
        .join("agents")
}

/// 获取项目级 agent 目录
pub fn project_agents_dir(cwd: &Path) -> PathBuf {
    cwd.join(".axagent").join("agents")
}

/// 加载所有来源的 agent 定义
pub fn load_all_agents(cwd: &Path) -> Vec<AgentDefinition> {
    let mut all = Vec::new();
    all.extend(load_agents_from_dir(&user_agents_dir()));
    all.extend(load_agents_from_dir(&project_agents_dir(cwd)));
    all
}

// ── Agent MEMORY.md 加载 ──

/// 获取 agent 类型的 MEMORY.md 路径
///
/// 按作用域返回不同路径：
/// - `User`: `~/.axagent/agent-memory/<agent_type>/MEMORY.md`
/// - `Project`: `<project>/.axagent/agent-memory/<agent_type>/MEMORY.md`
/// - `Local`: `<project>/.axagent/agent-memory-local/<agent_type>/MEMORY.md`
pub fn agent_memory_path(
    agent_type: &str,
    scope: &crate::agent_def_types::MemoryScope,
    cwd: &Path,
) -> PathBuf {
    match scope {
        crate::agent_def_types::MemoryScope::User => dirs::home_dir()
            .unwrap_or_default()
            .join(".axagent")
            .join("agent-memory")
            .join(agent_type)
            .join("MEMORY.md"),
        crate::agent_def_types::MemoryScope::Project => cwd
            .join(".axagent")
            .join("agent-memory")
            .join(agent_type)
            .join("MEMORY.md"),
        crate::agent_def_types::MemoryScope::Local => cwd
            .join(".axagent")
            .join("agent-memory-local")
            .join(agent_type)
            .join("MEMORY.md"),
    }
}

/// 加载 agent 的持久化记忆内容
///
/// 按优先级加载：Local > Project > User（后面的覆盖前面的）
/// 返回格式化的记忆文本，可直接插入 system prompt
pub fn load_agent_memory(
    agent_type: &str,
    memory_scope: Option<&crate::agent_def_types::MemoryScope>,
    cwd: &Path,
) -> Option<String> {
    let scope = memory_scope.unwrap_or(&crate::agent_def_types::MemoryScope::Project);
    let path = agent_memory_path(agent_type, scope, cwd);

    match std::fs::read_to_string(&path) {
        Ok(content) if !content.trim().is_empty() => {
            tracing::info!("加载 agent 记忆: {} ({:?})", agent_type, scope);
            Some(format!(
                "## Agent 持久化记忆 ({})\n\n{}",
                agent_type,
                content.trim()
            ))
        },
        _ => None,
    }
}

/// 保存 agent 记忆内容
pub fn save_agent_memory(
    agent_type: &str,
    scope: &crate::agent_def_types::MemoryScope,
    cwd: &Path,
    content: &str,
) -> std::io::Result<()> {
    let path = agent_memory_path(agent_type, scope, cwd);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)
}

/// 加载所有作用域的 agent 记忆（User + Project + Local），合并返回
pub fn load_all_agent_memories(agent_type: &str, cwd: &Path) -> Option<String> {
    let scopes = [
        crate::agent_def_types::MemoryScope::User,
        crate::agent_def_types::MemoryScope::Project,
        crate::agent_def_types::MemoryScope::Local,
    ];

    let memories: Vec<String> = scopes
        .iter()
        .filter_map(|scope| load_agent_memory(agent_type, Some(scope), cwd))
        .collect();

    if memories.is_empty() {
        None
    } else {
        Some(memories.join("\n\n---\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_frontmatter() {
        let content = r#"---
name: test-agent
description: 测试 agent
tools: [Read, Grep]
model: haiku
background: true
---
# System Prompt

你是一个测试 agent。"#;

        let fm = parse_frontmatter(content).expect("should parse");
        assert!(fm
            .fields
            .iter()
            .any(|(k, v)| k == "name" && v == "test-agent"));
        assert!(fm
            .fields
            .iter()
            .any(|(k, v)| k == "description" && v == "测试 agent"));
        assert!(fm.fields.iter().any(|(k, v)| k == "model" && v == "haiku"));
        assert!(fm
            .fields
            .iter()
            .any(|(k, v)| k == "background" && v == "true"));
        assert!(fm.body.contains("你是一个测试 agent"));
    }

    #[test]
    fn parse_tools_array() {
        let result = parse_yaml_array("[FileRead, Grep, Glob]");
        assert_eq!(result, vec!["FileRead", "Grep", "Glob"]);
    }

    #[test]
    fn parse_empty_array() {
        let result = parse_yaml_array("[]");
        assert!(result.is_empty());
    }

    #[test]
    fn build_definition_from_frontmatter() {
        let content = r#"---
name: reviewer
description: 代码审查
whenToUse: 审查 PR 时使用
tools: [Read, Grep]
model: opus
maxTurns: 25
memoryScope: project
color: blue
---
你是一个代码审查专家。"#;

        let fm = parse_frontmatter(content).unwrap();
        let def = build_definition(
            &fm,
            AgentDefSource::User,
            Path::new("/tmp/agents/reviewer.md"),
        );

        assert_eq!(def.agent_type, "reviewer");
        assert_eq!(def.description, "代码审查");
        assert_eq!(def.when_to_use, "审查 PR 时使用");
        assert_eq!(def.tools, vec!["Read", "Grep"]);
        assert_eq!(def.model, Some("opus".to_string()));
        assert_eq!(def.max_turns, Some(25));
        assert!(def.system_prompt.is_some());
        assert!(def.system_prompt.unwrap().contains("代码审查专家"));
        assert!(!def.background);
    }
}
