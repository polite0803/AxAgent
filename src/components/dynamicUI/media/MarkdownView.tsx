// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Typography } from "antd";
import DOMPurify from "dompurify";
import { lazy, Suspense } from "react";

/**
 * Markdown 渲染组件。
 * 优先复用项目现有的 Markdown 渲染组件（NodeRenderer for markstream-react），
 * 如果不可用，降级为纯文本展示。
 *
 * 安全：输入 content 经 DOMPurify 去除所有 HTML 标签，防范 NL2UI / 导入 Schema 引入的存储型 XSS（D-07）。
 */
export const MarkdownView: React.FC<DynamicUIProps> = ({ schema }) => {
  const { content = "", className } = schema.props as {
    content?: string;
    className?: string;
  };

  if (!content) {
    return null;
  }

  // 白名单为空 → 剥离所有 HTML 标签（保留纯文本供 markdown 渲染器安全处理）
  const safeContent = DOMPurify.sanitize(String(content), { ALLOWED_TAGS: [] });

  if (!safeContent) {
    return null;
  }

  return (
    <div
      className={`dynamic-markdown ${className || ""}`}
      style={schema.style as React.CSSProperties}
    >
      <Suspense
        fallback={
          <Typography.Paragraph
            style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
          >
            {safeContent}
          </Typography.Paragraph>
        }
      >
        <LazyMarkdownRenderer content={safeContent} />
      </Suspense>
    </div>
  );
};

/** 延迟加载 Markdown 渲染器 */
const LazyMarkdownRenderer = lazy(
  () =>
    import("markstream-react")
      .then((mod) => {
        const NodeRenderer = mod.NodeRenderer as React.ComponentType<{
          content: string;
        }>;
        return {
          default: ({ content }: { content: string }) => <NodeRenderer content={content} />,
        };
      })
      .catch(() => ({
        default: ({ content }: { content: string }) => (
          <Typography.Paragraph
            style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
          >
            {content}
          </Typography.Paragraph>
        ),
      })),
);
