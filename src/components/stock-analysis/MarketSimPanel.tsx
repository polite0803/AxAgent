import { invoke } from "@/lib/invoke";
import type { SimRunRequest, SimRunResult } from "@/types/market-sim";
import { Button, Card, Col, Descriptions, Divider, Form, InputNumber, Row, Space, Spin, Statistic, Tag } from "antd";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * MarketSimPanel — ABIDES-inspired 多 Agent 市场模拟面板
 *
 * 用户可配置模拟参数，运行多 Agent DES 仿真，查看统计结果。
 * 集成在 /backtest 页面中作为 "市场模拟" 标签页。
 */
export function MarketSimPanel() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<SimRunResult | null>(null);
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const tokenRef = useRef(0);

  const handleRun = async () => {
    const values = await form.validateFields();
    const myToken = ++tokenRef.current;
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const request: SimRunRequest = {
        stockCode: values.stockCode ?? "000001",
        referencePrice: values.referencePrice ?? 1000,
        maxSimTimeNs: (values.maxSimTimeMs ?? 50) * 1_000_000,
        agentConfig: {
          marketMakers: values.marketMakers ?? 1,
          momentumAgents: values.momentumAgents ?? 1,
          valueAgents: values.valueAgents ?? 1,
          noiseAgents: values.noiseAgents ?? 2,
        },
      };

      const res = await invoke<SimRunResult>("market_sim_run", { request });
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
      {/* 配置区 */}
      <Card
        size="small"
        title={t("stockAnalysis.marketSimPanel.simConfig")}
        className="[&_.ant-card-head-title]:flex [&_.ant-card-head-title]:items-center"
      >
        <Form
          form={form}
          layout="inline"
          initialValues={{
            stockCode: "000001",
            referencePrice: 1000,
            maxSimTimeMs: 50,
            marketMakers: 1,
            momentumAgents: 1,
            valueAgents: 1,
            noiseAgents: 2,
          }}
          style={{ flexWrap: "wrap", gap: 12 }}
        >
          <Form.Item label={t("stockAnalysis.marketSimPanel.stockCode")} name="stockCode" rules={[{ required: true }]}>
            <InputNumber style={{ width: 110 }} />
          </Form.Item>
          <Form.Item
            label={t("stockAnalysis.marketSimPanel.referencePrice")}
            name="referencePrice"
            rules={[{ required: true }]}
          >
            <InputNumber style={{ width: 120 }} min={1} />
          </Form.Item>
          <Form.Item
            label={t("stockAnalysis.marketSimPanel.simDuration")}
            name="maxSimTimeMs"
            rules={[{ required: true }]}
          >
            <InputNumber style={{ width: 120 }} min={1} max={1000} />
          </Form.Item>
          <Divider style={{ margin: "8px 0" }} />
          <Form.Item label={t("stockAnalysis.marketSimPanel.marketMaker")} name="marketMakers">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label={t("stockAnalysis.marketSimPanel.momentum")} name="momentumAgents">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label={t("stockAnalysis.marketSimPanel.value")} name="valueAgents">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label={t("stockAnalysis.marketSimPanel.noise")} name="noiseAgents">
            <InputNumber style={{ width: 80 }} min={0} max={10} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" onClick={handleRun} loading={loading}>
              {loading ? t("stockAnalysis.marketSimPanel.simulating") : t("stockAnalysis.marketSimPanel.runSimulation")}
            </Button>
          </Form.Item>
        </Form>
      </Card>

      {/* 结果区 */}
      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-8">
            <Space orientation="vertical" align="center">
              <Spin size="large" />
              <span className="text-secondary text-sm">{t("stockAnalysis.marketSimPanel.desRunning")}</span>
            </Space>
          </div>
        </Card>
      )}

      {error && (
        <Card size="small">
          <div className="py-4 text-center">
            <span className="text-red">{error}</span>
          </div>
        </Card>
      )}

      {result && !loading && (
        <>
          {/* 核心指标 */}
          <Row gutter={[12, 12]}>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.marketSimPanel.totalEvents")}
                  value={result.totalEvents}
                  suffix={t("stockAnalysis.marketSimPanel.eventsSuffix")}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.marketSimPanel.totalTrades")}
                  value={result.stats.totalTrades}
                  suffix={t("stockAnalysis.marketSimPanel.tradesSuffix")}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.marketSimPanel.wallClock")}
                  value={result.wallClockMs}
                  suffix="ms"
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.marketSimPanel.finalMidPrice")}
                  value={result.finalMidPrice ?? "—"}
                  suffix={result.finalMidPrice ? t("stockAnalysis.marketSimPanel.fenSuffix") : ""}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
          </Row>

          {/* 详细统计 */}
          <Card
            size="small"
            title={
              <span>
                {t("stockAnalysis.marketSimPanel.simDetails")}{" "}
                <Tag color="blue" style={{ marginRight: 0 }}>
                  {result.stockCode}
                </Tag>
              </span>
            }
          >
            <Descriptions column={3} size="small" bordered>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.simTimeVirtual")}>
                {(result.simTimeNs / 1_000_000).toFixed(2)} ms
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.agentCount")}>
                {result.agentCount}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.refPrice")}>
                {result.referencePrice} {t("stockAnalysis.marketSimPanel.fenUnit")}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.maxQueueDepth")}>
                {result.stats.maxQueueDepth}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.totalOrders")}>
                {result.stats.totalOrders}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.marketSimPanel.totalTradesLabel")}>
                {result.stats.totalTrades > 0
                  ? `${result.stats.totalTrades} ${t("stockAnalysis.marketSimPanel.tradesUnit")}`
                  : "0"}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {/* 首次进入提示 */}
      {!result && !loading && !error && (
        <Card size="small">
          <div className="py-8 text-center text-secondary">
            <p className="mb-2 text-base">{t("stockAnalysis.marketSimPanel.emptyHint")}</p>
            <p className="text-sm">
              {t("stockAnalysis.marketSimPanel.emptyDesc")}
            </p>
          </div>
        </Card>
      )}
    </div>
  );
}
