// SPDX-License-Identifier: AGPL-3.0-only

import type { ContentBlock } from "@/types";
import { Alert, theme, Typography } from "antd";
import { FileEdit, Search, Terminal, Wrench } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";

interface ToolCallBlockViewProps {
  blocks: ContentBlock[];
}

const toolIcons: Record<string, React.ReactNode> = {
  bash: <Terminal size={14} />,
  write: <FileEdit size={14} />,
  read: <Search size={14} />,
  edit: <FileEdit size={14} />,
  glob: <Search size={14} />,
  grep: <Search size={14} />,
  ls: <Search size={14} />,
  echo: <Terminal size={14} />,
  add: <Terminal size={14} />,
};

function getToolIcon(toolName: string): React.ReactNode {
  const lower = toolName.toLowerCase();
  for (const [key, icon] of Object.entries(toolIcons)) {
    if (lower.includes(key)) { return icon; }
  }
  return <Wrench size={14} />;
}

function getInputSummary(input: string): string {
  if (input.length > 80) {
    return input.slice(0, 80) + "…";
  }
  return input;
}

export const ToolCallBlockView = React.memo(
  function ToolCallBlockView({ blocks }: ToolCallBlockViewProps) {
    const { t } = useTranslation();
    const { token } = theme.useToken();

    // Group tool_use + tool_result by id
    const toolUseBlocks = blocks.filter(
      (b): b is ContentBlock & { type: "tool_use" } => b.type === "tool_use",
    );
    const toolResultBlocks = blocks.filter(
      (b): b is ContentBlock & { type: "tool_result" } => b.type === "tool_result",
    );
    const resultByUseId = new Map(
      toolResultBlocks.map((r) => [r.tool_use_id, r]),
    );

    if (toolUseBlocks.length === 0) { return null; }

    return (
      <div style={{ margin: "8px 0 0" }}>
        <Typography.Text
          type="secondary"
          style={{ fontSize: 12, display: "block", marginBottom: 4 }}
        >
          {t("chat.inspector.toolCalls")}
        </Typography.Text>
        <div className="thought-chain">
          {toolUseBlocks.map((use) => {
            const result = resultByUseId.get(use.id);
            const hasResult = result !== undefined;
            const isError = result?.is_error ?? false;
            const inputDisplay = use.input;

            return (
              <div
                key={use.id}
                className={`tc-item tc-${hasResult ? (isError ? "error" : "success") : "loading"}`}
              >
                <div className="tc-dot" />
                <div className="tc-line" />
                <div className="tc-body">
                  <div className="tc-header">
                    <span className="tc-icon">{getToolIcon(use.name)}</span>
                    <span className="tc-title">
                      <span>{use.name}</span>
                    </span>
                  </div>
                  <div className="tc-desc">
                    <Typography.Text
                      type="secondary"
                      style={{ fontSize: 12, fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)" }}
                      ellipsis
                    >
                      {getInputSummary(inputDisplay)}
                    </Typography.Text>
                  </div>

                  {/* Expandable input detail */}
                  {inputDisplay && (
                    <div className="tc-content">
                      <details style={{ margin: 0 }}>
                        <summary
                          style={{
                            fontSize: 12,
                            color: token.colorTextSecondary,
                            cursor: "pointer",
                            userSelect: "none",
                          }}
                        >
                          {t("chat.inspector.toolInput")}
                        </summary>
                        <pre
                          style={{
                            margin: "4px 0 0",
                            padding: 8,
                            fontSize: 12,
                            fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                            backgroundColor: token.colorBgTextHover,
                            borderRadius: token.borderRadius,
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            maxHeight: 200,
                            overflow: "auto",
                          }}
                        >
                          {inputDisplay}
                        </pre>
                      </details>

                      {/* Expandable output detail */}
                      {hasResult && result && result.output && (
                        <details style={{ margin: "4px 0 0" }}>
                          <summary
                            style={{
                              fontSize: 12,
                              color: token.colorTextSecondary,
                              cursor: "pointer",
                              userSelect: "none",
                            }}
                          >
                            {t("chat.inspector.toolOutput")}
                          </summary>
                          <div
                            style={{
                              margin: "4px 0 0",
                              padding: 8,
                              fontSize: 12,
                              fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                              backgroundColor: token.colorBgTextHover,
                              borderRadius: token.borderRadius,
                              whiteSpace: "pre-wrap",
                              wordBreak: "break-all",
                              maxHeight: 200,
                              overflow: "auto",
                            }}
                          >
                            {isError
                              ? (
                                <Alert
                                  message={t("chat.inspector.toolError")}
                                  description={result!.output}
                                  type="error"
                                  showIcon
                                  style={{ margin: 0, fontSize: 12 }}
                                  banner
                                />
                              )
                              : result!.output}
                          </div>
                        </details>
                      )}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  },
);
