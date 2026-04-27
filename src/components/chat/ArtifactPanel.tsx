import { useState, useMemo } from "react";
import { Card, Button, Segmented, Space, Tooltip, Dropdown, message } from "antd";
import {
  CopyOutlined,
  ExpandOutlined,
  CheckOutlined,
  CodeOutlined,
  EyeOutlined,
  ColumnWidthOutlined,
  MoreOutlined,
} from "@ant-design/icons";
import type { ArtifactFormat, ArtifactPreviewMode } from "@/types/artifact";
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
  const [copied, setCopied] = useState(false);
  const [currentMode, setCurrentMode] = useState<ArtifactPreviewMode>(previewMode);

  const canPreview = useMemo(() => {
    if (!artifact) return false;
    return ["html", "css", "javascript", "jsx", "tsx", "svg", "mermaid", "d2"].includes(artifact.format);
  }, [artifact]);

  const handleCopy = async () => {
    if (!artifact) return;
    try {
      await navigator.clipboard.writeText(artifact.content);
      setCopied(true);
      message.success("Copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
    } catch {
      message.error("Failed to copy");
    }
  };

  const handleDownload = () => {
    if (!artifact) return;
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
          No artifact selected
        </div>
      </Card>
    );
  }

  const menuItems = [
    { key: "copy", label: "Copy code" },
    { key: "download", label: "Download" },
    { key: "fullscreen", label: "Fullscreen" },
  ];

  return (
    <Card
      size="small"
      title={
        <Space>
          <span>{artifact.title || "Untitled"}</span>
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
              { value: "code", icon: <CodeOutlined />, label: "Code" },
              { value: "split", icon: <ColumnWidthOutlined />, label: "Split" },
              { value: "preview", icon: <EyeOutlined />, label: "Preview" },
            ]}
          />
          <Tooltip title={copied ? "Copied!" : "Copy code"}>
            <Button size="small" icon={copied ? <CheckOutlined /> : <CopyOutlined />} onClick={handleCopy} />
          </Tooltip>
          <Tooltip title="Fullscreen">
            <Button size="small" icon={<ExpandOutlined />} onClick={onFullscreen} />
          </Tooltip>
          <Dropdown
            menu={{
              items: menuItems,
              onClick: ({ key }) => {
                if (key === "copy") handleCopy();
                else if (key === "download") handleDownload();
                else if (key === "fullscreen") onFullscreen?.();
              },
            }}
          >
            <Button size="small" icon={<MoreOutlined />} />
          </Dropdown>
        </Space>
      }
      styles={{ body: { padding: 0, height: "calc(100% - 57px)" } }}
    >
      <div style={{ display: "flex", height: "100%" }}>
        {currentMode === "code" && (
          <div style={{ width: "100%", overflow: "auto", padding: 16 }}>
            <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
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
            <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
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
              <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-word", fontSize: 13 }}>
                {artifact.content}
              </pre>
            </div>
            <div style={{ width: "50%", overflow: "auto" }}>
              {canPreview ? (
                <ArtifactPreview code={artifact.content} format={artifact.format} />
              ) : (
                <div style={{ padding: 16, textAlign: "center", color: "#999" }}>
                  Preview not available for this format
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </Card>
  );
}
