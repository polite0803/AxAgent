// SPDX-License-Identifier: AGPL-3.0-only

import { Button, theme } from "antd";
import { Check, Shield, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";

interface PermissionMenuProps {
  /** 当前 agent 权限模式（受控，供 InputArea 在会话切换时同步） */
  permissionMode: string;
  /** 权限模式变更回调（含账号权限确认弹窗逻辑） */
  onChange: (mode: string) => void;
}

/**
 * Agent 权限模式选择菜单（default / accept_edits / full_access）。
 * 仅在 agent 模式下渲染，变更交由 InputArea 处理确认弹窗与后端持久化。
 */
export function PermissionMenu({ permissionMode, onChange }: PermissionMenuProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const icon = (() => {
    switch (permissionMode) {
      case "accept_edits":
        return <ShieldCheck size={14} style={{ color: token.colorPrimary }} />;
      case "full_access":
        return <ShieldAlert size={14} style={{ color: token.colorError }} />;
      default:
        return <Shield size={14} />;
    }
  })();

  const label = (() => {
    switch (permissionMode) {
      case "accept_edits":
        return t("common.permissionAcceptEdits");
      case "full_access":
        return t("common.permissionFullAccess");
      default:
        return t("common.permissionDefault");
    }
  })();

  const items: DropdownItem[] = [
    {
      key: "default",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionDefault")}
          {permissionMode === "default" && <Check size={14} style={{ color: token.colorPrimary }} />}
        </span>
      ),
      icon: <Shield size={14} />,
      onClick: () => onChange("default"),
    },
    {
      key: "accept_edits",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionAcceptEdits")}
          {permissionMode === "accept_edits" && <Check size={14} style={{ color: token.colorPrimary }} />}
        </span>
      ),
      icon: <ShieldCheck size={14} style={{ color: token.colorPrimary }} />,
      onClick: () => onChange("accept_edits"),
    },
    {
      key: "full_access",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionFullAccess")}
          {permissionMode === "full_access" && <Check size={14} style={{ color: token.colorError }} />}
        </span>
      ),
      icon: <ShieldAlert size={14} style={{ color: token.colorError }} />,
      onClick: () => onChange("full_access"),
    },
  ];

  return (
    <DropdownMenu items={items}>
      <Button
        type="text"
        size="small"
        icon={icon}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
          fontSize: 12,
          ...(permissionMode === "full_access" ? { color: token.colorError } : {}),
        }}
      >
        {label}
      </Button>
    </DropdownMenu>
  );
}
