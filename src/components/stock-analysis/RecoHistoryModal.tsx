import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { Button, Card, Checkbox, Collapse, Empty, List, message, Modal, Table, Tag, Typography } from "antd";
const { Text } = Typography;
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface RecoHistoryRow {
  generatedAt: string;
  period: string;
  stockCount: number;
  styles: string;
  createdAt: string;
}

interface RecoDetailItem {
  id: string;
  generatedAt: string;
  period: string;
  stockCode: string;
  stockName: string;
  style: string;
  confidence: number;
  synthetic: number;
  seedPoolJson?: string | null;
  pickData?: string | null;
  createdAt: string;
}

/** 风格颜色映射 */
const STYLE_COLOR: Record<string, string> = {
  trend: "red",
  value: "blue",
  capital: "orange",
  reversion: "purple",
  watchlist: "cyan",
  serenity: "green",
};

export function RecoHistoryModal() {
  const { t } = useTranslation();
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);

  // ── 历史列表 ──
  const [open, setOpen] = useState(false);
  const [data, setData] = useState<RecoHistoryRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [deleting, setDeleting] = useState(false);

  // ── 详情视图 ──
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailItems, setDetailItems] = useState<RecoDetailItem[]>([]);
  const [detailRow, setDetailRow] = useState<RecoHistoryRow | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<RecoHistoryRow[]>("list_reco_history", { limit: 50 });
      setData(list ?? []);
    } catch { /* */ }
    setLoading(false);
  }, []);

  const openDetail = useCallback(async (row: RecoHistoryRow) => {
    setDetailRow(row);
    setDetailOpen(true);
    setDetailLoading(true);
    try {
      const items = await invoke<RecoDetailItem[]>("get_reco_detail", {
        generatedAt: row.generatedAt,
      });
      setDetailItems(items ?? []);
    } catch (e) {
      console.error("加载荐股详情失败", e);
      setDetailItems([]);
    }
    setDetailLoading(false);
  }, []);

  /** 按风格分组 */
  const grouped = useMemo(() => {
    const map = new Map<string, RecoDetailItem[]>();
    for (const item of detailItems) {
      const list = map.get(item.style) ?? [];
      list.push(item);
      map.set(item.style, list);
    }
    return map;
  }, [detailItems]);

  const handleAnalyze = useCallback(
    (code: string) => {
      if (code) {
        setDetailOpen(false);
        startAnalysis(code);
      }
    },
    [startAnalysis],
  );

  return (
    <>
      <Button
        size="small"
        onClick={() => {
          setOpen(true);
          loadData();
        }}
      >
        {t("stockAnalysis.recommendation.recoHistory.viewHistory")}
      </Button>

      {/* ── 历史列表 Modal ── */}
      <Modal
        title={t("stockAnalysis.recommendation.recoHistory.title")}
        open={open}
        onCancel={() => {
          setOpen(false);
          setSelected([]);
        }}
        footer={selected.length > 0
          ? (
            <div className="flex items-center gap-2">
              <span className="text-xs text-gray-400">
                {t("stockAnalysis.recommendation.recoHistory.selectedCount", { count: selected.length })}
              </span>
              <Button size="small" onClick={() => setSelected([])}>
                {t("stockAnalysis.recommendation.recoHistory.exitSelect")}
              </Button>
              <Button
                size="small"
                danger
                loading={deleting}
                onClick={async () => {
                  setDeleting(true);
                  try {
                    await invoke("batch_delete_reco_history", { generatedAts: selected });
                    message.success(
                      t("stockAnalysis.recommendation.recoHistory.deleteSuccess", { count: selected.length }),
                    );
                    setData((prev) => prev.filter((r) => !selected.includes(r.generatedAt)));
                    setSelected([]);
                  } catch (e) {
                    message.error(String(e));
                  }
                  setDeleting(false);
                }}
              >
                {t("stockAnalysis.recommendation.recoHistory.batchDelete", { count: selected.length })}
              </Button>
            </div>
          )
          : null}
        width={640}
      >
        <Table
          size="small"
          loading={loading}
          dataSource={data}
          rowKey="generatedAt"
          pagination={false}
          onRow={(record) => ({
            className: "cursor-pointer",
            onClick: () => openDetail(record),
          })}
          columns={[
            {
              title: (
                <Checkbox
                  checked={data.length > 0 && selected.length === data.length}
                  indeterminate={selected.length > 0 && selected.length < data.length}
                  onChange={(e) => {
                    setSelected(e.target.checked ? data.map((r) => r.generatedAt) : []);
                  }}
                />
              ),
              key: "select",
              width: 40,
              render: (_, r) => (
                <Checkbox
                  checked={selected.includes(r.generatedAt)}
                  onClick={(e) => e.stopPropagation()}
                  onChange={(e) => {
                    setSelected(
                      e.target.checked
                        ? [...selected, r.generatedAt]
                        : selected.filter((g) => g !== r.generatedAt),
                    );
                  }}
                />
              ),
            },
            {
              title: t("stockAnalysis.recommendation.recoHistory.generatedAt"),
              dataIndex: "generatedAt",
              key: "generatedAt",
              render: (v: string) => (
                <span className="text-xs font-mono">
                  {new Date(v).toLocaleString()}
                </span>
              ),
            },
            {
              title: t("stockAnalysis.recommendation.recoHistory.period"),
              dataIndex: "period",
              key: "period",
              render: (v: string) => {
                const camel = v.replace(/(_[a-z])/g, (m) => m[1].toUpperCase());
                const periodKey = `stockAnalysis.period${camel.charAt(0).toUpperCase() + camel.slice(1)}`;
                return (
                  <Tag className="text-[10px] m-0">
                    {t(periodKey)}
                  </Tag>
                );
              },
            },
            {
              title: t("stockAnalysis.recommendation.recoHistory.stockCount"),
              dataIndex: "stockCount",
              key: "stockCount",
              render: (v: number) => <span className="text-xs">{v}</span>,
            },
            {
              title: t("stockAnalysis.recommendation.recoHistory.styles"),
              dataIndex: "styles",
              key: "styles",
              render: (v: string) => (
                <div className="flex gap-1 flex-wrap" onClick={(e) => e.stopPropagation()}>
                  {v.split(",").map((s) => (
                    <Tag key={s} color={STYLE_COLOR[s]} className="text-[10px] m-0">
                      {t(`stockAnalysis.recommendation.style${s.charAt(0).toUpperCase() + s.slice(1)}`)}
                    </Tag>
                  ))}
                </div>
              ),
            },
          ]}
        />
      </Modal>

      {/* ── 详情视图 Modal ── */}
      <Modal
        title={detailRow
          ? `${t("stockAnalysis.recommendation.recoHistory.title")} — ${
            new Date(detailRow.generatedAt).toLocaleString()
          }`
          : ""}
        open={detailOpen}
        onCancel={() => {
          setDetailOpen(false);
          setDetailItems([]);
          setDetailRow(null);
        }}
        footer={null}
        width={700}
      >
        {detailLoading
          ? (
            <div className="py-8 text-center text-sm text-gray-400">
              {t("common.loading")}
            </div>
          )
          : detailItems.length === 0
          ? (
            <Empty
              description={t("stockAnalysis.recommendation.recoHistory.detailEmpty")}
            />
          )
          : (
            <Collapse
              ghost
              size="small"
              defaultActiveKey={Array.from(grouped.keys()).slice(0, 3)}
              items={Array.from(grouped.entries()).map(([style, items]) => ({
                key: style,
                label: (
                  <div className="flex items-center gap-2">
                    <Tag color={STYLE_COLOR[style] ?? "default"} className="m-0 text-xs">
                      {t(`stockAnalysis.recommendation.style${style.charAt(0).toUpperCase() + style.slice(1)}`)}
                    </Tag>
                    <span className="text-xs text-gray-500">
                      ({items.length})
                    </span>
                  </div>
                ),
                children: (
                  <List
                    size="small"
                    dataSource={items}
                    renderItem={(p) => (
                      <List.Item className="py-1">
                        <Card
                          size="small"
                          hoverable
                          className="w-full"
                          onClick={() => handleAnalyze(p.stockCode)}
                        >
                          <div className="text-xs w-full flex flex-col gap-0.5">
                            <div className="flex items-center gap-1.5">
                              <Tag className="m-0 text-[10px]">{p.stockCode}</Tag>
                              <span className="font-medium truncate flex-1">{p.stockName}</span>
                              <Tag color="volcano" className="m-0 text-[10px]">BUY</Tag>
                              {p.synthetic === 1 && (
                                <Tag color="orange" className="m-0 text-[10px]">
                                  {t("stockAnalysis.recommendation.tagSynthetic")}
                                </Tag>
                              )}
                              <Tag color="blue" className="m-0 text-[10px]">
                                {t("stockAnalysis.recommendation.row.confidence")} {p.confidence}
                              </Tag>
                            </div>
                            <Text type="secondary" className="text-[10px]">
                              {t("stockAnalysis.recommendation.recoHistory.generatedAt")}:{" "}
                              {new Date(p.generatedAt).toLocaleString()}
                            </Text>
                          </div>
                        </Card>
                      </List.Item>
                    )}
                  />
                ),
              }))}
            />
          )}
      </Modal>
    </>
  );
}
