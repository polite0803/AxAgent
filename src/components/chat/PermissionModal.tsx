import { useAgentStore } from "@/stores";
import type { PermissionRequestEvent } from "@/types/agent";
import { Button, Modal, Space, Tag, Typography } from "antd";
import {
  CheckCircle,
  Eye,
  Info,
  Pencil,
  Shield,
  ShieldCheck,
  ShieldX,
  Terminal,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useState } from "react";

const { Text, Paragraph } = Typography;

type RiskLevel = "read_only" | "write" | "execute";

const RISK_CONFIG: Record<
  RiskLevel,
  { color: string; icon: React.ReactNode; label: string; bgColor: string }
> = {
  read_only: {
    color: "blue",
    icon: <Eye size={14} />,
    label: "只读",
    bgColor: "rgba(0, 120, 250, 0.1)",
  },
  write: {
    color: "orange",
    icon: <Pencil size={14} />,
    label: "写入",
    bgColor: "rgba(250, 173, 20, 0.1)",
  },
  execute: {
    color: "red",
    icon: <Terminal size={14} />,
    label: "执行",
    bgColor: "rgba(255, 77, 79, 0.1)",
  },
};

/**
 * 从工具名称和输入参数生成操作摘要
 */
function generateOperationSummary(
  toolName: string,
  input: Record<string, unknown>,
): string {
  const lowerTool = toolName.toLowerCase();

  if (
    lowerTool.includes("read")
    || lowerTool.includes("grep")
    || lowerTool.includes("glob")
    || lowerTool.includes("ls")
  ) {
    if (input.path) { return `读取 ${input.path}`; }
    if (input.file) { return `读取文件 ${input.file}`; }
    return "读取文件或目录内容";
  }

  if (lowerTool.includes("write") || lowerTool.includes("edit")) {
    if (input.file_path || input.path) {
      return `写入文件 ${input.file_path || input.path}`;
    }
    return "创建或修改文件";
  }

  if (
    lowerTool.includes("bash")
    || lowerTool.includes("shell")
    || lowerTool.includes("exec")
  ) {
    if (input.command) {
      const cmd = String(input.command).slice(0, 60);
      return `执行命令: ${cmd}${cmd.length >= 60 ? "..." : ""}`;
    }
    return "执行 Shell 命令";
  }

  if (lowerTool.includes("delete") || lowerTool.includes("remove")) {
    if (input.path) { return `删除 ${input.path}`; }
    return "删除文件或目录";
  }

  if (lowerTool.includes("search") || lowerTool.includes("web")) {
    return "搜索网络获取信息";
  }

  if (lowerTool.includes("mcp")) {
    return "执行 MCP 工具";
  }

  return `执行 ${toolName}`;
}

/**
 * 权限审批弹窗
 *
 * 当 agent 需要权限审批时弹出 Modal，显示工具名称、参数和风险等级。
 * 提供"允许"、"拒绝"、"始终允许"三个操作按钮。
 */
export const PermissionModal: React.FC = () => {
  const pendingPermissions = useAgentStore((s) => s.pendingPermissions);
  const approveToolUse = useAgentStore((s) => s.approveToolUse);

  // 当前显示的权限请求（取 pending 列表中的第一个）
  const pendingEntries = useMemo(
    () => Object.entries(pendingPermissions),
    [pendingPermissions],
  );

  const [currentIndex, setCurrentIndex] = useState(0);
  const [loading, setLoading] = useState<string | null>(null);
  const [showDetails, setShowDetails] = useState(false);

  // 当列表变化时重置索引
  useEffect(() => {
    if (pendingEntries.length === 0) {
      setCurrentIndex(0);
      setShowDetails(false);
    } else if (currentIndex >= pendingEntries.length) {
      setCurrentIndex(0);
    }
  }, [pendingEntries.length]);

  const currentEntry =
    pendingEntries.length > 0 && currentIndex < pendingEntries.length
      ? pendingEntries[currentIndex]
      : null;

  const [requestId, permissionRequest]: [string, PermissionRequestEvent] | [null, null] =
    currentEntry ?? [null, null];

  const riskLevel: RiskLevel = permissionRequest?.riskLevel ?? "read_only";
  const riskCfg = RISK_CONFIG[riskLevel];

  const operationSummary = useMemo(() => {
    if (!permissionRequest) { return ""; }
    return generateOperationSummary(
      permissionRequest.toolName,
      permissionRequest.input,
    );
  }, [permissionRequest]);

  const inputJson = useMemo(() => {
    if (!permissionRequest?.input) { return ""; }
    try {
      return JSON.stringify(permissionRequest.input, null, 2);
    } catch {
      return String(permissionRequest.input);
    }
  }, [permissionRequest]);

  const handleDecision = useCallback(
    async (decision: string) => {
      if (!permissionRequest || !requestId) { return; }

      setLoading(decision);
      try {
        await approveToolUse(
          permissionRequest.conversationId,
          requestId,
          decision,
          permissionRequest.toolName,
        );
      } catch (e) {
        console.error("[PermissionModal] 审批失败:", e);
      } finally {
        setLoading(null);
        // 移动到下一个待审批项
      }
    },
    [permissionRequest, requestId, approveToolUse],
  );

  const visible = pendingEntries.length > 0 && permissionRequest != null;

  if (!visible) {
    return null;
  }

  const inputPreview = inputJson.slice(0, 500);

  return (
    <Modal
      title={
        <Space size={8}>
          <Shield size={18} style={{ color: "var(--ant-color-primary)" }} />
          <span>权限审批</span>
          {permissionRequest && (
            <Tag color={riskCfg.color} style={{ display: "inline-flex", alignItems: "center", gap: 4, margin: 0 }}>
              {riskCfg.icon}
              {riskCfg.label}
            </Tag>
          )}
        </Space>
      }
      open={visible}
      closable={false}
      maskClosable={false}
      width={520}
      footer={
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            width: "100%",
          }}
        >
          <div style={{ fontSize: 12, color: "var(--ant-color-text-secondary)" }}>
            {pendingEntries.length > 1
              ? `待审批 ${pendingEntries.length} 项 (当前第 ${currentIndex + 1} 项)`
              : "待审批 1 项"}
          </div>
          <Space size={8}>
            <Button
              danger
              icon={<ShieldX size={14} />}
              loading={loading === "deny"}
              onClick={() => handleDecision("deny")}
            >
              拒绝
            </Button>
            <Button
              icon={<ShieldCheck size={14} />}
              loading={loading === "allow_once"}
              onClick={() => handleDecision("allow_once")}
            >
              允许
            </Button>
            <Button
              type="primary"
              icon={<CheckCircle size={14} />}
              loading={loading === "allow_always"}
              onClick={() => handleDecision("allow_always")}
            >
              始终允许
            </Button>
          </Space>
        </div>
      }
      onCancel={() => handleDecision("deny")}
      destroyOnHidden
    >
      <Space direction="vertical" size={16} style={{ width: "100%" }}>
        {/* 工具名称和风险等级 */}
        <div
          style={{
            padding: "10px 14px",
            backgroundColor: riskCfg.bgColor,
            borderRadius: 8,
            borderLeft: `3px solid var(--ant-color-${riskCfg.color})`,
          }}
        >
          <Space size={4} align="start">
            <Info
              size={14}
              style={{
                color: `var(--ant-color-${riskCfg.color})`,
                marginTop: 2,
                flexShrink: 0,
              }}
            />
            <div>
              <Text strong style={{ fontSize: 14, display: "block" }}>
                {permissionRequest?.toolName ?? "未知工具"}
              </Text>
              <Text type="secondary" style={{ fontSize: 13 }}>
                {operationSummary}
              </Text>
            </div>
          </Space>
        </div>

        {/* 请求原因 */}
        {permissionRequest?.riskLevel && (
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              风险等级：{riskCfg.label}
              {riskLevel === "execute"
                ? " — 此操作可能修改系统状态，请谨慎审批"
                : riskLevel === "write"
                ? " — 此操作将创建或修改文件"
                : " — 此操作仅读取数据，不会修改任何内容"}
            </Text>
          </div>
        )}

        {/* 输入参数 */}
        <div>
          <div
            onClick={() => setShowDetails(!showDetails)}
            style={{
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: 4,
              marginBottom: showDetails ? 8 : 0,
            }}
          >
            <Text type="secondary" style={{ fontSize: 12 }}>
              {showDetails ? "收起参数详情" : "展开参数详情"}
            </Text>
          </div>
          {showDetails && (
            <pre
              style={{
                margin: 0,
                padding: 10,
                fontSize: 12,
                fontFamily: "monospace",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 200,
                overflow: "auto",
                lineHeight: 1.5,
              }}
            >
              {inputJson || "(无参数)"}
            </pre>
          )}
          {!showDetails && inputPreview && (
            <Paragraph
              type="secondary"
              ellipsis={{ rows: 2 }}
              style={{
                fontSize: 12,
                fontFamily: "monospace",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                padding: "6px 10px",
                borderRadius: 6,
                margin: 0,
              }}
            >
              {inputPreview}
            </Paragraph>
          )}
        </div>
      </Space>
    </Modal>
  );
};

export default PermissionModal;
