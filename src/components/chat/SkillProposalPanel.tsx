// SPDX-License-Identifier: AGPL-3.0-only

/* eslint-disable react-refresh/only-export-components */
import { message } from "@/lib/toast";
import { useSkillStore } from "@/stores";
import type { SkillProposal } from "@/types";
import { Button, Card, Empty, Modal, Space, Spin, Tag, theme, Typography } from "antd";
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

const SkillProposalCard: React.FC<SkillProposalCardProps> = ({
  proposal,
  onCreate,
  onDismiss,
  t,
}) => {
  const { token } = theme.useToken();
  const confidencePercent = Math.round(proposal.confidence * 100);

  const getConfidenceColor = () => {
    if (confidencePercent >= 70) {
      return "green";
    }
    if (confidencePercent >= 50) {
      return "orange";
    }
    return "default";
  };

  const getEventLabel = () => {
    switch (proposal.triggerEvent) {
      case "successful_multi_step_workflow":
        return t("skill.proposal.successWorkflow");
      case "failed_workflow_needing_improvement":
        return t("skill.proposal.failedWorkflow");
      case "partial_success_requiring_refinement":
        return t("skill.proposal.partialSuccess");
      default:
        return proposal.triggerEvent;
    }
  };

  return (
    <Card
      size="small"
      style={{ marginBottom: 12, border: "1px solid #f0f0f0" }}
      styles={{ body: { padding: 16 } }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
        }}
      >
        <div style={{ flex: 1 }}>
          <Space align="center" style={{ marginBottom: 8 }}>
            <Sparkles size={16} style={{ color: token.colorWarning }} />
            <Text strong style={{ fontSize: 15 }}>
              {proposal.suggestedName}
            </Text>
            <Tag color={getConfidenceColor()}>
              {confidencePercent}% {t("skill.proposal.confidence")}
            </Tag>
            <Tag>{getEventLabel()}</Tag>
          </Space>

          <Paragraph type="secondary" style={{ marginBottom: 8, fontSize: 13 }}>
            {proposal.taskDescription}
          </Paragraph>

          <details style={{ fontSize: 12 }}>
            <summary style={{ cursor: "pointer", color: token.colorPrimary }}>
              {t("skill.proposal.viewContent")}
            </summary>
            <pre
              style={{
                background: token.colorFillQuaternary,
                padding: 12,
                borderRadius: 4,
                marginTop: 8,
                whiteSpace: "pre-wrap",
                fontSize: 12,
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {proposal.suggestedContent}
            </pre>
          </details>
        </div>

        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 8,
            marginLeft: 16,
          }}
        >
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
            onClick={() => onDismiss(proposal.suggestedName)}
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

export const SkillProposalPanel: React.FC<SkillProposalPanelProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [loading, setLoading] = useState(false);
  const { loadSkillProposals, createSkillFromProposal } = useSkillStore();
  const [localProposals, setLocalProposals] = useState<SkillProposal[]>([]);

  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (open) {
      setLoading(true);
      loadSkillProposals()
        .then((proposals) => {
          setLocalProposals(proposals || []);
        })
        .finally(() => setLoading(false));
    }
  }, [open, loadSkillProposals]);
  /* eslint-enable react-hooks/set-state-in-effect */

  const handleCreate = async (proposal: SkillProposal) => {
    try {
      await createSkillFromProposal(
        proposal.suggestedName,
        proposal.taskDescription,
        proposal.suggestedContent,
      );
      message.success(
        t("skill.proposal.created") + " " + proposal.suggestedName,
      );
      setLocalProposals((prev) => prev.filter((p) => p.suggestedName !== proposal.suggestedName));
    } catch {
      message.error(t("skill.proposal.error"));
    }
  };

  const handleDismiss = (name: string) => {
    setLocalProposals((prev) => prev.filter((p) => p.suggestedName !== name));
  };

  return (
    <Modal
      title={
        <Space>
          <Lightbulb size={18} style={{ color: token.colorWarning }} />
          <span>{t("skill.proposal.title")}</span>
          {localProposals.length > 0 && <Tag color="blue">{localProposals.length}</Tag>}
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
      destroyOnHidden
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
            image={<Sparkles size={48} style={{ color: token.colorTextQuaternary }} />}
            description={
              <Text type="secondary">
                {t(
                  "skill.proposal.empty",
                )}
              </Text>
            }
          />
        )
        : (
          <div style={{ maxHeight: 500, overflow: "auto", padding: "8px 0" }}>
            <Paragraph
              type="secondary"
              style={{ marginBottom: 16, fontSize: 13 }}
            >
              {t(
                "skill.proposal.hint",
              )}
            </Paragraph>
            {localProposals.map((proposal) => (
              <SkillProposalCard
                key={proposal.suggestedName}
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
