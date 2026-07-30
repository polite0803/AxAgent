// 交易记录批量导入面板 — 从券商/通达信导出的 CSV 文件批量导入成交记录
//
// 流程：选择 CSV 文件 → 后端解析预览 → 确认导入 → 写入 trades 表 + 同步持仓。
// 支持通达信/东方财富/通用 CSV 格式（后端自动识别列名）。

import { invoke } from "@/lib/invoke";
import { FileOutlined } from "@ant-design/icons";
import { Alert, App, Button, Card, Space, Statistic, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useState } from "react";
import { useTranslation } from "react-i18next";

/** 单行解析结果（与后端 ImportRow 对应） */
interface ImportRow {
  row: number;
  stockCode: string;
  stockName: string;
  direction: string;
  price: number;
  quantity: number;
  tradeDate: string;
  tradeTime: string;
  fee: number | null;
  notes: string | null;
  errors: string[];
}

/** 导入摘要（与后端 ImportSummary 对应） */
interface ImportSummary {
  total: number;
  valid: number;
  skipped: number;
  failed: number;
  errors: [number, string][];
  preview: ImportRow[];
}

const { Text } = Typography;

export function TradeImportPanel() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportRow[]>([]);
  const [summary, setSummary] = useState<ImportSummary | null>(null);

  /** 选择 CSV 文件并解析预览 */
  const handleSelectFile = async () => {
    setLoading(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{ name: "CSV", extensions: ["csv", "CSV", "txt"] }],
      });
      if (!selected || typeof selected !== "string") {
        setLoading(false);
        return;
      }
      setFilePath(selected);
      const rows = await invoke<ImportRow[]>("parse_trades_csv", { filePath: selected });
      setPreview(rows);
      setSummary(null);
      if (rows.length === 0) {
        message.warning(t("trade.import.emptyFile"));
      }
    } catch (e) {
      // 浏览器开发模式下 @tauri-apps/plugin-dialog 不可用
      const err = e as Error;
      if (err?.message?.includes("dialog") || err?.message?.includes("Tauri")) {
        message.error(t("trade.import.devModeHint"));
      } else {
        message.error(`${t("trade.import.parseFailed")}: ${err?.message ?? e}`);
      }
    } finally {
      setLoading(false);
    }
  };

  /** 确认导入 */
  const handleImport = async () => {
    if (!filePath) {
      message.warning(t("trade.import.noFile"));
      return;
    }
    setImporting(true);
    try {
      const result = await invoke<ImportSummary>("import_trades", { filePath });
      setSummary(result);
      message.success(
        t("trade.import.imported", {
          valid: result.valid,
          skipped: result.skipped,
          failed: result.failed,
        }),
      );
      // 导入成功后清空预览
      setPreview([]);
      setFilePath(null);
    } catch (e) {
      message.error(`${t("trade.import.importFailed")}: ${(e as Error)?.message ?? e}`);
    } finally {
      setImporting(false);
    }
  };

  const columns: ColumnsType<ImportRow> = [
    {
      title: t("trade.import.columns.row"),
      dataIndex: "row",
      width: 60,
      fixed: "left",
    },
    {
      title: t("trade.import.columns.stockCode"),
      dataIndex: "stockCode",
      width: 100,
      fixed: "left",
    },
    {
      title: t("trade.import.columns.stockName"),
      dataIndex: "stockName",
      width: 120,
    },
    {
      title: t("trade.import.columns.direction"),
      dataIndex: "direction",
      width: 70,
      render: (v: string) => (
        <Tag color={v === "buy" ? "red" : "green"}>
          {v === "buy" ? t("trade.buy") : t("trade.sell")}
        </Tag>
      ),
    },
    {
      title: t("trade.import.columns.price"),
      dataIndex: "price",
      width: 90,
      align: "right",
    },
    {
      title: t("trade.import.columns.quantity"),
      dataIndex: "quantity",
      width: 90,
      align: "right",
    },
    {
      title: t("trade.import.columns.tradeDate"),
      dataIndex: "tradeDate",
      width: 110,
    },
    {
      title: t("trade.import.columns.tradeTime"),
      dataIndex: "tradeTime",
      width: 90,
    },
    {
      title: t("trade.import.columns.fee"),
      dataIndex: "fee",
      width: 80,
      align: "right",
      render: (v: number | null) => v ?? "-",
    },
    {
      title: t("trade.import.columns.errors"),
      dataIndex: "errors",
      render: (errors: string[]) =>
        errors.length > 0
          ? (
            <Space size={4} wrap>
              {errors.map((e, i) => (
                <Tag key={i} color="red">
                  {e}
                </Tag>
              ))}
            </Space>
          )
          : <Tag color="green">OK</Tag>,
    },
  ];

  return (
    <Card
      title={
        <Space>
          <FileOutlined />
          {t("trade.import.title")}
        </Space>
      }
      extra={
        <Space>
          <Button loading={loading} onClick={handleSelectFile}>
            {t("trade.import.selectFile")}
          </Button>
          <Button
            type="primary"
            loading={importing}
            disabled={!filePath || preview.length === 0}
            onClick={handleImport}
          >
            {t("trade.import.confirm")}
          </Button>
        </Space>
      }
    >
      <Alert
        type="info"
        showIcon
        title={t("trade.import.formatHint")}
        style={{ marginBottom: 12 }}
      />

      {filePath && (
        <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
          {t("trade.import.selectedFile")}: {filePath}
        </Text>
      )}

      {summary && (
        <Space size="large" style={{ marginBottom: 12 }}>
          <Statistic title={t("trade.import.summaryTotal")} value={summary.total} />
          <Statistic
            title={t("trade.import.summaryValid")}
            value={summary.valid}
            styles={{ content: { color: "#52c41a" } }}
          />
          <Statistic
            title={t("trade.import.summarySkipped")}
            value={summary.skipped}
            styles={{ content: { color: "#faad14" } }}
          />
          <Statistic
            title={t("trade.import.summaryFailed")}
            value={summary.failed}
            styles={{ content: { color: summary.failed > 0 ? "#ff4d4f" : undefined } }}
          />
        </Space>
      )}

      {preview.length > 0 && (
        <Table<ImportRow>
          columns={columns}
          dataSource={preview}
          rowKey="row"
          size="small"
          scroll={{ x: 900, y: 400 }}
          pagination={{ pageSize: 20, showSizeChanger: false }}
        />
      )}
    </Card>
  );
}
