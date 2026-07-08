import { invoke } from "@/lib/invoke";
import type { SimRunRequest, SimRunResult } from "@/types/market-sim";
import { Button, Card, Col, Descriptions, Divider, Form, InputNumber, Row, Space, Spin, Statistic, Tag } from "antd";
import { useRef, useState } from "react";

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
        title="⚡ 模拟配置"
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
          <Form.Item label="股票代码" name="stockCode" rules={[{ required: true }]}>
            <InputNumber style={{ width: 110 }} />
          </Form.Item>
          <Form.Item label="参考价(分)" name="referencePrice" rules={[{ required: true }]}>
            <InputNumber style={{ width: 120 }} min={1} />
          </Form.Item>
          <Form.Item label="模拟时长(ms)" name="maxSimTimeMs" rules={[{ required: true }]}>
            <InputNumber style={{ width: 120 }} min={1} max={1000} />
          </Form.Item>
          <Divider style={{ margin: "8px 0" }} />
          <Form.Item label="做市商" name="marketMakers">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label="动量" name="momentumAgents">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label="价值" name="valueAgents">
            <InputNumber style={{ width: 80 }} min={0} max={5} />
          </Form.Item>
          <Form.Item label="噪声" name="noiseAgents">
            <InputNumber style={{ width: 80 }} min={0} max={10} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" onClick={handleRun} loading={loading}>
              {loading ? "模拟中..." : "运行模拟"}
            </Button>
          </Form.Item>
        </Form>
      </Card>

      {/* 结果区 */}
      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-8">
            <Space direction="vertical" align="center">
              <Spin size="large" />
              <span className="text-secondary text-sm">DES 仿真运行中 ... 5 Agents 博弈中</span>
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
                  title="处理事件"
                  value={result.totalEvents}
                  suffix="条"
                  valueStyle={{ fontSize: 22 }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title="成交笔数"
                  value={result.stats.totalTrades}
                  suffix="笔"
                  valueStyle={{ fontSize: 22 }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title="墙壁时间"
                  value={result.wallClockMs}
                  suffix="ms"
                  valueStyle={{ fontSize: 22 }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title="最终中间价"
                  value={result.finalMidPrice ?? "—"}
                  suffix={result.finalMidPrice ? "分" : ""}
                  valueStyle={{ fontSize: 22 }}
                />
              </Card>
            </Col>
          </Row>

          {/* 详细统计 */}
          <Card
            size="small"
            title={
              <span>
                📊 模拟详情 ·{" "}
                <Tag color="blue" style={{ marginRight: 0 }}>
                  {result.stockCode}
                </Tag>
              </span>
            }
          >
            <Descriptions column={3} size="small" bordered>
              <Descriptions.Item label="模拟时间(虚拟)">
                {(result.simTimeNs / 1_000_000).toFixed(2)} ms
              </Descriptions.Item>
              <Descriptions.Item label="Agent 数量">{result.agentCount}</Descriptions.Item>
              <Descriptions.Item label="参考价">{result.referencePrice} 分</Descriptions.Item>
              <Descriptions.Item label="队列深度峰值">{result.stats.maxQueueDepth}</Descriptions.Item>
              <Descriptions.Item label="总订单数">{result.stats.totalOrders}</Descriptions.Item>
              <Descriptions.Item label="合计成交">
                {result.stats.totalTrades > 0 ? `${result.stats.totalTrades} 笔` : "0"}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {/* 首次进入提示 */}
      {!result && !loading && !error && (
        <Card size="small">
          <div className="py-8 text-center text-secondary">
            <p className="mb-2 text-base">配置上方参数，点击"运行模拟"启动多 Agent 市场仿真</p>
            <p className="text-sm">
              模拟内核包含：交易所(Exchange) + 做市商(MarketMaker) + 动量(Momentum) + 价值(Value) + 噪声(Noise) Agents
            </p>
          </div>
        </Card>
      )}
    </div>
  );
}
