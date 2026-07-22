// SPDX-License-Identifier: AGPL-3.0-only

//! 数学公式支持（LaTeX 子集 → Unicode + 上下标结构）
//!
//! 完整 LaTeX 解析超出范围。本模块实现"工程可用"的子集：
//!
//! - 希腊字母：`\alpha` → α、`\beta` → β …
//! - 常用符号：`\sum` → ∑、`\int` → ∫、`\sqrt` → √、`\infty` → ∞ …
//! - 关系运算：`\leq` → ≤、`\geq` → ≥、`\neq` → ≠、`\approx` → ≈ …
//! - 上下标：`x^2` / `x_i` / `x^{2n}` / `x_{i,j}`
//! - 分式简化：`\frac{a}{b}` → a/b（精度有限但可读）
//! - 根式：`\sqrt{x}` → √x
//! - 组合字符：`\bar{x}` → x̄
//!
//! 输出 `Vec<MathSegment>`：每个片段携带基字符 + 可选上下标，
//! 供 PDF / DOCX / PPTX 导出端做位置微调。

/// 数学片段：基字符 + 可选上标 + 可选下标
#[derive(Debug, Clone, PartialEq)]
pub struct MathSegment {
    /// 基字符（可为空字符串，纯上下标）
    pub text: String,
    /// 上标（如 `2` 表示 x²）
    pub sup: Option<String>,
    /// 下标（如 `i` 表示 xᵢ）
    pub sub: Option<String>,
}

impl MathSegment {
    pub fn plain(s: impl Into<String>) -> Self {
        Self { text: s.into(), sup: None, sub: None }
    }

    pub fn with_sup(s: impl Into<String>, sup: impl Into<String>) -> Self {
        Self { text: s.into(), sup: Some(sup.into()), sub: None }
    }

    pub fn with_sub(s: impl Into<String>, sub: impl Into<String>) -> Self {
        Self { text: s.into(), sup: None, sub: Some(sub.into()) }
    }
}

/// LaTeX → 段列表（轻量解析）
///
/// 示例：
/// - `x^2` → `[MathSegment { text: "x", sup: Some("2") }]`
/// - `\frac{a}{b}` → `[MathSegment { text: "a/b" }]`
/// - `\sqrt{x+y}` → `[MathSegment { text: "√(x+y)" }]`
pub fn parse_latex(latex: &str) -> Vec<MathSegment> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = latex.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // 命令名收集到第一个非字母字符
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_alphabetic() {
                    name.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(mapped) = lookup_command(&name) {
                tokens.push(Token::Symbol(mapped));
            } else if name == "frac" {
                tokens.push(Token::Frac);
            } else if name == "sqrt" {
                tokens.push(Token::Sqrt);
            } else if name == "bar" {
                tokens.push(Token::Bar);
            } else if name == "vec" {
                tokens.push(Token::Vec);
            } else if name == "hat" {
                tokens.push(Token::Hat);
            } else if name == "tilde" {
                tokens.push(Token::Tilde);
            } else if name == "text" || name == "mathrm" || name == "mathrm" {
                // \text{...} 视为字面量；花括号内容在下方 Group 中识别
                tokens.push(Token::Literal);
            } else if name == "left" || name == "right" {
                // 忽略 \left \right 分隔符
            } else if !name.is_empty() {
                // 未知命令：保留原始形式
                tokens.push(Token::Symbol(format!("\\{}", name)));
            }
        } else if c == '^' {
            tokens.push(Token::Sup);
        } else if c == '_' {
            tokens.push(Token::Sub);
        } else if c == '{' {
            // 收集花括号内容（支持一层嵌套，深度 1）
            let mut depth = 1;
            let mut content = String::new();
            for nc in chars.by_ref() {
                if nc == '{' {
                    depth += 1;
                    content.push(nc);
                } else if nc == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    content.push(nc);
                } else {
                    content.push(nc);
                }
            }
            tokens.push(Token::Group(content));
        } else if c == '}' {
            // 多余的 }：忽略
        } else if c == ' ' || c == '\t' {
            // 忽略空白
        } else {
            tokens.push(Token::Symbol(c.to_string()));
        }
    }
    // 解析 token 列表 → MathSegment 列表
    assemble(&tokens)
}

#[derive(Debug, Clone)]
enum Token {
    Symbol(String),
    Group(String),
    Sup,
    Sub,
    Frac,
    Sqrt,
    Bar,
    Vec,
    Hat,
    Tilde,
    Literal,
}

fn assemble(tokens: &[Token]) -> Vec<MathSegment> {
    let mut out: Vec<MathSegment> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            // 孤立的 Sup/Sub（没有主体可附着）跳过
            Token::Sup | Token::Sub => {
                i += 1;
            },
            _ => {
                let (seg, consumed) = process_body(&tokens[i..]);
                out.push(seg);
                i += consumed;
            },
        }
    }
    out
}

/// 处理主体 token（Symbol/Group/Frac/Sqrt/Bar/...）并吸收其后紧邻的 sub/sup 附件
fn process_body(tokens: &[Token]) -> (MathSegment, usize) {
    if tokens.is_empty() {
        return (MathSegment::plain(""), 0);
    }
    let mut seg = match &tokens[0] {
        Token::Symbol(s) | Token::Group(s) => MathSegment::plain(s.clone()),
        Token::Frac => {
            let (num, n1) = read_group_or_symbol(tokens, 1);
            let (den, n2) = read_group_or_symbol(tokens, 1 + n1);
            let mut s = MathSegment::plain(format!("{}/{}", num, den));
            attach_after(&mut s, tokens, 1 + n1 + n2);
            let consumed =
                1 + n1 + n2 + (s.sup.is_some() as usize) * 2 + (s.sub.is_some() as usize) * 2;
            return (s, consumed);
        },
        Token::Sqrt => {
            let (inner, n) = read_group_or_symbol(tokens, 1);
            let s = if inner.chars().count() == 1 {
                format!("√{}", inner)
            } else {
                format!("√({})", inner)
            };
            return (MathSegment::plain(s), 1 + n);
        },
        Token::Bar => {
            let (inner, n) = read_group_or_symbol(tokens, 1);
            return (MathSegment::plain(format!("{}̄", inner)), 1 + n);
        },
        Token::Vec => {
            let (inner, n) = read_group_or_symbol(tokens, 1);
            return (MathSegment::plain(format!("{}⃗", inner)), 1 + n);
        },
        Token::Hat => {
            let (inner, n) = read_group_or_symbol(tokens, 1);
            return (MathSegment::plain(format!("{}̂", inner)), 1 + n);
        },
        Token::Tilde => {
            let (inner, n) = read_group_or_symbol(tokens, 1);
            return (MathSegment::plain(format!("{}̃", inner)), 1 + n);
        },
        Token::Literal => {
            // \text{...}：紧跟一个 Group
            if tokens.len() > 1
                && let Token::Group(s) = &tokens[1]
            {
                let mut seg = MathSegment::plain(s.clone());
                attach_after(&mut seg, tokens, 2);
                let consumed = 2
                    + if seg.sup.is_some() { 2 } else { 0 }
                    + if seg.sub.is_some() { 2 } else { 0 };
                return (seg, consumed);
            }
            return (MathSegment::plain(""), 1);
        },
        Token::Sup | Token::Sub => unreachable!(),
    };
    let consumed = attach_after(&mut seg, tokens, 1);
    (seg, consumed)
}

/// 在主体之后尝试吸收 1~2 个 sub/sup 附件，返回总消耗 token 数
fn attach_after(seg: &mut MathSegment, tokens: &[Token], mut pos: usize) -> usize {
    let mut consumed = 1; // 已消耗主体
    // 第一次：可能 sub 或 sup
    if pos < tokens.len() {
        match &tokens[pos] {
            Token::Sub => {
                if let Some(s) = read_target(tokens, pos + 1) {
                    seg.sub = Some(s);
                    consumed += 2;
                    pos += 2;
                } else {
                    consumed += 1;
                    pos += 1;
                }
            },
            Token::Sup => {
                if let Some(s) = read_target(tokens, pos + 1) {
                    seg.sup = Some(s);
                    consumed += 2;
                    pos += 2;
                } else {
                    consumed += 1;
                    pos += 1;
                }
            },
            _ => return consumed,
        }
    }
    // 第二次：另一个顺序的 sub 或 sup
    if pos < tokens.len() {
        match &tokens[pos] {
            Token::Sub if seg.sub.is_none() => {
                if let Some(s) = read_target(tokens, pos + 1) {
                    seg.sub = Some(s);
                    consumed += 2;
                }
            },
            Token::Sup if seg.sup.is_none() => {
                if let Some(s) = read_target(tokens, pos + 1) {
                    seg.sup = Some(s);
                    consumed += 2;
                }
            },
            _ => {},
        }
    }
    consumed
}

fn read_target(tokens: &[Token], pos: usize) -> Option<String> {
    if pos < tokens.len() {
        match &tokens[pos] {
            Token::Group(s) | Token::Symbol(s) => Some(s.clone()),
            _ => None,
        }
    } else {
        None
    }
}

fn read_group_or_symbol(tokens: &[Token], start: usize) -> (String, usize) {
    read_target(tokens, start).map(|s| (s, 1)).unwrap_or((String::new(), 0))
}

/// 命令名字 → Unicode 符号
fn lookup_command(name: &str) -> Option<String> {
    Some(
        match name {
            // 希腊字母（小写）
            "alpha" => "α",
            "beta" => "β",
            "gamma" => "γ",
            "delta" => "δ",
            "epsilon" => "ε",
            "varepsilon" => "ε",
            "zeta" => "ζ",
            "eta" => "η",
            "theta" => "θ",
            "vartheta" => "ϑ",
            "iota" => "ι",
            "kappa" => "κ",
            "lambda" => "λ",
            "mu" => "μ",
            "nu" => "ν",
            "xi" => "ξ",
            "omicron" => "ο",
            "pi" => "π",
            "varpi" => "ϖ",
            "rho" => "ρ",
            "varrho" => "ϱ",
            "sigma" => "σ",
            "varsigma" => "ς",
            "tau" => "τ",
            "upsilon" => "υ",
            "phi" => "φ",
            "varphi" => "ϕ",
            "chi" => "χ",
            "psi" => "ψ",
            "omega" => "ω",
            // 希腊字母（大写）
            "Alpha" => "Α",
            "Beta" => "Β",
            "Gamma" => "Γ",
            "Delta" => "Δ",
            "Epsilon" => "Ε",
            "Zeta" => "Ζ",
            "Eta" => "Η",
            "Theta" => "Θ",
            "Iota" => "Ι",
            "Kappa" => "Κ",
            "Lambda" => "Λ",
            "Mu" => "Μ",
            "Nu" => "Ν",
            "Xi" => "Ξ",
            "Omicron" => "Ο",
            "Pi" => "Π",
            "Rho" => "Ρ",
            "Sigma" => "Σ",
            "Tau" => "Τ",
            "Upsilon" => "Υ",
            "Phi" => "Φ",
            "Chi" => "Χ",
            "Psi" => "Ψ",
            "Omega" => "Ω",
            // 运算符
            "sum" => "∑",
            "prod" => "∏",
            "int" => "∫",
            "iint" => "∬",
            "iiint" => "∭",
            "oint" => "∮",
            "partial" => "∂",
            "nabla" => "∇",
            "pm" => "±",
            "mp" => "∓",
            "times" => "×",
            "div" => "÷",
            "cdot" => "·",
            "bullet" => "•",
            "circ" => "∘",
            "oplus" => "⊕",
            "ominus" => "⊖",
            "otimes" => "⊗",
            // 关系
            "leq" => "≤",
            "le" => "≤",
            "geq" => "≥",
            "ge" => "≥",
            "neq" => "≠",
            "ne" => "≠",
            "approx" => "≈",
            "equiv" => "≡",
            "sim" => "∼",
            "simeq" => "≃",
            "cong" => "≅",
            "propto" => "∝",
            "ll" => "≪",
            "gg" => "≫",
            "in" => "∈",
            "notin" => "∉",
            "ni" => "∋",
            "subset" => "⊂",
            "supset" => "⊃",
            "subseteq" => "⊆",
            "supseteq" => "⊇",
            "cup" => "∪",
            "cap" => "∩",
            "setminus" => "∖",
            "emptyset" => "∅",
            // 箭头
            "to" => "→",
            "rightarrow" => "→",
            "leftarrow" => "←",
            "Rightarrow" => "⇒",
            "Leftarrow" => "⇐",
            "leftrightarrow" => "↔",
            "Leftrightarrow" => "⇔",
            "mapsto" => "↦",
            "uparrow" => "↑",
            "downarrow" => "↓",
            // 逻辑
            "forall" => "∀",
            "exists" => "∃",
            "nexists" => "∄",
            "neg" => "¬",
            "land" => "∧",
            "wedge" => "∧",
            "lor" => "∨",
            "vee" => "∨",
            // 其它
            "infty" => "∞",
            "ldots" => "…",
            "cdots" => "⋯",
            "vdots" => "⋮",
            "ddots" => "⋱",
            "hbar" => "ℏ",
            "ell" => "ℓ",
            "Re" => "ℜ",
            "Im" => "ℑ",
            "aleph" => "ℵ",
            "angle" => "∠",
            // 括号
            "langle" => "⟨",
            "rangle" => "⟩",
            "lceil" => "⌈",
            "rceil" => "⌉",
            "lfloor" => "⌊",
            "rfloor" => "⌋",
            _ => return None,
        }
        .to_string(),
    )
}

/// 将段列表渲染成单一字符串（用于无位置渲染能力的目标，如 XLSX）
pub fn segments_to_plain(segments: &[MathSegment]) -> String {
    let mut out = String::new();
    for seg in segments {
        out.push_str(&seg.text);
        if let Some(s) = &seg.sup {
            out.push_str(&format!("^{}", s));
        }
        if let Some(s) = &seg.sub {
            out.push_str(&format!("_{}", s));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_greek_letters() {
        let segs = parse_latex("\\alpha + \\beta = \\gamma");
        let text = segments_to_plain(&segs);
        // LaTeX 中空白字符是词法分隔符，被吞是标准行为
        assert_eq!(text, "α+β=γ");
        assert_eq!(segs[0].text, "α");
        assert_eq!(segs[2].text, "β");
        assert_eq!(segs[4].text, "γ");
    }

    #[test]
    fn parse_superscript() {
        let segs = parse_latex("x^2");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "x");
        assert_eq!(segs[0].sup.as_deref(), Some("2"));
    }

    #[test]
    fn parse_subscript() {
        let segs = parse_latex("x_i");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "x");
        assert_eq!(segs[0].sub.as_deref(), Some("i"));
    }

    #[test]
    fn parse_sup_sub_combined() {
        let segs = parse_latex("x_i^2");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "x");
        assert_eq!(segs[0].sup.as_deref(), Some("2"));
        assert_eq!(segs[0].sub.as_deref(), Some("i"));
    }

    #[test]
    fn parse_grouped_sup() {
        let segs = parse_latex("e^{2n}");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "e");
        assert_eq!(segs[0].sup.as_deref(), Some("2n"));
    }

    #[test]
    fn parse_frac() {
        let segs = parse_latex("\\frac{a}{b}");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "a/b");
    }

    #[test]
    fn parse_sqrt() {
        let segs = parse_latex("\\sqrt{x+1}");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "√(x+1)");
    }

    #[test]
    fn parse_sqrt_single() {
        let segs = parse_latex("\\sqrt{2}");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "√2");
    }

    #[test]
    fn parse_inequality() {
        let segs = parse_latex("a \\leq b \\neq c");
        let text = segments_to_plain(&segs);
        // LaTeX 空白被吞
        assert_eq!(text, "a≤b≠c");
    }

    #[test]
    fn parse_bar() {
        let segs = parse_latex("\\bar{x}");
        assert_eq!(segs[0].text, "x̄");
    }

    #[test]
    fn parse_complex_expression() {
        // 勾股定理
        let segs = parse_latex("a^2 + b^2 = c^2");
        let text = segments_to_plain(&segs);
        assert_eq!(text, "a^2+b^2=c^2");
        // 验证结构
        assert_eq!(segs[0].sup.as_deref(), Some("2"));
        assert_eq!(segs[2].sup.as_deref(), Some("2"));
        assert_eq!(segs[4].sup.as_deref(), Some("2"));
    }

    #[test]
    fn parse_text_literal() {
        let segs = parse_latex("\\text{where } x > 0");
        let text = segments_to_plain(&segs);
        // \text{} 内部空白保留，外部空白被吞
        assert_eq!(text, "where x>0");
    }

    #[test]
    fn parse_unknown_command_kept() {
        let segs = parse_latex("\\foo bar");
        let text = segments_to_plain(&segs);
        assert!(text.contains("\\foo"));
    }

    #[test]
    fn parse_integral() {
        let segs = parse_latex("\\int_0^\\infty e^{-x} dx");
        let text = segments_to_plain(&segs);
        assert!(text.contains("∫"));
        assert!(text.contains("∞"));
    }
}
