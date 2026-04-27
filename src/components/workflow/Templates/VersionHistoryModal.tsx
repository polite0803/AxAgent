import { useWorkflowEditorStore } from "@/stores";
import { Button, List, message, Modal, Spin, Tag } from "antd";
import { History, RotateCcw } from "lucide-react";
import React, { useEffect, useState } from "react";
import type { WorkflowTemplateResponse } from "../types";

interface VersionHistoryModalProps {
  visible: boolean;
  template: WorkflowTemplateResponse | null;
  onClose: () => void;
  onLoadVersion: (template: WorkflowTemplateResponse) => void;
}

export const VersionHistoryModal: React.FC<VersionHistoryModalProps> = ({
  visible,
  template,
  onClose,
  onLoadVersion,
}) => {
  const [versions, setVersions] = useState<number[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingVersions, setLoadingVersions] = useState(false);
  const { loadTemplateVersions, loadTemplateByVersion } = useWorkflowEditorStore();

  useEffect(() => {
    if (visible && template?.id) {
      loadVersions();
    }
  }, [visible, template?.id]);

  const loadVersions = async () => {
    if (!template?.id) { return; }
    setLoadingVersions(true);
    try {
      const vers = await loadTemplateVersions(template.id);
      setVersions(vers.sort((a, b) => b - a));
    } catch (error) {
      message.error("加载版本历史失败");
    } finally {
      setLoadingVersions(false);
    }
  };

  const handleLoadVersion = async (version: number) => {
    if (!template?.id) { return; }
    setLoading(true);
    try {
      await loadTemplateByVersion(template.id, version);
      const versionedTemplate = useWorkflowEditorStore.getState().currentTemplate;
      if (versionedTemplate) {
        onLoadVersion(versionedTemplate);
        message.success(`已加载版本 ${version}`);
        onClose();
      }
    } catch (error) {
      message.error("加载版本失败");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <History size={18} />
          <span>版本历史 - {template?.name}</span>
        </div>
      }
      open={visible}
      onCancel={onClose}
      footer={null}
      width={500}
    >
      {loadingVersions
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : (
          <List
            dataSource={versions}
            locale={{ emptyText: "暂无版本历史" }}
            renderItem={(version) => (
              <List.Item
                actions={[
                  <Button
                    key="load"
                    type="link"
                    size="small"
                    icon={<RotateCcw size={14} />}
                    onClick={() => handleLoadVersion(version)}
                    disabled={loading}
                  >
                    加载此版本
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <Tag color={version === Math.max(...versions) ? "green" : "default"}>
                        v{version}
                      </Tag>
                      {version === template?.version && <Tag color="blue">当前版本</Tag>}
                    </div>
                  }
                  description={`版本 ${version}`}
                />
              </List.Item>
            )}
          />
        )}
      {loading && (
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Spin tip="加载版本..." />
        </div>
      )}
    </Modal>
  );
};
