// SPDX-License-Identifier: AGPL-3.0-only

import { MemoryApprovalPanel } from "@/components/memory/MemoryApprovalPanel";
import { MemorySettings } from "@/components/settings/MemorySettings";
import { theme } from "antd";

export function MemoryPage() {
  const { token } = theme.useToken();

  return (
    <div
      className="h-full"
      style={{ overflow: "auto", backgroundColor: token.colorBgElevated }}
    >
      <MemoryApprovalPanel />
      <MemorySettings />
    </div>
  );
}
