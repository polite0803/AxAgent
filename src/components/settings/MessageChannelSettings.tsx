import { usePlatformStore } from "@/stores";
import { App, Button, Tabs } from "antd";
import { useTranslation } from "react-i18next";
import { GatewayConfigPanel } from "./GatewayConfigPanel";
import { PlatformStatusCard } from "./PlatformStatusCard";

export function MessageChannelSettings() {
  const { t } = useTranslation();
  const reconcile = usePlatformStore((s) => s.reconcile);
  const { message } = App.useApp();

  const handleApply = async () => {
    try {
      const report = await reconcile();
      if (report.started.length > 0) {
        message.success(t("settings.platform.applyStarted", { platforms: report.started.join(", ") }));
      }
      if (report.stopped.length > 0) {
        message.info(t("settings.platform.applyStopped", { platforms: report.stopped.join(", ") }));
      }
      if (report.errors.length > 0) {
        message.error(t("settings.platform.applyErrors", { errors: report.errors.map((e) => e[0]).join(", ") }));
      }
    } catch {
      message.error(t("settings.platform.applyFailed"));
    }
  };

  const items = [
    {
      key: "config",
      label: t("settings.platform.tabConfig"),
      children: <GatewayConfigPanel />,
    },
    {
      key: "status",
      label: t("settings.platform.tabStatus"),
      children: <PlatformStatusCard />,
    },
  ];

  return (
    <div className="p-6 pb-12">
      <div className="flex items-center justify-between mb-4">
        <h2 style={{ fontSize: 18, fontWeight: 600 }}>{t("settings.messageChannels")}</h2>
        <Button type="primary" onClick={handleApply}>
          {t("settings.platform.applyConfig")}
        </Button>
      </div>
      <Tabs items={items} />
    </div>
  );
}
