import { Divider, Input, message, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { EmailNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const EmailPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [messageApi, messageContextHolder] = message.useMessage();
  const n = node as unknown as EmailNode;
  const c = n.config || {
    to: [],
    subject: "",
    body: "",
    smtp_host: "",
    smtp_port: 587,
    smtp_user: "",
    smtp_pass: "",
    output_var: "",
  };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const handleAIGenerateBody = async () => {
    const subject = c.subject || "";
    const result = await aiGenerate({
      systemPrompt:
        "你是一名专业的邮件文案撰写助手。根据用户提供的邮件主题（或简短描述），生成一封结构清晰、语言得体的正式邮件正文。"
        + "只输出邮件正文，不要 Subject/To/署名等任何前缀或 Markdown 标记。如果提供了已有正文，则改写优化它。",
      userPrompt: subject
        ? `Subject: ${subject}\n\n${c.body || ""}`.trim()
        : (c.body || ""),
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    sc("body", result);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleAIGenerateSubject = async () => {
    const body = c.body || "";
    if (!body.trim()) {
      messageApi.warning(t("workflow.aiAssist.failed"));
      return;
    }
    const result = await aiGenerate({
      systemPrompt: "你是一名邮件主题优化助手。根据用户提供的邮件正文，生成一个简洁、明确、长度 < 80 字符的邮件主题。"
        + "只输出主题文本本身，不要任何前缀、解释或 Markdown 标记。",
      userPrompt: body,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    sc("subject", result);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {messageContextHolder}
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.to")}
        </label>
        <Input
          value={c.to.join(", ")}
          onChange={(e) => sc("to", e.target.value.split(",").map((s) => s.trim()))}
          size="small"
          placeholder={t("workflow.props.emailToPlaceholder")}
        />
      </div>
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.subject")}
          </label>
          <AIAssistButton
            labelKey="suggest"
            loading={aiGenerating}
            onClick={handleAIGenerateSubject}
            compact
          />
        </div>
        <Input
          value={c.subject}
          onChange={(e) => sc("subject", e.target.value)}
          size="small"
        />
      </div>
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.body")}
          </label>
          <AIAssistButton
            labelKey="generate"
            loading={aiGenerating}
            onClick={handleAIGenerateBody}
            compact
          />
        </div>
        <Input.TextArea
          value={c.body}
          onChange={(e) => sc("body", e.target.value)}
          rows={5}
          size="small"
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
