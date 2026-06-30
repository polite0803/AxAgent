// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Alert, Input } from "antd";
import { lazy, Suspense } from "react";

/**
 * 代码编辑器，基于 Monaco Editor。
 * 如果 Monaco Editor 不可用，降级到 Ant Design Input.TextArea。
 */
export const CodeEditorView: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    language = "plaintext",
    value = "",
    readOnly = false,
    height = "300px",
  } = schema.props as {
    language?: string;
    value?: string;
    readOnly?: boolean;
    height?: string;
  };

  return (
    <div style={schema.style as React.CSSProperties}>
      <Suspense
        fallback={
          <Input.TextArea
            value={value}
            readOnly={readOnly}
            rows={15}
            style={{ fontFamily: "monospace" }}
          />
        }
      >
        <LazyMonacoEditor
          language={language}
          value={value}
          readOnly={readOnly}
          height={height}
        />
      </Suspense>
    </div>
  );
};

/** 延迟加载 Monaco Editor */
const LazyMonacoEditor = lazy(
  () =>
    import("@monaco-editor/react").catch(() => ({
      default: ({
        value,
        readOnly,
        height,
      }: {
        language: string;
        value: string;
        readOnly: boolean;
        height: string;
      }) => (
        <Input.TextArea
          value={value}
          readOnly={readOnly}
          rows={Math.max(5, Math.min(30, parseInt(height) / 20 || 15))}
          style={{ fontFamily: "monospace" }}
        />
      ),
    })) as Promise<{
      default: React.ComponentType<{
        language: string;
        value: string;
        readOnly: boolean;
        height: string;
      }>;
    }>,
);

// 实际 Monaco 编辑器实现（当 @monaco-editor/react 可用时）
// 由 lazy import 自动处理，无需显式实现

export default CodeEditorView;
