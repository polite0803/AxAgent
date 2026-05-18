import { useWikiStore } from "@/stores/feature/wikiStore";
import type { NoteVersion } from "@/types";
import { HistoryOutlined, RollbackOutlined } from "@ant-design/icons";
import { Button, Drawer, Empty, List, message, Popconfirm, Spin, theme, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface VersionHistoryPanelProps {
  noteId: string;
  open: boolean;
  onClose: () => void;
  onRestore?: () => void;
}

function computeDiffLines(oldText: string, newText: string) {
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");

  const result: { type: "unchanged" | "added" | "removed"; line: string }[] = [];
  const maxLen = Math.max(oldLines.length, newLines.length);

  for (let i = 0; i < maxLen; i++) {
    const oldLine = i < oldLines.length ? oldLines[i] : undefined;
    const newLine = i < newLines.length ? newLines[i] : undefined;

    if (oldLine === newLine) {
      result.push({ type: "unchanged", line: oldLine! });
    } else {
      if (oldLine !== undefined) {
        result.push({ type: "removed", line: oldLine });
      }
      if (newLine !== undefined) {
        result.push({ type: "added", line: newLine });
      }
    }
  }

  return result;
}

function DiffView({ oldContent, newContent }: { oldContent: string; newContent: string }) {
  const { token } = theme.useToken();
  const diffLines = computeDiffLines(oldContent, newContent);

  return (
    <div
      className="text-xs font-mono overflow-auto"
      style={{
        maxHeight: 300,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        padding: 8,
      }}
    >
      {/* static diff lines from computed text comparison, safe to use index as key */}
      {diffLines.map((dl, i) => (
        <div
          key={`${dl.type}-${i}`}
          style={{
            backgroundColor: dl.type === "added"
              ? token.colorSuccessBg
              : dl.type === "removed"
              ? token.colorErrorBg
              : "transparent",
            textDecoration: dl.type === "removed" ? "line-through" : "none",
            opacity: dl.type === "removed" ? 0.7 : 1,
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
        >
          <span style={{ display: "inline-block", width: 16, color: token.colorTextQuaternary }}>
            {dl.type === "added" ? "+" : dl.type === "removed" ? "-" : " "}
          </span>
          {dl.line}
        </div>
      ))}
    </div>
  );
}

function formatTimestamp(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

function shortHash(hash: string): string {
  return hash.length > 8 ? hash.slice(0, 8) : hash;
}

export function VersionHistoryPanel({ noteId, open, onClose, onRestore }: VersionHistoryPanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { loadVersions, restoreVersion } = useWikiStore();

  const [versions, setVersions] = useState<NoteVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<NoteVersion | null>(null);
  const [diffVersion, setDiffVersion] = useState<NoteVersion | null>(null);
  const [restoring, setRestoring] = useState(false);

  const loadVersionList = useCallback(async () => {
    setLoading(true);
    const result = await loadVersions(noteId);
    setVersions(result);
    setLoading(false);
  }, [noteId, loadVersions]);

  useEffect(() => {
    if (open && noteId) {
      loadVersionList();
      setSelectedVersion(null);
      setDiffVersion(null);
    }
  }, [open, noteId, loadVersionList]);

  const handleRestore = async (versionId: number) => {
    setRestoring(true);
    const updated = await restoreVersion(noteId, versionId);
    setRestoring(false);
    if (updated) {
      message.success(t("wiki.versionRestored"));
      onRestore?.();
      loadVersionList();
    }
  };

  const handleSelectVersion = (version: NoteVersion) => {
    if (selectedVersion?.id === version.id) {
      setSelectedVersion(null);
      setDiffVersion(null);
    } else {
      setSelectedVersion(version);
      setDiffVersion(null);
    }
  };

  const handleDiff = (version: NoteVersion) => {
    if (diffVersion?.id === version.id) {
      setDiffVersion(null);
    } else {
      setDiffVersion(version);
    }
  };

  return (
    <Drawer
      title={
        <span>
          <HistoryOutlined style={{ marginRight: 8 }} />
          {t("wiki.versionHistory")}
        </span>
      }
      open={open}
      onClose={onClose}
      width={520}
      styles={{ body: { padding: 0 } }}
    >
      {loading
        ? (
          <div className="flex items-center justify-center py-12">
            <Spin size="large" />
          </div>
        )
        : versions.length === 0
        ? (
          <div className="py-12">
            <Empty description={t("wiki.noVersions")} />
          </div>
        )
        : (
          <div className="flex flex-col h-full">
            <List
              className="flex-1 overflow-auto"
              dataSource={versions}
              renderItem={(version) => (
                <List.Item
                  style={{
                    cursor: "pointer",
                    backgroundColor: selectedVersion?.id === version.id ? token.colorPrimaryBg : "transparent",
                    padding: "8px 16px",
                    borderLeft: selectedVersion?.id === version.id
                      ? `3px solid ${token.colorPrimary}`
                      : "3px solid transparent",
                  }}
                  onClick={() => handleSelectVersion(version)}
                  actions={[
                    <Tooltip key="diff" title={t("wiki.compareDiff")}>
                      <Button
                        size="small"
                        type={diffVersion?.id === version.id ? "primary" : "text"}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDiff(version);
                        }}
                      >
                        Diff
                      </Button>
                    </Tooltip>,
                    <Popconfirm
                      key="restore"
                      title={t("wiki.confirmRestore")}
                      onConfirm={() => handleRestore(version.id)}
                      okText={t("wiki.restore")}
                    >
                      <Button
                        size="small"
                        icon={<RollbackOutlined />}
                        loading={restoring}
                        onClick={(e) => e.stopPropagation()}
                      >
                        {t("wiki.restore")}
                      </Button>
                    </Popconfirm>,
                  ]}
                >
                  <List.Item.Meta
                    title={
                      <span className="text-sm">
                        <Text type="secondary" className="text-xs">
                          {formatTimestamp(version.createdAt)}
                        </Text>
                        <Text
                          className="ml-2 text-xs font-mono"
                          style={{ color: token.colorTextQuaternary }}
                        >
                          {shortHash(version.contentHash)}
                        </Text>
                      </span>
                    }
                    description={
                      <span className="text-xs">
                        {version.author} &middot; {version.title}
                      </span>
                    }
                  />
                </List.Item>
              )}
            />

            {selectedVersion && !diffVersion && (
              <div
                className="border-t p-3"
                style={{
                  maxHeight: 300,
                  overflow: "auto",
                  backgroundColor: token.colorBgContainer,
                  borderColor: token.colorBorderSecondary,
                }}
              >
                <Text className="text-xs font-mono" style={{ whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                  {selectedVersion.content}
                </Text>
              </div>
            )}

            {diffVersion && selectedVersion && (
              <div className="border-t p-3" style={{ borderColor: token.colorBorderSecondary }}>
                <Text className="text-xs mb-2 block" type="secondary">
                  {t("wiki.diffLabel")}: {shortHash(diffVersion.contentHash)} → {shortHash(selectedVersion.contentHash)}
                </Text>
                <DiffView oldContent={diffVersion.content} newContent={selectedVersion.content} />
              </div>
            )}
          </div>
        )}
    </Drawer>
  );
}
