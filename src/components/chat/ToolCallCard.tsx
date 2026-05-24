import type { ToolCallState } from "@/types";
import { Alert, Tag, theme, Typography } from "antd";
import { FileEdit, Search, Terminal, Wrench } from "lucide-react";
import { useMemo } from "react";
import React from "react";
import { useTranslation } from "react-i18next";
import { extractFileChanges, FileChangeList } from "./DiffViewer";

interface ToolCallChainProps {
  toolCalls: ToolCallState[];
}

interface ChainItem {
  key: string;
  icon: React.ReactNode;
  title: React.ReactNode;
  description: React.ReactNode;
  status: "loading" | "success" | "error" | "abort";
  collapsible: boolean;
  content: React.ReactNode;
}

const statusMap: Record<string, ChainItem["status"]> = {
  queued: "loading",
  running: "loading",
  success: "success",
  failed: "error",
  cancelled: "abort",
};

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
  const entry = Object.entries(toolIcons).find(
    ([key]) => lower.indexOf(key) !== -1,
  );
  return entry ? entry[1] : <Wrench size={14} />;
}

function getInputSummary(input: Record<string, unknown>): string {
  try {
    const inputStr = typeof input === "string" ? input : JSON.stringify(input, null, 2);
    if (inputStr.length > 80) {
      return inputStr.slice(0, 80) + "…";
    }
    return inputStr;
  } catch {
    return String(input);
  }
}

export const ToolCallCard = React.memo(
  function ToolCallCard({ toolCalls }: ToolCallChainProps) {
    const { t } = useTranslation();
    const { token } = theme.useToken();

    const chainItems: ChainItem[] = useMemo(() => {
      return toolCalls.map((tc) => {
        const contentParts: React.ReactNode[] = [];

        // Input details
        if (tc.input && Object.keys(tc.input).length > 0) {
          contentParts.push(
            <details key="input" style={{ margin: 0 }}>
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
                  fontFamily: "monospace",
                  backgroundColor: token.colorBgTextHover,
                  borderRadius: token.borderRadius,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 200,
                  overflow: "auto",
                }}
              >
                {typeof tc.input === "string"
                  ? tc.input
                  : JSON.stringify(tc.input, null, 2)}
              </pre>
            </details>,
          );
        }

        // Output details
        if (tc.output) {
          contentParts.push(
            <details key="output" style={{ margin: 0 }}>
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
                  fontFamily: "monospace",
                  backgroundColor: token.colorBgTextHover,
                  borderRadius: token.borderRadius,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 200,
                  overflow: "auto",
                }}
              >
                {tc.isError
                  ? (
                    <Alert
                      message={t("chat.inspector.toolError")}
                      description={tc.output}
                      type="error"
                      showIcon
                      style={{ margin: 0, fontSize: 12 }}
                      banner
                    />
                  )
                  : (
                    tc.output
                  )}
              </div>
            </details>,
          );
        }

        // Approval status
        if (tc.approvalStatus) {
          contentParts.push(
            <div
              key="approval"
              style={{
                marginTop: 4,
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <Tag
                color={tc.approvalStatus === "approved"
                  ? "green"
                  : tc.approvalStatus === "denied"
                  ? "red"
                  : "orange"}
                style={{ fontSize: 12, padding: "2px 6px" }}
              >
                {t(
                  `chat.inspector.approval${tc.approvalStatus.charAt(0).toUpperCase() + tc.approvalStatus.slice(1)}`,
                  tc.approvalStatus,
                )}
              </Tag>
            </div>,
          );
        }

        return {
          key: tc.toolUseId,
          icon: getToolIcon(tc.toolName),
          title: (
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span>{tc.toolName}</span>
              {tc.approvalStatus && (
                <Tag
                  color={tc.approvalStatus === "approved"
                    ? "green"
                    : tc.approvalStatus === "denied"
                    ? "red"
                    : "orange"}
                  style={{ fontSize: 12, padding: "2px 4px" }}
                >
                  {tc.approvalStatus}
                </Tag>
              )}
            </div>
          ),
          description: (
            <Typography.Text
              type="secondary"
              style={{ fontSize: 12, fontFamily: "monospace" }}
              ellipsis
            >
              {getInputSummary(tc.input)}
            </Typography.Text>
          ),
          status: statusMap[tc.executionStatus] || "loading",
          collapsible: tc.executionStatus === "success"
            || tc.executionStatus === "failed"
            || tc.executionStatus === "cancelled",
          content: contentParts.length > 0
            ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {contentParts}
              </div>
            )
            : undefined,
        } satisfies ChainItem;
      });
    }, [toolCalls, token, t]);

    const fileChanges = useMemo(
      () =>
        extractFileChanges(
          toolCalls.flatMap((tc) =>
            tc.executionStatus === "success"
              ? [{ toolName: tc.toolName, input: tc.input, output: tc.output }]
              : []
          ),
        ),
      [toolCalls],
    );

    if (chainItems.length === 0 && fileChanges.length === 0) {
      return null;
    }

    return (
      <div style={{ margin: "8px 0 12px" }}>
        {chainItems.length > 0 && (
          <>
            <Typography.Text
              type="secondary"
              style={{ fontSize: 12, display: "block", marginBottom: 4 }}
            >
              {t("chat.inspector.toolCalls")}
            </Typography.Text>
            <div className="thought-chain">
              {chainItems.map((item) => (
                <div key={item.key} className={`tc-item tc-${item.status}`}>
                  <div className="tc-dot" />
                  <div className="tc-line" />
                  <div className="tc-body">
                    <div className="tc-header">
                      <span className="tc-icon">{item.icon}</span>
                      <span className="tc-title">{item.title}</span>
                    </div>
                    {item.description && <div className="tc-desc">{item.description}</div>}
                    {item.content && <div className="tc-content">{item.content}</div>}
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
        <FileChangeList changes={fileChanges} />
      </div>
    );
  },
  (prevProps, nextProps) => {
    const prev = prevProps.toolCalls;
    const next = nextProps.toolCalls;
    if (prev.length !== next.length) {
      return false;
    }
    for (let i = 0; i < prev.length; i++) {
      const a = prev[i];
      const b = next[i];
      if (
        a.toolUseId !== b.toolUseId
        || a.toolName !== b.toolName
        || a.executionStatus !== b.executionStatus
        || a.approvalStatus !== b.approvalStatus
        || a.output !== b.output
        || a.isError !== b.isError
      ) {
        return false;
      }
    }
    return true;
  },
);
