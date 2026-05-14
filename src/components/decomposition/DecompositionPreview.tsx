import { Alert, Button, Modal, Space, Spin, Steps, Table, Tag, Typography } from "antd";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
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
      title={t("decomposition.title")}
      open={visible}
      onCancel={onClose}
      width={720}
      footer={
        <Space>
          <Button onClick={onClose}>{t("decomposition.cancel")}</Button>
          <Button
            type="primary"
            loading={confirming}
            disabled={!!hasUnresolvedDeps}
            onClick={handleConfirm}
          >
            {t("decomposition.confirmDecompose")}
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
                  { title: t("decomposition.stepParse") },
                  { title: t("decomposition.stepPreview") },
                  { title: t("decomposition.stepComplete") },
                ]}
                style={{ marginBottom: 24 }}
              />

              {hasUnresolvedDeps && (
                <Alert
                  type="warning"
                  showIcon
                  style={{ marginBottom: 16 }}
                  message={t("decomposition.unresolvedDeps")}
                />
              )}

              <Typography.Title level={5}>{t("decomposition.workflowSteps")}</Typography.Title>
              <Table
                dataSource={[]}
                columns={[
                  { title: t("decomposition.colName"), dataIndex: "name", key: "name" },
                  {
                    title: t("decomposition.colDescription"),
                    dataIndex: "description",
                    key: "description",
                    ellipsis: true,
                  },
                  {
                    title: t("decomposition.colEntryType"),
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
                  <Typography.Title level={5}>
                    {t("decomposition.toolDependencies")} ({preview.tool_dependencies.length})
                  </Typography.Title>
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
                      <Typography.Title level={5} style={{ marginTop: 0 }}>
                        {t("decomposition.processing")} {activeDep.name}
                      </Typography.Title>
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

              <Typography.Title level={5}>{t("decomposition.sourceInfo")}</Typography.Title>
              <Paragraph type="secondary">
                {t("decomposition.market")}: {preview.original_source.market}
                {preview.original_source.repo && ` | ${t("decomposition.repo")}: ${preview.original_source.repo}`}
                {preview.original_source.version
                  && ` | ${t("decomposition.version")}: ${preview.original_source.version}`}
              </Paragraph>
            </div>
          )
          : (
            <div style={{ textAlign: "center", padding: "40px 0" }}>
              <Text type="secondary">{t("decomposition.parsing")}</Text>
            </div>
          )}
      </Spin>
    </Modal>
  );
};
