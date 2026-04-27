import { useSkillStore } from "@/stores";
import type { SkillProposal } from "@/types";
import { Button, Card, Empty, message, Modal, Space, Spin, Tag, Typography } from "antd";
import { Check, Lightbulb, Sparkles, X } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface SkillProposalCardProps {
  proposal: SkillProposal;
  onCreate: (proposal: SkillProposal) => void;
  onDismiss: (name: string) => void;
  t: (key: string) => string;
}

const SkillProposalCard: React.FC<SkillProposalCardProps> = ({ proposal, onCreate, onDismiss, t }) => {
  const confidencePercent = Math.round(proposal.confidence * 100);

  const getConfidenceColor = () => {
    if (confidencePercent >= 70) { return "green"; }
    if (confidencePercent >= 50) { return "orange"; }
    return "default";
  };

  const getEventLabel = () => {
    switch (proposal.trigger_event) {
      case "successful_multi_step_workflow":
        return t("skill.proposal.successWorkflow");
      case "failed_workflow_needing_improvement":
        return t("skill.proposal.failedWorkflow");
      case "partial_success_requiring_refinement":
        return t("skill.proposal.partialSuccess");
      default:
        return proposal.trigger_event;
    }
  };

  return (
    <Card
      size="small"
      style={{ marginBottom: 12, border: "1px solid #f0f0f0" }}
      styles={{ body: { padding: 16 } }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div style={{ flex: 1 }}>
          <Space align="center" style={{ marginBottom: 8 }}>
            <Sparkles size={16} style={{ color: "#faad14" }} />
            <Text strong style={{ fontSize: 15 }}>{proposal.suggested_name}</Text>
            <Tag color={getConfidenceColor()}>{confidencePercent}% {t("skill.proposal.confidence")}</Tag>
            <Tag>{getEventLabel()}</Tag>
          </Space>

          <Paragraph type="secondary" style={{ marginBottom: 8, fontSize: 13 }}>
            {proposal.task_description}
          </Paragraph>

          <details style={{ fontSize: 12 }}>
            <summary style={{ cursor: "pointer", color: "#1890ff" }}>
              {t("skill.proposal.viewContent")}
            </summary>
            <pre
              style={{
                background: "#f5f5f5",
                padding: 12,
                borderRadius: 4,
                marginTop: 8,
                whiteSpace: "pre-wrap",
                fontSize: 11,
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {proposal.suggested_content}
            </pre>
          </details>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginLeft: 16 }}>
          <Button
            type="primary"
            size="small"
            icon={<Check size={14} />}
            onClick={() => onCreate(proposal)}
          >
            {t("skill.proposal.create")}
          </Button>
          <Button
            size="small"
            icon={<X size={14} />}
            onClick={() => onDismiss(proposal.suggested_name)}
          >
            {t("skill.proposal.dismiss")}
          </Button>
        </div>
      </div>
    </Card>
  );
};

interface SkillProposalPanelProps {
  open: boolean;
  onClose: () => void;
}

export const SkillProposalPanel: React.FC<SkillProposalPanelProps> = ({ open, onClose }) => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const { loadSkillProposals, createSkillFromProposal } = useSkillStore();
  const [localProposals, setLocalProposals] = useState<SkillProposal[]>([]);

  useEffect(() => {
    if (open) {
      setLoading(true);
      loadSkillProposals().then((proposals) => {
        setLocalProposals(proposals || []);
      }).finally(() => setLoading(false));
    }
  }, [open, loadSkillProposals]);

  const handleCreate = async (proposal: SkillProposal) => {
    try {
      await createSkillFromProposal(
        proposal.suggested_name,
        proposal.task_description,
        proposal.suggested_content,
      );
      message.success(t("skill.proposal.created") + " " + proposal.suggested_name);
      setLocalProposals((prev) => prev.filter((p) => p.suggested_name !== proposal.suggested_name));
    } catch (e) {
      message.error(t("skill.proposal.error", "Failed to create skill"));
    }
  };

  const handleDismiss = (name: string) => {
    setLocalProposals((prev) => prev.filter((p) => p.suggested_name !== name));
  };

  return (
    <Modal
      title={
        <Space>
          <Lightbulb size={18} style={{ color: "#faad14" }} />
          <span>{t("skill.proposal.title")}</span>
          {localProposals.length > 0 && <Tag color="blue">{localProposals.length}</Tag>}
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
      destroyOnClose
    >
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : localProposals.length === 0
        ? (
          <Empty
            image={<Sparkles size={48} style={{ color: "#d9d9d9" }} />}
            description={
              <Text type="secondary">
                {t(
                  "skill.proposal.empty",
                  "No skill proposals yet. Complete complex tasks with the Agent to generate suggestions.",
                )}
              </Text>
            }
          />
        )
        : (
          <div style={{ maxHeight: 500, overflow: "auto", padding: "8px 0" }}>
            <Paragraph type="secondary" style={{ marginBottom: 16, fontSize: 13 }}>
              {t(
                "skill.proposal.hint",
                "These skills were auto-generated from your successful Agent workflows. Review and create them to automate similar tasks in the future.",
              )}
            </Paragraph>
            {localProposals.map((proposal) => (
              <SkillProposalCard
                key={proposal.suggested_name}
                proposal={proposal}
                onCreate={handleCreate}
                onDismiss={handleDismiss}
                t={t}
              />
            ))}
          </div>
        )}
    </Modal>
  );
};

export const useSkillProposalBadge = () => {
  const { skillProposals, loadSkillProposals } = useSkillStore();

  useEffect(() => {
    loadSkillProposals();
  }, [loadSkillProposals]);

  return skillProposals.length;
};
