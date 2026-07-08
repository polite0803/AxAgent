import { invoke } from "@/lib/invoke";
import type { QuantSimResult } from "@/types/market-sim";
import { Button, Card, Descriptions, InputNumber, Select, Spin, Statistic, Tag } from "antd";
import { useRef, useState } from "react";

const STRATEGIES = [
  { value: "ma_cross", label: "双均线交叉 (MA 5/20)" },
  { value: "macd", label: "MACD 金叉/死叉" },
  { value: "rsi", label: "RSI 超买超卖 (14/70/30)" },
  { value: "boll", label: "布林带上下轨" },
  { value: "turtle", label: "海龟通道突破" },
];

export function QuantSimPanel() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<QuantSimResult | null>(null);
  const [stockCode, setStockCode] = useState("000001");
  const [refPrice, setRefPrice] = useState(1000);
  const [simMs, setSimMs] = useState(500);
  const [strategy, setStrategy] = useState("ma_cross");
  const tokenRef = useRef(0);

  const handleRun = async () => {
    const myToken = ++tokenRef.current;
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await invoke<QuantSimResult>("market_sim_run_strategy", {
        request: {
          stockCode,
          referencePrice: refPrice,
          strategyName: strategy,
          maxSimTimeMs: simMs,
        },
      });
      if (myToken !== tokenRef.current) {
        return;
      }
      setResult(res);
    } catch (e: unknown) {
      if (myToken !== tokenRef.current) {
        return;
      }
      setError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
    } finally {
      if (myToken === tokenRef.current) {
        setLoading(false);
      }
    }
  };

  return (
    <div className="space-y-4">
      <Card size="small" title="⚡ 量化策略 DES 模拟">
        <div className="mb-3 flex flex-wrap items-center gap-4">
          <label className="text-sm font-medium">
            股票代码
            <InputNumber
              className="ml-2"
              style={{ width: 110 }}
              value={stockCode}
              onChange={(v) => setStockCode(v ?? "000001")}
            />
          </label>
          <label className="text-sm font-medium">
            参考价(分)
            <InputNumber
              className="ml-2"
              style={{ width: 120 }}
              min={1}
              value={refPrice}
              onChange={(v) => setRefPrice(v ?? 1000)}
            />
          </label>
          <label className="text-sm font-medium">
            时长(ms)
            <InputNumber
              className="ml-2"
              style={{ width: 100 }}
              min={1}
              max={5000}
              value={simMs}
              onChange={(v) => setSimMs(v ?? 500)}
            />
          </label>
        </div>

        <div className="mb-3 flex items-center gap-4">
          <label className="text-sm font-medium">策略</label>
          <Select
            style={{ width: 240 }}
            value={strategy}
            onChange={setStrategy}
            options={STRATEGIES}
          />
          <Button type="primary" onClick={handleRun} loading={loading}>
            {loading ? "模拟中..." : "运行模拟"}
          </Button>
        </div>

        <div className="text-xs text-secondary">
          将 quant 内置策略作为 Agent 注入 DES，与做市商/噪声同场博弈
        </div>
      </Card>

      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-6">
            <Spin tip="正在运行 DES 模拟..." />
          </div>
        </Card>
      )}

      {error && (
        <Card size="small">
          <div className="py-3 text-center text-red">{error}</div>
        </Card>
      )}

      {result && !loading && (
        <>
          <div className="grid grid-cols-4 gap-3">
            <Card size="small" hoverable>
              <Statistic title="处理事件" value={result.totalEvents} suffix="个" />
            </Card>
            <Card size="small" hoverable>
              <Statistic title="成交笔数" value={result.totalTrades} suffix="笔" />
            </Card>
            <Card size="small" hoverable>
              <Statistic
                title="终止价"
                value={result.finalMidPrice ?? "—"}
                suffix="分"
              />
            </Card>
            <Card size="small" hoverable>
              <Statistic title="墙钟耗时" value={result.wallClockMs} suffix="ms" />
            </Card>
          </div>

          <Card size="small" title="💡 解读">
            <Descriptions column={1} size="small">
              <Descriptions.Item label="策略">
                <Tag color="blue">
                  {STRATEGIES.find((s) => s.value === strategy)?.label ?? strategy}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="市场活动">
                {result.totalEvents > 0
                  ? `在 ${simMs}ms 虚拟时间内产生了 ${result.totalEvents} 个事件、${result.totalTrades} 笔成交。`
                  : "模拟未产生事件，请检查参数。"}
              </Descriptions.Item>
              <Descriptions.Item label="报价">
                {result.finalMidPrice
                  ? `终止中间价 ${result.finalMidPrice} 分 (参考价 ${refPrice} 分)`
                  : "无可用报价"}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {!result && !loading && !error && (
        <Card size="small">
          <div className="py-6 text-center text-secondary">
            <p className="mb-1 text-base">选择一个量化策略，在 DES 合成市场中运行</p>
            <p className="text-sm">
              可用的策略包括双均线交叉、MACD、RSI、布林带、海龟通道
            </p>
          </div>
        </Card>
      )}
    </div>
  );
}
