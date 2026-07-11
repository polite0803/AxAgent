// SPDX-License-Identifier: AGPL-3.0-only

import type { CreateMcpServerInput, McpServer, RealtimeConfig } from "@/types";
import { Button, Form, Input, Modal, Select, theme } from "antd";
import type { FormInstance } from "antd";
import { Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ConversationSettingsModal } from "./ConversationSettingsModal";
import { ModelSelector } from "./ModelSelector";
import { VoiceCall } from "./VoiceCall";

export function InputAreaModals(props: {
  settingsOpen: boolean;
  setSettingsOpen: (v: boolean) => void;
  connectorModalOpen: boolean;
  setConnectorModalOpen: (v: boolean) => void;
  editingMcpServer: McpServer | null;
  setEditingMcpServer: (v: McpServer | null) => void;
  mcpForm: FormInstance;
  updateMcpServer: (id: string, input: Partial<CreateMcpServerInput>) => Promise<void>;
  createMcpServer: (input: CreateMcpServerInput) => Promise<McpServer | null>;
  messageApi: { success: (msg: string) => void; error: (msg: string) => void };
  sourceModalOpen: boolean;
  setSourceModalOpen: (v: boolean) => void;
  sourcePopoverContent: React.ReactNode;
  hasRealtimeVoice: boolean;
  voiceCallVisible: boolean;
  setVoiceCallVisible: (v: boolean) => void;
  voiceConfig: RealtimeConfig;
  isDragging: boolean;
  multiModelOpen: boolean;
  setMultiModelOpen: (v: boolean) => void;
  handleMultiModelSelect: (
    models: Array<{ providerId: string; model_id: string }>,
  ) => void;
  companionModels: Array<{ providerId: string; model_id: string }>;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const {
    settingsOpen,
    setSettingsOpen,
    connectorModalOpen,
    setConnectorModalOpen,
    editingMcpServer,
    setEditingMcpServer,
    mcpForm,
    updateMcpServer,
    createMcpServer,
    messageApi,
    sourceModalOpen,
    setSourceModalOpen,
    sourcePopoverContent,
    hasRealtimeVoice,
    voiceCallVisible,
    setVoiceCallVisible,
    voiceConfig,
    isDragging,
    multiModelOpen,
    setMultiModelOpen,
    handleMultiModelSelect,
    companionModels,
  } = props;

  return (
    <>
      <ConversationSettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

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
          initialValues={{
            transport: "stdio",
          }}
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
                  <Input
                    placeholder={t("chat.connector.placeholderEndpoint")}
                  />
                </Form.Item>
              )}
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={t("chat.sources.title")}
        open={sourceModalOpen}
        onCancel={() => setSourceModalOpen(false)}
        footer={
          <Button type="primary" onClick={() => setSourceModalOpen(false)}>
            {t("common.confirm")}
          </Button>
        }
        width={420}
        destroyOnHidden
      >
        {sourcePopoverContent}
      </Modal>

      {hasRealtimeVoice && (
        <VoiceCall
          visible={voiceCallVisible}
          onClose={() => setVoiceCallVisible(false)}
          config={voiceConfig}
        />
      )}

      {/* Drag-and-drop overlay */}
      {isDragging && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            zIndex: "var(--z-modal)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: token.colorBgMask,
            backdropFilter: "blur(4px)",
          }}
        >
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: 12,
              padding: "40px 60px",
              borderRadius: 16,
              border: `2px dashed ${token.colorPrimary}`,
              backgroundColor: token.colorBgElevated,
            }}
          >
            <Upload size={48} style={{ color: token.colorPrimary }} />
            <span
              style={{
                fontSize: 16,
                fontWeight: 500,
                color: token.colorText,
              }}
            >
              {t("chat.dropToAttach")}
            </span>
          </div>
        </div>
      )}

      {/* Multi-model selector (trigger hidden, controlled via multiModelOpen state) */}
      <ModelSelector
        multiSelect
        open={multiModelOpen}
        onOpenChange={setMultiModelOpen}
        onMultiSelect={handleMultiModelSelect}
        defaultSelectedModels={companionModels}
      >
        <span />
      </ModelSelector>
    </>
  );
}
