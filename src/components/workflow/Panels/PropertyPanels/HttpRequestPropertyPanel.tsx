import { Button, Divider, Input, InputNumber, Select, theme } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import type { HttpRequestNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface HttpRequestPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

const HTTP_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const BODY_TYPES = ["json", "form", "text"];

export const HttpRequestPropertyPanel: React.FC<HttpRequestPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const httpNode = node as HttpRequestNode;
  const config = httpNode.config || {
    url: "", method: "GET", headers: {}, body: undefined,
    body_type: "json", timeout_secs: 30, output_var: "",
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddHeader = () => {
    const key = "header_" + Date.now();
    onUpdate({ config: { ...config, headers: { ...config.headers, [key]: "" } } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.url")}
        </label>
        <Input
          value={config.url}
          onChange={(e) => handleConfigChange("url", e.target.value)}
          size="small"
          placeholder="https://api.example.com/data"
        />
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.method")}
          </label>
          <Select
            value={config.method}
            onChange={(v) => handleConfigChange("method", v)}
            size="small"
            style={{ width: "100%" }}
            options={HTTP_METHODS.map((m) => ({ value: m, label: m }))}
          />
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.timeout")}
          </label>
          <InputNumber
            value={config.timeout_secs}
            onChange={(v) => handleConfigChange("timeout_secs", v ?? 30)}
            size="small"
            style={{ width: "100%" }}
            min={5}
            max={300}
            addonAfter="s"
          />
        </div>
      </div>

      {["POST", "PUT", "PATCH"].includes(config.method) && (
        <>
          <div>
            <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
              {t("workflow.props.bodyType")}
            </label>
            <Select
              value={config.body_type}
              onChange={(v) => handleConfigChange("body_type", v)}
              size="small"
              style={{ width: "100%" }}
              options={BODY_TYPES.map((t) => ({ value: t, label: t }))}
            />
          </div>
          <div>
            <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
              {t("workflow.props.body")}
            </label>
            <Input.TextArea
              value={config.body ?? ""}
              onChange={(e) => handleConfigChange("body", e.target.value || undefined)}
              rows={4}
              size="small"
              placeholder={config.body_type === "json" ? '{"key": "value"}' : "body content"}
            />
          </div>
        </>
      )}

      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.headers")}
          </label>
          <Button type="link" size="small" icon={<Plus size={12} />} onClick={handleAddHeader}>
            {t("workflow.props.addHeader")}
          </Button>
        </div>
        {Object.entries(config.headers || {}).map(([key, value]) => (
          <div key={key} style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
            <Input
              value={key}
              onChange={(e) => {
                const h = { ...config.headers };
                delete h[key];
                h[e.target.value] = value;
                onUpdate({ config: { ...config, headers: h } });
              }}
              size="small"
              style={{ width: 80 }}
              placeholder={t("workflow.props.headerKey")}
            />
            <span style={{ color: token.colorTextTertiary }}>:</span>
            <Input
              value={value}
              onChange={(e) => onUpdate({ config: { ...config, headers: { ...config.headers, [key]: e.target.value } } })}
              size="small"
              style={{ flex: 1 }}
              placeholder={t("workflow.props.headerValue")}
            />
            <Button type="text" danger size="small" icon={<Trash2 size={11} />}
              onClick={() => { const h = { ...config.headers }; delete h[key]; onUpdate({ config: { ...config, headers: h } }); }} />
          </div>
        ))}
      </div>

      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          value={config.output_var}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
