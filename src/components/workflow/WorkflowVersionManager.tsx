// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowVersionManager — 工作流版本管理

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowVersion } from "@/types";
import { App, Button, Drawer, Empty, Popconfirm, Space, Table, Tag, Timeline, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface WorkflowVersionManagerProps {
  workflow: WorkflowDefinition;
  open: boolean;
  onClose: () => void;
}

export function WorkflowVersionManager({ workflow, open, onClose }: WorkflowVersionManagerProps) {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const [versions, setVersions] = useState<WorkflowVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<number[]>([]);

  const getVersionHistory = useWorkflowStore((s) => s.getVersionHistory);
  const restoreVersion = useWorkflowStore((s) => s.restoreVersion);

  const statusLabelMap: Record<string, string> = useMemo(() => ({
    draft: t("workflow.list.draft"),
    active: t("workflow.list.active"),
    archived: t("workflow.list.archived"),
  }), [t]);

  const loadVersions = useCallback(async () => {
    setLoading(true);
    try {
      const v = await getVersionHistory(workflow.id);
      setVersions(v);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [workflow.id, getVersionHistory, message]);

  useEffect(() => {
    const tid = setTimeout(() => {
      if (open) { loadVersions(); }
    }, 0);
    return () => clearTimeout(tid);
  }, [open, loadVersions]);

  const handleRestore = useCallback(
    async (version: number) => {
      try {
        await restoreVersion(workflow.id, version);
        message.success(t("workflow.version.restoreSuccess", { version }));
        loadVersions();
      } catch (e) {
        message.error(String(e));
      }
    },
    [workflow.id, restoreVersion, loadVersions, message, t],
  );

  const handleCompare = useCallback(() => {
    if (selectedVersions.length === 2) {
      modal.info({
        title: t("workflow.version.version"),
        width: 700,
        content: (
          <div>
            <Text>
              {t("workflow.version.compareHint", { v1: selectedVersions[0], v2: selectedVersions[1] })}
            </Text>
            <div style={{ marginTop: 16 }}>
              <Text type="secondary">{t("workflow.version.diffPlaceholder")}</Text>
            </div>
          </div>
        ),
      });
    }
  }, [selectedVersions, t, modal]);

  const columns = [
    {
      title: t("workflow.version.version"),
      dataIndex: "version",
      key: "version",
      width: 80,
      render: (v: number) => <Tag color="blue">v{v}</Tag>,
    },
    {
      title: t("workflow.version.updateTime"),
      dataIndex: "updatedAt",
      key: "updatedAt",
      render: (v: number) => new Date(v).toLocaleString(),
    },
    {
      title: t("workflow.version.changeSummary"),
      dataIndex: "summary",
      key: "summary",
    },
    {
      title: t("workflow.version.status"),
      dataIndex: "status",
      key: "status",
      render: (v: string) => {
        const colorMap: Record<string, string> = { draft: "default", active: "success", archived: "warning" };
        return <Tag color={colorMap[v] ?? "default"}>{statusLabelMap[v] ?? v}</Tag>;
      },
    },
    {
      title: t("workflow.version.action"),
      key: "actions",
      width: 140,
      render: (_: unknown, record: WorkflowVersion) => (
        <Popconfirm
          title={t("workflow.version.confirmRestore", { version: record.version })}
          description={t("workflow.version.currentWorkflowWillBeOverwritten")}
          onConfirm={() => handleRestore(record.version)}
        >
          <Button size="small" type="link">
            {t("workflow.version.restoreToVersion")}
          </Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <Drawer
      title={t("workflow.version.versionManager", { name: workflow.name })}
      open={open}
      onClose={onClose}
      styles={{ wrapper: { width: 640 } }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {versions.length === 0 && !loading ? <Empty description={t("workflow.version.noVersionHistory")} /> : (
          <>
            <Space>
              <Button
                size="small"
                disabled={selectedVersions.length !== 2}
                onClick={handleCompare}
              >
                {t("workflow.version.compareSelected", { selected: selectedVersions.length })}
              </Button>
              <Button
                size="small"
                onClick={() => setSelectedVersions([])}
                disabled={selectedVersions.length === 0}
              >
                {t("workflow.version.clearSelection")}
              </Button>
            </Space>

            <Table
              dataSource={versions}
              columns={columns}
              rowKey="version"
              size="small"
              loading={loading}
              pagination={false}
              rowSelection={{
                type: "checkbox",
                selectedRowKeys: selectedVersions,
                onChange: (keys) => setSelectedVersions(keys as number[]),
              }}
            />

            <div>
              <Title level={5} style={{ marginTop: 16 }}>{t("workflow.version.timeline")}</Title>
              <Timeline
                items={versions.map((v) => ({
                  color: v.status === "active" ? "green" : v.status === "archived" ? "orange" : "gray",
                  children: (
                    <div>
                      <Text strong>v{v.version}</Text> — <Text type="secondary">{v.summary}</Text>
                      <br />
                      <Text style={{ fontSize: 11 }} type="secondary">
                        {new Date(v.updatedAt).toLocaleString()}
                      </Text>
                    </div>
                  ),
                }))}
              />
            </div>
          </>
        )}
      </div>
    </Drawer>
  );
}
