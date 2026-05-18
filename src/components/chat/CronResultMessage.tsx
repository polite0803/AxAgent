import { Card, Tag, Typography } from "antd";
import { CheckCircle, Clock, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface CronResultMessageProps {
  jobName: string;
  schedule: string;
  description?: string;
  result: string;
  success: boolean;
  timestamp: number;
  platform?: string;
}

export function CronResultMessage({
  jobName,
  schedule,
  description,
  result,
  success,
  timestamp,
  platform,
}: CronResultMessageProps) {
  const { t, i18n } = useTranslation();
  return (
    <Card
      size="small"
      style={{ maxWidth: 500 }}
      title={
        <div className="flex items-center gap-2">
          <Clock size={14} />
          <span>{t("cronResult.prefix", { jobName })}</span>
          {success
            ? <Tag color="success" icon={<CheckCircle size={12} />}>{t("cronResult.success")}</Tag>
            : <Tag color="error" icon={<XCircle size={12} />}>{t("cronResult.failed")}</Tag>}
        </div>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <div className="flex items-center justify-between">
          <Text type="secondary">{t("cronResult.schedule")}</Text>
          <Text code>{schedule}</Text>
        </div>
        {description && (
          <div className="flex items-center justify-between">
            <Text type="secondary">{t("cronResult.scheduleDescription")}</Text>
            <Text>{description}</Text>
          </div>
        )}
        {platform && (
          <div className="flex items-center justify-between">
            <Text type="secondary">{t("cronResult.platform")}</Text>
            <Tag>{platform}</Tag>
          </div>
        )}
        <div className="flex items-center justify-between">
          <Text type="secondary">{t("cronResult.time")}</Text>
          <Text>{new Date(timestamp).toLocaleString(i18n.language)}</Text>
        </div>
        <Paragraph
          ellipsis={{ rows: 3, expandable: true }}
          style={{ marginTop: 8, marginBottom: 0 }}
        >
          {result || t("cronResult.emptyResult")}
        </Paragraph>
      </div>
    </Card>
  );
}
