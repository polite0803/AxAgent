// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { App, Button, Card, Dropdown, Empty, Input, Popconfirm, Select, Space, Spin, Tag } from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface WatchlistItem {
  id: string;
  stockCode: string;
  stockName: string;
  group: string;
  notes?: string;
  createdAt: number;
}

interface QuoteSnapshot {
  price: number;
  changePct: number;
  pe?: number;
  timestamp: string;
}

const DEFAULT_GROUP = "__default__";

type SortKey = "name" | "change" | "code";

export function WatchlistPanel() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const store = useStockAnalysisStore();
  const stockCode = store.stockCode;
  const stockName = store.stockName;
  const getStockQuote = store.getStockQuote;
  const getStockKline = store.getStockKline;
  const startAnalysis = store.startAnalysis;
  const watchlistVersion = useStockAnalysisStore((s) => s.watchlistVersion);

  const [items, setItems] = useState<WatchlistItem[]>([]);
  const [quotes, setQuotes] = useState<Record<string, QuoteSnapshot>>({});
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey>("change");
  const [activeGroup, setActiveGroup] = useState(DEFAULT_GROUP);
  const [newGroup, setNewGroup] = useState("");
  const [editingGroup, setEditingGroup] = useState<string | null>(null);

  // 分组列表（从 DB settings 表）
  const groupsRef = useRef<string[]>([]);
  const [groupsVersion, setGroupsVersion] = useState(0);

  // 从 DB 加载分组
  const loadGroups = useCallback(async () => {
    try {
      const g = await invoke<string[]>("watchlist_list_groups");
      // 防御：后端可能返回 null（测试 mock 或异常路径），确保始终为数组
      groupsRef.current = Array.isArray(g) ? g : [];
      setGroupsVersion((v) => v + 1);
    } catch {
      groupsRef.current = [];
      setGroupsVersion((v) => v + 1);
    }
  }, []);

  useEffect(() => {
    loadGroups();
  }, [loadGroups]);

  const groups = useMemo(() => groupsRef.current ?? [], [groupsVersion]);

  const removeGroupAndReassign = async (g: string) => {
    try {
      const next = groupsRef.current.filter((x) => x !== g);
      await invoke("watchlist_save_groups", { groups: next });
      groupsRef.current = next;
      setGroupsVersion((v) => v + 1);
      setActiveGroup(DEFAULT_GROUP);
      setEditingGroup(null);
      // 将该分组下所有自选股移回默认分组
      const moving = items.filter((i) => i.group === g);
      for (const item of moving) {
        try {
          await invoke("watchlist_update_group", { id: item.id, groupName: DEFAULT_GROUP });
        } catch { /* 继续 */ }
      }
      setItems((prev) => prev.map((i) => (i.group === g ? { ...i, group: DEFAULT_GROUP } : i)));
    } catch {
      message.error(t("common.error"));
    }
  };

  const loadWatchlist = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<WatchlistItem[]>("list_watchlist");
      if (Array.isArray(list)) {
        // 从 notes JSON 恢复 group 信息
        const parsed = list.map((w: WatchlistItem & { group?: string }) => {
          let group = DEFAULT_GROUP;
          try {
            if (w.notes) {
              const n = JSON.parse(w.notes);
              if (n.group) { group = n.group; }
            }
          } catch { /* ignore */ }
          return { ...w, group };
        });
        setItems(parsed);
      }
    } catch { /* 后端未运行 */ }
    setLoading(false);
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<WatchlistItem[]>("list_watchlist")
      .then((list) => {
        if (cancelled) { return; }
        if (Array.isArray(list)) {
          const parsed = list.map((w: WatchlistItem & { group?: string }) => {
            let group = DEFAULT_GROUP;
            try {
              if (w.notes) {
                const n = JSON.parse(w.notes);
                if (n.group) { group = n.group; }
              }
            } catch { /* ignore */ }
            return { ...w, group };
          });
          setItems(parsed);
        }
      })
      .catch(() => {/* 后端未运行 */})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [watchlistVersion]);

  // 当前分组下的自选股
  const groupItems = useMemo(
    () => items.filter((i) => i.group === activeGroup),
    [items, activeGroup],
  );

  // 实时行情刷新
  const refreshQuotes = useCallback(async () => {
    if (groupItems.length === 0) { return; }
    setRefreshing(true);
    const snap: Record<string, QuoteSnapshot> = {};
    for (const item of groupItems) {
      try {
        const q = await invoke("get_stock_quote", { stockCode: item.stockCode }) as Record<string, unknown>;
        snap[item.stockCode] = {
          price: (q?.price as number) ?? 0,
          changePct: (q?.changePct as number) ?? 0,
          pe: q?.pe as number | undefined,
          timestamp: (q?.timestamp ?? "") as string,
        };
      } catch { /* skip */ }
    }
    setQuotes(snap);
    setRefreshing(false);
  }, [groupItems]);

  // P1-2: 用 RealTimeQuoteWatcher 替代 15s setInterval 轮询。
  // - groupItems 变化时调用 watch_stock_quotes 命令加入后端监控
  // - 监听 stock-quote-update 事件实时更新 quotes 状态（延迟 2s）
  // - 组件卸载时调用 unwatch_stock_quotes 释放后端资源
  useEffect(() => {
    if (groupItems.length === 0) {
      setQuotes({});
      return;
    }

    // 1) 首次拉取一次行情作为初始快照
    refreshQuotes();

    // 2) 加入后端监控（Active 优先级 2s 轮询，replace=true 替换旧列表）
    const codes = groupItems.map((i) => i.stockCode);
    invoke("watch_stock_quotes", {
      stockCodes: codes,
      priority: "active",
      replace: true,
    }).catch((e) => console.warn("[WatchlistPanel] watch_stock_quotes 失败:", e));

    // 3) 监听 stock-quote-update 事件，增量更新 quotes 状态
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<{
          stockCode: string;
          current: { price: number; changePct: number; pe?: number; timestamp: string };
          changePct: number;
          trigger: string;
        }>("stock-quote-update", (event) => {
          const { stockCode, current } = event.payload;
          setQuotes((prev) => ({
            ...prev,
            [stockCode]: {
              price: current.price ?? 0,
              changePct: current.changePct ?? 0,
              pe: current.pe,
              timestamp: current.timestamp ?? "",
            },
          }));
        });
        if (cancelled) {
          unlisten?.();
          unlisten = null;
        }
      } catch (e) {
        console.error("[WatchlistPanel] listen stock-quote-update 失败:", e);
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
      // 组件卸载 / groupItems 变化时，从后端监控列表移除
      invoke("unwatch_stock_quotes", { stockCodes: codes }).catch((e) =>
        console.warn("[WatchlistPanel] unwatch_stock_quotes 失败:", e)
      );
    };
  }, [groupItems, refreshQuotes]);

  // 排序
  const sorted = useMemo(() => {
    return [...groupItems].sort((a, b) => {
      const qa = quotes[a.stockCode];
      const qb = quotes[b.stockCode];
      switch (sortKey) {
        case "change":
          return (qb?.changePct ?? -999) - (qa?.changePct ?? -999);
        case "code":
          return a.stockCode.localeCompare(b.stockCode);
        case "name":
          return a.stockName.localeCompare(b.stockName);
        default:
          return 0;
      }
    });
  }, [groupItems, quotes, sortKey]);

  const addCurrent = async () => {
    if (!stockCode || !stockName) { return; }
    try {
      const notes = JSON.stringify({ group: activeGroup !== DEFAULT_GROUP ? activeGroup : DEFAULT_GROUP });
      await invoke("add_to_watchlist", { stockCode, stockName, notes });
      loadWatchlist();
    } catch { /* 静默 */ }
  };

  const remove = async (id: string) => {
    try {
      await invoke("remove_from_watchlist", { id });
      loadWatchlist();
    } catch { /* 静默 */ }
  };

  const moveToGroup = async (item: WatchlistItem, targetGroup: string) => {
    try {
      await invoke("watchlist_update_group", { id: item.id, groupName: targetGroup });
      setItems((prev) => prev.map((i) => (i.id === item.id ? { ...i, group: targetGroup } : i)));
    } catch { /* 静默 */ }
  };

  const handleClick = async (code: string) => {
    navigate(`/stock-analysis?code=${encodeURIComponent(code)}`);
    await getStockQuote(code);
    await getStockKline(code, "daily", 120);
    startAnalysis(code);
  };

  const analyzeAll = async () => {
    for (const item of sorted) {
      try {
        await getStockQuote(item.stockCode);
        await getStockKline(item.stockCode, "daily", 120);
        startAnalysis(item.stockCode);
      } catch { /* 继续 */ }
    }
    message.info(t("stockAnalysis.watchlist.analysisStarted", { count: sorted.length }));
  };

  const addGroup = async () => {
    const name = newGroup.trim();
    if (!name || groups.includes(name)) { return; }
    const updated = [...groups, name];
    try {
      await invoke("watchlist_save_groups", { groups: updated });
      groupsRef.current = updated;
      setGroupsVersion((v) => v + 1);
      setNewGroup("");
      setActiveGroup(name);
    } catch { /* 静默 */ }
  };

  if (loading) {
    return (
      <Card
        size="small"
        title={`⭐ ${t("stockAnalysis.watchlist._default")}`}
        styles={{ body: { padding: "8px" } }}
      >
        <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
      </Card>
    );
  }

  return (
    <Card
      size="small"
      title={`⭐ ${t("stockAnalysis.watchlist._default")}`}
      styles={{ body: { padding: "8px" } }}
    >
      {/* 分组 Tab */}
      <div className="flex items-center gap-1 mb-2 flex-wrap">
        <Tag
          color={activeGroup === DEFAULT_GROUP ? "blue" : "default"}
          className="cursor-pointer m-0 text-xs"
          onClick={() => setActiveGroup(DEFAULT_GROUP)}
        >
          {t("stockAnalysis.watchlist.all", { count: items.length })}
        </Tag>
        {groups.map((g) => (
          editingGroup === g
            ? (
              <Input
                key={g}
                size="small"
                style={{ width: 100 }}
                defaultValue={g}
                autoFocus
                onPressEnter={async (e) => {
                  const newName = (e.target as HTMLInputElement).value.trim();
                  if (newName && newName !== g) {
                    const updated = groupsRef.current.map((x) => (x === g ? newName : x));
                    try {
                      await invoke("watchlist_save_groups", { groups: updated });
                      groupsRef.current = updated;
                      setGroupsVersion((v) => v + 1);
                      // 同时更新该分组下自选股的 group 名
                      for (const item of items.filter((i) => i.group === g)) {
                        try {
                          await invoke("watchlist_update_group", { id: item.id, groupName: newName });
                        } catch { /* continue */ }
                      }
                      setItems((prev) => prev.map((i) => (i.group === g ? { ...i, group: newName } : i)));
                    } catch { /* 静默 */ }
                  }
                  setEditingGroup(null);
                }}
                onBlur={() => setEditingGroup(null)}
              />
            )
            : (
              <Tag
                key={g}
                color={activeGroup === g ? "blue" : "default"}
                className="cursor-pointer m-0 text-xs"
                closable={false}
                onClose={() => removeGroupAndReassign(g)}
                onClick={() => setActiveGroup(g)}
                onDoubleClick={() => setEditingGroup(g)}
              >
                {g} ({items.filter((i) => i.group === g).length})
              </Tag>
            )
        ))}
        <Input
          size="small"
          placeholder={t("stockAnalysis.watchlist.newGroup")}
          style={{ width: 80 }}
          value={newGroup}
          onChange={(e) => setNewGroup(e.target.value)}
          onPressEnter={addGroup}
          onBlur={addGroup}
        />
      </div>

      {/* 工具栏 */}
      <div className="flex items-center justify-between mb-1">
        <Space size={4}>
          <Button size="small" icon={<PlusOutlined />} disabled={!stockCode} onClick={() => addCurrent()}>
            {t("stockAnalysis.addToWatchlist")}
          </Button>
          <Button size="small" loading={refreshing} onClick={refreshQuotes}>
            {t("stockAnalysis.watchlist.refreshQuotes")}
          </Button>
        </Space>
        <Space size={4}>
          <Select
            size="small"
            style={{ width: 90 }}
            value={sortKey}
            onChange={(v) => setSortKey(v)}
            options={[
              { value: "change", label: t("stockAnalysis.watchlist.sortByChange") },
              { value: "code", label: t("stockAnalysis.watchlist.sortByCode") },
              { value: "name", label: t("stockAnalysis.watchlist.sortByName") },
            ]}
          />
          {sorted.length > 1 && (
            <Button size="small" type="primary" onClick={analyzeAll}>
              {t("stockAnalysis.watchlist.analyzeAll")}
            </Button>
          )}
        </Space>
      </div>

      {sorted.length === 0
        ? <Empty description={t("stockAnalysis.watchlistEmpty")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : (
          <List
            size="small"
            dataSource={sorted}
            renderItem={(item) => {
              const q = quotes[item.stockCode];
              const up = (q?.changePct ?? 0) >= 0;
              const changeStr = q ? `${up ? "+" : ""}${q.changePct.toFixed(2)}%` : "—";
              return (
                <List.Item
                  style={{ cursor: "pointer", padding: "4px 8px" }}
                  onClick={() => handleClick(item.stockCode)}
                  actions={[
                    activeGroup !== DEFAULT_GROUP && (
                      <Dropdown
                        key="move"
                        menu={{
                          items: [
                            { key: DEFAULT_GROUP, label: t("stockAnalysis.watchlist.moveToAll") },
                            ...groups.filter((g) => g !== activeGroup).map((g) => ({ key: g, label: g })),
                          ],
                          onClick: ({ key }) => moveToGroup(item, key),
                        }}
                      >
                        <Button size="small" type="text" className="text-xs">
                          {t("stockAnalysis.watchlist.move")}
                        </Button>
                      </Dropdown>
                    ),
                    <Popconfirm
                      key="del"
                      title={t("common.confirm")}
                      onConfirm={() => remove(item.id)}
                    >
                      <Button size="small" type="text" danger icon={<DeleteOutlined />} />
                    </Popconfirm>,
                  ].filter(Boolean)}
                >
                  <div className="flex items-center gap-2 w-full">
                    <Tag className="m-0 text-xs shrink-0">{item.stockCode}</Tag>
                    <span className="text-xs flex-1 truncate">{item.stockName}</span>
                    {q && (
                      <Space size={4} className="shrink-0">
                        <span className="text-xs font-mono">{q.price.toFixed(2)}</span>
                        <span className={`text-xs ${up ? "text-red-500" : "text-green-500"}`}>
                          {changeStr}
                        </span>
                        {q.pe && <span className="text-xs text-gray-400">PE {q.pe.toFixed(1)}</span>}
                      </Space>
                    )}
                  </div>
                </List.Item>
              );
            }}
          />
        )}
    </Card>
  );
}
