// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import type { UISchema } from "@/types/dynamicUI";
import type { NL2UIResult } from "@/types/workflow";
import {
  BulbOutlined,
  CheckCircleOutlined,
  EyeOutlined,
  PlayCircleOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import { Button, Collapse, List, Progress, Space, Statistic, Tag, theme, Typography } from "antd";
import React from "react";

const { Text, Title } = Typography;

interface NL2UIResultViewProps {
  result: NL2UIResult;
  onApply: (schema: UISchema) => void;
  loading?: boolean;
}

export const NL2UIResultView: React.FC<NL2UIResultViewProps> = React.memo(
  ({ result, onApply, loading }) => {
    const { token } = theme.useToken();
    const { schema, confidence, phases, suggestions } = result;
    const percent = Math.round(confidence * 100);
    const strokeColor = percent < 60 ? token.colorWarning : percent < 80 ? token.colorPrimary : token.colorSuccess;
    const componentCount = countComponents(schema);

    return (
      <div style={{ display: "flex", flexDirection: "column", gap: 16, padding: 16 }}>
        {/* 置信度 + 标题 */}
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <Progress
            type="circle"
            percent={percent}
            size={80}
            strokeColor={strokeColor}
          />
          <div>
            <Title level={5} style={{ marginBottom: 2 }}>UI Schema</Title>
            <Space>
              <Tag color="purple">{schema.type}</Tag>
              <Text type="secondary">{componentCount} 个组件</Text>
            </Space>
          </div>
        </div>

        {/* 组件统计 */}
        <div style={{ display: "flex", gap: 16 }}>
          <Statistic title="根类型" value={schema.type} />
          <Statistic title="组件数" value={componentCount} />
          <Statistic title="层级深度" value={maxDepth(schema)} />
        </div>

        {/* 实时预览 */}
        <Collapse
          ghost
          items={[
            {
              key: "preview",
              label: (
                <Space>
                  <EyeOutlined />实时预览
                </Space>
              ),
              children: (
                <div
                  style={{
                    border: `1px solid ${token.colorBorderSecondary}`,
                    borderRadius: token.borderRadius,
                    padding: 12,
                    minHeight: 160,
                    background: token.colorBgLayout,
                    maxHeight: 400,
                    overflow: "auto",
                  }}
                >
                  <DynamicUIRenderer schema={schema} />
                </div>
              ),
            },
          ]}
        />

        {/* 解析阶段 */}
        <Collapse
          ghost
          items={[
            {
              key: "phases",
              label: "解析阶段",
              children: (
                <List
                  size="small"
                  dataSource={phases}
                  renderItem={(p) => (
                    <List.Item style={{ padding: "2px 0" }}>
                      <Space>
                        {p.status === "done"
                          ? <CheckCircleOutlined style={{ color: token.colorSuccess }} />
                          : <ThunderboltOutlined style={{ color: token.colorPrimary }} />}
                        <Text strong>{p.phase}</Text>
                        <Text type="secondary">{p.detail}</Text>
                      </Space>
                    </List.Item>
                  )}
                />
              ),
            },
          ]}
        />

        {/* AI 建议 */}
        <div>
          <Text strong>
            <BulbOutlined /> AI 建议：
          </Text>
          <List
            size="small"
            dataSource={suggestions}
            renderItem={(s) => (
              <List.Item style={{ padding: "2px 0", border: "none" }}>
                <Text type="secondary" style={{ fontSize: 12 }}>{s}</Text>
              </List.Item>
            )}
          />
        </div>

        {/* 应用按钮 */}
        <Button
          type="primary"
          icon={<PlayCircleOutlined />}
          block
          onClick={() => onApply(schema)}
          loading={loading}
        >
          应用此 UI
        </Button>
      </div>
    );
  },
);

// ── 辅助函数 ──

function countComponents(schema: UISchema): number {
  let count = 1;
  if (schema.children) {
    for (const child of schema.children) {
      count += countComponents(child);
    }
  }
  return count;
}

function maxDepth(schema: UISchema, depth = 1): number {
  if (!schema.children || schema.children.length === 0) { return depth; }
  return Math.max(...schema.children.map((c) => maxDepth(c, depth + 1)));
}
