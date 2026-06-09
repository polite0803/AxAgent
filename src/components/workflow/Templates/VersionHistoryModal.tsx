import { useWorkflowEditorStore } from "@/stores";
import { invoke } from "@/lib/invoke";
import { Button, List, message, Modal, Select, Spin, Tag, theme, Tooltip } from "antd";
import { History, RotateCcw } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowTemplateResponse } from "../types";
import type { WorkflowEdge, WorkflowNode } from "../types";

interface VersionHistoryModalProps {
  visible: boolean;
  template: WorkflowTemplateResponse | null;
  onClose: () => void;
  onLoadVersion: (template: WorkflowTemplateResponse) => void;
}

interface DiffEntry {
  type: "added" | "removed" | "modified";
  label: string;
  detail?: string;
}

/** 对两个版本的节点/边进行差异分析 */
function computeDiff(
  nodesA: WorkflowNode[],
  edgesA: WorkflowEdge[],
  nodesB: WorkflowNode[],
  edgesB: WorkflowEdge[],
): DiffEntry[] {
  const results: DiffEntry[] = [];

  const nodesByA = new Map(nodesA.map((n) => [n.id, n]));
  const nodesByB = new Map(nodesB.map((n) => [n.id, n]));

  // 新增节点
  for (const nb of nodesB) {
    if (!nodesByA.has(nb.id)) {
      results.push({
        type: "added",
        label: `${nb.type}(${nb.title})`,
        detail: "新增节点",
      });
    }
  }

  // 删除节点
  for (const na of nodesA) {
    if (!nodesByB.has(na.id)) {
      results.push({
        type: "removed",
        label: `${na.type}(${na.title})`,
        detail: "删除节点",
      });
    }
  }

  // 修改节点（同 id，不同 type 或 config）
  for (const nb of nodesB) {
    const na = nodesByA.get(nb.id);
    if (!na) continue;
    if (na.type !== nb.type) {
      results.push({
        type: "modified",
        label: `${nb.id}: ${na.type} → ${nb.type}`,
        detail: "节点类型变更",
      });
    } else if (JSON.stringify(na.config) !== JSON.stringify(nb.config)) {
      results.push({
        type: "modified",
        label: `${nb.type}(${nb.title})`,
        detail: "节点配置变更",
      });
    }
  }

  // 边差异
  const edgeKey = (e: WorkflowEdge) => `${e.source}→${e.target}:${e.edge_type}`;
  const edgeSetA = new Set(edgesA.map(edgeKey));
  const edgeSetB = new Set(edgesB.map(edgeKey));

  for (const eb of edgesB) {
    if (!edgeSetA.has(edgeKey(eb))) {
      results.push({
        type: "added",
        label: `${eb.source} → ${eb.target}`,
        detail: `新增边 (${eb.edge_type})`,
      });
    }
  }
  for (const ea of edgesA) {
    if (!edgeSetB.has(edgeKey(ea))) {
      results.push({
        type: "removed",
        label: `${ea.source} → ${ea.target}`,
        detail: `删除边 (${ea.edge_type})`,
      });
    }
  }

  // 统计汇总排序：added → modified → removed
  const order = { added: 0, modified: 1, removed: 2 };
  results.sort((a, b) => order[a.type] - order[b.type]);
  return results;
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
  const [diffVerA, setDiffVerA] = useState<number | null>(null);
  const [diffVerB, setDiffVerB] = useState<number | null>(null);
  const [diffEntries, setDiffEntries] = useState<DiffEntry[] | null>(null);
  const [rollbackVersion, setRollbackVersion] = useState<number | null>(null);
  const [rollingBack, setRollingBack] = useState(false);

  const { loadTemplateVersions, loadTemplateByVersion } = useWorkflowEditorStore();
  const { t } = useTranslation();
  const { token } = theme.useToken();

  useEffect(() => {
    if (visible && template?.id) {
      loadVersions();
    }
  }, [visible, template?.id]);

  const loadVersions = async () => {
    if (!template?.id) return;
    setLoadingVersions(true);
    try {
      const vers = await loadTemplateVersions(template.id);
      setVersions(vers.sort((a, b) => b - a));
    } catch {
      message.error(t("workflow.versionHistory.loadFailed"));
    } finally {
      setLoadingVersions(false);
    }
  };

  const handleLoadVersion = async (version: number) => {
    if (!template?.id) return;
    setLoading(true);
    try {
      await loadTemplateByVersion(template.id, version);
      const versionedTemplate = useWorkflowEditorStore.getState().currentTemplate;
      if (versionedTemplate) {
        onLoadVersion(versionedTemplate);
        message.success(t("workflow.versionHistory.loadedVersion", { version }));
        onClose();
      }
    } catch {
      message.error(t("workflow.versionHistory.loadVersionFailed"));
    } finally {
      setLoading(false);
    }
  };

  const handleCompare = useCallback(async () => {
    if (!template?.id || diffVerA == null || diffVerB == null) return;
    try {
      const store = useWorkflowEditorStore.getState();
      await store.loadTemplateByVersion(template.id, diffVerA);
      const nodesA = [...store.nodes];
      const edgesA = [...store.edges];
      await store.loadTemplateByVersion(template.id, diffVerB);
      const nodesB = [...store.nodes];
      const edgesB = [...store.edges];

      // 恢复到当前版本以保持编辑器状态
      // （不恢复的话，store 会保留 diffB 的数据）
      await store.loadTemplateByVersion(template.id, template.version);

      setDiffEntries(computeDiff(nodesA, edgesA, nodesB, edgesB));
    } catch {
      setDiffEntries([]);
      message.error("对比失败");
    }
  }, [template, diffVerA, diffVerB]);

  /** 回滚到指定版本：用旧版本数据创建新版本 */
  const handleRollback = useCallback(async () => {
    if (!template?.id || rollbackVersion == null) return;
    setRollingBack(true);
    try {
      // 1. 加载旧版本数据
      const store = useWorkflowEditorStore.getState();
      await store.loadTemplateByVersion(template.id, rollbackVersion);
      const oldData = store.currentTemplate;
      if (!oldData) {
        message.error("无法加载旧版本数据");
        setRollingBack(false);
        return;
      }

      // 2. 保存为当前模板的新版本（后端自动创建版本快照 + version++）
      const input = {
        name: oldData.name,
        description: oldData.description,
        icon: oldData.icon,
        tags: oldData.tags,
        trigger_config: oldData.trigger_config,
        nodes: oldData.nodes,
        edges: oldData.edges,
        input_schema: oldData.input_schema,
        output_schema: oldData.output_schema,
        variables: oldData.variables,
        error_config: oldData.error_config,
      };
      await invoke<boolean>("update_workflow_template", { id: template.id, input });

      // 3. 重新加载模板以获取新版本号
      await store.loadTemplate(template.id);
      message.success(t("workflow.versionHistory.rolledBack", { version: rollbackVersion }));
      setRollbackVersion(null);
      // 刷新版本列表
      loadVersions();
    } catch (err) {
      message.error(`回滚失败: ${err}`);
    } finally {
      setRollingBack(false);
    }
  }, [template, rollbackVersion, t]);

  const diffCount = diffEntries
    ? { added: diffEntries.filter((d) => d.type === "added").length,
        modified: diffEntries.filter((d) => d.type === "modified").length,
        removed: diffEntries.filter((d) => d.type === "removed").length }
    : null;

  const maxVersion = Math.max(...versions, template?.version || 0);

  return (
    <Modal
      title={
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <History size={18} />
          <span>{t("workflow.versionHistory.title", { name: template?.name })}</span>
          <Tag color="blue" style={{ marginLeft: 8 }}>v{template?.version}</Tag>
        </div>
      }
      open={visible}
      onCancel={onClose}
      footer={null}
      width={640}
    >
      {/* ── 版本列表 ──────────────────────────────────────── */}
      {loadingVersions
        ? <div style={{ textAlign: "center", padding: 40 }}><Spin /></div>
        : (
          <List
            size="small"
            dataSource={versions}
            locale={{ emptyText: t("workflow.versionHistory.noHistory") }}
            renderItem={(version) => {
              const isCurrent = version === template?.version;
              const isLatest = version === maxVersion;
              return (
                <List.Item
                  actions={[
                    <Tooltip key="rollback" title={t("workflow.versionHistory.rollbackTo", { v: version })}>
                      <Button
                        type="link"
                        size="small"
                        icon={<RotateCcw size={14} />}
                        onClick={() => setRollbackVersion(version)}
                        disabled={loading || isCurrent}
                      >
                        {t("workflow.versionHistory.rollback")}
                      </Button>
                    </Tooltip>,
                    <Button
                      key="load"
                      type="link"
                      size="small"
                      onClick={() => handleLoadVersion(version)}
                      disabled={loading}
                    >
                      {t("workflow.versionHistory.loadThisVersion")}
                    </Button>,
                  ]}
                  style={{
                    background: rollbackVersion === version ? `${token.colorWarningBg}` : undefined,
                  }}
                >
                  <List.Item.Meta
                    title={
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <Tag color={isLatest ? "green" : isCurrent ? "blue" : "default"}>
                          v{version}
                        </Tag>
                        {isCurrent && <Tag color="blue">{t("workflow.versionHistory.currentVersion")}</Tag>}
                        {isLatest && !isCurrent && <Tag color="green">{t("workflow.versionHistory.latest")}</Tag>}
                      </div>
                    }
                    description={t("workflow.versionHistory.version", { version })}
                  />
                </List.Item>
              );
            }}
          />
        )}

      {/* ── 版本对比（仅 ≥2 版本时展示） ──────────────────── */}
      {versions.length >= 2 && (
        <div style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 12 }}>
          <div style={{ fontWeight: 500, fontSize: 12, marginBottom: 8, display: "flex", gap: 8, alignItems: "center" }}>
            <span>{t("workflow.versionHistory.compareVersions")}</span>
            <Select
              size="small"
              placeholder="vA"
              style={{ width: 80 }}
              value={diffVerA}
              onChange={setDiffVerA}
              options={versions.map((v) => ({ value: v, label: `v${v}` }))}
            />
            <span style={{ color: token.colorTextQuaternary }}>vs</span>
            <Select
              size="small"
              placeholder="vB"
              style={{ width: 80 }}
              value={diffVerB}
              onChange={setDiffVerB}
              options={versions.map((v) => ({ value: v, label: `v${v}` }))}
            />
            <Button size="small" onClick={handleCompare} disabled={diffVerA == null || diffVerB == null || diffVerA === diffVerB}>
              {t("workflow.versionHistory.compare")}
            </Button>
            {diffEntries && (
              <span style={{ fontSize: 11, color: token.colorTextQuaternary, marginLeft: 4 }}>
                <span style={{ color: token.colorSuccess }}>+{diffCount?.added}</span>
                {" "}
                <span style={{ color: token.colorWarning }}>~{diffCount?.modified}</span>
                {" "}
                <span style={{ color: token.colorError }}>-{diffCount?.removed}</span>
              </span>
            )}
          </div>

          {/* diff 结果 */}
          {diffEntries && diffEntries.length > 0 && (
            <div
              style={{
                maxHeight: 280,
                overflowY: "auto",
                fontSize: 12,
                border: `1px solid ${token.colorBorderSecondary}`,
                borderRadius: 6,
                padding: 8,
              }}
            >
              {diffEntries.map((entry, i) => {
                const color = entry.type === "added" ? token.colorSuccess
                  : entry.type === "removed" ? token.colorError
                  : token.colorWarning;
                const prefix = entry.type === "added" ? "+" : entry.type === "removed" ? "−" : "~";
                return (
                  <div
                    key={i}
                    style={{
                      padding: "3px 8px",
                      marginBottom: 2,
                      borderRadius: 4,
                      background: `${color}12`,
                      color,
                      display: "flex",
                      gap: 8,
                      alignItems: "center",
                    }}
                  >
                    <span style={{ fontWeight: 700, minWidth: 16 }}>{prefix}</span>
                    <span style={{ flex: 1 }}>{entry.label}</span>
                    {entry.detail && (
                      <span style={{ fontSize: 10, opacity: 0.7, fontStyle: "italic" }}>{entry.detail}</span>
                    )}
                  </div>
                );
              })}
            </div>
          )}
          {diffEntries && diffEntries.length === 0 && (
            <div style={{ color: token.colorTextQuaternary, fontSize: 12 }}>
              {t("workflow.versionHistory.noChanges")}
            </div>
          )}
        </div>
      )}

      {/* ── 回滚确认 ─────────────────────────────────────── */}
      {rollbackVersion != null && (
        <div
          style={{
            marginTop: 12,
            padding: 12,
            background: `${token.colorWarningBg}`,
            borderRadius: 6,
            border: `1px solid ${token.colorWarningBorder}`,
            display: "flex",
            alignItems: "center",
            gap: 12,
          }}
        >
          <span style={{ flex: 1, fontSize: 13 }}>
            {t("workflow.versionHistory.rollbackConfirm", { version: rollbackVersion })}
          </span>
          <Button size="small" onClick={() => setRollbackVersion(null)}>
            {t("cancel")}
          </Button>
          <Button type="primary" danger size="small" loading={rollingBack} onClick={handleRollback}>
            {t("workflow.versionHistory.rollback")}
          </Button>
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
