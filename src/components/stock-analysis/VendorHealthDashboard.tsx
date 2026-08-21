/**
 * 数据源健康仪表盘 — 展示所有 vendor 的实时健康状态
 *
 * 数据来源: get_vendor_health_all 后端命令
 * 覆盖: 在线状态 / 成功率 / 连续失败 / 窗口失败 / 最后错误 / 最后成功时间
 *
 * 2026-08-01 修复：原实现只渲染 health_tracker 已记录的 vendor，未被调用的
 * vendor（ths/baidu_stock/iwencai/neodata/akshare/cninfo/mootdx/sina/xueqiu/
 * guba/browser_eastmoney/international）完全隐藏——用户只见 1-2 个。
 * 改为合并已知 vendor 列表（VENDOR_LABELS 全集），未调用的标"未探测"。
 */

import { invoke } from "@/lib/invoke";
import { Card, Spin, Tag, Tooltip } from "antd";
import { ChevronDown, ChevronUp } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 对应后端 VendorHealth struct */
interface VendorHealthItem {
  name: string;
  consecutiveFailures: number;
  totalSuccesses: number;
  totalFailures: number;
  status: "healthy" | "degraded" | "disabled";
  lastError: string | null;
  lastSuccessAt: number | null;
  lastFailureAt: number | null;
}

const VENDOR_LABEL_KEYS: Record<string, string> = {
  tencent: "stockAnalysis.settings.vendor.labels.tencent",
  eastmoney: "stockAnalysis.settings.vendor.labels.eastmoney",
  sina: "stockAnalysis.settings.vendor.labels.sina",
  ths: "stockAnalysis.settings.vendor.labels.ths",
  cninfo: "stockAnalysis.settings.vendor.labels.cninfo",
  baidu_stock: "stockAnalysis.settings.vendor.labels.baiduStock",
  iwencai: "stockAnalysis.settings.vendor.labels.iwencai",
  akshare: "stockAnalysis.settings.vendor.labels.akshare",
  mootdx: "stockAnalysis.settings.vendor.labels.mootdx",
  browser_eastmoney: "stockAnalysis.settings.vendor.labels.browserEastmoney",
  neodata: "stockAnalysis.settings.vendor.labels.neodata",
  xueqiu: "stockAnalysis.settings.vendor.labels.xueqiu",
  guba: "stockAnalysis.settings.vendor.labels.guba",
  international: "stockAnalysis.settings.vendor.labels.international",
};

const vendorLabelEntries = Object.entries(VENDOR_LABEL_KEYS);

const STATUS_COLORS: Record<string, string> = {
  healthy: "#22c55e",
  degraded: "#eab308",
  disabled: "#ef4444",
  untouched: "#6B7280",
};

const STATUS_LABEL_KEYS: Record<string, string> = {
  healthy: "stockAnalysis.settings.vendor.status.healthy",
  degraded: "stockAnalysis.settings.vendor.status.degraded",
  disabled: "stockAnalysis.settings.vendor.status.disabled",
  untouched: "stockAnalysis.settings.vendor.status.untouched",
};

interface VendorCardProps {
  nameKey: string;
  status: "healthy" | "degraded" | "disabled" | "untouched";
  totalSuccesses?: number;
  totalFailures?: number;
  consecutiveFailures?: number;
  lastError?: string | null;
  lastSuccessAt?: number | null;
  lastFailureAt?: number | null;
  formatTime: (ms: number | null | undefined) => string;
  t: ReturnType<typeof useTranslation>["t"];
}

function VendorCard(props: VendorCardProps) {
  const {
    nameKey,
    status,
    totalSuccesses,
    totalFailures,
    consecutiveFailures,
    lastError,
    lastSuccessAt,
    lastFailureAt,
    formatTime,
    t,
  } = props;
  return (
    <Tooltip
      title={status === "untouched"
        ? (
          <div className="text-xs">
            {t("stockAnalysis.settings.vendorUntouchedHint")}
          </div>
        )
        : (
          <div className="text-xs space-y-1">
            <div>
              {t("stockAnalysis.settings.vendorTotalSuccess")}: {(totalSuccesses ?? 0).toLocaleString()}
            </div>
            <div>
              {t("stockAnalysis.settings.vendorTotalFailures")}: {(totalFailures ?? 0).toLocaleString()}
            </div>
            <div>{t("stockAnalysis.settings.vendorConsecutiveFailures")}: {consecutiveFailures ?? 0}</div>
            {lastError && <div className="text-red-300">{t("stockAnalysis.settings.vendorLastError")}: {lastError}
            </div>}
          </div>
        )}
    >
      <div
        className="flex items-center justify-between p-2 rounded-md text-xs cursor-default"
        style={{
          backgroundColor: status === "healthy"
            ? "rgba(34,197,94,0.08)"
            : status === "degraded"
            ? "rgba(234,179,8,0.08)"
            : status === "disabled"
            ? "rgba(239,68,68,0.08)"
            : "rgba(107,114,128,0.06)",
        }}
      >
        <div className="flex items-center gap-2 min-w-0">
          <span
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: STATUS_COLORS[status] ?? "#6B7280" }}
          />
          <span className="text-gray-200 truncate">{t(nameKey)}</span>
          <Tag
            className="text-[10px] leading-none px-1 py-0"
            color={STATUS_COLORS[status] ?? "default"}
          >
            {t(STATUS_LABEL_KEYS[status] ?? status)}
          </Tag>
        </div>
        <div className="text-gray-500 text-[10px] text-right shrink-0 ml-2">
          {status !== "untouched" && (
            <>
              <div>{t("stockAnalysis.settings.vendorLastSuccess")}: {formatTime(lastSuccessAt ?? null)}</div>
              {lastFailureAt && (
                <div className="text-red-400/60">
                  {t("stockAnalysis.settings.vendorLastFailure")}: {formatTime(lastFailureAt)}
                </div>
              )}
            </>
          )}
          {status === "untouched" && <div className="text-gray-600">-</div>}
        </div>
      </div>
    </Tooltip>
  );
}

export function VendorHealthDashboard() {
  const { t } = useTranslation();
  const [data, setData] = useState<VendorHealthItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<VendorHealthItem[]>("get_vendor_health_all");
      setData(result);
    } catch (e: unknown) {
      setError(
        typeof e === "string"
          ? e
          : e instanceof Error
          ? e.message
          : t("stockAnalysis.settings.vendor.loadFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
    // 每 30s 自动刷新
    const interval = setInterval(load, 30_000);
    return () => clearInterval(interval);
  }, [load]);

  // 统计
  const healthyCount = data.filter((d) => d.status === "healthy").length;
  const degradedCount = data.filter((d) => d.status === "degraded").length;
  const disabledCount = data.filter((d) => d.status === "disabled").length;
  const untouchedCount = Object.keys(VENDOR_LABEL_KEYS).length - data.length;

  const formatTime = (ms: number | null | undefined): string => {
    if (!ms) { return "-"; }
    const d = new Date(ms);
    const now = Date.now();
    const diffMin = Math.floor((now - ms) / 60000);
    if (diffMin < 1) { return t("stockAnalysis.settings.vendor.justNow"); }
    if (diffMin < 60) {
      return t("stockAnalysis.settings.vendor.minutesAgo", { count: diffMin });
    }
    return d.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <Card
      size="small"
      title={
        <button
          type="button"
          onClick={() => setCollapsed(!collapsed)}
          className="flex items-center gap-3 w-full text-left bg-transparent border-none cursor-pointer p-0 hover:opacity-80 transition-opacity"
        >
          <span>{collapsed ? <ChevronDown className="w-4 h-4" /> : <ChevronUp className="w-4 h-4" />}</span>
          <span>{t("stockAnalysis.settings.vendorHealth")}</span>
          <div className="flex gap-2 text-xs">
            <Tag color="green">{healthyCount} {t("stockAnalysis.settings.vendor.status.healthy")}</Tag>
            {degradedCount > 0 && (
              <Tag color="orange">{degradedCount} {t("stockAnalysis.settings.vendor.status.degraded")}</Tag>
            )}
            {disabledCount > 0 && (
              <Tag color="red">{disabledCount} {t("stockAnalysis.settings.vendor.status.disabled")}</Tag>
            )}
            {untouchedCount > 0 && <Tag>{untouchedCount} {t("stockAnalysis.settings.vendor.status.untouched")}</Tag>}
          </div>
        </button>
      }
      extra={
        <button
          onClick={load}
          className="text-xs text-blue-400 hover:text-blue-300 disabled:opacity-50"
          disabled={loading}
        >
          {loading ? t("common.loading") : t("common.refresh")}
        </button>
      }
      className="bg-gray-900/50"
      styles={{ body: collapsed ? { display: "none" } : undefined }}
    >
      {loading && data.length === 0
        ? (
          <div className="flex justify-center py-8">
            <Spin />
          </div>
        )
        : error
        ? <div className="text-red-400 text-center py-4 text-sm">{error}</div>
        : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
            {vendorLabelEntries.map(([vendorName, labelKey]) => {
              const vendor = data.find((d) => d.name === vendorName);
              if (vendor) {
                return (
                  <VendorCard
                    key={vendorName}
                    nameKey={labelKey}
                    status={vendor.status}
                    totalSuccesses={vendor.totalSuccesses}
                    totalFailures={vendor.totalFailures}
                    consecutiveFailures={vendor.consecutiveFailures}
                    lastError={vendor.lastError}
                    lastSuccessAt={vendor.lastSuccessAt}
                    lastFailureAt={vendor.lastFailureAt}
                    formatTime={formatTime}
                    t={t}
                  />
                );
              }
              return (
                <VendorCard
                  key={vendorName}
                  nameKey={labelKey}
                  status="untouched"
                  formatTime={formatTime}
                  t={t}
                />
              );
            })}
          </div>
        )}
    </Card>
  );
}
