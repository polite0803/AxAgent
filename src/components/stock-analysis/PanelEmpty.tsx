import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { SettingOutlined } from "@ant-design/icons";
import { Alert, Button, Empty, Space, Typography } from "antd";
import { AlertTriangle, Clock } from "lucide-react";
import { useTranslation } from "react-i18next";

export type PanelEmptyKind =
  | "noData"
  | "vendorDisabled"
  | "noStock"
  | "backendOffline"
  | "connectionFailed"
  /** 缺陷 F 修复:replay 模式下数据降级(无历史语义/被跳过) */
  | "replayDegraded";

export interface PanelEmptyProps {
  kind: PanelEmptyKind;
  /** vendor names to show when kind === "vendorDisabled" */
  vendorNames?: string[];
  /** description override (defaults to i18n key) */
  description?: string;
  /** "open settings" button onClick handler (page-level) */
  onOpenSettings?: () => void;
  /** hide the "open settings" CTA */
  hideCta?: boolean;
  /** image style override */
  image?: React.ReactNode;
  /** 缺陷 F 修复: 显式传入降级原因(replayDegraded 时显示);省略时用全局降级 count */
  reason?: string;
}

/**
 * 统一面板空状态：用于 stock-analysis 右侧栏各面板在"无数据/数据源未启用/未选股"等场景下。
 * 不再以"网络/API 不可用"搪塞，而是给出明确原因 + 可执行的恢复动作。
 */
export function PanelEmpty(props: PanelEmptyProps) {
  const { t } = useTranslation();
  const { kind, vendorNames, description, onOpenSettings, hideCta, image, reason } = props;

  // 缺陷 F 修复: 当 kind==="noData" 且当前在 replay 模式,自动升级为 replayDegraded 文案
  const mode = useTimeAnchorStore((s) => s.mode);
  const degradationCount = useTimeAnchorStore((s) => s.degradationCount);
  const isReplay = mode === "replay" || mode === "backtest_sweep";
  const effectiveKind: PanelEmptyKind = kind === "noData" && isReplay ? "replayDegraded" : kind;

  const descText = (() => {
    if (description) { return description; }
    switch (effectiveKind) {
      case "noData":
        return t("stockAnalysis.settings.panels.noData");
      case "noStock":
        return t("stockAnalysis.selectStockFirst");
      case "backendOffline":
        return t("stockAnalysis.settings.vendor.backendOffline");
      case "connectionFailed":
        return t("stockAnalysis.settings.vendor.connectionFailed");
      case "replayDegraded":
        return reason
          ? t("stockAnalysis.empty.replayDegradedWithReason", { reason })
          : degradationCount > 0
          ? t("stockAnalysis.empty.replayDegradedWithCount", { n: degradationCount })
          : t("stockAnalysis.empty.replayDegraded");
      case "vendorDisabled":
        return vendorNames && vendorNames.length > 0
          ? t("stockAnalysis.settings.vendor.disabled", { names: vendorNames.join(" / ") })
          : t("stockAnalysis.settings.vendor.disabledGeneric");
      default:
        return t("stockAnalysis.settings.panels.noData");
    }
  })();

  const showCta = !hideCta && onOpenSettings && (kind === "vendorDisabled" || kind === "backendOffline");

  // 缺陷 F 修复: replayDegraded 走特殊渲染 — 紫色 Alert 而非 Empty
  if (effectiveKind === "replayDegraded") {
    return (
      <Alert
        type="warning"
        showIcon
        icon={<AlertTriangle size={16} />}
        message={
          <Space size={6}>
            <Clock size={12} className="text-purple-500" />
            <span>{t("stockAnalysis.empty.replayDegradedTitle")}</span>
          </Space>
        }
        description={descText}
        className="text-xs"
        data-testid="panel-empty-replay-degraded"
      />
    );
  }

  // Empty 的 description 传 string 才会覆盖 antd 内置的 "No data"，
  // 传 ReactNode 时 antd 会同时渲染默认文案和自定义节点，叠加出两层。
  return (
    <Empty
      image={image ?? Empty.PRESENTED_IMAGE_SIMPLE}
      description={showCta
        ? (
          <Space orientation="vertical" size={4} className="text-center">
            <Typography.Text type="secondary" className="text-xs">
              {descText}
            </Typography.Text>
            <Button
              size="small"
              type="link"
              icon={<SettingOutlined />}
              onClick={onOpenSettings}
              className="px-0"
            >
              {t("stockAnalysis.settings.openDataSource")}
            </Button>
          </Space>
        )
        : descText}
    />
  );
}
