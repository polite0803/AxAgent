import { useWorkflowEditorStore } from "@/stores";
import { Button, List, message, Modal, Spin, Tag, theme } from "antd";
import { History, RotateCcw } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  const { token } = theme.useToken();

  useEffect(() => {
    if (visible && template?.id) {
      loadVersions();
    }
  }, [visible, template?.id]);

  const loadVersions = async () => {
    if (!template?.id) {
      return;
    }
    setLoadingVersions(true);
    try {
      const vers = await loadTemplateVersions(template.id);
      setVersions(vers.sort((a, b) => b - a));
    } catch (error) {
      message.error(t("workflow.versionHistory.loadFailed"));
    } finally {
      setLoadingVersions(false);
    }
  };

  const handleLoadVersion = async (version: number) => {
    if (!template?.id) {
      return;
    }
    setLoading(true);
    try {
      await loadTemplateByVersion(template.id, version);
      const versionedTemplate = useWorkflowEditorStore.getState().currentTemplate;
      if (versionedTemplate) {
        onLoadVersion(versionedTemplate);
        message.success(
          t("workflow.versionHistory.loadedVersion", { version }),
        );
        onClose();
      }
    } catch (error) {
      message.error(t("workflow.versionHistory.loadVersionFailed"));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <History size={18} />
          <span>
            {t("workflow.versionHistory.title", { name: template?.name })}
          </span>
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
            locale={{ emptyText: t("workflow.versionHistory.noHistory") }}
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
                    {t("workflow.versionHistory.loadThisVersion")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <div
                      style={{ display: "flex", alignItems: "center", gap: 8 }}
                    >
                      <Tag
                        color={version === Math.max(...versions) ? "green" : "default"}
                      >
                        v{version}
                      </Tag>
                      {version === template?.version && (
                        <Tag color="blue">
                          {t("workflow.versionHistory.currentVersion")}
                        </Tag>
                      )}
                    </div>
                  }
                  description={t("workflow.versionHistory.version", { version })}
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
            background: token.colorBgMask,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Spin tip={t("workflow.versionHistory.loading")} />
        </div>
      )}
    </Modal>
  );
};
