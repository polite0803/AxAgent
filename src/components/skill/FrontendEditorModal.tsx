import { invoke } from "@/lib/invoke";
import type { SkillManifest } from "@/types";
import { Button, Input, message, Modal, Space, Tabs, Typography } from "antd";
import { Lightbulb } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface FrontendEditorModalProps {
  open: boolean;
  skillName: string;
  currentManifest?: SkillManifest;
  onClose: () => void;
  onSaved: () => void;
}

function formatJson(obj: unknown): string {
  return JSON.stringify(obj, null, 2);
}

const DEFAULT_MANIFEST: Partial<SkillManifest> = {
  capabilities: [],
  permissions: { commands: [], events: [] },
};

export function FrontendEditorModal({ open, skillName, currentManifest, onClose, onSaved }: FrontendEditorModalProps) {
  const { t } = useTranslation();
  const [editorTab, setEditorTab] = useState<"json" | "preview">("json");
  const [jsonText, setJsonText] = useState(() => formatJson(DEFAULT_MANIFEST));
  const [jsonError, setJsonError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);

  useEffect(() => {
    if (!open) {
      setAnalyzing(false);
    }
  }, [open]);

  useEffect(() => {
    if (open) {
      const d = currentManifest ? structuredClone(currentManifest) : structuredClone(DEFAULT_MANIFEST);
      setJsonText(formatJson(d));
      setJsonError(null);
      setEditorTab("json");
    }
  }, [open, currentManifest]);

  const handleAnalyze = useCallback(async () => {
    setAnalyzing(true);
    try {
      const result = await invoke<SkillManifest>("skill_analyze_frontend", { name: skillName });
      setJsonText(formatJson(result));
      message.success(t("skill.analyzeSuccess"));
    } catch (e) {
      message.error(t("skill.analyzeError", { error: String(e) }));
    } finally {
      setAnalyzing(false);
    }
  }, [skillName]);

  const handleJsonChange = useCallback((value: string) => {
    setJsonText(value);
    try {
      JSON.parse(value);
      setJsonError(null);
    } catch (e) {
      setJsonError(String(e));
    }
  }, []);

  const handleSave = useCallback(async () => {
    try {
      setSaving(true);
      const finalData = JSON.parse(jsonText);
      await invoke("skill_set_manifest", { name: skillName, manifest: finalData });
      message.success(t("skillEditor.manifestSaved"));
      onClose();
      onSaved();
    } catch (e) {
      message.error(t("skillEditor.saveError", { error: String(e) }));
      setSaving(false);
    }
  }, [jsonText, skillName, onClose, onSaved]);

  return (
    <Modal
      title={t("skillEditor.editManifestTitle", { name: skillName })}
      open={open}
      onCancel={onClose}
      onOk={handleSave}
      confirmLoading={saving}
      width={700}
      okText={t("common.save")}
      cancelText={t("common.cancel")}
      footer={(_, { OkBtn, CancelBtn }) => (
        <Space>
          <Button
            icon={<Lightbulb size={14} />}
            loading={analyzing}
            onClick={handleAnalyze}
          >
            {t("skill.analyzeFrontend")}
          </Button>
          <CancelBtn />
          <OkBtn />
        </Space>
      )}
    >
      <Tabs
        activeKey={editorTab}
        onChange={(k) => setEditorTab(k as "json" | "preview")}
        items={[
          {
            key: "json",
            label: t("skillEditor.jsonTab"),
            children: (
              <div>
                <Text type="secondary" style={{ fontSize: 12, marginBottom: 8, display: "block" }}>
                  {t("skillEditor.manifestHint")}
                </Text>
                <Input.TextArea
                  id="frontend-editor-modal-input-textarea-63"
                  value={jsonText}
                  onChange={(e) => handleJsonChange(e.target.value)}
                  rows={18}
                  style={{ fontFamily: "monospace", fontSize: 13 }}
                />
                {jsonError && (
                  <Text type="danger" style={{ fontSize: 12 }}>
                    {t("skillEditor.jsonFormatError", { error: jsonError })}
                  </Text>
                )}
              </div>
            ),
          },
          {
            key: "preview",
            label: t("skillEditor.previewTab"),
            children: (
              <pre
                style={{
                  maxHeight: 400,
                  overflow: "auto",
                  fontSize: 12,
                  fontFamily: "monospace",
                  background: "var(--color-fill-secondary)",
                  padding: 12,
                  borderRadius: 6,
                }}
              >
                {formatJson(JSON.parse(jsonText || "{}"))}
              </pre>
            ),
          },
        ]}
      />
    </Modal>
  );
}
