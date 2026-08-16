// SPDX-License-Identifier: AGPL-3.0-only

import { Button } from "antd";
import { ExternalLink, FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Tooltip } from "@/components/layout/Tooltip";
import { abbreviatePath } from "./InputAreaUtils";

interface WorkspaceDirMenuProps {
  /** 当前 agent 工作目录 */
  cwd: string | null;
  /** 会话是否已有消息（有消息时锁定不可更改） */
  disabled: boolean;
  /** 选择工作目录回调 */
  onSelect: () => void;
  /** 在系统文件管理器中打开回调（仅在已有目录时触发） */
  onOpen: (cwd: string) => void;
}

const WORKSPACE_BUTTON_MAX_WIDTH = 400;

/**
 * Agent 工作目录选择 + 打开按钮组。
 * 负责展示当前目录（缩写）、选择目录、以及在文件管理器中打开目录。
 */
export function WorkspaceDirMenu({ cwd, disabled, onSelect, onOpen }: WorkspaceDirMenuProps) {
  const { t } = useTranslation();

  return (
    <>
      <Tooltip
        title={disabled
          ? t("chat.workspaceLocked")
          : cwd || t("common.workingDirectory")}
      >
        <Button
          type="text"
          size="small"
          icon={<FolderOpen size={14} />}
          onClick={onSelect}
          disabled={disabled}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            maxWidth: WORKSPACE_BUTTON_MAX_WIDTH,
          }}
        >
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              fontSize: 12,
            }}
          >
            {cwd ? abbreviatePath(cwd) : t("common.selectDirectory")}
          </span>
        </Button>
      </Tooltip>
      {cwd && (
        <Tooltip title={t("common.openDirectory")}>
          <Button
            type="text"
            size="small"
            icon={<ExternalLink size={14} />}
            onClick={() => onOpen(cwd)}
            style={{ fontSize: 12, minWidth: "auto", padding: "0 4px" }}
          />
        </Tooltip>
      )}
    </>
  );
}
