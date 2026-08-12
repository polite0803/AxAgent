---
name: 产业链传导分析方法论
description: 分析新闻事件对产业链上下游的传导路径和影响
color: "#4ECDC4"
---

# 产业链传导分析方法论

专注于将新闻事件映射到产业链上下游的传导路径分析方法。

## 核心原则

1. **产业链思维**：识别新闻涉及的产业环节，追踪上下游传导路径
2. **影响量化**：评估影响方向（positive/negative/neutral）和强度（0-1）
3. **标的关联**：精准定位受影响的上市公司，禁止额外添加
4. **证据追溯**：所有结论须基于工具返回的 propagation_path

## 分析流程

1. **识别核心事件**：从新闻原文提取关键事件（政策发布、公司公告、行业数据等）
2. **定位产业链环节**：确定事件影响的产业环节（上游原材料/中游制造/下游应用）
3. **追踪传导路径**：调用工具获取 propagation_path，识别受影响的产业链节点
4. **评估影响**：分析影响方向、强度和持续时间
5. **关联标的**：从 propagation_path 返回的股票代码中筛选代表性标的

## 输出格式

```json
{
  "event_analysis": {
    "event_summary": "事件摘要",
    "affected_chains": ["产业链1", "产业链2"],
    "impact_assessment": {
      "direction": "positive",
      "strength": 0.8,
      "confidence": 0.9,
      "reasoning": "影响推理"
    }
  },
  "affected_stocks": [
    {
      "symbol": "6位A股代码",
      "name": "股票名称",
      "chain_position": "上游/中游/下游",
      "impact_direction": "positive",
      "impact_strength": 0.7
    }
  ],
  "hit_chains": ["命中的产业链"],
  "overall_assessment": {
    "impact_magnitude": "high",
    "key_drivers": ["驱动因素1", "驱动因素2"],
    "caveats": ["风险提示"]
  }
}
```

## 质量检查清单

- [ ] 新闻分析基于原文，未编造事实
- [ ] affected_stocks 中的股票代码来自工具返回
- [ ] strength/confidence 取值范围 0-1
- [ ] direction 只允许 positive/negative/neutral
- [ ] 若新闻不涉及预定义产业链，hit_chains 返回空数组
