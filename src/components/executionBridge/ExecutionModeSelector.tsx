// SPDX-License-Identifier: AGPL-3.0-only

import { useExecutionBridgeStore } from "@/stores";
import type { ExecutionMode } from "@/types";
import { SafetyOutlined, ThunderboltFilled, ThunderboltOutlined } from "@ant-design/icons";
import { Radio, Space, Spin, Tag } from "antd";
import { useTranslation } from "react-i18next";

interface ExecutionModeSelectorProps {
  /** 是否紧凑模式（用于嵌入其他面板） */
  compact?: boolean;
}

const MODE_OPTIONS: Array<{
  value: ExecutionMode;
  icon: React.ReactNode;
  color: string;
}> = [
  {
    value: "manual",
    icon: <SafetyOutlined />,
    color: "default",
  },
  {
    value: "semi_auto",
    icon: <ThunderboltOutlined />,
    color: "processing",
  },
  {
    value: "full_auto",
    icon: <ThunderboltFilled />,
    color: "error",
  },
];

export function ExecutionModeSelector({ compact = false }: ExecutionModeSelectorProps) {
  const { t } = useTranslation();
  const mode = useExecutionBridgeStore((s) => s.mode);
  const loading = useExecutionBridgeStore((s) => s.loading);
  const setMode = useExecutionBridgeStore((s) => s.setMode);

  const handleModeChange = async (newMode: ExecutionMode) => {
    if (newMode === mode) { return; }
    await setMode(newMode);
  };

  return (
    <Spin spinning={loading} size="small">
      <Radio.Group
        value={mode}
        onChange={(e) => handleModeChange(e.target.value)}
        className={compact ? "!flex !gap-2" : "!flex !flex-col !gap-3"}
      >
        {MODE_OPTIONS.map((option) => (
          <Radio.Button
            key={option.value}
            value={option.value}
            className="!h-auto !py-2 !px-3"
          >
            <Space direction="vertical" size={4} className="text-left">
              <Space size={8}>
                <span
                  className={`text-${
                    option.color === "error" ? "red" : option.color === "processing" ? "blue" : "default"
                  }`}
                >
                  {option.icon}
                </span>
                <span className="font-medium">
                  {t(`executionBridge.mode.${option.value}`)}
                </span>
                <Tag color={option.color} className="!m-0">
                  {option.value === mode
                    ? t("executionBridge.mode.title")
                    : ""}
                </Tag>
              </Space>
              {!compact && (
                <span className="text-xs text-gray-500 ml-6">
                  {t(`executionBridge.mode.${option.value}Desc`)}
                </span>
              )}
            </Space>
          </Radio.Button>
        ))}
      </Radio.Group>
    </Spin>
  );
}
