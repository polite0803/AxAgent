---
name: industry-chain-analysis
description: 产业链传导分析 — 5 大预定义产业链的关键节点与传导路径
version: 1.0.0
category: invest
domain: invest
platforms: []
tags:
  - 产业链
  - 传导分析
  - 跨市场
requires_toolsets: []
fallback_for_toolsets: []
---

# 产业链传导分析 SKILL

## 5 大预定义产业链

### 1. AI 算力链（ai_compute）

**节点拓扑**：

```
GPU/AI 加速芯片 → 光模块 → IDC/数据中心 → 液冷 → 电力设备 → AI 应用
```

**关键节点**：

- `gpu`：GPU/AI 加速芯片（A 股：688256 寒武纪；美股：NVDA / AMD）
- `optical_module`：光模块（A 股：300308 中际旭创 / 002281 光迅科技 / 300502 新易盛）
- `idc`：IDC/数据中心（A 股：300383 光环新网 / 600819 耀皮玻璃）
- `liquid_cooling`：液冷（A 股：002335 科华数据 / 300444 司南导航）
- `power`：电力设备（A 股：600406 国电南瑞）
- `ai_application`：AI 应用（A 股：002415 海康威视 / 688111 金山办公）

**传导逻辑**：

- 正向：英伟达 CapEx 上修 → 利好国产 GPU 替代 + 光模块需求
- 负向：美对华 AI 芯片出口管制 → 短期利空英伟达，长期利好国产替代

### 2. 半导体链（semiconductor）

**节点拓扑**：

```
设备 → 材料 → 晶圆代工 → 封测 → EDA/IP
```

**关键节点**：

- `equipment`：半导体设备（A 股：002371 北方华创 / 688012 中微公司）
- `materials`：半导体材料（A 股：688396 华峰测控 / 300655 硅宝科技）
- `foundry`：晶圆代工（A 股：688981 中芯国际；美股：TSM）
- `osat`：封测（A 股：002156 通富微电 / 600584 长电科技）
- `eda_ip`：EDA/IP（A 股：688521 芯原股份）

### 3. 光模块链（optical_module）

**节点拓扑**：

```
硅光 → 光芯片 → CPO → 连接器
```

### 4. 新能源车链（nev）

**节点拓扑**：

```
锂矿 → 正极 → 电池 → 电机 → 整车 → 充电桩
```

**关键节点**：

- `lithium_mining`：锂矿（A 股：002460 赣锋锂业 / 002466 天齐锂业）
- `cathode`：正极（A 股：300073 当升科技）
- `battery`：电池（A 股：300750 宁德时代）
- `motor`：电机（A 股：002196 方正电机）
- `vehicle`：整车（A 股：002594 比亚迪；美股：NIO / XPEV / LI）
- `charging_pile`：充电桩（A 股：300648 星云股份）

### 5. 消费电子链（consumer_electronics）

**节点拓扑**：

```
面板 → 声学 → 光学 → 连接器 → 组装
```

## 传导算法

**BFS 传导**：

- 起始节点 `strength = 1.0`，`lag = 0`
- 每经过一条边：`strength *= edge.strength`，`lag += edge.lag_days`
- 当 `strength < 0.1` 时停止该分支
- 已访问节点不重复访问（防环）

**方向判定**：

- `positive`：利好（订单增加 / 价格上涨 / 政策扶持）
- `negative`：利空（订单减少 / 价格下跌 / 政策限制）
- `neutral`：中性（无明确方向）

## 工具调用建议

- `map_news_to_cross_market_stocks(news_text)`：新闻正文 → 命中产业链 + 激活节点
- `get_industry_chain_propagation(chain_id, start_node_id, direction)`：完整传导路径

## 使用流程

```
1. 输入新闻文本（如"英伟达 CapEx 上修"）
2. 调用 map_news_to_cross_market_stocks → 命中 ai_compute 链
3. 对每条命中链调用 get_industry_chain_propagation
4. LLM 综合判断：方向 / 强度 / 持续性 / 受影响标的
5. 输出：A 股 / 美股 / 港股三市场标的清单
```

## 风险提示

- 产业链传导存在滞后（lag_days），短期不一定立即反映
- 跨市场传导受汇率 / 政策 / 流动性影响
- 同一事件对不同节点影响方向可能不同（如限制出口利空代工但利好国产替代）
