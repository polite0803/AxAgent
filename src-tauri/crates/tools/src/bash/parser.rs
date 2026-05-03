//! Bash 命令解析器
//!
//! 解析 shell 命令为结构化的命令序列：提取 argv、重定向、环境变量。

use std::collections::HashMap;

/// 解析后的命令
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// 命令行参数
    pub argv: Vec<String>,
    /// 输入/输出重定向
    pub redirects: Vec<Redirect>,
    /// 环境变量赋值（如 FOO=bar cmd）
    pub env_vars: HashMap<String, String>,
    /// 是否为后台运行（&）
    pub background: bool,
    /// 下一个管道命令
    pub next_pipe: Option<Box<ParsedCommand>>,
    /// 下一个逻辑连接（&& 或 ||）
    pub next_conditional: Option<(ConditionalOp, Box<ParsedCommand>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalOp {
    And, // &&
    Or,  // ||
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectKind {
    Input,        // <
    Output,       // >
    Append,       // >>
    InputHere,    // <<
    InputHereStr, // <<<
    OutputForce,  // >|
    DupInput,     // <&
    DupOutput,    // >&
    ErrToOut,     // 2>&1
    OutToErr,     // 1>&2
}

/// 简单词法解析——将命令字符串拆为 token
#[derive(Debug)]
enum Token {
    Word(String),
    Pipe,           // |
    And,            // &&
    Or,             // ||
    Semicolon,      // ;
    Background,     // &
    RedirectIn,     // <
    RedirectOut,    // >
    RedirectAppend, // >>
}

/// 解析 shell 命令为 ParsedCommand
///
/// 当前实现使用简单词法分析，不依赖 tree-sitter。
/// 后续可升级为 tree-sitter-bash 以获得完整 AST。
pub fn parse_command(input: &str) -> Result<ParsedCommand, String> {
    let tokens = tokenize(input)?;
    parse_tokens(&tokens)
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // 空白
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // 管道
        if c == '|' {
            if i + 1 < chars.len() && chars[i + 1] == '|' {
                tokens.push(Token::Or);
                i += 2;
            } else {
                tokens.push(Token::Pipe);
                i += 1;
            }
            continue;
        }

        // &&
        if c == '&' && i + 1 < chars.len() && chars[i + 1] == '&' {
            tokens.push(Token::And);
            i += 2;
            continue;
        }

        // &
        if c == '&' {
            tokens.push(Token::Background);
            i += 1;
            continue;
        }

        // ;
        if c == ';' {
            tokens.push(Token::Semicolon);
            i += 1;
            continue;
        }

        // >>
        if c == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
            tokens.push(Token::RedirectAppend);
            i += 2;
            continue;
        }

        // >
        if c == '>' {
            tokens.push(Token::RedirectOut);
            i += 1;
            continue;
        }

        // <
        if c == '<' {
            tokens.push(Token::RedirectIn);
            i += 1;
            continue;
        }

        // 引号字符串
        if c == '\'' || c == '"' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && quote == '"' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip closing quote
            }
            tokens.push(Token::Word(s));
            continue;
        }

        // 普通词
        let mut s = String::new();
        while i < chars.len()
            && !chars[i].is_whitespace()
            && !matches!(chars[i], '|' | '&' | ';' | '<' | '>')
        {
            s.push(chars[i]);
            i += 1;
        }
        tokens.push(Token::Word(s));
    }

    Ok(tokens)
}

fn parse_tokens(tokens: &[Token]) -> Result<ParsedCommand, String> {
    let mut argv = Vec::new();
    let mut redirects = Vec::new();
    let mut env_vars = HashMap::new();
    let mut background = false;
    let mut next_pipe = None;
    let mut next_conditional = None;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(w) => {
                // 检测环境变量赋值 (KEY=VALUE)
                if let Some(eq_pos) = w.find('=') {
                    if eq_pos > 0 && w[..eq_pos].chars().all(|c| c.is_alphanumeric() || c == '_') {
                        let key = w[..eq_pos].to_string();
                        let value = w[eq_pos + 1..].to_string();
                        env_vars.insert(key, value);
                        i += 1;
                        continue;
                    }
                }
                argv.push(w.clone());
            },
            Token::Pipe => {
                let remaining = &tokens[i + 1..];
                next_pipe = Some(Box::new(parse_tokens(remaining)?));
                break;
            },
            Token::And => {
                let remaining = &tokens[i + 1..];
                next_conditional = Some((ConditionalOp::And, Box::new(parse_tokens(remaining)?)));
                break;
            },
            Token::Or => {
                let remaining = &tokens[i + 1..];
                next_conditional = Some((ConditionalOp::Or, Box::new(parse_tokens(remaining)?)));
                break;
            },
            Token::Background => {
                background = true;
            },
            Token::Semicolon => {
                break; // 后面的视为独立命令
            },
            Token::RedirectOut | Token::RedirectAppend | Token::RedirectIn => {
                let kind = match &tokens[i] {
                    Token::RedirectOut => RedirectKind::Output,
                    Token::RedirectAppend => RedirectKind::Append,
                    Token::RedirectIn => RedirectKind::Input,
                    _ => unreachable!(),
                };
                if i + 1 < tokens.len() {
                    if let Token::Word(target) = &tokens[i + 1] {
                        redirects.push(Redirect {
                            kind,
                            target: target.clone(),
                        });
                        i += 1;
                    }
                }
            },
        }
        i += 1;
    }

    Ok(ParsedCommand {
        argv,
        redirects,
        env_vars,
        background,
        next_pipe,
        next_conditional,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let cmd = parse_command("git status").unwrap();
        assert_eq!(cmd.argv, vec!["git", "status"]);
        assert!(cmd.next_pipe.is_none());
    }

    #[test]
    fn test_command_with_flags() {
        let cmd = parse_command("ls -la /tmp").unwrap();
        assert_eq!(cmd.argv, vec!["ls", "-la", "/tmp"]);
    }

    #[test]
    fn test_pipe() {
        let cmd = parse_command("cat file.txt | grep error").unwrap();
        assert_eq!(cmd.argv, vec!["cat", "file.txt"]);
        assert!(cmd.next_pipe.is_some());
        assert_eq!(cmd.next_pipe.unwrap().argv, vec!["grep", "error"]);
    }

    #[test]
    fn test_redirect() {
        let cmd = parse_command("echo hello > out.txt").unwrap();
        assert_eq!(cmd.argv, vec!["echo", "hello"]);
        assert_eq!(cmd.redirects.len(), 1);
        assert_eq!(cmd.redirects[0].kind, RedirectKind::Output);
        assert_eq!(cmd.redirects[0].target, "out.txt");
    }

    #[test]
    fn test_quoted_args() {
        let cmd = parse_command(r#"echo "hello world""#).unwrap();
        assert_eq!(cmd.argv, vec!["echo", "hello world"]);
    }
}
