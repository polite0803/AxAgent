// SPDX-License-Identifier: AGPL-3.0-only

import { Button } from "antd";
import { Globe } from "lucide-react";
import { useTranslation } from "react-i18next";

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { useGatewayLinkStore } from "@/stores";

interface GatewayMenuProps {
  /** 选择网关回调（由 InputArea 记录选中值，用于发送时创建网关会话） */
  onSelect: (id: string) => void;
}

/**
 * 网关（Gateway）链接选择菜单。
 * 仅在存在已启用的已连接网关时渲染，用于通过网关通道发起对话。
 */
export function GatewayMenu({ onSelect }: GatewayMenuProps) {
  const { t } = useTranslation();
  const gatewayLinks = useGatewayLinkStore((s) => s.links);

  const connectedGateways = gatewayLinks.filter(
    (l) => l.enabled && l.status === "connected",
  );

  if (connectedGateways.length === 0) {
    return null;
  }

  const items: DropdownItem[] = connectedGateways.map((gw) => ({
    key: `gateway:${gw.id}`,
    icon: <Globe size={14} />,
    label: gw.name,
    onClick: () => onSelect(gw.id),
  }));

  return (
    <DropdownMenu items={items}>
      <Tooltip title={t("chat.mode.gateway")}>
        <Button type="text" size="small" icon={<Globe size={14} />} />
      </Tooltip>
    </DropdownMenu>
  );
}
