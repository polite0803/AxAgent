import { CheckOutlined, DownloadOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { Alert, Button, Space, Typography } from "antd";
import React, { useState } from "react";
import { useDecompositionStore } from "../../stores/feature/decompositionStore";
import type { ToolDependency } from "../../types";

const { Text, Paragraph } = Typography;

interface ToolInstallPanelProps {
  dependency: ToolDependency;
  onComplete?: () => void;
}

export const ToolInstallPanel: React.FC<ToolInstallPanelProps> = ({ dependency, onComplete }) => {
  const { generateMissingTool } = useDecompositionStore();
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);

  const isAutoInstallable = dependency.status === "auto_installable";
  const isManualInstallable = dependency.status === "manual_installable";
  const needsGeneration = dependency.status === "needs_generation";

  const handleAutoInstall = async () => {
    setInstalling(true);
    // Auto-install would call the appropriate MCP/plugin install flow
    // For now, mark as complete
    setInstalling(false);
    setInstalled(true);
    onComplete?.();
  };

  const handleGenerate = async () => {
    setInstalling(true);
    try {
      await generateMissingTool(
        dependency.name,
        dependency.install_instructions || dependency.name,
        { type: "object", properties: {} },
        { type: "object", properties: {} },
      );
      setInstalled(true);
      onComplete?.();
    } finally {
      setInstalling(false);
    }
  };

  if (installed) {
    return <Alert type="success" message={`${dependency.name} 已就绪`} showIcon />;
  }

  return (
    <div style={{ padding: "12px 0" }}>
      <Text strong style={{ display: "block", marginBottom: 8 }}>{dependency.name}</Text>

      {isAutoInstallable && (
        <Space direction="vertical" style={{ width: "100%" }}>
          <Paragraph type="secondary" style={{ fontSize: 12 }}>
            此工具可自动安装，点击下方按钮开始安装。
          </Paragraph>
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            loading={installing}
            onClick={handleAutoInstall}
            size="small"
          >
            自动安装
          </Button>
        </Space>
      )}

      {isManualInstallable && (
        <Space direction="vertical" style={{ width: "100%" }}>
          <Paragraph type="secondary" style={{ fontSize: 12 }}>
            此工具需要手动安装，请按照以下说明操作：
          </Paragraph>
          {dependency.install_instructions && (
            <div
              style={{
                padding: 8,
                background: "#f5f5f5",
                borderRadius: 4,
                fontFamily: "monospace",
                fontSize: 12,
                whiteSpace: "pre-wrap",
              }}
            >
              {dependency.install_instructions}
            </div>
          )}
          <Button
            icon={<CheckOutlined />}
            onClick={() => {
              setInstalled(true);
              onComplete?.();
            }}
            size="small"
          >
            已完成安装
          </Button>
        </Space>
      )}

      {needsGeneration && (
        <Space direction="vertical" style={{ width: "100%" }}>
          <Paragraph type="secondary" style={{ fontSize: 12 }}>
            此工具无法通过安装获取，将使用 Developer Agent 自动生成 Prompt 模板实现。
          </Paragraph>
          <Button
            type="primary"
            icon={<ThunderboltOutlined />}
            loading={installing}
            onClick={handleGenerate}
            size="small"
          >
            生成工具
          </Button>
        </Space>
      )}
    </div>
  );
};
