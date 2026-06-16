// SPDX-License-Identifier-Identifier: AGPL-3.0-only

import { Divider, Input, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { StorageNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const StoragePropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const sn = node as unknown as StorageNode;
  const config = sn.config || { backend: "sqlite", operation: "insert", input_var: "", collection: "", output_var: "" };

  const setCfg = (key: string, val: unknown) => onUpdate({ config: { ...config, [key]: val } });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.backend")}
        </label>
        <Select
          value={config.backend}
          onChange={(v) => setCfg("backend", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "sqlite", label: "SQLite" },
            { value: "vectorDb", label: t("workflow.props.vectorDb") },
            { value: "fileSystem", label: t("workflow.props.fileSystem") },
          ]}
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.operation")}
        </label>
        <Select
          value={config.operation}
          onChange={(v) => setCfg("operation", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "insert", label: t("workflow.props.insert") },
            { value: "upsert", label: t("workflow.props.upsert") },
            { value: "append", label: t("workflow.props.append") },
          ]}
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.inputVariable")}
        </label>
        <Input
          value={config.input_var ?? ""}
          onChange={(e) => setCfg("input_var", e.target.value || undefined)}
          size="small"
          placeholder={t("workflow.props.inputVariablePlaceholder")}
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.collection")}
        </label>
        <Input
          value={config.collection ?? ""}
          onChange={(e) => setCfg("collection", e.target.value || undefined)}
          size="small"
          placeholder={t("workflow.props.collectionPlaceholder")}
        />
      </div>
      {config.operation === "upsert" && (
        <div>
          <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.keyVariable")}
          </label>
          <Input
            value={config.key_var ?? ""}
            onChange={(e) => setCfg("key_var", e.target.value || undefined)}
            size="small"
            placeholder={t("workflow.props.keyVariablePlaceholder")}
          />
        </div>
      )}
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          value={config.output_var ?? ""}
          onChange={(e) => setCfg("output_var", e.target.value)}
          size="small"
        />
      </div>
      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
