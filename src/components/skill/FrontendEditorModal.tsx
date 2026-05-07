import { invoke } from "@/lib/invoke";
import type { SkillManifest } from "@/types";
import { Button, Input, message, Modal, Space, Tabs, Typography } from "antd";
import { Lightbulb } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

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
  const [editorTab, setEditorTab] = useState<"json" | "preview">("json");
  const [jsonText, setJsonText] = useState(formatJson(DEFAULT_MANIFEST));
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
      message.success("智能分析完成");
    } catch (e) {
      message.error(`智能分析失败: ${String(e)}`);
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
      message.success("清单配置已保存");
      onClose();
      onSaved();
    } catch (e) {
      message.error(`保存失败: ${String(e)}`);
      setSaving(false);
    }
  }, [jsonText, skillName, onClose, onSaved]);

  return (
    <Modal
      title={`编辑 Skill 清单 — ${skillName}`}
      open={open}
      onCancel={onClose}
      onOk={handleSave}
      confirmLoading={saving}
      width={700}
      okText="保存"
      cancelText="取消"
      footer={(_, { OkBtn, CancelBtn }) => (
        <Space>
          <Button
            icon={<Lightbulb size={14} />}
            loading={analyzing}
            onClick={handleAnalyze}
          >
            AI 分析生成
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
            label: "JSON 编辑",
            children: (
              <div>
                <Text type="secondary" style={{ fontSize: 12, marginBottom: 8, display: "block" }}>
                  编辑{" "}
                  <code>skill-manifest.json</code>。capability
                  类型：page、panel、toolbar、chatCommand、statusBar、navigation、settings。
                </Text>
                <Input.TextArea
                  value={jsonText}
                  onChange={(e) => handleJsonChange(e.target.value)}
                  rows={18}
                  style={{ fontFamily: "monospace", fontSize: 13 }}
                />
                {jsonError && (
                  <Text type="danger" style={{ fontSize: 12 }}>
                    JSON 错误: {jsonError}
                  </Text>
                )}
              </div>
            ),
          },
          {
            key: "preview",
            label: "预览",
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
