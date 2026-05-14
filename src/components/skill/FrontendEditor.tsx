import type { SkillCapability, SkillManifest } from "@/types";
import { Button, Input, message, Modal, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface FrontendEditorProps {
  skillName: string;
  sourcePath: string;
  currentManifest?: SkillManifest;
  onSaved: () => void;
}

export function FrontendEditor({ skillName, currentManifest, onSaved }: FrontendEditorProps) {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const [jsonText, setJsonText] = useState(
    currentManifest
      ? JSON.stringify(currentManifest, null, 2)
      : JSON.stringify(
        {
          name: skillName,
          version: "0.1.0",
          description: "",
          capabilities: [],
          permissions: { commands: [], events: [] },
        },
        null,
        2,
      ),
  );
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    try {
      JSON.parse(jsonText);
      setSaving(true);
      const { invoke } = await import("@/lib/invoke");
      await invoke<SkillCapability[]>("skill_set_manifest", {
        name: skillName,
        manifest: JSON.parse(jsonText),
      });
      message.success(t("skillEditor.manifestSaved"));
      setVisible(false);
      onSaved();
    } catch (e) {
      message.error(t("skillEditor.jsonFormatError", { error: String(e) }));
      setSaving(false);
    }
  };

  return (
    <>
      <Button size="small" onClick={() => setVisible(true)}>
        {t("skillEditor.editManifest")}
      </Button>
      <Modal
        title={t("skillEditor.editManifestTitle", { name: skillName })}
        open={visible}
        onCancel={() => setVisible(false)}
        onOk={handleSave}
        confirmLoading={saving}
        width={700}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
      >
        <Typography.Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 12 }}>
          {t("skillEditor.editManifestDesc")}{" "}<code>skill-manifest.json</code>. {t("skillEditor.supportedCapabilities")}: page, panel, toolbar, chatCommand, statusBar, navigation, settings.
        </Typography.Paragraph>
        <Input.TextArea
          id="frontend-editor-input-textarea-62"
          value={jsonText}
          onChange={(e) => setJsonText(e.target.value)}
          rows={20}
          style={{ fontFamily: "monospace", fontSize: 13 }}
        />
      </Modal>
    </>
  );
}
