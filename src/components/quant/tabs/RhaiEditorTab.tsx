// RhaiEditorTab — Rhai 脚本编辑 + 注册

import { Alert, App, Button, Card, Form, Input, Space, Tag, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useStrategyStore } from "@/stores/feature/quant";
import { DEFAULT_RHAI_TEMPLATE } from "@/types";

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

export function RhaiEditorTab() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const registerRhai = useStrategyStore((s) => s.registerRhai);
  const isRegistering = useStrategyStore((s) => s.isRegistering);

  const [name, setName] = useState("MyRhaiStrategy");
  const [version, setVersion] = useState("1.0.0");
  const [description, setDescription] = useState("");
  const [script, setScript] = useState(DEFAULT_RHAI_TEMPLATE);
  const [upsert, setUpsert] = useState(true);
  const [lastError, setLastError] = useState<string | null>(null);
  const [lastSaved, setLastSaved] = useState<string | null>(null);

  const onSave = async () => {
    setLastError(null);
    setLastSaved(null);
    try {
      const result = await registerRhai({
        name,
        version,
        description: description || null,
        scriptSource: script,
        params: {},
        walkForwardEnabled: true,
        upsert,
      });
      setLastSaved(`${result.name}@${result.version}`);
      void message.success(t("quant.rhai.saved"));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setLastError(msg);
    }
  };

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Card title={t("quant.rhai.title")} size="small">
        <Paragraph type="secondary">{t("quant.rhai.hint")}</Paragraph>
        <Form layout="vertical" size="small">
          <div style={{ display: "grid", gap: 16, gridTemplateColumns: "1fr 120px" }}>
            <Form.Item label={t("quant.rhai.name")}>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </Form.Item>
            <Form.Item label={t("quant.rhai.version")}>
              <Input value={version} onChange={(e) => setVersion(e.target.value)} />
            </Form.Item>
          </div>
          <Form.Item label="描述">
            <Input
              value={description}
              placeholder="策略说明（可选）"
              onChange={(e) => setDescription(e.target.value)}
            />
          </Form.Item>
          <Form.Item label={t("quant.rhai.source")}>
            <TextArea
              value={script}
              onChange={(e) => setScript(e.target.value)}
              autoSize={{ minRows: 14, maxRows: 30 }}
              style={{ fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 13 }}
            />
          </Form.Item>
          <Space>
            <Button type="primary" onClick={onSave} loading={isRegistering}>
              {t("quant.rhai.save")}
            </Button>
            <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <input
                type="checkbox"
                checked={upsert}
                onChange={(e) => setUpsert(e.target.checked)}
              />
              <Text type="secondary">upsert（同名同版本覆盖）</Text>
            </label>
          </Space>
        </Form>
      </Card>

      {lastError && (
        <Alert
          type="error"
          showIcon
          message={t("quant.rhai.compileError")}
          description={
            <pre style={{ whiteSpace: "pre-wrap", maxHeight: 200, overflow: "auto", margin: 0 }}>
              {lastError}
            </pre>
          }
        />
      )}
      {lastSaved && (
        <Alert
          type="success"
          showIcon
          message={t("quant.rhai.saved")}
          description={
            <Space>
              <Tag color="green">{lastSaved}</Tag>
            </Space>
          }
        />
      )}
    </Space>
  );
}
