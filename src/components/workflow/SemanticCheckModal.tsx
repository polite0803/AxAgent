import type { NodeSkillMatch, SkillMatchResult, SkillReplacementAction } from "@/components/workflow/types";
import { useWorkflowEditorStore } from "@/stores";
import { Button, Descriptions, message, Modal, Space, Tag } from "antd";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";

interface SemanticCheckModalProps {
  open: boolean;
  onClose: () => void;
  matches: NodeSkillMatch[];
  onApplyReplacement: (nodeId: string, existingSkillId: string, action: SkillReplacementAction) => void;
}

export const SemanticCheckModal: React.FC<SemanticCheckModalProps> = ({
  open,
  onClose,
  matches,
  onApplyReplacement,
}) => {
  const { t } = useTranslation("chat");
  const { clearSemanticCheckResult } = useWorkflowEditorStore();
  const [selectedActions, setSelectedActions] = useState<
    Record<string, { skillId: string; action: SkillReplacementAction }>
  >({});

  const handleClose = () => {
    clearSemanticCheckResult();
    onClose();
  };

  const handleSelectAction = (nodeId: string, skillId: string, action: SkillReplacementAction) => {
    setSelectedActions((prev) => ({
      ...prev,
      [nodeId]: { skillId, action },
    }));
  };

  const handleApply = () => {
    Object.entries(selectedActions).forEach(([nodeId, { skillId, action }]) => {
      onApplyReplacement(nodeId, skillId, action);
    });
    message.success(t("workflow.semanticCheckApplied"));
    handleClose();
  };

  const getActionButton = (match: SkillMatchResult, nodeId: string) => {
    const currentSelection = selectedActions[nodeId];
    const isReplaceSelected = currentSelection?.skillId === match.existing_skill.id
      && currentSelection?.action === "replace";
    const isKeepSelected = currentSelection?.skillId === match.existing_skill.id && currentSelection?.action === "keep";

    return (
      <Space direction="vertical" style={{ width: "100%" }}>
        <Button
          type={isReplaceSelected ? "primary" : "default"}
          onClick={() => handleSelectAction(nodeId, match.existing_skill.id, "replace")}
          style={{ width: "100%" }}
        >
          {t("workflow.replaceWithExisting")}
        </Button>
        <Button
          type={isKeepSelected ? "primary" : "default"}
          onClick={() => handleSelectAction(nodeId, match.existing_skill.id, "keep")}
          style={{ width: "100%" }}
        >
          {t("workflow.keepGeneratedSkill")}
        </Button>
      </Space>
    );
  };

  const renderMatchCard = (match: SkillMatchResult, nodeId: string) => {
    const similarityPercent = Math.round(match.similarity_score * 100);
    const similarityColor = similarityPercent >= 80 ? "green" : similarityPercent >= 60 ? "orange" : "red";

    return (
      <div
        key={match.existing_skill.id}
        style={{
          border: "1px solid #d9d9d9",
          borderRadius: 8,
          padding: 16,
          marginBottom: 12,
          backgroundColor: "#fafafa",
        }}
      >
        <Space style={{ width: "100%", justifyContent: "space-between", marginBottom: 8 }}>
          <Descriptions column={1} size="small" style={{ flex: 1 }}>
            <Descriptions.Item label={t("workflow.existingSkill")}>
              <strong>{match.existing_skill.name}</strong>
            </Descriptions.Item>
          </Descriptions>
          <Tag color={similarityColor} style={{ fontSize: 16, padding: "4px 12px" }}>
            {similarityPercent}%
          </Tag>
        </Space>
        <div style={{ marginBottom: 8 }}>
          <strong>{t("workflow.matchReasons")}:</strong>
          <div style={{ marginTop: 4 }}>
            {match.match_reasons.map((reason, i) => <Tag key={i} color="cyan">{reason}</Tag>)}
          </div>
        </div>
        <div style={{ marginTop: 12 }}>
          {getActionButton(match, nodeId)}
        </div>
      </div>
    );
  };

  const renderNodeMatches = (nodeMatch: NodeSkillMatch) => {
    const nodeId = nodeMatch.node_id || "unknown";
    return (
      <div key={nodeId} style={{ marginBottom: 24 }}>
        <h4 style={{ marginBottom: 12 }}>
          {t("workflow.generatedSkill")}: <Tag color="purple">{nodeMatch.skill_name}</Tag>
        </h4>
        <div style={{ marginLeft: 16 }}>
          {nodeMatch.matches.map((match) => renderMatchCard(match, nodeId))}
        </div>
      </div>
    );
  };

  return (
    <Modal
      title={t("workflow.semanticCheckTitle")}
      open={open}
      onCancel={handleClose}
      width={900}
      footer={[
        <Button key="skip" onClick={handleClose}>
          {t("workflow.skipSemanticCheck")}
        </Button>,
        <Button key="apply" type="primary" onClick={handleApply}>
          {t("workflow.applySemanticCheck")}
        </Button>,
      ]}
    >
      <div style={{ maxHeight: 500, overflowY: "auto" }}>
        <p style={{ marginBottom: 16, color: "#666" }}>
          {t("workflow.semanticCheckDescription")}
        </p>
        {matches.map((nodeMatch) => renderNodeMatches(nodeMatch))}
      </div>
    </Modal>
  );
};
