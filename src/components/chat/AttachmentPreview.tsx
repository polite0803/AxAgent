// SPDX-License-Identifier: AGPL-3.0-only

import { CloseCircleFilled } from "@ant-design/icons";
import { App, Image, Tag } from "antd";
import { AlertCircle, FileImage, Paperclip } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { invoke, logIpcError } from "@/lib/invoke";
import type { Attachment } from "@/types";

// eslint-disable-next-line react-refresh/only-export-components
export const ATTACHMENT_IMG_STYLE: React.CSSProperties = {
  maxWidth: 200,
  maxHeight: 160,
  borderRadius: 8,
  objectFit: "cover" as const,
};

export function AttachmentPreview({
  att,
  themeColor,
}: {
  att: Attachment;
  themeColor: string;
}) {
  const { t } = useTranslation();
  const { modal } = App.useApp();
  const isImage = att.file_type?.startsWith("image/");
  const mountedRef = useRef(true);

  const [src, setSrc] = useState<string | null>(() => {
    if (!isImage) {
      return null;
    }
    if (att.data) {
      return `data:${att.file_type};base64,${att.data}`;
    }
    return null;
  });
  const failedRef = useRef(false);
  const [fileExists, setFileExists] = useState<boolean | null>(null);

  useEffect(() => {
    if (!att.file_path) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setFileExists(false);
      return;
    }
    invoke<boolean>("check_attachment_exists", { filePath: att.file_path })
      .then((exists) => {
        if (mountedRef.current) {
          setFileExists(exists);
        }
      })
      .catch(() => {
        if (mountedRef.current) {
          setFileExists(false);
        }
      });
    return () => {
      mountedRef.current = false;
    };
  }, [att.file_path]);

  useEffect(() => {
    if (!isImage || src || failedRef.current) {
      return;
    }
    if (!att.file_path || fileExists === false) {
      failedRef.current = true;
      return;
    }
    if (fileExists === null) {
      return;
    }
    invoke<string>("read_attachment_preview", { filePath: att.file_path })
      .then((dataUrl) => {
        if (mountedRef.current) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (mountedRef.current) {
          failedRef.current = true;
        }
      });
  }, [isImage, att.file_path, src, fileExists]);

  if (fileExists === false) {
    const showMissingModal = () => {
      invoke<string>("resolve_attachment_path", { filePath: att.file_path })
        .then((absPath) => {
          modal.confirm({
            icon: <CloseCircleFilled style={{ color: "#ff4d4f" }} />,
            title: t("chat.attachmentNotFound"),
            content: absPath,
            okText: t("chat.attachmentOk"),
            cancelText: t("chat.attachmentRevealLocation"),
            onCancel: () => {
              invoke("reveal_attachment_file", {
                filePath: att.file_path,
              }).catch(logIpcError("reveal_attachment_file"));
            },
          });
        })
        .catch(() => {
          modal.error({
            title: t("chat.attachmentNotFound"),
            content: att.file_path || att.file_name,
            okText: t("chat.attachmentOk"),
          });
        });
    };
    return (
      <Tag
        icon={<AlertCircle size={12} />}
        color="error"
        style={{ margin: 0, cursor: "pointer" }}
        onClick={showMissingModal}
      >
        {att.file_name}
      </Tag>
    );
  }

  if (fileExists === null && !src) {
    return (
      <Tag
        icon={isImage ? <FileImage size={12} /> : <Paperclip size={12} />}
        style={{ margin: 0, cursor: "default", opacity: 0.5 }}
      >
        {att.file_name}
      </Tag>
    );
  }

  if (isImage && src) {
    return (
      <Image
        src={src}
        alt={att.file_name}
        style={ATTACHMENT_IMG_STYLE}
        preview={{ mask: { blur: true }, scaleStep: 0.5 }}
      />
    );
  }

  const handleOpen = () => {
    if (att.file_path) {
      invoke("open_attachment_file", { filePath: att.file_path }).catch(
        logIpcError("open_attachment_file"),
      );
    }
  };

  const handleReveal = () => {
    if (att.file_path) {
      invoke("reveal_attachment_file", { filePath: att.file_path }).catch(
        logIpcError("reveal_attachment_file"),
      );
    }
  };

  const contextMenuItems = att.file_path
    ? [
      { key: "open", label: t("chat.attachmentOpen"), onClick: handleOpen },
      {
        key: "reveal",
        label: t("chat.attachmentRevealInFinder"),
        onClick: handleReveal,
      },
    ]
    : [];

  const tag = (
    <Tag
      icon={isImage ? <FileImage size={12} /> : <Paperclip size={12} />}
      color={themeColor}
      style={{ margin: 0, cursor: att.file_path ? "pointer" : "default" }}
      onClick={att.file_path ? handleOpen : undefined}
    >
      {att.file_name}
    </Tag>
  );

  if (!att.file_path) {
    return tag;
  }

  return (
    <DropdownMenu items={contextMenuItems} trigger={["contextMenu"]}>
      {tag}
    </DropdownMenu>
  );
}
