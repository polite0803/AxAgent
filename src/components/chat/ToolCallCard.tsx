import type { ToolCallState } from "@/types/agent";
import { ThoughtChain, type ThoughtChainItemType } from "@ant-design/x";
import { Alert, Tag, theme, Typography } from "antd";
import { FileEdit, Search, Terminal, Wrench } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

interface ToolCallChainProps {
  toolCalls: ToolCallState[];
}

const statusMap: Record<string, ThoughtChainItemType["status"]> = {
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
  for (const [key, icon] of Object.entries(toolIcons)) {
    if (lower.includes(key)) { return icon; }
  }
  return <Wrench size={14} />;
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

export function ToolCallCard({ toolCalls }: ToolCallChainProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const chainItems: ThoughtChainItemType[] = useMemo(() => {
    return toolCalls.map((tc) => {
      const contentParts: React.ReactNode[] = [];

      // Input details
      if (tc.input && Object.keys(tc.input).length > 0) {
        contentParts.push(
          <details key="input" style={{ margin: 0 }}>
            <summary style={{ fontSize: 12, color: token.colorTextSecondary, cursor: "pointer", userSelect: "none" }}>
              {t("chat.inspector.toolInput")}
            </summary>
            <pre
              style={{
                margin: "4px 0 0",
                padding: 8,
                fontSize: 11,
                fontFamily: "monospace",
                backgroundColor: token.colorBgTextHover,
                borderRadius: token.borderRadius,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {typeof tc.input === 'string' ? tc.input : JSON.stringify(tc.input, null, 2)}
            </pre>
          </details>,
        );
      }

      // Output details
      if (tc.output) {
        contentParts.push(
          <details key="output" style={{ margin: 0 }}>
            <summary style={{ fontSize: 12, color: token.colorTextSecondary, cursor: "pointer", userSelect: "none" }}>
              {t("chat.inspector.toolOutput")}
            </summary>
            <div
              style={{
                margin: "4px 0 0",
                padding: 8,
                fontSize: 11,
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
                    style={{ margin: 0, fontSize: 11 }}
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
          <div key="approval" style={{ marginTop: 4, display: "flex", alignItems: "center", gap: 4 }}>
            <Tag
              color={tc.approvalStatus === "approved" ? "green" : tc.approvalStatus === "denied" ? "red" : "orange"}
              style={{ fontSize: 10, padding: "2px 6px" }}
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
                color={tc.approvalStatus === "approved" ? "green" : tc.approvalStatus === "denied" ? "red" : "orange"}
                style={{ fontSize: 10, padding: "2px 4px" }}
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
        collapsible: tc.executionStatus === "success" || tc.executionStatus === "failed"
          || tc.executionStatus === "cancelled",
        content: contentParts.length > 0
          ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {contentParts}
            </div>
          )
          : undefined,
      } satisfies ThoughtChainItemType;
    });
  }, [toolCalls, token, t]);

  if (chainItems.length === 0) { return null; }

  return (
    <div style={{ margin: "8px 0 12px" }}>
      <Typography.Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
        {t("chat.inspector.toolCalls", "工具调用")}
      </Typography.Text>
      <ThoughtChain
        items={chainItems}
        line="dashed"
        styles={{
          item: { padding: "6px 0" },
          itemContent: { fontSize: 12 },
        }}
      />
    </div>
  );
}
