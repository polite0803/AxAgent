// SPDX-License-Identifier: AGPL-3.0-only

import type { NL2SkillResult, SkillDefinition } from "@/types/workflow";
import { BulbOutlined, CheckCircleOutlined, PlayCircleOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { Button, Collapse, List, Progress, Space, Statistic, Tag, Typography, theme } from "antd";
import React from "react";

const { Text, Title } = Typography;

interface NL2SkillResultViewProps {
  result: NL2SkillResult;
  onApply: (skill: SkillDefinition) => void;
  loading?: boolean;
}

export const NL2SkillResultView: React.FC<NL2SkillResultViewProps> = React.memo(
  ({ result, onApply, loading }) => {
    const { token } = theme.useToken();
    const { skill, confidence, suggestions, phases } = result;
    const percent = Math.round(confidence * 100);
    const strokeColor = percent < 60 ? token.colorWarning : percent < 80 ? token.colorPrimary : token.colorSuccess;

    return (
      <div style={{ display: "flex", flexDirection: "column", gap: 16, padding: 16 }}>
        {/* 置信度 + 技能名 */}
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <Progress
            type="circle"
            percent={percent}
            size={80}
            strokeColor={strokeColor}
          />
          <div>
            <Title level={5} style={{ marginBottom: 2 }}>{skill.name}</Title>
            <Text type="secondary">{skill.description}</Text>
          </div>
        </div>

        {/* 技能概要 */}
        <div style={{ display: "flex", gap: 16 }}>
          <Statistic title="触发词" value={skill.triggers.length} suffix="个" />
          <Statistic title="参数" value={skill.parameters.length} suffix="个" />
          <Statistic title="工具" value={skill.tools.length} suffix="个" />
        </div>

        {/* 触发词 */}
        <div>
          <Text strong>触发词：</Text>
          <Space style={{ marginLeft: 8 }}>
            {skill.triggers.map((t) => (
              <Tag key={t} color="blue">{t}</Tag>
            ))}
          </Space>
        </div>

        {/* 参数列表 */}
        <div>
          <Text strong>参数：</Text>
          <List
            size="small"
            dataSource={skill.parameters}
            renderItem={(p) => (
              <List.Item style={{ padding: "4px 0" }}>
                <Space>
                  <Tag color={p.required ? "red" : "default"}>
                    {p.required ? "必填" : "可选"}
                  </Tag>
                  <Text code>{p.name}</Text>
                  <Text type="secondary">({p.type})</Text>
                  <Text>{p.description}</Text>
                </Space>
              </List.Item>
            )}
          />
        </div>

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
                        {p.status === "done" ? (
                          <CheckCircleOutlined style={{ color: token.colorSuccess }} />
                        ) : (
                          <ThunderboltOutlined style={{ color: token.colorPrimary }} />
                        )}
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
          onClick={() => onApply(skill)}
          loading={loading}
        >
          应用此技能
        </Button>
      </div>
    );
  },
);
