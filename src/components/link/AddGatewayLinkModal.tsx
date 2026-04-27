import { useGatewayLinkStore } from "@/stores";
import type { CreateGatewayLinkInput, GatewayLinkType } from "@/types";
import { App, Form, Input, Modal, Select, Switch } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface AddGatewayLinkModalProps {
  open: boolean;
  onClose: () => void;
}

const LINK_TYPES: { value: GatewayLinkType; labelKey: string }[] = [
  { value: "openclaw", labelKey: "OpenClaw" },
  { value: "hermes", labelKey: "Hermes" },
  { value: "custom", labelKey: "link.typeCustom" },
];

export function AddGatewayLinkModal({ open, onClose }: AddGatewayLinkModalProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const createLink = useGatewayLinkStore((s) => s.createLink);
  const fetchLinks = useGatewayLinkStore((s) => s.fetchLinks);
  const selectLink = useGatewayLinkStore((s) => s.selectLink);
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      const input: CreateGatewayLinkInput = {
        name: values.name,
        link_type: values.link_type,
        endpoint: values.endpoint.replace(/\/+$/, ""),
        api_key: values.api_key || null,
        auto_sync_models: values.auto_sync_models ?? false,
        auto_sync_skills: values.auto_sync_skills ?? false,
      };
      const link = await createLink(input);
      await fetchLinks();
      selectLink(link.id);
      message.success(t("link.addSuccess"));
      form.resetFields();
      onClose();
    } catch (e) {
      if (e && typeof e === "object" && "errorFields" in e) { return; }
      message.error(t("link.addFailed"));
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    form.resetFields();
    onClose();
  };

  return (
    <Modal
      title={t("link.addGateway")}
      open={open}
      onOk={handleOk}
      onCancel={handleCancel}
      confirmLoading={loading}
      okText={t("common.add")}
      cancelText={t("common.cancel")}
      width={480}
      destroyOnClose
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{ link_type: "openclaw", auto_sync_models: false, auto_sync_skills: false }}
      >
        <Form.Item
          name="name"
          label={t("link.gatewayName")}
          rules={[{ required: true, message: t("link.nameRequired") }]}
        >
          <Input placeholder={t("link.namePlaceholder")} />
        </Form.Item>

        <Form.Item
          name="link_type"
          label={t("link.gatewayType")}
          rules={[{ required: true }]}
        >
          <Select
            options={LINK_TYPES.map((lt) => ({
              value: lt.value,
              label: lt.labelKey.startsWith("link.") ? t(lt.labelKey) : lt.labelKey,
            }))}
          />
        </Form.Item>

        <Form.Item
          name="endpoint"
          label={t("link.endpoint")}
          rules={[{ required: true, message: t("link.endpointRequired") }]}
        >
          <Input placeholder="https://192.168.0.108:18789" />
        </Form.Item>

        <Form.Item name="api_key" label={t("link.apiKey")}>
          <Input.Password placeholder={t("link.apiKeyPlaceholder")} />
        </Form.Item>

        <Form.Item name="auto_sync_models" label={t("link.autoSyncModels")} valuePropName="checked">
          <Switch />
        </Form.Item>

        <Form.Item name="auto_sync_skills" label={t("link.autoSyncSkills")} valuePropName="checked">
          <Switch />
        </Form.Item>
      </Form>
    </Modal>
  );
}
