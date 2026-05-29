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
  const [diffA, setDiffA] = useState<number | null>(null);
  const [diffB, setDiffB] = useState<number | null>(null);
  const [diffData, setDiffData] = useState<{ added: string[]; removed: string[] } | null>(null);
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

  const handleCompare = async () => {
    if (!template?.id || diffA == null || diffB == null) { return; }
    try {
      // Load both versions and compare node titles + types
      const store = useWorkflowEditorStore.getState();
      try {
        await store.loadTemplateByVersion(template.id, diffA);
        const nodesA = store.nodes;
        await store.loadTemplateByVersion(template.id, diffB);
        const nodesB = store.nodes;
        const aSet = new Set<string>(nodesA.map((n) => `${n.type}:${n.title}`));
        const bSet = new Set<string>(nodesB.map((n) => `${n.type}:${n.title}`));
        const addedList: string[] = [];
        const removedList: string[] = [];
        for (const x of bSet) { if (!aSet.has(x)) { addedList.push(x); } }
        for (const x of aSet) { if (!bSet.has(x)) { removedList.push(x); } }
        setDiffData({ added: addedList, removed: removedList });
      } catch {
        setDiffData({ added: [], removed: [] });
      }
      setDiffA(null);
      setDiffB(null);
    } catch { /* diff failed, ignore */ }
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
      {/* Version diff */}
      {versions.length >= 2 && (
        <div style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 12 }}>
          <div style={{ fontWeight: 500, fontSize: 12, marginBottom: 8 }}>
            {t("workflow.versionHistory.compareVersions")}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
            <Tag
              color="blue"
              style={{ cursor: "pointer", opacity: diffA != null ? 1 : 0.5 }}
              onClick={() =>
                setDiffA(diffA == null ? versions[0] : null)}
            >
              {diffA != null ? `v${diffA}` : "A"}
            </Tag>
            <span style={{ color: token.colorTextQuaternary }}>vs</span>
            <Tag
              color="orange"
              style={{ cursor: "pointer", opacity: diffB != null ? 1 : 0.5 }}
              onClick={() => setDiffB(diffB == null ? versions[versions.length - 1] : null)}
            >
              {diffB != null ? `v${diffB}` : "B"}
            </Tag>
            <Button size="small" onClick={handleCompare} disabled={diffA == null || diffB == null}>
              {t("workflow.versionHistory.compare")}
            </Button>
          </div>
          {diffData && (
            <div style={{ fontSize: 12 }}>
              {diffData.added.length > 0 && (
                <div style={{ marginBottom: 4 }}>
                  <span style={{ color: token.colorSuccess }}>+ {diffData.added.length} added</span>
                  {diffData.added.slice(0, 5).map((x, i) => (
                    <div key={i} style={{ color: token.colorSuccess, paddingLeft: 12 }}>+ {x}</div>
                  ))}
                  {diffData.added.length > 5 && (
                    <div style={{ color: token.colorTextQuaternary, paddingLeft: 12 }}>
                      ... and {diffData.added.length - 5} more
                    </div>
                  )}
                </div>
              )}
              {diffData.removed.length > 0 && (
                <div>
                  <span style={{ color: token.colorError }}>- {diffData.removed.length} removed</span>
                  {diffData.removed.slice(0, 5).map((x, i) => (
                    <div key={i} style={{ color: token.colorError, paddingLeft: 12 }}>- {x}</div>
                  ))}
                  {diffData.removed.length > 5 && (
                    <div style={{ color: token.colorTextQuaternary, paddingLeft: 12 }}>
                      ... and {diffData.removed.length - 5} more
                    </div>
                  )}
                </div>
              )}
              {diffData.added.length === 0 && diffData.removed.length === 0 && (
                <div style={{ color: token.colorTextQuaternary }}>{t("workflow.versionHistory.noChanges")}</div>
              )}
            </div>
          )}
        </div>
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
