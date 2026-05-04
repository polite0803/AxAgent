import { Alert, Button, Modal, Space, Spin, Steps, Table, Tag, Typography } from "antd";
import React, { useState } from "react";
import { useDecompositionStore } from "../../stores/feature/decompositionStore";
import type { ToolDependency } from "../../types";
import { ToolDependencyList } from "./ToolDependencyList";
import { ToolGenerationPreview } from "./ToolGenerationPreview";
import { ToolInstallPanel } from "./ToolInstallPanel";

const { Text, Paragraph } = Typography;

interface DecompositionPreviewProps {
  visible: boolean;
  request: {
    name: string;
    description: string;
    content: string;
    source: string;
    version?: string;
    repo?: string;
  };
  onClose: () => void;
  onComplete: () => void;
}

export const DecompositionPreview: React.FC<DecompositionPreviewProps> = ({
  visible,
  request,
  onClose,
  onComplete,
}) => {
  const { preview, loading, confirmDecomposition } = useDecompositionStore();
  const [confirming, setConfirming] = useState(false);
  const [activeDep, setActiveDep] = useState<ToolDependency | null>(null);

  const handleDepAction = (dep: ToolDependency) => {
    setActiveDep(dep);
  };

  const handleConfirm = async () => {
    setConfirming(true);
    try {
      await confirmDecomposition(request.name, request.description);
      onComplete();
    } finally {
      setConfirming(false);
    }
  };

  const hasUnresolvedDeps = preview?.tool_dependencies.some(
    (d) => d.status !== "satisfied",
  );

  return (
    <Modal
      title="复合技能分解预览"
      open={visible}
      onCancel={onClose}
      width={720}
      footer={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button
            type="primary"
            loading={confirming}
            disabled={!!hasUnresolvedDeps}
            onClick={handleConfirm}
          >
            确认分解
          </Button>
        </Space>
      }
    >
      <Spin spinning={loading}>
        {preview
          ? (
            <div>
              <Steps
                size="small"
                current={1}
                items={[
                  { title: "解析" },
                  { title: "分解预览" },
                  { title: "完成" },
                ]}
                style={{ marginBottom: 24 }}
              />

              {hasUnresolvedDeps && (
                <Alert
                  type="warning"
                  showIcon
                  style={{ marginBottom: 16 }}
                  message="存在未解决的工具依赖，请处理后再确认分解"
                />
              )}

              <Typography.Title level={5}>工作流步骤</Typography.Title>
              <Table
                dataSource={[]}
                columns={[
                  { title: "名称", dataIndex: "name", key: "name" },
                  { title: "描述", dataIndex: "description", key: "description", ellipsis: true },
                  {
                    title: "入口类型",
                    dataIndex: "entry_type",
                    key: "entry_type",
                    width: 90,
                    render: (t: string) => <Tag>{t}</Tag>,
                  },
                ]}
                rowKey="id"
                size="small"
                pagination={false}
                style={{ marginBottom: 16 }}
              />

              {preview.tool_dependencies.length > 0 && (
                <>
                  <Typography.Title level={5}>工具依赖 ({preview.tool_dependencies.length})</Typography.Title>
                  <ToolDependencyList
                    dependencies={preview.tool_dependencies}
                    onAction={handleDepAction}
                  />

                  {activeDep && (
                    <div
                      style={{
                        marginTop: 16,
                        padding: 12,
                        background: "#fafafa",
                        borderRadius: 8,
                        border: "1px solid #d9d9d9",
                      }}
                    >
                      <Typography.Title level={5} style={{ marginTop: 0 }}>处理: {activeDep.name}</Typography.Title>
                      {(activeDep.status === "needs_generation")
                        ? (
                          <>
                            <ToolGenerationPreview dependency={activeDep} />
                            <ToolInstallPanel
                              dependency={activeDep}
                              onComplete={() => setActiveDep(null)}
                            />
                          </>
                        )
                        : (
                          <ToolInstallPanel
                            dependency={activeDep}
                            onComplete={() => setActiveDep(null)}
                          />
                        )}
                    </div>
                  )}
                </>
              )}

              <Typography.Title level={5}>来源信息</Typography.Title>
              <Paragraph type="secondary">
                市场: {preview.original_source.market}
                {preview.original_source.repo && ` | 仓库: ${preview.original_source.repo}`}
                {preview.original_source.version && ` | 版本: ${preview.original_source.version}`}
              </Paragraph>
            </div>
          )
          : (
            <div style={{ textAlign: "center", padding: "40px 0" }}>
              <Text type="secondary">正在解析复合技能...</Text>
            </div>
          )}
      </Spin>
    </Modal>
  );
};
