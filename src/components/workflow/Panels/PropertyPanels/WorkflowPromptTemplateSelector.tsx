import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import { Button, Input, Modal, Tag, theme } from "antd";
import { FileText, Search } from "lucide-react";
import type { PromptTemplate } from "@/types";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface Props {
  value?: string;
  onChange: (templateId: string | undefined, content: string) => void;
  placeholder?: string;
}

export const WorkflowPromptTemplateSelector: React.FC<Props> = ({
  value,
  onChange,
  placeholder,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { templates, loadTemplates } = usePromptTemplateStore();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");

  useEffect(() => {
    if (templates.length === 0) { loadTemplates(); }
  }, [templates.length, loadTemplates]);

  const filtered = useMemo(() => {
    if (!search) { return templates; }
    const q = search.toLowerCase();
    return templates.filter((t: PromptTemplate) => t.name?.toLowerCase().includes(q) || t.content?.toLowerCase().includes(q));
  }, [templates, search]);

  const selectedTemplate = templates.find((t: PromptTemplate) => t.id === value);

  return (
    <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
      {selectedTemplate && (
        <Tag style={{ margin: 0, fontSize: 11 }} closable onClose={() => onChange(undefined, "")}>
          <FileText size={11} style={{ marginRight: 2 }} />
          {selectedTemplate.name}
        </Tag>
      )}
      <Button size="small" type="dashed" onClick={() => setOpen(true)} style={{ fontSize: 11 }}>
        {t("workflow.props.selectTemplate")}
      </Button>

      <Modal
        title={t("workflow.props.selectTemplate")}
        open={open}
        onCancel={() => setOpen(false)}
        footer={null}
        width={480}
      >
        <Input
          prefix={<Search size={14} />}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={placeholder ?? t("workflow.props.searchTemplate")}
          size="small"
          style={{ marginBottom: 8 }}
        />
        <div style={{ maxHeight: 300, overflow: "auto", display: "flex", flexDirection: "column", gap: 4 }}>
          {filtered.map((t: PromptTemplate) => (
            <div
              key={t.id}
              onClick={() => {
                onChange(t.id, t.content || "");
                setOpen(false);
              }}
              style={{
                padding: "8px 10px",
                borderRadius: 6,
                cursor: "pointer",
                background: value === t.id ? token.colorPrimaryBg : "transparent",
                border: value === t.id ? "1px solid " + token.colorPrimary : "1px solid transparent",
              }}
            >
              <div style={{ fontSize: 13, fontWeight: 500, color: token.colorText }}>{t.name}</div>
              <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
                {(t.content || "").slice(0, 120)}...
              </div>
            </div>
          ))}
          {filtered.length === 0 && (
            <div style={{ textAlign: "center", color: token.colorTextTertiary, padding: 20, fontSize: 12 }}>
              {t("workflow.props.noTemplates")}
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
};
