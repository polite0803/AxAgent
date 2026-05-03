import { Button, Space, Tag, theme, Tooltip, Typography } from "antd";
import {
  Check,
  FileCode,
  FileDiff,
  GitBranch,
  Minus,
  Plus,
  X,
} from "lucide-react";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

// ── Types ────────────────────────────────────────────────────────────────

export interface FileChange {
  filePath: string;
  originalContent: string;
  modifiedContent: string;
  operation: "write" | "edit" | "delete";
  language?: string;
}

export interface FileChangeReview {
  changes: FileChange[];
  onAccept: (filePath: string) => void;
  onReject: (filePath: string) => void;
  onAcceptAll: () => void;
  onRejectAll: () => void;
}

// ── Language detection ───────────────────────────────────────────────────

function detectLanguage(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase();
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    py: "python",
    rs: "rust",
    go: "go",
    java: "java",
    css: "css",
    html: "html",
    json: "json",
    md: "markdown",
    svg: "xml",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    sql: "sql",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
  };
  return map[ext ?? ""] ?? "plaintext";
}

// ── Monaco Diff Editor ───────────────────────────────────────────────────

interface MonacoDiffEditorProps {
  original: string;
  modified: string;
  language: string;
  height?: number;
  readOnly?: boolean;
}

declare global {
  interface Window {
    monaco: typeof import("monaco-editor");
  }
}

function MonacoDiffEditor({
  original,
  modified,
  language,
  height = 300,
  readOnly = true,
}: MonacoDiffEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<import("monaco-editor").editor.IStandaloneDiffEditor | null>(null);

  useEffect(() => {
    if (!containerRef.current || typeof window.monaco === "undefined") { return; }

    const diffEditor = window.monaco.editor.createDiffEditor(containerRef.current, {
      theme: "vs-dark",
      readOnly,
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 12,
      lineNumbers: "on",
      scrollBeyondLastLine: false,
      wordWrap: "on",
      padding: { top: 8 },
      renderSideBySide: true,
      originalEditable: false,
    });

    const originalModel = window.monaco.editor.createModel(original, language);
    const modifiedModel = window.monaco.editor.createModel(modified, language);
    diffEditor.setModel({ original: originalModel, modified: modifiedModel });

    editorRef.current = diffEditor;

    return () => {
      originalModel.dispose();
      modifiedModel.dispose();
      diffEditor.dispose();
    };
  }, []);

  useEffect(() => {
    if (editorRef.current) {
      const models = editorRef.current.getModel();
      if (models) {
        if (models.original.getValue() !== original) {
          models.original.setValue(original);
        }
        if (models.modified.getValue() !== modified) {
          models.modified.setValue(modified);
        }
      }
    }
  }, [original, modified]);

  return (
    <div
      ref={containerRef}
      style={{ height, width: "100%", border: "1px solid var(--border-color)", borderRadius: 8, overflow: "hidden" }}
    />
  );
}

// ── Diff Stat Bar ────────────────────────────────────────────────────────

function DiffStatBar({ original, modified }: { original: string; modified: string }) {
  const { token } = theme.useToken();
  const stats = useMemo(() => {
    const origLines = original.split("\n");
    const modLines = modified.split("\n");

    let additions = 0;
    let deletions = 0;
    const maxLen = Math.max(origLines.length, modLines.length);
    const minLen = Math.min(origLines.length, modLines.length);

    for (let i = 0; i < minLen; i++) {
      if (origLines[i] !== modLines[i]) {
        additions++;
        deletions++;
      }
    }
    if (modLines.length > origLines.length) {
      additions += modLines.length - origLines.length;
    }
    if (origLines.length > modLines.length) {
      deletions += origLines.length - modLines.length;
    }

    return { additions, deletions, total: maxLen };
  }, [original, modified]);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: 12 }}>
      <span style={{ display: "flex", alignItems: "center", gap: 3, color: token.colorSuccess }}>
        <Plus size={12} /> {stats.additions}
      </span>
      <span style={{ display: "flex", alignItems: "center", gap: 3, color: token.colorError }}>
        <Minus size={12} /> {stats.deletions}
      </span>
      <span style={{ color: token.colorTextSecondary }}>
        {stats.total} 行
      </span>
    </div>
  );
}

// ── FileChangeCard ───────────────────────────────────────────────────────

interface FileChangeCardProps {
  change: FileChange;
  onAccept?: (filePath: string) => void;
  onReject?: (filePath: string) => void;
  defaultExpanded?: boolean;
}

export const FileChangeCard = React.memo(function FileChangeCard({
  change,
  onAccept,
  onReject,
  defaultExpanded = false,
}: FileChangeCardProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [expanded, setExpanded] = useState(defaultExpanded);
  const [status, setStatus] = useState<"pending" | "accepted" | "rejected">("pending");

  const lang = change.language ?? detectLanguage(change.filePath);
  const isNew = change.operation === "write" && !change.originalContent;
  const isDeleted = change.operation === "delete";

  const handleAccept = () => {
    setStatus("accepted");
    onAccept?.(change.filePath);
  };

  const handleReject = () => {
    setStatus("rejected");
    onReject?.(change.filePath);
  };

  return (
    <div
      style={{
        border: `1px solid ${status === "accepted" ? token.colorSuccess : status === "rejected" ? token.colorError : token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        marginBottom: 8,
        overflow: "hidden",
        opacity: status === "rejected" ? 0.5 : 1,
        transition: "opacity 0.2s, border-color 0.2s",
      }}
    >
      {/* Header */}
      <div
        role="button"
        tabIndex={0}
        aria-expanded={expanded}
        aria-label={`${change.filePath} - ${isNew ? "新建" : isDeleted ? "删除" : "修改"}`}
        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setExpanded(!expanded); } }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          backgroundColor: token.colorFillQuaternary,
          borderBottom: expanded ? `1px solid ${token.colorBorderSecondary}` : "none",
          cursor: "pointer",
        }}
        onClick={() => setExpanded(!expanded)}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isNew
            ? <FileCode size={14} style={{ color: token.colorSuccess }} />
            : isDeleted
            ? <FileCode size={14} style={{ color: token.colorError }} />
            : <FileDiff size={14} style={{ color: token.colorWarning }} />}
          <Typography.Text style={{ fontSize: 13, fontFamily: "monospace" }}>
            {change.filePath}
          </Typography.Text>
          {isNew && (
            <Tag color="green" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>新建</Tag>
          )}
          {isDeleted && (
            <Tag color="red" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>删除</Tag>
          )}
          {change.operation === "edit" && (
            <Tag color="orange" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>修改</Tag>
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {!isDeleted && !isNew && (
            <DiffStatBar original={change.originalContent} modified={change.modifiedContent} />
          )}
          {status === "pending" && (
            <Space size={4} onClick={(e) => e.stopPropagation()}>
              <Tooltip title={t("chat.diff.accept")}>
                <Button
                  size="small"
                  type="primary"
                  icon={<Check size={12} />}
                  onClick={handleAccept}
                />
              </Tooltip>
              <Tooltip title={t("chat.diff.reject")}>
                <Button
                  size="small"
                  icon={<X size={12} />}
                  onClick={handleReject}
                  danger
                />
              </Tooltip>
            </Space>
          )}
          {status === "accepted" && (
            <Tag color="green" style={{ margin: 0 }}>
              <Check size={10} /> {t("chat.diff.accepted")}
            </Tag>
          )}
          {status === "rejected" && (
            <Tag color="red" style={{ margin: 0 }}>
              <X size={10} /> {t("chat.diff.rejected")}
            </Tag>
          )}
        </div>
      </div>

      {/* Diff Content */}
      {expanded && !isDeleted && (
        <div style={{ padding: 4 }}>
          {isNew ? (
            <div
              style={{
                padding: 12,
                backgroundColor: token.colorFillQuaternary,
                borderRadius: token.borderRadiusSM,
                maxHeight: 300,
                overflow: "auto",
                fontSize: 12,
                fontFamily: "monospace",
                whiteSpace: "pre-wrap",
              }}
            >
              <div
                style={{
                  color: token.colorSuccess,
                  marginBottom: 8,
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                }}
              >
                <Plus size={12} /> 新文件
              </div>
              {change.modifiedContent}
            </div>
          ) : (
            <MonacoDiffEditor
              original={change.originalContent}
              modified={change.modifiedContent}
              language={lang}
              height={Math.min(400, Math.max(150, change.modifiedContent.split("\n").length * 22))}
            />
          )}
        </div>
      )}
      {expanded && isDeleted && (
        <div
          style={{
            padding: 12,
            backgroundColor: token.colorFillQuaternary,
            borderRadius: token.borderRadiusSM,
            maxHeight: 300,
            overflow: "auto",
            fontSize: 12,
            fontFamily: "monospace",
            whiteSpace: "pre-wrap",
            color: token.colorError,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 8 }}>
            <Minus size={12} /> 已删除内容
          </div>
          {change.originalContent}
        </div>
      )}
    </div>
  );
});

// ── FileChangeList ───────────────────────────────────────────────────────

interface FileChangeListProps {
  changes: FileChange[];
  onAccept?: (filePath: string) => void;
  onReject?: (filePath: string) => void;
  onAcceptAll?: () => void;
  onRejectAll?: () => void;
}

export const FileChangeList = React.memo(function FileChangeList({
  changes,
  onAccept,
  onReject,
  onAcceptAll,
}: FileChangeListProps) {
  const { t } = useTranslation();
  const [expandedAll, setExpandedAll] = useState(false);

  if (changes.length === 0) { return null; }

  return (
    <div style={{ marginTop: 8 }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 8,
          padding: "4px 0",
        }}
      >
        <Typography.Text strong style={{ fontSize: 13, display: "flex", alignItems: "center", gap: 6 }}>
          <GitBranch size={14} />
          {changes.length} {t("chat.diff.fileChanges", "个文件变更")}
        </Typography.Text>
        <Space size={4}>
          <Button
            size="small"
            type="text"
            onClick={() => setExpandedAll(!expandedAll)}
          >
            {expandedAll ? t("chat.diff.collapseAll") : t("chat.diff.expandAll")}
          </Button>
          {onAcceptAll && (
            <Button size="small" type="primary" icon={<Check size={12} />} onClick={onAcceptAll}>
              {t("chat.diff.acceptAll")}
            </Button>
          )}
        </Space>
      </div>
      {changes.map((change) => (
        <FileChangeCard
          key={change.filePath}
          change={change}
          onAccept={onAccept}
          onReject={onReject}
          defaultExpanded={expandedAll}
        />
      ))}
    </div>
  );
});

// ── Utility: extract file changes from tool call ─────────────────────────

export function extractFileChanges(toolCalls: { toolName: string; input: Record<string, unknown>; output?: string }[]): FileChange[] {
  const changes: FileChange[] = [];

  for (const tc of toolCalls) {
    const lower = tc.toolName.toLowerCase();
    if (!lower.includes("write") && !lower.includes("edit") && !lower.includes("delete")) {
      continue;
    }

    const filePath = (tc.input.file_path ?? tc.input.path ?? tc.input.filePath ?? "") as string;
    if (!filePath) { continue; }

    const modifiedContent = (tc.input.content ?? tc.input.contents ?? tc.input.text ?? "") as string;
    const originalContent = (tc.input.original_content ?? tc.input.old_content ?? tc.input.old_str ?? "") as string;

    changes.push({
      filePath,
      originalContent,
      modifiedContent: modifiedContent || (tc.output ?? ""),
      operation: originalContent ? "edit" : modifiedContent ? "write" : "delete",
    });
  }

  return changes;
}
