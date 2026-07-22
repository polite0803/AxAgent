// 文档模板系统：使用 MiniJinja 渲染封面、页眉、页脚、目录等可配置字段。
// 三种格式（PDF/Word/PPTX）共用同一套模板接口，输出由各导出工具自行嵌入。

use serde::Serialize;

/// 默认封面模板：居中显示大标题、副标题、日期
pub const DEFAULT_COVER_TEMPLATE: &str = "{{ title }}\n\n{{ subtitle }}\n\n{{ date }}";

/// 默认页眉模板：右对齐显示文档标题
pub const DEFAULT_HEADER_TEMPLATE: &str = "{{ title }}";

/// 默认页脚模板：居中显示 "页 X / Y"
pub const DEFAULT_FOOTER_TEMPLATE: &str = "页 {{ page_no }} / {{ total_pages }}";

/// 默认目录模板：每行 "第 N 级  标题"
pub const DEFAULT_TOC_TEMPLATE: &str =
    "{% for item in items %}{{ item.indent }}{{ item.title }}\n{% endfor %}";

/// 渲染一个文本模板。返回纯文本（无格式标记），由调用方决定如何呈现。
/// 用户可以传自定义模板字符串覆盖默认；变量名见 `TemplateContext` 字段。
pub fn render_template<T: Serialize>(template: &str, ctx: &T) -> Result<String, String> {
    let env = minijinja::Environment::new();
    let tmpl = env.template_from_str(template).map_err(|e| format!("模板语法错误: {}", e))?;
    let rendered = tmpl.render(ctx).map_err(|e| format!("模板渲染失败: {}", e))?;
    Ok(rendered)
}

/// 渲染封面。用户未传 cover_template 时使用默认。
pub fn render_cover(ctx: &TemplateContext, user_template: Option<&str>) -> Result<String, String> {
    let tmpl = user_template.unwrap_or(DEFAULT_COVER_TEMPLATE);
    render_template(tmpl, ctx)
}

/// 渲染页眉。
pub fn render_header(ctx: &TemplateContext, user_template: Option<&str>) -> Result<String, String> {
    let tmpl = user_template.unwrap_or(DEFAULT_HEADER_TEMPLATE);
    render_template(tmpl, ctx)
}

/// 渲染页脚。
pub fn render_footer(ctx: &TemplateContext, user_template: Option<&str>) -> Result<String, String> {
    let tmpl = user_template.unwrap_or(DEFAULT_FOOTER_TEMPLATE);
    render_template(tmpl, ctx)
}

/// 渲染目录（TOC）：把 Heading 列表按模板输出
pub fn render_toc(items: &[TocItem], user_template: Option<&str>) -> Result<String, String> {
    let tmpl = user_template.unwrap_or(DEFAULT_TOC_TEMPLATE);
    let ctx = TemplateContext {
        title: String::new(),
        subtitle: String::new(),
        date: String::new(),
        author: String::new(),
        page_no: 0,
        total_pages: 0,
        items: items.to_vec(),
    };
    render_template(tmpl, &ctx)
}

/// 从 Markdown 文档中提取标题作为目录项
pub fn extract_toc_from_md(doc: &crate::markdown::MdDocument) -> Vec<TocItem> {
    let mut items = Vec::new();
    for block in &doc.blocks {
        if let crate::markdown::MdBlock::Heading { level, text } = block {
            items.push(TocItem {
                level: *level as u32,
                title: text.clone(),
                indent: "  ".repeat((*level as usize).saturating_sub(1)),
            });
        }
    }
    items
}

/// 模板渲染上下文
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateContext {
    pub title: String,
    pub subtitle: String,
    pub date: String,
    pub author: String,
    pub page_no: u32,
    pub total_pages: u32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<TocItem>,
}

/// 目录项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TocItem {
    pub level: u32,
    pub title: String,
    pub indent: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_cover_default() {
        let ctx = TemplateContext {
            title: "年度报告".to_string(),
            subtitle: "2025 年".to_string(),
            date: "2025-12-31".to_string(),
            author: "张三".to_string(),
            page_no: 0,
            total_pages: 0,
            items: vec![],
        };
        let s = render_cover(&ctx, None).unwrap();
        assert!(s.contains("年度报告"));
        assert!(s.contains("2025 年"));
        assert!(s.contains("2025-12-31"));
    }

    #[test]
    fn render_cover_custom_template() {
        let ctx = TemplateContext {
            title: "T".to_string(),
            subtitle: "S".to_string(),
            date: "D".to_string(),
            author: "A".to_string(),
            page_no: 0,
            total_pages: 0,
            items: vec![],
        };
        let s = render_cover(&ctx, Some("=={{ title }}== by {{ author }}")).unwrap();
        assert_eq!(s, "==T== by A");
    }

    #[test]
    fn render_header_footer_with_pagination() {
        let ctx = TemplateContext {
            title: "Doc".to_string(),
            subtitle: String::new(),
            date: String::new(),
            author: String::new(),
            page_no: 3,
            total_pages: 10,
            items: vec![],
        };
        let h = render_header(&ctx, None).unwrap();
        assert_eq!(h, "Doc");
        let f = render_footer(&ctx, None).unwrap();
        assert_eq!(f, "页 3 / 10");
    }

    #[test]
    fn render_toc_with_loop() {
        let items = vec![
            TocItem { level: 1, title: "第一章".to_string(), indent: String::new() },
            TocItem { level: 2, title: "1.1 节".to_string(), indent: "  ".to_string() },
            TocItem { level: 1, title: "第二章".to_string(), indent: String::new() },
        ];
        let s = render_toc(&items, None).unwrap();
        assert!(s.contains("第一章"));
        assert!(s.contains("  1.1 节"));
        assert!(s.contains("第二章"));
    }

    #[test]
    fn extract_toc_from_md_works() {
        let md = "# 标题 1\n\n## 子节 1.1\n\n# 标题 2\n";
        let doc = crate::markdown::parse_markdown(md);
        let items = extract_toc_from_md(&doc);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "标题 1");
        assert_eq!(items[0].level, 1);
        assert_eq!(items[1].level, 2);
    }

    #[test]
    fn template_syntax_error_returns_err() {
        let ctx = TemplateContext {
            title: "T".to_string(),
            subtitle: String::new(),
            date: String::new(),
            author: String::new(),
            page_no: 0,
            total_pages: 0,
            items: vec![],
        };
        let r = render_cover(&ctx, Some("{{ unclosed"));
        assert!(r.is_err());
    }
}
