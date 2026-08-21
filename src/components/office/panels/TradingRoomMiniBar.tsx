// SPDX-License-Identifier: AGPL-3.0-only

/**
 * TradingRoomMiniBar — 交易室实时行情 mini 条。
 *
 * 仅在「投研办公室」场景（investment_office）下显示，置于 Phaser 画布顶部。
 * 从 stockAnalysisStore 订阅当前选中股票的实时行情（quote），以紧凑的单行
 * 像素风条带展示：股票代码 / 名称 / 当前价 / 涨跌幅 / 涨跌额 / 成交量 /
 * 成交额 / PE / PB / 总市值 / 涨停价 / 跌停价 / 时间戳。
 *
 * 交互：
 *   - 手动刷新按钮（按 stockCode 拉取最新行情）
 *   - 自动刷新开关（30s 轮询；沿用 stockAnalysisStore.autoRefresh）
 *   - 切换股票：弹窗内搜索 + 选择，结果写回 stockAnalysisStore
 *
 * 着色规则：A 股惯例 — 红涨绿跌（涨 > 0 红色 / 跌 < 0 绿色 / 平 灰色）。
 */

import { useStockAnalysisStore } from "@/stores";
import type { StockSearchResult } from "@/types";
import { Button, Modal, Spin, Switch, Tag, theme, Tooltip, Typography } from "antd";
import type { TFunction } from "i18next";
import { Activity, AlertTriangle, RefreshCw, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const AUTO_REFRESH_INTERVAL_MS = 30_000;

// ── 内联数字格式化（项目暂无 lib/numberFormat）──

/** 保留 2 位小数的价格（不补千分位，避免破坏像素对齐） */
function formatPrice(n: number): string {
  if (!Number.isFinite(n)) { return "—"; }
  return n.toFixed(2);
}

/**
 * 大数缩写：股 / 元 → 万 / 亿。
 * - shares: volume（股）
 * - currency: amount / mv（元）
 */
function formatBigNumber(n: number, kind: "shares" | "currency", t: TFunction): string {
  if (!Number.isFinite(n)) { return "—"; }
  const abs = Math.abs(n);
  const unit = kind === "currency"
    ? t("office.roomMiniBar.trading.unitYuan")
    : t("office.roomMiniBar.trading.unitShares");
  if (abs >= 1e8) { return `${(n / 1e8).toFixed(2)} ${t("office.roomMiniBar.trading.unitYi")}${unit}`; }
  if (abs >= 1e4) { return `${(n / 1e4).toFixed(2)} ${t("office.roomMiniBar.trading.unitWan")}${unit}`; }
  return `${n.toFixed(0)} ${unit}`;
}

/** 把 ISO 时间戳或 yyyy-MM-dd HH:mm:ss 格式化为可读的「HH:mm:ss」 */
function formatTimestamp(ts: string): string {
  if (!ts) { return "—"; }
  const m = ts.match(/(\d{2}:\d{2}:\d{2})/);
  return m ? m[1] : ts;
}

/** A 股惯例：涨红跌绿平灰 */
function pickChangeColor(
  changePct: number,
  upColor: string,
  downColor: string,
  flatColor: string,
): string {
  if (changePct > 0) { return upColor; }
  if (changePct < 0) { return downColor; }
  return flatColor;
}

/** 单字段格子 */
function Field({
  label,
  value,
  color,
  mono = true,
}: {
  label: string;
  value: string;
  color?: string;
  mono?: boolean;
}) {
  const { token } = theme.useToken();
  return (
    <Tooltip title={label} mouseEnterDelay={0.4}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 1,
          padding: "0 8px",
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          minWidth: 0,
        }}
      >
        <span style={{ fontSize: 9, color: token.colorTextQuaternary, lineHeight: 1 }}>
          {label}
        </span>
        <span
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: color ?? token.colorText,
            fontFamily: mono ? "ui-monospace, 'JetBrains Mono', monospace" : "inherit",
            whiteSpace: "nowrap",
            lineHeight: 1.2,
          }}
        >
          {value}
        </span>
      </div>
    </Tooltip>
  );
}

export interface TradingRoomMiniBarProps {
  /** 当前 fleet 使用的场景模板 slug — 仅 investment_office 时渲染 */
  sceneTemplateSlug?: string;
}

export function TradingRoomMiniBar({ sceneTemplateSlug }: TradingRoomMiniBarProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const quote = useStockAnalysisStore((s) => s.quote);
  const quoteLoading = useStockAnalysisStore((s) => s.quoteLoading);
  const quoteError = useStockAnalysisStore((s) => s.quoteError);
  const autoRefresh = useStockAnalysisStore((s) => s.autoRefresh);
  const setAutoRefresh = useStockAnalysisStore((s) => s.setAutoRefresh);
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const searchStock = useStockAnalysisStore((s) => s.searchStock);
  const searchResults = useStockAnalysisStore((s) => s.searchResults);

  const [pickerOpen, setPickerOpen] = useState(false);
  const [keyword, setKeyword] = useState("");
  const [picking, setPicking] = useState(false);

  // 自动刷新：30s 轮询（仅在开关打开且 stockCode 非空时）
  useEffect(() => {
    if (!autoRefresh || !stockCode) { return; }
    const timer = setInterval(() => {
      void getStockQuote(stockCode);
    }, AUTO_REFRESH_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [autoRefresh, stockCode, getStockQuote]);

  // 首次有 stockCode 但 quote 为空时自动拉一次
  useEffect(() => {
    if (stockCode && !quote && !quoteLoading) {
      void getStockQuote(stockCode);
    }
  }, [stockCode, quote, quoteLoading, getStockQuote]);

  // 切换股票弹窗打开时清空 keyword
  useEffect(() => {
    if (!pickerOpen) {
      setKeyword("");
    }
  }, [pickerOpen]);

  // 仅 investment_office 场景渲染（hooks 已在上，early return 安全）
  if (sceneTemplateSlug !== "investment_office") {
    return null;
  }

  const handleRefresh = () => {
    if (!stockCode) { return; }
    void getStockQuote(stockCode);
  };

  const handleSearch = async (kw: string) => {
    setKeyword(kw);
    await searchStock(kw);
  };

  const handlePick = async (item: StockSearchResult) => {
    setPicking(true);
    try {
      // 切股：直接调 getStockQuote，store 内部会把 stockCode/stockName 一起写入
      await getStockQuote(item.code);
      setPickerOpen(false);
      setKeyword("");
    } finally {
      setPicking(false);
    }
  };

  const changePct = quote?.changePct ?? 0;
  const changeColor = pickChangeColor(
    changePct,
    token.colorErrorText, // 涨红
    token.colorSuccess, // 跌绿（A 股惯例）
    token.colorTextSecondary, // 平灰
  );
  const changeAbs = quote ? quote.price - quote.preClose : 0;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 0,
        padding: "6px 8px",
        background: token.colorBgLayout,
        borderRadius: 6,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 12,
        overflowX: "auto",
      }}
    >
      {/* 左侧：股票标识 + 切换按钮 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "0 8px 0 4px",
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          flexShrink: 0,
        }}
      >
        <Activity size={14} color={token.colorPrimary} />
        <Tooltip title={t("office.roomMiniBar.trading.switchStock")}>
          <Button
            size="small"
            type="text"
            icon={<Search size={12} />}
            onClick={() => setPickerOpen(true)}
            style={{ padding: "0 4px" }}
          />
        </Tooltip>
        <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
          <span style={{ fontSize: 9, color: token.colorTextQuaternary, lineHeight: 1 }}>
            {t("office.roomMiniBar.trading.stock")}
          </span>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <code
              style={{
                fontFamily: "ui-monospace, 'JetBrains Mono', monospace",
                fontSize: 12,
                fontWeight: 700,
                color: token.colorText,
              }}
            >
              {stockCode || "—"}
            </code>
            {stockName && stockName !== stockCode && (
              <span
                style={{
                  fontSize: 11,
                  color: token.colorTextSecondary,
                  maxWidth: 120,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {stockName}
              </span>
            )}
            {quote?.isSt && (
              <Tag
                color="red"
                style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
              >
                ST
              </Tag>
            )}
          </div>
        </div>
      </div>

      {/* 中间：行情字段 */}
      {quote
        ? (
          <>
            <Field
              label={t("office.roomMiniBar.trading.price")}
              value={formatPrice(quote.price)}
              color={changeColor}
            />
            <Field
              label={t("office.roomMiniBar.trading.changePct")}
              value={`${changePct >= 0 ? "+" : ""}${changePct.toFixed(2)}%`}
              color={changeColor}
            />
            <Field
              label={t("office.roomMiniBar.trading.changeAbs")}
              value={`${changeAbs >= 0 ? "+" : ""}${formatPrice(changeAbs)}`}
              color={changeColor}
            />
            <Field
              label={t("office.roomMiniBar.trading.open")}
              value={formatPrice(quote.open)}
            />
            <Field
              label={t("office.roomMiniBar.trading.high")}
              value={formatPrice(quote.high)}
              color={token.colorErrorText}
            />
            <Field
              label={t("office.roomMiniBar.trading.low")}
              value={formatPrice(quote.low)}
              color={token.colorSuccess}
            />
            <Field
              label={t("office.roomMiniBar.trading.preClose")}
              value={formatPrice(quote.preClose)}
            />
            <Field
              label={t("office.roomMiniBar.trading.volume")}
              value={formatBigNumber(quote.volume, "shares", t)}
            />
            <Field
              label={t("office.roomMiniBar.trading.amount")}
              value={formatBigNumber(quote.amount, "currency", t)}
            />
            <Field
              label={t("office.roomMiniBar.trading.pe")}
              value={quote.pe != null ? quote.pe.toFixed(2) : "—"}
            />
            <Field
              label={t("office.roomMiniBar.trading.pb")}
              value={quote.pb != null ? quote.pb.toFixed(2) : "—"}
            />
            <Field
              label={t("office.roomMiniBar.trading.totalMv")}
              value={quote.totalMv != null ? formatBigNumber(quote.totalMv, "currency", t) : "—"}
            />
            <Field
              label={t("office.roomMiniBar.trading.circulatingMv")}
              value={quote.circulatingMv != null
                ? formatBigNumber(quote.circulatingMv, "currency", t)
                : "—"}
            />
            <Field
              label={t("office.roomMiniBar.trading.limitUp")}
              value={quote.limitUp != null ? formatPrice(quote.limitUp) : "—"}
              color={token.colorErrorText}
            />
            <Field
              label={t("office.roomMiniBar.trading.limitDown")}
              value={quote.limitDown != null ? formatPrice(quote.limitDown) : "—"}
              color={token.colorSuccess}
            />
            <Field
              label={t("office.roomMiniBar.trading.turnoverRate")}
              value={`${(quote.turnoverRate * 100).toFixed(2)}%`}
            />
            <Field
              label={t("office.roomMiniBar.trading.timestamp")}
              value={formatTimestamp(quote.timestamp)}
              mono={false}
            />
          </>
        )
        : (
          <div style={{ padding: "0 12px", color: token.colorTextQuaternary, fontSize: 11 }}>
            {quoteLoading
              ? t("office.roomMiniBar.trading.loading")
              : stockCode
              ? (quoteError ?? t("office.roomMiniBar.trading.noQuote"))
              : t("office.roomMiniBar.trading.noStockSelected")}
          </div>
        )}

      {/* 右侧：刷新 + 自动刷新开关 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "0 0 0 8px",
          marginLeft: "auto",
          flexShrink: 0,
        }}
      >
        {quoteError && (
          <Tooltip title={quoteError}>
            <AlertTriangle size={12} color={token.colorError} />
          </Tooltip>
        )}
        <Tooltip title={t("office.roomMiniBar.trading.refresh")}>
          <Button
            size="small"
            type="text"
            icon={<RefreshCw size={12} />}
            loading={quoteLoading}
            onClick={handleRefresh}
            disabled={!stockCode}
            style={{ padding: "0 4px" }}
          />
        </Tooltip>
        <Tooltip title={t("office.roomMiniBar.trading.autoRefreshHint")}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
            <Text type="secondary" style={{ fontSize: 10 }}>
              {t("office.roomMiniBar.trading.autoRefresh")}
            </Text>
            <Switch size="small" checked={autoRefresh} onChange={setAutoRefresh} />
          </span>
        </Tooltip>
      </div>

      {/* 切换股票弹窗 */}
      <Modal
        open={pickerOpen}
        title={t("office.roomMiniBar.trading.pickerTitle")}
        onCancel={() => {
          setPickerOpen(false);
          setKeyword("");
        }}
        footer={null}
        width={420}
        styles={{ body: { paddingTop: 12 } }}
      >
        <StockPicker
          keyword={keyword}
          results={searchResults}
          loading={picking}
          onSearch={handleSearch}
          onPick={handlePick}
        />
      </Modal>
    </div>
  );
}

/** 股票搜索器 */
function StockPicker({
  keyword,
  results,
  loading,
  onSearch,
  onPick,
}: {
  keyword: string;
  results: StockSearchResult[];
  loading: boolean;
  onSearch: (kw: string) => void;
  onPick: (item: StockSearchResult) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const list = useMemo(() => results.slice(0, 12), [results]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <input
        ref={inputRef}
        value={keyword}
        onChange={(e) => onSearch(e.target.value)}
        placeholder={t("office.roomMiniBar.trading.pickerPlaceholder")}
        style={{
          padding: "6px 10px",
          borderRadius: 4,
          border: `1px solid ${token.colorBorder}`,
          fontSize: 12,
          outline: "none",
          width: "100%",
          background: token.colorBgContainer,
          color: token.colorText,
        }}
      />
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 16 }}>
            <Spin size="small" />
          </div>
        )
        : list.length === 0
        ? (
          <div
            style={{
              textAlign: "center",
              color: token.colorTextQuaternary,
              fontSize: 12,
              padding: 12,
            }}
          >
            {keyword.length < 2
              ? t("office.roomMiniBar.trading.pickerHint")
              : t("office.roomMiniBar.trading.pickerEmpty")}
          </div>
        )
        : (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 4,
              maxHeight: 280,
              overflowY: "auto",
            }}
          >
            {list.map((r) => (
              <button
                key={r.code}
                onClick={() => onPick(r)}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  padding: "6px 8px",
                  background: token.colorBgContainer,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  borderRadius: 4,
                  cursor: "pointer",
                  textAlign: "left",
                  fontSize: 12,
                }}
              >
                <span>
                  <code
                    style={{
                      fontFamily: "ui-monospace, monospace",
                      color: token.colorPrimary,
                      fontWeight: 700,
                    }}
                  >
                    {r.code}
                  </code>
                  <span style={{ marginLeft: 8 }}>{r.name}</span>
                </span>
                <Tag style={{ fontSize: 10, margin: 0 }}>{r.market}</Tag>
              </button>
            ))}
          </div>
        )}
    </div>
  );
}
