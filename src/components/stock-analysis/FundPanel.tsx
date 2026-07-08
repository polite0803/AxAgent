import { invoke } from "@/lib/invoke";
import { PlusOutlined, ReloadOutlined } from "@ant-design/icons";
import { App, Button, Card, DatePicker, Input, InputNumber, Modal, Select, Space, Table } from "antd";
import dayjs from "dayjs";
import { useCallback, useEffect, useRef, useState } from "react";

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
    if (form.amount <= 0) { return message.warning("金额必须 > 0"); }
    try {
      await invoke("record_fund_transfer", {
        transferType: form.transferType,
        amount: form.amount,
        transferDate: form.transferDate,
        fee: form.fee > 0 ? form.fee : null,
        notes: form.notes || null,
      });
      message.success("已记录");
      setModalOpen(false);
      setForm({ transferType: "deposit", amount: 0, transferDate: dayjs().format("YYYY-MM-DD"), fee: 0, notes: "" });
      loadData();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns = [
    { title: "类型", dataIndex: "transferType", width: 56, render: (v: string) => v === "deposit" ? "入金" : "出金" },
    { title: "金额", dataIndex: "amount", width: 72, render: (v: number) => v.toFixed(0) },
    { title: "日期", dataIndex: "transferDate", width: 80 },
    { title: "手续费", dataIndex: "fee", width: 56, render: (v: number | null) => v ? v.toFixed(0) : "-" },
    { title: "备注", dataIndex: "notes", render: (v: string | null) => v || "-" },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex justify-between items-center">
          <span>资金流水</span>
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
            总入金<div style={{ fontWeight: "bold" }}>{summary.totalDeposits.toFixed(0)}</div>
          </div>
          <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
            总出金<div style={{ fontWeight: "bold" }}>{summary.totalWithdrawals.toFixed(0)}</div>
          </div>
          <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
            净入金<div
              style={{ color: summary.netInflow >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontWeight: "bold" }}
            >
              {summary.netInflow >= 0 ? "+" : ""}
              {summary.netInflow.toFixed(0)}
            </div>
          </div>
        </div>
      )}

      <Table size="small" dataSource={transfers.slice(0, 10)} rowKey="id" pagination={false} columns={columns} />

      <Modal title="记录资金流水" open={modalOpen} onCancel={() => setModalOpen(false)} onOk={handleRecord} width={360}>
        <div className="flex flex-col gap-2">
          <Select
            value={form.transferType}
            onChange={(v) => setForm({ ...form, transferType: v })}
            options={[{ value: "deposit", label: "入金（转入）" }, { value: "withdrawal", label: "出金（转出）" }]}
          />
          <InputNumber
            placeholder="金额"
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
            placeholder="手续费（可选）"
            value={form.fee}
            onChange={(v) => setForm({ ...form, fee: v || 0 })}
            min={0}
            style={{ width: "100%" }}
          />
          <Input
            placeholder="备注（可选）"
            value={form.notes}
            onChange={(e) => setForm({ ...form, notes: e.target.value })}
          />
        </div>
      </Modal>
    </Card>
  );
}
