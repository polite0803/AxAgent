// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { lazy, Suspense } from "react";
import { Typography } from "antd";

/**
 * Markdown 渲染组件。
 * 优先复用项目现有的 Markdown 渲染组件（NodeRenderer for markstream-react），
 * 如果不可用，降级为纯文本展示。
 */
export const MarkdownView: React.FC<DynamicUIProps> = ({ schema }) => {
  const { content = "", className } = schema.props as {
    content?: string;
    className?: string;
  };

  if (!content) {
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
            {String(content)}
          </Typography.Paragraph>
        }
      >
        <LazyMarkdownRenderer content={String(content)} />
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
          default: ({ content }: { content: string }) => (
            <NodeRenderer content={content} />
          ),
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

export default MarkdownView;
