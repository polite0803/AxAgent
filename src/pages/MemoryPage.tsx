import MemorySettings from "@/components/settings/MemorySettings";
import { theme } from "antd";

export function MemoryPage() {
  const { token } = theme.useToken();

  return (
    <div className="h-full" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      <MemorySettings />
    </div>
  );
}
