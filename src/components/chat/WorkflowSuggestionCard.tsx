// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Card, theme, Typography } from "antd";
import { ArrowRight, Lightbulb, X } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export interface WorkflowMatch {
  templateId: string;
  templateName: string;
  similarity: number;
}

export interface WorkflowSuggestionCardProps {
  match: WorkflowMatch;
  onSwitch: (templateId: string) => void;
  onDismiss: () => void;
}

export function WorkflowSuggestionCard({
  match,
  onSwitch,
  onDismiss,
}: WorkflowSuggestionCardProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <Card
      size="small"
      style={{
        margin: "16px 0",
        borderColor: token.colorWarningBorder,
        background: token.colorWarningBg,
        borderRadius: token.borderRadiusLG,
      }}
    >
      <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
        <Lightbulb
          size={20}
          color={token.colorWarning}
          style={{ marginTop: 2 }}
        />
        <div style={{ flex: 1 }}>
          <Text strong style={{ fontSize: 13 }}>
            {t("chat.workflow.suggestionHint")}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 13 }}>
            {t("chat.workflow.suggestionDesc", {
              name: match.templateName,
            })}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("chat.workflow.suggestionReason")}
          </Text>
          <div style={{ marginTop: 10, display: "flex", gap: 8 }}>
            <Button
              size="small"
              type="primary"
              icon={<ArrowRight size={14} />}
              onClick={() => onSwitch(match.templateId)}
            >
              {t("chat.workflow.switchToWorkflow")}
            </Button>
            <Button size="small" icon={<X size={14} />} onClick={onDismiss}>
              {t("chat.workflow.dismiss")}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}
