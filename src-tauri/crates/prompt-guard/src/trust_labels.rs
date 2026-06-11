// SPDX-License-Identifier: AGPL-3.0-only

/// 外部数据源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    /// RAG 知识库检索结果
    RagKnowledgeBase,
    /// 指令文件 (CLAUDE.md 等)
    InstructionFile,
    /// 网页抓取内容
    WebScrape,
    /// Git 上下文信息
    GitContext,
    /// 其他外部数据
    Other,
}

impl SourceType {
    pub fn label(&self) -> &str {
        match self {
            Self::RagKnowledgeBase => "rag",
            Self::InstructionFile => "instructions",
            Self::WebScrape => "web",
            Self::GitContext => "git",
            Self::Other => "external",
        }
    }

    /// 从 label 字符串反向解析 SourceType（用于 harness trait 的桥接）
    pub fn from_label(label: &str) -> Self {
        match label {
            "rag" => Self::RagKnowledgeBase,
            "instructions" => Self::InstructionFile,
            "web" => Self::WebScrape,
            "git" => Self::GitContext,
            _ => Self::Other,
        }
    }

    pub fn risk_level(&self) -> &str {
        match self {
            Self::RagKnowledgeBase => "medium",
            Self::InstructionFile => "medium",
            Self::WebScrape => "high",
            Self::GitContext => "low",
            Self::Other => "unknown",
        }
    }
}

/// L4: 信任标签生成器
pub struct TrustLabeler;

impl TrustLabeler {
    /// 为外部数据源生成信任前缀标签
    pub fn label(source: SourceType, source_id: &str) -> String {
        format!(
            "[UNTRUSTED-SOURCE:{}/{} risk={}]",
            source.label(),
            source_id,
            source.risk_level()
        )
    }

    /// 包装带标签的外部数据
    pub fn wrap_labeled(source: SourceType, source_id: &str, content: &str) -> String {
        let label = Self::label(source, source_id);
        format!("{label}\n{content}\n[/UNTRUSTED-SOURCE]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_rag_source() {
        let label = TrustLabeler::label(SourceType::RagKnowledgeBase, "kb-main");
        assert!(label.contains("[UNTRUSTED-SOURCE:rag/kb-main"));
        assert!(label.contains("risk=medium"));
    }

    #[test]
    fn labels_web_scrape() {
        let label = TrustLabeler::label(SourceType::WebScrape, "docs.rs");
        assert!(label.contains("risk=high"));
    }

    #[test]
    fn labels_git_context() {
        let label = TrustLabeler::label(SourceType::GitContext, "status");
        assert!(label.contains("risk=low"));
    }

    #[test]
    fn wraps_labeled_content() {
        let wrapped = TrustLabeler::wrap_labeled(
            SourceType::InstructionFile,
            "CLAUDE.md",
            "Project rules here",
        );
        assert!(wrapped.starts_with("[UNTRUSTED-SOURCE:instructions/CLAUDE.md"));
        assert!(wrapped.contains("Project rules here"));
        assert!(wrapped.ends_with("[/UNTRUSTED-SOURCE]"));
    }
}
