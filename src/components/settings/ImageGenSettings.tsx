// SPDX-License-Identifier: AGPL-3.0-only

import { PasteButton } from "@/components/common/PasteButton";
import { invoke, logIpcError } from "@/lib/invoke";
import { Button, Card, Form, Input, Select, Space, Switch, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ImageGenConfig {
  default_provider: string;
  flux_api_token: string;
  openai_api_key: string;
  openai_base_url: string;
  default_width: number;
  default_height: number;
  default_steps: number;
  save_to_artifact: boolean;
}

const DEFAULT_CONFIG: ImageGenConfig = {
  default_provider: "flux",
  flux_api_token: "",
  openai_api_key: "",
  openai_base_url: "https://api.openai.com/v1",
  default_width: 1024,
  default_height: 1024,
  default_steps: 4,
  save_to_artifact: true,
};

export function ImageGenSettings() {
  const { t } = useTranslation();
  const [form] = Form.useForm<ImageGenConfig>();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);

  useEffect(() => {
    invoke<ImageGenConfig>("get_image_gen_config")
      .then((config) => {
        form.setFieldsValue(config);
      })
      .catch(logIpcError("get_image_gen_config"))
      .finally(() => setInitialLoading(false));
  }, [form]);

  const handleSave = async () => {
    setLoading(true);
    try {
      const values = await form.validateFields();
      await invoke("save_image_gen_config", { config: values });
    } catch (e) {
      logIpcError("save_image_gen_config")(e);
    } finally {
      setLoading(false);
    }
  };

  if (initialLoading) {
    return (
      <Card title={t("imageGen.title")} style={{ marginBottom: 16 }}>
        <Typography.Text>{t("imageGen.loading")}</Typography.Text>
      </Card>
    );
  }

  return (
    <Card
      title={t("imageGen.title")}
      style={{ marginBottom: 16 }}
      extra={
        <Button type="primary" onClick={handleSave} loading={loading}>
          {t("common.save")}
        </Button>
      }
    >
      <Form form={form} layout="vertical" initialValues={DEFAULT_CONFIG}>
        <Form.Item
          name="default_provider"
          label={t("imageGen.defaultProvider")}
        >
          <Select
            options={[
              { value: "flux", label: t("imageGen.providerFlux") },
              { value: "dall-e", label: t("imageGen.providerDalle") },
            ]}
          />
        </Form.Item>

        <Form.Item
          name="flux_api_token"
          label={t("imageGen.replicateApiToken")}
        >
          <Space.Compact style={{ width: "100%" }}>
            <Input.Password
              name="flux_api_token"
              placeholder={t("imageGen.replicateApiTokenPlaceholder")}
            />
            <PasteButton onPaste={(text) => form.setFieldValue("flux_api_token", text)} />
          </Space.Compact>
        </Form.Item>

        <Form.Item
          name="openai_api_key"
          label={t("imageGen.openaiApiKeyDalle")}
        >
          <Space.Compact style={{ width: "100%" }}>
            <Input.Password
              name="openai_api_key"
              placeholder={t("imageGen.openaiApiKeyPlaceholder")}
            />
            <PasteButton onPaste={(text) => form.setFieldValue("openai_api_key", text)} />
          </Space.Compact>
        </Form.Item>

        <Form.Item name="openai_base_url" label={t("imageGen.openaiBaseUrl")}>
          <Input
            name="openai_base_url"
            placeholder={t("imageGen.openaiBaseUrlPlaceholder")}
          />
        </Form.Item>

        <Form.Item
          name="save_to_artifact"
          label={t("imageGen.autoSaveArtifact")}
          valuePropName="checked"
        >
          <Switch />
        </Form.Item>
      </Form>
    </Card>
  );
}
