// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { LearnSkillInput, LearnSkillResult, PendingSkillOperation, SkillLearningConfig } from "@/types";
import { App, Button, Divider, Empty, Form, Input, Select, Space, Switch, Tag, Tooltip } from "antd";
import { CheckCircle2, GraduationCap, ListTodo, Settings2, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 技能学习闭环面板：/learn 生成 + 待审批操作 + 学习配置（借鉴 Hermes Agent） */
export function SkillLearningPanel() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [learnForm] = Form.useForm();
  const [tab, setTab] = useState<"learn" | "pending" | "config">("learn");

  const [pendingOps, setPendingOps] = useState<PendingSkillOperation[]>([]);
  const [config, setConfig] = useState<SkillLearningConfig | null>(null);
  const [learning, setLearning] = useState(false);

  async function loadPending() {
    try {
      const ops = await invoke<PendingSkillOperation[]>("get_pending_skill_operations");
      setPendingOps(ops);
    } catch (e) {
      message.error(String(e));
    }
  }

  async function loadConfig() {
    try {
      const cfg = await invoke<SkillLearningConfig>("get_skill_learning_config");
      setConfig(cfg);
    } catch (e) {
      message.error(String(e));
    }
  }

  useEffect(() => {
    if (tab === "pending") {
      loadPending();
    }
    if (tab === "config") {
      loadConfig();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab]);

  async function handleLearn(
    values: { name?: string; description?: string; sourceType: string; content: string; autoApprove?: boolean },
  ) {
    setLearning(true);
    try {
      const input: LearnSkillInput = {
        name: values.name,
        description: values.description,
        sourceType: values.sourceType,
        content: values.content,
        autoApprove: values.autoApprove,
      };
      const result = await invoke<LearnSkillResult>("learn_skill", { input });
      if (result.requiresApproval) {
        message.success(t("skillLearning.learnSubmitted", { id: result.operationId ?? "" }));
      } else {
        message.success(t("skillLearning.learnCreated", { name: result.skillName }));
      }
      learnForm.resetFields();
    } catch (e) {
      message.error(String(e));
    } finally {
      setLearning(false);
    }
  }

  async function handleApprove(id: string) {
    try {
      await invoke<string>("approve_skill_operation", { operationId: id });
      message.success(t("skillLearning.approved"));
      loadPending();
    } catch (e) {
      message.error(String(e));
    }
  }

  async function handleReject(id: string) {
    try {
      await invoke<string>("reject_skill_operation", { operationId: id, reason: t("skillLearning.rejectedByUser") });
      message.success(t("skillLearning.rejected"));
      loadPending();
    } catch (e) {
      message.error(String(e));
    }
  }

  async function handleConfigChange(patch: Partial<SkillLearningConfig>) {
    if (!config) {
      return;
    }
    try {
      const next = { ...config, ...patch };
      await invoke<string>("update_skill_learning_config", { config: next });
      setConfig(next);
      message.success(t("skillLearning.configSaved"));
    } catch (e) {
      message.error(String(e));
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          display: "flex",
          gap: 8,
          padding: "0 0 12px",
          borderBottom: "1px solid var(--color-border-secondary)",
        }}
      >
        <Button
          size="small"
          type={tab === "learn" ? "primary" : "default"}
          icon={<GraduationCap size={14} />}
          onClick={() => setTab("learn")}
        >
          {t("skillLearning.tabLearn")}
        </Button>
        <Button
          size="small"
          type={tab === "pending" ? "primary" : "default"}
          icon={<ListTodo size={14} />}
          onClick={() => {
            setTab("pending");
            loadPending();
          }}
        >
          {t("skillLearning.tabPending")}
          {pendingOps.length > 0 && <Tag color="orange" style={{ marginInlineStart: 4 }}>{pendingOps.length}</Tag>}
        </Button>
        <Button
          size="small"
          type={tab === "config" ? "primary" : "default"}
          icon={<Settings2 size={14} />}
          onClick={() => {
            setTab("config");
            loadConfig();
          }}
        >
          {t("skillLearning.tabConfig")}
        </Button>
      </div>

      <div style={{ flex: 1, minHeight: 0, overflow: "auto", paddingTop: 12 }}>
        {tab === "learn" && (
          <Form
            form={learnForm}
            layout="vertical"
            onFinish={handleLearn}
            initialValues={{ sourceType: "document", autoApprove: false }}
          >
            <Form.Item name="name" label={t("skillLearning.learnName")}>
              <Input placeholder={t("skillLearning.learnNamePlaceholder")} allowClear />
            </Form.Item>
            <Form.Item name="description" label={t("skillLearning.learnDesc")}>
              <Input placeholder={t("skillLearning.learnDescPlaceholder")} allowClear />
            </Form.Item>
            <Form.Item name="sourceType" label={t("skillLearning.learnSourceType")} rules={[{ required: true }]}>
              <Select
                options={[
                  { value: "document", label: t("skillLearning.sourceDocument") },
                  { value: "conversation", label: t("skillLearning.sourceConversation") },
                  { value: "codebase", label: t("skillLearning.sourceCodebase") },
                  { value: "mixed", label: t("skillLearning.sourceMixed") },
                ]}
              />
            </Form.Item>
            <Form.Item
              name="content"
              label={t("skillLearning.learnContent")}
              rules={[{ required: true, message: t("skillLearning.learnContentRequired") }]}
            >
              <Input.TextArea rows={8} placeholder={t("skillLearning.learnContentPlaceholder")} />
            </Form.Item>
            <Form.Item name="autoApprove" label={t("skillLearning.autoApprove")} valuePropName="checked">
              <Switch />
            </Form.Item>
            <Button type="primary" htmlType="submit" loading={learning} icon={<GraduationCap size={14} />}>
              {t("skillLearning.learnSubmit")}
            </Button>
          </Form>
        )}

        {tab === "pending" && (
          pendingOps.length === 0
            ? <Empty description={t("skillLearning.noPending")} style={{ marginTop: 40 }} />
            : (
              <Space direction="vertical" style={{ width: "100%" }}>
                {pendingOps.map((op) => (
                  <div
                    key={op.id}
                    style={{ border: "1px solid var(--color-border-secondary)", borderRadius: 8, padding: 12 }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                      <Tag color="blue">{op.operationType}</Tag>
                      {op.skillName && <b>{op.skillName}</b>}
                      <Tag color={op.riskLevel === "critical" || op.riskLevel === "high" ? "red" : "green"}>
                        {op.riskLevel}
                      </Tag>
                      <span style={{ flex: 1 }} />
                      <Tooltip title={op.reason}>
                        <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
                          {new Date(op.createdAt).toLocaleString()}
                        </span>
                      </Tooltip>
                    </div>
                    <div
                      style={{
                        fontSize: 12,
                        color: "var(--color-text-secondary)",
                        marginBottom: 8,
                        maxHeight: 80,
                        overflow: "auto",
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {op.content.slice(0, 500)}
                      {op.content.length > 500 ? "…" : ""}
                    </div>
                    <div style={{ display: "flex", gap: 8 }}>
                      <Button
                        size="small"
                        type="primary"
                        icon={<CheckCircle2 />}
                        onClick={() => handleApprove(op.id)}
                      >
                        {t("skillLearning.approve")}
                      </Button>
                      <Button size="small" danger icon={<XCircle />} onClick={() => handleReject(op.id)}>
                        {t("skillLearning.reject")}
                      </Button>
                    </div>
                  </div>
                ))}
              </Space>
            )
        )}

        {tab === "config" && config && (
          <Space direction="vertical" style={{ width: "100%" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span>{t("skillLearning.cfgSkillCreation")}</span>
              <Switch
                checked={config.enableSkillCreation}
                onChange={(v) => handleConfigChange({ enableSkillCreation: v })}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span>{t("skillLearning.cfgSkillPatching")}</span>
              <Switch
                checked={config.enableSkillPatching}
                onChange={(v) => handleConfigChange({ enableSkillPatching: v })}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span>{t("skillLearning.cfgBackgroundReview")}</span>
              <Switch
                checked={config.enableBackgroundReview}
                onChange={(v) => handleConfigChange({ enableBackgroundReview: v })}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <Tooltip title={t("skillLearning.cfgWriteGateTip")}>
                <span>{t("skillLearning.cfgWriteGate")}</span>
              </Tooltip>
              <Switch
                checked={config.writeApprovalGate}
                onChange={(v) => handleConfigChange({ writeApprovalGate: v })}
              />
            </div>
            <Divider style={{ margin: "8px 0" }} />
            <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
              {t("skillLearning.minToolCalls")}: {config.minToolCallsForCreation} · {t("skillLearning.maxReviewMsgs")}:
              {" "}
              {config.maxReviewMessages}
            </div>
          </Space>
        )}
      </div>
    </div>
  );
}
