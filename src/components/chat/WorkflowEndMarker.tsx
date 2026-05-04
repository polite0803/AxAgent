import { Button, Card, theme, Typography } from "antd";
import { Archive, CheckCircle, ListChecks } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

export interface WorkflowEndMarkerProps {
  workflowName: string;
  stepCount: number;
  completedCount: number;
  durationSeconds: number;
  onViewDetails?: () => void;
  onArchive?: () => void;
}

export function WorkflowEndMarker({
  workflowName,
  stepCount,
  completedCount,
  durationSeconds,
  onViewDetails,
  onArchive,
}: WorkflowEndMarkerProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const minutes = Math.floor(durationSeconds / 60);
  const seconds = durationSeconds % 60;
  const durationStr = minutes > 0
    ? `${minutes} 分 ${seconds} 秒`
    : `${seconds} 秒`;

  return (
    <Card
      size="small"
      style={{
        margin: "16px 0",
        borderColor: token.colorSuccessBorder,
        background: token.colorSuccessBg,
        borderRadius: token.borderRadiusLG,
      }}
    >
      <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
        <CheckCircle
          size={20}
          color={token.colorSuccess}
          style={{ marginTop: 2 }}
        />
        <div style={{ flex: 1 }}>
          <Title
            level={5}
            style={{
              margin: 0,
              color: token.colorSuccess,
            }}
          >
            {t("chat.workflow.completedTitle", "工作流执行完成")}
          </Title>
          <Text type="secondary" style={{ fontSize: 13 }}>
            {t("chat.workflow.completedDesc", {
              name: workflowName,
              completed: completedCount,
              total: stepCount,
            })}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("chat.workflow.duration", "耗时")}: {durationStr}
          </Text>
          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            {onViewDetails && (
              <Button
                size="small"
                icon={<ListChecks size={14} />}
                onClick={onViewDetails}
              >
                {t("chat.workflow.viewDetails", "查看步骤详情")}
              </Button>
            )}
            {onArchive && (
              <Button
                size="small"
                type="primary"
                icon={<Archive size={14} />}
                onClick={onArchive}
              >
                {t("chat.workflow.archiveSession", "归档此会话")}
              </Button>
            )}
          </div>
        </div>
      </div>
    </Card>
  );
}
