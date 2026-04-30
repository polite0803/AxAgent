import { GatewayConfigPanel } from "./GatewayConfigPanel";
import { PlatformStatusCard } from "./PlatformStatusCard";
import { usePlatformStore } from "@/stores";
import { Button, Tabs, App } from "antd";

export function MessageChannelSettings() {
  const reconcile = usePlatformStore((s) => s.reconcile);
  const { message } = App.useApp();

  const handleApply = async () => {
    try {
      const report = await reconcile();
      if (report.started.length > 0) {
        message.success(`已启动: ${report.started.join(", ")}`);
      }
      if (report.stopped.length > 0) {
        message.info(`已停止: ${report.stopped.join(", ")}`);
      }
      if (report.errors.length > 0) {
        message.error(`错误: ${report.errors.map((e) => e[0]).join(", ")}`);
      }
    } catch {
      message.error("应用配置失败");
    }
  };

  const items = [
    {
      key: "config",
      label: "平台配置",
      children: <GatewayConfigPanel />,
    },
    {
      key: "status",
      label: "连接状态",
      children: <PlatformStatusCard />,
    },
  ];

  return (
    <div className="p-6 pb-12">
      <div className="flex items-center justify-between mb-4">
        <h2 style={{ fontSize: 18, fontWeight: 600 }}>消息渠道</h2>
        <Button type="primary" onClick={handleApply}>
          应用配置
        </Button>
      </div>
      <Tabs items={items} />
    </div>
  );
}
