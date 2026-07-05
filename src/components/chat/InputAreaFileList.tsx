// SPDX-License-Identifier: AGPL-3.0-only

import { Image } from "antd";
import type { GlobalToken } from "antd";
import { Trash2 } from "lucide-react";
import { formatFileSize, getFileIcon, getFileTypeCategory } from "./InputAreaUtils";

export function InputAreaFileList({
  attachedFiles,
  attachmentObjectUrls,
  removeFile,
  token,
}: {
  attachedFiles: File[];
  attachmentObjectUrls: string[];
  removeFile: (index: number) => void;
  token: GlobalToken;
}) {
  if (attachedFiles.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-2 mb-2">
      {attachedFiles.map((file, idx) => {
        const fileCategory = getFileTypeCategory(file.type);
        const isImage = fileCategory === "image";
        const isPreviewable = isImage
          && file.type !== "image/gif"
          && file.type !== "image/svg+xml";

        return (
          <div
            key={`${file.name}-${file.size}-${file.lastModified}`}
            className="relative group"
            style={{
              backgroundColor: token.colorFillTertiary,
              borderRadius: token.borderRadius,
              border: `1px solid ${token.colorBorderSecondary}`,
              overflow: "hidden",
              maxWidth: isImage ? 120 : 200,
            }}
          >
            {isImage && (
              <div
                style={{
                  width: 120,
                  height: 80,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  backgroundColor: token.colorFillSecondary,
                  overflow: "hidden",
                }}
              >
                {isPreviewable
                  ? (
                    <Image
                      src={attachmentObjectUrls[idx]}
                      alt={file.name}
                      style={{
                        width: "100%",
                        height: "100%",
                        objectFit: "cover",
                      }}
                      preview={{ mask: { blur: true }, scaleStep: 0.5 }}
                    />
                  )
                  : (
                    <img
                      src={attachmentObjectUrls[idx]}
                      alt={file.name}
                      style={{
                        width: "100%",
                        height: "100%",
                        objectFit: "cover",
                      }}
                    />
                  )}
              </div>
            )}
            <div
              className={`flex items-center gap-1.5 px-2 py-1 ${isImage ? "" : ""}`}
              style={!isImage ? { maxWidth: 200 } : undefined}
            >
              {!isImage && (
                <span style={{ color: token.colorPrimary, flexShrink: 0 }}>
                  {getFileIcon(fileCategory)}
                </span>
              )}
              <span
                className="text-xs truncate"
                style={{
                  color: token.colorText,
                  flex: 1,
                  maxWidth: isImage ? 100 : 140,
                }}
                title={file.name}
              >
                {file.name}
              </span>
              <span
                className="text-xs"
                style={{ color: token.colorTextSecondary, flexShrink: 0 }}
              >
                {formatFileSize(file.size)}
              </span>
              <Trash2
                size={14}
                className="cursor-pointer shrink-0"
                style={{ color: token.colorTextSecondary }}
                onClick={() => removeFile(idx)}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}
