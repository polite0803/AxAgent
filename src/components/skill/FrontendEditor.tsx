import { useSkillExtensionStore } from "@/stores";
import type { SkillFrontendExtension } from "@/types";
import { Button, Input, message, Modal, Typography } from "antd";
import { useState } from "react";

interface FrontendEditorProps {
  skillName: string;
  sourcePath: string;
  currentFrontend?: SkillFrontendExtension;
  onSaved: () => void;
}

export function FrontendEditor({ skillName, currentFrontend, onSaved }: FrontendEditorProps) {
  const [visible, setVisible] = useState(false);
  const [jsonText, setJsonText] = useState(
    currentFrontend
      ? JSON.stringify(currentFrontend, null, 2)
      : JSON.stringify(
        {
          navigation: [],
          pages: [],
          commands: [],
          panels: [],
          settingsSections: [],
        },
        null,
        2,
      ),
  );
  const [saving, setSaving] = useState(false);
  const setSkillFrontend = useSkillExtensionStore((s) => s.setSkillFrontend);

  const handleSave = async () => {
    try {
      const parsed = JSON.parse(jsonText);
      setSaving(true);
      await setSkillFrontend(skillName, parsed);
      message.success(`前端扩展配置已保存`);
      setVisible(false);
      onSaved();
    } catch (e) {
      message.error(`JSON 格式错误: ${String(e)}`);
      setSaving(false);
    }
  };

  return (
    <>
      <Button size="small" onClick={() => setVisible(true)}>
        编辑前端扩展
      </Button>
      <Modal
        title={`编辑技能前端扩展 — ${skillName}`}
        open={visible}
        onCancel={() => setVisible(false)}
        onOk={handleSave}
        confirmLoading={saving}
        width={700}
        okText="保存"
        cancelText="取消"
      >
        <Typography.Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 12 }}>
          编辑 <code>skill-manifest.json</code> 中的 <code>frontend</code>{" "}
          字段。支持的扩展类型：navigation（导航）、pages（页面）、commands（命令）、panels（面板）、settingsSections（设置段）。
        </Typography.Paragraph>
        <Input.TextArea
          value={jsonText}
          onChange={(e) => setJsonText(e.target.value)}
          rows={20}
          style={{ fontFamily: "monospace", fontSize: 13 }}
        />
      </Modal>
    </>
  );
}
