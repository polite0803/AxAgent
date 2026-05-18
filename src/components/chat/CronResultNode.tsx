import { theme } from "antd";
import { Loader } from "lucide-react";
import type { NodeComponentProps } from "markstream-react";
import { useTranslation } from "react-i18next";
import { CronResultMessage } from "./CronResultMessage";

type CronResultNodeData = {
  type: "cron-result";
  content?: string;
  attrs?: Record<string, string> | [string, string][];
  loading?: boolean;
};

function getAttrValue(
  attrs: CronResultNodeData["attrs"],
  key: string,
): string | undefined {
  if (!attrs) { return undefined; }
  if (Array.isArray(attrs)) {
    const entry = attrs.find(([name]) => name === key);
    return entry?.[1];
  }
  return attrs[key];
}

export function CronResultNode(props: NodeComponentProps<CronResultNodeData>) {
  const { node } = props;
  const { token } = theme.useToken();
  const { t } = useTranslation();

  if (!node) { return null; }

  const jobName = getAttrValue(node.attrs, "jobName");
  const schedule = getAttrValue(node.attrs, "schedule");
  const description = getAttrValue(node.attrs, "description");
  const success = getAttrValue(node.attrs, "success") !== "false";
  const platform = getAttrValue(node.attrs, "platform");
  const timestampStr = getAttrValue(node.attrs, "timestamp");
  const timestamp = timestampStr ? Number(timestampStr) : NaN;

  const isLoading = !jobName || !schedule || isNaN(timestamp);

  if (isLoading) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 12px",
          marginBottom: 8,
          borderRadius: 8,
          backgroundColor: token.colorFillQuaternary,
        }}
      >
        <Loader size={14} className="animate-spin" style={{ color: token.colorPrimary }} />
        <span style={{ color: token.colorTextSecondary, fontSize: 13 }}>
          {t("cronResult.loading")}
        </span>
      </div>
    );
  }

  return (
    <CronResultMessage
      jobName={jobName}
      schedule={schedule}
      description={description}
      result={node.content || ""}
      success={success}
      timestamp={timestamp}
      platform={platform}
    />
  );
}
