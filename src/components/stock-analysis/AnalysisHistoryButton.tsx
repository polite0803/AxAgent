import { invoke } from "@/lib/invoke";
import { App, Button, Dropdown, Input } from "antd";
import { Check, History, Pencil, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface AnalysisRecord {
  id: string;
  stockCode: string;
  stockName: string;
  analysisDate: string;
  decisionJson: string | null;
  blackboardSnapshot: string | null;
  createdAt: number;
  status: string;
}

/** 个股分析页搜索框下方的历史分析快捷按钮 */
export function AnalysisHistoryButton() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [records, setRecords] = useState<AnalysisRecord[]>([]);
  const [open, setOpen] = useState(false);
  // editingId !== null 表示正在重命名该记录
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) { return; }
    let cancelled = false;
    invoke<AnalysisRecord[]>("list_stock_analyses", { limit: 30, offset: 0 }).then((list) => {
      if (cancelled || !Array.isArray(list)) { return; }
      setRecords(list);
    }).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(() => {
    if (editingId && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingId]);

  const handleDelete = useCallback((e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    void invoke("delete_stock_analysis", { analysisId: id })
      .then(() => {
        setRecords((prev) => prev.filter((r) => r.id !== id));
        message.success(t("stockAnalysis.deleteSuccess"));
      })
      .catch(() => {
        message.error(t("stockAnalysis.deleteFailed"));
      });
  }, [t, message]);

  const startRename = useCallback((e: React.MouseEvent, r: AnalysisRecord) => {
    e.stopPropagation();
    setEditingId(r.id);
    setEditValue(r.stockName);
  }, []);

  const confirmRename = useCallback((e: React.MouseEvent | KeyboardEvent, id: string) => {
    e.stopPropagation?.();
    const newName = editValue.trim();
    if (!newName) { return; }
    void invoke("rename_stock_analysis", { analysisId: id, newName })
      .then(() => {
        setRecords((prev) => prev.map((r) => r.id === id ? { ...r, stockName: newName } : r));
        setEditingId(null);
        message.success(t("stockAnalysis.renameSuccess"));
      })
      .catch(() => {
        message.error(t("stockAnalysis.renameFailed"));
      });
  }, [editValue, t, message]);

  const cancelRename = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingId(null);
  }, []);

  const displayLabel = (r: AnalysisRecord) => {
    const date = r.analysisDate || (r.createdAt ? new Date(r.createdAt).toLocaleDateString() : "");
    return date ? `${r.stockName} (${date})` : r.stockName;
  };

  return (
    <Dropdown
      open={open}
      onOpenChange={setOpen}
      trigger={["click"]}
      popupRender={() => (
        <div
          style={{
            width: 440,
            maxHeight: 400,
            overflowY: "auto",
            background: "var(--surface, #1a1a2e)",
            border: "1px solid var(--color-border, #333)",
            borderRadius: 8,
            padding: 4,
          }}
        >
          {records.length === 0
            ? (
              <div style={{ padding: "12px 8px", color: "var(--muted, #888)", textAlign: "center", fontSize: 12 }}>
                {t("stockAnalysis.noHistory")}
              </div>
            )
            : records.map((r) => (
              <div
                key={r.id}
                style={{
                  padding: "6px 10px",
                  cursor: "pointer",
                  borderRadius: 4,
                  fontSize: 13,
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}
                onClick={() => {
                  if (editingId) { return; }
                  setOpen(false);
                  navigate(`/stock-analysis/${r.id}`);
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "var(--hover, rgba(255,255,255,0.06))";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = "transparent";
                }}
              >
                {/* 名称：编辑模式 vs 显示模式 */}
                {editingId === r.id
                  ? (
                    <div
                      style={{ flex: 1, display: "flex", alignItems: "center", gap: 4 }}
                      onClick={(e) => e.stopPropagation()}
                    >
                      <Input
                        // eslint-disable-next-line @typescript-eslint/no-explicit-any
                        ref={inputRef as any}
                        size="small"
                        value={editValue}
                        onChange={(e) => setEditValue(e.target.value)}
                        onKeyDown={(e) => {
                          // eslint-disable-next-line @typescript-eslint/no-explicit-any
                          if (e.key === "Enter") { confirmRename(e as any, r.id); }
                          if (e.key === "Escape") { setEditingId(null); }
                        }}
                        style={{ width: 200, height: 26, fontSize: 12 }}
                      />
                      <span
                        style={{ cursor: "pointer", display: "inline-flex", color: "var(--sa-green, #22c55e)" }}
                        onClick={(e) => confirmRename(e, r.id)}
                      >
                        <Check size={14} />
                      </span>
                      <span
                        style={{ cursor: "pointer", display: "inline-flex", color: "var(--muted, #888)" }}
                        onClick={cancelRename}
                      >
                        <X size={14} />
                      </span>
                    </div>
                  )
                  : (
                    <span
                      style={{
                        fontWeight: 500,
                        flex: 1,
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                    >
                      {displayLabel(r)}
                    </span>
                  )}

                <span style={{ color: "var(--muted, #888)", fontSize: 11 }}>{r.stockCode}</span>
                {r.status !== "completed"
                  ? (
                    <span style={{ color: "var(--sa-red, #ef4444)", fontSize: 10, marginRight: 4 }}>
                      {r.status}
                    </span>
                  )
                  : <span style={{ color: "var(--sa-green, #22c55e)", fontSize: 10, marginRight: 4 }}>✓</span>}
                {/* 重命名按钮 */}
                <span
                  title={t("stockAnalysis.rename")}
                  onClick={(e) => startRename(e, r)}
                  style={{
                    padding: "2px 4px",
                    borderRadius: 4,
                    cursor: "pointer",
                    color: "var(--muted, #888)",
                    display: "inline-flex",
                    alignItems: "center",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.color = "var(--accent, #7c3aed)";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.color = "var(--muted, #888)";
                  }}
                  onMouseDown={(e) => e.stopPropagation()}
                >
                  <Pencil size={12} />
                </span>
                {/* 删除按钮 */}
                <span
                  title={t("stockAnalysis.deleteAnalysis")}
                  onClick={(e) => handleDelete(e, r.id)}
                  style={{
                    padding: "2px 4px",
                    borderRadius: 4,
                    cursor: "pointer",
                    color: "var(--muted, #888)",
                    display: "inline-flex",
                    alignItems: "center",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.color = "var(--sa-red, #ef4444)";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.color = "var(--muted, #888)";
                  }}
                  onMouseDown={(e) => e.stopPropagation()}
                >
                  <Trash2 size={12} />
                </span>
              </div>
            ))}
        </div>
      )}
    >
      <Button size="small" type="text" style={{ fontSize: 12, color: "var(--muted, #888)" }}>
        <History size={13} style={{ marginRight: 4 }} />
        {t("stockAnalysis.history")}
        {records.length > 0 && <span style={{ marginLeft: 4, fontSize: 11, opacity: 0.6 }}>({records.length})</span>}
      </Button>
    </Dropdown>
  );
}
