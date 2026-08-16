// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { McpServerIcon } from "@/components/shared/McpServerIcon";
import { useConversationStore, useMcpStore, useUIStore } from "@/stores";
import type { CreateMcpServerInput, McpServer } from "@/types";
import { App, Badge, Button, Checkbox, Form, Input, Modal, Popover, Select, theme } from "antd";
import { Plug } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

export function McpMenu() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();
  const navigate = useNavigate();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);

  const mcpServers = useMcpStore((s) => s.servers);
  const createMcpServer = useMcpStore((s) => s.createServer);
  const updateMcpServer = useMcpStore((s) => s.updateServer);
  const enabledMcpServerIds = useConversationStore(
    (s) => s.enabledMcpServerIds,
  );
  const toggleMcpServer = useConversationStore((s) => s.toggleMcpServer);
  const mcpMode = useConversationStore((s) => s.mcpMode);
  const setMcpMode = useConversationStore((s) => s.setMcpMode);

  const [mcpPopoverOpen, setMcpPopoverOpen] = useState(false);
  const [connectorModalOpen, setConnectorModalOpen] = useState(false);
  const [editingMcpServer, setEditingMcpServer] = useState<McpServer | null>(
    null,
  );
  const [mcpForm] = Form.useForm();

  // 连接器 modal 打开时设置初始值
  useEffect(() => {
    if (connectorModalOpen) {
      if (editingMcpServer) {
        mcpForm.setFieldsValue({
          name: editingMcpServer.name,
          transport: editingMcpServer.transport,
          command: editingMcpServer.command || "",
          args: editingMcpServer.argsJson
            ? editingMcpServer.argsJson.split(/\s+/).filter(Boolean).join(" ")
            : "",
          endpoint: editingMcpServer.endpoint || "",
        });
      } else {
        mcpForm.resetFields();
      }
    }
  }, [connectorModalOpen, editingMcpServer, mcpForm]);

  const enabledOverlap = enabledMcpServerIds.filter((id) => mcpServers.some((s) => s.id === id && s.enabled));
  const enabledCount = enabledOverlap.length;

  const popoverContent = useMemo(() => {
    const enabledServers = mcpServers.filter((s) => s.enabled);
    if (enabledServers.length === 0) {
      return (
        <div style={{ padding: "8px 0", minWidth: 220 }}>
          <div
            style={{
              color: token.colorTextSecondary,
              fontSize: 12,
              marginBottom: 8,
            }}
          >
            {t("chat.connector.noServers")}
          </div>
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              setSettingsSection("mcpServers");
              navigate("/settings");
            }}
          >
            {t("chat.connector.goConfig")}
          </Button>
        </div>
      );
    }

    const builtinServers = enabledServers.filter((s) => s.source === "builtin");
    const customServers = enabledServers.filter((s) => s.source === "custom");
    const isManual = mcpMode === "manual";

    const renderGroup = (title: string, servers: typeof mcpServers) => (
      <div key={title}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextSecondary,
            padding: "4px 0",
            fontWeight: 600,
          }}
        >
          {title}
        </div>
        {servers.map((server) => (
          <div key={server.id} style={{ padding: "3px 0" }}>
            <Checkbox
              checked={enabledMcpServerIds.includes(server.id)}
              disabled={!isManual}
              onChange={() => toggleMcpServer(server.id)}
            >
              <span
                style={{
                  fontSize: 13,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <McpServerIcon server={server} size={18} />
                <span>
                  <span style={{ fontWeight: 500 }}>
                    {server.alias || server.name}
                  </span>
                  {server.description && (
                    <span
                      style={{
                        display: "block",
                        fontSize: 12,
                        color: token.colorTextSecondary,
                        lineHeight: "16px",
                      }}
                    >
                      {server.description}
                    </span>
                  )}
                  {server.alias && (
                    <span
                      style={{
                        display: "block",
                        fontSize: 10,
                        color: token.colorTextQuaternary,
                        lineHeight: "14px",
                      }}
                    >
                      {server.name}
                    </span>
                  )}
                </span>
              </span>
            </Checkbox>
          </div>
        ))}
      </div>
    );

    return (
      <div
        style={{
          minWidth: 260,
          maxHeight: 360,
          overflowY: "auto",
          padding: "4px 0",
        }}
      >
        <div
          style={{
            padding: "4px 0 8px",
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
            marginBottom: 8,
          }}
        >
          <div
            style={{
              fontSize: 12,
              color: token.colorTextSecondary,
              marginBottom: 6,
            }}
          >
            {t("chat.mcp.mode")}
          </div>
          <div style={{ display: "flex", gap: 4 }}>
            {(["auto", "manual", "disabled"] as const).map((mode) => (
              <Button
                key={mode}
                size="small"
                type={mcpMode === mode ? "primary" : "default"}
                onClick={() => setMcpMode(mode)}
                style={{ flex: 1, fontSize: 12 }}
              >
                {mode === "auto"
                  ? t("chat.mcp.modeAuto")
                  : mode === "manual"
                  ? t("chat.mcp.modeManual")
                  : t("chat.mcp.modeDisabled")}
              </Button>
            ))}
          </div>
          <div
            style={{
              fontSize: 10,
              color: token.colorTextQuaternary,
              marginTop: 4,
            }}
          >
            {mcpMode === "auto"
              ? t("chat.mcp.modeAutoDesc")
              : mcpMode === "manual"
              ? t("chat.mcp.modeManualDesc")
              : t("chat.mcp.modeDisabledDesc")}
          </div>
        </div>
        {builtinServers.length > 0
          && renderGroup(t("settings.mcp.builtin"), builtinServers)}
        {builtinServers.length > 0 && customServers.length > 0 && (
          <div
            style={{
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              margin: "6px 0",
            }}
          />
        )}
        {customServers.length > 0
          && renderGroup(t("settings.mcp.custom"), customServers)}
        <div
          style={{
            marginTop: 12,
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            paddingTop: 8,
            display: "flex",
            gap: 8,
          }}
        >
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              setEditingMcpServer(null);
              setConnectorModalOpen(true);
            }}
          >
            {t("chat.connector.add")}
          </Button>
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              const customServer = customServers.length > 0
                ? customServers[0]
                : null;
              setEditingMcpServer(customServer);
              setConnectorModalOpen(true);
            }}
          >
            {t("chat.connector.custom")}
          </Button>
        </div>
      </div>
    );
  }, [
    mcpServers,
    enabledMcpServerIds,
    toggleMcpServer,
    mcpMode,
    setMcpMode,
    navigate,
    setSettingsSection,
    token,
    t,
  ]);

  return (
    <>
      <Popover
        trigger="click"
        placement="topLeft"
        content={popoverContent}
        arrow={false}
        open={mcpPopoverOpen}
        onOpenChange={setMcpPopoverOpen}
      >
        <Tooltip
          title={t("chat.connector.title")}
          open={mcpPopoverOpen ? false : undefined}
        >
          <Badge
            count={enabledCount}
            size="small"
            offset={[-4, 4]}
            color={token.colorPrimary}
          >
            <Button
              type="text"
              size="small"
              icon={<Plug size={14} />}
              style={enabledCount > 0
                ? { color: token.colorPrimary }
                : undefined}
            />
          </Badge>
        </Tooltip>
      </Popover>

      <Modal
        title={editingMcpServer
          ? t("chat.connector.custom")
          : t("chat.connector.add")}
        open={connectorModalOpen}
        onCancel={() => setConnectorModalOpen(false)}
        onOk={async () => {
          try {
            const values = await mcpForm.validateFields();
            const input: CreateMcpServerInput = {
              name: values.name,
              transport: values.transport as "stdio" | "http" | "sse",
              command: values.command,
              args: values.args
                ? values.args.split(/\s+/).filter(Boolean)
                : undefined,
              endpoint: values.endpoint,
              enabled: false,
            };
            if (editingMcpServer) {
              await updateMcpServer(editingMcpServer.id, input);
              messageApi.success(t("common.saved"));
            } else {
              await createMcpServer(input);
              messageApi.success(t("common.saved"));
            }
            mcpForm.resetFields();
            setConnectorModalOpen(false);
            setEditingMcpServer(null);
          } catch {
            // validation error, form will show errors
          }
        }}
        destroyOnHidden
      >
        <Form
          form={mcpForm}
          layout="vertical"
          size="small"
          initialValues={{ transport: "stdio" }}
        >
          <Form.Item
            name="name"
            label={t("common.name")}
            rules={[{ required: true }]}
          >
            <Input placeholder={t("chat.connector.placeholderName")} />
          </Form.Item>
          <Form.Item
            name="transport"
            label={t("common.type")}
            rules={[{ required: true }]}
          >
            <Select
              options={[
                { label: "stdio", value: "stdio" },
                { label: "HTTP", value: "http" },
                { label: "SSE", value: "sse" },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="command"
            label={t("chat.connector.command")}
            rules={[{ required: true }]}
          >
            <Input placeholder={t("chat.connector.placeholderCommand")} />
          </Form.Item>
          <Form.Item name="args" label={t("chat.connector.args")}>
            <Input placeholder={t("chat.connector.placeholderArgs")} />
          </Form.Item>
          <Form.Item
            noStyle
            shouldUpdate={(prev, cur) => prev.transport !== cur.transport}
          >
            {({ getFieldValue }) =>
              getFieldValue("transport") !== "stdio" && (
                <Form.Item
                  name="endpoint"
                  label={t("chat.connector.endpoint")}
                  rules={[{ required: true }]}
                >
                  <Input placeholder={t("chat.connector.placeholderEndpoint")} />
                </Form.Item>
              )}
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
