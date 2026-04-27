import NodeRenderer from "markstream-react";
import { memo } from "react";

interface MarkdownPreviewProps {
  content: string;
  isDark?: boolean;
}

export const MarkdownPreview = memo(function MarkdownPreview({
  content,
  isDark = false,
}: MarkdownPreviewProps) {
  return (
    <div
      style={{ padding: 16, overflow: "auto", height: "100%" }}
    >
      <NodeRenderer
        content={content}
        isDark={isDark}
        customId="artifact-preview"
      />
    </div>
  );
});
