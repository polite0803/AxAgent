import { invoke } from "@/lib/invoke";
import { Button, Card, Form, Input, Select, Switch, Typography } from "antd";
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
  const [form] = Form.useForm<ImageGenConfig>();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);

  useEffect(() => {
    invoke<ImageGenConfig>("get_image_gen_config")
      .then((config) => {
        form.setFieldsValue(config);
      })
      .catch(console.error)
      .finally(() => setInitialLoading(false));
  }, [form]);

  const handleSave = async () => {
    setLoading(true);
    try {
      const values = await form.validateFields();
      await invoke("save_image_gen_config", { config: values });
    } catch (e) {
      console.error("Failed to save image gen config:", e);
    } finally {
      setLoading(false);
    }
  };

  if (initialLoading) {
    return (
      <Card title="图像生成" style={{ marginBottom: 16 }}>
        <Typography.Text>加载中...</Typography.Text>
      </Card>
    );
  }

  return (
    <Card
      title="图像生成"
      style={{ marginBottom: 16 }}
      extra={
        <Button type="primary" onClick={handleSave} loading={loading}>
          保存
        </Button>
      }
    >
      <Form form={form} layout="vertical" initialValues={DEFAULT_CONFIG}>
        <Form.Item name="default_provider" label="默认 Provider">
          <Select
            options={[
              { value: "flux", label: "Flux (Replicate)" },
              { value: "dall-e", label: "DALL-E 3 (OpenAI)" },
            ]}
          />
        </Form.Item>

        <Form.Item name="flux_api_token" label="Replicate API Token">
          <Input.Password placeholder="r8_..." />
        </Form.Item>

        <Form.Item name="openai_api_key" label="OpenAI API Key (DALL-E)">
          <Input.Password placeholder="sk-..." />
        </Form.Item>

        <Form.Item name="openai_base_url" label="OpenAI Base URL">
          <Input placeholder="https://api.openai.com/v1" />
        </Form.Item>

        <Form.Item name="save_to_artifact" label="自动保存为 Artifact" valuePropName="checked">
          <Switch />
        </Form.Item>
      </Form>
    </Card>
  );
}
