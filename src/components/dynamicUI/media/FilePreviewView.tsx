// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { DownloadOutlined, FileOutlined } from "@ant-design/icons";
import { Button, Image, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type PreviewMode = "image" | "text" | "unknown";

export const FilePreviewView: React.FC<DynamicUIProps> = ({ schema }) => {
  const { t } = useTranslation();
  const { filePath, fileUrl } = schema.props as {
    filePath?: string;
    fileUrl?: string;
  };

  const url = fileUrl || filePath || "";
  const fileName = filePath ? filePath.split(/[/\\]/).pop() || "" : "";
  const mode = useMemo(() => detectFileType(url), [url]);

  switch (mode) {
    case "image":
      return (
        <div style={schema.style as React.CSSProperties}>
          <Image src={url} alt={fileName} style={{ maxWidth: "100%" }} />
        </div>
      );

    case "text":
      return (
        <div
          className="bg-gray-50 dark:bg-gray-800 rounded p-4 overflow-auto max-h-96"
          style={schema.style as React.CSSProperties}
        >
          <Text
            code
            style={{ whiteSpace: "pre-wrap", wordBreak: "break-all" }}
          >
            {fileName ? t("dynamicUI.filePreview", { fileName }) : t("dynamicUI.textFile")}
          </Text>
        </div>
      );

    default:
      return (
        <div
          className="flex flex-col items-center justify-center border rounded p-6 bg-gray-50 dark:bg-gray-800"
          style={schema.style as React.CSSProperties}
        >
          <FileOutlined style={{ fontSize: 48, color: "#8c8c8c" }} />
          <Text className="mt-2 mb-1 font-medium">
            {fileName || t("dynamicUI.unknownFile")}
          </Text>
          <Text type="secondary" className="mb-4 text-sm">
            {t("dynamicUI.previewNotSupported")}
          </Text>
          {url
            ? (
              <Button
                type="primary"
                icon={<DownloadOutlined />}
                href={url}
                download={fileName}
              >
                {t("dynamicUI.downloadFile")}
              </Button>
            )
            : null}
        </div>
      );
  }
};

function detectFileType(url: string): PreviewMode {
  const ext = url.split(".").pop()?.toLowerCase() || "";

  const imageExts = [
    "jpg",
    "jpeg",
    "png",
    "gif",
    "svg",
    "webp",
    "bmp",
    "ico",
  ];
  if (imageExts.includes(ext)) {
    return "image";
  }

  const textExts = ["txt", "md", "json", "xml", "yaml", "yml", "csv", "log"];
  if (textExts.includes(ext)) {
    return "text";
  }

  return "unknown";
}
