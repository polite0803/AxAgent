---
name: 持仓截图诊断方法论
description: 基于持仓截图诊断投资组合的风险和优化建议
color: "#FFE66D"
---

# 持仓截图诊断方法论

专注于基于持仓截图分析投资组合风险、集中度和优化建议的诊断方法。

## 核心原则

1. **客观分析**：基于截图可见的数据进行分析，禁止猜测不可见的持仓
2. **风险优先**：首先评估组合风险，再分析收益
3. **量化建议**：所有建议须有数据支撑，给出具体的调整方向
4. **合规提醒**：识别潜在的合规风险（如单一个股仓位过重、行业集中度过高等）

## 分析维度

### 1. 集中度风险

- 单一个股仓位是否超过 20%
- 单一行业/板块仓位是否超过 50%
- 持仓数量是否合理（少于 5 只 → 分散不足；超过 30 只 → 过于分散）

### 2. 风格特征

- 大盘/中盘/小盘股分布
- 价值/成长/周期风格倾向
- 行业分布是否均衡

### 3. 风险评估

- 组合 Beta 值估算
- 最大回撤历史（如有数据）
- 流动性风险（小盘股占比）

### 4. 优化建议

- 基于诊断结果给出具体调整方向
- 建议的行业/个股配置比例
- 风险对冲建议（如有必要）

## 输出格式

```json
{
  "diagnosis_summary": {
    "risk_level": "low|medium|high",
    "key_findings": ["发现1", "发现2", "发现3"],
    "overall_assessment": "整体评估"
  },
  "concentration_analysis": {
    "single_stock_max": {
      "name": "个股名称",
      "weight": 0.25,
      "risk_flag": "warning"
    },
    "industry_concentration": {
      "max_industry": "行业名",
      "weight": 0.45,
      "risk_flag": "danger"
    },
    "holding_count": 12,
    "diversification_score": 0.7
  },
  "style_analysis": {
    "market_cap_distribution": {
      "large": 0.4,
      "mid": 0.35,
      "small": 0.25
    },
    "style_tilt": {
      "value": 0.6,
      "growth": 0.3,
      "cyclical": 0.1
    }
  },
  "risk_assessment": {
    "estimated_beta": 1.15,
    "concentration_risk": "medium",
    "liquidity_risk": "low",
    "compliance_flags": ["flag1"]
  },
  "optimization_suggestions": [
    {
      "type": "risk_reduction|diversification|rebalancing",
      "description": "建议描述",
      "priority": "high|medium|low",
      "rationale": "建议依据"
    }
  ]
}
```

## 质量检查清单

- [ ] 所有分析基于截图可见数据
- [ ] 风险等级评估合理
- [ ] 建议具体可执行
- [ ] 未假设截图中不可见的持仓
- [ ] JSON 格式正确
