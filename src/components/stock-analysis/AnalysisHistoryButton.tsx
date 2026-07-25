import { invoke } from "@/lib/invoke";
import { getActionTagStyle, getActionTKey, parseAction } from "@/lib/stock-analysis-utils";
import { App, Button, Dropdown, Input, Tag } from "antd";
import { Check, History, Pencil, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface AnalysisRecord {
  id: string;
  stockCode: string;
  stockName: string;
  analysisDate: string;
  /** 决策动作（后端直返，如 BUY/SELL/HOLD/WAIT/UNCERTAIN） */
  decisionAction: string | null;
  /** 决策仓位百分比（后端直返，0-100） */
  decisionPositionPct: number | null;
  /** 完整决策 JSON（含 confidence 等，部分旧数据可能为 null） */
  decisionJson: string | null;
  /** 列表场景不返回，详情页通过 get_stock_analysis 单独获取 */
  blackboardSnapshot?: string | null;
  createdAt: number;
  updatedAt?: number;
  status: string;
  /** 版本化分析：指向原始记录 ID，null 表示首次分析 */
  parentAnalysisId: string | null;
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

  // 按 stockCode 分组：组名 "股票名称(股票代码)"，组内按时间倒序
  const grouped = useMemo(() => {
    const map = new Map<string, { stockName: string; stockCode: string; items: AnalysisRecord[] }>();
    for (const r of records) {
      if (!map.has(r.stockCode)) {
        map.set(r.stockCode, { stockName: r.stockName, stockCode: r.stockCode, items: [] });
      }
      map.get(r.stockCode)!.items.push(r);
    }
    for (const g of map.values()) {
      g.items.sort((a, b) => b.createdAt - a.createdAt);
    }
    return Array.from(map.values());
  }, [records]);

  // 记录名：日期（精确到日期），重跑版本加 ↻ 标记
  const dateLabel = (r: AnalysisRecord) => {
    const date = r.analysisDate || (r.createdAt ? new Date(r.createdAt).toLocaleDateString() : "");
    return r.parentAnalysisId ? `${date} ↻` : date;
  };

  // 分析时间：精确到分钟（HH:mm），用于区分同日多次分析
  const timeLabel = (r: AnalysisRecord) => {
    if (!r.createdAt) { return ""; }
    return new Date(r.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  // 解析决策信息：优先用后端直返字段（decisionAction / decisionPositionPct），
  // decisionJson 仅用于提取 confidence 等额外字段（兼容旧数据）。
  // 这样即使 decisionJson 为 null，只要 decisionAction 有值就能显示 Tag。
  const decisionInfo = (r: AnalysisRecord): {
    action: string;
    positionPct?: number;
    confidence?: number;
  } | null => {
    // 优先使用后端直返字段
    const action = r.decisionAction ? parseAction(r.decisionAction) : null;
    if (!action) {
      // 旧数据：从 decisionJson 解析
      if (!r.decisionJson) { return null; }
      try {
        const d = JSON.parse(r.decisionJson) as Record<string, unknown>;
        const a = parseAction(d.action as string);
        const positionPct = typeof d.positionPct === "number" ? d.positionPct : undefined;
        const confidence = typeof d.confidence === "number" ? d.confidence : undefined;
        return { action: a, positionPct, confidence };
      } catch {
        return null;
      }
    }
    // 新数据：直返字段 + decisionJson 补充 confidence
    let confidence: number | undefined;
    if (r.decisionJson) {
      try {
        const d = JSON.parse(r.decisionJson) as Record<string, unknown>;
        if (typeof d.confidence === "number") { confidence = d.confidence; }
      } catch { /* */ }
    }
    return {
      action,
      positionPct: r.decisionPositionPct ?? undefined,
      confidence,
    };
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
            : grouped.map((g) => (
              <div key={g.stockCode}>
                {/* 组标题：股票名称(股票代码) - 名称用主题色，代码用次要色 */}
                <div
                  style={{
                    padding: "4px 10px",
                    fontSize: 11,
                    fontWeight: 600,
                    borderBottom: "1px solid var(--color-border, #333)",
                    marginBottom: 2,
                    position: "sticky",
                    top: 0,
                    background: "var(--surface, #1a1a2e)",
                    zIndex: 1,
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                  }}
                >
                  <span style={{ color: "var(--accent, #7c3aed)" }}>{g.stockName}</span>
                  <span style={{ color: "var(--muted, #888)", fontSize: 10 }}>
                    ({g.stockCode})
                  </span>
                </div>
                {/* 组内记录：日期为记录名 */}
                {g.items.map((r) => (
                  <div
                    key={r.id}
                    style={{
                      padding: "6px 10px 6px 20px",
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
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div className="flex items-center gap-1">
                            <span
                              style={{
                                fontWeight: 500,
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                              }}
                            >
                              {dateLabel(r)}
                            </span>
                            {/* 决策结论 Tag */}
                            {(() => {
                              const info = decisionInfo(r);
                              if (!info) { return null; }
                              return (
                                <Tag
                                  style={getActionTagStyle(info.action)}
                                >
                                  {t(getActionTKey(info.action))}
                                </Tag>
                              );
                            })()}
                          </div>
                          {timeLabel(r) && (
                            <div style={{ fontSize: 10, color: "var(--muted, #888)", lineHeight: 1.2 }}>
                              {t("stockAnalysis.analysisTime")} {timeLabel(r)}
                            </div>
                          )}
                          {/* 重要要素：仓位 + 置信度 */}
                          {(() => {
                            const info = decisionInfo(r);
                            if (!info) { return null; }
                            const parts: string[] = [];
                            if (info.positionPct != null) {
                              parts.push(`${t("stockAnalysis.decision.positionPct")}${info.positionPct}%`);
                            }
                            if (info.confidence != null) {
                              parts.push(`${t("stockAnalysis.decision.confidence")}${info.confidence.toFixed(0)}%`);
                            }
                            if (parts.length === 0) { return null; }
                            return (
                              <div style={{ fontSize: 10, color: "var(--muted, #888)", lineHeight: 1.2 }}>
                                {parts.join(" · ")}
                              </div>
                            );
                          })()}
                        </div>
                      )}

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
