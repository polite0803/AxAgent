import { invoke } from "@/lib/invoke";
import { PlusOutlined, ReloadOutlined } from "@ant-design/icons";
import { App, Button, Card, DatePicker, Input, InputNumber, Modal, Select, Space, Table } from "antd";
import dayjs from "dayjs";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface FundTransfer {
  id: string;
  transferType: string;
  amount: number;
  transferDate: string;
  fee: number | null;
  notes: string | null;
  createdAt: number;
}

interface FundSummary {
  totalDeposits: number;
  totalWithdrawals: number;
  netInflow: number;
  transferCount: number;
}

export function FundPanel() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [loading, setLoading] = useState(false);
  const [transfers, setTransfers] = useState<FundTransfer[]>([]);
  const [summary, setSummary] = useState<FundSummary | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const mountedRef = useRef(true);
  const [form, setForm] = useState({
    transferType: "deposit",
    amount: 0,
    transferDate: dayjs().format("YYYY-MM-DD"),
    fee: 0,
    notes: "",
  });

  const loadData = useCallback(async () => {
    if (!mountedRef.current) { return; }
    setLoading(true);
    try {
      const [t, s] = await Promise.all([
        invoke<FundTransfer[]>("list_fund_transfers", { limit: 100 }),
        invoke<FundSummary>("get_fund_summary"),
      ]);
      if (!mountedRef.current) { return; }
      if (Array.isArray(t)) { setTransfers(t); }
      if (s) { setSummary(s); }
    } catch {
      // silent
    } finally {
      if (mountedRef.current) { setLoading(false); }
    }
  }, []);

  // 首次挂载加载（使用 loadData，避免与 useEffect 重复调用）
  useEffect(() => {
    mountedRef.current = true;
    const id = setTimeout(() => loadData(), 0);
    return () => {
      clearTimeout(id);
      mountedRef.current = false;
    };
  }, [loadData]);

  const handleRecord = async () => {
    if (form.amount <= 0) { return message.warning(t("stockAnalysis.fundPanel.amountMustBeGreaterThanZero")); }
    try {
      await invoke("record_fund_transfer", {
        transferType: form.transferType,
        amount: form.amount,
        transferDate: form.transferDate,
        fee: form.fee > 0 ? form.fee : null,
        notes: form.notes || null,
      });
      message.success(t("stockAnalysis.fundPanel.recorded"));
      setModalOpen(false);
      setForm({ transferType: "deposit", amount: 0, transferDate: dayjs().format("YYYY-MM-DD"), fee: 0, notes: "" });
      loadData();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns = [
    {
      title: t("stockAnalysis.fundPanel.type"),
      dataIndex: "transferType",
      width: 56,
      render: (v: string) =>
        v === "deposit" ? t("stockAnalysis.fundPanel.deposit") : t("stockAnalysis.fundPanel.withdrawal"),
    },
    { title: t("stockAnalysis.fundPanel.amount"), dataIndex: "amount", width: 72, render: (v: number) => v.toFixed(0) },
    { title: t("stockAnalysis.fundPanel.date"), dataIndex: "transferDate", width: 80 },
    {
      title: t("stockAnalysis.fundPanel.fee"),
      dataIndex: "fee",
      width: 56,
      render: (v: number | null) => v ? v.toFixed(0) : "-",
    },
    { title: t("stockAnalysis.fundPanel.notes"), dataIndex: "notes", render: (v: string | null) => v || "-" },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex justify-between items-center">
          <span>{t("stockAnalysis.fundPanel.fundFlow")}</span>
          <Space size={4}>
            <Button size="small" icon={<PlusOutlined />} onClick={() => setModalOpen(true)} />
            <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadData} />
          </Space>
        </div>
      }
      styles={{ body: { padding: "8px 10px", maxHeight: 280, overflowY: "auto" } }}
    >
      {/* 汇总 */}
      {summary && (
        <div className="grid grid-cols-3 gap-1 mb-2 text-xs">
          <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
            {t("stockAnalysis.fundPanel.totalDeposits")}
            <div style={{ fontWeight: "bold" }}>{summary.totalDeposits.toFixed(0)}</div>
          </div>
          <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
            {t("stockAnalysis.fundPanel.totalWithdrawals")}
            <div style={{ fontWeight: "bold" }}>{summary.totalWithdrawals.toFixed(0)}</div>
          </div>
          <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
            {t("stockAnalysis.fundPanel.netInflow")}
            <div
              style={{ color: summary.netInflow >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontWeight: "bold" }}
            >
              {summary.netInflow >= 0 ? "+" : ""}
              {summary.netInflow.toFixed(0)}
            </div>
          </div>
        </div>
      )}

      <Table size="small" dataSource={transfers.slice(0, 10)} rowKey="id" pagination={false} columns={columns} />

      <Modal
        title={t("stockAnalysis.fundPanel.recordFundFlow")}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleRecord}
        width={360}
      >
        <div className="flex flex-col gap-2">
          <Select
            value={form.transferType}
            onChange={(v) => setForm({ ...form, transferType: v })}
            options={[{ value: "deposit", label: t("stockAnalysis.fundPanel.depositTransferIn") }, {
              value: "withdrawal",
              label: t("stockAnalysis.fundPanel.withdrawalTransferOut"),
            }]}
          />
          <InputNumber
            placeholder={t("stockAnalysis.fundPanel.amount")}
            value={form.amount}
            onChange={(v) => setForm({ ...form, amount: v || 0 })}
            min={0}
            style={{ width: "100%" }}
            prefix="¥"
          />
          <DatePicker
            value={dayjs(form.transferDate)}
            onChange={(d) => setForm({ ...form, transferDate: d?.format("YYYY-MM-DD") || form.transferDate })}
            style={{ width: "100%" }}
          />
          <InputNumber
            placeholder={t("stockAnalysis.fundPanel.feeOptional")}
            value={form.fee}
            onChange={(v) => setForm({ ...form, fee: v || 0 })}
            min={0}
            style={{ width: "100%" }}
          />
          <Input
            placeholder={t("stockAnalysis.fundPanel.notesOptional")}
            value={form.notes}
            onChange={(e) => setForm({ ...form, notes: e.target.value })}
          />
        </div>
      </Modal>
    </Card>
  );
}
