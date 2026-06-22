// SPDX-License-Identifier: AGPL-3.0-only

import { type DropdownItem, DropdownMenu } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import type { ArtifactFormat, ArtifactPreviewMode } from "@/types";
import {
  CheckOutlined,
  CodeOutlined,
  ColumnWidthOutlined,
  CopyOutlined,
  ExpandOutlined,
  EyeOutlined,
  MoreOutlined,
} from "@ant-design/icons";
import { Button, Card, message, Segmented, Space } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArtifactPreview } from "./ArtifactPreview";

interface ArtifactPanelProps {
  artifact?: {
    id: string;
    title: string;
    kind: string;
    content: string;
    format: ArtifactFormat;
  };
  previewMode?: ArtifactPreviewMode;
  onPreviewModeChange?: (mode: ArtifactPreviewMode) => void;
  onFullscreen?: () => void;
}

export function ArtifactPanel({
  artifact,
  previewMode = "split",
  onPreviewModeChange,
  onFullscreen,
}: ArtifactPanelProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [currentMode, setCurrentMode] = useState<ArtifactPreviewMode>(previewMode);

  const canPreview = useMemo(() => {
    if (!artifact) {
      return false;
    }
    return [
      "html",
      "css",
      "javascript",
      "jsx",
      "tsx",
      "svg",
      "mermaid",
      "d2",
    ].includes(artifact.format);
  }, [artifact]);

  const handleCopy = async () => {
    if (!artifact) {
      return;
    }
    try {
      await navigator.clipboard.writeText(artifact.content);
      setCopied(true);
      message.success(t("artifact.copiedToClipboard"));
      setTimeout(() => setCopied(false), 2000);
    } catch {
      message.error(t("artifact.copyFailed"));
    }
  };

  const handleDownload = () => {
    if (!artifact) {
      return;
    }
    const blob = new Blob([artifact.content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${artifact.title || "artifact"}.${artifact.format}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleModeChange = (mode: ArtifactPreviewMode) => {
    setCurrentMode(mode);
    onPreviewModeChange?.(mode);
  };

  if (!artifact) {
    return (
      <Card size="small">
        <div style={{ textAlign: "center", padding: "40px 0", color: "#999" }}>
          {t("artifactPanel.noArtifactSelected")}
        </div>
      </Card>
    );
  }

  const menuItems: DropdownItem[] = [
    { key: "copy", label: t("artifactPanel.copyCode"), onClick: handleCopy },
    { key: "download", label: t("artifactPanel.download"), onClick: handleDownload },
    { key: "fullscreen", label: t("artifactPanel.fullscreen"), onClick: onFullscreen },
  ];

  return (
    <Card
      size="small"
      title={
        <Space>
          <span>{artifact.title || t("artifactPanel.untitled")}</span>
          <span style={{ fontSize: 12, color: "#999" }}>{artifact.kind}</span>
          <span style={{ fontSize: 12, color: "#999" }}>{artifact.format}</span>
        </Space>
      }
      extra={
        <Space>
          <Segmented
            size="small"
            value={currentMode}
            onChange={(val) => handleModeChange(val as ArtifactPreviewMode)}
            options={[
              { value: "code", icon: <CodeOutlined />, label: t("artifactPanel.segmentedCode") },
              { value: "split", icon: <ColumnWidthOutlined />, label: t("artifactPanel.segmentedSplit") },
              { value: "preview", icon: <EyeOutlined />, label: t("artifactPanel.segmentedPreview") },
            ]}
          />
          <Tooltip title={copied ? t("artifactPanel.copied") : t("artifactPanel.copyCode")}>
            <Button
              size="small"
              icon={copied ? <CheckOutlined /> : <CopyOutlined />}
              onClick={handleCopy}
            />
          </Tooltip>
          <Tooltip title={t("artifactPanel.fullscreen")}>
            <Button
              size="small"
              icon={<ExpandOutlined />}
              onClick={onFullscreen}
            />
          </Tooltip>
          <DropdownMenu items={menuItems}>
            <Button size="small" icon={<MoreOutlined />} />
          </DropdownMenu>
        </Space>
      }
      styles={{ body: { padding: 0, height: "calc(100% - 57px)" } }}
    >
      <div style={{ display: "flex", height: "100%" }}>
        {currentMode === "code" && (
          <div style={{ width: "100%", overflow: "auto", padding: 16 }}>
            <pre
              style={{
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {artifact.content}
            </pre>
          </div>
        )}

        {currentMode === "preview" && canPreview && (
          <div style={{ width: "100%", height: "100%" }}>
            <ArtifactPreview code={artifact.content} format={artifact.format} />
          </div>
        )}

        {currentMode === "preview" && !canPreview && (
          <div style={{ width: "100%", overflow: "auto", padding: 16 }}>
            <pre
              style={{
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {artifact.content}
            </pre>
          </div>
        )}

        {currentMode === "split" && (
          <>
            <div
              style={{
                width: "50%",
                overflow: "auto",
                borderRight: "1px solid #f0f0f0",
                padding: 16,
                background: "#fafafa",
              }}
            >
              <pre
                style={{
                  margin: 0,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                  fontSize: 13,
                }}
              >
                {artifact.content}
              </pre>
            </div>
            <div style={{ width: "50%", overflow: "auto" }}>
              {canPreview
                ? (
                  <ArtifactPreview
                    code={artifact.content}
                    format={artifact.format}
                  />
                )
                : (
                  <div
                    style={{ padding: 16, textAlign: "center", color: "#999" }}
                  >
                    {t("artifactPanel.previewNotAvailable")}
                  </div>
                )}
            </div>
          </>
        )}
      </div>
    </Card>
  );
}
