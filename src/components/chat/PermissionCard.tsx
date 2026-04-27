import { useAgentStore } from "@/stores";
import { Button, Card, Modal, Space, Tag, theme, Tooltip, Typography } from "antd";
import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Clock,
  Eye,
  Info,
  Pencil,
  Shield,
  ShieldCheck,
  ShieldX,
  Terminal,
} from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type RiskLevel = "read_only" | "write" | "execute";

interface PermissionCardProps {
  conversationId: string;
  toolUseId: string;
  toolName: string;
  input: Record<string, unknown>;
  status: "pending" | "approved" | "denied" | "expired";
  riskLevel?: RiskLevel;
}

const RISK_CONFIG: Record<
  RiskLevel,
  { color: string; icon: React.ReactNode; label: string; tooltip: string; bgColor: string }
> = {
  read_only: {
    color: "blue",
    icon: <Eye size={14} />,
    label: "Read Only",
    tooltip: "This operation only reads data and cannot modify anything",
    bgColor: "rgba(0, 120, 250, 0.1)",
  },
  write: {
    color: "orange",
    icon: <Pencil size={14} />,
    label: "Write",
    tooltip: "This operation may create or modify files in your workspace",
    bgColor: "rgba(250, 173, 20, 0.1)",
  },
  execute: {
    color: "red",
    icon: <Terminal size={14} />,
    label: "Execute",
    tooltip: "This operation executes commands that may have system-wide effects",
    bgColor: "rgba(255, 77, 79, 0.1)",
  },
};

function generateOperationSummary(toolName: string, input: Record<string, unknown>): string {
  const lowerTool = toolName.toLowerCase();

  if (
    lowerTool.includes("read") || lowerTool.includes("grep") || lowerTool.includes("glob") || lowerTool.includes("ls")
  ) {
    if (input.path) { return `Read contents from ${input.path}`; }
    if (input.file) { return `Read file ${input.file}`; }
    return "Read files or directory contents";
  }

  if (lowerTool.includes("write") || lowerTool.includes("edit")) {
    if (input.file) { return `Write to file ${input.file}`; }
    if (input.path) { return `Create or modify ${input.path}`; }
    return "Create or modify files";
  }

  if (lowerTool.includes("bash") || lowerTool.includes("shell") || lowerTool.includes("exec")) {
    if (input.command) { return `Execute command: ${String(input.command).slice(0, 50)}...`; }
    return "Execute shell command";
  }

  if (lowerTool.includes("delete") || lowerTool.includes("remove")) {
    if (input.path) { return `Delete ${input.path}`; }
    return "Delete files or directories";
  }

  if (lowerTool.includes("mkdir") || lowerTool.includes("create_dir")) {
    if (input.path) { return `Create directory ${input.path}`; }
    return "Create directory";
  }

  if (lowerTool.includes("search") || lowerTool.includes("web")) {
    return "Search the web for information";
  }

  if (lowerTool.includes("mcp")) {
    if (input.operation) { return `MCP operation: ${input.operation}`; }
    return "Execute MCP tool";
  }

  return `Execute ${toolName}`;
}

function extractAffectedPaths(input: Record<string, unknown>): string[] {
  const paths: string[] = [];
  const pathFields = ["path", "file", "files", "directory", "dir", "target", "destination", "source"];

  for (const field of pathFields) {
    if (input[field]) {
      const val = input[field];
      if (Array.isArray(val)) {
        paths.push(...val.map(String));
      } else {
        paths.push(String(val));
      }
    }
  }

  return [...new Set(paths)].slice(0, 5);
}

const PermissionCard: React.FC<PermissionCardProps> = ({
  conversationId,
  toolUseId,
  toolName,
  input,
  status,
  riskLevel = "write",
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [expanded, setExpanded] = useState(false);
  const [showDangerConfirm, setShowDangerConfirm] = useState(false);
  const [appeared, setAppeared] = useState(false);
  const approveToolUse = useAgentStore((state) => state.approveToolUse);
  const [loading, setLoading] = useState<string | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => setAppeared(true), 50);
    return () => clearTimeout(timer);
  }, []);

  const riskCfg = RISK_CONFIG[riskLevel];
  const operationSummary = useMemo(() => generateOperationSummary(toolName, input), [toolName, input]);
  const affectedPaths = useMemo(() => extractAffectedPaths(input), [input]);
  const isDangerous = riskLevel === "execute";

  const handleApprove = async (decision: string) => {
    if (isDangerous && decision !== "deny" && !showDangerConfirm) {
      setShowDangerConfirm(true);
      return;
    }
    setShowDangerConfirm(false);
    setLoading(decision);
    try {
      await approveToolUse(conversationId, toolUseId, decision, toolName);
    } catch (e) {
      console.error("[PermissionCard] handleApprove failed:", e);
    } finally {
      setLoading(null);
    }
  };

  const inputStr = JSON.stringify(input, null, 2);

  const borderColor = status === "pending"
    ? isDangerous
      ? token.colorErrorBorder
      : riskLevel === "write"
      ? token.colorWarningBorder
      : token.colorWarningBorder
    : status === "approved"
    ? token.colorSuccessBorder
    : status === "denied"
    ? token.colorErrorBorder
    : token.colorBorderSecondary;

  const cardStyle: React.CSSProperties = {
    margin: "8px 0",
    borderColor,
    borderRadius: 8,
    opacity: appeared ? 1 : 0,
    transform: appeared ? "translateY(0)" : "translateY(-10px)",
    transition: "all 0.3s ease-out",
    boxShadow: status === "pending" && isDangerous ? `0 0 0 1px ${token.colorErrorBorder}` : undefined,
  };

  return (
    <>
      <Card size="small" style={cardStyle}>
        <Space orientation="vertical" style={{ width: "100%" }} size={12}>
          <Space align="start" style={{ width: "100%", justifyContent: "space-between" }}>
            <Space align="center" size={8}>
              <div
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: 8,
                  backgroundColor: riskCfg.bgColor,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: token.colorPrimary,
                }}
              >
                <Shield size={18} />
              </div>
              <div>
                <Text strong style={{ fontSize: 14, display: "block" }}>
                  {t("common.permissionRequired", "Permission Required")}
                </Text>
                <Space size={4} style={{ marginTop: 2 }}>
                  <Tag style={{ margin: 0 }}>{toolName}</Tag>
                  <Tooltip title={riskCfg.tooltip}>
                    <Tag
                      color={riskCfg.color}
                      style={{ display: "inline-flex", alignItems: "center", gap: 4, margin: 0, cursor: "help" }}
                    >
                      {riskCfg.icon}
                      {riskCfg.label}
                    </Tag>
                  </Tooltip>
                </Space>
              </div>
            </Space>
            {status === "pending" && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, color: token.colorTextSecondary }}>
                <Clock size={12} />
                <Text type="secondary" style={{ fontSize: 11 }}>
                  {t("common.waitingForApproval", "Waiting for approval")}
                </Text>
              </div>
            )}
          </Space>

          <div
            style={{
              padding: "10px 12px",
              backgroundColor: riskCfg.bgColor,
              borderRadius: 6,
              borderLeft: `3px solid ${token.colorPrimary}`,
            }}
          >
            <Space size={4} align="start">
              <Info size={14} style={{ color: token.colorPrimary, marginTop: 2, flexShrink: 0 }} />
              <Text style={{ fontSize: 13, lineHeight: 1.5 }}>
                {operationSummary}
              </Text>
            </Space>
          </div>

          {affectedPaths.length > 0 && (
            <div>
              <Text type="secondary" style={{ fontSize: 11 }}>
                {t("common.affectedPaths", "Affected paths")}:
              </Text>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 4 }}>
                {affectedPaths.map((p, i) => (
                  <Tag key={i} style={{ fontSize: 11, fontFamily: "monospace" }}>
                    {p.length > 40 ? p.slice(0, 40) + "..." : p}
                  </Tag>
                ))}
              </div>
            </div>
          )}

          <div
            onClick={() => setExpanded(!expanded)}
            style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}
          >
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("common.viewDetails", "View details")} ({t("common.toolInput", "Tool Input")})
            </Text>
          </div>
          {expanded && (
            <pre
              style={{
                margin: 0,
                padding: 10,
                fontSize: 11,
                fontFamily: "monospace",
                backgroundColor: token.colorBgTextHover,
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {inputStr}
            </pre>
          )}

          {status === "pending"
            ? (
              <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                <Button
                  size="small"
                  type="primary"
                  icon={<ShieldCheck size={14} />}
                  loading={loading === "allow_once"}
                  onClick={() => handleApprove("allow_once")}
                  style={{ borderRadius: 6 }}
                >
                  {t("common.allowOnce", "Allow Once")}
                </Button>
                <Button
                  size="small"
                  icon={<ShieldCheck size={14} />}
                  loading={loading === "allow_always"}
                  onClick={() => handleApprove("allow_always")}
                  style={{ borderRadius: 6 }}
                >
                  {t("common.allowAlways", "Always Allow")}
                </Button>
                <Button
                  size="small"
                  danger
                  icon={<ShieldX size={14} />}
                  loading={loading === "deny"}
                  onClick={() => handleApprove("deny")}
                  style={{ borderRadius: 6 }}
                >
                  {t("common.deny", "Deny")}
                </Button>
                <Tooltip
                  title={t(
                    "common.learnAboutPermissions",
                    'Approving "Allow Once" grants permission for this single instance. "Always Allow" saves this decision for future use.',
                  )}
                >
                  <Info size={14} style={{ color: token.colorTextSecondary, cursor: "help" }} />
                </Tooltip>
              </div>
            )
            : status === "approved"
            ? (
              <Space>
                <ShieldCheck size={16} style={{ color: token.colorSuccess }} />
                <Text type="success" style={{ fontSize: 13 }}>{t("common.approved", "Approved")}</Text>
              </Space>
            )
            : status === "denied"
            ? (
              <Space>
                <ShieldX size={16} style={{ color: token.colorError }} />
                <Text type="danger" style={{ fontSize: 13 }}>{t("common.denied", "Denied")}</Text>
              </Space>
            )
            : (
              <Space>
                <AlertTriangle size={16} style={{ color: token.colorWarning }} />
                <Text type="warning" style={{ fontSize: 13 }}>
                  {t("common.expired", "Expired (Agent disconnected)")}
                </Text>
              </Space>
            )}
        </Space>
      </Card>

      <Modal
        title={
          <Space>
            <AlertTriangle size={18} style={{ color: token.colorError }} />
            <span>{t("common.confirmExecute", "Confirm Execute")}</span>
          </Space>
        }
        open={showDangerConfirm}
        onOk={() => handleApprove("allow_once")}
        onCancel={() => setShowDangerConfirm(false)}
        okText={t("common.allowOnce", "Allow Once")}
        cancelText={t("common.cancel", "Cancel")}
        okButtonProps={{ danger: true }}
      >
        <Space direction="vertical" size={12}>
          <Text>
            {t(
              "common.dangerousOperationWarning",
              "This operation will execute commands that may have system-wide effects:",
            )}
          </Text>
          <ul style={{ margin: 0, paddingLeft: 20 }}>
            <li>
              <Text code>{toolName}</Text>
            </li>
            {affectedPaths.slice(0, 3).map((p, i) => (
              <li key={i}>
                <Text code style={{ fontSize: 12 }}>{p}</Text>
              </li>
            ))}
          </ul>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("common.executionWarning", "This action cannot be undone. Are you sure you want to proceed?")}
          </Text>
        </Space>
      </Modal>
    </>
  );
};

export default PermissionCard;
