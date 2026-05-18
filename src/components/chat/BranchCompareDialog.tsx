import { Modal, Select, Space, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Message } from "@/types";

import { BranchComparePanel } from "./BranchComparePanel";

export interface BranchCompareDialogProps {
  open: boolean;
  onClose: () => void;
  versions: Message[];
  isDarkMode: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: string[];
  codeFontFamily?: string;
}

export function BranchCompareDialog({
  open,
  onClose,
  versions,
  isDarkMode,
  codeBlockDarkTheme,
  codeBlockLightTheme,
  codeBlockThemes,
  codeFontFamily,
}: BranchCompareDialogProps) {
  const { t } = useTranslation();

  const options = useMemo(
    () =>
      versions
        .flatMap((v) =>
          v.role === "assistant"
            ? [{
              label: `${v.model_id ?? t("chat.branch.unknownModel")} — v${v.version_index}${
                v.is_active ? ` (${t("chat.branch.current")})` : ""
              }`,
              value: v.id,
            }]
            : []
        ),
    [versions, t],
  );

  const [leftId, setLeftId] = useState<string | undefined>(options[0]?.value);
  const [rightId, setRightId] = useState<string | undefined>(options[1]?.value ?? options[0]?.value);

  useEffect(() => {
    if (open) {
      setLeftId(options[0]?.value);
      setRightId(options[1]?.value ?? options[0]?.value);
    }
  }, [open, options]);

  const leftMessage = useMemo(
    () => versions.find((v) => v.id === leftId),
    [versions, leftId],
  );
  const rightMessage = useMemo(
    () => versions.find((v) => v.id === rightId),
    [versions, rightId],
  );

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  if (versions.length < 1) { return null; }

  return (
    <Modal
      title={t("chat.branch.compareTitle")}
      open={open}
      onCancel={handleClose}
      footer={null}
      width="90vw"
      style={{ maxWidth: 1400, top: 24 }}
      styles={{ body: { padding: "16px 24px", height: "70vh", overflow: "auto" } }}
      destroyOnClose
    >
      <Space style={{ marginBottom: 16, width: "100%" }} size={12}>
        <div style={{ flex: 1 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
            {t("chat.branch.left")}
          </Typography.Text>
          <Select
            value={leftId}
            onChange={setLeftId}
            options={options}
            style={{ width: "100%" }}
            placeholder={t("chat.branch.selectVersion")}
          />
        </div>
        <div style={{ flex: 1 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
            {t("chat.branch.right")}
          </Typography.Text>
          <Select
            value={rightId}
            onChange={setRightId}
            options={options}
            style={{ width: "100%" }}
            placeholder={t("chat.branch.selectVersion")}
          />
        </div>
      </Space>
      <BranchComparePanel
        leftMessage={leftMessage}
        rightMessage={rightMessage}
        isDarkMode={isDarkMode}
        codeBlockDarkTheme={codeBlockDarkTheme}
        codeBlockLightTheme={codeBlockLightTheme}
        codeBlockThemes={codeBlockThemes}
        codeFontFamily={codeFontFamily}
      />
    </Modal>
  );
}
