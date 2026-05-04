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
            {t("chat.workflow.suggestionHint", "提示")}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 13 }}>
            {t("chat.workflow.suggestionDesc", {
              name: match.templateName,
            })}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("chat.workflow.suggestionReason", "使用工作流可以获得更结构化的多步骤执行")}
          </Text>
          <div style={{ marginTop: 10, display: "flex", gap: 8 }}>
            <Button
              size="small"
              type="primary"
              icon={<ArrowRight size={14} />}
              onClick={() => onSwitch(match.templateId)}
            >
              {t("chat.workflow.switchToWorkflow", "切换到此工作流")}
            </Button>
            <Button
              size="small"
              icon={<X size={14} />}
              onClick={onDismiss}
            >
              {t("chat.workflow.dismiss", "忽略")}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}
