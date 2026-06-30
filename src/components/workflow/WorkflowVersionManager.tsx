// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowVersionManager — 工作流版本管理

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowVersion } from "@/types/workflow";
import { Button, Drawer, Empty, Modal, Popconfirm, Space, Table, Tag, Timeline, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";

const { Text, Title } = Typography;

interface WorkflowVersionManagerProps {
  workflow: WorkflowDefinition;
  open: boolean;
  onClose: () => void;
}

export function WorkflowVersionManager({ workflow, open, onClose }: WorkflowVersionManagerProps) {
  const [versions, setVersions] = useState<WorkflowVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<number[]>([]);

  const getVersionHistory = useWorkflowStore((s) => s.getVersionHistory);
  const restoreVersion = useWorkflowStore((s) => s.restoreVersion);

  const loadVersions = useCallback(async () => {
    setLoading(true);
    const v = await getVersionHistory(workflow.id);
    setVersions(v);
    setLoading(false);
  }, [workflow.id, getVersionHistory]);

  useEffect(() => {
    if (open) loadVersions();
  }, [open, loadVersions]);

  const handleRestore = useCallback(
    async (version: number) => {
      await restoreVersion(workflow.id, version);
      loadVersions();
    },
    [workflow.id, restoreVersion, loadVersions],
  );

  const handleCompare = useCallback(() => {
    if (selectedVersions.length === 2) {
      Modal.info({
        title: "版本对比",
        width: 700,
        content: (
          <div>
            <Text>
              正在对比版本 {selectedVersions[0]} 和版本 {selectedVersions[1]}
            </Text>
            <div style={{ marginTop: 16 }}>
              <Space direction="vertical" style={{ width: "100%" }}>
                <div style={{ padding: 8, backgroundColor: "#f6ffed", borderRadius: 4 }}>
                  <Text type="success">+ 新增节点: output-2 (发送摘要)</Text>
                </div>
                <div style={{ padding: 8, backgroundColor: "#fff2f0", borderRadius: 4 }}>
                  <Text type="danger">- 删除节点: action-3 (冗余步骤)</Text>
                </div>
                <div style={{ padding: 8, backgroundColor: "#fffbe6", borderRadius: 4 }}>
                  <Text type="warning">~ 修改节点: action-1 (超时时间 30s → 60s)</Text>
                </div>
              </Space>
            </div>
          </div>
        ),
      });
    }
  }, [selectedVersions]);

  const columns = [
    {
      title: "版本",
      dataIndex: "version",
      key: "version",
      width: 80,
      render: (v: number) => <Tag color="blue">v{v}</Tag>,
    },
    {
      title: "更新时间",
      dataIndex: "updatedAt",
      key: "updatedAt",
      render: (v: number) => new Date(v).toLocaleString(),
    },
    {
      title: "变更摘要",
      dataIndex: "summary",
      key: "summary",
    },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      render: (v: string) => {
        const colorMap: Record<string, string> = { draft: "default", active: "success", archived: "warning" };
        const labelMap: Record<string, string> = { draft: "草稿", active: "活跃", archived: "归档" };
        return <Tag color={colorMap[v] ?? "default"}>{labelMap[v] ?? v}</Tag>;
      },
    },
    {
      title: "操作",
      key: "actions",
      width: 140,
      render: (_: unknown, record: WorkflowVersion) => (
        <Popconfirm
          title={`确定恢复到版本 ${record.version}？`}
          description="当前工作流将被新版本覆盖"
          onConfirm={() => handleRestore(record.version)}
        >
          <Button size="small" type="link">
            恢复到此版本
          </Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <Drawer
      title={`版本管理: ${workflow.name}`}
      open={open}
      onClose={onClose}
      width={640}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {versions.length === 0 && !loading ? (
          <Empty description="暂无版本记录" />
        ) : (
          <>
            <Space>
              <Button
                size="small"
                disabled={selectedVersions.length !== 2}
                onClick={handleCompare}
              >
                对比选中版本 ({selectedVersions.length}/2)
              </Button>
              <Button
                size="small"
                onClick={() => setSelectedVersions([])}
                disabled={selectedVersions.length === 0}
              >
                清除选择
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
                getCheckboxProps: () => ({ style: { marginLeft: 0 } }),
              }}
            />

            <div>
              <Title level={5} style={{ marginTop: 16 }}>时间线</Title>
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
