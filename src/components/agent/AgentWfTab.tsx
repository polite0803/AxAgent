// SPDX-License-Identifier: AGPL-3.0-only

import type { NL2SkillRequest, NL2SkillResult, NL2UIRequest, NL2UIResult, SkillDefinition } from "@/types/workflow";
import type { UISchema } from "@/types/dynamicUI";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import { NL2SkillResultView } from "@/components/workflow/AIPanel/NL2SkillResultView";
import { NL2UIResultView } from "@/components/workflow/AIPanel/NL2UIResultView";
import { LayoutOutlined, SendOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { Button, Empty, Input, Progress, Select, Space, Tabs, Typography } from "antd";
import { useState } from "react";

const { TextArea } = Input;
const { Text } = Typography;

type GenerationMode = "skill" | "ui";

/**
 * NL 生成标签页 — Phase 4
 *
 * 支持两种生成模式：
 * - NL2Skill：自然语言 → 技能定义（SkillDefinition）
 * - NL2UI：自然语言 → 动态 UI Schema（UISchema）
 */
export function AgentWfTab() {
  const [mode, setMode] = useState<GenerationMode>("skill");
  const [prompt, setPrompt] = useState("");
  const [skillType, setSkillType] = useState<NL2SkillRequest["skillType"]>("chat");
  const [uiType, setUIType] = useState<NL2UIRequest["uiType"]>("dashboard");
  const [isGenerating, setIsGenerating] = useState(false);

  const parseSkillFromNaturalLanguage = useWorkflowStore((s) => s.parseSkillFromNaturalLanguage);
  const parseUIFromNaturalLanguage = useWorkflowStore((s) => s.parseUIFromNaturalLanguage);
  const parseProgress = useWorkflowStore((s) => s.parseProgress);

  const [skillResult, setSkillResult] = useState<NL2SkillResult | null>(null);
  const [uiResult, setUIResult] = useState<NL2UIResult | null>(null);

  const handleGenerate = async () => {
    if (!prompt.trim()) return;
    setIsGenerating(true);
    setSkillResult(null);
    setUIResult(null);

    try {
      if (mode === "skill") {
        const result = await parseSkillFromNaturalLanguage({ prompt, skillType });
        setSkillResult(result);
      } else {
        const result = await parseUIFromNaturalLanguage({ prompt, uiType });
        setUIResult(result);
      }
    } catch (err) {
      console.warn("[AgentWfTab] NL generation failed:", err);
    } finally {
      setIsGenerating(false);
    }
  };

  const handleApplySkill = (skill: SkillDefinition) => {
    console.log("[AgentWfTab] Apply skill:", skill);
  };

  const handleApplyUI = (schema: UISchema) => {
    console.log("[AgentWfTab] Apply UI schema:", schema);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* 模式切换 */}
      <Tabs
        activeKey={mode}
        onChange={(k) => {
          setMode(k as GenerationMode);
          setSkillResult(null);
          setUIResult(null);
        }}
        size="small"
        style={{ paddingLeft: 8, paddingRight: 8 }}
        items={[
          { key: "skill", label: <Space><ThunderboltOutlined />NL2Skill</Space> },
          { key: "ui", label: <Space><LayoutOutlined />NL2UI</Space> },
        ]}
      />

      {/* 输入区 */}
      <div style={{ padding: "0 12px 8px", display: "flex", flexDirection: "column", gap: 8 }}>
        <TextArea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={
            mode === "skill"
              ? "描述你需要的技能，如：创建一个自动回复客服消息的技能，支持多轮对话"
              : "描述你需要的界面，如：一个数据看板，展示请求量、成功率和最近7天趋势"
          }
          rows={3}
          disabled={isGenerating}
        />
        <div style={{ display: "flex", gap: 8 }}>
          {mode === "skill" ? (
            <Select
              value={skillType}
              onChange={setSkillType}
              size="small"
              style={{ width: 100 }}
              options={[
                { label: "对话", value: "chat" },
                { label: "工具", value: "tool" },
                { label: "工作流", value: "workflow" },
                { label: "自动化", value: "automation" },
              ]}
            />
          ) : (
            <Select
              value={uiType}
              onChange={setUIType}
              size="small"
              style={{ width: 110 }}
              options={[
                { label: "仪表盘", value: "dashboard" },
                { label: "表单", value: "form" },
                { label: "设置", value: "settings" },
                { label: "报告", value: "report" },
                { label: "自定义", value: "custom" },
              ]}
            />
          )}
          <Button
            type="primary"
            icon={<SendOutlined />}
            onClick={handleGenerate}
            loading={isGenerating}
            disabled={!prompt.trim()}
            style={{ marginLeft: "auto" }}
          >
            生成
          </Button>
        </div>
      </div>

      {/* 进度条 */}
      {isGenerating && (
        <div style={{ padding: "0 12px 8px" }}>
          <Progress percent={100} size="small" status="active" />
          <Text type="secondary" style={{ fontSize: 12 }}>{parseProgress}</Text>
        </div>
      )}

      {/* 结果区 */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {mode === "skill" && skillResult && (
          <NL2SkillResultView result={skillResult} onApply={handleApplySkill} loading={false} />
        )}
        {mode === "ui" && uiResult && (
          <NL2UIResultView result={uiResult} onApply={handleApplyUI} loading={false} />
        )}
        {!isGenerating && !skillResult && !uiResult && (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 24 }}>
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <span style={{ color: "var(--color-text-secondary)", fontSize: 13 }}>
                  输入描述后点击生成
                </span>
              }
            />
          </div>
        )}
      </div>
    </div>
  );
}
